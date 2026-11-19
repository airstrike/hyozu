//! Reductions over observations. Returned coordinates can feed any mark.
use crate::cartesian::Interval;
use std::collections::BTreeMap;

/// Apply a reducer to the finite y values at each distinct finite x.
pub fn at<T>(samples: &[[f64; 2]], reduce: impl Fn(&[f64]) -> T) -> Vec<(f64, T)> {
    let mut points: Vec<_> = samples
        .iter()
        .copied()
        .filter(|p| p.iter().all(|n| n.is_finite()))
        .collect();
    points.sort_by(|a, b| a[0].total_cmp(&b[0]));
    points
        .chunk_by(|a, b| a[0] == b[0])
        .map(|rows| {
            let values: Vec<_> = rows.iter().map(|r| r[1]).collect();
            (rows[0][0], reduce(&values))
        })
        .collect()
}

/// Reduce y in each occupied x interval. Bin centers honor unequal widths;
/// interior edges use the right bin and the final edge belongs to the last.
pub fn bins<T>(samples: &[[f64; 2]], edges: &[f64], reduce: impl Fn(&[f64]) -> T) -> Option<Vec<(f64, T)>> {
    if !crate::field::axis(edges) {
        return None;
    }
    let mut cells = vec![Vec::new(); edges.len() - 1];
    for &[x, y] in samples {
        if y.is_finite()
            && let Some(i) = crate::field::bin(x, edges)
        {
            cells[i].push(y);
        }
    }
    Some(
        cells
            .into_iter()
            .zip(edges.windows(2))
            .filter(|(v, _)| !v.is_empty())
            .map(|(v, e)| (e[0].midpoint(e[1]), reduce(&v)))
            .collect(),
    )
}

/// Split arbitrary records by a caller-supplied key and run one transformation
/// per group. The callback can return a mark, records, or a scalar.
pub fn groups<K: Ord, T, R>(
    records: impl IntoIterator<Item = T>,
    key: impl Fn(&T) -> K,
    transform: impl Fn(&K, &[T]) -> R,
) -> Vec<(K, R)> {
    let mut groups = BTreeMap::<K, Vec<T>>::new();
    for record in records {
        groups.entry(key(&record)).or_default().push(record);
    }
    groups
        .into_iter()
        .map(|(k, g)| {
            let value = transform(&k, &g);
            (k, value)
        })
        .collect()
}

/// Keep the first representative of each key, in original input order.
pub fn unique<K: Ord, T>(records: impl IntoIterator<Item = T>, key: impl Fn(&T) -> K) -> Vec<T> {
    let mut seen = std::collections::BTreeSet::new();
    records.into_iter().filter(|r| seen.insert(key(r))).collect()
}

/// Sample mean and standard error at each x. Singleton intervals collapse
/// onto their observation. This is the default point-range summary in ggplot2.
pub fn mean_se(samples: &[[f64; 2]], multiplier: f64) -> Option<crate::Data> {
    if !multiplier.is_finite() || multiplier < 0. {
        return None;
    }
    let stats = at(samples, |values| {
        let mut mean = 0.;
        let mut ss = 0.;
        for (i, &y) in values.iter().enumerate() {
            let d = y - mean;
            mean += d / (i + 1) as f64;
            ss += d * (y - mean);
        }
        let se = if values.len() > 1 {
            (ss / ((values.len() - 1) as f64 * values.len() as f64)).sqrt()
        } else {
            0.
        };
        (mean, multiplier * se)
    });
    let rows: Vec<_> = stats
        .into_iter()
        .map(|(x, (m, e))| {
            (
                Interval {
                    x,
                    low: m - e,
                    high: m + e,
                },
                m,
            )
        })
        .collect();
    Some(crate::cartesian::point_ranges(&rows, 0.))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reducers_preserve_boundaries_negative_zero_and_group_order() {
        let p = [[-0., 1.], [0., 3.], [1., 5.], [2., 7.], [3., 11.], [0., f64::NAN]];
        assert_eq!(at(&p, |s| s.iter().sum::<f64>()), [
            (0., 4.),
            (1., 5.),
            (2., 7.),
            (3., 11.)
        ]);
        assert_eq!(bins(&p, &[0., 1., 3.], |v| v.len()).expect("ordered bins"), [
            (0.5, 2),
            (2., 3)
        ]);
        assert_eq!(
            groups(
                [("z", 1), ("a", 2), ("z", 3)],
                |v| v.0,
                |_, v| v.iter().map(|p| p.1).sum::<i32>()
            ),
            [("a", 2), ("z", 4)]
        );
        assert_eq!(unique([3, 1, 3, 2, 1], |n| *n), [3, 1, 2]);
    }
}
