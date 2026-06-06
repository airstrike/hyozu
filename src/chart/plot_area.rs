use crate::core::Rectangle;
use crate::core::layout::{Limits, Node};
use crate::core::widget::{Tree, tree};
use crate::data::Datum;

use crate::widget::renderer::geometry;

pub mod area;
pub mod band;
pub mod bars;
pub mod boxplot;
pub mod choropleth;
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
/// plus the transform applied to the y axis. Carries no pixel geometry — the
/// destination rectangle is supplied per call to [`to_pixel`] / [`to_data_x`].
#[derive(Debug, Clone, Copy)]
pub struct Domain {
    /// Data range along the x axis.
    pub x: Range,
    /// Data range along the y axis.
    pub y: Range,
    /// Transform applied to the y domain when mapping to pixels.
    /// Honored by numeric-axis marks via [`to_pixel`]; categorical or
    /// geographic marks (Pie, Treemap, Choropleth) bypass the domain
    /// entirely and aren't affected.
    pub y_transform: crate::scale::Transform,
}

/// Transform a data point (f64) to pixel coordinates (f32) within `rect`.
pub fn to_pixel(domain: &Domain, rect: Rectangle, datum: Datum) -> crate::core::Point {
    let x = if domain.x.max > domain.x.min {
        rect.x + (((datum.x - domain.x.min) / (domain.x.max - domain.x.min)) as f32) * rect.width
    } else {
        rect.x + rect.width / 2.0
    };

    let y = if domain.y.max > domain.y.min {
        let unit = domain.y_transform.map_to_unit(datum.y, domain.y.min, domain.y.max) as f32;
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
        domain.x.min + (t as f64) * (domain.x.max - domain.x.min)
    }
}

/// Build the `content` child node from the inset data-mapping rect.
///
/// The node is the shared data-mapping region — the plot rect inset by
/// the per-edge margins. Its bounds are plot-local (origin at
/// `content_rect.{x, y}` = `insets.{left, top}`). Primary and secondary
/// planes map data into this same rectangle. The node is structural:
/// marks still translate by the plot node at draw, never this child.
fn content_node(content_rect: Rectangle) -> Node {
    Node::new(crate::core::Size::new(content_rect.width, content_rect.height))
        .move_to(crate::core::Point::new(content_rect.x, content_rect.y))
}

/// The coordinate plane for transforming data coords to pixels.
#[derive(Debug, Clone)]
pub struct Plane {
    /// Data domain mapped onto [`Plane::bounds`].
    pub domain: Domain,
    pub bounds: Rectangle,
}

/// Pixel margins reserved on each edge of the plot area to seat the
/// data-mapping region. The same per-edge value seats the axis ticks and the
/// content node, so marks and ticks stay aligned by construction.
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

impl Plane {
    /// Transform a data point (f64) to pixel coordinates (f32) within the plane bounds.
    pub fn to_pixel(&self, datum: Datum) -> crate::core::Point {
        to_pixel(&self.domain, self.bounds, datum)
    }

    /// Inverse of the x part of `to_pixel`: map a pixel x-coordinate back to
    /// a data-space x value.
    pub fn to_data_x(&self, pixel_x: f32) -> f64 {
        to_data_x(&self.domain, self.bounds, pixel_x)
    }
}

