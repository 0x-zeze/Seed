use cosmwasm_std::{
    entry_point, to_json_binary, BankMsg, Binary, Coin, CosmosMsg, Decimal, Decimal256, Deps,
    DepsMut, Empty, Env, Int128, MessageInfo, Order, Reply, Response, StdError, StdResult,
    SubMsg, SubMsgResult, Uint128, Uint256,
};
use cw_storage_plus::Bound;
use cw2::set_contract_version;
use injective_std::types::injective::exchange::v1beta1::{
    DerivativeOrder, MsgCreateDerivativeMarketOrder, MsgDeposit, MsgWithdraw, OrderInfo, OrderType,
};

pub const REPLY_OPEN_POSITION: u64 = 1;
pub const REPLY_CLOSE_POSITION: u64 = 2;

use crate::error::ContractError;
use crate::helpers::{
    add_u128, coin_u128, ensure_not_paused, ensure_owner, ensure_trader_or_owner, one_coin,
    pending_yield, settle, sub_u128,
};
#[cfg(test)]
use crate::helpers::add_i128;
use crate::msg::{
    AmountResponse, ConfigResponse, DistributionInfoResponse, ExecuteMsg, InstantiateMsg,
    OpenPositionsResponse, PositionsResponse, QueryMsg, RiskConfigResponse,
    TreasuryStateResponse, UserInfoResponse, UserStakeResponse,
};
use crate::state::{
    Config, Position, CONFIG, CONTRACT_NAME, CONTRACT_VERSION, GLOBAL_INDEX, LAST_DISTRIBUTION,
    LAST_POSITION_OPENED, PENDING_OWNER, POSITIONS, REALIZED_PNL_NET, RESERVED_MARGIN,
    REWARDS_RESERVED, TOTAL_DISTRIBUTED, TOTAL_STAKED, TREASURY_BALANCE, USER_CLAIMED,
    USER_PENDING, USER_STAKE,
};

