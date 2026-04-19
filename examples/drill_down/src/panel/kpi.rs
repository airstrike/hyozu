//! KPI panel — single scalar metric with caller-supplied title.
//!
//! The panel emits a `tatami::Query` with `Axes::Scalar` and a single
//! metric. When the metric pick is cleared the panel shows a placeholder
//! and does not fire.

use iced::widget::{Renderer, container, pick_list, row, text};
use iced::{Alignment, Element, Length, Theme};

use tatami::query::{Options, Tuple};
use tatami::schema::Schema;

use crate::{dashboard, metric};

/// Per-panel bindings.
#[derive(Debug, Clone)]
pub struct State {
    /// The metric displayed on the KPI tile.
    pub metric: Option<metric::Pick>,
}

impl State {
    /// Build a [`tatami::Query`] from this state against `schema`, seeded
    /// with the trail's slicer.
    ///
    /// Returns `None` when the metric pick is absent or does not resolve.
    #[must_use]
    pub fn query(&self, schema: &Schema, slicer: Tuple) -> Option<tatami::Query> {
        let name = metric::resolve(schema, self.metric?)?;
        Some(tatami::Query {
            axes: tatami::Axes::Scalar,
            slicer,
            metrics: vec![name],
            options: Options::default(),
        })
    }
}

/// Per-panel picker message.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum Message {
    /// User picked (or cleared) the panel's metric.
    MetricPicked(Option<metric::Pick>),
}

/// Apply a picker change. Returns `true` when the query must re-fire.
pub fn apply(state: &mut State, message: Message) -> bool {
    match message {
        Message::MetricPicked(pick) => {
            if state.metric == pick {
                return false;
            }
            state.metric = pick;
            true
        }
    }
}

/// Render the card body once the query has resolved.
#[must_use]
pub fn render<'a>(
    results: &'a tatami::Results,
    schema: &'a Schema,
    state: &State,
) -> Element<'a, dashboard::Message, Theme, Renderer> {
    let title = state
        .metric
        .and_then(|p| metric::label(schema, p))
        .unwrap_or("(metric)");
    let body: Element<'a, dashboard::Message, Theme, Renderer> = match results {
        tatami::Results::Scalar(scalar) => hyozu::tatami::card(scalar, hyozu::tatami::KpiLayout {
            title,
            subtitle: None,
            primary: 0,
            delta: None,
            sparkline: None,
        }),
        other => text(format!("unexpected KPI shape: {}", variant_tag(other)))
            .size(14)
            .into(),
    };
    container(body).padding([8, 12]).into()
}

/// Chrome row — a metric picker. Shown above the card body regardless of
/// query state so the user can change bindings while the query is pending.
#[must_use]
pub fn chrome<'a>(
    state: &State,
    metric_options: Vec<metric::Choice>,
) -> Element<'a, dashboard::Message, Theme, Renderer> {
    let selected = state
        .metric
        .and_then(|p| metric_options.iter().find(|c| c.pick == p).cloned());
    let picker = pick_list(selected, metric_options, |c: &metric::Choice| c.label.clone())
        .on_select(|c: metric::Choice| dashboard::Message::Kpi(Message::MetricPicked(Some(c.pick))))
        .placeholder("(metric)")
        .text_size(12)
        .width(Length::Fixed(180.0));

    row![text("Metric").size(12), picker]
        .spacing(6)
        .align_y(Alignment::Center)
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
