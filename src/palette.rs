use crate::color::Color;
use crate::data::mark::Mark;

/// Strategy for selecting data visualization colors.
#[derive(Debug, Clone, PartialEq)]
pub enum Palette {
    /// Distinct hues for categorical data (pie, multi-series).
    Categorical,
    /// Shades of theme's primary color (1-2 series bars, gauge).
    Sequential,
    /// Interpolate between 2+ color stops in OKLch.
    Gradient(Vec<crate::core::Color>),
}

/// Seed colors extracted from a theme, used to generate palettes.
#[derive(Debug, Clone, Copy)]
pub struct PaletteSeed {
    pub primary: crate::core::Color,
    pub secondary: crate::core::Color,
    pub success: crate::core::Color,
    pub warning: crate::core::Color,
    pub danger: crate::core::Color,
    pub background: crate::core::Color,
}

/// A resolved set of colors ready for rendering.
#[derive(Debug, Clone)]
pub struct Resolved {
    colors: Vec<Color>,
}

impl Resolved {
    /// Resolve a palette strategy into concrete colors.
    pub fn resolve(palette: &Palette, seed: &PaletteSeed, n: usize) -> Self {
        let n = n.max(1);
        let colors = match palette {
            Palette::Categorical => generate_categorical(seed, n),
            Palette::Sequential => generate_sequential(seed.primary, seed.background, n),
            Palette::Gradient(stops) => generate_gradient(stops, n),
        };
        Self { colors }
    }

    /// Get the color at the given index, wrapping via modulo.
    pub fn get(&self, index: usize) -> Color {
        self.colors[index % self.colors.len()]
    }

    /// Returns the number of colors.
    pub fn len(&self) -> usize {
        self.colors.len()
    }

    /// Returns true if empty.
    pub fn is_empty(&self) -> bool {
        self.colors.is_empty()
    }

    /// Returns a reference to the underlying colors.
    pub fn colors(&self) -> &[Color] {
        &self.colors
    }
}

impl Palette {
    /// Infer the best default palette for a set of marks.
    pub fn default_for(marks: &[Mark]) -> Self {
        // Special cases for single marks
        if marks.len() == 1 {
            return match &marks[0] {
                Mark::Area(area) => {
                    if area.series.len() >= 3 {
                        Palette::Categorical
                    } else {
                        Palette::Sequential
                    }
                }
                Mark::Pie(_) => Palette::Categorical,
                Mark::Gauge(_) => Palette::Sequential,
                Mark::Waterfall(_) => Palette::Sequential,
                Mark::Bars(bars) => {
                    if bars.series.len() >= 3 {
                        Palette::Categorical
                    } else {
                        Palette::Sequential
                    }
                }
                Mark::BoxPlot(bp) => {
                    if bp.entries.len() >= 3 {
                        Palette::Categorical
                    } else {
                        Palette::Sequential
                    }
                }
                Mark::Treemap(_) => Palette::Categorical,
                Mark::BubbleMap(_) => Palette::Categorical,
                Mark::Line(_) | Mark::Xy(_) => Palette::Sequential,
                Mark::Heatmap(_) => Palette::Sequential,
                Mark::Violin(v) => {
                    if v.entries.len() >= 3 {
                        Palette::Categorical
                    } else {
                        Palette::Sequential
                    }
                }
                Mark::Tick(_) => Palette::Sequential,
                Mark::Rule(_) => Palette::Sequential,
            };
        }

        // Multiple marks: use categorical for distinct series
        Palette::Categorical
    }
}

/// Count how many color slots a set of marks needs.
pub fn count_color_slots(marks: &[Mark]) -> usize {
    marks
        .iter()
        .map(|mark| match mark {
            Mark::Area(area) => area.series.len(),
            Mark::Bars(bars) => bars.series.len(),
            Mark::BoxPlot(bp) => bp.entries.len(),
            Mark::Line(_) => 1,
            Mark::Xy(_) => 1,
            Mark::Pie(pie) => pie.slices.len(),
            Mark::Gauge(_) => 1,
            Mark::Treemap(tm) => tm.items.len(),
            Mark::Waterfall(_) => 3,
            Mark::Heatmap(_) => 0, // heatmap uses its own color_stops
            Mark::BubbleMap(bm) => bm.points.len(),
            Mark::Violin(v) => v.entries.len(),
            Mark::Tick(_) => 0,
            Mark::Rule(_) => 0,
        })
        .sum::<usize>()
        .max(1)
}

// === OKLch color space utilities ===

#[derive(Debug, Clone, Copy)]
pub(crate) struct Oklch {
    pub(crate) l: f32,
    pub(crate) c: f32,
    pub(crate) h: f32,
    pub(crate) a: f32,
}

