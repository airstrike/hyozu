//! Property descriptors for XY scatter charts.

pub mod marker;

use crate::color::Color;
use crate::data::mark::xy;
use crate::map::Map;

/// A property of an XY scatter chart.
#[derive(Debug, Clone, PartialEq)]
pub enum Property {
    /// Series color (used as the marker fill when no marker color is set).
    Color(Option<Color>),
    /// A property of the marker configuration.
    Marker(marker::Property),
}

impl Map for Property {}

impl Property {
    /// Applies this property to an Xy mark.
    pub fn apply(&self, xy: &mut xy::Xy) {
        match self {
            Property::Color(c) => xy.set_color(*c),
            Property::Marker(p) => p.apply(xy.marker_mut()),
        }
    }
}

// `Marker` is exposed as an explicit constructor below because it shadows the
// re-exported marker submodule name; `Color` is exposed as a function for
// the same reason it is in other props modules (collision with the imported
// `Color` type).

/// Wraps a color value into an XY property.
#[allow(non_snake_case)]
pub fn Color(value: Option<Color>) -> Property {
    Property::Color(value)
}

/// Wraps a marker sub-property into an XY chart-wide property.
#[allow(non_snake_case)]
pub fn Marker(value: marker::Property) -> Property {
    Property::Marker(value)
}
