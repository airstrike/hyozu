/// Layout strategy for multiple bar series.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Layout {
    /// Bars are arranged side-by-side (default).
    #[default]
    Grouped,
    /// Bars are stacked cumulatively.
    Stacked,
    /// Bars are overlaid directly on top of each other.
    Overlaid,
}
