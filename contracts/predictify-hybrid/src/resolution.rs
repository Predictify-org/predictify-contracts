use soroban_sdk::{contracttype, symbol_short, Address, Env, Map, String, Symbol, Vec};

use crate::bets::BetStorage;
use crate::err::Error;
use alloc::string::ToString;

use crate::markets::{CommunityConsensus, MarketAnalytics, MarketStateManager, MarketUtils};

use crate::oracles::{OracleFactory, OracleUtils};
// use crate::reentrancy_guard::ReentrancyGuard; // Removed - module no longer exists
use crate::types::*;

/// Resolution management system for Predictify Hybrid contract
///
/// This module provides a comprehensive resolution system with:
/// - Oracle resolution functions and utilities
/// - Market resolution logic and validation
/// - Resolution analytics and statistics
/// - Resolution helper utilities and testing functions
/// - Resolution state management and tracking

// ===== RESOLUTION TYPES =====

/// Enumeration of possible resolution states for market lifecycle management.
///
/// This enum tracks the progression of a market through its resolution phases,
/// from initial creation through final outcome determination. Each state represents
/// a specific stage in the resolution process with distinct validation rules and
/// available operations.
///
/// # State Transitions
///
/// The typical resolution flow follows this pattern:
/// ```text
/// Active → OracleResolved → MarketResolved → [Disputed] → Finalized
/// ```
///
/// **Alternative flows:**
/// - Direct admin resolution: `Active → MarketResolved → Finalized`
/// - Dispute flow: `MarketResolved → Disputed → Finalized`
/// - Oracle-only flow: `Active → OracleResolved → MarketResolved → Finalized`
///
/// # Example Usage
///
/// ```rust
/// # use soroban_sdk::{Env, Symbol};
/// # use predictify_hybrid::resolution::{ResolutionState, ResolutionUtils};
/// # use predictify_hybrid::markets::Market;
/// # let env = Env::default();
/// # let market = Market::default(); // Placeholder
///
/// // Check current resolution state
/// let current_state = ResolutionUtils::get_resolution_state(&env, &market);
///
/// match current_state {
///     ResolutionState::Active => {
///         println!("Market is active, ready for oracle resolution");
///         // Can fetch oracle results
///     },
///     ResolutionState::OracleResolved => {
///         println!("Oracle result available, can proceed to market resolution");
///         // Can combine with community consensus
///     },
///     ResolutionState::MarketResolved => {
///         println!("Market resolved, awaiting finalization or disputes");
///         // Can be disputed or finalized
///     },
///     ResolutionState::Disputed => {
///         println!("Resolution is under dispute");
///         // Dispute resolution process active
///     },
///     ResolutionState::Finalized => {
///         println!("Resolution is final and immutable");
///         // No further changes allowed
///     },
/// }
/// ```
///
/// # State Validation
///
/// Each state has specific validation requirements:
/// - **Active**: Market must be within voting period
/// - **OracleResolved**: Oracle data must be valid and recent
/// - **MarketResolved**: Final outcome must be determined
/// - **Disputed**: Dispute must be properly filed and active
/// - **Finalized**: Resolution must be complete and immutable
///
/// # Business Rules
///
/// State transitions enforce business logic:
/// - Markets cannot skip resolution states arbitrarily
/// - Finalized resolutions cannot be changed
/// - Disputed resolutions require proper dispute resolution
/// - Oracle resolution requires valid oracle data
///
/// # Integration Points
///
/// Resolution states integrate with:
/// - **Market Management**: Controls available market operations
/// - **Voting System**: Determines when voting periods end
/// - **Dispute System**: Manages dispute lifecycle
/// - **Oracle System**: Coordinates oracle data fetching
/// - **Admin Functions**: Enables administrative overrides
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[contracttype]
pub enum ResolutionState {
    /// Market is active, no resolution yet
    Active,
    /// Oracle result fetched, pending final resolution
    OracleResolved,
    /// Market fully resolved with final outcome
    MarketResolved,
    /// Resolution disputed
    Disputed,
    /// Resolution finalized after dispute
    Finalized,
}

/// Comprehensive oracle resolution result containing all data needed for market resolution.
///
/// This structure captures the complete oracle response for a market, including
/// the raw price data, comparison logic, outcome determination, and metadata
/// necessary for validation and audit trails.
///
/// # Core Components
///
/// **Market Context:**
/// - **Market ID**: Unique identifier linking resolution to specific market
/// - **Timestamp**: When the oracle resolution was performed
/// - **Provider**: Which oracle service provided the data
///
/// **Oracle Data:**
/// - **Price**: Current asset price from oracle feed
/// - **Threshold**: Market-defined price threshold for comparison
/// - **Comparison**: Comparison operator ("gt", "lt", "eq")
/// - **Feed ID**: Specific oracle feed identifier used
///
/// **Resolution Result:**
/// - **Oracle Result**: Final outcome ("yes"/"no") based on price comparison
///
/// # Example Usage
///
/// ```rust
/// # use soroban_sdk::{Env, Symbol, String, Address};
/// # use predictify_hybrid::resolution::{OracleResolutionManager, OracleResolution};
/// # use predictify_hybrid::types::OracleProvider;
/// # let env = Env::default();
/// # let market_id = Symbol::new(&env, "btc_50k");
/// # let oracle_contract = Address::generate(&env);
///
/// // Fetch oracle resolution for a market
/// let oracle_resolution = OracleResolutionManager::fetch_oracle_result(
///     &env,
///     &market_id,
///     &oracle_contract
/// )?;
///
/// // Examine oracle resolution details
/// println!("Market: {}", oracle_resolution.market_id);
/// println!("Oracle result: {}", oracle_resolution.oracle_result);
/// println!("Price: ${}", oracle_resolution.price / 100);
/// println!("Threshold: ${}", oracle_resolution.threshold / 100);
/// println!("Comparison: {}", oracle_resolution.comparison);
/// println!("Provider: {:?}", oracle_resolution.provider);
/// println!("Feed: {}", oracle_resolution.feed_id);
///
/// // Validate oracle resolution
/// OracleResolutionManager::validate_oracle_resolution(&env, &oracle_resolution)?;
///
/// // Calculate confidence score
/// let confidence = OracleResolutionManager::calculate_oracle_confidence(&oracle_resolution);
/// println!("Oracle confidence: {}%", confidence);
/// # Ok::<(), predictify_hybrid::errors::Error>(())
/// ```
///
/// # Price Comparison Logic
///
/// The oracle resolution evaluates market conditions:
/// ```rust
/// # use soroban_sdk::{Env, String};
/// # use predictify_hybrid::oracles::OracleUtils;
/// # let env = Env::default();
///
/// // Example: BTC above $50,000?
/// let btc_price = 52_000_00;    // $52,000 (8 decimal precision)
/// let threshold = 50_000_00;    // $50,000
/// let comparison = String::from_str(&env, "gt"); // Greater than
///
/// let outcome = OracleUtils::determine_outcome(
///     btc_price,
///     threshold,
///     &comparison,
///     &env
/// )?;
///
/// assert_eq!(outcome, String::from_str(&env, "yes")); // BTC > $50k = "yes"
/// # Ok::<(), predictify_hybrid::errors::Error>(())
/// ```
///
/// # Validation Requirements
///
/// Oracle resolutions must meet criteria:
/// - **Valid Price**: Price must be positive and within reasonable bounds
/// - **Recent Data**: Timestamp must be within acceptable staleness limits
/// - **Supported Provider**: Oracle provider must be supported on current network
/// - **Valid Feed**: Feed ID must exist and be active
/// - **Proper Comparison**: Comparison operator must be supported
///
/// # Integration with Market Resolution
///
/// Oracle resolutions feed into broader market resolution:
/// - **Hybrid Resolution**: Combined with community consensus
/// - **Oracle-Only**: Used directly as final outcome
/// - **Dispute Input**: Provides data for dispute resolution
/// - **Confidence Scoring**: Contributes to overall resolution confidence
///
/// # Audit and Transparency
///
/// All oracle resolution data is preserved for:
/// - **Audit Trails**: Complete record of resolution process
/// - **Dispute Evidence**: Data available for dispute proceedings
/// - **Analytics**: Historical analysis of oracle performance
/// - **Transparency**: Public verification of resolution logic
#[derive(Clone, Debug)]
#[contracttype]
pub struct OracleResolution {
    pub market_id: Symbol,
    pub oracle_result: String,
    pub price: i128,
    pub threshold: i128,
    pub comparison: String,
    pub timestamp: u64,
    pub provider: OracleProvider,
    pub feed_id: String,
}

/// Comprehensive market resolution result combining oracle data with community consensus.
///
/// This structure represents the final resolution of a prediction market, incorporating
/// data from multiple sources (oracle feeds, community voting, admin decisions) to
/// determine the authoritative market outcome with confidence scoring and audit trails.
///
/// # Resolution Components
///
/// **Core Resolution Data:**
/// - **Market ID**: Unique identifier for the resolved market
/// - **Final Outcome**: Definitive market result ("yes"/"no" or custom outcomes)
/// - **Resolution Timestamp**: When the resolution was finalized
/// - **Resolution Method**: How the resolution was determined
///
/// **Data Sources:**
/// - **Oracle Result**: Outcome from oracle price feeds
/// - **Community Consensus**: Aggregated community voting results
/// - **Confidence Score**: Statistical confidence in the resolution (0-100)
///
/// # Resolution Methods
///
/// Markets can be resolved through various methods:
/// - **Oracle Only**: Based purely on oracle price data
/// - **Community Only**: Based on community voting consensus
/// - **Hybrid**: Combines oracle data with community input
/// - **Admin Override**: Administrative decision overrides other methods
/// - **Dispute Resolution**: Outcome determined through dispute process
///
/// # Example Usage
///
/// ```rust
/// # use soroban_sdk::{Env, Symbol, String};
/// # use predictify_hybrid::resolution::{MarketResolutionManager, MarketResolution, ResolutionMethod};
/// # let env = Env::default();
/// # let market_id = Symbol::new(&env, "btc_prediction");
///
/// // Resolve a market using hybrid method
/// let resolution = MarketResolutionManager::resolve_market(&env, &market_id)?;
///
/// // Examine resolution details
/// println!("Market: {}", resolution.market_id);
/// println!("Final outcome: {}", resolution.final_outcome);
/// println!("Oracle result: {}", resolution.oracle_result);
/// println!("Community consensus: {}% ({})",
///     resolution.community_consensus.percentage,
///     resolution.community_consensus.outcome
/// );
/// println!("Resolution method: {:?}", resolution.resolution_method);
/// println!("Confidence: {}%", resolution.confidence_score);
///
/// // Validate the resolution
/// MarketResolutionManager::validate_market_resolution(&env, &resolution)?;
///
/// // Check resolution method
/// match resolution.resolution_method {
///     ResolutionMethod::Hybrid => {
///         println!("Resolution combines oracle and community data");
///     },
///     ResolutionMethod::OracleOnly => {
///         println!("Resolution based purely on oracle data");
///     },
///     ResolutionMethod::AdminOverride => {
///         println!("Resolution was administratively determined");
///     },
///     _ => println!("Other resolution method used"),
/// }
/// # Ok::<(), predictify_hybrid::errors::Error>(())
/// ```
///
/// # Confidence Scoring
///
/// Resolution confidence is calculated based on:
/// - **Oracle Reliability**: Historical oracle accuracy and freshness
/// - **Community Agreement**: Level of consensus in community voting
/// - **Data Quality**: Quality and recency of underlying data
/// - **Method Reliability**: Inherent reliability of resolution method
///
/// ```rust
/// # use predictify_hybrid::resolution::MarketResolution;
/// # let resolution = MarketResolution::default(); // Placeholder
///
/// // Interpret confidence scores
/// match resolution.confidence_score {
///     90..=100 => println!("Very high confidence resolution"),
///     80..=89 => println!("High confidence resolution"),
///     70..=79 => println!("Moderate confidence resolution"),
///     60..=69 => println!("Low confidence resolution"),
///     _ => println!("Very low confidence - may need review"),
/// }
/// ```
///
/// # Resolution Validation
///
/// Market resolutions undergo validation to ensure:
/// - **Outcome Consistency**: Oracle and community data alignment
/// - **Method Appropriateness**: Resolution method suitable for market type
/// - **Data Quality**: All input data meets quality standards
/// - **Timestamp Validity**: Resolution timing is appropriate
/// - **Confidence Thresholds**: Confidence score meets minimum requirements
///
/// # Integration Points
///
/// Market resolutions integrate with:
/// - **Payout System**: Determines winner payouts and distributions
/// - **Dispute System**: Can be challenged through dispute mechanisms
/// - **Analytics**: Contributes to platform performance metrics
/// - **Audit System**: Provides complete resolution audit trails
/// - **Event System**: Triggers resolution events for transparency
///
/// # Immutability and Finalization
///
/// Once finalized, market resolutions are immutable except through:
/// - **Dispute Process**: Formal dispute resolution procedures
/// - **Admin Override**: Emergency administrative corrections
/// - **System Upgrades**: Protocol-level corrections (rare)
#[derive(Clone, Debug)]
#[contracttype]
pub struct MarketResolution {
    pub market_id: Symbol,
    pub final_outcome: String,
    pub oracle_result: String,
    pub community_consensus: CommunityConsensus,
    pub resolution_timestamp: u64,
    pub resolution_method: ResolutionMethod,
    pub confidence_score: u32,
}

