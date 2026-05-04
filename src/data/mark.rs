pub mod annotation;
pub mod area;
pub mod band;
pub mod bar;
pub mod boxplot;
pub mod bubble_map;
pub mod choropleth;
pub mod gauge;
pub mod heatmap;
pub mod line;
pub mod pie;
pub mod rule;
pub mod tick;
pub mod treemap;
pub mod violin;
pub mod waterfall;
pub mod xy;

pub use area::{Area, IntoAreas, area, areas};
pub use band::{Band, BandOrientation, band};
pub use bar::{Bars, IntoBars, bar, bars};
pub use boxplot::{BoxPlot, boxplot, entry, entry_from_data};
pub use bubble_map::{BubbleMap, MapPoint, bubble_map, map_point};
pub use choropleth::{
    Choropleth, ChoroplethEntry, IntoChoropleth, Normalization, choropleth, choropleth_entry,
    choropleth_entry_available,
};
pub use gauge::{Gauge, gauge};
pub use heatmap::{Heatmap, heatmap};
pub use line::{IntoLines, Line, LineStyle, line};
pub use pie::{Pie, pie};
pub use rule::{Rule, rule};
pub use tick::{Tick, tick};
pub use treemap::{Treemap, treemap};
pub use violin::{Violin, violin, violin_entry, violin_from_data};
pub use waterfall::{Waterfall, waterfall};
pub use xy::{Xy, xy};

/// Visual style of the legend swatch next to an entry's label.
#[derive(Debug, Clone, Default)]
pub enum LegendSwatch {
    /// Filled rounded square. Default for fill-based marks (bars, pie, etc.).
    #[default]
    Square,
    /// Horizontal line with an optional marker overlay — used for line and
    /// area series.
    Line {
        style: line::LineStyle,
        marker: Option<line::marker::Marker>,
    },
}

/// A single entry in the chart legend.
#[derive(Debug, Clone)]
pub struct LegendEntry {
    /// The display name for this entry.
    pub name: String,
    /// Optional color for the swatch. When `None`, the palette index is used.
    pub color: Option<crate::color::Color>,
    /// Visual style of the swatch preceding the label.
    pub swatch: LegendSwatch,
}

/// Represents a visual mark in a chart (bars, lines, scatter, etc.)
///
/// Parameterized over `Message`, `Theme`, `Renderer` so the `Pie` variant
/// can carry a donut-hole overlay closure. Other variants ignore the
/// parameters; defaults make the common case ergonomic.
pub enum Mark<Message = (), Theme = crate::core::Theme, Renderer = crate::widget::Renderer>
where
    Message: 'static,
    Theme: 'static,
    Renderer: 'static,
{
    Area(Area),
    Band(Band),
    Bars(Bars),
    BoxPlot(BoxPlot),
    BubbleMap(BubbleMap),
    Choropleth(Choropleth),
    Line(Line),
    Pie(Pie<Message, Theme, Renderer>),
    Gauge(Gauge),
    Treemap(Treemap),
    Waterfall(Waterfall),
    Xy(Xy),
    Rule(Rule),
    Tick(Tick),
    Heatmap(Heatmap),
    Violin(Violin),
}

// Manual Debug / Clone — the derives would require `Theme: Debug + Clone`
// and `Renderer: Debug + Clone`, which the default `iced_widget::Renderer`
// does not satisfy.
impl<Message, Theme, Renderer> std::fmt::Debug for Mark<Message, Theme, Renderer>
where
    Message: 'static,
    Theme: 'static,
    Renderer: 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Mark::Area(v) => f.debug_tuple("Area").field(v).finish(),
            Mark::Band(v) => f.debug_tuple("Band").field(v).finish(),
            Mark::Bars(v) => f.debug_tuple("Bars").field(v).finish(),
            Mark::BoxPlot(v) => f.debug_tuple("BoxPlot").field(v).finish(),
            Mark::BubbleMap(v) => f.debug_tuple("BubbleMap").field(v).finish(),
            Mark::Choropleth(v) => f.debug_tuple("Choropleth").field(v).finish(),
            Mark::Line(v) => f.debug_tuple("Line").field(v).finish(),
            Mark::Pie(v) => f.debug_tuple("Pie").field(v).finish(),
            Mark::Gauge(v) => f.debug_tuple("Gauge").field(v).finish(),
            Mark::Treemap(v) => f.debug_tuple("Treemap").field(v).finish(),
            Mark::Waterfall(v) => f.debug_tuple("Waterfall").field(v).finish(),
            Mark::Xy(v) => f.debug_tuple("Xy").field(v).finish(),
            Mark::Rule(v) => f.debug_tuple("Rule").field(v).finish(),
            Mark::Tick(v) => f.debug_tuple("Tick").field(v).finish(),
            Mark::Heatmap(v) => f.debug_tuple("Heatmap").field(v).finish(),
            Mark::Violin(v) => f.debug_tuple("Violin").field(v).finish(),
        }
    }
}

