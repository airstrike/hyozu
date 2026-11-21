//! How one axis carries data onto the unit interval: a [`Transform`] plus
//! a direction.

use crate::data::axis::Order;
use crate::scale::Transform;

/// One axis's data-to-unit mapping.
///
/// The [`Transform`] shapes the spacing (linear, log, sqrt); the [`Order`]
/// says which end of the unit interval the domain minimum lands on.
/// [`Order::Reverse`] puts the minimum at `1.0`, so a reversed y-axis
/// grows downward and a reversed x-axis grows leftward. Plot-area marks,
/// gridlines and axis guides all map through the same `Mapping`, so a
/// reversed or log axis stays aligned with its ticks by construction.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Mapping {
    /// Spacing of values along the axis.
    pub transform: Transform,
    /// Which end of the axis the domain minimum sits on.
    pub order: Order,
}

impl Mapping {
    /// Builds a mapping from a transform and a direction.
    pub fn new(transform: Transform, order: Order) -> Self {
        Self { transform, order }
    }

    /// Maps `value` in `[min, max]` onto the unit interval, honoring the
    /// transform and then the direction.
    pub fn map_to_unit(&self, value: f64, min: f64, max: f64) -> f64 {
        let unit = self.transform.map_to_unit(value, min, max);
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
        self.transform.unit_to_value(unit, min, max)
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