/// Enumeration of available market resolution methods and their characteristics.
///
/// This enum defines the different approaches available for resolving prediction markets,
/// each with distinct data sources, validation requirements, and confidence characteristics.
/// The choice of resolution method depends on market type, data availability, and
/// community participation levels.
///
/// # Resolution Method Types
///
/// **Automated Methods:**
/// - **Oracle Only**: Purely algorithmic based on price feed data
/// - **Community Only**: Based entirely on community voting consensus
/// - **Hybrid**: Combines oracle data with community input for balanced resolution
///
/// **Manual Methods:**
/// - **Admin Override**: Administrative decision for exceptional circumstances
/// - **Dispute Resolution**: Outcome determined through formal dispute process
///
/// # Method Selection Logic
///
/// Resolution methods are typically selected based on:
/// ```rust
/// # use predictify_hybrid::resolution::ResolutionMethod;
/// # use predictify_hybrid::markets::CommunityConsensus;
/// # use soroban_sdk::{Env, String};
/// # let env = Env::default();
///
/// // Example method selection logic
/// fn select_resolution_method(
///     oracle_available: bool,
///     community_participation: u32,
///     consensus_strength: u32
/// ) -> ResolutionMethod {
///     match (oracle_available, community_participation, consensus_strength) {
///         (true, participation, consensus) if participation > 50 && consensus > 75 => {
///             ResolutionMethod::Hybrid // Strong community + oracle
///         },
///         (true, participation, _) if participation < 30 => {
///             ResolutionMethod::OracleOnly // Low community participation
///         },
///         (false, participation, consensus) if participation > 100 && consensus > 80 => {
///             ResolutionMethod::CommunityOnly // No oracle, strong community
///         },
///         _ => ResolutionMethod::AdminOverride // Fallback to admin
///     }
/// }
/// ```
///
/// # Example Usage
///
/// ```rust
/// # use soroban_sdk::{Env, String};
/// # use predictify_hybrid::resolution::{ResolutionMethod, MarketResolutionAnalytics};
/// # use predictify_hybrid::markets::CommunityConsensus;
/// # let env = Env::default();
///
/// // Determine resolution method based on available data
/// let oracle_result = String::from_str(&env, "yes");
/// let community_consensus = CommunityConsensus {
///     outcome: String::from_str(&env, "yes"),
///     votes: 150,
///     total_votes: 200,
///     percentage: 75,
/// };
///
/// let method = MarketResolutionAnalytics::determine_resolution_method(
///     &oracle_result,
///     &community_consensus
/// );
///
/// match method {
///     ResolutionMethod::Hybrid => {
///         println!("Using hybrid resolution - oracle and community agree");
///     },
///     ResolutionMethod::OracleOnly => {
///         println!("Using oracle-only resolution - low community participation");
///     },
///     ResolutionMethod::CommunityOnly => {
///         println!("Using community-only resolution - oracle unavailable");
///     },
///     ResolutionMethod::AdminOverride => {
///         println!("Using admin override - exceptional circumstances");
///     },
///     ResolutionMethod::DisputeResolution => {
///         println!("Using dispute resolution - conflicting data sources");
///     },
/// }
/// ```
///
/// # Method Characteristics
///
/// **Oracle Only:**
/// - **Speed**: Fastest resolution method
/// - **Objectivity**: Purely algorithmic, no human bias
/// - **Reliability**: Depends on oracle data quality
/// - **Use Case**: Clear-cut price-based markets
///
/// **Community Only:**
/// - **Participation**: Requires active community engagement
/// - **Flexibility**: Can handle subjective or complex outcomes
/// - **Consensus**: Relies on community agreement
/// - **Use Case**: Subjective or oracle-unavailable markets
///
/// **Hybrid:**
/// - **Balance**: Combines objective data with community wisdom
/// - **Validation**: Cross-validates oracle data with community input
/// - **Confidence**: Generally highest confidence scores
/// - **Use Case**: Most standard prediction markets
///
/// **Admin Override:**
/// - **Authority**: Administrative decision with full authority
/// - **Speed**: Can be immediate when needed
/// - **Responsibility**: Requires admin accountability
/// - **Use Case**: Emergency situations or system failures
///
/// **Dispute Resolution:**
/// - **Process**: Formal dispute resolution procedures
/// - **Thoroughness**: Most comprehensive review process
/// - **Time**: Longest resolution time
/// - **Use Case**: Contested or controversial outcomes
///
/// # Integration with Confidence Scoring
///
/// Different methods contribute to confidence scores:
/// - **Hybrid**: Highest confidence when oracle and community agree
/// - **Oracle Only**: High confidence for clear price-based outcomes
/// - **Community Only**: Confidence based on participation and consensus
/// - **Admin Override**: Confidence based on admin justification
/// - **Dispute Resolution**: Confidence based on dispute outcome strength
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[contracttype]
pub enum ResolutionMethod {
    /// Oracle only resolution
    OracleOnly,
    /// Community consensus only
    CommunityOnly,
    /// Hybrid oracle + community
    Hybrid,
    /// Admin override
    AdminOverride,
    /// Dispute resolution
    DisputeResolution,
    /// Administrative force-resolve (bypasses time/state checks, idempotent).
    /// Used for emergency overrides regardless of market state.
    ForceResolve,
}

/// Precomputed payout totals persisted at resolution time (O(1) reads on claim/distribute).
///
/// Built once when winning outcomes are set; invalidated when outcomes or pool change.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedOutcomeSummary {
    /// Sum of winning-side stakes (votes + bets, deduplicated).
    pub winning_total: i128,
    /// Total market pool at resolution (`market.total_staked`).
    pub total_pool: i128,
    /// Number of winning outcomes (tie split divisor).
    pub num_winning_outcomes: u32,
}

/// Storage-backed cache for resolved market payout math.
///
/// Time: O(V + B) once at `refresh`; O(1) on payout paths.
/// Space: O(1) per market (single summary struct).
pub struct ResolutionOutcomeCache;

impl ResolutionOutcomeCache {
    fn storage_key(market_id: &Symbol) -> (Symbol, Symbol) {
        (symbol_short!("res_out"), market_id.clone())
    }

    let mut market: Market = env
        .storage()
        .persistent()
        .get(&market_id)
        .unwrap_or_else(|| {
            soroban_sdk::panic_with_error!(env, Error::MarketNotFound);
        });

    // Check if market is resolved
    let winning_outcomes = match &market.winning_outcomes {
        Some(outcomes) => outcomes,
        None => return Err(Error::MarketNotResolved),
    };

    // Get all bettors
    let bettors = bets::BetStorage::get_all_bets_for_market(&env, &market_id);

    // Get fee from legacy storage (backward compatible)
    let fee_percent = env
        .storage()
        .persistent()
        .get(&Symbol::new(&env, "platform_fee"))
        .unwrap_or(200);

    let mut has_unclaimed_winners = false;

    // Check voters
    for (user, outcome) in market.votes.iter() {
        if winning_outcomes.contains(&outcome) {
            if !market
                .claimed
                .get((*user).clone())
                .map(|info| info.is_claimed())
                .unwrap_or(false)
            {
                has_unclaimed_winners = true;
                break;
            }
        }
    }

    if !has_unclaimed_winners {
        for user in bettors.iter() {
            if let Some(bet) = bets::BetStorage::get_bet(&env, &market_id, &user) {
                if winning_outcomes.contains(&bet.outcome)
                    && !market
                        .claimed
                        .get((*user).clone())
                        .map(|info| info.is_claimed())
                        .unwrap_or(false)
                {
                    has_unclaimed_winners = true;
                    break;
                }
            }
        }
    }

    if !has_unclaimed_winners {
        return Ok(0);
    }

    let summary = resolution::ResolutionOutcomeCache::require(&env, &market_id, &market)?;
    let winning_total = summary.winning_total;
    if winning_total == 0 {
        return Ok(0);
    }

    let total_pool = summary.total_pool;
    let fee_denominator = 10000i128;
    let mut total_distributed: i128 = 0;

    // Create budget guard with 100,000 instruction threshold
    let budget_guard = gas::BudgetGuard::new(&env, 100000);

    // 1. Distribute to Voters
    let mut voter_count = 0u32;
    for (user, outcome) in market.votes.iter() {
        if winning_outcomes.contains(&outcome) {
            if market
                .claimed
                .get((*user).clone())
                .map(|info| info.is_claimed())
                .unwrap_or(false)
            {
                continue;
            }

            let user_stake = market.stakes.get((*user).clone()).unwrap_or(0);
            if user_stake > 0 {
                let user_share = (user_stake
                    .checked_mul(fee_denominator - fee_percent)
                    .ok_or(Error::InvalidInput)?)
                    / fee_denominator;
                let payout = (user_share
                    .checked_mul(total_pool)
                    .ok_or(Error::InvalidInput)?)
                    / winning_total;

                if payout >= 0 {
                    market
                        .claimed
                        .set((*user).clone(), ClaimInfo::new(&env, payout));
                    if payout > 0 {
                        total_distributed = total_distributed
                            .checked_add(payout)
                            .ok_or(Error::InvalidInput)?;

                        storage::BalanceStorage::add_balance(
                            &env,
                            &user,
                            &ReflectorAsset::Stellar,
                            payout,
                        )?;

                        events::EventEmitter::emit_winnings_claimed(&env, &market_id, &user, payout);
                    }
                }
            }
        }

        voter_count += 1;
        if voter_count % 10 == 0 {
            budget_guard.check()?;
        }
    }

    /// Get oracle resolution for a market

    pub fn get_oracle_resolution(
        _env: &Env,
        _market_id: &Symbol,
    ) -> Result<Option<OracleResolution>, Error> {
        // For now, return None since we don't store complex types in storage
        // In a real implementation, you would store this in a more sophisticated way

        Ok(None)
    }

    /// Validate oracle resolution
    pub fn validate_oracle_resolution(
        _env: &Env,
        resolution: &OracleResolution,
    ) -> Result<(), Error> {
        // Validate price is positive
        if resolution.price <= 0 {
            return Err(Error::InvalidInput);
        }

        // Validate threshold is positive
        if resolution.threshold <= 0 {
            return Err(Error::InvalidInput);
        }

        // Validate outcome is not empty
        if resolution.oracle_result.is_empty() {
            return Err(Error::InvalidInput);
        }

        Ok(())
    }

    /// Calculate oracle confidence score
    pub fn calculate_oracle_confidence(resolution: &OracleResolution) -> u32 {
        OracleResolutionAnalytics::calculate_confidence_score(resolution)
    }

