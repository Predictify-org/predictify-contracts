#![cfg(test)]

use predictify_hybrid::{
    Error, OracleConfig, OracleProvider, PredictifyHybrid, PredictifyHybridClient,
    Market, MarketState,
};
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token::StellarAssetClient,
    Address, Env, String, Symbol, Vec, Map,
};

// Fixture setup, similar to predictify-hybrid's auth_snapshot tests
struct Fixture {
    env: Env,
    cid: Address,
    admin: Address,
    token_id: Address,
}

impl Fixture {
    fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let cid = env.register(PredictifyHybrid, ());

        // Register a Stellar asset so the contract's token client resolves.
        let token_id = env
            .register_stellar_asset_contract_v2(Address::generate(&env))
            .address();

        // Wire the token before initializing so stake transfers work.
        env.as_contract(&cid, || {
            env.storage()
                .persistent()
                .set(&Symbol::new(&env, "TokenID"), &token_id);
        });

        PredictifyHybridClient::new(&env, &cid).initialize(&admin, &Some(200i128), &None);

        Fixture {
            env,
            cid,
            admin,
            token_id,
        }
    }

    fn client(&self) -> PredictifyHybridClient<'_> {
        PredictifyHybridClient::new(&self.env, &self.cid)
    }

    fn user(&self) -> Address {
        let u = Address::generate(&self.env);
        StellarAssetClient::new(&self.env, &self.token_id).mint(&u, &100_000_000_000i128);
        u
    }

    fn oracle(&self) -> OracleConfig {
        OracleConfig {
            provider: OracleProvider::reflector(),
            oracle_address: Address::from_str(
                &self.env,
                "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF",
            ),
            feed_id: String::from_str(&self.env, "BTC/USD"),
            threshold: 50_000,
            comparison: String::from_str(&self.env, "gt"),
        }
    }

    fn market(&self) -> Symbol {
        let mut outcomes = Vec::new(&self.env);
        outcomes.push_back(String::from_str(&self.env, "yes"));
        outcomes.push_back(String::from_str(&self.env, "no"));
        self.client().create_market(
            &self.admin,
            &String::from_str(&self.env, "Will BTC reach 100k?"),
            &outcomes,
            &30u32,
            &self.oracle(),
            &None,
            &86_400u64,
            &None,
            &None,
            &None,
        )
    }

    fn advance_past_end(&self) {
        self.env
            .ledger()
            .with_mut(|l| l.timestamp += 31 * 24 * 60 * 60);
    }
}

/// Helper to measure CPU and memory costs of a contract invocation.
fn measure_gas<F, R>(env: &Env, f: F) -> (u64, u64, R)
where
    F: FnOnce() -> R,
{
    env.budget().reset_default();
    let start_cpu = env.budget().cpu_instruction_cost();
    let start_mem = env.budget().memory_allocation_size_in_bytes();
    let res = f();
    let end_cpu = env.budget().cpu_instruction_cost();
    let end_mem = env.budget().memory_allocation_size_in_bytes();
    let cpu_used = end_cpu.saturating_sub(start_cpu);
    let mem_used = end_mem.saturating_sub(start_mem);
    (cpu_used, mem_used, res)
}

#[test]
fn test_gas_resolve_market_manual() {
    let f = Fixture::new();
    let market_id = f.market();
    
    // Add votes for realistic resolution state
    let user1 = f.user();
    let user2 = f.user();
    f.client().vote(&user1, &market_id, &String::from_str(&f.env, "yes"), &10_000_000i128);
    f.client().vote(&user2, &market_id, &String::from_str(&f.env, "no"), &5_000_000i128);
    
    f.advance_past_end();
    let outcome = String::from_str(&f.env, "yes");

    let (cpu, mem, _) = measure_gas(&f.env, || {
        f.client().resolve_market_manual(&f.admin, &market_id, &outcome);
    });
    
    std::println!("=== Gas Snapshot: resolve_market_manual ===");
    std::println!("CPU Instructions: {}", cpu);
    std::println!("Memory Bytes:     {}", mem);
    std::println!("===========================================");

    // Ample headroom thresholds for test environment stability
    assert!(cpu < 10_000_000, "CPU cost too high: {}", cpu);
    assert!(mem < 2_000_000, "Memory cost too high: {}", mem);
}

