//! Visual channels that an [`Encoding`](super::Encoding) can target.
//!
//! A `Channel` declares three things:
//! - `Input` — the per-point value the user's extractor closure produces.
//! - `Output` — what the encoding resolves each point to (e.g. a color, a
//!   pixel diameter).
//! - `Kind` — the channel-specific configuration enum stored inside the
//!   `Encoding`. Each channel has a different configuration story (Fill is
//!   keyed/categorical; Size is continuous), so the configuration type lives
//!   on the channel rather than being baked into a single `Kind` enum.
//!
//! Channel markers are sealed: only crate-defined markers may implement
//! `Channel`. They're also intentionally only reachable as `channel::Fill` /
//! `channel::Size` so they never collide with `iced_widget::canvas::Fill`
//! inside the renderer.

use std::ops::RangeInclusive;
use std::sync::Arc;

use crate::color::Color;
use crate::palette::Palette;

mod sealed {
    pub trait Sealed {}
}

/// A visual channel an encoding can target.
///
/// Sealed: only crate-defined markers may implement it.
pub trait Channel: sealed::Sealed {
    /// What the extractor closure returns per point.
    type Input;
    /// What resolution produces per point.
    type Output;
    /// Channel-specific configuration enum stored in `Encoding<C>`.
    type Kind: Clone + PartialEq + std::fmt::Debug + Send + Sync + 'static;
}

// ============================================================================
// Fill channel — categorical: maps a string key to a color via ordinal /
// manual / closure-based lookup.
// ============================================================================

/// The fill (color) channel: maps each point to a color.
#[derive(Debug, Clone, Copy)]
pub struct Fill;

impl sealed::Sealed for Fill {}
impl Channel for Fill {
    type Input = String;
    type Output = Color;
    type Kind = FillKind;
}

/// Shared manual-lookup closure: maps a string key to an optional color.
pub(crate) type FillLookup = Arc<dyn Fn(&str) -> Option<Color> + Send + Sync>;

/// How a fill encoding's extracted key maps to the channel value.
#[derive(Clone)]
pub enum FillKind {
    /// Assign successive palette slots to keys in order of first appearance.
    /// The `source` decides whether those slots come from a seed-derived
    /// palette (theme-driven, default [`Palette::TONAL`]) or from an
    /// explicit user-provided color list.
    Ordinal { source: OrdinalSource },
    /// Explicit lookup table. Missing keys fall through the resolution chain.
    Manual { mapping: Vec<(String, Color)> },
    /// Closure-based lookup. `None` from the closure also falls through.
    ManualWith { lookup: FillLookup },
}

/// How an ordinal fill encoding picks its colors. Mutually exclusive by
/// construction — calling `Encoding::range` replaces any prior palette
/// choice, and `Encoding::palette` replaces any prior range.
#[derive(Clone, Debug, PartialEq)]
pub enum OrdinalSource {
    /// Build the palette from the design seed at draw time with the given
    /// flavor. `None` means "inherit from the chart's `Data::palette`
    /// setting, or fall back to [`Palette::TONAL`] if the chart
    /// didn't set one." `Some(p)` means "always use `p`, regardless of
    /// the chart's setting." See D17 for the inheritance story.
    Seed(Option<Palette>),
    /// Use the user-supplied color list verbatim (insertion-order assignment).
    Range(Vec<Color>),
}

impl PartialEq for FillKind {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (FillKind::Ordinal { source: a }, FillKind::Ordinal { source: b }) => a == b,
            (FillKind::Manual { mapping: a }, FillKind::Manual { mapping: b }) => a == b,
            (FillKind::ManualWith { lookup: a }, FillKind::ManualWith { lookup: b }) => Arc::ptr_eq(a, b),
            _ => false,
        }
    }
}

impl std::fmt::Debug for FillKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FillKind::Ordinal { source } => f.debug_struct("Ordinal").field("source", source).finish(),
            FillKind::Manual { mapping } => f.debug_struct("Manual").field("entries", &mapping.len()).finish(),
            FillKind::ManualWith { .. } => f.debug_struct("ManualWith").finish_non_exhaustive(),
        }
    }
}

// ============================================================================
// Size channel — continuous: maps a numeric input through a scale function
// (Sqrt by default, perceptually correct for area) onto a pixel range.
// ============================================================================

/// The size channel: maps each point to a pixel diameter.
#[derive(Debug, Clone, Copy)]
pub struct Size;

impl sealed::Sealed for Size {}
impl Channel for Size {
    type Input = f64;
    type Output = f32;
    type Kind = SizeKind;
}

/// Configuration for a size encoding.
///
/// In v1 only the `Continuous` mode exists; manual numeric lookup is omitted
/// because it's rarely useful and the float-key matching story is awkward.
/// Add it later if a real use case shows up.
#[derive(Clone, Debug, PartialEq)]
pub enum SizeKind {
    Continuous {
        /// Input domain. `None` means "auto-detect from the data at resolve
        /// time" (computes min/max across the points).
        domain: Option<RangeInclusive<f64>>,
        /// Output pixel range (diameter, not area).
        range: RangeInclusive<f32>,
        /// Scale function applied between domain and range.
        scale: SizeScale,
    },
}

/// Scale function for a size encoding.
///
/// `Sqrt` is the default because humans perceive bubble *area*, not
/// diameter (Stevens' power law). Sqrt-scaling the input value before
/// linearly mapping to a diameter range means a value twice as large
/// produces a bubble with twice the visual area, which is the perceptually
/// correct behavior for "this number → bubble size" encodings.
///
/// Use `Linear` when the underlying data is already an area (e.g. land
/// area in km²) and you want the diameter to scale linearly with it.
/// Use `Log` for highly skewed numeric distributions.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SizeScale {
    Linear,
    #[default]
    Sqrt,
    Log,
}