    // ── Median Config Management ───────────────────────────────────────────────────

    /// Persist the three-oracle median configuration to contract storage.
    ///
    /// Must be called once from the contract admin initialiser before any
    /// market uses [`resolve_with_median`].  Re-calling overwrites the
    /// stored configuration.
    ///
    /// # Arguments
    /// - `env`    – Soroban environment.
    /// - `config` – [`MedianOracleConfig`] to store globally.
    pub fn set_median_config(env: &Env, config: &MedianOracleConfig) {
        env.storage()
            .persistent()
            .set(&symbol_short!("med_cfg"), config);
    }

    /// Load the three-oracle median configuration from contract storage.
    ///
    /// # Errors
    /// Returns [`Error::ConfigNotFound`] when no configuration has been
    /// stored via [`set_median_config`].
    pub fn get_median_config(env: &Env) -> Result<MedianOracleConfig, Error> {
        env.storage()
            .persistent()
            .get(&symbol_short!("med_cfg"))
            .ok_or(Error::ConfigNotFound)
    }

    // ── Core Median Resolver ───────────────────────────────────────────────────

    /// Resolve a market using a confidence-weighted median of Pyth,
    /// Reflector, and Band Protocol price feeds.
    ///
    /// # Algorithm
    ///
    /// 1. **Validate** – Enforce the resolution-timeout guard and delegate
    ///    standard pre-resolution checks to
    ///    [`OracleResolutionValidator::validate_market_for_oracle_resolution`].
    ///
    /// 2. **Fetch** – Query `OraclePriceData` from Pyth, Reflector, and
    ///    Band sequentially (WASM is single-threaded).  Failed fetches yield
    ///    a quote with `included = false`; they do not abort the resolver.
    ///
    /// 3. **Weight** – Convert each oracle's confidence interval to a
    ///    basis-point weight:
    ///    ```text
    ///    weight_bps = price × 10 000 / (price + confidence)
    ///    ```
    ///    Oracles that do not report a confidence interval receive
    ///    5 000 bps (medium weight).
    ///
    /// 4. **Baseline median** – Compute the unweighted simple median of the
    ///    successfully fetched prices (used *only* for outlier detection).
    ///
    /// 5. **Outlier filter** – Discard any quote where
    ///    ```text
    ///    |price − baseline_median| × 10 000 / baseline_median
    ///        > max_deviation_bps
    ///    ```
    ///    The quote's `included` flag is set to `false`.
    ///
    /// 6. **Minimum sources** – Return [`Error::OracleNoConsensus`] if fewer
    ///    than `MedianOracleConfig::min_sources` quotes remain.
    ///
    /// 7. **Weighted median** – Sort the surviving `(price, weight)` pairs
    ///    ascending and return the price at which the cumulative weight first
    ///    reaches ⌈total_weight / 2⌉.
    ///
    /// 8. **Outcome** – Apply the market's threshold comparison to the
    ///    weighted-median price to produce "yes" or "no".
    ///
    /// 9. **Persist & emit** – Store the oracle result in the market,
    ///    emit [`OracleConsensusReachedEvent`] (topic `orc_cons`) for
    ///    backward-compatible monitoring, and emit a per-oracle detail
    ///    event (topic `orc_med_q`) carrying the full
    ///    `Vec<OracleQuote>`.
    ///
    /// # Errors
    ///
    /// | Error | Cause |
    /// |---|---|
    /// | `ConfigNotFound` | No `MedianOracleConfig` stored. |
    /// | `ResolutionTimeoutReached` | `now ≥ end_time + resolution_timeout`. |
    /// | `MarketClosed` | Market has not yet ended. |
    /// | `MarketResolved` | Market already has an oracle result. |
    /// | `OracleNoConsensus` | Fewer than `min_sources` non-outlier quotes. |
    pub fn resolve_with_median(
        env: &Env,
        market_id: &Symbol,
    ) -> Result<MedianResolutionResult, Error> {
        // ── 1. Load market and validate state ─────────────────────────────────
        let mut market = MarketStateManager::get_market(env, market_id)?;
        let current_time = env.ledger().timestamp();

        // Refuse resolution after the timeout window closes.
        if current_time >= market.end_time.saturating_add(market.resolution_timeout) {
            crate::events::EventEmitter::emit_resolution_timeout(env, market_id, current_time);
            return Err(Error::ResolutionTimeoutReached);
        }

        // Standard pre-resolution checks (market ended, not already resolved).
        OracleResolutionValidator::validate_market_for_oracle_resolution(env, &market)?;

        // ── 2. Load median config ────────────────────────────────────────
        let med_cfg = Self::get_median_config(env)?;
        let feed_id = market.oracle_config.feed_id.clone();
        let threshold = market.oracle_config.threshold;
        let comparison = market.oracle_config.comparison.clone();

        // ── 3. Fetch from all three oracles sequentially ────────────────────
        let mut raw_quotes: Vec<OracleQuote> = Vec::new(env);

        // Pyth (currently OracleUnavailable on Stellar; quote will be excluded).
        {
            let oracle = crate::oracles::PythOracle::new(med_cfg.pyth_address.clone());
            raw_quotes.push_back(Self::fetch_quote(
                env,
                &oracle,
                OracleProvider::pyth(),
                &feed_id,
            ));
        }
        // Reflector – primary Stellar oracle.
        {
            let oracle = crate::oracles::ReflectorOracle::new(med_cfg.reflector_address.clone());
            raw_quotes.push_back(Self::fetch_quote(
                env,
                &oracle,
                OracleProvider::reflector(),
                &feed_id,
            ));
        }
        // Band Protocol.
        {
            let oracle = crate::oracles::BandProtocolOracle::new(med_cfg.band_address.clone());
            raw_quotes.push_back(Self::fetch_quote(
                env,
                &oracle,
                OracleProvider::band_protocol(),
                &feed_id,
            ));
        }

        // ── 4. Unweighted baseline median for outlier detection ─────────────
        let baseline_prices = Self::collect_included_sorted(env, &raw_quotes);
        let initial_count = baseline_prices.len() as u32;
        if initial_count < med_cfg.min_sources {
            return Err(Error::OracleNoConsensus);
        }
        let baseline_median = Self::simple_median(&baseline_prices);

        // ── 5. Mark outliers ─────────────────────────────────────────────────
        let mut final_quotes: Vec<OracleQuote> = Vec::new(env);
        for q in raw_quotes.iter() {
            let mut out = q.clone();
            if out.included && baseline_median > 0 {
                let abs_diff: i128 = if out.price > baseline_median {
                    out.price.saturating_sub(baseline_median)
                } else {
                    baseline_median.saturating_sub(out.price)
                };
                // deviation_bps = |price - median| * 10_000 / median
                let deviation_bps: u64 = (abs_diff as u64)
                    .saturating_mul(10_000)
                    .saturating_div(baseline_median as u64);
                if deviation_bps > med_cfg.max_deviation_bps as u64 {
                    out.included = false; // Outlier: exclude from weighted median.
                }
            }
            final_quotes.push_back(out);
        }

        // ── 6. Enforce minimum source count ────────────────────────────────
        let mut included_count: u32 = 0;
        for q in final_quotes.iter() {
            if q.included {
                included_count += 1;
            }
        }
        if included_count < med_cfg.min_sources {
            return Err(Error::OracleNoConsensus);
        }

        // ── 7. Confidence-weighted median ────────────────────────────────────
        let weighted_median = Self::weighted_median(&final_quotes)?;

        // ── 8. Outcome determination ────────────────────────────────────────
        let outcome =
            OracleUtils::determine_outcome(weighted_median, threshold, &comparison, env)?;

        // ── 9. Persist oracle result and emit events ──────────────────────
        MarketStateManager::set_oracle_result(&mut market, outcome.clone());
        MarketStateManager::update_market(env, market_id, &market);

        // Compute aggregate statistics for the consensus event.
        let avg_price = Self::average_included_price(&final_quotes);
        let price_var = Self::price_variance(&final_quotes, avg_price);
        let confidence_score = Self::aggregate_confidence(included_count, &final_quotes);

        // Standard OracleConsensusReachedEvent for backward-compatible monitoring.
        crate::events::EventEmitter::emit_oracle_consensus_reached(
            env,
            market_id,
            &outcome,
            included_count,
            3, // total oracle sources attempted
            avg_price,
            price_var,
        );

        // Per-oracle detail event with the full quote vector.
        crate::events::EventEmitter::emit_oracle_median_quotes(env, market_id, &final_quotes);

        Ok(MedianResolutionResult {
            market_id: market_id.clone(),
            outcome,
            weighted_median_price: weighted_median,
            threshold,
            comparison,
            quotes: final_quotes,
            included_count,
            confidence_score,
            timestamp: current_time,
        })
    }

    // ── Private helpers ─────────────────────────────────────────────────────

    /// Fetch a single oracle quote, absorbing network/decode errors into
    /// `included = false`.
    ///
    /// On a successful fetch with a positive price, the confidence interval
    /// is converted to a basis-point weight via [`Self::confidence_to_weight`].
    /// Any error (oracle unavailable, stale data, invalid feed, …) produces
    /// a quote with `price = 0`, `weight_bps = 0`, and `included = false`
    /// so that the caller can continue gathering remaining sources.
    fn fetch_quote<O: crate::oracles::OracleInterface>(
        env: &Env,
        oracle: &O,
        provider: OracleProvider,
        feed_id: &String,
    ) -> OracleQuote {
        match oracle.get_price_data(env, feed_id) {
            Ok(data) if data.price > 0 => {
                let (confidence_bps, weight_bps) =
                    Self::confidence_to_weight(data.price, data.confidence);
                OracleQuote {
                    provider,
                    price: data.price,
                    confidence_bps,
                    weight_bps,
                    included: true,
                }
            }
            _ => OracleQuote {
                provider,
                price: 0,
                confidence_bps: 0,
                weight_bps: 0,
                included: false,
            },
        }
    }

    /// Derive a basis-point weight from a raw oracle confidence interval.
    ///
    /// ### Formula
    /// ```text
    /// weight_bps = price × 10 000 / (price + confidence)
    /// ```
    /// This maps a tighter interval (lower `confidence`) to a weight closer
    /// to 10 000 and a wider interval to a weight closer to 0.
    ///
    /// ### Special cases
    /// - `confidence = None`  → medium weight **5 000** bps (unknown quality).
    /// - `confidence ≤ 0`     → maximum weight **10 000** bps (perfect certainty).
    /// - `price ≤ 0`         → **(0, 0)** (invalid quote; caller marks `included = false`).
    ///
    /// ### Returns
    /// `(confidence_bps, weight_bps)` where `confidence_bps` is the
    /// confidence interval expressed as a fraction of `price` in BPS.
    fn confidence_to_weight(price: i128, confidence: Option<i128>) -> (u32, u32) {
        if price <= 0 {
            return (0, 0);
        }
        match confidence {
            None => (0, 5_000), // Unknown interval → medium weight.
            Some(c) if c <= 0 => (0, 10_000), // Zero interval → max weight.
            Some(c) => {
                // confidence as a fraction of price, in BPS (clamped to 10 000).
                let conf_bps: u32 = ((c as u64)
                    .saturating_mul(10_000)
                    .saturating_div(price as u64))
                .min(10_000) as u32;
                // inverse-uncertainty weight (clamped to [1, 10 000]).
                let weight_bps: u32 = ((price as u64)
                    .saturating_mul(10_000)
                    .saturating_div((price as u64).saturating_add(c as u64)))
                .max(1)
                .min(10_000) as u32;
                (conf_bps, weight_bps)
            }
        }
    }

