//! `drill_down` — schema-blind multi-panel dashboard example.
//!
//! Library surface for the `drill_down` binary. Validates the
//! `tatami` OLAP cube + `hyozu::tatami` adapter end-to-end, with a UI
//! that discovers its dimensions, hierarchies, levels, and metrics via
//! `Cube::schema()` at runtime. Concrete names live only in the binary's
//! `main.rs` and the cube's own schema builder.

pub mod axis;
pub mod dashboard;
pub mod data;
pub mod metric;
pub mod panel;
pub mod query_state;
pub mod slicer;
pub mod spec;
pub mod trail;

pub use dashboard::Dashboard;
pub use panel::Panel;
pub use query_state::QueryState;
pub use spec::DashboardSpec;
pub use trail::Trail;
