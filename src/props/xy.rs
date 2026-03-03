//! Property descriptors for XY scatter charts.

use crate::color::Color;
use crate::map::Map;

/// A property of an XY scatter chart.
#[derive(Debug, Clone, PartialEq)]
pub enum Property {
    /// Point color
    Color(Option<Color>),
}

impl Map for Property {}

impl Property {
    /// Applies this property to an Xy mark.
    pub fn apply(&self, xy: &mut crate::data::mark::xy::Xy) {
        match self {
            Property::Color(c) => xy.color = *c,
        }
    }
}

/// Wraps a color value into an XY property.
#[allow(non_snake_case)]
pub fn Color(value: Option<Color>) -> Property {
    Property::Color(value)
}
