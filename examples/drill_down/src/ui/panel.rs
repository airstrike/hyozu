//! Closed enum of rendered panels.
//!
//! Each variant owns its heading and knows how to build its own
//! `tatami::Query` from `(current trail query, view)`.

use crate::ui::queries;
use crate::ui::view::View;

/// One of the five rendered panels in the dashboard.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Panel {
    /// KPI strip — single scalar with delta.
    Kpi,
    /// Choropleth + centroid bubbles map.
    Map,
    /// Right-rail top-N bars + donut.
    Rail,
    /// Bottom-strip time series.
    Line,
    /// Sidebar filter chips.
    Chips,
}

impl Panel {
    /// All panels, in display order.
    pub const ALL: [Self; 5] = [Self::Kpi, Self::Map, Self::Rail, Self::Line, Self::Chips];

    /// Short human-readable heading — shown in the panel chrome.
    #[must_use]
    pub fn heading(self) -> &'static str {
        match self {
            Self::Kpi => "KPI",
            Self::Map => "Map",
            Self::Rail => "Top Properties",
            Self::Line => "Time Series",
            Self::Chips => "Filters",
        }
    }

    /// Build the `tatami::Query` this panel needs from the current trail
    /// query and view mode. Chips is view-invariant; the other four consume
    /// `view`.
    #[must_use]
    pub fn query(self, current: &tatami::Query, view: &View) -> tatami::Query {
        match self {
            Self::Kpi => queries::kpi(current, view),
            Self::Map => queries::map(current, view),
            Self::Rail => queries::rail(current, view),
            Self::Line => queries::line(current, view),
            Self::Chips => queries::chips(current),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_lists_five_panels() {
        assert_eq!(Panel::ALL.len(), 5);
    }

    #[test]
    fn headings_are_nonempty_and_distinct() {
        let headings: Vec<&str> = Panel::ALL.iter().map(|p| p.heading()).collect();
        assert!(headings.iter().all(|h| !h.is_empty()));
        let mut sorted = headings.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), headings.len());
    }
}
