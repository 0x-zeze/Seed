# Seed Vault — Deposit Treasury Function

## Overview
Admin or authorized address deposits USDT (or other stable) into the vault to be used as margin for RWA perpetual positions (iAssets).

## Message
```rust
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    DepositTreasury {},
    // ...
}
```

## Parameters
- **Funds**: Must contain USDT (or configured treasury denom)
- Only callable by admin / authorized role

## Logic
1. Validate sender is admin.
2. Validate denom is the treasury asset (e.g. `peggy0x...` USDT or native).
3. Increase `TREASURY_BALANCE`.
4. Optionally auto-deposit into the exchange module subaccount.
5. Emit event: `treasury_deposited`.

## State Changes
- `TREASURY_BALANCE`: `Uint128`

## Errors
- Unauthorized
- Wrong denom
- Zero amount

## Example Call
```bash
injectived tx wasm execute $CONTRACT '{"deposit_treasury":{}}' \
  --amount 1000000usdt \
  --from $ADMIN \
  --chain-id injective-1
```

## Notes
- Treasury funds are later used as margin when opening derivative positions on iAssets via the exchange module.