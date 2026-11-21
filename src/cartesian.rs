//! Cartesian geometry for interval, financial and discrete signal charts.
pub mod error;

use crate::Diagram;

/// Lower and upper values at a scalar x coordinate.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Interval {
    pub x: f64,
    pub low: f64,
    pub high: f64,
}

impl Interval {
    fn valid(self) -> bool {
        self.x.is_finite() && self.low.is_finite() && self.high.is_finite() && self.low <= self.high
    }
}

/// Draw independent line segments with common color and data-space endpoints.
pub fn segments(lines: &[([f64; 2], [f64; 2])]) -> Diagram {
    let mut d = Diagram::cartesian();
    let id = d.series("Segments");
    for &(a, b) in lines {
        d.path(id, vec![a, b], false, 1.);
    }
    d
}

/// Independent horizontal spans sharing a vertical lane. Each span is
/// [start, width] and the lane is [bottom, height], all in data coordinates.
pub fn broken_bars(name: &str, spans: &[[f64; 2]], lane: [f64; 2]) -> Option<Diagram> {
    let [bottom, height] = lane;
    let top = bottom + height;
    if !bottom.is_finite() || !top.is_finite() || height <= 0. {
        return None;
    }
    let mut d = Diagram::cartesian();
    let id = d.series(name);
    for &[start, width] in spans {
        let end = start + width;
        if width > 0. && start.is_finite() && end.is_finite() {
            d.path(
                id,
                vec![[start, bottom], [end, bottom], [end, top], [start, top]],
                true,
                1.,
            );
        }
    }
    Some(d)
}

/// Vertical ranges with symmetric caps. Set cap width to zero for lineranges.
pub fn error_bars(intervals: &[Interval], cap_width: f64) -> Diagram {
    capped("Interval", intervals, cap_width)
}

fn capped(name: &str, intervals: &[Interval], cap_width: f64) -> Diagram {
    let mut d = Diagram::cartesian();
    let id = d.series(name);
    let half = if cap_width.is_finite() {
        cap_width.max(0.) / 2.
    } else {
        0.
    };
    for &v in intervals.iter().filter(|v| v.valid()) {
        d.path(id, vec![[v.x, v.low], [v.x, v.high]], false, 1.);
        if half > 0. {
            for y in [v.low, v.high] {
                d.path(id, vec![[v.x - half, y], [v.x + half, y]], false, 1.);
            }
        }
    }
    d
}

/// The polygon between successive upper and lower boundaries, in input order.
/// Invalid intervals break the ribbon into independent runs.
pub fn ribbon(intervals: &[Interval]) -> Diagram {
    let mut d = Diagram::cartesian();
    let id = d.series("Interval");
    for run in intervals.split(|v| !v.valid()).filter(|run| run.len() > 1) {
        let mut p: Vec<_> = run.iter().map(|v| [v.x, v.high]).collect();
        p.extend(run.iter().rev().map(|v| [v.x, v.low]));
        d.path(id, p, true, 0.4);
    }
    d
}

/// One open, high, low and close record at a scalar x coordinate.
#[derive(Clone, Copy, Debug)]
pub struct Ohlc {
    pub x: f64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
}

/// Candle bodies distinguish increases and decreases by palette slot.
/// Width uses x-axis units. Invalid records are omitted.
pub fn candlesticks(quotes: &[Ohlc], width: f64) -> Diagram {
    stock(quotes, width, true)
}

/// A high–low stem, open tick to its left and closing tick to its right.
pub fn ohlc(quotes: &[Ohlc], width: f64) -> Diagram {
    stock(quotes, width, false)
}

