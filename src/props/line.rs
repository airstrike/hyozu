//! Property descriptors for line charts.

use crate::color::Color;
use crate::data::mark::line::label;
use crate::data::mark::Line;
use crate::map::Map;

/// A property of a line chart.
#[derive(Debug, Clone, PartialEq)]
pub enum Property {
    /// Line color
    Color(Option<Color>),
    /// Line width
    Width(f32),
    /// Which points show data labels
    Show(label::Show),
}

impl Map for Property {}

impl Property {
    /// Applies this property to a Line mark.
    pub fn apply(&self, line: &mut Line) {
        match self {
            Property::Color(c) => line.color = *c,
            Property::Width(w) => line.width = *w,
            Property::Show(show) => {
                if let Some(lbl) = &mut line.label {
                    lbl.show = *show;
                }
            }
        }
    }
}

/// Wraps a color value into a line property.
#[allow(non_snake_case)]
pub fn Color(value: Option<Color>) -> Property {
    Property::Color(value)
}

/// Wraps a width value into a line property.
#[allow(non_snake_case)]
pub fn Width(value: f32) -> Property {
    Property::Width(value)
}

/// Wraps a label show value into a line property.
#[allow(non_snake_case)]
pub fn Show(value: label::Show) -> Property {
    Property::Show(value)
}
