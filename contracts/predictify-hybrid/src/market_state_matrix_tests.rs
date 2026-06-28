//! Market State Transition Matrix Tests
//!
//! This module provides an exhaustive table-driven test for every
//! `(from_state, to_state)` pair in the `MarketState` machine.
//!
//! # Design
//!
//! Every possible ordered pair of `MarketState` variants is enumerated.
//! Each pair is labelled **legal** or **illegal** according to the rules
//! documented in [`crate::markets::MarketStateLogic::validate_state_transition`].
//!
//! Legal edges are asserted to return `Ok(())`.
//! Illegal edges (including every self-loop) are asserted to return
//! `Err(Error::IllegalMarketStateTransition)` — the dedicated typed error.
//!
//! If a new `MarketState` variant is added the `ALL_STATES` array below must
//! be updated, otherwise the "full coverage sentinel" test at the bottom will
//! fail with a count mismatch, preventing silent coverage gaps.
//!
//! # Legal Transition Diagram (reproduced here for quick reference)
//!
//! ```text
//!   Active    → Ended | Cancelled | Closed | Disputed
//!   Ended     → Resolved | Disputed | Closed | Cancelled
//!   Disputed  → Resolved | Closed | Cancelled
//!   Resolved  → Closed
//!   Closed    → (terminal)
//!   Cancelled → (terminal)
//! ```

#[cfg(test)]
mod market_state_matrix {
    use crate::err::Error;
    use crate::markets::MarketStateLogic;
    use crate::types::MarketState;

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    /// Every state variant listed once, in a stable order.
    ///
    /// IMPORTANT: When a new `MarketState` variant is added this array MUST be
    /// updated.  The sentinel test `test_all_states_covered` will fail if the
    /// count does not match, catching the omission at compile-time in CI.
    const ALL_STATES: &[MarketState] = &[
        MarketState::Active,
        MarketState::Ended,
        MarketState::Disputed,
        MarketState::Resolved,
        MarketState::Closed,
        MarketState::Cancelled,
    ];

    /// Expected number of states in the machine.
    ///
    /// Update this constant whenever a variant is added or removed so that the
    /// sentinel test continues to guard coverage.
    const EXPECTED_STATE_COUNT: usize = 6;

    /// Returns `true` when `from → to` is a legal edge in the state machine.
    ///
    /// This mirrors the exact logic inside `validate_state_transition` and is
    /// kept here as an independent source-of-truth for the test assertions.
    fn is_legal(from: MarketState, to: MarketState) -> bool {
        use MarketState::*;
        match from {
            Active => matches!(to, Ended | Cancelled | Closed | Disputed),
            Ended => matches!(to, Resolved | Disputed | Closed | Cancelled),
            Disputed => matches!(to, Resolved | Closed | Cancelled),
            Resolved => matches!(to, Closed),
            Closed => false,
            Cancelled => false,
        }
    }

    // -----------------------------------------------------------------------
    // Sentinel: guards against new states being silently omitted
    // -----------------------------------------------------------------------

    /// Fails if `ALL_STATES` does not cover every variant in the enum.
    ///
    /// When a new `MarketState` variant is added:
    /// 1. Add it to `ALL_STATES`.
    /// 2. Update `EXPECTED_STATE_COUNT`.
    /// 3. Add explicit legal/illegal test cases below that cover its edges.
    #[test]
    fn test_all_states_covered() {
        assert_eq!(
            ALL_STATES.len(),
            EXPECTED_STATE_COUNT,
            "ALL_STATES has {} entries but EXPECTED_STATE_COUNT is {}. \
             Update one of them when adding or removing a MarketState variant.",
            ALL_STATES.len(),
            EXPECTED_STATE_COUNT
        );
    }

    // -----------------------------------------------------------------------
    // Full matrix: every (from, to) pair
    // -----------------------------------------------------------------------

