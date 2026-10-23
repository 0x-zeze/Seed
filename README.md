# Seed — Project Overview

> **Notice:** Testnet only, not audited, do not deposit real funds.

**Seed** is a community RWA yield vault token launched on Sprout.fun (Injective).

## Core Idea
1. Launch `$SEED` on Sprout.fun (fair launch → graduate to Choice).
2. Users stake `$SEED` in a CosmWasm vault.
3. Treasury (USDT from fees + deposits) is used as margin to open positions on Injective **iAssets** (RWA perps: stocks, commodities, FX).
4. Realized profit is distributed to stakers.

## Status

### Implemented
- **Staking & Unstaking**: 1:1 `$SEED` staking and unstaking with no deposit/withdrawal fees and no lock period. Unstaking preserves accrued yield. Unstaking remains operational while paused.
- **Treasury Management**: Owner depositing of treasury margin (`DepositTreasury`). Allowed while paused so owner can recapitalize.
- **Continuous Reward Index**: MasterChef-style index tracking staker rewards lazily (`settle`). High-water mark distribution (`distribute` requires `REALIZED_PNL_NET > TOTAL_DISTRIBUTED`). Stakers claim yield via `Claim`, which remains operational while paused.
- **Local Position Accounting & Risk Limits**: `OpenPosition` and `ClosePosition` enforce programmatic constraints:
  - Market allowlist (`allowed_markets`)
  - Leverage ceiling (`max_leverage`) across explicit and implicit notional leverage
  - Exposure ceiling (`max_exposure_bps`) as a fraction of total treasury
  - Position size and margin caps (`max_position_size`, `max_position_margin`)
  - Trading cooldown timer (`cooldown_seconds`)
  - Single active position per market restriction
  - Margin reservation and release between free treasury and reserved margin
  - ClosePosition releases full reserved margin and records 0 realized PnL locally
- **Governance & Circuit Breaker**: Emergency pause/unpause (`Pause`/`Unpause`) blocking new stakes, distributions, and position opening. Two-step ownership transfer (`ProposeOwner`/`AcceptOwner`), separate trader assignment (`UpdateTrader`), and risk parameter updates (`UpdateConfig`).
- **Comprehensive Queries**: `Config`, `UserStake`, `TotalStaked`, `TreasuryBalance`, `TreasuryState`, `PendingYield`, `Positions`, `OpenPositions`, `UserInfo`, `DistributionInfo`, `RiskConfig`.
- **Testnet Deployment**: Stored and instantiated on Injective testnet (`injective-888`) with code ID `40036` and contract `inj1e9vlszrj8p3e8zvamt386chuefm782xg46uw7k`.
- **Testnet Smoke Test**: 8/8 testnet checks passing (stake, unstake, deposit treasury, pause blocking stake, unpause, open/close position, reject non-whitelisted market, two-step ownership round trip).
- **Unit Test Suite**: 58 passing unit tests covering role boundaries, risk limit rejections, index math, and state transitions.

### Not Implemented Yet
- **Exchange Integration**: Subaccount deposits, order creation (`MsgCreateDerivativeMarketOrder` / `MsgCreateDerivativeLimitOrder`), and exchange settlement message emission are not implemented.
- **Chain-Derived PnL**: `ClosePosition` does not query the Injective exchange module or settle subaccount balances; it records 0 realized PnL locally. Callers cannot supply arbitrary PnL values.
- **Partial Close**: Partial position close is rejected (`PartialCloseNotSupported`).
- **End-to-End Distribute / Claim with Trading Profits**: Because exchange integration is pending, `Distribute` and `Claim` have not been exercised end to end with real trading profits on-chain.

## Tech Stack
- Token: Sprout.fun (TokenFactory / MTS)
- Vault: **CosmWasm** (Rust 1.86 MSRV)
- RWA exposure: Injective Exchange Module (derivative markets / iAssets)
- Dependencies: `cosmwasm-std` 3.0.5, `injective-std` 1.20.0 (injective-cosmwasm / injective-math crates removed)

## Recommended Build Order
1. Basic stake / unstake + state (Complete)
2. Deposit treasury + reward index distribution (Complete)
3. Open / close position local accounting & risk limits (Complete)
4. Testnet deployment & smoke validation (Complete)
5. Exchange module integration (subaccounts, orders, PnL settlement) (Next)
6. End-to-end profit distribution & claim testing (Next)
7. Frontend + dashboard (Next)

## Contract

`contracts/seed-vault` is the CosmWasm vault implementation:

- `Stake` / `Unstake` — 1:1 `$SEED`, no lock, no fee. Unstaking preserves accrued yield.
- `DepositTreasury` / `Distribute` / `Claim` — MasterChef-style continuous reward index. Realized profit moves from free treasury into segregated rewards.
- `OpenPosition` / `ClosePosition` — local accounting only. Opening checks the market allowlist, leverage, size, and exposure, then moves margin from free treasury into `RESERVED_MARGIN` and stores one position per market. Closing requires that position, releases the full reserved margin back to free treasury, and books realized PnL as 0. Exchange integration is **not implemented yet**: there is no subaccount deposit, no order placement, and no PnL read from the Injective chain. A caller cannot supply a PnL value. Partial close is rejected.
- `Pause` / `Unpause` — Emergency circuit breaker. Halts new staking and trading while keeping withdrawals and claims open.

```powershell
cargo test --manifest-path contracts/seed-vault/Cargo.toml --lib
cargo clippy --manifest-path contracts/seed-vault/Cargo.toml --all-targets -- -D warnings
```

