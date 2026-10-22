use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Uint128;

use crate::state::Position;

#[cw_serde]
pub struct InstantiateMsg {
    pub owner: String,
    pub seed_denom: String,
    pub treasury_denom: String,
    /// Optional trader. Empty means only the owner can manage positions.
    pub trader: Option<String>,
    pub allowed_markets: Vec<String>,
    #[serde(default)]
    pub max_leverage: Option<cosmwasm_std::Decimal>,
    #[serde(default)]
    pub max_position_size: Option<Uint128>,
    #[serde(default)]
    pub max_exposure_bps: Option<u16>,
    /// Alias for backwards compatibility with earlier callers
    #[serde(default)]
    pub max_total_margin_bps: Option<u16>,
    #[serde(default)]
    pub max_position_margin: Option<Uint128>,
    #[serde(default)]
    pub cooldown_seconds: Option<u64>,
}

#[cw_serde]
pub enum ExecuteMsg {
    Stake {},
    Unstake {
        amount: Uint128,
    },
    DepositTreasury {},
    Distribute {
        amount: Uint128,
    },
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
        quantity: Option<String>,
    },
    Claim {},
    Pause {},
    Unpause {},
    ProposeOwner {
        new_owner: String,
    },
    AcceptOwner {},
    UpdateTrader {
        new_trader: Option<String>,
    },
    UpdateConfig {
        max_leverage: Option<cosmwasm_std::Decimal>,
        max_position_size: Option<Uint128>,
        max_exposure_bps: Option<u16>,
        max_total_margin_bps: Option<u16>,
        max_position_margin: Option<Uint128>,
        cooldown_seconds: Option<u64>,
        allowed_markets: Option<Vec<String>>,
    },
    /// TEMPORARY: test-only way to set realized PnL until position close updates it.
    #[cfg(test)]
    RecordPnl {
        delta: cosmwasm_std::Int128,
    },
}

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
        #[serde(default)]
        start_after: Option<String>,
        #[serde(default)]
        limit: Option<u32>,
    },
    #[returns(UserInfoResponse)]
    UserInfo { user: String },
    #[returns(DistributionInfoResponse)]
    DistributionInfo {},
    #[returns(RiskConfigResponse)]
    RiskConfig {},
}

#[cw_serde]
pub struct ConfigResponse {
    pub seed_token: String,
    pub treasury_denom: String,
    pub owner: String,
    pub paused: bool,
    pub trader: Option<String>,
    pub max_leverage: String,
    pub max_position_size: Uint128,
    pub max_total_margin_bps: u16,
    pub max_exposure_bps: u16,
    pub max_position_margin: Uint128,
    pub cooldown_seconds: u64,
    pub allowed_markets: Vec<String>,
}

#[cw_serde]
pub struct RiskConfigResponse {
    pub allowed_markets: Vec<String>,
    pub max_leverage: String,
    pub max_exposure_bps: u16,
    pub max_position_margin: Uint128,
    pub cooldown_seconds: u64,
    pub current_exposure_bps: u16,
    pub current_used_exposure: Uint128,
    pub reserved_margin: Uint128,
    pub total_treasury: Uint128,
}

#[cw_serde]
pub struct UserStakeResponse {
    pub amount: Uint128,
}

#[cw_serde]
pub struct AmountResponse {
    pub amount: Uint128,
}

#[cw_serde]
pub struct PositionsResponse {
    pub positions: Vec<String>,
}

#[cw_serde]
pub struct TreasuryStateResponse {
    pub balance: Uint128,
    pub reserved_margin: Uint128,
    pub undistributed: Uint128,
}

#[cw_serde]
pub struct OpenPositionsResponse {
    pub positions: Vec<Position>,
}

pub type TotalStakedResponse = AmountResponse;


#[cw_serde]
pub struct UserInfoResponse {
    pub stake: Uint128,
    pub pending: Uint128,
    pub claimed: Uint128,
    pub share: String,
}

#[cw_serde]
pub struct DistributionInfoResponse {
    pub total_distributed: Uint128,
    pub rewards_reserved: Uint128,
    pub last_distribution: u64,
    pub global_index: String,
}