    /// Collect the prices of all included quotes and return them sorted
    /// ascending in a Soroban `Vec`.
    ///
    /// Uses a fixed three-slot array internally so no heap allocation is
    /// needed; the WASM budget for ≤ 3 comparisons is negligible.
    fn collect_included_sorted(env: &Env, quotes: &Vec<OracleQuote>) -> Vec<i128> {
        // Collect up to 3 included prices into a fixed array.
        let mut buf: [i128; 3] = [0; 3];
        let mut n: usize = 0;
        for q in quotes.iter() {
            if q.included && n < 3 {
                buf[n] = q.price;
                n += 1;
            }
        }
        // Bubble-sort the first n elements (n ≤ 3, so O(n²) is negligible).
        for i in 0..n {
            for j in 0..n.saturating_sub(i + 1) {
                if buf[j] > buf[j + 1] {
                    buf.swap(j, j + 1);
                }
            }
        }
        // Build Soroban Vec.
        let mut result: Vec<i128> = Vec::new(env);
        for i in 0..n {
            result.push_back(buf[i]);
        }
        result
    }

    /// Compute the unweighted simple median of a **sorted** price list.
    ///
    /// For an odd number of elements the true middle value is returned.
    /// For an even number the arithmetic mean of the two middle values is
    /// returned (integer truncation; acceptable precision for outlier
    /// detection on typical oracle prices).
    /// Returns 0 for an empty list (callers always guard with `min_sources`).
    fn simple_median(sorted: &Vec<i128>) -> i128 {
        let n = sorted.len() as usize;
        if n == 0 {
            return 0;
        }
        if n % 2 == 1 {
            sorted.get((n / 2) as u32).unwrap_or(0)
        } else {
            let lo = sorted.get((n / 2 - 1) as u32).unwrap_or(0);
            let hi = sorted.get((n / 2) as u32).unwrap_or(0);
            // Overflow-safe average: avoids (lo + hi) overflow for large prices.
            (lo / 2) + (hi / 2) + ((lo % 2 + hi % 2) / 2)
        }
    }

    /// Compute the confidence-weighted median of the included quotes.
    ///
    /// Sorts the `(price, weight)` pairs ascending (using a fixed array so
    /// no heap allocation is needed), then walks from the lowest price
    /// upward accumulating weights until the cumulative weight first reaches
    /// ⌈ total / 2 ⌉.  The price at that point is the weighted median.
    ///
    /// # Errors
    /// Returns [`Error::OracleNoConsensus`] when no included quotes exist.
    fn weighted_median(quotes: &Vec<OracleQuote>) -> Result<i128, Error> {
        // Collect at most 3 (price, weight) pairs.
        let mut pairs: [(i128, u32); 3] = [(0, 0); 3];
        let mut n: usize = 0;
        for q in quotes.iter() {
            if q.included && n < 3 {
                pairs[n] = (q.price, q.weight_bps.max(1));
                n += 1;
            }
        }
        if n == 0 {
            return Err(Error::OracleNoConsensus);
        }
        // Insertion sort by price ascending.
        for i in 1..n {
            let mut j = i;
            while j > 0 && pairs[j - 1].0 > pairs[j].0 {
                pairs.swap(j - 1, j);
                j -= 1;
            }
        }
        // Accumulate weights until ⌈ total / 2 ⌉ is reached.
        let mut total: u64 = 0;
        for i in 0..n {
            total = total.saturating_add(pairs[i].1 as u64);
        }
        let half: u64 = (total + 1) / 2; // ceiling division
        let mut cumulative: u64 = 0;
        let mut result: i128 = 0;
        for i in 0..n {
            cumulative = cumulative.saturating_add(pairs[i].1 as u64);
            result = pairs[i].0;
            if cumulative >= half {
                break;
            }
        }
        Ok(result)
    }

    /// Arithmetic mean price of all included quotes.
    /// Used to populate the `average_price` field of
    /// [`OracleConsensusReachedEvent`].
    fn average_included_price(quotes: &Vec<OracleQuote>) -> i128 {
        let mut sum: i128 = 0;
        let mut count: u32 = 0;
        for q in quotes.iter() {
            if q.included {
                sum = sum.saturating_add(q.price);
                count += 1;
            }
        }
        if count == 0 {
            0
        } else {
            sum / count as i128
        }
    }

    /// Integer proxy for price variance among included quotes.
    ///
    /// Computes the mean of squared deviations from `avg`, scaling each
    /// squared term down by 10 000 before accumulating to keep the value
    /// within i128 range for typical oracle prices (up to ~10¹³ base units).
    /// Used to populate the `price_variance` field of
    /// [`OracleConsensusReachedEvent`].
    fn price_variance(quotes: &Vec<OracleQuote>, avg: i128) -> i128 {
        let mut sum_sq: i128 = 0;
        let mut count: u32 = 0;
        for q in quotes.iter() {
            if q.included {
                let diff = q.price.saturating_sub(avg);
                sum_sq = sum_sq
                    .saturating_add(diff.saturating_mul(diff).saturating_div(10_000));
                count += 1;
            }
        }
        if count == 0 {
            0
        } else {
            sum_sq / count as i128
        }
    }

    /// Aggregate confidence score in [0, 100] for [`MedianResolutionResult`].
    ///
    /// Base score reflects the number of surviving sources:
    /// - 1 source → 60
    /// - 2 sources → 75
    /// - 3 sources → 90
    ///
    /// A bonus of up to 10 points is added based on the average
    /// `weight_bps` of the included quotes (10 000 bps = +10 bonus).
    fn aggregate_confidence(included_count: u32, quotes: &Vec<OracleQuote>) -> u32 {
        let base: u32 = match included_count {
            0 => 0,
            1 => 60,
            2 => 75,
            _ => 90, // ≥ 3 sources
        };
        let mut total_weight: u64 = 0;
        let mut count: u32 = 0;
        for q in quotes.iter() {
            if q.included {
                total_weight = total_weight.saturating_add(q.weight_bps as u64);
                count += 1;
            }
        }
        let bonus: u32 = if count > 0 {
            // avg_weight / 1_000 gives a score in [0, 10] since max weight = 10 000.
            (total_weight / count as u64 / 1_000).min(10) as u32
        } else {
            0
        };
        (base + bonus).min(100)
    }
}

// ===== MARKET RESOLUTION =====

/// Comprehensive market resolution management system combining multiple data sources.
///
/// The Market Resolution Manager orchestrates the complete market resolution process,
/// integrating oracle data, community consensus, admin decisions, and dispute outcomes
/// to determine final market results. It serves as the central coordinator for all
/// resolution methods and ensures consistent, reliable market outcomes.
///
/// # Core Responsibilities
///
/// **Resolution Orchestration:**
/// - **Multi-Source Integration**: Combine oracle, community, and admin data
/// - **Method Selection**: Choose appropriate resolution method based on available data
/// - **Confidence Calculation**: Determine overall confidence in resolution outcome
/// - **Validation**: Ensure resolution meets quality and consistency standards
///
/// **Market Lifecycle Management:**
/// - **Resolution Triggering**: Initiate resolution when markets are ready
/// - **State Management**: Track resolution progress through various states
/// - **Finalization**: Complete resolution process and make outcomes immutable
/// - **Event Emission**: Notify system components of resolution events
///
/// # Resolution Methods Supported
///
/// **Hybrid Resolution (Recommended):**
/// - Combines oracle price data with community voting
/// - Highest confidence when sources agree
/// - Fallback logic when sources disagree
///
/// **Oracle-Only Resolution:**
/// - Pure algorithmic resolution based on price feeds
/// - Fast and objective for clear-cut price-based markets
/// - Used when community participation is insufficient
///
/// **Community-Only Resolution:**
/// - Based entirely on community voting consensus
/// - Used when oracle data is unavailable or inappropriate
/// - Requires sufficient participation and consensus
///
/// **Admin Override:**
/// - Administrative decision for exceptional circumstances
/// - Used for emergency situations or system failures
/// - Requires proper admin authentication and justification
///
/// # Example Usage
///
/// ```rust
/// # use soroban_sdk::{Env, Symbol, Address, String};
/// # use predictify_hybrid::resolution::{MarketResolutionManager, MarketResolution, ResolutionMethod};
/// # let env = Env::default();
/// # let market_id = Symbol::new(&env, "btc_prediction_market");
/// # let admin = Address::generate(&env);
///
/// // Resolve a market using hybrid method (oracle + community)
/// let resolution = MarketResolutionManager::resolve_market(&env, &market_id)?;
///
/// println!("Market Resolution Complete:");
/// println!("Market: {}", resolution.market_id);
/// println!("Final outcome: {}", resolution.final_outcome);
/// println!("Method: {:?}", resolution.resolution_method);
/// println!("Confidence: {}%", resolution.confidence_score);
///
/// // Display resolution details
/// match resolution.resolution_method {
///     ResolutionMethod::Hybrid => {
///         println!("Oracle result: {}", resolution.oracle_result);
///         println!("Community consensus: {}% ({})",
///             resolution.community_consensus.percentage,
///             resolution.community_consensus.outcome
///         );
///     },
///     ResolutionMethod::OracleOnly => {
///         println!("Resolved purely based on oracle: {}", resolution.oracle_result);
///     },
///     ResolutionMethod::AdminOverride => {
///         println!("Administrative override resolution");
///     },
///     _ => println!("Other resolution method used"),
/// }
///
/// // Validate the resolution
/// MarketResolutionManager::validate_market_resolution(&env, &resolution)?;
///
/// // Admin can finalize with override if needed
/// if resolution.confidence_score < 70 {
///     let admin_resolution = MarketResolutionManager::finalize_market(
///         &env,
///         &admin,
///         &market_id,
///         &String::from_str(&env, "yes")
///     )?;
///     println!("Admin finalized with outcome: {}", admin_resolution.final_outcome);
/// }
/// # Ok::<(), predictify_hybrid::errors::Error>(())
/// ```
///
/// # Resolution Decision Logic
///
/// The manager uses sophisticated logic to determine final outcomes:
/// ```rust
/// # use soroban_sdk::{Env, String};
/// # use predictify_hybrid::resolution::ResolutionMethod;
/// # use predictify_hybrid::markets::CommunityConsensus;
/// # let env = Env::default();
///
/// // Example resolution decision logic
/// fn determine_final_outcome(
///     oracle_result: &String,
///     community_consensus: &CommunityConsensus,
///     oracle_confidence: u32,
///     community_confidence: u32
/// ) -> (String, ResolutionMethod) {
///     let env = Env::default();
///
///     // Check if oracle and community agree
///     if oracle_result == &community_consensus.outcome {
///         // Agreement - use hybrid method with high confidence
///         (oracle_result.clone(), ResolutionMethod::Hybrid)
///     } else if oracle_confidence > 85 && community_confidence < 60 {
///         // Strong oracle, weak community - use oracle
///         (oracle_result.clone(), ResolutionMethod::OracleOnly)
///     } else if community_confidence > 85 && oracle_confidence < 60 {
///         // Strong community, weak oracle - use community
///         (community_consensus.outcome.clone(), ResolutionMethod::CommunityOnly)
///     } else {
///         // Conflict requires admin intervention
///         (String::from_str(&env, "disputed"), ResolutionMethod::AdminOverride)
///     }
/// }
/// ```
///
/// # Confidence Scoring
///
/// Resolution confidence is calculated from multiple factors:
/// - **Oracle Confidence**: Quality and freshness of oracle data
/// - **Community Confidence**: Participation level and consensus strength
/// - **Method Reliability**: Inherent reliability of chosen resolution method
/// - **Data Consistency**: Agreement between different data sources
///
/// ```rust
/// # use predictify_hybrid::resolution::MarketResolution;
/// # let resolution = MarketResolution::default(); // Placeholder
///
/// // Interpret confidence levels
/// match resolution.confidence_score {
///     95..=100 => println!("Extremely high confidence - virtually certain outcome"),
///     85..=94 => println!("Very high confidence - strong evidence for outcome"),
///     75..=84 => println!("High confidence - good evidence for outcome"),
///     65..=74 => println!("Moderate confidence - reasonable evidence"),
///     50..=64 => println!("Low confidence - weak evidence, consider review"),
///     _ => println!("Very low confidence - outcome uncertain, needs attention"),
/// }
/// ```
///
/// # Error Handling and Fallbacks
///
/// The manager handles various failure scenarios:
/// - **Oracle Failures**: Fallback to community-only resolution
/// - **Low Participation**: Fallback to oracle-only or admin resolution
/// - **Data Conflicts**: Escalate to dispute resolution process
/// - **System Errors**: Graceful degradation with error reporting
///
/// # Integration with Other Systems
///
/// Market Resolution Manager integrates with:
/// - **Oracle System**: Fetches and validates oracle data
/// - **Voting System**: Retrieves community consensus data
/// - **Dispute System**: Handles disputed resolutions
/// - **Admin System**: Processes administrative overrides
/// - **Event System**: Emits resolution events for transparency
/// - **Analytics System**: Records resolution metrics and performance
///
/// # Performance and Scalability
///
/// The manager optimizes for:
/// - **Batch Processing**: Resolve multiple markets efficiently
/// - **Parallel Resolution**: Handle independent resolutions concurrently
/// - **Caching**: Cache resolution data to avoid redundant calculations
/// - **Event-Driven**: React to market state changes automatically
pub struct MarketResolutionManager;

