//! Property descriptors for pie/donut charts.

pub mod label;

use crate::color::Color;
use crate::map::Map;

/// A property of a pie chart.
#[derive(Debug, Clone, PartialEq)]
pub enum Property {
    /// Inner hole radius as proportion of outer radius (0.0 = pie, up to 0.99 = thin donut)
    Hole(f32),
    /// Gap between slices in pixels
    Gap(f32),
    /// Per-slice color override
    SliceColor { index: usize, color: Option<Color> },
    /// Label property applied to all slices
    Label(label::Property),
    /// Per-slice label property override
    SliceLabel { index: usize, property: label::Property },
}

impl Map for Property {}

impl Property {
    /// Applies this property to a Pie mark.
    pub fn apply(&self, pie: &mut crate::data::mark::pie::Pie) {
        match self {
            Property::Hole(v) => pie.hole = v.clamp(0.0, 0.99),
            Property::Gap(v) => pie.gap = v.max(0.0),
            Property::SliceColor { index, color } => {
                if let Some(slice) = pie.slices.get_mut(*index) {
                    slice.color = *color;
                }
            }
            Property::Label(p) => apply_chart_label(p, pie),
            Property::SliceLabel { index, property } => {
                if let Some(slice) = pie.slices_mut().get_mut(*index) {
                    apply_slice_label(property, slice);
                }
            }
        }
    }
}

fn apply_chart_label(p: &label::Property, pie: &mut crate::data::mark::pie::Pie) {
    match p {
        label::Property::Position(v) => pie.set_label_position(*v),
        label::Property::Show(v) => pie.set_label_show(*v),
        label::Property::Color(c) => pie.set_label_color(*c),
        label::Property::Size(s) => pie.set_label_size(*s),
        label::Property::Weight(w) => pie.set_label_weight(*w),
        label::Property::Style(s) => pie.set_label_style(*s),
        label::Property::Fill(f) => pie.set_label_fill(*f),
    }
}

fn apply_slice_label(p: &label::Property, slice: &mut crate::data::mark::pie::Slice) {
    match p {
        label::Property::Position(v) => slice.set_label_position(*v),
        label::Property::Show(v) => slice.set_label_show(*v),
        label::Property::Color(c) => slice.set_label_color(*c),
        label::Property::Size(s) => slice.set_label_size(*s),
        label::Property::Weight(w) => slice.set_label_weight(*w),
        label::Property::Style(s) => slice.set_label_style(*s),
        label::Property::Fill(f) => slice.set_label_fill(*f),
    }
}

pub use Property::{Gap, Hole};

/// Wraps a per-slice color override into a pie property.
#[allow(non_snake_case)]
pub fn SliceColor(index: usize, color: Option<Color>) -> Property {
    Property::SliceColor { index, color }
}

/// Wraps a per-slice label property into a pie property.
#[allow(non_snake_case)]
pub fn SliceLabel(index: usize, property: label::Property) -> Property {
    Property::SliceLabel { index, property }
}
