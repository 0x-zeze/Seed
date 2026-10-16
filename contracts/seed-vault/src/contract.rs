use cosmwasm_std::{
    entry_point, to_json_binary, BankMsg, Binary, Coin, CosmosMsg, Decimal, Decimal256, Deps,
    DepsMut, Env, Int128, MessageInfo, Response, StdError, StdResult, Uint128, Uint256,
};
use cw2::set_contract_version;
use injective_cosmwasm::{InjectiveMsgWrapper, InjectiveQueryWrapper};

use crate::error::ContractError;
use crate::helpers::{
    add_i128, add_u128, coin_u128, ensure_admin, ensure_not_paused, one_coin, pending_yield,
    settle, sub_u128,
};
use crate::msg::{
    AmountResponse, ConfigResponse, DistributionInfoResponse, ExecuteMsg, InstantiateMsg,
    PositionsResponse, QueryMsg, UserInfoResponse, UserStakeResponse,
};
use crate::state::{
    Config, CONFIG, CONTRACT_NAME, CONTRACT_VERSION, GLOBAL_INDEX, LAST_DISTRIBUTION,
    PENDING_ADMIN, REALIZED_PNL_NET, RESERVED_MARGIN, REWARDS_RESERVED, TOTAL_DISTRIBUTED,
    TOTAL_STAKED, TREASURY_BALANCE, USER_CLAIMED, USER_PENDING, USER_STAKE,
};

#[entry_point]
pub fn instantiate(
    deps: DepsMut<InjectiveQueryWrapper>,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response<InjectiveMsgWrapper>, ContractError> {
    if msg.seed_denom.is_empty() || msg.treasury_denom.is_empty() {
        return Err(StdError::msg("seed and treasury denom are required").into());
    }
    if msg.seed_denom == msg.treasury_denom {
        return Err(StdError::msg("seed and treasury denom must differ").into());
    }

    if msg.max_total_margin_bps > 10_000 {
        return Err(StdError::msg("max_total_margin_bps cannot exceed 10000").into());
    }
    if msg.max_leverage < Decimal::one() {
        return Err(StdError::msg("max_leverage must be at least 1").into());
    }

    let config = Config {
        admin: deps.api.addr_validate(&msg.admin)?,
        seed_denom: msg.seed_denom,
        treasury_denom: msg.treasury_denom,
        paused: false,
        keeper: msg
            .keeper
            .as_deref()
            .map(|keeper| deps.api.addr_validate(keeper))
            .transpose()?,
        max_leverage: msg.max_leverage,
        max_position_size: msg.max_position_size,
        max_total_margin_bps: msg.max_total_margin_bps,
        allowed_markets: msg.allowed_markets,
    };

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    CONFIG.save(deps.storage, &config)?;
    TOTAL_STAKED.save(deps.storage, &Uint128::zero())?;
    TREASURY_BALANCE.save(deps.storage, &Uint128::zero())?;
    RESERVED_MARGIN.save(deps.storage, &Uint128::zero())?;
    GLOBAL_INDEX.save(deps.storage, &Decimal256::zero())?;
    REWARDS_RESERVED.save(deps.storage, &Uint128::zero())?;
    TOTAL_DISTRIBUTED.save(deps.storage, &Uint128::zero())?;
    REALIZED_PNL_NET.save(deps.storage, &Int128::zero())?;
    LAST_DISTRIBUTION.save(deps.storage, &0u64)?;

    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_attribute("admin", config.admin)
        .add_attribute("seed_denom", config.seed_denom)
        .add_attribute("treasury_denom", config.treasury_denom))
}

