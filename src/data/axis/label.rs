use std::sync::Arc;

/// Horizontal text alignment for axis labels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextAlign {
    Left,
    Center,
    Right,
}

/// Placement of labels relative to tick marks.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Placement {
    /// Labels positioned ON tick marks (default for continuous data)
    OnTicks,
    /// Labels positioned BETWEEN tick marks (default for bars)
    #[default]
    BetweenTicks,
}

/// What to do when a label's intrinsic width exceeds its allotted column
/// (the pixel distance to its nearest neighbor tick).
///
/// Applies to horizontal (bottom/top) axis labels. Vertical axis labels are
/// stacked by row and don't currently collide horizontally — they keep
/// intrinsic widths regardless of this setting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum Overflow {
    /// Truncate the last visual line with an ellipsis (`…`) when the label
    /// exceeds its column width. Axis height is unchanged. This is the
    /// default: the least surprising behavior for dense dashboards.
    #[default]
    Ellipsize,
    /// Wrap at word boundaries. The axis grows vertically to fit the tallest
    /// wrapped label. No information loss, but asymmetric wrapping can look
    /// uneven across columns.
    Wrap,
}

/// Axis labels configuration containing placement, values, and formatting
#[derive(Clone, Default)]
pub struct Labels {
    /// Where to place labels relative to ticks
    pub placement: Option<Placement>,

    /// Custom categorical labels
    pub values: Option<Vec<String>>,

    /// Format function for numeric labels
    pub format: Option<Arc<dyn Fn(f64) -> String + Send + Sync>>,

    /// Text alignment override (defaults: Center for x-axis, Right for y-axis)
    pub align: Option<TextAlign>,

    /// What to do when a label's intrinsic width exceeds its column.
    pub overflow: Overflow,

    /// Typography override for tick labels.
    pub text: crate::text::Style,
}

impl std::fmt::Debug for Labels {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Labels")
            .field("placement", &self.placement)
            .field("values", &self.values)
            .field("format", &self.format.as_ref().map(|_| "<function>"))
            .field("align", &self.align)
            .field("overflow", &self.overflow)
            .field("text", &self.text)
            .finish()
    }
}

impl Labels {
    /// Create new empty labels configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the placement
    pub fn with_placement(mut self, placement: Placement) -> Self {
        self.placement = Some(placement);
        self
    }

    /// Set categorical values
    pub fn with_values(mut self, values: Vec<String>) -> Self {
        self.values = Some(values);
        self
    }

    /// Set format function
    pub fn with_format<F>(mut self, format: F) -> Self
    where
        F: Fn(f64) -> String + Send + Sync + 'static,
    {
        self.format = Some(Arc::new(format));
        self
    }

    /// Set text alignment
    pub fn with_align(mut self, align: TextAlign) -> Self {
        self.align = Some(align);
        self
    }

    /// Set the overflow strategy for labels that exceed their column width.
    pub fn with_overflow(mut self, overflow: Overflow) -> Self {
        self.overflow = overflow;
        self
    }

    /// Check if labels are auto-generated (no custom values or format)
    pub fn is_auto(&self) -> bool {
        self.values.is_none() && self.format.is_none()
    }
}

// Natural conversions from common types

impl From<Placement> for Labels {
    fn from(placement: Placement) -> Self {
        Labels {
            placement: Some(placement),
            ..Default::default()
        }
    }
}

impl From<Vec<String>> for Labels {
    fn from(v: Vec<String>) -> Self {
        Labels {
            values: Some(v),
            ..Default::default()
        }
    }
}

impl From<Vec<&str>> for Labels {
    fn from(v: Vec<&str>) -> Self {
        Labels {
            values: Some(v.iter().map(|s| s.to_string()).collect()),
            ..Default::default()
        }
    }
}

impl<const N: usize> From<[&str; N]> for Labels {
    fn from(arr: [&str; N]) -> Self {
        Labels {
            values: Some(arr.iter().map(|s| s.to_string()).collect()),
            ..Default::default()
        }
    }
}

impl<const N: usize> From<[String; N]> for Labels {
    fn from(arr: [String; N]) -> Self {
        Labels {
            values: Some(arr.to_vec()),
            ..Default::default()
        }
    }
}

// From format functions
impl<F> From<F> for Labels
where
    F: Fn(f64) -> String + Send + Sync + 'static,
{
    fn from(f: F) -> Self {
        Labels {
            format: Some(Arc::new(f)),
            ..Default::default()
        }
    }
}

