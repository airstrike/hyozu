//! Pipe/map trait for composable property chains.
//!
//! Allows chaining property constructors with `.map()`:
//!
//! ```ignore
//! let on_position = |p| {
//!     props::bar::label::Property::Position(p)
//!         .map(props::bar::Label)
//!         .map(item::Bars.with(0))
//!         .map(Message::Set)
//! };
//! ```

/// A trait for piping values through functions.
///
/// Similar to Elixir's `|>` operator or F#'s `|>`.
pub trait Map: Sized {
    /// Pipes this value through a function.
    fn map<B>(self, f: impl FnOnce(Self) -> B) -> B {
        f(self)
    }
}
