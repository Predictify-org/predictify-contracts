# Predictify Hybrid Voting System

## Overview

The Predictify Hybrid contract features a sophisticated voting and dispute resolution system that combines community consensus with oracle-based resolution. This document outlines the architecture, components, and implementation details of the voting system.

## Architecture

### Core Components

The voting system consists of several key components:

1. **Voting Structures** - Data structures for votes, statistics, and payouts
2. **Voting Manager** - Core voting operations and state management
3. **Dispute System** - Stake-based dispute resolution with dynamic thresholds
4. **Validation System** - Input validation and business rule enforcement
5. **Analytics System** - Voting statistics and market insights
6. **Utility Functions** - Helper functions for common operations

## Voting Structures

### Vote Structure

```rust
pub struct Vote {
    pub user: Address,
    pub outcome: String,
    pub stake: i128,
    pub timestamp: u64,
}
```

**Purpose**: Represents a user's vote on a prediction market outcome.

**Fields**:
- `user`: The voter's address
- `outcome`: The chosen outcome (e.g., "yes", "no")
- `stake`: Amount staked in the vote (in stroops)
- `timestamp`: When the vote was cast

### Voting Statistics

```rust
pub struct VotingStats {
    pub total_votes: u32,
    pub total_staked: i128,
    pub outcome_distribution: Map<String, i128>,
    pub unique_voters: u32,
}
```

**Purpose**: Provides comprehensive analytics about voting activity.

**Fields**:
- `total_votes`: Number of votes cast
- `total_staked`: Total amount staked across all votes
- `outcome_distribution`: Stake distribution by outcome
- `unique_voters`: Number of unique participants

### Payout Data

```rust
pub struct PayoutData {
    pub user_stake: i128,
    pub winning_total: i128,
    pub total_pool: i128,
    pub fee_percentage: i128,
    pub payout_amount: i128,
}
```

**Purpose**: Calculates user payouts based on voting results.

## Dispute System

### Dynamic Dispute Thresholds

The system implements sophisticated dispute thresholds that adjust based on market characteristics:

```rust
pub struct DisputeThreshold {
    pub market_id: Symbol,
    pub base_threshold: i128,
    pub adjusted_threshold: i128,
    pub market_size_factor: i128,
    pub activity_factor: i128,
    pub complexity_factor: i128,
    pub timestamp: u64,
}
```

### Threshold Adjustment Factors

```rust
pub struct ThresholdAdjustmentFactors {
    pub market_size_factor: i128,
    pub activity_factor: i128,
    pub complexity_factor: i128,
    pub total_adjustment: i128,
}
```

**Adjustment Logic**:
- **Market Size**: Larger markets require higher dispute thresholds
- **Activity Level**: High-activity markets may need lower thresholds
- **Complexity**: Complex markets (multiple outcomes) require higher thresholds

### Threshold History

```rust
pub struct ThresholdHistoryEntry {
    pub market_id: Symbol,
    pub old_threshold: i128,
    pub new_threshold: i128,
    pub adjustment_reason: String,
    pub adjusted_by: Address,
    pub timestamp: u64,
}
```

**Purpose**: Tracks all threshold adjustments for transparency and auditability.

## Voting Manager

### Core Operations

The `VotingManager` provides the main interface for voting operations:

#### Process Vote
```rust
pub fn process_vote(
    env: &Env,
    user: Address,
    market_id: Symbol,
    outcome: String,
    stake: i128,
) -> Result<(), Error>
```

**Functionality**:
- Validates user authentication
- Checks market state and voting eligibility
- Validates outcome and stake amount
- Records vote and updates market statistics
- Transfers stake from user to contract

#### Process Dispute
```rust
pub fn process_dispute(
    env: &Env,
    user: Address,
    market_id: Symbol,
    stake: i128,
) -> Result<(), Error>
```

**Functionality**:
- Validates dispute eligibility
- Checks dispute threshold requirements
- Records dispute stake
- Extends market resolution period
- Triggers dispute resolution process

#### Process Claim
```rust
pub fn process_claim(
    env: &Env,
    user: Address,
    market_id: Symbol,
) -> Result<i128, Error>
```

**Functionality**:
- Validates market resolution
- Calculates user payout
- Transfers winnings to user
- Marks payout as claimed
- Updates market statistics

## Validation System

### Voting Validator

The `VotingValidator` ensures all voting operations comply with business rules:

#### Authentication Validation
```rust
pub fn validate_user_authentication(user: &Address) -> Result<(), Error>
pub fn validate_admin_authentication(env: &Env, admin: &Address) -> Result<(), Error>
```

#### Market State Validation
```rust
pub fn validate_market_for_voting(env: &Env, market: &Market) -> Result<(), Error>
pub fn validate_market_for_dispute(env: &Env, market: &Market) -> Result<(), Error>
pub fn validate_market_for_claim(env: &Env, market: &Market, user: &Address) -> Result<(), Error>
```

#### Parameter Validation
```rust
pub fn validate_vote_parameters(
    env: &Env,
    outcome: &String,
    valid_outcomes: &Vec<String>,
    stake: i128,
) -> Result<(), Error>
```

### Threshold Validator

The `ThresholdValidator` manages dispute threshold rules:

```rust
pub fn validate_threshold_limits(threshold: i128) -> Result<(), Error>
pub fn validate_threshold_adjustment_permissions(env: &Env, admin: &Address) -> Result<(), Error>
```

## Analytics System

### Voting Analytics

The `VotingAnalytics` provides insights into voting patterns:

#### Participation Metrics
```rust
pub fn calculate_participation_rate(market: &Market) -> f64
pub fn calculate_average_stake(market: &Market) -> i128
```

#### Distribution Analysis
```rust
pub fn calculate_stake_distribution(market: &Market) -> Map<String, i128>
pub fn calculate_voting_power_concentration(market: &Market) -> f64
```

#### Top Voters
```rust
pub fn get_top_voters(market: &Market, limit: usize) -> Vec<(Address, i128)>
```

## Utility Functions

### Transfer Operations

```rust
pub fn transfer_stake(env: &Env, user: &Address, stake: i128) -> Result<(), Error>
pub fn transfer_winnings(env: &Env, user: &Address, amount: i128) -> Result<(), Error>
pub fn transfer_fees(env: &Env, admin: &Address, amount: i128) -> Result<(), Error>
```

### Calculation Functions

```rust
pub fn calculate_user_payout(env: &Env, market: &Market, user: &Address) -> Result<i128, Error>
pub fn calculate_fee_amount(market: &Market) -> Result<i128, Error>
```

### Query Functions

```rust
pub fn has_user_voted(market: &Market, user: &Address) -> bool
pub fn get_user_vote(market: &Market, user: &Address) -> Option<(String, i128)>
pub fn has_user_claimed(market: &Market, user: &Address) -> bool
```

## Constants and Configuration

### Voting Constants

```rust
/// Minimum stake amount for voting (0.1 XLM)
pub const MIN_VOTE_STAKE: i128 = 100_000;

/// Minimum stake amount for disputes (10 XLM)
pub const MIN_DISPUTE_STAKE: i128 = 10_000_000;

/// Maximum dispute threshold (100 XLM)
pub const MAX_DISPUTE_THRESHOLD: i128 = 100_000_000;

/// Base dispute threshold (10 XLM)
pub const BASE_DISPUTE_THRESHOLD: i128 = 10_000_000;

/// Market size threshold for large markets (1000 XLM)
pub const LARGE_MARKET_THRESHOLD: i128 = 1_000_000_000;

/// Activity level threshold for high activity (100 votes)
pub const HIGH_ACTIVITY_THRESHOLD: u32 = 100;

/// Platform fee percentage (2%)
pub const FEE_PERCENTAGE: i128 = 2;

/// Dispute extension period in hours
pub const DISPUTE_EXTENSION_HOURS: u32 = 24;
```

## Usage Examples

### Creating a Vote

```rust
use predictify_hybrid::voting::{Vote, VotingManager};

// User votes "yes" with 0.5 XLM stake
VotingManager::process_vote(
    &env,
    user_address,
    market_id,
    String::from_str(&env, "yes"),
    500_000, // 0.5 XLM
)?;
```

### Initiating a Dispute

```rust
// User disputes market resolution with 15 XLM
VotingManager::process_dispute(
    &env,
    user_address,
    market_id,
    15_000_000, // 15 XLM
)?;
```

### Claiming Winnings

```rust
// User claims their winnings
let payout = VotingManager::process_claim(
    &env,
    user_address,
    market_id,
)?;
```

### Getting Voting Statistics

```rust
use predictify_hybrid::voting::VotingUtils;

let stats = VotingUtils::get_voting_stats(&market);
println!("Total votes: {}", stats.total_votes);
println!("Total staked: {} stroops", stats.total_staked);
```

## Integration Points

### Market System Integration

The voting system integrates with the market system through:
- Market state validation
- Outcome validation
- Stake management
- Resolution coordination

### Oracle System Integration

Voting results combine with oracle data for hybrid resolution:
- 70% oracle weight
- 30% community consensus weight
- Dispute resolution when oracle and community disagree

### Fee System Integration

The voting system manages platform fees:
- 2% platform fee on all stakes
- Fee collection after market resolution
- Fee distribution to platform admin

## Testing

The voting system includes comprehensive testing utilities:

```rust
pub mod testing {
    pub fn create_test_vote(env: &Env, user: Address, outcome: String, stake: i128) -> Vote
    pub fn create_test_voting_stats(env: &Env) -> VotingStats
    pub fn create_test_payout_data() -> PayoutData
    pub fn validate_vote_structure(vote: &Vote) -> Result<(), Error>
    pub fn validate_voting_stats(stats: &VotingStats) -> Result<(), Error>
}
```

## Error Handling

The voting system includes comprehensive error handling for:
- Invalid market states
- Insufficient stakes
- Authentication failures
- Threshold violations
- Duplicate operations
- Invalid parameters

## Performance Considerations

### Gas Optimization

- Efficient data structures for vote storage
- Optimized threshold calculations
- Minimal storage operations
- Batch processing where possible

### Scalability

- Support for large numbers of voters
- Efficient dispute threshold calculations
- Optimized payout calculations
- Minimal on-chain computation

---

*This voting system provides a robust foundation for prediction market operations with sophisticated dispute resolution and community consensus mechanisms.* 