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

/// Creates a custom "shadcn" theme, using Nova's neutral base for
/// background/text/primary and the canonical shadcn chart palette for the
/// semantic accent slots.
///
/// shadcn/ui's Nova style is a component preset that layers on top of a base
/// color (here: neutral). Neutral itself only defines one non-gray token
/// (`destructive`), so a straight port produces near-identical pale grays in
/// any categorical palette slot. To give hyozu a usable 5-way categorical
/// palette we pull the three additional distinct hues from shadcn's published
/// chart palette (`--chart-1`..`--chart-5`), which shipped with every v3 base
/// color and is still the canonical "shadcn chart" color set.
pub fn shadcn() -> Theme {
    Theme::custom("shadcn", Seed {
        background: color!(0xffffff), // Nova neutral background
        text: color!(0x0a0a0a),       // Nova neutral foreground
        primary: color!(0x171717),    // Nova neutral primary (Neutral 900)
        success: color!(0x2a9d90),    // chart-2 light — teal
        warning: color!(0xe8c468),    // chart-4 light — gold / amber
        danger: color!(0xe76e50),     // chart-1 light — warm red
    })
}

/// Creates a custom "shadcn Dark" theme, using Nova's neutral base for
/// background/text/primary and the canonical shadcn chart palette for the
/// semantic accent slots. See [`shadcn`] for the rationale on sourcing the
/// non-gray hues from shadcn's chart palette rather than from Nova itself.
pub fn shadcn_dark() -> Theme {
    Theme::custom("shadcn Dark", Seed {
        background: color!(0x0a0a0a), // Nova neutral background
        text: color!(0xfafafa),       // Nova neutral foreground
        primary: color!(0xe5e5e5),    // Nova neutral primary (Neutral 200)
        success: color!(0x2eb88a),    // chart-2 dark — green
        warning: color!(0xe88c30),    // chart-3 dark — orange / amber
        danger: color!(0xe23670),     // chart-5 dark — magenta-red
    })
}

/// Returns all available themes, including iced's built-in themes and our custom themes.
pub fn all_themes() -> impl Iterator<Item = Theme> {
    [paper(), paper_dark(), hyozu(), hyozu_dark(), shadcn(), shadcn_dark()]
        .into_iter()
        .chain(Theme::ALL.iter().cloned())
}
