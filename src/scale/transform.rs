//! Numeric-axis transform: how values map to screen position.

/// How a numeric domain maps onto the unit interval before reaching the
/// screen.
///
/// Affects numeric-axis marks (Bar, Line, Area, Xy). Pie, Choropleth,
/// TileGrid, and BubbleMap don't read this — pie is angle-based, the
/// others are categorical or geographic.
///
/// [`Transform::Log`] is defined here for the v1 surface; the renderer
/// hookup that actually honors it lands in a follow-up pass.
#[non_exhaustive]
#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
pub enum Transform {
    /// Identity: position is `(value - min) / (max - min)`.
    #[default]
    Linear,
    /// Logarithmic: position is
    /// `(log10(value) - log10(min)) / (log10(max) - log10(min))`.
    /// Renderer clamps non-positive values to a small epsilon.
    Log,
}
