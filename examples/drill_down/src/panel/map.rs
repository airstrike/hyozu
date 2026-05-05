//! Map panel — choropleth + centroid bubbles over a geographic level.
//!
//! The panel's `rows` axis selects the dim+hierarchy+level whose members
//! are the map's features; `fill` is the metric driving the choropleth
//! colour; `size` is an optional metric driving the bubble magnitudes.

use std::sync::Arc;

use iced::widget::{Renderer, pick_list, row, text};
use iced::{Alignment, Element, Length, Theme};

use hyozu::{GeoData, MapScope, legend};
use tatami::query::{MemberRef, Options, Tuple};
use tatami::schema::Schema;
use tatami::series;

use crate::{axis, dashboard, data, metric};

/// Per-panel bindings.
#[derive(Debug, Clone)]
pub struct State {
    /// The dim + level whose members the map plots.
    pub rows: axis::Pick,
    /// Metric driving the choropleth fill.
    pub fill: Option<metric::Pick>,
    /// Optional metric driving the centroid bubble magnitude. `None` hides
    /// the bubble layer.
    pub size: Option<metric::Pick>,
    /// Which geographic scope the map zooms to.
    pub scope: MapScope,
}

impl State {
    /// Build a [`tatami::Query`] for this panel. Returns `None` when rows
    /// or the fill metric are absent / unresolvable.
    #[must_use]
    pub fn query(&self, schema: &Schema, slicer: Tuple) -> Option<tatami::Query> {
        let rows = axis::build_set(schema, &self.rows)?;
        let fill = metric::resolve(schema, self.fill?)?;
        let mut metrics = vec![fill];
        if let Some(pick) = self.size
            && let Some(name) = metric::resolve(schema, pick)
        {
            metrics.push(name);
        }
        Some(tatami::Query {
            axes: tatami::Axes::Series { rows },
            slicer,
            metrics,
            options: Options::default(),
        })
    }

    /// Resolve a clicked feature id to the full-path [`MemberRef`] carried
    /// in the last map series result. The `series` argument is the
    /// x-axis backing the currently-rendered map; matching against its
    /// members preserves the root-to-leaf path (e.g. `World/West/US/CA`)
    /// needed for drill-down, rather than synthesising a single-segment
    /// path from the feature id alone.
    #[must_use]
    pub fn drill_member(&self, series: &series::Result, id: &str) -> Option<MemberRef> {
        series.x().iter().find(|m| leaf(&m.path).as_str() == id).cloned()
    }
}

/// Return the leaf (deepest) segment of a path.
fn leaf(path: &tatami::Path) -> &tatami::schema::Name {
    path.segments().last().expect("path is non-empty")
}

/// Build the chart `Data` for the map panel from a `Results::Series`.
///
/// Returns `None` when the results shape isn't `Series`.
#[must_use]
pub fn build_data(
    results: &tatami::Results,
    geo: &Arc<GeoData>,
    centroids: &Arc<data::Centroids>,
    scope: MapScope,
    selected: Vec<String>,
) -> Option<hyozu::Data> {
    let tatami::Results::Series(series) = results else {
        return None;
    };

    let fill_row = series.rows().first()?;
    let size_row = series.rows().get(1);

    let mut entries: Vec<hyozu::ChoroplethEntry> = Vec::with_capacity(series.x().len());
    for (i, (member, fill_cell)) in series.x().iter().zip(fill_row.values.iter()).enumerate() {
        let id = leaf(&member.path).as_str().to_owned();
        let fill = hyozu::tatami::cell_f64(fill_cell);
        if fill.is_finite() {
            entries.push(hyozu::choropleth_entry(id, fill));
            continue;
        }
        // No fill value, but mark the feature as "available" if the size
        // metric has a positive value here — a sibling layer will draw
        // the bubble, so the choropleth should render the feature with a
        // muted neutral fill instead of fading it into the land background.
        let has_size = size_row
            .and_then(|row| row.values.get(i))
            .map(|cell| {
                let v = hyozu::tatami::cell_f64(cell);
                v.is_finite() && v > 0.0
            })
            .unwrap_or(false);
        if has_size {
            entries.push(hyozu::choropleth_entry_available(id));
        }
    }

    let mut points: Vec<hyozu::MapPoint> = Vec::with_capacity(series.x().len());
    if let Some(size_row) = size_row {
        for (member, cell) in series.x().iter().zip(size_row.values.iter()) {
            let code = leaf(&member.path).as_str();
            let Some((lat, lon)) = centroids.get(code) else {
                continue;
            };
            let value = hyozu::tatami::cell_f64(cell);
            let has_data = !value.is_nan() && value > 0.0;
            let magnitude = if has_data { value } else { 0.0 };
            let mut point = hyozu::map_point(lat, lon, magnitude).label(code.to_owned());
            if has_data {
                point = point.id(code.to_owned());
            }
            points.push(point);
        }
    }

    let mut choropleth_mark = hyozu::choropleth(entries).legend(legend::Config::right());
    if !selected.is_empty() {
        choropleth_mark = choropleth_mark.selected(selected);
    }
    let choropleth = hyozu::Mark::Choropleth(choropleth_mark);
    let bubbles = hyozu::Mark::Xy(hyozu::bubble_map(points));
    Some(
        hyozu::data(vec![choropleth, bubbles])
            .geo_data(geo.clone())
            .geo_scope(scope)
            .geo_basemap(false),
    )
}

