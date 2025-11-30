//! Property descriptors for bar series.

use crate::color::Color;
use crate::data::mark::bar::label::Position;
use crate::data::mark::bar::{Label, Series};
use crate::map::Map;

/// A property of a bar series.
#[derive(Debug, Clone, PartialEq)]
pub enum Property {
    /// Series color
    Color(Option<Color>),
    /// Data label configuration
    Label(Option<Label>),
    /// Just the label position (keeps existing label config)
    LabelPosition(Position),
}

impl Map for Property {}

impl Property {
    /// Applies this property to a series.
    pub fn apply(&self, series: &mut Series) {
        match self {
            Property::Color(c) => series.color = *c,
            Property::Label(l) => series.label = l.clone(),
            Property::LabelPosition(p) => series.set_label_position(*p),
        }
    }
}

/// Wraps a color value into a series property.
#[allow(non_snake_case)]
pub fn Color(value: Option<Color>) -> Property {
    Property::Color(value)
}

/// Wraps a label value into a series property.
#[allow(non_snake_case)]
pub fn Label(value: Option<Label>) -> Property {
    Property::Label(value)
}

/// Wraps a label position into a series property.
#[allow(non_snake_case)]
pub fn LabelPosition(value: Position) -> Property {
    Property::LabelPosition(value)
}
