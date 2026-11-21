//! What a line or area draws where a value is missing.

use crate::core::Point;

/// How a line or area treats a point whose value is missing (non-finite),
/// Excel's "Show empty cells as".
///
/// The policy shapes only the drawn path. Hover, markers and labels skip a
/// missing point under every policy.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum Nulls {
    /// Break the path, leaving a gap (default).
    #[default]
    Gap,
    /// Draw the missing value as zero: a line dips to the zero baseline and
    /// an area collapses onto its floor there.
    Zero,
    /// Bridge the gap, joining the points on either side.
    Connect,
}

/// One contiguous stretch of a path, with every vertex finite.
///
/// `upper` is the drawn edge; `lower` is the floor under it, index for index
/// (an area's baseline, or a line's zero baseline).
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct Run {
    pub(crate) upper: Vec<Point>,
    pub(crate) lower: Vec<Point>,
}

impl Nulls {
    /// Splits an envelope into the runs this policy draws.
    ///
    /// A point is missing when its upper vertex is non-finite. A column whose
    /// floor is non-finite (its x is missing) has nowhere to stand, so it
    /// breaks the path under every policy except [`Connect`](Self::Connect),
    /// which skips it. Runs of a single vertex are kept, since an area or a
    /// marker can still use them; a stroke ignores them.
    pub(crate) fn runs(self, upper: &[Point], lower: &[Point]) -> Vec<Run> {
        let mut runs = Vec::new();
        let mut run = Run::default();
        for (&u, &l) in upper.iter().zip(lower) {
            let column = if !finite(l) {
                None
            } else if finite(u) {
                Some(u)
            } else {
                match self {
                    Self::Gap => None,
                    Self::Zero => Some(l),
                    Self::Connect => continue,
                }
            };
            match column {
                Some(u) => {
                    run.upper.push(u);
                    run.lower.push(l);
                }
                None if self == Self::Connect => {}
                None => {
                    if !run.upper.is_empty() {
                        runs.push(std::mem::take(&mut run));
                    }
                }
            }
        }
        if !run.upper.is_empty() {
            runs.push(run);
        }
        runs
    }
}

fn finite(p: Point) -> bool {
    p.x.is_finite() && p.y.is_finite()
}

#[cfg(test)]
mod tests {
    use super::*;

    const FLOOR: f32 = 100.0;

    fn envelope(ys: &[f32]) -> (Vec<Point>, Vec<Point>) {
        let upper = ys
            .iter()
            .enumerate()
            .map(|(i, &y)| Point::new(i as f32 * 10.0, y))
            .collect::<Vec<_>>();
        let lower = upper.iter().map(|p| Point::new(p.x, FLOOR)).collect();
        (upper, lower)
    }

    #[test]
    fn gap_splits_at_each_missing_point() {
        let (upper, lower) = envelope(&[10.0, 20.0, f32::NAN, 30.0, f32::NAN, f32::NAN, 40.0]);
        let runs = Nulls::Gap.runs(&upper, &lower);
        let lengths: Vec<_> = runs.iter().map(|r| r.upper.len()).collect();
        assert_eq!(lengths, [2, 1, 1]);
        assert_eq!(runs[1].upper[0], Point::new(30.0, 30.0));
    }

    #[test]
    fn connect_bridges_missing_points() {
        let (upper, lower) = envelope(&[10.0, f32::NAN, f32::NAN, 30.0, 40.0]);
        let runs = Nulls::Connect.runs(&upper, &lower);
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].upper, [
            Point::new(0.0, 10.0),
            Point::new(30.0, 30.0),
            Point::new(40.0, 40.0)
        ]);
        assert_eq!(runs[0].lower.len(), 3);
    }

    #[test]
    fn zero_drops_missing_points_to_the_floor() {
        let (upper, lower) = envelope(&[10.0, f32::NAN, 30.0]);
        let runs = Nulls::Zero.runs(&upper, &lower);
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].upper[1], Point::new(10.0, FLOOR));
    }

    #[test]
    fn a_missing_column_breaks_every_policy_but_connect() {
        let (mut upper, mut lower) = envelope(&[10.0, 20.0, 30.0]);
        upper[1].x = f32::NAN;
        lower[1].x = f32::NAN;
        assert_eq!(Nulls::Gap.runs(&upper, &lower).len(), 2);
        assert_eq!(Nulls::Zero.runs(&upper, &lower).len(), 2);
        assert_eq!(Nulls::Connect.runs(&upper, &lower).len(), 1);
    }

    #[test]
    fn all_missing_yields_no_runs() {
        let (upper, lower) = envelope(&[f32::NAN, f32::NAN]);
        assert!(Nulls::Gap.runs(&upper, &lower).is_empty());
        assert!(Nulls::Connect.runs(&upper, &lower).is_empty());
    }
}
