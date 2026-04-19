//! [`tatami::series::Result`] → hyozu [`Data`] with [`Mark::Line`] or
//! [`Mark::Bars`].
//!
//! One mark per [`tatami::series::Row`]; every row shares the series'
//! x-axis members. Row labels become the mark's legend name. Missing /
//! Error cells become NaN — line charts break the line at the gap; bar
//! charts drop the bar.

use tatami::query::MemberRef;
use tatami::series;

use crate::tatami::cell::cells_f64;
use crate::{Bars, Data, Mark, bar};
// `crate::line` is hyozu's Line mark constructor; this module exports its
// own `line` function returning `Data`. No aliased import — we call the
// mark constructor via its module path (`crate::data::mark::line::line`)
// inside the body where the name would otherwise collide.

/// Build a multi-series line chart from a [`tatami::series::Result`].
///
/// One [`Mark::Line`] per [`series::Row`]; the returned [`Data`] carries
/// categorical x-axis labels taken from the leaf segment of each member's
/// path. Callers can override via [`Data::x_axis`] on the return value.
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

    let data: Data = marks.into();
    data.x_axis_labels(leaf_labels(result.x()))
}

/// Build a grouped bar chart from a [`tatami::series::Result`].
///
/// One [`bar::Series`] per [`series::Row`], all gathered into a single
/// [`Mark::Bars`]. Grouped layout by default — the caller can switch to
/// stacked on the returned `Data` via the usual mark builder pattern. The
/// returned [`Data`] carries categorical x-axis labels taken from the leaf
/// segment of each member's path.
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

    let data: Data = Mark::Bars(Bars::from_series(series_list)).into();
    data.x_axis_labels(leaf_labels(result.x()))
}

/// Return the leaf segment (deepest path component) of each member, as a
/// parallel `Vec<String>`. Used for categorical x-axis labelling: a member
/// at path `World/West/US/CA` contributes `"CA"`.
fn leaf_labels(members: &[MemberRef]) -> Vec<String> {
    members
        .iter()
        .map(|m| {
            m.path
                .segments()
                .last()
                .map(|n| n.as_str().to_owned())
                .unwrap_or_default()
        })
        .collect()
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

    fn deep_mr(leaf: &str) -> MemberRef {
        MemberRef::new(
            name("Geography"),
            name("Default"),
            Path::with(name("World"), vec![name("North"), name("US"), name(leaf)]),
        )
    }

    #[test]
    fn bars_x_axis_labels_use_leaf_segment() {
        let result = series::Result::new(vec![deep_mr("CA"), deep_mr("NY"), deep_mr("TX")], vec![series::Row {
            label: "Revenue".into(),
            values: vec![valid(1.0), valid(2.0), valid(3.0)],
        }]);
        let data = bars(&result);
        let axis = data.x_axis_ref().expect("x axis present");
        let values = axis.labels.values.as_ref().expect("categorical labels");
        let owned: Vec<&str> = values.iter().map(String::as_str).collect();
        assert_eq!(owned, vec!["CA", "NY", "TX"]);
    }

    #[test]
    fn line_x_axis_labels_use_leaf_segment() {
        let result = series::Result::new(vec![deep_mr("CA"), deep_mr("NY")], vec![series::Row {
            label: "Revenue".into(),
            values: vec![valid(1.0), valid(2.0)],
        }]);
        let data = line(&result);
        let axis = data.x_axis_ref().expect("x axis present");
        let values = axis.labels.values.as_ref().expect("categorical labels");
        let owned: Vec<&str> = values.iter().map(String::as_str).collect();
        assert_eq!(owned, vec!["CA", "NY"]);
    }
}
