//! Statistical charts. Compute these when the underlying observations change.
use crate::{Data, Line, Mark, Xy, area, areas, axis, bars, data, line, xy};

/// Frequencies for a set of strictly increasing finite bin edges.
/// Intervals are left closed and right open, with the final edge included.
#[derive(Clone, Debug)]
pub struct Bins {
    edges: Vec<f64>,
    counts: Vec<usize>,
    total: usize,
}

impl Bins {
    /// Drops observations outside the specified range and nonfinite observations.
    /// Returns `None` for fewer than two edges or unordered/nonfinite edges.
    pub fn new(edges: &[f64], observations: &[f64]) -> Option<Self> {
        if edges.len() < 2 || edges.iter().any(|v| !v.is_finite()) || edges.windows(2).any(|w| w[0] >= w[1]) {
            return None;
        }
        let mut counts = vec![0; edges.len() - 1];
        let last = counts.len() - 1;
        let mut total = 0;
        for &x in observations {
            if x >= edges[0] && x <= edges[last + 1] {
                let i = (edges.partition_point(|&edge| edge <= x) - 1).min(last);
                counts[i] += 1;
                total += 1;
            }
        }
        Some(Self {
            edges: edges.to_vec(),
            counts,
            total,
        })
    }

    pub fn counts(&self) -> &[usize] {
        &self.counts
    }

    pub fn edges(&self) -> &[f64] {
        &self.edges
    }

    fn intervals(&self, density: bool) -> impl Iterator<Item = (f64, f64, f64)> + '_ {
        self.edges.windows(2).zip(&self.counts).map(move |(w, &n)| {
            let height = if density {
                n as f64 / self.total.max(1) as f64 / (w[1] - w[0])
            } else {
                n as f64
            };
            (w[0], w[1], height)
        })
    }

    /// Draw contiguous intervals in data coordinates. With density enabled,
    /// the integral of the plotted heights is one for a nonempty sample.
    pub fn histogram(&self, density: bool) -> Data {
        let points: Vec<_> = self
            .intervals(density)
            .flat_map(|(lo, hi, h)| [(lo, h), (hi, h)])
            .collect();
        data(areas(area(points).opacity(0.7)).gradient(false))
            .x_axis(|a| a.with_kind(axis::Kind::Scalar))
            .x_axis_bounds(self.edges.first().copied(), self.edges.last().copied())
            .y_axis_bounds(0., None)
    }

    /// Joins heights at the bin centers, in data coordinates.
    pub fn frequency_polygon(&self, density: bool) -> Line {
        line(
            self.intervals(density)
                .map(|(lo, hi, h)| (lo.midpoint(hi), h))
                .collect::<Vec<_>>(),
        )
    }
}

/// Empirical cumulative distribution. Each distinct observation contributes
/// one vertical jump weighted by its multiplicity. Nonfinite samples are skipped.
pub fn ecdf(samples: &[f64]) -> Data {
    let sorted = finite_sorted(samples);
    let mut points = Vec::new();
    let mut start = 0;
    while start < sorted.len() {
        let x = sorted[start];
        let end = start + sorted[start..].partition_point(|&v| v == x);
        points.push((x, start as f64 / sorted.len() as f64));
        points.push((x, end as f64 / sorted.len() as f64));
        start = end;
    }
    data(line(points))
        .x_axis(|a| a.with_kind(axis::Kind::Scalar))
        .y_axis_bounds(0., 1.)
}

/// Gaussian kernel density curve with the same bandwidth, kernel and support
/// as the library's violin plots. Nonfinite observations are skipped.
pub fn density(samples: &[f64]) -> Data {
    let finite: Vec<_> = samples.iter().copied().filter(|v| v.is_finite()).collect();
    let entry = crate::violin_from_data(&finite);
    data(line(entry.density))
        .x_axis(|a| a.with_kind(axis::Kind::Scalar))
        .y_axis_bounds(0., None)
}

fn finite_sorted(values: &[f64]) -> Vec<f64> {
    let mut sorted: Vec<_> = values.iter().copied().filter(|v| v.is_finite()).collect();
    sorted.sort_by(f64::total_cmp);
    sorted
}

