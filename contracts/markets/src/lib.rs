#![no_std]
// Markets package – thin crate that satisfies the Cargo workspace structure.
// All market logic lives in `predictify-hybrid`; this crate hosts focused
// integration-style tests (e.g. error-code stability checks) that require a
// separate compilation unit so they can be run with `cargo test -p markets`.
