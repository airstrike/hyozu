//! Fits for conditional means, including a regression confidence band.
use crate::cartesian::Interval;
use crate::{Data, Mark, Xy, data, line};

/// Ordinary least-squares line and optional pointwise confidence band on its
/// mean. Supply a critical t value for the desired level and residual degrees
/// of freedom. At least two distinct finite x positions are required.
pub fn linear(samples: &[[f64; 2]], critical: Option<f64>) -> Option<Data> {
    let p: Vec<_> = samples
        .iter()
        .copied()
        .filter(|p| p.iter().all(|v| v.is_finite()))
        .collect();
    if p.len() < 2 || critical.is_some_and(|c| !c.is_finite() || c < 0.) {
        return None;
    }
    let (mut mx, mut my, mut xx, mut xy) = (0., 0., 0., 0.);
    let (mut lo, mut hi) = (p[0][0], p[0][0]);
    for (i, &[x, y]) in p.iter().enumerate() {
        let n = (i + 1) as f64;
        let dx = x - mx;
        let dy = y - my;
        mx += dx / n;
        my += dy / n;
        xx += dx * (x - mx);
        xy += dx * (y - my);
        lo = lo.min(x);
        hi = hi.max(x);
    }
    if xx <= 0. || !xx.is_finite() {
        return None;
    }
    let slope = xy / xx;
    let predict = |x| my + slope * (x - mx);
    let mut marks = Vec::new();
    if let Some(t) = critical
        && p.len() > 2
    {
        let variance = p.iter().map(|&[x, y]| (y - predict(x)).powi(2)).sum::<f64>() / (p.len() - 2) as f64;
        let intervals: Vec<_> = (0..=64)
            .map(|i| {
                let x = lo + (hi - lo) * f64::from(i) / 64.;
                let margin = t * (variance * (1. / p.len() as f64 + (x - mx).powi(2) / xx)).sqrt();
                Interval {
                    x,
                    low: predict(x) - margin,
                    high: predict(x) + margin,
                }
            })
            .collect();
        marks.push(Mark::Diagram(crate::cartesian::ribbon(&intervals)));
    }
    marks.push(Mark::Line(
        line([(lo, predict(lo)), (hi, predict(hi))]).with_name("Linear fit"),
    ));
    Some(data(marks).x_axis(|a| a.with_kind(crate::axis::Kind::Scalar)))
}

/// Locally weighted linear regression with a tricube distance kernel.
/// Span is the fraction of samples in each neighborhood. Duplicate x values
/// use their local mean when the design has no horizontal variation.
pub fn loess(samples: &[[f64; 2]], span: f64) -> Option<Data> {
    if !span.is_finite() || span <= 0. || span > 1. {
        return None;
    }
    let mut p: Vec<_> = samples
        .iter()
        .copied()
        .filter(|p| p.iter().all(|v| v.is_finite()))
        .collect();
    if p.len() < 2 {
        return None;
    }
    p.sort_by(|a, b| a[0].total_cmp(&b[0]));
    let k = ((p.len() as f64 * span).ceil() as usize).clamp(2, p.len());
    let mut fits = Vec::new();
    for &[x, _] in &p {
        if fits.last().is_some_and(|&(prev, _)| prev == x) {
            continue;
        }
        let mut distances: Vec<_> = p.iter().map(|s| (s[0] - x).abs()).collect();
        distances.select_nth_unstable_by(k - 1, f64::total_cmp);
        let radius = distances[k - 1];
        let (mut w, mut wx, mut wy, mut wxx, mut wxy) = (0., 0., 0., 0., 0.);
        for &[xx, y] in &p {
            let dx = xx - x;
            let weight = if radius == 0. {
                f64::from(dx == 0.)
            } else {
                (1. - (dx.abs() / radius).min(1.).powi(3)).powi(3)
            };
            w += weight;
            wx += weight * dx;
            wy += weight * y;
            wxx += weight * dx * dx;
            wxy += weight * dx * y;
        }
        if w == 0. {
            continue;
        }
        let det = w * wxx - wx * wx;
        let y = if det > 0. { (wy * wxx - wx * wxy) / det } else { wy / w };
        fits.push((x, y));
    }
    Some(data(line(fits)).x_axis(|a| a.with_kind(crate::axis::Kind::Scalar)))
}

