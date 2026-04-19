//! Panel dispatch — a closed enum identifying one of the rendered panels.
//!
//! Each variant corresponds to a state type under `panel::*`; the
//! [`Dashboard`](crate::Dashboard) holds one instance of each.

pub mod kpi;
pub mod line;
pub mod map;
pub mod rail;

/// Identifies one of the rendered panels on the dashboard.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Panel {
    /// KPI tile.
    Kpi,
    /// Map panel — choropleth + bubble overlay.
    Map,
    /// Right-rail Top-N bars.
    Rail,
    /// Time-series line.
    Line,
}

impl Panel {
    /// Every panel in display order.
    pub const ALL: [Self; 4] = [Self::Kpi, Self::Map, Self::Rail, Self::Line];

    /// Short heading shown in the card chrome.
    #[must_use]
    pub fn heading(self) -> &'static str {
        match self {
            Self::Kpi => "KPI",
            Self::Map => "Map",
            Self::Rail => "Top-N",
            Self::Line => "Time Series",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_lists_four_panels() {
        assert_eq!(Panel::ALL.len(), 4);
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
