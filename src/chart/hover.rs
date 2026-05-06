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
    /// Stroke around a polygon's projected rings. Used by the choropleth
    /// hover overlay to outline the hovered feature. The rings are
    /// shared with the chart's geometry cache via [`std::sync::Arc`] so
    /// constructing this highlight on a hover change is O(1) — the
    /// underlying `Vec<Vec<...>>` is not cloned.
    Stroke {
        rings: std::sync::Arc<Vec<Vec<(f32, f32)>>>,
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
/// - `ChoroplethArea` — geo-projected polygon marks (Choropleth).
///   Hit-tested by point-in-polygon over the cached projected rings.
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
    /// Hover over a choropleth polygon area.
    ChoroplethArea {
        /// Index of the `Mark::Choropleth` in the marks list.
        mark_idx: usize,
        /// Index of the feature within `Plane::projected_polygons` /
        /// `Plane::filtered_ids`.
        feature_idx: usize,
    },
}

/// A single row in the tooltip box, fully resolved and ready for the
/// canvas-drawn renderer to lay out (swatch, text, position).
///
/// One [`Row`] per visible entry — cartesian hovers may produce
/// many (one per series at the snapped data-x), pie / geo / area
/// hovers produce one.
pub(crate) struct Row {
    pub tooltip: crate::data::tooltip::TooltipEntry,
    /// Anchor point for tooltip-box vertical centering.
    pub anchor: crate::core::Point,
    pub color: crate::core::Color,
    pub highlight: Highlight,
}

// ──────────────────────────────────────────────────────────────────
// Public hover API: closure input/output types consumed by the
// chart's overlay rendering. Wired in by the upcoming overlay
// refactor; the types live here so callers can name them.
// ──────────────────────────────────────────────────────────────────

/// What the user's hover closure receives — the resolved data and
/// metadata for the mark currently under the cursor.
///
/// Variants mirror [`Geometry`]: one per hit-test shape, not one
/// per mark type. A single closure can match across cartesian, pie,
/// geographic, and choropleth marks on the same chart.
///
/// Theme-resolved mark colors are intentionally **not** exposed
/// here — they live on a draw-time cache that isn't reachable from
/// the overlay-construction path. Closures that want a colored
/// swatch should drive it from their own palette / theme, or use
/// `iced::widget::container::Style::styled` to pull it from the
/// theme directly.
#[non_exhaustive]
pub enum Entry<'a> {
    /// Hover over a Cartesian mark (Line / Area / Xy / Bars).
    Cartesian {
        mark_idx: usize,
        series_idx: usize,
        point_idx: usize,
        x: f64,
        y: f64,
        series_name: Option<&'a str>,
    },
    /// Hover over a Pie / Donut slice.
    Pie {
        mark_idx: usize,
        slice_idx: usize,
        label: Option<&'a str>,
        value: f64,
    },
    /// Hover over a geo-projected point mark (geo-Xy bubble).
    Geographic {
        mark_idx: usize,
        point_idx: usize,
        label: Option<&'a str>,
        value: f64,
    },
    /// Hover over a choropleth polygon area.
    Choropleth {
        mark_idx: usize,
        feature_idx: usize,
        feature_id: &'a str,
        feature_name: &'a str,
        properties: &'a std::collections::HashMap<String, String>,
        /// `None` when the feature has an "available, no value"
        /// entry, or no entry at all.
        value: Option<f64>,
    },
}

/// Where the annotation positions itself relative to its anchor.
///
/// `FollowCursor` matches the existing tooltip-box behavior — the
/// box anchors near the cursor and flips horizontally across the
/// chart's mid-x to stay inside the viewport. The fixed sides
/// (`Top` / `Bottom` / `Left` / `Right`) anchor relative to the
/// hovered mark, which is what most static-position tooltips want.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Position {
    Top,
    Bottom,
    Left,
    Right,
    #[default]
    FollowCursor,
}

