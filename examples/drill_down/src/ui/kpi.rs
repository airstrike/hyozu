//! KPI card rendering via the `hyozu::tatami::card` adapter.
//!
//! The KPI panel's query emits a [`tatami::scalar::Result`] with two cells:
//! slot 0 is the primary value for the active measure; slot 1 is the
//! month-over-month delta. `hyozu::tatami::card` reads those indices per
//! the `KpiLayout` contract.

use iced::widget::Renderer;
use iced::{Element, Theme};

use crate::ui::dashboard::Message;
use crate::ui::view::Measure;

/// Render the KPI panel's scalar result as an iced KPI card.
///
/// Assumes the query shape from `queries::kpi` — metrics = [primary, MoM].
/// Falls through to a muted text stub for non-Scalar variants (shouldn't
/// happen in practice — the Kpi panel's query always uses `Axes::Scalar`).
pub fn render<'a>(results: &'a tatami::Results, measure: Measure) -> Element<'a, Message, Theme, Renderer> {
    match results {
        tatami::Results::Scalar(scalar) => hyozu::tatami::card(scalar, hyozu::tatami::KpiLayout {
            title: measure_title(measure),
            subtitle: Some("vs prior month"),
            primary: 0,
            delta: Some(1),
            sparkline: None,
        }),
        // Defensive — every other Results variant falls back to debug text.
        // Reached only if queries::kpi drifts away from Axes::Scalar.
        _ => iced::widget::text(format!("unexpected KPI shape: {:?}", variant_tag(results)))
            .size(14)
            .into(),
    }
}

fn measure_title(measure: Measure) -> &'static str {
    match measure {
        Measure::Revenue => "Revenue",
        Measure::Occupancy => "Occupancy",
        Measure::Adr => "ADR",
        Measure::RevPar => "RevPAR",
    }
}

fn variant_tag(results: &tatami::Results) -> &'static str {
    match results {
        tatami::Results::Scalar(_) => "Scalar",
        tatami::Results::Series(_) => "Series",
        tatami::Results::Pivot(_) => "Pivot",
        tatami::Results::Rollup(_) => "Rollup",
        _ => "<unknown>",
    }
}
