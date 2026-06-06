pub mod donut;
pub mod guide;
pub mod hover;
pub mod legend;
pub mod plot_area;
pub mod scale_legend;
mod scene;
pub mod title;
pub mod value;

use std::borrow::Cow;

use crate::animation;
use crate::core::time::Instant;
use crate::core::widget::{Tree, tree};
use crate::core::{
    Element, Event, Layout, Length, Padding, Point, Rectangle, Shell, Size, Vector, Widget, layout, mouse, overlay,
    window,
};
use crate::data::tooltip::TooltipEntry;
use crate::widget::Renderer;

use crate::{Action, Data, design};
use scene::Scene;

const DEFAULT_PADDING: Padding = Padding::new(10.0);

/// Pixels to expand each legend entry's cached rect by when hit-testing,
/// so small clickable swatches are forgiving to hover/click at the edges.
const LEGEND_HIT_PADDING: f32 = 2.0;

/// Chart widget for displaying data visualizations.
pub struct Chart<'a, Message, Design = crate::core::Theme, Theme = crate::core::Theme>
where
    Design: design::Design + Clone,
{
    scene: Scene<'a, Message, Renderer>,
    generation: u64,
    padding: Padding,
    width: Length,
    height: Length,
    on_action: Option<Box<dyn Fn(Action) -> Message + 'a>>,
    design: Option<Cow<'a, Design>>,
    style: StyleFn<'a>,
    kind: Kind<'a, Message, Theme>,
    /// Mirror of [`Data::animate`] captured at construction. Gates the
    /// animation tick and the data-change snapshot so opted-out charts
    /// don't keep requesting redraws.
    animate: bool,
    /// Optional user-supplied hover annotation closure. When set, the
    /// chart calls it with a [`hover::Entry`] resolved from the
    /// hovered mark and renders the returned [`hover::Annotation`]
    /// as an overlay above the chart. When unset, the chart falls
    /// back to its built-in canvas-drawn tooltip box.
    hover_fn: Option<hover::HoverFn<'a, Message, Theme>>,
    _theme: std::marker::PhantomData<Theme>,
}

/// Variant tag for [`Chart`], carrying any variant-specific overlay state.
///
/// `chart()` constructs `Kind::Generic`; `donut()` constructs
/// `Kind::Donut(donut::Chart { .. })` so the donut variant can host a
/// center overlay element without forcing every chart to pay for it.
#[non_exhaustive]
pub enum Kind<'a, Message, Theme> {
    /// No variant-specific overlay; the chart renders only its scene.
    Generic,
    /// Donut variant with optional center overlay; see [`donut::Chart`].
    Donut(donut::Chart<'a, Message, Theme>),
}

/// Internal state for the chart widget.
struct State {
    generation: u64,
    is_pressed: bool,
    hover: Option<hover::Geometry>,
    /// Last cursor position observed inside the chart's bounds, in
    /// absolute screen coordinates. Updated alongside [`Self::hover`]
    /// during cursor-move handling and consumed by the annotation
    /// overlay for cursor-anchored positioning.
    last_cursor: Option<Point>,
    /// Persisted widget tree for the annotation overlay's inner
    /// element. iced's runtime calls [`overlay::Overlay::layout`] in
    /// one pass and [`overlay::Overlay::draw`] in another, with a
    /// fresh overlay instance for each — so the inner element's
    /// per-widget state (text paragraphs, etc.) must live somewhere
    /// stable across both. Reconciled via [`Tree::diff`] each frame
    /// so widget identity carries forward when the user's closure
    /// produces structurally-similar elements.
    annotation_tree: Tree,
    /// Names of series hidden via legend click toggles. Lives with the
    /// widget state and is reset when the tree is dropped.
    hidden_series: std::collections::HashSet<String>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            generation: 0,
            is_pressed: false,
            hover: None,
            last_cursor: None,
            annotation_tree: Tree::empty(),
            hidden_series: std::collections::HashSet::new(),
        }
    }
}

/// Creates a chart widget from data.
///
/// The Theme type is inferred from context (e.g., the Element type in your view).
/// Call `.design()` to use a custom design type separate from your Theme.
pub fn chart<'a, Message, Theme>(data: &'a Data) -> Chart<'a, Message, Theme, Theme>
where
    Theme: design::Design + Clone,
{
    Chart::new(data)
}

/// Creates a donut-variant chart from data.
///
/// Equivalent to [`chart`] with the donut variant pre-selected so
/// donut-only knobs (`.hole`, `.center`, `.center_inset`) take effect.
/// The default hole radius is `0.5`; pass `.hole(0.0)` for a full pie
/// or any other proportion in roughly `0.0..1.0`.
pub fn donut<'a, Message, Theme>(data: &'a Data) -> Chart<'a, Message, Theme, Theme>
where
    Theme: design::Design + Clone,
{
    let mut chart = Chart::new(data);
    chart.kind = Kind::Donut(donut::Chart::new());
    chart
}

impl<'a, Message, Design, Theme> Chart<'a, Message, Design, Theme>
where
    Design: design::Design + Clone,
{
    /// Creates a new chart widget from data.
    ///
    /// Uses the application's theme for styling by default.
    pub fn new(data: &'a Data) -> Self {
        let scene = Scene::new(data);

        Chart {
            scene,
            generation: data.generation,
            padding: DEFAULT_PADDING,
            width: Length::Fill,
            height: Length::Fill,
            on_action: None,
            design: None,
            style: Box::new(default),
            kind: Kind::Generic,
            animate: data.animate,
            hover_fn: None,
            _theme: std::marker::PhantomData,
        }
    }

    /// Sets the width of the chart.
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// Sets the height of the chart.
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }

    /// Sets the [`Padding`] of the chart.
    pub fn padding(mut self, padding: impl Into<Padding>) -> Self {
        self.padding = padding.into();
        self
    }

    /// Sets a custom design for this chart, changing the Design type.
    ///
    /// Accepts either a reference or an owned value.
    /// If not set, the chart will use the application's theme.
    pub fn design<D: design::Design + Clone + 'a>(
        self,
        design: impl design::IntoDesign<'a, D>,
    ) -> Chart<'a, Message, D, Theme> {
        Chart {
            scene: self.scene,
            generation: self.generation,
            padding: self.padding,
            width: self.width,
            height: self.height,
            on_action: self.on_action,
            design: Some(design.into_design()),
            style: self.style,
            kind: self.kind,
            animate: self.animate,
            hover_fn: self.hover_fn,
            _theme: std::marker::PhantomData,
        }
    }

    /// Sets the action handler for the chart.
    pub fn on_action(mut self, f: impl Fn(Action) -> Message + 'a) -> Self {
        self.on_action = Some(Box::new(f));
        self
    }

    /// Installs a hover annotation closure.
    ///
    /// The closure runs each time the cursor enters a new mark and
    /// produces an [`hover::Annotation`] — an iced [`Element`] plus
    /// chrome (position, padding, gap, viewport snapping) — that the
    /// chart renders as an overlay above the hovered mark.
    ///
    /// Match on [`hover::Entry`] to dispatch by mark shape:
    /// `Cartesian` (line/area/xy/bars), `Pie`, `Geographic`
    /// (geo-Xy bubbles), `Choropleth` (polygon areas).
    ///
    /// When unset, the chart falls back to its built-in tooltip box
    /// (resolved through [`Data::tooltip`] formatter).
    ///
    /// [`Element`]: crate::core::Element
    /// [`Data::tooltip`]: crate::Data::tooltip
    pub fn hover<F>(mut self, f: F) -> Self
    where
        F: for<'b> Fn(&hover::Entry<'b>) -> hover::Annotation<'a, Message, Theme> + 'a,
    {
        self.hover_fn = Some(Box::new(f));
        self
    }

    /// Sets the style of the [`Chart`].
    pub fn style(mut self, style: impl Fn(&dyn design::Design) -> Style + 'a) -> Self {
        self.style = Box::new(style);
        self
    }

    /// Sets the inner hole radius for a donut chart, as a proportion
    /// of the outer radius.
    ///
    /// No-op on a generic chart created via [`chart()`]. The default
    /// for `donut(...)` is `0.5`. Out-of-range values pass through to
    /// the renderer as-is — clamping is the renderer's call.
    pub fn hole(mut self, hole: f32) -> Self {
        if let Kind::Donut(d) = &mut self.kind {
            d.hole = hole;
        }
        self
    }

    /// Sets a center overlay element on a donut chart, sized to the
    /// full plot area (`Placement::Stack`).
    ///
    /// No-op on a generic chart created via [`chart()`]. The overlay
    /// composites above the donut so its content (e.g. a card with a
    /// big number and label) isn't clipped by the hole.
    pub fn center(mut self, element: impl Into<Element<'a, Message, Theme, Renderer>>) -> Self {
        if let Kind::Donut(d) = &mut self.kind {
            d.center = Some(donut::Center::new(element.into(), donut::Placement::Stack));
        }
        self
    }

    /// Sets a center overlay element on a donut chart, inscribed in
    /// the donut hole (`Placement::Inset`).
    ///
    /// No-op on a generic chart created via [`chart()`]. The element
    /// is sized to the largest axis-aligned square that fits inside
    /// the hole, so circular badges and tight numbers stay readable.
    pub fn center_inset(mut self, element: impl Into<Element<'a, Message, Theme, Renderer>>) -> Self {
        if let Kind::Donut(d) = &mut self.kind {
            d.center = Some(donut::Center::new(element.into(), donut::Placement::Inset));
        }
        self
    }
}

/// Compute the absolute plot bounds from chart bounds, padding, and the content rect.
fn compute_plot_bounds(
    chart_bounds: Rectangle,
    padding: Padding,
    plot_area_offset: Point,
    content_rect: Rectangle,
) -> Rectangle {
    Rectangle {
        x: chart_bounds.x + padding.left + plot_area_offset.x + content_rect.x,
        y: chart_bounds.y + padding.top + plot_area_offset.y + content_rect.y,
        width: content_rect.width,
        height: content_rect.height,
    }
}

/// Scan the plot area tree for the cursor's hover target.
///
/// Tighter geometries take priority over looser ones. Pie slices are
/// the tightest (full polar bounds). Geo bubbles are next (Euclidean
/// proximity to a projected point). The Cartesian "nearest pixel-x"
/// snap is the fallback because it accepts any cursor inside its
/// 30px x-snap threshold and would otherwise swallow precise hits.
fn find_nearest_hover<Message>(
    local: Point,
    plot_area_tree: &Tree,
    plot_area: &plot_area::PlotArea<'_, Message, Renderer>,
    domain: &plot_area::Domain,
    content_rect: Rectangle,
    geo_plane: Option<&plot_area::geo::Plane>,
) -> Option<hover::Geometry> {
    if let Some(pie_hover) = find_pie_hover(local, plot_area_tree) {
        return Some(pie_hover);
    }
    if let Some(geo_hover) = find_geo_hover(local, plot_area_tree, plot_area) {
        return Some(geo_hover);
    }
    if let Some(geo_plane) = geo_plane
        && let Some(choropleth_hover) = find_choropleth_hover(local, plot_area, geo_plane)
    {
        return Some(choropleth_hover);
    }
    find_nearest_cartesian_hover(local, plot_area_tree, plot_area, domain, content_rect)
}

/// Scan Cartesian (line/area/xy/bars) marks for the nearest pixel x,
/// then collect all entries at that x. Returns `None` if no hoverable
/// points exist or the cursor is too far from any point.
///
/// Geo-coord Xy series are excluded — their `pixel_points` are projected
/// `(lon, lat)` positions, not cartesian data, so snap-to-x against them
/// would produce nonsensical hover state. Those bubbles get hit-tested
/// by [`find_geo_hover`] before this function runs.
fn find_nearest_cartesian_hover<Message>(
    local: Point,
    plot_area_tree: &Tree,
    plot_area: &plot_area::PlotArea<'_, Message, Renderer>,
    domain: &plot_area::Domain,
    content_rect: Rectangle,
) -> Option<hover::Geometry> {
    let line_tag = tree::Tag::of::<plot_area::line::State<<Renderer as crate::core::text::Renderer>::Paragraph>>();
    let area_tag = tree::Tag::of::<plot_area::area::State<<Renderer as crate::core::text::Renderer>::Paragraph>>();
    let xy_tag = tree::Tag::of::<plot_area::xy::State>();
    let bars_tag = tree::Tag::of::<plot_area::bars::State<<Renderer as crate::core::text::Renderer>::Paragraph>>();

    let is_geo_xy = |mark_idx: usize| {
        matches!(
            plot_area.series.get(mark_idx),
            Some(plot_area::Series::Xy(xy)) if xy.data.coord_kind == crate::mark::xy::CoordKind::Geo
        )
    };

    // Horizontal bar charts invert the axes: categories run along Y and bars
    // extend along X. Hover then snaps along Y (the category axis) instead of
    // X, and only bars participate — continuous marks (line/area/xy) are
    // inherently X-major and don't appear in a horizontal-bar chart.
    let horizontal = plot_area.series.iter().any(
        |s| matches!(s, plot_area::Series::Bars(b) if b.data.direction == crate::mark::bar::Direction::Horizontal),
    );
    let cursor_primary = if horizontal { local.y } else { local.x };

    // A bar's coordinate along the category (primary) axis: center-x for
    // vertical columns, center-y for horizontal bars.
    let bar_primary = |rect: &Rectangle| {
        if horizontal {
            rect.y + rect.height / 2.0
        } else {
            rect.x + rect.width / 2.0
        }
    };
    let rect_finite = |rect: &Rectangle| {
        rect.x.is_finite() && rect.y.is_finite() && rect.width.is_finite() && rect.height.is_finite()
    };

    let mut best_dist = f32::INFINITY;
    let mut best_primary = 0.0f32;

    // Pass 1: find the globally nearest primary-axis coordinate. Treat
    // non-finite pixel points as gaps (NaN-as-gap policy) so a
    // `(finite_x, NaN_y)` datum can't become a hover target and feed a NaN
    // center into the tessellator.
    for (mark_idx, child) in plot_area_tree.children.iter().enumerate() {
        if !horizontal && child.tag == line_tag {
            let s = child
                .state
                .downcast_ref::<plot_area::line::State<<Renderer as crate::core::text::Renderer>::Paragraph>>();
            for pt in &s.pixel_points {
                if !pt.x.is_finite() || !pt.y.is_finite() {
                    continue;
                }
                let d = (pt.x - cursor_primary).abs();
                if d < best_dist {
                    best_dist = d;
                    best_primary = pt.x;
                }
            }
        } else if !horizontal && child.tag == area_tag {
            let s = child
                .state
                .downcast_ref::<plot_area::area::State<<Renderer as crate::core::text::Renderer>::Paragraph>>();
            for sub in &s.series_points {
                for pt in sub {
                    if !pt.x.is_finite() || !pt.y.is_finite() {
                        continue;
                    }
                    let d = (pt.x - cursor_primary).abs();
                    if d < best_dist {
                        best_dist = d;
                        best_primary = pt.x;
                    }
                }
            }
        } else if !horizontal && child.tag == xy_tag {
            if is_geo_xy(mark_idx) {
                continue;
            }
            let s = child.state.downcast_ref::<plot_area::xy::State>();
            for pt in &s.pixel_points {
                if !pt.x.is_finite() || !pt.y.is_finite() {
                    continue;
                }
                let d = (pt.x - cursor_primary).abs();
                if d < best_dist {
                    best_dist = d;
                    best_primary = pt.x;
                }
            }
        } else if child.tag == bars_tag {
            let s = child
                .state
                .downcast_ref::<plot_area::bars::State<<Renderer as crate::core::text::Renderer>::Paragraph>>();
            for rects in &s.series_rects {
                for rect in rects {
                    if !rect_finite(rect) {
                        continue;
                    }
                    let primary = bar_primary(rect);
                    let d = (primary - cursor_primary).abs();
                    if d < best_dist {
                        best_dist = d;
                        best_primary = primary;
                    }
                }
            }
        }
    }

    if best_dist == f32::INFINITY {
        return None;
    }

    // Snap threshold: 30px max distance
    if best_dist > 30.0 {
        return None;
    }

    // `data_x` drives the tracking line (continuous marks only) and hover-
    // change detection. For horizontal bars the snap is along Y, so there is
    // no meaningful data-x; entries discriminate the hovered row.
    let data_x = if horizontal {
        0.0
    } else {
        plot_area::to_data_x(domain, content_rect, best_primary)
    };

    // Pass 2: collect all entries at that primary coordinate (within tolerance).
    let tolerance = 0.5f32;
    // Bars need a wider tolerance because a bar's center can differ from a
    // line's pixel coordinate.
    let bar_tolerance = 4.0f32;
    let mut entries = Vec::new();

    for (mark_idx, child) in plot_area_tree.children.iter().enumerate() {
        if !horizontal && child.tag == line_tag {
            let s = child
                .state
                .downcast_ref::<plot_area::line::State<<Renderer as crate::core::text::Renderer>::Paragraph>>();
            for (pt_idx, pt) in s.pixel_points.iter().enumerate() {
                if !pt.x.is_finite() || !pt.y.is_finite() {
                    continue;
                }
                if (pt.x - best_primary).abs() <= tolerance {
                    entries.push((mark_idx, 0, pt_idx));
                    break; // one entry per line series
                }
            }
        } else if !horizontal && child.tag == area_tag {
            let s = child
                .state
                .downcast_ref::<plot_area::area::State<<Renderer as crate::core::text::Renderer>::Paragraph>>();
            for (ser_idx, sub) in s.series_points.iter().enumerate() {
                for (pt_idx, pt) in sub.iter().enumerate() {
                    if !pt.x.is_finite() || !pt.y.is_finite() {
                        continue;
                    }
                    if (pt.x - best_primary).abs() <= tolerance {
                        entries.push((mark_idx, ser_idx, pt_idx));
                        break;
                    }
                }
            }
        } else if !horizontal && child.tag == xy_tag {
            if is_geo_xy(mark_idx) {
                continue;
            }
            let s = child.state.downcast_ref::<plot_area::xy::State>();
            for (pt_idx, pt) in s.pixel_points.iter().enumerate() {
                if !pt.x.is_finite() || !pt.y.is_finite() {
                    continue;
                }
                if (pt.x - best_primary).abs() <= tolerance {
                    entries.push((mark_idx, 0, pt_idx));
                    break;
                }
            }
        } else if child.tag == bars_tag {
            let s = child
                .state
                .downcast_ref::<plot_area::bars::State<<Renderer as crate::core::text::Renderer>::Paragraph>>();
            for (ser_idx, rects) in s.series_rects.iter().enumerate() {
                for (bar_idx, rect) in rects.iter().enumerate() {
                    if !rect_finite(rect) {
                        continue;
                    }
                    if (bar_primary(rect) - best_primary).abs() <= bar_tolerance {
                        entries.push((mark_idx, ser_idx, bar_idx));
                        break;
                    }
                }
            }
        }
    }

    if entries.is_empty() {
        return None;
    }
    Some(hover::Geometry::Cartesian { data_x, entries })
}

