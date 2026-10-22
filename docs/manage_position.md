# Seed Vault — Manage Position (Open / Close / Adjust)

## Overview
Admin (or keeper) opens, closes, or adjusts perpetual positions on Injective iAssets (RWA derivatives) using the vault’s treasury as margin. This is the core RWA yield generation mechanism.

## Message
```rust
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    OpenPosition {
        market_id: String,
        is_buy: bool,
        quantity: String,
        price: String,          // for limit; "0" for market
        margin: String,
        leverage: Option<String>,
    },
    ClosePosition {
        market_id: String,
        quantity: Option<String>, // None = full close
    },
    // ...
}
```

## Logic (Open Position)
1. Verify admin/keeper.
2. Check sufficient treasury balance.
3. Build `MsgCreateDerivativeMarketOrder` or `MsgCreateDerivativeLimitOrder` using `injective-std` / protobuf.
4. Wrap it as `CosmosMsg::Any` and return it in the response.
5. Track the position internally (or rely on exchange module queries).
6. Deduct margin from `TREASURY_BALANCE` (or track reserved margin).

## Logic (Close Position)
1. Query current position via exchange querier.
2. Send reduce-only order or full close.
3. Realize PnL back into treasury.

## State Changes
- `ACTIVE_POSITIONS` map (optional, for local tracking)
- `TREASURY_BALANCE` (margin reserved / released)

## Errors
- `Unauthorized`
- `MarketNotAllowed { market_id }`
- `LeverageTooHigh`
- `ExposureLimitExceeded`
- `PositionTooLarge`
- `CooldownActive`
- `InsufficientTreasury`

## Example Call
```bash
injectived tx wasm execute $CONTRACT '{
  "open_position": {
    "market_id": "0x...",
    "is_buy": true,
    "quantity": "1000000",
    "price": "0",
    "margin": "5000000"
  }
}' --from $ADMIN --chain-id injective-1
```

## Notes
- Use low leverage (1-3x) initially.
- Prefer funding-rate farming or delta-neutral strategies.
- Always set a max leverage / risk limit in config.
- Use `injective-cosmwasm` helpers for encoding the Any message.