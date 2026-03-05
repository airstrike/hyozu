//! Property descriptors for bar charts.

pub mod label;
pub mod series;

use crate::data::mark::bar;
use crate::map::Map;

/// A property of a bar chart.
#[derive(Debug, Clone, PartialEq)]
pub enum Property {
    /// Bar size (proportion of available width, 0.1 to 1.0)
    Size(f32),
    /// Spacing between bars in a group (0.0 to 1.0)
    Spacing(f32),
    /// Layout strategy (grouped or stacked)
    Layout(bar::Layout),
    /// Label property (applies to all series)
    Label(label::Property),
    /// A property of a specific series
    Series { index: usize, property: series::Property },
}

impl Map for Property {}

impl Property {
    /// Applies this property to a Bars mark.
    pub fn apply(&self, bars: &mut bar::Bars) {
        match self {
            Property::Size(v) => bars.set_size(*v),
            Property::Spacing(v) => bars.set_spacing(*v),
            Property::Layout(v) => bars.layout = *v,
            Property::Label(p) => p.apply(bars),
            Property::Series { index, property } => {
                if let Some(series) = bars.series_mut().get_mut(*index) {
                    property.apply(series);
                }
            }
        }
    }
}

// Re-export enum variants as constructor functions
pub use Property::{Label, Layout, Size, Spacing};

/// Creates a series property wrapper for a given series index.
///
/// Use with `.map()` to transform series properties:
/// ```ignore
/// color_picker(color).map(props::bar::series::Color).map(props::bar::Series(0))
/// ```
#[allow(non_snake_case)]
pub fn Series(index: usize) -> impl Fn(series::Property) -> Property {
    move |property| Property::Series { index, property }
}