#[entry_point]
pub fn instantiate(
    deps: DepsMut<Empty>,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response<Empty>, ContractError> {
    if msg.seed_denom.is_empty() || msg.treasury_denom.is_empty() {
        return Err(StdError::msg("seed and treasury denom are required").into());
    }
    if msg.seed_denom == msg.treasury_denom {
        return Err(StdError::msg("seed and treasury denom must differ").into());
    }

    let max_leverage = msg
        .max_leverage
        .unwrap_or_else(|| Decimal::from_ratio(3u128, 1u128));
    if max_leverage < Decimal::one() {
        return Err(StdError::msg("max_leverage must be at least 1").into());
    }

    let max_exposure_bps = msg
        .max_exposure_bps
        .or(msg.max_total_margin_bps)
        .unwrap_or(5000);
    if max_exposure_bps > 10_000 {
        return Err(StdError::msg("max_exposure_bps cannot exceed 10000").into());
    }

    let max_position_margin = msg.max_position_margin.unwrap_or(Uint128::MAX);
    if max_position_margin.is_zero() {
        return Err(StdError::msg("max_position_margin must be greater than zero").into());
    }

    let max_position_size = msg.max_position_size.unwrap_or(Uint128::MAX);
    if max_position_size.is_zero() {
        return Err(StdError::msg("max_position_size must be greater than zero").into());
    }

    let cooldown_seconds = msg.cooldown_seconds.unwrap_or(0);

    let config = Config {
        owner: deps.api.addr_validate(&msg.owner)?,
        seed_denom: msg.seed_denom,
        treasury_denom: msg.treasury_denom,
        paused: false,
        trader: msg
            .trader
            .as_deref()
            .map(|trader| deps.api.addr_validate(trader))
            .transpose()?,
        allowed_markets: msg.allowed_markets,
        max_leverage,
        max_position_size,
        max_exposure_bps,
        max_position_margin,
        cooldown_seconds,
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
    LAST_POSITION_OPENED.save(deps.storage, &0u64)?;

    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_attribute("owner", config.owner)
        .add_attribute("seed_denom", config.seed_denom)
        .add_attribute("treasury_denom", config.treasury_denom))
}

#[entry_point]
pub fn execute(
    deps: DepsMut<Empty>,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response<Empty>, ContractError> {
    match msg {
        ExecuteMsg::Stake {} => stake(deps, info),
        ExecuteMsg::Unstake { amount } => unstake(deps, info, amount),
        ExecuteMsg::DepositTreasury {} => deposit_treasury(deps, info),
        ExecuteMsg::Distribute { amount } => distribute(deps, env, info, amount),
        ExecuteMsg::Pause {} => set_paused(deps, info, true),
        ExecuteMsg::Unpause {} => set_paused(deps, info, false),
        ExecuteMsg::ProposeOwner { new_owner } => propose_owner(deps, info, new_owner),
        ExecuteMsg::AcceptOwner {} => accept_owner(deps, info),
        ExecuteMsg::UpdateTrader { new_trader } => update_trader(deps, info, new_trader),
        ExecuteMsg::UpdateConfig {
            max_leverage,
            max_position_size,
            max_exposure_bps,
            max_total_margin_bps,
            max_position_margin,
            cooldown_seconds,
            allowed_markets,
        } => update_config(
            deps,
            info,
            max_leverage,
            max_position_size,
            max_exposure_bps,
            max_total_margin_bps,
            max_position_margin,
            cooldown_seconds,
            allowed_markets,
        ),
        ExecuteMsg::Claim {} => claim(deps, info),
        ExecuteMsg::OpenPosition {
            market_id,
            is_buy,
            quantity,
            price,
            margin,
            leverage,
        } => open_position(
            deps, env, info, market_id, is_buy, quantity, price, margin, leverage,
        ),
        ExecuteMsg::ClosePosition {
            market_id,
            quantity,
        } => close_position(deps, env, info, market_id, quantity),
        #[cfg(test)]
        ExecuteMsg::RecordPnl { delta } => record_pnl(deps, info, delta),
    }
}

fn stake(
    deps: DepsMut<Empty>,
    info: MessageInfo,
) -> Result<Response<Empty>, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    ensure_not_paused(&config)?;
    let amount = coin_u128(one_coin(&info.funds, &config.seed_denom)?.amount)?;

    settle(deps.storage, &info.sender)?;
    USER_STAKE.update(
        deps.storage,
        &info.sender,
        |stake| -> Result<_, ContractError> { add_u128(stake.unwrap_or_default(), amount) },
    )?;
    let new_total = TOTAL_STAKED.update(deps.storage, |total| add_u128(total, amount))?;

    Ok(Response::new()
        .add_attribute("action", "seed_staked")
        .add_attribute("user", info.sender)
        .add_attribute("amount", amount)
        .add_attribute("total_staked", new_total))
}

fn unstake(
    deps: DepsMut<Empty>,
    info: MessageInfo,
    amount: Uint128,
) -> Result<Response<Empty>, ContractError> {
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
    let new_total = TOTAL_STAKED.update(deps.storage, |total| sub_u128(total, amount))?;

    let send = CosmosMsg::Bank(BankMsg::Send {
        to_address: info.sender.to_string(),
        amount: vec![Coin::new(amount, config.seed_denom)],
    });

    Ok(Response::new()
        .add_message(send)
        .add_attribute("action", "seed_unstaked")
        .add_attribute("user", info.sender)
        .add_attribute("amount", amount)
        .add_attribute("remaining_stake", remaining)
        .add_attribute("total_staked", new_total))
}

fn deposit_treasury(
    deps: DepsMut<Empty>,
    info: MessageInfo,
) -> Result<Response<Empty>, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    ensure_owner(&config, &info.sender)?;
    let amount = coin_u128(one_coin(&info.funds, &config.treasury_denom)?.amount)?;

    let new_balance = TREASURY_BALANCE.update(deps.storage, |balance| add_u128(balance, amount))?;
    // TODO: deposit into the exchange module subaccount once margin positions are wired.
    // Funds stay on the contract balance for now.

    Ok(Response::new()
        .add_attribute("action", "treasury_deposited")
        .add_attribute("owner", info.sender)
        .add_attribute("amount", amount)
        .add_attribute("treasury_balance", new_balance))
}