#[entry_point]
pub fn execute(
    deps: DepsMut<InjectiveQueryWrapper>,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response<InjectiveMsgWrapper>, ContractError> {
    match msg {
        ExecuteMsg::Stake {} => stake(deps, info),
        ExecuteMsg::Unstake { amount } => unstake(deps, info, amount),
        ExecuteMsg::DepositTreasury {} => deposit_treasury(deps, info),
        ExecuteMsg::Distribute { amount } => distribute(deps, env, info, amount),
        ExecuteMsg::Pause {} => set_paused(deps, info, true),
        ExecuteMsg::Unpause {} => set_paused(deps, info, false),
        ExecuteMsg::ProposeAdmin { new_admin } => propose_admin(deps, info, new_admin),
        ExecuteMsg::AcceptAdmin {} => accept_admin(deps, info),
        ExecuteMsg::Claim {} => claim(deps, info),
        ExecuteMsg::OpenPosition {
            market_id,
            is_buy,
            quantity,
            price,
            margin,
            leverage,
        } => open_position(
            deps, info, market_id, is_buy, quantity, price, margin, leverage,
        ),
        ExecuteMsg::ClosePosition {
            market_id,
            quantity,
        } => close_position(deps, info, market_id, quantity),
        #[cfg(test)]
        ExecuteMsg::RecordPnl { delta } => record_pnl(deps, info, delta),
    }
}

fn stake(
    deps: DepsMut<InjectiveQueryWrapper>,
    info: MessageInfo,
) -> Result<Response<InjectiveMsgWrapper>, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    ensure_not_paused(&config)?;
    let amount = coin_u128(one_coin(&info.funds, &config.seed_denom)?.amount)?;

    settle(deps.storage, &info.sender)?;
    USER_STAKE.update(
        deps.storage,
        &info.sender,
        |stake| -> Result<_, ContractError> { add_u128(stake.unwrap_or_default(), amount) },
    )?;
    TOTAL_STAKED.update(deps.storage, |total| add_u128(total, amount))?;

    Ok(Response::new()
        .add_attribute("action", "seed_staked")
        .add_attribute("user", info.sender)
        .add_attribute("amount", amount))
}

fn unstake(
    deps: DepsMut<InjectiveQueryWrapper>,
    info: MessageInfo,
    amount: Uint128,
) -> Result<Response<InjectiveMsgWrapper>, ContractError> {
    if amount.is_zero() {
        return Err(ContractError::ZeroAmount);
    }
    let config = CONFIG.load(deps.storage)?;
    let staked = USER_STAKE
        .may_load(deps.storage, &info.sender)?
        .unwrap_or_default();
    if staked < amount {
        return Err(ContractError::InsufficientStake {
            have: staked.to_string(),
            need: amount.to_string(),
        });
    }

    settle(deps.storage, &info.sender)?;
    let remaining = sub_u128(staked, amount)?;
    USER_STAKE.save(deps.storage, &info.sender, &remaining)?;
    TOTAL_STAKED.update(deps.storage, |total| sub_u128(total, amount))?;

    let send = CosmosMsg::Bank(BankMsg::Send {
        to_address: info.sender.to_string(),
        amount: vec![Coin::new(amount, config.seed_denom)],
    });

    Ok(Response::new()
        .add_message(send)
        .add_attribute("action", "seed_unstaked")
        .add_attribute("user", info.sender)
        .add_attribute("amount", amount))
}

fn deposit_treasury(
    deps: DepsMut<InjectiveQueryWrapper>,
    info: MessageInfo,
) -> Result<Response<InjectiveMsgWrapper>, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    ensure_admin(&config, &info.sender)?;
    ensure_not_paused(&config)?;
    let amount = coin_u128(one_coin(&info.funds, &config.treasury_denom)?.amount)?;

    TREASURY_BALANCE.update(deps.storage, |balance| add_u128(balance, amount))?;
    // TODO: deposit into the exchange module subaccount once margin positions are wired.
    // Funds stay on the contract balance for now.

    Ok(Response::new()
        .add_attribute("action", "treasury_deposited")
        .add_attribute("admin", info.sender)
        .add_attribute("amount", amount))
}

