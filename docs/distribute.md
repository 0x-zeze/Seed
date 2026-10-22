# Seed Vault — Distribute Function

## Overview
Admin moves realized profit from the treasury into the staker reward pool. This is the only way yield enters the reward system. It works together with the reward index in `claim_yield.md`: distributing raises a global index, and each staker's share is settled lazily from that index.

Why an index and not `user_stake / total_staked * profit` at claim time: the old formula lets someone stake right before a claim and take a share of profit they did not wait for. With the index, a new staker starts at the current index and earns nothing from past distributions.

## Message
```rust
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Distribute { amount: Uint128 },
    Pause {},
    Unpause {},
    ProposeAdmin { new_admin: String },
    AcceptAdmin {},
    // ...
}
```

## State
```rust
pub const GLOBAL_INDEX: Item<Decimal256> = Item::new("global_index");          // reward per 1 staked token
pub const REALIZED_PNL_NET: Item<Int128> = Item::new("realized_pnl_net");      // cumulative realized PnL (can be negative)
pub const TOTAL_DISTRIBUTED: Item<Uint128> = Item::new("total_distributed");   // cumulative profit already sent to reward pool
pub const REWARDS_RESERVED: Item<Uint128> = Item::new("rewards_reserved");     // distributed but not yet claimed (liability)
pub const LAST_DISTRIBUTION: Item<u64> = Item::new("last_distribution");       // block height
pub const PENDING_ADMIN: Item<Addr> = Item::new("pending_admin");
```

Treasury accounting must always satisfy:
```
vault_balance(treasury_denom) >= TREASURY_BALANCE (free) + margin_in_exchange + REWARDS_RESERVED
```
`REWARDS_RESERVED` is segregated on purpose. Once profit is distributed, later trading losses cannot touch it.

## Logic (Distribute)
1. Validate sender is admin (or keeper role), contract not paused.
2. `TOTAL_STAKED > 0`, otherwise reject with `NoStakers` (nobody to receive it, do not let the index divide by zero).
3. `distributable = max(0, REALIZED_PNL_NET - TOTAL_DISTRIBUTED)`. Require `0 < amount <= distributable` and `amount <= TREASURY_BALANCE` (free balance).
4. `GLOBAL_INDEX += Decimal256::from_ratio(amount, TOTAL_STAKED)`
5. `TOTAL_DISTRIBUTED += amount`, `REWARDS_RESERVED += amount`, `TREASURY_BALANCE -= amount`.
6. Set `LAST_DISTRIBUTION` to the current block height.
7. Emit event: `yield_distributed` with `amount`, `new_index`, `total_staked`.

```rust
// sketch, adjust to your cosmwasm-std version and use checked math everywhere
let total = TOTAL_STAKED.load(deps.storage)?;
ensure!(!total.is_zero(), ContractError::NoStakers {});

let net = REALIZED_PNL_NET.load(deps.storage)?;
let done = TOTAL_DISTRIBUTED.load(deps.storage)?;
let distributable = (net - done.u128() as i128).max(0) as u128;
ensure!(amount.u128() > 0 && amount.u128() <= distributable, ContractError::InvalidAmount {});

GLOBAL_INDEX.update(deps.storage, |i| -> StdResult<_> {
    Ok(i + Decimal256::from_ratio(amount, total))
})?;
REWARDS_RESERVED.update(deps.storage, |r| -> StdResult<_> { Ok(r + amount) })?;
TOTAL_DISTRIBUTED.update(deps.storage, |d| -> StdResult<_> { Ok(d + amount) })?;
TREASURY_BALANCE.update(deps.storage, |t| t.checked_sub(amount).map_err(Into::into))?;
```

## How losses are handled
- `REALIZED_PNL_NET` is a running total. A realized loss lowers it, a realized profit raises it.
- Only profit above `TOTAL_DISTRIBUTED` can be distributed (a high-water mark). If the vault loses money after a distribution, the next profits first have to recover the loss before anything new is distributable.
- Already distributed yield stays safe in `REWARDS_RESERVED` and is never clawed back from stakers.
- Staked `$SEED` principal is not touched by trading losses. Losses hit treasury (USDT) only.

`REALIZED_PNL_NET` is updated when a position is closed (see `manage_position.md`): compare the margin released back versus the margin originally reserved, and add the difference. Partial closes should update it proportionally. This needs the subaccount accounting between the contract and the exchange module to be defined first.

## Admin Controls (guardrails)
Keep these in the contract, not just in the notes:

- **Admin should be a multisig**, not a single key.
- **Two-step admin transfer**: `ProposeAdmin` then `AcceptAdmin` from the new address, so the admin role cannot be sent to a wrong address by mistake.
- **Pause**: `Pause` / `Unpause`, admin only. Recommended behavior while paused:
  - blocks: `Stake`, `Distribute`, `OpenPosition`, `DepositTreasury`
  - still allowed: `Unstake` and `Claim`, so users can always exit and collect what they already earned
- **Risk limits in config, enforced in `OpenPosition`**: `max_leverage`, `max_position_size`, `max_total_margin_bps` (share of treasury allowed as margin), and an allowlist of `market_id`.
- **Optional cap per distribution**, e.g. `max_distribution_bps` of treasury, to limit damage from a bad call.
- **Optional timelock** on config changes (risk limits, keeper address) so the community can see changes before they apply.

## State Changes
- `GLOBAL_INDEX`, `TOTAL_DISTRIBUTED`, `REWARDS_RESERVED`, `TREASURY_BALANCE`, `LAST_DISTRIBUTION`
- `REALIZED_PNL_NET` is changed by position close logic, not by this function

## Errors
- Unauthorized
- Contract paused
- NoStakers
- Amount is zero or above distributable profit
- Insufficient free treasury balance

## Example Call
```bash
injectived tx wasm execute $CONTRACT '{"distribute":{"amount":"2500000"}}' \
  --from $ADMIN \
  --chain-id injective-1
```

## Notes
- Distribute on a fixed schedule (weekly, for example) and announce it. Predictable timing is better for trust than random timing.
- Precision: `Decimal256` has 18 decimals. If `$SEED` uses 6 decimals this is fine. If total staked is very large compared to a tiny distribution, the index increase can round to zero, so set a `min_distribution`.
- Rounding always goes down in favor of the vault. The leftover dust stays in `REWARDS_RESERVED` and can be swept into the next distribution.
- Optional extra protection against just-in-time staking: add an unstake cooldown or minimum stake duration before a position counts for a new distribution.