/// A floating annotation built from an iced [`Element`] plus
/// positioning chrome. Returned by the user's hover closure
/// (`Chart::hover`) and rendered by the chart's overlay above the
/// chart at hover time.
///
/// Trivial cases use the [`From<Element>`] impl:
/// `column![text("hello")].into()`. Chrome customization uses the
/// builder methods.
///
/// [`Element`]: crate::core::Element
pub struct Annotation<'a, Message, Theme = crate::core::Theme> {
    pub(crate) content: crate::core::Element<'a, Message, Theme, crate::widget::Renderer>,
    pub(crate) position: Position,
    pub(crate) padding: crate::core::Padding,
    pub(crate) gap: f32,
    pub(crate) snap_within_viewport: bool,
}

impl<'a, Message, Theme> Annotation<'a, Message, Theme> {
    /// Wraps `content` in an annotation with default chrome
    /// (cursor-following, 8px padding, 8px gap, viewport-snapped).
    pub fn new(content: impl Into<crate::core::Element<'a, Message, Theme, crate::widget::Renderer>>) -> Self {
        Self {
            content: content.into(),
            position: Position::default(),
            padding: crate::core::Padding::new(8.0),
            gap: 8.0,
            snap_within_viewport: true,
        }
    }

    /// Where the annotation sits relative to its anchor. Default
    /// [`Position::FollowCursor`].
    pub fn position(mut self, position: Position) -> Self {
        self.position = position;
        self
    }

    /// Inner padding around `content` inside the annotation's
    /// background container. Default 8px on all sides.
    pub fn padding(mut self, padding: impl Into<crate::core::Padding>) -> Self {
        self.padding = padding.into();
        self
    }

    /// Pixel gap between the anchor and the annotation's nearest
    /// edge. Default 8.0.
    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }

    /// Whether the annotation clamps within the chart's viewport
    /// rather than overflowing past its edges. Default `true`.
    pub fn snap_within_viewport(mut self, snap: bool) -> Self {
        self.snap_within_viewport = snap;
        self
    }
}

impl<'a, Message, Theme> From<crate::core::Element<'a, Message, Theme, crate::widget::Renderer>>
    for Annotation<'a, Message, Theme>
{
    fn from(element: crate::core::Element<'a, Message, Theme, crate::widget::Renderer>) -> Self {
        Annotation::new(element)
    }
}

/// Boxed hover annotation closure — the type [`crate::Chart::hover`]
/// stores once installed.
///
/// The inner [`Entry`] is HRTB-bound (`for<'b>`) because its
/// borrowed metadata (feature names, properties, etc.) lives in
/// the chart's widget-tree state, not in the user's `&'a Data`
/// borrow. Closure callers `.to_string()` any short-lived strings
/// they want to embed in their owned [`Annotation`].
pub(crate) type HoverFn<'a, Message, Theme> = Box<dyn for<'b> Fn(&Entry<'b>) -> Annotation<'a, Message, Theme> + 'a>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::Theme;
    use crate::widget::text;

    #[test]
    fn annotation_builder_round_trip() {
        let a: Annotation<'_, (), Theme> = Annotation::new(text("hello"))
            .position(Position::Top)
            .padding(crate::core::Padding::new(4.0))
            .gap(2.0)
            .snap_within_viewport(false);
        assert_eq!(a.position, Position::Top);
        assert_eq!(a.padding, crate::core::Padding::new(4.0));
        assert_eq!(a.gap, 2.0);
        assert!(!a.snap_within_viewport);
    }

    #[test]
    fn annotation_default_chrome() {
        let a: Annotation<'_, (), Theme> = Annotation::new(text("x"));
        assert_eq!(a.position, Position::FollowCursor);
        assert_eq!(a.padding, crate::core::Padding::new(8.0));
        assert_eq!(a.gap, 8.0);
        assert!(a.snap_within_viewport);
    }

    #[test]
    fn annotation_from_element() {
        let element: crate::core::Element<'_, (), Theme, crate::widget::Renderer> = text("x").into();
        let a: Annotation<'_, (), Theme> = element.into();
        assert_eq!(a.position, Position::FollowCursor);
    }
}
