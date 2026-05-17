//! Hyozu-specific iced widgets.
//!
//! This module re-exports [`iced_widget`] so internal chart code and callers
//! can use the same widget paths while hyozu adds small local widgets that
//! should not require a dependency on another app crate.

pub mod fit_text;

pub use fit_text::FitText;
pub use iced_widget::*;

/// Creates a new [`FitText`] from the given content.
///
/// [`FitText`] scales its font size to fit the bounds it is laid out into,
/// up to a configurable ceiling. See the [`fit_text`](mod@crate::widget::fit_text)
/// module docs for the semantics.
pub fn fit_text<'a, Theme, Renderer>(content: impl crate::core::text::IntoFragment<'a>) -> FitText<'a, Theme, Renderer>
where
    Theme: fit_text::Catalog,
    Renderer: crate::core::text::Renderer,
{
    FitText::new(content)
}
