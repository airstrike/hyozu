//! Property descriptors for bar series.

use crate::color::Color;
use crate::core::Pixels;
use crate::data::mark::bar::label::Position;
use crate::data::mark::bar::{Label, Series};
use crate::map::Map;

/// A per-point label property override.
#[derive(Debug, Clone, PartialEq)]
pub enum PointLabelProperty {
    /// Point label color
    Color(Option<Color>),
    /// Point label font size
    Size(Option<Pixels>),
    /// Point label font weight
    Weight(Option<crate::core::font::Weight>),
    /// Point label font style
    Style(Option<crate::core::font::Style>),
    /// Point label background fill color
    Fill(Option<Color>),
}

impl Map for PointLabelProperty {}

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
    /// Series-level label color
    LabelColor(Option<Color>),
    /// Series-level label size
    LabelSize(Option<Pixels>),
    /// Series-level label font weight
    LabelWeight(Option<crate::core::font::Weight>),
    /// Series-level label font style
    LabelStyle(Option<crate::core::font::Style>),
    /// Series-level label background fill
    LabelFill(Option<Color>),
    /// Per-point label property override
    PointLabel { index: usize, property: PointLabelProperty },
}

impl Map for Property {}

impl Property {
    /// Applies this property to a series.
    pub fn apply(&self, series: &mut Series) {
        match self {
            Property::Color(c) => {
                series.color = *c;
                series.point_colors.clear();
                series.color_by = None;
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
            Property::LabelColor(c) => series.set_label_color(*c),
            Property::LabelSize(s) => series.set_label_size(*s),
            Property::LabelWeight(w) => series.set_label_weight(*w),
            Property::LabelStyle(s) => series.set_label_style(*s),
            Property::LabelFill(f) => series.set_label_fill(*f),
            Property::PointLabel { index, property } => match property {
                PointLabelProperty::Color(c) => series.set_point_label_color(*index, *c),
                PointLabelProperty::Size(s) => series.set_point_label_size(*index, *s),
                PointLabelProperty::Weight(w) => series.set_point_label_weight(*index, *w),
                PointLabelProperty::Style(s) => series.set_point_label_style(*index, *s),
                PointLabelProperty::Fill(f) => series.set_point_label_fill(*index, *f),
            },
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

/// Wraps a per-point label property into a series property.
#[allow(non_snake_case)]
pub fn PointLabel(index: usize, property: PointLabelProperty) -> Property {
    Property::PointLabel { index, property }
}
