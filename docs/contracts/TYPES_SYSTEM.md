# Predictify Hybrid Types System (Code-Synced)

This document is the canonical field/variant reference for every `#[contracttype]` defined in `/contracts/predictify-hybrid/src/types.rs`.

## Scope

- Source of truth: `/contracts/predictify-hybrid/src/types.rs`
- Included here: all `#[contracttype]` enums and structs in that file
- Not included: non-`contracttype` helper structs (for example, `OraclePriceData`)

## Enums

### `MarketState`
- `Active`
- `Ended`
- `Disputed`
- `Resolved`
- `Closed`
- `Cancelled`

### `OracleProvider`
- `Reflector`
- `Pyth`
- `BandProtocol`
- `DIA`

### `ReflectorAsset`
- `Stellar`
- `BTC`
- `ETH`
- `Other(Symbol)`

### `OracleVerificationStatus`
- `Pending`
- `InProgress`
- `Verified`
- `InvalidSignature`
- `StaleData`
- `OracleUnavailable`
- `ThresholdNotMet`
- `NoConsensus`

### `MarketStatus`
- `Active`
- `Ended`
- `Disputed`
- `Resolved`
- `Closed`
- `Cancelled`

### `BetStatus`
- `Active`
- `Won`
- `Lost`
- `Refunded`
- `Cancelled`

### `EventVisibility`
- `Public`
- `Private`

## Structs

### `OracleConfig`
| Field | Type |
|---|---|
| `provider` | `OracleProvider` |
| `oracle_address` | `Address` |
| `feed_id` | `String` |
| `threshold` | `i128` |
| `comparison` | `String` |

### `Market`
| Field | Type |
|---|---|
| `admin` | `Address` |
| `question` | `String` |
| `outcomes` | `Vec<String>` |
| `end_time` | `u64` |
| `oracle_config` | `OracleConfig` |
| `has_fallback` | `bool` |
| `fallback_oracle_config` | `OracleConfig` |
| `resolution_timeout` | `u64` |
| `oracle_result` | `Option<String>` |
| `votes` | `Map<Address, String>` |
| `stakes` | `Map<Address, i128>` |
| `claimed` | `Map<Address, bool>` |
| `total_staked` | `i128` |
| `dispute_stakes` | `Map<Address, i128>` |
| `winning_outcomes` | `Option<Vec<String>>` |
| `fee_collected` | `bool` |
| `state` | `MarketState` |
| `total_extension_days` | `u32` |
| `max_extension_days` | `u32` |
| `extension_history` | `Vec<MarketExtension>` |
| `category` | `Option<String>` |
| `tags` | `Vec<String>` |
| `min_pool_size` | `Option<i128>` |
| `bet_deadline` | `u64` |
| `dispute_window_seconds` | `u64` |

### `BetLimits`
| Field | Type |
|---|---|
| `min_bet` | `i128` |
| `max_bet` | `i128` |

### `EventHistoryEntry`
| Field | Type |
|---|---|
| `market_id` | `Symbol` |
| `question` | `String` |
| `outcomes` | `Vec<String>` |
| `end_time` | `u64` |
| `created_at` | `u64` |
| `state` | `MarketState` |
| `winning_outcome` | `Option<String>` |
| `total_staked` | `i128` |
| `archived_at` | `Option<u64>` |
| `category` | `String` |
| `tags` | `Vec<String>` |

### `PlatformStatistics`
| Field | Type |
|---|---|
| `total_events_created` | `u64` |
| `total_bets_placed` | `u64` |
| `total_volume` | `i128` |
| `total_fees_collected` | `i128` |
| `active_events_count` | `u32` |

### `UserStatistics`
| Field | Type |
|---|---|
| `total_bets_placed` | `u64` |
| `total_amount_wagered` | `i128` |
| `total_winnings` | `i128` |
| `total_bets_won` | `u64` |
| `win_rate` | `u32` |
| `last_activity_ts` | `u64` |

### `OracleResult`
| Field | Type |
|---|---|
| `market_id` | `Symbol` |
| `outcome` | `String` |
| `price` | `i128` |
| `threshold` | `i128` |
| `comparison` | `String` |
| `provider` | `OracleProvider` |
| `feed_id` | `String` |
| `timestamp` | `u64` |
| `block_number` | `u32` |
| `is_verified` | `bool` |
| `confidence_score` | `u32` |
| `sources_count` | `u32` |
| `signature` | `Option<String>` |
| `error_message` | `Option<String>` |

### `GlobalOracleValidationConfig`
| Field | Type |
|---|---|
| `max_staleness_secs` | `u64` |
| `max_confidence_bps` | `u32` |

### `EventOracleValidationConfig`
| Field | Type |
|---|---|
| `max_staleness_secs` | `u64` |
| `max_confidence_bps` | `u32` |

### `MultiOracleResult`
| Field | Type |
|---|---|
| `market_id` | `Symbol` |
| `final_outcome` | `String` |
| `individual_results` | `Vec<OracleResult>` |
| `consensus_reached` | `bool` |
| `consensus_threshold` | `u32` |
| `agreement_percentage` | `u32` |
| `timestamp` | `u64` |

### `OracleSource`
| Field | Type |
|---|---|
| `source_id` | `Symbol` |
| `provider` | `OracleProvider` |
| `contract_address` | `Address` |
| `weight` | `u32` |
| `is_active` | `bool` |
| `priority` | `u32` |
| `last_success` | `u64` |
| `failure_count` | `u32` |

