//! Property descriptors for bar labels.

use crate::color::Color;
use crate::core::Pixels;
use crate::data::mark::bar;
use crate::map::Map;

/// A property of a bar label.
#[derive(Debug, Clone, PartialEq)]
pub enum Property {
    /// Label position
    Position(bar::label::Position),
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

impl Property {
    /// Applies this property to all series in a Bars mark.
    pub fn apply(&self, bars: &mut bar::Bars) {
        for series in bars.series_mut() {
            match self {
                Property::Position(p) => series.set_label_position(*p),
                Property::Color(c) => series.set_label_color(*c),
                Property::Size(s) => series.set_label_size(*s),
                Property::Weight(w) => series.set_label_weight(*w),
                Property::Style(s) => series.set_label_style(*s),
                Property::Fill(f) => series.set_label_fill(*f),
            }
        }
    }
}

// Re-export enum variant as constructor function
pub use Property::Position;
