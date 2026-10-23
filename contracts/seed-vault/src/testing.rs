use cosmwasm_std::testing::{message_info, mock_dependencies, mock_env};
use cosmwasm_std::{
    coin, coins, from_json, BankMsg, Coin, CosmosMsg, Decimal, Decimal256, Int128, Reply,
    SubMsgResponse, SubMsgResult, Uint128,
};

fn inj_deps() -> cosmwasm_std::OwnedDeps<
    cosmwasm_std::testing::MockStorage,
    cosmwasm_std::testing::MockApi,
    cosmwasm_std::testing::MockQuerier,
    cosmwasm_std::Empty,
> {
    let mut deps = mock_dependencies();
    deps.api = deps.api.with_prefix("inj");
    deps
}

use crate::addresses::addr20;
use crate::contract::{
    execute, instantiate, query, reply, REPLY_CLOSE_POSITION, REPLY_OPEN_POSITION,
};
use crate::error::ContractError;
use crate::helpers::pending_yield;
use crate::msg::{
    AmountResponse, ConfigResponse, DistributionInfoResponse, ExecuteMsg, InstantiateMsg,
    OpenPositionsResponse, PositionsResponse, QueryMsg, RiskConfigResponse, TotalStakedResponse,
    TreasuryStateResponse, UserInfoResponse, UserStakeResponse,
};
use crate::state::{
    CONFIG, GLOBAL_INDEX, LAST_DISTRIBUTION, PENDING_OWNER, POSITIONS, REALIZED_PNL_NET,
    RESERVED_MARGIN, REWARDS_RESERVED, TOTAL_DISTRIBUTED, TOTAL_STAKED, TREASURY_BALANCE,
    USER_CLAIMED, USER_INDEX, USER_PENDING, USER_STAKE,
};

const SEED: &str = "factory/inj1seed/seed";
const USDT: &str = "peggy0xusdt";
const MARKET_1: &str = "0xmarket1";

#[test]
fn instantiate_stores_config_and_zero_counters() {
    let mut deps = inj_deps();
    let env = mock_env();
    let owner = addr20("owner");
    let info = message_info(&owner, &[]);

    instantiate(
        deps.as_mut(),
        env.clone(),
        info,
        InstantiateMsg {
            owner: owner.to_string(),
            seed_denom: SEED.to_string(),
            treasury_denom: USDT.to_string(),
            trader: None,
            allowed_markets: vec![MARKET_1.to_string()],
            max_leverage: Some(Decimal::from_ratio(3u128, 1u128)),
            max_position_size: Some(Uint128::new(10_000_000)),
            max_exposure_bps: Some(5_000),
            max_total_margin_bps: None,
            max_position_margin: Some(Uint128::MAX),
            cooldown_seconds: Some(0),
        },
    )
    .unwrap();

    let raw = query(deps.as_ref(), env, QueryMsg::Config {}).unwrap();
    let config: ConfigResponse = from_json(raw).unwrap();
    assert_eq!(config.owner, owner.as_str());
    assert_eq!(config.seed_token, SEED);
    assert_eq!(config.treasury_denom, USDT);
    assert!(!config.paused);
    assert_eq!(config.trader, None);
    assert_eq!(config.max_leverage, "3");
    assert_eq!(config.max_position_size, Uint128::new(10_000_000));
    assert_eq!(config.max_exposure_bps, 5_000);
    assert_eq!(config.max_position_margin, Uint128::MAX);
    assert_eq!(config.cooldown_seconds, 0);
    assert_eq!(config.allowed_markets, vec![MARKET_1.to_string()]);

    let stored = CONFIG.load(&deps.storage).unwrap();
    assert_eq!(stored.owner, owner);
    assert_eq!(stored.seed_denom, SEED);
    assert_eq!(stored.treasury_denom, USDT);
    assert!(!stored.paused);
    assert_eq!(stored.trader, None);
    assert_eq!(stored.max_leverage, Decimal::from_ratio(3u128, 1u128));
    assert_eq!(stored.max_position_size, Uint128::new(10_000_000));
    assert_eq!(stored.max_exposure_bps, 5_000);
    assert_eq!(stored.max_position_margin, Uint128::MAX);
    assert_eq!(stored.cooldown_seconds, 0);
    assert_eq!(stored.allowed_markets, vec![MARKET_1.to_string()]);

    assert!(TOTAL_STAKED.load(&deps.storage).unwrap().is_zero());
    assert!(TREASURY_BALANCE.load(&deps.storage).unwrap().is_zero());
    assert!(RESERVED_MARGIN.load(&deps.storage).unwrap().is_zero());
    assert!(GLOBAL_INDEX.load(&deps.storage).unwrap().is_zero());
    assert!(REWARDS_RESERVED.load(&deps.storage).unwrap().is_zero());
    assert!(TOTAL_DISTRIBUTED.load(&deps.storage).unwrap().is_zero());
    assert_eq!(
        REALIZED_PNL_NET.load(&deps.storage).unwrap(),
        Int128::zero()
    );
    assert_eq!(LAST_DISTRIBUTION.load(&deps.storage).unwrap(), 0);

    assert!(USER_STAKE.is_empty(&deps.storage));
    assert!(USER_INDEX.is_empty(&deps.storage));
    assert!(USER_PENDING.is_empty(&deps.storage));
    assert!(USER_CLAIMED.is_empty(&deps.storage));
}

fn setup() -> (
    cosmwasm_std::OwnedDeps<
        cosmwasm_std::testing::MockStorage,
        cosmwasm_std::testing::MockApi,
        cosmwasm_std::testing::MockQuerier,
        cosmwasm_std::Empty,
    >,
    cosmwasm_std::Addr,
) {
    let mut deps = inj_deps();
    let owner = addr20("owner");
    instantiate(
        deps.as_mut(),
        mock_env(),
        message_info(&owner, &[]),
        InstantiateMsg {
            owner: owner.to_string(),
            seed_denom: SEED.to_string(),
            treasury_denom: USDT.to_string(),
            trader: None,
            allowed_markets: vec![MARKET_1.to_string()],
            max_leverage: Some(Decimal::from_ratio(3u128, 1u128)),
            max_position_size: Some(Uint128::new(10_000_000)),
            max_exposure_bps: Some(5_000),
            max_total_margin_bps: None,
            max_position_margin: Some(Uint128::MAX),
            cooldown_seconds: Some(0),
        },
    )
    .unwrap();
    (deps, owner)
}

fn stake(
    deps: &mut cosmwasm_std::DepsMut<cosmwasm_std::Empty>,
    user: &cosmwasm_std::Addr,
    amount: u128,
) {
    execute(
        deps.branch(),
        mock_env(),
        message_info(user, &coins(amount, SEED)),
        ExecuteMsg::Stake {},
    )
    .unwrap();
}

fn record_pnl(
    deps: &mut cosmwasm_std::DepsMut<cosmwasm_std::Empty>,
    admin: &cosmwasm_std::Addr,
    delta: i128,
) {
    execute(
        deps.branch(),
        mock_env(),
        message_info(admin, &[]),
        ExecuteMsg::RecordPnl {
            delta: Int128::new(delta),
        },
    )
    .unwrap();
}

fn deposit(
    deps: &mut cosmwasm_std::DepsMut<cosmwasm_std::Empty>,
    admin: &cosmwasm_std::Addr,
    amount: u128,
) {
    execute(
        deps.branch(),
        mock_env(),
        message_info(admin, &coins(amount, USDT)),
        ExecuteMsg::DepositTreasury {},
    )
    .unwrap();
}

#[test]
fn stake_then_unstake_full_amount() {
    let (mut deps, _) = setup();
    let user = addr20("user");
    stake(&mut deps.as_mut(), &user, 100);

    assert_eq!(
        USER_STAKE.load(&deps.storage, &user).unwrap(),
        Uint128::new(100)
    );
    assert_eq!(TOTAL_STAKED.load(&deps.storage).unwrap(), Uint128::new(100));
    assert_eq!(
        USER_INDEX.load(&deps.storage, &user).unwrap(),
        GLOBAL_INDEX.load(&deps.storage).unwrap()
    );

    let res = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&user, &[]),
        ExecuteMsg::Unstake {
            amount: Uint128::new(100),
        },
    )
    .unwrap();

    assert_eq!(
        USER_STAKE.load(&deps.storage, &user).unwrap(),
        Uint128::zero()
    );
    assert_eq!(TOTAL_STAKED.load(&deps.storage).unwrap(), Uint128::zero());
    assert_eq!(res.attributes[0].key, "action");
    assert_eq!(res.attributes[0].value, "seed_unstaked");
    assert_eq!(
        res.messages[0].msg,
        CosmosMsg::Bank(BankMsg::Send {
            to_address: user.to_string(),
            amount: coins(100, SEED),
        })
    );
}

#[test]
fn unstake_more_than_staked_fails() {
    let (mut deps, _) = setup();
    let user = addr20("user");
    stake(&mut deps.as_mut(), &user, 40);

    let err = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&user, &[]),
        ExecuteMsg::Unstake {
            amount: Uint128::new(41),
        },
    )
    .unwrap_err();

    assert!(matches!(err, ContractError::InsufficientStake { .. }));
    assert_eq!(
        USER_STAKE.load(&deps.storage, &user).unwrap(),
        Uint128::new(40)
    );
    assert_eq!(TOTAL_STAKED.load(&deps.storage).unwrap(), Uint128::new(40));
}

#[test]
fn stake_wrong_denom_fails() {
    let (mut deps, _) = setup();
    let user = addr20("user");

    let err = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&user, &coins(10, USDT)),
        ExecuteMsg::Stake {},
    )
    .unwrap_err();

    assert!(matches!(err, ContractError::WrongDenom { .. }));
    assert!(USER_STAKE.may_load(&deps.storage, &user).unwrap().is_none());
    assert!(TOTAL_STAKED.load(&deps.storage).unwrap().is_zero());

    let zero = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&user, &[coin(0, SEED)]),
        ExecuteMsg::Stake {},
    )
    .unwrap_err();
    assert!(matches!(zero, ContractError::ZeroAmount));
}

#[test]
fn staking_twice_adds_up() {
    let (mut deps, _) = setup();
    let user = addr20("user");
    stake(&mut deps.as_mut(), &user, 25);
    stake(&mut deps.as_mut(), &user, 75);

    assert_eq!(
        USER_STAKE.load(&deps.storage, &user).unwrap(),
        Uint128::new(100)
    );
    assert_eq!(TOTAL_STAKED.load(&deps.storage).unwrap(), Uint128::new(100));
}

#[test]
fn non_owner_cannot_deposit_treasury() {
    let (mut deps, _) = setup();
    let user = addr20("user");

    let err = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&user, &coins(1_000, USDT)),
        ExecuteMsg::DepositTreasury {},
    )
    .unwrap_err();

    assert!(matches!(err, ContractError::Unauthorized));
    assert!(TREASURY_BALANCE.load(&deps.storage).unwrap().is_zero());
}

#[test]
fn deposit_treasury_wrong_denom_fails() {
    let (mut deps, admin) = setup();

    let err = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&admin, &coins(1_000, SEED)),
        ExecuteMsg::DepositTreasury {},
    )
    .unwrap_err();

    assert!(matches!(err, ContractError::WrongDenom { .. }));
    assert!(TREASURY_BALANCE.load(&deps.storage).unwrap().is_zero());
}

#[test]
fn deposit_treasury_zero_fails() {
    let (mut deps, admin) = setup();

    let err = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&admin, &[coin(0, USDT)]),
        ExecuteMsg::DepositTreasury {},
    )
    .unwrap_err();

    assert!(matches!(err, ContractError::ZeroAmount));
    assert!(TREASURY_BALANCE.load(&deps.storage).unwrap().is_zero());
}

#[test]
fn deposit_treasury_increases_balance() {
    let (mut deps, admin) = setup();

    let res = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&admin, &coins(1_000_000, USDT)),
        ExecuteMsg::DepositTreasury {},
    )
    .unwrap();

    assert_eq!(
        TREASURY_BALANCE.load(&deps.storage).unwrap(),
        Uint128::new(1_000_000)
    );
    assert!(res.messages.is_empty());
    assert_eq!(res.attributes[0].value, "treasury_deposited");

    execute(
        deps.as_mut(),
        mock_env(),
        message_info(&admin, &coins(250_000, USDT)),
        ExecuteMsg::DepositTreasury {},
    )
    .unwrap();

    assert_eq!(
        TREASURY_BALANCE.load(&deps.storage).unwrap(),
        Uint128::new(1_250_000)
    );
}

#[test]
fn distribute_with_no_stakers_fails() {
    let (mut deps, admin) = setup();
    deposit(&mut deps.as_mut(), &admin, 1_000);
    record_pnl(&mut deps.as_mut(), &admin, 1_000);

    let err = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&admin, &[]),
        ExecuteMsg::Distribute {
            amount: Uint128::new(1),
        },
    )
    .unwrap_err();

    assert!(matches!(err, ContractError::NoStakers));
    assert!(GLOBAL_INDEX.load(&deps.storage).unwrap().is_zero());
    assert!(TOTAL_DISTRIBUTED.load(&deps.storage).unwrap().is_zero());
}

#[test]
fn distribute_above_distributable_fails() {
    let (mut deps, admin) = setup();
    stake(&mut deps.as_mut(), &addr20("user"), 100);
    deposit(&mut deps.as_mut(), &admin, 500);
    record_pnl(&mut deps.as_mut(), &admin, 100);

    let err = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&admin, &[]),
        ExecuteMsg::Distribute {
            amount: Uint128::new(101),
        },
    )
    .unwrap_err();

    assert!(matches!(err, ContractError::InvalidAmount));
    assert_eq!(
        TREASURY_BALANCE.load(&deps.storage).unwrap(),
        Uint128::new(500)
    );
    assert!(REWARDS_RESERVED.load(&deps.storage).unwrap().is_zero());
}

#[test]
fn loss_then_profit_distributes_only_the_net() {
    let (mut deps, admin) = setup();
    stake(&mut deps.as_mut(), &addr20("user"), 1_000);
    deposit(&mut deps.as_mut(), &admin, 10_000);

    record_pnl(&mut deps.as_mut(), &admin, -400);
    record_pnl(&mut deps.as_mut(), &admin, 700);
    assert_eq!(
        REALIZED_PNL_NET.load(&deps.storage).unwrap(),
        Int128::new(300)
    );

    let blocked = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&admin, &[]),
        ExecuteMsg::Distribute {
            amount: Uint128::new(301),
        },
    )
    .unwrap_err();
    assert!(matches!(blocked, ContractError::InvalidAmount));

    let res = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&admin, &[]),
        ExecuteMsg::Distribute {
            amount: Uint128::new(300),
        },
    )
    .unwrap();

    assert_eq!(res.attributes[0].value, "yield_distributed");
    assert_eq!(
        GLOBAL_INDEX.load(&deps.storage).unwrap(),
        cosmwasm_std::Decimal256::from_ratio(300u128, 1_000u128)
    );
    assert_eq!(
        TOTAL_DISTRIBUTED.load(&deps.storage).unwrap(),
        Uint128::new(300)
    );
    assert_eq!(
        REWARDS_RESERVED.load(&deps.storage).unwrap(),
        Uint128::new(300)
    );
    assert_eq!(
        TREASURY_BALANCE.load(&deps.storage).unwrap(),
        Uint128::new(9_700)
    );
    assert_eq!(
        LAST_DISTRIBUTION.load(&deps.storage).unwrap(),
        mock_env().block.height
    );

    record_pnl(&mut deps.as_mut(), &admin, -50);
    let nothing_new = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&admin, &[]),
        ExecuteMsg::Distribute {
            amount: Uint128::new(1),
        },
    )
    .unwrap_err();
    assert!(matches!(nothing_new, ContractError::InvalidAmount));
    assert_eq!(
        REWARDS_RESERVED.load(&deps.storage).unwrap(),
        Uint128::new(300)
    );
}

