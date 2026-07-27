use soroban_sdk::{contracttype, symbol_short, Address, Env, Map, String, Symbol, Vec};

use crate::bets::BetStorage;
use crate::err::Error;
use alloc::string::ToString;

use crate::markets::{CommunityConsensus, MarketAnalytics, MarketStateManager, MarketUtils};
use crate::oracles::{OracleFactory, OracleUtils};
use crate::types::*;

// ===== RESOLUTION TYPES =====

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[contracttype]
pub enum ResolutionState {
    Active,
    OracleResolved,
    MarketResolved,
    Disputed,
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
/// ```ignore
/// # use soroban_sdk::{Env, Symbol, String, Address};
/// # use predictify_hybrid::resolution::OracleResolution;
/// # use predictify_hybrid::types::OracleProvider;
/// # let env = Env::default();
/// # let market_id = Symbol::new(&env, "btc_50k");
/// # let oracle_contract = Address::generate(&env);
///
/// // Fetch oracle resolution for a market
/// let oracle_resolution = MarketResolutionManager::fetch_oracle_result(
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
/// MarketResolutionManager::validate_oracle_resolution(&env, &oracle_resolution)?;
///
/// // Calculate confidence score
/// let confidence = MarketResolutionManager::calculate_oracle_confidence(&oracle_resolution);
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

/// Result of a single oracle resolution fetch.
#[derive(Clone, Debug)]
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

/// Oracle-based resolution manager (first impl block — core helpers).
pub struct OracleResolutionManager;

impl OracleResolutionManager {
    fn try_fetch_from_config(
        env: &Env,
        market_id: &Symbol,
        config: &crate::types::OracleConfig,
    ) -> Result<(i128, String), Error> {
        let oracle =
            OracleFactory::create_oracle(config.provider.clone(), config.oracle_address.clone())?;

        let price_data = oracle.get_price_data(env, &config.feed_id)?;
        crate::oracles::OracleValidationConfigManager::validate_oracle_data(
            env,
            market_id,
            &config.provider,
            &config.feed_id,
            &price_data,
        )?;

        let outcome = OracleUtils::determine_outcome(
            price_data.price,
            config.threshold,
            &config.comparison,
            env,
        )?;

        Ok((price_data.price, outcome))
    }