/// Profit above the high-water mark moves from free treasury into the reward index.
fn distribute(
    deps: DepsMut<InjectiveQueryWrapper>,
    env: Env,
    info: MessageInfo,
    amount: Uint128,
) -> Result<Response<InjectiveMsgWrapper>, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    ensure_operator(&config, &info.sender)?;
    ensure_not_paused(&config)?;
    if amount.is_zero() {
        return Err(ContractError::InvalidAmount);
    }

    let total = TOTAL_STAKED.load(deps.storage)?;
    if total.is_zero() {
        return Err(ContractError::NoStakers);
    }

    let distributable = distributable_profit(
        REALIZED_PNL_NET.load(deps.storage)?,
        TOTAL_DISTRIBUTED.load(deps.storage)?,
    )?;
    if amount > distributable {
        return Err(ContractError::InvalidAmount);
    }

    let treasury = TREASURY_BALANCE.load(deps.storage)?;
    if amount > treasury {
        return Err(ContractError::InsufficientTreasury {
            free: treasury.to_string(),
            need: amount.to_string(),
        });
    }

    let increment = Decimal256::from_ratio(amount, total);
    if increment.is_zero() {
        return Err(ContractError::InvalidAmount);
    }

    let new_index = GLOBAL_INDEX.update(deps.storage, |index| -> Result<_, ContractError> {
        index
            .checked_add(increment)
            .map_err(|err| ContractError::Std(StdError::msg(err)))
    })?;
    TOTAL_DISTRIBUTED.update(deps.storage, |done| add_u128(done, amount))?;
    REWARDS_RESERVED.update(deps.storage, |reserved| add_u128(reserved, amount))?;
    TREASURY_BALANCE.update(deps.storage, |free| sub_u128(free, amount))?;
    LAST_DISTRIBUTION.save(deps.storage, &env.block.height)?;

    Ok(Response::new()
        .add_attribute("action", "yield_distributed")
        .add_attribute("amount", amount)
        .add_attribute("new_index", new_index.to_string())
        .add_attribute("total_staked", total))
}

/// `max(0, REALIZED_PNL_NET - TOTAL_DISTRIBUTED)`. A negative net distributes nothing.
fn distributable_profit(net: Int128, distributed: Uint128) -> Result<Uint128, ContractError> {
    if net.is_negative() || net.is_zero() {
        return Ok(Uint128::zero());
    }
    let net_u128 = u128::try_from(net.i128())
        .map_err(|err| ContractError::Std(StdError::msg(err.to_string())))?;
    Ok(Uint128::new(net_u128).saturating_sub(distributed))
}

fn set_paused(
    deps: DepsMut<InjectiveQueryWrapper>,
    info: MessageInfo,
    paused: bool,
) -> Result<Response<InjectiveMsgWrapper>, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;
    ensure_admin(&config, &info.sender)?;
    config.paused = paused;
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new()
        .add_attribute("action", if paused { "paused" } else { "unpaused" })
        .add_attribute("admin", info.sender))
}

fn propose_admin(
    deps: DepsMut<InjectiveQueryWrapper>,
    info: MessageInfo,
    new_admin: String,
) -> Result<Response<InjectiveMsgWrapper>, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    ensure_admin(&config, &info.sender)?;
    let pending = deps.api.addr_validate(&new_admin)?;
    if pending == config.admin {
        return Err(ContractError::Unauthorized);
    }
    PENDING_ADMIN.save(deps.storage, &pending)?;

    Ok(Response::new()
        .add_attribute("action", "admin_proposed")
        .add_attribute("pending_admin", pending))
}

fn accept_admin(
    deps: DepsMut<InjectiveQueryWrapper>,
    info: MessageInfo,
) -> Result<Response<InjectiveMsgWrapper>, ContractError> {
    let pending = PENDING_ADMIN
        .may_load(deps.storage)?
        .ok_or(ContractError::Unauthorized)?;
    if info.sender != pending {
        return Err(ContractError::Unauthorized);
    }

    CONFIG.update(deps.storage, |mut config| -> Result<_, ContractError> {
        config.admin = pending.clone();
        Ok(config)
    })?;
    PENDING_ADMIN.remove(deps.storage);

    Ok(Response::new()
        .add_attribute("action", "admin_accepted")
        .add_attribute("admin", pending))
}