#[test]
fn pause_blocks_stake_and_distribute_but_allows_deposit_and_unstake() {
    let (mut deps, admin) = setup();
    let user = addr20("user");
    stake(&mut deps.as_mut(), &user, 50);
    deposit(&mut deps.as_mut(), &admin, 100);
    record_pnl(&mut deps.as_mut(), &admin, 10);

    execute(
        deps.as_mut(),
        mock_env(),
        message_info(&admin, &[]),
        ExecuteMsg::Pause {},
    )
    .unwrap();
    assert!(CONFIG.load(&deps.storage).unwrap().paused);

    let stake_err = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&user, &coins(1, SEED)),
        ExecuteMsg::Stake {},
    )
    .unwrap_err();
    assert!(matches!(stake_err, ContractError::Paused));

    // DepositTreasury is allowed while paused so owner can recapitalize
    let deposit_res = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&admin, &coins(1, USDT)),
        ExecuteMsg::DepositTreasury {},
    )
    .unwrap();
    assert_eq!(deposit_res.attributes[0].key, "action");
    assert_eq!(deposit_res.attributes[0].value, "treasury_deposited");

    let distribute_err = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&admin, &[]),
        ExecuteMsg::Distribute {
            amount: Uint128::new(1),
        },
    )
    .unwrap_err();
    assert!(matches!(distribute_err, ContractError::Paused));

    execute(
        deps.as_mut(),
        mock_env(),
        message_info(&user, &[]),
        ExecuteMsg::Unstake {
            amount: Uint128::new(50),
        },
    )
    .unwrap();
    assert!(USER_STAKE.load(&deps.storage, &user).unwrap().is_zero());

    execute(
        deps.as_mut(),
        mock_env(),
        message_info(&admin, &[]),
        ExecuteMsg::Unpause {},
    )
    .unwrap();
    assert!(!CONFIG.load(&deps.storage).unwrap().paused);
}

#[test]
fn owner_transfer_requires_acceptance() {
    let (mut deps, owner) = setup();
    let next = addr20("next");
    let stranger = addr20("stranger");

    let self_prop = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&owner, &[]),
        ExecuteMsg::ProposeOwner {
            new_owner: owner.to_string(),
        },
    )
    .unwrap_err();
    assert!(matches!(self_prop, ContractError::Unauthorized));

    execute(
        deps.as_mut(),
        mock_env(),
        message_info(&owner, &[]),
        ExecuteMsg::ProposeOwner {
            new_owner: next.to_string(),
        },
    )
    .unwrap();
    assert_eq!(PENDING_OWNER.load(&deps.storage).unwrap(), next);
    assert_eq!(CONFIG.load(&deps.storage).unwrap().owner, owner);

    let early = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&stranger, &[]),
        ExecuteMsg::AcceptOwner {},
    )
    .unwrap_err();
    assert!(matches!(early, ContractError::Unauthorized));

    execute(
        deps.as_mut(),
        mock_env(),
        message_info(&next, &[]),
        ExecuteMsg::AcceptOwner {},
    )
    .unwrap();

    assert_eq!(CONFIG.load(&deps.storage).unwrap().owner, next);
    assert!(PENDING_OWNER.may_load(&deps.storage).unwrap().is_none());

    let old = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&owner, &[]),
        ExecuteMsg::Pause {},
    )
    .unwrap_err();
    assert!(matches!(old, ContractError::Unauthorized));
}

fn distribute(
    deps: &mut cosmwasm_std::DepsMut<cosmwasm_std::Empty>,
    admin: &cosmwasm_std::Addr,
    amount: u128,
) {
    execute(
        deps.branch(),
        mock_env(),
        message_info(admin, &[]),
        ExecuteMsg::Distribute {
            amount: Uint128::new(amount),
        },
    )
    .unwrap();
}

fn claim(
    deps: &mut cosmwasm_std::DepsMut<cosmwasm_std::Empty>,
    user: &cosmwasm_std::Addr,
) -> cosmwasm_std::Response<cosmwasm_std::Empty> {
    execute(
        deps.branch(),
        mock_env(),
        message_info(user, &[]),
        ExecuteMsg::Claim {},
    )
    .unwrap()
}

#[test]
fn open_position_enforces_limits_and_reserves_margin() {
    let (mut deps, admin) = setup();
    let stranger = addr20("stranger");

    // Deposit 10,000,000 USDT to free treasury
    execute(
        deps.as_mut(),
        mock_env(),
        message_info(&admin, &coins(10_000_000, USDT)),
        ExecuteMsg::DepositTreasury {},
    )
    .unwrap();

    // 1. Non-admin fails
    let err = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&stranger, &[]),
        ExecuteMsg::OpenPosition {
            market_id: MARKET_1.to_string(),
            is_buy: true,
            quantity: "1000".to_string(),
            price: "1000".to_string(),
            margin: "1000000".to_string(),
            leverage: None,
        },
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::Unauthorized));

    // 2. Unallowed market fails
    let err = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&admin, &[]),
        ExecuteMsg::OpenPosition {
            market_id: "0xunknown_market".to_string(),
            is_buy: true,
            quantity: "1000".to_string(),
            price: "1000".to_string(),
            margin: "1000000".to_string(),
            leverage: None,
        },
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::MarketNotAllowed { .. }));

    // 3. Exceeding max_position_size (max is 10_000_000)
    let err = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&admin, &[]),
        ExecuteMsg::OpenPosition {
            market_id: MARKET_1.to_string(),
            is_buy: true,
            quantity: "10000001".to_string(),
            price: "1".to_string(),
            margin: "1000000".to_string(),
            leverage: None,
        },
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::PositionTooLarge));

    // 4. Exceeding max_leverage explicitly (max is 3)
    let err = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&admin, &[]),
        ExecuteMsg::OpenPosition {
            market_id: MARKET_1.to_string(),
            is_buy: true,
            quantity: "1000".to_string(),
            price: "1000".to_string(),
            margin: "1000000".to_string(),
            leverage: Some("4".to_string()),
        },
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::LeverageTooHigh));

    // 5. Notional exceeding margin * max_leverage
    let err = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&admin, &[]),
        ExecuteMsg::OpenPosition {
            market_id: MARKET_1.to_string(),
            is_buy: true,
            quantity: "4000".to_string(),
            price: "1000".to_string(),
            margin: "1000000".to_string(),
            leverage: None,
        },
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::LeverageTooHigh));

    // 6. Insufficient treasury (need 15_000_000, have 10_000_000)
    let err = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&admin, &[]),
        ExecuteMsg::OpenPosition {
            market_id: MARKET_1.to_string(),
            is_buy: true,
            quantity: "1000".to_string(),
            price: "1".to_string(),
            margin: "15000000".to_string(),
            leverage: None,
        },
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::InsufficientTreasury { .. }));

    // 7. Exceeding max_exposure_bps (50% of 10,000,000 = 5,000,000 max reserved)
    let err = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&admin, &[]),
        ExecuteMsg::OpenPosition {
            market_id: MARKET_1.to_string(),
            is_buy: true,
            quantity: "1000".to_string(),
            price: "1".to_string(),
            margin: "6000000".to_string(),
            leverage: None,
        },
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::ExposureLimitExceeded));

    // 8. Valid open position reserves margin from treasury
    let res = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&admin, &[]),
        ExecuteMsg::OpenPosition {
            market_id: MARKET_1.to_string(),
            is_buy: true,
            quantity: "2000".to_string(),
            price: "2000".to_string(),
            margin: "2000000".to_string(),
            leverage: Some("2".to_string()),
        },
    )
    .unwrap();

    assert_eq!(res.messages.len(), 2);
    #[allow(deprecated)]
    match &res.messages[0].msg {
        CosmosMsg::Stargate { type_url, .. } => {
            assert_eq!(type_url, "/injective.exchange.v1beta1.MsgDeposit");
        }
        _ => panic!("expected MsgDeposit stargate message"),
    }
    assert_eq!(res.messages[1].id, crate::contract::REPLY_OPEN_POSITION);
    #[allow(deprecated)]
    match &res.messages[1].msg {
        CosmosMsg::Stargate { type_url, .. } => {
            assert_eq!(type_url, "/injective.exchange.v1beta1.MsgCreateDerivativeMarketOrder");
        }
        _ => panic!("expected MsgCreateDerivativeMarketOrder stargate message"),
    }
    assert_eq!(res.attributes[0].value, "open_position");
    assert_eq!(
        TREASURY_BALANCE.load(&deps.storage).unwrap(),
        Uint128::new(8_000_000)
    );
    assert_eq!(
        RESERVED_MARGIN.load(&deps.storage).unwrap(),
        Uint128::new(2_000_000)
    );
}

#[test]
fn trader_can_open_and_close_positions() {
    let mut deps = inj_deps();
    let owner = addr20("owner");
    let trader = addr20("trader");
    let stranger = addr20("stranger");

    instantiate(
        deps.as_mut(),
        mock_env(),
        message_info(&owner, &[]),
        InstantiateMsg {
            owner: owner.to_string(),
            seed_denom: SEED.to_string(),
            treasury_denom: USDT.to_string(),
            trader: Some(trader.to_string()),
            allowed_markets: vec![MARKET_1.to_string()],
            max_leverage: Some(Decimal::from_ratio(3u128, 1u128)),
            max_position_size: Some(Uint128::new(10_000_000)),
            max_exposure_bps: Some(8_000),
            max_total_margin_bps: None,
            max_position_margin: Some(Uint128::new(5_000_000)),
            cooldown_seconds: Some(0),
        },
    )
    .unwrap();

    // Deposit treasury
    execute(
        deps.as_mut(),
        mock_env(),
        message_info(&owner, &coins(5_000_000, USDT)),
        ExecuteMsg::DepositTreasury {},
    )
    .unwrap();

    // Trader opens position successfully
    execute(
        deps.as_mut(),
        mock_env(),
        message_info(&trader, &[]),
        ExecuteMsg::OpenPosition {
            market_id: MARKET_1.to_string(),
            is_buy: false,
            quantity: "1000".to_string(),
            price: "1000".to_string(),
            margin: "1000000".to_string(),
            leverage: None,
        },
    )
    .unwrap();

    assert_eq!(
        RESERVED_MARGIN.load(&deps.storage).unwrap(),
        Uint128::new(1_000_000)
    );

    // Stranger cannot close
    let err = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&stranger, &[]),
        ExecuteMsg::ClosePosition {
            market_id: MARKET_1.to_string(),
            quantity: None,
        },
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::Unauthorized));

    // Trader closes the full position successfully
    let res = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&trader, &[]),
        ExecuteMsg::ClosePosition {
            market_id: MARKET_1.to_string(),
            quantity: Some("1000".to_string()),
        },
    )
    .unwrap();
    assert_eq!(res.attributes[0].value, "close_position");
    assert_eq!(RESERVED_MARGIN.load(&deps.storage).unwrap(), Uint128::zero());
    assert_eq!(
        TREASURY_BALANCE.load(&deps.storage).unwrap(),
        Uint128::new(5_000_000)
    );
}
#[test]
fn trader_cannot_distribute_or_deposit_treasury_but_owner_can() {
    let mut deps = inj_deps();
    let owner = addr20("owner");
    let trader = addr20("trader");
    let stranger = addr20("stranger");
    let user = addr20("staker");

    instantiate(
        deps.as_mut(),
        mock_env(),
        message_info(&owner, &[]),
        InstantiateMsg {
            owner: owner.to_string(),
            seed_denom: SEED.to_string(),
            treasury_denom: USDT.to_string(),
            trader: Some(trader.to_string()),
            allowed_markets: vec![MARKET_1.to_string()],
            max_leverage: Some(Decimal::from_ratio(3u128, 1u128)),
            max_position_size: Some(Uint128::new(10_000_000)),
            max_exposure_bps: Some(8_000),
            max_total_margin_bps: None,
            max_position_margin: Some(Uint128::new(5_000_000)),
            cooldown_seconds: Some(0),
        },
    )
    .unwrap();

    // Trader cannot deposit treasury
    let err = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&trader, &coins(5_000_000, USDT)),
        ExecuteMsg::DepositTreasury {},
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::Unauthorized));

    // Owner deposits treasury successfully
    execute(
        deps.as_mut(),
        mock_env(),
        message_info(&owner, &coins(5_000_000, USDT)),
        ExecuteMsg::DepositTreasury {},
    )
    .unwrap();

    // Staker stakes
    execute(
        deps.as_mut(),
        mock_env(),
        message_info(&user, &coins(1_000_000, SEED)),
        ExecuteMsg::Stake {},
    )
    .unwrap();

    // Record realized PnL
    execute(
        deps.as_mut(),
        mock_env(),
        message_info(&owner, &[]),
        ExecuteMsg::RecordPnl {
            delta: Int128::new(2_000_000),
        },
    )
    .unwrap();

    // Stranger cannot distribute
    let err = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&stranger, &[]),
        ExecuteMsg::Distribute {
            amount: Uint128::new(1_000_000),
        },
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::Unauthorized));

    // Trader CANNOT distribute (restricted exclusively to trading orders)
    let err = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&trader, &[]),
        ExecuteMsg::Distribute {
            amount: Uint128::new(1_000_000),
        },
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::Unauthorized));

    // Owner distributes successfully
    let res = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&owner, &[]),
        ExecuteMsg::Distribute {
            amount: Uint128::new(1_000_000),
        },
    )
    .unwrap();
    assert_eq!(res.attributes[0].value, "yield_distributed");
    assert_eq!(
        TOTAL_DISTRIBUTED.load(&deps.storage).unwrap(),
        Uint128::new(1_000_000)
    );
}

#[test]
fn pause_blocks_open_position_but_allows_close() {
    let (mut deps, admin) = setup();

    execute(
        deps.as_mut(),
        mock_env(),
        message_info(&admin, &coins(5_000_000, USDT)),
        ExecuteMsg::DepositTreasury {},
    )
    .unwrap();

    // Pause the contract
    execute(
        deps.as_mut(),
        mock_env(),
        message_info(&admin, &[]),
        ExecuteMsg::Pause {},
    )
    .unwrap();

    // OpenPosition fails when paused
    let err = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&admin, &[]),
        ExecuteMsg::OpenPosition {
            market_id: MARKET_1.to_string(),
            is_buy: true,
            quantity: "1000".to_string(),
            price: "1000".to_string(),
            margin: "1000000".to_string(),
            leverage: None,
        },
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::Paused));

    execute(
        deps.as_mut(),
        mock_env(),
        message_info(&admin, &[]),
        ExecuteMsg::Unpause {},
    )
    .unwrap();
    execute(
        deps.as_mut(),
        mock_env(),
        message_info(&admin, &[]),
        ExecuteMsg::OpenPosition {
            market_id: MARKET_1.to_string(),
            is_buy: true,
            quantity: "1000".to_string(),
            price: "1000".to_string(),
            margin: "1000000".to_string(),
            leverage: None,
        },
    )
    .unwrap();
    execute(
        deps.as_mut(),
        mock_env(),
        message_info(&admin, &[]),
        ExecuteMsg::Pause {},
    )
    .unwrap();

    // ClosePosition succeeds even when paused
    let res = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&admin, &[]),
        ExecuteMsg::ClosePosition {
            market_id: MARKET_1.to_string(),
            quantity: None,
        },
    )
    .unwrap();
    assert_eq!(res.attributes[0].value, "close_position");
}

fn sent_usdt(
    res: &cosmwasm_std::Response<cosmwasm_std::Empty>,
    user: &cosmwasm_std::Addr,
    amount: u128,
) {
    assert_eq!(res.attributes[0].value, "yield_claimed");
    assert_eq!(
        res.messages[0].msg,
        CosmosMsg::Bank(BankMsg::Send {
            to_address: user.to_string(),
            amount: coins(amount, USDT),
        })
    );
}

