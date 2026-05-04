/// Visual annotation drawn at a hovered data point.
///
/// Each mark type produces a default variant when building tooltip
/// entries. The renderer matches exhaustively — adding a variant
/// forces handling at every consumer.
#[non_exhaustive]
pub(crate) enum Annotation {
    /// Filled circle + white ring. Default for Line, Area, Xy.
    PointMarker { pixel: crate::core::Point, radius: f32 },
    /// No visual at the data point. Default for Bars.
    None,
}

/// Hover state, split by chart geometry.
///
/// Cartesian marks (line/area/xy/bars) snap to a single data-x and
/// collect every series that has a point near it. Pie marks hit-test
/// a slice by polar angle, so they record the mark and slice index
/// directly.
#[non_exhaustive]
pub(crate) enum State {
    /// Hover over a Cartesian mark snapped to `data_x`.
    Cartesian {
        /// Snapped data-space x coordinate.
        data_x: f64,
        /// Matching entries: (mark_index, series_index, point_index).
        entries: Vec<(usize, usize, usize)>,
    },
    /// Hover over a single pie slice.
    Pie {
        /// Index of the `Mark::Pie` in the marks list.
        mark_idx: usize,
        /// Index of the slice within the pie.
        slice_idx: usize,
    },
}

/// A single tooltip entry ready for rendering.
pub(crate) struct Entry {
    pub tooltip: crate::data::tooltip::TooltipEntry,
    /// Anchor point for tooltip-box vertical centering.
    pub anchor: crate::core::Point,
    pub color: crate::core::Color,
    pub annotation: Annotation,
}