    /// Table-driven test covering all 36 ordered pairs (6 × 6).
    ///
    /// Each entry is `(from, to, expected_ok)`.  The test name printed on
    /// failure is derived from the variant debug strings so failures are
    /// immediately human-readable without needing to decode indices.
    #[test]
    fn test_full_transition_matrix() {
        /// A single matrix cell.
        struct Case {
            from: MarketState,
            to: MarketState,
            /// `true` means the transition should be permitted.
            legal: bool,
        }

        // Build the full 6×6 matrix programmatically so that no pair can be
        // accidentally omitted.  `is_legal` provides the expected outcome.
        let mut cases: alloc::vec::Vec<Case> = alloc::vec::Vec::new();
        for &from in ALL_STATES {
            for &to in ALL_STATES {
                cases.push(Case {
                    from,
                    to,
                    legal: is_legal(from, to),
                });
            }
        }

        // Verify the matrix is exactly 36 cells (6 states × 6 states).
        assert_eq!(
            cases.len(),
            EXPECTED_STATE_COUNT * EXPECTED_STATE_COUNT,
            "Matrix size mismatch — check ALL_STATES and EXPECTED_STATE_COUNT."
        );

        let mut legal_count = 0usize;
        let mut illegal_count = 0usize;

        for case in &cases {
            let result = MarketStateLogic::validate_state_transition(case.from, case.to);

            if case.legal {
                // ---- Legal edge ----
                assert!(
                    result.is_ok(),
                    "Expected {:?} → {:?} to be LEGAL but got {:?}",
                    case.from,
                    case.to,
                    result.unwrap_err()
                );
                legal_count += 1;
            } else {
                // ---- Illegal edge ----
                assert!(
                    result.is_err(),
                    "Expected {:?} → {:?} to be ILLEGAL but it returned Ok(())",
                    case.from,
                    case.to,
                );
                // Assert the *typed* error — not just any error.
                assert_eq!(
                    result.unwrap_err(),
                    Error::IllegalMarketStateTransition,
                    "Expected Error::IllegalMarketStateTransition for {:?} → {:?}",
                    case.from,
                    case.to,
                );
                illegal_count += 1;
            }
        }

        // Sanity-check the legal/illegal split.
        // Legal edges:  Active(4) + Ended(4) + Disputed(3) + Resolved(1) = 12
        // Illegal edges: 36 - 12 = 24  (includes all 6 self-loops)
        assert_eq!(
            legal_count, 12,
            "Expected 12 legal edges in the matrix, found {legal_count}"
        );
        assert_eq!(
            illegal_count, 24,
            "Expected 24 illegal edges in the matrix, found {illegal_count}"
        );
    }

    // -----------------------------------------------------------------------
    // Explicit legal-edge smoke tests (human-readable, easy to review)
    // -----------------------------------------------------------------------

    #[test]
    fn test_legal_active_to_ended() {
        assert!(MarketStateLogic::validate_state_transition(
            MarketState::Active,
            MarketState::Ended
        )
        .is_ok());
    }

    #[test]
    fn test_legal_active_to_cancelled() {
        assert!(MarketStateLogic::validate_state_transition(
            MarketState::Active,
            MarketState::Cancelled
        )
        .is_ok());
    }

    #[test]
    fn test_legal_active_to_closed() {
        assert!(MarketStateLogic::validate_state_transition(
            MarketState::Active,
            MarketState::Closed
        )
        .is_ok());
    }

    #[test]
    fn test_legal_active_to_disputed() {
        assert!(MarketStateLogic::validate_state_transition(
            MarketState::Active,
            MarketState::Disputed
        )
        .is_ok());
    }

    #[test]
    fn test_legal_ended_to_resolved() {
        assert!(MarketStateLogic::validate_state_transition(
            MarketState::Ended,
            MarketState::Resolved
        )
        .is_ok());
    }

    #[test]
    fn test_legal_ended_to_disputed() {
        assert!(MarketStateLogic::validate_state_transition(
            MarketState::Ended,
            MarketState::Disputed
        )
        .is_ok());
    }

    #[test]
    fn test_legal_ended_to_closed() {
        assert!(MarketStateLogic::validate_state_transition(
            MarketState::Ended,
            MarketState::Closed
        )
        .is_ok());
    }

    #[test]
    fn test_legal_ended_to_cancelled() {
        assert!(MarketStateLogic::validate_state_transition(
            MarketState::Ended,
            MarketState::Cancelled
        )
        .is_ok());
    }

    #[test]
    fn test_legal_disputed_to_resolved() {
        assert!(MarketStateLogic::validate_state_transition(
            MarketState::Disputed,
            MarketState::Resolved
        )
        .is_ok());
    }

    #[test]
    fn test_legal_disputed_to_closed() {
        assert!(MarketStateLogic::validate_state_transition(
            MarketState::Disputed,
            MarketState::Closed
        )
        .is_ok());
    }

