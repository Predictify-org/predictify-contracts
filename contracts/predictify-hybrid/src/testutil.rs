//! Shared Soroban fixtures, following the setup pattern in `test.rs`.
use crate::{PredictifyHybrid, PredictifyHybridClient};
use soroban_sdk::{
    testutils::{Address as _, EnvTestConfig},
    token::StellarAssetClient,
    Address, Env, Symbol,
};

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
        env.set_config(EnvTestConfig {
            capture_snapshot_at_drop: false,
        });
        env.mock_all_auths();
        let admin = Address::generate(&env);
        let token = env
            .register_stellar_asset_contract_v2(admin.clone())
            .address();
        let contract_id = env.register(PredictifyHybrid, ());
        Self {
            env,
            contract_id,
            admin,
            token,
        }
    }

    pub fn new() -> Self {
        let ctx = Self::uninitialized();
        ctx.client().initialize(&ctx.admin, &None, &None);
        ctx.with_contract(|| {
            ctx.env
                .storage()
                .persistent()
                .set(&Symbol::new(&ctx.env, "TokenID"), &ctx.token)
        });
        ctx
    }

    pub fn client(&self) -> PredictifyHybridClient<'_> {
        PredictifyHybridClient::new(&self.env, &self.contract_id)
    }

    pub fn with_contract<T>(&self, f: impl FnOnce() -> T) -> T {
        self.env.as_contract(&self.contract_id, f)
    }

    /// Create an active market using the same initialized contract and token.
    pub fn market(&self) -> Symbol {
        use crate::{OracleConfig, OracleProvider};
        use soroban_sdk::{vec, String};
        self.client().create_market(
            &self.admin,
            &String::from_str(&self.env, "Will BTC exceed the target?"),
            &vec![
                &self.env,
                String::from_str(&self.env, "yes"),
                String::from_str(&self.env, "no"),
            ],
            &30,
            &OracleConfig::new(
                OracleProvider::reflector(),
                Address::generate(&self.env),
                String::from_str(&self.env, "BTC"),
                25_000,
                String::from_str(&self.env, "gt"),
            ),
            &None,
            &86_400,
            &None,
            &None,
            &None,
            &None,
            &None,
        )
    }

    pub fn funded_user(&self, amount: i128) -> Address {
        let user = Address::generate(&self.env);
        StellarAssetClient::new(&self.env, &self.token).mint(&user, &amount);
        user
    }
}

#[test]
fn storage_writes_are_visible_through_client() {
    use crate::{storage::BalanceStorage, types::ReflectorAsset};
    let ctx = TestCtx::new();
    let user = ctx.funded_user(100);
    ctx.with_contract(|| {
        BalanceStorage::add_balance(&ctx.env, &user, &ReflectorAsset::Stellar, 42).unwrap()
    });
    assert_eq!(
        ctx.client()
            .get_balance(&user, &ReflectorAsset::Stellar)
            .amount,
        42
    );
}

#[test]
fn market_and_funding_helpers_share_the_initialized_contract() {
    use soroban_sdk::token::Client;
    let ctx = TestCtx::new();
    let user = ctx.funded_user(123);
    assert_eq!(Client::new(&ctx.env, &ctx.token).balance(&user), 123);
    let id = ctx.market();
    let market = ctx.client().get_market(&id).unwrap();
    assert_eq!(market.total_staked, 0);
}
