//! Adaptive color contrast system for chart elements.
//!
//! # Overview
//!
//! This module provides a smart color resolution system that automatically selects
//! appropriate text colors based on their background, ensuring good contrast while
//! respecting the theme's color palette.
//!
//! # How It Works
//!
//! The system operates in three stages:
//!
//! ## 1. Text Pair Creation ([`Pair`])
//!
//! Each theme provides a pair of text color options via [`Design::text_pair()`]:
//! - **on_light**: The theme's default text color
//! - **on_dark**: An HSL-inverted version (preserves hue, flips lightness)
//!
//! This pair is computed once and reused across all chart elements.
//!
//! ## 2. Color Resolution ([`Pair::resolve()`])
//!
//! When placing text on a background (like labels on bars), the system:
//!
//! 1. **Tries the fallback first** (typically the chart background color):
//!    - If it provides good contrast (≥ 2.5) AND is competitive with the pair options,
//!      use it to respect the theme's color palette
//!    - "Competitive" means within 80% of the best pair option, or when the best pair
//!      option is poor (< 3.0)
//!
//! 2. **Falls back to the text pair** if the background doesn't work:
//!    - Choose whichever pair option (on_light or on_dark) has better contrast
//!    - When both are close (within 15%), prefer on_light (the original text color)
//!
//! ## 3. Usage Pattern
//!
//! ```ignore
//! // Resolve bar color against chart background
//! let bar_color = color_spec.resolve(chart_bg, text_pair, None);
//!
//! // Resolve label color against bar, with chart background as fallback
//! let label_color = label_spec.resolve(bar_color, text_pair, Some(chart_bg));
//! ```
//!
//! # Key Functions
//!
//! - [`Pair::resolve()`] - Main contrast resolution logic
//! - [`invert_brightness()`] - HSL-based color inversion for creating text pairs
//! - [`Color::resolve()`] - Higher-level color resolution for [`Color`] enum
//!
//! [`Design::text_pair()`]: crate::design::Design::text_pair

/// A pair of colors for light and dark backgrounds.
///
/// See the [module documentation](self) for details on how pairs are used in
/// the contrast resolution system.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pair {
    /// Color to use on light backgrounds.
    pub on_light: crate::core::Color,
    /// Color to use on dark backgrounds.
    pub on_dark: crate::core::Color,
}

impl Pair {
    /// Create a new color pair.
    pub const fn new(
        on_light: crate::core::Color,
        on_dark: crate::core::Color,
    ) -> Self {
        Self { on_light, on_dark }
    }

    /// Resolve to the appropriate color based on contrast with the background.
    ///
    /// This is the core of the adaptive color system. It selects a color that provides
    /// good contrast while respecting the theme's palette.
    ///
    /// # Arguments
    ///
    /// * `background` - The color this text will be placed on (e.g., a bar color)
    /// * `fallback` - Optional fallback color to prefer (typically the chart background).
    ///   When provided, this color is tried first to maintain theme consistency.
    ///
    /// # Algorithm
    ///
    /// 1. If `fallback` is provided and has good contrast (≥ 2.5) with `background`,
    ///    AND is competitive with the best pair option (within 80%, or when best < 3.0),
    ///    **use the fallback** to respect the theme.
    ///
    /// 2. Otherwise, choose from the pair:
    ///    - Pick whichever option (on_light or on_dark) has better contrast
    ///    - If both are close (within 15%), prefer on_light (original text color)
    ///
    /// # Examples
    ///
    /// ```ignore
    /// // Label outside bar - no fallback needed
    /// let label_color = text_pair.resolve(chart_background, None);
    ///
    /// // Label inside bar - use chart background as fallback
    /// let label_color = text_pair.resolve(bar_color, Some(chart_background));
    /// ```
    pub fn resolve(
        self,
        background: crate::core::Color,
        fallback: Option<crate::core::Color>,
    ) -> crate::core::Color {
        // Calculate contrast for both pair options
        let contrast_light = background.relative_contrast(self.on_light);
        let contrast_dark = background.relative_contrast(self.on_dark);
        let best_pair_contrast = contrast_light.max(contrast_dark);

        // If we have a fallback, prefer it if it's competitive
        if let Some(fallback_color) = fallback {
            let fallback_contrast =
                background.relative_contrast(fallback_color);
            // Use fallback if it's good enough (>= 2.5) AND competitive with the best pair option
            // "Competitive" means within 80% of the best, or if best is poor (<3.0), fallback is decent (>2.5)
            if fallback_contrast >= 2.5
                && (fallback_contrast >= best_pair_contrast * 0.8
                    || best_pair_contrast < 3.0)
            {
                return fallback_color;
            }
        }

        // Use the better pair option
        // But prefer on_light (original text) when both are close (within 15%)
        if contrast_light > contrast_dark {
            self.on_light
        } else if contrast_dark > contrast_light * 1.15 {
            self.on_dark
        } else {
            // They're close - prefer on_light (original text color)
            self.on_light
        }
    }
}

