use cosmwasm_std::testing::{message_info, mock_env};
use cosmwasm_std::{coin, coins, from_json, BankMsg, CosmosMsg, Decimal, Int128, Uint128};
use injective_cosmwasm::mock_dependencies;

use crate::addresses::addr20;
use crate::contract::{execute, instantiate, query};
use crate::error::ContractError;
use crate::helpers::pending_yield;
use crate::msg::{
    AmountResponse, ConfigResponse, DistributionInfoResponse, ExecuteMsg, InstantiateMsg,
    PositionsResponse, QueryMsg, UserInfoResponse, UserStakeResponse,
};
use crate::state::{
    CONFIG, GLOBAL_INDEX, LAST_DISTRIBUTION, PENDING_ADMIN, REALIZED_PNL_NET, RESERVED_MARGIN,
    REWARDS_RESERVED, TOTAL_DISTRIBUTED, TOTAL_STAKED, TREASURY_BALANCE, USER_CLAIMED, USER_INDEX,
    USER_PENDING, USER_STAKE,
};

const SEED: &str = "factory/inj1seed/seed";
const USDT: &str = "peggy0xusdt";
const MARKET_1: &str = "0xmarket1";

#[test]
fn instantiate_stores_config_and_zero_counters() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let admin = addr20("admin");
    let info = message_info(&admin, &[]);

    instantiate(
        deps.as_mut(),
        env.clone(),
        info,
        InstantiateMsg {
            admin: admin.to_string(),
            seed_denom: SEED.to_string(),
            treasury_denom: USDT.to_string(),
            keeper: None,
            max_leverage: Decimal::from_ratio(3u128, 1u128),
            max_position_size: Uint128::new(10_000_000),
            max_total_margin_bps: 5_000,
            allowed_markets: vec![MARKET_1.to_string()],
        },
    )
    .unwrap();

    let raw = query(deps.as_ref(), env, QueryMsg::Config {}).unwrap();
    let config: ConfigResponse = from_json(raw).unwrap();
    assert_eq!(config.admin, admin.as_str());
    assert_eq!(config.seed_token, SEED);
    assert_eq!(config.treasury_denom, USDT);
    assert!(!config.paused);
    assert_eq!(config.keeper, None);
    assert_eq!(config.max_leverage, "3");
    assert_eq!(config.max_position_size, Uint128::new(10_000_000));
    assert_eq!(config.max_total_margin_bps, 5_000);
    assert_eq!(config.allowed_markets, vec![MARKET_1.to_string()]);

    let stored = CONFIG.load(&deps.storage).unwrap();
    assert_eq!(stored.admin, admin);
    assert_eq!(stored.seed_denom, SEED);
    assert_eq!(stored.treasury_denom, USDT);
    assert!(!stored.paused);
    assert_eq!(stored.keeper, None);
    assert_eq!(stored.max_leverage, Decimal::from_ratio(3u128, 1u128));
    assert_eq!(stored.max_position_size, Uint128::new(10_000_000));
    assert_eq!(stored.max_total_margin_bps, 5_000);
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
        injective_cosmwasm::WasmMockQuerier,
        injective_cosmwasm::InjectiveQueryWrapper,
    >,
    cosmwasm_std::Addr,
) {
    let mut deps = mock_dependencies();
    let admin = addr20("admin");
    instantiate(
        deps.as_mut(),
        mock_env(),
        message_info(&admin, &[]),
        InstantiateMsg {
            admin: admin.to_string(),
            seed_denom: SEED.to_string(),
            treasury_denom: USDT.to_string(),
            keeper: None,
            max_leverage: Decimal::from_ratio(3u128, 1u128),
            max_position_size: Uint128::new(10_000_000),
            max_total_margin_bps: 5_000,
            allowed_markets: vec![MARKET_1.to_string()],
        },
    )
    .unwrap();
    (deps, admin)
}