fn stock(quotes: &[Ohlc], width: f64, candlestick: bool) -> Diagram {
    let mut d = Diagram::cartesian();
    let up = d.series("Up");
    let down = d.series("Down");
    if !width.is_finite() || width <= 0. {
        return d;
    }
    for q in quotes {
        if [q.x, q.open, q.high, q.low, q.close].iter().any(|v| !v.is_finite())
            || q.low > q.open.min(q.close)
            || q.high < q.open.max(q.close)
        {
            continue;
        }
        let id = if q.close >= q.open { up } else { down };
        let (left, right) = (q.x - width / 2., q.x + width / 2.);
        d.path(id, vec![[q.x, q.low], [q.x, q.high]], false, 1.);
        if candlestick && q.open != q.close {
            d.path(
                id,
                vec![[left, q.open], [right, q.open], [right, q.close], [left, q.close]],
                true,
                1.,
            );
        } else {
            d.path(id, vec![[left, q.open], [q.x, q.open]], false, 1.);
            d.path(id, vec![[q.x, q.close], [right, q.close]], false, 1.);
        }
    }
    d
}

/// Hover targets for a path drawn through `run`.
fn observed(run: &[[f64; 2]]) -> Vec<([f64; 2], crate::mark::diagram::Sample)> {
    run.iter()
        .map(|&[x, y]| ([x, y], crate::mark::diagram::Sample::Cartesian { x, y }))
        .collect()
}

/// Piecewise constant signal. A nonfinite observation splits the path.
///
/// `placement` says where each jump sits between two observations, the
/// same [`Step`](crate::line::Step) a stepped line takes.
pub fn steps(samples: &[[f64; 2]], placement: crate::line::Step) -> Diagram {
    use crate::line::Step;

    let mut d = Diagram::cartesian();
    let id = d.series("Step");
    for run in samples
        .split(|p| p.iter().any(|v| !v.is_finite()))
        .filter(|run| run.len() > 1)
    {
        let mut p = vec![run[0]];
        for w in run.windows(2) {
            let (a, b) = (w[0], w[1]);
            let x = match placement {
                Step::Before => a[0],
                Step::After => b[0],
                Step::Mid => a[0].midpoint(b[0]),
            };
            p.extend([[x, a[1]], [x, b[1]], b]);
        }
        d.trace(id, p, observed(run));
    }
    d
}

/// A stem from the supplied baseline to each point and a marker at its tip.
pub fn stem(samples: &[[f64; 2]], baseline: f64) -> crate::Data {
    let mut d = Diagram::cartesian();
    let id = d.series("Stem");
    let finite: Vec<_> = samples
        .iter()
        .copied()
        .filter(|p| p.iter().all(|v| v.is_finite()) && baseline.is_finite())
        .collect();
    for &[x, y] in &finite {
        d.path(id, vec![[x, baseline], [x, y]], false, 1.);
    }
    let tips: Vec<_> = finite.into_iter().map(|[x, y]| (x, y)).collect();
    crate::data(vec![crate::Mark::Diagram(d), crate::Mark::Xy(crate::xy(tips))])
}

/// Event raster. Each row is an independent group of event times. Height is
/// the length of each mark in row units.
pub fn events(rows: &[&[f64]], height: f64) -> Diagram {
    let mut d = Diagram::cartesian();
    if !height.is_finite() || height <= 0. {
        return d;
    }
    for (i, row) in rows.iter().enumerate() {
        let id = d.entry(&format!("Group {}", i + 1), 0.);
        let y = i as f64;
        for &x in *row {
            d.path(id, vec![[x, y - height / 2.], [x, y + height / 2.]], false, 1.);
        }
    }
    d
}

/// A high–low–close record with no opening price.
#[derive(Clone, Copy, Debug)]
pub struct Hlc {
    pub x: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
}

pub fn hlc(quotes: &[Hlc], width: f64) -> Diagram {
    let mut d = Diagram::cartesian();
    let id = d.series("Price");
    if !width.is_finite() || width <= 0. {
        return d;
    }
    for q in quotes.iter().filter(|q| {
        [q.x, q.high, q.low, q.close].iter().all(|v| v.is_finite()) && q.low <= q.close && q.close <= q.high
    }) {
        d.path(id, vec![[q.x, q.low], [q.x, q.high]], false, 1.);
        d.path(id, vec![[q.x, q.close], [q.x + 0.5 * width, q.close]], false, 1.);
    }
    d
}