#[test]
fn claim_pays_the_full_distribution() {
    let (mut deps, admin) = setup();
    let user = addr20("user");
    stake(&mut deps.as_mut(), &user, 1_000);
    deposit(&mut deps.as_mut(), &admin, 500);
    record_pnl(&mut deps.as_mut(), &admin, 500);
    distribute(&mut deps.as_mut(), &admin, 500);

    let res = claim(&mut deps.as_mut(), &user);
    sent_usdt(&res, &user, 500);
    assert!(USER_PENDING.load(&deps.storage, &user).unwrap().is_zero());
    assert_eq!(
        USER_CLAIMED.load(&deps.storage, &user).unwrap(),
        Uint128::new(500)
    );
    assert!(REWARDS_RESERVED.load(&deps.storage).unwrap().is_zero());

    let again = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&user, &[]),
        ExecuteMsg::Claim {},
    )
    .unwrap_err();
    assert!(matches!(again, ContractError::NothingToClaim));
}

#[test]
fn later_staker_misses_the_first_distribution() {
    let (mut deps, admin) = setup();
    let a = addr20("alice");
    let b = addr20("bob");
    stake(&mut deps.as_mut(), &a, 100);
    deposit(&mut deps.as_mut(), &admin, 300);
    record_pnl(&mut deps.as_mut(), &admin, 300);

    distribute(&mut deps.as_mut(), &admin, 100);
    stake(&mut deps.as_mut(), &b, 100);
    distribute(&mut deps.as_mut(), &admin, 200);

    let a_claim = claim(&mut deps.as_mut(), &a);
    let b_claim = claim(&mut deps.as_mut(), &b);
    sent_usdt(&a_claim, &a, 200);
    sent_usdt(&b_claim, &b, 100);
}

#[test]
fn staking_after_a_distribution_earns_nothing() {
    let (mut deps, admin) = setup();
    let a = addr20("alice");
    let b = addr20("bob");
    stake(&mut deps.as_mut(), &a, 100);
    deposit(&mut deps.as_mut(), &admin, 80);
    record_pnl(&mut deps.as_mut(), &admin, 80);
    distribute(&mut deps.as_mut(), &admin, 80);

    stake(&mut deps.as_mut(), &b, 900);
    let err = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&b, &[]),
        ExecuteMsg::Claim {},
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::NothingToClaim));

    let a_claim = claim(&mut deps.as_mut(), &a);
    sent_usdt(&a_claim, &a, 80);
}

#[test]
fn unstake_keeps_pending_yield_for_a_later_claim() {
    let (mut deps, admin) = setup();
    let user = addr20("user");
    stake(&mut deps.as_mut(), &user, 40);
    deposit(&mut deps.as_mut(), &admin, 40);
    record_pnl(&mut deps.as_mut(), &admin, 40);
    distribute(&mut deps.as_mut(), &admin, 40);

    execute(
        deps.as_mut(),
        mock_env(),
        message_info(&user, &[]),
        ExecuteMsg::Unstake {
            amount: Uint128::new(40),
        },
    )
    .unwrap();
    assert_eq!(
        USER_PENDING.load(&deps.storage, &user).unwrap(),
        Uint128::new(40)
    );

    execute(
        deps.as_mut(),
        mock_env(),
        message_info(&admin, &[]),
        ExecuteMsg::Pause {},
    )
    .unwrap();
    let res = claim(&mut deps.as_mut(), &user);
    sent_usdt(&res, &user, 40);
}

#[test]
fn distribute_with_zero_stake_is_rejected() {
    let (mut deps, admin) = setup();
    deposit(&mut deps.as_mut(), &admin, 10);
    record_pnl(&mut deps.as_mut(), &admin, 10);

    let err = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&admin, &[]),
        ExecuteMsg::Distribute {
            amount: Uint128::new(10),
        },
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::NoStakers));
}

#[test]
fn claims_never_exceed_total_distributed() {
    let (mut deps, admin) = setup();
    let users = [addr20("a"), addr20("b"), addr20("c")];
    deposit(&mut deps.as_mut(), &admin, 10_000);
    record_pnl(&mut deps.as_mut(), &admin, 1_000);

    let steps: &[(usize, &str, u128)] = &[
        (0, "stake", 100),
        (0, "distribute", 30),
        (1, "stake", 50),
        (0, "claim", 0),
        (0, "distribute", 90),
        (2, "stake", 250),
        (1, "unstake", 20),
        (0, "distribute", 120),
        (1, "claim", 0),
        (0, "unstake", 100),
        (2, "claim", 0),
        (0, "claim", 0),
        (1, "stake", 10),
        (0, "distribute", 40),
        (2, "unstake", 250),
        (1, "claim", 0),
    ];

    let mut claimed = Uint128::zero();
    for (who, action, amount) in steps {
        let user = &users[*who];
        match *action {
            "stake" => stake(&mut deps.as_mut(), user, *amount),
            "unstake" => {
                execute(
                    deps.as_mut(),
                    mock_env(),
                    message_info(user, &[]),
                    ExecuteMsg::Unstake {
                        amount: Uint128::new(*amount),
                    },
                )
                .unwrap();
            }
            "distribute" => distribute(&mut deps.as_mut(), &admin, *amount),
            "claim" => {
                let res = claim(&mut deps.as_mut(), user);
                let paid = USER_CLAIMED.load(&deps.storage, user).unwrap();
                claimed = paid;
                assert_eq!(res.attributes[0].value, "yield_claimed");
            }
            _ => unreachable!(),
        }

        let distributed = TOTAL_DISTRIBUTED.load(&deps.storage).unwrap();
        let reserved = REWARDS_RESERVED.load(&deps.storage).unwrap();
        let mut owed = Uint128::zero();
        for user in &users {
            owed += pending_yield(&deps.storage, user).unwrap();
            owed += USER_CLAIMED
                .may_load(&deps.storage, user)
                .unwrap()
                .unwrap_or_default();
        }
        assert!(owed <= distributed);
        assert!(reserved <= distributed);
        assert_eq!(distributed, reserved + claimed_total(&deps.storage, &users));
    }
    assert!(!claimed.is_zero());
}

fn claimed_total(storage: &dyn cosmwasm_std::Storage, users: &[cosmwasm_std::Addr]) -> Uint128 {
    users.iter().fold(Uint128::zero(), |sum, user| {
        sum + USER_CLAIMED
            .may_load(storage, user)
            .unwrap()
            .unwrap_or_default()
    })
}

fn pending(
    deps: &cosmwasm_std::Deps<cosmwasm_std::Empty>,
    user: &cosmwasm_std::Addr,
) -> Uint128 {
    let bin = query(
        *deps,
        mock_env(),
        QueryMsg::PendingYield {
            user: user.to_string(),
        },
    )
    .unwrap();
    from_json::<AmountResponse>(bin).unwrap().amount
}

#[test]
fn pending_yield_matches_the_claim_payout() {
    let (mut deps, admin) = setup();
    let user = addr20("user");
    stake(&mut deps.as_mut(), &user, 200);

    assert!(pending(&deps.as_ref(), &user).is_zero());
    let before = USER_INDEX.may_load(&deps.storage, &user).unwrap();

    deposit(&mut deps.as_mut(), &admin, 50);
    record_pnl(&mut deps.as_mut(), &admin, 50);
    distribute(&mut deps.as_mut(), &admin, 50);

    let shown = pending(&deps.as_ref(), &user);
    assert_eq!(shown, Uint128::new(50));
    assert_eq!(USER_INDEX.may_load(&deps.storage, &user).unwrap(), before);
    assert!(USER_PENDING
        .may_load(&deps.storage, &user)
        .unwrap()
        .unwrap_or_default()
        .is_zero());

    let info = from_json::<UserInfoResponse>(
        query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::UserInfo {
                user: user.to_string(),
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(info.stake, Uint128::new(200));
    assert_eq!(info.pending, shown);
    assert!(info.claimed.is_zero());
    assert_eq!(info.share, "1");

    let dist = from_json::<DistributionInfoResponse>(
        query(deps.as_ref(), mock_env(), QueryMsg::DistributionInfo {}).unwrap(),
    )
    .unwrap();
    assert_eq!(dist.total_distributed, Uint128::new(50));
    assert_eq!(dist.rewards_reserved, Uint128::new(50));
    assert_eq!(dist.last_distribution, mock_env().block.height);

    let res = claim(&mut deps.as_mut(), &user);
    sent_usdt(&res, &user, shown.u128());
    assert!(pending(&deps.as_ref(), &user).is_zero());

    let positions = from_json::<PositionsResponse>(
        query(deps.as_ref(), mock_env(), QueryMsg::Positions {}).unwrap(),
    )
    .unwrap();
    assert!(positions.positions.is_empty());

    let stake_view = from_json::<UserStakeResponse>(
        query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::UserStake {
                user: user.to_string(),
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(stake_view.amount, Uint128::new(200));
    let total = from_json::<AmountResponse>(
        query(deps.as_ref(), mock_env(), QueryMsg::TotalStaked {}).unwrap(),
    )
    .unwrap();
    assert_eq!(total.amount, Uint128::new(200));
}

#[test]
fn distribute_too_small_for_index_is_rejected() {
    let mut deps = inj_deps();
    let admin = addr20("admin");
    let user = addr20("whale");

    instantiate(
        deps.as_mut(),
        mock_env(),
        message_info(&admin, &[]),
        InstantiateMsg {
            owner: admin.to_string(),
            seed_denom: SEED.to_string(),
            treasury_denom: USDT.to_string(),
            trader: None,
            allowed_markets: vec![MARKET_1.to_string()],
            max_leverage: Some(Decimal::from_ratio(3u128, 1u128)),
            max_position_size: Some(Uint128::MAX),
            max_exposure_bps: Some(10_000),
            max_total_margin_bps: None,
            max_position_margin: Some(Uint128::MAX),
            cooldown_seconds: Some(0),
        },
    )
    .unwrap();

    // Deposit large treasury
    execute(
        deps.as_mut(),
        mock_env(),
        message_info(&admin, &coins(100_000_000_000_000, USDT)),
        ExecuteMsg::DepositTreasury {},
    )
    .unwrap();

    // Record PnL
    execute(
        deps.as_mut(),
        mock_env(),
        message_info(&admin, &[]),
        ExecuteMsg::RecordPnl {
            delta: Int128::new(100_000_000_000_000),
        },
    )
    .unwrap();

    // Whale stakes 10^20 tokens (e.g. 100 with 18 decimals)
    execute(
        deps.as_mut(),
        mock_env(),
        message_info(&user, &coins(100_000_000_000_000_000_000u128, SEED)),
        ExecuteMsg::Stake {},
    )
    .unwrap();

    // Distributing 1 micro-USDT: 1 * 10^18 / 10^20 = 0 in Decimal256!
    // Must be rejected to avoid trapping funds in REWARDS_RESERVED.
    let err = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&admin, &[]),
        ExecuteMsg::Distribute {
            amount: Uint128::new(1),
        },
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::InvalidAmount));
}

#[test]
fn open_position_high_notional_no_overflow() {
    let mut deps = inj_deps();
    let admin = addr20("admin");

    instantiate(
        deps.as_mut(),
        mock_env(),
        message_info(&admin, &[]),
        InstantiateMsg {
            owner: admin.to_string(),
            seed_denom: SEED.to_string(),
            treasury_denom: USDT.to_string(),
            trader: None,
            allowed_markets: vec![MARKET_1.to_string()],
            max_leverage: Some(Decimal::from_ratio(5u128, 1u128)),
            max_position_size: Some(Uint128::MAX),
            max_exposure_bps: Some(10_000),
            max_total_margin_bps: None,
            max_position_margin: Some(Uint128::MAX),
            cooldown_seconds: Some(0),
        },
    )
    .unwrap();

    let initial_treasury = 300_000_000_000_000_000_000_000_000_000_000_000_000u128;
    execute(
        deps.as_mut(),
        mock_env(),
        message_info(&admin, &coins(initial_treasury, USDT)),
        ExecuteMsg::DepositTreasury {},
    )
    .unwrap();

    // qty = 2 * 10^19 (fits in u128)
    // price = 2.5 * 10^19 (fits in u128)
    // qty * price = 5 * 10^38 > u128::MAX (~3.4 * 10^38)
    // margin = 2 * 10^38 (fits in u128)
    // leverage = 5 * 10^38 / 2 * 10^38 = 2.5x <= max_leverage 5x
    let qty_str = "20000000000000000000";
    let price_str = "25000000000000000000";
    let margin_str = "200000000000000000000000000000000000000";

    let res = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&admin, &[]),
        ExecuteMsg::OpenPosition {
            market_id: MARKET_1.to_string(),
            is_buy: true,
            quantity: qty_str.to_string(),
            price: price_str.to_string(),
            margin: margin_str.to_string(),
            leverage: None,
        },
    );
    assert!(res.is_ok());
}


#[test]
fn update_trader_by_owner_and_unauthorized_checks() {
    let (mut deps, owner) = setup();
    let trader1 = addr20("trader1");
    let trader2 = addr20("trader2");
    let stranger = addr20("stranger");

    // Deposit treasury by owner
    execute(
        deps.as_mut(),
        mock_env(),
        message_info(&owner, &coins(5_000_000, USDT)),
        ExecuteMsg::DepositTreasury {},
    )
    .unwrap();

    // Stranger cannot update trader
    let err = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&stranger, &[]),
        ExecuteMsg::UpdateTrader {
            new_trader: Some(trader1.to_string()),
        },
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::Unauthorized));

    // Owner sets trader1
    let res = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&owner, &[]),
        ExecuteMsg::UpdateTrader {
            new_trader: Some(trader1.to_string()),
        },
    )
    .unwrap();
    assert_eq!(res.attributes[0].value, "trader_updated");
    assert_eq!(res.attributes[1].value, trader1.as_str());

    let conf: ConfigResponse =
        from_json(query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap()).unwrap();
    assert_eq!(conf.trader, Some(trader1.to_string()));

    // Trader1 cannot update trader
    let err = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&trader1, &[]),
        ExecuteMsg::UpdateTrader {
            new_trader: Some(trader2.to_string()),
        },
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::Unauthorized));

    // Trader1 can open position
    execute(
        deps.as_mut(),
        mock_env(),
        message_info(&trader1, &[]),
        ExecuteMsg::OpenPosition {
            market_id: MARKET_1.to_string(),
            is_buy: true,
            quantity: "1000".to_string(),
            price: "1000".to_string(),
            margin: "1000000".to_string(),
            leverage: None,
        },
    )
    .unwrap();

    // Trader2 cannot open position yet
    let err = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&trader2, &[]),
        ExecuteMsg::OpenPosition {
            market_id: MARKET_1.to_string(),
            is_buy: true,
            quantity: "1000".to_string(),
            price: "1000".to_string(),
            margin: "1000000".to_string(),
            leverage: None,
        },
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::Unauthorized));

    // Owner rotates trader to trader2
    execute(
        deps.as_mut(),
        mock_env(),
        message_info(&owner, &[]),
        ExecuteMsg::UpdateTrader {
            new_trader: Some(trader2.to_string()),
        },
    )
    .unwrap();

    // Old trader1 can no longer close or open position
    let err = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&trader1, &[]),
        ExecuteMsg::ClosePosition {
            market_id: MARKET_1.to_string(),
            quantity: None,
        },
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::Unauthorized));

    // New trader2 can close position
    let res = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&trader2, &[]),
        ExecuteMsg::ClosePosition {
            market_id: MARKET_1.to_string(),
            quantity: None,
        },
    )
    .unwrap();
    assert_eq!(res.attributes[0].value, "close_position");

    // Owner revokes trader completely (sets to None)
    execute(
        deps.as_mut(),
        mock_env(),
        message_info(&owner, &[]),
        ExecuteMsg::UpdateTrader { new_trader: None },
    )
    .unwrap();

    let conf: ConfigResponse =
        from_json(query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap()).unwrap();
    assert_eq!(conf.trader, None);

    // Trader2 can no longer open position
    let err = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&trader2, &[]),
        ExecuteMsg::OpenPosition {
            market_id: MARKET_1.to_string(),
            is_buy: true,
            quantity: "1000".to_string(),
            price: "1000".to_string(),
            margin: "1000000".to_string(),
            leverage: None,
        },
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::Unauthorized));

    // Owner can still manage positions even with trader = None
    let res = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&owner, &[]),
        ExecuteMsg::OpenPosition {
            market_id: MARKET_1.to_string(),
            is_buy: true,
            quantity: "1000".to_string(),
            price: "1000".to_string(),
            margin: "1000000".to_string(),
            leverage: None,
        },
    );
    assert!(res.is_ok());
}


