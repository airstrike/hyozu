//! Named color scales for continuous data visualization.
//!
//! Provides both a [`Scale`] enum for ergonomic use and free functions
//! that return raw color stop arrays.
//!
//! # Example
//!
//! ```ignore
//! use hyozu::{choropleth, Scale};
//!
//! // Named scale
//! choropleth([("USA", 25.5)]).scale(Scale::Viridis);
//!
//! // Negation reverses the scale
//! choropleth([("USA", 25.5)]).scale(-Scale::GreenRed);
//!
//! // Custom colors
//! choropleth([("USA", 25.5)]).scale(Scale::custom([Color::BLACK, Color::WHITE]));
//! ```

use crate::core::Color;

// ── Scale enum ──────────────────────────────────────────────────────

/// A named or custom color scale for continuous data visualization.
///
/// Use `-scale` to reverse any scale:
/// ```ignore
/// let reversed = -Scale::GreenRed; // becomes Red → Green
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum Scale {
    // Diverging
    GreenRed,
    RedGreen,
    GreenAmberRed,
    RedAmberGreen,
    BlueRed,
    PurpleOrange,
    BrownTeal,
    PurpleGreen,
    // Sequential
    Blues,
    Greens,
    Reds,
    Purples,
    Oranges,
    PurpleYellow,
    // Multi-stop
    Viridis,
    Plasma,
    Turbo,
    // User-defined
    Custom(Vec<Color>),
}

impl Scale {
    pub const ALL: &[Scale] = &[
        Scale::RedGreen,
        Scale::RedAmberGreen,
        Scale::GreenRed,
        Scale::GreenAmberRed,
        Scale::BlueRed,
        Scale::PurpleOrange,
        Scale::BrownTeal,
        Scale::PurpleGreen,
        Scale::Blues,
        Scale::Greens,
        Scale::Reds,
        Scale::Purples,
        Scale::Oranges,
        Scale::PurpleYellow,
        Scale::Viridis,
        Scale::Plasma,
        Scale::Turbo,
    ];

    /// Creates a custom scale from color stops.
    pub fn custom(stops: impl Into<Vec<Color>>) -> Self {
        Scale::Custom(stops.into())
    }

    /// Returns the color stops for this scale.
    pub fn stops(&self) -> Vec<Color> {
        match self {
            Scale::GreenRed => green_red().into(),
            Scale::RedGreen => red_green().into(),
            Scale::GreenAmberRed => green_amber_red().into(),
            Scale::RedAmberGreen => red_amber_green().into(),
            Scale::BlueRed => blue_red().into(),
            Scale::PurpleOrange => purple_orange().into(),
            Scale::BrownTeal => brown_teal().into(),
            Scale::PurpleGreen => purple_green().into(),
            Scale::Blues => blues().into(),
            Scale::Greens => greens().into(),
            Scale::Reds => reds().into(),
            Scale::Purples => purples().into(),
            Scale::Oranges => oranges().into(),
            Scale::PurpleYellow => purple_yellow().into(),
            Scale::Viridis => viridis().into(),
            Scale::Plasma => plasma().into(),
            Scale::Turbo => turbo().into(),
            Scale::Custom(stops) => stops.clone(),
        }
    }

    /// Returns a reversed copy of this scale.
    pub fn reversed(self) -> Self {
        let mut stops = self.stops();
        stops.reverse();
        Scale::Custom(stops)
    }
}

impl std::ops::Neg for Scale {
    type Output = Self;
    fn neg(self) -> Self {
        self.reversed()
    }
}

impl std::fmt::Display for Scale {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Scale::GreenRed => "Green → Red",
            Scale::RedGreen => "Red → Green",
            Scale::GreenAmberRed => "Green → Amber → Red",
            Scale::RedAmberGreen => "Red → Amber → Green",
            Scale::BlueRed => "Blue → Red",
            Scale::PurpleOrange => "Purple → Orange",
            Scale::BrownTeal => "Brown → Teal",
            Scale::PurpleGreen => "Purple → Green",
            Scale::Blues => "Blues",
            Scale::Greens => "Greens",
            Scale::Reds => "Reds",
            Scale::Purples => "Purples",
            Scale::Oranges => "Oranges",
            Scale::PurpleYellow => "Purple → Yellow",
            Scale::Viridis => "Viridis",
            Scale::Plasma => "Plasma",
            Scale::Turbo => "Turbo",
            Scale::Custom(_) => "Custom",
        })
    }
}

