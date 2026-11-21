//! Wheel zoom and drag pan over a chart's continuous axes.
//!
//! The chart keeps the visible window in its widget state, one [`Window`]
//! per screen axis. A window is a sub-interval of the axis's full domain
//! measured in the axis transform's unit space, so a log axis zooms and
//! pans in log space and a sqrt axis in sqrt space. Reversed axes flip the
//! screen direction before the window math, so the value under the cursor
//! stays put either way.
//!
//! Each change reaches the application as [`crate::Action::Viewport`],
//! carrying the window as data values. [`Viewport::apply`] copies it onto
//! another chart's axes to link zoom across views.

use crate::core::{Point, Rectangle, Vector};
use crate::data::axis::Order;
use crate::scale::Mapping;

/// Which axes respond to wheel zoom and drag pan.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Axes {
    /// Only the horizontal axis zooms and pans.
    #[default]
    X,
    /// Only the vertical axis zooms and pans.
    Y,
    /// Both axes zoom and pan together.
    Both,
}

impl Axes {
    fn x(self) -> bool {
        matches!(self, Self::X | Self::Both)
    }

    fn y(self) -> bool {
        matches!(self, Self::Y | Self::Both)
    }
}

/// The visible region of a zoomed chart, as data values on the primary axes.
///
/// An axis showing its full domain reports `None`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Viewport {
    x: Option<[f64; 2]>,
    y: Option<[f64; 2]>,
}

impl Viewport {
    /// The visible `[min, max]` of the horizontal axis, if zoomed.
    pub fn x(&self) -> Option<[f64; 2]> {
        self.x
    }

    /// The visible `[min, max]` of the vertical axis, if zoomed.
    pub fn y(&self) -> Option<[f64; 2]> {
        self.y
    }

    /// Whether both axes show their full domain.
    pub fn is_full(&self) -> bool {
        self.x.is_none() && self.y.is_none()
    }

    /// Shows the same region on another chart: zoomed axes get explicit
    /// bounds, full axes return to their automatic bounds. The secondary x
    /// axis follows the primary one. Don't apply a viewport back onto the
    /// chart that emitted it: that chart already shows it.
    pub fn apply(self, data: &mut crate::Data) {
        for axis in [data.primary.x_axis_mut(), data.secondary.x_axis_mut()]
            .into_iter()
            .flatten()
        {
            axis.lower_bound = self.x.map(|[lo, _]| lo);
            axis.upper_bound = self.x.map(|[_, hi]| hi);
        }
        if let Some(axis) = data.primary.y_axis_mut() {
            axis.lower_bound = self.y.map(|[lo, _]| lo);
            axis.upper_bound = self.y.map(|[_, hi]| hi);
        }
        data.invalidate();
    }
}

/// One axis's visible window, as a sub-interval of `[0, 1]` in the unit
/// space of the axis transform over its full domain.
///
/// Constructed only through [`Window::FULL`] and the clamping
/// transformations, so `0 <= lo < hi <= 1` and the span never drops below
/// [`Window::MIN_SPAN`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Window {
    lo: f64,
    hi: f64,
}

impl Default for Window {
    fn default() -> Self {
        Self::FULL
    }
}

impl Window {
    /// The whole domain.
    pub(crate) const FULL: Self = Self { lo: 0.0, hi: 1.0 };

    /// The deepest zoom: one thousandth of the full domain.
    const MIN_SPAN: f64 = 1e-3;

    /// Clamps a requested interval into the full domain, keeping its span
    /// where possible and its position otherwise.
    fn clamped(lo: f64, hi: f64) -> Self {
        if !lo.is_finite() || !hi.is_finite() {
            return Self::FULL;
        }
        let span = (hi - lo).clamp(Self::MIN_SPAN, 1.0);
        let lo = lo.clamp(0.0, 1.0 - span);
        Self { lo, hi: lo + span }
    }

    fn span(self) -> f64 {
        self.hi - self.lo
    }

    pub(crate) fn is_full(self) -> bool {
        self.lo <= 0.0 && self.hi >= 1.0
    }

