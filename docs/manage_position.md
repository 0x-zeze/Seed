# Seed Vault — Manage Position (Open / Close)

## Overview
Authorized **trader** or **owner** manages perpetual positions on Injective iAssets (RWA derivatives) using the vault's treasury as margin.

> **Status Notice:** Exchange module integration is **not implemented yet**. In the current release, `OpenPosition` and `ClosePosition` execute full local accounting, reserve margin, enforce risk invariants, and record 0 realized PnL locally. No subaccount deposits, order messages, or chain settlements occur on-chain yet.

## Message
```rust
#[cw_serde]
pub enum ExecuteMsg {
    OpenPosition {
        market_id: String,
        is_buy: bool,
        quantity: String,
        price: String,
        margin: String,
        leverage: Option<String>,
    },
    ClosePosition {
        market_id: String,
        quantity: Option<String>, // Must equal full position quantity or None; partial close is rejected
    },
    // ...
}
```

## Logic (Open Position)
1. Verify caller is authorized (`ensure_trader_or_owner`: sender must be `config.owner` or `config.trader`).
2. Verify contract is not paused (`ensure_not_paused`).
3. Enforce trading cooldown: `now >= last_opened + config.cooldown_seconds` (`ContractError::CooldownActive`).
4. Validate market allowlist: `market_id` must exist in `config.allowed_markets` (`ContractError::MarketNotAllowed`).
5. Parse and validate non-zero amounts for `quantity`, `price`, and `margin`. `price` and `margin` are integer chain strings. `quantity` is a contract amount and may include one decimal point (`"0.01"`).
6. Enforce single position per market: reject if market already exists in `POSITIONS` (`ContractError::PositionAlreadyOpen`).
7. Enforce position limits: reject if `margin > config.max_position_margin` or the whole-contract part of `quantity` exceeds `config.max_position_size` (`ContractError::PositionTooLarge`). A quantity below 1 contract is allowed.
8. Enforce leverage limits: validate explicit `leverage` (if provided) and implicit notional leverage against `config.max_leverage` (`ContractError::LeverageTooHigh`). Notional is `quantity * chain_price`. Chain price already carries `10^6`, and margin is quote base units, so both sides of the comparison are micro-units. Do not divide the product by `10^6` again.
9. Enforce free treasury solvency: require `margin <= TREASURY_BALANCE` (`ContractError::InsufficientTreasury`).
10. Enforce portfolio exposure: require `new_reserved <= total_treasury * config.max_exposure_bps / 10000` (`ContractError::ExposureLimitExceeded`).
11. Deduct margin from `TREASURY_BALANCE` and add to `RESERVED_MARGIN`.
12. Store position record in `POSITIONS` map.
13. Update `LAST_POSITION_OPENED` timestamp to current block time.
14. *(Pending Exchange Integration)* Injective order creation message generation is a stub returning empty messages (`vec![]`). Real subaccount deposit and order routing will be added in exchange integration phase.

## Logic (Close Position)
1. Verify caller is authorized (`ensure_trader_or_owner`).
2. Verify position exists in `POSITIONS` (`ContractError::PositionNotFound`).
3. Enforce full close only: if `quantity` is specified, it must exactly equal `position.quantity`; partial closes are rejected with `ContractError::PartialCloseNotSupported`.
4. Release reserved margin: deduct `position.margin` from `RESERVED_MARGIN` and credit back to `TREASURY_BALANCE`.
5. Remove position record from `POSITIONS` map.
6. Book realized PnL as 0 locally (caller cannot supply a PnL value).
7. *(Pending Exchange Integration)* Querying the Injective exchange module, settling subaccount balances, and booking chain-derived PnL will replace this local release.

## State Changes
- `POSITIONS`: Maps `market_id → Position`
- `TREASURY_BALANCE`: Decremented on open, restored on close
- `RESERVED_MARGIN`: Incremented on open, decremented on close
- `LAST_POSITION_OPENED`: Timestamp updated on open

## Errors
- `ContractError::Unauthorized`: Caller is neither owner nor trader
- `ContractError::Paused`: Contract is paused (applies to `OpenPosition`; `ClosePosition` is permitted while paused)
- `ContractError::CooldownActive`: Elapsed time since last open is less than `cooldown_seconds`
- `ContractError::MarketNotAllowed`: Market ID is not in `allowed_markets`
- `ContractError::PositionAlreadyOpen`: An open position already exists for this market ID
- `ContractError::ZeroAmount`: Margin or quantity is zero
- `ContractError::InvalidPrice`: Price is zero
- `ContractError::PositionTooLarge`: Position margin or size exceeds configured caps
- `ContractError::LeverageTooHigh`: Leverage exceeds `max_leverage` ceiling
- `ContractError::InsufficientTreasury`: Free treasury cannot cover position margin
- `ContractError::ExposureLimitExceeded`: Position would exceed `max_exposure_bps`
- `ContractError::PositionNotFound`: No position open on specified market
- `ContractError::PartialCloseNotSupported`: Specified close quantity does not equal full position quantity

## Example Call (CLI)
```bash
injectived tx wasm execute $CONTRACT '{
  "open_position": {
    "market_id": "0x135de28700392fb1c17d40d5170a74f30055a4ad522feddafec42fbbbb780897",
    "is_buy": false,
    "quantity": "0.01",
    "price": "1818700000",
    "margin": "18187000000",
    "leverage": "1"
  }
}' --from $TRADER --chain-id injective-888
```

`price` is chain format (`$1,818.70` × 10^6). `quantity` is contracts, so `"0.01"` is one hundredth of a contract, not an integer minimum of 1. Notional compared with margin is `0.01 × 1_818_700_000 = 18_187_000_000`, and that is the 1x margin. The order message receives these strings unchanged.

## Notes
- Exchange module integration will utilize `injective-std 1.20.0` protobuf types once subaccount IDs and wire formatting are finalized.
- `ClosePosition` is permitted even while paused so risk can be unwound during emergency halts.