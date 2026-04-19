//! Property descriptors for pie labels.

use crate::data::mark::pie::label;
use crate::map::Map;

/// A property of a pie label.
#[derive(Debug, Clone, PartialEq)]
pub enum Property {
    /// Label position (Inside/Outside/Edge).
    Position(label::Position),
    /// Which slices show labels.
    Show(label::Show),
    /// Label color
    Color(Option<crate::color::Color>),
    /// Label font size
    Size(Option<crate::core::Pixels>),
    /// Label font weight
    Weight(Option<crate::core::font::Weight>),
    /// Label font style
    Style(Option<crate::core::font::Style>),
    /// Label background fill color
    Fill(Option<crate::color::Color>),
}

impl Map for Property {}

pub use Property::{Position, Show, Style, Weight};

#[allow(non_snake_case)]
pub fn Color(value: Option<crate::color::Color>) -> Property {
    Property::Color(value)
}

#[allow(non_snake_case)]
pub fn Size(value: Option<crate::core::Pixels>) -> Property {
    Property::Size(value)
}

#[allow(non_snake_case)]
pub fn Fill(value: Option<crate::color::Color>) -> Property {
    Property::Fill(value)
}