/// Pays settled yield. Open while paused so users can always collect what they earned.
fn claim(
    deps: DepsMut<InjectiveQueryWrapper>,
    info: MessageInfo,
) -> Result<Response<InjectiveMsgWrapper>, ContractError> {
    settle(deps.storage, &info.sender)?;
    let amount = USER_PENDING
        .may_load(deps.storage, &info.sender)?
        .unwrap_or_default();
    if amount.is_zero() {
        return Err(ContractError::NothingToClaim);
    }

    USER_PENDING.save(deps.storage, &info.sender, &Uint128::zero())?;
    USER_CLAIMED.update(
        deps.storage,
        &info.sender,
        |claimed| -> Result<_, ContractError> { add_u128(claimed.unwrap_or_default(), amount) },
    )?;
    REWARDS_RESERVED.update(deps.storage, |reserved| sub_u128(reserved, amount))?;

    let config = CONFIG.load(deps.storage)?;
    let send = CosmosMsg::Bank(BankMsg::Send {
        to_address: info.sender.to_string(),
        amount: vec![Coin::new(amount, config.treasury_denom)],
    });

    Ok(Response::new()
        .add_message(send)
        .add_attribute("action", "yield_claimed")
        .add_attribute("user", info.sender)
        .add_attribute("amount", amount))
}

/// TEMPORARY until `ClosePosition` updates realized PnL from released margin.
#[cfg(test)]
fn record_pnl(
    deps: DepsMut<InjectiveQueryWrapper>,
    info: MessageInfo,
    delta: Int128,
) -> Result<Response<InjectiveMsgWrapper>, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    ensure_admin(&config, &info.sender)?;
    REALIZED_PNL_NET.update(deps.storage, |net| add_i128(net, delta))?;
    Ok(Response::new().add_attribute("action", "pnl_recorded"))
}

fn ensure_operator(config: &Config, sender: &cosmwasm_std::Addr) -> Result<(), ContractError> {
    if sender == config.admin || config.keeper.as_ref() == Some(sender) {
        Ok(())
    } else {
        Err(ContractError::Unauthorized)
    }
}

fn parse_units(raw: &str, name: &str) -> Result<Uint128, ContractError> {
    if raw.is_empty() || !raw.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(ContractError::InvalidAmount);
    }
    raw.parse::<Uint128>()
        .map_err(|_| StdError::msg(format!("invalid {name}")).into())
}