/// Profit above the high-water mark moves from free treasury into the reward index.
fn distribute(
    deps: DepsMut<Empty>,
    env: Env,
    info: MessageInfo,
    amount: Uint128,
) -> Result<Response<Empty>, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    ensure_owner(&config, &info.sender)?;
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
        .add_attribute("new_reward_index", new_index.to_string())
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
    deps: DepsMut<Empty>,
    info: MessageInfo,
    paused: bool,
) -> Result<Response<Empty>, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;
    ensure_owner(&config, &info.sender)?;
    config.paused = paused;
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new()
        .add_attribute("action", if paused { "paused" } else { "unpaused" })
        .add_attribute("owner", info.sender))
}

fn propose_owner(
    deps: DepsMut<Empty>,
    info: MessageInfo,
    new_owner: String,
) -> Result<Response<Empty>, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    ensure_owner(&config, &info.sender)?;
    let pending = deps.api.addr_validate(&new_owner)?;
    if pending == config.owner {
        return Err(ContractError::Unauthorized);
    }
    PENDING_OWNER.save(deps.storage, &pending)?;

    Ok(Response::new()
        .add_attribute("action", "owner_proposed")
        .add_attribute("current_owner", info.sender)
        .add_attribute("pending_owner", pending))
}

fn accept_owner(
    deps: DepsMut<Empty>,
    info: MessageInfo,
) -> Result<Response<Empty>, ContractError> {
    let pending = PENDING_OWNER
        .may_load(deps.storage)?
        .ok_or(ContractError::Unauthorized)?;
    if info.sender != pending {
        return Err(ContractError::Unauthorized);
    }

    let prev = CONFIG.load(deps.storage)?.owner;
    CONFIG.update(deps.storage, |mut config| -> Result<_, ContractError> {
        config.owner = pending.clone();
        Ok(config)
    })?;
    PENDING_OWNER.remove(deps.storage);

    Ok(Response::new()
        .add_attribute("action", "owner_accepted")
        .add_attribute("previous_owner", prev)
        .add_attribute("owner", pending))
}

fn update_trader(
    deps: DepsMut<Empty>,
    info: MessageInfo,
    new_trader: Option<String>,
) -> Result<Response<Empty>, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;
    ensure_owner(&config, &info.sender)?;

    let trader_addr = new_trader
        .as_deref()
        .map(|trader| deps.api.addr_validate(trader))
        .transpose()?;

    config.trader = trader_addr.clone();
    CONFIG.save(deps.storage, &config)?;

    let mut res = Response::new().add_attribute("action", "trader_updated");
    if let Some(trader) = trader_addr {
        res = res.add_attribute("trader", trader);
    } else {
        res = res.add_attribute("trader", "none");
    }
    res = res.add_attribute("owner", info.sender);
    Ok(res)
}

#[allow(clippy::too_many_arguments)]
fn update_config(
    deps: DepsMut<Empty>,
    info: MessageInfo,
    max_leverage: Option<Decimal>,
    max_position_size: Option<Uint128>,
    max_exposure_bps: Option<u16>,
    max_total_margin_bps: Option<u16>,
    max_position_margin: Option<Uint128>,
    cooldown_seconds: Option<u64>,
    allowed_markets: Option<Vec<String>>,
) -> Result<Response<Empty>, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;
    ensure_owner(&config, &info.sender)?;

    let exposure = max_exposure_bps.or(max_total_margin_bps);
    if let Some(max_bps) = exposure {
        if max_bps > 10_000 {
            return Err(StdError::msg("max_exposure_bps cannot exceed 10000").into());
        }
        config.max_exposure_bps = max_bps;
    }

    if let Some(lev) = max_leverage {
        if lev < Decimal::one() {
            return Err(StdError::msg("max_leverage must be at least 1").into());
        }
        config.max_leverage = lev;
    }

    if let Some(pos_margin) = max_position_margin {
        if pos_margin.is_zero() {
            return Err(StdError::msg("max_position_margin must be greater than zero").into());
        }
        config.max_position_margin = pos_margin;
    }

    if let Some(pos_size) = max_position_size {
        if pos_size.is_zero() {
            return Err(StdError::msg("max_position_size must be greater than zero").into());
        }
        config.max_position_size = pos_size;
    }

    if let Some(cooldown) = cooldown_seconds {
        config.cooldown_seconds = cooldown;
    }

    if let Some(ref markets) = allowed_markets {
        config.allowed_markets = markets.clone();
    }

    CONFIG.save(deps.storage, &config)?;

    let mut res = Response::new()
        .add_attribute("action", "config_updated")
        .add_attribute("owner", info.sender);

    if let Some(lev) = max_leverage {
        res = res.add_attribute("max_leverage", lev.to_string());
    }
    if let Some(pos_size) = max_position_size {
        res = res.add_attribute("max_position_size", pos_size);
    }
    if let Some(max_bps) = exposure {
        res = res.add_attribute("max_exposure_bps", max_bps.to_string());
    }
    if let Some(pos_margin) = max_position_margin {
        res = res.add_attribute("max_position_margin", pos_margin);
    }
    if let Some(cooldown) = cooldown_seconds {
        res = res.add_attribute("cooldown_seconds", cooldown.to_string());
    }
    if let Some(ref markets) = allowed_markets {
        res = res.add_attribute("allowed_markets", markets.join(","));
    }

    Ok(res)
}

