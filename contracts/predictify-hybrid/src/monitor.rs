#![allow(dead_code)]

use alloc::format;
use soroban_sdk::{contracttype, symbol_short, Address, Env, Map, String, Symbol, Vec};

use crate::errors::Error;
use crate::events::EventEmitter;

// ===== CONSTANTS =====

/// Default maximum number of events the bounded queue can hold.
pub const DEFAULT_QUEUE_CAPACITY: u32 = 128;

/// Minimum allowed queue capacity.
pub const MIN_QUEUE_CAPACITY: u32 = 1;

/// Maximum allowed queue capacity.
pub const MAX_QUEUE_CAPACITY: u32 = 10_000;

// ===== STORAGE KEYS =====

/// Persistent storage key prefix for queue state.
const QUEUE_STATE_KEY: &str = "mon_q_state";

/// Persistent storage key prefix for queue entries.
const QUEUE_ENTRY_KEY: &str = "mon_q_entry";

/// Persistent storage key prefix for overflow tracking.
const QUEUE_OVERFLOW_KEY: &str = "mon_q_overflow";

// ===== TYPES =====

/// Category of a monitor event.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MonitorEventCategory {
    /// Market lifecycle event (created, ended, resolved, etc.).
    Market,
    /// Oracle health or data event.
    Oracle,
    /// Bet placement or cancellation event.
    Bet,
    /// Fee collection event.
    Fee,
    /// Dispute event.
    Dispute,
    /// Circuit breaker state change.
    CircuitBreaker,
    /// Admin action event.
    Admin,
    /// Generic system event.
    System,
}

/// Severity level for a monitor event.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
pub enum MonitorEventSeverity {
    /// Informational event, no action needed.
    Info,
    /// Warning event, may need attention.
    Warning,
    /// Critical event, requires immediate attention.
    Critical,
}

/// A single monitor event stored in the bounded queue.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MonitorEvent {
    /// Unique event identifier.
    pub event_id: Symbol,
    /// Category of the event.
    pub category: MonitorEventCategory,
    /// Severity level.
    pub severity: MonitorEventSeverity,
    /// Human-readable event message.
    pub message: String,
    /// Optional related market identifier.
    pub market_id: Option<Symbol>,
    /// Optional actor address that triggered the event.
    pub actor: Option<Address>,
    /// Ledger timestamp when the event was created.
    pub timestamp: u64,
    /// Additional key-value metadata for the event.
    pub metadata: Map<String, String>,
}

/// State of the bounded monitor queue, stored in persistent storage.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MonitorQueueState {
    /// Head index (next position to pop from).
    pub head: u32,
    /// Tail index (next position to push to).
    pub tail: u32,
    /// Current number of events in the queue.
    pub len: u32,
    /// Maximum capacity of the queue.
    pub capacity: u32,
    /// Total number of events pushed since initialization.
    pub total_pushed: u64,
    /// Total number of events popped since initialization.
    pub total_popped: u64,
    /// Total number of overflow events (events discarded due to full queue).
    pub overflow_count: u64,
    /// Timestamp of the last overflow event.
    pub last_overflow_timestamp: u64,
}

/// Summary statistics for the bounded queue.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MonitorQueueStats {
    /// Current number of events in the queue.
    pub current_len: u32,
    /// Maximum capacity of the queue.
    pub capacity: u32,
    /// Total events pushed since initialization.
    pub total_pushed: u64,
    /// Total events popped since initialization.
    pub total_popped: u64,
    /// Total overflow events discarded.
    pub overflow_count: u64,
    /// Timestamp of the last overflow.
    pub last_overflow_timestamp: u64,
    /// Whether the queue is currently full.
    pub is_full: bool,
    /// Whether the queue is currently empty.
    pub is_empty: bool,
}