### `OracleFetchRequest`
| Field | Type |
|---|---|
| `market_id` | `Symbol` |
| `feed_id` | `String` |
| `max_data_age` | `u64` |
| `required_confirmations` | `u32` |
| `use_fallback` | `bool` |
| `min_confidence` | `u32` |

### `ReflectorPriceData`
| Field | Type |
|---|---|
| `price` | `i128` |
| `timestamp` | `u64` |
| `source` | `String` |

### `MarketExtension`
| Field | Type |
|---|---|
| `additional_days` | `u32` |
| `admin` | `Address` |
| `reason` | `String` |
| `fee_amount` | `i128` |
| `timestamp` | `u64` |

### `ExtensionStats`
| Field | Type |
|---|---|
| `total_extensions` | `u32` |
| `total_extension_days` | `u32` |
| `max_extension_days` | `u32` |
| `can_extend` | `bool` |
| `extension_fee_per_day` | `i128` |

### `MarketCreationParams`
| Field | Type |
|---|---|
| `admin` | `Address` |
| `question` | `String` |
| `outcomes` | `Vec<String>` |
| `duration_days` | `u32` |
| `oracle_config` | `OracleConfig` |
| `creation_fee` | `i128` |

### `CommunityConsensus`
| Field | Type |
|---|---|
| `outcome` | `String` |
| `votes` | `u32` |
| `total_votes` | `u32` |
| `percentage` | `i128` |

### `MarketPauseInfo`
| Field | Type |
|---|---|
| `is_paused` | `bool` |
| `paused_at` | `u64` |
| `pause_duration_hours` | `u32` |
| `paused_by` | `Address` |
| `pause_end_time` | `u64` |
| `original_state` | `MarketState` |

### `EventDetailsQuery`
| Field | Type |
|---|---|
| `market_id` | `Symbol` |
| `question` | `String` |
| `outcomes` | `Vec<String>` |
| `created_at` | `u64` |
| `end_time` | `u64` |
| `status` | `MarketStatus` |
| `oracle_provider` | `String` |
| `feed_id` | `String` |
| `total_staked` | `i128` |
| `winning_outcome` | `Option<String>` |
| `oracle_result` | `Option<String>` |
| `participant_count` | `u32` |
| `vote_count` | `u32` |
| `admin` | `Address` |

### `UserBetQuery`
| Field | Type |
|---|---|
| `user` | `Address` |
| `market_id` | `Symbol` |
| `outcome` | `String` |
| `stake_amount` | `i128` |
| `voted_at` | `u64` |
| `is_winning` | `bool` |
| `has_claimed` | `bool` |
| `potential_payout` | `i128` |
| `dispute_stake` | `i128` |

### `UserBalanceQuery`
| Field | Type |
|---|---|
| `user` | `Address` |
| `available_balance` | `i128` |
| `total_staked` | `i128` |
| `total_winnings` | `i128` |
| `active_bet_count` | `u32` |
| `resolved_market_count` | `u32` |
| `unclaimed_balance` | `i128` |

### `MarketPoolQuery`
| Field | Type |
|---|---|
| `market_id` | `Symbol` |
| `total_pool` | `i128` |
| `outcome_pools` | `Map<String, i128>` |
| `platform_fees` | `i128` |
| `implied_probability_yes` | `u32` |
| `implied_probability_no` | `u32` |

### `ContractStateQuery`
| Field | Type |
|---|---|
| `total_markets` | `u32` |
| `active_markets` | `u32` |
| `resolved_markets` | `u32` |
| `total_value_locked` | `i128` |
| `total_fees_collected` | `i128` |
| `unique_users` | `u32` |
| `contract_version` | `String` |
| `last_update` | `u64` |

### `MultipleBetsQuery`
| Field | Type |
|---|---|
| `bets` | `Vec<UserBetQuery>` |
| `total_stake` | `i128` |
| `total_potential_payout` | `i128` |
| `winning_bets` | `u32` |

### `Bet`
| Field | Type |
|---|---|
| `user` | `Address` |
| `market_id` | `Symbol` |
| `outcome` | `String` |
| `amount` | `i128` |
| `timestamp` | `u64` |
| `status` | `BetStatus` |

### `BetStats`
| Field | Type |
|---|---|
| `total_bets` | `u32` |
| `total_amount_locked` | `i128` |
| `unique_bettors` | `u32` |
| `outcome_totals` | `Map<String, i128>` |

### `Event`
| Field | Type |
|---|---|
| `id` | `Symbol` |
| `description` | `String` |
| `outcomes` | `Vec<String>` |
| `end_time` | `u64` |
| `oracle_config` | `OracleConfig` |
| `has_fallback` | `bool` |
| `fallback_oracle_config` | `OracleConfig` |
| `resolution_timeout` | `u64` |
| `admin` | `Address` |
| `created_at` | `u64` |
| `status` | `MarketState` |
| `visibility` | `EventVisibility` |
| `allowlist` | `Vec<Address>` |

### `Balance`
| Field | Type |
|---|---|
| `user` | `Address` |
| `asset` | `ReflectorAsset` |
| `amount` | `i128` |

## Maintenance Rule

When adding, removing, or renaming any field or variant in `types.rs`, update this document in the same PR.
