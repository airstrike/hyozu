//! Grammar-of-graphics style encodings: bind each point of a series to a
//! visual channel (color, in v1) via a closure and a scale rule.
//!
//! An `Encoding<C>` is built from a key-extraction closure via [`key`] and
//! refined with channel-specific methods like [`Encoding::manual`],
//! [`Encoding::manual_with`], and [`Encoding::range`]. In v1 only
//! [`channel::Fill`] exists as a channel marker.
//!
//! # Example
//!
//! ```
//! use hyozu::encoding;
//!
//! // Six bars, each keyed by month — encoded into distinct palette slots.
//! const MONTHS: [&str; 6] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun"];
//! let enc = encoding::key(|i, _| MONTHS[i]);
//! ```

use std::marker::PhantomData;
use std::sync::Arc;

use crate::color::Color;
use crate::data::Datum;
use crate::palette::{Palette, PaletteSeed, Resolved};

pub mod channel;

use channel::{Channel, Fill};

/// Shared extractor closure: maps `(point_index, datum)` to a string key.
type Extractor = Arc<dyn Fn(usize, &Datum) -> String + Send + Sync>;

/// Shared manual-lookup closure: maps a string key to an optional color.
type Lookup = Arc<dyn Fn(&str) -> Option<Color> + Send + Sync>;

/// A binding from each point of a series to a visual value on a channel `C`.
///
/// Build with [`key`]; refine with channel-specific methods like
/// [`Encoding::manual`], [`Encoding::manual_with`], or [`Encoding::range`]
/// (all available on `Encoding<channel::Fill>`).
pub struct Encoding<C: Channel> {
    extractor: Extractor,
    kind: Kind,
    _channel: PhantomData<C>,
}

/// Private: how the encoding's extracted key maps to the channel value.
#[derive(Clone)]
enum Kind {
    /// Assign successive palette slots to keys in order of first appearance.
    /// `range = None` means "build a categorical palette from the design seed
    /// at draw time, sized to the number of distinct keys."
    Ordinal { range: Option<Vec<Color>> },
    /// Explicit lookup table. Missing keys fall through the resolution chain.
    Manual { mapping: Vec<(String, Color)> },
    /// Closure-based lookup. `None` from the closure also falls through.
    ManualWith { lookup: Lookup },
}

/// Build a fill-channel encoding from a key-extraction closure.
///
/// In v1 this returns a concrete `Encoding<channel::Fill>`. When additional
/// channels arrive, this becomes `fill_by` plus siblings (`size_by`, etc.) —
/// not a generic — to avoid a type-inference gotcha on bare `let` bindings.
///
/// The closure receives the point index and the raw [`Datum`]. The returned
/// value is any `Display` type — strings, integers, booleans, or custom enums
/// implementing `Display` — and is stamped onto the datum as its key.
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
    Encoding {
        extractor: Arc::new(move |i, d| f(i, d).to_string()),
        kind: Kind::Ordinal { range: None },
        _channel: PhantomData,
    }
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
// Required so bar::props::Property can keep deriving PartialEq when a future
// Property::ColorBy variant arrives.
impl<C: Channel> PartialEq for Encoding<C> {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.extractor, &other.extractor) && self.kind == other.kind
    }
}

impl PartialEq for Kind {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Kind::Ordinal { range: a }, Kind::Ordinal { range: b }) => a == b,
            (Kind::Manual { mapping: a }, Kind::Manual { mapping: b }) => a == b,
            (Kind::ManualWith { lookup: a }, Kind::ManualWith { lookup: b }) => Arc::ptr_eq(a, b),
            _ => false,
        }
    }
}