impl Default for MonitorQueueState {
    fn default() -> Self {
        Self {
            head: 0,
            tail: 0,
            len: 0,
            capacity: DEFAULT_QUEUE_CAPACITY,
            total_pushed: 0,
            total_popped: 0,
            overflow_count: 0,
            last_overflow_timestamp: 0,
        }
    }
}

// ===== BOUNDED MONITOR QUEUE =====

/// A FIFO bounded queue for contract monitor events with overflow tracking.
///
/// The queue is backed by persistent Soroban storage and uses a circular buffer
/// design. When the queue reaches capacity, pushing a new event discards the
/// oldest event and emits an `OverflowEvent` so that off-chain indexers can
/// track data loss.
///
/// # Design
///
/// - **Circular buffer**: `head` points to the oldest event, `tail` to the next
///   write slot. Indices wrap around using modulo arithmetic.
/// - **Capacity bounds**: Must be within `[MIN_QUEUE_CAPACITY, MAX_QUEUE_CAPACITY]`.
/// - **Overflow**: When `len == capacity`, the oldest event is evicted before the
///   new event is written. The overflow counter and timestamp are updated, and an
///   `OverflowEvent` is emitted.
/// - **Atomic operations**: Each push/pop reads the queue state, performs the
///   mutation, and writes the state back in a single storage transaction.
///
/// # Storage Layout
///
/// - `{QUEUE_STATE_KEY}` → `MonitorQueueState`
/// - `{QUEUE_ENTRY_KEY}_{index}` → `MonitorEvent`
/// - `{QUEUE_OVERFLOW_KEY}` → overflow metadata
pub struct BoundedMonitorQueue;

impl BoundedMonitorQueue {
    /// Initialize a new bounded queue with the given capacity.
    ///
    /// # Arguments
    ///
    /// * `env` - The Soroban environment.
    /// * `capacity` - Maximum number of events the queue can hold. Must be
    ///   between `MIN_QUEUE_CAPACITY` and `MAX_QUEUE_CAPACITY` inclusive.
    ///
    /// # Errors
    ///
    /// - [`Error::QueueCapacityOutOfRange`] if `capacity` is outside
    ///   `[MIN_QUEUE_CAPACITY, MAX_QUEUE_CAPACITY]`.
    /// - [`Error::QueueAlreadyInitialized`] if the queue has already been initialized.
    ///   Re-initialization is rejected so an existing queue can never be silently reset.
    ///
    /// Both conditions previously surfaced as a generic `InvalidInput` or a bare
    /// `panic!`, neither of which a client could distinguish from any other failure.
    pub fn initialize(env: &Env, capacity: u32) -> Result<(), Error> {
        if capacity < MIN_QUEUE_CAPACITY || capacity > MAX_QUEUE_CAPACITY {
            return Err(Error::QueueCapacityOutOfRange);
        }

        let state_key = Symbol::new(env, QUEUE_STATE_KEY);
        if env.storage().persistent().has(&state_key) {
            return Err(Error::QueueAlreadyInitialized);
        }

        let state = MonitorQueueState {
            head: 0,
            tail: 0,
            len: 0,
            capacity,
            total_pushed: 0,
            total_popped: 0,
            overflow_count: 0,
            last_overflow_timestamp: 0,
        };

        env.storage().persistent().set(&state_key, &state);
        Ok(())
    }

    /// Returns the current queue state from persistent storage.
    ///
    /// # Panics
    ///
    /// Panics if the queue has not been initialized.
    pub fn get_state(env: &Env) -> MonitorQueueState {
        let state_key = Symbol::new(env, QUEUE_STATE_KEY);
        env.storage()
            .persistent()
            .get(&state_key)
            .expect("monitor queue not initialized")
    }

    /// Write queue state back to persistent storage.
    fn store_state(env: &Env, state: &MonitorQueueState) {
        let state_key = Symbol::new(env, QUEUE_STATE_KEY);
        env.storage().persistent().set(&state_key, state);
    }

