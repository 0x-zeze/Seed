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
    // ...
}
```

## Responses

### Config
```json
{
  "seed_token": "factory/...",
  "treasury_denom": "usdt",
  "admin": "inj1...",
  "paused": false
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