#[test]
fn update_config_by_owner_and_unauthorized_checks() {
    let (mut deps, owner) = setup();
    let trader = addr20("trader");
    let stranger = addr20("stranger");

    // Stranger and trader cannot update config
    let err = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&stranger, &[]),
        ExecuteMsg::UpdateConfig {
            max_leverage: Some(Decimal::from_ratio(10u128, 1u128)),
            max_position_size: None,
            max_exposure_bps: None,
            max_total_margin_bps: None,
            max_position_margin: None,
            cooldown_seconds: None,
            allowed_markets: None,
        },
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::Unauthorized));

    let err = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&trader, &[]),
        ExecuteMsg::UpdateConfig {
            max_leverage: Some(Decimal::from_ratio(10u128, 1u128)),
            max_position_size: None,
            max_exposure_bps: None,
            max_total_margin_bps: None,
            max_position_margin: None,
            cooldown_seconds: None,
            allowed_markets: None,
        },
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::Unauthorized));

    // Invalid max_exposure_bps (> 10000) rejected
    let err = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&owner, &[]),
        ExecuteMsg::UpdateConfig {
            max_leverage: None,
            max_position_size: None,
            max_exposure_bps: Some(10_001),
            max_total_margin_bps: None,
            max_position_margin: None,
            cooldown_seconds: None,
            allowed_markets: None,
        },
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::Std(_)));

    // Invalid max_leverage (< 1) rejected
    let err = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&owner, &[]),
        ExecuteMsg::UpdateConfig {
            max_leverage: Some(Decimal::from_ratio(1u128, 2u128)),
            max_position_size: None,
            max_exposure_bps: None,
            max_total_margin_bps: None,
            max_position_margin: None,
            cooldown_seconds: None,
            allowed_markets: None,
        },
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::Std(_)));

    // Invalid max_position_margin (0) rejected
    let err = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&owner, &[]),
        ExecuteMsg::UpdateConfig {
            max_leverage: None,
            max_position_size: None,
            max_exposure_bps: None,
            max_total_margin_bps: None,
            max_position_margin: Some(Uint128::zero()),
            cooldown_seconds: None,
            allowed_markets: None,
        },
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::Std(_)));

    // Owner updates config successfully
    let res = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&owner, &[]),
        ExecuteMsg::UpdateConfig {
            max_leverage: Some(Decimal::from_ratio(10u128, 1u128)),
            max_position_size: Some(Uint128::new(50_000_000)),
            max_exposure_bps: Some(7_500),
            max_total_margin_bps: None,
            max_position_margin: Some(Uint128::new(25_000_000)),
            cooldown_seconds: Some(60),
            allowed_markets: Some(vec!["0xmarket1".to_string(), "0xmarket2".to_string()]),
        },
    )
    .unwrap();
    assert_eq!(res.attributes[0].value, "config_updated");

    let conf: ConfigResponse =
        from_json(query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap()).unwrap();
    assert_eq!(conf.max_leverage, "10");
    assert_eq!(conf.max_position_size, Uint128::new(50_000_000));
    assert_eq!(conf.max_exposure_bps, 7_500);
    assert_eq!(conf.max_position_margin, Uint128::new(25_000_000));
    assert_eq!(conf.cooldown_seconds, 60);
    assert_eq!(
        conf.allowed_markets,
        vec!["0xmarket1".to_string(), "0xmarket2".to_string()]
    );
}

#[test]
fn trader_cannot_execute_owner_administrative_functions() {
    let mut deps = inj_deps();
    let owner = addr20("owner");
    let trader = addr20("trader");

    instantiate(
        deps.as_mut(),
        mock_env(),
        message_info(&owner, &[]),
        InstantiateMsg {
            owner: owner.to_string(),
            seed_denom: SEED.to_string(),
            treasury_denom: USDT.to_string(),
            trader: Some(trader.to_string()),
            allowed_markets: vec![MARKET_1.to_string()],
            max_leverage: Some(Decimal::from_ratio(3u128, 1u128)),
            max_position_size: Some(Uint128::new(10_000_000)),
            max_exposure_bps: Some(8_000),
            max_total_margin_bps: None,
            max_position_margin: Some(Uint128::new(5_000_000)),
            cooldown_seconds: Some(0),
        },
    )
    .unwrap();

    // Trader cannot Pause
    let err = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&trader, &[]),
        ExecuteMsg::Pause {},
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::Unauthorized));

    // Trader cannot Unpause
    let err = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&trader, &[]),
        ExecuteMsg::Unpause {},
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::Unauthorized));

    // Trader cannot ProposeOwner
    let err = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&trader, &[]),
        ExecuteMsg::ProposeOwner {
            new_owner: trader.to_string(),
        },
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::Unauthorized));

    // Trader cannot RecordPnl
    let err = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&trader, &[]),
        ExecuteMsg::RecordPnl {
            delta: Int128::new(100),
        },
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::Unauthorized));
}

#[test]
fn test_market_not_allowed_rejection() {
    let (mut deps, owner) = setup();

    execute(
        deps.as_mut(),
        mock_env(),
        message_info(&owner, &coins(10_000_000, USDT)),
        ExecuteMsg::DepositTreasury {},
    )
    .unwrap();

    let err = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&owner, &[]),
        ExecuteMsg::OpenPosition {
            market_id: "0xunlisted_market".to_string(),
            is_buy: true,
            quantity: "1000".to_string(),
            price: "1000".to_string(),
            margin: "1000000".to_string(),
            leverage: None,
        },
    )
    .unwrap_err();

    assert!(matches!(
        err,
        ContractError::MarketNotAllowed { market_id } if market_id == "0xunlisted_market"
    ));
}

#[test]
fn test_leverage_too_high_rejection() {
    let (mut deps, owner) = setup();

    execute(
        deps.as_mut(),
        mock_env(),
        message_info(&owner, &coins(10_000_000, USDT)),
        ExecuteMsg::DepositTreasury {},
    )
    .unwrap();

    // 1. Explicit leverage > max_leverage (3x)
    let err = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&owner, &[]),
        ExecuteMsg::OpenPosition {
            market_id: MARKET_1.to_string(),
            is_buy: true,
            quantity: "1000".to_string(),
            price: "1000".to_string(),
            margin: "1000000".to_string(),
            leverage: Some("3.5".to_string()),
        },
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::LeverageTooHigh));

    // 2. Implicit leverage: notional (qty * price) > max_leverage * margin
    // 4000 * 1000 = 4_000_000 notional, 3x margin = 3_000_000
    let err = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&owner, &[]),
        ExecuteMsg::OpenPosition {
            market_id: MARKET_1.to_string(),
            is_buy: true,
            quantity: "4000".to_string(),
            price: "1000".to_string(),
            margin: "1000000".to_string(),
            leverage: None,
        },
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::LeverageTooHigh));
}

#[test]
fn test_exposure_limit_exceeded_rejection() {
    let (mut deps, owner) = setup();

    execute(
        deps.as_mut(),
        mock_env(),
        message_info(&owner, &coins(10_000_000, USDT)),
        ExecuteMsg::DepositTreasury {},
    )
    .unwrap();

    // max_exposure_bps is 5000 (50%). Total treasury = 10_000_000.
    // Max allowed reserved margin = 5_000_000.
    // Attempting to reserve 5_000_001 exceeds exposure limit.
    let err = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&owner, &[]),
        ExecuteMsg::OpenPosition {
            market_id: MARKET_1.to_string(),
            is_buy: true,
            quantity: "1000".to_string(),
            price: "1".to_string(),
            margin: "5000001".to_string(),
            leverage: None,
        },
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::ExposureLimitExceeded));
}

#[test]
fn test_position_too_large_rejection() {
    let (mut deps, owner) = setup();

    execute(
        deps.as_mut(),
        mock_env(),
        message_info(&owner, &coins(20_000_000, USDT)),
        ExecuteMsg::DepositTreasury {},
    )
    .unwrap();

    // Set max_position_margin to 2_000_000 and max_position_size to 5_000_000
    execute(
        deps.as_mut(),
        mock_env(),
        message_info(&owner, &[]),
        ExecuteMsg::UpdateConfig {
            max_leverage: None,
            max_position_size: Some(Uint128::new(5_000_000)),
            max_exposure_bps: Some(10_000),
            max_total_margin_bps: None,
            max_position_margin: Some(Uint128::new(2_000_000)),
            cooldown_seconds: None,
            allowed_markets: None,
        },
    )
    .unwrap();

    // 1. Margin exceeds max_position_margin
    let err = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&owner, &[]),
        ExecuteMsg::OpenPosition {
            market_id: MARKET_1.to_string(),
            is_buy: true,
            quantity: "1000".to_string(),
            price: "1".to_string(),
            margin: "2000001".to_string(),
            leverage: None,
        },
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::PositionTooLarge));

    // 2. Quantity exceeds max_position_size
    let err = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&owner, &[]),
        ExecuteMsg::OpenPosition {
            market_id: MARKET_1.to_string(),
            is_buy: true,
            quantity: "5000001".to_string(),
            price: "1".to_string(),
            margin: "1000000".to_string(),
            leverage: None,
        },
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::PositionTooLarge));
}

#[test]
fn test_cooldown_active_rejection_and_resolution() {
    let (mut deps, owner) = setup();

    execute(
        deps.as_mut(),
        mock_env(),
        message_info(&owner, &coins(10_000_000, USDT)),
        ExecuteMsg::DepositTreasury {},
    )
    .unwrap();

    // Set cooldown_seconds = 60
    execute(
        deps.as_mut(),
        mock_env(),
        message_info(&owner, &[]),
        ExecuteMsg::UpdateConfig {
            max_leverage: None,
            max_position_size: None,
            max_exposure_bps: None,
            max_total_margin_bps: None,
            max_position_margin: None,
            cooldown_seconds: Some(60),
            allowed_markets: Some(vec![MARKET_1.to_string(), "0xmarket_other".to_string()]),
        },
    )
    .unwrap();

    let mut env = mock_env();
    env.block.time = cosmwasm_std::Timestamp::from_seconds(1_000);

    // First position open succeeds at t = 1000
    let res = execute(
        deps.as_mut(),
        env.clone(),
        message_info(&owner, &[]),
        ExecuteMsg::OpenPosition {
            market_id: MARKET_1.to_string(),
            is_buy: true,
            quantity: "1000".to_string(),
            price: "1000".to_string(),
            margin: "1000000".to_string(),
            leverage: None,
        },
    );
    assert!(res.is_ok());

    // Calling again at t = 1030 (< 1000 + 60) fails with CooldownActive
    env.block.time = cosmwasm_std::Timestamp::from_seconds(1_030);
    let err = execute(
        deps.as_mut(),
        env.clone(),
        message_info(&owner, &[]),
        ExecuteMsg::OpenPosition {
            market_id: MARKET_1.to_string(),
            is_buy: true,
            quantity: "1000".to_string(),
            price: "1000".to_string(),
            margin: "1000000".to_string(),
            leverage: None,
        },
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::CooldownActive));

    // Calling at t = 1059 fails with CooldownActive
    env.block.time = cosmwasm_std::Timestamp::from_seconds(1_059);
    let err = execute(
        deps.as_mut(),
        env.clone(),
        message_info(&owner, &[]),
        ExecuteMsg::OpenPosition {
            market_id: MARKET_1.to_string(),
            is_buy: true,
            quantity: "1000".to_string(),
            price: "1000".to_string(),
            margin: "1000000".to_string(),
            leverage: None,
        },
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::CooldownActive));

    // Calling at t = 1060 (exactly 60s) succeeds on a different market.
    env.block.time = cosmwasm_std::Timestamp::from_seconds(1_060);
    let res = execute(
        deps.as_mut(),
        env,
        message_info(&owner, &[]),
        ExecuteMsg::OpenPosition {
            market_id: "0xmarket_other".to_string(),
            is_buy: true,
            quantity: "1000".to_string(),
            price: "1000".to_string(),
            margin: "1000000".to_string(),
            leverage: None,
        },
    );
    assert!(res.is_ok());
}

