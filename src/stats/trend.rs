//! Excel chart trendlines. Each regression is ordinary least squares in
//! transformed coordinates, so coefficients and R² match `LINEST`, `LOGEST`
//! and Excel's trendline label:
//!
//! | Trendline   | Model               | Regression          |
//! |-------------|---------------------|---------------------|
//! | Linear      | y = m·x + b         | y on x              |
//! | Polynomial  | y = b + Σ cₖ·xᵏ     | y on x, …, xⁿ       |
//! | Exponential | y = a·e^(r·x)       | ln y on x           |
//! | Logarithmic | y = c·ln x + b      | y on ln x           |
//! | Power       | y = a·x^p           | ln y on ln x        |
//!
//! R² describes the regression in the transformed coordinates, as Excel
//! reports it. Nonfinite samples are skipped. Moving averages carry no fit.
mod regression;

use std::fmt;

use crate::{Line, line};
use regression::Regression;

/// Coefficients of a fitted trendline.
#[derive(Clone, Debug, PartialEq)]
pub enum Model {
    /// y = slope·x + intercept
    Linear { slope: f64, intercept: f64 },
    /// y = coefficients[0]·xⁿ + … + coefficients[n]. Highest power first,
    /// the order `LINEST` and NumPy's `polyfit` report.
    Polynomial { order: Order, coefficients: Vec<f64> },
    /// y = scale·e^(rate·x). `LOGEST` reports e^rate and scale.
    Exponential { scale: f64, rate: f64 },
    /// y = scale·ln(x) + intercept
    Logarithmic { scale: f64, intercept: f64 },
    /// y = scale·x^exponent
    Power { scale: f64, exponent: f64 },
}

/// Polynomial trendline order. Excel offers orders two through six.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Order {
    Two = 2,
    Three = 3,
    Four = 4,
    Five = 5,
    Six = 6,
}

/// Number of trailing points in each moving average, at least two.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Period(usize);

/// Distances in x units to extend a trendline before the first and after the
/// last observation. Both are finite and nonnegative.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Forecast {
    backward: f64,
    forward: f64,
}

/// A fitted trendline with its goodness of fit and observed x range.
#[derive(Clone, Debug)]
pub struct Fit {
    model: Model,
    regression: Regression,
    transform: Transform,
    domain: [f64; 2],
}

/// Why a trendline could not be fitted or drawn.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Error {
    /// Fewer distinct x positions than the model has free coefficients.
    /// With a fixed intercept, x = 0 does not count.
    TooFewPoints { required: usize, found: usize },
    /// Exponential and power trendlines need every y above zero.
    NonpositiveY([f64; 2]),
    /// Logarithmic and power trendlines need every x above zero.
    NonpositiveX([f64; 2]),
    /// A fixed intercept must be finite, and positive for an exponential.
    Intercept(f64),
    /// Forecast distances must be finite and nonnegative.
    Forecast { backward: f64, forward: f64 },
    /// A backward forecast would reach x ≤ 0 on a logarithmic or power trendline.
    ForecastDomain(f64),
    /// Polynomial orders run from two through six.
    Order(u8),
    /// A moving average spans at least two points.
    Period(usize),
    /// The design lost rank numerically, despite enough distinct positions.
    Singular,
    /// The trendline overflows over the drawn range.
    Overflow,
}

/// Coordinates the regression runs in.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Transform {
    Identity,
    LogX,
    LogY,
    LogBoth,
}

