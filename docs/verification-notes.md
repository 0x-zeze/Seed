> **Notice:** These are internal checks and developer verification notes, NOT an independent third-party audit.

# Seed Vault — Internal Verification Output

Combined execution output for role boundary verification, risk parameter enforcement, circuit breaker mechanics, and test suite execution.

---

## 1. Role Boundary Verification (Trader & Two-Step Ownership)
```bash
grep -rn "trader\|ProposeOwner\|AcceptOwner" contracts/seed-vault/src | head -30
```
```text
contracts/seed-vault/src/contract.rs:11:    add_i128, add_u128, coin_u128, ensure_not_paused, ensure_owner, ensure_trader_or_owner,
contracts/seed-vault/src/contract.rs:72:        trader: msg
contracts/seed-vault/src/contract.rs:73:            .trader
contracts/seed-vault/src/contract.rs:75:            .map(|trader| deps.api.addr_validate(trader))
contracts/seed-vault/src/contract.rs:118:        ExecuteMsg::ProposeOwner { new_owner } => propose_owner(deps, info, new_owner),
contracts/seed-vault/src/contract.rs:119:        ExecuteMsg::AcceptOwner {} => accept_owner(deps, info),
contracts/seed-vault/src/contract.rs:120:        ExecuteMsg::UpdateTrader { new_trader } => update_trader(deps, info, new_trader),
contracts/seed-vault/src/contract.rs:366:fn update_trader(
contracts/seed-vault/src/contract.rs:369:    new_trader: Option<String>,
contracts/seed-vault/src/contract.rs:374:    let trader_addr = new_trader
contracts/seed-vault/src/contract.rs:376:        .map(|trader| deps.api.addr_validate(trader))
contracts/seed-vault/src/contract.rs:379:    config.trader = trader_addr.clone();
contracts/seed-vault/src/contract.rs:382:    let mut res = Response::new().add_attribute("action", "trader_updated");
contracts/seed-vault/src/contract.rs:383:    if let Some(trader) = trader_addr {
contracts/seed-vault/src/contract.rs:384:        res = res.add_attribute("trader", trader);
contracts/seed-vault/src/contract.rs:386:        res = res.add_attribute("trader", "none");
contracts/seed-vault/src/contract.rs:564:    ensure_trader_or_owner(&config, &info.sender)?;
contracts/seed-vault/src/contract.rs:670:        .add_attribute("trader", &info.sender)
contracts/seed-vault/src/contract.rs:687:    ensure_trader_or_owner(&config, &info.sender)?;
contracts/seed-vault/src/contract.rs:708:        .add_attribute("trader", info.sender)
contracts/seed-vault/src/contract.rs:744:        trader: config.trader.map(|trader| trader.into_string()),
contracts/seed-vault/src/helpers.rs:15:pub fn ensure_trader_or_owner(config: &Config, sender: &Addr) -> Result<(), ContractError> {
contracts/seed-vault/src/helpers.rs:16:    if sender == config.owner || config.trader.as_ref() == Some(sender) {
contracts/seed-vault/src/msg.rs:11:    /// Optional trader. Empty means only the owner can manage positions.
contracts/seed-vault/src/msg.rs:12:    pub trader: Option<String>,
contracts/seed-vault/src/msg.rs:54:    ProposeOwner {
contracts/seed-vault/src/msg.rs:57:    AcceptOwner {},
contracts/seed-vault/src/msg.rs:59:        new_trader: Option<String>,
contracts/seed-vault/src/msg.rs:115:    pub trader: Option<String>,
contracts/seed-vault/src/state.rs:17:    pub trader: Option<Addr>,
```

---