// ── Helpers ──────────────────────────────────────────────────────────

/// Converts a `0xRRGGBB` hex literal to an iced Color.
const fn hex(rgb: u32) -> Color {
    Color {
        r: ((rgb >> 16) & 0xFF) as f32 / 255.0,
        g: ((rgb >> 8) & 0xFF) as f32 / 255.0,
        b: (rgb & 0xFF) as f32 / 255.0,
        a: 1.0,
    }
}

// ── Diverging scales (3-stop: low → mid → high) ─────────────────────

/// Green → White → Red. Low=good, high=bad (profit/loss, success rate).
pub fn green_red() -> [Color; 3] {
    [hex(0x1a9850), hex(0xffffff), hex(0xd73027)]
}

/// Red → White → Green. Low=bad, high=good (GDP, income, scores).
pub fn red_green() -> [Color; 3] {
    [hex(0xd73027), hex(0xffffff), hex(0x1a9850)]
}

/// Green → Amber → Red. Traffic-light scale (good → warning → bad).
pub fn green_amber_red() -> [Color; 3] {
    [hex(0x2d9e2c), hex(0xf0c040), hex(0xd73027)]
}

/// Red → Amber → Green. Inverted traffic-light (bad → warning → good).
pub fn red_amber_green() -> [Color; 3] {
    [hex(0xd73027), hex(0xf0c040), hex(0x2d9e2c)]
}

/// Blue → White → Red. Temperature, political.
pub fn blue_red() -> [Color; 3] {
    [hex(0x2166ac), hex(0xf7f7f7), hex(0xb2182b)]
}

/// Purple → White → Orange. Colorblind-friendly diverging.
pub fn purple_orange() -> [Color; 3] {
    [hex(0x7b3294), hex(0xf7f7f7), hex(0xe66101)]
}

/// Brown → White → Teal. Earth science.
pub fn brown_teal() -> [Color; 3] {
    [hex(0x8c510a), hex(0xf5f5f5), hex(0x01665e)]
}

/// Purple → White → Yellow-green. Diverging.
pub fn purple_green() -> [Color; 3] {
    [hex(0x762a83), hex(0xf7f7f7), hex(0x1b7837)]
}

// ── Sequential scales (2-stop: light → dark) ────────────────────────

/// Light blue → Dark blue.
pub fn blues() -> [Color; 2] {
    [hex(0xdeebf7), hex(0x08519c)]
}

/// Light green → Dark green.
pub fn greens() -> [Color; 2] {
    [hex(0xe5f5e0), hex(0x238b45)]
}

/// Light red → Dark red.
pub fn reds() -> [Color; 2] {
    [hex(0xfee0d2), hex(0xcb181d)]
}

/// Light purple → Dark purple.
pub fn purples() -> [Color; 2] {
    [hex(0xefedf5), hex(0x6a51a3)]
}

/// Light orange → Dark orange.
pub fn oranges() -> [Color; 2] {
    [hex(0xfee6ce), hex(0xe6550d)]
}

// ── Multi-stop sequential scales ────────────────────────────────────

/// Purple → Yellow. Warm sequential, good contrast on both light/dark.
pub fn purple_yellow() -> [Color; 3] {
    [hex(0x542788), hex(0xd8829a), hex(0xf1a340)]
}

/// Viridis — perceptually uniform, colorblind-safe. The gold standard.
pub fn viridis() -> [Color; 5] {
    [
        hex(0x440154),
        hex(0x3b528b),
        hex(0x21918c),
        hex(0x5ec962),
        hex(0xfde725),
    ]
}

/// Plasma — perceptually uniform, vibrant. Purple → Pink → Yellow.
pub fn plasma() -> [Color; 5] {
    [
        hex(0x0d0887),
        hex(0x7e03a8),
        hex(0xcc4778),
        hex(0xf89540),
        hex(0xf0f921),
    ]
}

/// Turbo — improved rainbow. Blue → Cyan → Yellow → Red.
pub fn turbo() -> [Color; 5] {
    [
        hex(0x30123b),
        hex(0x28bbec),
        hex(0xa2fc3c),
        hex(0xfb8022),
        hex(0x7a0403),
    ]
}