/// The trendline family being fitted.
#[derive(Clone, Copy, Debug)]
enum Family {
    Linear,
    Polynomial(Order),
    Exponential,
    Logarithmic,
    Power,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooFewPoints { required, found } => {
                write!(f, "trendline needs {required} distinct x positions, found {found}")
            }
            Self::NonpositiveY([x, y]) => write!(f, "y must be positive, found ({x}, {y})"),
            Self::NonpositiveX([x, y]) => write!(f, "x must be positive, found ({x}, {y})"),
            Self::Intercept(b) => write!(f, "invalid fixed intercept {b}"),
            Self::Forecast { backward, forward } => {
                write!(f, "invalid forecast: backward {backward}, forward {forward}")
            }
            Self::ForecastDomain(x) => write!(f, "forecast reaches x = {x}, outside the trendline's domain"),
            Self::Order(n) => write!(f, "polynomial order {n} is outside 2..=6"),
            Self::Period(n) => write!(f, "moving-average period {n} is below 2"),
            Self::Singular => write!(f, "trendline design is singular"),
            Self::Overflow => write!(f, "trendline overflows over its range"),
        }
    }
}

impl std::error::Error for Error {}

impl TryFrom<u8> for Order {
    type Error = Error;

    fn try_from(n: u8) -> Result<Self, Error> {
        Ok(match n {
            2 => Self::Two,
            3 => Self::Three,
            4 => Self::Four,
            5 => Self::Five,
            6 => Self::Six,
            _ => return Err(Error::Order(n)),
        })
    }
}

impl Order {
    /// Every order Excel offers, ascending.
    pub const ALL: [Self; 5] = [Self::Two, Self::Three, Self::Four, Self::Five, Self::Six];

    /// The highest power of x.
    pub fn degree(self) -> usize {
        self as usize
    }
}

impl Period {
    pub fn new(points: usize) -> Result<Self, Error> {
        if points >= 2 {
            Ok(Self(points))
        } else {
            Err(Error::Period(points))
        }
    }

    pub fn get(self) -> usize {
        self.0
    }
}

impl Forecast {
    /// Draw over the observed x range only.
    pub const NONE: Self = Self {
        backward: 0.,
        forward: 0.,
    };

    /// Extend by `backward` before the first and `forward` after the last
    /// observed x, in x units (periods on a category axis).
    pub fn new(backward: f64, forward: f64) -> Result<Self, Error> {
        let valid = |d: f64| d.is_finite() && d >= 0.;
        if valid(backward) && valid(forward) {
            Ok(Self { backward, forward })
        } else {
            Err(Error::Forecast { backward, forward })
        }
    }

    pub fn backward(self) -> f64 {
        self.backward
    }

    pub fn forward(self) -> f64 {
        self.forward
    }
}

impl Transform {
    fn forward(self, [x, y]: [f64; 2]) -> Result<[f64; 2], Error> {
        let (log_x, log_y) = self.logs();
        if log_y && y <= 0. {
            return Err(Error::NonpositiveY([x, y]));
        }
        if log_x && x <= 0. {
            return Err(Error::NonpositiveX([x, y]));
        }
        Ok([if log_x { x.ln() } else { x }, if log_y { y.ln() } else { y }])
    }

    fn logs(self) -> (bool, bool) {
        match self {
            Self::Identity => (false, false),
            Self::LogX => (true, false),
            Self::LogY => (false, true),
            Self::LogBoth => (true, true),
        }
    }
}

impl Fit {
    fn new(samples: &[[f64; 2]], family: Family, intercept: Option<f64>) -> Result<Self, Error> {
        let (transform, degree) = match family {
            Family::Linear => (Transform::Identity, 1),
            Family::Polynomial(order) => (Transform::Identity, order.degree()),
            Family::Exponential => (Transform::LogY, 1),
            Family::Logarithmic => (Transform::LogX, 1),
            Family::Power => (Transform::LogBoth, 1),
        };
        let finite: Vec<_> = samples
            .iter()
            .copied()
            .filter(|p| p.iter().all(|v| v.is_finite()))
            .collect();
        let points = finite
            .iter()
            .map(|&p| transform.forward(p))
            .collect::<Result<Vec<_>, _>>()?;
        let regression = Regression::fit(&points, degree, intercept)?;
        let c = regression.coefficients();
        let model = match family {
            Family::Linear => Model::Linear {
                slope: c[1],
                intercept: c[0],
            },
            Family::Polynomial(order) => Model::Polynomial {
                order,
                coefficients: c.iter().rev().copied().collect(),
            },
            Family::Exponential => Model::Exponential {
                scale: c[0].exp(),
                rate: c[1],
            },
            Family::Logarithmic => Model::Logarithmic {
                scale: c[1],
                intercept: c[0],
            },
            Family::Power => Model::Power {
                scale: c[0].exp(),
                exponent: c[1],
            },
        };
        let lo = finite.iter().map(|p| p[0]).fold(f64::INFINITY, f64::min);
        let hi = finite.iter().map(|p| p[0]).fold(f64::NEG_INFINITY, f64::max);
        Ok(Self {
            model,
            regression,
            transform,
            domain: [lo, hi],
        })
    }