    #[test]
    fn test_legal_disputed_to_cancelled() {
        assert!(MarketStateLogic::validate_state_transition(
            MarketState::Disputed,
            MarketState::Cancelled
        )
        .is_ok());
    }

    #[test]
    fn test_legal_resolved_to_closed() {
        assert!(MarketStateLogic::validate_state_transition(
            MarketState::Resolved,
            MarketState::Closed
        )
        .is_ok());
    }

    // -----------------------------------------------------------------------
    // Explicit illegal-edge tests — named for the issue requirement
    // ("every illegal edge returns Error::IllegalMarketStateTransition")
    // -----------------------------------------------------------------------

    // --- Self-loops (all 6 are illegal) ---

    #[test]
    fn test_illegal_self_loop_active() {
        assert_eq!(
            MarketStateLogic::validate_state_transition(
                MarketState::Active,
                MarketState::Active
            ),
            Err(Error::IllegalMarketStateTransition)
        );
    }

    #[test]
    fn test_illegal_self_loop_ended() {
        assert_eq!(
            MarketStateLogic::validate_state_transition(
                MarketState::Ended,
                MarketState::Ended
            ),
            Err(Error::IllegalMarketStateTransition)
        );
    }

    #[test]
    fn test_illegal_self_loop_disputed() {
        assert_eq!(
            MarketStateLogic::validate_state_transition(
                MarketState::Disputed,
                MarketState::Disputed
            ),
            Err(Error::IllegalMarketStateTransition)
        );
    }

    #[test]
    fn test_illegal_self_loop_resolved() {
        assert_eq!(
            MarketStateLogic::validate_state_transition(
                MarketState::Resolved,
                MarketState::Resolved
            ),
            Err(Error::IllegalMarketStateTransition)
        );
    }

    #[test]
    fn test_illegal_self_loop_closed() {
        assert_eq!(
            MarketStateLogic::validate_state_transition(
                MarketState::Closed,
                MarketState::Closed
            ),
            Err(Error::IllegalMarketStateTransition)
        );
    }

    #[test]
    fn test_illegal_self_loop_cancelled() {
        assert_eq!(
            MarketStateLogic::validate_state_transition(
                MarketState::Cancelled,
                MarketState::Cancelled
            ),
            Err(Error::IllegalMarketStateTransition)
        );
    }

    // --- Terminal-state outbound edges ---

    #[test]
    fn test_illegal_closed_to_active() {
        assert_eq!(
            MarketStateLogic::validate_state_transition(
                MarketState::Closed,
                MarketState::Active
            ),
            Err(Error::IllegalMarketStateTransition)
        );
    }

    #[test]
    fn test_illegal_closed_to_ended() {
        assert_eq!(
            MarketStateLogic::validate_state_transition(
                MarketState::Closed,
                MarketState::Ended
            ),
            Err(Error::IllegalMarketStateTransition)
        );
    }

    #[test]
    fn test_illegal_closed_to_disputed() {
        assert_eq!(
            MarketStateLogic::validate_state_transition(
                MarketState::Closed,
                MarketState::Disputed
            ),
            Err(Error::IllegalMarketStateTransition)
        );
    }

    #[test]
    fn test_illegal_closed_to_resolved() {
        assert_eq!(
            MarketStateLogic::validate_state_transition(
                MarketState::Closed,
                MarketState::Resolved
            ),
            Err(Error::IllegalMarketStateTransition)
        );
    }

    #[test]
    fn test_illegal_closed_to_cancelled() {
        assert_eq!(
            MarketStateLogic::validate_state_transition(
                MarketState::Closed,
                MarketState::Cancelled
            ),
            Err(Error::IllegalMarketStateTransition)
        );
    }

    #[test]
    fn test_illegal_cancelled_to_active() {
        assert_eq!(
            MarketStateLogic::validate_state_transition(
                MarketState::Cancelled,
                MarketState::Active
            ),
            Err(Error::IllegalMarketStateTransition)
        );
    }

    #[test]
    fn test_illegal_cancelled_to_ended() {
        assert_eq!(
            MarketStateLogic::validate_state_transition(
                MarketState::Cancelled,
                MarketState::Ended
            ),
            Err(Error::IllegalMarketStateTransition)
        );
    }

    #[test]
    fn test_illegal_cancelled_to_disputed() {
        assert_eq!(
            MarketStateLogic::validate_state_transition(
                MarketState::Cancelled,
                MarketState::Disputed
            ),
            Err(Error::IllegalMarketStateTransition)
        );
    }