#[test]
fn test_open_position_success_and_risk_config_query() {
    let (mut deps, owner) = setup();

    execute(
        deps.as_mut(),
        mock_env(),
        message_info(&owner, &coins(10_000_000, USDT)),
        ExecuteMsg::DepositTreasury {},
    )
    .unwrap();

    // Query RiskConfig initially
    let raw = query(deps.as_ref(), mock_env(), QueryMsg::RiskConfig {}).unwrap();
    let risk_conf: RiskConfigResponse = from_json(raw).unwrap();
    assert_eq!(risk_conf.allowed_markets, vec![MARKET_1.to_string()]);
    assert_eq!(risk_conf.max_leverage, "3");
    assert_eq!(risk_conf.max_exposure_bps, 5000);
    assert_eq!(risk_conf.max_position_margin, Uint128::MAX);
    assert_eq!(risk_conf.cooldown_seconds, 0);
    assert_eq!(risk_conf.current_exposure_bps, 0);
    assert_eq!(risk_conf.current_used_exposure, Uint128::zero());
    assert_eq!(risk_conf.reserved_margin, Uint128::zero());
    assert_eq!(risk_conf.total_treasury, Uint128::new(10_000_000));

    // Successful open position
    let res = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&owner, &[]),
        ExecuteMsg::OpenPosition {
            market_id: MARKET_1.to_string(),
            is_buy: true,
            quantity: "2000".to_string(),
            price: "2000".to_string(),
            margin: "2000000".to_string(),
            leverage: Some("2".to_string()),
        },
    )
    .unwrap();
    assert_eq!(res.attributes[0].value, "open_position");

    // Query RiskConfig after position open
    let raw = query(deps.as_ref(), mock_env(), QueryMsg::RiskConfig {}).unwrap();
    let risk_conf: RiskConfigResponse = from_json(raw).unwrap();
    assert_eq!(risk_conf.current_used_exposure, Uint128::new(2_000_000));
    assert_eq!(risk_conf.reserved_margin, Uint128::new(2_000_000));
    assert_eq!(risk_conf.total_treasury, Uint128::new(10_000_000));
    // 2_000_000 / 10_000_000 = 20% = 2000 bps
    assert_eq!(risk_conf.current_exposure_bps, 2000);
}
#[test]
fn test_instantiate_and_update_validation() {
    let mut deps = inj_deps();
    let owner = addr20("owner");

    // 1. max_leverage < 1 rejected on instantiate
    let err = instantiate(
        deps.as_mut(),
        mock_env(),
        message_info(&owner, &[]),
        InstantiateMsg {
            owner: owner.to_string(),
            seed_denom: SEED.to_string(),
            treasury_denom: USDT.to_string(),
            trader: None,
            allowed_markets: vec![MARKET_1.to_string()],
            max_leverage: Some(Decimal::from_ratio(1u128, 2u128)),
            max_position_size: None,
            max_exposure_bps: None,
            max_total_margin_bps: None,
            max_position_margin: None,
            cooldown_seconds: None,
        },
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::Std(_)));

    // 2. max_exposure_bps > 10000 rejected on instantiate
    let err = instantiate(
        deps.as_mut(),
        mock_env(),
        message_info(&owner, &[]),
        InstantiateMsg {
            owner: owner.to_string(),
            seed_denom: SEED.to_string(),
            treasury_denom: USDT.to_string(),
            trader: None,
            allowed_markets: vec![MARKET_1.to_string()],
            max_leverage: None,
            max_position_size: None,
            max_exposure_bps: Some(10_001),
            max_total_margin_bps: None,
            max_position_margin: None,
            cooldown_seconds: None,
        },
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::Std(_)));

    // 3. max_position_margin == 0 rejected on instantiate
    let err = instantiate(
        deps.as_mut(),
        mock_env(),
        message_info(&owner, &[]),
        InstantiateMsg {
            owner: owner.to_string(),
            seed_denom: SEED.to_string(),
            treasury_denom: USDT.to_string(),
            trader: None,
            allowed_markets: vec![MARKET_1.to_string()],
            max_leverage: None,
            max_position_size: None,
            max_exposure_bps: None,
            max_total_margin_bps: None,
            max_position_margin: Some(Uint128::zero()),
            cooldown_seconds: None,
        },
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::Std(_)));

    // 4. max_position_size == 0 rejected on instantiate
    let err = instantiate(
        deps.as_mut(),
        mock_env(),
        message_info(&owner, &[]),
        InstantiateMsg {
            owner: owner.to_string(),
            seed_denom: SEED.to_string(),
            treasury_denom: USDT.to_string(),
            trader: None,
            allowed_markets: vec![MARKET_1.to_string()],
            max_leverage: None,
            max_position_size: Some(Uint128::zero()),
            max_exposure_bps: None,
            max_total_margin_bps: None,
            max_position_margin: None,
            cooldown_seconds: None,
        },
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::Std(_)));
}
#[test]
fn test_unstake_while_paused() {
    let (mut deps, owner) = setup();
    let user = addr20("staker1");

    // User stakes 250 SEED
    stake(&mut deps.as_mut(), &user, 250);
    assert_eq!(USER_STAKE.load(&deps.storage, &user).unwrap(), Uint128::new(250));
    assert_eq!(TOTAL_STAKED.load(&deps.storage).unwrap(), Uint128::new(250));

    // Admin pauses the contract
    execute(
        deps.as_mut(),
        mock_env(),
        message_info(&owner, &[]),
        ExecuteMsg::Pause {},
    )
    .unwrap();
    assert!(CONFIG.load(&deps.storage).unwrap().paused);

    // Verify Stake is blocked while paused
    let stake_err = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&user, &coins(50, SEED)),
        ExecuteMsg::Stake {},
    )
    .unwrap_err();
    assert!(matches!(stake_err, ContractError::Paused));

    // Verify OpenPosition is blocked while paused
    let open_err = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&owner, &[]),
        ExecuteMsg::OpenPosition {
            market_id: MARKET_1.to_string(),
            is_buy: true,
            quantity: "100".to_string(),
            price: "10".to_string(),
            margin: "100".to_string(),
            leverage: None,
        },
    )
    .unwrap_err();
    assert!(matches!(open_err, ContractError::Paused));

    // Unstake partial while paused succeeds
    let res = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&user, &[]),
        ExecuteMsg::Unstake {
            amount: Uint128::new(100),
        },
    )
    .unwrap();

    // Verify bank message sent 100 SEED
    assert_eq!(res.messages.len(), 1);
    assert_eq!(
        res.messages[0].msg,
        CosmosMsg::Bank(BankMsg::Send {
            to_address: user.to_string(),
            amount: vec![Coin::new(Uint128::new(100), SEED)],
        })
    );
    assert_eq!(USER_STAKE.load(&deps.storage, &user).unwrap(), Uint128::new(150));
    assert_eq!(TOTAL_STAKED.load(&deps.storage).unwrap(), Uint128::new(150));

    // Unstake remaining while paused succeeds
    let res2 = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&user, &[]),
        ExecuteMsg::Unstake {
            amount: Uint128::new(150),
        },
    )
    .unwrap();
    assert_eq!(res2.messages.len(), 1);
    assert_eq!(
        res2.messages[0].msg,
        CosmosMsg::Bank(BankMsg::Send {
            to_address: user.to_string(),
            amount: vec![Coin::new(Uint128::new(150), SEED)],
        })
    );
    assert_eq!(USER_STAKE.load(&deps.storage, &user).unwrap(), Uint128::zero());
    assert_eq!(TOTAL_STAKED.load(&deps.storage).unwrap(), Uint128::zero());
}

#[test]
fn test_distribute_with_total_staked_zero() {
    let (mut deps, owner) = setup();

    // Deposit treasury and record positive realized PnL so distributable profit exists
    deposit(&mut deps.as_mut(), &owner, 10_000);
    record_pnl(&mut deps.as_mut(), &owner, 2_000);

    // Initial state check: no stakers
    assert_eq!(TOTAL_STAKED.load(&deps.storage).unwrap(), Uint128::zero());
    assert_eq!(TREASURY_BALANCE.load(&deps.storage).unwrap(), Uint128::new(10_000));
    assert_eq!(TOTAL_DISTRIBUTED.load(&deps.storage).unwrap(), Uint128::zero());
    assert_eq!(GLOBAL_INDEX.load(&deps.storage).unwrap(), Decimal256::zero());

    // Attempting to distribute with total_staked == 0 must reject with NoStakers
    let err = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&owner, &[]),
        ExecuteMsg::Distribute {
            amount: Uint128::new(500),
        },
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::NoStakers));

    // Invariant check: all treasury and index state must remain completely untouched
    assert_eq!(TREASURY_BALANCE.load(&deps.storage).unwrap(), Uint128::new(10_000));
    assert_eq!(TOTAL_DISTRIBUTED.load(&deps.storage).unwrap(), Uint128::zero());
    assert_eq!(GLOBAL_INDEX.load(&deps.storage).unwrap(), Decimal256::zero());
    assert_eq!(REWARDS_RESERVED.load(&deps.storage).unwrap(), Uint128::zero());

    // Once a staker stakes, distribution succeeds
    let user = addr20("first_staker");
    stake(&mut deps.as_mut(), &user, 1_000);
    assert_eq!(TOTAL_STAKED.load(&deps.storage).unwrap(), Uint128::new(1_000));

    let res = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&owner, &[]),
        ExecuteMsg::Distribute {
            amount: Uint128::new(500),
        },
    )
    .unwrap();
    assert_eq!(res.attributes[0].key, "action");
    assert_eq!(res.attributes[0].value, "yield_distributed");
    assert_eq!(TREASURY_BALANCE.load(&deps.storage).unwrap(), Uint128::new(9_500));
    assert_eq!(TOTAL_DISTRIBUTED.load(&deps.storage).unwrap(), Uint128::new(500));
    assert_eq!(REWARDS_RESERVED.load(&deps.storage).unwrap(), Uint128::new(500));
    assert_eq!(
        GLOBAL_INDEX.load(&deps.storage).unwrap(),
        Decimal256::from_ratio(1u128, 2u128)
    );
}

#[test]
fn test_treasury_loss_staked_balances_unchanged_and_unstake_succeeds() {
    let (mut deps, owner) = setup();
    let alice = addr20("alice");
    let bob = addr20("bob");

    // Alice stakes 300 SEED, Bob stakes 700 SEED
    stake(&mut deps.as_mut(), &alice, 300);
    stake(&mut deps.as_mut(), &bob, 700);
    assert_eq!(USER_STAKE.load(&deps.storage, &alice).unwrap(), Uint128::new(300));
    assert_eq!(USER_STAKE.load(&deps.storage, &bob).unwrap(), Uint128::new(700));
    assert_eq!(TOTAL_STAKED.load(&deps.storage).unwrap(), Uint128::new(1_000));

    // Owner deposits 5,000 USDT into treasury
    deposit(&mut deps.as_mut(), &owner, 5_000);
    assert_eq!(TREASURY_BALANCE.load(&deps.storage).unwrap(), Uint128::new(5_000));

    // Simulate catastrophic trading loss / total liquidation of treasury
    // Realized PnL is deeply negative (-5,000 USDT)
    record_pnl(&mut deps.as_mut(), &owner, -5_000);
    assert_eq!(REALIZED_PNL_NET.load(&deps.storage).unwrap(), Int128::new(-5_000));

    // Free treasury drained / wiped to zero
    TREASURY_BALANCE.save(&mut deps.storage, &Uint128::zero()).unwrap();
    assert_eq!(TREASURY_BALANCE.load(&deps.storage).unwrap(), Uint128::zero());

    // INVARIANT 1: Staker principal balances in SEED are 100% isolated and unchanged
    assert_eq!(USER_STAKE.load(&deps.storage, &alice).unwrap(), Uint128::new(300));
    assert_eq!(USER_STAKE.load(&deps.storage, &bob).unwrap(), Uint128::new(700));
    assert_eq!(TOTAL_STAKED.load(&deps.storage).unwrap(), Uint128::new(1_000));

    // INVARIANT 2: Alice unstakes full 300 SEED without error despite zero treasury & negative PnL
    let res_alice = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&alice, &[]),
        ExecuteMsg::Unstake {
            amount: Uint128::new(300),
        },
    )
    .unwrap();
    assert_eq!(res_alice.messages.len(), 1);
    assert_eq!(
        res_alice.messages[0].msg,
        CosmosMsg::Bank(BankMsg::Send {
            to_address: alice.to_string(),
            amount: vec![Coin::new(Uint128::new(300), SEED)],
        })
    );
    assert_eq!(USER_STAKE.load(&deps.storage, &alice).unwrap(), Uint128::zero());
    assert_eq!(TOTAL_STAKED.load(&deps.storage).unwrap(), Uint128::new(700));

    // INVARIANT 3: Bob unstakes full 700 SEED without error
    let res_bob = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&bob, &[]),
        ExecuteMsg::Unstake {
            amount: Uint128::new(700),
        },
    )
    .unwrap();
    assert_eq!(res_bob.messages.len(), 1);
    assert_eq!(
        res_bob.messages[0].msg,
        CosmosMsg::Bank(BankMsg::Send {
            to_address: bob.to_string(),
            amount: vec![Coin::new(Uint128::new(700), SEED)],
        })
    );
    assert_eq!(USER_STAKE.load(&deps.storage, &bob).unwrap(), Uint128::zero());
    assert_eq!(TOTAL_STAKED.load(&deps.storage).unwrap(), Uint128::zero());
}




#[test]
fn test_action_events_emitted_on_stake_unstake_and_treasury() {
    let (mut deps, owner) = setup();
    let alice = addr20("alice");

    // 1. Stake event
    let res = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&alice, &[coin(100, SEED)]),
        ExecuteMsg::Stake {},
    )
    .unwrap();
    let attr = |key: &str| res.attributes.iter().find(|a| a.key == key).map(|a| a.value.as_str());
    assert_eq!(attr("action"), Some("seed_staked"));
    assert_eq!(attr("user"), Some(alice.as_str()));
    assert_eq!(attr("amount"), Some("100"));
    assert_eq!(attr("total_staked"), Some("100"));

    // 2. Unstake event
    let res = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&alice, &[]),
        ExecuteMsg::Unstake {
            amount: Uint128::new(40),
        },
    )
    .unwrap();
    let attr = |key: &str| res.attributes.iter().find(|a| a.key == key).map(|a| a.value.as_str());
    assert_eq!(attr("action"), Some("seed_unstaked"));
    assert_eq!(attr("user"), Some(alice.as_str()));
    assert_eq!(attr("amount"), Some("40"));
    assert_eq!(attr("remaining_stake"), Some("60"));
    assert_eq!(attr("total_staked"), Some("60"));

    // 3. DepositTreasury event
    let res = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&owner, &[coin(5000, USDT)]),
        ExecuteMsg::DepositTreasury {},
    )
    .unwrap();
    let attr = |key: &str| res.attributes.iter().find(|a| a.key == key).map(|a| a.value.as_str());
    assert_eq!(attr("action"), Some("treasury_deposited"));
    assert_eq!(attr("owner"), Some(owner.as_str()));
    assert_eq!(attr("amount"), Some("5000"));
    assert_eq!(attr("treasury_balance"), Some("5000"));
}

#[test]
fn test_action_events_emitted_on_positions_distribute_and_claim() {
    let (mut deps, owner) = setup();
    let alice = addr20("alice");

    // Setup: alice stakes 60, owner deposits 5000
    stake(&mut deps.as_mut(), &alice, 60);
    deposit(&mut deps.as_mut(), &owner, 5000);

    // 1. OpenPosition event
    let res = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&owner, &[]),
        ExecuteMsg::OpenPosition {
            market_id: MARKET_1.to_string(),
            is_buy: true,
            quantity: "100".to_string(),
            price: "30".to_string(),
            margin: "1000".to_string(),
            leverage: Some("3".to_string()),
        },
    )
    .unwrap();
    let attr = |key: &str| res.attributes.iter().find(|a| a.key == key).map(|a| a.value.as_str());
    assert_eq!(attr("action"), Some("open_position"));
    assert_eq!(attr("trader"), Some(owner.as_str()));
    assert_eq!(attr("market_id"), Some(MARKET_1));
    assert_eq!(attr("market"), Some(MARKET_1));
    assert_eq!(attr("margin"), Some("1000"));
    assert_eq!(attr("leverage"), Some("3"));
    assert_eq!(attr("is_buy"), Some("true"));
    assert_eq!(attr("quantity"), Some("100"));
    assert_eq!(attr("price"), Some("30"));

    // 2. ClosePosition event
    let res = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&owner, &[]),
        ExecuteMsg::ClosePosition {
            market_id: MARKET_1.to_string(),
            quantity: None,
        },
    )
    .unwrap();
    let attr = |key: &str| res.attributes.iter().find(|a| a.key == key).map(|a| a.value.as_str());
    assert_eq!(attr("action"), Some("close_position"));
    assert_eq!(attr("trader"), Some(owner.as_str()));
    assert_eq!(attr("market_id"), Some(MARKET_1));
    assert_eq!(attr("market"), Some(MARKET_1));
    assert_eq!(attr("quantity"), Some("100"));
    assert_eq!(attr("margin_released"), Some("1000"));
    assert_eq!(attr("realized_pnl"), Some("0"));

    // Record PnL so distribute succeeds
    record_pnl(&mut deps.as_mut(), &owner, 1000);

    // 3. Distribute event
    let res = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&owner, &[]),
        ExecuteMsg::Distribute {
            amount: Uint128::new(300),
        },
    )
    .unwrap();
    let attr = |key: &str| res.attributes.iter().find(|a| a.key == key).map(|a| a.value.as_str());
    assert_eq!(attr("action"), Some("yield_distributed"));
    assert_eq!(attr("amount"), Some("300"));
    assert_eq!(attr("new_index"), Some("5"));
    assert_eq!(attr("new_reward_index"), Some("5"));
    assert_eq!(attr("total_staked"), Some("60"));

    // 4. Claim event
    let res = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&alice, &[]),
        ExecuteMsg::Claim {},
    )
    .unwrap();
    let attr = |key: &str| res.attributes.iter().find(|a| a.key == key).map(|a| a.value.as_str());
    assert_eq!(attr("action"), Some("yield_claimed"));
    assert_eq!(attr("user"), Some(alice.as_str()));
    assert_eq!(attr("amount"), Some("300"));
}

