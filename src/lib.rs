pub mod alignment;
pub mod chart;
pub mod color;
pub mod data;
pub mod design;
pub mod geometry;
pub mod item;
pub mod map;
pub mod palette;
pub mod props;
pub mod target;
pub mod theme;

// Make our imports look like iced's built-in widgets
use {iced_core as core, iced_widget as widget};

// Re-export iced::Function for `.with()` partial application
pub use core::Function;

// Re-export commonly used items for convenience
pub use self::chart::chart;
pub use color::{Color, Pair};
pub use data::{Action, Data, Datum};
pub use design::Design;
pub use palette::{Palette, Resolved};
pub use target::Target;

pub use data::data;

pub use data::axis::{self, Axis, Orientation};
pub use data::mark::{
    self, Bars, BoxPlot, Gauge, LegendEntry, Line, Mark, Pie, Rule, Waterfall,
    Xy, bar, bars, boxplot, entry, entry_from_data, gauge, line, pie, rule,
    waterfall, xy,
};

pub use map::Map;
