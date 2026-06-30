//! Per-market multi-signer dispute resolution (issue #731).

use soroban_sdk::{contracttype, Address, Env, String, Symbol, Vec};
use crate::err::Error;

#[contracttype]
#[derive(Clone, Debug)]
pub struct MultiSigDisputeState {
    pub market_id: Symbol,
    pub threshold: u32,
    pub signers: Vec<Address>,
    pub approvals: Vec<Address>,
    pub proposed_outcome: String,
}

pub struct DisputeMultiSig;

impl DisputeMultiSig {
    fn key(env: &Env, market_id: &Symbol) -> (Symbol, Symbol) {
        (Symbol::new(env, "dms"), market_id.clone())
    }

    pub fn configure(
        env: &Env,
        admin: Address,
        market_id: Symbol,
        signers: Vec<Address>,
        threshold: u32,
        proposed_outcome: String,
    ) -> Result<(), Error> {
        admin.require_auth();
        if signers.is_empty() { return Err(Error::InvalidInput); }
        if threshold == 0 || threshold > signers.len() { return Err(Error::InvalidInput); }
        let state = MultiSigDisputeState {
            market_id: market_id.clone(), threshold, signers,
            approvals: Vec::new(env), proposed_outcome,
        };
        env.storage().instance().set(&Self::key(env, &market_id), &state);
        Ok(())
    }

    pub fn approve(env: &Env, signer: Address, market_id: Symbol) -> Result<bool, Error> {
        signer.require_auth();
        let key = Self::key(env, &market_id);
        let mut state: MultiSigDisputeState = env.storage().instance().get(&key).ok_or(Error::MarketNotFound)?;
        if !state.signers.iter().any(|s| s == signer) { return Err(Error::Unauthorized); }
        if state.approvals.iter().any(|s| s == signer) { return Err(Error::InvalidState); }
        state.approvals.push_back(signer);
        let reached = state.approvals.len() >= state.threshold;
        if reached { env.storage().instance().remove(&key); } else { env.storage().instance().set(&key, &state); }
        Ok(reached)
    }

    pub fn get_state(env: &Env, market_id: &Symbol) -> Option<MultiSigDisputeState> {
        env.storage().instance().get(&Self::key(env, market_id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Env};

    #[test]
    fn test_threshold_one_resolves_on_single_approval() {
        let env = Env::default(); env.mock_all_auths();
        let admin = Address::generate(&env);
        let s = Address::generate(&env);
        let mid = Symbol::new(&env, "mkt1");
        DisputeMultiSig::configure(&env, admin, mid.clone(), soroban_sdk::vec![&env, s.clone()], 1, String::from_str(&env, "YES")).unwrap();
        assert!(DisputeMultiSig::approve(&env, s, mid.clone()).unwrap());
        assert!(DisputeMultiSig::get_state(&env, &mid).is_none());
    }

    #[test]
    fn test_two_of_two_requires_both() {
        let env = Env::default(); env.mock_all_auths();
        let admin = Address::generate(&env);
        let s1 = Address::generate(&env); let s2 = Address::generate(&env);
        let mid = Symbol::new(&env, "mkt2");
        DisputeMultiSig::configure(&env, admin, mid.clone(), soroban_sdk::vec![&env, s1.clone(), s2.clone()], 2, String::from_str(&env, "NO")).unwrap();
        assert!(!DisputeMultiSig::approve(&env, s1, mid.clone()).unwrap());
        assert!(DisputeMultiSig::approve(&env, s2, mid.clone()).unwrap());
    }

    #[test]
    fn test_threshold_zero_rejected() {
        let env = Env::default(); env.mock_all_auths();
        let admin = Address::generate(&env); let s = Address::generate(&env);
        assert!(DisputeMultiSig::configure(&env, admin, Symbol::new(&env, "m"), soroban_sdk::vec![&env, s], 0, String::from_str(&env, "X")).is_err());
    }

    #[test]
    fn test_unauthorised_signer_rejected() {
        let env = Env::default(); env.mock_all_auths();
        let admin = Address::generate(&env); let auth = Address::generate(&env); let intruder = Address::generate(&env);
        let mid = Symbol::new(&env, "mkt3");
        DisputeMultiSig::configure(&env, admin, mid.clone(), soroban_sdk::vec![&env, auth], 1, String::from_str(&env, "YES")).unwrap();
        assert!(DisputeMultiSig::approve(&env, intruder, mid).is_err());
    }
}