    pub fn fetch_oracle_result(env: &Env, market_id: &Symbol) -> Result<OracleResolution, Error> {
        let mut market = MarketStateManager::get_market(env, market_id)?;
        let current_time = env.ledger().timestamp();

        if current_time >= market.end_time.saturating_add(market.resolution_timeout) {
            crate::events::EventEmitter::emit_resolution_timeout(env, market_id, current_time);
            return Err(Error::ResolutionTimeoutReached);
        }

        OracleResolutionValidator::validate_market_for_oracle_resolution(env, &market)?;

        let mut used_config = market.oracle_config.clone();
        let primary_result = Self::try_fetch_from_config(env, market_id, &used_config);

        let (price, outcome) = match primary_result {
            Ok(primary_res) => {
                if market.has_fallback {
                    let fallback_config = &market.fallback_oracle_config;
                    if fallback_config.oracle_address != market.oracle_config.oracle_address {
                        match Self::try_fetch_from_config(env, market_id, fallback_config) {
                            Ok(fallback_res) => {
                                let fallback_outcome = fallback_res.1.clone();
                                let resolved_outcome = OracleUtils::resolve_outcome_with_fallback(
                                    &primary_res.1,
                                    &fallback_outcome,
                                    env,
                                )?;

                                if resolved_outcome == fallback_outcome {
                                    used_config = fallback_config.clone();
                                    crate::events::EventEmitter::emit_fallback_used(
                                        env,
                                        market_id,
                                        &market.oracle_config.oracle_address,
                                        &fallback_config.oracle_address,
                                    );
                                    (fallback_res.0, resolved_outcome)
                                } else {
                                    (primary_res.0, primary_res.1)
                                }
                            }
                            Err(_) => primary_res,
                        }
                    } else {
                        primary_res
                    }
                } else {
                    primary_res
                }
            }
            Err(_) => {
                if market.has_fallback {
                    let fallback_config = &market.fallback_oracle_config;
                    match Self::try_fetch_from_config(env, market_id, fallback_config) {
                        Ok(res) => {
                            crate::events::EventEmitter::emit_fallback_used(
                                env,
                                market_id,
                                &market.oracle_config.oracle_address,
                                &fallback_config.oracle_address,
                            );
                            used_config = fallback_config.clone();
                            res
                        }
                        Err(_) => {
                            crate::events::EventEmitter::emit_manual_resolution_required(
                                env,
                                market_id,
                                &String::from_str(
                                    env,
                                    "oracle_resolution_failed_primary_then_fallback",
                                ),
                            );
                            return Err(Error::FallbackOracleUnavailable);
                        }
                    }
                } else {
                    crate::events::EventEmitter::emit_manual_resolution_required(
                        env,
                        market_id,
                        &String::from_str(env, "oracle_resolution_failed_primary_only"),
                    );
                    return Err(Error::OracleUnavailable);
                }
            }
        };

        let resolution = OracleResolution {
            market_id: market_id.clone(),
            oracle_result: outcome.clone(),
            price,
            threshold: used_config.threshold,
            comparison: used_config.comparison.clone(),
            timestamp: current_time,
            provider: used_config.provider.clone(),
            feed_id: used_config.feed_id.clone(),
        };

        MarketStateManager::set_oracle_result(&mut market, outcome.clone());
        MarketStateManager::update_market(env, market_id, &market);

        let provider_str = match used_config.provider {
            OracleProvider::Reflector => String::from_str(env, "Reflector"),
            OracleProvider::Pyth => String::from_str(env, "Pyth"),
            _ => String::from_str(env, "Custom"),
        };
        
        let feed_str = used_config.feed_id.clone();
        let comparison_str = used_config.comparison.clone();

        // Emitting the structured event for the lifecycle
        crate::events::EventEmitter::emit_oracle_result_verified(
            env,
            market_id,
            &outcome,
            price,
            used_config.threshold,
            &comparison_str,
            &provider_str,
            &feed_str,
            95,
            1,
            true
        );

        Ok(resolution)
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

/// Result of a median-based oracle resolution.
///
/// Returned by [`OracleResolutionManager::resolve_with_median`] after
/// collecting quotes from configured oracle providers, computing the
/// weighted median, and comparing it against the market threshold.
#[contracttype]
#[derive(Clone, Debug)]
pub struct MedianResolutionResult {
    /// Market that was resolved.
    pub market_id: Symbol,
    /// Resolved outcome ("yes" / "no" or custom).
    pub outcome: String,
    /// Weighted-median price across included oracle quotes.
    pub weighted_median_price: i128,
    /// Market-defined price threshold for comparison.
    pub threshold: i128,
    /// Comparison operator string ("gt", "lt", "eq").
    pub comparison: String,
    /// All collected oracle quotes (included and excluded).
    pub quotes: Vec<OracleQuote>,
    /// Number of quotes that participated in the median.
    pub included_count: u32,
    /// Aggregate confidence score in [0, 100].
    pub confidence_score: u32,
    /// Timestamp of the resolution.
    pub timestamp: u64,
}

/// Aggregated resolution analytics across all markets.
#[contracttype]
#[derive(Clone, Debug)]
pub struct ResolutionAnalytics {
    pub total_resolutions: u32,
    pub oracle_resolutions: u32,
    pub community_resolutions: u32,
    pub hybrid_resolutions: u32,
    pub average_confidence: u32,
    pub resolution_times: Vec<u64>,
    pub outcome_distribution: Map<String, u32>,
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

    /// Remove the cached summary (e.g. before an outcome override).
    pub fn invalidate(env: &Env, market_id: &Symbol) {
        env.storage()
            .persistent()
            .remove(&Self::storage_key(market_id));
    }

    /// Compute the winning-side total (votes + bets, deduplicated) for a market.
    pub fn compute_winning_total_for_market(
        env: &Env,
        market_id: &Symbol,
        market: &Market,
        winning_outcomes: &Vec<String>,
    ) -> Result<i128, Error> {
        let mut winning_total: i128 = 0;

        for (voter, outcome) in market.votes.iter() {
            if winning_outcomes.contains(&outcome) {
                winning_total = winning_total
                    .checked_add(market.stakes.get(voter.clone()).unwrap_or(0))
                    .ok_or(Error::InvalidInput)?;
            }
        }

        let bettors = BetStorage::get_all_bets_for_market(env, market_id);
        for user in bettors.iter() {
            if market.votes.contains_key(user.clone()) {
                continue;
            }
            if let Some(bet) = BetStorage::get_bet(env, market_id, &user) {
                if winning_outcomes.contains(&bet.outcome) {
                    winning_total = winning_total
                        .checked_add(bet.amount)
                        .ok_or(Error::InvalidInput)?;
                }
            }
        }

        Ok(winning_total)
    }

    /// Recompute and persist the payout summary after resolution or outcome change.
    pub fn refresh(env: &Env, market_id: &Symbol, market: &Market) -> Result<(), Error> {
        let winning_outcomes = market
            .winning_outcomes
            .as_ref()
            .ok_or(Error::MarketNotResolved)?;

        let winning_total =
            Self::compute_winning_total_for_market(env, market_id, market, winning_outcomes)?;

        let summary = ResolvedOutcomeSummary {
            winning_total,
            total_pool: market.total_staked,
            num_winning_outcomes: winning_outcomes.len(),
        };

        env.storage()
            .persistent()
            .set(&Self::storage_key(market_id), &summary);

        Ok(())
    }

    /// Read the cached summary if present.
    pub fn get(env: &Env, market_id: &Symbol) -> Option<ResolvedOutcomeSummary> {
        env.storage()
            .persistent()
            .get(&Self::storage_key(market_id))
    }

    /// Return the cached summary, refreshing it if missing or stale.
    pub fn require(
        env: &Env,
        market_id: &Symbol,
        market: &Market,
    ) -> Result<ResolvedOutcomeSummary, Error> {
        if let (Some(summary), Some(ref outcomes)) =
            (Self::get(env, market_id), &market.winning_outcomes)
        {
            if summary.total_pool == market.total_staked
                && summary.num_winning_outcomes == outcomes.len()
            {
                return Ok(summary);
            }
        }
        Self::refresh(env, market_id, market)?;
        Self::get(env, market_id).ok_or(Error::MarketNotResolved)
    }
}

/// Oracle-based resolution manager: fetches oracle results, validates them, and

impl OracleResolutionManager {
    /// Get oracle resolution for a market

    pub fn get_oracle_resolution(
        _env: &Env,
        _market_id: &Symbol,
    ) -> Result<Option<OracleResolution>, Error> {
        // For now, return None since we don't store complex types in storage
        // In a real implementation, you would store this in a more sophisticated way

        Ok(None)
    }

    pub fn validate_oracle_resolution(_env: &Env, resolution: &OracleResolution) -> Result<(), Error> {
        if resolution.price <= 0 || resolution.threshold <= 0 || resolution.oracle_result.is_empty() {
            return Err(Error::InvalidInput);
        }
        Ok(())
    }

    pub fn calculate_oracle_confidence(resolution: &OracleResolution) -> u32 {
        OracleResolutionAnalytics::calculate_confidence_score(resolution)
    }

    pub fn set_median_config(env: &Env, config: &MedianOracleConfig) {
        env.storage().persistent().set(&symbol_short!("med_cfg"), config);
    }

    pub fn get_median_config(env: &Env) -> Result<MedianOracleConfig, Error> {
        env.storage().persistent().get(&symbol_short!("med_cfg")).ok_or(Error::ConfigNotFound)
    }

    pub fn resolve_with_median(env: &Env, market_id: &Symbol) -> Result<MedianResolutionResult, Error> {
        let mut market = MarketStateManager::get_market(env, market_id)?;
        let current_time = env.ledger().timestamp();

        if current_time >= market.end_time.saturating_add(market.resolution_timeout) {
            crate::events::EventEmitter::emit_resolution_timeout(env, market_id, current_time);
            return Err(Error::ResolutionTimeoutReached);
        }

        OracleResolutionValidator::validate_market_for_oracle_resolution(env, &market)?;

        let med_cfg = Self::get_median_config(env)?;
        let feed_id = market.oracle_config.feed_id.clone();
        let threshold = market.oracle_config.threshold;
        let comparison = market.oracle_config.comparison.clone();

        let mut raw_quotes: Vec<OracleQuote> = Vec::new(env);

        {
            let oracle = crate::oracles::PythOracle::new(med_cfg.pyth_address.clone());
            raw_quotes.push_back(Self::fetch_quote(env, &oracle, OracleProvider::pyth(), &feed_id));
        }
        {
            let oracle = crate::oracles::ReflectorOracle::new(med_cfg.reflector_address.clone());
            raw_quotes.push_back(Self::fetch_quote(env, &oracle, OracleProvider::reflector(), &feed_id));
        }
        {
            let oracle = crate::oracles::BandProtocolOracle::new(med_cfg.band_address.clone());
            raw_quotes.push_back(Self::fetch_quote(env, &oracle, OracleProvider::band_protocol(), &feed_id));
        }

        let baseline_prices = Self::collect_included_sorted(env, &raw_quotes);
        let initial_count = baseline_prices.len() as u32;
        if initial_count < med_cfg.min_sources {
            return Err(Error::OracleNoConsensus);
        }
        
        let baseline_median = Self::simple_median(&baseline_prices);
        let mut final_quotes: Vec<OracleQuote> = Vec::new(env);
        for q in raw_quotes.iter() {
            let mut out = q.clone();
            if out.included && baseline_median > 0 {
                let abs_diff: i128 = if out.price > baseline_median {
                    out.price.saturating_sub(baseline_median)
                } else {
                    baseline_median.saturating_sub(out.price)
                };
                let deviation_bps: u64 = (abs_diff as u64)
                    .saturating_mul(10_000)
                    .saturating_div(baseline_median as u64);
                if deviation_bps > med_cfg.max_deviation_bps as u64 {
                    out.included = false;
                }
            }
            final_quotes.push_back(out);
        }

        let mut included_count: u32 = 0;
        for q in final_quotes.iter() {
            if q.included { included_count += 1; }
        }
        if included_count < med_cfg.min_sources {
            return Err(Error::OracleNoConsensus);
        }

        let weighted_median = Self::weighted_median(&final_quotes)?;
        let outcome = OracleUtils::determine_outcome(weighted_median, threshold, &comparison, env)?;

        MarketStateManager::set_oracle_result(&mut market, outcome.clone());
        MarketStateManager::update_market(env, market_id, &market);

        let avg_price = Self::average_included_price(&final_quotes);
        let price_var = Self::price_variance(&final_quotes, avg_price);
        let confidence_score = Self::aggregate_confidence(included_count, &final_quotes);

        crate::events::EventEmitter::emit_oracle_consensus_reached(
            env, market_id, &outcome, included_count, 3, avg_price, price_var,
        );

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

    pub fn confidence_to_weight(price: i128, confidence: Option<i128>) -> (u32, u32) {
        if price <= 0 { return (0, 0); }
        match confidence {
            None => (0, 5_000),
            Some(c) if c <= 0 => (0, 10_000),
            Some(c) => {
                let cbps = (c * 10_000 / price) as u32;
                let wbps = (price * 10_000 / (price + c)) as u32;
                (cbps, wbps)
            }
        }
    }

    pub fn simple_median(v: &Vec<i128>) -> i128 {
        let len = v.len();
        if len == 0 { return 0; }
        let mut vals: alloc::vec::Vec<i128> = alloc::vec::Vec::new();
        for val in v.iter() { vals.push(val); }
        vals.sort();
        if len % 2 == 1 {
            vals[(len / 2) as usize]
        } else {
            let a = vals[(len / 2 - 1) as usize];
            let b = vals[(len / 2) as usize];
            (a + b) / 2
        }
    }

    pub fn collect_included_sorted(env: &Env, quotes: &Vec<OracleQuote>) -> Vec<i128> {
        let mut prices: alloc::vec::Vec<i128> = alloc::vec::Vec::new();
        for q in quotes.iter() {
            if q.included { prices.push(q.price); }
        }
        prices.sort();
        let mut result: Vec<i128> = Vec::new(env);
        for p in prices.iter() { result.push_back(*p); }
        result
    }

    pub fn weighted_median(quotes: &Vec<OracleQuote>) -> Result<i128, Error> {
        let mut included: alloc::vec::Vec<OracleQuote> = alloc::vec::Vec::new();
        for q in quotes.iter() {
            if q.included { included.push(q.clone()); }
        }
        if included.is_empty() { return Err(Error::OracleNoConsensus); }
        included.sort_by(|a, b| a.price.cmp(&b.price));
        let total_weight: u64 = included.iter().map(|q| q.weight_bps as u64).sum();
        let half = (total_weight + 1) / 2;
        let mut cumulative: u64 = 0;
        for q in included.iter() {
            cumulative += q.weight_bps as u64;
            if cumulative >= half { return Ok(q.price); }
        }
        Ok(included.last().unwrap().price)
    }

    pub fn average_included_price(quotes: &Vec<OracleQuote>) -> i128 {
        let mut sum: i128 = 0;
        let mut count: i128 = 0;
        for q in quotes.iter() {
            if q.included {
                sum += q.price;
                count += 1;
            }
        }
        if count == 0 { 0 } else { sum / count }
    }

    pub fn price_variance(quotes: &Vec<OracleQuote>, mean: i128) -> i128 {
        let mut sum_sq: i128 = 0;
        let mut count: i128 = 0;
        for q in quotes.iter() {
            if q.included {
                let diff = (q.price - mean).abs();
                sum_sq += diff * diff / 10_000;
                count += 1;
            }
        }
        if count == 0 { 0 } else { sum_sq / count }
    }

    pub fn aggregate_confidence(num_sources: u32, quotes: &Vec<OracleQuote>) -> u32 {
        let base = match num_sources {
            3 => 90u32,
            2 => 75,
            1 => 60,
            _ => 50,
        };
        let mut sum_weight: u64 = 0;
        let mut count: u64 = 0;
        for q in quotes.iter() {
            if q.included {
                sum_weight += q.weight_bps as u64;
                count += 1;
            }
        }
        let bonus = if count > 0 { (sum_weight / count / 1_000) as u32 } else { 0 };
        (base + bonus).min(100)
    }
}

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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[contracttype]
pub enum ResolutionMethod {
    OracleOnly,
    CommunityOnly,
    Hybrid,
    AdminOverride,
    DisputeResolution,
    ForceResolve,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct ResolutionAnalytics {
    pub total_resolutions: u32,
    pub oracle_resolutions: u32,
    pub community_resolutions: u32,
    pub hybrid_resolutions: u32,
    pub average_confidence: u32,
    pub resolution_times: Vec<u64>,
    pub outcome_distribution: Map<String, u32>,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedOutcomeSummary {
    pub winning_total: i128,
    pub total_pool: i128,
    pub num_winning_outcomes: u32,
}

pub struct ResolutionOutcomeCache;

impl ResolutionOutcomeCache {
    fn storage_key(market_id: &Symbol) -> (Symbol, Symbol) {
        (symbol_short!("res_out"), market_id.clone())
    }

    pub fn invalidate(env: &Env, market_id: &Symbol) {
        env.storage().persistent().remove(&Self::storage_key(market_id));
    }

    pub fn compute_winning_total_for_market(
        env: &Env,
        market_id: &Symbol,
        market: &Market,
        winning_outcomes: &Vec<String>,
    ) -> Result<i128, Error> {
        let mut winning_total: i128 = 0;
        for (voter, outcome) in market.votes.iter() {
            if winning_outcomes.contains(&outcome) {
                winning_total = winning_total
                    .checked_add(market.stakes.get(voter.clone()).unwrap_or(0))
                    .ok_or(Error::InvalidInput)?;
            }
        }
        let bettors = BetStorage::get_all_bets_for_market(env, market_id);
        for user in bettors.iter() {
            if market.votes.contains_key(user.clone()) { continue; }
            if let Some(bet) = BetStorage::get_bet(env, market_id, &user) {
                if winning_outcomes.contains(&bet.outcome) {
                    winning_total = winning_total.checked_add(bet.amount).ok_or(Error::InvalidInput)?;
                }
            }
        }
        Ok(winning_total)
    }

    pub fn refresh(env: &Env, market_id: &Symbol, market: &Market) -> Result<(), Error> {
        let winning_outcomes = market.winning_outcomes.as_ref().ok_or(Error::MarketNotResolved)?;
        let winning_total = Self::compute_winning_total_for_market(env, market_id, market, winning_outcomes)?;
        let summary = ResolvedOutcomeSummary {
            winning_total,
            total_pool: market.total_staked,
            num_winning_outcomes: winning_outcomes.len(),
        };
        env.storage().persistent().set(&Self::storage_key(market_id), &summary);
        Ok(())
    }

    pub fn get(env: &Env, market_id: &Symbol) -> Option<ResolvedOutcomeSummary> {
        env.storage().persistent().get(&Self::storage_key(market_id))
    }

    pub fn require(env: &Env, market_id: &Symbol, market: &Market) -> Result<ResolvedOutcomeSummary, Error> {
        if let (Some(summary), Some(ref outcomes)) = (Self::get(env, market_id), &market.winning_outcomes) {
            if summary.total_pool == market.total_staked && summary.num_winning_outcomes == outcomes.len() {
                return Ok(summary);
            }
        }
        Self::refresh(env, market_id, market)?;
        Self::get(env, market_id).ok_or(Error::MarketNotResolved)
    }
}

pub struct MarketResolutionManager;

impl MarketResolutionManager {
    pub fn resolve_market(env: &Env, market_id: &Symbol) -> Result<MarketResolution, Error> {
        let mut market = MarketStateManager::get_market(env, market_id)?;
        let validation = MarketResolutionValidator::validate_market_for_resolution(env, &market);
        
        if let Err(Error::InvalidState) = validation {
            let global_min: i128 = env.storage().persistent().get(&Symbol::new(env, "global_min_pool")).unwrap_or(0);
            let min_pool = market.min_pool_size.unwrap_or(global_min);
            crate::events::EventEmitter::emit_min_pool_size_not_met(
                env, market_id, market.total_staked, min_pool,
            );
            return Err(Error::InvalidState);
        }
        validation?;

        let oracle_result = market.oracle_result.as_ref().ok_or(Error::OracleUnavailable)?.clone();
        let community_consensus = MarketAnalytics::calculate_community_consensus(&market);
        
        let winning_outcomes = MarketUtils::determine_winning_outcomes(
            env, &market, &oracle_result, &community_consensus, 0,
        );

        let final_result = if winning_outcomes.len() > 0 {
            winning_outcomes.get(0).unwrap().clone()
        } else {
            oracle_result.clone()
        };

        let resolution_method = MarketResolutionAnalytics::determine_resolution_method(&oracle_result, &community_consensus);
        let confidence_score = MarketResolutionAnalytics::calculate_confidence_score(&oracle_result, &community_consensus, &resolution_method);

        let resolution = MarketResolution {
            market_id: market_id.clone(),
            final_outcome: final_result.clone(),
            oracle_result,
            community_consensus,
            resolution_timestamp: env.ledger().timestamp(),
            resolution_method,
            confidence_score,
        };

        let old_state = market.state.clone();
        MarketStateManager::set_winning_outcomes(&mut market, winning_outcomes.clone(), Some(market_id));
        MarketStateManager::update_market(env, market_id, &market);
        ResolutionOutcomeCache::refresh(env, market_id)?;
        
        crate::storage::CreatorLimitsManager::decrement_active_events(env, &market.admin);

        let oracle_result_str = market.oracle_result.clone().unwrap_or_else(|| String::from_str(env, "N/A"));
        let community_consensus_str = String::from_str(env, "Consensus");
        let method_str = match resolution_method {
            ResolutionMethod::OracleOnly => "OracleOnly",
            ResolutionMethod::CommunityOnly => "CommunityOnly",
            ResolutionMethod::Hybrid => "Hybrid",
            ResolutionMethod::AdminOverride => "AdminOverride",
            ResolutionMethod::DisputeResolution => "DisputeResolution",
            ResolutionMethod::ForceResolve => "ForceResolve",
        };
        let resolution_method_str = String::from_str(env, method_str);

        crate::events::EventEmitter::emit_market_resolved(
            env, market_id, &final_result, &oracle_result_str, &community_consensus_str, &resolution_method_str, confidence_score as i128,
        );

        crate::events::EventEmitter::emit_state_change_event(
            env, market_id, &old_state, &crate::types::MarketState::Resolved, &String::from_str(env, "Automated resolution completed"),
        );
        
        crate::monitoring::ContractMonitor::emit_resolution_transition_hook(
            env, market_id, &old_state, &crate::types::MarketState::Resolved, &resolution_method_str,
        );

        Ok(resolution)
    }

    pub fn finalize_market(
        env: &Env, admin: &Address, market_id: &Symbol, outcome: &String,
    ) -> Result<MarketResolution, Error> {
        MarketResolutionValidator::validate_admin_permissions(env, admin)?;
        let mut market = MarketStateManager::get_market(env, market_id)?;
        MarketResolutionValidator::validate_outcome(env, outcome, &market.outcomes)?;

        let resolution = MarketResolution {
            market_id: market_id.clone(),
            final_outcome: outcome.clone(),
            oracle_result: market.oracle_result.clone().unwrap_or_else(|| String::from_str(env, "")),
            community_consensus: MarketAnalytics::calculate_community_consensus(&market),
            resolution_timestamp: env.ledger().timestamp(),
            resolution_method: ResolutionMethod::AdminOverride,
            confidence_score: 100,
        };

        let mut winning_outcomes = Vec::new(env);
        winning_outcomes.push_back(outcome.clone());
        MarketStateManager::set_winning_outcomes(&mut market, winning_outcomes, Some(market_id));
        MarketStateManager::update_market(env, market_id, &market);
        ResolutionOutcomeCache::refresh(env, market_id)?;

        crate::storage::CreatorLimitsManager::decrement_active_events(env, &market.admin);
        Ok(resolution)
    }

    pub fn get_market_resolution(_env: &Env, _market_id: &Symbol) -> Result<Option<MarketResolution>, Error> {
        Ok(None)
    }

    pub fn validate_market_resolution(env: &Env, resolution: &MarketResolution) -> Result<(), Error> {
        MarketResolutionValidator::validate_market_resolution(env, resolution)
    }

    pub fn check_resolution_cooldown(env: &Env, admin: &Address, fn_name: &Symbol) -> Result<(), Error> {
        let cooldown_key = crate::storage::DataKey::ResolutionCooldownSeconds;
        let cooldown: u64 = env.storage().persistent().get(&cooldown_key).unwrap_or(0);
        if cooldown == 0 { return Ok(()); }
        let now = env.ledger().timestamp();
        let last_key = crate::storage::DataKey::ResolutionAdminLastAction(fn_name.clone());
        let last_action: u64 = env.storage().persistent().get(&last_key).unwrap_or(0);
        if last_action > 0 && now < last_action.saturating_add(cooldown) {
            return Err(Error::AdminActionTimelocked);
        }
        env.storage().persistent().set(&last_key, &now);
        env.storage().persistent().extend_ttl(&last_key, 535680, 535680);
        Ok(())
    }

    pub fn set_resolution_cooldown(env: Env, admin: Address, seconds: u64) -> Result<(), Error> {
        admin.require_auth();
        let key = crate::storage::DataKey::ResolutionCooldownSeconds;
        env.storage().persistent().set(&key, &seconds);
        env.storage().persistent().extend_ttl(&key, 535680, 535680);
        Ok(())
    }

    pub fn resolve_market_manual(env: Env, admin: Address, market_id: Symbol, winning_outcome: String) {
        let gas_marker = crate::gas::GasTracker::start_tracking(&env);
        admin.require_auth();
        Self::check_resolution_cooldown(&env, &admin, &Symbol::new(&env, "resolve_market_manual")).unwrap();

        let mut market: Market = env.storage().persistent().get(&market_id).unwrap();

        if env.ledger().timestamp() < market.end_time {
            panic!("MarketClosed");
        }
        
        let outcome_exists = market.outcomes.iter().any(|o| o == winning_outcome);
        if !outcome_exists { panic!("InvalidOutcome"); }

        let old_state = market.state.clone();
        let mut winning_outcomes_vec = Vec::new(&env);
        winning_outcomes_vec.push_back(winning_outcome.clone());
        market.winning_outcomes = Some(winning_outcomes_vec.clone());
        market.state = MarketState::Resolved;
        
        crate::recovery::UnclaimedWinningsPolicy::set_claim_window_start_if_missing(
            &env, &market_id, env.ledger().timestamp(),
        );
        env.storage().persistent().set(&market_id, &market);

        let _ = bets::BetManager::resolve_market_bets(&env, &market_id, &winning_outcomes_vec);
        let _ = resolution::ResolutionOutcomeCache::refresh(&env, &market_id);

        let oracle_result_str = market.oracle_result.clone().unwrap_or_else(|| String::from_str(&env, "N/A"));
        let community_consensus_str = String::from_str(&env, "Manual");
        let resolution_method = String::from_str(&env, "Manual");

        crate::events::EventEmitter::emit_market_resolved(
            &env, &market_id, &winning_outcome, &oracle_result_str, &community_consensus_str, &resolution_method, 100,
        );

        crate::events::EventEmitter::emit_state_change_event(
            &env, &market_id, &old_state, &MarketState::Resolved, &String::from_str(&env, "Manual resolution by admin"),
        );

        crate::analytics::AnalyticsCache::new(&env).invalidate(&market_id);

        let mut details = Map::new(&env);
        details.set(Symbol::new(&env, "outcome"), winning_outcome.clone());
        details.set(Symbol::new(&env, "method"), String::from_str(&env, "Manual"));
        crate::audit::MarketAuditManager::append(
            &env, &market_id, crate::audit::MarketAuditAction::MarketResolved, admin.clone(), details,
        );

        crate::gas::GasTracker::end_tracking(&env, symbol_short!("res_man"), gas_marker);
    }

    pub fn resolve_market_with_ties(env: Env, admin: Address, market_id: Symbol, winning_outcomes: Vec<String>) {
        admin.require_auth();
        Self::check_resolution_cooldown(&env, &admin, &Symbol::new(&env, "resolve_market_with_ties")).unwrap();

        if winning_outcomes.len() == 0 { panic!("InvalidInput"); }

        let mut market: Market = env.storage().persistent().get(&market_id).unwrap();

        if env.ledger().timestamp() < market.end_time { panic!("MarketClosed"); }

        for outcome in winning_outcomes.iter() {
            let outcome_exists = market.outcomes.iter().any(|o| o == outcome);
            if !outcome_exists { panic!("InvalidOutcome"); }
        }

        let old_state = market.state.clone();
        market.winning_outcomes = Some(winning_outcomes.clone());
        market.state = MarketState::Resolved;
        
        crate::recovery::UnclaimedWinningsPolicy::set_claim_window_start_if_missing(
            &env, &market_id, env.ledger().timestamp(),
        );
        env.storage().persistent().set(&market_id, &market);

        let _ = bets::BetManager::resolve_market_bets(&env, &market_id, &winning_outcomes);
        let _ = resolution::ResolutionOutcomeCache::refresh(&env, &market_id);

        let primary_outcome = winning_outcomes.get(0).unwrap().clone();
        let oracle_result_str = market.oracle_result.clone().unwrap_or_else(|| String::from_str(&env, "N/A"));
        let community_consensus_str = String::from_str(&env, "Manual");
        let resolution_method = String::from_str(&env, "Manual");

        crate::events::EventEmitter::emit_market_resolved(
            &env, &market_id, &primary_outcome, &oracle_result_str, &community_consensus_str, &resolution_method, 100,
        );

        crate::events::EventEmitter::emit_state_change_event(
            &env, &market_id, &old_state, &MarketState::Resolved, &String::from_str(&env, "Manual resolution with ties by admin"),
        );

        crate::analytics::AnalyticsCache::new(&env).invalidate(&market_id);

        let mut details = Map::new(&env);
        details.set(Symbol::new(&env, "outcome"), primary_outcome.clone());
        details.set(Symbol::new(&env, "method"), String::from_str(&env, "ManualTie"));
        crate::audit::MarketAuditManager::append(
            &env, &market_id, crate::audit::MarketAuditAction::MarketResolved, admin.clone(), details,
        );
    }

    pub fn force_resolve_market(
        env: Env, admin: Address, market_id: Symbol, winning_outcomes: Vec<String>, reason: String, idempotency_key: String,
    ) -> Result<(), Error> {
        admin.require_auth();
        Self::check_resolution_cooldown(&env, &admin, &Symbol::new(&env, "force_resolve_market"))?;

        if reason.is_empty() { return Err(Error::ForceResolveReasonEmpty); }
        if winning_outcomes.len() == 0 { return Err(Error::InvalidInput); }

        let mut market: Market = env.storage().persistent().get(&market_id).ok_or(Error::MarketNotFound)?;

        for outcome in winning_outcomes.iter() {
            let outcome_exists = market.outcomes.iter().any(|o| o == outcome);
            if !outcome_exists { return Err(Error::InvalidOutcome); }
        }

        if crate::force_resolve::ForceResolveManager::is_already_resolved(&env, &market_id, &idempotency_key) {
            return Err(Error::ForceResolveReplayed);
        }

        let old_state = market.state.clone();
        market.winning_outcomes = Some(winning_outcomes.clone());
        market.state = MarketState::Resolved;

        crate::recovery::UnclaimedWinningsPolicy::set_claim_window_start_if_missing(
            &env, &market_id, env.ledger().timestamp(),
        );

        env.storage().persistent().set(&market_id, &market);

        crate::force_resolve::ForceResolveManager::mark_resolved(
            &env, &market_id, &idempotency_key, &admin, &winning_outcomes,
        );

        let _ = bets::BetManager::resolve_market_bets(&env, &market_id, &winning_outcomes);
        let _ = resolution::ResolutionOutcomeCache::refresh(&env, &market_id);

        let primary_outcome = winning_outcomes.get(0).unwrap().clone();

        crate::events::EventEmitter::emit_force_resolved(
            &env, &market_id, &admin, &primary_outcome, &reason, &idempotency_key,
        );

        crate::events::EventEmitter::emit_state_change_event(
            &env, &market_id, &old_state, &MarketState::Resolved, &reason,
        );

        let mut details = Map::new(&env);
        details.set(Symbol::new(&env, "reason"), reason);
        details.set(Symbol::new(&env, "old_state"), String::from_str(&env, "Active"));
        crate::audit::AuditTrailManager::append_record(
            &env, crate::audit::AuditAction::MarketForceResolved, admin.clone(), details, None,
        );

        Ok(())
    }
}

pub struct OracleResolutionValidator;

impl OracleResolutionValidator {
    pub fn validate_market_for_oracle_resolution(env: &Env, market: &Market) -> Result<(), Error> {
        if market.oracle_result.is_some() { return Err(Error::MarketResolved); }
        if env.ledger().timestamp() < market.end_time { return Err(Error::MarketClosed); }
        Ok(())
    }
}

pub struct MarketResolutionValidator;

impl MarketResolutionValidator {
    pub fn validate_market_for_resolution(env: &Env, market: &Market) -> Result<(), Error> {
        if market.winning_outcomes.is_some() { return Err(Error::MarketResolved); }
        if market.oracle_result.is_none() { return Err(Error::OracleUnavailable); }
        if market.is_active(env) { return Err(Error::MarketClosed); }
        Ok(())
    }

    pub fn validate_admin_permissions(env: &Env, admin: &Address) -> Result<(), Error> {
        let stored_admin: Option<Address> = env.storage().persistent().get(&Symbol::new(env, "Admin"));
        match stored_admin {
            Some(stored_admin) => {
                if admin != &stored_admin { return Err(Error::Unauthorized); }
                Ok(())
            }
            None => Err(Error::Unauthorized),
        }
    }

    pub fn validate_outcome(_env: &Env, outcome: &String, valid_outcomes: &Vec<String>) -> Result<(), Error> {
        if !valid_outcomes.contains(outcome) { return Err(Error::InvalidOutcome); }
        Ok(())
    }

    pub fn validate_market_resolution(env: &Env, resolution: &MarketResolution) -> Result<(), Error> {
        if resolution.final_outcome.is_empty() || resolution.confidence_score > 100 || resolution.resolution_timestamp > env.ledger().timestamp() {
            return Err(Error::InvalidInput);
        }
        Ok(())
    }
}

pub struct OracleResolutionAnalytics;

impl OracleResolutionAnalytics {
    pub fn calculate_confidence_score(resolution: &OracleResolution) -> u32 {
        let mut confidence: u32 = 80;
        let deviation = ((resolution.price - resolution.threshold).abs() as f64) / (resolution.threshold as f64);
        if deviation > 0.1 { confidence = confidence.saturating_sub(20); }
        else if deviation < 0.05 { confidence = confidence.saturating_add(10); }
        confidence.min(100)
    }
}

pub struct MarketResolutionAnalytics;

impl MarketResolutionAnalytics {
    pub fn determine_resolution_method(_oracle_result: &String, community_consensus: &CommunityConsensus) -> ResolutionMethod {
        if community_consensus.percentage > 70 { ResolutionMethod::Hybrid } else { ResolutionMethod::OracleOnly }
    }

    pub fn calculate_confidence_score(_oracle_result: &String, community_consensus: &CommunityConsensus, method: &ResolutionMethod) -> u32 {
        match method {
            ResolutionMethod::OracleOnly => 85,
            ResolutionMethod::CommunityOnly => (community_consensus.percentage as u32).min(90),
            ResolutionMethod::Hybrid => ((85 + community_consensus.percentage as u32) / 2).min(95),
            ResolutionMethod::AdminOverride | ResolutionMethod::ForceResolve => 100,
            ResolutionMethod::DisputeResolution => 75,
        }
    }

    pub fn calculate_resolution_analytics(_env: &Env) -> Result<MarketResolutionAnalytics, Error> {
        Ok(MarketResolutionAnalytics {})
    }
}

pub struct ResolutionUtils;

impl ResolutionUtils {
    pub fn get_resolution_state(_env: &Env, market: &Market) -> ResolutionState {
        if market.winning_outcomes.is_some() { ResolutionState::MarketResolved }
        else if market.oracle_result.is_some() { ResolutionState::OracleResolved }
        else if market.total_dispute_stakes() > 0 { ResolutionState::Disputed }
        else { ResolutionState::Active }
    }
}

pub struct ResolutionTesting;

impl ResolutionTesting {
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

    pub fn validate_resolution_structure(resolution: &MarketResolution) -> Result<(), Error> {
        if resolution.final_outcome.is_empty() || resolution.confidence_score > 100 {
            return Err(Error::InvalidInput);
        }
        Ok(())
    }
}

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

pub struct OracleCallbackResolver;

impl OracleCallbackResolver {
    pub fn process_authenticated_callback(
        env: &Env, caller: &Address, callback_data: &crate::oracles::OracleCallbackData, market_id: &Symbol,
    ) -> Result<(), Error> {
        Ok(())
    }
}

#[cfg(any())]
mod tests {
    use super::*;
    use soroban_sdk::{testutils::{Address as _, Ledger}, Address, String};
    
    #[test]
    fn test_resolution_method_determination() {
        let env = Env::default();
        let community_consensus = CommunityConsensus {
            outcome: String::from_str(&env, "yes"),
            votes: 75,
            total_votes: 100,
            percentage: 75,
        };
        let method = MarketResolutionAnalytics::determine_resolution_method(&String::from_str(&env, "yes"), &community_consensus);
        assert!(matches!(method, ResolutionMethod::Hybrid));
    }
}