/// Price against a secondary value axis and volume columns against the
/// primary axis. Both layers use their actual x positions.
pub fn with_volume(price: Diagram, volume: &[[f64; 2]], width: f64) -> crate::Data {
    let mut bars = Diagram::cartesian();
    let id = bars.series("Volume");
    if width.is_finite() && width > 0. {
        for &[x, n] in volume
            .iter()
            .filter(|p| p[0].is_finite() && p[1].is_finite() && p[1] >= 0.)
        {
            let (lo, hi) = (x - width / 2., x + width / 2.);
            bars.path(id, vec![[lo, 0.], [hi, 0.], [hi, n], [lo, n]], true, 0.4);
        }
    }
    let mut chart = crate::data(bars)
        .secondary(vec![price.into()])
        .y_axis(|a| a.title("Volume"))
        .right_axis(|a| a.title("Price"));
    // Make both plots use identical x-axis bounds even if records are missing
    // at either end or the quote and column widths differ.
    let a = chart.primary_bounds();
    let b = chart.secondary_area().bounds();
    let (lo, hi) = (a.x_min.min(b.x_min), a.x_max.max(b.x_max));
    chart = chart.x_axis_bounds(lo, hi).top_axis(|a| a.with_bounds(lo, hi).none());
    chart
}

/// Oriented capped ranges. Lower and upper values are x coordinates.
pub fn horizontal_ranges(intervals: &[Interval], cap: f64) -> Diagram {
    let mut d = error_bars(intervals, cap);
    d.transpose();
    d
}

/// Mean markers with independent lower/upper ranges.
pub fn point_ranges(rows: &[(Interval, f64)], caps: f64) -> crate::Data {
    let valid: Vec<_> = rows.iter().filter(|(r, m)| r.valid() && m.is_finite()).collect();
    let intervals: Vec<_> = valid.iter().map(|(r, _)| *r).collect();
    let points: Vec<_> = valid.into_iter().map(|(r, m)| (r.x, *m)).collect();
    crate::data(vec![error_bars(&intervals, caps).into(), crate::xy(points).into()])
}

/// Rectangular intervals with a horizontal measure through each box.
pub fn crossbars(rows: &[(Interval, f64)], width: f64) -> Diagram {
    let mut d = Diagram::cartesian();
    let id = d.series("Range");
    if !width.is_finite() || width <= 0. {
        return d;
    }
    for &(r, m) in rows.iter().filter(|(r, m)| r.valid() && m.is_finite()) {
        let (l, h) = (r.x - width / 2., r.x + width / 2.);
        d.path(id, vec![[l, r.low], [h, r.low], [h, r.high], [l, r.high]], true, 0.35);
        d.path(id, vec![[l, m], [h, m]], false, 1.);
    }
    d
}

/// Filled polygon rings in Cartesian coordinates, drawn in input order.
pub fn polygons<S: AsRef<str>, V: AsRef<[[f64; 2]]>>(groups: &[(S, V)]) -> Diagram {
    let mut d = Diagram::cartesian();
    for (name, p) in groups {
        if p.as_ref().len() < 3 {
            continue;
        }
        let id = d.series(name.as_ref());
        d.path(id, p.as_ref().to_vec(), true, 1.);
    }
    d
}

/// Segments from each origin at its angle in radians and signed length.
pub fn spokes(samples: &[([f64; 2], f64, f64)]) -> Diagram {
    segments(
        &samples
            .iter()
            .map(|&(p, a, l)| (p, [p[0] + a.cos() * l, p[1] + a.sin() * l]))
            .collect::<Vec<_>>(),
    )
}