## 2. On-Chain Risk Limits & Engine
```bash
grep -rn "allowed_markets\|max_exposure\|cooldown\|max_position" contracts/seed-vault/src | head -30
```
```text
contracts/seed-vault/src/contract.rs:47:    let max_exposure_bps = msg
contracts/seed-vault/src/contract.rs:48:        .max_exposure_bps
contracts/seed-vault/src/contract.rs:51:    if max_exposure_bps > 10_000 {
contracts/seed-vault/src/contract.rs:52:        return Err(StdError::msg("max_exposure_bps cannot exceed 10000").into());
contracts/seed-vault/src/contract.rs:55:    let max_position_margin = msg.max_position_margin.unwrap_or(Uint128::MAX);
contracts/seed-vault/src/contract.rs:56:    if max_position_margin.is_zero() {
contracts/seed-vault/src/contract.rs:57:        return Err(StdError::msg("max_position_margin must be greater than zero").into());
contracts/seed-vault/src/contract.rs:60:    let max_position_size = msg.max_position_size.unwrap_or(Uint128::MAX);
contracts/seed-vault/src/contract.rs:61:    if max_position_size.is_zero() {
contracts/seed-vault/src/contract.rs:62:        return Err(StdError::msg("max_position_size must be greater than zero").into());
contracts/seed-vault/src/contract.rs:65:    let cooldown_seconds = msg.cooldown_seconds.unwrap_or(0);
contracts/seed-vault/src/contract.rs:77:        allowed_markets: msg.allowed_markets,
contracts/seed-vault/src/contract.rs:79:        max_position_size,
contracts/seed-vault/src/contract.rs:80:        max_exposure_bps,
contracts/seed-vault/src/contract.rs:81:        max_position_margin,
contracts/seed-vault/src/contract.rs:82:        cooldown_seconds,
contracts/seed-vault/src/contract.rs:123:            max_position_size,
contracts/seed-vault/src/contract.rs:124:            max_exposure_bps,
contracts/seed-vault/src/contract.rs:126:            max_position_margin,
contracts/seed-vault/src/contract.rs:127:            cooldown_seconds,
contracts/seed-vault/src/contract.rs:128:            allowed_markets,
contracts/seed-vault/src/contract.rs:133:            max_position_size,
contracts/seed-vault/src/contract.rs:134:            max_exposure_bps,
contracts/seed-vault/src/contract.rs:136:            max_position_margin,
contracts/seed-vault/src/contract.rs:137:            cooldown_seconds,
contracts/seed-vault/src/contract.rs:138:            allowed_markets,
contracts/seed-vault/src/contract.rs:397:    max_position_size: Option<Uint128>,
contracts/seed-vault/src/contract.rs:398:    max_exposure_bps: Option<u16>,
contracts/seed-vault/src/contract.rs:400:    max_position_margin: Option<Uint128>,
contracts/seed-vault/src/contract.rs:401:    cooldown_seconds: Option<u64>,
```

---