/// Only place that books realized PnL and releases reserved margin.
/// `released` is the margin the exchange returns. The gap versus `reserved` is the PnL.
fn apply_realized_pnl(
    storage: &mut dyn cosmwasm_std::Storage,
    reserved: Uint128,
    released: Uint128,
) -> Result<Int128, ContractError> {
    let released_i = Int128::new(
        i128::try_from(released.u128())
            .map_err(|err| ContractError::Std(StdError::msg(err.to_string())))?,
    );
    let reserved_i = Int128::new(
        i128::try_from(reserved.u128())
            .map_err(|err| ContractError::Std(StdError::msg(err.to_string())))?,
    );
    let pnl = released_i
        .checked_sub(reserved_i)
        .map_err(|err| ContractError::Std(StdError::msg(err)))?;

    REALIZED_PNL_NET.update(storage, |net| add_i128(net, pnl))?;
    RESERVED_MARGIN.update(storage, |margin| sub_u128(margin, reserved))?;
    TREASURY_BALANCE.update(storage, |free| add_u128(free, released))?;
    Ok(pnl)
}
#[allow(clippy::too_many_arguments)]
fn open_position(
    deps: DepsMut<InjectiveQueryWrapper>,
    info: MessageInfo,
    market_id: String,
    is_buy: bool,
    quantity: String,
    price: String,
    margin: String,
    leverage: Option<String>,
) -> Result<Response<InjectiveMsgWrapper>, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    ensure_operator(&config, &info.sender)?;
    ensure_not_paused(&config)?;

    if !config.allowed_markets.iter().any(|m| m == &market_id) {
        return Err(ContractError::MarketNotAllowed { market_id });
    }

    let qty_u128 = parse_units(&quantity, "quantity")?;
    let price_u128 = parse_units(&price, "price")?;
    let margin_u128 = parse_units(&margin, "margin")?;

    if qty_u128.is_zero() || margin_u128.is_zero() {
        return Err(ContractError::ZeroAmount);
    }

    if qty_u128 > config.max_position_size {
        return Err(ContractError::RiskLimit {
            reason: format!(
                "quantity {} exceeds max_position_size {}",
                qty_u128, config.max_position_size
            ),
        });
    }

    if let Some(ref lev_str) = leverage {
        let lev: Decimal = lev_str
            .parse()
            .map_err(|_| StdError::msg("invalid leverage"))?;
        if lev > config.max_leverage {
            return Err(ContractError::RiskLimit {
                reason: format!(
                    "leverage {} exceeds max_leverage {}",
                    lev, config.max_leverage
                ),
            });
        }
    }

    if !price_u128.is_zero() {
        let notional_256 = Uint256::from(qty_u128)
            .checked_mul(Uint256::from(price_u128))
            .map_err(|e| ContractError::Std(StdError::msg(e)))?;
        let max_notional = Decimal256::from(config.max_leverage)
            .checked_mul(Decimal256::from_ratio(margin_u128, 1u128))
            .map_err(|e| ContractError::Std(StdError::msg(e)))?;
        if Decimal256::from_ratio(notional_256, 1u128) > max_notional {
            return Err(ContractError::RiskLimit {
                reason: "notional exceeds max leverage limit".to_string(),
            });
        }
    }

    let free_treasury = TREASURY_BALANCE.load(deps.storage)?;
    if margin_u128 > free_treasury {
        return Err(ContractError::InsufficientTreasury {
            free: free_treasury.to_string(),
            need: margin_u128.to_string(),
        });
    }

    let cur_reserved = RESERVED_MARGIN.load(deps.storage)?;
    let total_pool = add_u128(free_treasury, cur_reserved)?;
    let max_allowed_reserved = total_pool.multiply_ratio(
        Uint128::from(config.max_total_margin_bps),
        Uint128::new(10_000),
    );
    let new_reserved = add_u128(cur_reserved, margin_u128)?;
    if new_reserved > max_allowed_reserved {
        return Err(ContractError::RiskLimit {
            reason: format!(
                "total reserved margin {} exceeds max_total_margin_bps limit {}",
                new_reserved, max_allowed_reserved
            ),
        });
    }

    TREASURY_BALANCE.update(deps.storage, |bal| sub_u128(bal, margin_u128))?;
    RESERVED_MARGIN.update(deps.storage, |res| add_u128(res, margin_u128))?;

    // TODO: Build the Injective derivative order as CosmosMsg::Any or InjectiveMsgWrapper
    // using injective-std / injective-cosmwasm types once the exchange subaccount id and
    // protobuf wire encoding are finalized. Returning empty message list for now.
    let _ = (is_buy, price_u128);
    let messages: Vec<CosmosMsg<InjectiveMsgWrapper>> = vec![];

    Ok(Response::new()
        .add_messages(messages)
        .add_attribute("action", "open_position")
        .add_attribute("market_id", market_id)
        .add_attribute("margin", margin_u128))
}

