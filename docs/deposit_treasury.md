# Seed Vault — Deposit Treasury Function

## Overview
Contract **owner** deposits treasury margin funds (e.g. USDT, or `inj` in testnet) into the vault to provide trading collateral for RWA perpetual positions (iAssets).

## Message
```rust
#[cw_serde]
pub enum ExecuteMsg {
    DepositTreasury {},
    // ...
}
```

## Parameters
- **Sender**: Contract owner (`info.sender == config.owner`). Trader or external callers are rejected.
- **Funds**: Must contain exactly one coin matching the configured `treasury_denom` (`info.funds`).
- **Amount**: Extracted directly from `info.funds`.

## Logic
1. Validate sender is contract owner via `ensure_owner(deps.as_ref(), &info.sender)`.
2. Validate denom matches `config.treasury_denom` and amount is strictly positive.
3. Increase `TREASURY_BALANCE` by the deposited amount.
4. Emit response with attributes: `action = "treasury_deposited"`, `amount = amount`.

## State Changes
- `TREASURY_BALANCE`: `Uint128` (incremented by deposited amount)

## Errors
- `ContractError::Unauthorized`: Caller is not contract owner (trader is also unauthorized)
- `ContractError::InvalidFunds`: Denom does not match `treasury_denom`, or multiple coins sent
- `ContractError::ZeroAmount`: Deposited amount is zero

## Example Call (CLI)
```bash
injectived tx wasm execute $CONTRACT '{"deposit_treasury":{}}' \
  --amount 1000000000000000000inj \
  --from $OWNER \
  --chain-id injective-888
```

## Notes
- **Permitted While Paused**: `DepositTreasury` is intentionally allowed during a contract pause so the owner can recapitalize the treasury without having to unpause trading or staking.
- Deposited funds remain in the contract's bank balance. Auto-depositing into an Injective exchange subaccount is **not implemented yet**.
- These funds represent free treasury margin and back derivative positions up to `max_exposure_bps`.