fn stake(
    deps: &mut cosmwasm_std::DepsMut<injective_cosmwasm::InjectiveQueryWrapper>,
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
    deps: &mut cosmwasm_std::DepsMut<injective_cosmwasm::InjectiveQueryWrapper>,
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
    deps: &mut cosmwasm_std::DepsMut<injective_cosmwasm::InjectiveQueryWrapper>,
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
fn non_admin_cannot_deposit_treasury() {
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
fn pause_blocks_stake_distribute_and_deposit_but_not_unstake() {
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

    let deposit_err = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&admin, &coins(1, USDT)),
        ExecuteMsg::DepositTreasury {},
    )
    .unwrap_err();
    assert!(matches!(deposit_err, ContractError::Paused));

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
fn admin_transfer_requires_acceptance() {
    let (mut deps, admin) = setup();
    let next = addr20("next");
    let stranger = addr20("stranger");

    execute(
        deps.as_mut(),
        mock_env(),
        message_info(&admin, &[]),
        ExecuteMsg::ProposeAdmin {
            new_admin: next.to_string(),
        },
    )
    .unwrap();
    assert_eq!(PENDING_ADMIN.load(&deps.storage).unwrap(), next);
    assert_eq!(CONFIG.load(&deps.storage).unwrap().admin, admin);

    let early = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&stranger, &[]),
        ExecuteMsg::AcceptAdmin {},
    )
    .unwrap_err();
    assert!(matches!(early, ContractError::Unauthorized));

    execute(
        deps.as_mut(),
        mock_env(),
        message_info(&next, &[]),
        ExecuteMsg::AcceptAdmin {},
    )
    .unwrap();

    assert_eq!(CONFIG.load(&deps.storage).unwrap().admin, next);
    assert!(PENDING_ADMIN.may_load(&deps.storage).unwrap().is_none());

    let old = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&admin, &[]),
        ExecuteMsg::Pause {},
    )
    .unwrap_err();
    assert!(matches!(old, ContractError::Unauthorized));
}

fn distribute(
    deps: &mut cosmwasm_std::DepsMut<injective_cosmwasm::InjectiveQueryWrapper>,
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
    deps: &mut cosmwasm_std::DepsMut<injective_cosmwasm::InjectiveQueryWrapper>,
    user: &cosmwasm_std::Addr,
) -> cosmwasm_std::Response<injective_cosmwasm::InjectiveMsgWrapper> {
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
    assert!(matches!(err, ContractError::RiskLimit { .. }));

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
    assert!(matches!(err, ContractError::RiskLimit { .. }));

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
    assert!(matches!(err, ContractError::RiskLimit { .. }));

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

    // 7. Exceeding max_total_margin_bps (50% of 10,000,000 = 5,000,000 max reserved)
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
    assert!(matches!(err, ContractError::RiskLimit { .. }));

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

    assert_eq!(res.messages.len(), 0);
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
fn keeper_can_open_and_close_positions() {
    let mut deps = mock_dependencies();
    let admin = addr20("admin");
    let keeper = addr20("keeper");
    let stranger = addr20("stranger");

    instantiate(
        deps.as_mut(),
        mock_env(),
        message_info(&admin, &[]),
        InstantiateMsg {
            admin: admin.to_string(),
            seed_denom: SEED.to_string(),
            treasury_denom: USDT.to_string(),
            keeper: Some(keeper.to_string()),
            max_leverage: Decimal::from_ratio(3u128, 1u128),
            max_position_size: Uint128::new(10_000_000),
            max_total_margin_bps: 8_000,
            allowed_markets: vec![MARKET_1.to_string()],
        },
    )
    .unwrap();

    // Deposit treasury
    execute(
        deps.as_mut(),
        mock_env(),
        message_info(&admin, &coins(5_000_000, USDT)),
        ExecuteMsg::DepositTreasury {},
    )
    .unwrap();

    // Keeper opens position successfully
    execute(
        deps.as_mut(),
        mock_env(),
        message_info(&keeper, &[]),
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

    // Keeper closes position successfully
    let res = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&keeper, &[]),
        ExecuteMsg::ClosePosition {
            market_id: MARKET_1.to_string(),
            quantity: Some("500".to_string()),
        },
    )
    .unwrap();
    assert_eq!(res.attributes[0].value, "close_position");
}
#[test]
fn keeper_can_distribute() {
    let mut deps = mock_dependencies();
    let admin = addr20("admin");
    let keeper = addr20("keeper");
    let stranger = addr20("stranger");
    let user = addr20("staker");

    instantiate(
        deps.as_mut(),
        mock_env(),
        message_info(&admin, &[]),
        InstantiateMsg {
            admin: admin.to_string(),
            seed_denom: SEED.to_string(),
            treasury_denom: USDT.to_string(),
            keeper: Some(keeper.to_string()),
            max_leverage: Decimal::from_ratio(3u128, 1u128),
            max_position_size: Uint128::new(10_000_000),
            max_total_margin_bps: 8_000,
            allowed_markets: vec![MARKET_1.to_string()],
        },
    )
    .unwrap();

    // Deposit treasury
    execute(
        deps.as_mut(),
        mock_env(),
        message_info(&admin, &coins(5_000_000, USDT)),
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
        message_info(&admin, &[]),
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

    // Keeper distributes successfully
    let res = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&keeper, &[]),
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
    res: &cosmwasm_std::Response<injective_cosmwasm::InjectiveMsgWrapper>,
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
    deps: &cosmwasm_std::Deps<injective_cosmwasm::InjectiveQueryWrapper>,
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