/// Per-panel picker message.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum Message {
    /// User picked (or cleared) the rows dim.
    RowsDimPicked(Option<axis::DimChoice>),
    /// User picked the rows level.
    RowsLevelPicked(Option<axis::LevelChoice>),
    /// User picked (or cleared) the fill metric.
    FillPicked(Option<metric::Pick>),
    /// User picked (or cleared) the bubble-size metric.
    SizePicked(Option<metric::Pick>),
}

/// Apply a picker change. Returns `true` when the query must re-fire.
pub fn apply(state: &mut State, schema: &Schema, message: Message) -> bool {
    match message {
        Message::RowsDimPicked(choice) => {
            let next = axis::axis_for(schema, choice);
            if state.rows == next {
                return false;
            }
            state.rows = next;
            true
        }
        Message::RowsLevelPicked(choice) => {
            let next = axis::level_for(state.rows, choice);
            if state.rows == next {
                return false;
            }
            state.rows = next;
            true
        }
        Message::FillPicked(pick) => {
            if state.fill == pick {
                return false;
            }
            state.fill = pick;
            true
        }
        Message::SizePicked(pick) => {
            if state.size == pick {
                return false;
            }
            state.size = pick;
            true
        }
    }
}

/// Render the map card body from a cached `Data`.
#[must_use]
pub fn render<'a>(data: &'a hyozu::Data) -> Element<'a, dashboard::Message, Theme, Renderer> {
    hyozu::chart(data).height(Length::Fill).into()
}

/// Fallback text for shapes the panel can't render.
#[must_use]
pub fn fallback(results: &tatami::Results) -> Element<'static, dashboard::Message, Theme, Renderer> {
    text(format!("unexpected map shape: {}", variant_tag(results)))
        .size(14)
        .into()
}

