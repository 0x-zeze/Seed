# Seed Vault — Query Messages

## Overview
Read-only queries for frontends, dashboards, and client integrations.

## Query Messages
```rust
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(ConfigResponse)]
    Config {},
    #[returns(UserStakeResponse)]
    UserStake { user: String },
    #[returns(AmountResponse)]
    TotalStaked {},
    #[returns(AmountResponse)]
    TreasuryBalance {},
    #[returns(TreasuryStateResponse)]
    TreasuryState {},
    #[returns(AmountResponse)]
    PendingYield { user: String },
    #[returns(PositionsResponse)]
    Positions {},
    #[returns(OpenPositionsResponse)]
    OpenPositions {
        start_after: Option<String>,
        limit: Option<u32>,
    },
    #[returns(UserInfoResponse)]
    UserInfo { user: String },
    #[returns(DistributionInfoResponse)]
    DistributionInfo {},
    #[returns(RiskConfigResponse)]
    RiskConfig {},
}
```

## Responses

### Config
```json
{
  "seed_token": "factory/inj1.../seed",
  "treasury_denom": "inj",
  "owner": "inj1e9vlszrj8p3e8zvamt386chuefm782xg46uw7k",
  "paused": false,
  "trader": "inj1e9vlszrj8p3e8zvamt386chuefm782xg46uw7k",
  "max_leverage": "2",
  "max_position_size": "1000000000000000000",
  "max_total_margin_bps": 3000,
  "max_exposure_bps": 3000,
  "max_position_margin": "1000000000000000000",
  "cooldown_seconds": 300,
  "allowed_markets": [
    "0x0511ddc4e6586f3bfe1acb2dd905f8b8a82c97e1edaef65470f6a2cdbb493b60"
  ]
}
```

### RiskConfig
```json
{
  "allowed_markets": [
    "0x0511ddc4e6586f3bfe1acb2dd905f8b8a82c97e1edaef65470f6a2cdbb493b60"
  ],
  "max_leverage": "2",
  "max_exposure_bps": 3000,
  "max_position_margin": "1000000000000000000",
  "cooldown_seconds": 300,
  "current_exposure_bps": 0,
  "current_used_exposure": "0",
  "reserved_margin": "0",
  "total_treasury": "1000000000000000000"
}
```

### TreasuryState
```json
{
  "balance": "1000000000000000000",
  "reserved_margin": "0",
  "undistributed": "0"
}
```

### UserInfo
```json
{
  "stake": "1000000000000000000",
  "pending": "0",
  "claimed": "0",
  "share": "1"
}
```

### DistributionInfo
```json
{
  "total_distributed": "0",
  "rewards_reserved": "0",
  "last_distribution": 0,
  "global_index": "0"
}
```

### UserStake
```json
{
  "amount": "1000000000000000000"
}
```

### PendingYield
```json
{
  "amount": "0"
}
```

## Example CLI (Testnet)
```bash
injectived query wasm contract-state smart $CONTRACT '{"config":{}}' --chain-id injective-888
injectived query wasm contract-state smart $CONTRACT '{"risk_config":{}}' --chain-id injective-888
injectived query wasm contract-state smart $CONTRACT '{"treasury_state":{}}' --chain-id injective-888
injectived query wasm contract-state smart $CONTRACT '{"user_stake":{"user":"inj1..."}}' --chain-id injective-888
injectived query wasm contract-state smart $CONTRACT '{"pending_yield":{"user":"inj1..."}}' --chain-id injective-888
```