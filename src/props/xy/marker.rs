//! Property descriptors for XY scatter markers.

use crate::data::mark::line::marker as line_marker;
use crate::map::Map;

/// A property of an XY scatter marker.
#[derive(Debug, Clone, PartialEq)]
pub enum Property {
    /// Marker shape (Circle/Square/Diamond/...).
    Shape(line_marker::Shape),
    /// Which points show markers.
    Show(line_marker::Show),
    /// Marker size in pixels.
    Size(f32),
    /// Marker fill color. `None` falls back to the series color.
    Color(Option<crate::color::Color>),
    /// Marker stroke (outline) color. `None` removes the outline.
    Stroke(Option<crate::color::Color>),
    /// Marker stroke width in pixels.
    StrokeWidth(f32),
}

impl Map for Property {}

impl Property {
    /// Applies this property to a marker configuration.
    pub fn apply(&self, marker: &mut line_marker::Marker) {
        match self {
            Property::Shape(s) => marker.set_shape(*s),
            Property::Show(s) => marker.set_show(*s),
            Property::Size(s) => marker.set_size(*s),
            Property::Color(c) => marker.set_color(*c),
            Property::Stroke(c) => marker.set_stroke(*c),
            Property::StrokeWidth(w) => marker.set_stroke_width(*w),
        }
    }
}

// Re-export enum variants as constructor functions where naming permits.
// `Color` collides with the `crate::color::Color` type, so it's exposed via
// the explicit constructor function below.
pub use Property::{Shape, Show, Size, Stroke, StrokeWidth};

#[allow(non_snake_case)]
pub fn Color(value: Option<crate::color::Color>) -> Property {
    Property::Color(value)
}
