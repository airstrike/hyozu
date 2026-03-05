pub mod bar;
pub mod gauge;
pub mod line;
pub mod pie;
pub mod rule;
pub mod violin;
pub mod waterfall;
pub mod xy;

pub use bar::{Bars, IntoBars, bar, bars};
pub use gauge::{Gauge, gauge};
pub use line::{IntoLines, Line, line};
pub use pie::{Pie, pie};
pub use rule::{Rule, rule};
pub use violin::{Violin, violin, violin_entry, violin_from_data};
pub use waterfall::{Waterfall, waterfall};
pub use xy::{Xy, xy};

/// A single entry in the chart legend.
#[derive(Debug, Clone)]
pub struct LegendEntry {
    /// The display name for this entry.
    pub name: String,
    /// Optional color for the swatch. When `None`, the palette index is used.
    pub color: Option<crate::color::Color>,
}

/// Represents a visual mark in a chart (bars, lines, scatter, etc.)
#[derive(Debug, Clone)]
pub enum Mark {
    Bars(Bars),
    Line(Line),
    Pie(Pie),
    Gauge(Gauge),
    Waterfall(Waterfall),
    Xy(Xy),
    Rule(Rule),
    Violin(Violin),
}

impl Mark {
    /// Extract legend entries from this mark.
    ///
    /// Returns entries with names and optional colors for the legend.
    /// Only marks that have names set will produce entries.
    pub fn legend_entries(&self) -> Vec<LegendEntry> {
        match self {
            Mark::Bars(bars) => bars
                .series
                .iter()
                .filter_map(|s| {
                    s.name.as_ref().map(|name| LegendEntry {
                        name: name.clone(),
                        color: s.color,
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
                    }]
                })
                .unwrap_or_default(),
            Mark::Violin(v) => v
                .entries
                .iter()
                .filter_map(|e| {
                    e.name.as_ref().map(|name| LegendEntry {
                        name: name.clone(),
                        color: e.color,
                    })
                })
                .collect(),
            // Rule, Gauge don't contribute to legend
            Mark::Rule(_) | Mark::Gauge(_) | Mark::Waterfall(_) => Vec::new(),
        }
    }
}

impl From<Bars> for Mark {
    fn from(bars: Bars) -> Self {
        Mark::Bars(bars)
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

impl From<Violin> for Mark {
    fn from(violin: Violin) -> Self {
        Mark::Violin(violin)
    }
}
