use std::borrow::Cow;

use crate::color::{Color, Pair};
use crate::core::{Font, theme};

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
    fn data_colors(&self) -> Vec<Color>;

    /// Returns the color for dividers (grid lines, separators).
    fn divider_color(&self) -> Color;

    /// Returns the color for axis lines and ticks.
    fn axis_color(&self) -> Color;

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

    fn divider_color(&self) -> Color {
        (*self).divider_color()
    }

    fn axis_color(&self) -> Color {
        (*self).axis_color()
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
        let extended = self.extended_palette();
        vec![
            extended.primary.strong.color.into(),
            extended.primary.base.color.into(),
            extended.success.base.color.into(),
            extended.warning.base.color.into(),
            extended.danger.base.color.into(),
        ]
    }

    fn divider_color(&self) -> Color {
        let text = theme::Base::base(self).text_color;
        // Derive divider from text color with low opacity for subtlety
        Color::from_rgba(text.r, text.g, text.b, 0.15)
    }

    fn axis_color(&self) -> Color {
        theme::Base::base(self).text_color.into()
    }

    fn font(&self) -> Font {
        Font::default()
    }

    fn font_size(&self) -> f32 {
        12.0
    }
}
