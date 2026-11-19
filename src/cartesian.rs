//! Cartesian geometry for interval, financial and discrete signal charts.
use crate::Diagram;

/// Lower and upper values at a scalar x coordinate.
#[derive(Clone, Copy, Debug)]
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

/// Vertical ranges with symmetric caps. Set cap width to zero for lineranges.
pub fn error_bars(intervals: &[Interval], cap_width: f64) -> Diagram {
    let mut d = Diagram::cartesian();
    let id = d.series("Interval");
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

/// Where each jump occurs between two observation positions.
#[derive(Clone, Copy, Debug)]
pub enum Step {
    Before,
    After,
    Midpoint,
}

/// Piecewise constant signal. A nonfinite observation splits the path.
pub fn steps(samples: &[[f64; 2]], placement: Step) -> Diagram {
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
                Step::Midpoint => a[0].midpoint(b[0]),
            };
            p.extend([[x, a[1]], [x, b[1]], b]);
        }
        d.path(id, p, false, 1.);
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

#[cfg(test)]
mod tests {
    use super::*;
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
            Step::Midpoint,
        );
        assert_eq!(d.shapes.len(), 2);
        assert_eq!(d.shapes[0].points[2], [0.5, 2.]);
    }
}
