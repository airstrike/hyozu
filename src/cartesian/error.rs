//! Excel's series error-bar amounts. Each amount turns a series into the
//! [`Interval`]s that [`error_bars`](super::error_bars) and
//! [`horizontal_ranges`](super::horizontal_ranges) draw.
use std::fmt;

use super::Interval;

/// How far each error bar reaches from its value.
#[derive(Clone, Debug, PartialEq)]
pub enum Amount {
    /// The same distance at every point.
    Fixed(Magnitude),
    /// A percentage of each point's absolute value, so 5 means 5%.
    Percent(Magnitude),
    /// This many sample standard deviations of the series. Like Excel, every
    /// bar is centered on the series mean rather than on its own point.
    StdDev(Magnitude),
    /// The standard error of the series mean, s / √n, around each point.
    StdErr,
    /// Separate plus and minus distances, per point or one pair for all.
    Custom(Custom),
}

/// Which side of the value an error bar covers.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Direction {
    #[default]
    Both,
    Plus,
    Minus,
}

/// A finite, nonnegative error distance or multiplier.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct Magnitude(f64);

/// Plus and minus distances: one pair per series sample, or, like Excel's
/// single-value custom amount, one pair that every sample shares.
#[derive(Clone, Debug, PartialEq)]
pub struct Custom {
    plus: Distances,
    minus: Distances,
}

/// One side's custom distances.
#[derive(Clone, Debug, PartialEq)]
enum Distances {
    /// One distance per sample, in input order.
    Each(Vec<Magnitude>),
    /// The same distance at every sample.
    All(Magnitude),
}

/// Why error amounts could not be computed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Error {
    /// Distances and multipliers must be finite and nonnegative.
    Magnitude(f64),
    /// Custom plus and minus lists, or a custom list and its series, differ in length.
    Length { expected: usize, found: usize },
    /// Standard deviation and standard error need two finite values.
    TooFewPoints(usize),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Magnitude(v) => write!(f, "error amount {v} is not a finite nonnegative number"),
            Self::Length { expected, found } => {
                write!(f, "expected {expected} custom error amounts, found {found}")
            }
            Self::TooFewPoints(n) => write!(f, "error amount needs two finite values, found {n}"),
        }
    }
}

impl std::error::Error for Error {}

impl Magnitude {
    pub fn new(value: f64) -> Result<Self, Error> {
        if value.is_finite() && value >= 0. {
            Ok(Self(value))
        } else {
            Err(Error::Magnitude(value))
        }
    }

    pub fn get(self) -> f64 {
        self.0
    }
}

impl Custom {
    /// Pair the plus and minus distances for each point in order.
    pub fn new(plus: &[f64], minus: &[f64]) -> Result<Self, Error> {
        if plus.len() != minus.len() {
            return Err(Error::Length {
                expected: plus.len(),
                found: minus.len(),
            });
        }
        let magnitudes = |v: &[f64]| v.iter().map(|&v| Magnitude::new(v)).collect::<Result<Vec<_>, _>>();
        Ok(Self {
            plus: Distances::Each(magnitudes(plus)?),
            minus: Distances::Each(magnitudes(minus)?),
        })
    }

    /// One plus and one minus distance for every point, which may differ,
    /// unlike [`Amount::Fixed`].
    pub fn uniform(plus: f64, minus: f64) -> Result<Self, Error> {
        Ok(Self {
            plus: Distances::All(Magnitude::new(plus)?),
            minus: Distances::All(Magnitude::new(minus)?),
        })
    }

    /// How many points the distances are listed for, or `None` when one
    /// pair covers every point.
    pub fn len(&self) -> Option<usize> {
        match &self.plus {
            Distances::Each(each) => Some(each.len()),
            Distances::All(_) => None,
        }
    }

    /// Whether the distances are listed for no points at all.
    pub fn is_empty(&self) -> bool {
        self.len() == Some(0)
    }

    /// The plus and minus distances at sample `index`.
    fn at(&self, index: usize) -> (f64, f64) {
        (self.plus.at(index), self.minus.at(index))
    }
}

impl Distances {
    /// The distance at sample `index`. The caller has checked an `Each`
    /// list against the series length.
    fn at(&self, index: usize) -> f64 {
        match self {
            Self::Each(each) => each.get(index).map_or(0.0, |m| m.0),
            Self::All(all) => all.0,
        }
    }
}