    /// Scales the window by `factor` (below one zooms in) about `at`, a
    /// fraction of the current window in domain direction.
    fn zoom(self, at: f64, factor: f64) -> Self {
        let anchor = self.lo + self.span() * at.clamp(0.0, 1.0);
        Self::clamped(
            anchor - (anchor - self.lo) * factor,
            anchor + (self.hi - anchor) * factor,
        )
    }

    /// Slides the window by `by` of its own span, in domain direction.
    fn pan(self, by: f64) -> Self {
        let shift = by * self.span();
        Self::clamped(self.lo + shift, self.hi + shift)
    }

    /// The window's ends as data values over the full domain `home`.
    pub(crate) fn bounds(self, home: [f64; 2], mapping: Mapping) -> [f64; 2] {
        let [min, max] = home;
        let lo = mapping.transform.unit_to_value(self.lo, min, max);
        let hi = mapping.transform.unit_to_value(self.hi, min, max);
        [lo.min(hi), lo.max(hi)]
    }
}

/// A zoomable axis at the time of an event: its full domain and mapping.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Axis {
    pub(crate) home: [f64; 2],
    pub(crate) mapping: Mapping,
}

impl Axis {
    /// Converts a screen fraction (0 at the left or bottom edge) into a
    /// fraction along the domain, which runs the other way when reversed.
    fn toward(self, screen: f64) -> f64 {
        match self.mapping.order {
            Order::Forward => screen,
            Order::Reverse => 1.0 - screen,
        }
    }
}

/// The plot's data rectangle in window coordinates and the axes that can
/// zoom. Categorical and absent axes are `None`.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Plot {
    pub(crate) rect: Rectangle,
    pub(crate) x: Option<Axis>,
    pub(crate) y: Option<Axis>,
}

impl Plot {
    /// Screen fractions of `point`: `x` from the left edge, `y` from the
    /// bottom edge.
    fn fractions(&self, point: Point) -> (f64, f64) {
        let x = f64::from((point.x - self.rect.x) / self.rect.width.max(1.0));
        let y = f64::from((self.rect.y + self.rect.height - point.y) / self.rect.height.max(1.0));
        (x, y)
    }
}

/// The visible windows of both screen axes.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct State {
    pub(crate) x: Window,
    pub(crate) y: Window,
}

impl State {
    /// Zoom factor per wheel line. Four lines halve the visible span.
    const STEP: f64 = 0.8408964152537145;

    /// Pixels a high-precision scroll device sends per wheel line.
    pub(crate) const PIXELS_PER_LINE: f32 = 40.0;

    pub(crate) fn is_zoomed(&self) -> bool {
        !(self.x.is_full() && self.y.is_full())
    }

    /// Zooms about the cursor by `lines` wheel lines; positive zooms in.
    /// Returns whether the windows changed.
    pub(crate) fn wheel(&mut self, axes: Axes, plot: &Plot, cursor: Point, lines: f32) -> bool {
        let factor = Self::STEP.powf(f64::from(lines));
        let (sx, sy) = plot.fractions(cursor);
        let before = *self;
        if axes.x()
            && let Some(axis) = plot.x
        {
            self.x = self.x.zoom(axis.toward(sx), factor);
        }
        if axes.y()
            && let Some(axis) = plot.y
        {
            self.y = self.y.zoom(axis.toward(sy), factor);
        }
        *self != before
    }

    /// The windows `start` would show after the content followed the
    /// pointer by `delta` pixels. Returns whether that differs from `self`.
    pub(crate) fn pan(&mut self, start: Self, axes: Axes, plot: &Plot, delta: Vector) -> bool {
        let before = *self;
        let fraction = |pixels: f32, length: f32| f64::from(pixels / length.max(1.0));
        if axes.x()
            && let Some(axis) = plot.x
        {
            let along = axis.toward(fraction(delta.x, plot.rect.width)) - axis.toward(0.0);
            self.x = start.x.pan(-along);
        }
        if axes.y()
            && let Some(axis) = plot.y
        {
            let along = axis.toward(fraction(-delta.y, plot.rect.height)) - axis.toward(0.0);
            self.y = start.y.pan(-along);
        }
        *self != before
    }