/// Sample a cubic Bézier with two control points between each pair of endpoints.
pub fn curves(curves: &[[[f64; 2]; 4]], steps: usize) -> Diagram {
    let mut d = Diagram::cartesian();
    let id = d.series("Curves");
    if steps == 0 {
        return d;
    }
    for curve in curves {
        let p = (0..=steps)
            .map(|i| {
                let t = i as f64 / steps as f64;
                let s = 1. - t;
                std::array::from_fn(|j| {
                    s * s * s * curve[0][j]
                        + 3. * s * s * t * curve[1][j]
                        + 3. * s * t * t * curve[2][j]
                        + t * t * t * curve[3][j]
                })
            })
            .collect();
        d.path(id, p, false, 1.);
    }
    d
}

/// A strip of ticks at the supplied baseline. `length` uses perpendicular data units.
pub fn rug(samples: &[f64], baseline: f64, length: f64, horizontal: bool) -> Diagram {
    segments(
        &samples
            .iter()
            .map(|&x| {
                if horizontal {
                    ([x, baseline], [x, baseline + length])
                } else {
                    ([baseline, x], [baseline + length, x])
                }
            })
            .collect::<Vec<_>>(),
    )
}

/// Piecewise-constant values on explicit consecutive intervals, optionally
/// connecting both ends to a baseline.
pub fn stairs(edges: &[f64], values: &[f64], baseline: Option<f64>) -> Option<Diagram> {
    if values.is_empty()
        || edges.len().checked_sub(1)? != values.len()
        || edges.iter().chain(values).any(|v| !v.is_finite())
        || edges.windows(2).any(|w| w[0] >= w[1])
        || baseline.is_some_and(|b| !b.is_finite())
    {
        return None;
    }
    let mut d = Diagram::cartesian();
    let id = d.series("Stairs");
    let mut path = Vec::new();
    if let Some(b) = baseline {
        path.push([edges[0], b]);
    }
    for (w, &y) in edges.windows(2).zip(values) {
        path.extend([[w[0], y], [w[1], y]]);
    }
    if let Some(b) = baseline {
        path.push([*edges.last()?, b]);
    }
    d.path(id, path, false, 1.);
    Some(d)
}

/// Sample a function over an inclusive domain. Nonfinite function values
/// split the line into visible segments.
pub fn function(range: [f64; 2], intervals: usize, f: impl Fn(f64) -> f64) -> Option<Diagram> {
    let [a, b] = range;
    if intervals == 0 || !a.is_finite() || !b.is_finite() || a >= b {
        return None;
    }
    let mut d = Diagram::cartesian();
    let id = d.series("Function");
    let mut path = Vec::new();
    for i in 0..=intervals {
        let t = i as f64 / intervals as f64;
        let x = a * (1. - t) + b * t;
        let y = f(x);
        if y.is_finite() {
            path.push([x, y]);
        } else {
            d.path(id, std::mem::take(&mut path), false, 1.);
        }
    }
    d.path(id, path, false, 1.);
    Some(d)
}

/// Apply a normalized connection path independently to every pair of
/// adjacent finite observations. Coordinates (0,0) and (1,1) denote its
/// endpoints. Missing data splits the result into separate paths.
pub fn connect(samples: &[[f64; 2]], pattern: &[[f64; 2]]) -> Option<Diagram> {
    if pattern.is_empty()
        || pattern.iter().flatten().any(|v| !v.is_finite())
        || (pattern.first() != Some(&[0., 0.]) && pattern.last() != Some(&[1., 1.]))
    {
        return None;
    }
    let mut d = Diagram::cartesian();
    let id = d.series("Connected observations");
    for run in samples
        .split(|p| p.iter().any(|v| !v.is_finite()))
        .filter(|r| r.len() > 1)
    {
        let mut path = vec![run[0]];
        for w in run.windows(2) {
            let (a, b) = (w[0], w[1]);
            for &[u, v] in pattern.iter().filter(|p| **p != [0., 0.]) {
                path.push([a[0] + (b[0] - a[0]) * u, a[1] + (b[1] - a[1]) * v]);
            }
            if path.last() != Some(&b) {
                path.push(b);
            }
        }
        d.path(id, path, false, 1.);
    }
    Some(d)
}