    /// Compute the storage key for a queue entry at the given logical index.
    fn entry_key(env: &Env, index: u32) -> Symbol {
        Symbol::new(env, &format!("{}_{}", QUEUE_ENTRY_KEY, index))
    }

    /// Push a monitor event onto the back of the queue.
    ///
    /// If the queue is at capacity, the oldest event is evicted and an
    /// overflow event is emitted via `EventEmitter::emit_monitor_queue_overflow`
    /// so that off-chain indexers can track data loss.
    ///
    /// # Arguments
    ///
    /// * `env` - The Soroban environment.
    /// * `event` - The monitor event to enqueue.
    ///
    /// # Returns
    ///
    /// * `Ok(true)` - Event enqueued but caused an eviction (overflow).
    /// * `Ok(false)` - Event enqueued normally.
    /// * `Err(Error)` - Storage or validation error.
    pub fn push(env: &Env, event: &MonitorEvent) -> Result<bool, Error> {
        let mut state = Self::get_state(env);
        let mut overflowed = false;
        let mut evicted_event_id: Option<Symbol> = None;

        // If queue is full, read the evicted event's id before removing it.
        if state.len == state.capacity {
            let head_key = Self::entry_key(env, state.head);
            if let Some(evicted) =
                env.storage().persistent().get::<Symbol, MonitorEvent>(&head_key)
            {
                evicted_event_id = Some(evicted.event_id);
            }
            Self::remove_entry(env, state.head);
            state.head = (state.head + 1) % state.capacity;
            state.len = state.len.saturating_sub(1);
            state.overflow_count = state.overflow_count.saturating_add(1);
            state.last_overflow_timestamp = env.ledger().timestamp();
            overflowed = true;

            // Emit overflow event for off-chain indexers.
            Self::emit_overflow_event(env, &state, evicted_event_id);
        }

        // Write the new event at the tail position.
        let entry_key = Self::entry_key(env, state.tail);
        env.storage().persistent().set(&entry_key, event);

        state.tail = (state.tail + 1) % state.capacity;
        state.len = state.len.saturating_add(1);
        state.total_pushed = state.total_pushed.saturating_add(1);

        Self::store_state(env, &state);
        Ok(overflowed)
    }

    /// Pop the oldest event from the front of the queue.
    ///
    /// # Arguments
    ///
    /// * `env` - The Soroban environment.
    ///
    /// # Returns
    ///
    /// * `Ok(Some(event))` - The oldest event if the queue is non-empty.
    /// * `Ok(None)` - The queue is empty.
    pub fn pop(env: &Env) -> Result<Option<MonitorEvent>, Error> {
        let mut state = Self::get_state(env);

        if state.len == 0 {
            return Ok(None);
        }

        let entry_key = Self::entry_key(env, state.head);
        let event: MonitorEvent = env
            .storage()
            .persistent()
            .get(&entry_key)
            .ok_or(Error::InvalidState)?;

        Self::remove_entry(env, state.head);

        state.head = (state.head + 1) % state.capacity;
        state.len = state.len.saturating_sub(1);
        state.total_popped = state.total_popped.saturating_add(1);

        Self::store_state(env, &state);
        Ok(Some(event))
    }

    /// Peek at the oldest event without removing it.
    ///
    /// # Returns
    ///
    /// * `Ok(Some(event))` - The oldest event if the queue is non-empty.
    /// * `Ok(None)` - The queue is empty.
    pub fn peek(env: &Env) -> Result<Option<MonitorEvent>, Error> {
        let state = Self::get_state(env);

        if state.len == 0 {
            return Ok(None);
        }

        let entry_key = Self::entry_key(env, state.head);
        let event: Option<MonitorEvent> = env.storage().persistent().get(&entry_key);
        Ok(event)
    }

