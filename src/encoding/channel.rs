//! Visual channels that an [`Encoding`](super::Encoding) can target.
//!
//! Channel markers are intentionally only reachable as `channel::Fill` so they
//! never collide with `iced_widget::canvas::Fill` inside the renderer.

mod sealed {
    pub trait Sealed {}
}

/// A visual channel an encoding can target (color/fill, and in future size,
/// opacity, stroke, …). Sealed: only crate-defined markers may implement it.
pub trait Channel: sealed::Sealed {}

/// The fill (color) channel: maps each point to a color.
#[derive(Debug, Clone, Copy)]
pub struct Fill;

impl sealed::Sealed for Fill {}
impl Channel for Fill {}
