//! The path a line or area envelope takes between consecutive points.
//!
//! Interpolation runs in pixel space, after the scales have mapped each
//! datum, so a monotone curve stays monotone on screen and a step lands on
//! pixel corners. It only changes the drawn path: hover, markers and labels
//! keep reading the original points.

use crate::core::Point;

/// Cubic segments are flattened into this many chords each.
const SAMPLES: usize = 16;

/// How a path travels between consecutive points.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum Interpolation {
    /// Straight segments (default).
    #[default]
    Linear,
    /// Monotone cubic in x (Steffen's method, d3 `curveMonotoneX`): smooth,
    /// and between two points it never leaves their value range, so it
    /// cannot invent a peak or a trough the data does not have.
    Monotone,
    /// Natural cubic spline: twice continuously differentiable with zero
    /// curvature at both ends. Smoother than [`Monotone`](Self::Monotone),
    /// but it can overshoot the data between points.
    Natural,
    /// Piecewise constant.
    Step(Step),
}

/// Where a step's vertical jump sits between two points.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Step {
    /// The value changes at the start of the interval, so each point's
    /// value holds over the interval that ends at it.
    Before,
    /// The value holds until the next point, then jumps.
    After,
    /// The value jumps halfway between the two points.
    Mid,
}

impl From<Step> for Interpolation {
    fn from(step: Step) -> Self {
        Self::Step(step)
    }
}

impl Interpolation {
    /// Traces the path through one contiguous run of finite pixel points.
    ///
    /// The result starts at the first point and ends at the last, and passes
    /// through every point in between. Linear returns the run unchanged, a
    /// step adds one corner per interval (two for [`Step::Mid`]), and the
    /// cubic curves add [`SAMPLES`] − 1 chords per interval.
    pub(crate) fn trace(self, run: &[Point]) -> Vec<Point> {
        if run.len() < 2 {
            return run.to_vec();
        }
        match self {
            Self::Linear => run.to_vec(),
            Self::Step(step) => step.trace(run),
            Self::Monotone if run.len() > 2 => flatten(run, &monotone_controls(run)),
            Self::Natural if run.len() > 2 => flatten(run, &natural_controls(run)),
            Self::Monotone | Self::Natural => run.to_vec(),
        }
    }
}

impl Step {
    fn trace(self, run: &[Point]) -> Vec<Point> {
        let corners = match self {
            Self::Before | Self::After => 1,
            Self::Mid => 2,
        };
        let mut path = Vec::with_capacity(1 + (run.len() - 1) * (corners + 1));
        path.push(run[0]);
        for pair in run.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            match self {
                Self::Before => path.push(Point::new(a.x, b.y)),
                Self::After => path.push(Point::new(b.x, a.y)),
                Self::Mid => {
                    let x = (a.x + b.x) / 2.0;
                    path.extend([Point::new(x, a.y), Point::new(x, b.y)]);
                }
            }
            path.push(b);
        }
        path
    }
}

/// The two inner Bézier control points of each interval.
type Controls = Vec<(Point, Point)>;

/// Steffen's monotone tangents, as d3's `curveMonotoneX` computes them.
/// Each interior tangent is zero at a local extremum and otherwise bounded
/// by the neighboring secants, which keeps every segment inside the range of
/// its endpoints.
fn monotone_controls(run: &[Point]) -> Controls {
    let n = run.len();
    let secant = |i: usize| {
        let h = run[i + 1].x - run[i].x;
        finite_or_zero((run[i + 1].y - run[i].y) / h)
    };
    let mut tangents = vec![0.0_f32; n];
    for i in 1..n - 1 {
        let (h0, h1) = (run[i].x - run[i - 1].x, run[i + 1].x - run[i].x);
        let (s0, s1) = (secant(i - 1), secant(i));
        let p = (s0 * h1 + s1 * h0) / (h0 + h1);
        let bound = s0.abs().min(s1.abs()).min(0.5 * p.abs());
        tangents[i] = finite_or_zero((sign(s0) + sign(s1)) * bound);
    }
    // One-sided endpoint tangents: the parabola through the secant and the
    // neighbor's tangent.
    tangents[0] = (3.0 * secant(0) - tangents[1]) / 2.0;
    tangents[n - 1] = (3.0 * secant(n - 2) - tangents[n - 2]) / 2.0;

    run.windows(2)
        .enumerate()
        .map(|(i, pair)| {
            let (a, b) = (pair[0], pair[1]);
            let third = (b.x - a.x) / 3.0;
            (
                Point::new(a.x + third, a.y + third * tangents[i]),
                Point::new(b.x - third, b.y - third * tangents[i + 1]),
            )
        })
        .collect()
}

