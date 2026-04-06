use iced::widget::{column, text};
use iced::{Element, Fill, Font, Theme, font};

use crate::model::sizing::Show;
use crate::{card, model, theme};

pub fn view<'a, Message: 'a>(
    card_model: &'a model::card::Card,
    state: &'a card::State,
    body: &'a model::metric::Body,
    theme: &'a Theme,
) -> Element<'a, Message> {
    let record = body.data.iter().find(|r| r.matches(&state.selections));

    let Some(record) = record else {
        return text("No data").size(11).color(theme::muted(theme)).into();
    };

    let value = state.shows(card_model, Show::Value).then(|| {
        text(&record.value)
            .size(28)
            .font(Font {
                weight: font::Weight::Bold,
                ..Font::DEFAULT
            })
            .into()
    });

    let delta = state
        .shows(card_model, Show::Delta)
        .then(|| record.delta.as_ref())
        .flatten()
        .map(|delta| {
            let delta_text = match &delta.period {
                Some(period) => format!("{} {}", delta.display, period),
                None => delta.display.clone(),
            };

            text(delta_text)
                .size(11)
                .color(theme::direction_color(delta.direction, theme))
                .into()
        });

    let chart = state
        .shows(card_model, Show::Chart)
        .then(|| state.chart_data.as_ref())
        .flatten()
        .map(|chart_data| {
            hyozu::chart(chart_data)
                .style(hyozu::chart::transparent)
                .width(Fill)
                .height(Fill)
                .into()
        });

    column([value, delta, chart].into_iter().flatten())
        .spacing(4)
        .width(Fill)
        .height(Fill)
        .into()
}

/// Build chart data from a metric body and its series.
pub(crate) fn build_chart(body: &model::metric::Body, series: &[model::metric::Series], theme: &Theme) -> hyozu::Data {
    let chart_kind = body.chart.as_ref().map(|c| c.kind).unwrap_or(model::metric::Kind::Line);

    match chart_kind {
        model::metric::Kind::Line => build_line(series, theme),
        model::metric::Kind::Bar => build_bar(series, theme),
    }
}

fn build_line(series: &[model::metric::Series], theme: &Theme) -> hyozu::Data {
    let needs_normalize = series.len() > 1 && {
        let maxes: Vec<f64> = series
            .iter()
            .map(|s| s.data.iter().copied().fold(f64::NEG_INFINITY, f64::max))
            .collect();

        maxes.len() >= 2 && {
            let ratio = maxes.iter().copied().fold(f64::NEG_INFINITY, f64::max)
                / maxes.iter().copied().fold(f64::INFINITY, f64::min).max(1.0);
            ratio > 10.0
        }
    };

    let marks: Vec<hyozu::Mark> = series
        .iter()
        .map(|s| {
            let data: Vec<f64> = if needs_normalize {
                normalize(&s.data)
            } else {
                s.data.clone()
            };

            let mut l = hyozu::line(data);
            if let Some(c) = s.color {
                l = l.color(theme::resolve(c, theme));
            }
            if let Some(n) = &s.label {
                l = l.with_name(n);
            }
            hyozu::Mark::from(l)
        })
        .collect();

    let mut data = hyozu::data(marks).x_axis(hyozu::Axis::none).y_axis(hyozu::Axis::none);

    if series.len() > 1 {
        data = data.legend(hyozu::LegendPosition::Above);
    }
    data
}

fn build_bar(series: &[model::metric::Series], theme: &Theme) -> hyozu::Data {
    let bar_series: Vec<_> = series
        .iter()
        .map(|s| {
            let mut b = hyozu::bar(s.data.clone());
            if let Some(c) = s.color {
                b = b.with_color(theme::resolve(c, theme));
            }
            if let Some(n) = &s.label {
                b = b.with_name(n);
            }
            b
        })
        .collect();

    let bars = hyozu::bars(bar_series).data_labels(None);

    let mut data = hyozu::data(bars).x_axis(hyozu::Axis::none).y_axis(hyozu::Axis::none);

    if series.len() > 1 {
        data = data.legend(hyozu::LegendPosition::Above);
    }
    data
}

fn normalize(data: &[f64]) -> Vec<f64> {
    let base = data.first().copied().unwrap_or(1.0);
    if base == 0.0 {
        return data.to_vec();
    }
    data.iter().map(|&v| (v / base - 1.0) * 100.0).collect()
}