    /// Drain up to `max_count` events from the queue, returning them in FIFO order.
    ///
    /// If `max_count` is 0 or the queue is empty, returns an empty vector.
    /// Each drained event is removed from the queue.
    ///
    /// # Arguments
    ///
    /// * `env` - The Soroban environment.
    /// * `max_count` - Maximum number of events to drain. Use `u32::MAX` to drain all.
    ///
    /// # Returns
    ///
    /// A vector of drained events in FIFO order (oldest first).
    pub fn drain(env: &Env, max_count: u32) -> Result<Vec<MonitorEvent>, Error> {
        let mut result = Vec::new(env);
        let count = max_count.min(Self::get_state(env).len);

        for _ in 0..count {
            match Self::pop(env)? {
                Some(event) => result.push_back(event),
                None => break,
            }
        }

        Ok(result)
    }

    /// Remove all events from the queue, resetting it to empty.
    ///
    /// This does **not** reset the overflow counter or total push/pop counters.
    /// Use `reset` for a full reset.
    pub fn clear(env: &Env) {
        let mut state = Self::get_state(env);

        // Remove all stored entries.
        for i in 0..state.capacity {
            Self::remove_entry(env, i);
        }

        state.head = 0;
        state.tail = 0;
        state.len = 0;

        Self::store_state(env, &state);
    }

    /// Fully reset the queue to its initial empty state, clearing all counters.
    pub fn reset(env: &Env) {
        let state_key = Symbol::new(env, QUEUE_STATE_KEY);
        let state = Self::get_state(env);

        // Remove all stored entries.
        for i in 0..state.capacity {
            Self::remove_entry(env, i);
        }

        let fresh = MonitorQueueState {
            head: 0,
            tail: 0,
            len: 0,
            capacity: state.capacity,
            total_pushed: 0,
            total_popped: 0,
            overflow_count: 0,
            last_overflow_timestamp: 0,
        };

        env.storage().persistent().set(&state_key, &fresh);
    }

    /// Return summary statistics for the queue.
    pub fn stats(env: &Env) -> MonitorQueueStats {
        let state = Self::get_state(env);
        MonitorQueueStats {
            current_len: state.len,
            capacity: state.capacity,
            total_pushed: state.total_pushed,
            total_popped: state.total_popped,
            overflow_count: state.overflow_count,
            last_overflow_timestamp: state.last_overflow_timestamp,
            is_full: state.len == state.capacity,
            is_empty: state.len == 0,
        }
    }

    /// Return the current number of events in the queue.
    pub fn len(env: &Env) -> u32 {
        Self::get_state(env).len
    }

    /// Return `true` if the queue contains no events.
    pub fn is_empty(env: &Env) -> bool {
        Self::get_state(env).len == 0
    }

    /// Return `true` if the queue is at capacity.
    pub fn is_full(env: &Env) -> bool {
        let state = Self::get_state(env);
        state.len == state.capacity
    }

    /// Return the configured capacity of the queue.
    pub fn capacity(env: &Env) -> u32 {
        Self::get_state(env).capacity
    }

    /// Return all events currently in the queue in FIFO order without removing them.
    ///
    /// This allocates a `Vec` proportional to queue length. Use only for
    /// read-only inspection (e.g., debugging, snapshotting).
    pub fn peek_all(env: &Env) -> Result<Vec<MonitorEvent>, Error> {
        let state = Self::get_state(env);
        let mut result = Vec::new(env);

        if state.len == 0 {
            return Ok(result);
        }

        let mut idx = state.head;
        for _ in 0..state.len {
            let entry_key = Self::entry_key(env, idx);
            if let Some(event) = env.storage().persistent().get::<Symbol, MonitorEvent>(&entry_key) {
                result.push_back(event);
            }
            idx = (idx + 1) % state.capacity;
        }

        Ok(result)
    }

    // ===== PRIVATE HELPERS =====

    /// Delete a single entry from storage.
    fn remove_entry(env: &Env, index: u32) {
        let key = Self::entry_key(env, index);
        env.storage().persistent().remove(&key);
    }

