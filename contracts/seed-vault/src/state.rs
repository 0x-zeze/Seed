use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Decimal, Decimal256, Int128, Uint128};
use cw_storage_plus::{Item, Map};

pub const CONTRACT_NAME: &str = "crates.io:seed-vault";
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Quote decimals used to turn chain price × quantity into quote base units.
///
/// Assumption: every allowed market quotes a 6-decimal stablecoin (USDT/USDC),
/// matching the preferred ETH/INJ/GOLD perps. Chain price is human price × 10^6.
/// Quantity is already in contract units (a fractional string such as "0.01" is
/// 0.01 contracts, not 0.01 × 10^6). Margin is quote base units (micro-USDT).
///
/// Human notional in quote base units is therefore
/// `quantity * price / 10^QUOTE_DECIMALS`, not the raw product. This constant is
/// not configurable per market.
pub const QUOTE_DECIMALS: u32 = 6;

#[cw_serde]
pub struct Config {
    pub owner: Addr,
    /// Native `$SEED` denom from Sprout.fun (TokenFactory).
    pub seed_denom: String,
    /// Margin / yield denom, e.g. a peggy USDT denom.
    pub treasury_denom: String,
    pub paused: bool,
    /// Trader may open and close positions. `None` means owner only.
    pub trader: Option<Addr>,
    /// Markets the vault is allowed to trade. Empty means none.
    pub allowed_markets: Vec<String>,
    /// Maximum leverage, e.g. `3` means 3x. Compared against margin and notional. Default 3x.
    pub max_leverage: Decimal,
    /// Maximum order quantity, in the market's base units.
    pub max_position_size: Uint128,
    /// Total reserved margin across open positions must be <= this % of treasury in bps. Default 5000 (50%).
    pub max_exposure_bps: u16,
    /// Cap per single position's margin.
    pub max_position_margin: Uint128,
    /// Minimum time in seconds between two OpenPosition calls.
    pub cooldown_seconds: u64,
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

/// Second step of an ownership transfer. Empty until `ProposeOwner`.
pub const PENDING_OWNER: Item<Addr> = Item::new("pending_owner");


/// Timestamp (in seconds) of the last OpenPosition call. Zero before the first.
pub const LAST_POSITION_OPENED: Item<u64> = Item::new("last_position_opened");

#[cw_serde]
pub struct Position {
    pub market_id: String,
    pub is_buy: bool,
    /// Contract quantity. May be fractional (`"0.01"` is 0.01 contracts).
    pub quantity: Decimal256,
    pub price: Uint128,
    pub margin: Uint128,
    pub leverage: Option<String>,
    pub opened_at: u64,
}

/// Active open positions keyed by market_id.
pub const POSITIONS: Map<&str, Position> = Map::new("positions");

