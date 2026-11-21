//! How one axis carries data onto the unit interval: a [`Transform`] plus
//! a direction.

use crate::data::axis::breaks::Squeeze;
use crate::data::axis::{Break, Order};
use crate::scale::Transform;

/// One axis's data-to-unit mapping.
///
/// The [`Transform`] shapes the spacing (linear, log, sqrt); the [`Order`]
/// says which end of the unit interval the domain minimum lands on.
/// [`Order::Reverse`] puts the minimum at `1.0`, so a reversed y-axis
/// grows downward and a reversed x-axis grows leftward. Plot-area marks,
/// gridlines and axis guides all map through the same `Mapping`, so a
/// reversed or log axis stays aligned with its ticks by construction.
///
/// A [`Break`] closes a range of values up into a narrow gap, keeping one
/// scale on both sides of it.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Mapping {
    /// Spacing of values along the axis.
    pub transform: Transform,
    /// Which end of the axis the domain minimum sits on.
    pub order: Order,
    /// The range of values the axis leaves out, if it's broken.
    pub gap: Option<Break>,
}

impl Mapping {
    /// Builds a mapping from a transform and a direction.
    pub fn new(transform: Transform, order: Order) -> Self {
        Self {
            transform,
            order,
            gap: None,
        }
    }

    /// Leaves out the values in `gap`, if any.
    pub fn with_gap(mut self, gap: Option<Break>) -> Self {
        self.gap = gap;
        self
    }

    /// Maps `value` in `[min, max]` onto the unit interval, honoring the
    /// transform, then the break, then the direction.
    pub fn map_to_unit(&self, value: f64, min: f64, max: f64) -> f64 {
        let unit = self.transform.map_to_unit(value, min, max);
        let unit = self.squeeze(min, max).map_or(unit, |squeeze| squeeze.apply(unit));
        match self.order {
            Order::Forward => unit,
            Order::Reverse => 1.0 - unit,
        }
    }

    /// Inverse of [`Self::map_to_unit`].
    pub fn unit_to_value(&self, unit: f64, min: f64, max: f64) -> f64 {
        let unit = match self.order {
            Order::Forward => unit,
            Order::Reverse => 1.0 - unit,
        };
        let unit = self.squeeze(min, max).map_or(unit, |squeeze| squeeze.invert(unit));
        self.transform.unit_to_value(unit, min, max)
    }

    /// Share of the axis the left-out values would take unbroken, over
    /// `[min, max]`: `0` without a break inside the domain.
    pub fn gap_share(&self, min: f64, max: f64) -> f64 {
        self.squeeze(min, max).map_or(0.0, |squeeze| squeeze.width())
    }

    /// The gap's ends on the unit interval over `[min, max]`, in axis
    /// direction, when a break falls inside the domain.
    pub fn gap_units(&self, min: f64, max: f64) -> Option<(f64, f64)> {
        let (start, end) = self.squeeze(min, max)?.gap();
        Some(match self.order {
            Order::Forward => (start, end),
            Order::Reverse => (1.0 - end, 1.0 - start),
        })
    }

    /// The break placed on `[min, max]`'s unit interval.
    fn squeeze(&self, min: f64, max: f64) -> Option<Squeeze> {
        let gap = self.gap?;
        Squeeze::new(
            self.transform.map_to_unit(gap.from(), min, max),
            self.transform.map_to_unit(gap.to(), min, max),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn reverse_flips_the_unit_interval() {
        let forward = Mapping::new(Transform::Linear, Order::Forward);
        let reverse = Mapping::new(Transform::Linear, Order::Reverse);
        assert!(close(forward.map_to_unit(0.0, 0.0, 10.0), 0.0));
        assert!(close(reverse.map_to_unit(0.0, 0.0, 10.0), 1.0));
        assert!(close(reverse.map_to_unit(10.0, 0.0, 10.0), 0.0));
        assert!(close(reverse.map_to_unit(2.5, 0.0, 10.0), 0.75));
    }

    #[test]
    fn reverse_composes_with_log() {
        let m = Mapping::new(Transform::Log, Order::Reverse);
        assert!(close(m.map_to_unit(1.0, 1.0, 1000.0), 1.0));
        assert!(close(m.map_to_unit(10.0, 1.0, 1000.0), 2.0 / 3.0));
        assert!(close(m.map_to_unit(1000.0, 1.0, 1000.0), 0.0));
    }

    #[test]
    fn broken_mapping_closes_the_gap_and_inverts() {
        let m = Mapping::new(Transform::Linear, Order::Forward).with_gap(Break::new(100.0, 900.0));
        // 0..100 and 900..1000 share the axis less the gap.
        assert!(close(m.map_to_unit(0.0, 0.0, 1000.0), 0.0));
        assert!(close(m.map_to_unit(1000.0, 0.0, 1000.0), 1.0));
        let half = (1.0 - Break::GAP) / 2.0;
        assert!(close(m.map_to_unit(100.0, 0.0, 1000.0), half));
        assert!(close(m.map_to_unit(900.0, 0.0, 1000.0), half + Break::GAP));
        assert!(close(m.gap_share(0.0, 1000.0), 0.8));
        let (start, end) = m.gap_units(0.0, 1000.0).expect("inside");
        assert!(close(start, half) && close(end, half + Break::GAP));
        for value in [0.0, 50.0, 100.0, 500.0, 900.0, 950.0] {
            let back = m.unit_to_value(m.map_to_unit(value, 0.0, 1000.0), 0.0, 1000.0);
            assert!((back - value).abs() < 1e-9, "{value} -> {back}");
        }
        let reversed = Mapping {
            order: Order::Reverse,
            ..m
        };
        assert!(close(reversed.map_to_unit(1000.0, 0.0, 1000.0), 0.0));
        let (start, end) = reversed.gap_units(0.0, 1000.0).expect("inside");
        assert!(close(start, 1.0 - half - Break::GAP) && close(end, 1.0 - half));
        // A break outside the domain changes nothing.
        assert!(close(m.map_to_unit(50.0, 0.0, 90.0), 50.0 / 90.0));
    }

    #[test]
    fn unit_to_value_inverts_in_both_directions() {
        for transform in [Transform::Linear, Transform::Log, Transform::Sqrt] {
            for order in [Order::Forward, Order::Reverse] {
                let m = Mapping::new(transform, order);
                for value in [1.0, 3.0, 40.0, 250.0, 1000.0] {
                    let unit = m.map_to_unit(value, 1.0, 1000.0);
                    let back = m.unit_to_value(unit, 1.0, 1000.0);
                    assert!(
                        (back - value).abs() < 1e-9 * value,
                        "{transform:?} {order:?}: {value} -> {unit} -> {back}"
                    );
                }
            }
        }
    }
}
