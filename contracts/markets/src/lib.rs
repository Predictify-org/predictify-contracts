#![no_std]

use soroban_sdk::{contract, contractimpl, Env};

#[contract]
pub struct MarketsContract;

#[contractimpl]
impl MarketsContract {
    /// Returns the deployed markets contract API version.
    ///
    /// This read-only entrypoint exposes the contract version so clients can
    /// verify that they are communicating with the expected markets contract
    /// interface before invoking other operations. It returns the compile-time
    /// version identifier and does not modify contract state.
    ///
    /// # Returns
    ///
    /// The markets contract version, currently `7`.
    ///
    /// # Errors
    ///
    /// This function does not emit or return contract errors.
    pub fn version(_env: Env) -> u32 {
        7
    }
}

#[cfg(test)]
mod tests {
    use super::MarketsContract;
    use soroban_sdk::Env;

    #[test]
    fn version_returns_current_contract_version() {
        assert_eq!(MarketsContract::version(Env::default()), 7);
    }
}