    pub fn model(&self) -> &Model {
        &self.model
    }

    /// Coefficient of determination of the regression Excel reports.
    pub fn r_squared(&self) -> f64 {
        self.regression.r_squared()
    }

    /// The smallest and largest observed x.
    pub fn domain(&self) -> [f64; 2] {
        self.domain
    }

    /// The trendline's value at x, if x is in its domain and the value is finite.
    pub fn predict(&self, x: f64) -> Option<f64> {
        let (log_x, log_y) = self.transform.logs();
        if !x.is_finite() || (log_x && x <= 0.) {
            return None;
        }
        let v = self.regression.predict(if log_x { x.ln() } else { x });
        let y = if log_y { v.exp() } else { v };
        y.is_finite().then_some(y)
    }

    /// The equation as Excel labels it, with five significant digits.
    pub fn equation(&self) -> String {
        match &self.model {
            Model::Linear { slope, intercept } => format!("y = {}{}", term(*slope, "x"), constant(*intercept)),
            Model::Polynomial { coefficients, .. } => {
                let n = coefficients.len() - 1;
                let mut terms = coefficients[..n]
                    .iter()
                    .enumerate()
                    .filter(|&(_, c)| *c != 0.)
                    .map(|(i, &c)| (c, format!("x{}", superscript(n - i))));
                let Some((c, power)) = terms.next() else {
                    return format!("y = {}", number(coefficients[n]));
                };
                let mut text = format!("y = {}", term(c, &power));
                for (c, power) in terms {
                    text.push_str(&format!(" {} {}", sign(c), term(c.abs(), &power)));
                }
                text.push_str(&constant(coefficients[n]));
                text
            }
            Model::Exponential { scale, rate } => format!("y = {}e^({})", number(*scale), term(*rate, "x")),
            Model::Logarithmic { scale, intercept } => format!("y = {}{}", term(*scale, "ln(x)"), constant(*intercept)),
            Model::Power { scale, exponent } => format!("y = {}x^{}", number(*scale), number(*exponent)),
        }
    }