/// A natural cubic spline in each coordinate, solved as d3's `curveNatural`
/// does: a tridiagonal system for the first control point of every interval.
fn natural_controls(run: &[Point]) -> Controls {
    let xs = natural_axis(&run.iter().map(|p| p.x).collect::<Vec<_>>());
    let ys = natural_axis(&run.iter().map(|p| p.y).collect::<Vec<_>>());
    xs.into_iter()
        .zip(ys)
        .map(|((ax, bx), (ay, by))| (Point::new(ax, ay), Point::new(bx, by)))
        .collect()
}

/// Control coordinates for one axis of a natural spline through `x`, which
/// holds at least three values.
fn natural_axis(x: &[f32]) -> Vec<(f32, f32)> {
    let n = x.len() - 1;
    let mut a = vec![0.0_f32; n];
    let mut b = vec![0.0_f32; n];
    let mut r = vec![0.0_f32; n];
    b[0] = 2.0;
    r[0] = x[0] + 2.0 * x[1];
    for i in 1..n - 1 {
        a[i] = 1.0;
        b[i] = 4.0;
        r[i] = 4.0 * x[i] + 2.0 * x[i + 1];
    }
    a[n - 1] = 2.0;
    b[n - 1] = 7.0;
    r[n - 1] = 8.0 * x[n - 1] + x[n];
    for i in 1..n {
        let m = a[i] / b[i - 1];
        b[i] -= m;
        r[i] -= m * r[i - 1];
    }
    let mut first = vec![0.0_f32; n];
    first[n - 1] = r[n - 1] / b[n - 1];
    for i in (0..n - 1).rev() {
        first[i] = (r[i] - first[i + 1]) / b[i];
    }
    (0..n)
        .map(|i| {
            let second = if i == n - 1 {
                (x[n] + first[n - 1]) / 2.0
            } else {
                2.0 * x[i + 1] - first[i + 1]
            };
            (first[i], second)
        })
        .collect()
}

/// Flattens one cubic Bézier per interval into chords, landing exactly on
/// every data point.
fn flatten(run: &[Point], controls: &[(Point, Point)]) -> Vec<Point> {
    let mut path = Vec::with_capacity(1 + controls.len() * SAMPLES);
    path.push(run[0]);
    for (pair, &(c1, c2)) in run.windows(2).zip(controls) {
        let (a, b) = (pair[0], pair[1]);
        for k in 1..SAMPLES {
            let t = k as f32 / SAMPLES as f32;
            let u = 1.0 - t;
            let (w0, w1, w2, w3) = (u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t);
            path.push(Point::new(
                w0 * a.x + w1 * c1.x + w2 * c2.x + w3 * b.x,
                w0 * a.y + w1 * c1.y + w2 * c2.y + w3 * b.y,
            ));
        }
        path.push(b);
    }
    path
}

/// d3's sign: zero counts as positive, so a flat secant next to a rising one
/// still sums to a nonzero factor that the zero bound then cancels.
fn sign(x: f32) -> f32 {
    if x < 0.0 { -1.0 } else { 1.0 }
}

