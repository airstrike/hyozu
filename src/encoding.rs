//! Grammar-of-graphics style encodings: bind each point of a series to a
//! visual channel via a closure and a scale rule.
//!
//! In v1 there are two channels:
//!
//! - [`channel::Fill`] — categorical color via [`key`] / [`fill_by`] and
//!   the ordinal/manual builder methods.
//! - [`channel::Size`] — continuous pixel diameter via [`size_by`] and a
//!   selectable scale (Sqrt by default — perceptually correct for area).
//!
//! Each channel is a separate `Encoding<C>` instance; the entry points are
//! sibling functions ([`key`]/[`fill_by`] vs [`size_by`]) rather than a
//! generic builder, intentionally — that avoids a type-inference gotcha on
//! bare `let` bindings where rustc can't decide the channel type from the
//! closure alone.
//!
//! # Example — fill (categorical)
//!
//! ```
//! use hyozu::encoding;
//!
//! const MONTHS: [&str; 6] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun"];
//! let enc = encoding::key(|i, _| MONTHS[i]);
//! ```
//!
//! # Example — size (continuous bubble)
//!
//! ```
//! use hyozu::encoding;
//!
//! // Each point's y value drives the bubble's pixel diameter (4–24 px),
//! // with a sqrt scale so area is perceptually proportional to y.
//! let enc = encoding::size_by(|_, d| d.y).range(4.0..=24.0);
//! ```

use std::marker::PhantomData;
use std::ops::RangeInclusive;
use std::sync::Arc;

use crate::color::Color;
use crate::data::Datum;
use crate::palette::{Palette, PaletteSeed, Resolved};

pub mod channel;

use channel::{Channel, Fill, FillKind, OrdinalSource, Size, SizeKind, SizeScale};

/// Type alias for the per-point extractor closure that pulls a channel's
/// input value out of a datum. Generic over the channel input type so the
/// `Fill` channel can extract a `String` key while the `Size` channel
/// extracts an `f64`.
type Extractor<I> = Arc<dyn Fn(usize, &Datum) -> I + Send + Sync>;

/// A binding from each point of a series to a visual value on a channel `C`.
///
/// Build with [`key`] / [`fill_by`] for [`channel::Fill`] or [`size_by`] for
/// [`channel::Size`]; refine with channel-specific methods on
/// `Encoding<Fill>` / `Encoding<Size>`.
pub struct Encoding<C: Channel> {
    extractor: Extractor<C::Input>,
    kind: C::Kind,
    _channel: PhantomData<C>,
}

impl<C: Channel> Clone for Encoding<C> {
    fn clone(&self) -> Self {
        Self {
            extractor: self.extractor.clone(),
            kind: self.kind.clone(),
            _channel: PhantomData,
        }
    }
}

impl<C: Channel> std::fmt::Debug for Encoding<C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Encoding")
            .field("kind", &self.kind)
            .finish_non_exhaustive()
    }
}

// PartialEq via Arc::ptr_eq on the extractor + value-compare on Kind.
// Required so `bar::props::Property` and `xy::props::Property` can derive
// `PartialEq` even when they hold an `Encoding<C>`. The Arc pointer
// comparison is intentionally conservative — two encodings built from the
// "same" closure literal aren't equal because each builder call wraps a
// fresh allocation.
impl<C: Channel> PartialEq for Encoding<C> {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.extractor, &other.extractor) && self.kind == other.kind
    }
}

// ============================================================================
// Fill channel — entry point + builder methods
// ============================================================================

/// Build a fill-channel encoding from a key-extraction closure.
///
/// The closure receives the point index and the raw [`Datum`]. The returned
/// value is any `Display` type — strings, integers, booleans, or custom enums
/// implementing `Display` — and is stamped onto the datum as its key.
///
/// Alias of [`fill_by`]; kept for backwards compatibility.
///
/// # Example
///
/// ```
/// use hyozu::encoding;
///
/// let enc = encoding::key(|i, _| if i % 2 == 0 { "even" } else { "odd" });
/// ```
pub fn key<F, S>(f: F) -> Encoding<Fill>
where
    F: Fn(usize, &Datum) -> S + Send + Sync + 'static,
    S: std::fmt::Display,
{
    fill_by(f)
}