/// Pays settled yield. Open while paused so users can always collect what they earned.
fn claim(
    deps: DepsMut<Empty>,
    info: MessageInfo,
) -> Result<Response<Empty>, ContractError> {
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
    deps: DepsMut<Empty>,
    info: MessageInfo,
    delta: Int128,
) -> Result<Response<Empty>, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    ensure_owner(&config, &info.sender)?;
    REALIZED_PNL_NET.update(deps.storage, |net| add_i128(net, delta))?;
    Ok(Response::new().add_attribute("action", "pnl_recorded"))
}

fn parse_units(raw: &str, name: &str) -> Result<Uint128, ContractError> {
    if raw.is_empty() || !raw.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(ContractError::InvalidAmount);
    }
    raw.parse::<Uint128>()
        .map_err(|_| StdError::msg(format!("invalid {name}")).into())
}

#[allow(clippy::too_many_arguments)]
fn open_position(
    deps: DepsMut<Empty>,
    env: Env,
    info: MessageInfo,
    market_id: String,
    is_buy: bool,
    quantity: String,
    price: String,
    margin: String,
    leverage: Option<String>,
) -> Result<Response<Empty>, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    ensure_trader_or_owner(&config, &info.sender)?;
    ensure_not_paused(&config)?;

    if !config.allowed_markets.iter().any(|m| m == &market_id) {
        return Err(ContractError::MarketNotAllowed { market_id });
    }

    let now = env.block.time.seconds();
    let last_opened = LAST_POSITION_OPENED
        .may_load(deps.storage)?
        .unwrap_or_default();
    if last_opened > 0
        && config.cooldown_seconds > 0
        && now < last_opened.saturating_add(config.cooldown_seconds)
    {
        return Err(ContractError::CooldownActive);
    }

    let qty_u128 = parse_units(&quantity, "quantity")?;
    let price_u128 = parse_units(&price, "price")?;
    let margin_u128 = parse_units(&margin, "margin")?;

    if qty_u128.is_zero() || margin_u128.is_zero() {
        return Err(ContractError::ZeroAmount);
    }
    if price_u128.is_zero() {
        return Err(ContractError::InvalidPrice);
    }

    if POSITIONS.has(deps.storage, &market_id) {
        return Err(ContractError::PositionAlreadyOpen { market_id });
    }

    if margin_u128 > config.max_position_margin || qty_u128 > config.max_position_size {
        return Err(ContractError::PositionTooLarge);
    }

    if let Some(ref lev_str) = leverage {
        let lev: Decimal = lev_str
            .parse()
            .map_err(|_| StdError::msg("invalid leverage"))?;
        if lev > config.max_leverage {
            return Err(ContractError::LeverageTooHigh);
        }
    }

    let notional_256 = Uint256::from(qty_u128)
        .checked_mul(Uint256::from(price_u128))
        .map_err(|e| ContractError::Std(StdError::msg(e)))?;
    let max_notional = Decimal256::from(config.max_leverage)
        .checked_mul(Decimal256::from_ratio(margin_u128, 1u128))
        .map_err(|e| ContractError::Std(StdError::msg(e)))?;
    if Decimal256::from_ratio(notional_256, 1u128) > max_notional {
        return Err(ContractError::LeverageTooHigh);
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
        Uint128::from(config.max_exposure_bps),
        Uint128::new(10_000),
    );
    let new_reserved = add_u128(cur_reserved, margin_u128)?;
    if new_reserved > max_allowed_reserved {
        return Err(ContractError::ExposureLimitExceeded);
    }

    TREASURY_BALANCE.update(deps.storage, |bal| sub_u128(bal, margin_u128))?;
    RESERVED_MARGIN.update(deps.storage, |res| add_u128(res, margin_u128))?;
    LAST_POSITION_OPENED.save(deps.storage, &now)?;

    let lev_val = if let Some(ref lev) = leverage {
        lev.clone()
    } else {
        let notional = Uint256::from(qty_u128)
            .checked_mul(Uint256::from(price_u128))
            .map_err(|e| ContractError::Std(StdError::msg(e)))?;
        Decimal256::from_ratio(notional, margin_u128).to_string()
    };

    let pos = Position {
        market_id: market_id.clone(),
        is_buy,
        quantity: qty_u128,
        price: price_u128,
        margin: margin_u128,
        leverage: leverage.clone(),
        opened_at: now,
    };
    POSITIONS.save(deps.storage, &market_id, &pos)?;

    let subaccount_id = crate::addresses::default_subaccount_id(&env.contract.address)?;

    let deposit_msg: CosmosMsg<Empty> = MsgDeposit {
        sender: env.contract.address.to_string(),
        subaccount_id: subaccount_id.clone(),
        amount: Some(injective_std::types::cosmos::base::v1beta1::Coin {
            denom: config.treasury_denom.clone(),
            amount: margin_u128.to_string(),
        }),
    }
    .into();

    let order_msg: CosmosMsg<Empty> = MsgCreateDerivativeMarketOrder {
        sender: env.contract.address.to_string(),
        order: Some(DerivativeOrder {
            market_id: market_id.clone(),
            order_info: Some(OrderInfo {
                subaccount_id,
                fee_recipient: env.contract.address.to_string(),
                price: price_u128.to_string(),
                quantity: qty_u128.to_string(),
                cid: "".to_string(),
            }),
            order_type: if is_buy {
                OrderType::Buy as i32
            } else {
                OrderType::Sell as i32
            },
            margin: margin_u128.to_string(),
            trigger_price: "".to_string(),
        }),
    }
    .into();

    let order_submsg = SubMsg::reply_on_success(order_msg, REPLY_OPEN_POSITION);

    Ok(Response::new()
        .add_message(deposit_msg)
        .add_submessage(order_submsg)
        .add_attribute("action", "open_position")
        .add_attribute("trader", &info.sender)
        .add_attribute("market_id", &market_id)
        .add_attribute("market", &market_id)
        .add_attribute("margin", margin_u128)
        .add_attribute("leverage", lev_val)
        .add_attribute("is_buy", is_buy.to_string())
        .add_attribute("quantity", qty_u128)
        .add_attribute("price", price_u128))
}

