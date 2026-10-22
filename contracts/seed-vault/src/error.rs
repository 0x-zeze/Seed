use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("unauthorized")]
    Unauthorized,

    #[error("contract is paused")]
    Paused,

    #[error("wrong denom: expected {expected}, got {got}")]
    WrongDenom { expected: String, got: String },

    #[error("zero amount")]
    ZeroAmount,

    #[error("insufficient stake: have {have}, need {need}")]
    InsufficientStake { have: String, need: String },

    #[error("no stakers")]
    NoStakers,

    #[error("nothing to claim")]
    NothingToClaim,

    #[error("invalid amount")]
    InvalidAmount,

    #[error("insufficient treasury: free {free}, need {need}")]
    InsufficientTreasury { free: String, need: String },

    #[error("market {market_id} is not allowed")]
    MarketNotAllowed { market_id: String },

    #[error("leverage too high")]
    LeverageTooHigh,

    #[error("exposure limit exceeded")]
    ExposureLimitExceeded,

    #[error("position too large")]
    PositionTooLarge,

    #[error("cooldown active")]
    CooldownActive,

    #[error("position exceeds a risk limit: {reason}")]
    RiskLimit { reason: String },

    #[error("{msg} is not implemented")]
    NotImplemented { msg: String },
}