/// Chrome row — rows dim / rows level / fill metric / size metric pickers.
#[must_use]
pub fn chrome<'a>(
    schema: &'a Schema,
    state: &State,
    dim_options: Vec<axis::DimChoice>,
    metric_options: Vec<metric::Choice>,
) -> Element<'a, dashboard::Message, Theme, Renderer> {
    let selected_dim = axis::current_dim(&dim_options, &state.rows);
    let dim_picker = pick_list(selected_dim, dim_options, |c: &axis::DimChoice| c.label.clone())
        .on_select(|c: axis::DimChoice| dashboard::Message::Map(Message::RowsDimPicked(Some(c))))
        .placeholder("(rows)")
        .text_size(12)
        .width(Length::Fixed(140.0));

    let level_element: Element<'a, dashboard::Message, Theme, Renderer> = match state.rows {
        axis::Pick::Pick { dim, hierarchy, level } => {
            let options = axis::level_choices(schema, dim);
            let selected = options
                .iter()
                .find(|c| c.hierarchy == hierarchy && c.level == level)
                .cloned();
            pick_list(selected, options, |c: &axis::LevelChoice| c.label.clone())
                .on_select(|c: axis::LevelChoice| dashboard::Message::Map(Message::RowsLevelPicked(Some(c))))
                .placeholder("(level)")
                .text_size(12)
                .width(Length::Fixed(140.0))
                .into()
        }
        axis::Pick::None => text("").into(),
    };

    let fill_selected = state
        .fill
        .and_then(|p| metric_options.iter().find(|c| c.pick == p).cloned());
    let fill_picker = pick_list(fill_selected, metric_options.clone(), |c: &metric::Choice| {
        c.label.clone()
    })
    .on_select(|c: metric::Choice| dashboard::Message::Map(Message::FillPicked(Some(c.pick))))
    .placeholder("(fill)")
    .text_size(12)
    .width(Length::Fixed(160.0));

    let size_selected = state
        .size
        .and_then(|p| metric_options.iter().find(|c| c.pick == p).cloned());
    let size_picker = pick_list(size_selected, metric_options, |c: &metric::Choice| c.label.clone())
        .on_select(|c: metric::Choice| dashboard::Message::Map(Message::SizePicked(Some(c.pick))))
        .placeholder("(size)")
        .text_size(12)
        .width(Length::Fixed(160.0));

    let rows_group = row![text("Rows").size(12), dim_picker, level_element]
        .spacing(4)
        .align_y(Alignment::Center);
    let fill_group = row![text("Fill").size(12), fill_picker]
        .spacing(4)
        .align_y(Alignment::Center);
    let size_group = row![text("Size").size(12), size_picker]
        .spacing(4)
        .align_y(Alignment::Center);

    row![rows_group, fill_group, size_group]
        .spacing(12)
        .align_y(Alignment::Center)
        .wrap()
        .into()
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

    fn state_mr_deep(leaf_name: &str) -> MemberRef {
        MemberRef::new(
            name("Geography"),
            name("Default"),
            Path::with(name("World"), vec![name("North"), name("US"), name(leaf_name)]),
        )
    }

    fn valid(v: f64) -> Cell {
        Cell::Valid {
            value: v,
            unit: None,
            format: None,
        }
    }

    fn missing() -> Cell {
        Cell::Missing {
            reason: tatami::missing::Reason::NoFacts,
        }
    }

    #[test]
    fn non_series_results_yield_none() {
        let scalar = tatami::Results::Scalar(tatami::scalar::Result::new(Tuple::empty(), vec![valid(1.0)]));
        let geo = Arc::new(GeoData::new(Vec::new()));
        let centroids = Arc::new(data::Centroids::from_geo(&geo));
        assert!(build_data(&scalar, &geo, &centroids, MapScope::UnitedStates, Vec::new()).is_none());
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
        let data = build_data(&results, &geo, &centroids, MapScope::UnitedStates, Vec::new()).expect("built");
        assert_eq!(data.marks().len(), 2);
        match &data.marks()[0] {
            hyozu::Mark::Choropleth(c) => assert_eq!(c.entries().len(), 2),
            _ => panic!("expected Choropleth as first mark"),
        }
        match &data.marks()[1] {
            hyozu::Mark::Xy(xy) => assert_eq!(xy.points().len(), 0),
            _ => panic!("expected Xy (bubble_map) as second mark"),
        }
    }

    #[test]
    fn missing_fill_with_positive_size_yields_available_entry() {
        // CA has both fill and size → Value entry.
        // TX has missing fill but a positive size → Available entry.
        // NM has missing fill and zero size → no entry at all.
        let result = series::Result::new(vec![state_mr("CA"), state_mr("TX"), state_mr("NM")], vec![
            series::Row {
                label: "Revenue".into(),
                values: vec![valid(4_500_000.0), missing(), missing()],
            },
            series::Row {
                label: "room_nights_sold".into(),
                values: vec![valid(723.0), valid(150.0), valid(0.0)],
            },
        ]);
        let results = tatami::Results::Series(result);
        let geo = Arc::new(GeoData::new(Vec::new()));
        let centroids = Arc::new(data::Centroids::from_geo(&geo));
        let data = build_data(&results, &geo, &centroids, MapScope::UnitedStates, Vec::new()).expect("built");

        let hyozu::Mark::Choropleth(c) = &data.marks()[0] else {
            panic!("expected Choropleth as first mark");
        };
        let entries = c.entries();
        assert_eq!(entries.len(), 2, "NM with no fill and zero size should be skipped");

        let ca = entries.iter().find(|e| e.id().as_str() == "CA").expect("CA entry");
        assert!(matches!(ca.value(), Some(v) if v == 4_500_000.0), "CA value");

        let tx = entries.iter().find(|e| e.id().as_str() == "TX").expect("TX entry");
        assert!(tx.value().is_none(), "TX should be value-less Available entry");
    }

    #[test]
    fn drill_member_returns_full_path_for_leaf_click() {
        let result = series::Result::new(vec![state_mr_deep("CA"), state_mr_deep("NY")], vec![series::Row {
            label: "Revenue".into(),
            values: vec![valid(10.0), valid(20.0)],
        }]);
        let state = State {
            rows: axis::Pick::None,
            fill: None,
            size: None,
            scope: MapScope::UnitedStates,
        };
        let picked = state.drill_member(&result, "CA").expect("resolved");
        assert_eq!(picked.path.len(), 4);
        assert_eq!(leaf(&picked.path).as_str(), "CA");
    }
}