impl MarketResolutionManager {
    /// Resolve a market by combining oracle results and community votes
    pub fn resolve_market(env: &Env, market_id: &Symbol) -> Result<MarketResolution, Error> {
        // Get the market from storage
        let mut market = MarketStateManager::get_market(env, market_id)?;

        // Validate market for resolution (includes min pool size check)
        let validation = MarketResolutionValidator::validate_market_for_resolution(env, &market);
        if let Err(Error::InvalidState) = validation {
            let global_min: i128 = env
                .storage()
                .persistent()
                .get(&Symbol::new(env, "global_min_pool"))
                .unwrap_or(0);
            let min_pool = market.min_pool_size.unwrap_or(global_min);
            crate::events::EventEmitter::emit_min_pool_size_not_met(
                env,
                market_id,
                market.total_staked,
                min_pool,
            );
            return Err(Error::InvalidState);
        }
        validation?;

        // Retrieve the oracle result
        let oracle_result = market
            .oracle_result
            .as_ref()
            .ok_or(Error::OracleUnavailable)?
            .clone();

        // Calculate community consensus
        let community_consensus = MarketAnalytics::calculate_community_consensus(&market);

        // Determine winning outcome(s) using multi-outcome resolution with tie detection
        // This handles both single winner and tie cases (pool split)
        let winning_outcomes = MarketUtils::determine_winning_outcomes(
            env,
            &market,
            &oracle_result,
            &community_consensus,
            0, // Tie threshold: 0 = exact ties only
        );

        // For resolution record, use first outcome (or comma-separated for display)
        let final_result = if winning_outcomes.len() > 0 {
            if winning_outcomes.len() == 1 {
                winning_outcomes.get(0).unwrap().clone()
            } else {
                // For ties, just use the first outcome for the final result field
                // The full list is stored in winning_outcomes
                winning_outcomes.get(0).unwrap().clone()
            }
        } else {
            oracle_result.clone()
        };

        // Determine resolution method
        let resolution_method = MarketResolutionAnalytics::determine_resolution_method(
            &oracle_result,
            &community_consensus,
        );

        // Calculate confidence score
        let confidence_score = MarketResolutionAnalytics::calculate_confidence_score(
            &oracle_result,
            &community_consensus,
            &resolution_method,
        );

        // Create market resolution record
        let resolution = MarketResolution {
            market_id: market_id.clone(),
            final_outcome: final_result.clone(),
            oracle_result,
            community_consensus,
            resolution_timestamp: env.ledger().timestamp(),
            resolution_method,
            confidence_score,
        };

        // Capture old state for event
        let old_state = market.state.clone();

        // Set winning outcome(s) - supports both single winner and ties
        MarketStateManager::set_winning_outcomes(
            &mut market,
            winning_outcomes.clone(),
            Some(market_id),
        );
        MarketStateManager::update_market(env, market_id, &market);
        ResolutionOutcomeCache::refresh(env, market_id, &market)?;

        // Decrement active event count since the event is resolved
        crate::storage::CreatorLimitsManager::decrement_active_events(env, &market.admin);

        // Emit market resolved event
        let oracle_result_str = market
            .oracle_result
            .clone()
            .unwrap_or_else(|| soroban_sdk::String::from_str(env, "N/A"));
        let community_consensus_str = soroban_sdk::String::from_str(env, "Consensus");
        let method_str = match resolution_method {
            ResolutionMethod::OracleOnly => "OracleOnly",
            ResolutionMethod::CommunityOnly => "CommunityOnly",
            ResolutionMethod::Hybrid => "Hybrid",
            ResolutionMethod::AdminOverride => "AdminOverride",
            ResolutionMethod::DisputeResolution => "DisputeResolution",
            ResolutionMethod::ForceResolve => "ForceResolve",
        };
        let resolution_method_str = soroban_sdk::String::from_str(env, method_str);

        crate::events::EventEmitter::emit_market_resolved(
            env,
            market_id,
            &final_result,
            &oracle_result_str,
            &community_consensus_str,
            &resolution_method_str,
            confidence_score as i128,
        );

        // Emit state change event
        crate::events::EventEmitter::emit_state_change_event(
            env,
            market_id,
            &old_state,
            &crate::types::MarketState::Resolved,
            &soroban_sdk::String::from_str(env, "Automated resolution completed"),
        );
        crate::monitoring::ContractMonitor::emit_resolution_transition_hook(
            env,
            market_id,
            &old_state,
            &crate::types::MarketState::Resolved,
            &resolution_method_str,
        );

        Ok(resolution)
    }

    /// Finalize market with admin override
    pub fn finalize_market(
        env: &Env,
        admin: &Address,
        market_id: &Symbol,
        outcome: &String,
    ) -> Result<MarketResolution, Error> {
        // Validate admin permissions
        MarketResolutionValidator::validate_admin_permissions(env, admin)?;

        // Get the market
        let mut market = MarketStateManager::get_market(env, market_id)?;

        // Validate outcome
        MarketResolutionValidator::validate_outcome(env, outcome, &market.outcomes)?;

        // Create resolution record
        let resolution = MarketResolution {
            market_id: market_id.clone(),
            final_outcome: outcome.clone(),
            oracle_result: market
                .oracle_result
                .clone()
                .unwrap_or_else(|| String::from_str(env, "")),
            community_consensus: MarketAnalytics::calculate_community_consensus(&market),
            resolution_timestamp: env.ledger().timestamp(),
            resolution_method: ResolutionMethod::AdminOverride,
            confidence_score: 100, // Admin override has full confidence
        };

        // Set final outcome(s) - convert single outcome to vector
        let mut winning_outcomes = Vec::new(env);
        winning_outcomes.push_back(outcome.clone());
        MarketStateManager::set_winning_outcomes(&mut market, winning_outcomes, Some(market_id));
        MarketStateManager::update_market(env, market_id, &market);
        ResolutionOutcomeCache::refresh(env, market_id, &market)?;

        // Decrement active event count since the event is manually finalized
        crate::storage::CreatorLimitsManager::decrement_active_events(env, &market.admin);

        Ok(resolution)
    }

    /// Get market resolution

    pub fn get_market_resolution(
        _env: &Env,
        _market_id: &Symbol,
    ) -> Result<Option<MarketResolution>, Error> {
        // For now, return None since we don't store complex types in storage
        // In a real implementation, you would store this in a more sophisticated way

        Ok(None)
    }

    /// Validate market resolution
    pub fn validate_market_resolution(
        env: &Env,
        resolution: &MarketResolution,
    ) -> Result<(), Error> {
        MarketResolutionValidator::validate_market_resolution(env, resolution)
    }
}

// ===== RESOLUTION VALIDATION =====

/// Oracle resolution validation
pub struct OracleResolutionValidator;

impl OracleResolutionValidator {
    /// Validate market for oracle resolution
    pub fn validate_market_for_oracle_resolution(env: &Env, market: &Market) -> Result<(), Error> {
        // Check if the market has already been resolved
        if market.oracle_result.is_some() {
            return Err(Error::MarketResolved);
        }

        // Check if the market ended (we can only fetch oracle result after market ends)
        let current_time = env.ledger().timestamp();
        if current_time < market.end_time {
            return Err(Error::MarketClosed);
        }

        Ok(())
    }

    /// Validate oracle resolution
    pub fn validate_oracle_resolution(
        _env: &Env,
        resolution: &OracleResolution,
    ) -> Result<(), Error> {
        // Validate price is positive
        if resolution.price <= 0 {
            return Err(Error::InvalidInput);
        }

        // Validate threshold is positive
        if resolution.threshold <= 0 {
            return Err(Error::InvalidInput);
        }

        // Validate outcome is not empty
        if resolution.oracle_result.is_empty() {
            return Err(Error::InvalidInput);
        }

        Ok(())
    }
}

/// Market resolution validation
pub struct MarketResolutionValidator;

impl MarketResolutionValidator {
    /// Validate market for resolution
    pub fn validate_market_for_resolution(env: &Env, market: &Market) -> Result<(), Error> {
        // Check if market is already resolved
        if market.winning_outcomes.is_some() {
            return Err(Error::MarketResolved);
        }

        // Check if oracle result is available
        if market.oracle_result.is_none() {
            return Err(Error::OracleUnavailable);
        }

        // Check if market has ended
        if market.is_active(env) {
            return Err(Error::MarketClosed);
        }

        // Check minimum pool size requirement (per-market override, else global)
        let global_min: i128 = env
            .storage()
            .persistent()
            .get(&Symbol::new(env, "global_min_pool"))
            .unwrap_or(0);
        let min_pool = market.min_pool_size.unwrap_or(global_min);

        // Only check if min pool is set
        if min_pool > 0 {
            // Get token decimals to normalize amounts for comparison
            let token_client = crate::markets::MarketUtils::get_token_client(env)?;
            let token_decimals = token_client.decimals() as u32;

            // Normalize both total staked and min pool to canonical scale for comparison
            let normalized_total = crate::tokens::normalize_amount(market.total_staked, token_decimals);
            let normalized_min = crate::tokens::normalize_amount(min_pool, token_decimals);

            if normalized_total < normalized_min {
                return Err(Error::InvalidState);
            }
        }

        Ok(())
    }

