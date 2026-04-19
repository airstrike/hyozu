//! UI layer — dashboard state, navigation trail, per-panel async state.
//!
//! Phase 1 ships the plumbing: [`trail::Trail`], [`view::View`],
//! [`panel::Panel`], [`query_state::QueryState`], [`dashboard::Dashboard`],
//! and [`queries`] builders. Real panel rendering lands in later phases.

pub mod dashboard;
pub mod panel;
pub mod queries;
pub mod query_state;
pub mod trail;
pub mod view;

pub use dashboard::Dashboard;
pub use panel::Panel;
pub use query_state::QueryState;
pub use trail::Trail;
pub use view::{Measure, Period, View};
