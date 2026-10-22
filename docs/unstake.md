# Seed Vault — Unstake Function

## Overview
Users withdraw their staked `$SEED` tokens from the vault. Yield must be claimed separately (or auto-claimed on unstake).

## Message
```rust
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Unstake { amount: Uint128 },
    // ...
}
```

## Parameters
- `amount`: Amount of `$SEED` to unstake

## Logic
1. Check user has sufficient staked balance.
2. Deduct from `USER_STAKE` and `TOTAL_STAKED`.
3. Transfer `$SEED` tokens back to the user via bank send.
4. Emit event: `seed_unstaked`.

## State Changes
- Decrease `USER_STAKE[user]`
- Decrease `TOTAL_STAKED`

## Errors
- Insufficient stake
- Zero amount
- Contract paused

## Example Call (CLI)
```bash
injectived tx wasm execute $CONTRACT '{"unstake":{"amount":"50000000"}}' \
  --from $WALLET \
  --chain-id injective-1
```

## Notes
- Consider adding a small unstaking fee or lock period to reduce short-term farming.
- If using share tokens, burn shares and return proportional underlying.