/// Build a fill-channel encoding from a key-extraction closure.
///
/// Sibling of [`size_by`]; both names exist so call sites read as
/// `encoding::fill_by(...)` / `encoding::size_by(...)` when both channels
/// are in play.
pub fn fill_by<F, S>(f: F) -> Encoding<Fill>
where
    F: Fn(usize, &Datum) -> S + Send + Sync + 'static,
    S: std::fmt::Display,
{
    Encoding {
        extractor: Arc::new(move |i, d| f(i, d).to_string()),
        kind: FillKind::Ordinal {
            // None = "inherit from the chart's Data::palette, or
            // fall back to Categorical". D17.
            source: OrdinalSource::Seed(None),
        },
        _channel: PhantomData,
    }
}

impl Encoding<Fill> {
    /// Override the default seed-derived categorical range with explicit colors.
    /// Keys still map by order of first appearance.
    ///
    /// # Example
    ///
    /// ```
    /// use hyozu::encoding;
    ///
    /// const REGIONS: [&str; 3] = ["North", "South", "East"];
    /// let enc = encoding::key(|i, _| REGIONS[i])
    ///     .range([0x355070, 0x6D597A, 0xB56576]);
    /// ```
    pub fn range<Col>(mut self, range: impl IntoIterator<Item = Col>) -> Self
    where
        Col: Into<Color>,
    {
        self.kind = FillKind::Ordinal {
            source: OrdinalSource::Range(range.into_iter().map(Into::into).collect()),
        };
        self
    }

    /// Use a seed-derived palette of the given flavor for ordinal assignment.
    /// The palette is generated from the design's `PaletteSeed` at draw time,
    /// so it follows the theme.
    ///
    /// Default is [`Palette::Categorical`] (distinct hues). Use
    /// [`Palette::Sequential`] for shades of the theme's primary color, or
    /// [`Palette::Gradient`] with explicit stops for a custom interpolated
    /// gradient.
    ///
    /// Mutually exclusive with [`range`](Self::range): calling `.palette(...)`
    /// replaces any prior `.range(...)` configuration, and vice versa.
    ///
    /// # Example
    ///
    /// ```
    /// use hyozu::encoding;
    /// use hyozu::palette::Palette;
    ///
    /// const MONTHS: [&str; 6] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun"];
    /// // Each month gets a shade of the theme's primary color.
    /// let enc = encoding::key(|i, _| MONTHS[i]).palette(Palette::Sequential);
    /// ```
    pub fn palette(mut self, flavor: Palette) -> Self {
        self.kind = FillKind::Ordinal {
            source: OrdinalSource::Seed(Some(flavor)),
        };
        self
    }

    /// Use an explicit key → color lookup. Unmatched keys fall back through
    /// the resolution priority chain (see `GOG.md` § 7).
    ///
    /// # Example
    ///
    /// ```
    /// use hyozu::encoding;
    ///
    /// let enc = encoding::key(|_, d| if d.y >= 100.0 { "above" } else { "below" })
    ///     .manual([("above", 0x1A6DAA), ("below", 0x93C5E8)]);
    /// ```
    pub fn manual<K, Col>(mut self, mapping: impl IntoIterator<Item = (K, Col)>) -> Self
    where
        K: Into<String>,
        Col: Into<Color>,
    {
        self.kind = FillKind::Manual {
            mapping: mapping.into_iter().map(|(k, c)| (k.into(), c.into())).collect(),
        };
        self
    }

    /// Use a closure to look up the color for each extracted key. Returning
    /// `None` falls back through the resolution priority chain. Use this when
    /// the mapping is computed (not a static table).
    ///
    /// # Example
    ///
    /// ```
    /// use hyozu::{Color, encoding};
    ///
    /// let enc = encoding::key(|i, _| format!("{}", i)).manual_with(|k| {
    ///     let n: i32 = k.parse().ok()?;
    ///     if n >= 5 { Some(Color::from_rgb8(26, 109, 170)) } else { None }
    /// });
    /// ```
    pub fn manual_with<F>(mut self, lookup: F) -> Self
    where
        F: Fn(&str) -> Option<Color> + Send + Sync + 'static,
    {
        self.kind = FillKind::ManualWith {
            lookup: Arc::new(lookup),
        };
        self
    }

