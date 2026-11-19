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
