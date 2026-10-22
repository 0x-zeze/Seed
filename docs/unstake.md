# Seed Vault — Unstake Function

## Overview
Users withdraw their staked `$SEED` tokens from the vault. Accrued yield is lazily settled into `USER_PENDING` during the unstake call and can be claimed separately via `Claim`.

## Message
```rust
#[cw_serde]
pub enum ExecuteMsg {
    Unstake { amount: Uint128 },
    // ...
}
```

## Parameters
- `amount`: Amount of `$SEED` tokens to unstake (must be greater than zero and less than or equal to active stake)

## Logic
1. Validate that `amount` is greater than zero (`ContractError::ZeroAmount`).
2. Settle the user's accrued yield to the current `GLOBAL_INDEX` via `settle(deps.storage, &info.sender)`. This locks in all earnings up to the unstake block into `USER_PENDING`.
3. Check that the user has sufficient staked balance (`USER_STAKE[info.sender] >= amount`).
4. Deduct `amount` from `USER_STAKE[info.sender]`.
5. Deduct `amount` from `TOTAL_STAKED`.
6. Construct and send a `BankMsg::Send` returning `amount` of `config.seed_denom` to `info.sender`.
7. Emit response with attributes: `action = "seed_unstaked"`, `user = info.sender`, `amount = amount`.

## State Changes
- `USER_INDEX`: Updated to current `GLOBAL_INDEX`
- `USER_PENDING`: Accrues any unsettled yield earned prior to unstaking
- `USER_STAKE`: Decremented by `amount`
- `TOTAL_STAKED`: Decremented by `amount`

## Errors
- `ContractError::ZeroAmount`: Amount requested is zero
- `ContractError::InsufficientStake`: User attempts to unstake more than their active stake balance

## Example Call (CLI)
```bash
injectived tx wasm execute $CONTRACT '{"unstake":{"amount":"500000000000000000"}}' \
  --from $WALLET \
  --chain-id injective-888
```

## Notes
- **Unconditional Availability**: `Unstake` is explicitly permitted while the contract is paused, ensuring stakers can always withdraw their deposited principal.
- Unstaking does not forfeit accrued yield; earned rewards remain stored in `USER_PENDING` and can be claimed via `Claim` at any time, even after full unstake.
- No withdrawal fees or unstaking cooldown periods are imposed in the vault.