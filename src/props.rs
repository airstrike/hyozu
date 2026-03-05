//! Property descriptors for chart introspection and modification.
//!
//! This module provides a compositional way to address and modify
//! properties of chart elements. Properties can be chained with `.map()`
//! to build paths from raw values to messages.
//!
//! # Example
//!
//! ```ignore
//! use iced::widget::slider;
//! use hyozu::{props, item, Function};
//!
//! slider(0.0..=1.0, current_size, props::bar::Size)
//!     .map(item::Bars.with(0))
//!     .map(Message::Set)
//! ```

pub mod area;
pub mod axis;
pub mod bar;
pub mod gauge;
pub mod line;
pub mod pie;
pub mod rule;
pub mod waterfall;
pub mod xy;
