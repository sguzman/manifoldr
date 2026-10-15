//! Reusable Rust client and typed wire models for the Manifold Markets API.
//!
//! Network operations are async. Trade placement defaults to Manifold's `dryRun`
//! simulation unless explicitly passed `execute = true`.

pub mod api;
pub mod realtime;

#[cfg(test)]
#[path = "tests.rs"]
mod fixture_tests;