/// Hit-test a point against a single pie's pre-laid-out polar state.
///
/// Returns the index of the slice containing `local`, or `None` if
/// the point is in the donut hole, outside the outer radius, or in
/// an inter-slice gap. Shared by the pie hover scan and the pie click
/// dispatch in `update` so both use the exact same hit region.
fn pie_slice_at(
    local: Point,
    pie_state: &plot_area::pie::State<<Renderer as crate::core::text::Renderer>::Paragraph>,
) -> Option<usize> {
    let (cx, cy) = pie_state.center;
    let dx = local.x - cx;
    let dy = local.y - cy;
    let dist = (dx * dx + dy * dy).sqrt();

    if dist < pie_state.inner_radius || dist > pie_state.outer_radius {
        return None;
    }

    let mut angle = dy.atan2(dx);
    let first_start = pie_state.slice_angles.first().map(|(s, _)| *s).unwrap_or(0.0);
    if angle < first_start {
        angle += std::f32::consts::TAU;
    }

    pie_state
        .slice_angles
        .iter()
        .position(|(start, end)| angle >= *start && angle <= *end)
}

/// Hit-test the cursor against any pie marks in the plot area tree.
///
/// The cursor must lie within `[inner_radius, outer_radius]` of the
/// pie's center and inside one slice's `[start_angle, end_angle]`
/// sweep. Returns the first matching slice across pie marks.
fn find_pie_hover(local: Point, plot_area_tree: &Tree) -> Option<hover::Geometry> {
    let pie_tag = tree::Tag::of::<plot_area::pie::State<<Renderer as crate::core::text::Renderer>::Paragraph>>();

    for (mark_idx, mark_tree) in plot_area_tree.children.iter().enumerate() {
        if mark_tree.tag != pie_tag {
            continue;
        }
        let pie_state = mark_tree
            .state
            .downcast_ref::<plot_area::pie::State<<Renderer as crate::core::text::Renderer>::Paragraph>>();
        if let Some(slice_idx) = pie_slice_at(local, pie_state) {
            return Some(hover::Geometry::Pie { mark_idx, slice_idx });
        }
    }
    None
}

/// Hit-test the cursor against any geo-projected Xy bubbles.
///
/// Walks every `Series::Xy` whose `coord_kind == Geo` and finds the
/// closest bubble within `radius + 4px` of the cursor. Distance ties
/// resolve by smallest `(distance - radius)`, so a cursor inside two
/// overlapping bubbles snaps to the one whose interior it penetrates
/// further. Returns `None` when no geo bubble is in range — the
/// caller falls through to the cartesian snap-to-x.
fn find_geo_hover<Message>(
    local: Point,
    plot_area_tree: &Tree,
    plot_area: &plot_area::PlotArea<'_, Message, Renderer>,
) -> Option<hover::Geometry> {
    let xy_tag = tree::Tag::of::<plot_area::xy::State>();
    let mut best: Option<(usize, usize, f32)> = None;

    for (mark_idx, series) in plot_area.series.iter().enumerate() {
        let plot_area::Series::Xy(xy) = series else {
            continue;
        };
        if xy.data.coord_kind != crate::mark::xy::CoordKind::Geo {
            continue;
        }
        let Some(child) = plot_area_tree.children.get(mark_idx) else {
            continue;
        };
        if child.tag != xy_tag {
            continue;
        }
        let state = child.state.downcast_ref::<plot_area::xy::State>();
        for (point_idx, pixel) in state.pixel_points.iter().enumerate() {
            let radius = state.resolved_sizes.get(point_idx).copied().unwrap_or(0.0) / 2.0;
            let dx = local.x - pixel.x;
            let dy = local.y - pixel.y;
            let dist = (dx * dx + dy * dy).sqrt();
            let tolerance = radius + 4.0;
            if dist > tolerance {
                continue;
            }
            // `dist - radius` is negative when the cursor is inside
            // the marker; smallest value wins so deeper-penetrated
            // overlaps take priority over neighbours just inside
            // their tolerance band.
            let key = dist - radius;
            let take = match best {
                Some((_, _, b)) => key < b,
                None => true,
            };
            if take {
                best = Some((mark_idx, point_idx, key));
            }
        }
    }

    best.map(|(mark_idx, point_idx, _)| hover::Geometry::Geographic { mark_idx, point_idx })
}

fn find_geo_feature_click<Message>(
    local: Point,
    plot_area_tree: &Tree,
    plot_area: &plot_area::PlotArea<'_, Message, Renderer>,
) -> Option<crate::target::Target> {
    let hover::Geometry::Geographic { mark_idx, point_idx } = find_geo_hover(local, plot_area_tree, plot_area)? else {
        return None;
    };
    let plot_area::Series::Xy(xy) = plot_area.series.get(mark_idx)? else {
        return None;
    };
    geo_feature_target_for_point(xy.data, mark_idx, point_idx)
}

fn geo_feature_target_for_point(
    xy: &crate::mark::xy::Xy,
    mark_idx: usize,
    point_idx: usize,
) -> Option<crate::target::Target> {
    if xy.coord_kind != crate::mark::xy::CoordKind::Geo {
        return None;
    }
    let id = xy.feature_id_at(point_idx)?.clone();
    Some(crate::target::Target::Feature { mark: mark_idx, id })
}

/// Resolves a [`hover::Geometry`] into the public [`hover::Entry`]
/// the user's hover closure consumes.
///
/// The returned [`Entry`] borrows from the widget tree state and
/// from `plot_area`'s data references — both are reachable from
/// `&'b mut Chart` inside [`Widget::overlay`], so the inferred
/// lifetime is the reborrow lifetime, not the chart's `'a` data
/// borrow. The closure type accepts this via HRTB.
fn build_hover_entry<'b, Message>(
    geometry: &'b hover::Geometry,
    plot_area: &'b plot_area::PlotArea<'_, Message, Renderer>,
    plot_area_tree: &'b Tree,
    geo_plane: Option<&'b plot_area::geo::Plane>,
) -> Option<hover::Entry<'b>> {
    match geometry {
        hover::Geometry::Cartesian { entries, .. } => {
            let (mark_idx, series_idx, point_idx) = *entries.first()?;
            let series = plot_area.series.get(mark_idx)?;
            match series {
                plot_area::Series::Line(line) => {
                    let pt = line.data.points.get(point_idx)?;
                    Some(hover::Entry::Cartesian {
                        mark_idx,
                        series_idx,
                        point_idx,
                        x: pt.x,
                        y: pt.y,
                        series_name: line.data.name(),
                    })
                }
                plot_area::Series::Area(area) => {
                    let ser = area.data.series.get(series_idx)?;
                    let pt = ser.points.get(point_idx)?;
                    Some(hover::Entry::Cartesian {
                        mark_idx,
                        series_idx,
                        point_idx,
                        x: pt.x,
                        y: pt.y,
                        series_name: ser.name(),
                    })
                }
                plot_area::Series::Xy(xy) => {
                    let pt = xy.data.points.get(point_idx)?;
                    Some(hover::Entry::Cartesian {
                        mark_idx,
                        series_idx,
                        point_idx,
                        x: pt.x,
                        y: pt.y,
                        series_name: xy.data.name.as_deref(),
                    })
                }
                plot_area::Series::Bars(bars) => {
                    let bar_series = bars.data.series.get(series_idx)?;
                    let pt = bar_series.points.get(point_idx)?;
                    Some(hover::Entry::Cartesian {
                        mark_idx,
                        series_idx,
                        point_idx,
                        x: pt.x,
                        y: pt.y,
                        series_name: bar_series.name(),
                    })
                }
                _ => None,
            }
        }
        hover::Geometry::Pie { mark_idx, slice_idx } => {
            let plot_area::Series::Pie(pie) = plot_area.series.get(*mark_idx)? else {
                return None;
            };
            let slice = pie.data.slices.get(*slice_idx)?;
            let total: f64 = pie.data.slices.iter().map(|s| s.value()).sum();
            Some(hover::Entry::Pie {
                mark_idx: *mark_idx,
                slice_idx: *slice_idx,
                label: slice.get_name(),
                value: slice.value(),
                total,
            })
        }
        hover::Geometry::Geographic { mark_idx, point_idx } => {
            let plot_area::Series::Xy(xy) = plot_area.series.get(*mark_idx)? else {
                return None;
            };
            let pt = xy.data.points.get(*point_idx)?;
            // Per-point label populated by `bubble_map(...)`; absent
            // for plain `xy(...).on_geo()`. Tooltip value channel
            // similarly per-point when available.
            let label = xy
                .data
                .labels
                .get(*point_idx)
                .and_then(|l| l.as_deref())
                .or(xy.data.name.as_deref());
            let value = xy
                .data
                .tooltip_values
                .get(*point_idx)
                .copied()
                .flatten()
                .unwrap_or(pt.y);
            Some(hover::Entry::Geographic {
                mark_idx: *mark_idx,
                point_idx: *point_idx,
                label,
                value,
            })
        }
        hover::Geometry::ChoroplethArea { mark_idx, feature_idx } => {
            let plot_area::Series::Choropleth(c) = plot_area.series.get(*mark_idx)? else {
                return None;
            };
            let geo_plane = geo_plane?;
            let feature_id = geo_plane.filtered_ids.get(*feature_idx)?;
            let feature = geo_plane.geo.as_ref()?.get(feature_id.clone())?;
            let value = c
                .data
                .entries()
                .iter()
                .find(|e| e.id() == feature_id)
                .and_then(|e| e.value());
            // Touch the per-feature widget tree so the borrow checker
            // sees `plot_area_tree` as live for the entry's lifetime.
            // (No state to read today; placeholder for future
            // per-feature highlight caches.)
            let _ = plot_area_tree;
            Some(hover::Entry::Choropleth {
                mark_idx: *mark_idx,
                feature_idx: *feature_idx,
                feature_id: feature_id.as_str(),
                feature_name: feature.name.as_str(),
                properties: &feature.properties,
                value,
            })
        }
    }
}

/// Renders a [`hover::Annotation`] (an iced [`Element`] plus
/// positioning chrome) as a floating overlay above the chart.
///
/// Constructed inside [`Chart::overlay`] when a hover closure is
/// installed and the cursor is over a hovered mark. The inner
/// element's [`Tree`] is borrowed from the chart's
/// [`State::annotation_tree`] so per-widget state (text
/// paragraphs, button hover state, etc.) survives across the
/// runtime's separate layout and draw passes — iced creates a
/// fresh overlay instance for each pass, so anything owned by
/// the overlay would be discarded between them.
struct AnnotationOverlay<'a, 'b, Message, Theme> {
    annotation: hover::Annotation<'a, Message, Theme>,
    tree: &'b mut Tree,
    cursor: Point,
    chart_bounds: Rectangle,
}

impl<Message, Theme> overlay::Overlay<Message, Theme, Renderer> for AnnotationOverlay<'_, '_, Message, Theme>
where
    Theme: design::Design,
{
    fn layout(&mut self, renderer: &Renderer, bounds: Size) -> layout::Node {
        let viewport = Rectangle::with_size(bounds);
        let limits = if self.annotation.snap_within_viewport {
            layout::Limits::new(Size::ZERO, viewport.size())
        } else {
            layout::Limits::new(Size::ZERO, Size::INFINITE)
        }
        .shrink(self.annotation.padding);

        let content_layout = self
            .annotation
            .content
            .as_widget_mut()
            .layout(self.tree, renderer, &limits);

        let content_size = content_layout.size();
        let pad = self.annotation.padding;
        let outer_size = Size::new(
            content_size.width + pad.left + pad.right,
            content_size.height + pad.top + pad.bottom,
        );

        // Position the outer rectangle relative to the cursor / mark
        // anchor based on the annotation's [`Position`] knob. The
        // chart's flavor of `FollowCursor` flips horizontally around
        // the chart's mid-x so the box never covers the data: cursor
        // on the left half → box extends right, cursor on the right
        // half → box extends left.
        let cursor = self.cursor;
        let chart_mid_x = self.chart_bounds.x + self.chart_bounds.width / 2.0;
        let outer_origin = match self.annotation.position {
            hover::Position::Top => Point::new(
                cursor.x - outer_size.width / 2.0,
                cursor.y - outer_size.height - self.annotation.gap,
            ),
            hover::Position::Bottom => Point::new(cursor.x - outer_size.width / 2.0, cursor.y + self.annotation.gap),
            hover::Position::Left => Point::new(
                cursor.x - outer_size.width - self.annotation.gap,
                cursor.y - outer_size.height / 2.0,
            ),
            hover::Position::Right => Point::new(cursor.x + self.annotation.gap, cursor.y - outer_size.height / 2.0),
            hover::Position::FollowCursor => {
                let on_left_half = cursor.x < chart_mid_x;
                let x = if on_left_half {
                    cursor.x + self.annotation.gap
                } else {
                    cursor.x - outer_size.width - self.annotation.gap
                };
                Point::new(x, cursor.y - outer_size.height / 2.0)
            }
        };

        let mut outer = Rectangle {
            x: outer_origin.x,
            y: outer_origin.y,
            width: outer_size.width,
            height: outer_size.height,
        };

        if self.annotation.snap_within_viewport {
            if outer.x < viewport.x {
                outer.x = viewport.x;
            } else if viewport.x + viewport.width < outer.x + outer.width {
                outer.x = viewport.x + viewport.width - outer.width;
            }
            if outer.y < viewport.y {
                outer.y = viewport.y;
            } else if viewport.y + viewport.height < outer.y + outer.height {
                outer.y = viewport.y + viewport.height - outer.height;
            }
        }

        layout::Node::with_children(outer.size(), vec![
            content_layout.translate(Vector::new(self.annotation.padding.left, self.annotation.padding.top)),
        ])
        .translate(Vector::new(outer.x, outer.y))
    }

    fn draw(
        &self,
        renderer: &mut Renderer,
        theme: &Theme,
        defaults: &crate::core::renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
    ) {
        use crate::core::renderer::Renderer as _;

        let bounds = layout.bounds();
        let background = theme.background_color();
        let text_pair = theme.text_pair();
        let seed = theme.seed();
        let divider_color = theme.divider_color().resolve(background, text_pair, &seed, None);

        // Solid background so text on top of any underlying chart
        // content remains readable. The 1px border picks up the
        // theme's divider color, matching the canvas-drawn default
        // tooltip box for visual continuity.
        let box_bg = crate::core::Color { a: 1.0, ..background };
        renderer.fill_quad(
            crate::core::renderer::Quad {
                bounds,
                border: crate::core::Border {
                    width: 1.0,
                    radius: 4.0.into(),
                    color: divider_color,
                },
                ..Default::default()
            },
            box_bg,
        );

        let content_layout = layout.children().next().expect("annotation has one child");
        self.annotation
            .content
            .as_widget()
            .draw(self.tree, renderer, theme, defaults, content_layout, cursor, &bounds);
    }

    fn mouse_interaction(&self, layout: Layout<'_>, cursor: mouse::Cursor, renderer: &Renderer) -> mouse::Interaction {
        let content_layout = layout.children().next().expect("annotation has one child");
        self.annotation.content.as_widget().mouse_interaction(
            self.tree,
            content_layout,
            cursor,
            &layout.bounds(),
            renderer,
        )
    }
}

