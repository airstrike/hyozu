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

/// Hover state: which data-x is snapped and which entries matched.
pub(crate) struct State {
    /// Snapped data-space x coordinate.
    pub data_x: f64,
    /// Matching entries: (mark_index, series_index, point_index).
    pub entries: Vec<(usize, usize, usize)>,
}

/// A single tooltip entry ready for rendering.
pub(crate) struct Entry {
    pub tooltip: crate::data::tooltip::TooltipEntry,
    /// Anchor point for tooltip-box vertical centering.
    pub anchor: crate::core::Point,
    pub color: crate::core::Color,
    pub annotation: Annotation,
}
