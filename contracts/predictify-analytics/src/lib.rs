//! # predictify-analytics (placeholder)
//!
//! This crate was introduced as a separate deployable workspace member by
//! #1466, but its `src/` was never committed, which broke workspace
//! resolution for every `cargo` command in CI (the workspace includes
//! `contracts/*`). This placeholder lib keeps the workspace buildable and
//! the crate's CI job green until the real analytics contract sources are
//! landed.