/// Hit-test the cursor against the choropleth's projected polygons.
///
/// Walks every `Series::Choropleth` and runs
/// [`plot_area::geo::hit_test`] against the chart-level
/// [`plot_area::geo::Plane`] (which holds the projected rings + bboxes
/// shared across geo-aware marks). Returns `None` when the cursor is
/// over no polygon — the caller falls through to the cartesian snap.
///
/// In practice a chart only carries one choropleth mark at a time, so
/// the first match wins; the loop is structured to accommodate future
/// multi-choropleth layers without ordering surprises.
fn find_choropleth_hover<Message>(
    local: Point,
    plot_area: &plot_area::PlotArea<'_, Message, Renderer>,
    geo_plane: &plot_area::geo::Plane,
) -> Option<hover::Geometry> {
    for (mark_idx, series) in plot_area.series.iter().enumerate() {
        let plot_area::Series::Choropleth(c) = series else {
            continue;
        };
        if let Some(feature_idx) = plot_area::geo::hit_test(geo_plane, local)
            && let Some(feature_id) = geo_plane.filtered_ids.get(feature_idx)
            && choropleth_feature_has_hover_value(c.data, feature_id)
        {
            return Some(hover::Geometry::ChoroplethArea { mark_idx, feature_idx });
        }
    }
    None
}

fn choropleth_feature_has_hover_value(
    choropleth: &crate::mark::choropleth::Choropleth,
    feature_id: &crate::feature::Id,
) -> bool {
    choropleth
        .entries()
        .iter()
        .any(|entry| entry.id() == feature_id && entry.value().is_some_and(f64::is_finite))
}

/// Returns a mutable handle to the [`animation::Tick`] hosted on
/// whichever per-mark `State` the tree node carries, or `None` when
/// the mark type doesn't host a tick. Tag is matched linearly so each
/// new animated mark adds exactly one branch.
fn animation_tick_mut(tree: &mut Tree) -> Option<&mut animation::Tick> {
    let tag = tree.tag;
    if tag == tree::Tag::of::<plot_area::pie::State<<Renderer as crate::core::text::Renderer>::Paragraph>>() {
        return Some(
            &mut tree
                .state
                .downcast_mut::<plot_area::pie::State<<Renderer as crate::core::text::Renderer>::Paragraph>>()
                .tick,
        );
    }
    if tag == tree::Tag::of::<plot_area::bars::State<<Renderer as crate::core::text::Renderer>::Paragraph>>() {
        return Some(
            &mut tree
                .state
                .downcast_mut::<plot_area::bars::State<<Renderer as crate::core::text::Renderer>::Paragraph>>()
                .tick,
        );
    }
    if tag == tree::Tag::of::<plot_area::waterfall::State<<Renderer as crate::core::text::Renderer>::Paragraph>>() {
        return Some(
            &mut tree
                .state
                .downcast_mut::<plot_area::waterfall::State<<Renderer as crate::core::text::Renderer>::Paragraph>>()
                .tick,
        );
    }
    if tag == tree::Tag::of::<plot_area::line::State<<Renderer as crate::core::text::Renderer>::Paragraph>>() {
        return Some(
            &mut tree
                .state
                .downcast_mut::<plot_area::line::State<<Renderer as crate::core::text::Renderer>::Paragraph>>()
                .tick,
        );
    }
    if tag == tree::Tag::of::<plot_area::area::State<<Renderer as crate::core::text::Renderer>::Paragraph>>() {
        return Some(
            &mut tree
                .state
                .downcast_mut::<plot_area::area::State<<Renderer as crate::core::text::Renderer>::Paragraph>>()
                .tick,
        );
    }
    if tag == tree::Tag::of::<plot_area::xy::State>() {
        return Some(&mut tree.state.downcast_mut::<plot_area::xy::State>().tick);
    }
    if tag == tree::Tag::of::<plot_area::heatmap::State<<Renderer as crate::core::text::Renderer>::Paragraph>>() {
        return Some(
            &mut tree
                .state
                .downcast_mut::<plot_area::heatmap::State<<Renderer as crate::core::text::Renderer>::Paragraph>>()
                .tick,
        );
    }
    if tag == tree::Tag::of::<plot_area::treemap::State<<Renderer as crate::core::text::Renderer>::Paragraph>>() {
        return Some(
            &mut tree
                .state
                .downcast_mut::<plot_area::treemap::State<<Renderer as crate::core::text::Renderer>::Paragraph>>()
                .tick,
        );
    }
    if tag == tree::Tag::of::<plot_area::choropleth::State>() {
        return Some(&mut tree.state.downcast_mut::<plot_area::choropleth::State>().tick);
    }
    if tag == tree::Tag::of::<plot_area::gauge::State<<Renderer as crate::core::text::Renderer>::Paragraph>>() {
        return Some(
            &mut tree
                .state
                .downcast_mut::<plot_area::gauge::State<<Renderer as crate::core::text::Renderer>::Paragraph>>()
                .tick,
        );
    }
    if tag == tree::Tag::of::<plot_area::band::State<<Renderer as crate::core::text::Renderer>::Paragraph>>() {
        return Some(
            &mut tree
                .state
                .downcast_mut::<plot_area::band::State<<Renderer as crate::core::text::Renderer>::Paragraph>>()
                .tick,
        );
    }
    if tag == tree::Tag::of::<plot_area::boxplot::State>() {
        return Some(&mut tree.state.downcast_mut::<plot_area::boxplot::State>().tick);
    }
    if tag == tree::Tag::of::<plot_area::violin::State>() {
        return Some(&mut tree.state.downcast_mut::<plot_area::violin::State>().tick);
    }
    None
}

/// Snapshots a pie's pre-rebuild `slice_angles` onto the post-rebuild
/// tree's `previous_angles` and arms the next redraw to interpolate
/// from there. The caller is responsible for confirming both nodes
/// carry a `pie::State`.
fn replant_pie(old_mark: &Tree, new_mark: &mut Tree, animate: bool) {
    let old_state = old_mark
        .state
        .downcast_ref::<plot_area::pie::State<<Renderer as crate::core::text::Renderer>::Paragraph>>();
    let new_state = new_mark
        .state
        .downcast_mut::<plot_area::pie::State<<Renderer as crate::core::text::Renderer>::Paragraph>>();
    if animate {
        new_state.previous_angles = old_state.slice_angles.clone();
        new_state.tick.pending_start = true;
    } else {
        new_state.tick.pending_start = false;
    }
}

/// Snapshots a Bars mark's pre-rebuild `series_rects` onto the
/// post-rebuild tree's `previous_series_rects` and arms the next
/// redraw to interpolate from there. The caller is responsible for
/// confirming both nodes carry a `bars::State`.
fn replant_bars(old_mark: &Tree, new_mark: &mut Tree, animate: bool) {
    let old_state = old_mark
        .state
        .downcast_ref::<plot_area::bars::State<<Renderer as crate::core::text::Renderer>::Paragraph>>();
    let new_state = new_mark
        .state
        .downcast_mut::<plot_area::bars::State<<Renderer as crate::core::text::Renderer>::Paragraph>>();
    if animate {
        new_state.previous_series_rects = old_state.series_rects.clone();
        new_state.tick.pending_start = true;
    } else {
        new_state.tick.pending_start = false;
    }
}

/// Snapshots a Waterfall mark's pre-rebuild `rects` and `tops` onto the
/// post-rebuild tree's `previous_rects` and `previous_tops` and arms
/// the next redraw to interpolate from there. The caller is responsible
/// for confirming both nodes carry a `waterfall::State`.
fn replant_waterfall(old_mark: &Tree, new_mark: &mut Tree, animate: bool) {
    let old_state = old_mark
        .state
        .downcast_ref::<plot_area::waterfall::State<<Renderer as crate::core::text::Renderer>::Paragraph>>();
    let new_state = new_mark
        .state
        .downcast_mut::<plot_area::waterfall::State<<Renderer as crate::core::text::Renderer>::Paragraph>>();
    if animate {
        new_state.previous_rects = old_state.rects.clone();
        new_state.previous_tops = old_state.tops.clone();
        new_state.tick.pending_start = true;
    } else {
        new_state.tick.pending_start = false;
    }
}

/// Snapshots a Line mark's pre-rebuild `pixel_points` onto the
/// post-rebuild tree's `previous_pixel_points` and arms the next
/// redraw to interpolate from there. The caller is responsible for
/// confirming both nodes carry a `line::State`.
fn replant_line(old_mark: &Tree, new_mark: &mut Tree, animate: bool) {
    let old_state = old_mark
        .state
        .downcast_ref::<plot_area::line::State<<Renderer as crate::core::text::Renderer>::Paragraph>>();
    let new_state = new_mark
        .state
        .downcast_mut::<plot_area::line::State<<Renderer as crate::core::text::Renderer>::Paragraph>>();
    if animate {
        new_state.previous_pixel_points = old_state.pixel_points.clone();
        new_state.tick.pending_start = true;
    } else {
        new_state.tick.pending_start = false;
    }
}

/// Snapshots an Area mark's pre-rebuild `series_points` and
/// `series_baselines` onto the post-rebuild tree's
/// `previous_series_points` and `previous_series_baselines` and arms
/// the next redraw to interpolate from there. The caller is
/// responsible for confirming both nodes carry an `area::State`.
fn replant_area(old_mark: &Tree, new_mark: &mut Tree, animate: bool) {
    let old_state = old_mark
        .state
        .downcast_ref::<plot_area::area::State<<Renderer as crate::core::text::Renderer>::Paragraph>>();
    let new_state = new_mark
        .state
        .downcast_mut::<plot_area::area::State<<Renderer as crate::core::text::Renderer>::Paragraph>>();
    if animate {
        new_state.previous_series_points = old_state.series_points.clone();
        new_state.previous_series_baselines = old_state.series_baselines.clone();
        new_state.tick.pending_start = true;
    } else {
        new_state.tick.pending_start = false;
    }
}

/// Snapshots an Xy mark's pre-rebuild `pixel_points` and
/// `resolved_sizes` onto the post-rebuild tree's
/// `previous_pixel_points` and `previous_resolved_sizes` and arms the
/// next redraw to interpolate from there. The caller is responsible
/// for confirming both nodes carry an `xy::State`.
fn replant_xy(old_mark: &Tree, new_mark: &mut Tree, animate: bool) {
    let old_state = old_mark.state.downcast_ref::<plot_area::xy::State>();
    let new_state = new_mark.state.downcast_mut::<plot_area::xy::State>();
    if animate {
        new_state.previous_pixel_points = old_state.pixel_points.clone();
        new_state.previous_resolved_sizes = old_state.resolved_sizes.clone();
        new_state.tick.pending_start = true;
    } else {
        new_state.tick.pending_start = false;
    }
}

/// Snapshots a Heatmap mark's pre-rebuild `cell_colors` onto the
/// post-rebuild tree's `previous_cell_colors` and arms the next redraw
/// to interpolate from there. The caller is responsible for confirming
/// both nodes carry a `heatmap::State`.
fn replant_heatmap(old_mark: &Tree, new_mark: &mut Tree, animate: bool) {
    let old_state = old_mark
        .state
        .downcast_ref::<plot_area::heatmap::State<<Renderer as crate::core::text::Renderer>::Paragraph>>();
    let new_state = new_mark
        .state
        .downcast_mut::<plot_area::heatmap::State<<Renderer as crate::core::text::Renderer>::Paragraph>>();
    if animate {
        new_state.previous_cell_colors = old_state.cell_colors.borrow().clone();
        new_state.tick.pending_start = true;
    } else {
        new_state.tick.pending_start = false;
    }
}

/// Snapshots a Treemap mark's pre-rebuild `item_rects` onto the
/// post-rebuild tree's `previous_item_rects` and arms the next redraw
/// to interpolate from there. The caller is responsible for confirming
/// both nodes carry a `treemap::State`.
fn replant_treemap(old_mark: &Tree, new_mark: &mut Tree, animate: bool) {
    let old_state = old_mark
        .state
        .downcast_ref::<plot_area::treemap::State<<Renderer as crate::core::text::Renderer>::Paragraph>>();
    let new_state = new_mark
        .state
        .downcast_mut::<plot_area::treemap::State<<Renderer as crate::core::text::Renderer>::Paragraph>>();
    if animate {
        new_state.previous_item_rects = old_state.item_rects.clone();
        new_state.tick.pending_start = true;
    } else {
        new_state.tick.pending_start = false;
    }
}

/// Snapshots a Choropleth mark's pre-rebuild `fill_colors` onto the
/// post-rebuild tree's `previous_fill_colors` and arms the next redraw
/// to interpolate from there. The projection cache lives on the
/// shared [`plot_area::geo::Plane`] (sibling-replanted by
/// [`replant_geo_plane`]); only the data-change animation snapshot
/// belongs here. The caller is responsible for confirming both nodes
/// carry a `choropleth::State`.
fn replant_choropleth(old_mark: &Tree, new_mark: &mut Tree, animate: bool) {
    let old_state = old_mark.state.downcast_ref::<plot_area::choropleth::State>();
    let new_state = new_mark.state.downcast_mut::<plot_area::choropleth::State>();

    if animate {
        new_state.previous_fill_colors = old_state.fill_colors.borrow().clone();
        new_state.tick.pending_start = true;
    } else {
        new_state.tick.pending_start = false;
    }
}

/// Snapshots a Gauge mark's pre-rebuild `value_angle` onto the
/// post-rebuild tree's `previous_value_angle` and arms the next redraw
/// to interpolate from there. The caller is responsible for confirming
/// both nodes carry a `gauge::State`.
fn replant_gauge(old_mark: &Tree, new_mark: &mut Tree, animate: bool) {
    let old_state = old_mark
        .state
        .downcast_ref::<plot_area::gauge::State<<Renderer as crate::core::text::Renderer>::Paragraph>>();
    let new_state = new_mark
        .state
        .downcast_mut::<plot_area::gauge::State<<Renderer as crate::core::text::Renderer>::Paragraph>>();
    if animate {
        new_state.previous_value_angle = old_state.value_angle;
        new_state.tick.pending_start = true;
    } else {
        new_state.tick.pending_start = false;
    }
}