## 3. Circuit Breaker (`paused` / `Paused`)
```bash
grep -rn "paused\|Paused" contracts/seed-vault/src | head -30
```
```text
contracts/seed-vault/src/contract.rs:11:    add_i128, add_u128, coin_u128, ensure_not_paused, ensure_owner, ensure_trader_or_owner,
contracts/seed-vault/src/contract.rs:71:        paused: false,
contracts/seed-vault/src/contract.rs:116:        ExecuteMsg::Pause {} => set_paused(deps, info, true),
contracts/seed-vault/src/contract.rs:117:        ExecuteMsg::Unpause {} => set_paused(deps, info, false),
contracts/seed-vault/src/contract.rs:165:    ensure_not_paused(&config)?;
contracts/seed-vault/src/contract.rs:249:    ensure_not_paused(&config)?;
contracts/seed-vault/src/contract.rs:308:fn set_paused(
contracts/seed-vault/src/contract.rs:311:    paused: bool,
contracts/seed-vault/src/contract.rs:315:    config.paused = paused;
contracts/seed-vault/src/contract.rs:319:        .add_attribute("action", if paused { "paused" } else { "unpaused" })
contracts/seed-vault/src/contract.rs:472:/// Pays settled yield. Open while paused so users can always collect what they earned.
contracts/seed-vault/src/contract.rs:565:    ensure_not_paused(&config)?;
contracts/seed-vault/src/contract.rs:743:        paused: config.paused,
contracts/seed-vault/src/error.rs:12:    #[error("contract is paused")]
contracts/seed-vault/src/error.rs:13:    Paused,
contracts/seed-vault/src/helpers.rs:23:pub fn ensure_not_paused(config: &Config) -> Result<(), ContractError> {
contracts/seed-vault/src/helpers.rs:24:    if config.paused {
contracts/seed-vault/src/helpers.rs:25:        return Err(ContractError::Paused);
contracts/seed-vault/src/msg.rs:114:    pub paused: bool,
contracts/seed-vault/src/state.rs:15:    pub paused: bool,
contracts/seed-vault/src/testing.rs:56:    assert!(!config.paused);
contracts/seed-vault/src/testing.rs:69:    assert!(!stored.paused);
contracts/seed-vault/src/testing.rs:496:    assert!(CONFIG.load(&deps.storage).unwrap().paused);
contracts/seed-vault/src/testing.rs:505:    assert!(matches!(stake_err, ContractError::Paused));
contracts/seed-vault/src/testing.rs:507:    // DepositTreasury is allowed while paused so owner can recapitalize
contracts/seed-vault/src/testing.rs:527:    assert!(matches!(distribute_err, ContractError::Paused));
contracts/seed-vault/src/testing.rs:547:    assert!(!CONFIG.load(&deps.storage).unwrap().paused);
contracts/seed-vault/src/testing.rs:1011:    // OpenPosition fails when paused
contracts/seed-vault/src/testing.rs:1026:    assert!(matches!(err, ContractError::Paused));
contracts/seed-vault/src/testing.rs:1028:    // ClosePosition succeeds even when paused
```

---

## 4. Test Suite Execution (Unit Tests & Invariants)
```bash
cargo test --manifest-path contracts/seed-vault/Cargo.toml --lib 2>&1 | tail -30
```
```text
test testing::test_action_events_emitted_on_positions_distribute_and_claim ... ok
test testing::test_action_events_emitted_on_stake_unstake_and_treasury ... ok
test testing::test_cooldown_active_rejection_and_resolution ... ok
test testing::test_claim_twice_does_not_pay_twice ... ok
test testing::test_distribute_with_total_staked_zero ... ok
test testing::test_exhaustive_risk_limit_rejections_market_size_leverage ... ok
test testing::test_exhaustive_risk_limit_rejections_treasury_exposure_cooldown_paused ... ok
test testing::test_exposure_limit_exceeded_rejection ... ok
test testing::test_instantiate_and_update_validation ... ok
test testing::test_market_not_allowed_rejection ... ok
test testing::test_leverage_too_high_rejection ... ok
test testing::test_multiple_stakers_multiple_rounds_exact_payouts_and_dust_accounting ... ok
test testing::test_open_position_success_and_risk_config_query ... ok
test testing::test_position_too_large_rejection ... ok
test testing::test_new_queries_treasury_state_open_positions_total_staked ... ok
test testing::test_stake_before_and_after_distribute_later_staker_gets_no_old_profit ... ok
test testing::test_treasury_loss_staked_balances_unchanged_and_unstake_succeeds ... ok
test testing::test_role_permissions_non_trader_cannot_open_and_trader_cannot_call_owner_messages ... ok
test testing::test_unstake_while_paused ... ok
test testing::test_wrong_address_cannot_accept_ownership_transfer_exhaustive ... ok
test testing::trader_can_open_and_close_positions ... ok
test testing::trader_cannot_distribute_or_deposit_treasury_but_owner_can ... ok
test testing::trader_cannot_execute_owner_administrative_functions ... ok
test testing::unstake_keeps_pending_yield_for_a_later_claim ... ok
test testing::unstake_more_than_staked_fails ... ok
test testing::update_config_by_owner_and_unauthorized_checks ... ok
test testing::update_trader_by_owner_and_unauthorized_checks ... ok

test result: ok. 58 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s
```