fn close_position(
    deps: DepsMut<InjectiveQueryWrapper>,
    info: MessageInfo,
    market_id: String,
    quantity: Option<String>,
) -> Result<Response<InjectiveMsgWrapper>, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    ensure_operator(&config, &info.sender)?;

    if !config.allowed_markets.iter().any(|m| m == &market_id) {
        return Err(ContractError::MarketNotAllowed { market_id });
    }

    if let Some(ref q) = quantity {
        let qty = parse_units(q, "quantity")?;
        if qty.is_zero() {
            return Err(ContractError::ZeroAmount);
        }
    }

    // TODO: Query position from Injective exchange module, build reduce-only closing order,
    // and route PnL & released margin through apply_realized_pnl(deps.storage, reserved, released).
    let _ = apply_realized_pnl;

    Ok(Response::new()
        .add_attribute("action", "close_position")
        .add_attribute("market_id", market_id))
}

#[entry_point]
pub fn query(deps: Deps<InjectiveQueryWrapper>, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_json_binary(&query_config(deps)?),
        QueryMsg::UserStake { user } => to_json_binary(&query_user_stake(deps, user)?),
        QueryMsg::TotalStaked {} => to_json_binary(&AmountResponse {
            amount: TOTAL_STAKED.load(deps.storage)?,
        }),
        QueryMsg::TreasuryBalance {} => to_json_binary(&AmountResponse {
            amount: TREASURY_BALANCE.load(deps.storage)?,
        }),
        QueryMsg::PendingYield { user } => to_json_binary(&query_pending(deps, user)?),
        QueryMsg::Positions {} => to_json_binary(&PositionsResponse { positions: vec![] }),
        QueryMsg::UserInfo { user } => to_json_binary(&query_user_info(deps, user)?),
        QueryMsg::DistributionInfo {} => to_json_binary(&query_distribution(deps)?),
    }
}

fn query_config(deps: Deps<InjectiveQueryWrapper>) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;
    Ok(ConfigResponse {
        seed_token: config.seed_denom,
        treasury_denom: config.treasury_denom,
        admin: config.admin.into_string(),
        paused: config.paused,
        keeper: config.keeper.map(|keeper| keeper.into_string()),
        max_leverage: config.max_leverage.to_string(),
        max_position_size: config.max_position_size,
        max_total_margin_bps: config.max_total_margin_bps,
        allowed_markets: config.allowed_markets,
    })
}

fn query_user_stake(
    deps: Deps<InjectiveQueryWrapper>,
    user: String,
) -> StdResult<UserStakeResponse> {
    let user = deps.api.addr_validate(&user)?;
    Ok(UserStakeResponse {
        amount: USER_STAKE
            .may_load(deps.storage, &user)?
            .unwrap_or_default(),
    })
}

fn query_pending(deps: Deps<InjectiveQueryWrapper>, user: String) -> StdResult<AmountResponse> {
    let user = deps.api.addr_validate(&user)?;
    Ok(AmountResponse {
        amount: pending_yield(deps.storage, &user)?,
    })
}

fn query_user_info(deps: Deps<InjectiveQueryWrapper>, user: String) -> StdResult<UserInfoResponse> {
    let user = deps.api.addr_validate(&user)?;
    let stake = USER_STAKE
        .may_load(deps.storage, &user)?
        .unwrap_or_default();
    let total = TOTAL_STAKED.load(deps.storage)?;
    let share = if total.is_zero() {
        Decimal256::zero()
    } else {
        Decimal256::from_ratio(stake, total)
    };
    Ok(UserInfoResponse {
        stake,
        pending: pending_yield(deps.storage, &user)?,
        claimed: USER_CLAIMED
            .may_load(deps.storage, &user)?
            .unwrap_or_default(),
        share: share.to_string(),
    })
}

fn query_distribution(deps: Deps<InjectiveQueryWrapper>) -> StdResult<DistributionInfoResponse> {
    Ok(DistributionInfoResponse {
        total_distributed: TOTAL_DISTRIBUTED.load(deps.storage)?,
        rewards_reserved: REWARDS_RESERVED.load(deps.storage)?,
        last_distribution: LAST_DISTRIBUTION.load(deps.storage)?,
        global_index: GLOBAL_INDEX.load(deps.storage)?.to_string(),
    })
}
