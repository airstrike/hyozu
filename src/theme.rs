use crate::core::theme::{Custom, Palette};
use crate::core::{Theme, color};
use std::sync::Arc;

/// Creates a custom "Paper" theme with a warm, paper-like aesthetic.
pub fn paper() -> Theme {
    Theme::Custom(Arc::new(Custom::new("Paper".into(), Palette {
        background: color!(0xf2eede),
        text: color!(0x555555),
        primary: color!(0x1a1a1a), // Dark gray
        success: color!(0x1e6fcc), // Blue
        warning: color!(0x216609), // Green
        danger: color!(0xcc3e28),  // Red-orange
    })))
}

/// Creates a custom "Paper Dark" theme with a dark, paper-like aesthetic.
pub fn paper_dark() -> Theme {
    Theme::Custom(Arc::new(Custom::new("Paper Dark".into(), Palette {
        background: color!(0x1f1e1a), // Warm dark background
        text: color!(0xd4c8b0),       // Warm muted paper color
        primary: color!(0xe8dcc0),    // Warm light paper color
        success: color!(0x1e6fcc),    // Blue
        warning: color!(0x216609),    // Green
        danger: color!(0xcc3e28),     // Red-orange
    })))
}

/// Creates a custom "Hyozu" theme inspired by icy mountains and cherry blossoms.
pub fn hyozu() -> Theme {
    Theme::Custom(Arc::new(Custom::new("Hyozu".into(), Palette {
        background: color!(0xf5f0e6), // Vanilla cream
        text: color!(0x5b798a),       // Slate grey
        primary: color!(0xf5b1a4),    // Powder blush
        success: color!(0x96b0b2),    // Cool steel
        warning: color!(0xdba7b0),    // Soft blossom
        danger: color!(0x87172d),     // Burgundy
    })))
}

/// Creates a custom "Hyozu Dark" theme with a dark, icy aesthetic.
pub fn hyozu_dark() -> Theme {
    Theme::Custom(Arc::new(Custom::new("Hyozu Dark".into(), Palette {
        background: color!(0x1e252d), // Deep slate
        text: color!(0xf3ebd5),       // Vanilla cream
        primary: color!(0xf5b1a4),    // Powder blush
        success: color!(0x96b0b2),    // Cool steel
        warning: color!(0xdba7b0),    // Soft blossom
        danger: color!(0x87172d),     // Burgundy
    })))
}

/// Returns all available themes, including iced's built-in themes and our custom Paper themes.
pub fn all_themes() -> impl Iterator<Item = Theme> {
    [paper(), paper_dark(), hyozu(), hyozu_dark()]
        .into_iter()
        .chain(Theme::ALL.iter().cloned())
}
