//! Per-panel async query state.
//!
//! Each panel's latest `tatami::Query` is either in flight, completed
//! successfully with a `Results`, or completed with an error string.

/// Per-panel state of the most recent async query.
#[derive(Debug)]
#[non_exhaustive]
pub enum QueryState {
    /// Query in flight — no result yet.
    Running,
    /// Query completed successfully.
    Ok(tatami::Results),
    /// Query failed; string is the stringified backend error.
    Err(String),
}