/// Snapshots a Band mark's pre-rebuild `lower_pixel` and `upper_pixel`
/// onto the post-rebuild tree's `previous_lower_pixel` and
/// `previous_upper_pixel` and arms the next redraw to interpolate from
/// there. The caller is responsible for confirming both nodes carry a
/// `band::State`.
fn replant_band(old_mark: &Tree, new_mark: &mut Tree, animate: bool) {
    let old_state = old_mark
        .state
        .downcast_ref::<plot_area::band::State<<Renderer as crate::core::text::Renderer>::Paragraph>>();
    let new_state = new_mark
        .state
        .downcast_mut::<plot_area::band::State<<Renderer as crate::core::text::Renderer>::Paragraph>>();
    if animate {
        new_state.previous_lower_pixel = Some(old_state.lower_pixel);
        new_state.previous_upper_pixel = Some(old_state.upper_pixel);
        new_state.tick.pending_start = true;
    } else {
        new_state.tick.pending_start = false;
    }
}

/// Snapshots a BoxPlot mark's pre-rebuild `entries_layout` onto the
/// post-rebuild tree's `previous_entries` and arms the next redraw to
/// interpolate every component (box, whiskers, median, outliers) from
/// there. The caller is responsible for confirming both nodes carry a
/// `boxplot::State`.
fn replant_boxplot(old_mark: &Tree, new_mark: &mut Tree, animate: bool) {
    let old_state = old_mark.state.downcast_ref::<plot_area::boxplot::State>();
    let new_state = new_mark.state.downcast_mut::<plot_area::boxplot::State>();
    if animate {
        new_state.previous_entries = old_state.entries_layout.clone();
        new_state.tick.pending_start = true;
    } else {
        new_state.tick.pending_start = false;
    }
}

/// Snapshots a Violin mark's pre-rebuild `entries` onto the
/// post-rebuild tree's `previous_entries` and arms the next redraw to
/// interpolate every component (silhouette outline, box stats) from
/// there. The caller is responsible for confirming both nodes carry a
/// `violin::State`.
fn replant_violin(old_mark: &Tree, new_mark: &mut Tree, animate: bool) {
    let old_state = old_mark.state.downcast_ref::<plot_area::violin::State>();
    let new_state = new_mark.state.downcast_mut::<plot_area::violin::State>();
    if animate {
        new_state.previous_entries = old_state.entries.clone();
        new_state.tick.pending_start = true;
    } else {
        new_state.tick.pending_start = false;
    }
}

/// Walks old/new plot-area children pairwise and dispatches per-tag
/// to the matching `replant_<mark>` snapshot helper. A no-op when
/// either tree lacks a plot area.
fn replant_mark_animations(old_children: &[Tree], new_children: &mut [Tree], animate: bool) {
    let pie_tag = tree::Tag::of::<plot_area::pie::State<<Renderer as crate::core::text::Renderer>::Paragraph>>();
    let bars_tag = tree::Tag::of::<plot_area::bars::State<<Renderer as crate::core::text::Renderer>::Paragraph>>();
    let waterfall_tag =
        tree::Tag::of::<plot_area::waterfall::State<<Renderer as crate::core::text::Renderer>::Paragraph>>();
    let line_tag = tree::Tag::of::<plot_area::line::State<<Renderer as crate::core::text::Renderer>::Paragraph>>();
    let area_tag = tree::Tag::of::<plot_area::area::State<<Renderer as crate::core::text::Renderer>::Paragraph>>();
    let xy_tag = tree::Tag::of::<plot_area::xy::State>();
    let heatmap_tag =
        tree::Tag::of::<plot_area::heatmap::State<<Renderer as crate::core::text::Renderer>::Paragraph>>();
    let treemap_tag =
        tree::Tag::of::<plot_area::treemap::State<<Renderer as crate::core::text::Renderer>::Paragraph>>();
    let choropleth_tag = tree::Tag::of::<plot_area::choropleth::State>();
    let gauge_tag = tree::Tag::of::<plot_area::gauge::State<<Renderer as crate::core::text::Renderer>::Paragraph>>();
    let band_tag = tree::Tag::of::<plot_area::band::State<<Renderer as crate::core::text::Renderer>::Paragraph>>();
    let boxplot_tag = tree::Tag::of::<plot_area::boxplot::State>();
    let violin_tag = tree::Tag::of::<plot_area::violin::State>();
    let Some(old_scene) = old_children.first() else {
        return;
    };
    let Some(new_scene) = new_children.first_mut() else {
        return;
    };
    let Some(old_plot_area) = old_scene.children.get(6) else {
        return;
    };
    let Some(new_plot_area) = new_scene.children.get_mut(6) else {
        return;
    };

    for (old_mark, new_mark) in old_plot_area.children.iter().zip(new_plot_area.children.iter_mut()) {
        if old_mark.tag == pie_tag && new_mark.tag == pie_tag {
            replant_pie(old_mark, new_mark, animate);
            continue;
        }
        if old_mark.tag == bars_tag && new_mark.tag == bars_tag {
            replant_bars(old_mark, new_mark, animate);
            continue;
        }
        if old_mark.tag == waterfall_tag && new_mark.tag == waterfall_tag {
            replant_waterfall(old_mark, new_mark, animate);
            continue;
        }
        if old_mark.tag == line_tag && new_mark.tag == line_tag {
            replant_line(old_mark, new_mark, animate);
            continue;
        }
        if old_mark.tag == area_tag && new_mark.tag == area_tag {
            replant_area(old_mark, new_mark, animate);
            continue;
        }
        if old_mark.tag == xy_tag && new_mark.tag == xy_tag {
            replant_xy(old_mark, new_mark, animate);
            continue;
        }
        if old_mark.tag == heatmap_tag && new_mark.tag == heatmap_tag {
            replant_heatmap(old_mark, new_mark, animate);
            continue;
        }
        if old_mark.tag == treemap_tag && new_mark.tag == treemap_tag {
            replant_treemap(old_mark, new_mark, animate);
            continue;
        }
        if old_mark.tag == choropleth_tag && new_mark.tag == choropleth_tag {
            replant_choropleth(old_mark, new_mark, animate);
            continue;
        }
        if old_mark.tag == gauge_tag && new_mark.tag == gauge_tag {
            replant_gauge(old_mark, new_mark, animate);
            continue;
        }
        if old_mark.tag == band_tag && new_mark.tag == band_tag {
            replant_band(old_mark, new_mark, animate);
            continue;
        }
        if old_mark.tag == boxplot_tag && new_mark.tag == boxplot_tag {
            replant_boxplot(old_mark, new_mark, animate);
            continue;
        }
        if old_mark.tag == violin_tag && new_mark.tag == violin_tag {
            replant_violin(old_mark, new_mark, animate);
            continue;
        }
    }
}

/// Moves the chart-level `geo_plane` from the pre-rebuild
/// [`plot_area::State`] onto the post-rebuild one so the next layout's
/// `Arc::ptr_eq + prev_size + prev_scope + prev_projection` dirty-check
/// short-circuits when the geo inputs are unchanged. Sibling to
/// [`replant_mark_animations`] (which walks per-mark child trees but
/// never the plot-area's own state). A no-op when either tree lacks
/// the expected `[scene][plot_area]` shape — defensive against future
/// shape drift the way the animation walker is.
///
/// Takes `old_children` by `&mut` rather than `&` so the cached
/// projection (potentially many KB of `Vec<Vec<Vec<(f32, f32)>>>`)
/// moves rather than clones; the old tree is dropped immediately
/// after this call returns.
fn replant_geo_plane(old_children: &mut [Tree], new_children: &mut [Tree]) {
    let plot_area_tag = tree::Tag::of::<plot_area::State>();
    let Some(old_scene) = old_children.first_mut() else {
        return;
    };
    let Some(new_scene) = new_children.first_mut() else {
        return;
    };
    let Some(old_plot_area) = old_scene.children.get_mut(6) else {
        return;
    };
    let Some(new_plot_area) = new_scene.children.get_mut(6) else {
        return;
    };
    if old_plot_area.tag != plot_area_tag || new_plot_area.tag != plot_area_tag {
        return;
    }
    let old_state = old_plot_area.state.downcast_mut::<plot_area::State>();
    let new_state = new_plot_area.state.downcast_mut::<plot_area::State>();
    new_state.geo_plane = old_state.geo_plane.take();
}

/// Walks the plot-area subtree and advances every animated mark's
/// [`animation::Tick`] by one frame. Kicks pending sweeps off on the
/// first redraw that carries an `Instant` and asks for another redraw
/// while any sweep is still in flight. Gated at the top by the
/// chart-level `animate` toggle so opted-out charts don't churn.
fn advance_mark_animations<Message>(tree: &mut Tree, now: Instant, animate: bool, shell: &mut Shell<'_, Message>) {
    if !animate {
        return;
    }
    let Some(scene_tree) = tree.children.get_mut(0) else {
        return;
    };
    let Some(plot_area_tree) = scene_tree.children.get_mut(6) else {
        return;
    };

    for mark_tree in plot_area_tree.children.iter_mut() {
        if let Some(tick) = animation_tick_mut(mark_tree) {
            tick.advance(now, shell);
        }
    }
}

/// Lays out a donut center overlay child as a sibling of the scene
/// node, returning a `Node` already positioned in chart-local coords.
///
/// Resolves the first `Mark::Pie`'s laid-out state to inscribe an
/// `Inset` element inside the donut hole, or to fill the plot area
/// for `Stack`. When pie state isn't ready (no slices yet, or no
/// `Mark::Pie` in the scene), returns a zero-sized node so `draw`
/// becomes a no-op.
fn layout_donut_center<Message, Theme>(
    padding: Padding,
    plot_area_offset: Point,
    scene_tree: &Tree,
    element: &mut Element<'_, Message, Theme, Renderer>,
    placement: donut::Placement,
    center_tree: &mut Tree,
    renderer: &Renderer,
) -> layout::Node {
    let plot_area_tree = &scene_tree.children[6];
    let plot_area_state = plot_area_tree.state.downcast_ref::<plot_area::State>();
    if plot_area_state.domain.is_none() {
        return layout::Node::new(Size::ZERO);
    }
    let content_rect = plot_area_state.content_rect;

    let pie_tag = tree::Tag::of::<plot_area::pie::State<<Renderer as crate::core::text::Renderer>::Paragraph>>();
    let Some(pie_tree) = plot_area_tree.children.iter().find(|t| t.tag == pie_tag) else {
        return layout::Node::new(Size::ZERO);
    };
    let pie_state = pie_tree
        .state
        .downcast_ref::<plot_area::pie::State<<Renderer as crate::core::text::Renderer>::Paragraph>>();
    let (pie_center, inner_radius) = pie_state.center_geometry();

    // Origin of the plot-area's plane in chart-local coords. Mirrors
    // `compute_plot_bounds`'s offset chain so the overlay aligns with
    // the same rectangle hover/click hit-tests against.
    let plane_origin = Point::new(
        padding.left + plot_area_offset.x + content_rect.x,
        padding.top + plot_area_offset.y + content_rect.y,
    );

    let (origin, area_size) = match placement {
        donut::Placement::Stack => (plane_origin, Size::new(content_rect.width, content_rect.height)),
        donut::Placement::Inset => {
            // Largest axis-aligned square inscribed in a circle of radius
            // `inner_radius`: side = r * sqrt(2).
            let side = inner_radius * std::f32::consts::SQRT_2;
            let inset_origin = Point::new(
                plane_origin.x + pie_center.x - side / 2.0,
                plane_origin.y + pie_center.y - side / 2.0,
            );
            (inset_origin, Size::new(side, side))
        }
    };

    if area_size.width <= 0.0 || area_size.height <= 0.0 {
        return layout::Node::new(Size::ZERO);
    }

    let limits = layout::Limits::new(Size::ZERO, area_size);
    let node = element.as_widget_mut().layout(center_tree, renderer, &limits);
    node.move_to(origin)
}