fn close_position(
    deps: DepsMut<Empty>,
    env: Env,
    info: MessageInfo,
    market_id: String,
    quantity: Option<String>,
) -> Result<Response<Empty>, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    ensure_trader_or_owner(&config, &info.sender)?;

    let pos = POSITIONS
        .may_load(deps.storage, &market_id)?
        .ok_or_else(|| ContractError::PositionNotFound {
            market_id: market_id.clone(),
        })?;

    if let Some(ref q) = quantity {
        let qty = parse_units(q, "quantity")?;
        if qty.is_zero() {
            return Err(ContractError::ZeroAmount);
        }
        if qty != pos.quantity {
            return Err(ContractError::PartialCloseNotSupported);
        }
    }

    RESERVED_MARGIN.update(deps.storage, |margin| sub_u128(margin, pos.margin))?;
    TREASURY_BALANCE.update(deps.storage, |free| add_u128(free, pos.margin))?;
    POSITIONS.remove(deps.storage, &market_id);

    let subaccount_id = crate::addresses::default_subaccount_id(&env.contract.address)?;

    let close_order: CosmosMsg<Empty> = MsgCreateDerivativeMarketOrder {
        sender: env.contract.address.to_string(),
        order: Some(DerivativeOrder {
            market_id: market_id.clone(),
            order_info: Some(OrderInfo {
                subaccount_id: subaccount_id.clone(),
                fee_recipient: env.contract.address.to_string(),
                price: pos.price.to_string(),
                quantity: pos.quantity.to_string(),
                cid: "".to_string(),
            }),
            order_type: if pos.is_buy {
                OrderType::Sell as i32
            } else {
                OrderType::Buy as i32
            },
            margin: "0".to_string(),
            trigger_price: "".to_string(),
        }),
    }
    .into();

    let withdraw_msg: CosmosMsg<Empty> = MsgWithdraw {
        sender: env.contract.address.to_string(),
        subaccount_id,
        amount: Some(injective_std::types::cosmos::base::v1beta1::Coin {
            denom: config.treasury_denom,
            amount: pos.margin.to_string(),
        }),
    }
    .into();

    let close_submsg = SubMsg::reply_on_success(close_order, REPLY_CLOSE_POSITION);

    Ok(Response::new()
        .add_submessage(close_submsg)
        .add_message(withdraw_msg)
        .add_attribute("action", "close_position")
        .add_attribute("trader", info.sender)
        .add_attribute("market_id", &market_id)
        .add_attribute("market", &market_id)
        .add_attribute(
            "quantity",
            quantity.unwrap_or_else(|| pos.quantity.to_string()),
        )
        .add_attribute("margin_released", pos.margin)
        .add_attribute("realized_pnl", "0"))
}

