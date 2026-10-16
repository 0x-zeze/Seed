# Seed Vault — Claim Yield Function (v2, reward index)

## Overview
Stakers claim the yield they have earned from admin distributions. Yield is tracked with a reward index (MasterChef style): each distribution raises a global index, and each user's earnings are settled from the difference between the global index and their own index. This replaces the older `user_stake / total_staked * profit` formula, which could be gamed by staking right before a claim.

See `distribute.md` for how the global index is raised.

## Message
```rust
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Claim {},
    // ...
}
```

## State
```rust
pub const GLOBAL_INDEX: Item<Decimal256> = Item::new("global_index");                    // from distribute.md
pub const USER_INDEX: Map<&Addr, Decimal256> = Map::new("user_index");                   // index at the user's last settle
pub const USER_PENDING: Map<&Addr, Uint128> = Map::new("user_pending");                  // earned but not yet claimed
pub const USER_CLAIMED: Map<&Addr, Uint128> = Map::new("user_claimed");                  // optional, lifetime claimed (for dashboard)
```

## Core helper: settle
Run this for a user before anything that changes their stake, and at the start of `Claim`.

```rust
// sketch, use checked math and the right Decimal256/Uint256 conversion for your cosmwasm-std version
fn settle(storage: &mut dyn Storage, user: &Addr) -> StdResult<()> {
    let global = GLOBAL_INDEX.may_load(storage)?.unwrap_or_default();
    let user_index = USER_INDEX.may_load(storage, user)?.unwrap_or_default();
    let stake = USER_STAKE.may_load(storage, user)?.unwrap_or_default();

    // earned = stake * (global - user_index), rounded down
    let earned: Uint128 = reward_for(stake, global - user_index);

    USER_PENDING.update(storage, user, |p| -> StdResult<_> { Ok(p.unwrap_or_default() + earned) })?;
    USER_INDEX.save(storage, user, &global)?;
    Ok(())
}
```

Important: a brand new staker gets `USER_INDEX = GLOBAL_INDEX` at their first stake, so they earn nothing from distributions that happened before they joined.

## Logic (Claim)
1. Contract not paused for claim only if you choose to block it. Recommended: claim stays open even when paused (see `distribute.md`).
2. Call `settle(user)`.
3. `amount = USER_PENDING[user]`. Reject if zero.
4. Set `USER_PENDING[user] = 0`, `USER_CLAIMED[user] += amount`.
5. `REWARDS_RESERVED -= amount`.
6. Send `amount` of the treasury denom (USDT) to the user with a bank send.
7. Emit event: `yield_claimed` with `user`, `amount`.

Always update state before the transfer message goes out.

## Required changes to other functions
These are what actually make the index correct.

**stake.md**
- Call `settle(user)` first, then increase `USER_STAKE` and `TOTAL_STAKED`.
- If the user has never staked, initialize `USER_INDEX[user] = GLOBAL_INDEX`.

**unstake.md**
- Call `settle(user)` first, then reduce `USER_STAKE` and `TOTAL_STAKED`.
- Unstaking does not forfeit pending yield. It stays in `USER_PENDING` and can be claimed later.
- If `TOTAL_STAKED` becomes zero, `Distribute` is rejected until someone stakes again (`NoStakers`).

## Queries
`PendingYield` must not write to state, so compute the settle amount on the fly:

```
pending_yield(user) = USER_PENDING[user] + stake(user) * (GLOBAL_INDEX - USER_INDEX[user])
```

Extra queries for the dashboard:
```rust
PendingYield { user: String },        // as above
UserInfo { user: String },            // stake, pending, claimed, share of pool (%)
DistributionInfo {},                  // total_distributed, rewards_reserved, last_distribution height, global_index
```

## State Changes
- `USER_PENDING`, `USER_INDEX`, `USER_CLAIMED`
- `REWARDS_RESERVED` (decreases on claim)

## Errors
- Nothing to claim
- Contract paused (only if you decide to block claims while paused)

## Example Call
```bash
injectived tx wasm execute $CONTRACT '{"claim":{}}' \
  --from $USER \
  --chain-id injective-1
```

## Notes
- Keep `REWARDS_RESERVED` in sync: `sum(all USER_PENDING) + unsettled index earnings` must never exceed it, otherwise the vault cannot pay everyone. Add a test for this invariant.
- Round down on every settle. The tiny remainder stays in the vault.
- Do not auto-claim on unstake for the MVP. Keeping them separate is simpler and easier to audit.
- Nice to have later: `ClaimAndRestake` if the yield asset can be swapped to `$SEED`, and a per-user `USER_CLAIMED` view for the dashboard.

## Suggested tests
1. User A stakes, distribution happens, A claims the full amount.
2. A stakes, distribution 1, B stakes, distribution 2: A earns from both, B only from distribution 2.
3. B stakes right before claim but after the distribution: B gets 0.
4. Unstake with pending yield, then claim later: amount is unchanged.
5. Sum of all claims never exceeds the total distributed amount.
6. Distribute with `TOTAL_STAKED == 0` is rejected.
