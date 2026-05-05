//! Shared animation utilities for hyozu mark renderers.
//!
//! Helpers around iced's [`crate::core::animation::Animation`] primitive
//! (re-exported from the `lilt` crate) that are reused across the
//! entrance/transition animators on pie, bar, line, and other marks.
//!
//! # Example
//!
//! ```ignore
//! use crate::core::animation::{Animation, Easing};
//! use crate::animation;
//!
//! let progress = Animation::new(0.0_f32)
//!     .easing(Easing::Custom(animation::ease))
//!     .duration(animation::DEFAULT_DURATION);
//! ```

use std::time::Duration;

/// Default entrance/transition duration for marks. 500ms strikes a
/// balance between feeling responsive (the chart updates *now*) and
/// trackable (the eye can follow the sweep). D3's convention is 750ms,
/// Recharts ships 1500ms — both feel sluggish in dashboards that update
/// frequently.
pub const DEFAULT_DURATION: Duration = Duration::from_millis(500);

/// CSS `ease` — `cubic-bezier(0.25, 0.1, 0.25, 1.0)`. Soft ease-in with
/// a more pronounced ease-out, designer-tuned to feel natural at both
/// ends. Pass to [`crate::core::animation::Easing::Custom`].
pub fn ease(t: f32) -> f32 {
    cubic_bezier(0.25, 0.1, 0.25, 1.0, t)
}

/// Evaluates a cubic-bezier curve at parameter `x`.
///
/// The curve has control points `P0 = (0, 0)`, `P1 = (x1, y1)`,
/// `P2 = (x2, y2)`, `P3 = (1, 1)`. Given an `x` in `[0, 1]`, we solve
/// for the parameter `t` such that `B_x(t) = x` using a few iterations
/// of Newton's method, then evaluate `B_y(t)` at that `t`.
pub fn cubic_bezier(x1: f32, y1: f32, x2: f32, y2: f32, x: f32) -> f32 {
    if x <= 0.0 {
        return 0.0;
    }
    if x >= 1.0 {
        return 1.0;
    }

    let mut t = x;
    for _ in 0..8 {
        let t2 = t * t;
        let t3 = t2 * t;
        let bx = 3.0 * (1.0 - t) * (1.0 - t) * t * x1 + 3.0 * (1.0 - t) * t2 * x2 + t3;
        let dbx = 3.0 * (1.0 - t) * (1.0 - t) * x1 + 6.0 * (1.0 - t) * t * (x2 - x1) + 3.0 * t2 * (1.0 - x2);
        if dbx.abs() < 1e-6 {
            break;
        }
        t -= (bx - x) / dbx;
        t = t.clamp(0.0, 1.0);
    }

    let t2 = t * t;
    let t3 = t2 * t;
    3.0 * (1.0 - t) * (1.0 - t) * t * y1 + 3.0 * (1.0 - t) * t2 * y2 + t3
}
