pub mod alignment;
pub mod animation;
pub mod chart;
pub mod color;
pub mod data;
pub mod design;
pub mod encoding;
pub mod feature;
pub mod geo;
pub mod geometry;
pub mod item;
pub mod map;
pub mod palette;
pub mod props;
pub mod scale;
pub mod sparkline;
pub mod target;
#[cfg(feature = "tatami")]
pub mod tatami;
pub mod text;
pub mod theme;

// Make our imports look like iced's built-in widgets
use iced_core as core;
use iced_widget as widget;

// Re-export iced::Function for `.with()` partial application
pub use core::Function;

// Re-export commonly used items for convenience
pub use self::chart::{chart, donut};
pub use color::{Color, Pair};
pub use data::{Action, Data, Datum};
pub use design::Design;
pub use palette::{Palette, Resolved};
pub use scale::{Scale, Transform, XScale};
pub use target::Target;

pub use data::data;
pub use geo::{GeoData, MapScope, ProjectionKind};

pub use data::axis::{self, Axis, Orientation};
pub use data::legend;
pub use data::mark::{
    self, Area, Band, BandOrientation, Bars, BoxPlot, Choropleth, ChoroplethEntry, Gauge, Heatmap, LegendEntry, Line,
    LineStyle, MapPoint, Mark, Pie, Rule, Tick, Treemap, Violin, Waterfall, Xy, area, areas, band, bar, bars, boxplot,
    bubble_map, choropleth, choropleth_entry, choropleth_entry_available, entry, entry_from_data, gauge, heatmap, line,
    map_point, pie, rule, tick, treemap, violin, violin_entry, violin_from_data, waterfall, xy,
};
pub use data::tooltip::{ColoredText, Swatch, Tooltip, TooltipEntry};

pub use map::Map;
pub use sparkline::sparkline;