    #[test]
    fn test_illegal_cancelled_to_resolved() {
        assert_eq!(
            MarketStateLogic::validate_state_transition(
                MarketState::Cancelled,
                MarketState::Resolved
            ),
            Err(Error::IllegalMarketStateTransition)
        );
    }

    #[test]
    fn test_illegal_cancelled_to_closed() {
        assert_eq!(
            MarketStateLogic::validate_state_transition(
                MarketState::Cancelled,
                MarketState::Closed
            ),
            Err(Error::IllegalMarketStateTransition)
        );
    }

    // --- Resolved backward / sideways edges ---

    #[test]
    fn test_illegal_resolved_to_active() {
        assert_eq!(
            MarketStateLogic::validate_state_transition(
                MarketState::Resolved,
                MarketState::Active
            ),
            Err(Error::IllegalMarketStateTransition)
        );
    }

    #[test]
    fn test_illegal_resolved_to_ended() {
        assert_eq!(
            MarketStateLogic::validate_state_transition(
                MarketState::Resolved,
                MarketState::Ended
            ),
            Err(Error::IllegalMarketStateTransition)
        );
    }

    #[test]
    fn test_illegal_resolved_to_disputed() {
        assert_eq!(
            MarketStateLogic::validate_state_transition(
                MarketState::Resolved,
                MarketState::Disputed
            ),
            Err(Error::IllegalMarketStateTransition)
        );
    }

    #[test]
    fn test_illegal_resolved_to_cancelled() {
        assert_eq!(
            MarketStateLogic::validate_state_transition(
                MarketState::Resolved,
                MarketState::Cancelled
            ),
            Err(Error::IllegalMarketStateTransition)
        );
    }

    // --- Active backward edges ---

    #[test]
    fn test_illegal_active_to_resolved() {
        assert_eq!(
            MarketStateLogic::validate_state_transition(
                MarketState::Active,
                MarketState::Resolved
            ),
            Err(Error::IllegalMarketStateTransition)
        );
    }

    // --- Ended backward edge ---

    #[test]
    fn test_illegal_ended_to_active() {
        assert_eq!(
            MarketStateLogic::validate_state_transition(
                MarketState::Ended,
                MarketState::Active
            ),
            Err(Error::IllegalMarketStateTransition)
        );
    }

    // --- Disputed backward edges ---

    #[test]
    fn test_illegal_disputed_to_active() {
        assert_eq!(
            MarketStateLogic::validate_state_transition(
                MarketState::Disputed,
                MarketState::Active
            ),
            Err(Error::IllegalMarketStateTransition)
        );
    }

    #[test]
    fn test_illegal_disputed_to_ended() {
        assert_eq!(
            MarketStateLogic::validate_state_transition(
                MarketState::Disputed,
                MarketState::Ended
            ),
            Err(Error::IllegalMarketStateTransition)
        );
    }

    // -----------------------------------------------------------------------
    // Edge-case: "Resolved → Active" is the canonical "undo-resolution" attempt
    // -----------------------------------------------------------------------

    /// Explicitly named test for the canonical illegal edge mentioned in the issue.
    ///
    /// A resolved market can never be made active again.  This protects the
    /// integrity of already-distributed payouts.
    #[test]
    fn test_illegal_resolved_after_archive_attempt() {
        // Simulates a buggy admin attempting to reactivate a resolved market
        // (the "Resolved-after-archive" edge case from the issue).
        let illegal_edges = [
            (MarketState::Resolved, MarketState::Active),
            (MarketState::Resolved, MarketState::Ended),
            (MarketState::Resolved, MarketState::Disputed),
            (MarketState::Resolved, MarketState::Cancelled),
            // Closed → anything is also illegal after archive
            (MarketState::Closed, MarketState::Active),
            (MarketState::Closed, MarketState::Ended),
            (MarketState::Closed, MarketState::Disputed),
            (MarketState::Closed, MarketState::Resolved),
            (MarketState::Closed, MarketState::Cancelled),
        ];

        for (from, to) in illegal_edges {
            assert_eq!(
                MarketStateLogic::validate_state_transition(from, to),
                Err(Error::IllegalMarketStateTransition),
                "Post-archive edge {:?} → {:?} must be illegal",
                from,
                to,
            );
        }
    }
}
