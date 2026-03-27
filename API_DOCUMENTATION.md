# Predictify Hybrid Contract - API Documentation

## Overview

This document provides a complete API reference for the Predictify Hybrid smart contract. It details all public modules, their purposes, and available functions organized by functional domain.

---

## Table of Contents

1. [Admin Management](#admin-management)
2. [Balance Management](#balance-management)
3. [Bet Management](#bet-management)
4. [Market Management](#market-management)
5. [Query Functions](#query-functions)
6. [Voting & Disputes](#voting--disputes)
7. [Resolution Management](#resolution-management)
8. [Oracle Management](#oracle-management)
9. [Dispute Management](#dispute-management)
10. [Event System](#event-system)
11. [Governance](#governance)
12. [Configuration](#configuration)
13. [Fee Management](#fee-management)
14. [Market Extensions](#market-extensions)
15. [Monitoring System](#monitoring-system)

---

## Admin Management

**Module**: `admin.rs`

**Purpose**: Comprehensive admin system with role-based access control, multi-admin support, and action logging.

### Core Structures

- `AdminRole` (enum) - SuperAdmin, MarketAdmin, ConfigAdmin, FeeAdmin, ReadOnlyAdmin
- `AdminPermission` (enum) - Permissions like Initialize, CreateMarket, UpdateFees, ManageDispute
- `AdminAction` (struct) - Records of admin actions with timestamp and success status
- `AdminRoleAssignment` (struct) - Tracks admin roles, permissions, and activation status
- `MultisigConfig` (struct) - Configuration for multisig approval system
- `PendingAdminAction` (struct) - Pending actions awaiting multisig approval

### Primary Functions

**AdminInitializer**
- `initialize(env, admin)` - Initialize contract with primary admin
- `initialize_with_config(env, admin, config)` - Initialize with custom config
- `validate_initialization_params(env, admin)` - Validate initialization parameters

**AdminAccessControl**
- `validate_permission(env, admin, permission)` - Check if admin has required permission
- `require_admin_auth(env, admin)` - Require admin authentication
- `require_not_paused(env)` - Ensure contract is not paused

**ContractPauseManager**
- `is_contract_paused(env)` - Check if contract is paused
- `pause(env, admin)` - Pause contract operations
- `unpause(env, admin)` - Resume contract operations
- `transfer_admin(env, current_admin, new_admin)` - Transfer admin rights

**AdminRoleManager**
- `assign_role(env, admin, target, role, permissions)` - Assign role to admin
- `get_admin_role(env, admin)` - Get admin's current role
- `has_permission(env, admin, permission)` - Check specific permission
- `get_permissions_for_role(env, role)` - List all permissions for a role
- `deactivate_role(env, admin)` - Deactivate admin role

**AdminManager**
- `add_admin(env, admin, target, role)` - Add new admin
- `remove_admin(env, admin, target)` - Remove admin
- `update_admin_role(env, admin, target, new_role)` - Change admin role
- `get_admin_roles(env)` - Get all admin role assignments
- `deactivate_admin(env, admin, target)` - Deactivate admin
- `reactivate_admin(env, admin, target)` - Reactivate admin

**AdminFunctions**
- `close_market(env, admin, market_id)` - Close a market
- `finalize_market(env, admin, market_id, winning_outcome)` - Finalize market resolution
- `extend_market_duration(env, admin, market_id, additional_days)` - Extend market end time
- `update_fee_config(env, admin, fee_percentage, creation_fee)` - Update fee configuration
- `update_contract_config(env, admin, config)` - Update contract settings
- `reset_config_to_defaults(env, admin)` - Reset all configs to defaults

**AdminActionLogger**
- `log_action(env, admin, action, target, parameters, success)` - Log admin action
- `get_admin_actions(env, limit)` - Retrieve admin action history
- `get_admin_actions_for_admin(env, admin, limit)` - Get actions by specific admin

**Multisig System**
- `MultisigManager::set_threshold(env, admin, threshold)` - Set approval threshold
- `MultisigManager::get_config(env)` - Get multisig configuration
- `MultisigManager::create_pending_action(env, admin, action_type, target)` - Create pending action
- `MultisigManager::approve_action(env, admin, action_id)` - Approve pending action
- `MultisigManager::execute_action(env, action_id)` - Execute approved action
- `MultisigManager::requires_multisig(env)` - Check if multisig is enabled

---

## Balance Management

**Module**: `balances.rs`

**Purpose**: Manages user deposits, withdrawals, and balance tracking per asset with circuit breaker support.

### Core Functions

**BalanceManager**
- `deposit(env, user, asset, amount)` - Deposit funds into user balance
- `withdraw(env, user, asset, amount)` - Withdraw funds from user balance (checks circuit breaker)
- `get_balance(env, user, asset)` - Get current user balance for asset

---

## Bet Management

**Module**: `bets.rs`

**Purpose**: Handles bet placement, tracking, resolution, and payout calculation with limits and validation.

### Core Structures

- `Bet` (struct) - User bet with market ID, user, outcome, amount, and status
- `BetStats` (struct) - Market-wide bet statistics and outcome distribution
- `BetLimits` (struct) - Minimum and maximum bet amounts

### Primary Functions

**Bet Limits Management**
- `get_effective_bet_limits(env, market_id)` - Get active bet limits for market
- `set_global_bet_limits(env, limits)` - Set platform-wide bet limits
- `set_event_bet_limits(env, market_id, limits)` - Set market-specific limits

**BetManager**
- `place_bet(env, user, market_id, outcome, amount)` - Place single bet on outcome
- `place_bets(env, user, market_id, bets)` - Place multiple bets in batch
- `has_user_bet(env, market_id, user)` - Check if user has active bet
- `get_bet(env, market_id, user)` - Retrieve user's bet details
- `get_market_bet_stats(env, market_id)` - Get market betting statistics
- `resolve_market_bets(env, market_id, winning_outcome)` - Mark winning bets
- `refund_market_bets(env, market_id)` - Refund all bets (cancelled market)
- `calculate_bet_payout(env, market_id, user, winning_outcome)` - Calculate winnings

**BetStorage**
- `store_bet(env, bet)` - Store bet in persistent storage
- `get_bet(env, market_id, user)` - Retrieve stored bet
- `remove_bet(env, market_id, user)` - Delete bet from storage
- `get_market_bet_stats(env, market_id)` - Get aggregated betting stats
- `get_all_bets_for_market(env, market_id)` - Get all bettors for market

**BetValidator**
- `validate_market_for_betting(env, market)` - Check market state for bets
- `validate_bet_parameters(env, user, amount, outcome)` - Validate bet inputs
- `validate_bet_amount_against_limits(env, market_id, amount)` - Check amount limits
- `validate_bet_amount(amount)` - Check absolute amount constraints

**BetUtils**
- `lock_funds(env, user, amount)` - Lock user funds for bet
- `unlock_funds(env, user, amount)` - Release locked funds
- `get_contract_balance(env)` - Get total contract balance
- `has_sufficient_balance(env, user, amount)` - Check user has enough funds

**BetAnalytics**
- `calculate_implied_probability(env, market_id, outcome)` - Deduce outcome probability
- `calculate_payout_multiplier(env, market_id, outcome)` - Get winnings multiplier
- `get_market_summary(env, market_id)` - Get comprehensive market summary

---

## Market Management

**Module**: `markets.rs`

**Purpose**: Core market creation, state management, and lifecycle operations including pause/resume.

### Core Structures

- `Market` (struct) - Complete market with question, outcomes, state, and oracle config
- `MarketState` (enum) - Active, Closed, Resolved, Disputed, Paused, Finalized
- `MarketStats` (struct) - Market statistics and analytics
- `MarketStatus` (enum) - Status for API responses
- `OracleConfig` (struct) - Oracle provider configuration

### Market Creation

**MarketCreator**
- `create_market(env, admin, question, outcomes, duration_days, oracle_config)` - Create generic market
- `create_reflector_market(env, admin, oracle_address, question, outcomes, duration_days, asset, threshold, comparison)` - Create Reflector-based market
- `create_pyth_market(env, admin, oracle_address, question, outcomes, duration_days, feed_id, threshold, comparison)` - Create Pyth-based market
- `create_reflector_asset_market(env, admin, question, outcomes, duration_days, asset_symbol, threshold, comparison)` - Create market using Reflector asset

### Market State Management

**MarketStateManager**
- `get_market(env, market_id)` - Retrieve market details
- `update_market(env, market_id, market)` - Update market in storage
- `update_description(env, market_id, new_description)` - Change market question
- `remove_market(env, market_id)` - Delete market from storage
- `add_vote(env, market_id, user, outcome, stake)` - Record user vote
- `add_dispute_stake(env, market_id, user, stake, reason)` - Record dispute stake
- `mark_claimed(market, user)` - Mark user winnings as claimed
- `set_oracle_result(market, result)` - Store oracle's result
- `set_winning_outcome(market, outcome)` - Mark winning prediction
- `set_winning_outcomes(market, outcomes)` - Mark multiple winning outcomes
- `mark_fees_collected(market)` - Flag market fees as collected
- `extend_for_dispute(market, env, extension_hours)` - Extend for dispute period

### Market Validation

**MarketValidator**
- `validate_market_params(env, question, outcomes, duration)` - Validate market creation inputs
- `validate_oracle_config(env, config)` - Validate oracle configuration
- `validate_market_for_voting(env, market)` - Check market allows voting
- `validate_market_for_resolution(env, market)` - Check market ready for resolution
- `validate_outcome(env, market, outcome)` - Verify outcome in market
- `validate_stake(stake, min_stake)` - Validate stake amount

### Market Analytics

**MarketAnalytics**
- `get_market_stats(market)` - Calculate comprehensive market statistics
- `calculate_winning_stats(market, winning_outcome)` - Stats for winning outcome
- `get_user_stats(market, user)` - User-specific market statistics
- `calculate_community_consensus(market)` - Community agreement metrics

### Market Utilities

**MarketUtils**
- `generate_market_id(env)` - Create unique market identifier
- `calculate_end_time(env, duration_days)` - Calculate market end timestamp
- `process_creation_fee(env, admin)` - Deduct and record creation fee
- `get_token_client(env)` - Get token contract client
- `calculate_payout(env, market_id, user, outcome)` - Calculate user winnings
- `determine_final_result(env, market, oracle_result)` - Decide final outcome
- `determine_winning_outcomes(env, market, oracle_result)` - Determine all winners

### Market Pause Management

**MarketPauseManager**
- `pause_market(env, admin, market_id, reason)` - Temporarily suspend market
- `resume_market(env, admin, market_id)` - Reactivate paused market
- `validate_pause_conditions(env, market)` - Check market can be paused
- `is_market_paused(env, market_id)` - Check pause status
- `auto_resume_on_expiry(env, market_id)` - Auto-resume after pause period
- `get_market_pause_status(env, market_id)` - Get detailed pause info

---

## Query Functions

**Module**: `queries.rs`

**Purpose**: Read-only query interface for retrieving market, user, and contract state information.

### Primary Functions

**QueryManager**

**Market/Event Queries**
- `query_event_details(env, market_id)` - Get complete market information
- `query_event_status(env, market_id)` - Get market status and end time
- `get_all_markets(env)` - Get list of all market IDs

**User Bet Queries**
- `query_user_bet(env, user, market_id)` - Get user's participation details
- `query_user_bets(env, user)` - Get all user's bets across markets
- `query_user_balance(env, user)` - Get user balance for each asset
- `query_market_pool(env, market_id)` - Get market pool statistics

**Contract State Queries**
- `query_total_pool_size(env)` - Get total platform staking
- `query_contract_state(env)` - Get overall contract state and status

---

## Voting & Disputes

**Module**: `voting.rs`

**Purpose**: Voting mechanism, dispute management, and payout calculation with dynamic thresholds.

### Core Structures

- `Vote` (struct) - User vote with outcome, stake, and timestamp
- `VotingStats` (struct) - Market voting statistics and participation metrics
- `PayoutData` (struct) - Payout calculation data for winners
- `DisputeThreshold` (struct) - Dynamic dispute cost threshold
- `ThresholdAdjustmentFactors` (struct) - Factors adjusting dispute threshold

### Primary Functions

**VotingManager**
- `process_vote(env, user, market_id, outcome, stake)` - Record user vote with stake
- `process_dispute(env, user, market_id, stake, reason)` - Initiate dispute challenge
- `process_claim(env, user, market_id)` - Claim winnings after resolution
- `collect_fees(env, admin, market_id)` - Collect platform fees
- `calculate_dispute_threshold(env, market_id)` - Determine current dispute cost
- `update_dispute_thresholds(env, market_id)` - Recalculate thresholds
- `get_threshold_history(env, market_id, limit)` - Get historical threshold data

**ThresholdUtils**
- `get_threshold_adjustment_factors(env, market_id)` - Get adjustment factors
- `adjust_threshold_by_market_size(env, total_staked)` - Apply size-based adjustment
- `modify_threshold_by_activity(env, total_votes)` - Apply activity-based adjustment
- `calculate_complexity_factor(env, market)` - Calculate complexity adjustment
- `calculate_adjusted_threshold(env, base, factors)` - Apply all adjustments
- `store_dispute_threshold(env, threshold)` - Save threshold data
- `get_dispute_threshold(env, market_id)` - Retrieve stored threshold
- `validate_dispute_threshold(threshold, market_id)` - Validate threshold value

**VotingValidator**
- `validate_user_authentication(user)` - Verify user authentication
- `validate_admin_authentication(env, admin)` - Verify admin authentication
- `validate_market_for_voting(env, market)` - Check voting is allowed
- `validate_market_for_dispute(env, market)` - Check disputes are allowed
- `validate_market_for_claim(env, market, user)` - Check user can claim
- `validate_vote_parameters(env, user, market, outcome, stake)` - Validate vote inputs
- `validate_dispute_stake(stake)` - Check dispute stake meets minimum
- `validate_dispute_stake_with_threshold(stake, threshold)` - Check stake vs threshold

**VotingUtils**
- `transfer_stake(env, user, stake)` - Transfer stake to contract
- `transfer_winnings(env, user, amount)` - Send winnings to user
- `transfer_fees(env, admin, amount)` - Send collected fees to admin
- `calculate_user_payout(env, user, market_id, winning_outcome)` - Calculate user winnings
- `calculate_fee_amount(market)` - Calculate fee for market
- `get_voting_stats(market)` - Get market voting statistics
- `has_user_voted(market, user)` - Check if user participated
- `get_user_vote(market, user)` - Get user's vote details
- `has_user_claimed(market, user)` - Check if user claimed winnings

**VotingAnalytics**
- `calculate_participation_rate(market)` - Get voting participation percentage
- `calculate_average_stake(market)` - Get average vote stake
- `calculate_stake_distribution(market)` - Get per-outcome stake breakdown
- `calculate_voting_power_concentration(market)` - Measure stakeholder concentration
- `get_top_voters(market, limit)` - Get largest stakeholders

---

## Resolution Management

**Module**: `resolution.rs`

**Purpose**: Market resolution through oracles, manual resolution, and comprehensive lifecycle management.

### Core Structures

- `ResolutionState` (enum) - Active, OracleResolved, MarketResolved, Disputed, Finalized
- `OracleResolution` (struct) - Oracle result with confidence and timestamp
- `MarketResolution` (struct) - Final market outcome with determination method
- `ResolutionMethod` (enum) - Oracle, Community, AdminManual, Fallback

### Primary Functions

**OracleResolutionManager**
- `fetch_oracle_result(env, market_id)` - Get oracle's outcome data
- `get_oracle_resolution(env, market_id)` - Retrieve stored oracle resolution
- `validate_oracle_resolution(env, market, oracle_config)` - Verify oracle result
- `calculate_oracle_confidence(resolution)` - Calculate confidence score

**MarketResolutionManager**
- `resolve_market(env, market_id)` - Perform market resolution
- `finalize_market(env, market_id, winning_outcome)` - Finalize resolution
- `get_market_resolution(env, market_id)` - Get final resolution data
- `validate_market_resolution(env, market)` - Verify resolution validity

**ResolutionValidators**
- `OracleResolutionValidator::validate_market_for_oracle_resolution(env, market)` - Check oracle resolution readiness
- `OracleResolutionValidator::validate_oracle_resolution(env, market, result)` - Validate oracle data
- `MarketResolutionValidator::validate_market_for_resolution(env, market)` - Check market resolution readiness
- `MarketResolutionValidator::validate_admin_permissions(env, admin)` - Check admin rights
- `MarketResolutionValidator::validate_outcome(env, market, outcome)` - Validate outcome
- `MarketResolutionValidator::validate_market_resolution(env, market, resolution)` - Validate complete resolution

**ResolutionAnalytics**
- `OracleResolutionAnalytics::calculate_confidence_score(resolution)` - Score oracle confidence
- `OracleResolutionAnalytics::get_oracle_stats(env)` - Get oracle statistics
- `MarketResolutionAnalytics::determine_resolution_method(env, market)` - Determine resolution type
- `MarketResolutionAnalytics::calculate_confidence_score(resolution)` - Score market confidence
- `MarketResolutionAnalytics::calculate_resolution_analytics(env)` - Get resolution statistics
- `MarketResolutionAnalytics::update_resolution_analytics(env, market_id)` - Update resolution stats

**ResolutionUtils**
- `get_resolution_state(env, market)` - Get current resolution phase
- `can_resolve_market(env, market)` - Check if market can be resolved
- `get_resolution_eligibility(env, market)` - Get resolution readiness status
- `calculate_resolution_time(env, market)` - Get expected resolution timestamp
- `validate_resolution_parameters(env, market, outcome)` - Validate resolution inputs

---

## Oracle Management

**Module**: `oracles.rs`

**Purpose**: Oracle integration supporting Reflector, Pyth, and Band Protocol with health monitoring and fallback support.

### Core Structures

- `OracleProvider` (enum) - Reflector, Pyth, BandProtocol, Chainlink
- `OracleConfig` (struct) - Provider, feed_id, threshold, comparison operator
- `OracleResult` (struct) - Price, confidence, timestamp
- `OracleInstance` (enum) - Reflector or Pyth oracle instance

### Reflector Oracle

**ReflectorOracle**
- `new(contract_id)` - Create Reflector oracle instance
- `contract_id()` - Get oracle contract address
- `parse_feed_id(env, feed_id)` - Parse Reflector asset identifier
- `get_reflector_price(env, feed_id)` - Get current price for asset
- `check_health(env)` - Verify oracle operational status

**ReflectorOracleClient**
- `new(env, contract_id)` - Initialize client
- `lastprice(asset)` - Get latest price for asset
- `price(asset, timestamp)` - Get historical price at timestamp
- `twap(asset, records)` - Get time-weighted average price
- `is_healthy()` - Check oracle health status

### Pyth Oracle

**PythOracle**
- `new(contract_id)` - Create Pyth oracle instance
- `with_feeds(contract_id, feed_configs)` - Create with multiple feeds
- `add_feed_config(feed_config)` - Register new feed
- `get_feed_config(feed_id)` - Retrieve feed configuration
- `validate_feed_id(feed_id)` - Check feed is supported
- `get_supported_assets()` - List available assets
- `is_feed_active(feed_id)` - Check feed operational status
- `get_feed_count()` - Get number of feeds
- `scale_price(raw_price, feed_config)` - Apply decimal scaling
- `get_price_with_retry(env, feed_id, retries)` - Get price with retry logic

### Oracle Factory & Management

**OracleFactory**
- `create_pyth_oracle(contract_id)` - Create Pyth oracle
- `create_reflector_oracle(contract_id)` - Create Reflector oracle
- `create_oracle(provider, contract_id)` - Create provider-specific oracle
- `create_from_config(env, oracle_config)` - Create from config
- `is_provider_supported(provider)` - Check if provider supported
- `get_recommended_provider()` - Get default provider
- `create_pyth_oracle_with_feeds(contract_id, feeds)` - Create Pyth with feeds
- `create_hybrid_oracle(primary, fallback)` - Create dual-source oracle
- `get_default_feed_configs()` - Get standard feed list
- `validate_stellar_compatibility(config)` - Check Stellar compatibility

**OracleInstance Methods**
- `get_price(env, feed_id)` - Get asset price
- `get_price_data(env, feed_id)` - Get price with metadata
- `provider()` - Get oracle provider type
- `contract_id()` - Get provider contract address
- `is_healthy(env)` - Check oracle health

### Oracle Utilities

**OracleUtils**
- `compare_prices(price, threshold, comparison)` - Apply price comparison
- `determine_outcome(price, threshold, comparison, outcomes)` - Map price to outcome
- `validate_oracle_response(price)` - Validate price data format

### Oracle Whitelist & Validation

**OracleWhitelist**
- `initialize(env, admin)` - Initialize whitelist system
- `add_admin(env, current_admin, new_admin)` - Add whitelist admin
- `remove_admin(env, current_admin, admin_to_remove)` - Remove admin
- `require_admin(env, address)` - Check admin status
- `is_admin(env, address)` - Verify admin
- `add_oracle_to_whitelist(env, admin, oracle_address, metadata)` - Approve oracle
- `remove_oracle_from_whitelist(env, admin, oracle_address)` - Revoke oracle
- `validate_oracle_contract(env, oracle_address)` - Validate oracle contract
- `verify_oracle_health(env, oracle_address)` - Check oracle operational
- `get_approved_oracles(env)` - List approved oracles
- `get_oracle_metadata(env, oracle_address)` - Get oracle details
- `deactivate_oracle(env, admin, oracle_address)` - Disable oracle
- `reactivate_oracle(env, admin, oracle_address)` - Re-enable oracle

**OracleValidationConfigManager**
- `get_global_config(env)` - Get platform validation config
- `set_global_config(env, admin, config)` - Update platform config
- `get_event_config(env, market_id)` - Get market-specific config
- `set_event_config(env, admin, market_id, config)` - Set market config
- `get_effective_config(env, market_id)` - Get applicable config
- `validate_oracle_data(env, market_id, data)` - Validate oracle data

### Oracle Integration Manager

**OracleIntegrationManager**
- `verify_result(env, market_id, oracle_result)` - Verify oracle result
- `is_result_verified(env, market_id)` - Check verification status
- `get_oracle_result(env, market_id)` - Retrieve verified result
- `verify_result_with_retry(env, market_id, retries)` - Verify with retries
- `verify_oracle_authority(env, oracle_address)` - Verify oracle legitimacy
- `admin_override_result(env, admin, market_id, result)` - Manual override

---

## Dispute Management

**Module**: `disputes.rs`

**Purpose**: Comprehensive dispute system with voting, escalation, timeout handling, and fee distribution.

### Core Structures

- `Dispute` (struct) - Formal challenge with user, stake, status
- `DisputeStatus` (enum) - Active, Voting, Resolved, Escalated
- `DisputeStats` (struct) - Dispute statistics and participation metrics
- `DisputeResolution` (struct) - Dispute outcome and resolution details
- `DisputeVote` (struct) - Individual vote on dispute
- `DisputeTimeout` (struct) - Timeout period for dispute resolution
- `DisputeTimeoutStatus` (enum) - Pending, Expired, AutoResolved, Extended

### Primary Functions

**DisputeManager**
- `process_dispute(env, user, market_id, stake, reason)` - Initiate dispute
- `resolve_dispute(env, market_id, admin, outcome)` - Finalize dispute
- `get_dispute_stats(env, market_id)` - Get dispute statistics
- `get_market_disputes(env, market_id)` - Get all market disputes
- `has_user_disputed(env, market_id, user)` - Check user dispute status
- `get_user_dispute_stake(env, market_id, user)` - Get user dispute amount
- `vote_on_dispute(env, dispute_id, user, vote, stake)` - Vote on dispute
- `calculate_dispute_outcome(env, dispute_id)` - Calculate dispute result
- `distribute_dispute_fees(env, dispute_id, winner)` - Distribute outcome fees
- `escalate_dispute(env, dispute_id, escalation_reason)` - Escalate to higher review
- `get_dispute_votes(env, dispute_id)` - Get all dispute votes
- `validate_dispute_resolution_conditions(env, market_id)` - Check resolution readiness
- `set_dispute_timeout(env, dispute_id, timeout_hours)` - Set resolution deadline
- `check_dispute_timeout(env, dispute_id)` - Check timeout expiration
- `auto_resolve_dispute_on_timeout(env, dispute_id)` - Auto-resolve expired dispute
- `determine_timeout_outcome(env, dispute_id)` - Determine timeout result
- `emit_timeout_event(env, dispute_id, outcome)` - Emit timeout event
- `get_dispute_timeout_status(env, dispute_id)` - Get timeout status
- `extend_dispute_timeout(env, dispute_id, additional_hours)` - Extend deadline

**DisputeValidator**
- `validate_market_for_dispute(env, market)` - Check dispute eligibility
- `validate_market_for_resolution(env, market)` - Check resolution readiness
- `validate_admin_permissions(env, admin)` - Verify admin rights
- `validate_dispute_parameters(env, market, stake)` - Validate dispute inputs
- `validate_resolution_parameters(env, outcome)` - Validate resolution outcome
- `validate_dispute_voting_conditions(env, market)` - Check voting conditions
- `validate_user_hasnt_voted(env, dispute_id, user)` - Prevent double voting
- `validate_voting_completed(voting_data)` - Check voting phase end
- `validate_dispute_resolution_conditions(env, market)` - Check resolution readiness
- `validate_dispute_escalation_conditions(env, dispute)` - Check escalation readiness
- `validate_dispute_timeout_parameters(timeout_hours)` - Validate timeout duration
- `validate_dispute_timeout_extension_parameters(additional, current_timeout)` - Validate extension
- `validate_dispute_timeout_status_for_extension(timeout_status)` - Check can extend

**DisputeUtils**
- `add_dispute_to_market(market, dispute)` - Register dispute on market
- `extend_market_for_dispute(market, env)` - Extend market for dispute period
- `determine_final_outcome_with_disputes(market, disputes)` - Calculate final outcome
- `finalize_market_with_resolution(market, resolution)` - Apply resolution
- `extract_disputes_from_market(market)` - Get market disputes
- `has_user_disputed(market, user)` - Check user disputed
- `get_user_dispute_stake(market, user)` - Get user's dispute stake
- `calculate_dispute_impact(market)` - Measure dispute effect
- `add_vote_to_dispute(dispute_id, vote)` - Record dispute vote
- `get_dispute_voting(env, dispute_id)` - Get voting data
- `store_dispute_voting(env, dispute_id, voting)` - Save voting data
- `store_dispute_vote(env, dispute_id, vote)` - Record vote
- `get_dispute_votes(env, dispute_id)` - Retrieve votes
- `calculate_stake_weighted_outcome(voting_data)` - Get outcome by stake
- `distribute_fees_based_on_outcome(dispute_id, outcome)` - Distribute fees
- `store_dispute_fee_distribution(env, distribution)` - Save distribution
- `get_dispute_fee_distribution(env, dispute_id)` - Get distribution details
- `store_dispute_escalation(env, escalation)` - Save escalation
- `get_dispute_escalation(env, dispute_id)` - Get escalation details
- `store_dispute_timeout(env, timeout)` - Save timeout config
- `get_dispute_timeout(env, dispute_id)` - Get timeout config
- `has_dispute_timeout(env, dispute_id)` - Check timeout exists
- `remove_dispute_timeout(env, dispute_id)` - Remove timeout
- `get_active_timeouts(env)` - Get all pending timeouts
- `check_expired_timeouts(env)` - Find expired timeouts

**DisputeAnalytics**
- `calculate_dispute_stats(market)` - Calculate dispute statistics
- `calculate_dispute_impact(market)` - Measure impact on market
- `calculate_oracle_weight(market)` - Get oracle influence
- `calculate_community_weight(market)` - Get community influence
- `calculate_community_consensus(env, market)` - Get consensus metrics
- `get_top_disputers(env, market, limit)` - Get largest dispute stakers
- `calculate_dispute_participation_rate(market)` - Get participation percentage
- `calculate_timeout_stats(env)` - Get timeout statistics
- `get_timeout_analytics(env, dispute_id)` - Get timeout details

---

## Event System

**Module**: `events.rs`

**Purpose**: Comprehensive event emission for all contract operations enabling transparency and off-chain tracking.

### Event Types Emitted

- `MarketCreatedEvent` - New market creation
- `EventCreatedEvent` - Event structure creation
- `VoteCastEvent` - User vote submission
- `BetPlacedEvent` - Bet placement
- `BetStatusUpdatedEvent` - Bet outcome change
- `OracleResultEvent` - Oracle result received
- `MarketResolvedEvent` - Market resolution completion
- `DisputeCreatedEvent` - Dispute initiation
- `DisputeResolvedEvent` - Dispute conclusion
- `FeeCollectedEvent` - Fee collection from market
- `FeeWithdrawalAttemptEvent` - Fee withdrawal request
- `FeeWithdrawnEvent` - Fee withdrawal completion
- `OracleVerifInitiatedEvent` - Oracle verification start
- `OracleResultVerifiedEvent` - Oracle result verification complete
- `OracleVerificationFailedEvent` - Oracle verification failure
- `OracleValidationFailedEvent` - Oracle validation failure
- `OracleConsensusReachedEvent` - Oracle consensus achieved
- `OracleHealthStatusEvent` - Oracle health report
- `ExtensionRequestedEvent` - Market extension request
- `ConfigUpdatedEvent` - Configuration change
- `BetLimitsUpdatedEvent` - Bet limit change
- `StatisticsUpdatedEvent` - Statistics update
- `ErrorLoggedEvent` - Error occurrence
- `ErrorRecoveryEvent` - Error resolution
- `PerformanceMetricEvent` - Performance data
- `AdminActionEvent` - Admin action execution
- `AdminRoleEvent` - Admin role change
- `AdminPermissionEvent` - Permission modification
- `MarketClosedEvent` - Market closure
- `RefundOnOracleFailureEvent` - Refund for failed oracle
- `MarketFinalizedEvent` - Market finalization
- `AdminInitializedEvent` - Admin system initialization
- `AdminTransferredEvent` - Admin transfer
- `ContractPausedEvent` - Contract pause
- `ContractUnpausedEvent` - Contract resume
- `ContractInitializedEvent` - Contract initialization
- `PlatformFeeSetEvent` - Platform fee setting
- `DisputeTimeoutSetEvent` - Dispute timeout creation
- `DisputeTimeoutExpiredEvent` - Dispute timeout expiration
- `DisputeTimeoutExtendedEvent` - Dispute timeout extension
- `DisputeAutoResolvedEvent` - Dispute auto-resolution
- `GovernanceProposalCreatedEvent` - Governance proposal creation
- `GovernanceVoteCastEvent` - Governance vote
- `FallbackUsedEvent` - Fallback oracle usage
- `ResolutionTimeoutEvent` - Resolution deadline reached
- `GovernanceProposalExecutedEvent` - Proposal execution
- `ConfigInitializedEvent` - Configuration initialization
- `StorageCleanupEvent` - Storage cleanup
- `StorageOptimizationEvent` - Storage optimization
- `StorageMigrationEvent` - Storage migration
- `OracleDegradationEvent` - Oracle degradation
- `OracleRecoveryEvent` - Oracle recovery
- `ManualResolutionRequiredEvent` - Manual intervention needed
- `StateChangeEvent` - State transition
- `WinningsClaimedEvent` - Single claim
- `WinningsClaimedBatchEvent` - Batch claim
- `ClaimPeriodUpdatedEvent` - Claim period change
- `MarketClaimPeriodUpdatedEvent` - Market-specific claim period
- `TreasuryUpdatedEvent` - Treasury changes
- `UnclaimedWinningsSweptEvent` - Unclaimed funds sweep
- `ContractUpgradedEvent` - Contract upgrade
- `MarketDeadlineExtendedEvent` - Market deadline extension
- `MarketDescriptionUpdatedEvent` - Market description change
- `MarketOutcomesUpdatedEvent` - Market outcomes change
- `CategoryUpdatedEvent` - Market category change
- `TagsUpdatedEvent` - Market tags change
- `ContractRollbackEvent` - Contract rollback
- `UpgradeProposalCreatedEvent` - Upgrade proposal creation
- `CircuitBreakerEvent` - Circuit breaker trigger
- `MinPoolSizeNotMetEvent` - Minimum pool not met

### Core Functions

**EventEmitter**
- `emit_market_created(env, market_id, admin, question, outcomes)` - Emit market creation
- `emit_event_created(...)` - Emit event structure creation
- `emit_vote_cast(env, user, market_id, outcome, stake)` - Emit vote
- `emit_bet_placed(env, user, market_id, outcome, amount)` - Emit bet
- `emit_bet_status_updated(env, user, market_id, status)` - Emit bet update
- `emit_oracle_result(env, market_id, oracle_result)` - Emit oracle data
- `emit_oracle_verification_initiated(env, market_id, oracle_provider)` - Emit verification start
- `emit_oracle_result_verified(env, market_id, oracle_provider: confidence)` - Emit verification success
- `emit_oracle_verification_failed(env, market_id, reason)` - Emit verification failure
- `emit_oracle_validation_failed(env, market_id, validation_error)` - Emit validation failure
- `emit_oracle_consensus_reached(env)` - Emit consensus
- `emit_oracle_health_status(env, provider, is_healthy)` - Emit health status
- `emit_market_resolved(env, market_id, winning_outcome)` - Emit resolution
- `emit_dispute_created(env, market_id, user, stake)` - Emit dispute creation
- `emit_dispute_resolved(env, dispute_id, outcome)` - Emit dispute resolution
- `emit_fee_collected(env, admin, market_id, amount)` - Emit fee collection
- `emit_fee_withdrawal_attempt(env, admin, amount)` - Emit withdrawal attempt
- `emit_fee_withdrawn(env, admin, amount)` - Emit withdrawal completion
- `emit_extension_requested(env, market_id, additional_days)` - Emit extension request
- `emit_config_updated(env, config_type)` - Emit config change
- `emit_bet_limits_updated(env, min_bet, max_bet)` - Emit limit change
- `emit_error_logged(env, error_code, message)` - Emit error
- `emit_error_recovery_event(env, recovery_action)` - Emit recovery
- `emit_performance_metric(env, metric_type, value)` - Emit metric
- `emit_admin_action_logged(env, admin, action, success)` - Emit admin action

---

## Governance

**Module**: `governance.rs`

**Purpose**: On-chain governance system for protocol-level decisions and contract upgrades.

### Core Structures

- `GovernanceProposal` (struct) - Proposal with voting and execution details
- `GovernanceError` (enum) - ProposalExists, ProposalNotFound, VotingNotStarted, etc.

### Primary Functions

**GovernanceContract**
- `initialize(env, admin, voting_period_seconds, quorum_votes)` - Initialize governance
- `create_proposal(env, proposer, proposal_data)` - Create new proposal
- `vote(env, voter, proposal_id, vote_for)` - Vote on proposal (for/against)
- `validate_proposal(env, proposal)` - Validate proposal data
- `execute_proposal(env, proposal_id, caller)` - Execute passed proposal
- `list_proposals(env)` - Get all proposal IDs
- `get_proposal(env, proposal_id)` - Get proposal details
- `set_voting_period(env, admin, new_period_seconds)` - Change voting duration
- `set_quorum(env, admin, new_quorum)` - Change quorum requirement

---

## Configuration

**Module**: `config.rs`

**Purpose**: Centralized contract configuration management with environment profiles and runtime updates.

### Core Structures

- `ContractConfig` (struct) - Complete contract configuration
- `Environment` (enum) - Development, Testnet, Mainnet
- `FeeConfig` (struct) - Fee parameters
- `VotingConfig` (struct) - Voting parameters
- `MarketConfig` (struct) - Market constraints
- `ExtensionConfig` (struct) - Extension rules
- `ResolutionConfig` (struct) - Resolution parameters
- `OracleRuntimeConfig` (struct) - Oracle settings

### Configuration Management

**ConfigManager**
- `get_development_config(env)` - Get dev environment config
- `get_testnet_config(env)` - Get testnet environment config
- `get_mainnet_config(env)` - Get mainnet environment config
- `get_default_fee_config()` - Get default fee settings
- `get_mainnet_fee_config()` - Get mainnet fee settings
- `get_default_voting_config()` - Get default voting settings
- `get_mainnet_voting_config()` - Get mainnet voting settings
- `get_default_market_config()` - Get default market settings
- `get_default_extension_config()` - Get default extension settings
- `get_default_resolution_config()` - Get default resolution settings
- `get_default_oracle_config()` - Get default oracle settings
- `get_mainnet_oracle_config()` - Get mainnet oracle settings
- `store_config(env, config)` - Save configuration
- `get_config(env)` - Retrieve configuration
- `update_config(env, config)` - Update configuration
- `reset_to_defaults(env)` - Reset to default values
- `get_current_configuration(env)` - Get active configuration
- `get_configuration_history(env, limit)` - Get config change history
- `validate_configuration_changes(env, changes)` - Validate changes
- `update_fee_percentage(env, admin, percentage)` - Change fee percentage
- `update_dispute_threshold(env, admin, threshold)` - Change dispute cost
- `update_oracle_timeout(env, admin, timeout)` - Change oracle timeout
- `update_market_limits(env, admin, limits)` - Change market constraints

**ConfigValidator**
- `validate_contract_config(config)` - Validate entire config
- `validate_fee_config(config)` - Validate fee settings
- `validate_voting_config(config)` - Validate voting settings
- `validate_market_config(config)` - Validate market settings
- `validate_extension_config(config)` - Validate extension settings
- `validate_resolution_config(config)` - Validate resolution settings
- `validate_oracle_config(config)` - Validate oracle settings

**ConfigUtils**
- `is_mainnet(config)` - Check if mainnet
- `is_testnet(config)` - Check if testnet
- `is_development(config)` - Check if development
- `get_environment_name(config)` - Get environment name string
- `get_config_summary(config)` - Get config summary
- `fees_enabled(config)` - Check if fees active
- `get_fee_config(config)` - Get fee settings
- `get_voting_config(config)` - Get voting settings
- `get_market_config(config)` - Get market settings
- `get_extension_config(config)` - Get extension settings
- `get_resolution_config(config)` - Get resolution settings
- `get_oracle_config(config)` - Get oracle settings

---

## Fee Management

**Module**: `fees.rs`

**Purpose**: Fee calculation, collection, distribution, and analytics with tiered and dynamic pricing.

### Core Structures

- `FeeConfig` (struct) - Platform fee configuration
- `FeeTier` (struct) - Fee tier for market size
- `ActivityAdjustment` (struct) - Activity-based fee adjustment
- `FeeCalculationFactors` (struct) - All fee calculation inputs
- `FeeHistory` (struct) - Historical fee data
- `FeeCollection` (struct) - Fee collection record
- `FeeAnalytics` (struct) - Fee statistics
- `FeeBreakdown` (struct) - Fee component details
- `FeeWithdrawalSchedule` (struct) - Fee withdrawal schedule

### Fee Operations

**FeeManager**
- `collect_fees(env, admin, market_id)` - Collect market fees
- `process_creation_fee(env, admin)` - Process new market fee
- `get_fee_analytics(env)` - Get fee statistics
- `update_fee_config(env, admin, new_config)` - Change fee settings
- `get_fee_config(env)` - Get current fee config
- `validate_market_fees(env, market)` - Validate market fees
- `update_fee_structure(env, admin, structure)` - Update fee structure
- `get_fee_history(env, market_id)` - Get historical fees

**FeeCalculator**
- `calculate_platform_fee(market)` - Calculate fee amount
- `calculate_user_payout_after_fees(market, user, stake)` - Get payout minus fees
- `calculate_fee_breakdown(market)` - Get detailed fee breakdown
- `calculate_dynamic_fee(market)` - Calculate variable fee
- `calculate_dynamic_fee_by_market_id(env, market_id)` - Calculate dynamic fee
- `get_fee_tier_by_market_size(env, total_staked)` - Get applicable tier
- `adjust_fee_by_activity(market, adjustment)` - Apply activity adjustment
- `validate_fee_percentage(env, fee, market_id)` - Validate fee percentage
- `get_fee_calculation_factors(env, market_id)` - Get calculation inputs

**FeeValidator**
- `validate_admin_permissions(env, admin)` - Verify admin rights
- `validate_market_for_fee_collection(market)` - Check fee collection eligibility
- `validate_fee_amount(amount)` - Validate fee amount
- `validate_creation_fee(amount)` - Validate creation fee
- `validate_fee_config(config)` - Validate configuration
- `validate_market_fees(market)` - Validate market fees

**FeeUtils**
- `transfer_fees_to_admin(env, admin, amount)` - Send fees to admin
- `get_market_fee_stats(market)` - Get market fee statistics
- `can_collect_fees(market)` - Check if fees collectible
- `get_fee_eligibility(market)` - Get collection readiness

**FeeTracker**
- `record_fee_collection(env, market_id, amount, admin)` - Record collection
- `record_creation_fee(env, admin, amount)` - Record creation fee
- `record_config_change(env, old_config, new_config)` - Record config change
- `get_fee_history(env)` - Get all fee records
- `get_total_fees_collected(env)` - Get total collected
- `record_fee_structure_update(env, old_structure, new_structure)` - Record update

**FeeWithdrawalManager**
- `get_schedule(env)` - Get withdrawal schedule
- `set_schedule(env, admin, schedule)` - Set withdrawal schedule
- `get_last_withdrawal_ts(env)` - Get last withdrawal time
- `withdraw_fees(env, admin, amount)` - Withdraw collected fees

**FeeConfigManager**
- `store_fee_config(env, config)` - Save fee config
- `get_fee_config(env)` - Get fee config
- `reset_to_defaults(env)` - Reset to defaults
- `calculate_analytics(env)` - Calculate fee analytics
- `get_market_fee_stats(market)` - Get market fee stats
- `calculate_fee_efficiency(market)` - Get fee efficiency metrics

---

## Market Extensions

**Module**: `extensions.rs`

**Purpose**: Market duration extension management with fee handling and history tracking.

### Core Structures

- `ExtensionEvent` (struct) - Extension history entry

### Primary Functions

**ExtensionManager**
- `extend_market_duration(env, admin, market_id, additional_days, reason)` - Extend market end time
- `get_market_extension_history(env, market_id)` - Get all extensions for market
- `get_extension_stats(env, market_id)` - Get extension statistics
- `can_extend_market(env, market_id, admin)` - Check if extension allowed
- `calculate_extension_fee(additional_days)` - Calculate extension cost

**ExtensionValidator**
- `validate_extension_conditions(env, market_id, additional_days)` - Validate extension
- `check_extension_limits(env, market_id)` - Check limit constraints
- `can_extend_market(env, market_id, admin)` - Verify extensibility

**ExtensionUtils**
- `handle_extension_fees(env, admin, fee_amount)` - Process extension fees
- `emit_extension_event(env, market_id, admin, additional_days)` - Emit extension event
- `get_extension_events(env)` - Get all extension events

---

## Monitoring System

**Module**: `monitoring.rs`

**Purpose**: Comprehensive contract health monitoring, alerting, and performance metrics.

### Core Structures

- `MonitoringAlertType` (enum) - MarketHealth, OracleHealth, FeeCollection, DisputeResolution, Performance, Security
- `AlertSeverity` (enum) - Info, Warning, Critical, Emergency
- `MonitoringStatus` (enum) - Healthy, Warning, Critical, Unknown, Maintenance
- `TimeFrame` (enum) - LastHour, LastDay, LastWeek, LastMonth, Custom
- `MarketHealthMetrics` (struct) - Market health indicators
- `OracleHealthMetrics` (struct) - Oracle health indicators
- `FeeCollectionMetrics` (struct) - Fee collection statistics
- `DisputeResolutionMetrics` (struct) - Dispute metrics
- `PerformanceMetrics` (struct) - System performance data
- `MonitoringAlert` (struct) - Alert notification
- `MonitoringData` (struct) - Monitoring data snapshot

### Primary Functions

**ContractMonitor**
- `monitor_market_health(env, market_id)` - Check market health
- `monitor_oracle_health(env, oracle_provider)` - Check oracle health
- `monitor_fee_collection(env, market_id)` - Check fee collection status
- `monitor_dispute_resolution(env, market_id)` - Check dispute status
- `get_contract_performance_metrics(env)` - Get system performance
- `emit_monitoring_alert(env, alert)` - Emit health alert
- `validate_monitoring_data(env, data)` - Validate monitoring data

**MonitoringUtils**
- `create_alert(alert_type, severity, message)` - Create alert
- `is_data_stale(env, timestamp, max_age)` - Check data freshness
- `calculate_freshness_score(env, timestamp)` - Score data age
- `validate_thresholds(env, metrics, thresholds)` - Validate against thresholds

**MonitoringTestingUtils**
- `create_test_market_health_metrics(env, market_id)` - Create test market metrics
- `create_test_oracle_health_metrics(env, provider)` - Create test oracle metrics
- `create_test_fee_collection_metrics(env)` - Create test fee metrics
- `create_test_dispute_resolution_metrics(env, market_id)` - Create test dispute metrics
- `create_test_performance_metrics(env)` - Create test performance metrics
- `create_test_monitoring_alert(env)` - Create test alert
- `create_test_monitoring_data(env)` - Create test data
- `validate_test_data_structure(env, data)` - Validate test data

---

## Module Interaction Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                    Admin Management (admin.rs)                   │
│         - Roles & Permissions - Action Logging - Pause/Unpause   │
└────────────────────────────┬──────────────────────────────────────┘
                             │
        ┌────────────────────┼────────────────────┐
        │                    │                    │
        ▼                    ▼                    ▼
   ┌─────────────┐  ┌──────────────────┐  ┌──────────────────┐
   │Governance   │  │Market Management │  │ Configuration    │
   │(governance) │  │ (markets.rs)     │  │ (config.rs)      │
   └─────────────┘  └────────┬─────────┘  └──────────────────┘
                             │
        ┌────────────────────┼────────────────────┬────────────┐
        │                    │                    │            │
        ▼                    ▼                    ▼            ▼
   ┌────────┐    ┌────────────────┐   ┌──────────────┐  ┌──────────┐
   │  Bets  │    │  Voting &      │   │   Dispute    │  │ Extension│
   │(bets)  │    │  Disputes      │   │ (disputes)   │  │(extensions)
   │        │    │(voting)        │   │              │  │          │
   └────┬───┘    └────┬───────┬───┘   └──────┬───────┘  └──────────┘
        │             │       │              │
        └─────────────┼───────┼──────────────┘
                      │       │
                      ▼       ▼
              ┌──────────────────────┐
              │ Resolution System    │
              │ (resolution.rs)      │
              └──────┬───────────────┘
                     │
                     ▼
          ┌─────────────────────┐
          │ Oracle Management   │
          │ (oracles.rs)        │
          └─────────────────────┘
                     │
        ┌────────────┴────────────┐
        │                         │
        ▼                         ▼
   ┌──────────┐            ┌──────────────┐
   │  Fees    │            │  Balances    │
   │(fees)    │            │(balances)    │
   └──────────┘            └──────────────┘
        │                         │
        │       ┌─────────────────┘
        │       │
        └───┬───┴─────────────┐
            │                 │
            ▼                 ▼
       ┌──────────┐    ┌─────────────┐
       │  Queries │    │Monitoring   │
       │(queries) │    │(monitoring) │
       └──────────┘    └─────────────┘
            │                 │
            └────────┬────────┘
                     │
                     ▼
            ┌─────────────────┐
            │  Event System   │
            │  (events.rs)    │
            └─────────────────┘
```

---

## Data Flow Example: Market Creation to Resolution

```
1. Admin creates market
   AdminFunctions::create_market() → MarketCreator::create_market()
   
2. EventEmitter::emit_market_created()

3. Users place bets
   BetManager::place_bet() → MarketStateManager::add_vote()
   EventEmitter::emit_bet_placed()

4. Market reaches end time
   OracleResolutionManager::fetch_oracle_result() → OracleFactory

5. Oracle result received
   EventEmitter::emit_oracle_result()

6. Market resolution
   MarketResolutionManager::resolve_market() → VotingUtils::calculate_user_payout()

7. Disputers can challenge
   DisputeManager::process_dispute() → DisputeUtils::add_dispute_to_market()

8. Final resolution
   MarketResolutionManager::finalize_market() → EventEmitter::emit_market_resolved()

9. Users claim winnings
   VotingManager::process_claim() → FeeCalculator::calculate_user_payout_after_fees()
   EventEmitter::emit_winnings_claimed()

10. Fees collected
    FeeManager::collect_fees() → EventEmitter::emit_fee_collected()
```

---

## Error Handling

All functions return `Result<T, Error>` with comprehensive error types covering:
- Invalid input validation
- Authorization failures
- Market state violations
- Insufficient balances
- Oracle failures
- Dispute process errors
- Configuration errors

---

## Gas Optimization Considerations

- Minimal storage reads/writes
- Batch operations for reduced transactions
- Efficient data structures
- Caching where appropriate
- Query functions are read-only

---

## Security Features

- Role-based access control
- Multi-admin support with multisig
- Reentrancy guards
- Circuit breaker system
- Comprehensive input validation
- State consistency checks
- Oracle health monitoring
- Dispute resolution system

---

## Version

This documentation covers the current implementation of Predictify Hybrid. Version information and change logs can be found in the contract configuration and governance modules.