    /// Emit an overflow event when an eviction occurs.
    ///
    /// Delegates to `EventEmitter::emit_monitor_queue_overflow` so that all
    /// monitor overflow events flow through the centralized event system.
    fn emit_overflow_event(
        env: &Env,
        state: &MonitorQueueState,
        evicted_event_id: Option<Symbol>,
    ) {
        EventEmitter::emit_monitor_queue_overflow(
            env,
            state.overflow_count,
            evicted_event_id,
            state.capacity,
        );
    }
}

// ===== TESTS =====

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::Address as _;

    fn setup() -> (Env, soroban_sdk::Address) {
        let env = Env::default();
        let contract_id = env.register(crate::PredictifyHybrid, ());
        env.as_contract(&contract_id, || {
            BoundedMonitorQueue::initialize(&env, 4).unwrap();
        });
        (env, contract_id)
    }

    fn make_event(env: &Env, id: &str, category: MonitorEventCategory) -> MonitorEvent {
        MonitorEvent {
            event_id: Symbol::new(env, id),
            category,
            severity: MonitorEventSeverity::Info,
            message: String::from_str(env, "test event"),
            market_id: None,
            actor: None,
            timestamp: env.ledger().timestamp(),
            metadata: Map::new(env),
        }
    }

    #[test]
    fn initialize_sets_capacity() {
        let env = Env::default();
        let contract_id = env.register(crate::PredictifyHybrid, ());
        env.as_contract(&contract_id, || {
            BoundedMonitorQueue::initialize(&env, 10).unwrap();
            assert_eq!(BoundedMonitorQueue::capacity(&env), 10);
            assert_eq!(BoundedMonitorQueue::len(&env), 0);
            assert!(BoundedMonitorQueue::is_empty(&env));
            assert!(!BoundedMonitorQueue::is_full(&env));
        });
    }

    #[test]
    fn initialize_rejects_double_init() {
        // `setup()` already initializes the queue; the second call must return a
        // typed error rather than aborting the frame with a panic.
        let (env, contract_id) = setup();
        env.as_contract(&contract_id, || {
            let result = BoundedMonitorQueue::initialize(&env, 10);
            assert_eq!(result, Err(Error::QueueAlreadyInitialized));
        });
    }

    #[test]
    fn initialize_rejects_zero_capacity() {
        let env = Env::default();
        let contract_id = env.register(crate::PredictifyHybrid, ());
        env.as_contract(&contract_id, || {
            let result = BoundedMonitorQueue::initialize(&env, 0);
            assert_eq!(result, Err(Error::QueueCapacityOutOfRange));
        });
    }

    #[test]
    fn initialize_accepts_capacity_bounds() {
        let env = Env::default();
        let contract_id = env.register(crate::PredictifyHybrid, ());
        env.as_contract(&contract_id, || {
            assert!(BoundedMonitorQueue::initialize(&env, MIN_QUEUE_CAPACITY).is_ok());
        });

        let env2 = Env::default();
        let contract_id2 = env2.register(crate::PredictifyHybrid, ());
        env2.as_contract(&contract_id2, || {
            assert!(BoundedMonitorQueue::initialize(&env2, MAX_QUEUE_CAPACITY).is_ok());
        });
    }

    #[test]
    fn initialize_rejects_capacity_above_max() {
        let env = Env::default();
        let contract_id = env.register(crate::PredictifyHybrid, ());
        env.as_contract(&contract_id, || {
            let result = BoundedMonitorQueue::initialize(&env, MAX_QUEUE_CAPACITY + 1);
            assert_eq!(result, Err(Error::QueueCapacityOutOfRange));
        });
    }

    #[test]
    fn push_increments_len() {
        let (env, contract_id) = setup();
        env.as_contract(&contract_id, || {
            let ev = make_event(&env, "e1", MonitorEventCategory::Market);
            let overflowed = BoundedMonitorQueue::push(&env, &ev).unwrap();
            assert!(!overflowed);
            assert_eq!(BoundedMonitorQueue::len(&env), 1);
            assert_eq!(BoundedMonitorQueue::stats(&env).total_pushed, 1);
        });
    }

    #[test]
    fn pop_returns_fifo_order() {
        let (env, contract_id) = setup();
        env.as_contract(&contract_id, || {
            let ev1 = make_event(&env, "e1", MonitorEventCategory::Market);
            let ev2 = make_event(&env, "e2", MonitorEventCategory::Oracle);
            let ev3 = make_event(&env, "e3", MonitorEventCategory::Bet);

            BoundedMonitorQueue::push(&env, &ev1).unwrap();
            BoundedMonitorQueue::push(&env, &ev2).unwrap();
            BoundedMonitorQueue::push(&env, &ev3).unwrap();

            let popped1 = BoundedMonitorQueue::pop(&env).unwrap().unwrap();
            let popped2 = BoundedMonitorQueue::pop(&env).unwrap().unwrap();
            let popped3 = BoundedMonitorQueue::pop(&env).unwrap().unwrap();

            assert_eq!(popped1.event_id, Symbol::new(&env, "e1"));
            assert_eq!(popped2.event_id, Symbol::new(&env, "e2"));
            assert_eq!(popped3.event_id, Symbol::new(&env, "e3"));

            assert!(BoundedMonitorQueue::is_empty(&env));
            assert_eq!(BoundedMonitorQueue::stats(&env).total_popped, 3);
        });
    }

    #[test]
    fn pop_empty_queue_returns_none() {
        let (env, contract_id) = setup();
        env.as_contract(&contract_id, || {
            let result = BoundedMonitorQueue::pop(&env).unwrap();
            assert!(result.is_none());
        });
    }

    #[test]
    fn overflow_evicts_oldest_and_returns_true() {
        let (env, contract_id) = setup();
        env.as_contract(&contract_id, || {
            // Fill queue to capacity (4).
            for i in 0..4 {
                let ev = make_event(&env, &format!("e{}", i), MonitorEventCategory::System);
                BoundedMonitorQueue::push(&env, &ev).unwrap();
            }
            assert!(BoundedMonitorQueue::is_full(&env));

            // Push one more — should evict e0.
            let overflow_ev = make_event(&env, "overflow", MonitorEventCategory::System);
            let overflowed = BoundedMonitorQueue::push(&env, &overflow_ev).unwrap();
            assert!(overflowed);

            let stats = BoundedMonitorQueue::stats(&env);
            assert_eq!(stats.overflow_count, 1);
            assert_eq!(stats.current_len, 4); // still at capacity

            // The oldest surviving event should be e1.
            let first = BoundedMonitorQueue::pop(&env).unwrap().unwrap();
            assert_eq!(first.event_id, Symbol::new(&env, "e1"));
        });
    }

    #[test]
    fn multiple_overflows_track_cumulative_count() {
        let (env, contract_id) = setup();
        env.as_contract(&contract_id, || {
            // Fill to capacity.
            for i in 0..4 {
                let ev = make_event(&env, &format!("e{}", i), MonitorEventCategory::System);
                BoundedMonitorQueue::push(&env, &ev).unwrap();
            }

            // Push 3 more, each causes overflow.
            for i in 0..3 {
                let ev = make_event(
                    &env,
                    &format!("extra{}", i),
                    MonitorEventCategory::System,
                );
                BoundedMonitorQueue::push(&env, &ev).unwrap();
            }

            let stats = BoundedMonitorQueue::stats(&env);
            assert_eq!(stats.overflow_count, 3);
            assert_eq!(stats.total_pushed, 7);
            assert_eq!(stats.current_len, 4);

            // Queue should contain e3, extra0, extra1, extra2.
            let first = BoundedMonitorQueue::pop(&env).unwrap().unwrap();
            assert_eq!(first.event_id, Symbol::new(&env, "e3"));
        });
    }

    #[test]
    fn peek_does_not_remove() {
        let (env, contract_id) = setup();
        env.as_contract(&contract_id, || {
            let ev = make_event(&env, "peek_test", MonitorEventCategory::Market);
            BoundedMonitorQueue::push(&env, &ev).unwrap();

            let peeked = BoundedMonitorQueue::peek(&env).unwrap().unwrap();
            assert_eq!(peeked.event_id, Symbol::new(&env, "peek_test"));

            // Queue length unchanged.
            assert_eq!(BoundedMonitorQueue::len(&env), 1);
        });
    }

    #[test]
    fn peek_empty_returns_none() {
        let (env, contract_id) = setup();
        env.as_contract(&contract_id, || {
            assert!(BoundedMonitorQueue::peek(&env).unwrap().is_none());
        });
    }

    #[test]
    fn drain_returns_up_to_max_count() {
        let (env, contract_id) = setup();
        env.as_contract(&contract_id, || {
            for i in 0..3 {
                let ev = make_event(&env, &format!("d{}", i), MonitorEventCategory::System);
                BoundedMonitorQueue::push(&env, &ev).unwrap();
            }

            let drained = BoundedMonitorQueue::drain(&env, 2).unwrap();
            assert_eq!(drained.len(), 2);
            assert_eq!(BoundedMonitorQueue::len(&env), 1);

            // Drained events are in FIFO order.
            assert_eq!(drained.get(0).unwrap().event_id, Symbol::new(&env, "d0"));
            assert_eq!(drained.get(1).unwrap().event_id, Symbol::new(&env, "d1"));
        });
    }

    #[test]
    fn drain_all_with_large_max() {
        let (env, contract_id) = setup();
        env.as_contract(&contract_id, || {
            for i in 0..2 {
                let ev = make_event(&env, &format!("d{}", i), MonitorEventCategory::System);
                BoundedMonitorQueue::push(&env, &ev).unwrap();
            }

            let drained = BoundedMonitorQueue::drain(&env, u32::MAX).unwrap();
            assert_eq!(drained.len(), 2);
            assert!(BoundedMonitorQueue::is_empty(&env));
        });
    }

    #[test]
    fn drain_empty_queue_returns_empty_vec() {
        let (env, contract_id) = setup();
        env.as_contract(&contract_id, || {
            let drained = BoundedMonitorQueue::drain(&env, 10).unwrap();
            assert_eq!(drained.len(), 0);
        });
    }

    #[test]
    fn clear_removes_all_events() {
        let (env, contract_id) = setup();
        env.as_contract(&contract_id, || {
            for i in 0..3 {
                let ev = make_event(&env, &format!("c{}", i), MonitorEventCategory::System);
                BoundedMonitorQueue::push(&env, &ev).unwrap();
            }

            BoundedMonitorQueue::clear(&env);
            assert!(BoundedMonitorQueue::is_empty(&env));
            assert_eq!(BoundedMonitorQueue::len(&env), 0);

            // Counters preserved.
            assert_eq!(BoundedMonitorQueue::stats(&env).total_pushed, 3);
        });
    }

    #[test]
    fn reset_clears_everything() {
        let (env, contract_id) = setup();
        env.as_contract(&contract_id, || {
            for i in 0..3 {
                let ev = make_event(&env, &format!("r{}", i), MonitorEventCategory::System);
                BoundedMonitorQueue::push(&env, &ev).unwrap();
            }

            BoundedMonitorQueue::reset(&env);
            let stats = BoundedMonitorQueue::stats(&env);
            assert_eq!(stats.current_len, 0);
            assert_eq!(stats.total_pushed, 0);
            assert_eq!(stats.total_popped, 0);
            assert_eq!(stats.overflow_count, 0);
        });
    }

    #[test]
    fn peek_all_returns_fifo_without_removing() {
        let (env, contract_id) = setup();
        env.as_contract(&contract_id, || {
            let ev1 = make_event(&env, "a", MonitorEventCategory::Market);
            let ev2 = make_event(&env, "b", MonitorEventCategory::Oracle);
            BoundedMonitorQueue::push(&env, &ev1).unwrap();
            BoundedMonitorQueue::push(&env, &ev2).unwrap();

            let all = BoundedMonitorQueue::peek_all(&env).unwrap();
            assert_eq!(all.len(), 2);
            assert_eq!(all.get(0).unwrap().event_id, Symbol::new(&env, "a"));
            assert_eq!(all.get(1).unwrap().event_id, Symbol::new(&env, "b"));

            // Queue unchanged.
            assert_eq!(BoundedMonitorQueue::len(&env), 2);
        });
    }

    #[test]
    fn push_pop_wrap_around_indices() {
        let (env, contract_id) = setup();
        env.as_contract(&contract_id, || {
            // Fill and drain to move head/tail past 0.
            for i in 0..4 {
                let ev = make_event(&env, &format!("w{}", i), MonitorEventCategory::System);
                BoundedMonitorQueue::push(&env, &ev).unwrap();
            }
            // Drain 2: head=2, tail=0 (wrapped).
            BoundedMonitorQueue::drain(&env, 2).unwrap();

            // Push 3 more: should wrap tail around.
            for i in 0..3 {
                let ev = make_event(
                    &env,
                    &format!("wrap{}", i),
                    MonitorEventCategory::System,
                );
                BoundedMonitorQueue::push(&env, &ev).unwrap();
            }

            // Queue now has 5 entries: w2, w3, wrap0, wrap1, wrap2.
            // But capacity is 4, so the last push of wrap2 should have evicted w2.
            assert_eq!(BoundedMonitorQueue::len(&env), 4);

            let first = BoundedMonitorQueue::pop(&env).unwrap().unwrap();
            assert_eq!(first.event_id, Symbol::new(&env, "w3"));
        });
    }

    #[test]
    fn stats_matches_manual_tracking() {
        let (env, contract_id) = setup();
        env.as_contract(&contract_id, || {
            for i in 0..6 {
                let ev = make_event(&env, &format!("s{}", i), MonitorEventCategory::System);
                BoundedMonitorQueue::push(&env, &ev).unwrap();
            }
            BoundedMonitorQueue::pop(&env).unwrap();

            let stats = BoundedMonitorQueue::stats(&env);
            assert_eq!(stats.capacity, 4);
            assert_eq!(stats.current_len, 3); // 4 pushed, 1 popped
            assert_eq!(stats.total_pushed, 6);
            assert_eq!(stats.total_popped, 1);
            assert_eq!(stats.overflow_count, 2); // 2 evictions from the 5th and 6th push
            assert!(!stats.is_full);
            assert!(!stats.is_empty);
        });
    }

    #[test]
    fn capacity_boundaries() {
        let env = Env::default();
        let contract_id = env.register(crate::PredictifyHybrid, ());
        env.as_contract(&contract_id, || {
            // Minimum valid capacity.
            BoundedMonitorQueue::initialize(&env, MIN_QUEUE_CAPACITY).unwrap();
            assert_eq!(BoundedMonitorQueue::capacity(&env), 1);

            // Push and immediately pop to test single-slot queue.
            let ev = make_event(&env, "min", MonitorEventCategory::System);
            BoundedMonitorQueue::push(&env, &ev).unwrap();
            assert!(BoundedMonitorQueue::is_full(&env));

            let ev2 = make_event(&env, "min2", MonitorEventCategory::System);
            let overflowed = BoundedMonitorQueue::push(&env, &ev2).unwrap();
            assert!(overflowed);

            let popped = BoundedMonitorQueue::pop(&env).unwrap().unwrap();
            assert_eq!(popped.event_id, Symbol::new(&env, "min2"));
        });
    }
}