#[test]
fn test_gas_resolve_market_with_ties() {
    let f = Fixture::new();
    let market_id = f.market();
    
    let user1 = f.user();
    let user2 = f.user();
    f.client().vote(&user1, &market_id, &String::from_str(&f.env, "yes"), &10_000_000i128);
    f.client().vote(&user2, &market_id, &String::from_str(&f.env, "no"), &5_000_000i128);
    
    f.advance_past_end();
    let mut outcomes = Vec::new(&f.env);
    outcomes.push_back(String::from_str(&f.env, "yes"));
    outcomes.push_back(String::from_str(&f.env, "no"));

    let (cpu, mem, _) = measure_gas(&f.env, || {
        f.client().resolve_market_with_ties(&f.admin, &market_id, &outcomes);
    });
    
    std::println!("=== Gas Snapshot: resolve_market_with_ties ===");
    std::println!("CPU Instructions: {}", cpu);
    std::println!("Memory Bytes:     {}", mem);
    std::println!("==============================================");

    assert!(cpu < 10_000_000, "CPU cost too high: {}", cpu);
    assert!(mem < 2_000_000, "Memory cost too high: {}", mem);
}

#[test]
fn test_gas_resolve_market_legacy() {
    let f = Fixture::new();
    let market_id = f.market();
    f.advance_past_end();

    let (cpu, mem, _) = measure_gas(&f.env, || {
        let _ = f.client().try_resolve_market(&f.admin, &market_id);
    });
    
    std::println!("=== Gas Snapshot: resolve_market (legacy) ===");
    std::println!("CPU Instructions: {}", cpu);
    std::println!("Memory Bytes:     {}", mem);
    std::println!("=============================================");

    assert!(cpu < 5_000_000, "CPU cost too high: {}", cpu);
    assert!(mem < 1_000_000, "Memory cost too high: {}", mem);
}

#[test]
fn test_gas_resolve_dispute() {
    let f = Fixture::new();
    let market_id = f.market();
    
    // Add votes first
    let user1 = f.user();
    f.client().vote(&user1, &market_id, &String::from_str(&f.env, "yes"), &10_000_000i128);
    
    f.advance_past_end();
    
    // Setup dispute state directly in contract storage
    let user2 = f.user();
    f.env.as_contract(&f.cid, || {
        let mut market: Market = f.env.storage().persistent().get(&market_id).unwrap();
        market.oracle_result = Some(String::from_str(&f.env, "yes"));
        market.dispute_stakes.set(user2.clone(), 10_000_000i128);
        f.env.storage().persistent().set(&market_id, &market);
    });

    // Call resolve_dispute using the client
    let (cpu, mem, _) = measure_gas(&f.env, || {
        f.client().resolve_dispute(&f.admin, &market_id)
    });
    
    std::println!("=== Gas Snapshot: resolve_dispute ===");
    std::println!("CPU Instructions: {}", cpu);
    std::println!("Memory Bytes:     {}", mem);
    std::println!("=====================================");

    assert!(cpu < 10_000_000, "CPU cost too high: {}", cpu);
    assert!(mem < 2_000_000, "Memory cost too high: {}", mem);
}

#[test]
fn test_gas_fetch_oracle_result() {
    let f = Fixture::new();
    let market_id = f.market();
    f.advance_past_end();
    let oracle_address = Address::from_str(
        &f.env,
        "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF",
    );

    let (cpu, mem, _) = measure_gas(&f.env, || {
        f.client().fetch_oracle_result(&market_id, &oracle_address)
    });
    
    std::println!("=== Gas Snapshot: fetch_oracle_result ===");
    std::println!("CPU Instructions: {}", cpu);
    std::println!("Memory Bytes:     {}", mem);
    std::println!("=========================================");

    assert!(cpu < 10_000_000, "CPU cost too high: {}", cpu);
    assert!(mem < 2_000_000, "Memory cost too high: {}", mem);
}
