pub mod alignment;
pub mod chart;
pub mod color;
pub mod data;
pub mod design;
pub mod geometry;
pub mod item;
pub mod map;
pub mod props;
pub mod theme;

// Make our imports look like iced's built-in widgets
use {iced_core as core, iced_widget as widget};

// Re-export iced::Function for `.with()` partial application
pub use core::Function;

// Re-export commonly used items for convenience
pub use self::chart::chart;
pub use color::{Color, Pair};
pub use data::{Action, Data};
pub use design::Design;
pub use data::Datum;

pub use data::data;

pub use data::axis::{self, Axis, Orientation};
pub use data::mark::{self, Bars, Line, Mark, bar, bars, line};

pub use map::Map;
