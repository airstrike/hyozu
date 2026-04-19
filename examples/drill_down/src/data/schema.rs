//! Hewton schema — six dimensions, three stored measures, six metrics.
//!
//! This schema is a structure-for-structure mirror of the reference
//! implementation at `~/projects/tatami/examples/hewton/src/schema.rs`.
//! The drill_down example ships hewton's bundled CSV (`assets/hewton.csv`
//! is a copy of the tatami example's data), so the schemas must match
//! column-for-column. When the upstream hewton schema changes, update here
//! too.

use tatami::Expr;
use tatami::schema::{
    self, Aggregation, BinOp, Calendar, Dimension, Hierarchy, Level, Measure, Metric, Name, Schema, Unit,
};

/// Build the Hewton schema. Returns a validated `tatami::Schema` on success.
pub fn hewton_schema() -> Result<Schema, schema::Error> {
    Schema::builder()
        .dimension(geography())
        .dimension(brand_tier())
        .dimension(channel())
        .dimension(segment())
        .dimension(time())
        .dimension(scenario())
        .measure(amount())
        .measure(room_nights_sold())
        .measure(rooms_available())
        .metric(revenue())
        .metric(adr())
        .metric(occupancy())
        .metric(revpar())
        .metric(revenue_yoy())
        .metric(revenue_mom())
        .metric(adr_mom())
        .metric(occupancy_mom())
        .metric(revpar_mom())
        .build()
}

// ── Dimensions ─────────────────────────────────────────────────────────────

fn geography() -> Dimension {
    Dimension::regular(n("Geography")).hierarchy(
        Hierarchy::new(n("Default"))
            .level(Level::new(n("World"), n("world")))
            .level(Level::new(n("Region"), n("region")))
            .level(Level::new(n("Country"), n("country")))
            .level(Level::new(n("State"), n("state"))),
    )
}

fn brand_tier() -> Dimension {
    Dimension::regular(n("BrandTier")).hierarchy(Hierarchy::new(n("Default")).level(Level::new(n("Tier"), n("tier"))))
}

fn channel() -> Dimension {
    Dimension::regular(n("Channel"))
        .hierarchy(Hierarchy::new(n("Default")).level(Level::new(n("Channel"), n("channel"))))
}

fn segment() -> Dimension {
    Dimension::regular(n("Segment"))
        .hierarchy(Hierarchy::new(n("Default")).level(Level::new(n("Segment"), n("segment"))))
}

fn time() -> Dimension {
    Dimension::time(n("Time"), vec![Calendar::gregorian(n("Gregorian"))]).hierarchy(
        Hierarchy::new(n("Fiscal"))
            .level(Level::new(n("FiscalYear"), n("fy")))
            .level(Level::new(n("Quarter"), n("quarter")))
            .level(Level::new(n("Month"), n("month"))),
    )
}

fn scenario() -> Dimension {
    Dimension::scenario(n("Scenario"))
        .hierarchy(Hierarchy::new(n("Default")).level(Level::new(n("Scenario"), n("scenario"))))
}

// ── Measures ───────────────────────────────────────────────────────────────

fn amount() -> Measure {
    Measure::new(n("amount"), Aggregation::sum()).with_unit(Unit::parse("USD").expect("usd"))
}

fn room_nights_sold() -> Measure {
    Measure::new(n("room_nights_sold"), Aggregation::sum())
}

fn rooms_available() -> Measure {
    Measure::new(n("rooms_available"), Aggregation::sum())
}

// ── Metrics ────────────────────────────────────────────────────────────────

fn revenue() -> Metric {
    Metric::new(n("Revenue"), Expr::Ref { name: n("amount") }).with_unit(Unit::parse("USD").expect("usd"))
}

fn adr() -> Metric {
    Metric::new(n("ADR"), div(ref_("Revenue"), ref_("room_nights_sold"))).with_unit(Unit::parse("USD").expect("usd"))
}

fn occupancy() -> Metric {
    Metric::new(n("Occupancy"), div(ref_("room_nights_sold"), ref_("rooms_available"))).with_format("0.0%".into())
}

fn revpar() -> Metric {
    Metric::new(n("RevPAR"), div(ref_("Revenue"), ref_("rooms_available"))).with_unit(Unit::parse("USD").expect("usd"))
}

fn revenue_yoy() -> Metric {
    let prev = lag(ref_("Revenue"), "Time", 12);
    Metric::new(n("RevenueYoY"), div(sub(ref_("Revenue"), prev.clone()), prev)).with_format("0.0%".into())
}

fn revenue_mom() -> Metric {
    mom_metric("RevenueMoM", "Revenue")
}

fn adr_mom() -> Metric {
    mom_metric("AdrMoM", "ADR")
}

fn occupancy_mom() -> Metric {
    mom_metric("OccupancyMoM", "Occupancy")
}

fn revpar_mom() -> Metric {
    mom_metric("RevParMoM", "RevPAR")
}

/// Month-over-month change of any metric: `(cur - prev) / prev`.
fn mom_metric(mom_name: &str, base: &str) -> Metric {
    let prev = lag(ref_(base), "Time", 1);
    Metric::new(n(mom_name), div(sub(ref_(base), prev.clone()), prev)).with_format("0.0%".into())
}

// ── Local helpers — keep call sites readable ───────────────────────────────

fn ref_(name: &str) -> Expr {
    Expr::Ref { name: n(name) }
}

fn div(l: Expr, r: Expr) -> Expr {
    Expr::Binary {
        bin_op: BinOp::Div,
        l: Box::new(l),
        r: Box::new(r),
    }
}

fn sub(l: Expr, r: Expr) -> Expr {
    Expr::Binary {
        bin_op: BinOp::Sub,
        l: Box::new(l),
        r: Box::new(r),
    }
}

fn lag(of: Expr, dim: &str, n_periods: i32) -> Expr {
    Expr::Lag {
        of: Box::new(of),
        dim: n(dim),
        n: n_periods,
    }
}

fn n(s: &str) -> Name {
    Name::parse(s).expect("hewton identifiers are valid")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hewton_schema_builds_successfully() {
        let schema = hewton_schema().expect("schema builds");
        assert!(!schema.dimensions.is_empty());
        assert!(!schema.measures.is_empty());
    }

    #[test]
    fn hewton_schema_declares_expected_metrics() {
        let schema = hewton_schema().expect("schema builds");
        let metric_names: Vec<&str> = schema.metrics.iter().map(|m| m.name.as_str()).collect();
        for expected in ["Revenue", "ADR", "Occupancy", "RevPAR", "RevenueYoY", "RevenueMoM"] {
            assert!(
                metric_names.contains(&expected),
                "metric {expected} missing; have {metric_names:?}"
            );
        }
    }
}