impl std::fmt::Debug for Kind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Kind::Ordinal { range } => f
                .debug_struct("Ordinal")
                .field("range_len", &range.as_ref().map(|p| p.len()))
                .finish(),
            Kind::Manual { mapping } => f.debug_struct("Manual").field("entries", &mapping.len()).finish(),
            Kind::ManualWith { .. } => f.debug_struct("ManualWith").finish_non_exhaustive(),
        }
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
    ///     .range([0x355070u32, 0x6D597Au32, 0xB56576u32]);
    /// ```
    pub fn range<Col>(mut self, range: impl IntoIterator<Item = Col>) -> Self
    where
        Col: Into<Color>,
    {
        self.kind = Kind::Ordinal {
            range: Some(range.into_iter().map(Into::into).collect()),
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
    ///     .manual([("above", 0x1A6DAAu32), ("below", 0x93C5E8u32)]);
    /// ```
    pub fn manual<K, Col>(mut self, mapping: impl IntoIterator<Item = (K, Col)>) -> Self
    where
        K: Into<String>,
        Col: Into<Color>,
    {
        self.kind = Kind::Manual {
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
        self.kind = Kind::ManualWith {
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
    /// `seed` is the design's palette seed, used to build a fresh categorical
    /// palette sized to the number of distinct keys when the encoding has no
    /// explicit `range`. The encoding deliberately does NOT share the chart's
    /// main resolved palette; single-series bar charts default to
    /// `Palette::Sequential` of length 1, which would collapse every bar to a
    /// single color.
    pub(crate) fn resolve_fill(&self, points: &[Datum], seed: &PaletteSeed) -> Vec<Option<Color>> {
        let keys: Vec<String> = points.iter().enumerate().map(|(i, d)| (self.extractor)(i, d)).collect();

        match &self.kind {
            Kind::Ordinal { range } => {
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

                // Pick a palette: explicit `range`, or build a fresh
                // categorical palette from the seed sized to the distinct-key
                // count. The encoding does NOT share the chart's potentially-
                // Sequential palette.
                let palette_colors: Vec<Color> = if let Some(explicit) = range {
                    explicit.clone()
                } else {
                    let n = seen.len().max(1);
                    Resolved::resolve(&Palette::Categorical, seed, n).colors().to_vec()
                };

                if palette_colors.is_empty() {
                    return vec![None; keys.len()];
                }
                key_indices
                    .iter()
                    .map(|&i| Some(palette_colors[i % palette_colors.len()]))
                    .collect()
            }
            Kind::Manual { mapping } => keys
                .iter()
                .map(|k| mapping.iter().find(|(mk, _)| mk == k).map(|(_, c)| *c))
                .collect(),
            Kind::ManualWith { lookup } => keys.iter().map(|k| lookup(k)).collect(),
        }
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

    // --- Ordinal scale -------------------------------------------------------

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
        let got = enc.resolve_fill(&pts, &test_seed());

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
        let got = enc.resolve_fill(&pts, &test_seed());

        assert_eq!(got, vec![Some(c0), Some(c1), Some(c0), Some(c1), Some(c0)]);
    }

    #[test]
    fn ordinal_empty_range_falls_through() {
        // `.range(empty)` is the pathological case: the resolver's empty-range
        // guard kicks in and every point returns `None` so the caller falls
        // through the priority chain.
        let enc = key(|i, _| format!("{}", i)).range(std::iter::empty::<u32>());

        let pts = points(&[1.0, 2.0, 3.0]);
        let got = enc.resolve_fill(&pts, &test_seed());

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
        let got = enc.resolve_fill(&pts, &test_seed());

        assert_eq!(got.len(), 2);
        let c0 = got[0].expect("seed-derived palette should produce a color for key 'a'");
        let c1 = got[1].expect("seed-derived palette should produce a color for key 'b'");
        assert_ne!(
            c0, c1,
            "seed-derived categorical palette must produce distinct colors for distinct keys"
        );
    }

    // --- Manual lookup -------------------------------------------------------

    #[test]
    fn manual_returns_mapped_color_for_each_hit() {
        let labels = ["a", "b", "a"];
        let c0 = Color::from_rgb8(10, 10, 10);
        let c1 = Color::from_rgb8(20, 20, 20);
        let enc = key(move |i, _| labels[i]).manual([("a", c0), ("b", c1)]);

        let pts = points(&[1.0, 2.0, 3.0]);
        let got = enc.resolve_fill(&pts, &test_seed());

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
        let got = enc.resolve_fill(&pts, &test_seed());

        assert_eq!(got, vec![Some(c0), None]);
    }

    // --- manual_with closure -------------------------------------------------

    #[test]
    fn manual_with_returns_some_from_closure() {
        let labels = ["even", "odd", "even"];
        let c0 = Color::from_rgb8(10, 10, 10);
        let enc = key(move |i, _| labels[i]).manual_with(move |k| if k == "even" { Some(c0) } else { None });

        let pts = points(&[1.0, 2.0, 3.0]);
        let got = enc.resolve_fill(&pts, &test_seed());

        assert_eq!(got, vec![Some(c0), None, Some(c0)]);
    }

    #[test]
    fn manual_with_none_always_returns_none_for_every_point() {
        let enc = key(|i, _| format!("{}", i)).manual_with(|_| None);

        let pts = points(&[1.0, 2.0, 3.0]);
        let got = enc.resolve_fill(&pts, &test_seed());

        assert_eq!(got, vec![None, None, None]);
    }

    // --- Edge cases ----------------------------------------------------------

    #[test]
    fn empty_points_vec_returns_empty_vec() {
        // Pathological input: no points. Must not panic; returns an empty Vec.
        let enc = key(|i, _| format!("{}", i));

        let got = enc.resolve_fill(&[], &test_seed());

        assert!(got.is_empty());
    }
}
