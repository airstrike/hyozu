use std::borrow::Cow;

use crate::color::{Color, Pair};
use crate::core::{Font, color, theme};
use crate::palette::Seed;

/// Design system trait for chart styling.
///
/// This trait defines how charts should be styled, including colors, fonts,
/// and sizing. Implement this trait to customize the appearance of charts.
pub trait Design {
    /// Returns the background color for the chart.
    fn background_color(&self) -> crate::core::Color;

    /// Returns the default text color for chart elements.
    fn text_color(&self) -> Color;

    /// Returns the text color pair for adaptive contrast.
    fn text_pair(&self) -> Pair;

    /// Returns an ordered palette of colors for data visualization.
    /// The first color is used for the first series, second for the second series, etc.
    #[deprecated(note = "implement `seed()` directly instead of relying on data_colors fallback")]
    fn data_colors(&self) -> Vec<Color>;

    /// Returns the seed colors for palette generation.
    ///
    /// The default implementation derives a seed from `data_colors()` for
    /// backwards compatibility with older `Design` impls.
    fn seed(&self) -> Seed {
        #[allow(deprecated)]
        let colors = self.data_colors();
        let bg = self.background_color();
        let get = |i: usize| {
            if colors.is_empty() {
                return crate::core::Color::BLACK;
            }
            let base = colors
                .get(i % colors.len())
                .and_then(|c| match c {
                    Color::Fixed(c) => Some(*c),
                    _ => None,
                })
                .unwrap_or(crate::core::Color::BLACK);
            let wrap = i / colors.len();
            if wrap == 0 {
                base
            } else {
                crate::palette::shift_lightness(base, bg, wrap)
            }
        };
        Seed {
            primary: get(0),
            secondary: get(1),
            success: get(2),
            warning: get(3),
            danger: get(4),
            background: bg,
        }
    }

    /// Returns the color for dividers (grid lines, separators).
    fn divider_color(&self) -> Color;

    /// Returns the color for axis lines and ticks.
    fn axis_color(&self) -> Color;

    /// Returns the fill color for "in-scope feature with no entry"
    /// regions on data-driven maps (the choropleth's `Missing` state).
    ///
    /// Distinct from the decorative land/ocean fill used for purely
    /// out-of-scope features: this slot signals "this region IS part
    /// of the chart's universe, but no data was supplied for it."
    /// ggplot, D3, and Vega-Lite all reserve a dedicated knob for
    /// this — matching the convention here.
    ///
    /// The default is theme-aware: light gray (`0xDDDDDD`, the news-
    /// graphics convention used by NYT/FT/BBC) on light backgrounds,
    /// mid-dark gray (`0x6B6B6B`) on dark backgrounds.
    fn missing_fill(&self) -> crate::core::Color {
        if crate::palette::is_dark_background(self.background_color()) {
            color!(0x6B6B6B)
        } else {
            color!(0xDDDDDD)
        }
    }

    /// Returns the fill color used for the "ocean" — the decorative
    /// background painted under choropleth maps in regions that are
    /// outside any feature's polygon.
    ///
    /// Exposed on the trait so callers wrapping a choropleth chart can
    /// match this color in their own surrounding chrome (e.g. an iced
    /// `container` background, or the chart widget's own
    /// [`crate::chart::Style::background`]) without hardcoding an RGB
    /// constant that drifts when the theme changes.
    ///
    /// The default is theme-aware: a faint cool-tinted near-white
    /// (`0xF2F7FA`) on light backgrounds, and a deep navy
    /// (`0x1A2333`) on dark backgrounds.
    fn ocean_fill(&self) -> crate::core::Color {
        if crate::palette::is_dark_background(self.background_color()) {
            color!(0x1A2333)
        } else {
            color!(0xF2F7FA)
        }
    }

    /// Returns the color for major gridlines inside the plot area.
    ///
    /// Gridlines should sit visually *beneath* the axis frame, so the
    /// default is a neutral gray at low opacity — clearly more muted than
    /// [`axis_color()`]. Themes should override this to derive a color
    /// from their own text/palette.
    ///
    /// [`axis_color()`]: Self::axis_color
    fn grid_color(&self) -> Color {
        Color::from_rgba(0.5, 0.5, 0.5, 0.12)
    }

