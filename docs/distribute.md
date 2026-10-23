# Seed Vault — Distribute Function

## Overview
Contract **owner** moves realized trading profit from the free treasury into the staker reward pool. This is the only mechanism through which yield enters the reward system. Distributing increments a global reward index (`GLOBAL_INDEX`), and each staker's share is settled lazily against this index.

Why an index and not `user_stake / total_staked * profit` at claim time: the naive formula allows someone to stake immediately before a distribution and extract a share of profit they did not participate in. With the index, new stakers start at the current index and earn nothing from past distributions.

## Message
```rust
#[cw_serde]
pub enum ExecuteMsg {
    Distribute { amount: Uint128 },
    Pause {},
    Unpause {},
    ProposeOwner { new_owner: String },
    AcceptOwner {},
    UpdateTrader { new_trader: Option<String> },
    UpdateConfig { /* ... */ },
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
pub const PENDING_OWNER: Item<Addr> = Item::new("pending_owner");
```

Treasury accounting must always satisfy:
```text
vault_balance(treasury_denom) >= TREASURY_BALANCE (free) + margin_in_exchange + REWARDS_RESERVED
```
`REWARDS_RESERVED` is strictly segregated. Once profit is distributed, subsequent trading losses cannot touch it.

## Logic (Distribute)
1. Validate sender is owner (`ensure_owner`) and contract is not paused (`ensure_not_paused`).
2. Verify `TOTAL_STAKED > 0`, otherwise reject with `ContractError::NoStakers` (cannot divide index increment by zero).
3. Compute distributable profit: `distributable = max(0, REALIZED_PNL_NET - TOTAL_DISTRIBUTED)`. Require `0 < amount <= distributable`.
4. Validate that `amount <= TREASURY_BALANCE` (free treasury).
5. Increment `GLOBAL_INDEX += Decimal256::from_ratio(amount, TOTAL_STAKED)`. If index delta is zero (distribution too small relative to stake), reject with `ContractError::InvalidAmount`.
6. Update balances:
   - `TOTAL_DISTRIBUTED += amount`
   - `REWARDS_RESERVED += amount`
   - `TREASURY_BALANCE -= amount`
7. Record block height in `LAST_DISTRIBUTION`.
8. Emit response attributes: `action = "yield_distributed"`, `amount = amount`, `new_index = GLOBAL_INDEX`, `total_staked = TOTAL_STAKED`.

## How Losses Are Handled
- `REALIZED_PNL_NET` is a signed cumulative total (`Int128`). Realized losses reduce it; realized profits increase it.
- Only profit above `TOTAL_DISTRIBUTED` can be distributed (high-water mark). If the vault incurs a trading drawdown after a distribution, future profits must first recover that deficit before any new yield becomes distributable.
- Distributed yield stored in `REWARDS_RESERVED` is segregated and protected from clawbacks.
- Staked `$SEED` principal is independent of trading outcomes. Losses impact treasury margin only.

## Owner Controls & Guardrails
- **Two-Step Ownership Transfer**: `ProposeOwner` stores `PENDING_OWNER`. The proposed address must explicitly call `AcceptOwner` to complete the transfer, preventing loss of administrative control.
- **Circuit Breaker**: `Pause` / `Unpause` (owner only). Pausing halts `Stake`, `Distribute`, and `OpenPosition`, while preserving access to `Unstake`, `Claim`, and `DepositTreasury`.
- **Trader Role**: `UpdateTrader` allows assigning a separate trader address limited strictly to `OpenPosition` and `ClosePosition`. Trader cannot distribute profit, deposit treasury, or change configurations.
- **Risk Parameters**: Configurable caps on leverage, position size, position margin, exposure bps, cooldown, and allowed market IDs via `UpdateConfig`.

## State Changes
- `GLOBAL_INDEX`: Incremented by `amount / TOTAL_STAKED`
- `TOTAL_DISTRIBUTED`: Incremented by `amount`
- `REWARDS_RESERVED`: Incremented by `amount`
- `TREASURY_BALANCE`: Decremented by `amount`
- `LAST_DISTRIBUTION`: Updated to current block height

## Errors
- `ContractError::Unauthorized`: Caller is not owner
- `ContractError::Paused`: Contract is paused
- `ContractError::NoStakers`: Total staked tokens equal zero
- `ContractError::InvalidAmount`: Amount is zero, exceeds distributable profit (`REALIZED_PNL_NET - TOTAL_DISTRIBUTED`), or index increment rounds to zero
- `ContractError::InsufficientTreasury`: Free treasury balance is less than distribution amount

## Example Call (CLI)
```bash
injectived tx wasm execute $CONTRACT '{"distribute":{"amount":"250000000000000000"}}' \
  --from $OWNER \
  --chain-id injective-888
```

## Notes
- **Status Notice**: Because exchange integration (subaccounts, orders, PnL settlement) is not yet implemented, `Distribute` and `Claim` have not been exercised end to end on testnet with real trading profits.
- CosmWasm integer division truncates fractions down, protecting vault solvency. Dust remains in `REWARDS_RESERVED`.