impl<Message, Theme, Renderer> Clone for Mark<Message, Theme, Renderer>
where
    Message: 'static,
    Theme: 'static,
    Renderer: 'static,
{
    fn clone(&self) -> Self {
        match self {
            Mark::Area(v) => Mark::Area(v.clone()),
            Mark::Band(v) => Mark::Band(v.clone()),
            Mark::Bars(v) => Mark::Bars(v.clone()),
            Mark::BoxPlot(v) => Mark::BoxPlot(v.clone()),
            Mark::BubbleMap(v) => Mark::BubbleMap(v.clone()),
            Mark::Choropleth(v) => Mark::Choropleth(v.clone()),
            Mark::Line(v) => Mark::Line(v.clone()),
            Mark::Pie(v) => Mark::Pie(v.clone()),
            Mark::Gauge(v) => Mark::Gauge(v.clone()),
            Mark::Treemap(v) => Mark::Treemap(v.clone()),
            Mark::Waterfall(v) => Mark::Waterfall(v.clone()),
            Mark::Xy(v) => Mark::Xy(v.clone()),
            Mark::Rule(v) => Mark::Rule(v.clone()),
            Mark::Tick(v) => Mark::Tick(v.clone()),
            Mark::Heatmap(v) => Mark::Heatmap(v.clone()),
            Mark::Violin(v) => Mark::Violin(v.clone()),
        }
    }
}

impl<Message, Theme, Renderer> Mark<Message, Theme, Renderer>
where
    Message: 'static,
    Theme: 'static,
    Renderer: 'static,
{
    /// Extract legend entries from this mark.
    ///
    /// Returns entries with names and optional colors for the legend.
    /// Only marks that have names set will produce entries.
    pub fn legend_entries(&self) -> Vec<LegendEntry> {
        match self {
            Mark::Area(area) => area
                .series
                .iter()
                .filter_map(|s| {
                    s.name.as_ref().map(|name| LegendEntry {
                        name: name.clone(),
                        color: s.color,
                        swatch: LegendSwatch::Line {
                            style: s.style.clone(),
                            marker: s.marker.clone(),
                        },
                    })
                })
                .collect(),
            Mark::Bars(bars) => bars
                .series
                .iter()
                .filter_map(|s| {
                    s.name.as_ref().map(|name| LegendEntry {
                        name: name.clone(),
                        color: s.color,
                        swatch: LegendSwatch::Square,
                    })
                })
                .collect(),
            Mark::Line(line) => line
                .name
                .as_ref()
                .map(|name| {
                    vec![LegendEntry {
                        name: name.clone(),
                        color: line.color,
                        swatch: LegendSwatch::Line {
                            style: line.style.clone(),
                            marker: line.marker.clone(),
                        },
                    }]
                })
                .unwrap_or_default(),
            Mark::Pie(pie) => pie
                .slices
                .iter()
                .filter_map(|s| {
                    s.name.as_ref().map(|name| LegendEntry {
                        name: name.clone(),
                        color: s.color,
                        swatch: LegendSwatch::Square,
                    })
                })
                .collect(),
            Mark::Xy(xy) => xy
                .name
                .as_ref()
                .map(|name| {
                    vec![LegendEntry {
                        name: name.clone(),
                        color: xy.color,
                        swatch: LegendSwatch::Square,
                    }]
                })
                .unwrap_or_default(),
            Mark::BoxPlot(bp) => bp
                .entries
                .iter()
                .filter_map(|e| {
                    e.name.as_ref().map(|name| LegendEntry {
                        name: name.clone(),
                        color: e.color,
                        swatch: LegendSwatch::Square,
                    })
                })
                .collect(),
            Mark::Violin(v) => v
                .entries
                .iter()
                .filter_map(|e| {
                    e.name.as_ref().map(|name| LegendEntry {
                        name: name.clone(),
                        color: e.color,
                        swatch: LegendSwatch::Square,
                    })
                })
                .collect(),
            Mark::Treemap(tm) => tm
                .items
                .iter()
                .map(|item| LegendEntry {
                    name: item.label.clone(),
                    color: item.color,
                    swatch: LegendSwatch::Square,
                })
                .collect(),
            Mark::BubbleMap(bm) => bm
                .points
                .iter()
                .filter_map(|p| {
                    p.name.as_ref().map(|name| LegendEntry {
                        name: name.clone(),
                        color: p.color,
                        swatch: LegendSwatch::Square,
                    })
                })
                .collect(),
            // Rule, Band, Tick, Gauge, Heatmap, Choropleth don't contribute to legend
            Mark::Rule(_)
            | Mark::Band(_)
            | Mark::Tick(_)
            | Mark::Gauge(_)
            | Mark::Waterfall(_)
            | Mark::Heatmap(_)
            | Mark::Choropleth(_) => Vec::new(),
        }
    }

    /// Returns this mark's color-scale legend config plus title, when it
    /// owns one. Currently only [`Choropleth`] carries a scale legend; other
    /// marks return `None`. The scene uses this to decide whether to
    /// reserve edge space for an inset scale legend.
    pub fn scale_legend_config(&self) -> Option<(&crate::data::legend::Legend, Option<&str>)> {
        match self {
            Mark::Choropleth(c) => c.legend_config().map(|l| (l, c.legend_title_value())),
            _ => None,
        }
    }
}