/// Compare sample quantiles against a distribution's inverse CDF at
/// midpoint plotting positions (i + 0.5) / n.
pub fn qq(samples: &[f64], quantile: impl Fn(f64) -> f64) -> Xy {
    let sorted = finite_sorted(samples);
    let n = sorted.len() as f64;
    let points: Vec<_> = sorted
        .into_iter()
        .enumerate()
        .filter_map(|(i, y)| {
            let x = quantile((i as f64 + 0.5) / n);
            x.is_finite().then_some((x, y))
        })
        .collect();
    xy(points)
}

/// Counts sorted largest first with a cumulative fraction on the right axis.
/// Zero-count categories retain their relative order, including an all-zero input.
pub fn pareto<S: AsRef<str>>(entries: &[(S, u64)]) -> Data {
    let mut ranked: Vec<_> = entries.iter().collect();
    ranked.sort_by_key(|entry| std::cmp::Reverse(entry.1));
    let labels: Vec<_> = ranked.iter().map(|(label, _)| label.as_ref().to_owned()).collect();
    let counts: Vec<_> = ranked.iter().map(|(_, n)| *n as f64).collect();
    let total: f64 = counts.iter().sum();
    let mut sum = 0.;
    let cumulative: Vec<_> = counts
        .iter()
        .map(|n| {
            sum += n;
            if total == 0. { 0. } else { sum / total }
        })
        .collect();
    data(bars(counts))
        .x_axis_labels(labels)
        .secondary(vec![Mark::Line(line(cumulative).with_name("Cumulative share"))])
        .right_axis(|a| a.with_bounds(0., 1.).labels(|v: f64| format!("{:.0}%", v * 100.)))
}

pub mod fit;

/// Stacked dots at each bin midpoint, one dot per included observation.
/// The y value gives its one-based position within the stack.
pub fn dotplot(bins: &Bins) -> Xy {
    let mut points = Vec::with_capacity(bins.total);
    for (edges, &count) in bins.edges.windows(2).zip(&bins.counts) {
        let x = edges[0].midpoint(edges[1]);
        points.extend((1..=count).map(|i| (x, i as f64)));
    }
    xy(points)
}

/// A one-sided empirical survival function, 1 − F(x), using the same tie
/// and finite-observation handling as the empirical CDF.
pub fn survival(samples: &[f64]) -> Data {
    let sorted = finite_sorted(samples);
    let mut points = Vec::new();
    let mut i = 0;
    while i < sorted.len() {
        let x = sorted[i];
        let j = i + sorted[i..].partition_point(|v| *v == x);
        points.extend([
            (x, 1. - i as f64 / sorted.len() as f64),
            (x, 1. - j as f64 / sorted.len() as f64),
        ]);
        i = j;
    }
    data(line(points))
        .x_axis(|a| a.with_kind(axis::Kind::Scalar))
        .y_axis_bounds(0., 1.)
}

/// Normalize each category across the supplied nonnegative series. A column
/// with zero total stays zero. Ragged, negative or nonfinite input is rejected.
pub fn percent_bars<S: AsRef<str>, V: AsRef<[f64]>>(series: &[(S, V)]) -> Option<crate::Bars> {
    let values = percent_values(series)?;
    Some(
        crate::Bars::from_series(
            series
                .iter()
                .zip(values)
                .map(|((name, _), v)| crate::bar(v).with_name(name.as_ref()))
                .collect(),
        )
        .stacked(),
    )
}

/// Normalized area stacks with the same category contract as percent bars.
pub fn percent_areas<S: AsRef<str>, V: AsRef<[f64]>>(series: &[(S, V)]) -> Option<crate::Area> {
    let values = percent_values(series)?;
    Some(
        areas(
            series
                .iter()
                .zip(values)
                .map(|((name, _), v)| area(v).with_name(name.as_ref()))
                .collect::<Vec<_>>(),
        )
        .stacked(),
    )
}

fn percent_values<S, V: AsRef<[f64]>>(series: &[(S, V)]) -> Option<Vec<Vec<f64>>> {
    let len = series.first().map_or(0, |s| s.1.as_ref().len());
    let mut totals = vec![0.; len];
    for (_, s) in series {
        let v = s.as_ref();
        if v.len() != len || v.iter().any(|v| !v.is_finite() || *v < 0.) {
            return None;
        }
        for (t, n) in totals.iter_mut().zip(v) {
            *t += n;
        }
    }
    if totals.iter().any(|t| !t.is_finite()) {
        return None;
    }
    Some(
        series
            .iter()
            .map(|(_, s)| {
                s.as_ref()
                    .iter()
                    .zip(&totals)
                    .map(|(&n, &t)| if t > 0. { n / t } else { 0. })
                    .collect()
            })
            .collect(),
    )
}

