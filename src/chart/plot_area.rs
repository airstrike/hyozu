use crate::core::Rectangle;
use crate::core::layout::Limits;
use crate::core::widget::{Tree, tree};
use crate::data::Datum;

use crate::widget::renderer::geometry;

pub mod annotation;
pub mod area;
pub mod band;
pub mod bars;
pub mod boxplot;
pub mod choropleth;
pub(crate) mod diagram;
pub mod gauge;
pub mod geo;
pub mod heatmap;
pub mod line;
pub mod pie;
pub mod rule;
pub mod sector;
pub mod text;
pub mod tick;
pub mod treemap;
pub mod violin;
pub mod waterfall;
pub mod xy;

pub use band::Band;
pub use bars::Bars;
pub use boxplot::BoxPlot;
pub use choropleth::Choropleth;
pub use gauge::Gauge;
pub use heatmap::Heatmap;
pub use line::Line;
pub use pie::Pie;
pub use rule::Rule;
pub use text::Text;
pub use tick::Tick;
pub use treemap::Treemap;
pub use violin::Violin;
pub use waterfall::Waterfall;
pub use xy::Xy;

/// An inclusive interval of data values along a single axis.
#[derive(Debug, Clone, Copy)]
pub struct Range {
    /// Lower bound of the interval.
    pub min: f64,
    /// Upper bound of the interval.
    pub max: f64,
}

/// The data domain that the plot area maps to pixels: an x and y [`Range`]
/// plus how each axis maps onto the unit interval. Carries no pixel
/// geometry — the destination rectangle is supplied per call to
/// [`to_pixel`] / [`to_data_x`].
#[derive(Debug, Clone, Copy)]
pub struct Domain {
    /// Data range along the x axis.
    pub x: Range,
    /// Data range along the y axis.
    pub y: Range,
    /// Transform and direction of the x axis.
    pub x_mapping: crate::scale::Mapping,
    /// Transform and direction of the y axis. Honored by numeric-axis
    /// marks via [`to_pixel`]; categorical or geographic marks (Pie,
    /// Treemap, Choropleth) bypass the domain entirely and aren't
    /// affected.
    pub y_mapping: crate::scale::Mapping,
}

impl Domain {
    /// A domain over `x` and `y` with the given per-axis mappings.
    pub fn new(x: Range, y: Range, mappings: Mappings) -> Self {
        Self {
            x,
            y,
            x_mapping: mappings.x,
            y_mapping: mappings.y,
        }
    }

    /// A linear, forward domain over `x` and `y`.
    pub fn linear(x: Range, y: Range) -> Self {
        Self::new(x, y, Mappings::default())
    }
}

/// The x and y [`crate::scale::Mapping`]s of one axis pair.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Mappings {
    /// Mapping of the horizontal axis.
    pub x: crate::scale::Mapping,
    /// Mapping of the vertical axis.
    pub y: crate::scale::Mapping,
}

/// Transform a data point (f64) to pixel coordinates (f32) within `rect`.
pub fn to_pixel(domain: &Domain, rect: Rectangle, datum: Datum) -> crate::core::Point {
    let x = if domain.x.max > domain.x.min {
        let unit = domain.x_mapping.map_to_unit(datum.x, domain.x.min, domain.x.max) as f32;
        rect.x + unit * rect.width
    } else {
        rect.x + rect.width / 2.0
    };

    let y = if domain.y.max > domain.y.min {
        let unit = domain.y_mapping.map_to_unit(datum.y, domain.y.min, domain.y.max) as f32;
        rect.y + rect.height - unit * rect.height
    } else {
        rect.y + rect.height / 2.0
    };

    crate::core::Point::new(x, y)
}

/// Inverse of the x part of [`to_pixel`]: map a pixel x-coordinate back to a
/// data-space x value within `rect`.
pub fn to_data_x(domain: &Domain, rect: Rectangle, pixel_x: f32) -> f64 {
    if rect.width == 0.0 {
        domain.x.min
    } else {
        let t = (pixel_x - rect.x) / rect.width;
        domain.x_mapping.unit_to_value(t as f64, domain.x.min, domain.x.max)
    }
}

/// Returns `[min, max]` as a [`Range`], lifting a nonpositive `min` on a
/// log axis to `max / 1e4`.
///
/// Log axes can't show nonpositive values; the transform clamps to
/// `f64::EPSILON` for mapping anyway, which spans ~16 decades of empty
/// space below any real data. When the user passes `axis_bounds(0.0, …)`
/// on a log-scaled axis, this documented clamp keeps the rendered range
/// matching what the guide's tick generation produces.
fn log_safe_range(min: f64, max: f64, mapping: crate::scale::Mapping) -> Range {
    if matches!(mapping.transform, crate::scale::Transform::Log) && min <= 0.0 && max > 0.0 {
        Range { min: max / 1e4, max }
    } else {
        Range { min, max }
    }
}

/// Minor gridline positions strictly between consecutive major `ticks`.
///
/// Linear and sqrt axes split each interval into `subdivisions` equal
/// steps. Log axes place minors at `2..=9 × 10^k` inside each interval
/// (plus any decades a strided major sequence skipped), the familiar
/// semi-log paper ruling.
fn minor_ticks(ticks: &[f64], transform: crate::scale::Transform, subdivisions: usize) -> Vec<f64> {
    let mut minors = Vec::new();
    for pair in ticks.windows(2) {
        let (t0, t1) = (pair[0].min(pair[1]), pair[0].max(pair[1]));
        match transform {
            crate::scale::Transform::Log if t0 > 0.0 => {
                let mut decade = 10f64.powi(t0.log10().floor() as i32);
                while decade < t1 {
                    for k in 1..=9 {
                        let v = decade * k as f64;
                        // Skip values within float noise of a major tick.
                        if v > t0 * (1.0 + 1e-9) && v < t1 * (1.0 - 1e-9) {
                            minors.push(v);
                        }
                    }
                    decade *= 10.0;
                }
            }
            _ => {
                let step = (t1 - t0) / subdivisions as f64;
                minors.extend((1..subdivisions).map(|k| t0 + step * k as f64));
            }
        }
    }
    minors
}

/// Builds the shared data-mapping region: the plot rect of `size` inset
/// by the per-edge margins.
///
/// Its bounds are plot-local (origin at `insets.{left, top}`). Primary and
/// secondary planes map data into this same rectangle; marks still
/// translate by the plot area's layout at draw.
fn content_rect(size: crate::core::Size, insets: Insets) -> Rectangle {
    Rectangle {
        x: insets.left,
        y: insets.top,
        width: (size.width - insets.left - insets.right).max(0.0),
        height: (size.height - insets.top - insets.bottom).max(0.0),
    }
}

/// Pixel margins reserved on each edge of the plot area to seat the
/// data-mapping region. The same per-edge value seats the axis ticks and the
/// content rect, so marks and ticks stay aligned by construction.
#[derive(Debug, Clone, Copy, Default)]
pub struct Insets {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
}

/// Chart-level geographic configuration, sourced from
/// [`crate::Data::geo_data`] / [`crate::Data::geo_scope`] /
/// [`crate::Data::geo_projection`] / [`crate::Data::geo_basemap`] and
/// threaded into the per-mark layout pass for geo-aware marks
/// (Choropleth, geo-Xy). Cartesian marks ignore it.
#[derive(Debug, Clone)]
pub struct GeoConfig {
    pub geo: Option<std::sync::Arc<crate::geo::GeoData>>,
    pub scope: crate::geo::MapScope,
    pub projection: crate::geo::ProjectionKind,
    pub basemap: bool,
}

impl Default for GeoConfig {
    fn default() -> Self {
        Self {
            geo: None,
            scope: crate::geo::MapScope::World,
            projection: crate::geo::ProjectionKind::default(),
            basemap: true,
        }
    }
}

