use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Decimal, Decimal256, Int128, Uint128};
use cw_storage_plus::{Item, Map};

pub const CONTRACT_NAME: &str = "crates.io:seed-vault";
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cw_serde]
pub struct Config {
    pub admin: Addr,
    /// Native `$SEED` denom from Sprout.fun (TokenFactory).
    pub seed_denom: String,
    /// Margin / yield denom, e.g. a peggy USDT denom.
    pub treasury_denom: String,
    pub paused: bool,
    /// Keeper may open and close positions. `None` means admin only.
    pub keeper: Option<Addr>,
    /// Maximum leverage, e.g. `3` means 3x. Compared against margin and notional.
    pub max_leverage: Decimal,
    /// Maximum order quantity, in the market's base units.
    pub max_position_size: Uint128,
    /// Share of free treasury plus already reserved margin that may be reserved, in basis points.
    pub max_total_margin_bps: u16,
    /// Markets the vault is allowed to trade. Empty means none.
    pub allowed_markets: Vec<String>,
}

pub const CONFIG: Item<Config> = Item::new("config");

/// Staked `$SEED` per user. 1:1 with the amount sent to `Stake`.
pub const USER_STAKE: Map<&Addr, Uint128> = Map::new("user_stake");
/// Sum of every `USER_STAKE` balance.
pub const TOTAL_STAKED: Item<Uint128> = Item::new("total_staked");
/// Free treasury (USDT) not reserved for an open position or unclaimed yield.
pub const TREASURY_BALANCE: Item<Uint128> = Item::new("treasury_balance");
/// Margin moved out of free treasury for open positions and not yet released.
pub const RESERVED_MARGIN: Item<Uint128> = Item::new("reserved_margin");

/// Reward per 1 staked token. Raised only by `Distribute`.
pub const GLOBAL_INDEX: Item<Decimal256> = Item::new("global_index");
/// Index at the user's last settle. A new staker starts at `GLOBAL_INDEX`.
pub const USER_INDEX: Map<&Addr, Decimal256> = Map::new("user_index");
/// Yield already settled for the user and not yet claimed.
pub const USER_PENDING: Map<&Addr, Uint128> = Map::new("user_pending");
/// Lifetime yield already paid out, for the dashboard.
pub const USER_CLAIMED: Map<&Addr, Uint128> = Map::new("user_claimed");

/// Distributed but not yet claimed. Later trading losses cannot touch this.
pub const REWARDS_RESERVED: Item<Uint128> = Item::new("rewards_reserved");
/// Cumulative profit already moved into the reward index (high-water mark).
pub const TOTAL_DISTRIBUTED: Item<Uint128> = Item::new("total_distributed");
/// Cumulative realized PnL in treasury-denom base units. Losses are negative.
pub const REALIZED_PNL_NET: Item<Int128> = Item::new("realized_pnl_net");
/// Block height of the last successful distribution. Zero before the first.
pub const LAST_DISTRIBUTION: Item<u64> = Item::new("last_distribution");

/// Second step of an admin transfer. Empty until `ProposeAdmin`.
pub const PENDING_ADMIN: Item<Addr> = Item::new("pending_admin");
