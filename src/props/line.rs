//! Property descriptors for line charts.

use crate::color::Color;
use crate::core::Pixels;
use crate::data::mark::Line;
use crate::data::mark::line::label;
use crate::map::Map;

/// A property of a line chart.
#[derive(Debug, Clone, PartialEq)]
pub enum Property {
    /// Line color
    Color(Option<Color>),
    /// Line width
    Width(f32),
    /// Replaces the entire data label configuration.
    Label(Option<label::Label>),
    /// Just the label position (keeps existing label config).
    LabelPosition(label::Position),
    /// Just the label show mode (keeps existing label config).
    LabelShow(label::Show),
    /// Label color
    LabelColor(Option<Color>),
    /// Label size
    LabelSize(Option<Pixels>),
    /// Label font weight (bold, etc.)
    LabelWeight(Option<crate::core::font::Weight>),
    /// Label font style (italic, etc.)
    LabelStyle(Option<crate::core::font::Style>),
    /// Label background fill color
    LabelFill(Option<Color>),
}

impl Map for Property {}

impl Property {
    /// Applies this property to a Line mark.
    pub fn apply(&self, line: &mut Line) {
        match self {
            Property::Color(c) => line.color = *c,
            Property::Width(w) => line.width = *w,
            Property::Label(l) => line.label = l.clone(),
            Property::LabelPosition(p) => set_label_with(line, |lbl| lbl.set_position(*p)),
            Property::LabelShow(s) => set_label_with(line, |lbl| lbl.set_show(*s)),
            Property::LabelColor(c) => set_label_with(line, |lbl| lbl.set_color(*c)),
            Property::LabelSize(s) => set_label_with(line, |lbl| lbl.set_size(*s)),
            Property::LabelWeight(w) => set_label_with(line, |lbl| lbl.set_weight(*w)),
            Property::LabelStyle(s) => set_label_with(line, |lbl| lbl.set_style(*s)),
            Property::LabelFill(f) => set_label_with(line, |lbl| lbl.set_fill(*f)),
        }
    }
}

/// Lazily constructs a default label if none exists, then applies `f`.
/// Mirrors the lazy-create pattern used by area::Series and bar::Series.
fn set_label_with(line: &mut Line, f: impl FnOnce(&mut label::Label)) {
    let lbl = line.label.get_or_insert_with(label::Label::default);
    f(lbl);
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

#[allow(non_snake_case)]
pub fn Label(value: Option<label::Label>) -> Property {
    Property::Label(value)
}

#[allow(non_snake_case)]
pub fn LabelPosition(value: label::Position) -> Property {
    Property::LabelPosition(value)
}

/// Wraps a label show value into a line property.
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