    /// The trendline over the observed x range extended by the forecast.
    pub fn line(&self, forecast: Forecast) -> Result<Line, Error> {
        let lo = self.domain[0] - forecast.backward;
        let hi = self.domain[1] + forecast.forward;
        if self.transform.logs().0 && lo <= 0. {
            return Err(Error::ForecastDomain(lo));
        }
        let segments = if matches!(self.model, Model::Linear { .. }) {
            1
        } else {
            128
        };
        let points = (0..=segments)
            .map(|i| {
                let x = lo + (hi - lo) * i as f64 / segments as f64;
                self.predict(x).map(|y| (x, y)).ok_or(Error::Overflow)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let name = match &self.model {
            Model::Linear { .. } => "Linear".to_owned(),
            Model::Polynomial { order, .. } => format!("Polynomial (order {})", order.degree()),
            Model::Exponential { .. } => "Exponential".to_owned(),
            Model::Logarithmic { .. } => "Logarithmic".to_owned(),
            Model::Power { .. } => "Power".to_owned(),
        };
        Ok(line(points).with_name(name))
    }
}

/// Straight-line trendline, optionally through a fixed y-intercept.
pub fn linear(samples: &[[f64; 2]], intercept: Option<f64>) -> Result<Fit, Error> {
    Fit::new(samples, Family::Linear, finite_intercept(intercept)?)
}

/// Polynomial trendline of the given order, optionally through a fixed y-intercept.
pub fn polynomial(samples: &[[f64; 2]], order: Order, intercept: Option<f64>) -> Result<Fit, Error> {
    Fit::new(samples, Family::Polynomial(order), finite_intercept(intercept)?)
}

/// Exponential trendline y = a·e^(r·x), optionally with a fixed positive a.
pub fn exponential(samples: &[[f64; 2]], intercept: Option<f64>) -> Result<Fit, Error> {
    let log = match intercept {
        Some(a) if a.is_finite() && a > 0. => Some(a.ln()),
        Some(a) => return Err(Error::Intercept(a)),
        None => None,
    };
    Fit::new(samples, Family::Exponential, log)
}

/// Logarithmic trendline y = c·ln(x) + b. Excel offers no fixed intercept.
pub fn logarithmic(samples: &[[f64; 2]]) -> Result<Fit, Error> {
    Fit::new(samples, Family::Logarithmic, None)
}

/// Power trendline y = a·x^p. Excel offers no fixed intercept.
pub fn power(samples: &[[f64; 2]]) -> Result<Fit, Error> {
    Fit::new(samples, Family::Power, None)
}

/// Trailing moving average in input order, like Excel's: the point at each
/// sample from the period's last onward averages that sample's y and the
/// preceding ones. Sort a scatter series by x first for a time-ordered
/// average. Nonfinite samples are skipped. Excel shows no equation or R².
pub fn moving_average(samples: &[[f64; 2]], period: Period) -> Result<Line, Error> {
    let finite: Vec<_> = samples
        .iter()
        .copied()
        .filter(|p| p.iter().all(|v| v.is_finite()))
        .collect();
    let n = period.get();
    if finite.len() <= n {
        return Err(Error::TooFewPoints {
            required: n + 1,
            found: finite.len(),
        });
    }
    let points: Vec<_> = finite
        .windows(n)
        .map(|w| (w[n - 1][0], w.iter().map(|p| p[1]).sum::<f64>() / n as f64))
        .collect();
    if points.iter().any(|p| !p.1.is_finite()) {
        return Err(Error::Overflow);
    }
    Ok(line(points).with_name(format!("{n}-period moving average")))
}

fn finite_intercept(intercept: Option<f64>) -> Result<Option<f64>, Error> {
    match intercept {
        Some(b) if !b.is_finite() => Err(Error::Intercept(b)),
        other => Ok(other),
    }
}

/// Five significant digits, trailing zeros removed, scientific outside
/// 1e-4..1e10 in Excel's `1.2346E-05` form.
fn number(v: f64) -> String {
    if v == 0. {
        return "0".to_owned();
    }
    let exponent = v.abs().log10().floor() as i32;
    if !(-4..10).contains(&exponent) {
        let text = format!("{v:.4e}");
        let (mantissa, power) = text.split_once('e').unwrap_or((&text, "0"));
        let power: i32 = power.parse().unwrap_or(0);
        let sign = if power < 0 { '-' } else { '+' };
        return format!("{}E{sign}{:02}", trim(mantissa), power.abs());
    }
    let decimals = (4 - exponent).max(0) as usize;
    trim(&format!("{v:.decimals$}")).to_owned()
}

fn trim(text: &str) -> &str {
    if text.contains('.') {
        text.trim_end_matches('0').trim_end_matches('.')
    } else {
        text
    }
}

fn term(coefficient: f64, variable: &str) -> String {
    format!("{}{variable}", number(coefficient))
}

fn constant(c: f64) -> String {
    if c == 0. {
        String::new()
    } else {
        format!(" {} {}", sign(c), number(c.abs()))
    }
}

fn sign(c: f64) -> char {
    if c < 0. { '-' } else { '+' }
}

fn superscript(power: usize) -> &'static str {
    ["", "", "²", "³", "⁴", "⁵", "⁶"].get(power).copied().unwrap_or("")
}

#[cfg(test)]
mod tests;
