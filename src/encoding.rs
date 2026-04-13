//! Grammar-of-graphics style encodings: bind each point of a series to a
//! visual channel (color, in v1) via a closure and a scale rule.
//!
//! An `Encoding<C>` is built from a key-extraction closure via [`key`] and
//! refined with channel-specific methods like [`Encoding::manual`],
//! [`Encoding::manual_with`], and [`Encoding::range`]. In v1 only
//! [`channel::Fill`] exists as a channel marker.

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
