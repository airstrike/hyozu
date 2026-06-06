//! Numeric-axis transform: how values map to screen position.

/// Compute a "nice" step size for `span` divided into roughly `max_ticks`
/// intervals. Implements the Heckbert algorithm: snap the raw step to the
/// nearest of `{1, 2, 5, 10} × 10^k`, where `k` is chosen from the order
/// of magnitude of `span / max_ticks`.
pub(crate) fn nice_step(span: f64, max_ticks: usize) -> f64 {
    let raw = span / max_ticks as f64;
    let mag = 10f64.powf(raw.log10().floor());
    let frac = raw / mag;
    let nice_frac = if frac < 1.5 {
        1.0
    } else if frac < 3.0 {
        2.0
    } else if frac < 7.0 {
        5.0
    } else {
        10.0
    };
    nice_frac * mag
}

/// Round a linear domain `[lo, hi]` outward to nice numbers using
/// Heckbert's algorithm with `max_ticks` as the target tick count.
///
/// Edge behavior:
/// - Non-finite inputs return unchanged so callers don't need to guard.
/// - `lo == hi` (degenerate span) returns unchanged — there's no span to
///   nice and dividing by zero would produce NaN.
/// - `hi < lo` (inverted) nices the absolute span and preserves the
///   original argument order.
pub(crate) fn nice_domain(lo: f64, hi: f64, max_ticks: usize) -> (f64, f64) {
    if !lo.is_finite() || !hi.is_finite() || lo == hi {
        return (lo, hi);
    }
    let span = (hi - lo).abs();
    let step = nice_step(span, max_ticks);
    if hi >= lo {
        let nice_lo = (lo / step).floor() * step;
        let nice_hi = (hi / step).ceil() * step;
        (nice_lo, nice_hi)
    } else {
        let nice_lo = (lo / step).ceil() * step;
        let nice_hi = (hi / step).floor() * step;
        (nice_lo, nice_hi)
    }
}

/// Round a log-axis domain `[lo, hi]` outward to decade boundaries.
/// `lo` snaps down to `10^floor(log10(lo))`, `hi` up to
/// `10^ceil(log10(hi))`.
///
/// Returns the input unchanged when `lo <= 0`, since log-transformed
/// non-positive values are already a degenerate case handled by the
/// transform's `f64::EPSILON` clamp — niceing on top of that would
/// just produce a misleading display range.
pub(crate) fn nice_log_domain(lo: f64, hi: f64) -> (f64, f64) {
    if !lo.is_finite() || !hi.is_finite() || lo <= 0.0 || hi <= 0.0 || lo == hi {
        return (lo, hi);
    }
    let lo_dec = 10f64.powi(lo.log10().floor() as i32);
    let hi_dec = 10f64.powi(hi.log10().ceil() as i32);
    (lo_dec, hi_dec)
}

/// How a numeric domain maps onto the unit interval before reaching the
/// screen.
///
/// Affects numeric-axis marks (Bar, Line, Area, Xy). Pie, Choropleth,
/// and TileGrid don't read this — pie is angle-based, the others are
/// categorical or geographic.
#[non_exhaustive]
#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
pub enum Transform {
    /// Identity: position is `(value - min) / (max - min)`.
    #[default]
    Linear,
    /// Logarithmic: position is
    /// `(log10(value) - log10(min)) / (log10(max) - log10(min))`.
    ///
    /// Non-positive values can't be log-transformed; `map_to_unit` clamps
    /// `value`, `min`, and `max` up to [`f64::EPSILON`] before taking the
    /// log. ggplot2 emits warnings for these inputs and Observable Plot
    /// drops them; hyozu clamps and renders so the chart still produces
    /// output instead of disappearing on a single bad sample.
    Log,
    /// Square-root: position is `sqrt(value - min) / sqrt(max - min)`.
    ///
    /// Useful for skewed distributions where Log over-emphasizes the low
    /// end. Below-domain values (`value < min`) clamp to `0.0` — i.e. they
    /// land at the bottom of the visual range. This is asymmetric with
    /// [`Self::Log`], which clamps non-positive inputs up to
    /// [`f64::EPSILON`]; Sqrt's domain is `[min, ∞)` and a value beneath
    /// that floor is treated as the floor itself rather than an
    /// arbitrarily small positive number.
    Sqrt,
}

