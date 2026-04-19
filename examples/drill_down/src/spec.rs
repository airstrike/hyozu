//! Runtime configuration — the dashboard's bridge to a concrete cube's
//! vocabulary.
//!
//! Every name the dashboard reads flows through one [`DashboardSpec`]
//! value constructed in `main.rs`. The library surface never hard-codes
//! dimension, hierarchy, level, measure, or metric names; it consumes
//! indices into the cube's `Schema` that the spec provides.

use tatami::query::MemberRef;

use crate::panel;

/// Per-cube bindings supplied to [`crate::Dashboard::new`] at startup.
///
/// The caller (usually `main.rs`) resolves their cube's vocabulary — by
/// looking up dimension, hierarchy, level, measure, and metric names in the
/// `Schema` returned from `Cube::schema()` — into the indices carried by
/// the per-panel state values. The dashboard then operates entirely on
/// indices plus the schema it already holds a reference to.
#[derive(Debug, Clone)]
pub struct DashboardSpec {
    /// Window / header title.
    pub title: String,
    /// Members to pin on the first trail entry — pre-populates the query
    /// slicer so the initial view lands at a meaningful default (e.g. a
    /// single fiscal year + scenario).
    pub initial_slicer: Vec<MemberRef>,
    /// KPI panel initial bindings.
    pub kpi: panel::kpi::State,
    /// Map panel initial bindings.
    pub map: panel::map::State,
    /// Rail panel initial bindings.
    pub rail: panel::rail::State,
    /// Line panel initial bindings.
    pub line: panel::line::State,
}