#[test]
fn test_action_events_emitted_on_pause_and_governance() {
    let (mut deps, owner) = setup();
    let new_owner = addr20("new_owner");
    let trader = addr20("trader");

    // 1. Pause event
    let res = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&owner, &[]),
        ExecuteMsg::Pause {},
    )
    .unwrap();
    let attr = |key: &str| res.attributes.iter().find(|a| a.key == key).map(|a| a.value.as_str());
    assert_eq!(attr("action"), Some("paused"));
    assert_eq!(attr("owner"), Some(owner.as_str()));

    // 2. Unpause event
    let res = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&owner, &[]),
        ExecuteMsg::Unpause {},
    )
    .unwrap();
    let attr = |key: &str| res.attributes.iter().find(|a| a.key == key).map(|a| a.value.as_str());
    assert_eq!(attr("action"), Some("unpaused"));
    assert_eq!(attr("owner"), Some(owner.as_str()));

    // 3. ProposeOwner event
    let res = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&owner, &[]),
        ExecuteMsg::ProposeOwner {
            new_owner: new_owner.to_string(),
        },
    )
    .unwrap();
    let attr = |key: &str| res.attributes.iter().find(|a| a.key == key).map(|a| a.value.as_str());
    assert_eq!(attr("action"), Some("owner_proposed"));
    assert_eq!(attr("current_owner"), Some(owner.as_str()));
    assert_eq!(attr("pending_owner"), Some(new_owner.as_str()));

    // 4. AcceptOwner event
    let res = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&new_owner, &[]),
        ExecuteMsg::AcceptOwner {},
    )
    .unwrap();
    let attr = |key: &str| res.attributes.iter().find(|a| a.key == key).map(|a| a.value.as_str());
    assert_eq!(attr("action"), Some("owner_accepted"));
    assert_eq!(attr("previous_owner"), Some(owner.as_str()));
    assert_eq!(attr("owner"), Some(new_owner.as_str()));

    // 5. UpdateTrader event
    let res = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&new_owner, &[]),
        ExecuteMsg::UpdateTrader {
            new_trader: Some(trader.to_string()),
        },
    )
    .unwrap();
    let attr = |key: &str| res.attributes.iter().find(|a| a.key == key).map(|a| a.value.as_str());
    assert_eq!(attr("action"), Some("trader_updated"));
    assert_eq!(attr("owner"), Some(new_owner.as_str()));
    assert_eq!(attr("trader"), Some(trader.as_str()));

    // 6. UpdateConfig event
    let res = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&new_owner, &[]),
        ExecuteMsg::UpdateConfig {
            max_leverage: Some(Decimal::percent(500)),
            max_position_size: Some(Uint128::new(2_000_000)),
            max_exposure_bps: Some(6000),
            max_total_margin_bps: None,
            max_position_margin: Some(Uint128::new(3_000_000)),
            cooldown_seconds: Some(120),
            allowed_markets: Some(vec![MARKET_1.to_string()]),
        },
    )
    .unwrap();
    let attr = |key: &str| res.attributes.iter().find(|a| a.key == key).map(|a| a.value.as_str());
    assert_eq!(attr("action"), Some("config_updated"));
    assert_eq!(attr("owner"), Some(new_owner.as_str()));
    assert_eq!(attr("max_leverage"), Some("5"));
    assert_eq!(attr("max_position_size"), Some("2000000"));
    assert_eq!(attr("max_exposure_bps"), Some("6000"));
    assert_eq!(attr("max_position_margin"), Some("3000000"));
    assert_eq!(attr("cooldown_seconds"), Some("120"));
    assert_eq!(attr("allowed_markets"), Some(MARKET_1));
}

#[test]
fn test_new_queries_treasury_state_open_positions_total_staked() {
    let (mut deps, owner) = setup();
    let alice = addr20("alice");

    // 1. TotalStaked query initially zero
    let total: TotalStakedResponse = from_json(
        query(deps.as_ref(), mock_env(), QueryMsg::TotalStaked {}).unwrap(),
    )
    .unwrap();
    assert_eq!(total.amount, Uint128::zero());

    // 2. TreasuryState query initially zero
    let state: TreasuryStateResponse = from_json(
        query(deps.as_ref(), mock_env(), QueryMsg::TreasuryState {}).unwrap(),
    )
    .unwrap();
    assert_eq!(state.balance, Uint128::zero());
    assert_eq!(state.reserved_margin, Uint128::zero());
    assert_eq!(state.undistributed, Uint128::zero());

    // Staking & Treasury funding
    stake(&mut deps.as_mut(), &alice, 500);
    deposit(&mut deps.as_mut(), &owner, 10_000);
    record_pnl(&mut deps.as_mut(), &owner, 2_000);

    // Verify TotalStaked query
    let total: TotalStakedResponse = from_json(
        query(deps.as_ref(), mock_env(), QueryMsg::TotalStaked {}).unwrap(),
    )
    .unwrap();
    assert_eq!(total.amount, Uint128::new(500));

    // Open first position on MARKET_1
    execute(
        deps.as_mut(),
        mock_env(),
        message_info(&owner, &[]),
        ExecuteMsg::OpenPosition {
            market_id: MARKET_1.to_string(),
            is_buy: true,
            quantity: "1000".to_string(),
            price: "10".to_string(),
            margin: "4000".to_string(),
            leverage: Some("2.5".to_string()),
        },
    )
    .unwrap();

    // Verify storage pos directly
    let pos_in_storage = POSITIONS.load(&deps.storage, MARKET_1).unwrap();
    assert_eq!(pos_in_storage.margin, Uint128::new(4_000));

    // Verify TreasuryState query after position opened
    let state: TreasuryStateResponse = from_json(
        query(deps.as_ref(), mock_env(), QueryMsg::TreasuryState {}).unwrap(),
    )
    .unwrap();
    assert_eq!(state.balance, Uint128::new(6_000));
    assert_eq!(state.reserved_margin, Uint128::new(4_000));
    assert_eq!(state.undistributed, Uint128::new(2_000));

    // Verify Config query
    let config: ConfigResponse = from_json(
        query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap(),
    )
    .unwrap();
    assert_eq!(config.owner, owner.as_str());
    assert!(!config.paused);

    // Add MARKET_2 and open second position to test OpenPositions pagination
    let market_2 = "0xmarket2";
    execute(
        deps.as_mut(),
        mock_env(),
        message_info(&owner, &[]),
        ExecuteMsg::UpdateConfig {
            max_leverage: Some(Decimal::percent(500)),
            max_position_size: None,
            max_exposure_bps: Some(8000),
            max_total_margin_bps: None,
            max_position_margin: None,
            cooldown_seconds: Some(0),
            allowed_markets: Some(vec![MARKET_1.to_string(), market_2.to_string()]),
        },
    )
    .unwrap();

    execute(
        deps.as_mut(),
        mock_env(),
        message_info(&owner, &[]),
        ExecuteMsg::OpenPosition {
            market_id: market_2.to_string(),
            is_buy: false,
            quantity: "500".to_string(),
            price: "20".to_string(),
            margin: "2000".to_string(),
            leverage: Some("5".to_string()),
        },
    )
    .unwrap();

    // Query OpenPositions without pagination (all positions)
    let open_all: OpenPositionsResponse = from_json(
        query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::OpenPositions {
                start_after: None,
                limit: None,
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(open_all.positions.len(), 2);

    // Query OpenPositions with limit 1
    let page1: OpenPositionsResponse = from_json(
        query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::OpenPositions {
                start_after: None,
                limit: Some(1),
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(page1.positions.len(), 1);
    let first_market = page1.positions[0].market_id.clone();

    // Query next page with start_after
    let page2: OpenPositionsResponse = from_json(
        query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::OpenPositions {
                start_after: Some(first_market),
                limit: Some(1),
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(page2.positions.len(), 1);
    assert_ne!(page1.positions[0].market_id, page2.positions[0].market_id);

    // Close position on MARKET_1 and verify OpenPositions list decreases
    execute(
        deps.as_mut(),
        mock_env(),
        message_info(&owner, &[]),
        ExecuteMsg::ClosePosition {
            market_id: MARKET_1.to_string(),
            quantity: None,
        },
    )
    .unwrap();

    let open_after_close: OpenPositionsResponse = from_json(
        query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::OpenPositions {
                start_after: None,
                limit: None,
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(open_after_close.positions.len(), 1);
    assert_eq!(open_after_close.positions[0].market_id, market_2);
}

#[test]
fn test_stake_before_and_after_distribute_later_staker_gets_no_old_profit() {
    let (mut deps, owner) = setup();
    let alice = addr20("alice");
    let bob = addr20("bob");

    // Alice stakes 500 SEED
    stake(&mut deps.as_mut(), &alice, 500);

    // Deposit treasury and record PnL
    deposit(&mut deps.as_mut(), &owner, 10_000);
    record_pnl(&mut deps.as_mut(), &owner, 1_000);

    // Round 1: Distribute 300 USDT
    distribute(&mut deps.as_mut(), &owner, 300);

    // Alice's pending yield is 300
    assert_eq!(pending(&deps.as_ref(), &alice), Uint128::new(300));

    // Bob now stakes 500 SEED AFTER Round 1
    stake(&mut deps.as_mut(), &bob, 500);

    // Bob has zero pending yield from Round 1
    assert_eq!(pending(&deps.as_ref(), &bob), Uint128::zero());

    // Bob attempting to claim now fails with NothingToClaim
    let err = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&bob, &[]),
        ExecuteMsg::Claim {},
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::NothingToClaim));

    // Round 2: Distribute 400 USDT
    // Total staked is 1000 (Alice 500, Bob 500 = 50% each)
    distribute(&mut deps.as_mut(), &owner, 400);

    // Alice: 300 (round 1) + 200 (round 2) = 500
    assert_eq!(pending(&deps.as_ref(), &alice), Uint128::new(500));
    // Bob: 0 (round 1) + 200 (round 2) = 200
    assert_eq!(pending(&deps.as_ref(), &bob), Uint128::new(200));

    // Alice claims 500
    let res_a = claim(&mut deps.as_mut(), &alice);
    sent_usdt(&res_a, &alice, 500);

    // Bob claims 200 (strictly round 2 share, 0 from round 1)
    let res_b = claim(&mut deps.as_mut(), &bob);
    sent_usdt(&res_b, &bob, 200);

    // Both calling claim again fail with NothingToClaim
    let err_a = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&alice, &[]),
        ExecuteMsg::Claim {},
    )
    .unwrap_err();
    assert!(matches!(err_a, ContractError::NothingToClaim));

    let err_b = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&bob, &[]),
        ExecuteMsg::Claim {},
    )
    .unwrap_err();
    assert!(matches!(err_b, ContractError::NothingToClaim));
}

#[test]
fn test_claim_twice_does_not_pay_twice() {
    let (mut deps, owner) = setup();
    let alice = addr20("alice");

    // Alice stakes 1000 SEED
    stake(&mut deps.as_mut(), &alice, 1000);

    // Deposit treasury and distribute 500 USDT
    deposit(&mut deps.as_mut(), &owner, 5000);
    record_pnl(&mut deps.as_mut(), &owner, 1000);
    distribute(&mut deps.as_mut(), &owner, 500);

    // First claim succeeds and transfers 500 USDT
    let res = claim(&mut deps.as_mut(), &alice);
    sent_usdt(&res, &alice, 500);
    assert_eq!(USER_CLAIMED.load(&deps.storage, &alice).unwrap(), Uint128::new(500));
    assert_eq!(USER_PENDING.load(&deps.storage, &alice).unwrap(), Uint128::zero());

    // Second claim immediately fails with NothingToClaim
    let err2 = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&alice, &[]),
        ExecuteMsg::Claim {},
    )
    .unwrap_err();
    assert!(matches!(err2, ContractError::NothingToClaim));

    // Alice partially unstakes 500 SEED
    execute(
        deps.as_mut(),
        mock_env(),
        message_info(&alice, &[]),
        ExecuteMsg::Unstake {
            amount: Uint128::new(500),
        },
    )
    .unwrap();

    // Third claim still fails with NothingToClaim
    let err3 = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&alice, &[]),
        ExecuteMsg::Claim {},
    )
    .unwrap_err();
    assert!(matches!(err3, ContractError::NothingToClaim));

    // Alice unstakes the remainder (500 SEED)
    execute(
        deps.as_mut(),
        mock_env(),
        message_info(&alice, &[]),
        ExecuteMsg::Unstake {
            amount: Uint128::new(500),
        },
    )
    .unwrap();

    // Fourth claim still fails with NothingToClaim
    let err4 = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&alice, &[]),
        ExecuteMsg::Claim {},
    )
    .unwrap_err();
    assert!(matches!(err4, ContractError::NothingToClaim));

    // Total claimed remains exactly 500
    assert_eq!(USER_CLAIMED.load(&deps.storage, &alice).unwrap(), Uint128::new(500));
}


#[test]
fn test_role_permissions_non_trader_cannot_open_and_trader_cannot_call_owner_messages() {
    let mut deps = inj_deps();
    let owner = addr20("owner");
    let trader = addr20("trader");
    let stranger = addr20("stranger");

    instantiate(
        deps.as_mut(),
        mock_env(),
        message_info(&owner, &[]),
        InstantiateMsg {
            owner: owner.to_string(),
            seed_denom: SEED.to_string(),
            treasury_denom: USDT.to_string(),
            trader: Some(trader.to_string()),
            allowed_markets: vec![MARKET_1.to_string()],
            max_leverage: Some(Decimal::from_ratio(3u128, 1u128)),
            max_position_size: Some(Uint128::new(10_000_000)),
            max_exposure_bps: Some(8_000),
            max_total_margin_bps: None,
            max_position_margin: Some(Uint128::new(5_000_000)),
            cooldown_seconds: Some(0),
        },
    )
    .unwrap();

    // Owner deposits treasury
    execute(
        deps.as_mut(),
        mock_env(),
        message_info(&owner, &coins(5_000_000, USDT)),
        ExecuteMsg::DepositTreasury {},
    )
    .unwrap();

    // 1. Non-trader (stranger) cannot open position
    let err = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&stranger, &[]),
        ExecuteMsg::OpenPosition {
            market_id: MARKET_1.to_string(),
            is_buy: true,
            quantity: "100".to_string(),
            price: "100".to_string(),
            margin: "10000".to_string(),
            leverage: None,
        },
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::Unauthorized));

    // 2. Non-trader (stranger) cannot close position
    let err = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&stranger, &[]),
        ExecuteMsg::ClosePosition {
            market_id: MARKET_1.to_string(),
            quantity: None,
        },
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::Unauthorized));

    // 3. Trader CAN open position
    execute(
        deps.as_mut(),
        mock_env(),
        message_info(&trader, &[]),
        ExecuteMsg::OpenPosition {
            market_id: MARKET_1.to_string(),
            is_buy: true,
            quantity: "100".to_string(),
            price: "100".to_string(),
            margin: "10000".to_string(),
            leverage: None,
        },
    )
    .unwrap();

    // 4. Trader CAN close position
    execute(
        deps.as_mut(),
        mock_env(),
        message_info(&trader, &[]),
        ExecuteMsg::ClosePosition {
            market_id: MARKET_1.to_string(),
            quantity: None,
        },
    )
    .unwrap();

    // 5. Trader CANNOT call owner-only messages:
    // a) DepositTreasury
    let err = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&trader, &coins(1000, USDT)),
        ExecuteMsg::DepositTreasury {},
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::Unauthorized));

    // b) Distribute
    let err = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&trader, &[]),
        ExecuteMsg::Distribute { amount: Uint128::new(100) },
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::Unauthorized));

    // c) Pause
    let err = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&trader, &[]),
        ExecuteMsg::Pause {},
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::Unauthorized));

    // d) Unpause
    let err = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&trader, &[]),
        ExecuteMsg::Unpause {},
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::Unauthorized));

    // e) ProposeOwner
    let err = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&trader, &[]),
        ExecuteMsg::ProposeOwner { new_owner: trader.to_string() },
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::Unauthorized));

    // f) UpdateTrader
    let err = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&trader, &[]),
        ExecuteMsg::UpdateTrader { new_trader: None },
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::Unauthorized));

    // g) UpdateConfig
    let err = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&trader, &[]),
        ExecuteMsg::UpdateConfig {
            max_leverage: None,
            max_position_size: None,
            max_exposure_bps: None,
            max_total_margin_bps: None,
            max_position_margin: None,
            cooldown_seconds: None,
            allowed_markets: None,
        },
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::Unauthorized));

    // h) RecordPnl
    let err = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&trader, &[]),
        ExecuteMsg::RecordPnl { delta: Int128::new(100) },
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::Unauthorized));
}