impl Amount {
    /// Y error bars at each finite sample, in input order.
    pub fn vertical(&self, series: &[[f64; 2]], direction: Direction) -> Result<Vec<Interval>, Error> {
        self.intervals(series, direction, 1)
    }

    /// X error bars at each finite sample, in input order. Each interval's
    /// `x` holds the sample's y, as [`horizontal_ranges`](super::horizontal_ranges) expects.
    pub fn horizontal(&self, series: &[[f64; 2]], direction: Direction) -> Result<Vec<Interval>, Error> {
        self.intervals(series, direction, 0)
    }

    fn intervals(&self, series: &[[f64; 2]], direction: Direction, axis: usize) -> Result<Vec<Interval>, Error> {
        if let Self::Custom(custom) = self
            && let Some(found) = custom.len()
            && found != series.len()
        {
            return Err(Error::Length {
                expected: series.len(),
                found,
            });
        }
        let values: Vec<_> = series
            .iter()
            .filter(|p| p.iter().all(|v| v.is_finite()))
            .map(|p| p[axis])
            .collect();
        let spread = Spread::new(&values);
        if matches!(self, Self::StdDev(_) | Self::StdErr) {
            spread?;
        }
        series
            .iter()
            .enumerate()
            .filter(|(_, p)| p.iter().all(|v| v.is_finite()))
            .map(|(i, p)| {
                let value = p[axis];
                let (center, plus, minus) = match self {
                    Self::Fixed(d) => (value, d.0, d.0),
                    Self::Percent(r) => (value, value.abs() * r.0 / 100., value.abs() * r.0 / 100.),
                    Self::StdDev(k) => {
                        let s = spread?;
                        (s.mean, k.0 * s.deviation, k.0 * s.deviation)
                    }
                    Self::StdErr => (value, spread?.error, spread?.error),
                    Self::Custom(c) => {
                        let (plus, minus) = c.at(i);
                        (value, plus, minus)
                    }
                };
                let (plus, minus) = match direction {
                    Direction::Both => (plus, minus),
                    Direction::Plus => (plus, 0.),
                    Direction::Minus => (0., minus),
                };
                Ok(Interval {
                    x: p[1 - axis],
                    low: center - minus,
                    high: center + plus,
                })
            })
            .collect()
    }
}

/// Vertical ranges with symmetric caps, like [`error_bars`](super::error_bars),
/// under a series name for the legend and hover.
pub fn bars(name: &str, intervals: &[Interval], cap_width: f64) -> crate::Diagram {
    super::capped(name, intervals, cap_width)
}

/// Sample mean, standard deviation and standard error of the mean.
#[derive(Clone, Copy, Debug)]
struct Spread {
    mean: f64,
    deviation: f64,
    error: f64,
}