/// A series that can be rendered in the plot area
pub enum Series<'a, Message, Renderer>
where
    Message: 'a,
    Renderer: crate::core::text::Renderer<Font = crate::core::Font> + geometry::Renderer,
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
    Treemap(Treemap<'a, Message, Renderer>),
    Violin(Violin<'a, Message, Renderer>),
    Text(Text<'a, Message, Renderer>),
}

/// Which axis pair a mark is plotted against.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AxisSide {
    /// Primary (bottom + left) axis pair.
    Primary,
    /// Secondary (top + right) axis pair.
    Secondary,
}

/// State for a PlotArea - stores the coordinate planes for rendering
pub struct State {
    /// The coordinate plane for the primary (bottom/left) axes.
    pub plane: Option<Plane>,
    /// The coordinate plane for the secondary (top/right) axes, if any.
    pub secondary_plane: Option<Plane>,
    /// The inset data-mapping rectangle in plot-local coordinates (origin
    /// at `insets.{left, top}`), mirroring the `content` child node. The
    /// update/hover path has no child [`crate::core::Layout`] to descend,
    /// so it reads this cache instead of the live node tree.
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
    Renderer: crate::core::text::Renderer<Font = crate::core::Font> + geometry::Renderer,
{
    pub(crate) series: Vec<Series<'a, Message, Renderer>>,
    /// Which axis pair each series in `series` is plotted against.
    pub(crate) axis_side: Vec<AxisSide>,
}

impl<'a, Message, Renderer> PlotArea<'a, Message, Renderer>
where
    Message: 'a,
    Renderer: crate::core::text::Renderer<Font = crate::core::Font> + geometry::Renderer,
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
            crate::Mark::Treemap(tm) => Series::Treemap(Treemap::new(tm)),
            crate::Mark::Violin(v) => Series::Violin(Violin::new(v)),
            crate::Mark::Text(t) => Series::Text(Text::new(t)),
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
                Series::Treemap(tm) => tm.data.items.len(),
                Series::Waterfall(_) => 3,
                Series::Heatmap(_) => 0,
                Series::Choropleth(_) => 0,
                Series::Violin(v) => v.data.entries.len(),
                Series::Tick(_) => 0,
                Series::Rule(_) => 0,
                Series::Band(_) => 0,
                Series::Text(_) => 0,
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
                Series::Treemap(tm) => tm.state(),
                Series::Violin(v) => v.state(),
                Series::Text(t) => t.state(),
            })
            .collect();

        Tree {
            tag: tree::Tag::of::<State>(),
            state: tree::State::new(State {
                plane: None,
                secondary_plane: None,
                content_rect: Rectangle {
                    x: 0.0,
                    y: 0.0,
                    width: 0.0,
                    height: 0.0,
                },
                geo_plane: None,
            }),
            children,
        }
    }

    /// Reconcile the tree with current PlotArea state
    pub(super) fn diff(&self, tree: &mut Tree) {
        // Diff each series
        tree.diff_children_custom(
            &self.series,
            |tree, series| {
                // When the mark type changes (e.g. Bars → Pie), the tree
                // node still holds the old state type.  Detect this via
                // the tag and rebuild the node from scratch.
                let expected_tag = match series {
                    Series::Area(_) => tree::Tag::of::<area::State>(),
                    Series::Line(_) => tree::Tag::of::<line::State>(),
                    Series::Bars(_) => tree::Tag::of::<bars::State<Renderer::Paragraph>>(),
                    Series::BoxPlot(_) => tree::Tag::of::<boxplot::State>(),
                    Series::Choropleth(_) => tree::Tag::of::<choropleth::State>(),
                    Series::Pie(_) => tree::Tag::of::<pie::State>(),
                    Series::Gauge(_) => tree::Tag::of::<gauge::State>(),
                    Series::Waterfall(_) => tree::Tag::of::<waterfall::State>(),
                    Series::Xy(_) => tree::Tag::of::<xy::State>(),
                    Series::Rule(_) => tree::Tag::of::<rule::State>(),
                    Series::Band(_) => tree::Tag::of::<band::State>(),
                    Series::Tick(_) => tree::Tag::of::<tick::State>(),
                    Series::Heatmap(_) => tree::Tag::of::<heatmap::State>(),
                    Series::Treemap(_) => tree::Tag::of::<treemap::State>(),
                    Series::Violin(_) => tree::Tag::of::<violin::State>(),
                    Series::Text(_) => tree::Tag::of::<text::State>(),
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
                        Series::Treemap(tm) => tm.state(),
                        Series::Violin(v) => v.state(),
                        Series::Text(t) => t.state(),
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
                        Series::Treemap(tm) => tm.diff(tree),
                        Series::Violin(v) => v.diff(tree),
                        Series::Text(t) => t.diff(tree),
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
                Series::Treemap(tm) => tm.state(),
                Series::Violin(v) => v.state(),
                Series::Text(t) => t.state(),
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
                // Non-Cartesian marks don't use Cartesian bounds. Text is
                // a passive label layer that projects through whatever
                // plane the underlying point mark uses; its items don't
                // extend the chart's data range.
                Series::Pie(_) | Series::Gauge(_) | Series::Treemap(_) | Series::Choropleth(_) | Series::Text(_) => {}
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
    pub fn shape_labels(&self, tree: &mut Tree, renderer: &Renderer) {
        for (i, series) in self.series.iter().enumerate() {
            if let Series::Bars(bars) = series {
                let series_tree = &mut tree.children[i];
                let state = series_tree.state.downcast_mut::<bars::State<Renderer::Paragraph>>();
                bars.shape_labels(state, renderer);
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

    /// Layout the plot area - creates plane and delegates to each series.
    ///
    /// `y_transform` and `secondary_y_transform` carry the y-domain
    /// transform (Linear / Log) for the primary and secondary planes
    /// respectively. The x-axis transform isn't plumbed here yet;
    /// `XScale` doesn't expose a numeric transform on its `Linear`
    /// variant in the v1 surface.
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
        y_transform: crate::scale::Transform,
        secondary_y_transform: crate::scale::Transform,
        geo_config: &GeoConfig,
    ) -> Node {
        let state = tree.state.downcast_mut::<State>();
        let size = limits.max();

        // Use axis bounds if provided, otherwise compute from data
        let (x_min, x_max, mut y_min, y_max) = axis_bounds.unwrap_or_else(|| self.compute_data_bounds());

        // Log axes can't show non-positive values; the transform clamps
        // to `f64::EPSILON` for mapping anyway, which spans ~16 decades
        // of empty space below any real data. When the user passes
        // `y_axis_bounds(0.0, …)` on a log-scaled chart, clamp y_min to
        // a sane positive default derived from y_max so the rendered
        // range matches what tick generation produces.
        if matches!(y_transform, crate::scale::Transform::Log) && y_min <= 0.0 && y_max > 0.0 {
            y_min = y_max / 1e4;
        }

        let plot_rect = Rectangle {
            x: insets.left,
            y: insets.top,
            width: (size.width - insets.left - insets.right).max(0.0),
            height: (size.height - insets.top - insets.bottom).max(0.0),
        };

        let plane = Plane {
            domain: Domain {
                x: Range { min: x_min, max: x_max },
                y: Range { min: y_min, max: y_max },
                y_transform,
            },
            bounds: plot_rect,
        };

        // Build a secondary plane when the scene provides secondary axis
        // bounds. If not provided but secondary marks exist, fall back to
        // primary bounds so marks still render.
        let has_secondary_marks = self.axis_side.contains(&AxisSide::Secondary);
        let secondary_plane = if has_secondary_marks {
            let (sx_min, sx_max, mut sy_min, sy_max) = secondary_axis_bounds.unwrap_or((x_min, x_max, y_min, y_max));
            if matches!(secondary_y_transform, crate::scale::Transform::Log) && sy_min <= 0.0 && sy_max > 0.0 {
                sy_min = sy_max / 1e4;
            }
            Some(Plane {
                domain: Domain {
                    x: Range {
                        min: sx_min,
                        max: sx_max,
                    },
                    y: Range {
                        min: sy_min,
                        max: sy_max,
                    },
                    y_transform: secondary_y_transform,
                },
                bounds: plot_rect,
            })
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

        // Layout each series with its assigned plane. Reborrow the
        // shared geo plane before the dispatch loop so geo-aware marks
        // can read it without holding a mutable borrow on `state` —
        // the post-loop writes to `state.plane` and
        // `state.secondary_plane` reacquire the mutable borrow once
        // this scope ends.
        let geo_plane_ref: Option<&geo::Plane> = state.geo_plane.as_ref();
        for (i, series) in self.series.iter().enumerate() {
            let series_tree = &mut tree.children[i];
            let use_plane: &Plane = match self.axis_side[i] {
                AxisSide::Primary => &plane,
                AxisSide::Secondary => secondary_plane.as_ref().unwrap_or(&plane),
            };
            match series {
                Series::Area(a) => {
                    a.layout(series_tree, renderer, limits, use_plane, axis_obstacles);
                }
                Series::Line(line) => {
                    line.layout(series_tree, renderer, limits, use_plane, axis_obstacles);
                }
                Series::Bars(bars) => {
                    bars.layout(series_tree, renderer, limits, use_plane);
                }
                Series::BoxPlot(bp) => {
                    bp.layout(series_tree, renderer, limits, use_plane);
                }
                Series::Choropleth(c) => {
                    c.layout(series_tree, renderer, limits, use_plane, geo_plane_ref);
                }
                Series::Pie(pie) => {
                    pie.layout(series_tree, renderer, limits, use_plane);
                }
                Series::Gauge(gauge) => {
                    gauge.layout(series_tree, renderer, limits, use_plane);
                }
                Series::Waterfall(wf) => {
                    wf.layout(series_tree, renderer, limits, use_plane);
                }
                Series::Xy(xy) => {
                    xy.layout(series_tree, renderer, limits, use_plane, geo_plane_ref);
                }
                Series::Rule(rule) => {
                    rule.layout(series_tree, renderer, limits, use_plane);
                }
                Series::Band(band) => {
                    band.layout(series_tree, renderer, limits, use_plane);
                }
                Series::Tick(tick) => {
                    tick.layout(series_tree, renderer, limits, use_plane);
                }
                Series::Heatmap(hm) => {
                    hm.layout(series_tree, renderer, limits, use_plane);
                }
                Series::Treemap(tm) => {
                    tm.layout(series_tree, renderer, limits, use_plane);
                }
                Series::Violin(v) => {
                    v.layout(series_tree, renderer, limits, use_plane);
                }
                Series::Text(t) => {
                    t.layout(series_tree, renderer, limits, use_plane, geo_plane_ref);
                }
            }
        }

        // Store the planes for draw()
        state.plane = Some(plane);
        state.secondary_plane = secondary_plane;
        // Cache the inset data-mapping rect for the no-Layout hover path.
        state.content_rect = plot_rect;

        Node::with_children(size, vec![content_node(plot_rect)])
    }

    /// Draws major and minor gridlines into the plot area using the stored plane.
    ///
    /// Called before `draw()` so marks render on top of gridlines. The tick
    /// positions (in data coordinates) come from the axis guides; they are
    /// converted to pixel positions via the plane.
    #[allow(clippy::too_many_arguments)]
    pub fn draw_gridlines<D>(
        &self,
        tree: &crate::core::widget::Tree,
        renderer: &mut Renderer,
        design: &D,
        layout: crate::core::Layout<'_>,
        x_axis: Option<&crate::data::Axis>,
        y_axis: Option<&crate::data::Axis>,
        x_ticks: &[f64],
        y_ticks: &[f64],
    ) where
        D: crate::design::Design + ?Sized,
    {
        use crate::widget::canvas::{Frame, Path, Stroke};

        let state = tree.state.downcast_ref::<State>();
        let Some(plane) = state.plane.as_ref() else {
            return;
        };

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

        // Plane bounds are the data-mapping region inside the plot area,
        // expressed in plot-area-local pixel coordinates.
        let top = plane.bounds.y;
        let bottom = plane.bounds.y + plane.bounds.height;
        let left = plane.bounds.x;
        let right = plane.bounds.x + plane.bounds.width;

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
                for w in x_ticks.windows(2) {
                    let (t0, t1) = (w[0], w[1]);
                    let step = (t1 - t0) / minor_subdivs as f64;
                    for k in 1..minor_subdivs {
                        let t = t0 + step * k as f64;
                        let px = plane.to_pixel(crate::data::Datum::new(t, 0.0)).x;
                        if px >= left && px <= right {
                            let px = snap_h(px);
                            b.move_to(crate::core::Point::new(px, y0));
                            b.line_to(crate::core::Point::new(px, y1));
                        }
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
                for w in y_ticks.windows(2) {
                    let (t0, t1) = (w[0], w[1]);
                    let step = (t1 - t0) / minor_subdivs as f64;
                    for k in 1..minor_subdivs {
                        let t = t0 + step * k as f64;
                        let py = plane.to_pixel(crate::data::Datum::new(0.0, t)).y;
                        if py >= top && py <= bottom {
                            let py = snap_v(py);
                            b.move_to(crate::core::Point::new(x0, py));
                            b.line_to(crate::core::Point::new(x1, py));
                        }
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
                    let px = plane.to_pixel(crate::data::Datum::new(t, 0.0)).x;
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
                    let py = plane.to_pixel(crate::data::Datum::new(0.0, t)).y;
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
        layout: crate::core::Layout<'_>,
        bottom_axis: Option<&crate::data::Axis>,
        left_axis: Option<&crate::data::Axis>,
        top_axis: Option<&crate::data::Axis>,
        right_axis: Option<&crate::data::Axis>,
    ) where
        D: crate::design::Design + ?Sized,
    {
        use crate::widget::canvas::{Frame, Path, Stroke};

        let state = tree.state.downcast_ref::<State>();
        let Some(plane) = state.plane.as_ref() else {
            return;
        };

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

        let top = plane.bounds.y;
        let bottom = plane.bounds.y + plane.bounds.height;
        let left = plane.bounds.x;
        let right = plane.bounds.x + plane.bounds.width;

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

        if draws_left {
            let path = Path::line(
                crate::core::Point::new(x_left, y_top),
                crate::core::Point::new(x_left, y_bottom),
            );
            frame.stroke(&path, Stroke::default().with_width(1.0).with_color(resolve(left_axis)));
        }

        if draws_bottom {
            let path = Path::line(
                crate::core::Point::new(x_left, y_bottom),
                crate::core::Point::new(x_right, y_bottom),
            );
            frame.stroke(
                &path,
                Stroke::default().with_width(1.0).with_color(resolve(bottom_axis)),
            );
        }

        if draws_top {
            let path = Path::line(
                crate::core::Point::new(x_left, y_top),
                crate::core::Point::new(x_right, y_top),
            );
            frame.stroke(&path, Stroke::default().with_width(1.0).with_color(resolve(top_axis)));
        }

        if draws_right {
            let path = Path::line(
                crate::core::Point::new(x_right, y_top),
                crate::core::Point::new(x_right, y_bottom),
            );
            frame.stroke(&path, Stroke::default().with_width(1.0).with_color(resolve(right_axis)));
        }

        let geometry = frame.into_geometry();
        renderer.with_translation(crate::core::Vector::new(layout_bounds.x, layout_bounds.y), |renderer| {
            renderer.draw_geometry(geometry);
        });
    }

    /// Draws color-scale legends owned by member series (e.g. choropleth).
    ///
    /// `plot_layout` is the plot area's own layout (used when a legend's
    /// placement is `Overlaid`: the legend draws inside the plot bounds).
    /// `strip_rects` maps a series index within this plot area to the
    /// scene-reserved strip rectangle for that series' `Placement::Inset`
    /// legend — when present, the legend draws inside the strip instead.
    /// Series without an entry (or with no scale legend at all) don't draw
    /// anything.
    pub fn draw_scale_legends<D>(
        &self,
        tree: &crate::core::widget::Tree,
        renderer: &mut Renderer,
        design: &D,
        plot_layout: crate::core::Layout<'_>,
        strip_rects: &std::collections::HashMap<usize, crate::core::Rectangle>,
    ) where
        D: crate::design::Design + ?Sized,
    {
        let plot_bounds = plot_layout.bounds();
        for (i, series) in self.series.iter().enumerate() {
            if let Series::Choropleth(c) = series {
                let series_tree = &tree.children[i];
                let strip = strip_rects.get(&i).copied();
                c.draw_scale_legend(series_tree, renderer, design, plot_bounds, strip);
            }
        }
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
        layout: crate::core::Layout<'_>,
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
                    a.draw(
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
                    color_offset += a.data.series.len();
                }
                Series::Line(line) => {
                    let hidden = line.data.name().map(|n| hidden_series.contains(n)).unwrap_or(false);
                    if !hidden {
                        line.draw(
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
                    bars.draw(
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
                    pie.draw(
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
                    );
                    color_offset += pie.data.slices.len();
                }
                Series::Gauge(gauge) => {
                    gauge.draw(
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
                    );
                    color_offset += 1;
                }
                Series::Waterfall(wf) => {
                    wf.draw(series_tree, renderer, design, style, layout, cursor, viewport);
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
                    rule.draw(series_tree, renderer, design, style, layout, cursor, viewport);
                    // Rules don't consume color slots
                }
                Series::Band(band) => {
                    band.draw(series_tree, renderer, design, style, layout, cursor, viewport);
                    // Bands don't consume color slots
                }
                Series::Tick(tick) => {
                    tick.draw(series_tree, renderer, design, style, layout, cursor, viewport);
                    // Ticks don't consume color slots
                }
                Series::Heatmap(hm) => {
                    hm.draw(
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
                    // Heatmap uses gradient sampling, not discrete slots
                }
                Series::Treemap(tm) => {
                    tm.draw(
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
                    t.draw(series_tree, renderer, design, style, layout, cursor, viewport);
                    // Text labels don't claim a palette slot.
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::Size;

    /// The inset data-mapping rect in plot-local coordinates, as built in
    /// [`PlotArea::layout`]. Mirrors the `plot_rect` formula so the test
    /// asserts the content node against the value layout actually uses.
    fn plot_rect(size: Size, left: f32, right: f32, top: f32, bottom: f32) -> Rectangle {
        Rectangle {
            x: left,
            y: top,
            width: (size.width - left - right).max(0.0),
            height: (size.height - top - bottom).max(0.0),
        }
    }

    #[test]
    fn content_node_bounds_equal_inset_rect() {
        let size = Size::new(800.0, 600.0);
        let rect = plot_rect(size, 48.0, 12.0, 8.0, 40.0);

        let bounds = content_node(rect).bounds();

        // Plot-local origin sits at the inset corner, not the plot origin.
        assert_eq!(bounds.x, rect.x);
        assert_eq!(bounds.y, rect.y);
        assert_eq!(bounds.width, rect.width);
        assert_eq!(bounds.height, rect.height);
    }

    #[test]
    fn content_node_clamps_oversized_insets() {
        let size = Size::new(100.0, 100.0);
        // Insets exceeding the plot collapse the rect to zero, never negative.
        let rect = plot_rect(size, 80.0, 80.0, 70.0, 70.0);
        assert_eq!(rect.width, 0.0);
        assert_eq!(rect.height, 0.0);

        let bounds = content_node(rect).bounds();
        assert_eq!(bounds.width, 0.0);
        assert_eq!(bounds.height, 0.0);
    }
}
