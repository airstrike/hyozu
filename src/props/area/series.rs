//! Property descriptors for area series.

use crate::color::Color;
use crate::core::Pixels;
use crate::data::mark::area::Series;
use crate::data::mark::line::label::{self, Label};
use crate::data::mark::line::{LineStyle, marker};
use crate::map::Map;

/// A property of an area series.
#[derive(Debug, Clone, PartialEq)]
pub enum Property {
    /// Series color.
    Color(Option<Color>),
    /// Fill opacity (clamped to `[0.0, 1.0]`).
    Opacity(f32),
    /// Stroke width. `None` removes the upper-line stroke.
    Stroke(Option<f32>),
    /// Dash pattern for the upper-line stroke.
    Style(LineStyle),
    /// Marker configuration drawn at each data point on the upper envelope.
    Marker(Option<marker::Marker>),
    /// Replaces the entire data label configuration.
    Label(Option<Label>),
    /// Just the label position (keeps existing label config).
    LabelPosition(label::Position),
    /// Just the label show mode.
    LabelShow(label::Show),
    /// Series-level label color.
    LabelColor(Option<Color>),
    /// Series-level label size.
    LabelSize(Option<Pixels>),
    /// Series-level label font weight.
    LabelWeight(Option<crate::core::font::Weight>),
    /// Series-level label font style.
    LabelStyle(Option<crate::core::font::Style>),
    /// Series-level label background fill.
    LabelFill(Option<Color>),
}

impl Map for Property {}

impl Property {
    /// Applies this property to a series.
    pub fn apply(&self, series: &mut Series) {
        match self {
            Property::Color(c) => series.set_color(*c),
            Property::Opacity(o) => series.set_opacity(*o),
            Property::Stroke(s) => series.set_stroke(*s),
            Property::Style(s) => series.set_style(s.clone()),
            Property::Marker(m) => series.set_marker(m.clone()),
            Property::Label(l) => series.set_label(l.clone()),
            Property::LabelPosition(p) => series.set_label_position(*p),
            Property::LabelShow(s) => series.set_label_show(*s),
            Property::LabelColor(c) => series.set_label_color(*c),
            Property::LabelSize(s) => series.set_label_size(*s),
            Property::LabelWeight(w) => series.set_label_weight(*w),
            Property::LabelStyle(s) => series.set_label_style(*s),
            Property::LabelFill(f) => series.set_label_fill(*f),
        }
    }
}

#[allow(non_snake_case)]
pub fn Color(value: Option<Color>) -> Property {
    Property::Color(value)
}

#[allow(non_snake_case)]
pub fn Opacity(value: f32) -> Property {
    Property::Opacity(value)
}

#[allow(non_snake_case)]
pub fn Stroke(value: Option<f32>) -> Property {
    Property::Stroke(value)
}

#[allow(non_snake_case)]
pub fn Style(value: LineStyle) -> Property {
    Property::Style(value)
}

#[allow(non_snake_case)]
pub fn Marker(value: Option<marker::Marker>) -> Property {
    Property::Marker(value)
}

#[allow(non_snake_case)]
pub fn Label(value: Option<Label>) -> Property {
    Property::Label(value)
}

#[allow(non_snake_case)]
pub fn LabelPosition(value: label::Position) -> Property {
    Property::LabelPosition(value)
}

#[allow(non_snake_case)]
pub fn LabelShow(value: label::Show) -> Property {
    Property::LabelShow(value)
}

#[allow(non_snake_case)]
pub fn LabelColor(value: Option<Color>) -> Property {
    Property::LabelColor(value)
}

#[allow(non_snake_case)]
pub fn LabelSize(value: Option<Pixels>) -> Property {
    Property::LabelSize(value)
}

#[allow(non_snake_case)]
pub fn LabelWeight(value: Option<crate::core::font::Weight>) -> Property {
    Property::LabelWeight(value)
}

#[allow(non_snake_case)]
pub fn LabelStyle(value: Option<crate::core::font::Style>) -> Property {
    Property::LabelStyle(value)
}

#[allow(non_snake_case)]
pub fn LabelFill(value: Option<Color>) -> Property {
    Property::LabelFill(value)
}
