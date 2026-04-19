//! Time-series line chart rendering.
//!
//! `queries::line` emits an `Axes::Series` over `Time.Fiscal.Quarter` with a
//! single-metric row. The row's values line up with the series' x-axis
//! members (fiscal quarters). `hyozu::tatami::line` materialises that into a
//! `Mark::Line` chart.

use iced::widget::Renderer;
use iced::{Element, Theme};

use crate::ui::dashboard::Message;

/// Build the chart `Data` for the time-series panel from a `Results::Series`.
///
/// Returns `None` when the results shape isn't `Series` — the caller falls
/// back to a placeholder. Owning the `Data` at the call site keeps the chart
/// widget's `&Data` borrow valid for the rendered frame.
#[must_use]
pub fn build_data(results: &tatami::Results) -> Option<hyozu::Data> {
    let tatami::Results::Series(series) = results else {
        return None;
    };
    Some(hyozu::tatami::line(series))
}

/// Render the time-series panel from an owned `Data`. The period-grain
/// toggle lives in `view.period`; the header surface for it is deferred to a
/// later slice.
#[must_use]
pub fn render(data: &hyozu::Data) -> Element<'_, Message, Theme, Renderer> {
    hyozu::chart(data).height(iced::Length::Fixed(200.0)).into()
}

/// Fallback text when the results aren't a Series (shouldn't happen — the
/// line panel's query always resolves to `Axes::Series`).
#[must_use]
pub fn fallback(results: &tatami::Results) -> Element<'static, Message, Theme, Renderer> {
    let tag = match results {
        tatami::Results::Scalar(_) => "Scalar",
        tatami::Results::Series(_) => "Series",
        tatami::Results::Pivot(_) => "Pivot",
        tatami::Results::Rollup(_) => "Rollup",
        _ => "<unknown>",
    };
    iced::widget::text(format!("unexpected line shape: {tag}"))
        .size(14)
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    use tatami::query::{MemberRef, Path, Tuple};
    use tatami::schema::Name;
    use tatami::{Cell, series};

    fn name(s: &str) -> Name {
        Name::parse(s).expect("valid")
    }

    fn quarter_mr(head: &str) -> MemberRef {
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
    fn non_series_results_yield_none() {
        let scalar = tatami::Results::Scalar(tatami::scalar::Result::new(Tuple::empty(), vec![valid(1.0)]));
        assert!(build_data(&scalar).is_none());
    }

    #[test]
    fn series_result_builds_single_line_mark() {
        let result = series::Result::new(
            vec![quarter_mr("Q1"), quarter_mr("Q2"), quarter_mr("Q3"), quarter_mr("Q4")],
            vec![series::Row {
                label: "Revenue".into(),
                values: vec![valid(1.0), valid(2.0), valid(3.0), valid(4.0)],
            }],
        );
        let results = tatami::Results::Series(result);
        let data = build_data(&results).expect("series built");
        assert_eq!(data.marks().len(), 1);
        assert!(matches!(data.marks()[0], hyozu::Mark::Line(_)));
    }
}