impl<Message, Theme, Renderer> From<Area> for Mark<Message, Theme, Renderer>
where
    Message: 'static,
    Theme: 'static,
    Renderer: 'static,
{
    fn from(area: Area) -> Self {
        Mark::Area(area)
    }
}

impl<Message, Theme, Renderer> From<Bars> for Mark<Message, Theme, Renderer>
where
    Message: 'static,
    Theme: 'static,
    Renderer: 'static,
{
    fn from(bars: Bars) -> Self {
        Mark::Bars(bars)
    }
}

impl<Message, Theme, Renderer> From<BoxPlot> for Mark<Message, Theme, Renderer>
where
    Message: 'static,
    Theme: 'static,
    Renderer: 'static,
{
    fn from(bp: BoxPlot) -> Self {
        Mark::BoxPlot(bp)
    }
}

impl<Message, Theme, Renderer> From<Line> for Mark<Message, Theme, Renderer>
where
    Message: 'static,
    Theme: 'static,
    Renderer: 'static,
{
    fn from(line: Line) -> Self {
        Mark::Line(line)
    }
}

impl<Message, Theme, Renderer> From<Pie<Message, Theme, Renderer>> for Mark<Message, Theme, Renderer>
where
    Message: 'static,
    Theme: 'static,
    Renderer: 'static,
{
    fn from(pie: Pie<Message, Theme, Renderer>) -> Self {
        Mark::Pie(pie)
    }
}

impl<Message, Theme, Renderer> From<Gauge> for Mark<Message, Theme, Renderer>
where
    Message: 'static,
    Theme: 'static,
    Renderer: 'static,
{
    fn from(gauge: Gauge) -> Self {
        Mark::Gauge(gauge)
    }
}

impl<Message, Theme, Renderer> From<Waterfall> for Mark<Message, Theme, Renderer>
where
    Message: 'static,
    Theme: 'static,
    Renderer: 'static,
{
    fn from(waterfall: Waterfall) -> Self {
        Mark::Waterfall(waterfall)
    }
}

impl<Message, Theme, Renderer> From<Xy> for Mark<Message, Theme, Renderer>
where
    Message: 'static,
    Theme: 'static,
    Renderer: 'static,
{
    fn from(xy: Xy) -> Self {
        Mark::Xy(xy)
    }
}

impl<Message, Theme, Renderer> From<Rule> for Mark<Message, Theme, Renderer>
where
    Message: 'static,
    Theme: 'static,
    Renderer: 'static,
{
    fn from(rule: Rule) -> Self {
        Mark::Rule(rule)
    }
}

impl<Message, Theme, Renderer> From<Band> for Mark<Message, Theme, Renderer>
where
    Message: 'static,
    Theme: 'static,
    Renderer: 'static,
{
    fn from(band: Band) -> Self {
        Mark::Band(band)
    }
}

impl<Message, Theme, Renderer> From<Tick> for Mark<Message, Theme, Renderer>
where
    Message: 'static,
    Theme: 'static,
    Renderer: 'static,
{
    fn from(tick: Tick) -> Self {
        Mark::Tick(tick)
    }
}

impl<Message, Theme, Renderer> From<Heatmap> for Mark<Message, Theme, Renderer>
where
    Message: 'static,
    Theme: 'static,
    Renderer: 'static,
{
    fn from(heatmap: Heatmap) -> Self {
        Mark::Heatmap(heatmap)
    }
}

impl<Message, Theme, Renderer> From<Treemap> for Mark<Message, Theme, Renderer>
where
    Message: 'static,
    Theme: 'static,
    Renderer: 'static,
{
    fn from(treemap: Treemap) -> Self {
        Mark::Treemap(treemap)
    }
}

impl<Message, Theme, Renderer> From<Violin> for Mark<Message, Theme, Renderer>
where
    Message: 'static,
    Theme: 'static,
    Renderer: 'static,
{
    fn from(violin: Violin) -> Self {
        Mark::Violin(violin)
    }
}

impl<Message, Theme, Renderer> From<BubbleMap> for Mark<Message, Theme, Renderer>
where
    Message: 'static,
    Theme: 'static,
    Renderer: 'static,
{
    fn from(bm: BubbleMap) -> Self {
        Mark::BubbleMap(bm)
    }
}

impl<Message, Theme, Renderer> From<Choropleth> for Mark<Message, Theme, Renderer>
where
    Message: 'static,
    Theme: 'static,
    Renderer: 'static,
{
    fn from(c: Choropleth) -> Self {
        Mark::Choropleth(c)
    }
}
