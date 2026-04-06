use std::collections::BTreeMap;

use iced::widget::column;
use iced::{Element, Fill, Theme};

use crate::model;

pub mod footer;
pub mod metric;
pub mod table;

/// UI state for a card — sits over the model data.
pub struct State {
    pub selections: BTreeMap<String, String>,
    pub active_variant: model::sizing::Name,
    /// Cached chart data, rebuilt on selector change.
    pub chart_data: Option<hyozu::Data>,
}

impl State {
    pub fn new(card: &model::card::Card, theme: &Theme) -> Self {
        let selections: BTreeMap<String, String> = card
            .selectors
            .iter()
            .map(|(k, s)| (k.clone(), s.default.clone()))
            .collect();

        let chart_data = Self::build_chart(card, &selections, theme);

        Self {
            selections,
            active_variant: card.sizing.default,
            chart_data,
        }
    }

    /// The currently active sizing variant.
    pub fn variant<'a>(&self, card: &'a model::card::Card) -> &'a model::sizing::Variant {
        &card.sizing.variants[&self.active_variant]
    }

    /// Whether the active variant's show array contains an element.
    pub fn shows(&self, card: &model::card::Card, show: model::sizing::Show) -> bool {
        self.variant(card).show.contains(&show)
    }

    /// Rebuild cached chart data from the current selector state.
    pub fn rebuild_chart(&mut self, card: &model::card::Card, theme: &Theme) {
        self.chart_data = Self::build_chart(card, &self.selections, theme);
    }

    fn build_chart(
        card: &model::card::Card,
        selections: &BTreeMap<String, String>,
        theme: &Theme,
    ) -> Option<hyozu::Data> {
        let model::card::Body::Metric(body) = &card.body else {
            return None;
        };

        let record = body.data.iter().find(|r| r.matches(selections))?;
        let series = record.series.as_ref()?;
        let labels = record.labels.as_deref();

        Some(metric::build_chart(body, series, labels, theme))
    }
}

/// Card body view — value/delta/chart (metric) or table rows.
/// Header with selectors is rendered by dashboard.rs in the title_bar.
pub fn view<'a>(
    card: &'a model::card::Card,
    state: &'a State,
    theme: &'a Theme,
) -> Element<'a, crate::dashboard::Message> {
    let body: Element<'a, crate::dashboard::Message> = match &card.body {
        model::card::Body::Metric(body) => metric::view(card, state, body, theme),
        model::card::Body::Table(body) => table::view(card, state, body, theme),
    };

    let footer = footer::view(card, theme);

    column![body, footer]
        .spacing(4)
        .padding(iced::padding::all(10).top(0))
        .width(Fill)
        .height(Fill)
        .into()
}
