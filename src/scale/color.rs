//! Color encoding scale: how a numeric value becomes a color, and how
//! the legend formats domain values back into text.
//!
//! [`ColorScale<I>`] is a sibling to [`crate::scale::Scale<T>`] for
//! channels whose target is a color (Choropleth fills, Heatmap cells,
//! sequential bar coloring). It carries an optional explicit
//! `domain` (else inferred from data), an optional [`Palette`] (else the
//! mark's default), a [`Transform`] (default [`Transform::Linear`]), and
//! an optional `format` closure for legend ticks.
//!
//! ## Resolution
//!
//! Renderers don't read the fields directly — they call
//! [`ColorScale::resolved_domain`] / [`ColorScale::resolved_palette`]
//! with a fallback closure that supplies the data-derived default. The
//! user's explicit setting wins; otherwise the closure runs lazily.

use std::sync::Arc;

use crate::palette::Palette;
use crate::scale::{Format, Transform};

/// A color encoding scale: domain + palette + transform + format.
///
/// `ColorScale<I>` is plain data — clone is cheap (the format closure is
/// stored in an [`Arc`]). The type parameter `I` is the domain value
/// type: `f64` for sequential numeric scales, `i64` for integer counts,
/// etc.
///
/// ```ignore
/// use hyozu::scale::ColorScale;
///
/// let cs: ColorScale<f64> = ColorScale::default()
///     .sqrt()
///     .domain(0.0, 100.0);
/// ```
pub struct ColorScale<I> {
    /// Explicit domain `(lo, hi)`. `None` means "infer from data."
    pub domain: Option<(I, I)>,
    /// Explicit palette. `None` means "use the mark's default."
    pub palette: Option<Palette>,
    /// How the domain maps onto the unit interval before sampling the
    /// palette.
    pub transform: Transform,
    /// Closure that renders an `&I` into a display string for legend
    /// ticks. `None` means "fall back to the chart's default formatter
    /// for `I`."
    pub format: Option<Format<I>>,
}

impl<I> Default for ColorScale<I> {
    fn default() -> Self {
        Self {
            domain: None,
            palette: None,
            transform: Transform::Linear,
            format: None,
        }
    }
}

impl<I: Clone> Clone for ColorScale<I> {
    fn clone(&self) -> Self {
        Self {
            domain: self.domain.clone(),
            palette: self.palette.clone(),
            transform: self.transform,
            format: self.format.clone(),
        }
    }
}

impl<I: std::fmt::Debug> std::fmt::Debug for ColorScale<I> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ColorScale")
            .field("domain", &self.domain)
            .field("palette", &self.palette)
            .field("transform", &self.transform)
            .field("format", &self.format.as_ref().map(|_| "<function>"))
            .finish()
    }
}

impl<I: Clone> ColorScale<I> {
    /// Sets the explicit domain. Overrides any data-derived inference at
    /// resolve time.
    pub fn domain(mut self, lo: I, hi: I) -> Self {
        self.domain = Some((lo, hi));
        self
    }

    /// Sets the explicit palette. Overrides the mark's default at
    /// resolve time.
    pub fn palette(mut self, p: Palette) -> Self {
        self.palette = Some(p);
        self
    }

    /// Sets the transform to [`Transform::Linear`] (the default).
    pub fn linear(mut self) -> Self {
        self.transform = Transform::Linear;
        self
    }

    /// Sets the transform to [`Transform::Sqrt`]. Useful for skewed
    /// numeric distributions where Log over-emphasizes the low end.
    pub fn sqrt(mut self) -> Self {
        self.transform = Transform::Sqrt;
        self
    }

    /// Sets the transform to [`Transform::Log`].
    pub fn log(mut self) -> Self {
        self.transform = Transform::Log;
        self
    }

    /// Sets the format closure used for legend tick labels. Stored in an
    /// [`Arc`] so subsequent clones share the same allocation.
    pub fn format(mut self, f: impl Fn(&I) -> String + Send + Sync + 'static) -> Self {
        self.format = Some(Arc::new(f));
        self
    }

    /// Returns the user's explicit domain if set, else evaluates
    /// `fallback` (typically a data-derived `(min, max)` computation).
    pub fn resolved_domain(&self, fallback: impl FnOnce() -> (I, I)) -> (I, I) {
        self.domain.clone().unwrap_or_else(fallback)
    }

    /// Returns the user's explicit palette if set, else evaluates
    /// `fallback` (typically a theme-derived default palette).
    pub fn resolved_palette(&self, fallback: impl FnOnce() -> Palette) -> Palette {
        self.palette.clone().unwrap_or_else(fallback)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colorscale_clone_and_debug_compile() {
        let s: ColorScale<f64> = ColorScale::default();
        let s2: ColorScale<i64> = ColorScale::<i64>::default().sqrt().domain(0, 100);
        let _ = s.clone();
        let _ = s2.clone();
        let _ = format!("{:?}", s);
        let _ = format!("{:?}", s2);
    }

    #[test]
    fn default_is_linear_with_no_overrides() {
        let s: ColorScale<f64> = ColorScale::default();
        assert!(s.domain.is_none());
        assert!(s.palette.is_none());
        assert_eq!(s.transform, Transform::Linear);
    }

    #[test]
    fn builders_set_transform() {
        let s: ColorScale<f64> = ColorScale::default().sqrt();
        assert_eq!(s.transform, Transform::Sqrt);
        let s: ColorScale<f64> = ColorScale::default().log();
        assert_eq!(s.transform, Transform::Log);
        let s: ColorScale<f64> = ColorScale::default().linear();
        assert_eq!(s.transform, Transform::Linear);
    }

    #[test]
    fn resolved_domain_prefers_explicit() {
        let s: ColorScale<f64> = ColorScale::default().domain(0.0, 1.0);
        let (lo, hi) = s.resolved_domain(|| (10.0, 20.0));
        assert_eq!((lo, hi), (0.0, 1.0));
    }

    #[test]
    fn resolved_domain_falls_back_when_unset() {
        let s: ColorScale<f64> = ColorScale::default();
        let (lo, hi) = s.resolved_domain(|| (10.0, 20.0));
        assert_eq!((lo, hi), (10.0, 20.0));
    }
}
