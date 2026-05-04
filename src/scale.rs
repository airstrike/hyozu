//! Per-channel scale: how a numeric value (or category, or time) becomes
//! a glyph attribute, and how it formats back into text.
//!
//! [`Scale<T>`] carries two concerns: an optional `format` closure that
//! turns a `&T` into the string a viewer reads, and a [`Transform`] that
//! tells the renderer how to map the numeric domain onto the screen.
//!
//! ## Precedence
//!
//! Consumers walk a chain when looking up the format string for a value:
//!
//! 1. **Guide-level override** — e.g. [`legend::Config::value_format`] or
//!    [`Tooltip::format`]. Highest precedence; lets the legend show
//!    "$86.2M" while the in-mark label shows "86.2".
//! 2. **Mark-level override** — e.g. [`Pie::value_format`]. Applies only
//!    to the one mark that owns it.
//! 3. **Data-level scale** — set on [`Data`] via `Data::value_format`,
//!    `Data::y_format`, etc. Applies to every site that reads the
//!    relevant channel.
//! 4. **Built-in default** — [`default_f64_format`] for `f64`. The
//!    fallback that keeps charts readable with zero configuration.
//!
//! The chain stops at the first override that matches; sites don't
//! short-circuit once they've seen any non-default value, they walk the
//! whole list and pick the highest-precedence non-`None` entry.
//!
//! [`legend::Config::value_format`]: crate::data::legend::Config::value_format
//! [`Tooltip::format`]: crate::data::tooltip::Tooltip
//! [`Pie::value_format`]: crate::data::mark::pie::Pie::value_format
//! [`Data`]: crate::data::Data

use std::sync::Arc;

pub mod transform;
pub mod x;

pub use transform::Transform;
pub use x::XScale;

/// Shared format closure: `Arc` so it clones cheaply and crosses thread
/// boundaries (iced runs widget code on a worker pool).
pub type Format<T> = Arc<dyn Fn(&T) -> String + Send + Sync>;

/// A scale on one channel: format closure plus numeric transform.
///
/// `Scale<T>` is plain data — clone is cheap (the format closure is
/// stored in an [`Arc`]), no allocations on read. Construct via
/// [`Scale::new`] / [`Default::default`] and chain builder methods:
///
/// ```ignore
/// use hyozu::Scale;
///
/// let dollars: Scale<f64> = Scale::new().format(|v| format!("${v:.1}M"));
/// ```
///
/// See the module doc for how renderers walk the precedence chain.
pub struct Scale<T> {
    /// Closure that renders a `&T` into a display string. `None` means
    /// "use the next layer in the chain."
    pub format: Option<Format<T>>,
    /// How the numeric domain maps onto the screen position. Honored by
    /// numeric-axis marks (Bar, Line, Area, Xy); ignored by Pie,
    /// Choropleth, TileGrid, BubbleMap.
    pub transform: Transform,
}

impl<T> Default for Scale<T> {
    fn default() -> Self {
        Self {
            format: None,
            transform: Transform::default(),
        }
    }
}

impl<T> Clone for Scale<T> {
    fn clone(&self) -> Self {
        Self {
            format: self.format.clone(),
            transform: self.transform,
        }
    }
}

impl<T> std::fmt::Debug for Scale<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Scale")
            .field("format", &self.format.as_ref().map(|_| "<function>"))
            .field("transform", &self.transform)
            .finish()
    }
}

impl<T> Scale<T> {
    /// Returns an empty scale (no format closure, [`Transform::Linear`]).
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the format closure. Stored in an [`Arc`] so subsequent clones
    /// of this scale share the same allocation.
    pub fn format(mut self, f: impl Fn(&T) -> String + Send + Sync + 'static) -> Self {
        self.format = Some(Arc::new(f));
        self
    }

    /// Sets the transform to [`Transform::Linear`] (the default).
    pub fn linear(mut self) -> Self {
        self.transform = Transform::Linear;
        self
    }

    /// Sets the transform to [`Transform::Log`]. Honored by numeric-axis
    /// marks; the renderer maps values via `log10` between domain min and
    /// max.
    pub fn log(mut self) -> Self {
        self.transform = Transform::Log;
        self
    }
}

impl Scale<f64> {
    /// Formats `value` using this scale's format closure when set, else
    /// falls back to [`default_f64_format`].
    pub fn format_or_default(&self, value: f64) -> String {
        match &self.format {
            Some(f) => f(&value),
            None => default_f64_format(value),
        }
    }
}

/// Built-in `f64` formatter: integer-valued numbers render without a
/// fractional part, otherwise one decimal. The fallback used when no
/// chain layer supplies a format closure.
pub fn default_f64_format(value: f64) -> String {
    if value.fract().abs() < 0.001 {
        format!("{}", value as i64)
    } else {
        format!("{value:.1}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_f64_format_strips_trivial_fraction() {
        assert_eq!(default_f64_format(42.0), "42");
        assert_eq!(default_f64_format(42.0005), "42");
    }

    #[test]
    fn default_f64_format_keeps_one_decimal_for_non_integer() {
        assert_eq!(default_f64_format(42.5), "42.5");
        assert_eq!(default_f64_format(0.7), "0.7");
    }

    #[test]
    fn format_or_default_uses_closure_when_set() {
        let s: Scale<f64> = Scale::new().format(|v| format!("${v:.2}"));
        assert_eq!(s.format_or_default(7.5), "$7.50");
    }

    #[test]
    fn format_or_default_falls_back_when_unset() {
        let s: Scale<f64> = Scale::new();
        assert_eq!(s.format_or_default(42.5), "42.5");
    }

    #[test]
    fn log_builder_flips_transform() {
        let s: Scale<f64> = Scale::new().log();
        assert_eq!(s.transform, Transform::Log);
    }
}
