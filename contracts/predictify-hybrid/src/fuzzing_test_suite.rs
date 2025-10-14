// fuzzing_tests.rs
use std::collections::HashMap;
use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};
use anyhow::{Context, Result, anyhow};
use rand::{Rng, thread_rng};
use rand::distributions::Alphanumeric;

/// Fuzzing test configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FuzzConfig {
    pub iterations: usize,
    pub max_string_length: usize,
    pub max_array_length: usize,
    pub timeout_ms: u64,
    pub enable_edge_cases: bool,
    pub enable_mutation: bool,
    pub seed: Option<u64>,
}

impl Default for FuzzConfig {
    fn default() -> Self {
        Self {
            iterations: 1000,
            max_string_length: 1000,
            max_array_length: 100,
            timeout_ms: 30000,
            enable_edge_cases: true,
            enable_mutation: true,
            seed: None,
        }
    }
}

/// Input types for fuzzing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FuzzInput {
    String(String),
    Integer(i128),
    UnsignedInteger(u128),
    Boolean(bool),
    Address(String),
    Bytes(Vec<u8>),
    Array(Vec<FuzzInput>),
    Struct(HashMap<String, FuzzInput>),
}

/// Oracle response for fuzzing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OracleResponse {
    pub price: i128,
    pub confidence: u64,
    pub timestamp: u64,
    pub source: String,
    pub valid: bool,
}

/// Market state for fuzzing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketState {
    pub market_id: String,
    pub total_volume: u128,
    pub yes_votes: u128,
    pub no_votes: u128,
    pub resolution_time: u64,
    pub is_resolved: bool,
    pub oracle_price: Option<i128>,
}

/// Dispute scenario for fuzzing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisputeScenario {
    pub dispute_id: String,
    pub market_id: String,
    pub disputer: String,
    pub reason: String,
    pub stake_amount: u128,
    pub timestamp: u64,
}

/// Fee calculation for fuzzing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeCalculation {
    pub trade_amount: u128,
    pub fee_percentage: u64,
    pub market_cap: u128,
    pub liquidity: u128,
}