impl<'a, Message, Design, Theme> Widget<Message, Theme, Renderer> for Chart<'a, Message, Design, Theme>
where
    Design: design::Design + Clone + 'a,
    Theme: design::Design,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn children(&self) -> Vec<Tree> {
        // Tree-shape invariant:
        //   children[0] = Scene's tree
        //   children[1] = donut center child (only when `Kind::Donut` carries a `Center`)
        let mut children = vec![self.scene.state()];
        if let Kind::Donut(d) = &self.kind
            && let Some(c) = &d.center
        {
            children.push(Tree::new(&c.element));
        }
        children
    }

    fn diff(&self, tree: &mut Tree) {
        let state = tree.state.downcast_mut::<State>();
        if state.generation != self.generation {
            state.generation = self.generation;
            // Wholesale rebuild loses pie tree state, so snapshot the
            // previous slice angles per pie and replant them on the new
            // tree. The next sweep then interpolates from those instead
            // of mounting from `0`. Done positionally — each pie mark
            // stays at its plot-area child index across rebuilds, since
            // the data's mark vector drives both layouts.
            let mut old_children = std::mem::replace(&mut tree.children, self.children());
            replant_mark_animations(&old_children, &mut tree.children, self.animate);
            replant_geo_plane(&mut old_children, &mut tree.children);
            return;
        }
        self.scene.diff(&mut tree.children[0]);

        // Reconcile the optional donut center child at children[1].
        match &self.kind {
            Kind::Donut(d) => match &d.center {
                Some(c) => {
                    if tree.children.len() < 2 {
                        tree.children.push(Tree::new(&c.element));
                    } else {
                        tree.children[1].diff(c.element.as_widget());
                    }
                }
                None => {
                    tree.children.truncate(1);
                }
            },
            Kind::Generic => {
                tree.children.truncate(1);
            }
        }
    }

    fn size(&self) -> Size<Length> {
        Size {
            width: self.width,
            height: self.height,
        }
    }

    fn layout(&mut self, tree: &mut Tree, renderer: &Renderer, limits: &layout::Limits) -> layout::Node {
        use crate::core::Point;

        // Push donut-only state (e.g. hole radius) onto the renderer-side
        // pie before scene layout reads it.
        self.apply_kind_to_renderer();

        let size = limits.resolve(self.width, self.height, Size::ZERO);

        // Calculate inner size after padding
        let inner_size = Size::new(
            (size.width - self.padding.left - self.padding.right).max(0.0),
            (size.height - self.padding.top - self.padding.bottom).max(0.0),
        );

        let scene_origin = Point::new(self.padding.left, self.padding.top);

        // Split-borrow so we can lay out the optional center child after
        // the scene has populated pie state at scene_tree.children[6].
        let (scene_slot, rest) = tree.children.split_at_mut(1);
        let scene_tree = &mut scene_slot[0];

        // Delegate layout to scene with padded limits
        let scene_node = self
            .scene
            .layout(scene_tree, renderer, &layout::Limits::new(Size::ZERO, inner_size));

        let mut layout_children = vec![scene_node.move_to(scene_origin)];

        // Lay out the donut center child (if any) as a sibling of the
        // scene node, so `update`/`draw` can resolve its bounds via
        // `layout.children().nth(1)`.
        if let Kind::Donut(d) = &mut self.kind
            && let Some(center) = &mut d.center
            && let Some(center_tree) = rest.first_mut()
        {
            let center_node = layout_donut_center(
                self.padding,
                self.scene.plot_area_offset(),
                scene_tree,
                &mut center.element,
                center.placement,
                center_tree,
                renderer,
            );
            layout_children.push(center_node);
        }

        layout::Node::with_children(size, layout_children)
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        shell: &mut crate::core::Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        // Animation tick — independent of capture / tooltip / action so
        // mark sweeps run regardless of which widgets are listening.
        if let Event::Window(window::Event::RedrawRequested(now)) = event {
            advance_mark_animations(tree, *now, self.animate, shell);
        }

        if shell.is_event_captured() {
            return;
        }

        // Forward to the donut center child first when the cursor is
        // inside its bounds. Returning early on capture keeps us from
        // double-delivering to the chart's own click/hover routing.
        if let Kind::Donut(d) = &mut self.kind
            && let Some(center) = &mut d.center
            && let Some(center_layout) = layout.children().nth(1)
            && let Some(center_tree) = tree.children.get_mut(1)
            && cursor.is_over(center_layout.bounds())
        {
            center
                .element
                .as_widget_mut()
                .update(center_tree, event, center_layout, cursor, renderer, shell, viewport);
            if shell.is_event_captured() {
                return;
            }
        }

        let has_tooltip = self.scene.has_tooltip();
        let has_action = self.on_action.is_some();
        let legend_interactive = self.scene.legend().map(|l| l.interactive()).unwrap_or(false);

        // --- Interactive legend click-to-toggle ---
        //
        // Handled before plot-area actions so legend clicks take priority
        // even if an action handler is attached.
        // Hit-test before borrowing widget State mutably below.
        let legend_click =
            if legend_interactive && matches!(event, Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))) {
                self.legend_entry_at(tree, layout, cursor)
            } else {
                None
            };

        let state = tree.state.downcast_mut::<State>();

        if let Some(idx) = legend_click
            && let Some(legend) = self.scene.legend()
        {
            let entry = &legend.entries()[idx];
            if state.hidden_series.contains(&entry.name) {
                state.hidden_series.remove(&entry.name);
            } else {
                state.hidden_series.insert(entry.name.clone());
            }
            shell.capture_event();
            shell.request_redraw();
            return;
        }

        if !has_tooltip && !has_action {
            return;
        }

        // Compute absolute plot area bounds from the layout tree.
        let chart_bounds = layout.bounds();
        let plot_area_offset = self.scene.plot_area_offset();
        let scene_tree = &tree.children[0];
        let plot_area_state = scene_tree.children[6].state.downcast_ref::<plot_area::State>();
        let Some(domain) = &plot_area_state.domain else {
            return;
        };
        let content_rect = plot_area_state.content_rect;
        let plot_bounds = compute_plot_bounds(chart_bounds, self.padding, plot_area_offset, content_rect);

        match event {
            // === HOVER HANDLING ===
            Event::Mouse(mouse::Event::CursorMoved { .. }) => match cursor.position_in(plot_bounds) {
                Some(local) => {
                    let plot_area_tree = &scene_tree.children[6];
                    let new_hover = find_nearest_hover(
                        Point::new(local.x, local.y),
                        plot_area_tree,
                        self.scene.plot_area(),
                        domain,
                        content_rect,
                        plot_area_state.geo_plane.as_ref(),
                    );

                    // `data_x` is snapped to a discrete pixel-x of an actual data
                    // point and projected through a deterministic plane, so
                    // identical hovered points produce identical f64 bits — exact
                    // equality is sufficient (no need for an epsilon shim).
                    let changed = state.hover != new_hover;

                    state.hover = new_hover;
                    state.last_cursor = cursor.position();
                    if changed {
                        shell.request_redraw();
                    }
                }
                None => {
                    if state.hover.is_some() {
                        state.hover = None;
                        state.last_cursor = None;
                        shell.request_redraw();
                    }
                }
            },
            Event::Mouse(mouse::Event::CursorLeft) if state.hover.is_some() => {
                state.hover = None;
                state.last_cursor = None;
                shell.request_redraw();
            }

            // === CLICK HANDLING ===
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
                if has_action && cursor.is_over(plot_bounds) =>
            {
                state.is_pressed = true;
                shell.capture_event();
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) if has_action => {
                let on_action = self.on_action.as_ref().unwrap();

                if state.is_pressed {
                    state.is_pressed = false;
                    shell.capture_event();

                    // Use position_in to get plot-area-local coordinates
                    let Some(local) = cursor.position_in(plot_bounds) else {
                        // Released outside plot area — deselect
                        shell.publish(on_action(Action::Clicked(crate::target::Target::Mark(usize::MAX))));
                        return;
                    };

                    let local = Point::new(local.x, local.y);

                    // Hit-test against bar/pie elements in the tree
                    let plot_area_tree = &scene_tree.children[6];
                    let bars_tag =
                        tree::Tag::of::<plot_area::bars::State<<Renderer as crate::core::text::Renderer>::Paragraph>>();
                    let pie_tag =
                        tree::Tag::of::<plot_area::pie::State<<Renderer as crate::core::text::Renderer>::Paragraph>>();
                    let treemap_tag = tree::Tag::of::<
                        plot_area::treemap::State<<Renderer as crate::core::text::Renderer>::Paragraph>,
                    >();
                    let choropleth_tag = tree::Tag::of::<plot_area::choropleth::State>();

                    // First pass: hit-test labels (labels win when overlapping shapes)
                    for (mark_idx, mark_tree) in plot_area_tree.children.iter().enumerate() {
                        if mark_tree.tag == bars_tag {
                            let bars_state = mark_tree.state.downcast_ref::<plot_area::bars::State<<Renderer as crate::core::text::Renderer>::Paragraph>>();

                            for (series_idx, label_rects) in bars_state.label_rects.iter().enumerate() {
                                for (label_idx, maybe_rect) in label_rects.iter().enumerate() {
                                    if let Some(rect) = maybe_rect
                                        && rect.contains(local)
                                    {
                                        shell.publish(on_action(Action::Clicked(crate::target::Target::EntryLabel {
                                            mark: mark_idx,
                                            series: series_idx,
                                            index: label_idx,
                                        })));
                                        return;
                                    }
                                }
                            }
                        } else if mark_tree.tag == pie_tag {
                            let pie_state = mark_tree
                                .state
                                .downcast_ref::<plot_area::pie::State<<Renderer as crate::core::text::Renderer>::Paragraph>>();

                            for (label_idx, maybe_rect) in pie_state.label_rects.iter().enumerate() {
                                if let Some(rect) = maybe_rect
                                    && rect.contains(local)
                                {
                                    shell.publish(on_action(Action::Clicked(crate::target::Target::EntryLabel {
                                        mark: mark_idx,
                                        series: 0,
                                        index: label_idx,
                                    })));
                                    return;
                                }
                            }
                        }
                    }

                    if let Some(target) = find_geo_feature_click(local, plot_area_tree, self.scene.plot_area()) {
                        shell.publish(on_action(Action::Clicked(target)));
                        return;
                    }

                    // Second pass: hit-test shapes
                    for (mark_idx, mark_tree) in plot_area_tree.children.iter().enumerate() {
                        if mark_tree.tag == bars_tag {
                            let bars_state = mark_tree.state.downcast_ref::<plot_area::bars::State<<Renderer as crate::core::text::Renderer>::Paragraph>>();

                            for (series_idx, rects) in bars_state.series_rects.iter().enumerate() {
                                for (bar_idx, rect) in rects.iter().enumerate() {
                                    if rect.contains(local) {
                                        shell.publish(on_action(Action::Clicked(crate::target::Target::Entry {
                                            mark: mark_idx,
                                            series: series_idx,
                                            index: bar_idx,
                                        })));
                                        return;
                                    }
                                }
                            }
                        } else if mark_tree.tag == pie_tag {
                            let pie_state = mark_tree
                                .state
                                .downcast_ref::<plot_area::pie::State<<Renderer as crate::core::text::Renderer>::Paragraph>>();
                            if let Some(slice_idx) = pie_slice_at(local, pie_state) {
                                shell.publish(on_action(Action::Clicked(crate::target::Target::Entry {
                                    mark: mark_idx,
                                    series: 0,
                                    index: slice_idx,
                                })));
                                return;
                            }
                        } else if mark_tree.tag == treemap_tag {
                            let tm_state = mark_tree
                                .state
                                .downcast_ref::<plot_area::treemap::State<<Renderer as crate::core::text::Renderer>::Paragraph>>();

                            for (item_idx, rect) in tm_state.item_rects.iter().enumerate() {
                                if rect.contains(local) {
                                    shell.publish(on_action(Action::Clicked(crate::target::Target::Entry {
                                        mark: mark_idx,
                                        series: 0,
                                        index: item_idx,
                                    })));
                                    return;
                                }
                            }
                        } else if mark_tree.tag == choropleth_tag
                            && let Some(geo_plane) = plot_area_state.geo_plane.as_ref()
                        {
                            // Bbox pre-filter then ray-cast point-in-polygon.
                            // Geometry lives on the shared geo plane;
                            // the per-mark choropleth state holds only
                            // entry-derived view data.
                            for (feat_idx, &(min_x, min_y, max_x, max_y)) in geo_plane.feature_bboxes.iter().enumerate()
                            {
                                if local.x < min_x || local.x > max_x || local.y < min_y || local.y > max_y {
                                    continue;
                                }
                                // Ray-casting: count crossings of a horizontal ray to the right
                                let mut inside = false;
                                for ring in &geo_plane.projected_polygons[feat_idx] {
                                    let n = ring.len();
                                    let mut j = n.wrapping_sub(1);
                                    for i in 0..n {
                                        let (xi, yi) = ring[i];
                                        let (xj, yj) = ring[j];
                                        if ((yi > local.y) != (yj > local.y))
                                            && (local.x < (xj - xi) * (local.y - yi) / (yj - yi) + xi)
                                        {
                                            inside = !inside;
                                        }
                                        j = i;
                                    }
                                }
                                if inside && let Some(id) = geo_plane.filtered_ids.get(feat_idx) {
                                    shell.publish(on_action(Action::Clicked(crate::target::Target::Feature {
                                        mark: mark_idx,
                                        id: id.clone(),
                                    })));
                                    return;
                                }
                            }
                        }
                    }

                    // No element hit — report empty click
                    shell.publish(on_action(Action::Clicked(crate::target::Target::Mark(usize::MAX))));
                }
            }
            _ => {}
        }
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        defaults: &crate::core::renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        // Get the design to use (explicit or from theme)
        let design: &dyn design::Design = match &self.design {
            Some(d) => d.as_ref(),
            None => theme,
        };

        // Get style from the design and draw background
        let style = (self.style)(design);
        draw_background(renderer, &style, layout.bounds());

        // Draw scene contents (use child layout which accounts for padding)
        let scene_layout = layout.children().next().unwrap();

        let state = tree.state.downcast_ref::<State>();

        self.scene.draw(
            &tree.children[0],
            renderer,
            design,
            defaults,
            scene_layout,
            cursor,
            viewport,
            &state.hidden_series,
            style.corners,
            style.track,
        );

        // Draw the donut center overlay above the scene so the badge
        // text sits over the donut hole, clipped to the chart's bounds.
        // Tooltips draw after this so hover annotations stack on top.
        if let Kind::Donut(d) = &self.kind
            && let Some(center) = &d.center
            && let Some(center_layout) = layout.children().nth(1)
            && let Some(center_tree) = tree.children.get(1)
        {
            use crate::core::renderer::Renderer as _;
            renderer.with_layer(layout.bounds(), |renderer| {
                center.element.as_widget().draw(
                    center_tree,
                    renderer,
                    theme,
                    defaults,
                    center_layout,
                    cursor,
                    viewport,
                );
            });
        }

        // Tooltip overlay draws last so hover annotations composite above
        // every other element, including the donut center overlay.
        // Highlights (tracking line, point markers, polygon stroke,
        // bubble outline) always render. The tooltip box itself is
        // suppressed when a user-installed hover closure is present —
        // [`Chart::overlay`] takes over the box's role with a widget-
        // based annotation.
        if let Some(hover) = &state.hover
            && let Some(tooltip_config) = self.scene.tooltip()
        {
            let scene_tree = &tree.children[0];
            let plot_area_state = scene_tree.children[6].state.downcast_ref::<plot_area::State>();
            if let Some(domain) = &plot_area_state.domain {
                draw_tooltip_overlay(
                    renderer,
                    design,
                    layout.bounds(),
                    self.padding,
                    self.scene.plot_area_offset(),
                    domain,
                    plot_area_state.content_rect,
                    hover,
                    tooltip_config,
                    &self.scene,
                    scene_tree,
                    viewport,
                    cursor,
                    self.hover_fn.is_none(),
                    style.corners,
                );
            }
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        // Interactive legend entries — show a Pointer cursor when hovering
        // over a toggleable swatch so the affordance is clear.
        if self.legend_entry_at(tree, layout, cursor).is_some() {
            return mouse::Interaction::Pointer;
        }

        if self.scene.has_tooltip() {
            let chart_bounds = layout.bounds();
            let plot_area_offset = self.scene.plot_area_offset();
            let scene_tree = &tree.children[0];
            let plot_area_state = scene_tree.children[6].state.downcast_ref::<plot_area::State>();
            if plot_area_state.domain.is_some() {
                let plot_bounds = compute_plot_bounds(
                    chart_bounds,
                    self.padding,
                    plot_area_offset,
                    plot_area_state.content_rect,
                );
                if cursor.is_over(plot_bounds) {
                    return mouse::Interaction::Crosshair;
                }
            }
        }
        mouse::Interaction::None
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        _renderer: &Renderer,
        _viewport: &Rectangle,
        _translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        // Only the user-installed hover closure drives the widget-
        // based annotation overlay. The default tooltip box stays on
        // the canvas-drawn path in `Chart::draw`.
        let hover_fn = self.hover_fn.as_ref()?;

        // Two split borrows from `tree`: the State (for hover /
        // cursor / persisted annotation_tree) lives in `tree.state`;
        // the per-mark plot-area state lives in `tree.children[0]`.
        // Rust's split-borrow rules let us hold both as long as
        // they're disjoint Tree fields.
        let plot_area = self.scene.plot_area();
        let entry;
        let cursor;
        let chart_bounds = layout.bounds();
        let annotation_tree;
        {
            let scene_tree = &tree.children[0];
            let plot_area_state = scene_tree.children[6].state.downcast_ref::<plot_area::State>();
            let state = tree.state.downcast_mut::<State>();
            let geometry = state.hover.as_ref()?;
            entry = build_hover_entry(
                geometry,
                plot_area,
                &scene_tree.children[6],
                plot_area_state.geo_plane.as_ref(),
            )?;
            cursor = state
                .last_cursor
                .unwrap_or_else(|| Point::new(chart_bounds.center_x(), chart_bounds.center_y()));
            annotation_tree = &mut state.annotation_tree;
        }

        let annotation = hover_fn(&entry);
        // Reconcile the persisted tree with this frame's element.
        // When the closure returns the same widget shape (same hover
        // target), per-widget state survives; on shape changes the
        // tree rebuilds.
        annotation_tree.diff(&annotation.content);

        Some(overlay::Element::new(Box::new(AnnotationOverlay {
            annotation,
            tree: annotation_tree,
            cursor,
            chart_bounds,
        })))
    }
}

impl<'a, Message, Design, Theme> Chart<'a, Message, Design, Theme>
where
    Design: design::Design + Clone + 'a,
    Theme: design::Design,
{
    /// Hit-tests the cursor against legend entry rectangles.
    ///
    /// Returns `Some(idx)` if the legend is interactive and the cursor is
    /// over the `idx`-th entry swatch+label, else `None`. Shared between
    /// click handling in `update` and cursor feedback in `mouse_interaction`
    /// so both use the exact same hit region.
    fn legend_entry_at(&self, tree: &Tree, layout: Layout<'_>, cursor: mouse::Cursor) -> Option<usize> {
        let legend = self.scene.legend()?;
        if !legend.interactive() {
            return None;
        }
        let legend_rect = self.scene.legend_bounds()?;
        let chart_bounds = layout.bounds();
        let abs_legend = Rectangle {
            x: chart_bounds.x + self.padding.left + legend_rect.x,
            y: chart_bounds.y + self.padding.top + legend_rect.y,
            width: legend_rect.width,
            height: legend_rect.height,
        };
        let local = cursor.position_in(abs_legend)?;
        let scene_tree = &tree.children[0];
        let legend_tree = &scene_tree.children[1];
        let legend_state = legend_tree
            .state
            .downcast_ref::<legend::State<<Renderer as crate::core::text::Renderer>::Paragraph>>();
        let probe = Point::new(local.x, local.y);
        legend_state.entry_rects.iter().enumerate().find_map(|(i, maybe_rect)| {
            maybe_rect.and_then(|r| r.expand(LEGEND_HIT_PADDING).contains(probe).then_some(i))
        })
    }

    /// Pushes variant-specific renderer state down onto the scene's
    /// series before layout runs.
    ///
    /// For `Kind::Donut(d)`, the **first** `Series::Pie` adopts `d.hole`
    /// and any other pie series stay at `0.0`. For `Kind::Generic`,
    /// every pie is reset to `0.0` so a chart built via `chart(&data)`
    /// always renders as a full pie regardless of how `kind` was set
    /// previously. Called at the top of `layout` so `pie::Pie::layout`
    /// reads the right value.
    fn apply_kind_to_renderer(&mut self) {
        // Split-borrow `kind` and `scene` so we can read `kind` while
        // mutating series through `scene.plot_area_mut()`.
        let Self { kind, scene, .. } = self;
        let series = &mut scene.plot_area_mut().series;

        // Reset every pie to a full-pie default first so the `Donut`
        // arm only has to write the chosen hole onto the first match.
        for s in series.iter_mut() {
            if let plot_area::Series::Pie(pie) = s {
                pie.hole = 0.0;
            }
        }

        match kind {
            Kind::Donut(d) => {
                if let Some(plot_area::Series::Pie(pie)) =
                    series.iter_mut().find(|s| matches!(s, plot_area::Series::Pie(_)))
                {
                    pie.hole = d.hole;
                }
            }
            Kind::Generic => {}
        }
    }
}

