//! Shared Soroban fixtures, following the setup pattern in `test.rs`.
use crate::{PredictifyHybrid, PredictifyHybridClient};
use soroban_sdk::{testutils::{Address as _, EnvTestConfig}, token::StellarAssetClient, Address, Env, Symbol};

pub(crate) struct TestCtx {
    pub env: Env,
    pub contract_id: Address,
    pub admin: Address,
    pub token: Address,
}

impl TestCtx {
    /// Empty contract storage for default and legacy-fallback tests.
    pub fn uninitialized() -> Self {
        let mut env = Env::default();
        env.set_config(EnvTestConfig { capture_snapshot_at_drop: false });
        env.mock_all_auths();
        let admin = Address::generate(&env);
        let token = env.register_stellar_asset_contract_v2(admin.clone()).address();
        let contract_id = env.register(PredictifyHybrid, ());
        Self { env, contract_id, admin, token }
    }

    pub fn new() -> Self {
        let ctx = Self::uninitialized();
        ctx.client().initialize(&ctx.admin, &None, &None);
        ctx.with_contract(|| ctx.env.storage().persistent().set(&Symbol::new(&ctx.env, "TokenID"), &ctx.token));
        ctx
    }

    pub fn client(&self) -> PredictifyHybridClient<'_> {
        PredictifyHybridClient::new(&self.env, &self.contract_id)
    }

    pub fn with_contract<T>(&self, f: impl FnOnce() -> T) -> T {
        self.env.as_contract(&self.contract_id, f)
    }

    pub fn funded_user(&self, amount: i128) -> Address {
        let user = Address::generate(&self.env);
        StellarAssetClient::new(&self.env, &self.token).mint(&user, &amount);
        user
    }
}