    /// Resolve this encoding for a set of points.
    ///
    /// Returns one entry per point:
    /// - `Some(color)` — the encoding produced a color
    /// - `None` — the caller should fall back through the next priority step
    ///
    /// `seed` is the design's palette seed, used to build a fresh palette
    /// sized to the number of distinct keys when the encoding has no explicit
    /// `range`. The encoding deliberately does NOT share the chart's main
    /// resolved palette (D15): single-series bar charts default to
    /// `Palette::Sequential` of length 1, which would collapse every bar to a
    /// single color.
    ///
    /// `chart_default` is the user's `Data::palette(...)` setting (if any),
    /// used as the flavor when the encoding itself has no explicit `.palette()`
    /// override. This is the D17 inheritance path: `Data::palette(Sequential)`
    /// flows into encoded series automatically.
    pub(crate) fn resolve_fill(
        &self,
        points: &[Datum],
        seed: &PaletteSeed,
        chart_default: Option<&Palette>,
    ) -> Vec<Option<Color>> {
        let keys: Vec<String> = points.iter().enumerate().map(|(i, d)| (self.extractor)(i, d)).collect();

        match &self.kind {
            FillKind::Ordinal { source } => {
                // Compute distinct-key insertion order once.
                let mut seen: Vec<String> = Vec::new();
                let key_indices: Vec<usize> = keys
                    .iter()
                    .map(|k| {
                        if let Some(idx) = seen.iter().position(|s| s == k) {
                            idx
                        } else {
                            seen.push(k.clone());
                            seen.len() - 1
                        }
                    })
                    .collect();

                // Pick a palette: explicit `Range`, or build a fresh palette
                // from the seed at the requested flavor sized to the
                // distinct-key count. The encoding does NOT share the chart's
                // potentially-Sequential top-level palette — D15. Flavor
                // resolution: explicit `.palette(p)` on the encoding wins;
                // else inherit from `chart_default` (user's `Data::palette`);
                // else fall back to `Palette::Categorical`. D17.
                let palette_colors: Vec<Color> = match source {
                    OrdinalSource::Range(explicit) => explicit.clone(),
                    OrdinalSource::Seed(override_flavor) => {
                        let flavor: Palette = override_flavor
                            .clone()
                            .or_else(|| chart_default.cloned())
                            .unwrap_or(Palette::Categorical);
                        let n = seen.len().max(1);
                        Resolved::resolve(&flavor, seed, n).colors().to_vec()
                    }
                };

                if palette_colors.is_empty() {
                    return vec![None; keys.len()];
                }
                key_indices
                    .iter()
                    .map(|&i| Some(palette_colors[i % palette_colors.len()]))
                    .collect()
            }
            FillKind::Manual { mapping } => keys
                .iter()
                .map(|k| mapping.iter().find(|(mk, _)| mk == k).map(|(_, c)| *c))
                .collect(),
            FillKind::ManualWith { lookup } => keys.iter().map(|k| lookup(k)).collect(),
        }
    }
}

// ============================================================================
// Size channel — entry point + builder methods
// ============================================================================

/// Build a size-channel encoding from a numeric extractor closure.
///
/// The closure receives the point index and the raw [`Datum`] and returns a
/// numeric value. By default the encoding uses an auto-detected domain
/// (min/max across the data), a 2.0–20.0 px output range, and a Sqrt scale
/// (perceptually correct for bubble area).
///
/// # Example
///
/// ```
/// use hyozu::encoding;
///
/// // Each point's y value drives the bubble's pixel diameter, with the
/// // default sqrt scale so a point with twice the y produces twice the area.
/// let enc = encoding::size_by(|_, d| d.y).range(4.0..=24.0);
/// ```
pub fn size_by<F>(f: F) -> Encoding<Size>
where
    F: Fn(usize, &Datum) -> f64 + Send + Sync + 'static,
{
    Encoding {
        extractor: Arc::new(f),
        kind: SizeKind::Continuous {
            domain: None,
            range: 2.0..=20.0,
            scale: SizeScale::Sqrt,
        },
        _channel: PhantomData,
    }
}

impl Encoding<Size> {
    /// Set the input domain explicitly. By default the domain is auto-detected
    /// from the data (min/max across points) at resolve time.
    pub fn domain(mut self, domain: RangeInclusive<f64>) -> Self {
        let SizeKind::Continuous { domain: d, .. } = &mut self.kind;
        *d = Some(domain);
        self
    }

