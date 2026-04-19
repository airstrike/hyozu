//! Map panel — choropleth + centroid bubbles over a geographic level.
//!
//! The panel's `rows` axis selects the dim+hierarchy+level whose members
//! are the map's features; `fill` is the metric driving the choropleth
//! colour; `size` is an optional metric driving the bubble magnitudes.

use std::sync::Arc;

use iced::widget::{Renderer, pick_list, row, text};
use iced::{Alignment, Element, Length, Theme};

use hyozu::{GeoData, MapScope};
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
) -> Option<hyozu::Data> {
    let tatami::Results::Series(series) = results else {
        return None;
    };

    let fill_row = series.rows().first()?;
    let size_row = series.rows().get(1);

    let mut entries: Vec<hyozu::ChoroplethEntry> = Vec::with_capacity(series.x().len());
    for (member, cell) in series.x().iter().zip(fill_row.values.iter()) {
        let id = leaf(&member.path).as_str().to_owned();
        entries.push(hyozu::choropleth_entry(id, hyozu::tatami::cell_f64(cell)));
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

    let choropleth = hyozu::Mark::Choropleth(hyozu::choropleth(entries).geo(geo.clone()).scope(scope));
    let bubbles = hyozu::Mark::BubbleMap(hyozu::bubble_map(points).geo(geo.clone()).scope(scope).no_basemap());
    Some(hyozu::data(vec![choropleth, bubbles]))
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
    .width(Length::Fixed(160.0));

    let size_selected = state
        .size
        .and_then(|p| metric_options.iter().find(|c| c.pick == p).cloned());
    let size_picker = pick_list(size_selected, metric_options, |c: &metric::Choice| c.label.clone())
        .on_select(|c: metric::Choice| dashboard::Message::Map(Message::SizePicked(Some(c.pick))))
        .placeholder("(size)")
        .width(Length::Fixed(160.0));

    row![
        text("Rows").size(12),
        dim_picker,
        level_element,
        text("Fill").size(12),
        fill_picker,
        text("Size").size(12),
        size_picker,
    ]
    .spacing(6)
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

    #[test]
    fn non_series_results_yield_none() {
        let scalar = tatami::Results::Scalar(tatami::scalar::Result::new(Tuple::empty(), vec![valid(1.0)]));
        let geo = Arc::new(GeoData::new(Vec::new()));
        let centroids = Arc::new(data::Centroids::from_geo(&geo));
        assert!(build_data(&scalar, &geo, &centroids, MapScope::UnitedStates).is_none());
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
        let data = build_data(&results, &geo, &centroids, MapScope::UnitedStates).expect("built");
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
