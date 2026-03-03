//! Addressable chart elements for click detection and selection.

/// Identifies a specific element within a chart.
///
/// Used for hit-testing (what was clicked) and selection state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    /// The chart title.
    Title,
    /// A mark (series group) by index in the marks list.
    Mark(usize),
    /// A specific series within a mark.
    Series { mark: usize, series: usize },
    /// A specific data entry within a series.
    Entry {
        mark: usize,
        series: usize,
        index: usize,
    },
    /// The X axis.
    XAxis,
    /// The Y axis.
    YAxis,
    /// The legend area.
    Legend,
    /// A specific legend entry by index.
    LegendEntry(usize),
}