    /// Set the output pixel range (diameter, not area).
    pub fn range(mut self, range: RangeInclusive<f32>) -> Self {
        let SizeKind::Continuous { range: r, .. } = &mut self.kind;
        *r = range;
        self
    }

    /// Set the scale function explicitly.
    pub fn scale(mut self, scale: SizeScale) -> Self {
        let SizeKind::Continuous { scale: s, .. } = &mut self.kind;
        *s = scale;
        self
    }

    /// Linear scale: diameter is proportional to the input value. Use this
    /// when the input is already an area (e.g. land km²) and you want the
    /// diameter to scale linearly with it.
    pub fn linear(self) -> Self {
        self.scale(SizeScale::Linear)
    }

    /// Sqrt scale (the default): diameter is proportional to sqrt(value), so
    /// bubble *area* is perceptually proportional to the value. This is the
    /// right choice when the user thinks of the encoded value as "size of
    /// the thing" (population, sales, count).
    pub fn sqrt(self) -> Self {
        self.scale(SizeScale::Sqrt)
    }

    /// Log scale: diameter is proportional to log(value). Useful for highly
    /// skewed distributions where a few outliers would otherwise dominate.
    pub fn log(self) -> Self {
        self.scale(SizeScale::Log)
    }

    /// Resolve this encoding for a set of points.
    ///
    /// Returns one entry per point:
    /// - `Some(diameter_px)` — the encoding produced a pixel diameter
    /// - `None` — the input was non-finite; caller falls back to the marker's
    ///   configured size
    ///
    /// When `domain` is `None`, the min/max of the extracted values is used
    /// as the domain. A degenerate domain (min == max) maps every point to
    /// the midpoint of the output range.
    pub(crate) fn resolve_size(&self, points: &[Datum]) -> Vec<Option<f32>> {
        let inputs: Vec<f64> = points.iter().enumerate().map(|(i, d)| (self.extractor)(i, d)).collect();

        let SizeKind::Continuous { domain, range, scale } = &self.kind;

        // Effective domain: explicit, or auto from the inputs.
        let (d_min, d_max) = if let Some(d) = domain {
            (*d.start(), *d.end())
        } else {
            let mut min = f64::INFINITY;
            let mut max = f64::NEG_INFINITY;
            for &v in &inputs {
                if v.is_finite() {
                    if v < min {
                        min = v;
                    }
                    if v > max {
                        max = v;
                    }
                }
            }
            if min.is_infinite() || max.is_infinite() {
                // No finite inputs at all — every point falls back.
                return vec![None; inputs.len()];
            }
            (min, max)
        };

        let r_min = *range.start();
        let r_max = *range.end();
        let mid = (r_min + r_max) / 2.0;

        inputs
            .into_iter()
            .map(|v| {
                if !v.is_finite() {
                    return None;
                }
                if d_max == d_min {
                    return Some(mid);
                }
                let t = match scale {
                    SizeScale::Linear => (v - d_min) / (d_max - d_min),
                    SizeScale::Sqrt => {
                        // Stevens' law: linearly interpolate sqrt(value) so
                        // that the resulting diameter, when squared into an
                        // area, is linear in the original value. Inputs are
                        // clamped to >= 0 because sqrt of a negative is NaN.
                        let v_s = v.max(0.0).sqrt();
                        let lo_s = d_min.max(0.0).sqrt();
                        let hi_s = d_max.max(0.0).sqrt();
                        if hi_s == lo_s {
                            0.5
                        } else {
                            (v_s - lo_s) / (hi_s - lo_s)
                        }
                    }
                    SizeScale::Log => {
                        // Log requires positive inputs; clamp to MIN_POSITIVE
                        // so a 0 or negative input maps to the bottom of the
                        // range rather than producing NaN.
                        let v_l = v.max(f64::MIN_POSITIVE).ln();
                        let lo_l = d_min.max(f64::MIN_POSITIVE).ln();
                        let hi_l = d_max.max(f64::MIN_POSITIVE).ln();
                        if hi_l == lo_l {
                            0.5
                        } else {
                            (v_l - lo_l) / (hi_l - lo_l)
                        }
                    }
                };
                let t_clamped = t.clamp(0.0, 1.0) as f32;
                Some(r_min + (r_max - r_min) * t_clamped)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::palette::PaletteSeed;

    /// Build a stable test seed with distinct base colors so the seed-derived
    /// categorical palette is guaranteed to produce distinguishable slots.
    fn test_seed() -> PaletteSeed {
        PaletteSeed {
            primary: crate::core::Color::from_rgb8(50, 100, 200),
            secondary: crate::core::Color::from_rgb8(200, 50, 100),
            success: crate::core::Color::from_rgb8(100, 200, 50),
            warning: crate::core::Color::from_rgb8(220, 180, 40),
            danger: crate::core::Color::from_rgb8(220, 40, 40),
            background: crate::core::Color::WHITE,
        }
    }

    /// Construct a `Vec<Datum>` of length `ys.len()` with x = index, y as given.
    fn points(ys: &[f64]) -> Vec<Datum> {
        ys.iter().enumerate().map(|(i, &y)| Datum::new(i as f64, y)).collect()
    }

    // Ordinal scale
    #[test]
    fn ordinal_assigns_keys_by_first_appearance() {
        // Keys ["a", "b", "a", "c"] with a two-color explicit range should
        // produce slots [0, 1, 0, 2] — wrapping the third distinct key "c"
        // back to slot 0 is a separate test; here we test distinct-key
        // first-appearance ordering with a range big enough to fit.
        let labels = ["a", "b", "a", "c"];
        let c0 = Color::from_rgb8(10, 10, 10);
        let c1 = Color::from_rgb8(20, 20, 20);
        let c2 = Color::from_rgb8(30, 30, 30);
        let enc = key(move |i, _| labels[i]).range([c0, c1, c2]);

        let pts = points(&[1.0, 2.0, 3.0, 4.0]);
        let got = enc.resolve_fill(&pts, &test_seed(), None);

        assert_eq!(got, vec![Some(c0), Some(c1), Some(c0), Some(c2)]);
    }

    #[test]
    fn ordinal_wraps_when_more_keys_than_range_slots() {
        // Five distinct keys, only two range colors — palette wraps by modulo.
        let labels = ["a", "b", "c", "d", "e"];
        let c0 = Color::from_rgb8(10, 10, 10);
        let c1 = Color::from_rgb8(20, 20, 20);
        let enc = key(move |i, _| labels[i]).range([c0, c1]);

        let pts = points(&[1.0, 2.0, 3.0, 4.0, 5.0]);
        let got = enc.resolve_fill(&pts, &test_seed(), None);

        assert_eq!(got, vec![Some(c0), Some(c1), Some(c0), Some(c1), Some(c0)]);
    }

    #[test]
    fn ordinal_empty_range_falls_through() {
        // `.range(empty)` is the pathological case: the resolver's empty-range
        // guard kicks in and every point returns `None` so the caller falls
        // through the priority chain.
        let enc = key(|i, _| format!("{}", i)).range(std::iter::empty::<u32>());

        let pts = points(&[1.0, 2.0, 3.0]);
        let got = enc.resolve_fill(&pts, &test_seed(), None);

        assert_eq!(got, vec![None, None, None]);
    }

    #[test]
    fn ordinal_seed_palette_produces_distinct_colors_for_two_keys() {
        // D15 verification: with no explicit range, the encoding must build a
        // fresh categorical palette from the seed, not the chart's default
        // Sequential-of-length-1 palette. Two distinct keys must therefore
        // produce two *different* colors. This test fails under the broken
        // round-2 A1 design where all bars would collapse to one color.
        let labels = ["a", "b"];
        let enc = key(move |i, _| labels[i]);

        let pts = points(&[1.0, 2.0]);
        let got = enc.resolve_fill(&pts, &test_seed(), None);

        assert_eq!(got.len(), 2);
        let c0 = got[0].expect("seed-derived palette should produce a color for key 'a'");
        let c1 = got[1].expect("seed-derived palette should produce a color for key 'b'");
        assert_ne!(
            c0, c1,
            "seed-derived categorical palette must produce distinct colors for distinct keys"
        );
    }

    #[test]
    fn ordinal_inherits_chart_palette_when_encoding_has_no_explicit_flavor() {
        // D17: when the encoding itself has no `.palette()` override, it
        // should inherit the chart-level `Data::palette` choice. Here we
        // build an encoding and a "user set Sequential on Data" and verify
        // the encoding produces Sequential shades (not Categorical hues).
        let labels = ["a", "b", "c", "d"];
        let enc = key(move |i, _| labels[i]);

        let pts = points(&[1.0, 2.0, 3.0, 4.0]);
        let inherited = enc.resolve_fill(&pts, &test_seed(), Some(&Palette::Sequential));
        let default = enc.resolve_fill(&pts, &test_seed(), None);

        // Both paths must produce 4 colors and at least two distinct shades,
        // but they should NOT be identical — Sequential and Categorical pick
        // colors differently, and the inheritance path honors Sequential.
        let inherited_colors: Vec<Color> = inherited.into_iter().map(|c| c.unwrap()).collect();
        let default_colors: Vec<Color> = default.into_iter().map(|c| c.unwrap()).collect();
        assert_eq!(inherited_colors.len(), 4);
        assert_eq!(default_colors.len(), 4);
        assert_ne!(
            inherited_colors, default_colors,
            "Sequential-inherited palette should differ from default Categorical"
        );
    }

    #[test]
    fn ordinal_explicit_palette_override_beats_chart_default() {
        // D17 override path: when the encoding explicitly sets `.palette(X)`,
        // it must ignore the chart's `Data::palette` choice. Build an
        // encoding with `.palette(Categorical)` and a chart with Sequential;
        // the encoding should use Categorical, not inherit Sequential.
        let labels = ["a", "b", "c", "d"];
        let explicit = key(move |i, _| labels[i]).palette(Palette::Categorical);
        let inheriting = key(move |i, _| labels[i]);

        let pts = points(&[1.0, 2.0, 3.0, 4.0]);
        let explicit_colors = explicit.resolve_fill(&pts, &test_seed(), Some(&Palette::Sequential));
        let inheriting_colors = inheriting.resolve_fill(&pts, &test_seed(), Some(&Palette::Sequential));

        // The explicit-Categorical encoding must differ from the inheriting
        // (Sequential) encoding — i.e. the explicit .palette() override
        // successfully blocks the chart-level inheritance.
        assert_ne!(
            explicit_colors, inheriting_colors,
            "explicit .palette(Categorical) should override inherited Sequential"
        );
    }

    #[test]
    fn ordinal_palette_sequential_produces_varied_shades() {
        // `.palette(Palette::Sequential)` builds a Sequential palette (shades
        // of the seed's primary color) at draw time. With four distinct keys
        // we expect four slot lookups and not-all-identical colors — guards
        // against the method silently falling back to Categorical or to a
        // broken one-color Sequential.
        let labels = ["a", "b", "c", "d"];
        let enc = key(move |i, _| labels[i]).palette(Palette::Sequential);

        let pts = points(&[1.0, 2.0, 3.0, 4.0]);
        let got = enc.resolve_fill(&pts, &test_seed(), None);

        assert_eq!(got.len(), 4);
        let colors: Vec<Color> = got.into_iter().map(|c| c.expect("sequential slot")).collect();
        let all_same = colors.iter().all(|&c| c == colors[0]);
        assert!(
            !all_same,
            "Sequential palette should produce at least 2 distinct shades"
        );
    }

    #[test]
    fn ordinal_palette_replaces_prior_range_call() {
        // .palette(...) after .range(...) must reset the source to Seed(p),
        // not leave the earlier range in place. Last call wins — mutually
        // exclusive by construction.
        let labels = ["a", "b"];
        let sentinel = Color::from_rgb8(123, 45, 67);
        let enc = key(move |i, _| labels[i])
            .range([sentinel, sentinel])
            .palette(Palette::Sequential);

        let pts = points(&[1.0, 2.0]);
        let got = enc.resolve_fill(&pts, &test_seed(), None);

        // If .palette(...) correctly cleared the range, neither slot is the
        // sentinel color. The chances of Sequential producing (123,45,67) from
        // a (50,100,200) primary are astronomically low.
        assert_ne!(got[0], Some(sentinel));
        assert_ne!(got[1], Some(sentinel));
    }

    #[test]
    fn ordinal_range_replaces_prior_palette_call() {
        // The mirror of the above: .range(...) after .palette(...) must
        // install the explicit colors, not still build from the seed.
        let labels = ["a", "b"];
        let c0 = Color::from_rgb8(10, 20, 30);
        let c1 = Color::from_rgb8(40, 50, 60);
        let enc = key(move |i, _| labels[i]).palette(Palette::Sequential).range([c0, c1]);

        let pts = points(&[1.0, 2.0]);
        let got = enc.resolve_fill(&pts, &test_seed(), None);

        assert_eq!(got, vec![Some(c0), Some(c1)]);
    }

    // Manual lookup
    #[test]
    fn manual_returns_mapped_color_for_each_hit() {
        let labels = ["a", "b", "a"];
        let c0 = Color::from_rgb8(10, 10, 10);
        let c1 = Color::from_rgb8(20, 20, 20);
        let enc = key(move |i, _| labels[i]).manual([("a", c0), ("b", c1)]);

        let pts = points(&[1.0, 2.0, 3.0]);
        let got = enc.resolve_fill(&pts, &test_seed(), None);

        assert_eq!(got, vec![Some(c0), Some(c1), Some(c0)]);
    }

    #[test]
    fn manual_miss_returns_none_for_unmatched_key() {
        // Key "c" has no mapping → resolve_fill returns None for that slot,
        // which the caller falls through on.
        let labels = ["a", "c"];
        let c0 = Color::from_rgb8(10, 10, 10);
        let c1 = Color::from_rgb8(20, 20, 20);
        let enc = key(move |i, _| labels[i]).manual([("a", c0), ("b", c1)]);

        let pts = points(&[1.0, 2.0]);
        let got = enc.resolve_fill(&pts, &test_seed(), None);

        assert_eq!(got, vec![Some(c0), None]);
    }

    #[test]
    fn manual_with_returns_some_from_closure() {
        let labels = ["even", "odd", "even"];
        let c0 = Color::from_rgb8(10, 10, 10);
        let enc = key(move |i, _| labels[i]).manual_with(move |k| if k == "even" { Some(c0) } else { None });

        let pts = points(&[1.0, 2.0, 3.0]);
        let got = enc.resolve_fill(&pts, &test_seed(), None);

        assert_eq!(got, vec![Some(c0), None, Some(c0)]);
    }

    #[test]
    fn manual_with_none_always_returns_none_for_every_point() {
        let enc = key(|i, _| format!("{}", i)).manual_with(|_| None);

        let pts = points(&[1.0, 2.0, 3.0]);
        let got = enc.resolve_fill(&pts, &test_seed(), None);

        assert_eq!(got, vec![None, None, None]);
    }

    #[test]
    fn empty_points_vec_returns_empty_vec() {
        // Pathological input: no points. Must not panic; returns an empty Vec.
        let enc = key(|i, _| format!("{}", i));

        let got = enc.resolve_fill(&[], &test_seed(), None);

        assert!(got.is_empty());
    }

    // ========================================================================
    // Size channel tests
    // ========================================================================

    #[test]
    fn size_linear_maps_endpoints_and_midpoint_correctly() {
        // Linear scale: y=0 → 4px, y=10 → 24px, y=5 → 14px (exact midpoint).
        let enc = size_by(|_, d| d.y).domain(0.0..=10.0).range(4.0..=24.0).linear();
        let pts = points(&[0.0, 5.0, 10.0]);
        let got = enc.resolve_size(&pts);
        assert_eq!(got, vec![Some(4.0), Some(14.0), Some(24.0)]);
    }

    #[test]
    fn size_sqrt_default_scales_diameter_by_sqrt_of_value() {
        // Sqrt scale: a value 4× larger should produce a diameter 2× larger
        // (so its visual area is 4× — perceptually correct). Domain [0, 16],
        // range [0, 16]: sqrt(0)=0 → 0, sqrt(4)=2 → 0.5*16=8 wait — we
        // interpolate sqrt(v) between sqrt(0)=0 and sqrt(16)=4. So v=4
        // gives t = (2-0)/(4-0) = 0.5 → diameter 8. v=16 gives t=1 → 16.
        // Verify those exact values.
        let enc = size_by(|_, d| d.y).domain(0.0..=16.0).range(0.0..=16.0); // default Sqrt
        let pts = points(&[0.0, 4.0, 16.0]);
        let got = enc.resolve_size(&pts);
        assert_eq!(got, vec![Some(0.0), Some(8.0), Some(16.0)]);
    }

    #[test]
    fn size_default_scale_is_sqrt() {
        // size_by() with no explicit .linear()/.sqrt()/.log() must use Sqrt.
        // Build two identical encodings, one with explicit .sqrt() and one
        // without — they should produce the same output.
        let enc_default = size_by(|_, d| d.y).domain(0.0..=100.0).range(2.0..=20.0);
        let enc_explicit = size_by(|_, d| d.y).domain(0.0..=100.0).range(2.0..=20.0).sqrt();
        let pts = points(&[0.0, 25.0, 100.0]);
        assert_eq!(enc_default.resolve_size(&pts), enc_explicit.resolve_size(&pts));
    }

    #[test]
    fn size_auto_domain_uses_min_max_of_inputs() {
        // No explicit domain → infer [min, max] from the data. Linear so the
        // midpoint check is easy. Inputs [10, 20, 30] → domain [10, 30],
        // range [0, 100]: 10→0, 20→50, 30→100.
        let enc = size_by(|_, d| d.y).range(0.0..=100.0).linear();
        let pts = points(&[10.0, 20.0, 30.0]);
        let got = enc.resolve_size(&pts);
        assert_eq!(got, vec![Some(0.0), Some(50.0), Some(100.0)]);
    }

    #[test]
    fn size_degenerate_domain_returns_midpoint() {
        // domain min == max → every point lands on the midpoint of the range.
        let enc = size_by(|_, d| d.y).domain(5.0..=5.0).range(2.0..=20.0).linear();
        let pts = points(&[5.0, 5.0, 5.0]);
        let got = enc.resolve_size(&pts);
        assert_eq!(got, vec![Some(11.0), Some(11.0), Some(11.0)]);
    }

    #[test]
    fn size_input_outside_domain_clamps_to_range() {
        // Values below the explicit domain min clamp to range start; values
        // above the domain max clamp to range end. Guards against runaway
        // bubbles from outliers when the user has already set a fixed
        // domain expecting a known range.
        let enc = size_by(|_, d| d.y).domain(0.0..=10.0).range(2.0..=20.0).linear();
        let pts = points(&[-5.0, 0.0, 10.0, 50.0]);
        let got = enc.resolve_size(&pts);
        assert_eq!(got, vec![Some(2.0), Some(2.0), Some(20.0), Some(20.0)]);
    }

    #[test]
    fn size_non_finite_input_returns_none() {
        // NaN / inf inputs return None so the caller falls back to the
        // marker's configured size.
        let enc = size_by(|_, d| d.y).domain(0.0..=10.0).range(2.0..=20.0);
        let pts = points(&[f64::NAN, 5.0, f64::INFINITY]);
        let got = enc.resolve_size(&pts);
        assert_eq!(got[0], None);
        assert!(got[1].is_some());
        assert_eq!(got[2], None);
    }

    #[test]
    fn size_log_scale_handles_positive_values() {
        // Log scale: domain [1, 100] → range [0, 10]. Value 1 → 0,
        // value 100 → 10, value 10 → 5 (since log(10) = (log(1)+log(100))/2).
        let enc = size_by(|_, d| d.y).domain(1.0..=100.0).range(0.0..=10.0).log();
        let pts = points(&[1.0, 10.0, 100.0]);
        let got = enc.resolve_size(&pts);
        // Allow tiny floating-point slack on the midpoint
        assert_eq!(got[0], Some(0.0));
        assert!(got[1].is_some() && (got[1].unwrap() - 5.0).abs() < 1e-4);
        assert_eq!(got[2], Some(10.0));
    }

    #[test]
    fn size_empty_points_vec_returns_empty_vec() {
        let enc = size_by(|_, d| d.y);
        assert!(enc.resolve_size(&[]).is_empty());
    }

    #[test]
    fn size_all_non_finite_inputs_return_all_none() {
        // Every input non-finite → auto-domain has no valid bounds, so
        // every point returns None. Degenerate-but-valid early exit.
        let enc = size_by(|_, d| d.y);
        let pts = points(&[f64::NAN, f64::NAN]);
        let got = enc.resolve_size(&pts);
        assert_eq!(got, vec![None, None]);
    }
}
