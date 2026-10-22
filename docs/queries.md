# Seed Vault — Query Messages

## Overview
Read-only queries for the frontend and users.

## Query Messages
```rust
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Config {},
    UserStake { user: String },
    TotalStaked {},
    TreasuryBalance {},
    PendingYield { user: String },
    Positions {},
    RiskConfig {},
    // ...
}
```

## Responses

### Config
```json
{
  "seed_token": "factory/...",
  "treasury_denom": "usdt",
  "owner": "inj1...",
  "paused": false,
  "trader": "inj1...",
  "max_leverage": "3",
  "max_position_size": "10000000",
  "max_exposure_bps": 5000,
  "max_total_margin_bps": 5000,
  "max_position_margin": "5000000",
  "cooldown_seconds": 60,
  "allowed_markets": ["0x..."]
}
```

### RiskConfig
```json
{
  "allowed_markets": ["0x..."],
  "max_leverage": "3",
  "max_exposure_bps": 5000,
  "max_position_margin": "5000000",
  "cooldown_seconds": 60,
  "current_exposure_bps": 2500,
  "current_used_exposure": "2500000",
  "reserved_margin": "2500000",
  "total_treasury": "10000000"
}
```

### UserStake
```json
{
  "amount": "100000000"
}
```

### PendingYield
```json
{
  "amount": "1250000"
}
```

## Example CLI
```bash
injectived query wasm contract-state smart $CONTRACT '{"user_stake":{"user":"inj1..."}}'
injectived query wasm contract-state smart $CONTRACT '{"pending_yield":{"user":"inj1..."}}'
```

## Notes
- Also expose exchange module queries (positions, funding) by wrapping them or documenting direct queries to the indexer.