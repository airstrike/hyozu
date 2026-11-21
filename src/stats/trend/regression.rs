//! Polynomial least squares in a standardized variable, solved by Householder QR.
use super::Error;

/// A least-squares polynomial `v = p(u)`. Coefficients apply to the standardized
/// variable `t = (u - center) / scale`, which keeps the design well conditioned
/// for large offsets and higher orders.
#[derive(Clone, Debug)]
pub(super) struct Regression {
    center: f64,
    scale: f64,
    /// Ascending powers of `t`.
    basis: Vec<f64>,
    r_squared: f64,
}

impl Regression {
    /// Fit `degree` powers of `u`, plus a free constant or the supplied one.
    /// With a fixed constant the total sum of squares is taken about that
    /// constant, as `LINEST(v - b, …, FALSE)` does.
    pub(super) fn fit(points: &[[f64; 2]], degree: usize, intercept: Option<f64>) -> Result<Self, Error> {
        let mut positions: Vec<_> = points
            .iter()
            .map(|p| p[0])
            .filter(|&u| intercept.is_none() || u != 0.)
            .collect();
        positions.sort_by(f64::total_cmp);
        positions.dedup();
        let required = degree + usize::from(intercept.is_none());
        if positions.len() < required {
            return Err(Error::TooFewPoints {
                required,
                found: positions.len(),
            });
        }
        let n = points.len() as f64;
        let center = match intercept {
            Some(_) => 0.,
            None => points.iter().map(|p| p[0]).sum::<f64>() / n,
        };
        let scale = points.iter().map(|p| (p[0] - center).abs()).fold(0., f64::max);
        if !(scale > 0. && scale.is_finite() && center.is_finite()) {
            return Err(Error::Singular);
        }
        let offset = intercept.unwrap_or(0.);
        let first = usize::from(intercept.is_some());
        let columns = degree + 1 - first;
        let mut design: Vec<Vec<f64>> = (first..=degree)
            .map(|k| {
                points
                    .iter()
                    .map(|p| ((p[0] - center) / scale).powi(k as i32))
                    .collect()
            })
            .collect();
        let mut response: Vec<f64> = points.iter().map(|p| p[1] - offset).collect();
        let solution = least_squares(&mut design, &mut response, columns)?;
        let mut basis = vec![offset; degree + 1];
        basis[first..].copy_from_slice(&solution);
        let mut fit = Self {
            center,
            scale,
            basis,
            r_squared: 0.,
        };
        let residual: f64 = points.iter().map(|&[u, v]| (v - fit.predict(u)).powi(2)).sum();
        let reference = match intercept {
            Some(b) => b,
            None => points.iter().map(|p| p[1]).sum::<f64>() / n,
        };
        let total: f64 = points.iter().map(|p| (p[1] - reference).powi(2)).sum();
        fit.r_squared = if total > 0. { 1. - residual / total } else { 1. };
        if fit.basis.iter().all(|c| c.is_finite()) && fit.r_squared.is_finite() {
            Ok(fit)
        } else {
            Err(Error::Singular)
        }
    }

    pub(super) fn predict(&self, u: f64) -> f64 {
        let t = (u - self.center) / self.scale;
        self.basis.iter().rev().fold(0., |acc, c| acc * t + c)
    }

    pub(super) fn r_squared(&self) -> f64 {
        self.r_squared
    }

    /// Ascending coefficients of powers of `u` itself.
    pub(super) fn coefficients(&self) -> Vec<f64> {
        if self.center == 0. {
            return (0..self.basis.len())
                .map(|k| {
                    if k == 0 {
                        self.basis[0]
                    } else {
                        self.basis[k] / self.scale.powi(k as i32)
                    }
                })
                .collect();
        }
        // (u - c)^k = Σ_j C(k, j) u^j (-c)^(k - j)
        let mut raw = vec![0.; self.basis.len()];
        for (k, a) in self.basis.iter().enumerate() {
            let weight = a / self.scale.powi(k as i32);
            let mut binomial = 1.;
            for (j, slot) in raw.iter_mut().enumerate().take(k + 1) {
                *slot += weight * binomial * (-self.center).powi((k - j) as i32);
                binomial *= (k - j) as f64 / (j + 1) as f64;
            }
        }
        raw
    }
}

/// Minimize |A x - b| for column-major `A` with `columns` columns.
fn least_squares(a: &mut [Vec<f64>], b: &mut [f64], columns: usize) -> Result<Vec<f64>, Error> {
    let rows = b.len();
    let mut largest: f64 = 0.;
    for k in 0..columns {
        let norm = a[k][k..].iter().map(|v| v * v).sum::<f64>().sqrt();
        if !(norm > 0. && norm.is_finite()) {
            return Err(Error::Singular);
        }
        let alpha = if a[k][k] > 0. { -norm } else { norm };
        let mut reflector: Vec<f64> = a[k][k..].to_vec();
        reflector[0] -= alpha;
        let length = reflector.iter().map(|v| v * v).sum::<f64>();
        let reflect = |column: &mut [f64]| {
            let dot: f64 = reflector.iter().zip(&column[k..]).map(|(v, c)| v * c).sum();
            let factor = 2. * dot / length;
            for (c, v) in column[k..].iter_mut().zip(&reflector) {
                *c -= factor * v;
            }
        };
        for column in a.iter_mut().take(columns).skip(k) {
            reflect(column);
        }
        reflect(b);
        largest = largest.max(a[k][k].abs());
    }
    let tolerance = largest * f64::EPSILON * rows.max(columns) as f64;
    let mut x = vec![0.; columns];
    for k in (0..columns).rev() {
        let pivot = a[k][k];
        if pivot.abs() <= tolerance {
            return Err(Error::Singular);
        }
        let known: f64 = (k + 1..columns).map(|j| a[j][k] * x[j]).sum();
        x[k] = (b[k] - known) / pivot;
    }
    Ok(x)
}