/// A color that can be either fixed or adaptive to its background.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Color {
    /// A fixed color that doesn't change.
    Fixed(crate::core::Color),
    /// An adaptive color that picks between light/dark variants based on
    /// background.  If None, uses the design system's default text pair.
    Contrast(Option<Pair>),
    // Future: ContrastTint(Option<Pair>) - adapts hue to background like Material Design
}

impl Color {
    /// Creates a [`Color`] from its RGB components.
    pub const fn from_rgb(r: f32, g: f32, b: f32) -> Self {
        Color::Fixed(crate::core::Color::from_rgb(r, g, b))
    }

    /// Creates a [`Color`] from its RGBA components.
    pub const fn from_rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Color::Fixed(crate::core::Color::from_rgba(r, g, b, a))
    }

    /// Creates a [`Color`] from its RGB8 components.
    pub const fn from_rgb8(r: u8, g: u8, b: u8) -> Self {
        Color::Fixed(crate::core::Color::from_rgb8(r, g, b))
    }

    /// Creates a [`Color`] from its RGB8 components and an alpha value.
    pub const fn from_rgba8(r: u8, g: u8, b: u8, a: f32) -> Self {
        Color::Fixed(crate::core::Color::from_rgba8(r, g, b, a))
    }

    /// Creates a [`Color`] from its linear RGBA components.
    pub fn from_linear_rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self::Fixed(crate::core::Color::from_linear_rgba(r, g, b, a))
    }

    /// Creates an adaptive color from a custom light/dark pair.
    pub const fn contrast(
        on_light: crate::core::Color,
        on_dark: crate::core::Color,
    ) -> Self {
        Color::Contrast(Some(Pair::new(on_light, on_dark)))
    }

    /// Adaptive contrast using the design system's default text colors.
    pub const CONTRAST: Color = Color::Contrast(None);

    /// The black color.
    pub const BLACK: Color = Color::Fixed(crate::core::Color::BLACK);

    /// The white color.
    pub const WHITE: Color = Color::Fixed(crate::core::Color::WHITE);

    /// A color with no opacity.
    pub const TRANSPARENT: Color =
        Color::Fixed(crate::core::Color::TRANSPARENT);

    /// Resolve this color to a concrete iced Color given a background and design pair.
    /// Optionally provide a fallback color to use if the pair options have poor contrast.
    pub fn resolve(
        self,
        background: crate::core::Color,
        design_pair: Pair,
        fallback: Option<crate::core::Color>,
    ) -> crate::core::Color {
        match self {
            Color::Fixed(c) => c,
            Color::Contrast(Some(pair)) => pair.resolve(background, fallback),
            Color::Contrast(None) => design_pair.resolve(background, fallback),
        }
    }

    // /// Convert to iced Color, using white background for contrast resolution if needed.
    // /// For use when the background is unknown.
    // pub fn to_iced(self, design_pair: impl Into<Pair>) -> crate::core::Color {
    //     self.resolve(crate::core::Color::WHITE, design_pair.into(), None)
    // }

    /// Crisper version of the color. Light colors are darkened, dark colors are lightened.
    /// The effect is very, very subtle. Hue and chroma are preserved using perceptually uniform HCL color space.
    pub fn crisp(self) -> Color {
        match self {
            Color::Fixed(c) => Color::Fixed(adjust_lightness_hcl(c, true, 3.0)),
            Color::Contrast(Some(pair)) => Color::Contrast(Some(Pair::new(
                adjust_lightness_hcl(pair.on_light, true, 3.0),
                adjust_lightness_hcl(pair.on_dark, true, 3.0),
            ))),
            Color::Contrast(None) => Color::Contrast(None),
        }
    }

    /// Faded version of the color. Light colors are lightened, dark colors are darkened.
    /// The effect is very, very subtle. Hue and chroma are preserved using perceptually uniform HCL color space.
    pub fn faded(self) -> Color {
        match self {
            Color::Fixed(c) => {
                Color::Fixed(adjust_lightness_hcl(c, false, 3.0))
            }
            Color::Contrast(Some(pair)) => Color::Contrast(Some(Pair::new(
                adjust_lightness_hcl(pair.on_light, false, 3.0),
                adjust_lightness_hcl(pair.on_dark, false, 3.0),
            ))),
            Color::Contrast(None) => Color::Contrast(None),
        }
    }
}

