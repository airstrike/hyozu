//! Property descriptors for pie/donut charts.

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
        }
    }
}

pub use Property::{Gap, Hole};

/// Wraps a per-slice color override into a pie property.
#[allow(non_snake_case)]
pub fn SliceColor(index: usize, color: Option<Color>) -> Property {
    Property::SliceColor { index, color }
}