#[test]
fn test_wrong_address_cannot_accept_ownership_transfer_exhaustive() {
    let (mut deps, owner) = setup();
    let stranger = addr20("stranger");
    let intended_owner = addr20("intended_owner");

    // Phase 1: No transfer proposed yet -> anyone calling AcceptOwner fails
    let err = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&stranger, &[]),
        ExecuteMsg::AcceptOwner {},
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::Unauthorized));

    let err = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&intended_owner, &[]),
        ExecuteMsg::AcceptOwner {},
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::Unauthorized));

    let err = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&owner, &[]),
        ExecuteMsg::AcceptOwner {},
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::Unauthorized));

    // Phase 2: Owner proposes intended_owner
    execute(
        deps.as_mut(),
        mock_env(),
        message_info(&owner, &[]),
        ExecuteMsg::ProposeOwner {
            new_owner: intended_owner.to_string(),
        },
    )
    .unwrap();

    // Wrong address 1: stranger fails
    let err = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&stranger, &[]),
        ExecuteMsg::AcceptOwner {},
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::Unauthorized));

    // Wrong address 2: current owner fails
    let err = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&owner, &[]),
        ExecuteMsg::AcceptOwner {},
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::Unauthorized));

    // Intended address succeeds
    let res = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&intended_owner, &[]),
        ExecuteMsg::AcceptOwner {},
    )
    .unwrap();
    assert_eq!(res.attributes[0].value, "owner_accepted");
    assert_eq!(res.attributes[1].value, owner.as_str());
    assert_eq!(res.attributes[2].value, intended_owner.as_str());

    // Phase 3: Ownership is transferred; pending is cleared
    let cfg = CONFIG.load(&deps.storage).unwrap();
    assert_eq!(cfg.owner, intended_owner);
    assert!(PENDING_OWNER.may_load(&deps.storage).unwrap().is_none());

    // Replay attempts all fail
    let err = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&intended_owner, &[]),
        ExecuteMsg::AcceptOwner {},
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::Unauthorized));

    let err = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&owner, &[]),
        ExecuteMsg::AcceptOwner {},
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::Unauthorized));
}


#[test]
fn test_exhaustive_risk_limit_rejections_market_size_leverage() {
    let (mut deps, owner) = setup();

    execute(
        deps.as_mut(),
        mock_env(),
        message_info(&owner, &coins(10_000_000, USDT)),
        ExecuteMsg::DepositTreasury {},
    )
    .unwrap();

    // Configure specific risk bounds:
    // max_leverage: 3x, max_position_size: 10_000, max_position_margin: 2_000_000
    execute(
        deps.as_mut(),
        mock_env(),
        message_info(&owner, &[]),
        ExecuteMsg::UpdateConfig {
            max_leverage: Some(Decimal::from_ratio(3u128, 1u128)),
            max_position_size: Some(Uint128::new(10_000)),
            max_exposure_bps: Some(5_000),
            max_total_margin_bps: None,
            max_position_margin: Some(Uint128::new(2_000_000)),
            cooldown_seconds: Some(60),
            allowed_markets: Some(vec![MARKET_1.to_string()]),
        },
    )
    .unwrap();

    // 1. MarketNotAllowed
    let err = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&owner, &[]),
        ExecuteMsg::OpenPosition {
            market_id: "unlisted_market".to_string(),
            is_buy: true,
            quantity: "100".to_string(),
            price: "10".to_string(),
            margin: "1000".to_string(),
            leverage: None,
        },
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::MarketNotAllowed { .. }));

    // 2. ZeroAmount (quantity is 0)
    let err = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&owner, &[]),
        ExecuteMsg::OpenPosition {
            market_id: MARKET_1.to_string(),
            is_buy: true,
            quantity: "0".to_string(),
            price: "10".to_string(),
            margin: "1000".to_string(),
            leverage: None,
        },
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::ZeroAmount));

    // 3. ZeroAmount (margin is 0)
    let err = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&owner, &[]),
        ExecuteMsg::OpenPosition {
            market_id: MARKET_1.to_string(),
            is_buy: true,
            quantity: "100".to_string(),
            price: "10".to_string(),
            margin: "0".to_string(),
            leverage: None,
        },
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::ZeroAmount));

    // 4. PositionTooLarge (quantity > max_position_size: 10_001 > 10_000)
    let err = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&owner, &[]),
        ExecuteMsg::OpenPosition {
            market_id: MARKET_1.to_string(),
            is_buy: true,
            quantity: "10001".to_string(),
            price: "10".to_string(),
            margin: "100000".to_string(),
            leverage: None,
        },
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::PositionTooLarge));

    // 5. PositionTooLarge (margin > max_position_margin: 2_000_001 > 2_000_000)
    let err = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&owner, &[]),
        ExecuteMsg::OpenPosition {
            market_id: MARKET_1.to_string(),
            is_buy: true,
            quantity: "100".to_string(),
            price: "10".to_string(),
            margin: "2000001".to_string(),
            leverage: None,
        },
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::PositionTooLarge));

    // 6. LeverageTooHigh (explicit leverage 3.5 > max 3.0)
    let err = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&owner, &[]),
        ExecuteMsg::OpenPosition {
            market_id: MARKET_1.to_string(),
            is_buy: true,
            quantity: "100".to_string(),
            price: "10".to_string(),
            margin: "1000".to_string(),
            leverage: Some("3.5".to_string()),
        },
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::LeverageTooHigh));

    // 7. LeverageTooHigh (implicit notional: qty 1000 * price 40 = 40_000; margin = 10_000; 4x > 3x)
    let err = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&owner, &[]),
        ExecuteMsg::OpenPosition {
            market_id: MARKET_1.to_string(),
            is_buy: true,
            quantity: "1000".to_string(),
            price: "40".to_string(),
            margin: "10000".to_string(),
            leverage: None,
        },
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::LeverageTooHigh));
}


#[test]
fn test_exhaustive_risk_limit_rejections_treasury_exposure_cooldown_paused() {
    let (mut deps, owner) = setup();

    execute(
        deps.as_mut(),
        mock_env(),
        message_info(&owner, &coins(10_000_000, USDT)),
        ExecuteMsg::DepositTreasury {},
    )
    .unwrap();

    // 8. InsufficientTreasury (margin > free treasury: 10_000_001 > 10_000_000)
    let err = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&owner, &[]),
        ExecuteMsg::OpenPosition {
            market_id: MARKET_1.to_string(),
            is_buy: true,
            quantity: "100".to_string(),
            price: "10".to_string(),
            margin: "10000001".to_string(),
            leverage: None,
        },
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::InsufficientTreasury { .. }));

    // 9. ExposureLimitExceeded (50% max exposure = 5_000_000. Attempting 5_000_001)
    let err = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&owner, &[]),
        ExecuteMsg::OpenPosition {
            market_id: MARKET_1.to_string(),
            is_buy: true,
            quantity: "100".to_string(),
            price: "10".to_string(),
            margin: "5000001".to_string(),
            leverage: None,
        },
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::ExposureLimitExceeded));

    // Configure cooldown_seconds = 60
    execute(
        deps.as_mut(),
        mock_env(),
        message_info(&owner, &[]),
        ExecuteMsg::UpdateConfig {
            max_leverage: None,
            max_position_size: None,
            max_exposure_bps: None,
            max_total_margin_bps: None,
            max_position_margin: None,
            cooldown_seconds: Some(60),
            allowed_markets: None,
        },
    )
    .unwrap();

    // 10. CooldownActive: First open valid position at t = 1000
    let mut env = mock_env();
    env.block.time = cosmwasm_std::Timestamp::from_seconds(1000);
    execute(
        deps.as_mut(),
        env.clone(),
        message_info(&owner, &[]),
        ExecuteMsg::OpenPosition {
            market_id: MARKET_1.to_string(),
            is_buy: true,
            quantity: "100".to_string(),
            price: "10".to_string(),
            margin: "1000".to_string(),
            leverage: None,
        },
    )
    .unwrap();

    // Opening again at t = 1030 (< 1000 + 60s cooldown) fails
    env.block.time = cosmwasm_std::Timestamp::from_seconds(1030);
    let err = execute(
        deps.as_mut(),
        env.clone(),
        message_info(&owner, &[]),
        ExecuteMsg::OpenPosition {
            market_id: MARKET_1.to_string(),
            is_buy: true,
            quantity: "100".to_string(),
            price: "10".to_string(),
            margin: "1000".to_string(),
            leverage: None,
        },
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::CooldownActive));

    // 11. Paused: Pause the contract, then try to open position
    execute(
        deps.as_mut(),
        env.clone(),
        message_info(&owner, &[]),
        ExecuteMsg::Pause {},
    )
    .unwrap();

    env.block.time = cosmwasm_std::Timestamp::from_seconds(2000);
    let err = execute(
        deps.as_mut(),
        env,
        message_info(&owner, &[]),
        ExecuteMsg::OpenPosition {
            market_id: MARKET_1.to_string(),
            is_buy: true,
            quantity: "100".to_string(),
            price: "10".to_string(),
            margin: "1000".to_string(),
            leverage: None,
        },
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::Paused));
}


#[test]
fn test_multiple_stakers_multiple_rounds_exact_payouts_and_dust_accounting() {
    // -------------------------------------------------------------
    // PART 1: EXACT INTEGER DIVISION (Zero Dust)
    // -------------------------------------------------------------
    let (mut deps, owner) = setup();
    let a = addr20("staker_a");
    let b = addr20("staker_b");
    let c = addr20("staker_c");

    // Deposit ample treasury and record PnL
    deposit(&mut deps.as_mut(), &owner, 100_000);
    record_pnl(&mut deps.as_mut(), &owner, 50_000);

    // Initial stakes: A: 200, B: 300, C: 500 (Total = 1000)
    stake(&mut deps.as_mut(), &a, 200);
    stake(&mut deps.as_mut(), &b, 300);
    stake(&mut deps.as_mut(), &c, 500);
    assert_eq!(TOTAL_STAKED.load(&deps.storage).unwrap(), Uint128::new(1000));

    // Round 1: Distribute 1000 USDT
    distribute(&mut deps.as_mut(), &owner, 1000);

    // Dynamic stake rebalancing before Round 2:
    // A stakes 100 more (now 300)
    stake(&mut deps.as_mut(), &a, 100);
    // B unstakes 100 (now 200)
    execute(
        deps.as_mut(),
        mock_env(),
        message_info(&b, &[]),
        ExecuteMsg::Unstake {
            amount: Uint128::new(100),
        },
    )
    .unwrap();
    // C keeps 500
    // Total staked is still 1000 (A: 300, B: 200, C: 500)
    assert_eq!(TOTAL_STAKED.load(&deps.storage).unwrap(), Uint128::new(1000));

    // Round 2: Distribute 2000 USDT
    distribute(&mut deps.as_mut(), &owner, 2000);

    // Expected Payouts:
    // A: 200 (R1) + 600 (R2) = 800
    // B: 300 (R1) + 400 (R2) = 700
    // C: 500 (R1) + 1000 (R2) = 1500
    assert_eq!(pending(&deps.as_ref(), &a), Uint128::new(800));
    assert_eq!(pending(&deps.as_ref(), &b), Uint128::new(700));
    assert_eq!(pending(&deps.as_ref(), &c), Uint128::new(1500));

    let res_a = claim(&mut deps.as_mut(), &a);
    let res_b = claim(&mut deps.as_mut(), &b);
    let res_c = claim(&mut deps.as_mut(), &c);

    sent_usdt(&res_a, &a, 800);
    sent_usdt(&res_b, &b, 700);
    sent_usdt(&res_c, &c, 1500);

    let total_claimed_p1 = Uint128::new(800 + 700 + 1500);
    let total_distributed_p1 = Uint128::new(3000);
    assert_eq!(total_claimed_p1, total_distributed_p1);
    assert_eq!(REWARDS_RESERVED.load(&deps.storage).unwrap(), Uint128::zero());

    // -------------------------------------------------------------
    // PART 2: FRACTIONAL DIVISION & ROUNDING DUST ACCOUNTING
    // -------------------------------------------------------------
    let (mut deps2, owner2) = setup();
    let u1 = addr20("user1");
    let u2 = addr20("user2");
    let u3 = addr20("user3");

    deposit(&mut deps2.as_mut(), &owner2, 10_000);
    record_pnl(&mut deps2.as_mut(), &owner2, 10_000);

    // Three equal stakers: 100 each -> total 300
    stake(&mut deps2.as_mut(), &u1, 100);
    stake(&mut deps2.as_mut(), &u2, 100);
    stake(&mut deps2.as_mut(), &u3, 100);

    // Distribute 100 USDT over 300 stake
    // 100 / 300 = 0.333333333333333333...
    // Each staker earned: floor(100 * 0.333333333333333333) = 33 USDT
    distribute(&mut deps2.as_mut(), &owner2, 100);

    assert_eq!(pending(&deps2.as_ref(), &u1), Uint128::new(33));
    assert_eq!(pending(&deps2.as_ref(), &u2), Uint128::new(33));
    assert_eq!(pending(&deps2.as_ref(), &u3), Uint128::new(33));

    let res_u1 = claim(&mut deps2.as_mut(), &u1);
    let res_u2 = claim(&mut deps2.as_mut(), &u2);
    let res_u3 = claim(&mut deps2.as_mut(), &u3);

    sent_usdt(&res_u1, &u1, 33);
    sent_usdt(&res_u2, &u2, 33);
    sent_usdt(&res_u3, &u3, 33);

    let total_claimed_p2 = Uint128::new(33 * 3); // 99 USDT
    let total_distributed_p2 = Uint128::new(100); // 100 USDT
    let rounding_dust = total_distributed_p2 - total_claimed_p2; // 1 USDT
    assert_eq!(rounding_dust, Uint128::new(1));

    // WHERE DOES THE ROUNDING DUST GO?
    // 1. In contract state: REWARDS_RESERVED holds exactly the 1 USDT dust.
    assert_eq!(REWARDS_RESERVED.load(&deps2.storage).unwrap(), Uint128::new(1));

    // 2. In total accounting: TOTAL_DISTRIBUTED = 100, USER_CLAIMED sum = 99.
    let sum_user_claimed = USER_CLAIMED.load(&deps2.storage, &u1).unwrap()
        + USER_CLAIMED.load(&deps2.storage, &u2).unwrap()
        + USER_CLAIMED.load(&deps2.storage, &u3).unwrap();
    assert_eq!(sum_user_claimed, total_claimed_p2);
    assert_eq!(TOTAL_DISTRIBUTED.load(&deps2.storage).unwrap(), total_distributed_p2);

    // 3. In free treasury: Decremented by the full 100 USDT at distribution.
    // 4. In native bank balance: The 1 USDT token remains unspent inside the contract.
    // 5. Subsequent claims for all users fail with NothingToClaim, guaranteeing solvency.
    assert!(matches!(
        execute(deps2.as_mut(), mock_env(), message_info(&u1, &[]), ExecuteMsg::Claim {}).unwrap_err(),
        ContractError::NothingToClaim
    ));
    assert!(matches!(
        execute(deps2.as_mut(), mock_env(), message_info(&u2, &[]), ExecuteMsg::Claim {}).unwrap_err(),
        ContractError::NothingToClaim
    ));
    assert!(matches!(
        execute(deps2.as_mut(), mock_env(), message_info(&u3, &[]), ExecuteMsg::Claim {}).unwrap_err(),
        ContractError::NothingToClaim
    ));
}

