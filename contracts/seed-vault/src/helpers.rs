use cosmwasm_std::{
    Addr, Coin, Decimal256, Int128, OverflowError, StdError, Storage, Uint128, Uint256,
};

use crate::error::ContractError;
use crate::state::{Config, GLOBAL_INDEX, USER_INDEX, USER_PENDING, USER_STAKE};

pub fn ensure_admin(config: &Config, sender: &Addr) -> Result<(), ContractError> {
    if sender != config.admin {
        return Err(ContractError::Unauthorized);
    }
    Ok(())
}

pub fn ensure_not_paused(config: &Config) -> Result<(), ContractError> {
    if config.paused {
        return Err(ContractError::Paused);
    }
    Ok(())
}

fn checked(err: OverflowError) -> ContractError {
    ContractError::Std(StdError::msg(err))
}

pub fn add_i128(a: Int128, b: Int128) -> Result<Int128, ContractError> {
    a.checked_add(b).map_err(checked)
}

pub fn add_u128(a: Uint128, b: Uint128) -> Result<Uint128, ContractError> {
    a.checked_add(b).map_err(checked)
}

pub fn sub_u128(a: Uint128, b: Uint128) -> Result<Uint128, ContractError> {
    a.checked_sub(b).map_err(checked)
}

/// Coin amounts are `Uint256` in cosmwasm-std 3. Stake only accepts values that fit `Uint128`.
pub fn coin_u128(amount: Uint256) -> Result<Uint128, ContractError> {
    Uint128::try_from(amount).map_err(|err| ContractError::Std(StdError::msg(err.to_string())))
}

/// `earned = stake * (global_index - user_index)`, rounded down.
pub fn reward_for(stake: Uint128, index_delta: Decimal256) -> Result<Uint128, ContractError> {
    if stake.is_zero() || index_delta.is_zero() {
        return Ok(Uint128::zero());
    }
    let earned = Decimal256::from_ratio(stake, 1u128)
        .checked_mul(index_delta)
        .map_err(checked)?
        .to_uint_floor();
    coin_u128(earned)
}

/// Settle a user before anything that changes their stake, and at the start of claim.
/// A missing `USER_INDEX` is a first stake: set it to the current global index and earn nothing.
pub fn settle(storage: &mut dyn Storage, user: &Addr) -> Result<(), ContractError> {
    let global = GLOBAL_INDEX.may_load(storage)?.unwrap_or_default();
    let user_index = USER_INDEX.may_load(storage, user)?;

    if let Some(user_index) = user_index {
        let stake = USER_STAKE.may_load(storage, user)?.unwrap_or_default();
        let delta = global.checked_sub(user_index).map_err(checked)?;
        let earned = reward_for(stake, delta)?;
        USER_PENDING.update(storage, user, |pending| -> Result<_, ContractError> {
            add_u128(pending.unwrap_or_default(), earned)
        })?;
    }

    USER_INDEX.save(storage, user, &global)?;
    Ok(())
}

/// Pending yield including earnings not yet written. Queries use this so they stay read-only.
pub fn pending_yield(storage: &dyn Storage, user: &Addr) -> Result<Uint128, ContractError> {
    let pending = USER_PENDING.may_load(storage, user)?.unwrap_or_default();
    let Some(user_index) = USER_INDEX.may_load(storage, user)? else {
        return Ok(pending);
    };
    let global = GLOBAL_INDEX.may_load(storage)?.unwrap_or_default();
    let stake = USER_STAKE.may_load(storage, user)?.unwrap_or_default();
    let delta = global.checked_sub(user_index).map_err(checked)?;
    add_u128(pending, reward_for(stake, delta)?)
}

/// Exactly one non-zero coin of `denom`.
pub fn one_coin<'a>(funds: &'a [Coin], denom: &str) -> Result<&'a Coin, ContractError> {
    match funds {
        [coin] if coin.denom == denom => {
            if coin.amount.is_zero() {
                return Err(ContractError::ZeroAmount);
            }
            Ok(coin)
        }
        [coin] => Err(ContractError::WrongDenom {
            expected: denom.to_string(),
            got: coin.denom.clone(),
        }),
        _ => Err(ContractError::WrongDenom {
            expected: denom.to_string(),
            got: funds
                .iter()
                .map(|coin| coin.denom.clone())
                .collect::<Vec<_>>()
                .join(","),
        }),
    }
}
