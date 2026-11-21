//! Property descriptors for area charts.

pub mod label;
pub mod series;

use crate::data::mark::area;
use crate::map::Map;

/// A property of an area chart.
#[derive(Debug, Clone, PartialEq)]
pub enum Property {
    /// Layout strategy (overlaid or stacked).
    Layout(area::Layout),
    /// Vertical gradient fill toggle.
    Gradient(bool),
    /// Envelope interpolation shared by every series.
    Interpolation(area::Interpolation),
    /// What the envelopes do at a missing value.
    Nulls(area::Nulls),
    /// Excel drop lines; `None` removes them.
    DropLines(Option<area::Connector>),
    /// Label property applied to all series.
    Label(label::Property),
    /// A property of a specific series.
    Series { index: usize, property: series::Property },
}

impl Map for Property {}

impl Property {
    pub fn apply(&self, area: &mut area::Area) {
        match self {
            Property::Layout(l) => area.set_layout(*l),
            Property::Gradient(g) => area.set_gradient(*g),
            Property::Interpolation(i) => area.set_interpolation(*i),
            Property::Nulls(n) => area.set_nulls(*n),
            Property::DropLines(c) => area.set_drop_lines(c.clone()),
            Property::Label(p) => p.apply(area),
            Property::Series { index, property } => {
                if let Some(s) = area.series_mut().get_mut(*index) {
                    property.apply(s);
                }
            }
        }
    }
}

// Re-export enum variants as constructor functions
pub use Property::{DropLines, Gradient, Interpolation, Label, Layout, Nulls};

/// Creates a series property wrapper for a given series index.
///
/// Use with `.map()` to transform series properties:
/// ```ignore
/// color_picker(color).map(props::area::series::Color).map(props::area::Series(0))
/// ```
#[allow(non_snake_case)]
pub fn Series(index: usize) -> impl Fn(series::Property) -> Property {
    move |property| Property::Series { index, property }
}
