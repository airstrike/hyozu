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
        .spacing(2)
        .width(Fill)
        .height(Fill)
        .into()
}

/// Build chart data from a metric body and a matched record.
///
/// Each chart kind pulls the fields it needs from the record:
/// line/bar use `series` + `labels`, map uses `geo_points`.
pub(crate) fn build_chart(
    body: &model::metric::Body,
    record: &model::metric::Record,
    theme: &Theme,
) -> Option<hyozu::Data> {
    let chart_kind = body.chart.as_ref().map(|c| c.kind).unwrap_or(model::metric::Kind::Line);

    match chart_kind {
        model::metric::Kind::Line => {
            let series = record.series.as_ref()?;
            let mut data = build_line(series, theme);
            if let Some(labels) = &record.labels {
                data = data.x_axis_labels(labels.clone());
            }
            Some(data)
        }
        model::metric::Kind::Bar => {
            let series = record.series.as_ref()?;
            let mut data = build_bar(series, theme);
            if let Some(labels) = &record.labels {
                data = data.x_axis_labels(labels.clone());
            }
            Some(data)
        }
        model::metric::Kind::Map => {
            let geo_points = record.geo_points.as_ref()?;
            Some(build_map(geo_points, theme))
        }
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

    let muted = theme::muted(theme);
    let mut data = hyozu::data(marks)
        .x_axis(|a| muted_axis(a, muted))
        .y_axis(hyozu::Axis::none)
        .tooltip(hyozu::Swatch + hyozu::ColoredText);

    if series.len() > 1 {
        data = data.legend(hyozu::legend::Config::above());
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

    let muted = theme::muted(theme);
    let mut data = hyozu::data(bars)
        .x_axis(|a| muted_axis(a, muted))
        .y_axis(hyozu::Axis::none)
        .tooltip(hyozu::ColoredText);

    if series.len() > 1 {
        data = data.legend(hyozu::legend::Config::above());
    }
    data
}

/// X-axis with muted styling — very subtle line and small labels.
fn muted_axis(axis: hyozu::Axis, color: iced::Color) -> hyozu::Axis {
    let faint = iced::Color { a: 0.5, ..color };
    axis.show_grid(false)
        .with_axis_color(faint)
        .with_label_color(faint)
        .with_label_size(9)
}

/// Build chart data from geo points for a bubble-map card.
pub(crate) fn build_map(geo_points: &[model::metric::GeoPoint], theme: &Theme) -> hyozu::Data {
    let points: Vec<_> = geo_points
        .iter()
        .map(|gp| {
            let mut pt = hyozu::map_point(gp.lat, gp.lon, gp.value);
            if let Some(label) = &gp.label {
                pt = pt.label(label);
            }
            if let Some(c) = gp.color {
                pt = pt.color(theme::resolve(c, theme));
            }
            pt
        })
        .collect();

    hyozu::data(hyozu::bubble_map(points))
}

fn normalize(data: &[f64]) -> Vec<f64> {
    let base = data.first().copied().unwrap_or(1.0);
    if base == 0.0 {
        return data.to_vec();
    }
    data.iter().map(|&v| (v / base - 1.0) * 100.0).collect()
}