impl Spread {
    fn new(values: &[f64]) -> Result<Self, Error> {
        if values.len() < 2 {
            return Err(Error::TooFewPoints(values.len()));
        }
        let n = values.len() as f64;
        let mean = values.iter().sum::<f64>() / n;
        let deviation = (values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (n - 1.)).sqrt();
        Ok(Self {
            mean,
            deviation,
            error: deviation / n.sqrt(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// y = [2, 4, 4, 4, 5, 5, 7, 9] at x = 1..8. Mean 5, Σ(y - 5)² = 32,
    /// sample variance 32 / 7, s = √(32 / 7) and s / √8 = √(4 / 7).
    fn series() -> Vec<[f64; 2]> {
        [2., 4., 4., 4., 5., 5., 7., 9.]
            .iter()
            .enumerate()
            .map(|(i, &y)| [i as f64 + 1., y])
            .collect()
    }

    fn close(a: f64, b: f64) {
        assert!((a - b).abs() <= 1e-12, "{a} differs from {b}");
    }

    fn magnitude(v: f64) -> Magnitude {
        Magnitude::new(v).unwrap()
    }

    #[test]
    fn fixed_and_percent_surround_each_point() {
        let fixed = Amount::Fixed(magnitude(1.5))
            .vertical(&series(), Direction::Both)
            .unwrap();
        for (v, [x, y]) in fixed.iter().zip(series()) {
            assert_eq!((v.x, v.low, v.high), (x, y - 1.5, y + 1.5));
        }
        let percent = Amount::Percent(magnitude(10.))
            .vertical(&[[1., -20.], [2., 30.]], Direction::Both)
            .unwrap();
        close(percent[0].low, -22.);
        close(percent[0].high, -18.);
        close(percent[1].low, 27.);
        close(percent[1].high, 33.);
    }

    #[test]
    fn standard_deviation_is_centered_on_the_series_mean() {
        let s = (32f64 / 7.).sqrt();
        let bars = Amount::StdDev(magnitude(2.))
            .vertical(&series(), Direction::Both)
            .unwrap();
        assert_eq!(bars.len(), 8);
        for v in &bars {
            close(v.low, 5. - 2. * s);
            close(v.high, 5. + 2. * s);
        }
    }

    #[test]
    fn standard_error_surrounds_each_point() {
        let se = (4f64 / 7.).sqrt();
        let bars = Amount::StdErr.vertical(&series(), Direction::Both).unwrap();
        for (v, [_, y]) in bars.iter().zip(series()) {
            close(v.low, y - se);
            close(v.high, y + se);
        }
        assert_eq!(
            Amount::StdErr.vertical(&[[1., 2.], [2., f64::NAN]], Direction::Both),
            Err(Error::TooFewPoints(1))
        );
    }

    #[test]
    fn custom_amounts_follow_input_order_across_skipped_samples() {
        let custom = Custom::new(&[1., 2., 3.], &[0.5, 0.25, 0.]).unwrap();
        let bars = Amount::Custom(custom)
            .vertical(&[[1., 10.], [2., f64::NAN], [3., 30.]], Direction::Both)
            .unwrap();
        assert_eq!(bars.len(), 2);
        assert_eq!((bars[0].low, bars[0].high), (9.5, 11.));
        assert_eq!((bars[1].low, bars[1].high), (30., 33.));
        assert_eq!(
            Custom::new(&[1.], &[1., 2.]),
            Err(Error::Length { expected: 1, found: 2 })
        );
        assert_eq!(Custom::new(&[-1.], &[1.]), Err(Error::Magnitude(-1.)));
        let short = Amount::Custom(Custom::new(&[1.], &[1.]).unwrap());
        assert_eq!(
            short.vertical(&series(), Direction::Both),
            Err(Error::Length { expected: 8, found: 1 })
        );
    }

    #[test]
    fn uniform_custom_amount_applies_one_pair_to_every_point() {
        let uniform = Custom::uniform(2., 0.5).unwrap();
        assert_eq!(uniform.len(), None);
        assert!(!uniform.is_empty());
        let bars = Amount::Custom(uniform).vertical(&series(), Direction::Both).unwrap();
        assert_eq!(bars.len(), 8);
        for (v, [_, y]) in bars.iter().zip(series()) {
            assert_eq!((v.low, v.high), (y - 0.5, y + 2.));
        }
        assert_eq!(Custom::uniform(1., -1.), Err(Error::Magnitude(-1.)));
        assert_eq!(Custom::new(&[], &[]).map(|c| c.is_empty()), Ok(true));
    }

    #[test]
    fn direction_keeps_one_side() {
        let amount = Amount::Fixed(magnitude(1.));
        let plus = amount.vertical(&[[0., 3.]], Direction::Plus).unwrap();
        let minus = amount.vertical(&[[0., 3.]], Direction::Minus).unwrap();
        assert_eq!((plus[0].low, plus[0].high), (3., 4.));
        assert_eq!((minus[0].low, minus[0].high), (2., 3.));
    }

    #[test]
    fn horizontal_bars_measure_x_and_sit_at_y() {
        // x = [1, 2, 3]: mean 2, s = 1, s / √3.
        let p = [[1., 10.], [2., 20.], [3., 30.]];
        let bars = Amount::StdErr.horizontal(&p, Direction::Both).unwrap();
        let se = 1. / 3f64.sqrt();
        for (v, [x, y]) in bars.iter().zip(p) {
            assert_eq!(v.x, y);
            close(v.low, x - se);
            close(v.high, x + se);
        }
        let sd = Amount::StdDev(magnitude(1.)).horizontal(&p, Direction::Both).unwrap();
        close(sd[0].low, 1.);
        close(sd[0].high, 3.);
    }

    #[test]
    fn magnitudes_reject_negative_and_nonfinite_values() {
        assert_eq!(Magnitude::new(-0.5), Err(Error::Magnitude(-0.5)));
        assert!(Magnitude::new(f64::NAN).is_err());
        assert_eq!(Magnitude::new(0.).map(Magnitude::get), Ok(0.));
    }
}
