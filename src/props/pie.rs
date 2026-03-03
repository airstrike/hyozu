//! Property descriptors for pie/donut charts.

use crate::map::Map;

/// A property of a pie chart.
#[derive(Debug, Clone, PartialEq)]
pub enum Property {
    /// Inner hole radius as proportion of outer radius (0.0 = pie, up to 0.99 = thin donut)
    Hole(f32),
    /// Gap between slices in pixels
    Gap(f32),
}

impl Map for Property {}

impl Property {
    /// Applies this property to a Pie mark.
    pub fn apply(&self, pie: &mut crate::data::mark::pie::Pie) {
        match self {
            Property::Hole(v) => pie.hole = v.clamp(0.0, 0.99),
            Property::Gap(v) => pie.gap = v.max(0.0),
        }
    }
}

pub use Property::{Gap, Hole};
