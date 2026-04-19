//! Right-rail top-N bar chart rendering.
//!
//! `queries::rail` emits an `Axes::Series` over a top-N brand tier set with a
//! single-metric row. The row's values line up with the series' x-axis
//! members (brand tiers). `hyozu::tatami::bars` materialises that into a
//! `Mark::Bars` chart.

use iced::widget::Renderer;
use iced::{Element, Theme};

use crate::ui::dashboard::Message;

/// Build the chart `Data` for the rail panel from a `Results::Series`.
///
/// Returns `None` when the results shape isn't `Series` — the caller falls
/// back to a placeholder. Owning the `Data` at the call site keeps the chart
/// widget's `&Data` borrow valid for the rendered frame.
#[must_use]
pub fn build_data(results: &tatami::Results) -> Option<hyozu::Data> {
    let tatami::Results::Series(series) = results else {
        return None;
    };
    Some(hyozu::tatami::bars(series))
}

/// Render the rail panel from an owned `Data`. Click routing (drill into a
/// selected brand tier) is deferred to a later slice.
#[must_use]
pub fn render(data: &hyozu::Data) -> Element<'_, Message, Theme, Renderer> {
    hyozu::chart(data).height(iced::Length::Fixed(280.0)).into()
}

/// Fallback text when the results aren't a Series (shouldn't happen — the
/// rail panel's query always resolves to `Axes::Series`).
#[must_use]
pub fn fallback(results: &tatami::Results) -> Element<'static, Message, Theme, Renderer> {
    let tag = match results {
        tatami::Results::Scalar(_) => "Scalar",
        tatami::Results::Series(_) => "Series",
        tatami::Results::Pivot(_) => "Pivot",
        tatami::Results::Rollup(_) => "Rollup",
        _ => "<unknown>",
    };
    iced::widget::text(format!("unexpected rail shape: {tag}"))
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

    fn tier_mr(head: &str) -> MemberRef {
        MemberRef::new(name("BrandTier"), name("Default"), Path::of(name(head)))
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
    fn series_result_builds_single_bars_mark() {
        let result = series::Result::new(vec![tier_mr("Luxury"), tier_mr("Upscale"), tier_mr("Economy")], vec![
            series::Row {
                label: "Revenue".into(),
                values: vec![valid(100.0), valid(80.0), valid(40.0)],
            },
        ]);
        let results = tatami::Results::Series(result);
        let data = build_data(&results).expect("series built");
        assert_eq!(data.marks().len(), 1);
        assert!(matches!(data.marks()[0], hyozu::Mark::Bars(_)));
    }
}
