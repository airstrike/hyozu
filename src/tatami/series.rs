//! [`tatami::series::Result`] → hyozu [`Data`] with [`Mark::Line`] or
//! [`Mark::Bars`].
//!
//! One mark per [`tatami::series::Row`]; every row shares the series'
//! x-axis members. Row labels become the mark's legend name. Missing /
//! Error cells become NaN — line charts break the line at the gap; bar
//! charts drop the bar.

use tatami::series;

use crate::tatami::cell::cells_f64;
use crate::{Bars, Data, Mark, bar};
// `crate::line` is hyozu's Line mark constructor; this module exports its
// own `line` function returning `Data`. No aliased import — we call the
// mark constructor via its module path (`crate::data::mark::line::line`)
// inside the body where the name would otherwise collide.

/// Build a multi-series line chart from a [`tatami::series::Result`].
///
/// One [`Mark::Line`] per [`series::Row`]. The x-axis is shared; hyozu
/// infers integer positions from the y-vector length (0..N), which matches
/// the `x` member order — for display, a caller that wants member-labeled
/// categorical ticks should use `Data::x_axis(...)` on the returned value
/// with a categorical axis configured from `result.x()`.
#[must_use]
pub fn line(result: &series::Result) -> Data {
    let marks: Vec<Mark> = result
        .rows()
        .iter()
        .map(|row| {
            let mut built = crate::data::mark::line::line(cells_f64(&row.values));
            if !row.label.is_empty() {
                built = built.with_name(row.label.clone());
            }
            Mark::Line(built)
        })
        .collect();

    marks.into()
}

/// Build a grouped bar chart from a [`tatami::series::Result`].
///
/// One [`bar::Series`] per [`series::Row`], all gathered into a single
/// [`Mark::Bars`]. Grouped layout by default — the caller can switch to
/// stacked on the returned `Data` via the usual mark builder pattern.
#[must_use]
pub fn bars(result: &series::Result) -> Data {
    let series_list: Vec<bar::Series> = result
        .rows()
        .iter()
        .map(|row| {
            let mut s = bar::Series::new(cells_f64(&row.values));
            if !row.label.is_empty() {
                s = s.with_name(row.label.clone());
            }
            s
        })
        .collect();

    Mark::Bars(Bars::from_series(series_list)).into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tatami::query::{MemberRef, Path};
    use tatami::schema::Name;
    use tatami::{Cell, series};

    fn name(s: &str) -> Name {
        Name::parse(s).expect("valid")
    }

    fn mr(head: &str) -> MemberRef {
        MemberRef::new(name("Time"), name("Fiscal"), Path::of(name(head)))
    }

    fn valid(v: f64) -> Cell {
        Cell::Valid {
            value: v,
            unit: None,
            format: None,
        }
    }

    #[test]
    fn line_produces_one_mark_per_row() {
        let result = series::Result::new(vec![mr("Q1"), mr("Q2"), mr("Q3")], vec![
            series::Row {
                label: "Revenue".into(),
                values: vec![valid(1.0), valid(2.0), valid(3.0)],
            },
            series::Row {
                label: "Plan".into(),
                values: vec![valid(1.1), valid(2.1), valid(3.1)],
            },
        ]);
        let data = line(&result);
        assert_eq!(data.marks().len(), 2);
        assert!(matches!(data.marks()[0], Mark::Line(_)));
        assert!(matches!(data.marks()[1], Mark::Line(_)));
    }

    #[test]
    fn bars_produces_single_bars_mark_with_series_per_row() {
        let result = series::Result::new(vec![mr("Q1"), mr("Q2")], vec![
            series::Row {
                label: "Actual".into(),
                values: vec![valid(10.0), valid(20.0)],
            },
            series::Row {
                label: "Plan".into(),
                values: vec![valid(11.0), valid(21.0)],
            },
        ]);
        let data = bars(&result);
        assert_eq!(data.marks().len(), 1);
        assert!(matches!(data.marks()[0], Mark::Bars(_)));
    }

    #[test]
    fn empty_series_result_builds_empty_data() {
        let result = series::Result::new(Vec::new(), Vec::new());
        let line_data = line(&result);
        let bars_data = bars(&result);
        assert!(line_data.marks().is_empty());
        // `bars` emits one Mark::Bars even with zero series — preserves the
        // shape so downstream layouts still render axis infrastructure.
        assert_eq!(bars_data.marks().len(), 1);
    }
}
