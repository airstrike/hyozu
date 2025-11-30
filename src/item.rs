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
    /// A line chart at the given index with a property change
    Line(usize, props::line::Property),
    /// The X axis
    XAxis(props::axis::Property),
    /// The Y axis
    YAxis(props::axis::Property),
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

/// Creates a line item for a given mark index and property.
#[allow(non_snake_case)]
pub fn Line(index: usize, property: props::line::Property) -> Item {
    Item::Line(index, property)
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