pub(crate) fn to_oklch(color: crate::core::Color) -> Oklch {
    let [r, g, b, alpha] = color.into_linear();

    // linear RGB -> LMS
    let l = 0.41222146 * r + 0.53633255 * g + 0.051445995 * b;
    let m = 0.2119035 * r + 0.6806995 * g + 0.10739696 * b;
    let s = 0.08830246 * r + 0.28171885 * g + 0.6299787 * b;

    // Nonlinear transform (cube root)
    let l_ = l.cbrt();
    let m_ = m.cbrt();
    let s_ = s.cbrt();

    // LMS -> Oklab
    let l = 0.21045426 * l_ + 0.7936178 * m_ - 0.004072047 * s_;
    let a = 1.9779985 * l_ - 2.4285922 * m_ + 0.4505937 * s_;
    let b = 0.025904037 * l_ + 0.78277177 * m_ - 0.80867577 * s_;

    // Oklab -> Oklch
    let c = (a * a + b * b).sqrt();
    let h = b.atan2(a);

    Oklch { l, c, h, a: alpha }
}

pub(crate) fn from_oklch(oklch: Oklch) -> crate::core::Color {
    let Oklch { l, c, h, a: alpha } = oklch;

    let a = c * h.cos();
    let b = c * h.sin();

    // Oklab -> LMS (nonlinear)
    let l_ = l + 0.39633778 * a + 0.21580376 * b;
    let m_ = l - 0.105561346 * a - 0.06385417 * b;
    let s_ = l - 0.08948418 * a - 1.2914855 * b;

    // Cubing back
    let l = l_ * l_ * l_;
    let m = m_ * m_ * m_;
    let s = s_ * s_ * s_;

    let r = 4.0767417 * l - 3.3077116 * m + 0.23096994 * s;
    let g = -1.268438 * l + 2.6097574 * m - 0.34131938 * s;
    let b = -0.0041960863 * l - 0.7034186 * m + 1.7076147 * s;

    crate::core::Color::from_linear_rgba(r.clamp(0.0, 1.0), g.clamp(0.0, 1.0), b.clamp(0.0, 1.0), alpha)
}

/// Determine if a background is dark (OKLch lightness < 0.5).
fn is_dark_background(bg: crate::core::Color) -> bool {
    to_oklch(bg).l < 0.5
}

/// Shift a color's lightness to distinguish it from the original.
/// Each `pass` (1, 2, ...) shifts further.
/// Light backgrounds → lighter; dark backgrounds → darker.
pub fn shift_lightness(color: crate::core::Color, background: crate::core::Color, pass: usize) -> crate::core::Color {
    let mut oklch = to_oklch(color);
    let shift = 0.12 * pass as f32;
    if is_dark_background(background) {
        oklch.l = (oklch.l + shift).min(0.90);
    } else {
        oklch.l = (oklch.l - shift).max(0.25);
    }
    from_oklch(oklch)
}

// === Color generation algorithms ===

/// Generate categorical colors: distinct hues from theme's semantic colors.
fn generate_categorical(seed: &PaletteSeed, n: usize) -> Vec<Color> {
    // Base pool reordered for max visual separation:
    // primary, secondary, success, warning, danger
    let base_pool = [seed.primary, seed.secondary, seed.success, seed.warning, seed.danger];

    let mut colors = Vec::with_capacity(n);

    for i in 0..n {
        let pool_idx = i % base_pool.len();
        let pass = i / base_pool.len();

        if pass == 0 {
            // First pass: use base colors directly
            colors.push(Color::Fixed(base_pool[pool_idx]));
        } else {
            // Subsequent passes: shift lightness for distinction
            colors.push(Color::Fixed(shift_lightness(
                base_pool[pool_idx],
                seed.background,
                pass,
            )));
        }
    }

    colors
}

/// Generate sequential colors: shades of one hue.
fn generate_sequential(primary: crate::core::Color, background: crate::core::Color, n: usize) -> Vec<Color> {
    if n == 1 {
        return vec![Color::Fixed(primary)];
    }

    let oklch = to_oklch(primary);
    let dark_bg = is_dark_background(background);

    let (l_start, l_end) = if dark_bg {
        (0.80_f32, 0.45_f32)
    } else {
        (0.35_f32, 0.75_f32)
    };

    (0..n)
        .map(|i| {
            let t = i as f32 / (n - 1).max(1) as f32;
            let l = l_start + (l_end - l_start) * t;
            Color::Fixed(from_oklch(Oklch {
                l,
                c: oklch.c,
                h: oklch.h,
                a: oklch.a,
            }))
        })
        .collect()
}