    /// Returns the color for minor gridlines inside the plot area.
    ///
    /// Defaults to roughly half the weight of [`grid_color()`] so minor
    /// lines read as subdivisions without competing with major lines.
    ///
    /// [`grid_color()`]: Self::grid_color
    fn minor_grid_color(&self) -> Color {
        Color::from_rgba(0.5, 0.5, 0.5, 0.06)
    }

    /// Returns the default font for chart text.
    fn font(&self) -> Font;

    /// Returns the default font size for chart text.
    fn font_size(&self) -> f32;

    /// Default text style for the chart title. Themes override this to
    /// customise title typography; callers on [`crate::Data`] may further
    /// layer a per-chart [`crate::text::Style`] on top.
    ///
    /// The default is `font()` at 16px — the historical hardcoded title size.
    fn title_text(&self) -> crate::text::Style {
        crate::text::Style::new().font(self.font()).size(16.0)
    }

    /// Default text style for axis tick labels and titles.
    ///
    /// Defaults to `font()` at `font_size()`.
    fn axis_text(&self) -> crate::text::Style {
        crate::text::Style::new().font(self.font()).size(self.font_size())
    }

    /// Default text style for the legend.
    ///
    /// Defaults to `font()` at 10px — 2px smaller than the general 12px
    /// label size, matching the historical `DEFAULT_FONT_SIZE` in the
    /// legend renderer.
    fn legend_text(&self) -> crate::text::Style {
        crate::text::Style::new().font(self.font()).size(10.0)
    }

    /// Default text style for in-plot data labels (bar totals, line point
    /// labels, pie slice labels, etc.).
    ///
    /// Defaults to `font()` at `font_size()`.
    fn data_label_text(&self) -> crate::text::Style {
        crate::text::Style::new().font(self.font()).size(self.font_size())
    }

    /// Returns the default corner rounding applied to roundable marks:
    /// bar/column value-ends, pie & donut slices, and the gauge value arc.
    ///
    /// Pixel radii — interpreted per-corner for rectangular bars (so a
    /// `[tl, tr, br, bl]` radius rounds individual corners, matching
    /// Recharts' `radius` prop) and as a uniform cap radius for polar
    /// marks (matching Recharts' `cornerRadius`).
    ///
    /// Defaults to square (`0.0`) so rounding is strictly opt-in and no
    /// existing chart changes appearance. A theme overrides this for a
    /// house style; a per-chart [`crate::chart::Style::corners`] override
    /// (set inside [`crate::chart::Chart::style`]) layers on top.
    fn corners(&self) -> crate::core::border::Radius {
        0.0.into()
    }

    /// Returns the default fill for the "track" rail drawn behind bars: the
    /// faint full-extent rectangle that shows the axis range a bar grows
    /// within (Recharts `<Bar background>`; the same rail the gauge already
    /// paints behind its value arc).
    ///
    /// `None` (default) means no rail, so the feature is strictly opt-in and
    /// no existing chart changes appearance. A theme returns `Some(color)` to
    /// give every bar chart a house-style rail; a per-chart
    /// [`crate::chart::Style::track`] override (set inside
    /// [`crate::chart::Chart::style`]) layers on top. The rail adopts the
    /// design's [`corners`](Self::corners) radius.
    fn bar_track(&self) -> Option<crate::core::Color> {
        None
    }

    /// Returns an owned [`Cow`] of this design.
    fn to_cow(&self) -> Cow<'static, Self>
    where
        Self: Sized + Clone,
    {
        Cow::Owned(self.clone())
    }
}

/// A trait for converting a value into a [`Cow`] design.
pub trait IntoDesign<'a, D: Design + Clone> {
    /// Converts the value into a [`Cow`] design.
    fn into_design(self) -> Cow<'a, D>;
}

impl<'a, D: Design + Clone> IntoDesign<'a, D> for Cow<'a, D> {
    fn into_design(self) -> Cow<'a, D> {
        self
    }
}

