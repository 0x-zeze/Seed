# Seed Vault — Technical Specifications & Architecture

Comprehensive technical specifications, message interfaces, state schemas, and execution logic for the Seed Vault CosmWasm contract on Injective.

---

## Table of Contents
1. [Core Architecture & Accounting](#1-core-architecture--accounting)
2. [Stake (`ExecuteMsg::Stake`)](#2-stake-executemsgstake)
3. [Unstake (`ExecuteMsg::Unstake`)](#3-unstake-executemsgunstake)
4. [Treasury Deposits (`ExecuteMsg::DepositTreasury`)](#4-treasury-deposits-executemsgdeposittreasury)
5. [Position Management (`ExecuteMsg::OpenPosition`, `ClosePosition`)](#5-position-management-executemsgopenposition-closeposition)
6. [Yield Distribution (`ExecuteMsg::Distribute`)](#6-yield-distribution-executemsgdistribute)
7. [Yield Claiming (`ExecuteMsg::Claim`)](#7-yield-claiming-executemsgclaim)
8. [Governance & Administrative Handlers](#8-governance--administrative-handlers)
9. [Query Interface & Response Schemas](#9-query-interface--response-schemas)
10. [On-Chain Invariants & Solvency Guarantees](#10-on-chain-invariants--solvency-guarantees)

---

## 1. Core Architecture & Accounting

The Seed Vault operates three segregated asset and accounting layers:
```text
Total Vault Treasury Balance >= TREASURY_BALANCE (Free) + RESERVED_MARGIN (In-Flight) + REWARDS_RESERVED (Staker Liability)
```

1. **Staker Principal Pool**: `$SEED` tokens deposited by stakers, held strictly 1:1. Never exposed to trading risk or liquidations.
2. **Treasury Trading Pool**: USDT funds deployed as margin into Injective iAsset perpetual derivatives.
3. **Segregated Reward Liability (`REWARDS_RESERVED`)**: Realized net trading profit moved from free treasury into staker rewards. Completely immune to subsequent trading drawdowns.

---

## 2. Stake (`ExecuteMsg::Stake`)

Stakers deposit `$SEED` tokens to receive continuous pro-rata rights to trading yield.

### Message
```rust
ExecuteMsg::Stake {}
```

### Execution Logic
1. Validate contract is not paused (`ensure_not_paused`).
2. Validate transferred funds contain exact `config.seed_token` denom with non-zero amount.
3. Lazily settle the user's pending yield via `settle(storage, &sender)`.
4. If first-time staker, set `USER_INDEX[sender] = GLOBAL_INDEX` so prior distributions cannot be claimed.
5. Increase `USER_STAKE[sender] += amount` and `TOTAL_STAKED += amount`.
6. Emit `seed_staked` action event with `user` and `amount`.

---

## 3. Unstake (`ExecuteMsg::Unstake`)

Stakers withdraw `$SEED` tokens from the contract.

### Message
```rust
ExecuteMsg::Unstake { amount: Uint128 }
```

### Execution Logic
1. **Unconditional Exit**: Callable even while the contract is paused.
2. Require `amount > 0` and `USER_STAKE[sender] >= amount`.
3. Lazily settle current earnings via `settle(storage, &sender)` before modifying stake balances.
4. Deduct `USER_STAKE[sender] -= amount` and `TOTAL_STAKED -= amount`.
5. Dispatch `BankMsg::Send` returning `$SEED` tokens to the staker.
6. Emit `seed_unstaked` action event with `user` and `amount`. Accrued yield remains safely in `USER_PENDING` for future claims.

---

## 4. Treasury Deposits (`ExecuteMsg::DepositTreasury`)

Owner deposits USDT to fund the trading margin pool.

### Message
```rust
ExecuteMsg::DepositTreasury {}
```

### Execution Logic
1. Validate sender is `config.owner`.
2. Permitted while paused (allows capital injection and recapitalization during incidents).
3. Validate funds contain `config.treasury_denom` with non-zero amount.
4. Increment `TREASURY_BALANCE += amount`.
5. Emit `treasury_deposited` action event with `sender` and `amount`.


---

## 5. Position Management (`ExecuteMsg::OpenPosition`, `ClosePosition`)

Trader or Owner opens and closes derivative positions on Injective iAsset perpetual markets.

### Messages
```rust
ExecuteMsg::OpenPosition {
    market_id: String,
    is_buy: bool,
    quantity: Uint128,
    price: Option<Decimal256>,
    margin: Uint128,
    leverage: Option<Decimal256>,
}

ExecuteMsg::ClosePosition {
    market_id: String,
}
```

### Risk Guardrails Enforced in `OpenPosition`
1. `ensure_not_paused`: Blocked when contract is paused.
2. `ensure_trader_or_owner`: Sender must match `config.trader` or `config.owner`.
3. `MarketNotAllowed`: `market_id` must reside in `config.allowed_markets`.
4. `PositionTooLarge`: `quantity` cannot exceed `config.max_position_size`.
5. `PositionMarginExceeded`: `margin` cannot exceed `config.max_position_margin`.
6. `LeverageTooHigh`: Explicit `leverage` and implicit notional leverage (`(quantity * price) / margin`) cannot exceed `config.max_leverage`.
7. `ExposureLimitExceeded`: Total reserved margin + new margin cannot exceed `(TREASURY_BALANCE + RESERVED_MARGIN) * max_exposure_bps / 10000`.
8. `CooldownActive`: Elapsed time since last position opening must exceed `config.cooldown_seconds`.
9. `InsufficientTreasury`: Free `TREASURY_BALANCE >= margin`.

### Close Position Logic
1. Sender must be trader or owner.
2. Permitted while paused (positions can always be closed to de-risk).
3. Releases reserved margin back into free `TREASURY_BALANCE`.
4. Deletes entry from `POSITIONS`.
5. Realized PnL is tracked and updated upon trade settlement.

---

## 6. Yield Distribution (`ExecuteMsg::Distribute`)

Owner moves positive net realized profits from free treasury into staker rewards.

### Message
```rust
ExecuteMsg::Distribute { amount: Uint128 }
```

### Execution Logic & High-Water Mark Invariant
1. Sender must be `config.owner`; contract must not be paused.
2. `TOTAL_STAKED > 0`, otherwise reject with `NoStakers`.
3. Distributable ceiling: `distributable = max(0, REALIZED_PNL_NET - TOTAL_DISTRIBUTED)`.
   - Requires `0 < amount <= distributable` and `amount <= TREASURY_BALANCE`.
   - If trading incurred losses, future distributions require recovering past losses past the high-water mark.
4. Reward index increment:
   ```text
   increment = Decimal256::from_ratio(amount, TOTAL_STAKED)
   GLOBAL_INDEX += increment
   ```
   Requires non-zero increment to prevent zero-value distribution lockups.
5. State transition:
   - `TOTAL_DISTRIBUTED += amount`
   - `REWARDS_RESERVED += amount`
   - `TREASURY_BALANCE -= amount`
6. Emit `yield_distributed` action event with `amount`, `new_index`, and `total_staked`.

---

## 7. Yield Claiming (`ExecuteMsg::Claim`)

Stakers claim accrued yield calculated via the MasterChef-style continuous reward index.

### Message
```rust
ExecuteMsg::Claim {}
```

### Execution Logic & Rounding Dust Solvency
1. **Unconditional Availability**: Open even while paused.
2. Settle caller:
   ```rust
   delta = GLOBAL_INDEX - USER_INDEX[user]
   earned = floor(user_stake * delta)
   USER_PENDING[user] += earned
   USER_INDEX[user] = GLOBAL_INDEX
   ```
3. Load `amount = USER_PENDING[user]`. Reject with `NothingToClaim` if zero.
4. Update state: `USER_PENDING[user] = 0`, `USER_CLAIMED[user] += amount`, `REWARDS_RESERVED -= amount`.
5. Dispatch `BankMsg::Send` transferring `amount` of `treasury_denom` (USDT) to caller.
6. Mathematical Solvency: Truncation down ensures `sum(claims) <= total_distributed`. Remainder dust remains safe inside `REWARDS_RESERVED`.


---

## 8. Governance & Administrative Handlers

The contract enforces strict RBAC separation between Owner and Trader:

### 2-Step Ownership Transfer
Prevents catastrophic loss of contract administration due to typo or erroneous address inputs:
- `ExecuteMsg::ProposeOwner { new_owner }`: Owner designates candidate in `PENDING_OWNER`.
- `ExecuteMsg::AcceptOwner {}`: Must be sent directly from candidate address to claim ownership.

### Trader Assignment
- `ExecuteMsg::UpdateTrader { new_trader: Option<String> }`: Owner designates or revokes active trading address.

### Emergency Circuit Breaker
- `ExecuteMsg::Pause {}` / `ExecuteMsg::Unpause {}`: Owner can halt new deposits, trades, and stakes during market distress. Unstaking and yield claims remain permanently open.

### Risk Parameter Updates
- `ExecuteMsg::UpdateConfig { ... }`: Owner adjusts risk thresholds (`max_leverage`, `max_exposure_bps`, `max_position_size`, `max_position_margin`, `cooldown_seconds`, `allowed_markets`).

---

## 9. Query Interface & Response Schemas

All queries execute in read-only compute without mutating storage:

```rust
pub enum QueryMsg {
    Config {},
    UserStake { user: String },
    TotalStaked {},
    TreasuryBalance {},
    TreasuryState {},
    PendingYield { user: String },
    UserInfo { user: String },
    DistributionInfo {},
    Positions { start_after: Option<String>, limit: Option<u32> },
    RiskConfig {},
}
```

### JSON Response Schemas

#### `Config`
```json
{
  "seed_token": "factory/inj1.../seed",
  "treasury_denom": "peggy0xdAC17F958D2ee523a2206206994597C13D831ec7",
  "owner": "inj1owner...",
  "paused": false,
  "trader": "inj1trader...",
  "max_leverage": "3.0",
  "max_position_size": "10000000",
  "max_exposure_bps": 5000,
  "max_position_margin": "5000000",
  "cooldown_seconds": 60,
  "allowed_markets": ["0x..."]
}
```

#### `RiskConfig`
```json
{
  "allowed_markets": ["0x..."],
  "max_leverage": "3.0",
  "max_exposure_bps": 5000,
  "max_position_margin": "5000000",
  "cooldown_seconds": 60,
  "current_exposure_bps": 2500,
  "current_used_exposure": "2500000",
  "reserved_margin": "2500000",
  "total_treasury": "10000000"
}
```

#### `UserInfo`
```json
{
  "stake": "100000000",
  "pending": "1250000",
  "claimed": "500000",
  "pool_share_bps": 2500
}
```

#### `DistributionInfo`
```json
{
  "total_distributed": "5000000",
  "rewards_reserved": "3250000",
  "last_distribution": 12845000,
  "global_index": "0.050000000000000000"
}
```

---

## 10. On-Chain Invariants & Solvency Guarantees

1. **Vault Solvency**:
   `bank_balance(treasury_denom) >= TREASURY_BALANCE + RESERVED_MARGIN + REWARDS_RESERVED`
2. **Staker Asset Isolation**:
   `bank_balance(seed_token) == TOTAL_STAKED == sum(USER_STAKE)`
   Trading losses and perpetual margin liquidations affect solely the treasury trading pool and cannot alter or dilute staked `$SEED` balances.
3. **High-Water Mark Distribution Protection**:
   `TOTAL_DISTRIBUTED <= max(0, REALIZED_PNL_NET)`
   Yield can only be distributed if cumulative realized PnL exceeds previously distributed amounts.
4. **Rounding Conservation**:
   All fractional rewards round down (`floor`), preserving rounding remainder dust in `REWARDS_RESERVED`.
5. **Circuit Breaker Asymmetry**:
   Pausing immediately disables state-expanding and capital-committing handlers (`Stake`, `Distribute`, `OpenPosition`), but unconditionally preserves staker capital recovery (`Unstake`, `Claim`).
