//! X-axis scale: the domain kind decides which `Scale<T>` carries it.

use crate::scale::Scale;

/// X-axis scale tagged with its domain kind.
///
/// The x-axis differs from y/value/size/color because its domain isn't
/// always numeric: bar charts use string categories, time series use
/// timestamps, scatter plots use numbers. `XScale` is a closed sum so
/// renderers can dispatch without unwrapping a generic.
///
/// Construct each variant explicitly with the matching `Scale<T>`:
///
/// ```ignore
/// use hyozu::{Scale, XScale};
/// let categories: XScale = XScale::Category(Scale::new());
/// let timeline: XScale = XScale::Time(
///     Scale::new().format(|t: &jiff::Zoned| t.strftime("%Y-%m").to_string()),
/// );
/// ```
#[non_exhaustive]
#[derive(Clone, Debug)]
pub enum XScale {
    /// Categorical x-axis (bars, stacked bars, treemap by name).
    Category(Scale<String>),
    /// Numeric x-axis (scatter, xy, line by index).
    Linear(Scale<f64>),
    /// Temporal x-axis (time series).
    Time(Scale<jiff::Zoned>),
}

impl Default for XScale {
    fn default() -> Self {
        Self::Linear(Scale::default())
    }
}