impl<'a, Message, Design, Theme> From<Chart<'a, Message, Design, Theme>> for Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Design: design::Design + Clone + 'a,
    Theme: design::Design + 'a,
{
    fn from(chart: Chart<'a, Message, Design, Theme>) -> Self {
        Element::new(chart)
    }
}

/// Draws the background of a [`Chart`] given its [`Style`] and its `bounds`.
pub fn draw_background(renderer: &mut Renderer, style: &Style, bounds: Rectangle) {
    use crate::core::renderer::Renderer as _;

    if style.background.is_some() || style.border.width > 0.0 {
        renderer.fill_quad(
            crate::core::renderer::Quad {
                bounds,
                border: style.border,
                ..Default::default()
            },
            style.background.unwrap_or(crate::core::Color::TRANSPARENT),
        );
    }
}

/// Dispatch the tooltip overlay to the variant-specific renderer.
///
/// Wrapped layering, color resolution, and box layout all live in the
/// inner functions; this just routes Cartesian vs. Pie hovers.
///
/// `draw_box` controls whether the canvas-drawn tooltip box renders
/// at the end of each path. Highlights (tracking line, point
/// markers, polygon strokes, bubble outlines) always render —
/// they're the visual indicator of *what* is hovered, independent
/// of how the data card is presented.
#[allow(clippy::too_many_arguments)]
fn draw_tooltip_overlay<Message>(
    renderer: &mut Renderer,
    design: &dyn design::Design,
    chart_bounds: Rectangle,
    padding: Padding,
    plot_area_offset: Point,
    domain: &plot_area::Domain,
    content_rect: Rectangle,
    hover: &hover::Geometry,
    tooltip_config: &crate::data::tooltip::Tooltip,
    scene: &scene::Scene<'_, Message, Renderer>,
    scene_tree: &Tree,
    viewport: &Rectangle,
    cursor: mouse::Cursor,
    draw_box: bool,
    corners: crate::core::border::Radius,
) {
    if !hover_mark_indices_are_current(hover, scene.plot_area().series.len()) {
        return;
    }

    match hover {
        hover::Geometry::Cartesian { data_x, entries } => draw_cartesian_tooltip_overlay(
            renderer,
            design,
            chart_bounds,
            padding,
            plot_area_offset,
            domain,
            content_rect,
            *data_x,
            entries,
            tooltip_config,
            scene,
            scene_tree,
            viewport,
            draw_box,
        ),
        hover::Geometry::Pie { mark_idx, slice_idx } => draw_pie_tooltip_overlay(
            renderer,
            design,
            chart_bounds,
            padding,
            plot_area_offset,
            domain,
            content_rect,
            *mark_idx,
            *slice_idx,
            tooltip_config,
            scene,
            scene_tree,
            viewport,
            cursor,
            draw_box,
            corners,
        ),
        hover::Geometry::Geographic { mark_idx, point_idx } => draw_geo_tooltip_overlay(
            renderer,
            design,
            chart_bounds,
            padding,
            plot_area_offset,
            domain,
            content_rect,
            *mark_idx,
            *point_idx,
            tooltip_config,
            scene,
            scene_tree,
            viewport,
            draw_box,
        ),
        hover::Geometry::ChoroplethArea { mark_idx, feature_idx } => draw_choropleth_tooltip_overlay(
            renderer,
            design,
            chart_bounds,
            padding,
            plot_area_offset,
            domain,
            content_rect,
            *mark_idx,
            *feature_idx,
            tooltip_config,
            scene,
            scene_tree,
            viewport,
            cursor,
            draw_box,
        ),
    }
}

fn hover_mark_indices_are_current(hover: &hover::Geometry, mark_count: usize) -> bool {
    match hover {
        hover::Geometry::Cartesian { entries, .. } => {
            !entries.is_empty() && entries.iter().all(|(mark_idx, _, _)| *mark_idx < mark_count)
        }
        hover::Geometry::Pie { mark_idx, .. }
        | hover::Geometry::Geographic { mark_idx, .. }
        | hover::Geometry::ChoroplethArea { mark_idx, .. } => *mark_idx < mark_count,
    }
}

/// Draws the Cartesian tooltip overlay (tracking line, markers, box).
///
/// Builds one [`hover::Row`] per matched (mark, series, point) at the
/// snapped data-x, draws the dashed tracking line and any per-mark
/// annotations on a canvas frame, then composites the tooltip box via
/// [`draw_tooltip_box`].
#[allow(clippy::too_many_arguments, clippy::unit_arg)]
fn draw_cartesian_tooltip_overlay<Message>(
    renderer: &mut Renderer,
    design: &dyn design::Design,
    chart_bounds: Rectangle,
    padding: Padding,
    plot_area_offset: Point,
    domain: &plot_area::Domain,
    content_rect: Rectangle,
    data_x: f64,
    cartesian_entries: &[(usize, usize, usize)],
    tooltip_config: &crate::data::tooltip::Tooltip,
    scene: &scene::Scene<'_, Message, Renderer>,
    scene_tree: &Tree,
    viewport: &Rectangle,
    draw_box: bool,
) {
    use crate::core::renderer::Renderer as _;
    use crate::widget::canvas::{Frame, Path, Stroke};
    use crate::widget::renderer::geometry;

    let plot_bounds = compute_plot_bounds(chart_bounds, padding, plot_area_offset, content_rect);
    let plot_area = scene.plot_area();
    let plot_area_tree = &scene_tree.children[6];

    // Horizontal bars invert the axes (see `find_nearest_cartesian_hover`):
    // the tooltip anchors at the bar's value-end / row center rather than
    // the column top, and the box tracks the bar geometry rather than `data_x`.
    let horizontal = plot_area.series.iter().any(
        |s| matches!(s, plot_area::Series::Bars(b) if b.data.direction == crate::mark::bar::Direction::Horizontal),
    );

    let background = design.background_color();
    let text_pair = design.text_pair();
    let seed = design.seed();
    let text_color = design.text_color().resolve(background, text_pair, &seed, None);
    let palette = scene.resolve_palette(design);

    // Compute the tracking pixel x from data_x
    let tracking_pixel_x = {
        let t = if domain.x.max > domain.x.min {
            ((data_x - domain.x.min) / (domain.x.max - domain.x.min)) as f32
        } else {
            0.5
        };
        content_rect.x + t * content_rect.width
    };

    // Build tooltip entries and collect pixel positions
    let line_tag = tree::Tag::of::<plot_area::line::State<<Renderer as crate::core::text::Renderer>::Paragraph>>();
    let area_tag = tree::Tag::of::<plot_area::area::State<<Renderer as crate::core::text::Renderer>::Paragraph>>();
    let xy_tag = tree::Tag::of::<plot_area::xy::State>();
    let bars_tag = tree::Tag::of::<plot_area::bars::State<<Renderer as crate::core::text::Renderer>::Paragraph>>();

    let mut entries: Vec<hover::Row> = Vec::new();

    for &(mark_idx, series_idx, pt_idx) in cartesian_entries {
        let Some(series) = plot_area.series.get(mark_idx) else {
            continue;
        };
        let Some(child) = plot_area_tree.children.get(mark_idx) else {
            continue;
        };

        // Bars resolve their color through the full priority chain
        // (point_colors > color_by > series.color > palette). When this is
        // `Some`, the generic explicit_color/palette-fallback logic below is
        // bypassed. See GOG.md § 8b.
        let mut bars_resolved_color: Option<crate::core::Color> = None;

        let (datum, name, explicit_color, highlight, anchor) = if child.tag == line_tag {
            if let plot_area::Series::Line(line) = series {
                let Some(pt) = line.data.points.get(pt_idx) else {
                    continue;
                };
                let line_state = child
                    .state
                    .downcast_ref::<plot_area::line::State<<Renderer as crate::core::text::Renderer>::Paragraph>>();
                let Some(pixel) = line_state.pixel_points.get(pt_idx).copied() else {
                    continue;
                };
                (
                    crate::data::Datum { x: pt.x, y: pt.y },
                    line.data.name().map(|s| s.to_string()),
                    line.data.color,
                    hover::Highlight::PointMarker { pixel, radius: 4.0 },
                    pixel,
                )
            } else {
                continue;
            }
        } else if child.tag == area_tag {
            if let plot_area::Series::Area(area) = series {
                let Some(ser) = area.data.series.get(series_idx) else {
                    continue;
                };
                let Some(pt) = ser.points.get(pt_idx) else {
                    continue;
                };
                let area_state = child
                    .state
                    .downcast_ref::<plot_area::area::State<<Renderer as crate::core::text::Renderer>::Paragraph>>();
                let Some(pixel) = area_state
                    .series_points
                    .get(series_idx)
                    .and_then(|points| points.get(pt_idx))
                    .copied()
                else {
                    continue;
                };
                (
                    crate::data::Datum { x: pt.x, y: pt.y },
                    ser.name().map(|s| s.to_string()),
                    ser.color,
                    hover::Highlight::PointMarker { pixel, radius: 4.0 },
                    pixel,
                )
            } else {
                continue;
            }
        } else if child.tag == xy_tag {
            if let plot_area::Series::Xy(xy) = series {
                let Some(pt) = xy.data.points.get(pt_idx) else {
                    continue;
                };
                let xy_state = child.state.downcast_ref::<plot_area::xy::State>();
                let Some(pixel) = xy_state.pixel_points.get(pt_idx).copied() else {
                    continue;
                };
                (
                    crate::data::Datum { x: pt.x, y: pt.y },
                    xy.data.name.as_deref().map(|s| s.to_string()),
                    None,
                    hover::Highlight::PointMarker { pixel, radius: 4.0 },
                    pixel,
                )
            } else {
                continue;
            }
        } else if child.tag == bars_tag {
            if let plot_area::Series::Bars(bars) = series {
                let Some(bar_series) = bars.data.series.get(series_idx) else {
                    continue;
                };
                let Some(pt) = bar_series.points.get(pt_idx) else {
                    continue;
                };
                let bars_state = child
                    .state
                    .downcast_ref::<plot_area::bars::State<<Renderer as crate::core::text::Renderer>::Paragraph>>();
                let anchor = bars_state
                    .series_rects
                    .get(series_idx)
                    .and_then(|rects| rects.get(pt_idx))
                    .map(|rect| {
                        if horizontal {
                            // Value-end of the bar, vertically centered on its row.
                            Point::new(rect.x + rect.width, rect.y + rect.height / 2.0)
                        } else {
                            // Top-center of the column.
                            Point::new(rect.x + rect.width / 2.0, rect.y)
                        }
                    })
                    .unwrap_or_else(|| {
                        plot_area::to_pixel(domain, content_rect, crate::data::Datum { x: pt.x, y: pt.y })
                    });

                // Resolve the bar's displayed color via the full priority
                // chain (point_colors > color_by > series.color > palette)
                // so the tooltip swatch matches the actual bar color.
                let color_idx = plot_area.color_offset_for(mark_idx, series_idx);
                let fallback = palette.get(color_idx);
                let resolved = bar_series.resolved_color_at(pt_idx, &seed, scene.user_palette(), fallback);
                bars_resolved_color = Some(resolved.resolve(background, text_pair, &seed, None));

                (
                    crate::data::Datum { x: pt.x, y: pt.y },
                    bar_series.name().map(|s| s.to_string()),
                    None,
                    hover::Highlight::None,
                    anchor,
                )
            } else {
                continue;
            }
        } else {
            continue;
        };

        // Defense in depth: even if a non-finite datum slipped through
        // hover detection, refuse to build a circle / box anchor at NaN —
        // lyon's tessellator panics on PositionIsNaN.
        if !datum.x.is_finite() || !datum.y.is_finite() {
            continue;
        }

        // Resolve series color: bars pre-compute through the priority chain;
        // other marks use explicit mark color if set, else palette fallback.
        let series_color = if let Some(c) = bars_resolved_color {
            c
        } else if let Some(c) = explicit_color {
            c.resolve(background, text_pair, &seed, None)
        } else {
            let color_idx = plot_area.color_offset_for(mark_idx, series_idx);
            palette.get(color_idx).resolve(background, text_pair, &seed, None)
        };

        entries.push(hover::Row {
            tooltip: TooltipEntry {
                x: datum.x,
                y: datum.y,
                series_name: name,
                series_index: series_idx,
                mark_index: mark_idx,
                color: series_color,
            },
            anchor,
            color: series_color,
            highlight,
        });
    }

    if entries.is_empty() {
        return;
    }

    // Canvas geometry and renderer quads are bucketed separately inside
    // a layer. Keep focus chrome and the tooltip box in distinct layers
    // so the box can truly composite above tracking lines and markers.
    let has_continuous = entries
        .iter()
        .any(|re| matches!(re.highlight, hover::Highlight::PointMarker { .. }));
    let has_discrete = entries.iter().any(|re| matches!(re.highlight, hover::Highlight::None));

    // Center on the mean of the entry anchors. Discrete marks (bars) anchor
    // the box on their own geometry — the value-end for horizontal bars, the
    // column top for vertical — rather than the `data_x` tracking position,
    // which only applies to continuous marks.
    let mean_x: f32 = entries.iter().map(|re| re.anchor.x).sum::<f32>() / entries.len() as f32;
    let mean_y: f32 = entries.iter().map(|re| re.anchor.y).sum::<f32>() / entries.len() as f32;
    let anchor_x = if has_discrete && !has_continuous {
        plot_bounds.x + mean_x - content_rect.x
    } else {
        plot_bounds.x + tracking_pixel_x - content_rect.x
    };
    let anchor_y = plot_bounds.y + mean_y - content_rect.y;
    let draw_tracking_line = tooltip_config.tracking_line && has_continuous && !has_discrete;
    if draw_tracking_line || tooltip_config.markers {
        renderer.with_layer(*viewport, |renderer| {
            // --- Draw tracking line and markers via canvas Frame ---
            let frame_size = crate::core::Size::new(content_rect.width, content_rect.height);
            let mut frame = Frame::new(renderer, frame_size);

            // Tracking line — only for continuous marks (line/area/xy), not discrete (bars)
            if draw_tracking_line {
                let line_x = tracking_pixel_x - content_rect.x;
                let tracking_color = crate::core::Color { a: 0.18, ..text_color };
                let path = Path::line(Point::new(line_x, 0.0), Point::new(line_x, content_rect.height));
                let dash_pattern: [f32; 2] = [4.0, 3.0];
                let stroke = Stroke {
                    line_dash: crate::widget::canvas::LineDash {
                        segments: &dash_pattern,
                        offset: 0,
                    },
                    ..Stroke::default().with_width(1.0).with_color(tracking_color)
                };
                frame.stroke(&path, stroke);
            }

            // Hover annotations — each mark type declares its own visual.
            // The cartesian overlay only emits PointMarker / None entries;
            // Ring is produced by the geographic overlay and handled there.
            if tooltip_config.markers {
                for re in &entries {
                    match &re.highlight {
                        hover::Highlight::PointMarker { pixel, radius } => {
                            let cx = pixel.x - content_rect.x;
                            let cy = pixel.y - content_rect.y;

                            let circle = Path::circle(Point::new(cx, cy), *radius);
                            frame.fill(&circle, re.color);

                            let ring = Path::circle(Point::new(cx, cy), *radius);
                            frame.stroke(
                                &ring,
                                Stroke::default().with_width(1.5).with_color(crate::core::Color::WHITE),
                            );
                        }
                        hover::Highlight::None => {}
                        hover::Highlight::Ring { .. } => {}
                        hover::Highlight::Stroke { .. } => {}
                    }
                }
            }

            renderer.with_translation(crate::core::Vector::new(plot_bounds.x, plot_bounds.y), |renderer| {
                geometry::Renderer::draw_geometry(renderer, frame.into_geometry());
            });
        });
    }

    if draw_box {
        // Flip the box inward only when it would run past the plot's right
        // edge, so a bar tip with room to its right keeps the tooltip outside.
        let right_limit_x = plot_bounds.x + content_rect.width;
        renderer.with_layer(*viewport, |renderer| {
            draw_tooltip_box(
                renderer,
                design,
                tooltip_config,
                &entries,
                Point::new(anchor_x, anchor_y),
                right_limit_x,
                chart_bounds,
                text_color,
                viewport,
            );
        });
    }
}

