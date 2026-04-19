//! Property descriptors for waterfall labels (applied chart-wide).

use crate::data::mark::waterfall::{self, label};
use crate::map::Map;

/// A property of a waterfall label.
#[derive(Debug, Clone, PartialEq)]
pub enum Property {
    /// Label position (Above/End/Center/Base).
    Position(label::Position),
    /// Which entries show labels.
    Show(label::Show),
    /// Label color.
    Color(Option<crate::color::Color>),
    /// Label font size.
    Size(Option<crate::core::Pixels>),
    /// Label font weight.
    Weight(Option<crate::core::font::Weight>),
    /// Label font style.
    Style(Option<crate::core::font::Style>),
    /// Background fill color drawn behind the label text.
    Fill(Option<crate::color::Color>),
}

impl Map for Property {}

impl Property {
    /// Applies this property to a Waterfall mark.
    pub fn apply(&self, wf: &mut waterfall::Waterfall) {
        match self {
            Property::Position(p) => wf.set_label_position(*p),
            Property::Show(s) => wf.set_label_show(*s),
            Property::Color(c) => wf.set_label_color(*c),
            Property::Size(s) => wf.set_label_size(*s),
            Property::Weight(w) => wf.set_label_weight(*w),
            Property::Style(s) => wf.set_label_style(*s),
            Property::Fill(f) => wf.set_label_fill(*f),
        }
    }
}

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
