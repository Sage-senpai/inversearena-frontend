#![no_std]

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, panic_with_error, symbol_short, Address,
    Env, Symbol, Vec,
};

const ADMIN_KEY: Symbol = symbol_short!("ADMIN");
const TREASURY_KEY: Symbol = symbol_short!("TREAS");
const TOPIC_PAYOUT_EXECUTED: Symbol = symbol_short!("PAYOUT");
const TOPIC_DUST_COLLECTED: Symbol = symbol_short!("DUST");

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    Payout(u32, Address),
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct PayoutData {
    pub winner: Address,
    pub amount: i128,
    pub currency: Symbol,
    pub paid: bool,
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum PayoutError {
    UnauthorizedCaller = 1,
    InvalidAmount = 2,
    AlreadyPaid = 3,
    NoWinners = 4,
    TreasuryNotSet = 5,
}

#[contract]
pub struct PayoutContract;

#[contractimpl]
impl PayoutContract {
    /// Placeholder function — returns a fixed value for contract liveness checks.
    ///
    /// # Arguments
    /// * `env` - The Soroban environment.
    ///
    /// # Authorization
    /// None — open to any caller.
        pub fn hello(_env: Env) -> u32 {
        789
    }

    pub fn initialize(env: Env, admin: Address) {
        if env.storage().instance().has(&ADMIN_KEY) {
            panic!("already initialized");
        }
        env.storage().instance().set(&ADMIN_KEY, &admin);
    }

    pub fn admin(env: Env) -> Address {
        env.storage()
            .instance()
            .get(&ADMIN_KEY)
            .expect("not initialized")
    }

    pub fn set_treasury(env: Env, treasury: Address) {
        let admin = Self::admin(env.clone());
        admin.require_auth();
        env.storage().instance().set(&TREASURY_KEY, &treasury);
    }

    pub fn treasury(env: Env) -> Result<Address, PayoutError> {
        env.storage()
            .instance()
            .get(&TREASURY_KEY)
            .ok_or(PayoutError::TreasuryNotSet)
    }

    pub fn distribute_winnings(
        env: Env,
        caller: Address,
        idempotency_key: u32,
        winner: Address,
        amount: i128,
        currency: Symbol,
    ) -> Result<(), PayoutError> {
        let admin: Address = env
            .storage()
            .instance()
            .get(&ADMIN_KEY)
            .expect("not initialized");

        if caller != admin {
            panic_with_error!(&env, PayoutError::UnauthorizedCaller);
        }

        caller.require_auth();

        if amount <= 0 {
            panic_with_error!(&env, PayoutError::InvalidAmount);
        }

        let payout_key = DataKey::Payout(idempotency_key, winner.clone());
        if env
            .storage()
            .instance()
            .get::<_, PayoutData>(&payout_key)
            .is_some()
        {
            panic_with_error!(&env, PayoutError::AlreadyPaid);
        }

        let payout_data = PayoutData {
            winner: winner.clone(),
            amount,
            currency: currency.clone(),
            paid: true,
        };
        env.storage().instance().set(&payout_key, &payout_data);

        env.events()
            .publish((TOPIC_PAYOUT_EXECUTED,), (winner, amount, currency));

        Ok(())
    }

    pub fn is_payout_processed(env: Env, idempotency_key: u32, winner: Address) -> bool {
        let payout_key = DataKey::Payout(idempotency_key, winner);
        env.storage()
            .instance()
            .get::<_, PayoutData>(&payout_key)
            .map(|p| p.paid)
            .unwrap_or(false)
    }

    pub fn get_payout(env: Env, idempotency_key: u32, winner: Address) -> Option<PayoutData> {
        let payout_key = DataKey::Payout(idempotency_key, winner);
        env.storage().instance().get(&payout_key)
    }

    pub fn distribute_prize(
        env: Env,
        total_prize: i128,
        winners: Vec<Address>,
        currency: Address,
    ) -> Result<(), PayoutError> {
        let admin = Self::admin(env.clone());
        admin.require_auth();

        if total_prize <= 0 {
            return Err(PayoutError::InvalidAmount);
        }
        if winners.is_empty() {
            return Err(PayoutError::NoWinners);
        }

        let treasury = Self::treasury(env.clone())?;
        let count = winners.len() as i128;
        let share = total_prize / count;
        let dust = total_prize % count;

        for winner in winners.iter() {
            env.events()
                .publish((TOPIC_PAYOUT_EXECUTED,), (winner, share, currency.clone()));
        }

        if dust > 0 {
            env.events()
                .publish((TOPIC_DUST_COLLECTED,), (treasury, dust, currency));
        }

        Ok(())
    }
}

#[cfg(test)]
mod test;
