//! Chart item descriptors for addressing elements in a chart.
//!
//! Items represent top-level elements in a chart that can contain properties.
//! Use with `.map()` to wrap property changes into item changes.
//!
//! # Example
//!
//! ```ignore
//! use iced::widget::slider;
//! use hyozu::{props, item, Function};
//!
//! slider(0.0..=1.0, size, props::bar::Size)
//!     .map(item::Bars.with(0))
//!     .map(Message::Set)
//! ```

use crate::map::Map;
use crate::props;

/// A chart item that can be inspected or modified.
#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    /// The chart title
    Title(String),
    /// A bar chart at the given index with a property change
    Bars(usize, props::bar::Property),
    /// A box plot chart at the given index with a property change
    BoxPlot(usize, props::boxplot::Property),
    /// A line chart at the given index with a property change
    Line(usize, props::line::Property),
    /// A pie chart at the given index with a property change
    Pie(usize, props::pie::Property),
    /// A gauge chart at the given index with a property change
    Gauge(usize, props::gauge::Property),
    /// A waterfall chart at the given index with a property change
    Waterfall(usize, props::waterfall::Property),
    /// An XY scatter chart at the given index with a property change
    Xy(usize, props::xy::Property),
    /// A rule mark at the given index with a property change
    Rule(usize, props::rule::Property),
    /// The X axis
    XAxis(props::axis::Property),
    /// The Y axis
    YAxis(props::axis::Property),
    /// The palette strategy
    Palette(crate::palette::Palette),
    /// The selection state
    Selection(Option<crate::target::Target>),
}

impl Map for Item {}

/// Wraps a title string into an item.
#[allow(non_snake_case)]
pub fn Title(value: String) -> Item {
    Item::Title(value)
}

/// Creates a bars item for a given mark index and property.
///
/// # Example
/// ```ignore
/// slider(0.0..=1.0, size, props::bar::Size)
///     .map(|p| item::Bars(0, p))
/// ```
#[allow(non_snake_case)]
pub fn Bars(index: usize, property: props::bar::Property) -> Item {
    Item::Bars(index, property)
}

/// Creates a box plot item for a given mark index and property.
#[allow(non_snake_case)]
pub fn BoxPlot(index: usize, property: props::boxplot::Property) -> Item {
    Item::BoxPlot(index, property)
}

/// Creates a line item for a given mark index and property.
#[allow(non_snake_case)]
pub fn Line(index: usize, property: props::line::Property) -> Item {
    Item::Line(index, property)
}

/// Creates a pie item for a given mark index and property.
#[allow(non_snake_case)]
pub fn Pie(index: usize, property: props::pie::Property) -> Item {
    Item::Pie(index, property)
}

/// Creates a gauge item for a given mark index and property.
#[allow(non_snake_case)]
pub fn Gauge(index: usize, property: props::gauge::Property) -> Item {
    Item::Gauge(index, property)
}

/// Creates a waterfall item for a given mark index and property.
#[allow(non_snake_case)]
pub fn Waterfall(index: usize, property: props::waterfall::Property) -> Item {
    Item::Waterfall(index, property)
}

/// Creates an XY item for a given mark index and property.
#[allow(non_snake_case)]
pub fn Xy(index: usize, property: props::xy::Property) -> Item {
    Item::Xy(index, property)
}

/// Creates a rule item for a given mark index and property.
#[allow(non_snake_case)]
pub fn Rule(index: usize, property: props::rule::Property) -> Item {
    Item::Rule(index, property)
}

/// Wraps an x-axis property into an item.
#[allow(non_snake_case)]
pub fn XAxis(property: props::axis::Property) -> Item {
    Item::XAxis(property)
}

/// Wraps a y-axis property into an item.
#[allow(non_snake_case)]
pub fn YAxis(property: props::axis::Property) -> Item {
    Item::YAxis(property)
}

/// Wraps a palette strategy into an item.
#[allow(non_snake_case)]
pub fn Palette(palette: crate::palette::Palette) -> Item {
    Item::Palette(palette)
}

/// Wraps a selection target into an item.
#[allow(non_snake_case)]
pub fn Selection(target: Option<crate::target::Target>) -> Item {
    Item::Selection(target)
}