impl<'a, D: Design + Clone> IntoDesign<'a, D> for &'a D {
    fn into_design(self) -> Cow<'a, D> {
        Cow::Borrowed(self)
    }
}

/// Blanket implementation for references to types that implement Design.
impl<T: Design> Design for &T {
    fn background_color(&self) -> crate::core::Color {
        (*self).background_color()
    }

    fn text_color(&self) -> Color {
        (*self).text_color()
    }

    fn text_pair(&self) -> Pair {
        (*self).text_pair()
    }

    #[allow(deprecated)]
    fn data_colors(&self) -> Vec<Color> {
        (*self).data_colors()
    }

    fn seed(&self) -> Seed {
        (*self).seed()
    }

    fn divider_color(&self) -> Color {
        (*self).divider_color()
    }

    fn axis_color(&self) -> Color {
        (*self).axis_color()
    }

    fn missing_fill(&self) -> crate::core::Color {
        (*self).missing_fill()
    }

    fn ocean_fill(&self) -> crate::core::Color {
        (*self).ocean_fill()
    }

    fn grid_color(&self) -> Color {
        (*self).grid_color()
    }

    fn minor_grid_color(&self) -> Color {
        (*self).minor_grid_color()
    }

    fn font(&self) -> Font {
        (*self).font()
    }

    fn font_size(&self) -> f32 {
        (*self).font_size()
    }

    fn title_text(&self) -> crate::text::Style {
        (*self).title_text()
    }

    fn axis_text(&self) -> crate::text::Style {
        (*self).axis_text()
    }

    fn legend_text(&self) -> crate::text::Style {
        (*self).legend_text()
    }

    fn data_label_text(&self) -> crate::text::Style {
        (*self).data_label_text()
    }

    fn corners(&self) -> crate::core::border::Radius {
        (*self).corners()
    }
}

/// Default implementation of Design for iced::Theme.
impl Design for theme::Theme {
    fn background_color(&self) -> crate::core::Color {
        theme::Base::base(self).background_color
    }

    fn text_color(&self) -> Color {
        theme::Base::base(self).text_color.into()
    }

    fn text_pair(&self) -> Pair {
        let text = theme::Base::base(self).text_color;
        let inverted = crate::color::invert_brightness(text);

        // Provide two options: original text and inverted
        // Pair::resolve() will pick whichever has better contrast
        Pair::new(text, inverted)
    }

    fn data_colors(&self) -> Vec<Color> {
        let palette = self.palette();
        vec![
            palette.primary.strong.color.into(),
            palette.primary.base.color.into(),
            palette.success.base.color.into(),
            palette.warning.base.color.into(),
            palette.danger.base.color.into(),
        ]
    }

    fn seed(&self) -> Seed {
        let palette = self.palette();
        Seed {
            primary: palette.primary.base.color,
            secondary: palette.secondary.base.color,
            success: palette.success.base.color,
            warning: palette.warning.base.color,
            danger: palette.danger.base.color,
            background: self.background_color(),
        }
    }

    fn divider_color(&self) -> Color {
        let text = theme::Base::base(self).text_color;
        // Derive divider from text color with low opacity for subtlety
        Color::from_rgba(text.r, text.g, text.b, 0.15)
    }

    fn axis_color(&self) -> Color {
        theme::Base::base(self).text_color.into()
    }

    fn grid_color(&self) -> Color {
        // Major gridlines: subtle tint of the text color — sits clearly
        // below `axis_color` (text_color at full opacity).
        let text = theme::Base::base(self).text_color;
        Color::from_rgba(text.r, text.g, text.b, 0.08)
    }

    fn minor_grid_color(&self) -> Color {
        let text = theme::Base::base(self).text_color;
        Color::from_rgba(text.r, text.g, text.b, 0.04)
    }

    fn font(&self) -> Font {
        Font::default()
    }

    fn font_size(&self) -> f32 {
        12.0
    }

    fn corners(&self) -> crate::core::border::Radius {
        4.0.into()
    }
}
