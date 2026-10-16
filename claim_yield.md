# Seed Vault — Claim Yield Function

## Overview
Stakers claim their proportional share of realized profit from the RWA positions.

## Message
```rust
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Claim {},
    // or Claim { amount: Option<Uint128> }
}
```

## Logic
1. Calculate user’s pending yield = `(user_stake / total_staked) * distributable_profit`.
2. Check pending yield > 0.
3. Transfer USDT (or profit token) to the user.
4. Update user’s last claimed checkpoint / reduce pending.
5. Emit event: `yield_claimed`.

## State Changes
- Decrease `DISTRIBUTABLE_PROFIT` or update per-user claim index.
- Optional: `USER_CLAIMED` map.

## Errors
- Nothing to claim
- Contract paused

## Example Call
```bash
injectived tx wasm execute $CONTRACT '{"claim":{}}' \
  --from $USER \
  --chain-id injective-1
```

## Notes
- Two common models:
  - **Simple**: Admin calls `Distribute` which snapshots profit, then users claim.
  - **Index-based**: Continuous accrual using a reward index (like MasterChef).
- For MVP, start with admin-triggered distribution + claim.