// Implement Add for Placement to start the chain
impl std::ops::Add<Labels> for Placement {
    type Output = Labels;

    fn add(self, mut rhs: Labels) -> Self::Output {
        rhs.placement = Some(self);
        rhs
    }
}

// Allow Placement + array of &str
impl<const N: usize> std::ops::Add<[&str; N]> for Placement {
    type Output = Labels;

    fn add(self, rhs: [&str; N]) -> Self::Output {
        Labels {
            placement: Some(self),
            values: Some(rhs.iter().map(|s| s.to_string()).collect()),
            ..Default::default()
        }
    }
}

// Allow Placement + array of String
impl<const N: usize> std::ops::Add<[String; N]> for Placement {
    type Output = Labels;

    fn add(self, rhs: [String; N]) -> Self::Output {
        Labels {
            placement: Some(self),
            values: Some(rhs.into_iter().collect()),
            ..Default::default()
        }
    }
}

// Allow Placement + Vec<&str>
impl std::ops::Add<Vec<&str>> for Placement {
    type Output = Labels;

    fn add(self, rhs: Vec<&str>) -> Self::Output {
        Labels {
            placement: Some(self),
            values: Some(rhs.iter().map(|s| s.to_string()).collect()),
            ..Default::default()
        }
    }
}

// Allow Placement + Vec<String>
impl std::ops::Add<Vec<String>> for Placement {
    type Output = Labels;

    fn add(self, rhs: Vec<String>) -> Self::Output {
        Labels {
            placement: Some(self),
            values: Some(rhs),
            ..Default::default()
        }
    }
}

// Allow transforming existing categorical labels
impl<F, T> std::ops::Add<F> for Labels
where
    F: Fn(&str) -> T,
    T: std::fmt::Display,
{
    type Output = Labels;

    fn add(mut self, transform: F) -> Self::Output {
        // If we already have values, transform them
        if let Some(values) = self.values {
            self.values = Some(values.iter().map(|s| transform(s).to_string()).collect());
        }
        // Otherwise, this doesn't make sense - you can't transform values that don't exist
        // Just return self unchanged
        self
    }
}

// Typography composition — same pattern as data labels.
//   BetweenTicks + MONTHS + "Inter" + 14.0 + Semibold

impl std::ops::Add<crate::text::Style> for Placement {
    type Output = Labels;
    fn add(self, text: crate::text::Style) -> Labels {
        let mut l = Labels::from(self);
        l.text = text;
        l
    }
}

impl std::ops::Add<crate::text::Style> for Labels {
    type Output = Labels;
    fn add(mut self, text: crate::text::Style) -> Labels {
        self.text = text;
        self
    }
}

impl std::ops::Add<crate::core::Font> for Labels {
    type Output = Labels;
    fn add(mut self, font: crate::core::Font) -> Labels {
        self.text.family = Some(font);
        self
    }
}

impl std::ops::Add<crate::core::font::Weight> for Labels {
    type Output = Labels;
    fn add(mut self, weight: crate::core::font::Weight) -> Labels {
        self.text.weight = Some(weight);
        self
    }
}

impl std::ops::Add<crate::core::font::Style> for Labels {
    type Output = Labels;
    fn add(mut self, style: crate::core::font::Style) -> Labels {
        self.text.style = Some(style);
        self
    }
}

impl std::ops::Add<f32> for Labels {
    type Output = Labels;
    fn add(mut self, size: f32) -> Labels {
        self.text.size = Some(crate::core::Pixels(size));
        self
    }
}

impl std::ops::Add<crate::core::Pixels> for Labels {
    type Output = Labels;
    fn add(mut self, size: crate::core::Pixels) -> Labels {
        self.text.size = Some(size);
        self
    }
}

impl From<crate::text::Style> for Labels {
    fn from(text: crate::text::Style) -> Self {
        Labels {
            text,
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_default_overflow_is_ellipsize() {
        // A fresh Labels config must default to Ellipsize so existing charts
        // quietly improve (no more silent overflow) without any API change.
        assert_eq!(Labels::new().overflow, Overflow::Ellipsize);
        assert_eq!(Labels::default().overflow, Overflow::Ellipsize);
    }

    #[test]
    fn labels_with_overflow_builder_sets_field() {
        let labels = Labels::new().with_overflow(Overflow::Wrap);
        assert_eq!(labels.overflow, Overflow::Wrap);
    }
}
