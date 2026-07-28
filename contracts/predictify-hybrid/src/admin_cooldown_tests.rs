use crate::voting::VotingValidator;

#[cfg(test)]
mod tests {
    use soroban_sdk::{testutils::Address as _, Address, Env};

    #[test]
    fn admin_action_respects_cooldown() {
        let env = Env::default();

        let admin = Address::generate(&env);

        VotingValidator::record_admin_action(&env);

        let result =
            VotingValidator::validate_admin_cooldown(&env);

        assert!(result.is_err());
    }

    #[test]
    fn admin_action_allowed_after_cooldown() {
        let env = Env::default();

        let admin = Address::generate(&env);

        VotingValidator::record_admin_action(&env);

        env.ledger()
            .set_timestamp(
                env.ledger().timestamp() + 3601
            );

        let result =
            VotingValidator::validate_admin_cooldown(&env);

        assert!(result.is_ok());
    }
}