impl From<crate::core::Color> for Color {
    fn from(c: crate::core::Color) -> Self {
        Color::Fixed(c)
    }
}

impl From<Pair> for Color {
    fn from(pair: Pair) -> Self {
        Color::Contrast(Some(pair))
    }
}

impl From<u32> for Color {
    fn from(hex: u32) -> Self {
        let r = ((hex & 0xff0000) >> 16) as u8;
        let g = ((hex & 0x00ff00) >> 8) as u8;
        let b = (hex & 0x0000ff) as u8;
        Color::Fixed(crate::core::Color::from_rgb8(r, g, b))
    }
}

pub fn crisp(color: crate::core::Color) -> crate::core::Color {
    adjust_lightness_hcl(color, true, 10.0)
}

pub fn faded(color: crate::core::Color) -> crate::core::Color {
    adjust_lightness_hcl(color, false, 3.0)
}

/// Adjust color lightness in HCL color space while preserving hue and chroma.
/// If `crisp` is true, darkens light colors and lightens dark colors (increases contrast).
/// If `crisp` is false, lightens light colors and darkens dark colors (decreases contrast).
///
/// # Arguments
/// * `color` - The color to adjust
/// * `crisp` - True to increase contrast, false to decrease contrast
/// * `intensity` - How much to adjust lightness (on 0-100 scale). Typical values: 1-10
fn adjust_lightness_hcl(
    color: crate::core::Color,
    crisp: bool,
    intensity: f32,
) -> crate::core::Color {
    // Convert RGB -> Linear RGB -> XYZ -> LAB -> LCH (HCL)
    let linear = color.into_linear();

    // Linear RGB to XYZ (D65 illuminant)
    let x =
        0.4124564 * linear[0] + 0.3575761 * linear[1] + 0.1804375 * linear[2];
    let y =
        0.2126729 * linear[0] + 0.7151522 * linear[1] + 0.0721750 * linear[2];
    let z =
        0.0193339 * linear[0] + 0.119_192 * linear[1] + 0.9503041 * linear[2];

    // XYZ to LAB (D65 white point: 0.95047, 1.0, 1.08883)
    let f = |t: f32| {
        if t > 0.008856 {
            t.powf(1.0 / 3.0)
        } else {
            7.787 * t + 16.0 / 116.0
        }
    };

    let fx = f(x / 0.95047);
    let fy = f(y / 1.0);
    let fz = f(z / 1.08883);

    let lab_l = 116.0 * fy - 16.0;
    let lab_a = 500.0 * (fx - fy);
    let lab_b = 200.0 * (fy - fz);

    // LAB to LCH
    let lch_l = lab_l;
    let lch_c = (lab_a * lab_a + lab_b * lab_b).sqrt();
    let lch_h = lab_b.atan2(lab_a);

    // Adjust lightness (L ranges 0-100)
    let new_l = if crisp {
        if lch_l < 50.0 {
            (lch_l + intensity).min(100.0)
        } else {
            (lch_l - intensity).max(0.0)
        }
    } else {
        if lch_l < 50.0 {
            (lch_l - intensity).max(0.0)
        } else {
            (lch_l + intensity).min(100.0)
        }
    };

    // LCH back to LAB
    let new_lab_a = lch_c * lch_h.cos();
    let new_lab_b = lch_c * lch_h.sin();

    // LAB to XYZ
    let finv = |t: f32| {
        if t > 0.206893 {
            t.powi(3)
        } else {
            (t - 16.0 / 116.0) / 7.787
        }
    };

    let fy2 = (new_l + 16.0) / 116.0;
    let fx2 = new_lab_a / 500.0 + fy2;
    let fz2 = fy2 - new_lab_b / 200.0;

    let x2 = finv(fx2) * 0.95047;
    let y2 = finv(fy2) * 1.0;
    let z2 = finv(fz2) * 1.08883;

    // XYZ to Linear RGB
    let lr = 3.2404542 * x2 - 1.5371385 * y2 - 0.4985314 * z2;
    let lg = -0.969_266 * x2 + 1.8760108 * y2 + 0.0415560 * z2;
    let lb = 0.0556434 * x2 - 0.2040259 * y2 + 1.0572252 * z2;

    // Linear RGB to sRGB (gamma correction)
    crate::core::Color::from_linear_rgba(
        lr.clamp(0.0, 1.0),
        lg.clamp(0.0, 1.0),
        lb.clamp(0.0, 1.0),
        color.a,
    )
}