/// Deterministic uniform jitter in the supplied x and y half-widths.
pub fn jitter(samples: &[[f64; 2]], width: [f64; 2], seed: u64) -> Xy {
    let mut state = seed;
    let mut random = || {
        state = state.wrapping_add(0x9e3779b97f4a7c15);
        let mut z = state;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        let bits = (z ^ (z >> 31)) >> 11;
        bits as f64 * (1. / (1u64 << 53) as f64) * 2. - 1.
    };
    let [w, h] = width.map(|s| if s.is_finite() { s.abs() } else { 0. });
    crate::xy(
        samples
            .iter()
            .filter(|p| p.iter().all(|v| v.is_finite()))
            .map(|&[x, y]| (x + w * random(), y + h * random()))
            .collect::<Vec<_>>(),
    )
}

/// Conditional quantile line minimizing the asymmetric absolute residual loss.
/// For a fixed slope the optimal intercept is the corresponding residual quantile.
/// Centering x keeps the search stable for large timestamp offsets.
pub fn quantile(samples: &[[f64; 2]], probability: f64) -> Option<crate::Line> {
    if !(0. < probability && probability < 1.) {
        return None;
    }
    let mut points: Vec<_> = samples
        .iter()
        .copied()
        .filter(|p| p.iter().all(|v| v.is_finite()))
        .collect();
    points.sort_by(|a, b| a[0].total_cmp(&b[0]));
    let lo = points.first()?[0];
    let hi = points.last()?[0];
    if hi <= lo {
        return None;
    }
    for p in &mut points {
        p[0] = (p[0] - lo) / (hi - lo);
    }
    let rank = ((points.len() as f64 * probability).ceil() as usize)
        .saturating_sub(1)
        .min(points.len() - 1);
    let fit = |slope: f64| {
        let mut residuals: Vec<_> = points.iter().map(|&[x, y]| y - slope * x).collect();
        let intercept = *residuals.select_nth_unstable_by(rank, f64::total_cmp).1;
        let loss = residuals
            .iter()
            .map(|&r| {
                let e = r - intercept;
                if e >= 0. {
                    probability * e
                } else {
                    (probability - 1.) * e
                }
            })
            .sum::<f64>();
        (intercept, loss)
    };
    let (min, max) = points
        .iter()
        .fold((points[0][1], points[0][1]), |(a, b), p| (a.min(p[1]), b.max(p[1])));
    let mut width = (max - min).max(1.);
    let mut mid = 0.;
    let mut found = false;
    for _ in 0..128 {
        let loss = fit(mid).1;
        let left = fit(mid - width).1;
        let right = fit(mid + width).1;
        if loss <= left && loss <= right {
            found = true;
            break;
        }
        mid += if left < right { -width } else { width };
        width *= 2.;
        if !width.is_finite() {
            return None;
        }
    }
    if !found {
        return None;
    }
    let (mut a, mut b) = (mid - width, mid + width);
    for _ in 0..96 {
        let u = a + (b - a) / 3.;
        let v = b - (b - a) / 3.;
        if fit(u).1 <= fit(v).1 {
            b = v;
        } else {
            a = u;
        }
    }
    let slope = a.midpoint(b);
    let intercept = fit(slope).0;
    let end = intercept + slope;
    (intercept.is_finite() && end.is_finite())
        .then(|| line([(lo, intercept), (hi, end)]).with_name(format!("Quantile {probability}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn regression_is_stable_for_large_offset_x() {
        let b = 1e12;
        let d = linear(&[[b, 1.], [b + 1., 4.], [b + 2., 7.]], Some(2.)).expect("nondegenerate fit");
        let fit = d
            .marks()
            .iter()
            .find_map(|m| if let Mark::Line(l) = m { Some(l) } else { None })
            .expect("line");
        assert_eq!((fit.points[0].y, fit.points[1].y), (1., 7.));
        assert!(linear(&[[b, 0.], [b, 2.]], None).is_none());
    }
}