/// Reference line for a Q–Q plot through two linearly interpolated sample
/// percentiles and the corresponding theoretical quantiles.
pub fn qq_line(samples: &[f64], reference: [f64; 2], quantile: impl Fn(f64) -> f64) -> Option<Line> {
    let sorted = finite_sorted(samples);
    let [p, q] = reference;
    if sorted.len() < 2 || !(0. < p && p < q && q < 1.) {
        return None;
    }
    let sample = |p: f64| {
        let i = (sorted.len() - 1) as f64 * p;
        sorted[i.floor() as usize] * (1. - i.fract()) + sorted[i.ceil() as usize] * i.fract()
    };
    let (x1, x2) = (quantile(p), quantile(q));
    if !x1.is_finite() || !x2.is_finite() || x2 <= x1 {
        return None;
    }
    let slope = (sample(q) - sample(p)) / (x2 - x1);
    let lo = quantile(0.5 / sorted.len() as f64);
    let hi = quantile(1. - 0.5 / sorted.len() as f64);
    let y = |x| sample(p) + slope * (x - x1);
    [lo, hi, y(lo), y(hi)]
        .into_iter()
        .all(f64::is_finite)
        .then(|| line([(lo, y(lo)), (hi, y(hi))]).with_name("Reference"))
}

/// Bubble area represents the multiplicity of each distinct finite (x,y) pair.
pub fn count_points(samples: &[[f64; 2]], maximum_diameter: f32) -> Option<Xy> {
    if !maximum_diameter.is_finite() || maximum_diameter <= 0. {
        return None;
    }
    let mut points: Vec<_> = samples
        .iter()
        .filter(|p| p.iter().all(|v| v.is_finite()))
        .map(|p| p.map(|v| if v == 0. { 0. } else { v }))
        .collect();
    points.sort_by(|a, b| a[0].total_cmp(&b[0]).then(a[1].total_cmp(&b[1])));
    let mut counts = Vec::new();
    let mut positions = Vec::new();
    for group in points.chunk_by(|a, b| a == b) {
        positions.push((group[0][0], group[0][1]));
        counts.push(group.len() as f64);
    }
    let max = counts.iter().copied().fold(1., f64::max);
    Some(
        xy(positions).size_by(
            crate::encoding::size_by(move |i, _| counts[i])
                .domain(0.0..=max)
                .range(0.0..=maximum_diameter),
        ),
    )
}

/// Lines at the cumulative stack boundaries for aligned nonnegative categories.
pub fn stacked_lines<S: AsRef<str>, V: AsRef<[f64]>>(series: &[(S, V)], percent: bool) -> Option<Data> {
    let mut values = percent_values(series)?;
    let mut totals = vec![0.; values.first()?.len()];
    let mut marks = Vec::new();
    for ((name, raw), normalized) in series.iter().zip(&mut values) {
        let data = if percent { normalized.as_slice() } else { raw.as_ref() };
        for (sum, &v) in totals.iter_mut().zip(data) {
            *sum += v;
        }
        marks.push(Mark::Line(line(totals.clone()).with_name(name.as_ref())));
    }
    Some(data(marks))
}

