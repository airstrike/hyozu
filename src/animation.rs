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

use crate::core::Shell;
use crate::core::animation::{Animation, Easing};
use crate::core::time::Instant;

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

/// Per-mark entrance/transition lifecycle shared across animated marks.
///
/// Bundles the three pieces of state every animated mark needs: the
/// underlying [`Animation`] driver, a `pending_start` flag set when a
/// mount or data-change has queued a fresh sweep, and the most recent
/// [`Instant`] captured from `RedrawRequested` so `draw` can interpolate
/// without needing the event again.
///
/// The chart's per-frame walker calls [`Tick::advance`] on each animated
/// mark to kick pending sweeps off and request additional redraws while
/// any sweep is in flight; the mark's `draw` calls [`Tick::progress`] to
/// read the eased progress at the captured `Instant`.
pub struct Tick {
    /// Sweep progress (`0.0` collapsed, `1.0` fully drawn).
    pub progress: Animation<f32>,
    /// Set when the animation needs to be kicked off on the next
    /// `RedrawRequested`. The animation primitive needs an `Instant` that
    /// only the redraw event carries; the chart's walker consumes this
    /// flag and calls [`Animation::go_mut`].
    pub pending_start: bool,
    /// Most recent `RedrawRequested` time, captured by the chart widget's
    /// `update` so `draw` can interpolate the animation.
    pub now: Option<Instant>,
}

impl Tick {
    /// Constructs a fresh tick with the shared 500ms CSS-`ease` curve
    /// and `pending_start` already set, so the first redraw kicks the
    /// mount sweep off without any caller intervention.
    pub fn new() -> Self {
        Self {
            progress: Animation::new(0.0_f32)
                .easing(Easing::Custom(ease))
                .duration(DEFAULT_DURATION),
            pending_start: true,
            now: None,
        }
    }

    /// Returns the eased progress at the most recently observed
    /// `Instant`. `0.0` until the first `RedrawRequested` populates
    /// [`Self::now`].
    pub fn progress(&self) -> f32 {
        match self.now {
            Some(now) => self.progress.interpolate_with(|v| v, now),
            None => 0.0,
        }
    }

    /// Whether the sweep is still in flight, i.e. eased progress hasn't
    /// settled at `1.0`. Marks gate label / selection rendering on this
    /// so overlays don't pop in over geometry that's still sweeping.
    pub fn is_animating(&self) -> bool {
        self.progress() < 1.0 - f32::EPSILON
    }

    /// Advances the lifecycle by one frame. Records `now`; if a sweep
    /// is queued, kicks it off and asks for a redraw; otherwise asks
    /// for another redraw while the underlying animation is still in
    /// flight. The chart's walker is responsible for the chart-level
    /// `Data::animate` gate — this method assumes the caller already
    /// decided to advance.
    pub fn advance<Message>(&mut self, now: Instant, shell: &mut Shell<'_, Message>) {
        self.now = Some(now);
        if self.pending_start {
            self.progress.go_mut(1.0, now);
            self.pending_start = false;
            shell.request_redraw();
        } else if self.progress.is_animating(now) {
            shell.request_redraw();
        }
    }
}

impl Default for Tick {
    fn default() -> Self {
        Self::new()
    }
}