    /// Validate admin permissions
    pub fn validate_admin_permissions(env: &Env, admin: &Address) -> Result<(), Error> {
        let stored_admin: Option<Address> =
            env.storage().persistent().get(&Symbol::new(env, "Admin"));

        match stored_admin {
            Some(stored_admin) => {
                if admin != &stored_admin {
                    return Err(Error::Unauthorized);
                }

    /// Validate outcome
    pub fn validate_outcome(
        _env: &Env,
        outcome: &String,
        valid_outcomes: &Vec<String>,
    ) -> Result<(), Error> {
        if !valid_outcomes.contains(outcome) {
            return Err(Error::InvalidOutcome);
        }

        Ok(())
    }

    /// Validate market resolution
    pub fn validate_market_resolution(
        env: &Env,
        resolution: &MarketResolution,
    ) -> Result<(), Error> {
        // Validate final outcome is not empty
        if resolution.final_outcome.is_empty() {
            return Err(Error::InvalidInput);
        }

        // Validate confidence score is within range
        if resolution.confidence_score > 100 {
            return Err(Error::InvalidInput);
        }

        // Validate timestamp is reasonable
        let current_time = env.ledger().timestamp();
        if resolution.resolution_timestamp > current_time {
            return Err(Error::InvalidInput);
        }

        Ok(())
    }
}

// ===== RESOLUTION ANALYTICS =====

/// Oracle resolution analytics
pub struct OracleResolutionAnalytics;

impl OracleResolutionAnalytics {
    /// Calculate oracle confidence score
    pub fn calculate_confidence_score(resolution: &OracleResolution) -> u32 {
        // Base confidence for oracle resolution
        let mut confidence: u32 = 80;

        // Adjust based on price deviation from threshold
        let deviation = ((resolution.price - resolution.threshold).abs() as f64)
            / (resolution.threshold as f64);

        if deviation > 0.1 {
            // High deviation - lower confidence
            confidence = confidence.saturating_sub(20);
        } else if deviation < 0.05 {
            // Low deviation - higher confidence
            confidence = confidence.saturating_add(10);
        }

        confidence.min(100)
    }

    /// Get oracle resolution statistics
    pub fn get_oracle_stats(_env: &Env) -> Result<OracleStats, Error> {
        Ok(OracleStats::default())
    }
}

/// Market resolution analytics
pub struct MarketResolutionAnalytics;

impl MarketResolutionAnalytics {
    /// Determine resolution method
    pub fn determine_resolution_method(
        _oracle_result: &String,
        community_consensus: &CommunityConsensus,
    ) -> ResolutionMethod {
        if community_consensus.percentage > 70 {
            ResolutionMethod::Hybrid
        } else {
            ResolutionMethod::OracleOnly
        }
    }

    /// Calculate confidence score
    pub fn calculate_confidence_score(
        _oracle_result: &String,
        community_consensus: &CommunityConsensus,
        method: &ResolutionMethod,
    ) -> u32 {
        match method {
            ResolutionMethod::OracleOnly => 85,
            ResolutionMethod::CommunityOnly => {
                let base_confidence = community_consensus.percentage as u32;
                base_confidence.min(90)
            }
            ResolutionMethod::Hybrid => {
                let oracle_confidence = 85;
                let community_confidence = community_consensus.percentage as u32;
                ((oracle_confidence + community_confidence) / 2).min(95)
            }
            ResolutionMethod::AdminOverride => 100,
            ResolutionMethod::DisputeResolution => 75,
            ResolutionMethod::ForceResolve => 100,
        }
    }

    /// Calculate resolution analytics
    pub fn calculate_resolution_analytics(_env: &Env) -> Result<ResolutionAnalytics, Error> {
        Ok(ResolutionAnalytics::default())
    }

    /// Update resolution analytics
    pub fn update_resolution_analytics(
        _env: &Env,
        _resolution: &MarketResolution,
    ) -> Result<(), Error> {
        // For now, do nothing since we don't store complex types
        Ok(())
    }
}

// ===== RESOLUTION UTILITIES =====

/// Resolution utility functions
pub struct ResolutionUtils;

impl ResolutionUtils {
    /// Get resolution state for a market
    pub fn get_resolution_state(_env: &Env, market: &Market) -> ResolutionState {
        if market.winning_outcomes.is_some() {
            ResolutionState::MarketResolved
        } else if market.oracle_result.is_some() {
            ResolutionState::OracleResolved
        } else if market.total_dispute_stakes() > 0 {
            ResolutionState::Disputed
        } else {
            ResolutionState::Active
        }
    }

    /// Check if market can be resolved
    pub fn can_resolve_market(env: &Env, market: &Market) -> bool {
        market.has_ended(env) && market.oracle_result.is_some() && market.winning_outcomes.is_none()
    }

    /// Get resolution eligibility
    pub fn get_resolution_eligibility(env: &Env, market: &Market) -> (bool, String) {
        if !market.has_ended(env) {
            return (false, String::from_str(env, "Market has not ended"));
        }

        if market.oracle_result.is_none() {
            return (false, String::from_str(env, "Oracle result not available"));
        }

        if market.winning_outcomes.is_some() {
            return (false, String::from_str(env, "Market already resolved"));
        }

        (true, String::from_str(env, "Eligible for resolution"))
    }

    /// Calculate resolution time
    pub fn calculate_resolution_time(env: &Env, market: &Market) -> u64 {
        let current_time = env.ledger().timestamp();
        if current_time > market.end_time {
            current_time - market.end_time
        } else {
            0
        }
    }

    /// Validate resolution parameters
    pub fn validate_resolution_parameters(
        _env: &Env,
        market: &Market,
        outcome: &String,
    ) -> Result<(), Error> {
        // Validate outcome is in market outcomes
        if !market.outcomes.contains(outcome) {
            return Err(Error::InvalidOutcome);
        }

        // Validate market is not already resolved
        if market.winning_outcomes.is_some() {
            return Err(Error::MarketResolved);
        }

        Ok(())
    }
}

// ===== RESOLUTION TESTING =====

/// Resolution testing utilities
pub struct ResolutionTesting;

impl ResolutionTesting {
    /// Create test oracle resolution
    pub fn create_test_oracle_resolution(env: &Env, market_id: &Symbol) -> OracleResolution {
        OracleResolution {
            market_id: market_id.clone(),
            oracle_result: String::from_str(env, "yes"),
            price: 2500000,
            threshold: 2500000,
            comparison: String::from_str(env, "gt"),
            timestamp: env.ledger().timestamp(),
            provider: OracleProvider::pyth(),
            feed_id: String::from_str(env, "BTC/USD"),
        }
    }

    /// Create test market resolution
    pub fn create_test_market_resolution(env: &Env, market_id: &Symbol) -> MarketResolution {
        MarketResolution {
            market_id: market_id.clone(),
            final_outcome: String::from_str(env, "yes"),
            oracle_result: String::from_str(env, "yes"),
            community_consensus: CommunityConsensus {
                outcome: String::from_str(env, "yes"),
                votes: 6,
                total_votes: 10,
                percentage: 60,
            },
            resolution_timestamp: env.ledger().timestamp(),
            resolution_method: ResolutionMethod::Hybrid,
            confidence_score: 80,
        }
    }

    /// Validate resolution structure
    pub fn validate_resolution_structure(resolution: &MarketResolution) -> Result<(), Error> {
        if resolution.final_outcome.is_empty() {
            return Err(Error::InvalidInput);
        }

        if resolution.confidence_score > 100 {
            return Err(Error::InvalidInput);
        }

        Ok(())
    }

    /// Simulate resolution process
    pub fn simulate_resolution_process(
        env: &Env,
        market_id: &Symbol,
    ) -> Result<MarketResolution, Error> {
        // Fetch oracle result
        let _oracle_resolution = OracleResolutionManager::fetch_oracle_result(env, market_id)?;

        // Resolve market
        let market_resolution = MarketResolutionManager::resolve_market(env, market_id)?;

        Ok(market_resolution)
    }
}

// ===== STATISTICS TYPES =====

/// Oracle statistics
#[derive(Clone, Debug)]
#[contracttype]
pub struct OracleStats {
    pub total_resolutions: u32,
    pub successful_resolutions: u32,
    pub average_confidence: i128,
    pub provider_distribution: Map<OracleProvider, u32>,
}

impl Default for OracleStats {
    fn default() -> Self {
        Self {
            total_resolutions: 0,
            successful_resolutions: 0,
            average_confidence: 0,
            provider_distribution: Map::new(&soroban_sdk::Env::default()),
        }
    }
}

impl Default for ResolutionAnalytics {
    fn default() -> Self {
        Self {
            total_resolutions: 0,
            oracle_resolutions: 0,
            community_resolutions: 0,
            hybrid_resolutions: 0,
            average_confidence: 0,
            resolution_times: Vec::new(&soroban_sdk::Env::default()),
            outcome_distribution: Map::new(&soroban_sdk::Env::default()),
        }
    }
}

// ===== MODULE TESTS =====

#[cfg(any())]
mod tests {
    use super::*;
    use crate::{test::PredictifyTest, PredictifyHybridClient};
    use soroban_sdk::testutils::{Address as _, Ledger, LedgerInfo};

    #[test]
    fn test_oracle_resolution_manager_fetch_result() {
        let env = Env::default();
        let market_id = Symbol::new(&env, "test_market");
        let _oracle_contract = Address::generate(&env);

        // This test would require a mock oracle setup
        // For now, we'll test the validation logic
        let resolution = ResolutionTesting::create_test_oracle_resolution(&env, &market_id);
        assert_eq!(resolution.oracle_result, String::from_str(&env, "yes"));
        assert_eq!(resolution.price, 2500000);
    }

    #[test]
    fn test_market_resolution_manager_resolve_market() {
        let env = Env::default();
        let market_id = Symbol::new(&env, "test_market");

        // This test would require a complete market setup
        // For now, we'll test the resolution structure
        let resolution = ResolutionTesting::create_test_market_resolution(&env, &market_id);
        assert_eq!(resolution.final_outcome, String::from_str(&env, "yes"));
        assert_eq!(resolution.resolution_method, ResolutionMethod::Hybrid);
    }

    #[test]
    fn test_resolution_utils_get_state() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let market = Market::new(
            &env,
            admin,
            String::from_str(&env, "Test Market"),
            soroban_sdk::vec![
                &env,
                String::from_str(&env, "yes"),
                String::from_str(&env, "no"),
            ],
            env.ledger().timestamp() + 86400,
            OracleConfig {
                provider: OracleProvider::pyth(),
                oracle_address: Address::generate(&env),
                feed_id: String::from_str(&env, "BTC/USD"),
                threshold: 2500000,
                comparison: String::from_str(&env, "gt"),
            },
            None,
            86400,
            MarketState::Active,
        );

        let state = ResolutionUtils::get_resolution_state(&env, &market);
        assert_eq!(state, ResolutionState::Active);
    }

    #[test]
    fn test_resolution_analytics_determine_method() {
        let env = Env::default();
        let oracle_result = String::from_str(&env, "yes");
        let community_consensus = CommunityConsensus {
            outcome: String::from_str(&env, "yes"),
            votes: 8,
            total_votes: 10,
            percentage: 80,
        };

        let method = MarketResolutionAnalytics::determine_resolution_method(
            &oracle_result,
            &community_consensus,
        );
        assert_eq!(method, ResolutionMethod::Hybrid);
    }

    #[test]
    fn test_resolution_testing_utilities() {
        let env = Env::default();
        let market_id = Symbol::new(&env, "test_market");

        let oracle_resolution = ResolutionTesting::create_test_oracle_resolution(&env, &market_id);
        assert!(oracle_resolution.oracle_result == String::from_str(&env, "yes"));

        let market_resolution = ResolutionTesting::create_test_market_resolution(&env, &market_id);
        assert!(ResolutionTesting::validate_resolution_structure(&market_resolution).is_ok());
    }

    #[test]
    fn test_resolution_method_determination() {
        let env = Env::default();

        // Create test data
        let community_consensus = CommunityConsensus {
            outcome: String::from_str(&env, "yes"),
            votes: 75,
            total_votes: 100,
            percentage: 75,
        };

        // Test hybrid resolution
        let method = MarketResolutionAnalytics::determine_resolution_method(
            &String::from_str(&env, "yes"),
            &community_consensus,
        );
        assert!(matches!(method, ResolutionMethod::Hybrid));

        // Test oracle-only resolution
        let low_consensus = CommunityConsensus {
            outcome: String::from_str(&env, "yes"),
            votes: 60,
            total_votes: 100,
            percentage: 60,
        };
        let method = MarketResolutionAnalytics::determine_resolution_method(
            &String::from_str(&env, "yes"),
            &low_consensus,
        );
        assert!(matches!(method, ResolutionMethod::OracleOnly));
    }
}

// ===== MEDIAN RESOLUTION UNIT TESTS =====

/// Unit tests for `OracleResolutionManager` median-aggregation helpers.
///
/// These tests exercise the pure-logic helpers in isolation so they can run
/// without a full Soroban contract environment and without live oracle
/// contracts.  Integration behaviour (actual oracle calls, market storage)
/// is verified at the contract-integration test layer.
#[cfg(test)]
mod median_resolution_tests {
    use super::*;
    use soroban_sdk::testutils::Address as _;

    // ── Helpers ────────────────────────────────────────────────────────────

    fn make_env() -> Env {
        Env::default()
    }

    fn quote(provider: OracleProvider, price: i128, weight_bps: u32, included: bool) -> OracleQuote {
        OracleQuote {
            provider,
            price,
            confidence_bps: 0,
            weight_bps,
            included,
        }
    }

    // ── confidence_to_weight ───────────────────────────────────────────────

    #[test]
    fn test_weight_none_confidence_gives_medium_weight() {
        let (cbps, wbps) = OracleResolutionManager::confidence_to_weight(1_000_000, None);
        assert_eq!(cbps, 0, "unknown confidence should produce zero conf_bps");
        assert_eq!(wbps, 5_000, "unknown confidence should produce medium weight");
    }

    #[test]
    fn test_weight_zero_confidence_gives_max_weight() {
        let (cbps, wbps) = OracleResolutionManager::confidence_to_weight(1_000_000, Some(0));
        assert_eq!(cbps, 0);
        assert_eq!(wbps, 10_000, "zero-interval oracle should receive maximum weight");
    }

    #[test]
    fn test_weight_negative_confidence_gives_max_weight() {
        let (_cbps, wbps) = OracleResolutionManager::confidence_to_weight(500_000, Some(-1));
        assert_eq!(wbps, 10_000);
    }

    #[test]
    fn test_weight_inverse_relationship_tighter_interval_higher_weight() {
        // A tighter confidence interval (smaller c relative to price) should
        // yield a higher weight than a wide one.
        let (_c1, w_tight) =
            OracleResolutionManager::confidence_to_weight(1_000_000, Some(1_000));
        let (_c2, w_wide) =
            OracleResolutionManager::confidence_to_weight(1_000_000, Some(100_000));
        assert!(
            w_tight > w_wide,
            "tighter interval (c=1_000) should give higher weight than wide (c=100_000)"
        );
    }

    #[test]
    fn test_weight_known_values() {
        // price=1 000 000, confidence=1 000 000 (100 % uncertainty)
        // weight = 1 000 000 * 10 000 / (1 000 000 + 1 000 000) = 5 000
        let (_cbps, wbps) =
            OracleResolutionManager::confidence_to_weight(1_000_000, Some(1_000_000));
        assert_eq!(wbps, 5_000);
    }

    #[test]
    fn test_weight_non_positive_price_returns_zeros() {
        assert_eq!(
            OracleResolutionManager::confidence_to_weight(0, Some(100)),
            (0, 0),
            "zero price must return (0, 0)"
        );
        assert_eq!(
            OracleResolutionManager::confidence_to_weight(-1, None),
            (0, 0),
            "negative price must return (0, 0)"
        );
    }

    // ── simple_median ──────────────────────────────────────────────────────

    #[test]
    fn test_simple_median_single_element() {
        let env = make_env();
        let mut v: Vec<i128> = Vec::new(&env);
        v.push_back(42);
        assert_eq!(OracleResolutionManager::simple_median(&v), 42);
    }

    #[test]
    fn test_simple_median_two_elements_returns_average() {
        let env = make_env();
        let mut v: Vec<i128> = Vec::new(&env);
        v.push_back(100);
        v.push_back(200);
        // average of two middle values
        assert_eq!(OracleResolutionManager::simple_median(&v), 150);
    }

    #[test]
    fn test_simple_median_three_elements_returns_middle() {
        let env = make_env();
        let mut v: Vec<i128> = Vec::new(&env);
        v.push_back(100);
        v.push_back(200);
        v.push_back(300);
        assert_eq!(OracleResolutionManager::simple_median(&v), 200);
    }

    #[test]
    fn test_simple_median_empty_returns_zero() {
        let env = make_env();
        let v: Vec<i128> = Vec::new(&env);
        assert_eq!(OracleResolutionManager::simple_median(&v), 0);
    }

    // ── collect_included_sorted ────────────────────────────────────────────

    #[test]
    fn test_collect_included_sorted_filters_and_sorts() {
        let env = make_env();
        let mut quotes: Vec<OracleQuote> = Vec::new(&env);
        quotes.push_back(quote(OracleProvider::reflector(), 300, 5_000, true));
        quotes.push_back(quote(OracleProvider::pyth(), 0, 0, false)); // excluded
        quotes.push_back(quote(OracleProvider::band_protocol(), 100, 5_000, true));

        let sorted = OracleResolutionManager::collect_included_sorted(&env, &quotes);
        assert_eq!(sorted.len(), 2, "excluded quote must be filtered out");
        assert_eq!(sorted.get(0), Some(100), "prices should be sorted ascending");
        assert_eq!(sorted.get(1), Some(300));
    }

    #[test]
    fn test_collect_included_sorted_all_excluded_returns_empty() {
        let env = make_env();
        let mut quotes: Vec<OracleQuote> = Vec::new(&env);
        quotes.push_back(quote(OracleProvider::pyth(), 0, 0, false));
        quotes.push_back(quote(OracleProvider::reflector(), 0, 0, false));

        let sorted = OracleResolutionManager::collect_included_sorted(&env, &quotes);
        assert_eq!(sorted.len(), 0);
    }

    #[test]
    fn test_collect_included_sorted_three_unsorted() {
        let env = make_env();
        let mut quotes: Vec<OracleQuote> = Vec::new(&env);
        quotes.push_back(quote(OracleProvider::pyth(), 500, 5_000, true));
        quotes.push_back(quote(OracleProvider::reflector(), 100, 5_000, true));
        quotes.push_back(quote(OracleProvider::band_protocol(), 300, 5_000, true));

        let sorted = OracleResolutionManager::collect_included_sorted(&env, &quotes);
        assert_eq!(sorted.len(), 3);
        assert_eq!(sorted.get(0), Some(100));
        assert_eq!(sorted.get(1), Some(300));
        assert_eq!(sorted.get(2), Some(500));
    }

    // ── weighted_median ────────────────────────────────────────────────────

    #[test]
    fn test_weighted_median_three_equal_weights_picks_middle() {
        let env = make_env();
        let mut quotes: Vec<OracleQuote> = Vec::new(&env);
        quotes.push_back(quote(OracleProvider::pyth(), 100, 5_000, true));
        quotes.push_back(quote(OracleProvider::reflector(), 200, 5_000, true));
        quotes.push_back(quote(OracleProvider::band_protocol(), 300, 5_000, true));

        let median = OracleResolutionManager::weighted_median(&quotes).unwrap();
        // total weight = 15 000, half = 7 500.
        // After price 100 cumulative = 5 000 < 7 500 → continue.
        // After price 200 cumulative = 10 000 ≥ 7 500 → result = 200.
        assert_eq!(median, 200);
    }

    #[test]
    fn test_weighted_median_high_weight_on_high_price_skews_up() {
        let env = make_env();
        let mut quotes: Vec<OracleQuote> = Vec::new(&env);
        // Low price, low weight.
        quotes.push_back(quote(OracleProvider::pyth(), 100, 1_000, true));
        // High price, very high weight.
        quotes.push_back(quote(OracleProvider::reflector(), 300, 9_000, true));

        // total = 10 000, half = 5 000.
        // After p=100, cumulative = 1 000 < 5 000 → continue.
        // After p=300, cumulative = 10 000 ≥ 5 000 → result = 300.
        let median = OracleResolutionManager::weighted_median(&quotes).unwrap();
        assert_eq!(median, 300);
    }

    #[test]
    fn test_weighted_median_high_weight_on_low_price_stays_low() {
        let env = make_env();
        let mut quotes: Vec<OracleQuote> = Vec::new(&env);
        quotes.push_back(quote(OracleProvider::reflector(), 100, 9_000, true));
        quotes.push_back(quote(OracleProvider::pyth(), 300, 1_000, true));

        // total = 10 000, half = 5 000.
        // After p=100, cumulative = 9 000 ≥ 5 000 → result = 100.
        let median = OracleResolutionManager::weighted_median(&quotes).unwrap();
        assert_eq!(median, 100);
    }

    #[test]
    fn test_weighted_median_single_included_quote() {
        let env = make_env();
        let mut quotes: Vec<OracleQuote> = Vec::new(&env);
        quotes.push_back(quote(OracleProvider::reflector(), 250, 5_000, true));
        quotes.push_back(quote(OracleProvider::pyth(), 0, 0, false));

        let median = OracleResolutionManager::weighted_median(&quotes).unwrap();
        assert_eq!(median, 250);
    }

    #[test]
    fn test_weighted_median_no_included_returns_error() {
        let env = make_env();
        let mut quotes: Vec<OracleQuote> = Vec::new(&env);
        quotes.push_back(quote(OracleProvider::pyth(), 0, 0, false));
        quotes.push_back(quote(OracleProvider::reflector(), 0, 0, false));

        assert!(
            OracleResolutionManager::weighted_median(&quotes).is_err(),
            "no included quotes must return OracleNoConsensus"
        );
    }

    // ── average_included_price ─────────────────────────────────────────────

    #[test]
    fn test_average_included_price_two_quotes() {
        let env = make_env();
        let mut quotes: Vec<OracleQuote> = Vec::new(&env);
        quotes.push_back(quote(OracleProvider::reflector(), 100, 5_000, true));
        quotes.push_back(quote(OracleProvider::band_protocol(), 300, 5_000, true));
        quotes.push_back(quote(OracleProvider::pyth(), 0, 0, false)); // excluded

        assert_eq!(OracleResolutionManager::average_included_price(&quotes), 200);
    }

    #[test]
    fn test_average_included_price_all_excluded_returns_zero() {
        let env = make_env();
        let mut quotes: Vec<OracleQuote> = Vec::new(&env);
        quotes.push_back(quote(OracleProvider::pyth(), 0, 0, false));

        assert_eq!(OracleResolutionManager::average_included_price(&quotes), 0);
    }

    // ── price_variance ─────────────────────────────────────────────────────

    #[test]
    fn test_price_variance_identical_prices_is_zero() {
        let env = make_env();
        let mut quotes: Vec<OracleQuote> = Vec::new(&env);
        quotes.push_back(quote(OracleProvider::reflector(), 200, 5_000, true));
        quotes.push_back(quote(OracleProvider::band_protocol(), 200, 5_000, true));

        let var = OracleResolutionManager::price_variance(&quotes, 200);
        assert_eq!(var, 0, "identical prices have zero variance");
    }

    #[test]
    fn test_price_variance_symmetric_spread() {
        let env = make_env();
        let mut quotes: Vec<OracleQuote> = Vec::new(&env);
        // avg = 200; deviations = ±100; squared / 10 000 = 1 each.
        quotes.push_back(quote(OracleProvider::reflector(), 100, 5_000, true));
        quotes.push_back(quote(OracleProvider::band_protocol(), 300, 5_000, true));

        let var = OracleResolutionManager::price_variance(&quotes, 200);
        // sum_sq = (100²/10 000) + (100²/10 000) = 1 + 1 = 2; count = 2; result = 1.
        assert_eq!(var, 1);
    }

    // ── aggregate_confidence ───────────────────────────────────────────────

    #[test]
    fn test_aggregate_confidence_three_sources_max_weight() {
        let env = make_env();
        let mut quotes: Vec<OracleQuote> = Vec::new(&env);
        for prov in [
            OracleProvider::pyth(),
            OracleProvider::reflector(),
            OracleProvider::band_protocol(),
        ] {
            quotes.push_back(OracleQuote {
                provider: prov,
                price: 1_000,
                confidence_bps: 0,
                weight_bps: 10_000,
                included: true,
            });
        }
        // base = 90, bonus = avg_weight(10 000) / 1 000 = 10 → total = 100.
        assert_eq!(OracleResolutionManager::aggregate_confidence(3, &quotes), 100);
    }

    #[test]
    fn test_aggregate_confidence_two_sources_medium_weight() {
        let env = make_env();
        let mut quotes: Vec<OracleQuote> = Vec::new(&env);
        quotes.push_back(quote(OracleProvider::reflector(), 1_000, 5_000, true));
        quotes.push_back(quote(OracleProvider::band_protocol(), 1_000, 5_000, true));
        quotes.push_back(quote(OracleProvider::pyth(), 0, 0, false));

        // base = 75, bonus = avg_weight(5 000) / 1 000 = 5 → total = 80.
        assert_eq!(OracleResolutionManager::aggregate_confidence(2, &quotes), 80);
    }

    #[test]
    fn test_aggregate_confidence_one_source() {
        let env = make_env();
        let mut quotes: Vec<OracleQuote> = Vec::new(&env);
        quotes.push_back(quote(OracleProvider::reflector(), 1_000, 10_000, true));
        quotes.push_back(quote(OracleProvider::pyth(), 0, 0, false));
        quotes.push_back(quote(OracleProvider::band_protocol(), 0, 0, false));

        // base = 60, bonus = 10 → total = 70.
        assert_eq!(OracleResolutionManager::aggregate_confidence(1, &quotes), 70);
    }

    // ── set_median_config / get_median_config ──────────────────────────────

    #[test]
    fn test_set_and_get_median_config_round_trips() {
        let env = make_env();
        let pyth_addr = Address::generate(&env);
        let refl_addr = Address::generate(&env);
        let band_addr = Address::generate(&env);

        let config = MedianOracleConfig {
            pyth_address: pyth_addr.clone(),
            reflector_address: refl_addr.clone(),
            band_address: band_addr.clone(),
            max_deviation_bps: 200,
            min_sources: 2,
        };
        OracleResolutionManager::set_median_config(&env, &config);

        let loaded = OracleResolutionManager::get_median_config(&env)
            .expect("config should be present after set");
        assert_eq!(loaded.max_deviation_bps, 200);
        assert_eq!(loaded.min_sources, 2);
        assert_eq!(loaded.pyth_address, pyth_addr);
        assert_eq!(loaded.reflector_address, refl_addr);
        assert_eq!(loaded.band_address, band_addr);
    }

    #[test]
    fn test_get_median_config_returns_error_when_not_set() {
        // Fresh environment has no stored config.
        let env = make_env();
        assert!(
            OracleResolutionManager::get_median_config(&env).is_err(),
            "missing config must return ConfigNotFound"
        );
    }

    #[test]
    fn test_set_median_config_overwrites_previous() {
        let env = make_env();
        let first = MedianOracleConfig {
            pyth_address: Address::generate(&env),
            reflector_address: Address::generate(&env),
            band_address: Address::generate(&env),
            max_deviation_bps: 100,
            min_sources: 1,
        };
        OracleResolutionManager::set_median_config(&env, &first);

        let updated_band = Address::generate(&env);
        let second = MedianOracleConfig {
            band_address: updated_band.clone(),
            max_deviation_bps: 300,
            min_sources: 2,
            ..first.clone()
        };
        OracleResolutionManager::set_median_config(&env, &second);

        let loaded = OracleResolutionManager::get_median_config(&env).unwrap();
        assert_eq!(loaded.max_deviation_bps, 300, "config should be overwritten");
        assert_eq!(loaded.band_address, updated_band);
    }

    // ── fetch_quote ────────────────────────────────────────────────────────
    // fetch_quote absorbs oracle errors into included=false.
    // We test it indirectly via collect_included_sorted and weighted_median
    // since the concrete oracle impls require live WASM env contexts.
    // The BandProtocolOracle::parse_feed_id path is exercised by the
    // oracle-integration tests that mock the Band WASM binary.

    // ── Outlier detection via resolve_with_median integration path ─────────
    // The full resolve_with_median integration (market + live oracles) is
    // tested in the oracle_fallback_timeout_tests module, which sets up a
    // complete contract environment.  Here we verify the outlier-filter
    // arithmetic in isolation using concrete numbers.

    #[test]
    fn test_outlier_detection_skips_deviant_quote() {
        // Prices: 100 (Pyth), 102 (Reflector), 200 (Band, outlier at 200 %)
        // baseline median of [100, 102, 200] → 102
        // deviation of Band = |200 - 102| * 10 000 / 102 = 9 607 bps > 200 bps
        // → Band should be marked as outlier.
        let env = make_env();
        let mut quotes: Vec<OracleQuote> = Vec::new(&env);
        quotes.push_back(quote(OracleProvider::pyth(), 100, 5_000, true));
        quotes.push_back(quote(OracleProvider::reflector(), 102, 5_000, true));
        quotes.push_back(quote(OracleProvider::band_protocol(), 200, 5_000, true));

        let max_dev_bps: u32 = 200; // 2 %
        let baseline_prices = OracleResolutionManager::collect_included_sorted(&env, &quotes);
        let baseline_median = OracleResolutionManager::simple_median(&baseline_prices);
        assert_eq!(baseline_median, 102);

        // Manually apply the same filter logic as resolve_with_median.
        let mut filtered: Vec<OracleQuote> = Vec::new(&env);
        for q in quotes.iter() {
            let mut out = q.clone();
            if out.included && baseline_median > 0 {
                let abs_diff: i128 = if out.price > baseline_median {
                    out.price.saturating_sub(baseline_median)
                } else {
                    baseline_median.saturating_sub(out.price)
                };
                let dev_bps: u64 = (abs_diff as u64)
                    .saturating_mul(10_000)
                    .saturating_div(baseline_median as u64);
                if dev_bps > max_dev_bps as u64 {
                    out.included = false;
                }
            }
            filtered.push_back(out);
        }

        let mut included_after: u32 = 0;
        for q in filtered.iter() {
            if q.included {
                included_after += 1;
            }
        }
        assert_eq!(included_after, 2, "outlier (Band at 200) must be excluded");

        // Weighted median of [100, 102] with equal weights = 100 (first whose
        // cumulative weight ≥ half).
        let wm = OracleResolutionManager::weighted_median(&filtered).unwrap();
        // total weight = 5000+5000=10000, half=5001.
        // After price 100: cumulative=5000 < 5001 → continue.
        // After price 102: cumulative=10000 ≥ 5001 → result=102.
        assert_eq!(wm, 102);
    }

    #[test]
    fn test_no_outlier_when_all_prices_close() {
        let env = make_env();
        let mut quotes: Vec<OracleQuote> = Vec::new(&env);
        quotes.push_back(quote(OracleProvider::pyth(), 1_000, 5_000, true));
        quotes.push_back(quote(OracleProvider::reflector(), 1_010, 5_000, true));
        quotes.push_back(quote(OracleProvider::band_protocol(), 1_020, 5_000, true));

        let max_dev_bps: u32 = 200;
        let baseline = OracleResolutionManager::collect_included_sorted(&env, &quotes);
        let bm = OracleResolutionManager::simple_median(&baseline);
        assert_eq!(bm, 1_010);

        // Deviation of 1 000 from 1 010 = 10 * 10 000 / 1 010 ≈ 99 bps < 200 → included.
        // Deviation of 1 020 from 1 010 = 10 * 10 000 / 1 010 ≈ 99 bps < 200 → included.
        for q in quotes.iter() {
            if q.included {
                let abs_diff: i128 = (q.price - bm).abs();
                let dev_bps: u64 = (abs_diff as u64)
                    .saturating_mul(10_000)
                    .saturating_div(bm as u64);
                assert!(
                    dev_bps <= max_dev_bps as u64,
                    "price {} should not be an outlier (dev_bps={})",
                    q.price,
                    dev_bps
                );
            }
        }
    }
}

// ===== ORACLE CALLBACK AUTHENTICATION INTEGRATION =====

/// Oracle callback authentication integration for market resolution
///
/// This module integrates the oracle callback authentication system with market resolution,
/// ensuring that only authenticated oracle callbacks can update market outcomes.
pub struct OracleCallbackResolver;

impl OracleCallbackResolver {
    /// Process authenticated oracle callback for market resolution
    ///
    /// This method authenticates an oracle callback and processes the data for market resolution.
    /// It integrates with the resolution system to update market outcomes based on authenticated oracle data.
    ///
    /// # Arguments
    /// * `env` - Soroban environment
    /// * `caller` - Address of the calling oracle contract
    /// * `callback_data` - Authenticated callback data from the oracle
    /// * `market_id` - Market identifier to resolve
    ///
    /// # Returns
    /// * `Ok(())` if callback is processed and market is updated
    /// * `Err(Error)` if authentication fails or processing fails
    ///
    /// # Security Notes
    ///
    /// This method ensures that only authorized oracle contracts can update market outcomes
    /// through comprehensive authentication checks.
    pub fn process_authenticated_callback(
        env: &Env,
        caller: &Address,
        callback_data: &crate::oracles::OracleCallbackData,
        market_id: &Symbol,
    ) -> Result<(), Error> {
        // Create authentication system
        let auth = crate::oracles::OracleCallbackAuth::new(env);

        // Authenticate and process the callback
        auth.authenticate_and_process(caller, callback_data)?;

        // Update market resolution based on authenticated oracle data
        Self::update_market_resolution(env, callback_data, market_id)?;

        Ok(())
    }

    /// Update market resolution based on authenticated oracle data
    ///
    /// # Arguments
    /// * `env` - Soroban environment
    /// * `callback_data` - Authenticated callback data
    /// * `market_id` - Market identifier to update
    ///
    /// # Returns
    /// * `Ok(())` if market resolution is updated successfully
    /// * `Err(Error)` if update fails
    fn update_market_resolution(
        env: &Env,
        callback_data: &crate::oracles::OracleCallbackData,
        market_id: &Symbol,
    ) -> Result<(), Error> {
        // Get market state manager
        let market = MarketStateManager::get_market(env, market_id)?;

        // Validate market is ready for resolution
        OracleResolutionValidator::validate_market_for_oracle_resolution(env, &market)?;

        // Determine outcome based on oracle data
        let outcome = Self::determine_outcome_from_oracle_data(callback_data, &market)?;

        // Create oracle resolution with all required fields
        let resolution = OracleResolution {
            market_id: market_id.clone(),
            feed_id: callback_data.feed_id.clone(),
            comparison: String::from_str(env, "eq"),
            provider: market.oracle_config.provider.clone(),
            price: callback_data.price,
            timestamp: callback_data.timestamp,
            oracle_result: outcome.clone(),
            threshold: market.oracle_config.threshold,
        };

        // Validate resolution
        OracleResolutionValidator::validate_oracle_resolution(env, &resolution)?;

        // Update market with oracle resolution
        let mut updated_market = market;
        updated_market.oracle_result = Some(outcome.clone());

        // Store updated market
        MarketStateManager::update_market(env, market_id, &updated_market);

        // Emit resolution event
        crate::events::EventEmitter::emit_oracle_result(
            env,
            market_id,
            &outcome,
            &String::from_str(env, "direct"),
            &String::from_str(env, "callback"),
            callback_data.price,
            0,
            &String::from_str(env, "eq"),
        );

        Ok(())
    }

    /// Determine market outcome from oracle data
    ///
    /// # Arguments
    /// * `callback_data` - Authenticated callback data
    /// * `market` - Market to determine outcome for
    ///
    /// # Returns
    /// Determined outcome string
    fn determine_outcome_from_oracle_data(
        callback_data: &crate::oracles::OracleCallbackData,
        market: &Market,
    ) -> Result<String, Error> {
        // For binary markets (yes/no), determine outcome based on price comparison
        if market.outcomes.len() == 2 {
            let first_outcome = market.outcomes.get(0).unwrap();
            let yes_bytes = first_outcome.to_bytes();
            let first_is_yes = yes_bytes.len() == 3
                && yes_bytes.get(0).unwrap_or(0) == 'y' as u8
                && yes_bytes.get(1).unwrap_or(0) == 'e' as u8
                && yes_bytes.get(2).unwrap_or(0) == 's' as u8;

            let (yes_outcome, no_outcome) = if first_is_yes {
                (
                    market.outcomes.get(0).unwrap(),
                    market.outcomes.get(1).unwrap(),
                )
            } else {
                if matches!(bet.status, BetStatus::Active) {
                    bet.status = BetStatus::Lost;
                    let _ = bets::BetStorage::store_bet(&env, &bet);
                }
            }
        }

        bettor_count += 1;
        if bettor_count % 10 == 0 {
            budget_guard.check()?;
        }
    }

    budget_guard.check()?;
    env.storage().persistent().set(&market_id, &market);

    Ok(total_distributed)
}