/// Draws the pie hover tooltip — a single-row box anchored at the cursor.
///
/// Unlike Cartesian tooltips, pie hover has no tracking line or point
/// marker: the hovered slice outline is the visual annotation. The synthesized
/// [`TooltipEntry`] uses `x = slice_idx as f64` and `y = slice.value`
/// so any user-supplied `tooltip_config.format` closure can read the
/// slice value uniformly with line/bar entries.
#[allow(clippy::too_many_arguments)]
fn draw_pie_tooltip_overlay<Message>(
    renderer: &mut Renderer,
    design: &dyn design::Design,
    chart_bounds: Rectangle,
    padding: Padding,
    plot_area_offset: Point,
    _domain: &plot_area::Domain,
    content_rect: Rectangle,
    mark_idx: usize,
    slice_idx: usize,
    tooltip_config: &crate::data::tooltip::Tooltip,
    scene: &scene::Scene<'_, Message, Renderer>,
    scene_tree: &Tree,
    viewport: &Rectangle,
    cursor: mouse::Cursor,
    draw_box: bool,
    corners: crate::core::border::Radius,
) {
    use crate::core::renderer::Renderer as _;
    use crate::widget::canvas::{Frame, Stroke};
    use crate::widget::renderer::geometry;

    let plot_area = scene.plot_area();
    let plot_area_tree = &scene_tree.children[6];

    let Some(series) = plot_area.series.get(mark_idx) else {
        return;
    };
    let plot_area::Series::Pie(pie) = series else {
        return;
    };
    let Some(slice) = pie.data.slices.get(slice_idx) else {
        return;
    };
    let Some(child) = plot_area_tree.children.get(mark_idx) else {
        return;
    };
    if child.tag != tree::Tag::of::<plot_area::pie::State<<Renderer as crate::core::text::Renderer>::Paragraph>>() {
        return;
    }
    let pie_state = child
        .state
        .downcast_ref::<plot_area::pie::State<<Renderer as crate::core::text::Renderer>::Paragraph>>();

    let background = design.background_color();
    let text_pair = design.text_pair();
    let seed = design.seed();
    let text_color = design.text_color().resolve(background, text_pair, &seed, None);
    let palette = scene.resolve_palette(design);

    let slice_color = if let Some(c) = slice.get_color() {
        c.resolve(background, text_pair, &seed, None)
    } else {
        let color_idx = plot_area.color_offset_for(mark_idx, 0) + slice_idx;
        palette.get(color_idx).resolve(background, text_pair, &seed, None)
    };

    let Some(cursor_pos) = cursor.position() else {
        return;
    };
    let plot_bounds = compute_plot_bounds(chart_bounds, padding, plot_area_offset, content_rect);
    let right_limit_x = plot_bounds.x + plot_bounds.width;

    let entry = hover::Row {
        tooltip: TooltipEntry {
            x: slice_idx as f64,
            y: slice.value(),
            series_name: slice.get_name().map(|s| s.to_string()),
            series_index: 0,
            mark_index: mark_idx,
            color: slice_color,
        },
        anchor: cursor_pos,
        color: slice_color,
        highlight: hover::Highlight::None,
    };
    let entries = [entry];

    // When the user hasn't supplied an explicit `Tooltip::format`, swap
    // in a chain-aware closure so the hover string matches the legend
    // column. The user's override (when set) keeps its row-level
    // freedom and stays untouched.
    let chain_tooltip;
    let effective_tooltip: &crate::data::tooltip::Tooltip = if tooltip_config.format_is_default() {
        let mark_format = scene.primary_mark_value_format(mark_idx).cloned();
        chain_tooltip = tooltip_config.clone().format(move |entry: &TooltipEntry| {
            let formatted = match &mark_format {
                Some(f) => f(&entry.y),
                None => crate::scale::default_f64_format(entry.y),
            };
            match &entry.series_name {
                Some(name) => format!("{name}: {formatted}"),
                None => formatted,
            }
        });
        &chain_tooltip
    } else {
        tooltip_config
    };
    let effective_tooltip = effective_tooltip.clone().swatch(false);

    if tooltip_config.markers
        && let Some(&(start, end)) = pie_state.slice_angles.get(slice_idx)
        && (end - start).abs() > f32::EPSILON
    {
        // Build the hover outline from the same `slice_path` (explode +
        // rounding) the fill uses, so the highlight always traces the
        // exact petal shape.
        let corner = corners.top_left.max(0.0);
        let gap_offset = plot_area::pie::effective_gap(pie.data.gap, corner, pie.data.slices.len()) / 2.0;
        let path = plot_area::pie::slice_path(
            Point::new(pie_state.center.0, pie_state.center.1),
            pie_state.inner_radius,
            pie_state.outer_radius,
            start,
            end,
            gap_offset,
            corner,
        );
        renderer.with_layer(*viewport, |renderer| {
            let frame_size = crate::core::Size::new(content_rect.width, content_rect.height);
            let mut frame = Frame::new(renderer, frame_size);
            frame.stroke(
                &path,
                Stroke::default()
                    .with_color(pie_hover_stroke(background))
                    .with_width(1.0),
            );
            renderer.with_translation(crate::core::Vector::new(plot_bounds.x, plot_bounds.y), |renderer| {
                #[allow(clippy::unit_arg)]
                geometry::Renderer::draw_geometry(renderer, frame.into_geometry());
            });
        });
    }

    if draw_box {
        renderer.with_layer(*viewport, |renderer| {
            draw_tooltip_box(
                renderer,
                design,
                &effective_tooltip,
                &entries,
                cursor_pos,
                right_limit_x,
                chart_bounds,
                text_color,
                viewport,
            );
        });
    }
}

fn pie_hover_stroke(background: crate::core::Color) -> crate::core::Color {
    if crate::palette::is_dark_background(background) {
        crate::core::Color::from_rgba(1.0, 1.0, 1.0, 0.5)
    } else {
        crate::core::Color::from_rgba(0.0, 0.0, 0.0, 0.5)
    }
}

/// Draws the geographic hover overlay — a stroke ring around the
/// hovered bubble and a side-anchored tooltip box.
///
/// No tracking line: the bubble is itself the visual focus, and a
/// dashed crosshair across longitudes would carry no useful meaning
/// on a projected map. The tooltip box anchors at the bubble center
/// and clamps to the chart bounds via the existing flip-axis logic.
#[allow(clippy::too_many_arguments, clippy::unit_arg)]
fn draw_geo_tooltip_overlay<Message>(
    renderer: &mut Renderer,
    design: &dyn design::Design,
    chart_bounds: Rectangle,
    padding: Padding,
    plot_area_offset: Point,
    _domain: &plot_area::Domain,
    content_rect: Rectangle,
    mark_idx: usize,
    point_idx: usize,
    tooltip_config: &crate::data::tooltip::Tooltip,
    scene: &scene::Scene<'_, Message, Renderer>,
    scene_tree: &Tree,
    viewport: &Rectangle,
    draw_box: bool,
) {
    use crate::core::renderer::Renderer as _;
    use crate::widget::canvas::{Frame, Path, Stroke};
    use crate::widget::renderer::geometry;

    let plot_area = scene.plot_area();
    let plot_area_tree = &scene_tree.children[6];

    let Some(series) = plot_area.series.get(mark_idx) else {
        return;
    };
    let plot_area::Series::Xy(xy) = series else {
        return;
    };
    let Some(child) = plot_area_tree.children.get(mark_idx) else {
        return;
    };
    if child.tag != tree::Tag::of::<plot_area::xy::State>() {
        return;
    }
    let xy_state = child.state.downcast_ref::<plot_area::xy::State>();
    let Some(pixel) = xy_state.pixel_points.get(point_idx).copied() else {
        return;
    };
    let Some(point) = xy.data.points.get(point_idx).copied() else {
        return;
    };
    let diameter = xy_state
        .resolved_sizes
        .get(point_idx)
        .copied()
        .unwrap_or(xy.data.marker.size);
    let radius = diameter / 2.0;

    let background = design.background_color();
    let text_pair = design.text_pair();
    let seed = design.seed();
    let text_color = design.text_color().resolve(background, text_pair, &seed, None);
    let palette = scene.resolve_palette(design);

    let bubble_color = if let Some(c) = xy.data.color {
        c.resolve(background, text_pair, &seed, None)
    } else {
        let color_idx = plot_area.color_offset_for(mark_idx, 0);
        palette.get(color_idx).resolve(background, text_pair, &seed, None)
    };

    // Per-point label and tooltip value (populated for geo bubble points
    // built via `bubble_map(...)`; empty for plain `xy(...).on_geo()`).
    let point_label = xy.data.labels.get(point_idx).cloned().flatten();
    let point_value = xy.data.tooltip_values.get(point_idx).copied().flatten();

    // Tooltip text: prefer the per-point label as the descriptor when
    // present, falling back to the series name. The value runs through
    // the chart's value-format chain so a user-supplied `Data::value_format`
    // (e.g. `|v| format!("${v:.0}K")`) styles the magnitude consistently
    // with the legend.
    let chain_tooltip;
    let effective_tooltip: &crate::data::tooltip::Tooltip = if tooltip_config.format_is_default() {
        let mark_format = scene.primary_mark_value_format(mark_idx).cloned();
        let series_name = xy.data.name.clone();
        let label = point_label.clone();
        let value = point_value.unwrap_or(point.y);
        chain_tooltip = tooltip_config.clone().format(move |_entry: &TooltipEntry| {
            let formatted = match &mark_format {
                Some(f) => f(&value),
                None => crate::scale::default_f64_format(value),
            };
            let descriptor = label.as_deref().or(series_name.as_deref());
            match descriptor {
                Some(d) => format!("{d}: {formatted}"),
                None => formatted,
            }
        });
        &chain_tooltip
    } else {
        tooltip_config
    };

    let plot_bounds = compute_plot_bounds(chart_bounds, padding, plot_area_offset, content_rect);
    let anchor_abs = Point::new(
        plot_bounds.x + pixel.x - content_rect.x,
        plot_bounds.y + pixel.y - content_rect.y,
    );
    // Place the box on the side of the bubble that points away from the
    // closest chart edge, with its near edge sitting `radius + 8px` past
    // the bubble's outline. `draw_tooltip_box` prefers the right and flips
    // left when the box would pass `right_limit_x`, so passing `chart_right`
    // forces a right box (it always fits) and `chart_left` forces a left box
    // (it never fits). Anchoring at the bubble's edge (rather than its
    // center) means the box's `box_gap` shim lands fully outside the disc.
    let chart_center_x = chart_bounds.x + chart_bounds.width / 2.0;
    let on_left_half = anchor_abs.x < chart_center_x;
    let (box_anchor_x, right_limit_x) = if on_left_half {
        (anchor_abs.x + radius, chart_bounds.x + chart_bounds.width)
    } else {
        (anchor_abs.x - radius, chart_bounds.x)
    };

    let entry = hover::Row {
        tooltip: TooltipEntry {
            x: point.x,
            y: point.y,
            series_name: xy.data.name.as_deref().map(|s| s.to_string()),
            series_index: 0,
            mark_index: mark_idx,
            color: bubble_color,
        },
        anchor: anchor_abs,
        color: bubble_color,
        highlight: hover::Highlight::Ring {
            pixel,
            radius: radius + 2.0,
            color: bubble_color,
            width: 1.5,
        },
    };
    let entries = [entry];

    if tooltip_config.markers {
        renderer.with_layer(*viewport, |renderer| {
            // Ring around the hovered bubble. Drawn on a frame translated
            // to the plot bounds so coordinates match the cartesian path.
            let frame_size = crate::core::Size::new(content_rect.width, content_rect.height);
            let mut frame = Frame::new(renderer, frame_size);
            for re in &entries {
                match &re.highlight {
                    hover::Highlight::Ring {
                        pixel,
                        radius,
                        color,
                        width,
                    } => {
                        let cx = pixel.x - content_rect.x;
                        let cy = pixel.y - content_rect.y;
                        let path = Path::circle(Point::new(cx, cy), *radius);
                        frame.stroke(&path, Stroke::default().with_width(*width).with_color(*color));
                    }
                    hover::Highlight::PointMarker { .. } => {}
                    hover::Highlight::None => {}
                    hover::Highlight::Stroke { .. } => {}
                }
            }
            renderer.with_translation(crate::core::Vector::new(plot_bounds.x, plot_bounds.y), |renderer| {
                geometry::Renderer::draw_geometry(renderer, frame.into_geometry());
            });
        });
    }

    if draw_box {
        renderer.with_layer(*viewport, |renderer| {
            let box_anchor = Point::new(box_anchor_x, anchor_abs.y);
            draw_tooltip_box(
                renderer,
                design,
                effective_tooltip,
                &entries,
                box_anchor,
                right_limit_x,
                chart_bounds,
                text_color,
                viewport,
            );
        });
    }
}

