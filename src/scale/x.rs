//! X-axis scale: the domain kind decides which `Scale<T>` carries it.

use crate::scale::{Scale, Transform};

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

impl XScale {
    /// The numeric transform applied to x positions. Only
    /// [`XScale::Linear`] carries one; category and time axes map
    /// linearly.
    pub fn transform(&self) -> Transform {
        match self {
            Self::Linear(scale) => scale.transform,
            Self::Category(_) | Self::Time(_) => Transform::Linear,
        }
    }

    /// Whether auto-derived x bounds round outward to nice numbers.
    pub fn nice(&self) -> bool {
        match self {
            Self::Category(scale) => scale.nice,
            Self::Linear(scale) => scale.nice,
            Self::Time(scale) => scale.nice,
        }
    }

    /// Returns a numeric scale with `transform`. A [`XScale::Linear`] scale
    /// keeps its format and nice flag; a category or time scale has no
    /// numeric transform to set, so it's replaced by a default numeric
    /// scale carrying `transform`.
    pub fn with_transform(self, transform: Transform) -> Self {
        match self {
            Self::Linear(scale) => Self::Linear(scale.transform(transform)),
            Self::Category(_) | Self::Time(_) => Self::Linear(Scale::new().transform(transform)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linear_scale_reports_its_transform() {
        let x = XScale::Linear(Scale::new().log());
        assert_eq!(x.transform(), Transform::Log);
        assert_eq!(XScale::Category(Scale::new()).transform(), Transform::Linear);
    }

    #[test]
    fn with_transform_keeps_linear_format_and_replaces_categories() {
        let x = XScale::Linear(Scale::new().format(|v| format!("{v}x")).nice(false)).with_transform(Transform::Sqrt);
        let XScale::Linear(scale) = &x else {
            panic!("expected a numeric scale");
        };
        assert_eq!(scale.transform, Transform::Sqrt);
        assert!(!scale.nice);
        assert_eq!(scale.format_or_default(2.0), "2x");

        let x = XScale::Category(Scale::new()).with_transform(Transform::Log);
        assert_eq!(x.transform(), Transform::Log);
    }
}