#[entry_point]
pub fn reply(
    _deps: DepsMut<Empty>,
    _env: Env,
    msg: Reply,
) -> Result<Response<Empty>, ContractError> {
    match msg.id {
        REPLY_OPEN_POSITION => match msg.result {
            SubMsgResult::Ok(_) => {
                Ok(Response::new().add_attribute("action", "reply_open_position"))
            }
            SubMsgResult::Err(err) => Err(ContractError::Std(StdError::msg(format!(
                "open position execution failed: {err}"
            )))),
        },
        REPLY_CLOSE_POSITION => match msg.result {
            SubMsgResult::Ok(_) => {
                Ok(Response::new().add_attribute("action", "reply_close_position"))
            }
            SubMsgResult::Err(err) => Err(ContractError::Std(StdError::msg(format!(
                "close position execution failed: {err}"
            )))),
        },
        _ => Err(ContractError::Std(StdError::msg(format!(
            "unknown reply id: {}",
            msg.id
        )))),
    }
}

#[entry_point]
pub fn query(deps: Deps<Empty>, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_json_binary(&query_config(deps)?),
        QueryMsg::UserStake { user } => to_json_binary(&query_user_stake(deps, user)?),
        QueryMsg::TotalStaked {} => to_json_binary(&AmountResponse {
            amount: TOTAL_STAKED.load(deps.storage)?,
        }),
        QueryMsg::TreasuryBalance {} => to_json_binary(&AmountResponse {
            amount: TREASURY_BALANCE.load(deps.storage)?,
        }),
        QueryMsg::TreasuryState {} => to_json_binary(&query_treasury_state(deps)?),
        QueryMsg::PendingYield { user } => to_json_binary(&query_pending(deps, user)?),
        QueryMsg::Positions {} => to_json_binary(&query_positions(deps)?),
        QueryMsg::OpenPositions { start_after, limit } => {
            to_json_binary(&query_open_positions(deps, start_after, limit)?)
        }
        QueryMsg::UserInfo { user } => to_json_binary(&query_user_info(deps, user)?),
        QueryMsg::DistributionInfo {} => to_json_binary(&query_distribution(deps)?),
        QueryMsg::RiskConfig {} => to_json_binary(&query_risk_config(deps)?),
    }
}

