use std::borrow::Cow;

use crate::color::{Color, Pair};
use crate::core::{Font, theme};
use crate::palette::PaletteSeed;

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
    ///
    /// Deprecated: prefer `palette_seed()` with the palette system.
    fn data_colors(&self) -> Vec<Color>;

    /// Returns the seed colors for palette generation.
    ///
    /// The default implementation derives a seed from `data_colors()`.
    fn palette_seed(&self) -> PaletteSeed {
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
        PaletteSeed {
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

    fn data_colors(&self) -> Vec<Color> {
        (*self).data_colors()
    }

    fn palette_seed(&self) -> PaletteSeed {
        (*self).palette_seed()
    }

    fn divider_color(&self) -> Color {
        (*self).divider_color()
    }

    fn axis_color(&self) -> Color {
        (*self).axis_color()
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

    fn palette_seed(&self) -> PaletteSeed {
        let palette = self.palette();
        PaletteSeed {
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
}