fn finite_or_zero(x: f32) -> f32 {
    if x.is_finite() { x } else { 0.0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn points(ys: &[f32]) -> Vec<Point> {
        ys.iter()
            .enumerate()
            .map(|(i, &y)| Point::new(i as f32 * 40.0, y))
            .collect()
    }

    /// Plateaus and a sharp spike, where an unconstrained cubic overshoots.
    const SPIKY: [f32; 9] = [100.0, 100.0, 100.0, 20.0, 20.0, 180.0, 175.0, 60.0, 60.0];

    #[test]
    fn monotone_never_overshoots_its_neighbors() {
        let run = points(&SPIKY);
        let path = Interpolation::Monotone.trace(&run);
        assert_eq!(path.len(), 1 + (run.len() - 1) * SAMPLES);
        for (i, pair) in run.windows(2).enumerate() {
            let (lo, hi) = (pair[0].y.min(pair[1].y), pair[0].y.max(pair[1].y));
            for p in &path[i * SAMPLES..=(i + 1) * SAMPLES] {
                assert!(
                    p.y >= lo - 1e-3 && p.y <= hi + 1e-3,
                    "segment {i} left [{lo}, {hi}] at {p:?}"
                );
            }
        }
        let (lo, hi) = run.iter().fold((f32::INFINITY, f32::NEG_INFINITY), |(lo, hi), p| {
            (lo.min(p.y), hi.max(p.y))
        });
        assert!(path.iter().all(|p| p.y >= lo - 1e-3 && p.y <= hi + 1e-3));
    }

    #[test]
    fn monotone_holds_plateaus_flat() {
        let path = Interpolation::Monotone.trace(&points(&SPIKY));
        assert!(path[..=SAMPLES].iter().all(|p| (p.y - 100.0).abs() < 1e-3));
    }

    #[test]
    fn natural_overshoots_where_monotone_does_not() {
        let run = points(&SPIKY);
        let path = Interpolation::Natural.trace(&run);
        assert_eq!(path.len(), 1 + (run.len() - 1) * SAMPLES);
        let hi = SPIKY.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        let lo = SPIKY.iter().copied().fold(f32::INFINITY, f32::min);
        assert!(path.iter().any(|p| p.y > hi + 1.0 || p.y < lo - 1.0));
    }

    #[test]
    fn cubic_curves_pass_through_every_point() {
        let run = points(&SPIKY);
        for curve in [Interpolation::Monotone, Interpolation::Natural] {
            let path = curve.trace(&run);
            for (i, p) in run.iter().enumerate() {
                assert_eq!(path[i * SAMPLES], *p, "{curve:?} misses point {i}");
            }
        }
    }

    #[test]
    fn step_vertex_counts() {
        let run = points(&[1.0, 4.0, 2.0, 8.0, 5.0]);
        let n = run.len();
        assert_eq!(Interpolation::Linear.trace(&run).len(), n);
        assert_eq!(Interpolation::Step(Step::Before).trace(&run).len(), 2 * n - 1);
        assert_eq!(Interpolation::Step(Step::After).trace(&run).len(), 2 * n - 1);
        assert_eq!(Interpolation::Step(Step::Mid).trace(&run).len(), 3 * n - 2);
    }

    #[test]
    fn step_segments_are_axis_aligned() {
        let run = points(&[1.0, 4.0, 2.0, 8.0]);
        for step in [Step::Before, Step::After, Step::Mid] {
            let path = Interpolation::Step(step).trace(&run);
            assert!(
                path.windows(2).all(|w| w[0].x == w[1].x || w[0].y == w[1].y),
                "{step:?} has a diagonal"
            );
        }
        let before = Interpolation::Step(Step::Before).trace(&run);
        assert_eq!(before[1], Point::new(0.0, 4.0));
        let after = Interpolation::Step(Step::After).trace(&run);
        assert_eq!(after[1], Point::new(40.0, 1.0));
        let mid = Interpolation::Step(Step::Mid).trace(&run);
        assert_eq!(&mid[1..3], &[Point::new(20.0, 1.0), Point::new(20.0, 4.0)]);
    }

    #[test]
    fn short_runs_stay_straight() {
        let two = points(&[1.0, 9.0]);
        assert_eq!(Interpolation::Monotone.trace(&two), two);
        assert_eq!(Interpolation::Natural.trace(&two), two);
        let one = points(&[3.0]);
        assert_eq!(Interpolation::Step(Step::Mid).trace(&one), one);
    }

    #[test]
    fn duplicate_x_stays_finite() {
        let run = vec![
            Point::new(0.0, 0.0),
            Point::new(10.0, 5.0),
            Point::new(10.0, 9.0),
            Point::new(20.0, 2.0),
        ];
        let path = Interpolation::Monotone.trace(&run);
        assert!(path.iter().all(|p| p.x.is_finite() && p.y.is_finite()));
    }
}
