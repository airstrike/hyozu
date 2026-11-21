//! Axis breaks: a range of values an axis leaves out.

/// A range of values a broken axis leaves out.
///
/// The axis keeps one scale on both sides of the break and closes the
/// range up into a narrow gap, marked on the spine with a pair of slashes.
/// Marks are cut at the gap, so a bar that spans it shows both ends. Use
/// it when a few values dwarf the rest: a break keeps the small values
/// readable without a log scale.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Break {
    from: f64,
    to: f64,
}

impl Break {
    /// Share of the axis the gap takes, whatever range it stands for.
    pub(crate) const GAP: f64 = 0.03;

    /// The values strictly between `from` and `to`, in either order.
    /// Returns `None` for an empty or non-finite range.
    pub fn new(from: f64, to: f64) -> Option<Self> {
        let (from, to) = (from.min(to), from.max(to));
        (from.is_finite() && to.is_finite() && from < to).then_some(Self { from, to })
    }

    /// The lower end of the left-out range.
    pub fn from(&self) -> f64 {
        self.from
    }

    /// The upper end of the left-out range.
    pub fn to(&self) -> f64 {
        self.to
    }

    /// Whether `value` falls inside the left-out range, ends excluded.
    pub fn contains(&self, value: f64) -> bool {
        self.from < value && value < self.to
    }
}

/// A break placed on one axis's unit interval: where the left-out range
/// starts and ends before the axis closes it up.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Squeeze {
    start: f64,
    end: f64,
    /// How much the kept ranges shrink to make room for the gap.
    scale: f64,
}

impl Squeeze {
    /// Places a break whose ends map to `start` and `end` on the unit
    /// interval. `None` when the break misses the domain, or covers all of
    /// it.
    pub(crate) fn new(start: f64, end: f64) -> Option<Self> {
        let (start, end) = (start.clamp(0.0, 1.0), end.clamp(0.0, 1.0));
        let kept = 1.0 - (end - start);
        (end > start && kept > 0.0).then(|| Self {
            start,
            end,
            scale: (1.0 - Break::GAP) / kept,
        })
    }

    /// Share of the unit interval the left-out range covered.
    pub(crate) fn width(&self) -> f64 {
        self.end - self.start
    }

    /// Closes the break up: kept values shrink by one scale on both sides,
    /// and the left-out range shrinks into the gap.
    pub(crate) fn apply(&self, unit: f64) -> f64 {
        if unit <= self.start {
            unit * self.scale
        } else if unit >= self.end {
            1.0 - (1.0 - unit) * self.scale
        } else {
            self.start * self.scale + (unit - self.start) / self.width() * Break::GAP
        }
    }

    /// Inverse of [`Squeeze::apply`].
    pub(crate) fn invert(&self, unit: f64) -> f64 {
        let gap_start = self.start * self.scale;
        if unit <= gap_start {
            unit / self.scale
        } else if unit >= gap_start + Break::GAP {
            1.0 - (1.0 - unit) / self.scale
        } else {
            self.start + (unit - gap_start) / Break::GAP * self.width()
        }
    }

    /// The gap's ends on the closed-up unit interval.
    pub(crate) fn gap(&self) -> (f64, f64) {
        let start = self.start * self.scale;
        (start, start + Break::GAP)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn break_orders_its_ends_and_rejects_empty_ranges() {
        let b = Break::new(900.0, 120.0).expect("range");
        assert_eq!((b.from(), b.to()), (120.0, 900.0));
        assert!(b.contains(500.0) && !b.contains(120.0) && !b.contains(1000.0));
        assert_eq!(Break::new(5.0, 5.0), None);
        assert_eq!(Break::new(f64::NAN, 5.0), None);
    }

    #[test]
    fn squeeze_keeps_one_scale_on_both_sides() {
        // Leave out the middle half: the kept quarters stretch alike over
        // what the gap leaves.
        let s = Squeeze::new(0.25, 0.75).expect("inside");
        let scale = (1.0 - Break::GAP) / 0.5;
        assert!(close(s.apply(0.0), 0.0));
        assert!(close(s.apply(1.0), 1.0));
        assert!(close(s.apply(0.25), 0.25 * scale));
        assert!(close(s.apply(0.75), 0.25 * scale + Break::GAP));
        assert!(close(s.apply(0.1) - s.apply(0.0), s.apply(0.9) - s.apply(0.8)));
        let (start, end) = s.gap();
        assert!(close(end - start, Break::GAP));
    }

    #[test]
    fn squeeze_inverts() {
        let s = Squeeze::new(0.1, 0.6).expect("inside");
        for unit in [0.0, 0.05, 0.1, 0.3, 0.6, 0.8, 1.0] {
            assert!(close(s.invert(s.apply(unit)), unit), "{unit}");
        }
    }

    #[test]
    fn squeeze_needs_a_break_inside_the_domain_that_leaves_something() {
        assert_eq!(Squeeze::new(1.2, 1.5), None);
        assert_eq!(Squeeze::new(-0.5, 1.5), None);
        assert!(Squeeze::new(-0.5, 0.5).is_some());
    }
}