/// A series that can be rendered in the plot area
pub enum Series<'a, Message, Renderer>
where
    Message: 'a,
    Renderer: crate::core::text::Renderer + geometry::Renderer,
{
    Area(area::Area<'a, Message, Renderer>),
    Line(Line<'a, Message, Renderer>),
    Bars(Bars<'a, Message, Renderer>),
    BoxPlot(BoxPlot<'a, Message, Renderer>),
    Choropleth(Choropleth<'a, Message, Renderer>),
    Pie(Pie<'a, Message, Renderer>),
    Gauge(Gauge<'a, Message, Renderer>),
    Waterfall(Waterfall<'a, Message, Renderer>),
    Xy(Xy<'a, Message, Renderer>),
    Rule(Rule<'a, Message, Renderer>),
    Band(Band<'a, Message, Renderer>),
    Tick(Tick<'a, Message, Renderer>),
    Heatmap(Heatmap<'a, Message, Renderer>),
    Diagram(diagram::Diagram<'a>),
    Treemap(Treemap<'a, Message, Renderer>),
    Violin(Violin<'a, Message, Renderer>),
    Text(Text<'a, Message, Renderer>),
    Annotation(annotation::Annotation<'a, Message, Renderer>),
}

/// Which axis pair a mark is plotted against.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AxisSide {
    /// Primary (bottom + left) axis pair.
    Primary,
    /// Secondary (top + right) axis pair.
    Secondary,
}

/// State for a PlotArea - stores the data domains and the cached content
/// rect that together map data coordinates to pixels.
pub struct State {
    /// The data domain for the primary (bottom/left) axes. Mapped onto
    /// [`State::content_rect`] via the free [`to_pixel`] / [`to_data_x`]
    /// transforms.
    pub domain: Option<Domain>,
    /// The data domain for the secondary (top/right) axes, if any. Shares
    /// [`State::content_rect`] with the primary domain.
    pub secondary_domain: Option<Domain>,
    /// The inset data-mapping rectangle in plot-local coordinates (origin
    /// at `insets.{left, top}`), mirroring the `content` child node. The
    /// update/hover path has no child [`crate::core::Layout`] to descend,
    /// so it reads this cache instead of the live node tree. Both the
    /// primary and secondary domains map their data into this rectangle.
    pub content_rect: Rectangle,
    /// Chart-level geo projection cache, populated whenever a
    /// geo-aware mark (Choropleth, geo-Xy) is present. `None` for
    /// purely cartesian charts. Survives [`crate::chart::Chart`]'s
    /// wholesale `diff` rebuild via the sibling `replant_geo_plane`
    /// helper, which moves the cache from the old tree onto the new
    /// one so the next layout's dirty-check short-circuits when geo
    /// inputs are unchanged.
    pub geo_plane: Option<geo::Plane>,
}

/// A PlotArea renders the data series within the chart.
///
/// Like Guide and Title, this borrows data and is widget-like but doesn't
/// implement Widget. Keeps UI code separate from data viz.
/// It organizes and delegates to individual series (Line, Bars, etc.)
pub struct PlotArea<'a, Message, Renderer>
where
    Message: 'a,
    Renderer: crate::core::text::Renderer + geometry::Renderer,
{
    pub(crate) series: Vec<Series<'a, Message, Renderer>>,
    /// Which axis pair each series in `series` is plotted against.
    pub(crate) axis_side: Vec<AxisSide>,
}

impl<'a, Message, Renderer> PlotArea<'a, Message, Renderer>
where
    Message: 'a,
    Renderer: crate::core::text::Renderer + geometry::Renderer,
{
    /// Create a new PlotArea from mark data (primary axis).
    pub fn new(marks: &'a [crate::Mark]) -> Self {
        let series: Vec<Series<'a, Message, Renderer>> = marks.iter().map(Self::to_series).collect();
        let axis_side = vec![AxisSide::Primary; series.len()];
        Self { series, axis_side }
    }

    /// Appends marks plotted against the secondary (top/right) axis pair.
    pub fn with_secondary(mut self, marks: &'a [crate::Mark]) -> Self {
        for mark in marks {
            self.series.push(Self::to_series(mark));
            self.axis_side.push(AxisSide::Secondary);
        }
        self
    }

    /// Threads the resolved per-mark value-format closures into every
    /// `Pie` series. `value_formats[i]` is consumed by `series[i]` when
    /// the matching mark is a pie; non-pie marks ignore their slot.
    /// Caller passes `value_formats.len() == self.series.len()`.
    pub fn with_value_formats(mut self, value_formats: Vec<Option<crate::scale::Format<f64>>>) -> Self {
        debug_assert_eq!(self.series.len(), value_formats.len());
        for (series, fmt) in self.series.iter_mut().zip(value_formats) {
            if let Series::Pie(pie) = series {
                pie.set_value_format(fmt);
            }
        }
        self
    }

    /// Threads the chart-level [`crate::Data::animate`] flag into every
    /// mark renderer that has an animator. Mark types without one
    /// silently ignore the call.
    pub fn with_animate(mut self, animate: bool) -> Self {
        for series in self.series.iter_mut() {
            if let Series::Pie(pie) = series {
                pie.set_animate(animate);
            }
            if let Series::Bars(bars) = series {
                bars.set_animate(animate);
            }
            if let Series::Waterfall(wf) = series {
                wf.set_animate(animate);
            }
            if let Series::Line(line) = series {
                line.set_animate(animate);
            }
            if let Series::Area(area) = series {
                area.set_animate(animate);
            }
            if let Series::Xy(xy) = series {
                xy.set_animate(animate);
            }
            if let Series::Heatmap(hm) = series {
                hm.set_animate(animate);
            }
            if let Series::Treemap(tm) = series {
                tm.set_animate(animate);
            }
            if let Series::Choropleth(c) = series {
                c.set_animate(animate);
            }
            if let Series::Gauge(gauge) = series {
                gauge.set_animate(animate);
            }
            if let Series::Band(band) = series {
                band.set_animate(animate);
            }
            if let Series::BoxPlot(bp) = series {
                bp.set_animate(animate);
            }
            if let Series::Violin(v) = series {
                v.set_animate(animate);
            }
            if let Series::Diagram(d) = series {
                d.set_animate(animate);
            }
            if let Series::Annotation(a) = series {
                a.set_animate(animate);
            }
        }
        self
    }

    fn to_series(mark: &'a crate::Mark) -> Series<'a, Message, Renderer> {
        match mark {
            crate::Mark::Area(a) => Series::Area(area::Area::new(a)),
            crate::Mark::Line(line) => Series::Line(Line::new(line)),
            crate::Mark::Bars(bars) => Series::Bars(Bars::new(bars)),
            crate::Mark::BoxPlot(bp) => Series::BoxPlot(BoxPlot::new(bp)),
            crate::Mark::Choropleth(c) => Series::Choropleth(Choropleth::new(c)),
            crate::Mark::Pie(pie) => Series::Pie(Pie::new(pie)),
            crate::Mark::Gauge(gauge) => Series::Gauge(Gauge::new(gauge)),
            crate::Mark::Waterfall(wf) => Series::Waterfall(Waterfall::new(wf)),
            crate::Mark::Xy(xy) => Series::Xy(Xy::new(xy)),
            crate::Mark::Rule(rule) => Series::Rule(Rule::new(rule)),
            crate::Mark::Band(band) => Series::Band(Band::new(band)),
            crate::Mark::Tick(tick) => Series::Tick(Tick::new(tick)),
            crate::Mark::Heatmap(hm) => Series::Heatmap(Heatmap::new(hm)),
            crate::Mark::Diagram(d) => Series::Diagram(diagram::Diagram::new(d)),
            crate::Mark::Treemap(tm) => Series::Treemap(Treemap::new(tm)),
            crate::Mark::Violin(v) => Series::Violin(Violin::new(v)),
            crate::Mark::Text(t) => Series::Text(Text::new(t)),
            crate::Mark::Annotation(a) => Series::Annotation(annotation::Annotation::new(a)),
        }
    }

    /// Computes the cumulative palette index for a specific series within
    /// a mark. Iterates `series[0..mark_idx]` summing their slot counts,
    /// then adds `series_idx`.
    pub(crate) fn color_offset_for(&self, mark_idx: usize, series_idx: usize) -> usize {
        let mut offset: usize = 0;
        for series in self.series.iter().take(mark_idx) {
            offset += match series {
                Series::Area(a) => a.data.series.len(),
                Series::Bars(bars) => bars.data.series.len(),
                Series::BoxPlot(bp) => bp.data.entries.len(),
                Series::Line(_) => 1,
                Series::Xy(_) => 1,
                Series::Pie(pie) => pie.data.slices.len(),
                Series::Gauge(_) => 1,
                Series::Diagram(d) => d.data.entries.len(),
                Series::Treemap(tm) => tm.data.items.len(),
                Series::Waterfall(_) => 3,
                Series::Heatmap(_) => 0,
                Series::Choropleth(_) => 0,
                Series::Violin(v) => v.data.entries.len(),
                Series::Tick(_) => 0,
                Series::Rule(_) => 0,
                Series::Band(_) => 0,
                Series::Text(_) => 0,
                Series::Annotation(_) => 0,
            };
        }
        offset + series_idx
    }

    /// Returns the initial tree state for this PlotArea
    pub(super) fn state(&self) -> Tree {
        // Create children for each series
        let children = self
            .series
            .iter()
            .map(|s| match s {
                Series::Area(a) => a.state(),
                Series::Line(line) => line.state(),
                Series::Bars(bars) => bars.state(),
                Series::BoxPlot(bp) => bp.state(),
                Series::Choropleth(c) => c.state(),
                Series::Pie(pie) => pie.state(),
                Series::Gauge(gauge) => gauge.state(),
                Series::Waterfall(wf) => wf.state(),
                Series::Xy(xy) => xy.state(),
                Series::Rule(rule) => rule.state(),
                Series::Band(band) => band.state(),
                Series::Tick(tick) => tick.state(),
                Series::Heatmap(hm) => hm.state(),
                Series::Diagram(_) => diagram::state::<Renderer::Paragraph>(),
                Series::Treemap(tm) => tm.state(),
                Series::Violin(v) => v.state(),
                Series::Text(t) => t.state(),
                Series::Annotation(a) => a.state(),
            })
            .collect();

        Tree {
            tag: tree::Tag::of::<State>(),
            state: tree::State::new(State {
                domain: None,
                secondary_domain: None,
                content_rect: Rectangle {
                    x: 0.0,
                    y: 0.0,
                    width: 0.0,
                    height: 0.0,
                },
                geo_plane: None,
            }),
            children,
            ..Tree::empty()
        }
    }

    /// Reconcile the tree with current PlotArea state
    pub(super) fn diff(&mut self, tree: &mut Tree) {
        // Diff each series
        tree.diff_children_custom(
            &mut self.series,
            |tree, series| {
                // When the mark type changes (e.g. Bars → Pie), the tree
                // node still holds the old state type.  Detect this via
                // the tag and rebuild the node from scratch.
                let expected_tag = match series {
                    Series::Area(_) => tree::Tag::of::<area::State<Renderer::Paragraph>>(),
                    Series::Line(_) => tree::Tag::of::<line::State<Renderer::Paragraph>>(),
                    Series::Bars(_) => tree::Tag::of::<bars::State<Renderer::Paragraph>>(),
                    Series::BoxPlot(_) => tree::Tag::of::<boxplot::State>(),
                    Series::Choropleth(_) => tree::Tag::of::<choropleth::State>(),
                    Series::Pie(_) => tree::Tag::of::<pie::State<Renderer::Paragraph>>(),
                    Series::Gauge(_) => tree::Tag::of::<gauge::State<Renderer::Paragraph>>(),
                    Series::Waterfall(_) => tree::Tag::of::<waterfall::State<Renderer::Paragraph>>(),
                    Series::Xy(_) => tree::Tag::of::<xy::State>(),
                    Series::Rule(_) => tree::Tag::of::<rule::State<Renderer::Paragraph>>(),
                    Series::Band(_) => tree::Tag::of::<band::State<Renderer::Paragraph>>(),
                    Series::Tick(_) => tree::Tag::of::<tick::State>(),
                    Series::Heatmap(_) => tree::Tag::of::<heatmap::State<Renderer::Paragraph>>(),
                    Series::Diagram(_) => tree::Tag::of::<diagram::State<Renderer::Paragraph>>(),
                    Series::Treemap(_) => tree::Tag::of::<treemap::State<Renderer::Paragraph>>(),
                    Series::Violin(_) => tree::Tag::of::<violin::State>(),
                    Series::Text(_) => tree::Tag::of::<text::State<Renderer::Paragraph>>(),
                    Series::Annotation(_) => tree::Tag::of::<annotation::State<Renderer::Paragraph>>(),
                };

                if tree.tag != expected_tag {
                    *tree = match series {
                        Series::Area(a) => a.state(),
                        Series::Line(line) => line.state(),
                        Series::Bars(bars) => bars.state(),
                        Series::BoxPlot(bp) => bp.state(),
                        Series::Choropleth(c) => c.state(),
                        Series::Pie(pie) => pie.state(),
                        Series::Gauge(gauge) => gauge.state(),
                        Series::Waterfall(wf) => wf.state(),
                        Series::Xy(xy) => xy.state(),
                        Series::Rule(rule) => rule.state(),
                        Series::Band(band) => band.state(),
                        Series::Tick(tick) => tick.state(),
                        Series::Heatmap(hm) => hm.state(),
                        Series::Diagram(_) => diagram::state::<Renderer::Paragraph>(),
                        Series::Treemap(tm) => tm.state(),
                        Series::Violin(v) => v.state(),
                        Series::Text(t) => t.state(),
                        Series::Annotation(a) => a.state(),
                    };
                } else {
                    match series {
                        Series::Area(a) => a.diff(tree),
                        Series::Line(line) => line.diff(tree),
                        Series::Bars(bars) => bars.diff(tree),
                        Series::BoxPlot(bp) => bp.diff(tree),
                        Series::Choropleth(c) => c.diff(tree),
                        Series::Pie(pie) => pie.diff(tree),
                        Series::Gauge(gauge) => gauge.diff(tree),
                        Series::Waterfall(wf) => wf.diff(tree),
                        Series::Xy(xy) => xy.diff(tree),
                        Series::Rule(rule) => rule.diff(tree),
                        Series::Band(band) => band.diff(tree),
                        Series::Tick(tick) => tick.diff(tree),
                        Series::Heatmap(hm) => hm.diff(tree),
                        Series::Diagram(_) => {}
                        Series::Treemap(tm) => tm.diff(tree),
                        Series::Violin(v) => v.diff(tree),
                        Series::Text(t) => t.diff(tree),
                        Series::Annotation(a) => a.diff(tree),
                    }
                }
            },
            |series| match series {
                Series::Area(a) => a.state(),
                Series::Line(line) => line.state(),
                Series::Bars(bars) => bars.state(),
                Series::BoxPlot(bp) => bp.state(),
                Series::Choropleth(c) => c.state(),
                Series::Pie(pie) => pie.state(),
                Series::Gauge(gauge) => gauge.state(),
                Series::Waterfall(wf) => wf.state(),
                Series::Xy(xy) => xy.state(),
                Series::Rule(rule) => rule.state(),
                Series::Band(band) => band.state(),
                Series::Tick(tick) => tick.state(),
                Series::Heatmap(hm) => hm.state(),
                Series::Diagram(_) => diagram::state::<Renderer::Paragraph>(),
                Series::Treemap(tm) => tm.state(),
                Series::Violin(v) => v.state(),
                Series::Text(t) => t.state(),
                Series::Annotation(a) => a.state(),
            },
        );
    }

    /// Compute data bounds from all series.
    fn compute_data_bounds(&self) -> (f64, f64, f64, f64) {
        use crate::mark::area::Layout as AreaLayout;
        use crate::mark::bar::{Direction, Layout};
        use std::collections::HashMap;

        let mut x_min = f64::INFINITY;
        let mut x_max = f64::NEG_INFINITY;
        let mut y_min = f64::INFINITY;
        let mut y_max = f64::NEG_INFINITY;

        for series in &self.series {
            match series {
                Series::Bars(bars) if bars.data.direction == Direction::Horizontal => {
                    // For horizontal: x-values are categories (y-axis),
                    // y-values are magnitudes (x-axis)
                    if bars.data.layout == Layout::Stacked {
                        let mut sums: HashMap<i64, f64> = HashMap::new();
                        for bar_series in &bars.data.series {
                            for point in &bar_series.points {
                                let y_key = (point.x * 1000.0).round() as i64;
                                *sums.entry(y_key).or_insert(0.0) += point.y;
                                y_min = y_min.min(point.x);
                                y_max = y_max.max(point.x);
                            }
                        }
                        for sum in sums.values() {
                            x_min = x_min.min(*sum);
                            x_max = x_max.max(*sum);
                        }
                    } else {
                        for bar_series in &bars.data.series {
                            for point in &bar_series.points {
                                y_min = y_min.min(point.x);
                                y_max = y_max.max(point.x);
                                x_min = x_min.min(point.y);
                                x_max = x_max.max(point.y);
                            }
                        }
                    }
                }
                Series::Area(a) => {
                    if a.data.layout == AreaLayout::Stacked {
                        let mut sums: HashMap<i64, f64> = HashMap::new();
                        for s in &a.data.series {
                            for point in &s.points {
                                let x_key = (point.x * 1000.0).round() as i64;
                                *sums.entry(x_key).or_insert(0.0) += point.y;
                                x_min = x_min.min(point.x);
                                x_max = x_max.max(point.x);
                            }
                        }
                        for sum in sums.values() {
                            y_min = y_min.min(*sum);
                            y_max = y_max.max(*sum);
                        }
                    } else {
                        for s in &a.data.series {
                            for point in &s.points {
                                x_min = x_min.min(point.x);
                                x_max = x_max.max(point.x);
                                y_min = y_min.min(point.y);
                                y_max = y_max.max(point.y);
                            }
                        }
                    }
                }
                Series::Bars(bars) => {
                    // Vertical bars (default)
                    // For stacked layout, compute cumulative sums
                    if bars.data.layout == Layout::Stacked {
                        let mut sums: HashMap<i64, f64> = HashMap::new();

                        for bar_series in &bars.data.series {
                            for point in &bar_series.points {
                                let x_key = (point.x * 1000.0).round() as i64;
                                *sums.entry(x_key).or_insert(0.0) += point.y;

                                x_min = x_min.min(point.x);
                                x_max = x_max.max(point.x);
                            }
                        }

                        for sum in sums.values() {
                            y_min = y_min.min(*sum);
                            y_max = y_max.max(*sum);
                        }
                    } else {
                        // For grouped/overlaid, use individual values
                        for bar_series in &bars.data.series {
                            for point in &bar_series.points {
                                x_min = x_min.min(point.x);
                                x_max = x_max.max(point.x);
                                y_min = y_min.min(point.y);
                                y_max = y_max.max(point.y);
                            }
                        }
                    }
                }
                Series::Line(line) => {
                    for point in &line.data.points {
                        x_min = x_min.min(point.x);
                        x_max = x_max.max(point.x);
                        y_min = y_min.min(point.y);
                        y_max = y_max.max(point.y);
                    }
                }
                Series::Waterfall(wf) => {
                    let mut running: f64 = 0.0;
                    for (i, entry) in wf.data.entries.iter().enumerate() {
                        x_min = x_min.min(i as f64);
                        x_max = x_max.max(i as f64);
                        match entry.kind {
                            crate::mark::waterfall::EntryKind::Total => {
                                running = entry.value;
                            }
                            _ => {
                                running += entry.value;
                            }
                        }
                        y_min = y_min.min(running).min(0.0);
                        y_max = y_max.max(running);
                    }
                }
                Series::Xy(xy) => {
                    for point in &xy.data.points {
                        x_min = x_min.min(point.x);
                        x_max = x_max.max(point.x);
                        y_min = y_min.min(point.y);
                        y_max = y_max.max(point.y);
                    }
                }
                Series::BoxPlot(bp) => match bp.data.direction {
                    crate::mark::boxplot::Direction::Vertical => {
                        for (i, e) in bp.data.entries.iter().enumerate() {
                            x_min = x_min.min(i as f64);
                            x_max = x_max.max(i as f64);
                            y_min = y_min.min(e.min);
                            y_max = y_max.max(e.max);
                            for &o in &e.outliers {
                                y_min = y_min.min(o);
                                y_max = y_max.max(o);
                            }
                        }
                    }
                    crate::mark::boxplot::Direction::Horizontal => {
                        for (i, e) in bp.data.entries.iter().enumerate() {
                            y_min = y_min.min(i as f64);
                            y_max = y_max.max(i as f64);
                            x_min = x_min.min(e.min);
                            x_max = x_max.max(e.max);
                            for &o in &e.outliers {
                                x_min = x_min.min(o);
                                x_max = x_max.max(o);
                            }
                        }
                    }
                },
                Series::Rule(rule) => match rule.data.orientation() {
                    crate::mark::rule::RuleOrientation::Horizontal => {
                        y_min = y_min.min(rule.data.value());
                        y_max = y_max.max(rule.data.value());
                    }
                    crate::mark::rule::RuleOrientation::Vertical => {
                        x_min = x_min.min(rule.data.value());
                        x_max = x_max.max(rule.data.value());
                    }
                },
                Series::Band(band) => match band.data.orientation() {
                    crate::mark::band::BandOrientation::Horizontal => {
                        y_min = y_min.min(band.data.lower());
                        y_max = y_max.max(band.data.upper());
                    }
                    crate::mark::band::BandOrientation::Vertical => {
                        x_min = x_min.min(band.data.lower());
                        x_max = x_max.max(band.data.upper());
                    }
                },
                Series::Tick(tick) => {
                    for point in &tick.data.points {
                        match tick.data.orientation {
                            crate::mark::tick::Orientation::Vertical => {
                                // Horizontal bars: value on x-axis, category on y-axis
                                x_min = x_min.min(point.y);
                                x_max = x_max.max(point.y);
                                y_min = y_min.min(point.x);
                                y_max = y_max.max(point.x);
                            }
                            crate::mark::tick::Orientation::Horizontal => {
                                // Vertical bars: category on x-axis, value on y-axis
                                x_min = x_min.min(point.x);
                                x_max = x_max.max(point.x);
                                y_min = y_min.min(point.y);
                                y_max = y_max.max(point.y);
                            }
                        }
                    }
                }
                Series::Heatmap(hm) => {
                    x_min = x_min.min(0.0);
                    x_max = x_max.max((hm.data.cols() as f64 - 1.0).max(0.0));
                    y_min = y_min.min(0.0);
                    y_max = y_max.max((hm.data.rows() as f64 - 1.0).max(0.0));
                }
                Series::Violin(v) => match v.data.direction {
                    crate::mark::violin::Direction::Vertical => {
                        for (i, e) in v.data.entries.iter().enumerate() {
                            x_min = x_min.min(i as f64);
                            x_max = x_max.max(i as f64);
                            for &(val, _) in &e.density {
                                y_min = y_min.min(val);
                                y_max = y_max.max(val);
                            }
                        }
                    }
                    crate::mark::violin::Direction::Horizontal => {
                        for (i, e) in v.data.entries.iter().enumerate() {
                            y_min = y_min.min(i as f64);
                            y_max = y_max.max(i as f64);
                            for &(val, _) in &e.density {
                                x_min = x_min.min(val);
                                x_max = x_max.max(val);
                            }
                        }
                    }
                },
                Series::Diagram(d) => {
                    if let Some(b) = d.data.bounds() {
                        x_min = x_min.min(b.x_min);
                        x_max = x_max.max(b.x_max);
                        y_min = y_min.min(b.y_min);
                        y_max = y_max.max(b.y_max);
                    }
                }
                // Non-Cartesian marks don't use Cartesian bounds. Text is
                // a passive label layer that projects through whatever
                // plane the underlying point mark uses; its items don't
                // extend the chart's data range.
                // Annotations point into the range; they never extend it.
                Series::Pie(_)
                | Series::Gauge(_)
                | Series::Treemap(_)
                | Series::Choropleth(_)
                | Series::Text(_)
                | Series::Annotation(_) => {}
            }
        }

        // Ensure we have valid bounds even for empty data
        if x_min.is_infinite() || x_max.is_infinite() {
            x_min = 0.0;
            x_max = 1.0;
        }
        if y_min.is_infinite() || y_max.is_infinite() {
            y_min = 0.0;
            y_max = 1.0;
        }

        // For bar/waterfall/area charts, extend the value axis to include zero
        for series in &self.series {
            match series {
                Series::Bars(bars) if bars.data.direction == crate::mark::bar::Direction::Horizontal => {
                    x_min = x_min.min(0.0);
                    break;
                }
                Series::Area(_) | Series::Bars(_) | Series::Waterfall(_) => {
                    y_min = y_min.min(0.0);
                    break;
                }
                _ => {}
            }
        }

        (x_min, x_max, y_min, y_max)
    }

    /// Shapes every label-bearing series' text into its tree state ahead of
    /// the inset measure. Shaping depends only on content + font + size — not
    /// on the data rect — so it runs in the measurement pass; [`min_insets`]
    /// then reads the kept paragraphs' `min_bounds()`, and the final
    /// [`PlotArea::layout`] reuses the same shaped paragraphs (no reshape).
    pub fn shape_labels(&self, tree: &mut Tree, renderer: &Renderer, design: Option<&dyn crate::design::Design>) {
        for (i, series) in self.series.iter().enumerate() {
            if let Series::Bars(bars) = series {
                let series_tree = &mut tree.children[i];
                let state = series_tree.state.downcast_mut::<bars::State<Renderer::Paragraph>>();
                bars.shape_labels(state, renderer, design);
            }
        }
    }

    /// Returns the minimum pixel inset required at each edge of the data-mapping
    /// region for this plot area's series (e.g. data labels that extend past
    /// the end of a bar), folded across series. Used by the scene as a
    /// `min_inset` floor for axis guides so ticks, bars, and labels all align.
    ///
    /// Reads each bar label's already-shaped paragraph (see [`shape_labels`]),
    /// so callers must shape before measuring.
    pub fn min_insets(
        &self,
        tree: &Tree,
        plot_size: crate::core::Size,
        x_bounds: (f64, f64),
        y_bounds: (f64, f64),
    ) -> Insets {
        let mut insets = Insets::default();
        for (i, series) in self.series.iter().enumerate() {
            let Series::Bars(bars) = series else {
                continue;
            };
            let state = tree.children[i]
                .state
                .downcast_ref::<bars::State<Renderer::Paragraph>>();
            let series_insets = bars.min_insets(state, plot_size, x_bounds, y_bounds);
            insets.left = insets.left.max(series_insets.left);
            insets.right = insets.right.max(series_insets.right);
            insets.top = insets.top.max(series_insets.top);
            insets.bottom = insets.bottom.max(series_insets.bottom);
        }
        insets
    }

    /// Layout the plot area - builds the data domains and delegates to each series.
    ///
    /// `mappings` and `secondary_mappings` carry the per-axis transform
    /// and direction for the primary and secondary planes respectively.
    #[allow(clippy::too_many_arguments)]
    pub fn layout(
        &self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &Limits,
        axis_bounds: Option<(f64, f64, f64, f64)>, // (x_min, x_max, y_min, y_max) from primary axes
        secondary_axis_bounds: Option<(f64, f64, f64, f64)>, // bounds from secondary axes
        insets: Insets,                            // Data-rect margins (shared with axis ticks)
        axis_obstacles: &[Rectangle],              // Plot-local axis gutters labels avoid
        mappings: Mappings,
        secondary_mappings: Mappings,
        geo_config: &GeoConfig,
        design: Option<&dyn crate::design::Design>,
    ) {
        let state = tree.state.downcast_mut::<State>();
        let size = limits.bounds();

        // Use axis bounds if provided, otherwise compute from data
        let (x_min, x_max, y_min, y_max) = axis_bounds.unwrap_or_else(|| self.compute_data_bounds());

        let plot_rect = content_rect(size, insets);

        let domain = Domain::new(
            log_safe_range(x_min, x_max, mappings.x),
            log_safe_range(y_min, y_max, mappings.y),
            mappings,
        );

        // Build a secondary domain when the scene provides secondary axis
        // bounds. If not provided but secondary marks exist, fall back to
        // primary bounds so marks still render. Both domains map into the
        // shared `plot_rect`.
        let has_secondary_marks = self.axis_side.contains(&AxisSide::Secondary);
        let secondary_domain = if has_secondary_marks {
            let (sx_min, sx_max, sy_min, sy_max) = secondary_axis_bounds.unwrap_or((x_min, x_max, y_min, y_max));
            Some(Domain::new(
                log_safe_range(sx_min, sx_max, secondary_mappings.x),
                log_safe_range(sy_min, sy_max, secondary_mappings.y),
                secondary_mappings,
            ))
        } else {
            None
        };

        // Build (or reuse) the chart-level geo projection cache when a
        // geo-aware mark is in the series list. The plane's bounds are
        // the full plot-area limits-max with origin (0, 0); per-mark
        // renderers translate by `layout_bounds.{x, y}` when drawing.
        let needs_geo_plane = self.series.iter().any(|s| {
            matches!(s, Series::Choropleth(_))
                || matches!(s, Series::Xy(xy) if xy.data.coord_kind == crate::mark::xy::CoordKind::Geo)
                || matches!(s, Series::Text(t) if t.data.coord_kind == crate::mark::xy::CoordKind::Geo)
        });
        if needs_geo_plane {
            let geo_bounds = Rectangle {
                x: 0.0,
                y: 0.0,
                width: size.width,
                height: size.height,
            };
            state.geo_plane.get_or_insert_with(geo::Plane::new).ensure(
                geo_bounds,
                &geo_config.geo,
                geo_config.scope,
                geo_config.projection,
            );
        } else {
            state.geo_plane = None;
        }

        // Layout each series against its assigned domain, mapped into the
        // shared `plot_rect`. Reborrow the shared geo plane before the
        // dispatch loop so geo-aware marks can read it without holding a
        // mutable borrow on `state` — the post-loop writes to
        // `state.domain` and `state.secondary_domain` reacquire the mutable
        // borrow once this scope ends.
        let geo_plane_ref: Option<&geo::Plane> = state.geo_plane.as_ref();
        for (i, series) in self.series.iter().enumerate() {
            let series_tree = &mut tree.children[i];
            let use_domain: &Domain = match self.axis_side[i] {
                AxisSide::Primary => &domain,
                AxisSide::Secondary => secondary_domain.as_ref().unwrap_or(&domain),
            };
            match series {
                Series::Area(a) => {
                    a.layout(
                        series_tree,
                        renderer,
                        limits,
                        use_domain,
                        plot_rect,
                        axis_obstacles,
                        design,
                    );
                }
                Series::Line(line) => {
                    line.layout(
                        series_tree,
                        renderer,
                        limits,
                        use_domain,
                        plot_rect,
                        axis_obstacles,
                        design,
                    );
                }
                Series::Bars(bars) => {
                    bars.layout(series_tree, renderer, limits, use_domain, plot_rect, design);
                }
                Series::BoxPlot(bp) => {
                    bp.layout(series_tree, renderer, limits, use_domain, plot_rect);
                }
                Series::Choropleth(c) => {
                    c.layout(series_tree, renderer, limits, use_domain, plot_rect, geo_plane_ref);
                }
                Series::Pie(pie) => {
                    pie.layout(series_tree, renderer, limits, use_domain, plot_rect, design);
                }
                Series::Gauge(gauge) => {
                    gauge.layout(series_tree, renderer, limits, use_domain, plot_rect, design);
                }
                Series::Waterfall(wf) => {
                    wf.layout(series_tree, renderer, limits, use_domain, plot_rect, design);
                }
                Series::Xy(xy) => {
                    xy.layout(series_tree, renderer, limits, use_domain, plot_rect, geo_plane_ref);
                }
                Series::Rule(rule) => {
                    rule.layout(series_tree, renderer, limits, use_domain, plot_rect, design);
                }
                Series::Band(band) => {
                    band.layout(series_tree, renderer, limits, use_domain, plot_rect, design);
                }
                Series::Tick(tick) => {
                    tick.layout(series_tree, renderer, limits, use_domain, plot_rect);
                }
                Series::Heatmap(hm) => {
                    hm.layout(series_tree, renderer, limits, use_domain, plot_rect, design);
                }
                Series::Diagram(d) => diagram::layout(
                    d.data,
                    series_tree,
                    renderer,
                    limits.bounds(),
                    use_domain,
                    plot_rect,
                    design,
                ),
                Series::Treemap(tm) => {
                    tm.layout(series_tree, renderer, limits, use_domain, plot_rect, design);
                }
                Series::Violin(v) => {
                    v.layout(series_tree, renderer, limits, use_domain, plot_rect);
                }
                Series::Text(t) => {
                    t.layout(
                        series_tree,
                        renderer,
                        limits,
                        use_domain,
                        plot_rect,
                        geo_plane_ref,
                        design,
                    );
                }
                Series::Annotation(a) => a.layout(series_tree, renderer, design),
            }
        }

        // Store the domains for draw(); cache the shared inset data-mapping
        // rect for the no-Layout hover path.
        state.domain = Some(domain);
        state.secondary_domain = secondary_domain;
        state.content_rect = plot_rect;

        // Annotations place last: an entry target reads the geometry its
        // mark just laid out. Primary marks come first in `series`, so each
        // side's marks form one contiguous range.
        let primary = self.axis_side.iter().filter(|s| **s == AxisSide::Primary).count();
        let bounds = Rectangle::new(crate::core::Point::ORIGIN, size);
        for (i, series) in self.series.iter().enumerate() {
            let Series::Annotation(a) = series else {
                continue;
            };
            let (area, use_domain) = match self.axis_side[i] {
                AxisSide::Primary => (0..primary, &domain),
                AxisSide::Secondary => (primary..self.series.len(), secondary_domain.as_ref().unwrap_or(&domain)),
            };
            a.place(&mut tree.children, i, &self.series, area, use_domain, plot_rect, bounds);
        }

        tree.size = size;
    }

    /// The plot-local shift that carries `axis` from its own edge of the
    /// data rect to its [`crate::axis::Crossing`] value.
    ///
    /// Bottom and left axes cross the primary plane; top and right axes the
    /// secondary plane (or the primary when there's none). The crossing
    /// clamps into the data rect, so a value outside the perpendicular
    /// domain pins the axis to the nearer edge. Zero for edge axes and
    /// before the first layout.
    pub fn crossing_offset(&self, tree: &Tree, axis: &crate::data::Axis) -> crate::core::Vector {
        use crate::data::axis::{Crossing, Orientation};

        let Crossing::Value(value) = axis.crossing() else {
            return crate::core::Vector::ZERO;
        };
        let state = tree.state.downcast_ref::<State>();
        let domain = match axis.orientation() {
            Orientation::Bottom | Orientation::Left => state.domain.as_ref(),
            Orientation::Top | Orientation::Right => state.secondary_domain.as_ref().or(state.domain.as_ref()),
        };
        let Some(domain) = domain else {
            return crate::core::Vector::ZERO;
        };
        if !value.is_finite() {
            return crate::core::Vector::ZERO;
        }
        let rect = state.content_rect;
        let (top, bottom) = (rect.y, rect.y + rect.height);
        let (left, right) = (rect.x, rect.x + rect.width);
        match axis.orientation() {
            Orientation::Bottom | Orientation::Top => {
                let y = to_pixel(domain, rect, Datum::new(domain.x.min, value))
                    .y
                    .clamp(top, bottom);
                let edge = if axis.orientation() == Orientation::Bottom {
                    bottom
                } else {
                    top
                };
                crate::core::Vector::new(0.0, y - edge)
            }
            Orientation::Left | Orientation::Right => {
                let x = to_pixel(domain, rect, Datum::new(value, domain.y.min))
                    .x
                    .clamp(left, right);
                let edge = if axis.orientation() == Orientation::Left {
                    left
                } else {
                    right
                };
                crate::core::Vector::new(x - edge, 0.0)
            }
        }
    }

    /// Draws major and minor gridlines into the plot area.
    ///
    /// Called before `draw()` so marks render on top of gridlines. The tick
    /// positions (in data coordinates) come from the axis guides; they are
    /// converted to pixel positions via the stored [`Domain`] and the cached
    /// content rect.
    #[allow(clippy::too_many_arguments)]
    pub fn draw_gridlines<D>(
        &self,
        tree: &crate::core::widget::Tree,
        renderer: &mut Renderer,
        design: &D,
        layout: crate::core::Layout,
        x_axis: Option<&crate::data::Axis>,
        y_axis: Option<&crate::data::Axis>,
        x_ticks: &[f64],
        y_ticks: &[f64],
    ) where
        D: crate::design::Design + ?Sized,
    {
        use crate::widget::canvas::{Frame, Path, Stroke};

        let state = tree.state.downcast_ref::<State>();
        let Some(domain) = state.domain.as_ref() else {
            return;
        };
        // The cached `content_rect` is the data-mapping region these tick
        // conversions target; reading it here keeps gridlines on the same
        // rect the marks baked their pixels into.
        let rect = state.content_rect;

        let layout_bounds = layout.bounds();
        let size = layout_bounds.size();
        if size.width <= 0.0 || size.height <= 0.0 {
            return;
        }

        let bg = design.background_color();
        let text_pair = design.text_pair();
        let seed = design.seed();

        let shows_major_x = x_axis.map(|a| a.shows_grid()).unwrap_or(false);
        let shows_major_y = y_axis.map(|a| a.shows_grid()).unwrap_or(false);
        let shows_minor_x = x_axis.map(|a| a.shows_minor_grid()).unwrap_or(false);
        let shows_minor_y = y_axis.map(|a| a.shows_minor_grid()).unwrap_or(false);

        if !(shows_major_x || shows_major_y || shows_minor_x || shows_minor_y) {
            return;
        }

        let mut frame = Frame::new(renderer, size);

        // The content rect is the data-mapping region inside the plot area,
        // expressed in plot-area-local pixel coordinates.
        let top = rect.y;
        let bottom = rect.y + rect.height;
        let left = rect.x;
        let right = rect.x + rect.width;

        // Pixel-snap a coordinate to the nearest half-pixel row so a 1 px
        // stroke covers exactly one physical pixel (crisp line) and lands
        // 0.5 px inside the frame edges. This is shared with
        // `draw_axis_borders` so the extreme gridlines and the axis border
        // lines render at *exactly* the same pixel coordinates. Guarded
        // against degenerate frames (`size < 1.0`) so `clamp` never panics.
        let snap_h = |x: f32| {
            let hi = (size.width - 0.5).max(0.5);
            (x.round() + 0.5).clamp(0.5, hi)
        };
        let snap_v = |y: f32| {
            let hi = (size.height - 0.5).max(0.5);
            (y.round() + 0.5).clamp(0.5, hi)
        };

        let minor_subdivs: usize = 4;

        // --- Minor gridlines (drawn first, so majors render on top) ---
        if shows_minor_x && x_ticks.len() >= 2 {
            let color = x_axis
                .and_then(|a| a.minor_grid_color())
                .unwrap_or_else(|| design.minor_grid_color())
                .resolve(bg, text_pair, &seed, None);
            let y0 = snap_v(top);
            let y1 = snap_v(bottom);
            let path = Path::new(|b| {
                for t in minor_ticks(x_ticks, domain.x_mapping.transform, minor_subdivs) {
                    let px = to_pixel(domain, rect, crate::data::Datum::new(t, 0.0)).x;
                    if px >= left && px <= right {
                        let px = snap_h(px);
                        b.move_to(crate::core::Point::new(px, y0));
                        b.line_to(crate::core::Point::new(px, y1));
                    }
                }
            });
            frame.stroke(&path, Stroke::default().with_width(1.0).with_color(color));
        }

        if shows_minor_y && y_ticks.len() >= 2 {
            let color = y_axis
                .and_then(|a| a.minor_grid_color())
                .unwrap_or_else(|| design.minor_grid_color())
                .resolve(bg, text_pair, &seed, None);
            let x0 = snap_h(left);
            let x1 = snap_h(right);
            let path = Path::new(|b| {
                for t in minor_ticks(y_ticks, domain.y_mapping.transform, minor_subdivs) {
                    let py = to_pixel(domain, rect, crate::data::Datum::new(0.0, t)).y;
                    if py >= top && py <= bottom {
                        let py = snap_v(py);
                        b.move_to(crate::core::Point::new(x0, py));
                        b.line_to(crate::core::Point::new(x1, py));
                    }
                }
            });
            frame.stroke(&path, Stroke::default().with_width(1.0).with_color(color));
        }

        // --- Major gridlines ---
        if shows_major_x {
            let color = x_axis
                .and_then(|a| a.grid_color())
                .unwrap_or_else(|| design.grid_color())
                .resolve(bg, text_pair, &seed, None);
            let y0 = snap_v(top);
            let y1 = snap_v(bottom);
            let path = Path::new(|b| {
                for &t in x_ticks {
                    let px = to_pixel(domain, rect, crate::data::Datum::new(t, 0.0)).x;
                    if px >= left && px <= right {
                        let px = snap_h(px);
                        b.move_to(crate::core::Point::new(px, y0));
                        b.line_to(crate::core::Point::new(px, y1));
                    }
                }
            });
            frame.stroke(&path, Stroke::default().with_width(1.0).with_color(color));
        }

        if shows_major_y {
            let color = y_axis
                .and_then(|a| a.grid_color())
                .unwrap_or_else(|| design.grid_color())
                .resolve(bg, text_pair, &seed, None);
            let x0 = snap_h(left);
            let x1 = snap_h(right);
            let path = Path::new(|b| {
                for &t in y_ticks {
                    let py = to_pixel(domain, rect, crate::data::Datum::new(0.0, t)).y;
                    if py >= top && py <= bottom {
                        let py = snap_v(py);
                        b.move_to(crate::core::Point::new(x0, py));
                        b.line_to(crate::core::Point::new(x1, py));
                    }
                }
            });
            frame.stroke(&path, Stroke::default().with_width(1.0).with_color(color));
        }

        let geometry = frame.into_geometry();
        renderer.with_translation(crate::core::Vector::new(layout_bounds.x, layout_bounds.y), |renderer| {
            renderer.draw_geometry(geometry);
        });
    }

    /// Draws the plot area's axis border lines (all four sides).
    ///
    /// The borders are drawn *inside* the plot area frame using the same
    /// pixel-snap as `draw_gridlines`, so each border lands on the exact
    /// same row/column as its corresponding extreme gridline. This makes
    /// the axis frame structurally coincide with the data extent instead
    /// of relying on per-frame "+1 px" hacks to bridge the gap between
    /// the plot frame and a sibling axis frame.
    ///
    /// Each side is gated by its axis's `shows_line()` flag; if a side
    /// has no axis configured, no border is drawn there.
    ///
    /// Should be called *after* `draw()` so the borders render on top of
    /// the marks.
    #[allow(clippy::too_many_arguments)]
    pub fn draw_axis_borders<D>(
        &self,
        tree: &crate::core::widget::Tree,
        renderer: &mut Renderer,
        design: &D,
        layout: crate::core::Layout,
        bottom_axis: Option<&crate::data::Axis>,
        left_axis: Option<&crate::data::Axis>,
        top_axis: Option<&crate::data::Axis>,
        right_axis: Option<&crate::data::Axis>,
    ) where
        D: crate::design::Design + ?Sized,
    {
        use crate::widget::canvas::{Frame, Path, Stroke};

        let state = tree.state.downcast_ref::<State>();
        let Some(_domain) = state.domain.as_ref() else {
            return;
        };
        // The cached `content_rect` is the data extent the border lines
        // bracket; reading it here keeps the borders coincident with the
        // extreme gridlines (which read the same rect in `draw_gridlines`).
        let rect = state.content_rect;

        let layout_bounds = layout.bounds();
        let size = layout_bounds.size();
        if size.width <= 0.0 || size.height <= 0.0 {
            return;
        }

        let draws_left = left_axis.map(|a| a.shows_line()).unwrap_or(false);
        let draws_bottom = bottom_axis.map(|a| a.shows_line()).unwrap_or(false);
        let draws_top = top_axis.map(|a| a.shows_line()).unwrap_or(false);
        let draws_right = right_axis.map(|a| a.shows_line()).unwrap_or(false);
        if !(draws_left || draws_bottom || draws_top || draws_right) {
            return;
        }

        let bg = design.background_color();
        let text_pair = design.text_pair();
        let seed = design.seed();

        // Same snap functions as `draw_gridlines` — this is what makes the
        // border lines structurally coincide with the extreme gridlines.
        // Guarded against degenerate frames so `clamp` never panics.
        let snap_h = |x: f32| {
            let hi = (size.width - 0.5).max(0.5);
            (x.round() + 0.5).clamp(0.5, hi)
        };
        let snap_v = |y: f32| {
            let hi = (size.height - 0.5).max(0.5);
            (y.round() + 0.5).clamp(0.5, hi)
        };

        let top = rect.y;
        let bottom = rect.y + rect.height;
        let left = rect.x;
        let right = rect.x + rect.width;

        // Resolve each border's color lazily so we only pay for it when
        // the side is actually drawn. Colors come from the per-axis
        // override or fall back to `design.axis_color()`.
        let resolve = |axis: Option<&crate::data::Axis>| {
            axis.and_then(|a| a.axis_color())
                .unwrap_or_else(|| design.axis_color())
                .resolve(bg, text_pair, &seed, None)
        };

        let mut frame = Frame::new(renderer, size);
        let x_left = snap_h(left);
        let x_right = snap_h(right);
        let y_top = snap_v(top);
        let y_bottom = snap_v(bottom);

        // Spines that cross the plot at a data value move off their edge;
        // the extents they span stay the full data rect.
        let shift = |axis: Option<&crate::data::Axis>| {
            axis.map_or(crate::core::Vector::ZERO, |axis| self.crossing_offset(tree, axis))
        };
        let spine_left = snap_h(left + shift(left_axis).x);
        let spine_right = snap_h(right + shift(right_axis).x);
        let spine_top = snap_v(top + shift(top_axis).y);
        let spine_bottom = snap_v(bottom + shift(bottom_axis).y);

        if draws_left {
            let path = Path::line(
                crate::core::Point::new(spine_left, y_top),
                crate::core::Point::new(spine_left, y_bottom),
            );
            frame.stroke(&path, Stroke::default().with_width(1.0).with_color(resolve(left_axis)));
        }

        if draws_bottom {
            let path = Path::line(
                crate::core::Point::new(x_left, spine_bottom),
                crate::core::Point::new(x_right, spine_bottom),
            );
            frame.stroke(
                &path,
                Stroke::default().with_width(1.0).with_color(resolve(bottom_axis)),
            );
        }

        if draws_top {
            let path = Path::line(
                crate::core::Point::new(x_left, spine_top),
                crate::core::Point::new(x_right, spine_top),
            );
            frame.stroke(&path, Stroke::default().with_width(1.0).with_color(resolve(top_axis)));
        }

        if draws_right {
            let path = Path::line(
                crate::core::Point::new(spine_right, y_top),
                crate::core::Point::new(spine_right, y_bottom),
            );
            frame.stroke(&path, Stroke::default().with_width(1.0).with_color(resolve(right_axis)));
        }

        let geometry = frame.into_geometry();
        renderer.with_translation(crate::core::Vector::new(layout_bounds.x, layout_bounds.y), |renderer| {
            renderer.draw_geometry(geometry);
        });
    }

    /// Draws the plot area by delegating to each series
    #[allow(clippy::too_many_arguments)]
    #[allow(clippy::too_many_arguments)]
    pub fn draw<D>(
        &self,
        tree: &crate::core::widget::Tree,
        renderer: &mut Renderer,
        design: &D,
        style: &crate::core::renderer::Style,
        layout: crate::core::Layout,
        cursor: crate::core::mouse::Cursor,
        viewport: &crate::core::Rectangle,
        palette: &crate::palette::Resolved,
        chart_user_palette: Option<&crate::palette::Palette>,
        selection: &Option<crate::target::Target>,
        hidden_series: &std::collections::HashSet<String>,
        corners: crate::core::border::Radius,
        track: Option<crate::core::Color>,
        bottom_axis_line: bool,
        left_axis_line: bool,
    ) where
        D: crate::design::Design + ?Sized,
    {
        let mut labels = crate::chart::labels::Layer::new(crate::chart::labels::Space::new(layout.bounds()));
        self.draw_with_labels(
            tree,
            renderer,
            design,
            style,
            layout,
            cursor,
            viewport,
            palette,
            chart_user_palette,
            selection,
            hidden_series,
            corners,
            track,
            bottom_axis_line,
            left_axis_line,
            &mut labels,
        );
        labels.draw(renderer, viewport);
        self.draw_annotations(tree, renderer, design, layout, viewport);
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn draw_with_labels<'p, D>(
        &self,
        tree: &'p crate::core::widget::Tree,
        renderer: &mut Renderer,
        design: &D,
        style: &crate::core::renderer::Style,
        layout: crate::core::Layout,
        cursor: crate::core::mouse::Cursor,
        viewport: &crate::core::Rectangle,
        palette: &crate::palette::Resolved,
        chart_user_palette: Option<&crate::palette::Palette>,
        selection: &Option<crate::target::Target>,
        hidden_series: &std::collections::HashSet<String>,
        corners: crate::core::border::Radius,
        track: Option<crate::core::Color>,
        bottom_axis_line: bool,
        left_axis_line: bool,
        labels: &mut crate::chart::labels::Layer<'p, Renderer::Paragraph>,
    ) where
        D: crate::design::Design + ?Sized,
    {
        // Track cumulative color offset across series
        let mut color_offset: usize = 0;

        // The chart-level geo projection cache, threaded into geo-aware
        // marks (Choropleth) during draw. `None` for purely cartesian
        // charts.
        let plot_state = tree.state.downcast_ref::<State>();
        let geo_plane_ref: Option<&geo::Plane> = plot_state.geo_plane.as_ref();

        // Draw each series
        for (i, series) in self.series.iter().enumerate() {
            let series_tree = &tree.children[i];
            match series {
                Series::Area(a) => {
                    a.draw_with_labels(
                        series_tree,
                        renderer,
                        design,
                        style,
                        layout,
                        cursor,
                        viewport,
                        color_offset,
                        palette,
                        labels,
                    );
                    color_offset += a.data.series.len();
                }
                Series::Line(line) => {
                    let hidden = line.data.name().map(|n| hidden_series.contains(n)).unwrap_or(false);
                    if !hidden {
                        line.draw_with_labels(
                            series_tree,
                            renderer,
                            design,
                            style,
                            layout,
                            cursor,
                            viewport,
                            color_offset,
                            palette,
                            labels,
                        );
                    }
                    color_offset += 1;
                }
                Series::Bars(bars) => {
                    // A bar's baseline sits on the spine perpendicular to its
                    // growth: horizontal bars on the left spine, vertical on
                    // the bottom. The baseline end may round only when that
                    // spine is hidden.
                    let baseline_exposed = if bars.data.direction == crate::mark::bar::Direction::Horizontal {
                        !left_axis_line
                    } else {
                        !bottom_axis_line
                    };
                    bars.draw_with_labels(
                        series_tree,
                        renderer,
                        design,
                        style,
                        layout,
                        cursor,
                        viewport,
                        color_offset,
                        palette,
                        chart_user_palette,
                        i,
                        selection,
                        corners,
                        track,
                        baseline_exposed,
                        hidden_series,
                        labels,
                    );
                    color_offset += bars.data.series.len();
                }
                Series::BoxPlot(bp) => {
                    bp.draw(
                        series_tree,
                        renderer,
                        design,
                        style,
                        layout,
                        cursor,
                        viewport,
                        color_offset,
                        palette,
                    );
                    color_offset += bp.data.entries.len();
                }
                Series::Choropleth(c) => {
                    c.draw(
                        series_tree,
                        renderer,
                        design,
                        style,
                        layout,
                        cursor,
                        viewport,
                        color_offset,
                        palette,
                        geo_plane_ref,
                    );
                }
                Series::Pie(pie) => {
                    pie.draw_with_labels(
                        series_tree,
                        renderer,
                        design,
                        style,
                        layout,
                        cursor,
                        viewport,
                        color_offset,
                        palette,
                        i,
                        selection,
                        corners,
                        labels,
                    );
                    color_offset += pie.data.slices.len();
                }
                Series::Gauge(gauge) => {
                    gauge.draw_with_labels(
                        series_tree,
                        renderer,
                        design,
                        style,
                        layout,
                        cursor,
                        viewport,
                        color_offset,
                        palette,
                        corners,
                        labels,
                    );
                    color_offset += 1;
                }
                Series::Waterfall(wf) => {
                    wf.draw_with_labels(series_tree, renderer, design, style, layout, cursor, viewport, labels);
                    color_offset += 3;
                }
                Series::Xy(xy) => {
                    let hidden = xy.data.name().map(|n| hidden_series.contains(n)).unwrap_or(false);
                    if !hidden {
                        xy.draw(
                            series_tree,
                            renderer,
                            design,
                            style,
                            layout,
                            cursor,
                            viewport,
                            color_offset,
                            palette,
                        );
                    }
                    color_offset += 1;
                }
                Series::Rule(rule) => {
                    rule.draw_with_labels(series_tree, renderer, design, style, layout, cursor, viewport, labels);
                    // Rules don't consume color slots
                }
                Series::Band(band) => {
                    band.draw_with_labels(series_tree, renderer, design, style, layout, cursor, viewport, labels);
                    // Bands don't consume color slots
                }
                Series::Tick(tick) => {
                    tick.draw(series_tree, renderer, design, style, layout, cursor, viewport);
                    // Ticks don't consume color slots
                }
                Series::Heatmap(hm) => {
                    hm.draw_with_labels(
                        series_tree,
                        renderer,
                        design,
                        style,
                        layout,
                        cursor,
                        viewport,
                        color_offset,
                        palette,
                        labels,
                    );
                    // Heatmap uses gradient sampling, not discrete slots
                }
                Series::Diagram(d) => {
                    let selected = match selection {
                        Some(crate::Target::Entry { mark, index, .. }) if *mark == i => Some(*index),
                        _ => None,
                    };
                    diagram::draw(
                        d,
                        series_tree,
                        renderer,
                        design,
                        layout,
                        color_offset,
                        palette,
                        labels,
                        selected,
                    );
                    color_offset += d.data.entries.len();
                }
                Series::Treemap(tm) => {
                    tm.draw_with_labels(
                        series_tree,
                        renderer,
                        design,
                        style,
                        layout,
                        cursor,
                        viewport,
                        color_offset,
                        palette,
                        labels,
                    );
                    color_offset += tm.data.items.len();
                }
                Series::Violin(v) => {
                    v.draw(
                        series_tree,
                        renderer,
                        design,
                        style,
                        layout,
                        cursor,
                        viewport,
                        color_offset,
                        palette,
                    );
                    color_offset += v.data.entries.len();
                }
                Series::Text(t) => {
                    t.draw_with_labels(series_tree, renderer, design, style, layout, cursor, viewport, labels);
                    // Text labels don't claim a palette slot.
                }
                // Drawn by `draw_annotations`, above every mark and label.
                Series::Annotation(_) => {}
            }
        }
    }

    /// Draws every annotation on top of the marks and their data labels.
    /// Call after the labels layer from [`PlotArea::draw_with_labels`] has
    /// been drawn.
    pub(crate) fn draw_annotations<D>(
        &self,
        tree: &crate::core::widget::Tree,
        renderer: &mut Renderer,
        design: &D,
        layout: crate::core::Layout,
        viewport: &crate::core::Rectangle,
    ) where
        D: crate::design::Design + ?Sized,
    {
        for (i, series) in self.series.iter().enumerate() {
            if let Series::Annotation(a) = series {
                a.draw(&tree.children[i], renderer, design, layout, viewport);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::Size;

    fn insets(left: f32, right: f32, top: f32, bottom: f32) -> Insets {
        Insets {
            left,
            right,
            top,
            bottom,
        }
    }

    #[test]
    fn annotations_do_not_extend_plot_bounds() {
        let plain: Vec<crate::Mark> = vec![crate::line([2.0, 8.0, 5.0]).into()];
        let mut annotated = plain.clone();
        annotated.push(crate::annotation(-50.0, 1e6).note("off the chart").into());
        let bounds = |marks: &[crate::Mark]| PlotArea::<(), crate::widget::Renderer>::new(marks).compute_data_bounds();
        assert_eq!(bounds(&plain), bounds(&annotated));
    }

    #[test]
    fn content_rect_sits_at_inset_corner() {
        let size = Size::new(800.0, 600.0);
        let rect = content_rect(size, insets(48.0, 12.0, 8.0, 40.0));

        // Plot-local origin sits at the inset corner, not the plot origin.
        assert_eq!(rect.x, 48.0);
        assert_eq!(rect.y, 8.0);
        assert_eq!(rect.width, 800.0 - 48.0 - 12.0);
        assert_eq!(rect.height, 600.0 - 8.0 - 40.0);
    }

    #[test]
    fn content_rect_clamps_oversized_insets() {
        let size = Size::new(100.0, 100.0);
        // Insets exceeding the plot collapse the rect to zero, never negative.
        let rect = content_rect(size, insets(80.0, 80.0, 70.0, 70.0));
        assert_eq!(rect.width, 0.0);
        assert_eq!(rect.height, 0.0);
    }

    fn log_x_reversed_y() -> Domain {
        Domain::new(
            Range { min: 1.0, max: 1000.0 },
            Range { min: 0.0, max: 100.0 },
            Mappings {
                x: crate::scale::Mapping::new(crate::scale::Transform::Log, crate::axis::Order::Forward),
                y: crate::scale::Mapping::new(crate::scale::Transform::Linear, crate::axis::Order::Reverse),
            },
        )
    }

    #[test]
    fn to_pixel_honors_log_x_and_reversed_y() {
        let domain = log_x_reversed_y();
        let rect = Rectangle::new(crate::core::Point::new(10.0, 20.0), Size::new(300.0, 200.0));
        let p = to_pixel(&domain, rect, Datum::new(10.0, 0.0));
        // 10 sits one third along a three-decade axis.
        assert!((p.x - 110.0).abs() < 1e-3, "x was {}", p.x);
        // Reversed y: the minimum sits at the top edge.
        assert!((p.y - 20.0).abs() < 1e-3, "y was {}", p.y);
        let p = to_pixel(&domain, rect, Datum::new(1000.0, 100.0));
        assert!((p.x - 310.0).abs() < 1e-3);
        assert!((p.y - 220.0).abs() < 1e-3);
    }

    #[test]
    fn to_data_x_inverts_log_x() {
        let domain = log_x_reversed_y();
        let rect = Rectangle::new(crate::core::Point::new(10.0, 20.0), Size::new(300.0, 200.0));
        for x in [1.0, 4.0, 10.0, 250.0, 1000.0] {
            let px = to_pixel(&domain, rect, Datum::new(x, 50.0)).x;
            let back = to_data_x(&domain, rect, px);
            assert!((back - x).abs() < 1e-3 * x, "{x} -> {px} -> {back}");
        }
    }

    #[test]
    fn log_safe_range_lifts_nonpositive_minimum() {
        let log = crate::scale::Mapping::new(crate::scale::Transform::Log, crate::axis::Order::Forward);
        let r = log_safe_range(0.0, 500.0, log);
        assert_eq!((r.min, r.max), (0.05, 500.0));
        let r = log_safe_range(0.0, 500.0, crate::scale::Mapping::default());
        assert_eq!((r.min, r.max), (0.0, 500.0));
    }

    #[test]
    fn minor_ticks_rule_log_decades() {
        let minors = minor_ticks(&[1.0, 10.0, 100.0], crate::scale::Transform::Log, 4);
        let expected: Vec<f64> = (2..=9)
            .map(f64::from)
            .chain((2..=9).map(|k| f64::from(k) * 10.0))
            .collect();
        assert_eq!(minors.len(), expected.len());
        for (m, e) in minors.iter().zip(&expected) {
            assert!((m - e).abs() < 1e-9, "{m} vs {e}");
        }
        // A strided major sequence keeps the skipped decade as a minor.
        let minors = minor_ticks(&[1.0, 100.0], crate::scale::Transform::Log, 4);
        assert!(minors.iter().any(|m| (m - 10.0).abs() < 1e-9));
    }

    #[test]
    fn minor_ticks_split_linear_intervals() {
        let minors = minor_ticks(&[0.0, 4.0], crate::scale::Transform::Linear, 4);
        assert_eq!(minors, vec![1.0, 2.0, 3.0]);
    }
}
