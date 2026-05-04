//! Named color schemes for continuous data visualization.
//!
//! Provides both a [`Scheme`] enum for ergonomic use and free functions
//! that return raw color stop arrays.
//!
//! # Example
//!
//! ```ignore
//! use hyozu::{choropleth, palette};
//!
//! // Named scheme
//! choropleth([("USA", 25.5)]).scale(palette::Scheme::Viridis);
//!
//! // Negation reverses the scheme
//! choropleth([("USA", 25.5)]).scale(-palette::Scheme::GreenRed);
//!
//! // Custom colors
//! choropleth([("USA", 25.5)]).scale(palette::Scheme::custom([Color::BLACK, Color::WHITE]));
//! ```

use crate::core::Color;

/// A named or custom color scheme for continuous data visualization.
///
/// Use `-scheme` to reverse any scheme:
/// ```ignore
/// let reversed = -palette::Scheme::GreenRed; // becomes Red → Green
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum Scheme {
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

impl Scheme {
    pub const ALL: &[Scheme] = &[
        Scheme::RedGreen,
        Scheme::RedAmberGreen,
        Scheme::GreenRed,
        Scheme::GreenAmberRed,
        Scheme::BlueRed,
        Scheme::PurpleOrange,
        Scheme::BrownTeal,
        Scheme::PurpleGreen,
        Scheme::Blues,
        Scheme::Greens,
        Scheme::Reds,
        Scheme::Purples,
        Scheme::Oranges,
        Scheme::PurpleYellow,
        Scheme::Viridis,
        Scheme::Plasma,
        Scheme::Turbo,
    ];

    /// Creates a custom scheme from color stops.
    pub fn custom(stops: impl Into<Vec<Color>>) -> Self {
        Scheme::Custom(stops.into())
    }

    /// Returns the color stops for this scheme.
    pub fn stops(&self) -> Vec<Color> {
        match self {
            Scheme::GreenRed => green_red().into(),
            Scheme::RedGreen => red_green().into(),
            Scheme::GreenAmberRed => green_amber_red().into(),
            Scheme::RedAmberGreen => red_amber_green().into(),
            Scheme::BlueRed => blue_red().into(),
            Scheme::PurpleOrange => purple_orange().into(),
            Scheme::BrownTeal => brown_teal().into(),
            Scheme::PurpleGreen => purple_green().into(),
            Scheme::Blues => blues().into(),
            Scheme::Greens => greens().into(),
            Scheme::Reds => reds().into(),
            Scheme::Purples => purples().into(),
            Scheme::Oranges => oranges().into(),
            Scheme::PurpleYellow => purple_yellow().into(),
            Scheme::Viridis => viridis().into(),
            Scheme::Plasma => plasma().into(),
            Scheme::Turbo => turbo().into(),
            Scheme::Custom(stops) => stops.clone(),
        }
    }

    /// Returns a reversed copy of this scheme.
    pub fn reversed(self) -> Self {
        let mut stops = self.stops();
        stops.reverse();
        Scheme::Custom(stops)
    }
}

impl std::ops::Neg for Scheme {
    type Output = Self;
    fn neg(self) -> Self {
        self.reversed()
    }
}

impl std::fmt::Display for Scheme {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Scheme::GreenRed => "Green → Red",
            Scheme::RedGreen => "Red → Green",
            Scheme::GreenAmberRed => "Green → Amber → Red",
            Scheme::RedAmberGreen => "Red → Amber → Green",
            Scheme::BlueRed => "Blue → Red",
            Scheme::PurpleOrange => "Purple → Orange",
            Scheme::BrownTeal => "Brown → Teal",
            Scheme::PurpleGreen => "Purple → Green",
            Scheme::Blues => "Blues",
            Scheme::Greens => "Greens",
            Scheme::Reds => "Reds",
            Scheme::Purples => "Purples",
            Scheme::Oranges => "Oranges",
            Scheme::PurpleYellow => "Purple → Yellow",
            Scheme::Viridis => "Viridis",
            Scheme::Plasma => "Plasma",
            Scheme::Turbo => "Turbo",
            Scheme::Custom(_) => "Custom",
        })
    }
}

// ── Helpers ──────────────────────────────────────────────────────────

/// Converts a `0xRRGGBB` hex literal to an iced Color.
const fn hex(rgb: u32) -> Color {
    Color {
        r: ((rgb >> 16) & 0xff) as f32 / 255.0,
        g: ((rgb >> 8) & 0xff) as f32 / 255.0,
        b: (rgb & 0xff) as f32 / 255.0,
        a: 1.0,
    }
}

// ── Diverging schemes (3-stop: low → mid → high) ────────────────────

/// Green → White → Red. Low=good, high=bad (profit/loss, success rate).
pub fn green_red() -> [Color; 3] {
    [hex(0x1a9850), hex(0xffffff), hex(0xd73027)]
}

/// Red → White → Green. Low=bad, high=good (GDP, income, scores).
pub fn red_green() -> [Color; 3] {
    [hex(0xd73027), hex(0xffffff), hex(0x1a9850)]
}

/// Green → Amber → Red. Traffic-light scheme (good → warning → bad).
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

// ── Sequential schemes (2-stop: light → dark) ───────────────────────

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

// ── Multi-stop sequential schemes ───────────────────────────────────

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
