//! Map panel rendering — choropleth + centroid bubbles over US states.
//!
//! The `queries::map` builder emits a two-metric `Axes::Series` over
//! `Geography.State`. Row 0 carries the active measure (fills the choropleth);
//! row 1 carries `room_nights_sold` (sizes each centroid bubble). The bubble
//! mark is layered atop the choropleth without redrawing polygons
//! (`.no_basemap()`), so the two marks compose in one `hyozu::Data`. Each
//! bubble carries its state code as `id`, so click handlers can route it
//! into a drill navigate.

use std::sync::Arc;

use iced::widget::Renderer;
use iced::{Element, Theme};

use hyozu::{GeoData, MapScope};

use crate::data;
use crate::ui::dashboard::Message;
use crate::ui::view::Measure;

/// Build the chart `Data` for the map panel from a `Results::Series`.
///
/// Returns `None` when the results shape isn't `Series` — the caller falls
/// back to a placeholder. Owning the `Data` at the call site keeps the
/// chart widget's `&Data` borrow valid for the rendered frame.
#[must_use]
pub fn build_data(
    results: &tatami::Results,
    geo: &Arc<GeoData>,
    centroids: &Arc<data::Centroids>,
) -> Option<hyozu::Data> {
    let tatami::Results::Series(series) = results else {
        return None;
    };

    let fill_row = series.rows().first()?;
    let size_row = series.rows().get(1);

    let mut entries: Vec<hyozu::ChoroplethEntry> = Vec::with_capacity(series.x().len());
    for (member, cell) in series.x().iter().zip(fill_row.values.iter()) {
        let id = member.path.head().as_str().to_owned();
        entries.push(hyozu::choropleth_entry(id, hyozu::tatami::cell_f64(cell)));
    }

    let mut points: Vec<hyozu::MapPoint> = Vec::with_capacity(series.x().len());
    if let Some(size_row) = size_row {
        for (member, cell) in series.x().iter().zip(size_row.values.iter()) {
            let code = member.path.head().as_str();
            let Some((lat, lon)) = centroids.get(code) else {
                continue;
            };
            let magnitude = {
                let v = hyozu::tatami::cell_f64(cell);
                if v.is_nan() { 0.0 } else { v }
            };
            points.push(hyozu::map_point(lat, lon, magnitude).id(code.to_owned()));
        }
    }

    let choropleth = hyozu::Mark::Choropleth(
        hyozu::choropleth(entries)
            .geo(geo.clone())
            .scope(MapScope::UnitedStates),
    );
    let bubbles = hyozu::Mark::BubbleMap(
        hyozu::bubble_map(points)
            .geo(geo.clone())
            .scope(MapScope::UnitedStates)
            .no_basemap(),
    );

    Some(hyozu::data(vec![choropleth, bubbles]))
}

/// Render the map panel from an owned `Data`. Click handling is deferred —
/// a later slice wires `Action::Clicked(Target::Feature { id, .. })` into a
/// state drill via `Dashboard::navigate`.
#[must_use]
pub fn render<'a>(data: &'a hyozu::Data, _measure: Measure) -> Element<'a, Message, Theme, Renderer> {
    hyozu::chart(data).height(iced::Length::Fill).into()
}

/// Fallback text when the results aren't a Series (shouldn't happen — the
/// map panel's query always resolves to `Axes::Series`).
#[must_use]
pub fn fallback(results: &tatami::Results) -> Element<'static, Message, Theme, Renderer> {
    let tag = match results {
        tatami::Results::Scalar(_) => "Scalar",
        tatami::Results::Series(_) => "Series",
        tatami::Results::Pivot(_) => "Pivot",
        tatami::Results::Rollup(_) => "Rollup",
        _ => "<unknown>",
    };
    iced::widget::text(format!("unexpected map shape: {tag}"))
        .size(14)
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    use hyozu::GeoData;
    use tatami::query::{MemberRef, Path, Tuple};
    use tatami::schema::Name;
    use tatami::{Cell, series};

    fn name(s: &str) -> Name {
        Name::parse(s).expect("valid")
    }

    fn state_mr(head: &str) -> MemberRef {
        MemberRef::new(name("Geography"), name("Default"), Path::of(name(head)))
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
        let geo = Arc::new(GeoData::new(Vec::new()));
        let centroids = Arc::new(data::Centroids::from_geo(&geo));
        assert!(build_data(&scalar, &geo, &centroids).is_none());
    }

    #[test]
    fn series_without_centroids_still_yields_choropleth() {
        let result = series::Result::new(vec![state_mr("CA"), state_mr("NY")], vec![
            series::Row {
                label: "Revenue".into(),
                values: vec![valid(10.0), valid(20.0)],
            },
            series::Row {
                label: "room_nights_sold".into(),
                values: vec![valid(100.0), valid(200.0)],
            },
        ]);
        let results = tatami::Results::Series(result);
        let geo = Arc::new(GeoData::new(Vec::new()));
        let centroids = Arc::new(data::Centroids::from_geo(&geo));
        let data = build_data(&results, &geo, &centroids).expect("series built");
        assert_eq!(data.marks().len(), 2);
        match &data.marks()[0] {
            hyozu::Mark::Choropleth(c) => assert_eq!(c.entries().len(), 2),
            _ => panic!("expected Choropleth as first mark"),
        }
        match &data.marks()[1] {
            hyozu::Mark::BubbleMap(bm) => assert_eq!(bm.points().len(), 0),
            _ => panic!("expected BubbleMap as second mark"),
        }
    }
}