#[test]
fn test_close_position_on_delisted_market_releases_reserved_margin() {
    let mut deps = inj_deps();
    let owner = addr20("owner");
    let trader = addr20("trader");
    let delisted_market = MARKET_1;
    let other_market = "0xmarket_other";

    instantiate(
        deps.as_mut(),
        mock_env(),
        message_info(&owner, &[]),
        InstantiateMsg {
            owner: owner.to_string(),
            seed_denom: SEED.to_string(),
            treasury_denom: USDT.to_string(),
            trader: Some(trader.to_string()),
            allowed_markets: vec![delisted_market.to_string()],
            max_leverage: Some(Decimal::from_ratio(3u128, 1u128)),
            max_position_size: Some(Uint128::new(10_000_000)),
            max_exposure_bps: Some(5_000),
            max_total_margin_bps: None,
            max_position_margin: Some(Uint128::new(5_000_000)),
            cooldown_seconds: Some(0),
        },
    )
    .unwrap();

    // 1. Deposit treasury funds
    execute(
        deps.as_mut(),
        mock_env(),
        message_info(&owner, &coins(10_000_000, USDT)),
        ExecuteMsg::DepositTreasury {},
    )
    .unwrap();

    assert_eq!(
        TREASURY_BALANCE.load(&deps.storage).unwrap(),
        Uint128::new(10_000_000)
    );
    assert_eq!(
        RESERVED_MARGIN.load(&deps.storage).unwrap(),
        Uint128::zero()
    );

    // 2. Trader opens position on allowed market
    let open_margin = Uint128::new(2_000_000);
    execute(
        deps.as_mut(),
        mock_env(),
        message_info(&trader, &[]),
        ExecuteMsg::OpenPosition {
            market_id: delisted_market.to_string(),
            is_buy: true,
            quantity: "500".to_string(),
            price: "20".to_string(),
            margin: open_margin.to_string(),
            leverage: Some("2".to_string()),
        },
    )
    .unwrap();

    assert_eq!(
        RESERVED_MARGIN.load(&deps.storage).unwrap(),
        open_margin
    );
    assert_eq!(
        TREASURY_BALANCE.load(&deps.storage).unwrap(),
        Uint128::new(8_000_000)
    );
    assert!(POSITIONS.has(&deps.storage, delisted_market));

    // 3. Owner removes the market via UpdateConfig
    execute(
        deps.as_mut(),
        mock_env(),
        message_info(&owner, &[]),
        ExecuteMsg::UpdateConfig {
            max_leverage: None,
            max_position_size: None,
            max_exposure_bps: None,
            max_total_margin_bps: None,
            max_position_margin: None,
            cooldown_seconds: None,
            allowed_markets: Some(vec![other_market.to_string()]),
        },
    )
    .unwrap();

    let config: ConfigResponse = from_json(
        query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap(),
    )
    .unwrap();
    assert!(!config.allowed_markets.iter().any(|m| m == delisted_market));
    assert_eq!(config.allowed_markets, vec![other_market.to_string()]);

    // Trader cannot open a new position on the delisted market
    let err = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&trader, &[]),
        ExecuteMsg::OpenPosition {
            market_id: delisted_market.to_string(),
            is_buy: true,
            quantity: "100".to_string(),
            price: "20".to_string(),
            margin: "100000".to_string(),
            leverage: None,
        },
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::MarketNotAllowed { .. }));

    // 4. Trader closes existing position on delisted market successfully
    let res = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&trader, &[]),
        ExecuteMsg::ClosePosition {
            market_id: delisted_market.to_string(),
            quantity: None,
        },
    )
    .unwrap();

    let attr = |key: &str| res.attributes.iter().find(|a| a.key == key).map(|a| a.value.as_str());
    assert_eq!(attr("action"), Some("close_position"));
    assert_eq!(attr("trader"), Some(trader.as_str()));
    assert_eq!(attr("market_id"), Some(delisted_market));

    // 5. Reserved margin is released and treasury balance restored
    assert_eq!(
        RESERVED_MARGIN.load(&deps.storage).unwrap(),
        Uint128::zero()
    );
    assert_eq!(
        TREASURY_BALANCE.load(&deps.storage).unwrap(),
        Uint128::new(10_000_000)
    );
    assert!(!POSITIONS.has(&deps.storage, delisted_market));

    let state: TreasuryStateResponse = from_json(
        query(deps.as_ref(), mock_env(), QueryMsg::TreasuryState {}).unwrap(),
    )
    .unwrap();
    assert_eq!(state.reserved_margin, Uint128::zero());
    assert_eq!(state.balance, Uint128::new(10_000_000));
}

#[test]
fn close_releases_full_margin_and_restores_exposure_capacity() {
    let (mut deps, admin) = setup();

    execute(
        deps.as_mut(),
        mock_env(),
        message_info(&admin, &coins(10_000_000, USDT)),
        ExecuteMsg::DepositTreasury {},
    )
    .unwrap();

    // 50% of 10_000_000 is the exposure cap. Fill it with one position.
    execute(
        deps.as_mut(),
        mock_env(),
        message_info(&admin, &[]),
        ExecuteMsg::OpenPosition {
            market_id: MARKET_1.to_string(),
            is_buy: true,
            quantity: "1000".to_string(),
            price: "1".to_string(),
            margin: "5000000".to_string(),
            leverage: None,
        },
    )
    .unwrap();

    execute(
        deps.as_mut(),
        mock_env(),
        message_info(&admin, &[]),
        ExecuteMsg::UpdateConfig {
            max_leverage: None,
            max_position_size: None,
            max_exposure_bps: None,
            max_total_margin_bps: None,
            max_position_margin: None,
            cooldown_seconds: None,
            allowed_markets: Some(vec![MARKET_1.to_string(), "0xmarket_other".to_string()]),
        },
    )
    .unwrap();

    let err = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&admin, &[]),
        ExecuteMsg::OpenPosition {
            market_id: "0xmarket_other".to_string(),
            is_buy: true,
            quantity: "100".to_string(),
            price: "1".to_string(),
            margin: "1000000".to_string(),
            leverage: None,
        },
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::ExposureLimitExceeded));

    execute(
        deps.as_mut(),
        mock_env(),
        message_info(&admin, &[]),
        ExecuteMsg::ClosePosition {
            market_id: MARKET_1.to_string(),
            quantity: None,
        },
    )
    .unwrap();

    assert_eq!(RESERVED_MARGIN.load(&deps.storage).unwrap(), Uint128::zero());
    assert_eq!(
        TREASURY_BALANCE.load(&deps.storage).unwrap(),
        Uint128::new(10_000_000)
    );
    assert_eq!(REALIZED_PNL_NET.load(&deps.storage).unwrap(), Int128::zero());
    assert!(!POSITIONS.has(&deps.storage, MARKET_1));

    // Capacity is usable again once the reserved margin is released.
    execute(
        deps.as_mut(),
        mock_env(),
        message_info(&admin, &[]),
        ExecuteMsg::OpenPosition {
            market_id: "0xmarket_other".to_string(),
            is_buy: false,
            quantity: "1000".to_string(),
            price: "1".to_string(),
            margin: "5000000".to_string(),
            leverage: None,
        },
    )
    .unwrap();
    assert_eq!(
        RESERVED_MARGIN.load(&deps.storage).unwrap(),
        Uint128::new(5_000_000)
    );
}

#[test]
fn close_nonexistent_position_fails() {
    let (mut deps, admin) = setup();

    let err = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&admin, &[]),
        ExecuteMsg::ClosePosition {
            market_id: MARKET_1.to_string(),
            quantity: None,
        },
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::PositionNotFound { .. }));
    assert_eq!(RESERVED_MARGIN.load(&deps.storage).unwrap(), Uint128::zero());
}

#[test]
fn duplicate_open_on_same_market_fails() {
    let (mut deps, admin) = setup();

    execute(
        deps.as_mut(),
        mock_env(),
        message_info(&admin, &coins(10_000_000, USDT)),
        ExecuteMsg::DepositTreasury {},
    )
    .unwrap();

    execute(
        deps.as_mut(),
        mock_env(),
        message_info(&admin, &[]),
        ExecuteMsg::OpenPosition {
            market_id: MARKET_1.to_string(),
            is_buy: true,
            quantity: "1000".to_string(),
            price: "1000".to_string(),
            margin: "1000000".to_string(),
            leverage: None,
        },
    )
    .unwrap();

    let err = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&admin, &[]),
        ExecuteMsg::OpenPosition {
            market_id: MARKET_1.to_string(),
            is_buy: false,
            quantity: "500".to_string(),
            price: "1000".to_string(),
            margin: "500000".to_string(),
            leverage: None,
        },
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::PositionAlreadyOpen { .. }));
    assert_eq!(
        RESERVED_MARGIN.load(&deps.storage).unwrap(),
        Uint128::new(1_000_000)
    );
    assert_eq!(
        TREASURY_BALANCE.load(&deps.storage).unwrap(),
        Uint128::new(9_000_000)
    );
}

#[test]
fn open_position_rejects_zero_price() {
    let (mut deps, admin) = setup();

    execute(
        deps.as_mut(),
        mock_env(),
        message_info(&admin, &coins(10_000_000, USDT)),
        ExecuteMsg::DepositTreasury {},
    )
    .unwrap();

    // quantity * 0 / margin would otherwise skip the notional leverage check.
    let err = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&admin, &[]),
        ExecuteMsg::OpenPosition {
            market_id: MARKET_1.to_string(),
            is_buy: true,
            quantity: "10000000".to_string(),
            price: "0".to_string(),
            margin: "1000000".to_string(),
            leverage: None,
        },
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::InvalidPrice));
    assert_eq!(RESERVED_MARGIN.load(&deps.storage).unwrap(), Uint128::zero());
    assert!(!POSITIONS.has(&deps.storage, MARKET_1));
}

#[test]
fn partial_close_is_rejected_and_leaves_margin_reserved() {
    let (mut deps, admin) = setup();

    execute(
        deps.as_mut(),
        mock_env(),
        message_info(&admin, &coins(10_000_000, USDT)),
        ExecuteMsg::DepositTreasury {},
    )
    .unwrap();

    execute(
        deps.as_mut(),
        mock_env(),
        message_info(&admin, &[]),
        ExecuteMsg::OpenPosition {
            market_id: MARKET_1.to_string(),
            is_buy: true,
            quantity: "1000".to_string(),
            price: "1000".to_string(),
            margin: "1000000".to_string(),
            leverage: None,
        },
    )
    .unwrap();

    let err = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&admin, &[]),
        ExecuteMsg::ClosePosition {
            market_id: MARKET_1.to_string(),
            quantity: Some("500".to_string()),
        },
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::PartialCloseNotSupported));
    assert!(POSITIONS.has(&deps.storage, MARKET_1));
    assert_eq!(
        RESERVED_MARGIN.load(&deps.storage).unwrap(),
        Uint128::new(1_000_000)
    );
    assert_eq!(
        TREASURY_BALANCE.load(&deps.storage).unwrap(),
        Uint128::new(9_000_000)
    );
}




#[test]
fn test_close_position_emits_injective_derivative_and_withdraw_messages() {
    let (mut deps, admin) = setup();

    execute(
        deps.as_mut(),
        mock_env(),
        message_info(&admin, &coins(10_000_000, USDT)),
        ExecuteMsg::DepositTreasury {},
    )
    .unwrap();

    // 1. Open Long position
    let open_res = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&admin, &[]),
        ExecuteMsg::OpenPosition {
            market_id: MARKET_1.to_string(),
            is_buy: true,
            quantity: "1000".to_string(),
            price: "1000".to_string(),
            margin: "1000000".to_string(),
            leverage: None,
        },
    )
    .unwrap();
    assert_eq!(open_res.messages.len(), 2);

    // 2. Close Long position -> should emit Sell order + Withdraw
    let close_res = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&admin, &[]),
        ExecuteMsg::ClosePosition {
            market_id: MARKET_1.to_string(),
            quantity: Some("1000".to_string()),
        },
    )
    .unwrap();
    assert_eq!(close_res.messages.len(), 2);
    assert_eq!(close_res.messages[0].id, REPLY_CLOSE_POSITION);

    #[allow(deprecated)]
    match &close_res.messages[0].msg {
        CosmosMsg::Stargate { type_url, .. } => {
            assert_eq!(
                type_url,
                "/injective.exchange.v1beta1.MsgCreateDerivativeMarketOrder"
            );
        }
        _ => panic!("expected MsgCreateDerivativeMarketOrder stargate message"),
    }

    #[allow(deprecated)]
    match &close_res.messages[1].msg {
        CosmosMsg::Stargate { type_url, .. } => {
            assert_eq!(type_url, "/injective.exchange.v1beta1.MsgWithdraw");
        }
        _ => panic!("expected MsgWithdraw stargate message"),
    }
}

#[test]
#[allow(deprecated)]
fn test_reply_handlers_success_and_error() {
    let mut deps = inj_deps();
    let env = mock_env();

    // 1. REPLY_OPEN_POSITION success
    let reply_msg = Reply {
        id: REPLY_OPEN_POSITION,
        payload: cosmwasm_std::Binary::default(),
        gas_used: 10_000,
        result: SubMsgResult::Ok(SubMsgResponse {
            events: vec![],
            data: None,
            msg_responses: vec![],
        }),
    };
    let res = reply(deps.as_mut(), env.clone(), reply_msg).unwrap();
    assert_eq!(res.attributes[0].value, "reply_open_position");

    // 2. REPLY_CLOSE_POSITION success
    let reply_msg = Reply {
        id: REPLY_CLOSE_POSITION,
        payload: cosmwasm_std::Binary::default(),
        gas_used: 10_000,
        result: SubMsgResult::Ok(SubMsgResponse {
            events: vec![],
            data: None,
            msg_responses: vec![],
        }),
    };
    let res = reply(deps.as_mut(), env.clone(), reply_msg).unwrap();
    assert_eq!(res.attributes[0].value, "reply_close_position");

    // 3. REPLY_OPEN_POSITION error
    let reply_err = Reply {
        id: REPLY_OPEN_POSITION,
        payload: cosmwasm_std::Binary::default(),
        gas_used: 10_000,
        result: SubMsgResult::Err("insufficient margin in subaccount".to_string()),
    };
    let err = reply(deps.as_mut(), env.clone(), reply_err).unwrap_err();
    assert!(err.to_string().contains("open position execution failed"));

    // 4. REPLY_CLOSE_POSITION error
    let reply_err = Reply {
        id: REPLY_CLOSE_POSITION,
        payload: cosmwasm_std::Binary::default(),
        gas_used: 10_000,
        result: SubMsgResult::Err("market order slippage exceeded".to_string()),
    };
    let err = reply(deps.as_mut(), env.clone(), reply_err).unwrap_err();
    assert!(err.to_string().contains("close position execution failed"));

    // 5. Unknown reply id
    let reply_unknown = Reply {
        id: 999,
        payload: cosmwasm_std::Binary::default(),
        gas_used: 10_000,
        result: SubMsgResult::Ok(SubMsgResponse {
            events: vec![],
            data: None,
            msg_responses: vec![],
        }),
    };
    let err = reply(deps.as_mut(), env, reply_unknown).unwrap_err();
    assert!(err.to_string().contains("unknown reply id: 999"));
}
