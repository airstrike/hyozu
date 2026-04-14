//! Property descriptors for area labels (applied chart-wide to every series).

use crate::data::mark::area;
use crate::data::mark::line::label;
use crate::map::Map;

/// A property of an area label.
#[derive(Debug, Clone, PartialEq)]
pub enum Property {
    /// Label position (Above/Below/Left/Right/Auto).
    Position(label::Position),
    /// Which points show labels.
    Show(label::Show),
    /// Label color.
    Color(Option<crate::color::Color>),
    /// Label font size.
    Size(Option<crate::core::Pixels>),
    /// Label font weight (e.g. bold).
    Weight(Option<crate::core::font::Weight>),
    /// Label font style (e.g. italic).
    Style(Option<crate::core::font::Style>),
    /// Background fill color drawn behind the label text.
    Fill(Option<crate::color::Color>),
}

impl Map for Property {}

impl Property {
    /// Applies this property to every series in an Area mark.
    pub fn apply(&self, area: &mut area::Area) {
        for series in area.series_mut() {
            match self {
                Property::Position(p) => series.set_label_position(*p),
                Property::Show(s) => series.set_label_show(*s),
                Property::Color(c) => series.set_label_color(*c),
                Property::Size(s) => series.set_label_size(*s),
                Property::Weight(w) => series.set_label_weight(*w),
                Property::Style(s) => series.set_label_style(*s),
                Property::Fill(f) => series.set_label_fill(*f),
            }
        }
    }
}

// Re-export enum variants that don't collide with imported types.
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