/// Interpolate a smooth scatter path through the supplied observations in
/// input order with cubic Catmull–Rom segments. A nonfinite observation
/// splits the path. Markers, when enabled, stay on the original observations.
pub fn smooth_scatter(samples: &[[f64; 2]], markers: bool) -> crate::Data {
    let mut d = Diagram::cartesian();
    let id = d.series("Trend");
    for run in samples
        .split(|p| p.iter().any(|v| !v.is_finite()))
        .filter(|s| s.len() > 1)
    {
        let mut path = vec![run[0]];
        for i in 0..run.len() - 1 {
            let a = run[i.saturating_sub(1)];
            let b = run[i];
            let c = run[i + 1];
            let e = run[(i + 2).min(run.len() - 1)];
            for k in 1..=16 {
                let t = f64::from(k) / 16.;
                path.push(std::array::from_fn(|axis| {
                    let (a, b, c, e) = (a[axis], b[axis], c[axis], e[axis]);
                    0.5 * (2. * b
                        + (-a + c) * t
                        + (2. * a - 5. * b + 4. * c - e) * t * t
                        + (-a + 3. * b - 3. * c + e) * t * t * t)
                }));
            }
        }
        d.trace(id, path, observed(run));
    }
    let mut marks = vec![crate::Mark::Diagram(d)];
    if markers {
        marks.push(
            crate::xy(
                samples
                    .iter()
                    .filter(|p| p.iter().all(|v| v.is_finite()))
                    .map(|p| (p[0], p[1]))
                    .collect::<Vec<_>>(),
            )
            .into(),
        );
    }
    crate::data(marks)
}

#[cfg(test)]
mod interval_tests {
    use super::*;
    #[test]
    fn custom_connection_maps_relative_vertices_to_observations() {
        let d = connect(&[[2., 3.], [6., 7.]], &[[0.25, 0.75], [1., 1.]]).expect("connection");
        assert_eq!(d.shapes[0].points, [[2., 3.], [3., 6.], [6., 7.]]);
        assert!(connect(&[[0., 0.], [1., 1.]], &[[0.4, 0.7]]).is_none());
    }