Toolchain pinned to Rust 1.86 (MSRV) so wasm builds reproducibly with `cosmwasm/optimizer:0.17.0`. Realized PnL is only paid after an owner `Distribute`, once profit is settled in the contract. `ClosePosition` does not settle exchange PnL yet, so a close does not by itself create distributable profit.

## Documentation

All architectural specifications, message schemas, and verification records are organized in the [`docs/`](docs/) directory:

- [`docs/stake.md`](docs/stake.md) — Staking `$SEED` into the vault.
- [`docs/unstake.md`](docs/unstake.md) — Withdrawing `$SEED` (remains open during pause).
- [`docs/deposit_treasury.md`](docs/deposit_treasury.md) — Owner depositing margin into the treasury pool.
- [`docs/distribute.md`](docs/distribute.md) — Yield distribution logic, high-water mark validation, and global index accrual.
- [`docs/manage_position.md`](docs/manage_position.md) — Position management, local risk limits, and exchange integration roadmap.
- [`docs/claim_yield.md`](docs/claim_yield.md) — Staker reward claiming, lazy `settle` accounting, and rounding dust solvency.
- [`docs/queries.md`](docs/queries.md) — Read-only state queries for frontend integration (`Config`, `TreasuryState`, `OpenPositions`, `RiskConfig`, `UserInfo`, etc.).
- [`docs/verification-notes.md`](docs/verification-notes.md) — Internal developer testing and parameter verification notes.
- [`docs/testnet-deployment.md`](docs/testnet-deployment.md) — Injective testnet deployment records and independent verification instructions.

## Risks

### 1. Treasury Loss & Liquidation Risk
Treasury funds committed as margin to Injective derivative positions (iAssets) carry market risk. Adverse market movements, gap openings, or extreme volatility in underlying assets can cause realized losses or trigger liquidation of open positions. Liquidation results in a permanent loss of the reserved margin, reducing total treasury capital.

### 2. Yield Uncertainty
Yield is strictly contingent on generating positive realized trading profit or net positive funding rates on Injective derivative markets. Yield is variable and subject to market conditions. During unfavorable market conditions or trading drawdowns, realized PnL will be negative and the contract will distribute zero yield. Yield distributions only resume once net trading performance recovers above the prior high-water mark (`REALIZED_PNL_NET > TOTAL_DISTRIBUTED`).

### 3. Role Permissions & Boundaries

| Role | Permitted Actions | Strictly Prohibited Actions |
|---|---|---|
| **Owner** | Deposit treasury funds, distribute realized profit, trigger pause/unpause, initiate 2-step ownership transfer (`ProposeOwner`/`AcceptOwner`), assign or revoke trader address (`UpdateTrader`), update risk configurations (`UpdateConfig`). | Cannot withdraw, burn, or seize user-staked `$SEED` tokens; cannot tamper with individual user stake balances; cannot force-claim yield on behalf of other addresses. |
| **Trader** | Execute `OpenPosition` and `ClosePosition` on allowed markets within configured limits. | Cannot call owner handlers (`DepositTreasury`, `Distribute`, `Pause`, `Unpause`, `ProposeOwner`, `UpdateTrader`, `UpdateConfig`); cannot withdraw treasury funds; cannot alter staker balances. |
| **Stakers** | Call `Stake`, `Unstake`, and `Claim` (unstaking and claims remain available even during contract pause). | Cannot touch treasury funds, open positions, or modify configurations. |

### 4. On-Chain Risk Limits
The contract enforces programmatic safety invariants before any derivative position can be opened:

- **Market Allowlist (`allowed_markets`)**: Positions can only be opened on explicit market IDs approved in the configuration.
- **Maximum Leverage (`max_leverage`)**: Hard ceiling on position leverage (default 3x). Both explicit leverage and implicit notional leverage are checked. Notional is `quantity * chain_price` in the same micro-units as margin (`quote_decimals` fixed at 6, already inside the chain price). `price` and `margin` are chain integers; `quantity` is contracts and may be fractional (`"0.01"`).
- **Maximum Exposure (`max_exposure_bps`)**: Total reserved margin across all open positions cannot exceed a configured percentage of total treasury (default 5,000 bps = 50%).
- **Maximum Position Size (`max_position_size`)**: Base quantity limit for any individual order to prevent oversized exposure.
- **Maximum Position Margin (`max_position_margin`)**: Margin limit on a single position to prevent concentrated risk.
- **Trading Cooldown (`cooldown_seconds`)**: Minimum elapsed time required between consecutive `OpenPosition` executions to prevent rapid trade stacking.
- **Circuit Breaker (`paused`)**: Pausing immediately blocks `Stake`, `Distribute`, and `OpenPosition`, while preserving access to `Unstake` and `Claim`.

## Next Steps
- Implement exchange subaccount margin deposit and withdrawal mechanics.
- Route derivative orders (`MsgCreateDerivativeMarketOrder` / `MsgCreateDerivativeLimitOrder`) to the Injective exchange module via `CosmosMsg::Any`.
- Ingest exchange execution replies and book chain-derived PnL on `ClosePosition`.
- Test exchange response handlers using `injective-test-tube`.
- Exercise end-to-end profit distribution and claim flows with real trading profits on testnet.

**Project name**: Seed  
**Chain**: Injective  
**Launchpad**: Sprout.fun  
**Contract language**: CosmWasm (Rust)