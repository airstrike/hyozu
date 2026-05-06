/// Visual highlight drawn at a hovered data point.
///
/// Each mark type produces a default variant when building tooltip
/// entries. The renderer matches exhaustively — adding a variant
/// forces handling at every consumer.
#[non_exhaustive]
pub(crate) enum Highlight {
    /// Filled circle + white ring. Default for Line, Area, Xy.
    PointMarker { pixel: crate::core::Point, radius: f32 },
    /// No visual at the data point. Default for Bars.
    None,
    /// Stroke ring at `pixel` with the given radius/color/width. Used by
    /// the geographic hover overlay to outline the hovered bubble.
    Ring {
        pixel: crate::core::Point,
        radius: f32,
        color: crate::core::Color,
        width: f32,
    },
}

/// Hit-test geometry under the cursor.
///
/// **Variants are hit-test geometries, not mark identities.** A single
/// variant covers every mark that shares the same hit-test shape:
///
/// - `Cartesian` — Line, Area, Xy(cartesian), Bars, and any future mark
///   that snaps to a single data-x and collects per-series points there.
/// - `Pie` — polar slices (Pie / Donut). Hit-tested by `(inner_radius,
///   outer_radius)` + slice angle range.
/// - `Geographic` — geo-projected point marks (geo-Xy bubbles, future
///   geo-aware point overlays). Hit-tested by Euclidean proximity to
///   the projected pixel center.
///
/// Future readers should NOT try to make the variant set 1:1 with
/// [`crate::Mark`]. Several marks legitimately share one variant.
#[non_exhaustive]
#[derive(PartialEq)]
pub(crate) enum Geometry {
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
    /// Hover near a single geo-projected point (geo-Xy bubble).
    Geographic {
        /// Index of the `Mark::Xy` (with `coord_kind == Geo`) in the marks list.
        mark_idx: usize,
        /// Index of the point within `state.pixel_points`.
        point_idx: usize,
    },
}

/// A single tooltip entry ready for rendering.
pub(crate) struct Entry {
    pub tooltip: crate::data::tooltip::TooltipEntry,
    /// Anchor point for tooltip-box vertical centering.
    pub anchor: crate::core::Point,
    pub color: crate::core::Color,
    pub annotation: Highlight,
}
