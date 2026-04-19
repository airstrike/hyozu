//! Per-panel `tatami::Query` builders.
//!
//! Each constructor emits a concrete `tatami::Query` for its panel, keyed on
//! the current trail entry and the active view.

use std::num::NonZeroUsize;

use tatami::query::{Options, Set, Tuple};
use tatami::schema::Name;
use tatami::{Axes, MemberRef, Path, Query};

use crate::ui::view::{Measure, View};

/// Default landing query — FY2026 Actual, Revenue scalar.
#[must_use]
pub fn nation_default() -> Query {
    Query {
        axes: Axes::Scalar,
        slicer: Tuple::of([
            MemberRef::new(n("Time"), n("Fiscal"), Path::of(n("FY2026"))),
            MemberRef::scenario(n("Actual")),
        ])
        .expect("distinct dims"),
        metrics: vec![n("Revenue")],
        options: Options::default(),
    }
}

/// Name of the primary metric for a [`Measure`]. The name is the one
/// declared in `data::schema::hewton_schema`.
#[must_use]
pub fn measure_metric(measure: Measure) -> Name {
    match measure {
        Measure::Revenue => n("Revenue"),
        Measure::Occupancy => n("Occupancy"),
        Measure::Adr => n("ADR"),
        Measure::RevPar => n("RevPAR"),
    }
}

/// Name of the month-over-month companion metric for a [`Measure`].
/// Each is declared in `data::schema::hewton_schema` as a Lag-based
/// pct-change expression.
#[must_use]
pub fn measure_mom_metric(measure: Measure) -> Name {
    match measure {
        Measure::Revenue => n("RevenueMoM"),
        Measure::Occupancy => n("OccupancyMoM"),
        Measure::Adr => n("AdrMoM"),
        Measure::RevPar => n("RevParMoM"),
    }
}

/// KPI panel — scalar query returning the current measure + its MoM delta.
///
/// Metric slot 0 = primary value; slot 1 = month-over-month change.
/// `hyozu::tatami::card` reads those indices via `KpiLayout::primary` and
/// `KpiLayout::delta`.
#[must_use]
pub fn kpi(current: &Query, view: &View) -> Query {
    Query {
        axes: Axes::Scalar,
        slicer: current.slicer.clone(),
        metrics: vec![measure_metric(view.measure), measure_mom_metric(view.measure)],
        options: Options::default(),
    }
}

/// Map panel — one row per State, carrying two metrics.
///
/// Metric slot 0 is the active measure (drives choropleth fill); slot 1 is
/// `room_nights_sold` (drives centroid bubble size). The panel renderer reads
/// the two rows in that order.
#[must_use]
pub fn map(current: &Query, view: &View) -> Query {
    Query {
        axes: Axes::Series {
            rows: Set::members(n("Geography"), n("Default"), n("State")),
        },
        slicer: current.slicer.clone(),
        metrics: vec![measure_metric(view.measure), n("room_nights_sold")],
        options: Options::default(),
    }
}

/// Rail panel — Top-N brand tiers ranked by the current measure.
#[must_use]
pub fn rail(current: &Query, view: &View) -> Query {
    let metric = measure_metric(view.measure);
    Query {
        axes: Axes::Series {
            rows: Set::members(n("BrandTier"), n("Default"), n("Tier"))
                .top(NonZeroUsize::new(10).expect("10 > 0"), metric.clone()),
        },
        slicer: current.slicer.clone(),
        metrics: vec![metric],
        options: Options::default(),
    }
}

/// Time-series panel — one row per fiscal quarter for the current slicer.
#[must_use]
pub fn line(current: &Query, view: &View) -> Query {
    Query {
        axes: Axes::Series {
            rows: Set::members(n("Time"), n("Fiscal"), n("Quarter")),
        },
        slicer: current.slicer.clone(),
        metrics: vec![measure_metric(view.measure)],
        options: Options::default(),
    }
}

/// Filter-strip (chips) panel — pivot of Segment × Channel with counts.
///
/// View-invariant: this query does not depend on `view.measure` or
/// `view.period`, so [`crate::ui::Dashboard::dispatch_view_dependent`]
/// skips it.
#[must_use]
pub fn chips(current: &Query) -> Query {
    Query {
        axes: Axes::Pivot {
            rows: Set::members(n("Segment"), n("Default"), n("Segment")),
            columns: Set::members(n("Channel"), n("Default"), n("Channel")),
        },
        slicer: current.slicer.clone(),
        metrics: vec![n("Revenue")],
        options: Options::default(),
    }
}

fn n(s: &str) -> Name {
    Name::parse(s).expect("drill_down identifiers are valid")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::panel::Panel;

    #[test]
    fn nation_default_roundtrips_via_json() {
        let q = nation_default();
        let json = serde_json::to_string(&q).expect("serialize");
        let back: Query = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(q, back);
    }

    #[test]
    fn every_panel_produces_a_query_for_every_measure() {
        let base = nation_default();
        for measure in [Measure::Revenue, Measure::Occupancy, Measure::Adr, Measure::RevPar] {
            let view = View {
                measure,
                ..View::default()
            };
            for panel in Panel::ALL {
                let q = panel.query(&base, &view);
                assert!(
                    !q.metrics.is_empty(),
                    "panel {panel:?} measure {measure:?} produced empty metrics list"
                );
            }
        }
    }
}