fn query_config(deps: Deps<Empty>) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;
    Ok(ConfigResponse {
        seed_token: config.seed_denom,
        treasury_denom: config.treasury_denom,
        owner: config.owner.into_string(),
        paused: config.paused,
        trader: config.trader.map(|trader| trader.into_string()),
        max_leverage: config.max_leverage.to_string(),
        max_position_size: config.max_position_size,
        max_total_margin_bps: config.max_exposure_bps,
        max_exposure_bps: config.max_exposure_bps,
        max_position_margin: config.max_position_margin,
        cooldown_seconds: config.cooldown_seconds,
        allowed_markets: config.allowed_markets,
    })
}

fn query_risk_config(deps: Deps<Empty>) -> StdResult<RiskConfigResponse> {
    let config = CONFIG.load(deps.storage)?;
    let free_treasury = TREASURY_BALANCE.load(deps.storage)?;
    let reserved = RESERVED_MARGIN.load(deps.storage)?;
    let total_pool = free_treasury + reserved;
    let current_exposure_bps = if total_pool.is_zero() {
        0u16
    } else {
        reserved
            .multiply_ratio(10_000u128, total_pool)
            .u128() as u16
    };

    Ok(RiskConfigResponse {
        allowed_markets: config.allowed_markets,
        max_leverage: config.max_leverage.to_string(),
        max_exposure_bps: config.max_exposure_bps,
        max_position_margin: config.max_position_margin,
        cooldown_seconds: config.cooldown_seconds,
        current_exposure_bps,
        current_used_exposure: reserved,
        reserved_margin: reserved,
        total_treasury: total_pool,
    })
}

fn query_user_stake(
    deps: Deps<Empty>,
    user: String,
) -> StdResult<UserStakeResponse> {
    let user = deps.api.addr_validate(&user)?;
    Ok(UserStakeResponse {
        amount: USER_STAKE
            .may_load(deps.storage, &user)?
            .unwrap_or_default(),
    })
}

fn query_pending(deps: Deps<Empty>, user: String) -> StdResult<AmountResponse> {
    let user = deps.api.addr_validate(&user)?;
    Ok(AmountResponse {
        amount: pending_yield(deps.storage, &user)?,
    })
}

fn query_user_info(deps: Deps<Empty>, user: String) -> StdResult<UserInfoResponse> {
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

fn query_distribution(deps: Deps<Empty>) -> StdResult<DistributionInfoResponse> {
    Ok(DistributionInfoResponse {
        total_distributed: TOTAL_DISTRIBUTED.load(deps.storage)?,
        rewards_reserved: REWARDS_RESERVED.load(deps.storage)?,
        last_distribution: LAST_DISTRIBUTION.load(deps.storage)?,
        global_index: GLOBAL_INDEX.load(deps.storage)?.to_string(),
    })
}

fn query_treasury_state(deps: Deps<Empty>) -> StdResult<TreasuryStateResponse> {
    let balance = TREASURY_BALANCE.load(deps.storage)?;
    let reserved_margin = RESERVED_MARGIN.load(deps.storage)?;
    let net = REALIZED_PNL_NET.load(deps.storage)?;
    let distributed = TOTAL_DISTRIBUTED.load(deps.storage)?;
    let undistributed = distributable_profit(net, distributed)
        .map_err(|e| StdError::msg(e.to_string()))?;

    Ok(TreasuryStateResponse {
        balance,
        reserved_margin,
        undistributed,
    })
}

fn query_positions(deps: Deps<Empty>) -> StdResult<PositionsResponse> {
    let positions = POSITIONS
        .range(deps.storage, None, None, Order::Ascending)
        .take(100)
        .map(|res| res.map(|(market_id, _)| market_id))
        .collect::<StdResult<Vec<String>>>()?;

    Ok(PositionsResponse { positions })
}

fn query_open_positions(
    deps: Deps<Empty>,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<OpenPositionsResponse> {
    let limit = limit.unwrap_or(30).min(100) as usize;
    let start = start_after.as_deref().map(Bound::exclusive);
    let positions = POSITIONS
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|res| res.map(|(_, pos)| pos))
        .collect::<StdResult<Vec<Position>>>()?;

    Ok(OpenPositionsResponse { positions })
}
