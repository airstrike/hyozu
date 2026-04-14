//! Property descriptors for pie labels.

use crate::color::Color;
use crate::core::Pixels;
use crate::map::Map;

/// A property of a pie label.
#[derive(Debug, Clone, PartialEq)]
pub enum Property {
    /// Label color
    Color(Option<Color>),
    /// Label font size
    Size(Option<Pixels>),
    /// Label font weight
    Weight(Option<crate::core::font::Weight>),
    /// Label font style
    Style(Option<crate::core::font::Style>),
    /// Label background fill color
    Fill(Option<Color>),
}

impl Map for Property {}
