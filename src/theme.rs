use crate::core::theme::palette::Seed;
use crate::core::{Theme, color};

/// Creates a custom "Paper" theme with a warm, paper-like aesthetic.
pub fn paper() -> Theme {
    Theme::custom("Paper", Seed {
        background: color!(0xf2eede),
        text: color!(0x555555),
        primary: color!(0x1a1a1a), // Dark gray
        success: color!(0x216609), // Green
        warning: color!(0xc2850c), // Warm amber
        danger: color!(0xcc3e28),  // Red-orange
    })
}

/// Creates a custom "Paper Dark" theme with a dark, paper-like aesthetic.
pub fn paper_dark() -> Theme {
    Theme::custom("Paper Dark", Seed {
        background: color!(0x1f1e1a), // Warm dark background
        text: color!(0xd4c8b0),       // Warm muted paper color
        primary: color!(0xe8dcc0),    // Warm light paper color
        success: color!(0x216609),    // Green
        warning: color!(0xc2850c),    // Warm amber
        danger: color!(0xcc3e28),     // Red-orange
    })
}

/// Creates a custom "Hyozu" theme inspired by icy mountains and cherry blossoms.
pub fn hyozu() -> Theme {
    Theme::custom("Hyozu", Seed {
        background: color!(0xf5f0e6), // Vanilla cream
        text: color!(0x5b798a),       // Slate grey
        primary: color!(0xf5b1a4),    // Powder blush
        success: color!(0x96b0b2),    // Cool steel
        warning: color!(0xdba7b0),    // Soft blossom
        danger: color!(0x87172d),     // Burgundy
    })
}

/// Creates a custom "Hyozu Dark" theme with a dark, icy aesthetic.
pub fn hyozu_dark() -> Theme {
    Theme::custom("Hyozu Dark", Seed {
        background: color!(0x1e252d), // Deep slate
        text: color!(0xf3ebd5),       // Vanilla cream
        primary: color!(0xf5b1a4),    // Powder blush
        success: color!(0x96b0b2),    // Cool steel
        warning: color!(0xdba7b0),    // Soft blossom
        danger: color!(0x87172d),     // Burgundy
    })
}

/// Creates a custom "Basic Light" theme anchored to shadcn/ui's Nova palette:
/// a near-white background with vivid violet primary and saturated
/// green/amber/red semantic accents.
pub fn basic_light() -> Theme {
    Theme::custom("Basic Light", Seed {
        background: color!(0xfafafa), // Nova background (near-white, Neutral 50)
        text: color!(0x0a0a0a),       // Nova foreground (Neutral 950)
        primary: color!(0x7c3aed),    // Nova primary — Violet 600
        success: color!(0x16a34a),    // Green 600
        warning: color!(0xf59e0b),    // Amber 500
        danger: color!(0xdc2626),     // Red 600 (Nova destructive)
    })
}

/// Creates a custom "Basic" theme anchored to shadcn/ui's Nova palette:
/// a near-black background with vivid violet primary and saturated
/// green/amber/red semantic accents. See [`basic_light`] for the light variant.
pub fn basic() -> Theme {
    Theme::custom("Basic", Seed {
        background: color!(0x0a0a0a), // Nova background (near-black, Neutral 950)
        text: color!(0xfafafa),       // Nova foreground (Neutral 50)
        primary: color!(0x8b5cf6),    // Nova primary — Violet 500
        success: color!(0x22c55e),    // Green 500
        warning: color!(0xf59e0b),    // Amber 500
        danger: color!(0xef4444),     // Red 500 (Nova destructive)
    })
}

/// Returns all available themes, including iced's built-in themes and our custom themes.
pub fn all_themes() -> impl Iterator<Item = Theme> {
    [paper(), paper_dark(), hyozu(), hyozu_dark(), basic(), basic_light()]
        .into_iter()
        .chain(Theme::ALL.iter().cloned())
}
