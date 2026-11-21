//! Coordinated hover across charts that share an x dimension, like
//! Recharts' `syncId`.
//!
//! Keep one [`Cursor`] in application state per group of linked charts and
//! hand it to each chart with [`Chart::sync`]. The hovered chart reports
//! its snapped x as [`Action::Hovered`]; store it with [`Cursor::update`]
//! and every other chart in the group shows its tooltip, tracking line and
//! highlights at the same x. Bars link by category index, continuous marks
//! by data value.
//!
//! ```ignore
//! Message::Chart(action) => {
//!     self.cursor.update(&action);
//! }
//! // view
//! column(self.charts.iter().map(|data| {
//!     hyozu::chart(data).sync(self.cursor).on_action(Message::Chart).into()
//! }))
//! ```
//!
//! [`Chart::sync`]: crate::chart::Chart::sync

use crate::Action;

/// The x position hovered in a group of linked charts, if any.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Cursor {
    x: Option<f64>,
}

impl Cursor {
    /// No chart in the group is hovered.
    pub const IDLE: Self = Self { x: None };

    /// A hover at data-space `x`. A non-finite `x` is idle.
    pub fn at(x: f64) -> Self {
        Self {
            x: x.is_finite().then_some(x),
        }
    }

    /// The hovered x, if any.
    pub fn x(self) -> Option<f64> {
        self.x
    }

    /// Takes the cursor from an [`Action::Hovered`]; other actions leave it
    /// unchanged. Returns whether the cursor moved.
    pub fn update(&mut self, action: &Action) -> bool {
        match action {
            Action::Hovered(cursor) if cursor != self => {
                *self = *cursor;
                true
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_follows_hover_actions_and_ignores_others() {
        let mut cursor = Cursor::IDLE;
        assert!(cursor.update(&Action::Hovered(Cursor::at(3.0))));
        assert_eq!(cursor.x(), Some(3.0));
        assert!(!cursor.update(&Action::Hovered(Cursor::at(3.0))), "same x is no change");
        assert!(!cursor.update(&Action::Clicked(crate::Target::Title)));
        assert_eq!(cursor.x(), Some(3.0));
        assert!(cursor.update(&Action::Hovered(Cursor::IDLE)));
        assert_eq!(cursor.x(), None);
        assert_eq!(Cursor::at(f64::NAN), Cursor::IDLE);
    }
}