    #[test]
    fn smooth_scatter_passes_through_samples_and_preserves_gaps() {
        let d = smooth_scatter(
            &[[0., 1.], [1., 5.], [2., 2.], [f64::NAN, 0.], [3., 2.], [4., 7.]],
            true,
        );
        let crate::Mark::Diagram(s) = &d.marks()[0] else {
            panic!("path")
        };
        assert_eq!(s.shapes.len(), 2);
        assert_eq!(s.shapes[0].points[16], [1., 5.]);
        assert_eq!(s.shapes[1].points.last(), Some(&[4., 7.]));
    }
    #[test]
    fn stairs_gaps_and_transposition_preserve_data_coordinates() {
        let d = stairs(&[0., 2., 5.], &[3., 4.], Some(0.)).expect("stairs");
        assert_eq!(d.shapes[0].points, [
            [0., 0.],
            [0., 3.],
            [2., 3.],
            [2., 4.],
            [5., 4.],
            [5., 0.]
        ]);
        let f = function([-2., 2.], 4, |x| 1. / x).expect("sampled function");
        assert_eq!(f.shapes.len(), 2);
        let h = horizontal_ranges(
            &[Interval {
                x: 4.,
                low: 1.,
                high: 6.,
            }],
            0.2,
        );
        assert_eq!(h.shapes[0].points, [[1., 4.], [6., 4.]]);
    }
    #[test]
    fn volume_and_quotes_have_independent_y_axes_and_synchronized_x_axes() {
        let q = hlc(
            &[Hlc {
                x: 3.,
                low: 4.,
                close: 5.,
                high: 7.,
            }],
            0.4,
        );
        assert_eq!(q.shapes.len(), 2);
        let d = with_volume(q, &[[0., 1200.], [6., 800.]], 0.4);
        assert_eq!(d.primary_bounds().y_max, 1200.);
        assert_eq!(d.secondary_area().bounds().y_max, 7.);
        let bottom = d.x_axis_ref().expect("primary x");
        let top = d.secondary_area().x_axis.as_ref().expect("secondary x");
        assert_eq!(
            (bottom.lower_bound, bottom.upper_bound),
            (top.lower_bound, top.upper_bound)
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hats_start_at_each_category_reference_in_both_directions() {
        let chart = hat_graph(&[("reference", [3., 5.]), ("comparison", [4., 2.])]).expect("matched categories");
        assert_eq!(chart.shapes[0].points[0][1], 3.);
        assert_eq!(chart.shapes[1].points[0][1], 5.);
        for (s, (from, to)) in chart.shapes[2..].iter().zip([(3., 4.), (5., 2.)]) {
            assert_eq!((s.points[0][1], s.points[2][1]), (from, to));
        }
    }
    #[test]
    fn broken_spans_preserve_the_gaps_and_lane_bounds() {
        let d = broken_bars("A", &[[-2., 1.], [1., 3.], [2., f64::NAN]], [4., 2.]).expect("valid lane");
        assert_eq!(d.shapes.len(), 2);
        assert_eq!(d.shapes[0].points, [[-2., 4.], [-1., 4.], [-1., 6.], [-2., 6.]]);
        assert_eq!(d.shapes[1].points[1], [4., 4.]);
        assert!(broken_bars("A", &[], [0., 0.]).is_none());
    }

    #[test]
    fn financial_domain_covers_valid_wicks_and_half_widths() {
        let valid = Ohlc {
            x: 10.,
            open: 4.,
            high: 9.,
            low: 2.,
            close: 7.,
        };
        let d = candlesticks(
            &[valid, Ohlc {
                x: 90.,
                low: 8.,
                ..valid
            }],
            0.5,
        );
        let b = crate::data(d).primary_bounds();
        assert_eq!([b.x_min, b.x_max, b.y_min, b.y_max], [9.75, 10.25, 2., 9.]);
    }
    #[test]
    fn step_gaps_and_extents() {
        let d = steps(
            &[[0., 1.], [1., 2.], [f64::NAN, 0.], [3., 2.], [4., 1.]],
            crate::line::Step::Mid,
        );
        assert_eq!(d.shapes.len(), 2);
        assert_eq!(d.shapes[0].points[2], [0.5, 2.]);
    }
}

/// Compare each series with the first series independently within each
/// category. Each group starts at its reference value; the reference is a cap.
pub fn hat_graph<S: AsRef<str>, V: AsRef<[f64]>>(groups: &[(S, V)]) -> Option<Diagram> {
    let (_, base) = groups.first()?;
    let base = base.as_ref();
    if groups.len() < 2 || base.is_empty() || groups.iter().any(|(_, v)| v.as_ref().len() != base.len()) {
        return None;
    }
    let mut d = Diagram::cartesian();
    let width = 0.8 / groups.len() as f64;
    for (i, (name, values)) in groups.iter().enumerate() {
        let id = d.series(name.as_ref());
        for (j, (&lo, &hi)) in base.iter().zip(values.as_ref()).enumerate() {
            let x = j as f64 - 0.4 + i as f64 * width;
            if !(lo.is_finite() && hi.is_finite()) {
                continue;
            }
            if lo == hi {
                d.path(id, vec![[x, lo], [x + width, hi]], false, 1.);
            } else {
                d.path(id, vec![[x, lo], [x + width, lo], [x + width, hi], [x, hi]], true, 1.);
            }
        }
    }
    Some(d)
}
