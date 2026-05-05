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
    Choropleth, ChoroplethEntry, IntoChoropleth, choropleth, choropleth_entry, choropleth_entry_available,
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
    /// Optional raw numeric value shown in a second column to the right
    /// of the name. The legend renderer formats it via the precedence
    /// chain (legend override > mark override > data scale > default).
    /// When any entry has a value, the legend lays out as a two-column
    /// table; otherwise it stays in single-column mode.
    pub value: Option<f64>,
}

/// Represents a visual mark in a chart (bars, lines, scatter, etc.)
#[derive(Debug, Clone)]
pub enum Mark {
    Area(Area),
    Band(Band),
    Bars(Bars),
    BoxPlot(BoxPlot),
    BubbleMap(BubbleMap),
    Choropleth(Choropleth),
    Line(Line),
    Pie(Pie),
    Gauge(Gauge),
    Treemap(Treemap),
    Waterfall(Waterfall),
    Xy(Xy),
    Rule(Rule),
    Tick(Tick),
    Heatmap(Heatmap),
    Violin(Violin),
}

impl Mark {
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
                        value: None,
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
                        value: None,
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
                        value: None,
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
                        value: Some(s.value()),
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
                        value: None,
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
                        value: None,
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
                        value: None,
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
                    value: None,
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
                        value: None,
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
    pub fn scale_legend_config(&self) -> Option<(&crate::data::legend::Config, Option<&str>)> {
        match self {
            Mark::Choropleth(c) => c.legend_config().map(|l| (l, c.legend_title_value())),
            _ => None,
        }
    }
}

impl From<Area> for Mark {
    fn from(area: Area) -> Self {
        Mark::Area(area)
    }
}

impl From<Bars> for Mark {
    fn from(bars: Bars) -> Self {
        Mark::Bars(bars)
    }
}

impl From<BoxPlot> for Mark {
    fn from(bp: BoxPlot) -> Self {
        Mark::BoxPlot(bp)
    }
}

impl From<Line> for Mark {
    fn from(line: Line) -> Self {
        Mark::Line(line)
    }
}

impl From<Pie> for Mark {
    fn from(pie: Pie) -> Self {
        Mark::Pie(pie)
    }
}

impl From<Gauge> for Mark {
    fn from(gauge: Gauge) -> Self {
        Mark::Gauge(gauge)
    }
}

impl From<Waterfall> for Mark {
    fn from(waterfall: Waterfall) -> Self {
        Mark::Waterfall(waterfall)
    }
}

impl From<Xy> for Mark {
    fn from(xy: Xy) -> Self {
        Mark::Xy(xy)
    }
}

impl From<Rule> for Mark {
    fn from(rule: Rule) -> Self {
        Mark::Rule(rule)
    }
}

impl From<Band> for Mark {
    fn from(band: Band) -> Self {
        Mark::Band(band)
    }
}

impl From<Tick> for Mark {
    fn from(tick: Tick) -> Self {
        Mark::Tick(tick)
    }
}

impl From<Heatmap> for Mark {
    fn from(heatmap: Heatmap) -> Self {
        Mark::Heatmap(heatmap)
    }
}

impl From<Treemap> for Mark {
    fn from(treemap: Treemap) -> Self {
        Mark::Treemap(treemap)
    }
}

impl From<Violin> for Mark {
    fn from(violin: Violin) -> Self {
        Mark::Violin(violin)
    }
}

impl From<BubbleMap> for Mark {
    fn from(bm: BubbleMap) -> Self {
        Mark::BubbleMap(bm)
    }
}

impl From<Choropleth> for Mark {
    fn from(c: Choropleth) -> Self {
        Mark::Choropleth(c)
    }
}
