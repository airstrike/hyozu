//! Line panel — a time-series for the chosen metric across the chosen
//! level.
//!
//! The `rows` axis selects the dim + level whose members appear on the
//! x-axis; `metric` picks the single metric plotted. Renders as a single
//! `Mark::Line`.

use iced::widget::{Renderer, pick_list, row, text};
use iced::{Alignment, Element, Length, Theme};

use tatami::query::{Options, Tuple};
use tatami::schema::Schema;

use crate::{axis, dashboard, metric};

/// Per-panel bindings.
#[derive(Debug, Clone)]
pub struct State {
    /// The dim + level whose members appear on the x-axis.
    pub rows: axis::Pick,
    /// Metric plotted on the y-axis.
    pub metric: Option<metric::Pick>,
}

impl State {
    /// Build a [`tatami::Query`] for this panel. Returns `None` when rows
    /// or the metric are absent / unresolvable.
    #[must_use]
    pub fn query(&self, schema: &Schema, slicer: Tuple) -> Option<tatami::Query> {
        let rows = axis::build_set(schema, &self.rows)?;
        let name = metric::resolve(schema, self.metric?)?;
        Some(tatami::Query {
            axes: tatami::Axes::Series { rows },
            slicer,
            metrics: vec![name],
            options: Options::default(),
        })
    }
}

/// Build the chart `Data` for the line panel from a `Results::Series`.
#[must_use]
pub fn build_data(results: &tatami::Results) -> Option<hyozu::Data> {
    let tatami::Results::Series(series) = results else {
        return None;
    };
    Some(hyozu::tatami::line(series))
}

/// Per-panel picker message.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum Message {
    /// User picked (or cleared) the rows dim.
    RowsDimPicked(Option<axis::DimChoice>),
    /// User picked the rows level.
    RowsLevelPicked(Option<axis::LevelChoice>),
    /// User picked (or cleared) the panel's metric.
    MetricPicked(Option<metric::Pick>),
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
        Message::MetricPicked(pick) => {
            if state.metric == pick {
                return false;
            }
            state.metric = pick;
            true
        }
    }
}

/// Render the line card body from a cached `Data`.
#[must_use]
pub fn render<'a>(data: &'a hyozu::Data) -> Element<'a, dashboard::Message, Theme, Renderer> {
    hyozu::chart(data).height(Length::Fixed(200.0)).into()
}

/// Fallback text for shapes the panel can't render.
#[must_use]
pub fn fallback(results: &tatami::Results) -> Element<'static, dashboard::Message, Theme, Renderer> {
    text(format!("unexpected line shape: {}", variant_tag(results)))
        .size(14)
        .into()
}

/// Chrome row — rows dim / rows level / metric pickers.
#[must_use]
pub fn chrome<'a>(
    schema: &'a Schema,
    state: &State,
    dim_options: Vec<axis::DimChoice>,
    metric_options: Vec<metric::Choice>,
) -> Element<'a, dashboard::Message, Theme, Renderer> {
    let selected_dim = axis::current_dim(&dim_options, &state.rows);
    let dim_picker = pick_list(selected_dim, dim_options, |c: &axis::DimChoice| c.label.clone())
        .on_select(|c: axis::DimChoice| dashboard::Message::Line(Message::RowsDimPicked(Some(c))))
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
                .on_select(|c: axis::LevelChoice| dashboard::Message::Line(Message::RowsLevelPicked(Some(c))))
                .placeholder("(level)")
                .width(Length::Fixed(140.0))
                .into()
        }
        axis::Pick::None => text("").into(),
    };

    let metric_selected = state
        .metric
        .and_then(|p| metric_options.iter().find(|c| c.pick == p).cloned());
    let metric_picker = pick_list(metric_selected, metric_options, |c: &metric::Choice| c.label.clone())
        .on_select(|c: metric::Choice| dashboard::Message::Line(Message::MetricPicked(Some(c.pick))))
        .placeholder("(metric)")
        .width(Length::Fixed(180.0));

    row![
        text("Rows").size(12),
        dim_picker,
        level_element,
        text("Metric").size(12),
        metric_picker,
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