/// Draws the choropleth hover overlay — a stroke around the hovered
/// polygon's projected rings and a cursor-anchored tooltip box.
///
/// The hovered area carries no point-marker concept; the polygon itself
/// is the visual focus. The tooltip box anchors at the cursor and
/// flips left/right around the chart's mid-x via the same logic
/// [`draw_tooltip_box`] uses for cartesian charts.
#[allow(clippy::too_many_arguments, clippy::unit_arg)]
fn draw_choropleth_tooltip_overlay<Message>(
    renderer: &mut Renderer,
    design: &dyn design::Design,
    chart_bounds: Rectangle,
    padding: Padding,
    plot_area_offset: Point,
    _domain: &plot_area::Domain,
    content_rect: Rectangle,
    mark_idx: usize,
    feature_idx: usize,
    tooltip_config: &crate::data::tooltip::Tooltip,
    scene: &scene::Scene<'_, Message, Renderer>,
    scene_tree: &Tree,
    viewport: &Rectangle,
    cursor: mouse::Cursor,
    draw_box: bool,
) {
    use crate::core::renderer::Renderer as _;
    use crate::widget::canvas::{Frame, Path, Stroke};
    use crate::widget::renderer::geometry;

    let plot_area_state = scene_tree.children[6].state.downcast_ref::<plot_area::State>();
    let Some(geo_plane) = &plot_area_state.geo_plane else {
        return;
    };
    let plot_area = scene.plot_area();
    let Some(series) = plot_area.series.get(mark_idx) else {
        return;
    };
    let plot_area::Series::Choropleth(c) = series else {
        return;
    };
    let Some(feature_id) = geo_plane.filtered_ids.get(feature_idx).cloned() else {
        return;
    };
    let Some(rings) = geo_plane.projected_polygons.get(feature_idx) else {
        return;
    };

    // Look up the entry value (None when the feature has an
    // "available, no value" entry, or no entry at all).
    let entry_value = c
        .data
        .entries()
        .iter()
        .find(|e| e.id() == &feature_id)
        .and_then(|e| e.value());

    // Look up the human-readable feature name from the GeoData. Falls
    // back to the id when the feature has no `name` property.
    let feature_name = geo_plane
        .geo
        .as_ref()
        .and_then(|geo| geo.get(feature_id.clone()))
        .map(|f| f.name.clone())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| feature_id.to_string());

    let background = design.background_color();
    let text_pair = design.text_pair();
    let seed = design.seed();
    let text_color = design.text_color().resolve(background, text_pair, &seed, None);
    let palette = scene.resolve_palette(design);

    // Hovered-feature outline: a user-supplied closure on the mark
    // runs with the resolved design + palette in scope, mirroring
    // iced's theme-aware style closures. Without one, fall back to
    // the theme-derived defaults baked into [`HoverStyle::from_theme`].
    let style = match c.data.hover_style_fn() {
        Some(f) => f(design, &palette),
        None => crate::mark::choropleth::HoverStyle::from_theme(design),
    };
    let stroke_color = style.outline_color;
    let stroke_width = style.outline_width;

    // Swatch color: the resolved fill the choropleth painted for this
    // feature, snapshot at draw-time into `state.fill_colors` (aligned
    // 1:1 with `geo_plane.filtered_ids`). Falls back to the stroke
    // color when the snapshot is missing — only happens before the
    // first draw, in which case the swatch reads as a neutral chip.
    let swatch_color = {
        let plot_area_tree = &scene_tree.children[6];
        plot_area_tree
            .children
            .get(mark_idx)
            .filter(|child| child.tag == tree::Tag::of::<plot_area::choropleth::State>())
            .and_then(|child| {
                let state = child.state.downcast_ref::<plot_area::choropleth::State>();
                state.fill_colors.borrow().get(feature_idx).copied()
            })
            .unwrap_or(stroke_color)
    };

    let Some(cursor_pos) = cursor.position() else {
        return;
    };
    let plot_bounds = compute_plot_bounds(chart_bounds, padding, plot_area_offset, content_rect);
    let right_limit_x = plot_bounds.x + plot_bounds.width;

    let entry = hover::Row {
        tooltip: TooltipEntry {
            x: feature_idx as f64,
            y: entry_value.unwrap_or(f64::NAN),
            series_name: Some(feature_name),
            series_index: 0,
            mark_index: mark_idx,
            color: swatch_color,
        },
        anchor: cursor_pos,
        color: swatch_color,
        highlight: hover::Highlight::Stroke {
            rings: std::sync::Arc::new(rings.clone()),
            color: stroke_color,
            width: stroke_width,
        },
    };
    let entries = [entry];

    // Honor the chart-level value-format chain when the user hasn't
    // installed a custom format. Mirrors the choropleth legend's
    // chain — guide-level legend override, then mark-level
    // `ColorScale.format`, then the K/M/B abbreviation fallback —
    // so hover values pick up the same units the legend shows.
    let chain_tooltip;
    let effective_tooltip: &crate::data::tooltip::Tooltip = if tooltip_config.format_is_default() {
        let resolved_format = c
            .data
            .legend_config()
            .and_then(|l| l.value_format_ref().cloned())
            .or_else(|| c.data.color_scale_format())
            .or_else(|| scene.primary_mark_value_format(mark_idx).cloned());
        chain_tooltip = tooltip_config.clone().format(move |entry: &TooltipEntry| {
            let name = entry.series_name.clone().unwrap_or_default();
            if entry.y.is_nan() {
                return name;
            }
            let formatted = match &resolved_format {
                Some(f) => f(&entry.y),
                None => plot_area::choropleth::format_legend_value(entry.y),
            };
            if name.is_empty() {
                formatted
            } else {
                format!("{name}: {formatted}")
            }
        });
        &chain_tooltip
    } else {
        tooltip_config
    };

    if tooltip_config.markers {
        renderer.with_layer(*viewport, |renderer| {
            let frame_size = crate::core::Size::new(content_rect.width, content_rect.height);
            let mut frame = Frame::new(renderer, frame_size);
            for re in &entries {
                if let hover::Highlight::Stroke { rings, color, width } = &re.highlight {
                    for ring in rings.iter() {
                        if ring.len() < 3 {
                            continue;
                        }
                        let path = Path::new(|builder| {
                            let (x0, y0) = ring[0];
                            builder.move_to(Point::new(x0 - content_rect.x, y0 - content_rect.y));
                            for &(x, y) in &ring[1..] {
                                builder.line_to(Point::new(x - content_rect.x, y - content_rect.y));
                            }
                            builder.close();
                        });
                        frame.stroke(&path, Stroke::default().with_width(*width).with_color(*color));
                    }
                }
            }
            renderer.with_translation(crate::core::Vector::new(plot_bounds.x, plot_bounds.y), |renderer| {
                geometry::Renderer::draw_geometry(renderer, frame.into_geometry());
            });
        });
    }

    if draw_box {
        renderer.with_layer(*viewport, |renderer| {
            draw_tooltip_box(
                renderer,
                design,
                effective_tooltip,
                &entries,
                cursor_pos,
                right_limit_x,
                chart_bounds,
                text_color,
                viewport,
            );
        });
    }
}

/// Render the tooltip box body (background, swatches, text rows).
///
/// The caller picks the visual `anchor`, the `right_limit_x` past which a
/// right-placed box is considered to overflow, and the `clamp_bounds`
/// rectangle the box is kept inside. The box prefers sitting to the right of
/// the anchor and flips to the left only when its right edge would exceed
/// `right_limit_x` — so a tooltip stays outside the bar tip whenever there's
/// room, instead of flipping at a fixed midpoint. Pass `right_limit_x =
/// chart_right` to force right, or `chart_left` to force left (the box can
/// never fit, so it always flips). Returns nothing — the caller is
/// responsible for the surrounding layering wrapper.
#[allow(clippy::too_many_arguments)]
fn draw_tooltip_box(
    renderer: &mut Renderer,
    design: &dyn design::Design,
    tooltip_config: &crate::data::tooltip::Tooltip,
    entries: &[hover::Row],
    anchor: Point,
    right_limit_x: f32,
    clamp_bounds: Rectangle,
    text_color: crate::core::Color,
    viewport: &Rectangle,
) {
    use crate::core::renderer::Renderer as _;
    use crate::core::text::Renderer as _;

    let font = renderer.default_font();
    // Scale the tooltip with the chart's base font so it grows with the
    // design. The 1.5x line height reproduces the historical 18px at the
    // default 12px size, so existing charts are unchanged.
    let font_size: f32 = design.font_size();
    let line_height_px: f32 = font_size * 1.5;
    let swatch_size: f32 = 8.0;
    let swatch_gap: f32 = 6.0;
    let box_padding: f32 = 8.0;
    // Asymmetric right padding: the swatch circle has built-in
    // whitespace around its glyph mass, so the eye reads the left
    // gap as ~10px even though `box_padding` is 8. Match that on
    // the right by widening the right margin past the rightmost
    // glyph, since text bounds are flush to the digit's right edge.
    let box_padding_right: f32 = box_padding + 4.0;
    let box_gap: f32 = 8.0;

    let formatted: Vec<String> = entries.iter().map(|re| (tooltip_config.format)(&re.tooltip)).collect();

    let char_width = font_size * 0.58;
    let max_text_width: f32 = formatted
        .iter()
        .map(|s| s.len() as f32 * char_width)
        .fold(0.0f32, f32::max);

    let swatch_space = if tooltip_config.swatch {
        swatch_size + swatch_gap
    } else {
        0.0
    };
    let box_width = box_padding + swatch_space + max_text_width + box_padding_right;
    let box_height = box_padding * 2.0 + entries.len() as f32 * line_height_px;

    // Prefer placing the box just past the anchor (right of the bar tip /
    // cursor); flip it to the left only when its right edge would exceed
    // `right_limit_x` — so it stays outside whenever there's room rather than
    // flipping at a fixed midpoint. Then clamp horizontally so it never
    // clips, even when the flipped box would run off the left edge.
    let right_x = anchor.x + box_gap;
    let box_x = if right_x + box_width <= right_limit_x {
        right_x
    } else {
        anchor.x - box_width - box_gap
    };
    let box_x = box_x
        .max(clamp_bounds.x + 2.0)
        .min(clamp_bounds.x + clamp_bounds.width - box_width - 2.0);

    let box_y = (anchor.y - box_height / 2.0)
        .max(clamp_bounds.y + 2.0)
        .min(clamp_bounds.y + clamp_bounds.height - box_height - 2.0);

    let background = design.background_color();
    let text_pair = design.text_pair();
    let seed = design.seed();

    // Tooltip background — opaque by default (ensures readability on transparent charts)
    let tooltip_bg = crate::core::Color { a: 1.0, ..background };
    let divider_color = design.divider_color().resolve(background, text_pair, &seed, None);

    renderer.fill_quad(
        crate::core::renderer::Quad {
            bounds: Rectangle {
                x: box_x,
                y: box_y,
                width: box_width,
                height: box_height,
            },
            border: crate::core::Border {
                width: 1.0,
                radius: 4.0.into(),
                color: divider_color,
            },
            ..Default::default()
        },
        tooltip_bg,
    );

    // Draw each entry line
    for (i, (re, text)) in entries.iter().zip(formatted.iter()).enumerate() {
        let row_y = box_y + box_padding + i as f32 * line_height_px;
        let mut text_x = box_x + box_padding;

        // Colored swatch circle (if enabled)
        if tooltip_config.swatch {
            let swatch_y = row_y + (line_height_px - swatch_size) / 2.0;
            renderer.fill_quad(
                crate::core::renderer::Quad {
                    bounds: Rectangle {
                        x: text_x,
                        y: swatch_y,
                        width: swatch_size,
                        height: swatch_size,
                    },
                    border: crate::core::Border {
                        radius: (swatch_size / 2.0).into(),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                re.color,
            );
            text_x += swatch_size + swatch_gap;
        }

        // Text label — optionally colored to match series
        let label_color = if tooltip_config.colored_text {
            re.color
        } else {
            text_color
        };

        renderer.fill_text(
            crate::core::text::Text {
                content: text.clone(),
                bounds: Size::new(max_text_width + 10.0, line_height_px),
                size: font_size.into(),
                font,
                align_x: crate::core::alignment::Horizontal::Left.into(),
                align_y: crate::core::alignment::Vertical::Top,
                line_height: crate::core::text::LineHeight::default(),
                shaping: crate::core::text::Shaping::Basic,
                wrapping: crate::core::text::Wrapping::None,
                ellipsis: crate::core::text::Ellipsis::default(),
                hint_factor: renderer.scale_factor(),
                font_features: Vec::new(),
                font_variations: Vec::new(),
                letter_spacing: Default::default(),
                weight: None,
            },
            Point::new(text_x, row_y + (line_height_px - font_size) / 2.0),
            label_color,
            *viewport,
        );
    }
}

/// The appearance of a chart.
///
/// Construct via the built-in fns ([`default`], [`filled`], …) and extend
/// with struct-update syntax — `Style { corners: 6.0.into(), ..default(d) }`
/// — matching the iced ecosystem convention for style structs.
#[derive(Debug, Clone, Copy, Default)]
pub struct Style {
    /// The background color of the chart.
    pub background: Option<crate::core::Color>,
    /// The border of the chart.
    pub border: crate::core::Border,
    /// Corner rounding for roundable marks: bar/column value-ends, pie &
    /// donut slices, and the gauge value arc.
    ///
    /// Per-corner pixel radii for rectangular bars (Recharts `radius`);
    /// the uniform value is taken as the cap radius for polar marks
    /// (Recharts `cornerRadius`). The built-in style fns seed this from
    /// [`design::Design::corners`], so a theme default flows through;
    /// override it inside [`Chart::style`] to round per chart.
    pub corners: crate::core::border::Radius,
    /// Fill for the "track" rail behind bars: the faint full-axis-extent
    /// rectangle a bar grows within (Recharts `<Bar background>`). `None`
    /// disables it.
    ///
    /// Seeded from [`design::Design::bar_track`], so a theme default flows
    /// through; override inside [`Chart::style`] to toggle per chart. The
    /// rail adopts the same [`corners`](Self::corners) radius and renders as
    /// a full pill (both ends rounded); bars then round their baseline end
    /// too, so they read as free-floating pills resting on the rail.
    pub track: Option<crate::core::Color>,
}

/// A styling function for a [`Chart`].
pub type StyleFn<'a> = Box<dyn Fn(&dyn design::Design) -> Style + 'a>;

/// The default [`Chart`] style with background color and thin rounded border.
pub fn default(design: &dyn design::Design) -> Style {
    Style {
        background: Some(design.background_color()),
        border: crate::core::Border {
            width: 1.0,
            radius: 5.0.into(),
            color: design
                .divider_color()
                .resolve(design.background_color(), design.text_pair(), &design.seed(), None),
        },
        corners: design.corners(),
        track: design.bar_track(),
    }
}

/// A transparent [`Chart`] with no background or border. Mark corner
/// rounding still follows the design — the frame is bare, not the marks.
pub fn transparent(design: &dyn design::Design) -> Style {
    Style {
        corners: design.corners(),
        track: design.bar_track(),
        ..Style::default()
    }
}

/// A [`Chart`] with thin square border and no background.
pub fn bordered(design: &dyn design::Design) -> Style {
    Style {
        background: None,
        border: crate::core::Border {
            width: 1.0,
            radius: 0.0.into(),
            color: design
                .divider_color()
                .resolve(design.background_color(), design.text_pair(), &design.seed(), None),
        },
        corners: design.corners(),
        track: design.bar_track(),
    }
}

/// A [`Chart`] with a solid background color and thin rounded border.
pub fn filled(design: &dyn design::Design) -> Style {
    Style {
        background: Some(design.background_color()),
        border: crate::core::Border {
            width: 1.0,
            radius: 5.0.into(),
            color: design
                .divider_color()
                .resolve(design.background_color(), design.text_pair(), &design.seed(), None),
        },
        corners: design.corners(),
        track: design.bar_track(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn geo_point_feature_target_requires_geo_xy_with_id() {
        let bubbles = crate::bubble_map([
            crate::map_point(42.0, -72.0, 10.0).id("REG-NE"),
            crate::map_point(34.0, -118.0, 20.0),
        ]);

        assert_eq!(
            geo_feature_target_for_point(&bubbles, 1, 0),
            Some(crate::target::Target::Feature {
                mark: 1,
                id: "REG-NE".into(),
            }),
        );
        assert!(
            geo_feature_target_for_point(&bubbles, 1, 1).is_none(),
            "geo bubbles without ids remain click-silent",
        );

        let cartesian = crate::xy([(0.0, 0.0)]);
        assert!(
            geo_feature_target_for_point(&cartesian, 1, 0).is_none(),
            "cartesian xy marks do not emit geo feature targets",
        );
    }

    #[test]
    fn choropleth_hover_requires_finite_value_entry() {
        let entries = vec![
            crate::choropleth_entry("CA", 10.0),
            crate::choropleth_entry_available("TX"),
            crate::choropleth_entry("NY", f64::NAN),
        ];
        let choropleth = crate::choropleth(entries);

        assert!(choropleth_feature_has_hover_value(&choropleth, &"CA".into()));
        assert!(
            !choropleth_feature_has_hover_value(&choropleth, &"TX".into()),
            "available/no-value features should not show hover chrome"
        );
        assert!(
            !choropleth_feature_has_hover_value(&choropleth, &"NY".into()),
            "non-finite values render as available and should not hover"
        );
        assert!(!choropleth_feature_has_hover_value(&choropleth, &"WA".into()));
    }

    #[test]
    fn stale_hover_mark_indices_are_rejected_after_mark_set_changes() {
        assert!(!hover_mark_indices_are_current(
            &hover::Geometry::Geographic {
                mark_idx: 1,
                point_idx: 0,
            },
            1,
        ));
        assert!(!hover_mark_indices_are_current(
            &hover::Geometry::ChoroplethArea {
                mark_idx: 1,
                feature_idx: 0,
            },
            1,
        ));
        assert!(!hover_mark_indices_are_current(
            &hover::Geometry::Pie {
                mark_idx: 1,
                slice_idx: 0,
            },
            1,
        ));
        assert!(!hover_mark_indices_are_current(
            &hover::Geometry::Cartesian {
                data_x: 0.0,
                entries: vec![(0, 0, 0), (1, 0, 0)],
            },
            1,
        ));
    }

    #[test]
    fn current_hover_mark_indices_are_accepted() {
        assert!(hover_mark_indices_are_current(
            &hover::Geometry::Geographic {
                mark_idx: 1,
                point_idx: 0,
            },
            2,
        ));
        assert!(hover_mark_indices_are_current(
            &hover::Geometry::Cartesian {
                data_x: 0.0,
                entries: vec![(0, 0, 0), (1, 0, 0)],
            },
            2,
        ));
    }
}
