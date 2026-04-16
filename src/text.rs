//! Text styling shared across axis labels, data labels, legend, and title.
//!
//! [`Style`] is an optional, composable description of how a piece of chart
//! text should look: font family, size, weight, and italic. Every field is
//! `Option<_>`, so unset fields transparently fall back to a caller-provided
//! default (usually pulled from the active [`Design`](crate::Design)).

use crate::core::{Font, Pixels, font};

/// How a piece of chart text should be drawn.
///
/// Construct with [`Style::new`] and the fluent setters:
///
/// ```
/// use hyozu::text;
/// use iced::{Font, font};
///
/// let s = text::Style::new()
///     .font(Font::new("Inter"))
///     .size(14.0)
///     .weight(font::Weight::Semibold);
/// ```
///
/// Every field is `Option<_>`. When a renderer needs a concrete font, it
/// calls [`Style::resolved_font`] / [`Style::resolved_size`] with the default
/// pulled from the active [`Design`](crate::Design).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Style {
    /// Font family (plus any baseline weight/style baked into the `Font`).
    pub family: Option<Font>,
    /// Font size in pixels.
    pub size: Option<Pixels>,
    /// Font weight override (overrides whatever `family` carries).
    pub weight: Option<font::Weight>,
    /// Font style override (e.g. italic).
    pub style: Option<font::Style>,
}

impl Style {
    /// An empty style: every field unset, everything falls back to defaults.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the font family. Accepts anything convertible to a
    /// [`Font`], so you can pass `"Inter"` directly.
    pub fn font(mut self, font: impl Into<Font>) -> Self {
        self.family = Some(font.into());
        self
    }

    /// Sets the font size.
    pub fn size(mut self, size: impl Into<Pixels>) -> Self {
        self.size = Some(size.into());
        self
    }

    /// Sets the font weight (e.g. `font::Weight::Bold`).
    pub fn weight(mut self, weight: font::Weight) -> Self {
        self.weight = Some(weight);
        self
    }

    /// Sets the font style (e.g. `font::Style::Italic`).
    pub fn font_style(mut self, style: font::Style) -> Self {
        self.style = Some(style);
        self
    }

    /// Shortcut for `font_style(font::Style::Italic)`.
    pub fn italic(self) -> Self {
        self.font_style(font::Style::Italic)
    }

    /// Returns true if every field is unset.
    pub fn is_empty(&self) -> bool {
        self.family.is_none() && self.size.is_none() && self.weight.is_none() && self.style.is_none()
    }

    /// Merges this style with a fallback: fields set on `self` win, everything
    /// else is taken from `fallback`. Useful for layering per-element style on
    /// top of a design-wide default.
    pub fn or(self, fallback: &Style) -> Style {
        Style {
            family: self.family.or(fallback.family),
            size: self.size.or(fallback.size),
            weight: self.weight.or(fallback.weight),
            style: self.style.or(fallback.style),
        }
    }

    /// Resolves to a concrete [`Font`], merging `family` with any `weight` /
    /// `style` overrides on top of the caller-supplied `default`.
    ///
    /// This replaces the repeated render-site pattern:
    /// ```ignore
    /// let mut font = theme.font();
    /// if let Some(w) = label.weight { font.weight = w; }
    /// if let Some(s) = label.style  { font.style  = s; }
    /// ```
    pub fn resolved_font(&self, default: Font) -> Font {
        let mut font = self.family.unwrap_or(default);
        if let Some(w) = self.weight {
            font.weight = w;
        }
        if let Some(s) = self.style {
            font.style = s;
        }
        font
    }

    /// Resolves to a concrete pixel size, falling back to `default`.
    pub fn resolved_size(&self, default: f32) -> f32 {
        self.size.map(|p| p.0).unwrap_or(default)
    }
}

impl From<Pixels> for Style {
    fn from(size: Pixels) -> Self {
        Self::new().size(size)
    }
}

impl From<f32> for Style {
    fn from(size: f32) -> Self {
        Self::new().size(size)
    }
}

impl From<Font> for Style {
    fn from(font: Font) -> Self {
        let mut st = Self::new();
        st.family = Some(font);
        st
    }
}

impl From<&'static str> for Style {
    fn from(name: &'static str) -> Self {
        Self::new().font(name)
    }
}

/// Shorthand for `Style::from(f)`. Lets you write `font("Inter") + 14.0 + Bold`.
pub fn font(f: impl Into<Style>) -> Style {
    f.into()
}

// Composition via `+` so styles read naturally:
//   Style::from("Playfair Display") + 26.0 + Semibold

impl std::ops::Add<f32> for Style {
    type Output = Style;
    fn add(self, size: f32) -> Style {
        self.size(size)
    }
}

impl std::ops::Add<Pixels> for Style {
    type Output = Style;
    fn add(self, size: Pixels) -> Style {
        self.size(size)
    }
}

impl std::ops::Add<Font> for Style {
    type Output = Style;
    fn add(self, font: Font) -> Style {
        self.font(font)
    }
}

impl std::ops::Add<&'static str> for Style {
    type Output = Style;
    fn add(self, name: &'static str) -> Style {
        self.font(name)
    }
}

impl std::ops::Add<font::Weight> for Style {
    type Output = Style;
    fn add(self, weight: font::Weight) -> Style {
        self.weight(weight)
    }
}

impl std::ops::Add<font::Style> for Style {
    type Output = Style;
    fn add(self, style: font::Style) -> Style {
        self.font_style(style)
    }
}

// Re-export common weight/style variants for terse composition.
pub use crate::core::font::Style::Italic;
pub use crate::core::font::Weight::{Black, Bold, Light, Medium, Semibold, Thin};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_style_has_no_fields_set() {
        let s = Style::new();
        assert!(s.is_empty());
        assert!(s.family.is_none());
        assert!(s.size.is_none());
        assert!(s.weight.is_none());
        assert!(s.style.is_none());
    }

    #[test]
    fn builders_set_individual_fields() {
        let s = Style::new()
            .font(Font::new("Inter"))
            .size(14.0)
            .weight(font::Weight::Bold)
            .italic();
        assert_eq!(s.family, Some(Font::new("Inter")));
        assert_eq!(s.size, Some(Pixels(14.0)));
        assert_eq!(s.weight, Some(font::Weight::Bold));
        assert_eq!(s.style, Some(font::Style::Italic));
    }

    #[test]
    fn or_prefers_self_and_fills_from_fallback() {
        let fallback = Style::new().size(12.0).weight(font::Weight::Normal);
        let s = Style::new().size(16.0).or(&fallback);
        assert_eq!(s.size, Some(Pixels(16.0)));
        assert_eq!(s.weight, Some(font::Weight::Normal));
    }

    #[test]
    fn resolved_font_applies_overrides() {
        let base = Font::default();
        let style = Style::new().weight(font::Weight::Bold).italic();
        let resolved = style.resolved_font(base);
        assert_eq!(resolved.weight, font::Weight::Bold);
        assert_eq!(resolved.style, font::Style::Italic);
    }

    #[test]
    fn resolved_size_falls_back() {
        assert_eq!(Style::new().resolved_size(12.0), 12.0);
        assert_eq!(Style::new().size(18.0).resolved_size(12.0), 18.0);
    }
}
