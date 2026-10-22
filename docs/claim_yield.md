# Seed Vault — Claim Yield Function

## Overview
Stakers claim the yield they have earned from realized treasury profit distributions. Yield is calculated and distributed using a **MasterChef-style continuous reward index**:
- Each call to `Distribute` raises the global reward index (`GLOBAL_INDEX`) proportionally to the staker pool: `increment = amount / total_staked`.
- Each user's accrued earnings are lazily settled from the delta between `GLOBAL_INDEX` and their personal `USER_INDEX` checkpoint: `earned = floor(user_stake * (global_index - user_index))`.

This architecture prevents retroactive yield harvesting (flash-staking exploits): a new staker who enters after a profitable trade begins at the current `GLOBAL_INDEX` and earns zero past distributions.

See `distribute.md` for distribution mechanics and high-water mark validation.

---

## Message
```rust
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Claim {},
}
```

---

## State
```rust
pub const GLOBAL_INDEX: Item<Decimal256> = Item::new("global_index");          // Accumulated reward per 1 staked token
pub const USER_INDEX: Map<&Addr, Decimal256> = Map::new("user_index");         // Index checkpoint at user's last settle
pub const USER_PENDING: Map<&Addr, Uint128> = Map::new("user_pending");        // Settled but unclaimed yield
pub const USER_CLAIMED: Map<&Addr, Uint128> = Map::new("user_claimed");        // Cumulative lifetime yield claimed
pub const REWARDS_RESERVED: Item<Uint128> = Item::new("rewards_reserved");     // Distributed yield liability held in vault
```

---

## Core Helper: Settle
`settle` must be executed for a user before any operation that alters their stake (`Stake`, `Unstake`), and at the start of `Claim`.

```rust
pub fn settle(storage: &mut dyn Storage, user: &Addr) -> Result<(), ContractError> {
    let global = GLOBAL_INDEX.may_load(storage)?.unwrap_or_default();
    let user_index = USER_INDEX.may_load(storage, user)?;

    if let Some(user_index) = user_index {
        let stake = USER_STAKE.may_load(storage, user)?.unwrap_or_default();
        let delta = global.checked_sub(user_index).map_err(checked)?;
        let earned = reward_for(stake, delta)?;
        USER_PENDING.update(storage, user, |pending| -> Result<_, ContractError> {
            add_u128(pending.unwrap_or_default(), earned)
        })?;
    }

    USER_INDEX.save(storage, user, &global)?;
    Ok(())
}
```

- **First-time stakers**: A user with no prior `USER_INDEX` has their checkpoint initialized to the current `GLOBAL_INDEX`, ensuring they do not retroactively claim prior profits.
- **Unstaking**: Unstaking flushes earned yield into `USER_PENDING` before reducing the stake. The user may claim their accrued yield at any time even if their active stake is zero.

---

## Execution Logic (`Claim`)
1. **Unconditional Availability**: `Claim` remains open even when the vault is paused, ensuring stakers can always collect yield already earned.
2. Settle the user's pending rewards via `settle(storage, &user)`.
3. Load `amount = USER_PENDING[user]`. If `amount == 0`, reject with `ContractError::NothingToClaim`.
4. Reset `USER_PENDING[user] = 0`.
5. Accumulate `USER_CLAIMED[user] += amount`.
6. Deduct vault reward liability: `REWARDS_RESERVED -= amount`.
7. Construct and return a `BankMsg::Send` transferring `amount` of `treasury_denom` (USDT) to `info.sender`.
8. Emit response attributes:
   - `action`: `"yield_claimed"`
   - `user`: caller address
   - `amount`: claimed amount

---

## Related Functions Integration

### `stake.md`
- Calls `settle(user)` before modifying state.
- Increases `USER_STAKE` and `TOTAL_STAKED`.
- Sets `USER_INDEX = GLOBAL_INDEX` for new stakers.

### `unstake.md`
- Calls `settle(user)` before reducing `USER_STAKE` and `TOTAL_STAKED`.
- Leaves previously earned yield intact in `USER_PENDING`.
- Callable while paused.

### `distribute.md`
- Increases `GLOBAL_INDEX` by `amount / TOTAL_STAKED`.
- Segregates distributed funds from `TREASURY_BALANCE` into `REWARDS_RESERVED`.
- Blocked if `TOTAL_STAKED == 0` (`ContractError::NoStakers`).

---

## Queries
Read-only queries compute unsettled index yield dynamically without mutating contract storage:

```rust
// On-the-fly calculation:
pending_yield(user) = USER_PENDING[user] + reward_for(stake(user), GLOBAL_INDEX - USER_INDEX[user])
```

Available queries:
```rust
QueryMsg::PendingYield { user: String }
QueryMsg::UserInfo { user: String }       // Returns stake, pending, claimed, and pool share
QueryMsg::DistributionInfo {}             // Returns total_distributed, rewards_reserved, global_index, last_distribution
```

---

## State Changes
- `USER_PENDING[user]`: Set to `0`.
- `USER_CLAIMED[user]`: Incremented by claimed `amount`.
- `REWARDS_RESERVED`: Decremented by claimed `amount`.

---

## Errors
- `ContractError::NothingToClaim`: User has zero pending yield.
- `ContractError::Std(...)`: Address validation or serialization failure.

---

## Example Call (CLI)
```bash
injectived tx wasm execute $CONTRACT '{"claim":{}}' \
  --from $USER \
  --chain-id injective-888
```

---

## Invariants & Solvency Properties
1. **Mathematical Solvency**: CosmWasm integer division truncates fractions (`floor`), ensuring `sum(user_claims) <= total_distributed`. Fractional rounding dust remains safely accounted for inside `REWARDS_RESERVED` and contract bank balances.
2. **Liability Segregation**: Once profit is distributed into `REWARDS_RESERVED`, subsequent trading losses on Injective perpetuals cannot touch or dilute staker rewards.
3. **Double-Claim Prevention**: Immediate zeroing of `USER_PENDING` upon claim ensures idempotent execution; repeated calls reject with `NothingToClaim`.
4. **Testnet Notice**: Because exchange integration has not been finalized, `Distribute` and `Claim` have not yet been exercised end to end with real trading profits on testnet.
