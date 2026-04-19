//! UI layer — dashboard state, navigation trail, per-panel async state.
//!
//! The [`trail::Trail`] carries back/forward history; [`view::View`] carries
//! the active measure + period; [`panel::Panel`] enumerates the rendered
//! panels; [`query_state::QueryState`] tracks each panel's async state;
//! [`dashboard::Dashboard`] owns the cube handle plus geo/centroids and
//! dispatches per-panel `tatami::Query`s.

pub mod dashboard;
pub mod kpi;
pub mod map_panel;
pub mod panel;
pub mod queries;
pub mod query_state;
pub mod right_rail;
pub mod time_series;
pub mod trail;
pub mod view;

pub use dashboard::Dashboard;
pub use panel::Panel;
pub use query_state::QueryState;
pub use trail::Trail;
pub use view::{Measure, Period, View};
