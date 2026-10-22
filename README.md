# Seed — Project Overview

**Seed** is a community RWA yield vault token launched on Sprout.fun (Injective).

## Core Idea
1. Launch `$SEED` on Sprout.fun (fair launch → graduate to Choice).
2. Users stake `$SEED` in a CosmWasm vault.
3. Treasury (USDT from fees + deposits) is used as margin to open positions on Injective **iAssets** (RWA perps: stocks, commodities, FX).
4. Realized profit is distributed to stakers.

## Tech Stack
- Token: Sprout.fun (TokenFactory / MTS)
- Vault: **CosmWasm** (Rust)
- RWA exposure: Injective Exchange Module (derivative markets / iAssets)
- Interaction: `Any` messages + `injective-std` / `injective-cosmwasm`

## Recommended Build Order
1. Basic stake / unstake + state
2. Deposit treasury
3. Open / close position (Any message to exchange)
4. Claim yield
5. Frontend + dashboard

## Contract

`contracts/seed-vault` is the CosmWasm vault implementation:

- `Stake` / `Unstake` — 1:1 `$SEED`, no lock, no fee. Unstaking preserves accrued yield.
- `DepositTreasury` / `Distribute` / `Claim` — MasterChef-style continuous reward index. Realized profit moves from free treasury into segregated rewards.
- `OpenPosition` / `ClosePosition` — Injective derivative order execution, leverage limits, local margin reservation.
- `Pause` / `Unpause` — Emergency circuit breaker. Halts new staking and trading while keeping withdrawals and claims open.

```powershell
cargo test --manifest-path contracts/seed-vault/Cargo.toml --lib
cargo clippy --manifest-path contracts/seed-vault/Cargo.toml --all-targets -- -D warnings
```

Pinned to `injective-cosmwasm` 0.3.7 and `cosmwasm-std` 3. Realized PnL is only paid after an owner `Distribute`, once the USDT is settled in the contract.

## Documentation

The complete architectural specification, state schemas, and execution logic are consolidated into a single technical reference document:

- **[`docs/specifications.md`](docs/specifications.md)** — Comprehensive Seed Vault Specification:
  - Architecture & 3-Tier Accounting
  - Staking (`Stake`, `Unstake`)
  - Treasury & Position Management (`DepositTreasury`, `OpenPosition`, `ClosePosition`)
  - Continuous Reward Index & Yield Distribution (`Distribute`, `Claim`)
  - 2-Step Governance & Risk Engine
  - Full Query Interface & Response Schemas
  - Mathematical Invariants & Solvency Guarantees

## Risks

### 1. Treasury Loss & Liquidation Risk
Treasury funds committed as margin to Injective derivative positions (iAssets) carry market risk. Adverse market movements, gap openings, or extreme volatility in underlying assets can cause realized losses or trigger liquidation of open positions. Liquidation results in a permanent loss of the reserved margin, reducing total treasury capital.

### 2. Yield Uncertainty
Yield is strictly contingent on generating positive realized trading profit or net positive funding rates on Injective derivative markets. Yield is neither fixed nor guaranteed. During unfavorable market conditions or trading drawdowns, realized PnL will be negative and the contract will distribute zero yield. Yield distributions only resume once net trading performance recovers above the prior high-water mark (`REALIZED_PNL_NET > TOTAL_DISTRIBUTED`).

### 3. Role Permissions & Boundaries

| Role | Permitted Actions | Strictly Prohibited Actions |
|---|---|---|
| **Owner** | Deposit treasury funds, distribute realized profit, trigger pause/unpause, initiate 2-step ownership transfer (`ProposeOwner`/`AcceptOwner`), assign or revoke trader address, update risk configurations. | Cannot withdraw, burn, or seize user-staked `$SEED` tokens; cannot tamper with individual user stake balances; cannot force-claim yield on behalf of other addresses. |
| **Trader** | Execute `OpenPosition` and `ClosePosition` on allowed markets within configured limits. | Cannot call owner handlers (`DepositTreasury`, `Distribute`, `Pause`, `Unpause`, `ProposeOwner`, `UpdateTrader`, `UpdateConfig`); cannot withdraw treasury funds; cannot alter staker balances. |
| **Stakers** | Call `Stake`, `Unstake`, and `Claim` (unstaking and claims remain available even during contract pause). | Cannot touch treasury funds, open positions, or modify configurations. |

### 4. On-Chain Risk Limits
The contract enforces programmatic safety invariants before any derivative position can be opened:

- **Market Allowlist (`allowed_markets`)**: Positions can only be opened on explicit market IDs approved in the configuration.
- **Maximum Leverage (`max_leverage`)**: Hard ceiling on position leverage (default 3x). Both explicit leverage parameters and implicit notional leverage (`quantity * price / margin`) are validated against this limit.
- **Maximum Exposure (`max_exposure_bps`)**: Total reserved margin across all open positions cannot exceed a configured percentage of total treasury (default 5,000 bps = 50%).
- **Maximum Position Size (`max_position_size`)**: Base quantity limit for any individual order to prevent oversized exposure.
- **Maximum Position Margin (`max_position_margin`)**: Margin limit on a single position to prevent concentrated risk.
- **Trading Cooldown (`cooldown_seconds`)**: Minimum elapsed time required between consecutive `OpenPosition` executions to prevent rapid trade stacking.
- **Circuit Breaker (`paused`)**: Pausing immediately blocks `Stake`, `Distribute`, and `OpenPosition`, while guaranteeing unconditional access to `Unstake` and `Claim`.

## Next Steps
- Test exchange replies with `injective-test-tube` once the Injective toolchain is available
- Deploy to testnet first

**Project name**: Seed  
**Chain**: Injective  
**Launchpad**: Sprout.fun  
**Contract language**: CosmWasm (Rust)