impl Transform {
    /// Maps `value` from the domain `[min, max]` onto the unit interval
    /// `[0.0, 1.0]`. Used by every numeric-axis mark renderer (via
    /// [`crate::chart::plot_area::to_pixel`]) and by axis tick
    /// generation.
    ///
    /// Degenerate domains (`min == max`) return `0.5`. For [`Self::Log`],
    /// non-positive values are clamped to [`f64::EPSILON`].
    pub fn map_to_unit(&self, value: f64, min: f64, max: f64) -> f64 {
        match self {
            Self::Linear => {
                if (max - min).abs() < f64::EPSILON {
                    0.5
                } else {
                    (value - min) / (max - min)
                }
            }
            Self::Log => {
                let v = value.max(f64::EPSILON);
                let lo = min.max(f64::EPSILON);
                let hi = max.max(f64::EPSILON);
                if (hi.log10() - lo.log10()).abs() < f64::EPSILON {
                    0.5
                } else {
                    (v.log10() - lo.log10()) / (hi.log10() - lo.log10())
                }
            }
            Self::Sqrt => {
                let v = (value - min).max(0.0);
                let span = (max - min).max(0.0);
                if span <= f64::EPSILON {
                    0.0
                } else {
                    v.sqrt() / span.sqrt()
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn linear_maps_in_range() {
        let t = Transform::Linear;
        assert!(close(t.map_to_unit(5.0, 0.0, 10.0), 0.5));
        assert!(close(t.map_to_unit(2.5, 0.0, 10.0), 0.25));
    }

    #[test]
    fn linear_at_min_is_zero() {
        assert!(close(Transform::Linear.map_to_unit(0.0, 0.0, 10.0), 0.0));
    }

    #[test]
    fn linear_at_max_is_one() {
        assert!(close(Transform::Linear.map_to_unit(10.0, 0.0, 10.0), 1.0));
    }

    #[test]
    fn linear_min_equals_max_returns_half() {
        assert!(close(Transform::Linear.map_to_unit(7.0, 5.0, 5.0), 0.5));
    }

    #[test]
    fn linear_handles_negative_values() {
        // Linear path makes no log-domain assumption — negatives just
        // produce negative or >1 unit positions, which the renderer
        // happily extrapolates off-axis. Nothing to clamp here.
        let t = Transform::Linear;
        assert!(close(t.map_to_unit(-5.0, 0.0, 10.0), -0.5));
        assert!(close(t.map_to_unit(15.0, 0.0, 10.0), 1.5));
    }

    #[test]
    fn log_maps_decade_boundaries() {
        let t = Transform::Log;
        // [1, 1000] is 3 decades; 10 sits 1/3, 100 sits 2/3.
        assert!(close(t.map_to_unit(1.0, 1.0, 1000.0), 0.0));
        assert!(close(t.map_to_unit(10.0, 1.0, 1000.0), 1.0 / 3.0));
        assert!(close(t.map_to_unit(100.0, 1.0, 1000.0), 2.0 / 3.0));
        assert!(close(t.map_to_unit(1000.0, 1.0, 1000.0), 1.0));
    }

    #[test]
    fn log_clamps_non_positive_value() {
        // value=0 clamps to f64::EPSILON; result is far below 0 because
        // log10(EPSILON) is a large negative number against log10(1) = 0.
        let t = Transform::Log;
        let p = t.map_to_unit(0.0, 1.0, 100.0);
        assert!(p.is_finite(), "non-positive value should not produce NaN/Inf");
    }

    #[test]
    fn log_clamps_non_positive_min() {
        let t = Transform::Log;
        let p = t.map_to_unit(10.0, 0.0, 100.0);
        assert!(p.is_finite());
    }

    #[test]
    fn log_min_equals_max_returns_half() {
        assert!(close(Transform::Log.map_to_unit(7.0, 5.0, 5.0), 0.5));
    }

    #[test]
    fn sqrt_maps_quarter_to_half() {
        // sqrt(0.25) / sqrt(1.0) = 0.5
        assert!(close(Transform::Sqrt.map_to_unit(0.25, 0.0, 1.0), 0.5));
    }

    #[test]
    fn sqrt_maps_in_range() {
        let t = Transform::Sqrt;
        // sqrt(0)/sqrt(100) = 0; sqrt(100)/sqrt(100) = 1
        assert!(close(t.map_to_unit(0.0, 0.0, 100.0), 0.0));
        assert!(close(t.map_to_unit(100.0, 0.0, 100.0), 1.0));
        // sqrt(64)/sqrt(100) = 0.8
        assert!(close(t.map_to_unit(64.0, 0.0, 100.0), 0.8));
    }

    #[test]
    fn sqrt_min_equals_max_returns_zero() {
        // Asymmetric with Linear's 0.5 — Sqrt's degenerate domain has no
        // meaningful midpoint when the span is zero, and the Sqrt body
        // returns 0.0 to keep below-domain semantics consistent.
        assert!(close(Transform::Sqrt.map_to_unit(7.0, 5.0, 5.0), 0.0));
    }

    #[test]
    fn sqrt_clamps_below_domain_to_zero() {
        let t = Transform::Sqrt;
        assert!(close(t.map_to_unit(-10.0, 0.0, 100.0), 0.0));
        assert!(close(t.map_to_unit(-1.0, 5.0, 25.0), 0.0));
    }

    #[test]
    fn nice_domain_canonical_example() {
        // span = 13.3, raw step = 13.3/5 = 2.66, mag = 1, frac = 2.66.
        // 2.66 < 3.0 → nice_frac = 2.0, step = 2.0.
        // lo = floor(10.8/2)*2 = 10, hi = ceil(24.1/2)*2 = 26.
        let (lo, hi) = nice_domain(10.8, 24.1, 5);
        assert!(close(lo, 10.0), "lo was {lo}");
        assert!(close(hi, 26.0), "hi was {hi}");
    }

    #[test]
    fn nice_domain_unit_interval_is_idempotent() {
        let (lo, hi) = nice_domain(0.0, 1.0, 5);
        assert!(close(lo, 0.0));
        assert!(close(hi, 1.0));
    }

    #[test]
    fn nice_domain_negative_to_positive_rounds_outward() {
        // span = 12.1, raw step = 12.1/5 = 2.42, mag = 1, frac = 2.42
        // → nice_frac = 2.0 (since frac < 3.0), step = 2.
        // lo = floor(-3.7/2)*2 = -4, hi = ceil(8.4/2)*2 = 10.
        let (lo, hi) = nice_domain(-3.7, 8.4, 5);
        assert!(close(lo, -4.0), "lo was {lo}");
        assert!(close(hi, 10.0), "hi was {hi}");
    }

    #[test]
    fn nice_domain_degenerate_span_returns_input() {
        let (lo, hi) = nice_domain(7.0, 7.0, 5);
        assert!(close(lo, 7.0));
        assert!(close(hi, 7.0));
    }

    #[test]
    fn nice_domain_large_span() {
        // span = 900, raw = 180, mag = 100, frac = 1.8 → nice_frac = 2,
        // step = 200. lo = floor(100/200)*200 = 0, hi = ceil(1000/200)*200 = 1000.
        let (lo, hi) = nice_domain(100.0, 1000.0, 5);
        assert!(close(lo, 0.0));
        assert!(close(hi, 1000.0));
    }

    #[test]
    fn nice_domain_non_finite_input_unchanged() {
        let (lo, hi) = nice_domain(f64::NAN, 10.0, 5);
        assert!(lo.is_nan());
        assert!(close(hi, 10.0));
        let (lo, hi) = nice_domain(0.0, f64::INFINITY, 5);
        assert!(close(lo, 0.0));
        assert!(hi.is_infinite());
    }

    #[test]
    fn nice_log_domain_snaps_to_decades() {
        // 2.5..8500 → 1..10000
        let (lo, hi) = nice_log_domain(2.5, 8500.0);
        assert!(close(lo, 1.0));
        assert!(close(hi, 10000.0));
    }

    #[test]
    fn nice_log_domain_skips_non_positive() {
        // lo <= 0 leaves the transform's EPSILON-clamp behavior intact —
        // niceing on top would produce a misleading display range.
        let (lo, hi) = nice_log_domain(0.0, 100.0);
        assert!(close(lo, 0.0));
        assert!(close(hi, 100.0));
    }

    #[test]
    fn nice_log_domain_idempotent_at_decade_endpoints() {
        let (lo, hi) = nice_log_domain(1.0, 1000.0);
        assert!(close(lo, 1.0));
        assert!(close(hi, 1000.0));
    }
}
