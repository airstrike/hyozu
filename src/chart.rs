pub mod donut;
pub mod guide;
pub(crate) mod hover;
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
    Element, Event, Layout, Length, Padding, Point, Rectangle, Shell, Size, Widget, layout, mouse, window,
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
#[derive(Default)]
struct State {
    generation: u64,
    is_pressed: bool,
    hover: Option<hover::Geometry>,
    /// Names of series hidden via legend click toggles. Lives with the
    /// widget state and is reset when the tree is dropped.
    hidden_series: std::collections::HashSet<String>,
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
            _theme: std::marker::PhantomData,
        }
    }

    /// Sets the action handler for the chart.
    pub fn on_action(mut self, f: impl Fn(Action) -> Message + 'a) -> Self {
        self.on_action = Some(Box::new(f));
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

/// Compute the absolute plot bounds from chart bounds, padding, and plane.
fn compute_plot_bounds(
    chart_bounds: Rectangle,
    padding: Padding,
    plot_area_offset: Point,
    plane: &plot_area::Plane,
) -> Rectangle {
    Rectangle {
        x: chart_bounds.x + padding.left + plot_area_offset.x + plane.bounds.x,
        y: chart_bounds.y + padding.top + plot_area_offset.y + plane.bounds.y,
        width: plane.bounds.width,
        height: plane.bounds.height,
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
    plane: &plot_area::Plane,
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
    find_nearest_cartesian_hover(local, plot_area_tree, plot_area, plane)
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
    plane: &plot_area::Plane,
) -> Option<hover::Geometry> {
    let line_tag = tree::Tag::of::<plot_area::line::State>();
    let area_tag = tree::Tag::of::<plot_area::area::State>();
    let xy_tag = tree::Tag::of::<plot_area::xy::State>();
    let bars_tag = tree::Tag::of::<plot_area::bars::State>();

    let is_geo_xy = |mark_idx: usize| {
        matches!(
            plot_area.series.get(mark_idx),
            Some(plot_area::Series::Xy(xy)) if xy.data.coord_kind == crate::mark::xy::CoordKind::Geo
        )
    };

    let mut best_dist = f32::INFINITY;
    let mut best_pixel_x = 0.0f32;

    // Pass 1: find globally nearest pixel_x
    for (mark_idx, child) in plot_area_tree.children.iter().enumerate() {
        if child.tag == line_tag {
            let s = child.state.downcast_ref::<plot_area::line::State>();
            for pt in &s.pixel_points {
                let d = (pt.x - local.x).abs();
                if d < best_dist {
                    best_dist = d;
                    best_pixel_x = pt.x;
                }
            }
        } else if child.tag == area_tag {
            let s = child.state.downcast_ref::<plot_area::area::State>();
            for sub in &s.series_points {
                for pt in sub {
                    let d = (pt.x - local.x).abs();
                    if d < best_dist {
                        best_dist = d;
                        best_pixel_x = pt.x;
                    }
                }
            }
        } else if child.tag == xy_tag {
            if is_geo_xy(mark_idx) {
                continue;
            }
            let s = child.state.downcast_ref::<plot_area::xy::State>();
            for pt in &s.pixel_points {
                let d = (pt.x - local.x).abs();
                if d < best_dist {
                    best_dist = d;
                    best_pixel_x = pt.x;
                }
            }
        } else if child.tag == bars_tag {
            let s = child.state.downcast_ref::<plot_area::bars::State>();
            for rects in &s.series_rects {
                for rect in rects {
                    let center_x = rect.x + rect.width / 2.0;
                    let d = (center_x - local.x).abs();
                    if d < best_dist {
                        best_dist = d;
                        best_pixel_x = center_x;
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

    let data_x = plane.to_data_x(best_pixel_x);

    // Pass 2: collect all entries at that pixel_x (within tolerance)
    let tolerance = 0.5f32;
    // Bars need a wider tolerance because bar center x can differ from line pixel x
    let bar_tolerance = 4.0f32;
    let mut entries = Vec::new();

    for (mark_idx, child) in plot_area_tree.children.iter().enumerate() {
        if child.tag == line_tag {
            let s = child.state.downcast_ref::<plot_area::line::State>();
            for (pt_idx, pt) in s.pixel_points.iter().enumerate() {
                if (pt.x - best_pixel_x).abs() <= tolerance {
                    entries.push((mark_idx, 0, pt_idx));
                    break; // one entry per line series
                }
            }
        } else if child.tag == area_tag {
            let s = child.state.downcast_ref::<plot_area::area::State>();
            for (ser_idx, sub) in s.series_points.iter().enumerate() {
                for (pt_idx, pt) in sub.iter().enumerate() {
                    if (pt.x - best_pixel_x).abs() <= tolerance {
                        entries.push((mark_idx, ser_idx, pt_idx));
                        break;
                    }
                }
            }
        } else if child.tag == xy_tag {
            if is_geo_xy(mark_idx) {
                continue;
            }
            let s = child.state.downcast_ref::<plot_area::xy::State>();
            for (pt_idx, pt) in s.pixel_points.iter().enumerate() {
                if (pt.x - best_pixel_x).abs() <= tolerance {
                    entries.push((mark_idx, 0, pt_idx));
                    break;
                }
            }
        } else if child.tag == bars_tag {
            let s = child.state.downcast_ref::<plot_area::bars::State>();
            for (ser_idx, rects) in s.series_rects.iter().enumerate() {
                for (bar_idx, rect) in rects.iter().enumerate() {
                    let center_x = rect.x + rect.width / 2.0;
                    if (center_x - best_pixel_x).abs() <= bar_tolerance {
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
fn pie_slice_at(local: Point, pie_state: &plot_area::pie::State) -> Option<usize> {
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
    let pie_tag = tree::Tag::of::<plot_area::pie::State>();

    for (mark_idx, mark_tree) in plot_area_tree.children.iter().enumerate() {
        if mark_tree.tag != pie_tag {
            continue;
        }
        let pie_state = mark_tree.state.downcast_ref::<plot_area::pie::State>();
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

/// Hit-test the cursor against the choropleth's projected polygons.
///
/// Walks every `Series::Choropleth` and runs
/// [`plot_area::geo::hit_test`] against the chart-level
/// [`plot_area::Plane`] (which holds the projected rings + bboxes
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
        if !matches!(series, plot_area::Series::Choropleth(_)) {
            continue;
        }
        if let Some(feature_idx) = plot_area::geo::hit_test(geo_plane, local) {
            return Some(hover::Geometry::ChoroplethArea { mark_idx, feature_idx });
        }
    }
    None
}

/// Returns a mutable handle to the [`animation::Tick`] hosted on
/// whichever per-mark `State` the tree node carries, or `None` when
/// the mark type doesn't host a tick. Tag is matched linearly so each
/// new animated mark adds exactly one branch.
fn animation_tick_mut(tree: &mut Tree) -> Option<&mut animation::Tick> {
    let tag = tree.tag;
    if tag == tree::Tag::of::<plot_area::pie::State>() {
        return Some(&mut tree.state.downcast_mut::<plot_area::pie::State>().tick);
    }
    if tag == tree::Tag::of::<plot_area::bars::State>() {
        return Some(&mut tree.state.downcast_mut::<plot_area::bars::State>().tick);
    }
    if tag == tree::Tag::of::<plot_area::waterfall::State>() {
        return Some(&mut tree.state.downcast_mut::<plot_area::waterfall::State>().tick);
    }
    if tag == tree::Tag::of::<plot_area::line::State>() {
        return Some(&mut tree.state.downcast_mut::<plot_area::line::State>().tick);
    }
    if tag == tree::Tag::of::<plot_area::area::State>() {
        return Some(&mut tree.state.downcast_mut::<plot_area::area::State>().tick);
    }
    if tag == tree::Tag::of::<plot_area::xy::State>() {
        return Some(&mut tree.state.downcast_mut::<plot_area::xy::State>().tick);
    }
    if tag == tree::Tag::of::<plot_area::heatmap::State>() {
        return Some(&mut tree.state.downcast_mut::<plot_area::heatmap::State>().tick);
    }
    if tag == tree::Tag::of::<plot_area::treemap::State>() {
        return Some(&mut tree.state.downcast_mut::<plot_area::treemap::State>().tick);
    }
    if tag == tree::Tag::of::<plot_area::choropleth::State>() {
        return Some(&mut tree.state.downcast_mut::<plot_area::choropleth::State>().tick);
    }
    if tag == tree::Tag::of::<plot_area::gauge::State>() {
        return Some(&mut tree.state.downcast_mut::<plot_area::gauge::State>().tick);
    }
    if tag == tree::Tag::of::<plot_area::band::State>() {
        return Some(&mut tree.state.downcast_mut::<plot_area::band::State>().tick);
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
    let old_state = old_mark.state.downcast_ref::<plot_area::pie::State>();
    let new_state = new_mark.state.downcast_mut::<plot_area::pie::State>();
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
    let old_state = old_mark.state.downcast_ref::<plot_area::bars::State>();
    let new_state = new_mark.state.downcast_mut::<plot_area::bars::State>();
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
    let old_state = old_mark.state.downcast_ref::<plot_area::waterfall::State>();
    let new_state = new_mark.state.downcast_mut::<plot_area::waterfall::State>();
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
    let old_state = old_mark.state.downcast_ref::<plot_area::line::State>();
    let new_state = new_mark.state.downcast_mut::<plot_area::line::State>();
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
    let old_state = old_mark.state.downcast_ref::<plot_area::area::State>();
    let new_state = new_mark.state.downcast_mut::<plot_area::area::State>();
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
    let old_state = old_mark.state.downcast_ref::<plot_area::heatmap::State>();
    let new_state = new_mark.state.downcast_mut::<plot_area::heatmap::State>();
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
    let old_state = old_mark.state.downcast_ref::<plot_area::treemap::State>();
    let new_state = new_mark.state.downcast_mut::<plot_area::treemap::State>();
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
    let old_state = old_mark.state.downcast_ref::<plot_area::gauge::State>();
    let new_state = new_mark.state.downcast_mut::<plot_area::gauge::State>();
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
    let old_state = old_mark.state.downcast_ref::<plot_area::band::State>();
    let new_state = new_mark.state.downcast_mut::<plot_area::band::State>();
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
    let pie_tag = tree::Tag::of::<plot_area::pie::State>();
    let bars_tag = tree::Tag::of::<plot_area::bars::State>();
    let waterfall_tag = tree::Tag::of::<plot_area::waterfall::State>();
    let line_tag = tree::Tag::of::<plot_area::line::State>();
    let area_tag = tree::Tag::of::<plot_area::area::State>();
    let xy_tag = tree::Tag::of::<plot_area::xy::State>();
    let heatmap_tag = tree::Tag::of::<plot_area::heatmap::State>();
    let treemap_tag = tree::Tag::of::<plot_area::treemap::State>();
    let choropleth_tag = tree::Tag::of::<plot_area::choropleth::State>();
    let gauge_tag = tree::Tag::of::<plot_area::gauge::State>();
    let band_tag = tree::Tag::of::<plot_area::band::State>();
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
    let Some(plane) = &plot_area_state.plane else {
        return layout::Node::new(Size::ZERO);
    };

    let pie_tag = tree::Tag::of::<plot_area::pie::State>();
    let Some(pie_tree) = plot_area_tree.children.iter().find(|t| t.tag == pie_tag) else {
        return layout::Node::new(Size::ZERO);
    };
    let pie_state = pie_tree.state.downcast_ref::<plot_area::pie::State>();
    let (pie_center, inner_radius) = pie_state.center_geometry();

    // Origin of the plot-area's plane in chart-local coords. Mirrors
    // `compute_plot_bounds`'s offset chain so the overlay aligns with
    // the same rectangle hover/click hit-tests against.
    let plane_origin = Point::new(
        padding.left + plot_area_offset.x + plane.bounds.x,
        padding.top + plot_area_offset.y + plane.bounds.y,
    );

    let (origin, area_size) = match placement {
        donut::Placement::Stack => (plane_origin, Size::new(plane.bounds.width, plane.bounds.height)),
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
        let Some(plane) = &plot_area_state.plane else {
            return;
        };
        let plot_bounds = compute_plot_bounds(chart_bounds, self.padding, plot_area_offset, plane);

        match event {
            // === HOVER HANDLING ===
            Event::Mouse(mouse::Event::CursorMoved { .. }) => match cursor.position_in(plot_bounds) {
                Some(local) => {
                    let plot_area_tree = &scene_tree.children[6];
                    let new_hover = find_nearest_hover(
                        Point::new(local.x, local.y),
                        plot_area_tree,
                        self.scene.plot_area(),
                        plane,
                        plot_area_state.geo_plane.as_ref(),
                    );

                    // `data_x` is snapped to a discrete pixel-x of an actual data
                    // point and projected through a deterministic plane, so
                    // identical hovered points produce identical f64 bits — exact
                    // equality is sufficient (no need for an epsilon shim).
                    let changed = state.hover != new_hover;

                    state.hover = new_hover;
                    if changed {
                        shell.request_redraw();
                    }
                }
                None => {
                    if state.hover.is_some() {
                        state.hover = None;
                        shell.request_redraw();
                    }
                }
            },
            Event::Mouse(mouse::Event::CursorLeft) if state.hover.is_some() => {
                state.hover = None;
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
                    let bars_tag = tree::Tag::of::<plot_area::bars::State>();
                    let pie_tag = tree::Tag::of::<plot_area::pie::State>();
                    let treemap_tag = tree::Tag::of::<plot_area::treemap::State>();
                    let choropleth_tag = tree::Tag::of::<plot_area::choropleth::State>();

                    // First pass: hit-test labels (labels win when overlapping shapes)
                    for (mark_idx, mark_tree) in plot_area_tree.children.iter().enumerate() {
                        if mark_tree.tag == bars_tag {
                            let bars_state = mark_tree.state.downcast_ref::<plot_area::bars::State>();

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
                            let pie_state = mark_tree.state.downcast_ref::<plot_area::pie::State>();

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

                    // Second pass: hit-test shapes
                    for (mark_idx, mark_tree) in plot_area_tree.children.iter().enumerate() {
                        if mark_tree.tag == bars_tag {
                            let bars_state = mark_tree.state.downcast_ref::<plot_area::bars::State>();

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
                            let pie_state = mark_tree.state.downcast_ref::<plot_area::pie::State>();
                            if let Some(slice_idx) = pie_slice_at(local, pie_state) {
                                shell.publish(on_action(Action::Clicked(crate::target::Target::Entry {
                                    mark: mark_idx,
                                    series: 0,
                                    index: slice_idx,
                                })));
                                return;
                            }
                        } else if mark_tree.tag == treemap_tag {
                            let tm_state = mark_tree.state.downcast_ref::<plot_area::treemap::State>();

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
        if let Some(hover) = &state.hover
            && let Some(tooltip_config) = self.scene.tooltip()
        {
            let scene_tree = &tree.children[0];
            let plot_area_state = scene_tree.children[6].state.downcast_ref::<plot_area::State>();
            if let Some(plane) = &plot_area_state.plane {
                draw_tooltip_overlay(
                    renderer,
                    design,
                    layout.bounds(),
                    self.padding,
                    self.scene.plot_area_offset(),
                    plane,
                    hover,
                    tooltip_config,
                    &self.scene,
                    scene_tree,
                    viewport,
                    cursor,
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
            if let Some(plane) = &plot_area_state.plane {
                let plot_bounds = compute_plot_bounds(chart_bounds, self.padding, plot_area_offset, plane);
                if cursor.is_over(plot_bounds) {
                    return mouse::Interaction::Crosshair;
                }
            }
        }
        mouse::Interaction::None
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
#[allow(clippy::too_many_arguments)]
fn draw_tooltip_overlay<Message>(
    renderer: &mut Renderer,
    design: &dyn design::Design,
    chart_bounds: Rectangle,
    padding: Padding,
    plot_area_offset: Point,
    plane: &plot_area::Plane,
    hover: &hover::Geometry,
    tooltip_config: &crate::data::tooltip::Tooltip,
    scene: &scene::Scene<'_, Message, Renderer>,
    scene_tree: &Tree,
    viewport: &Rectangle,
    cursor: mouse::Cursor,
) {
    match hover {
        hover::Geometry::Cartesian { data_x, entries } => draw_cartesian_tooltip_overlay(
            renderer,
            design,
            chart_bounds,
            padding,
            plot_area_offset,
            plane,
            *data_x,
            entries,
            tooltip_config,
            scene,
            scene_tree,
            viewport,
        ),
        hover::Geometry::Pie { mark_idx, slice_idx } => draw_pie_tooltip_overlay(
            renderer,
            design,
            chart_bounds,
            padding,
            plot_area_offset,
            plane,
            *mark_idx,
            *slice_idx,
            tooltip_config,
            scene,
            viewport,
            cursor,
        ),
        hover::Geometry::Geographic { mark_idx, point_idx } => draw_geo_tooltip_overlay(
            renderer,
            design,
            chart_bounds,
            padding,
            plot_area_offset,
            plane,
            *mark_idx,
            *point_idx,
            tooltip_config,
            scene,
            scene_tree,
            viewport,
        ),
        hover::Geometry::ChoroplethArea { mark_idx, feature_idx } => draw_choropleth_tooltip_overlay(
            renderer,
            design,
            chart_bounds,
            padding,
            plot_area_offset,
            plane,
            *mark_idx,
            *feature_idx,
            tooltip_config,
            scene,
            scene_tree,
            viewport,
            cursor,
        ),
    }
}

/// Draws the Cartesian tooltip overlay (tracking line, markers, box).
///
/// Builds one [`hover::Entry`] per matched (mark, series, point) at the
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
    plane: &plot_area::Plane,
    data_x: f64,
    cartesian_entries: &[(usize, usize, usize)],
    tooltip_config: &crate::data::tooltip::Tooltip,
    scene: &scene::Scene<'_, Message, Renderer>,
    scene_tree: &Tree,
    viewport: &Rectangle,
) {
    use crate::core::renderer::Renderer as _;
    use crate::widget::canvas::{Frame, Path, Stroke};
    use crate::widget::renderer::geometry;

    let plot_bounds = compute_plot_bounds(chart_bounds, padding, plot_area_offset, plane);
    let plot_area = scene.plot_area();
    let plot_area_tree = &scene_tree.children[6];

    let background = design.background_color();
    let text_pair = design.text_pair();
    let seed = design.seed();
    let text_color = design.text_color().resolve(background, text_pair, &seed, None);
    let palette = scene.resolve_palette(design);

    // Compute the tracking pixel x from data_x
    let tracking_pixel_x = {
        let t = if plane.x_max > plane.x_min {
            ((data_x - plane.x_min) / (plane.x_max - plane.x_min)) as f32
        } else {
            0.5
        };
        plane.bounds.x + t * plane.bounds.width
    };

    // Build tooltip entries and collect pixel positions
    let line_tag = tree::Tag::of::<plot_area::line::State>();
    let area_tag = tree::Tag::of::<plot_area::area::State>();
    let xy_tag = tree::Tag::of::<plot_area::xy::State>();
    let bars_tag = tree::Tag::of::<plot_area::bars::State>();

    let mut entries: Vec<hover::Entry> = Vec::new();

    for &(mark_idx, series_idx, pt_idx) in cartesian_entries {
        let series = &plot_area.series[mark_idx];
        let child = &plot_area_tree.children[mark_idx];

        // Bars resolve their color through the full priority chain
        // (point_colors > color_by > series.color > palette). When this is
        // `Some`, the generic explicit_color/palette-fallback logic below is
        // bypassed. See GOG.md § 8b.
        let mut bars_resolved_color: Option<crate::core::Color> = None;

        let (datum, name, explicit_color, annotation) = if child.tag == line_tag {
            if let plot_area::Series::Line(line) = series {
                let pt = &line.data.points[pt_idx];
                let pixel = plane.to_pixel(crate::data::Datum { x: pt.x, y: pt.y });
                (
                    crate::data::Datum { x: pt.x, y: pt.y },
                    line.data.name().map(|s| s.to_string()),
                    line.data.color,
                    hover::Highlight::PointMarker { pixel, radius: 4.0 },
                )
            } else {
                continue;
            }
        } else if child.tag == area_tag {
            if let plot_area::Series::Area(area) = series {
                let ser = &area.data.series[series_idx];
                let pt = &ser.points[pt_idx];
                let pixel = plane.to_pixel(crate::data::Datum { x: pt.x, y: pt.y });
                (
                    crate::data::Datum { x: pt.x, y: pt.y },
                    ser.name().map(|s| s.to_string()),
                    ser.color,
                    hover::Highlight::PointMarker { pixel, radius: 4.0 },
                )
            } else {
                continue;
            }
        } else if child.tag == xy_tag {
            if let plot_area::Series::Xy(xy) = series {
                let pt = &xy.data.points[pt_idx];
                let pixel = plane.to_pixel(crate::data::Datum { x: pt.x, y: pt.y });
                (
                    crate::data::Datum { x: pt.x, y: pt.y },
                    xy.data.name.as_deref().map(|s| s.to_string()),
                    None,
                    hover::Highlight::PointMarker { pixel, radius: 4.0 },
                )
            } else {
                continue;
            }
        } else if child.tag == bars_tag {
            if let plot_area::Series::Bars(bars) = series {
                let bar_series = &bars.data.series[series_idx];
                let pt = &bar_series.points[pt_idx];

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
                )
            } else {
                continue;
            }
        } else {
            continue;
        };

        let anchor = plane.to_pixel(datum);

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

        entries.push(hover::Entry {
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
            annotation,
        });
    }

    if entries.is_empty() {
        return;
    }

    // Vertically center on mean y of entries, in absolute coords.
    let mean_y: f32 = entries.iter().map(|re| re.anchor.y).sum::<f32>() / entries.len() as f32;
    let anchor_x = plot_bounds.x + tracking_pixel_x - plane.bounds.x;
    let anchor_y = plot_bounds.y + mean_y - plane.bounds.y;

    // Wrap everything in a layer so it composites above chart content
    renderer.with_layer(*viewport, |renderer| {
        // --- Draw tracking line and markers via canvas Frame ---
        let frame_size = crate::core::Size::new(plane.bounds.width, plane.bounds.height);
        let mut frame = Frame::new(renderer, frame_size);

        // Tracking line — only for continuous marks (line/area/xy), not discrete (bars)
        let has_continuous = entries
            .iter()
            .any(|re| matches!(re.annotation, hover::Highlight::PointMarker { .. }));
        if tooltip_config.tracking_line && has_continuous {
            let line_x = tracking_pixel_x - plane.bounds.x;
            let tracking_color = crate::core::Color { a: 0.18, ..text_color };
            let path = Path::line(Point::new(line_x, 0.0), Point::new(line_x, plane.bounds.height));
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
                match &re.annotation {
                    hover::Highlight::PointMarker { pixel, radius } => {
                        let cx = pixel.x - plane.bounds.x;
                        let cy = pixel.y - plane.bounds.y;

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

        let flip_axis_x = plot_bounds.x + plane.bounds.width / 2.0;
        draw_tooltip_box(
            renderer,
            design,
            tooltip_config,
            &entries,
            Point::new(anchor_x, anchor_y),
            flip_axis_x,
            chart_bounds,
            text_color,
            viewport,
        );
    });
}

/// Draws the pie hover tooltip — a single-row box anchored at the cursor.
///
/// Unlike Cartesian tooltips, pie hover has no tracking line or marker:
/// the hovered slice is the visual annotation. The synthesized
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
    plane: &plot_area::Plane,
    mark_idx: usize,
    slice_idx: usize,
    tooltip_config: &crate::data::tooltip::Tooltip,
    scene: &scene::Scene<'_, Message, Renderer>,
    viewport: &Rectangle,
    cursor: mouse::Cursor,
) {
    use crate::core::renderer::Renderer as _;

    let plot_area = scene.plot_area();

    let plot_area::Series::Pie(pie) = &plot_area.series[mark_idx] else {
        return;
    };
    let Some(slice) = pie.data.slices.get(slice_idx) else {
        return;
    };

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
    let plot_bounds = compute_plot_bounds(chart_bounds, padding, plot_area_offset, plane);
    let flip_axis_x = plot_bounds.x + plot_bounds.width / 2.0;

    let entry = hover::Entry {
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
        annotation: hover::Highlight::None,
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

    renderer.with_layer(*viewport, |renderer| {
        draw_tooltip_box(
            renderer,
            design,
            effective_tooltip,
            &entries,
            cursor_pos,
            flip_axis_x,
            chart_bounds,
            text_color,
            viewport,
        );
    });
}

/// Draws the geographic hover overlay — a stroke ring around the
/// hovered bubble and a side-anchored tooltip box.
///
/// No tracking line: the bubble is itself the visual focus, and a
/// dashed crosshair across longitudes would carry no useful meaning
/// on a projected map. The tooltip box anchors at the bubble center
/// and clamps to the chart bounds via the existing flip-axis logic.
#[allow(clippy::too_many_arguments)]
fn draw_geo_tooltip_overlay<Message>(
    renderer: &mut Renderer,
    design: &dyn design::Design,
    chart_bounds: Rectangle,
    padding: Padding,
    plot_area_offset: Point,
    plane: &plot_area::Plane,
    mark_idx: usize,
    point_idx: usize,
    tooltip_config: &crate::data::tooltip::Tooltip,
    scene: &scene::Scene<'_, Message, Renderer>,
    scene_tree: &Tree,
    viewport: &Rectangle,
) {
    use crate::core::renderer::Renderer as _;
    use crate::widget::canvas::{Frame, Path, Stroke};
    use crate::widget::renderer::geometry;

    let plot_area = scene.plot_area();
    let plot_area_tree = &scene_tree.children[6];

    let plot_area::Series::Xy(xy) = &plot_area.series[mark_idx] else {
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

    let plot_bounds = compute_plot_bounds(chart_bounds, padding, plot_area_offset, plane);
    let anchor_abs = Point::new(
        plot_bounds.x + pixel.x - plane.bounds.x,
        plot_bounds.y + pixel.y - plane.bounds.y,
    );
    // Place the box on the side of the bubble that points away from the
    // closest chart edge, with its near edge sitting `radius + 8px` past
    // the bubble's outline. `draw_tooltip_box` interprets `flip_axis_x`
    // as: anchor < flip_axis_x → box extends rightward from anchor;
    // anchor > flip_axis_x → box extends leftward. Anchoring at the
    // bubble's edge (rather than its center) means the box's `box_gap`
    // shim from `draw_tooltip_box` lands fully outside the bubble,
    // not inside its disc.
    let chart_center_x = chart_bounds.x + chart_bounds.width / 2.0;
    let on_left_half = anchor_abs.x < chart_center_x;
    let (box_anchor_x, flip_axis_x) = if on_left_half {
        (anchor_abs.x + radius, chart_bounds.x + chart_bounds.width)
    } else {
        (anchor_abs.x - radius, chart_bounds.x)
    };

    let entry = hover::Entry {
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
        annotation: hover::Highlight::Ring {
            pixel,
            radius: radius + 2.0,
            color: bubble_color,
            width: 1.5,
        },
    };
    let entries = [entry];

    renderer.with_layer(*viewport, |renderer| {
        // Ring around the hovered bubble. Drawn on a frame translated
        // to the plot bounds so coordinates match the cartesian path.
        if tooltip_config.markers {
            let frame_size = crate::core::Size::new(plane.bounds.width, plane.bounds.height);
            let mut frame = Frame::new(renderer, frame_size);
            for re in &entries {
                match &re.annotation {
                    hover::Highlight::Ring {
                        pixel,
                        radius,
                        color,
                        width,
                    } => {
                        let cx = pixel.x - plane.bounds.x;
                        let cy = pixel.y - plane.bounds.y;
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
        }

        let box_anchor = Point::new(box_anchor_x, anchor_abs.y);
        draw_tooltip_box(
            renderer,
            design,
            effective_tooltip,
            &entries,
            box_anchor,
            flip_axis_x,
            chart_bounds,
            text_color,
            viewport,
        );
    });
}

/// Draws the choropleth hover overlay — a stroke around the hovered
/// polygon's projected rings and a cursor-anchored tooltip box.
///
/// The hovered area carries no point-marker concept; the polygon itself
/// is the visual focus. The tooltip box anchors at the cursor and
/// flips left/right around the chart's mid-x via the same logic
/// [`draw_tooltip_box`] uses for cartesian charts.
#[allow(clippy::too_many_arguments)]
fn draw_choropleth_tooltip_overlay<Message>(
    renderer: &mut Renderer,
    design: &dyn design::Design,
    chart_bounds: Rectangle,
    padding: Padding,
    plot_area_offset: Point,
    plane: &plot_area::Plane,
    mark_idx: usize,
    feature_idx: usize,
    tooltip_config: &crate::data::tooltip::Tooltip,
    scene: &scene::Scene<'_, Message, Renderer>,
    scene_tree: &Tree,
    viewport: &Rectangle,
    cursor: mouse::Cursor,
) {
    use crate::core::renderer::Renderer as _;
    use crate::widget::canvas::{Frame, Path, Stroke};
    use crate::widget::renderer::geometry;

    let plot_area_state = scene_tree.children[6].state.downcast_ref::<plot_area::State>();
    let Some(geo_plane) = &plot_area_state.geo_plane else {
        return;
    };
    let plot_area = scene.plot_area();
    let plot_area::Series::Choropleth(c) = &plot_area.series[mark_idx] else {
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

    // Hovered-feature outline: theme text color at moderate alpha so
    // it reads as foreground without overwhelming the polygon fill.
    let stroke_color = crate::core::Color { a: 0.85, ..text_color };
    let stroke_width = 1.25;

    let Some(cursor_pos) = cursor.position() else {
        return;
    };
    let plot_bounds = compute_plot_bounds(chart_bounds, padding, plot_area_offset, plane);
    let flip_axis_x = plot_bounds.x + plot_bounds.width / 2.0;

    let entry = hover::Entry {
        tooltip: TooltipEntry {
            x: feature_idx as f64,
            y: entry_value.unwrap_or(f64::NAN),
            series_name: Some(feature_name),
            series_index: 0,
            mark_index: mark_idx,
            color: stroke_color,
        },
        anchor: cursor_pos,
        color: stroke_color,
        annotation: hover::Highlight::Stroke {
            rings: std::sync::Arc::new(rings.clone()),
            color: stroke_color,
            width: stroke_width,
        },
    };
    let entries = [entry];

    // Honor the chart-level value-format chain when the user hasn't
    // installed a custom format. With no value, show just the feature
    // name.
    let chain_tooltip;
    let effective_tooltip: &crate::data::tooltip::Tooltip = if tooltip_config.format_is_default() {
        let mark_format = scene.primary_mark_value_format(mark_idx).cloned();
        chain_tooltip = tooltip_config.clone().format(move |entry: &TooltipEntry| {
            let name = entry.series_name.clone().unwrap_or_default();
            if entry.y.is_nan() {
                return name;
            }
            let formatted = match &mark_format {
                Some(f) => f(&entry.y),
                None => crate::scale::default_f64_format(entry.y),
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

    // Polygon stroke and tooltip box live on separate layers so the
    // tooltip's text and background composite above the stroke even
    // when the tooltip's anchor sits inside the hovered polygon.
    if tooltip_config.markers {
        renderer.with_layer(*viewport, |renderer| {
            let frame_size = crate::core::Size::new(plane.bounds.width, plane.bounds.height);
            let mut frame = Frame::new(renderer, frame_size);
            for re in &entries {
                if let hover::Highlight::Stroke { rings, color, width } = &re.annotation {
                    for ring in rings.iter() {
                        if ring.len() < 3 {
                            continue;
                        }
                        let path = Path::new(|builder| {
                            let (x0, y0) = ring[0];
                            builder.move_to(Point::new(x0 - plane.bounds.x, y0 - plane.bounds.y));
                            for &(x, y) in &ring[1..] {
                                builder.line_to(Point::new(x - plane.bounds.x, y - plane.bounds.y));
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

    renderer.with_layer(*viewport, |renderer| {
        draw_tooltip_box(
            renderer,
            design,
            effective_tooltip,
            &entries,
            cursor_pos,
            flip_axis_x,
            chart_bounds,
            text_color,
            viewport,
        );
    });
}

/// Render the tooltip box body (background, swatches, text rows).
///
/// The caller picks the visual `anchor`, the L/R `flip_axis_x` that
/// decides whether the box sits to the left or right of the anchor,
/// and the `clamp_bounds` rectangle that the box's vertical position
/// is clamped within. Returns nothing — the caller is responsible for
/// the surrounding layering wrapper.
#[allow(clippy::too_many_arguments)]
fn draw_tooltip_box(
    renderer: &mut Renderer,
    design: &dyn design::Design,
    tooltip_config: &crate::data::tooltip::Tooltip,
    entries: &[hover::Entry],
    anchor: Point,
    flip_axis_x: f32,
    clamp_bounds: Rectangle,
    text_color: crate::core::Color,
    viewport: &Rectangle,
) {
    use crate::core::renderer::Renderer as _;
    use crate::core::text::Renderer as _;

    let font = renderer.default_font();
    let font_size: f32 = 12.0;
    let line_height_px: f32 = 18.0;
    let swatch_size: f32 = 8.0;
    let swatch_gap: f32 = 6.0;
    let box_padding: f32 = 8.0;
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
    let box_width = box_padding * 2.0 + swatch_space + max_text_width;
    let box_height = box_padding * 2.0 + entries.len() as f32 * line_height_px;

    // Position tooltip box: right of anchor if to the left of flip_axis_x, else left
    let box_x = if anchor.x < flip_axis_x {
        anchor.x + box_gap
    } else {
        anchor.x - box_width - box_gap
    };

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
#[derive(Debug, Clone, Copy, Default)]
pub struct Style {
    /// The background color of the chart.
    pub background: Option<crate::core::Color>,
    /// The border of the chart.
    pub border: crate::core::Border,
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
    }
}

/// A transparent [`Chart`] with no background or border.
pub fn transparent(_design: &dyn design::Design) -> Style {
    Style::default()
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
    }
}
