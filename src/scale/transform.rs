//! Numeric-axis transform: how values map to screen position.

/// How a numeric domain maps onto the unit interval before reaching the
/// screen.
///
/// Affects numeric-axis marks (Bar, Line, Area, Xy). Pie, Choropleth,
/// TileGrid, and BubbleMap don't read this — pie is angle-based, the
/// others are categorical or geographic.
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
}

impl Transform {
    /// Maps `value` from the domain `[min, max]` onto the unit interval
    /// `[0.0, 1.0]`. Used by every numeric-axis mark renderer (via
    /// [`crate::chart::plot_area::Plane::to_pixel`]) and by axis tick
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
}