/// Invert the brightness/lightness of a color while preserving hue and saturation.
///
/// This function creates an alternative text color option by converting to HSL
/// color space and inverting the lightness component (L' = 1.0 - L).
///
/// # Why HSL Inversion?
///
/// Unlike simple RGB inversion which can produce odd colors, HSL inversion:
/// - **Preserves the hue** - A blue color inverts to a different shade of blue
/// - **Maintains saturation** - Colorfulness is retained
/// - **Flips brightness** - Light colors become dark, dark colors become light
///
/// This produces a more theme-consistent alternative than pure black/white.
///
/// # Examples
///
/// ```ignore
/// let light_blue = Color::from_rgb(0.7, 0.8, 0.9);  // Light blue
/// let dark_blue = invert_brightness(light_blue);     // Dark blue (same hue)
/// ```
///
/// # Note
///
/// The resulting color may not always provide perfect contrast with all backgrounds.
/// Use [`Pair::resolve()`] to automatically select the best option based on actual contrast.
pub const fn invert_brightness(
    color: crate::core::Color,
) -> crate::core::Color {
    let (h, s, l) = rgb_to_hsl(color.r, color.g, color.b);

    // Invert lightness
    let inverted_l = 1.0 - l;
    hsl_to_rgb(h, s, inverted_l, color.a)
}

/// Convert RGB to HSL color space.
const fn rgb_to_hsl(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;

    // Lightness
    let l = (max + min) / 2.0;

    // Saturation
    let s = if delta < 0.0001 {
        0.0
    } else if l < 0.5 {
        delta / (max + min)
    } else {
        delta / (2.0 - max - min)
    };

    // Hue
    let h = if delta < 0.0001 {
        0.0
    } else if (max - r).abs() < 0.0001 {
        ((g - b) / delta + if g < b { 6.0 } else { 0.0 }) / 6.0
    } else if (max - g).abs() < 0.0001 {
        ((b - r) / delta + 2.0) / 6.0
    } else {
        ((r - g) / delta + 4.0) / 6.0
    };

    (h, s, l)
}

/// Convert HSL to RGB color space.
const fn hsl_to_rgb(h: f32, s: f32, l: f32, a: f32) -> crate::core::Color {
    if s < 0.0001 {
        // Grayscale
        return crate::core::Color::from_rgba(l, l, l, a);
    }

    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - ((h * 6.0) % 2.0 - 1.0).abs());
    let m = l - c / 2.0;

    let (r, g, b) = match (h * 6.0) as i32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };

    crate::core::Color::from_rgba(r + m, g + m, b + m, a)
}