/// Data ellipse at a caller-supplied Mahalanobis radius using the sample
/// covariance matrix. A 95% bivariate normal probability contour uses
/// radius `(-2.0 * 0.05_f64.ln()).sqrt()`. A singular covariance collapses
/// the ellipse onto its principal axis.
pub fn ellipse(samples: &[[f64; 2]], radius: f64) -> Option<Line> {
    if !radius.is_finite() || radius <= 0. {
        return None;
    }
    let (mut n, mut mx, mut my, mut xx, mut yy, mut xy) = (0usize, 0., 0., 0., 0., 0.);
    for &[x, y] in samples.iter().filter(|p| p.iter().all(|v| v.is_finite())) {
        n += 1;
        let (dx, dy) = (x - mx, y - my);
        mx += dx / n as f64;
        my += dy / n as f64;
        xx += dx * (x - mx);
        yy += dy * (y - my);
        xy += dx * (y - my);
    }
    if n < 2 {
        return None;
    }
    let denom = (n - 1) as f64;
    let (a, b, c) = (xx / denom, xy / denom, yy / denom);
    let half = (a - c) * 0.5;
    let mid = (a + c) * 0.5;
    let delta = half.hypot(b);
    let major = (mid + delta).max(0.).sqrt() * radius;
    let minor = (mid - delta).max(0.).sqrt() * radius;
    if !major.is_finite() || !minor.is_finite() {
        return None;
    }
    let (sin, cos) = (0.5 * (2. * b).atan2(a - c)).sin_cos();
    Some(
        line(
            (0..=128)
                .map(|i| {
                    let t = (i % 128) as f64 * std::f64::consts::TAU / 128.;
                    let (v, u) = t.sin_cos();
                    (
                        mx + major * u * cos - minor * v * sin,
                        my + major * u * sin + minor * v * cos,
                    )
                })
                .collect::<Vec<_>>(),
        )
        .with_name("Data ellipse"),
    )
}

#[cfg(test)]
mod transform_tests {
    use super::*;
    #[test]
    fn quantile_resists_outliers_and_resolves_conditional_slope() {
        let p = [[0., 1.], [1., 3.], [2., 5.], [3., 7.], [4., 9.], [3., 900.], [
            1., -800.,
        ]];
        let l = fit::quantile(&p, 0.5).expect("median regression");
        assert!((l.points[0].y - 1.).abs() < 1e-7 && (l.points[1].y - 9.).abs() < 1e-7);
        assert!(fit::quantile(&[[0., 1.], [0., 2.]], 0.5).is_none());
    }
    #[test]
    fn data_ellipse_uses_the_sample_covariance_and_closes() {
        let l = ellipse(&[[-1., 0.], [1., 0.], [0., -1.], [0., 1.]], 1.).expect("normal contour");
        for p in &l.points {
            assert!((p.x * p.x + p.y * p.y - 2. / 3.).abs() < 1e-10);
        }
        assert_eq!(l.points.first().map(|p| p.x), l.points.last().map(|p| p.x));
    }
    #[test]
    fn qq_interpolation_count_area_and_normalized_stack() {
        let q = qq_line(&[1., 3., 5., 7., 9.], [0.25, 0.75], |p| p).expect("linear reference");
        assert!((q.points[0].y - 1.8).abs() < 1e-10);
        let c = count_points(&[[-0., 1.], [0., 1.], [0., 1.], [0., 1.], [2., 1.]], 20.).expect("duplicates");
        assert_eq!(c.points.len(), 2);
        assert_eq!(c.resolved_size_at(0, 0.), 20.);
        assert_eq!(c.resolved_size_at(1, 0.), 10.);
        let lines = stacked_lines(&[("a", [1., 0.]), ("b", [3., 0.])], true).expect("aligned series");
        let Mark::Line(l) = &lines.marks()[1] else {
            panic!("line")
        };
        assert_eq!([l.points[0].y, l.points[1].y], [1., 0.]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn irregular_histogram_conserves_mass_and_includes_last_edge() {
        let bins = Bins::new(&[0., 1., 3.], &[-1., 0., 1., 2., 3., f64::NAN, 4.]).expect("valid edges");
        assert_eq!(bins.counts(), &[1, 3]);
        assert_eq!(bins.intervals(true).collect::<Vec<_>>(), [
            (0., 1., 0.25),
            (1., 3., 0.375)
        ]);
        assert!(Bins::new(&[0., 0.], &[]).is_none());
        assert!(Bins::new(&[0., f64::NAN], &[]).is_none());
    }

    #[test]
    fn ecdf_collapses_ties_into_one_jump() {
        let chart = ecdf(&[4., 1., 1., 1., f64::INFINITY]);
        let points = &chart.line(0).expect("line").points;
        assert_eq!(points.iter().map(|p| (p.x, p.y)).collect::<Vec<_>>(), [
            (1., 0.),
            (1., 0.75),
            (4., 0.75),
            (4., 1.)
        ]);
    }
}