/// Fuzzing test result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FuzzResult {
    pub test_name: String,
    pub input: String,
    pub status: FuzzStatus,
    pub error_message: Option<String>,
    pub execution_time_ms: u128,
    pub vulnerability_detected: bool,
    pub severity: Severity,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FuzzStatus {
    Passed,
    Failed,
    Crashed,
    Timeout,
    Vulnerable,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Severity {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

/// Comprehensive fuzzing results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FuzzingTestResults {
    pub total_iterations: usize,
    pub passed: usize,
    pub failed: usize,
    pub crashed: usize,
    pub timeouts: usize,
    pub vulnerabilities_found: usize,
    pub execution_time_ms: u128,
    pub results_by_function: HashMap<String, Vec<FuzzResult>>,
    pub vulnerabilities: Vec<Vulnerability>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vulnerability {
    pub function: String,
    pub input: String,
    pub description: String,
    pub severity: Severity,
    pub reproduction_steps: String,
}

/// Main fuzzing test suite
pub struct FuzzingTestSuite {
    config: FuzzConfig,
    results: FuzzingTestResults,
    start_time: Option<Instant>,
}

impl FuzzingTestSuite {
    /// Create a new fuzzing test suite
    pub fn new(config: FuzzConfig) -> Self {
        Self {
            config,
            results: FuzzingTestResults {
                total_iterations: 0,
                passed: 0,
                failed: 0,
                crashed: 0,
                timeouts: 0,
                vulnerabilities_found: 0,
                execution_time_ms: 0,
                results_by_function: HashMap::new(),
                vulnerabilities: Vec::new(),
            },
            start_time: None,
        }
    }

    /// Run all fuzzing tests
    pub async fn run_all_fuzz_tests(&mut self) -> Result<FuzzingTestResults> {
        self.start_time = Some(Instant::now());
        
        println!("🎲 Starting Comprehensive Fuzzing Test Suite");
        println!("Configuration: {} iterations per test", self.config.iterations);
        println!("=" .repeat(60));

        // 1. Fuzz input validation
        self.fuzz_input_validation_suite().await?;

        // 2. Fuzz oracle responses
        self.fuzz_oracle_responses_suite().await?;

        // 3. Fuzz market states
        self.fuzz_market_states_suite().await?;

        // 4. Fuzz dispute scenarios
        self.fuzz_dispute_scenarios_suite().await?;

        // 5. Fuzz fee calculations
        self.fuzz_fee_calculations_suite().await?;

        // Calculate final statistics
        if let Some(start) = self.start_time {
            self.results.execution_time_ms = start.elapsed().as_millis();
        }

        self.print_summary();
        
        Ok(self.results.clone())
    }

    /// Fuzz input validation for all functions
    pub async fn fuzz_input_validation(&mut self, inputs: Vec<FuzzInput>) -> Result<Vec<FuzzResult>> {
        let mut results = Vec::new();
        
        for (idx, input) in inputs.iter().enumerate() {
            let start = Instant::now();
            let test_name = format!("input_validation_{}", idx);
            
            match self.test_input_validation(input).await {
                Ok(_) => {
                    results.push(FuzzResult {
                        test_name,
                        input: format!("{:?}", input),
                        status: FuzzStatus::Passed,
                        error_message: None,
                        execution_time_ms: start.elapsed().as_millis(),
                        vulnerability_detected: false,
                        severity: Severity::Info,
                    });
                    self.results.passed += 1;
                }
                Err(e) => {
                    let is_vuln = self.is_vulnerability(&e);
                    results.push(FuzzResult {
                        test_name,
                        input: format!("{:?}", input),
                        status: if is_vuln { FuzzStatus::Vulnerable } else { FuzzStatus::Failed },
                        error_message: Some(e.to_string()),
                        execution_time_ms: start.elapsed().as_millis(),
                        vulnerability_detected: is_vuln,
                        severity: if is_vuln { Severity::High } else { Severity::Low },
                    });
                    
                    if is_vuln {
                        self.results.vulnerabilities_found += 1;
                    } else {
                        self.results.failed += 1;
                    }
                }
            }
            
            self.results.total_iterations += 1;
        }
        
        Ok(results)
    }

    /// Fuzz oracle responses
    pub async fn fuzz_oracle_responses(&mut self, responses: Vec<OracleResponse>) -> Result<Vec<FuzzResult>> {
        let mut results = Vec::new();
        
        for (idx, response) in responses.iter().enumerate() {
            let start = Instant::now();
            let test_name = format!("oracle_response_{}", idx);
            
            match self.test_oracle_response(response).await {
                Ok(_) => {
                    results.push(FuzzResult {
                        test_name,
                        input: format!("{:?}", response),
                        status: FuzzStatus::Passed,
                        error_message: None,
                        execution_time_ms: start.elapsed().as_millis(),
                        vulnerability_detected: false,
                        severity: Severity::Info,
                    });
                    self.results.passed += 1;
                }
                Err(e) => {
                    let is_vuln = self.is_oracle_vulnerability(&e, response);
                    let severity = self.assess_oracle_severity(response);
                    
                    results.push(FuzzResult {
                        test_name,
                        input: format!("{:?}", response),
                        status: if is_vuln { FuzzStatus::Vulnerable } else { FuzzStatus::Failed },
                        error_message: Some(e.to_string()),
                        execution_time_ms: start.elapsed().as_millis(),
                        vulnerability_detected: is_vuln,
                        severity,
                    });
                    
                    if is_vuln {
                        self.results.vulnerabilities_found += 1;
                        self.record_vulnerability(
                            "oracle_response",
                            format!("{:?}", response),
                            "Oracle manipulation vulnerability detected",
                            severity,
                        );
                    } else {
                        self.results.failed += 1;
                    }
                }
            }
            
            self.results.total_iterations += 1;
        }
        
        Ok(results)
    }

    /// Fuzz market states
    pub async fn fuzz_market_states(&mut self, states: Vec<MarketState>) -> Result<Vec<FuzzResult>> {
        let mut results = Vec::new();
        
        for (idx, state) in states.iter().enumerate() {
            let start = Instant::now();
            let test_name = format!("market_state_{}", idx);
            
            match self.test_market_state(state).await {
                Ok(_) => {
                    results.push(FuzzResult {
                        test_name,
                        input: format!("{:?}", state),
                        status: FuzzStatus::Passed,
                        error_message: None,
                        execution_time_ms: start.elapsed().as_millis(),
                        vulnerability_detected: false,
                        severity: Severity::Info,
                    });
                    self.results.passed += 1;
                }
                Err(e) => {
                    let is_vuln = self.is_market_vulnerability(&e, state);
                    let severity = if is_vuln { Severity::Critical } else { Severity::Low };
                    
                    results.push(FuzzResult {
                        test_name,
                        input: format!("{:?}", state),
                        status: if is_vuln { FuzzStatus::Vulnerable } else { FuzzStatus::Failed },
                        error_message: Some(e.to_string()),
                        execution_time_ms: start.elapsed().as_millis(),
                        vulnerability_detected: is_vuln,
                        severity,
                    });
                    
                    if is_vuln {
                        self.results.vulnerabilities_found += 1;
                        self.record_vulnerability(
                            "market_state",
                            format!("{:?}", state),
                            "Market state manipulation vulnerability",
                            severity,
                        );
                    } else {
                        self.results.failed += 1;
                    }
                }
            }
            
            self.results.total_iterations += 1;
        }
        
        Ok(results)
    }

    /// Fuzz dispute scenarios
    pub async fn fuzz_dispute_scenarios(&mut self, scenarios: Vec<DisputeScenario>) -> Result<Vec<FuzzResult>> {
        let mut results = Vec::new();
        
        for (idx, scenario) in scenarios.iter().enumerate() {
            let start = Instant::now();
            let test_name = format!("dispute_scenario_{}", idx);
            
            match self.test_dispute_scenario(scenario).await {
                Ok(_) => {
                    results.push(FuzzResult {
                        test_name,
                        input: format!("{:?}", scenario),
                        status: FuzzStatus::Passed,
                        error_message: None,
                        execution_time_ms: start.elapsed().as_millis(),
                        vulnerability_detected: false,
                        severity: Severity::Info,
                    });
                    self.results.passed += 1;
                }
                Err(e) => {
                    let is_vuln = self.is_dispute_vulnerability(&e, scenario);
                    let severity = if is_vuln { Severity::High } else { Severity::Low };
                    
                    results.push(FuzzResult {
                        test_name,
                        input: format!("{:?}", scenario),
                        status: if is_vuln { FuzzStatus::Vulnerable } else { FuzzStatus::Failed },
                        error_message: Some(e.to_string()),
                        execution_time_ms: start.elapsed().as_millis(),
                        vulnerability_detected: is_vuln,
                        severity,
                    });
                    
                    if is_vuln {
                        self.results.vulnerabilities_found += 1;
                        self.record_vulnerability(
                            "dispute_scenario",
                            format!("{:?}", scenario),
                            "Dispute mechanism vulnerability",
                            severity,
                        );
                    } else {
                        self.results.failed += 1;
                    }
                }
            }
            
            self.results.total_iterations += 1;
        }
        
        Ok(results)
    }

    /// Fuzz fee calculations
    pub async fn fuzz_fee_calculations(&mut self, calculations: Vec<FeeCalculation>) -> Result<Vec<FuzzResult>> {
        let mut results = Vec::new();
        
        for (idx, calc) in calculations.iter().enumerate() {
            let start = Instant::now();
            let test_name = format!("fee_calculation_{}", idx);
            
            match self.test_fee_calculation(calc).await {
                Ok(_) => {
                    results.push(FuzzResult {
                        test_name,
                        input: format!("{:?}", calc),
                        status: FuzzStatus::Passed,
                        error_message: None,
                        execution_time_ms: start.elapsed().as_millis(),
                        vulnerability_detected: false,
                        severity: Severity::Info,
                    });
                    self.results.passed += 1;
                }
                Err(e) => {
                    let is_vuln = self.is_fee_vulnerability(&e, calc);
                    let severity = if is_vuln { Severity::Critical } else { Severity::Low };
                    
                    results.push(FuzzResult {
                        test_name,
                        input: format!("{:?}", calc),
                        status: if is_vuln { FuzzStatus::Vulnerable } else { FuzzStatus::Failed },
                        error_message: Some(e.to_string()),
                        execution_time_ms: start.elapsed().as_millis(),
                        vulnerability_detected: is_vuln,
                        severity,
                    });
                    
                    if is_vuln {
                        self.results.vulnerabilities_found += 1;
                        self.record_vulnerability(
                            "fee_calculation",
                            format!("{:?}", calc),
                            "Fee calculation overflow/underflow vulnerability",
                            severity,
                        );
                    } else {
                        self.results.failed += 1;
                    }
                }
            }
            
            self.results.total_iterations += 1;
        }
        
        Ok(results)
    }

    /// Generate fuzz inputs for a specific function
    pub fn generate_fuzz_inputs(&self, function: String) -> Vec<FuzzInput> {
        let mut inputs = Vec::new();
        let mut rng = thread_rng();
        
        for _ in 0..self.config.iterations {
            let input = match function.as_str() {
                "create_market" => self.generate_market_inputs(&mut rng),
                "place_bet" => self.generate_bet_inputs(&mut rng),
                "resolve_market" => self.generate_resolution_inputs(&mut rng),
                "dispute" => self.generate_dispute_inputs(&mut rng),
                "update_oracle" => self.generate_oracle_inputs(&mut rng),
                _ => self.generate_generic_inputs(&mut rng),
            };
            inputs.push(input);
        }
        
        // Add edge cases if enabled
        if self.config.enable_edge_cases {
            inputs.extend(self.generate_edge_cases(&function));
        }
        
        inputs
    }

    /// Validate fuzzing test results
    pub fn validate_fuzz_test_results(&self, results: Vec<FuzzResult>) -> Result<ValidationReport> {
        let mut report = ValidationReport {
            total_tests: results.len(),
            passed: 0,
            failed: 0,
            vulnerabilities: 0,
            critical_issues: Vec::new(),
            recommendations: Vec::new(),
        };
        
        for result in &results {
            match result.status {
                FuzzStatus::Passed => report.passed += 1,
                FuzzStatus::Failed => report.failed += 1,
                FuzzStatus::Vulnerable => {
                    report.vulnerabilities += 1;
                    if result.severity == Severity::Critical {
                        report.critical_issues.push(result.clone());
                    }
                }
                _ => {}
            }
        }
        
        // Generate recommendations
        if report.vulnerabilities > 0 {
            report.recommendations.push(
                "Address all identified vulnerabilities before mainnet deployment".to_string()
            );
        }
        
        if report.failed as f64 / report.total_tests as f64 > 0.1 {
            report.recommendations.push(
                "High failure rate detected - review input validation logic".to_string()
            );
        }
        
        Ok(report)
    }

    // Suite runners
    async fn fuzz_input_validation_suite(&mut self) -> Result<()> {
        println!("\n🎯 Phase 1: Input Validation Fuzzing");
        
        let functions = vec![
            "create_market",
            "place_bet",
            "resolve_market",
            "update_oracle",
            "dispute",
        ];
        
        for function in functions {
            println!("  Testing function: {}", function);
            let inputs = self.generate_fuzz_inputs(function.to_string());
            let results = self.fuzz_input_validation(inputs).await?;
            self.results.results_by_function.insert(function.to_string(), results);
        }
        
        Ok(())
    }

    async fn fuzz_oracle_responses_suite(&mut self) -> Result<()> {
        println!("\n🔮 Phase 2: Oracle Response Fuzzing");
        
        let responses = self.generate_oracle_responses();
        let results = self.fuzz_oracle_responses(responses).await?;
        self.results.results_by_function.insert("oracle_responses".to_string(), results);
        
        Ok(())
    }

    async fn fuzz_market_states_suite(&mut self) -> Result<()> {
        println!("\n📊 Phase 3: Market State Fuzzing");
        
        let states = self.generate_market_states();
        let results = self.fuzz_market_states(states).await?;
        self.results.results_by_function.insert("market_states".to_string(), results);
        
        Ok(())
    }

    async fn fuzz_dispute_scenarios_suite(&mut self) -> Result<()> {
        println!("\n⚖️  Phase 4: Dispute Scenario Fuzzing");
        
        let scenarios = self.generate_dispute_scenarios();
        let results = self.fuzz_dispute_scenarios(scenarios).await?;
        self.results.results_by_function.insert("dispute_scenarios".to_string(), results);
        
        Ok(())
    }

    async fn fuzz_fee_calculations_suite(&mut self) -> Result<()> {
        println!("\n💰 Phase 5: Fee Calculation Fuzzing");
        
        let calculations = self.generate_fee_calculations();
        let results = self.fuzz_fee_calculations(calculations).await?;
        self.results.results_by_function.insert("fee_calculations".to_string(), results);
        
        Ok(())
    }

    // Input generators
    fn generate_market_inputs(&self, rng: &mut impl Rng) -> FuzzInput {
        let mut map = HashMap::new();
        map.insert("market_id".to_string(), FuzzInput::String(self.random_string(rng, 32)));
        map.insert("question".to_string(), FuzzInput::String(self.random_string(rng, 200)));
        map.insert("resolution_time".to_string(), FuzzInput::UnsignedInteger(rng.gen()));
        map.insert("creator".to_string(), FuzzInput::Address(self.random_address(rng)));
        FuzzInput::Struct(map)
    }

    fn generate_bet_inputs(&self, rng: &mut impl Rng) -> FuzzInput {
        let mut map = HashMap::new();
        map.insert("market_id".to_string(), FuzzInput::String(self.random_string(rng, 32)));
        map.insert("amount".to_string(), FuzzInput::UnsignedInteger(rng.gen()));
        map.insert("outcome".to_string(), FuzzInput::Boolean(rng.gen()));
        map.insert("bettor".to_string(), FuzzInput::Address(self.random_address(rng)));
        FuzzInput::Struct(map)
    }

    fn generate_resolution_inputs(&self, rng: &mut impl Rng) -> FuzzInput {
        let mut map = HashMap::new();
        map.insert("market_id".to_string(), FuzzInput::String(self.random_string(rng, 32)));
        map.insert("outcome".to_string(), FuzzInput::Boolean(rng.gen()));
        map.insert("oracle_price".to_string(), FuzzInput::Integer(rng.gen()));
        FuzzInput::Struct(map)
    }

    fn generate_dispute_inputs(&self, rng: &mut impl Rng) -> FuzzInput {
        let mut map = HashMap::new();
        map.insert("dispute_id".to_string(), FuzzInput::String(self.random_string(rng, 32)));
        map.insert("market_id".to_string(), FuzzInput::String(self.random_string(rng, 32)));
        map.insert("stake".to_string(), FuzzInput::UnsignedInteger(rng.gen()));
        map.insert("reason".to_string(), FuzzInput::String(self.random_string(rng, 500)));
        FuzzInput::Struct(map)
    }

    fn generate_oracle_inputs(&self, rng: &mut impl Rng) -> FuzzInput {
        let mut map = HashMap::new();
        map.insert("price".to_string(), FuzzInput::Integer(rng.gen()));
        map.insert("confidence".to_string(), FuzzInput::UnsignedInteger(rng.gen::<u64>() as u128));
        map.insert("timestamp".to_string(), FuzzInput::UnsignedInteger(rng.gen::<u64>() as u128));
        FuzzInput::Struct(map)
    }

    fn generate_generic_inputs(&self, rng: &mut impl Rng) -> FuzzInput {
        match rng.gen_range(0..6) {
            0 => FuzzInput::String(self.random_string(rng, rng.gen_range(0..self.config.max_string_length))),
            1 => FuzzInput::Integer(rng.gen()),
            2 => FuzzInput::UnsignedInteger(rng.gen()),
            3 => FuzzInput::Boolean(rng.gen()),
            4 => FuzzInput::Address(self.random_address(rng)),
            _ => FuzzInput::Bytes(self.random_bytes(rng, rng.gen_range(0..100))),
        }
    }

    fn generate_edge_cases(&self, function: &str) -> Vec<FuzzInput> {
        let mut cases = Vec::new();
        
        // Common edge cases
        cases.push(FuzzInput::String("".to_string())); // Empty string
        cases.push(FuzzInput::String(" ".repeat(self.config.max_string_length))); // Max length
        cases.push(FuzzInput::Integer(i128::MAX)); // Max int
        cases.push(FuzzInput::Integer(i128::MIN)); // Min int
        cases.push(FuzzInput::Integer(0)); // Zero
        cases.push(FuzzInput::UnsignedInteger(u128::MAX)); // Max uint
        cases.push(FuzzInput::UnsignedInteger(0)); // Zero uint
        
        // Special characters
        cases.push(FuzzInput::String("'; DROP TABLE markets;--".to_string())); // SQL injection attempt
        cases.push(FuzzInput::String("<script>alert('xss')</script>".to_string())); // XSS attempt
        cases.push(FuzzInput::String("\0".repeat(100))); // Null bytes
        cases.push(FuzzInput::String("../../../etc/passwd".to_string())); // Path traversal
        
        cases
    }

    fn generate_oracle_responses(&self) -> Vec<OracleResponse> {
        let mut responses = Vec::new();
        let mut rng = thread_rng();
        
        for _ in 0..self.config.iterations {
            responses.push(OracleResponse {
                price: rng.gen_range(-1000000..1000000),
                confidence: rng.gen_range(0..10000),
                timestamp: rng.gen_range(0..u64::MAX),
                source: self.random_string(&mut rng, 20),
                valid: rng.gen(),
            });
        }
        
        // Add edge cases
        responses.push(OracleResponse {
            price: i128::MAX,
            confidence: 0,
            timestamp: 0,
            source: "".to_string(),
            valid: false,
        });
        
        responses.push(OracleResponse {
            price: i128::MIN,
            confidence: u64::MAX,
            timestamp: u64::MAX,
            source: "A".repeat(1000),
            valid: true,
        });
        
        responses
    }

    fn generate_market_states(&self) -> Vec<MarketState> {
        let mut states = Vec::new();
        let mut rng = thread_rng();
        
        for _ in 0..self.config.iterations {
            states.push(MarketState {
                market_id: self.random_string(&mut rng, 32),
                total_volume: rng.gen(),
                yes_votes: rng.gen(),
                no_votes: rng.gen(),
                resolution_time: rng.gen(),
                is_resolved: rng.gen(),
                oracle_price: if rng.gen() { Some(rng.gen()) } else { None },
            });
        }
        
        // Edge cases
        states.push(MarketState {
            market_id: "".to_string(),
            total_volume: 0,
            yes_votes: u128::MAX,
            no_votes: u128::MAX,
            resolution_time: 0,
            is_resolved: true,
            oracle_price: Some(i128::MAX),
        });
        
        states
    }

    fn generate_dispute_scenarios(&self) -> Vec<DisputeScenario> {
        let mut scenarios = Vec::new();
        let mut rng = thread_rng();
        
        for _ in 0..self.config.iterations {
            scenarios.push(DisputeScenario {
                dispute_id: self.random_string(&mut rng, 32),
                market_id: self.random_string(&mut rng, 32),
                disputer: self.random_address(&mut rng),
                reason: self.random_string(&mut rng, 500),
                stake_amount: rng.gen(),
                timestamp: rng.gen(),
            });
        }
        
        scenarios
    }

    fn generate_fee_calculations(&self) -> Vec<FeeCalculation> {
        let mut calculations = Vec::new();
        let mut rng = thread_rng();
        
        for _ in 0..self.config.iterations {
            calculations.push(FeeCalculation {
                trade_amount: rng.gen(),
                fee_percentage: rng.gen_range(0..10000),
                market_cap: rng.gen(),
                liquidity: rng.gen(),
            });
        }
        
        // Edge cases for overflow/underflow testing
        calculations.push(FeeCalculation {
            trade_amount: u128::MAX,
            fee_percentage: 10000,
            market_cap: u128::MAX,
            liquidity: u128::MAX,
        });
        
        calculations.push(FeeCalculation {
            trade_amount: 0,
            fee_percentage: 0,
            market_cap: 0,
            liquidity: 0,
        });
        
        calculations
    }

    // Helper functions
    fn random_string(&self, rng: &mut impl Rng, len: usize) -> String {
        rng.sample_iter(&Alphanumeric)
            .take(len)
            .map(char::from)
            .collect()
    }

    fn random_address(&self, rng: &mut impl Rng) -> String {
        format!("0x{}", self.random_string(rng, 40))
    }

    fn random_bytes(&self, rng: &mut impl Rng, len: usize) -> Vec<u8> {
        (0..len).map(|_| rng.gen()).collect()
    }

    // Vulnerability detection
    fn is_vulnerability(&self, error: &anyhow::Error) -> bool {
        let msg = error.to_string().to_lowercase();
        msg.contains("overflow") ||
        msg.contains("underflow") ||
        msg.contains("panic") ||
        msg.contains("unauthorized") ||
        msg.contains("reentrancy")
    }

    fn is_oracle_vulnerability(&self, error: &anyhow::Error, response: &OracleResponse) -> bool {
        let msg = error.to_string().to_lowercase();
        
        // Check for manipulation indicators
        (response.confidence == 0 && !msg.contains("low confidence")) ||
        (response.price == i128::MAX && !msg.contains("invalid price")) ||
        (response.timestamp == 0 && !msg.contains("invalid timestamp")) ||
        msg.contains("manipulation") ||
        msg.contains("price oracle attack")
    }

    fn is_market_vulnerability(&self, error: &anyhow::Error, state: &MarketState) -> bool {
        let msg = error.to_string().to_lowercase();
        
        // Check for state manipulation vulnerabilities
        (state.yes_votes > state.total_volume && !msg.contains("invalid votes")) ||
        (state.no_votes > state.total_volume && !msg.contains("invalid votes")) ||
        (state.is_resolved && state.resolution_time == 0 && !msg.contains("invalid resolution")) ||
        msg.contains("state manipulation") ||
        msg.contains("vote manipulation")
    }

    fn is_dispute_vulnerability(&self, error: &anyhow::Error, scenario: &DisputeScenario) -> bool {
        let msg = error.to_string().to_lowercase();
        
        // Check for dispute mechanism vulnerabilities
        (scenario.stake_amount == 0 && !msg.contains("invalid stake")) ||
        msg.contains("dispute manipulation") ||
        msg.contains("double dispute") ||
        msg.contains("front-running")
    }

    fn is_fee_vulnerability(&self, error: &anyhow::Error, calc: &FeeCalculation) -> bool {
        let msg = error.to_string().to_lowercase();
        
        // Check for fee calculation vulnerabilities (overflow/underflow)
        msg.contains("overflow") ||
        msg.contains("underflow") ||
        msg.contains("division by zero") ||
        (calc.trade_amount > 0 && calc.fee_percentage > 0 && 
         !msg.contains("fee calculated"))
    }

    fn assess_oracle_severity(&self, response: &OracleResponse) -> Severity {
        if response.price == i128::MAX || response.price == i128::MIN {
            Severity::Critical
        } else if response.confidence == 0 {
            Severity::High
        } else if !response.valid {
            Severity::Medium
        } else {
            Severity::Low
        }
    }

    fn record_vulnerability(
        &mut self,
        function: &str,
        input: String,
        description: &str,
        severity: Severity,
    ) {
        self.results.vulnerabilities.push(Vulnerability {
            function: function.to_string(),
            input,
            description: description.to_string(),
            severity,
            reproduction_steps: format!(
                "1. Call {} with provided input\n2. Observe vulnerability manifestation\n3. Analyze security impact",
                function
            ),
        });
    }

    // Test implementations (stubs to be replaced with actual contract calls)
    async fn test_input_validation(&self, input: &FuzzInput) -> Result<()> {
        // Simulate input validation
        tokio::time::sleep(Duration::from_micros(100)).await;
        
        match input {
            FuzzInput::String(s) if s.is_empty() => {
                Err(anyhow!("Empty string not allowed"))
            }
            FuzzInput::String(s) if s.len() > self.config.max_string_length => {
                Err(anyhow!("String exceeds maximum length"))
            }
            FuzzInput::String(s) if s.contains('\0') => {
                Err(anyhow!("Null bytes not allowed"))
            }
            FuzzInput::Integer(i) if *i == i128::MAX => {
                Err(anyhow!("Integer overflow detected"))
            }
            FuzzInput::Integer(i) if *i == i128::MIN => {
                Err(anyhow!("Integer underflow detected"))
            }
            FuzzInput::UnsignedInteger(u) if *u == u128::MAX => {
                Err(anyhow!("Potential overflow in unsigned integer"))
            }
            FuzzInput::Address(a) if !a.starts_with("0x") => {
                Err(anyhow!("Invalid address format"))
            }
            FuzzInput::Address(a) if a.len() != 42 => {
                Err(anyhow!("Invalid address length"))
            }
            _ => Ok(()),
        }
    }

    async fn test_oracle_response(&self, response: &OracleResponse) -> Result<()> {
        tokio::time::sleep(Duration::from_micros(100)).await;
        
        // Validate oracle response
        if response.confidence == 0 {
            return Err(anyhow!("Low confidence oracle response"));
        }
        
        if response.price == i128::MAX || response.price == i128::MIN {
            return Err(anyhow!("Invalid price: extreme value detected"));
        }
        
        if response.timestamp == 0 {
            return Err(anyhow!("Invalid timestamp"));
        }
        
        if response.source.is_empty() {
            return Err(anyhow!("Empty oracle source"));
        }
        
        if !response.valid {
            return Err(anyhow!("Oracle marked response as invalid"));
        }
        
        Ok(())
    }

    async fn test_market_state(&self, state: &MarketState) -> Result<()> {
        tokio::time::sleep(Duration::from_micros(100)).await;
        
        // Validate market state consistency
        if state.yes_votes > state.total_volume {
            return Err(anyhow!("Invalid votes: yes_votes exceeds total_volume"));
        }
        
        if state.no_votes > state.total_volume {
            return Err(anyhow!("Invalid votes: no_votes exceeds total_volume"));
        }
        
        if state.yes_votes.saturating_add(state.no_votes) != state.total_volume {
            return Err(anyhow!("Vote totals don't match total volume"));
        }
        
        if state.is_resolved && state.oracle_price.is_none() {
            return Err(anyhow!("Resolved market must have oracle price"));
        }
        
        if state.is_resolved && state.resolution_time == 0 {
            return Err(anyhow!("Invalid resolution: resolved but no resolution time"));
        }
        
        if state.market_id.is_empty() {
            return Err(anyhow!("Empty market ID"));
        }
        
        Ok(())
    }

    async fn test_dispute_scenario(&self, scenario: &DisputeScenario) -> Result<()> {
        tokio::time::sleep(Duration::from_micros(100)).await;
        
        // Validate dispute scenario
        if scenario.stake_amount == 0 {
            return Err(anyhow!("Invalid stake: cannot dispute with zero stake"));
        }
        
        if scenario.dispute_id.is_empty() {
            return Err(anyhow!("Empty dispute ID"));
        }
        
        if scenario.market_id.is_empty() {
            return Err(anyhow!("Empty market ID"));
        }
        
        if scenario.disputer.is_empty() {
            return Err(anyhow!("Empty disputer address"));
        }
        
        if scenario.reason.is_empty() {
            return Err(anyhow!("Dispute reason required"));
        }
        
        if scenario.timestamp == 0 {
            return Err(anyhow!("Invalid dispute timestamp"));
        }
        
        Ok(())
    }

    async fn test_fee_calculation(&self, calc: &FeeCalculation) -> Result<()> {
        tokio::time::sleep(Duration::from_micros(100)).await;
        
        // Validate fee calculation for overflow/underflow
        if calc.fee_percentage > 10000 {
            return Err(anyhow!("Invalid fee percentage: exceeds 100%"));
        }
        
        // Check for overflow in fee calculation
        let fee_result = calc.trade_amount
            .checked_mul(calc.fee_percentage as u128)
            .and_then(|v| v.checked_div(10000));
        
        if fee_result.is_none() {
            return Err(anyhow!("Overflow in fee calculation"));
        }
        
        // Check for division by zero in liquidity calculations
        if calc.liquidity == 0 && calc.market_cap > 0 {
            return Err(anyhow!("Division by zero: liquidity cannot be zero"));
        }
        
        // Validate market cap constraints
        if calc.market_cap > u128::MAX / 2 {
            return Err(anyhow!("Market cap exceeds safe threshold"));
        }
        
        Ok(())
    }

    fn print_summary(&self) {
        println!("\n" .repeat(2));
        println!("=" .repeat(60));
        println!("🎲 FUZZING TEST SUMMARY");
        println!("=" .repeat(60));
        println!("\n📊 Overall Statistics:");
        println!("   Total Iterations: {}", self.results.total_iterations);
        println!("   ✓ Passed: {}", self.results.passed);
        println!("   ✗ Failed: {}", self.results.failed);
        println!("   💥 Crashed: {}", self.results.crashed);
        println!("   ⏱ Timeouts: {}", self.results.timeouts);
        println!("   🚨 Vulnerabilities Found: {}", self.results.vulnerabilities_found);
        println!("   ⏰ Total Execution Time: {}ms", self.results.execution_time_ms);
        
        if !self.results.vulnerabilities.is_empty() {
            println!("\n🚨 VULNERABILITIES DETECTED:");
            for (idx, vuln) in self.results.vulnerabilities.iter().enumerate() {
                println!("\n   Vulnerability #{}", idx + 1);
                println!("   Function: {}", vuln.function);
                println!("   Severity: {:?}", vuln.severity);
                println!("   Description: {}", vuln.description);
                println!("   Input: {}", vuln.input);
            }
        }
        
        println!("\n" .repeat(1));
        println!("=" .repeat(60));
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationReport {
    pub total_tests: usize,
    pub passed: usize,
    pub failed: usize,
    pub vulnerabilities: usize,
    pub critical_issues: Vec<FuzzResult>,
    pub recommendations: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_fuzzing_suite_creation() {
        let config = FuzzConfig::default();
        let suite = FuzzingTestSuite::new(config);
        
        assert_eq!(suite.results.total_iterations, 0);
        assert_eq!(suite.results.vulnerabilities_found, 0);
    }

    #[tokio::test]
    async fn test_input_generation() {
        let config = FuzzConfig {
            iterations: 10,
            ..Default::default()
        };
        let suite = FuzzingTestSuite::new(config);
        
        let inputs = suite.generate_fuzz_inputs("create_market".to_string());
        assert!(inputs.len() >= 10);
    }

    #[tokio::test]
    async fn test_edge_case_generation() {
        let config = FuzzConfig {
            enable_edge_cases: true,
            ..Default::default()
        };
        let suite = FuzzingTestSuite::new(config);
        
        let edge_cases = suite.generate_edge_cases("test_function");
        assert!(!edge_cases.is_empty());
    }

    #[tokio::test]
    async fn test_input_validation_fuzz() {
        let config = FuzzConfig {
            iterations: 50,
            ..Default::default()
        };
        let mut suite = FuzzingTestSuite::new(config);
        
        let inputs = vec![
            FuzzInput::String("valid".to_string()),
            FuzzInput::String("".to_string()), // Should fail
            FuzzInput::Integer(42),
            FuzzInput::Integer(i128::MAX), // Should fail
        ];
        
        let results = suite.fuzz_input_validation(inputs).await.unwrap();
        assert_eq!(results.len(), 4);
        assert!(results.iter().any(|r| r.status == FuzzStatus::Failed));
    }

    #[tokio::test]
    async fn test_oracle_response_fuzz() {
        let config = FuzzConfig::default();
        let mut suite = FuzzingTestSuite::new(config);
        
        let responses = vec![
            OracleResponse {
                price: 1000,
                confidence: 95,
                timestamp: 1234567890,
                source: "pyth".to_string(),
                valid: true,
            },
            OracleResponse {
                price: i128::MAX,
                confidence: 0,
                timestamp: 0,
                source: "".to_string(),
                valid: false,
            },
        ];
        
        let results = suite.fuzz_oracle_responses(responses).await.unwrap();
        assert_eq!(results.len(), 2);
        assert!(results.iter().any(|r| r.status == FuzzStatus::Failed || r.status == FuzzStatus::Vulnerable));
    }

    #[tokio::test]
    async fn test_market_state_fuzz() {
        let config = FuzzConfig::default();
        let mut suite = FuzzingTestSuite::new(config);
        
        let states = vec![
            MarketState {
                market_id: "market1".to_string(),
                total_volume: 1000,
                yes_votes: 600,
                no_votes: 400,
                resolution_time: 1234567890,
                is_resolved: false,
                oracle_price: None,
            },
            MarketState {
                market_id: "".to_string(),
                total_volume: 100,
                yes_votes: 200, // Invalid: exceeds total
                no_votes: 0,
                resolution_time: 0,
                is_resolved: false,
                oracle_price: None,
            },
        ];
        
        let results = suite.fuzz_market_states(states).await.unwrap();
        assert_eq!(results.len(), 2);
        assert!(results.iter().any(|r| r.status == FuzzStatus::Failed));
    }

    #[tokio::test]
    async fn test_dispute_scenario_fuzz() {
        let config = FuzzConfig::default();
        let mut suite = FuzzingTestSuite::new(config);
        
        let scenarios = vec![
            DisputeScenario {
                dispute_id: "dispute1".to_string(),
                market_id: "market1".to_string(),
                disputer: "0x1234567890123456789012345678901234567890".to_string(),
                reason: "Invalid resolution".to_string(),
                stake_amount: 1000,
                timestamp: 1234567890,
            },
            DisputeScenario {
                dispute_id: "".to_string(),
                market_id: "".to_string(),
                disputer: "".to_string(),
                reason: "".to_string(),
                stake_amount: 0,
                timestamp: 0,
            },
        ];
        
        let results = suite.fuzz_dispute_scenarios(scenarios).await.unwrap();
        assert_eq!(results.len(), 2);
        assert!(results.iter().any(|r| r.status == FuzzStatus::Failed));
    }

    #[tokio::test]
    async fn test_fee_calculation_fuzz() {
        let config = FuzzConfig::default();
        let mut suite = FuzzingTestSuite::new(config);
        
        let calculations = vec![
            FeeCalculation {
                trade_amount: 1000,
                fee_percentage: 250, // 2.5%
                market_cap: 100000,
                liquidity: 50000,
            },
            FeeCalculation {
                trade_amount: u128::MAX,
                fee_percentage: 10000, // 100%
                market_cap: u128::MAX,
                liquidity: 0, // Division by zero
            },
        ];
        
        let results = suite.fuzz_fee_calculations(calculations).await.unwrap();
        assert_eq!(results.len(), 2);
        assert!(results.iter().any(|r| r.status == FuzzStatus::Failed || r.status == FuzzStatus::Vulnerable));
    }

    #[tokio::test]
    async fn test_vulnerability_detection() {
        let config = FuzzConfig::default();
        let suite = FuzzingTestSuite::new(config);
        
        let overflow_error = anyhow!("Integer overflow detected");
        assert!(suite.is_vulnerability(&overflow_error));
        
        let normal_error = anyhow!("Invalid input");
        assert!(!suite.is_vulnerability(&normal_error));
    }

    #[tokio::test]
    async fn test_validation_report() {
        let config = FuzzConfig::default();
        let suite = FuzzingTestSuite::new(config);
        
        let results = vec![
            FuzzResult {
                test_name: "test1".to_string(),
                input: "input1".to_string(),
                status: FuzzStatus::Passed,
                error_message: None,
                execution_time_ms: 10,
                vulnerability_detected: false,
                severity: Severity::Info,
            },
            FuzzResult {
                test_name: "test2".to_string(),
                input: "input2".to_string(),
                status: FuzzStatus::Vulnerable,
                error_message: Some("Critical vulnerability".to_string()),
                execution_time_ms: 15,
                vulnerability_detected: true,
                severity: Severity::Critical,
            },
        ];
        
        let report = suite.validate_fuzz_test_results(results).unwrap();
        assert_eq!(report.total_tests, 2);
        assert_eq!(report.passed, 1);
        assert_eq!(report.vulnerabilities, 1);
        assert_eq!(report.critical_issues.len(), 1);
    }

    #[tokio::test]
    async fn test_full_fuzzing_suite() {
        let config = FuzzConfig {
            iterations: 10, // Reduced for testing
            ..Default::default()
        };
        let mut suite = FuzzingTestSuite::new(config);
        
        let result = suite.run_all_fuzz_tests().await;
        assert!(result.is_ok());
        
        let results = result.unwrap();
        assert!(results.total_iterations > 0);
    }
}

// Example usage
#[tokio::main]
async fn main() -> Result<()> {
    println!("🎲 Fuzzing Test Suite for Smart Contracts\n");
    
    // Create configuration
    let config = FuzzConfig {
        iterations: 1000,
        max_string_length: 1000,
        max_array_length: 100,
        timeout_ms: 30000,
        enable_edge_cases: true,
        enable_mutation: true,
        seed: None,
    };
    
    // Create and run fuzzing suite
    let mut suite = FuzzingTestSuite::new(config);
    
    match suite.run_all_fuzz_tests().await {
        Ok(results) => {
            // Save results to file
            let json = serde_json::to_string_pretty(&results)?;
            std::fs::write("fuzzing_test_results.json", json)?;
            println!("\n💾 Results saved to fuzzing_test_results.json");
            
            // Generate validation report
            let all_results: Vec<FuzzResult> = results
                .results_by_function
                .values()
                .flat_map(|v| v.clone())
                .collect();
            
            let report = suite.validate_fuzz_test_results(all_results)?;
            
            println!("\n📋 Validation Report:");
            println!("   Total Tests: {}", report.total_tests);
            println!("   Passed: {}", report.passed);
            println!("   Failed: {}", report.failed);
            println!("   Vulnerabilities: {}", report.vulnerabilities);
            
            if !report.critical_issues.is_empty() {
                println!("\n🚨 CRITICAL ISSUES FOUND:");
                for issue in &report.critical_issues {
                    println!("   - {}: {}", issue.test_name, 
                        issue.error_message.as_ref().unwrap_or(&"Unknown error".to_string()));
                }
            }
            
            if !report.recommendations.is_empty() {
                println!("\n💡 Recommendations:");
                for rec in &report.recommendations {
                    println!("   - {}", rec);
                }
            }
            
            // Exit with appropriate code
            if results.vulnerabilities_found > 0 {
                println!("\n❌ Vulnerabilities detected! Review and fix before deployment.");
                std::process::exit(1);
            } else {
                println!("\n✅ No vulnerabilities detected in fuzzing tests!");
                Ok(())
            }
        }
        Err(e) => {
            eprintln!("\n❌ Fuzzing suite failed: {}", e);
            std::process::exit(1);
        }
    }
}