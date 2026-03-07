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
    /// Per-point color override
    PointColor { index: usize, color: Option<Color> },
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
            Property::Color(c) => {
                series.color = *c;
                series.point_colors.clear();
            }
            Property::PointColor { index, color } => {
                if let Some(c) = color {
                    series.set_point_color(*index, *c);
                } else {
                    series.clear_point_color(*index);
                }
            }
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

/// Wraps a per-point color override into a series property.
#[allow(non_snake_case)]
pub fn PointColor(index: usize, color: Option<Color>) -> Property {
    Property::PointColor { index, color }
}
