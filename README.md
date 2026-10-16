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

`contracts/seed-vault` is the CosmWasm MVP:

- `Stake` / `Unstake` — 1:1 `$SEED`, no lock, no fee
- `DepositTreasury` / `Distribute` / `Claim` — reward index, not a raw stake share of old profit
- `OpenPosition` / `ClosePosition` — Injective derivative order, leverage cap, local margin reservation
- `Pause` / `Unpause`

```powershell
cargo test --manifest-path contracts/seed-vault/Cargo.toml --lib
cargo clippy --manifest-path contracts/seed-vault/Cargo.toml --all-targets -- -D warnings
```

Pinned to `injective-cosmwasm` 0.3.7 and `cosmwasm-std` 3. Close releases the local reservation immediately. Realized PnL is only paid after an admin `Distribute`, once the USDT is actually on the contract. `injective-test-tube` and testnet deploy are still out of scope.

## Files in this folder
- `stake.md`
- `unstake.md`
- `deposit_treasury.md`
- `manage_position.md`
- `claim_yield.md`
- `queries.md`

## Next Steps
- Test exchange replies with `injective-test-tube` once the Injective toolchain is available
- Deploy to testnet first

**Project name**: Seed  
**Chain**: Injective  
**Launchpad**: Sprout.fun  
**Contract language**: CosmWasm (Rust)