/// Generate gradient colors: interpolate between stops in OKLch.
fn generate_gradient(stops: &[crate::core::Color], n: usize) -> Vec<Color> {
    if stops.is_empty() {
        return vec![Color::Fixed(crate::core::Color::BLACK); n];
    }
    if stops.len() == 1 || n == 1 {
        return vec![Color::Fixed(stops[0]); n];
    }

    let oklch_stops: Vec<Oklch> = stops.iter().map(|c| to_oklch(*c)).collect();

    (0..n)
        .map(|i| {
            let t = i as f32 / (n - 1).max(1) as f32;
            // Map t to segment
            let segment_t = t * (oklch_stops.len() - 1) as f32;
            let seg_idx = (segment_t.floor() as usize).min(oklch_stops.len() - 2);
            let local_t = segment_t - seg_idx as f32;

            let a = &oklch_stops[seg_idx];
            let b = &oklch_stops[seg_idx + 1];

            // Interpolate L and C linearly
            let l = a.l + (b.l - a.l) * local_t;
            let c = a.c + (b.c - a.c) * local_t;

            // Interpolate H via shortest arc
            let h = interpolate_hue(a.h, b.h, local_t);

            // Interpolate alpha
            let alpha = a.a + (b.a - a.a) * local_t;

            Color::Fixed(from_oklch(Oklch { l, c, h, a: alpha }))
        })
        .collect()
}

/// Interpolate between two hue angles (radians) via shortest arc.
fn interpolate_hue(h1: f32, h2: f32, t: f32) -> f32 {
    let mut diff = h2 - h1;

    // Normalize to [-PI, PI] for shortest path
    while diff > std::f32::consts::PI {
        diff -= std::f32::consts::TAU;
    }
    while diff < -std::f32::consts::PI {
        diff += std::f32::consts::TAU;
    }

    h1 + diff * t
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_seed() -> PaletteSeed {
        PaletteSeed {
            primary: crate::core::Color::from_rgb(0.2, 0.4, 0.8),
            secondary: crate::core::Color::from_rgb(0.6, 0.3, 0.7),
            success: crate::core::Color::from_rgb(0.2, 0.7, 0.3),
            warning: crate::core::Color::from_rgb(0.9, 0.7, 0.1),
            danger: crate::core::Color::from_rgb(0.8, 0.2, 0.2),
            background: crate::core::Color::WHITE,
        }
    }

    #[test]
    fn categorical_5_returns_5_distinct() {
        let seed = test_seed();
        let colors = generate_categorical(&seed, 5);
        assert_eq!(colors.len(), 5);
    }

    #[test]
    fn categorical_8_wraps_with_shift() {
        let seed = test_seed();
        let colors = generate_categorical(&seed, 8);
        assert_eq!(colors.len(), 8);
        // Colors 5-7 should differ from 0-2 (shifted lightness)
        for i in 5..8 {
            assert_ne!(colors[i], colors[i - 5]);
        }
    }

    #[test]
    fn sequential_1_returns_primary() {
        let seed = test_seed();
        let resolved = Resolved::resolve(&Palette::Sequential, &seed, 1);
        assert_eq!(resolved.len(), 1);
    }

    #[test]
    fn sequential_5_has_varying_lightness() {
        let seed = test_seed();
        let resolved = Resolved::resolve(&Palette::Sequential, &seed, 5);
        assert_eq!(resolved.len(), 5);
    }

    #[test]
    fn gradient_2_stops_3_samples() {
        let stops = vec![
            crate::core::Color::from_rgb(1.0, 0.0, 0.0),
            crate::core::Color::from_rgb(0.0, 0.0, 1.0),
        ];
        let colors = generate_gradient(&stops, 3);
        assert_eq!(colors.len(), 3);
    }

    #[test]
    fn resolved_wraps_around() {
        let seed = test_seed();
        let resolved = Resolved::resolve(&Palette::Categorical, &seed, 3);
        // Accessing beyond length should wrap
        let c0 = resolved.get(0);
        let c3 = resolved.get(3);
        assert_eq!(c0, c3);
    }

    #[test]
    fn default_for_pie_is_categorical() {
        let pie = crate::pie([1.0, 2.0]);
        let marks = vec![Mark::Pie(pie)];
        assert!(matches!(Palette::default_for(&marks), Palette::Categorical));
    }

    #[test]
    fn default_for_single_bars_is_sequential() {
        let bars = crate::bars([100, 200, 300]);
        let marks = vec![Mark::Bars(bars)];
        assert!(matches!(Palette::default_for(&marks), Palette::Sequential));
    }

    #[test]
    fn count_slots_pie_5() {
        let pie = crate::pie([1.0, 2.0, 3.0, 4.0, 5.0]);
        let marks = vec![Mark::Pie(pie)];
        assert_eq!(count_color_slots(&marks), 5);
    }

    #[test]
    fn oklch_roundtrip() {
        let original = crate::core::Color::from_rgb(0.5, 0.3, 0.8);
        let oklch = to_oklch(original);
        let back = from_oklch(oklch);
        // Allow small floating point error
        assert!((original.r - back.r).abs() < 0.01);
        assert!((original.g - back.g).abs() < 0.01);
        assert!((original.b - back.b).abs() < 0.01);
    }
}
