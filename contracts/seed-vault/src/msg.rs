use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Uint128;

#[cw_serde]
pub struct InstantiateMsg {
    pub admin: String,
    pub seed_denom: String,
    pub treasury_denom: String,
    /// Optional keeper. Empty means only the admin can manage positions.
    pub keeper: Option<String>,
    pub max_leverage: cosmwasm_std::Decimal,
    pub max_position_size: Uint128,
    /// Basis points of (free treasury + reserved margin). 10_000 is the whole book.
    pub max_total_margin_bps: u16,
    pub allowed_markets: Vec<String>,
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
    ProposeAdmin {
        new_admin: String,
    },
    AcceptAdmin {},
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
    #[returns(AmountResponse)]
    PendingYield { user: String },
    #[returns(PositionsResponse)]
    Positions {},
    #[returns(UserInfoResponse)]
    UserInfo { user: String },
    #[returns(DistributionInfoResponse)]
    DistributionInfo {},
}

#[cw_serde]
pub struct ConfigResponse {
    pub seed_token: String,
    pub treasury_denom: String,
    pub admin: String,
    pub paused: bool,
    pub keeper: Option<String>,
    pub max_leverage: String,
    pub max_position_size: Uint128,
    pub max_total_margin_bps: u16,
    pub allowed_markets: Vec<String>,
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