    /// Returns both axes to their full domain. Returns whether anything
    /// was zoomed.
    pub(crate) fn reset(&mut self) -> bool {
        let zoomed = self.is_zoomed();
        *self = Self::default();
        zoomed
    }

    /// The windows as data values on the plot's zoomable axes.
    pub(crate) fn viewport(&self, plot: &Plot) -> Viewport {
        let bounds = |window: Window, axis: Option<Axis>| {
            axis.filter(|_| !window.is_full())
                .map(|axis| window.bounds(axis.home, axis.mapping))
        };
        Viewport {
            x: bounds(self.x, plot.x),
            y: bounds(self.y, plot.y),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::Size;
    use crate::scale::Transform;

    fn plot(x: Mapping, y: Mapping) -> Plot {
        Plot {
            rect: Rectangle::new(Point::new(50.0, 20.0), Size::new(400.0, 200.0)),
            x: Some(Axis {
                home: [0.0, 100.0],
                mapping: x,
            }),
            y: Some(Axis {
                home: [1.0, 10_000.0],
                mapping: y,
            }),
        }
    }

    fn linear() -> Mapping {
        Mapping::new(Transform::Linear, Order::Forward)
    }

    /// Within pixel precision: the plot maps through `f32` screen space.
    fn close(a: [f64; 2], b: [f64; 2]) -> bool {
        (a[0] - b[0]).abs() < 1e-5 * b[0].abs().max(1.0) && (a[1] - b[1]).abs() < 1e-5 * b[1].abs().max(1.0)
    }

    /// The data value at a screen x, read through the current window.
    fn value_at(state: &State, plot: &Plot, screen_x: f32) -> f64 {
        let axis = plot.x.expect("x axis");
        let [lo, hi] = state.x.bounds(axis.home, axis.mapping);
        let (sx, _) = plot.fractions(Point::new(screen_x, 0.0));
        axis.mapping.unit_to_value(sx, lo, hi)
    }

    #[test]
    fn wheel_zooms_about_the_cursor_and_reports_bounds() {
        let plot = plot(linear(), linear());
        let mut state = State::default();
        // Cursor a quarter of the way across the plot, at x = 25.
        let cursor = Point::new(150.0, 120.0);
        assert!(state.wheel(Axes::X, &plot, cursor, 4.0));
        let viewport = state.viewport(&plot);
        assert!(close(viewport.x().expect("zoomed x"), [12.5, 62.5]), "{viewport:?}");
        assert_eq!(viewport.y(), None, "y stays full when only x zooms");
        assert!((value_at(&state, &plot, 150.0) - 25.0).abs() < 1e-9);
    }

    #[test]
    fn wheel_out_clamps_to_the_full_domain() {
        let plot = plot(linear(), linear());
        let mut state = State::default();
        assert!(!state.wheel(Axes::Both, &plot, Point::new(100.0, 100.0), -3.0));
        assert!(state.viewport(&plot).is_full());
        state.wheel(Axes::Both, &plot, Point::new(100.0, 100.0), 2.0);
        state.wheel(Axes::Both, &plot, Point::new(400.0, 40.0), -20.0);
        assert!(!state.is_zoomed(), "zooming far out returns to the full domain");
    }

    #[test]
    fn wheel_in_stops_at_the_minimum_span() {
        let plot = plot(linear(), linear());
        let mut state = State::default();
        for _ in 0..200 {
            state.wheel(Axes::X, &plot, Point::new(250.0, 100.0), 1.0);
        }
        let [lo, hi] = state.viewport(&plot).x().expect("zoomed");
        assert!((hi - lo - 0.1).abs() < 1e-9, "span {}", hi - lo);
    }

    #[test]
    fn drag_shifts_the_window_with_the_pointer_and_clamps_at_the_edge() {
        let plot = plot(linear(), linear());
        let mut state = State::default();
        state.wheel(Axes::Both, &plot, Point::new(250.0, 120.0), 4.0);
        let start = state;
        let before = value_at(&state, &plot, 200.0);
        // Dragging right by 40px moves the content right: the value that
        // sat at 200px now sits at 240px.
        assert!(state.pan(start, Axes::Both, &plot, Vector::new(40.0, 0.0)));
        assert!((value_at(&state, &plot, 240.0) - before).abs() < 1e-5);
        let [lo, hi] = state.viewport(&plot).x().expect("zoomed");
        assert!(close([lo, hi], [20.0, 70.0]), "{lo} {hi}");
        // A huge drag stops at the domain start, keeping the span.
        state.pan(start, Axes::Both, &plot, Vector::new(10_000.0, 0.0));
        assert!(close(state.viewport(&plot).x().expect("zoomed"), [0.0, 50.0]));
        // Dragging down pulls larger y values into view, up to the top.
        state.pan(start, Axes::Y, &plot, Vector::new(0.0, 10_000.0));
        let y = state.viewport(&plot).y().expect("zoomed y");
        assert!((y[1] - 10_000.0).abs() < 1e-6, "{y:?}");
    }

    #[test]
    fn log_axes_zoom_in_log_space() {
        let log = Mapping::new(Transform::Log, Order::Forward);
        let plot = plot(linear(), log);
        let mut state = State::default();
        // Cursor at the vertical middle: log-space center is 100.
        assert!(state.wheel(Axes::Y, &plot, Point::new(100.0, 120.0), 4.0));
        let y = state.viewport(&plot).y().expect("zoomed y");
        assert!(close(y, [10.0, 1000.0]), "{y:?}");
        // Dragging up by half the plot height shows one decade lower.
        let start = state;
        state.pan(start, Axes::Y, &plot, Vector::new(0.0, -100.0));
        let y = state.viewport(&plot).y().expect("zoomed y");
        assert!(close(y, [1.0, 100.0]), "{y:?}");
    }

    #[test]
    fn reversed_axes_keep_the_value_under_the_cursor() {
        let reversed = Mapping::new(Transform::Linear, Order::Reverse);
        let plot = plot(reversed, linear());
        let mut state = State::default();
        let before = value_at(&state, &plot, 150.0);
        assert!(
            (before - 75.0).abs() < 1e-9,
            "the left quarter of a reversed axis is 75"
        );
        state.wheel(Axes::X, &plot, Point::new(150.0, 100.0), 4.0);
        assert!(close(state.viewport(&plot).x().expect("zoomed"), [37.5, 87.5]));
        assert!((value_at(&state, &plot, 150.0) - before).abs() < 1e-9);
        // Dragging right on a reversed axis reveals larger values.
        let start = state;
        let anchor = value_at(&state, &plot, 200.0);
        state.pan(start, Axes::X, &plot, Vector::new(40.0, 0.0));
        assert!((value_at(&state, &plot, 240.0) - anchor).abs() < 1e-5);
        assert!(state.viewport(&plot).x().expect("zoomed")[1] > 87.5);
    }

    #[test]
    fn categorical_axes_ignore_zoom_and_reset_restores_full() {
        let mut plot = plot(linear(), linear());
        plot.x = None;
        let mut state = State::default();
        assert!(!state.wheel(Axes::X, &plot, Point::new(100.0, 100.0), 3.0));
        assert!(state.wheel(Axes::Both, &plot, Point::new(100.0, 100.0), 3.0));
        assert_eq!(state.viewport(&plot).x(), None);
        assert!(state.reset());
        assert!(!state.reset());
        assert!(state.viewport(&plot).is_full());
    }

    #[test]
    fn apply_copies_the_region_and_full_restores_auto_bounds() {
        let mut data = crate::data(crate::line([(0.0, 1.0), (10.0, 5.0)]));
        let generation = data.generation;
        let viewport = Viewport {
            x: Some([2.0, 4.0]),
            y: None,
        };
        viewport.apply(&mut data);
        let x = data.x_axis_ref().expect("x axis");
        assert_eq!((x.lower_bound, x.upper_bound), (Some(2.0), Some(4.0)));
        assert_ne!(data.generation, generation);
        Viewport::default().apply(&mut data);
        assert_eq!(data.x_axis_ref().expect("x axis").upper_bound, None);
    }
}
