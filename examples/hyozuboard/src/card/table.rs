use iced::widget::{column, container, row, scrollable, text};
use iced::{Element, Fill, Font, Theme, font};

use crate::model::sizing::Show;
use crate::{card, model, theme};

pub fn view<'a, Message: 'a>(
    card_model: &'a model::card::Card,
    state: &'a card::State,
    body: &'a model::table::Body,
    theme: &'a Theme,
) -> Element<'a, Message> {
    let variant = state.variant(card_model);
    let show_headers = variant.show.contains(&Show::Headers);
    let max_rows = variant.max_rows.unwrap_or(usize::MAX);
    let muted = theme::muted(theme);

    let visible_cols: Vec<_> = body
        .columns
        .iter()
        .filter(|c| c.kind != model::table::Kind::Url)
        .collect();

    let header = show_headers.then(|| header_row(&visible_cols, muted));

    let data = body
        .data
        .iter()
        .filter(|r| matches_selectors(r, &state.selections))
        .take(max_rows)
        .map(|r| data_row(&visible_cols, r, muted));

    let all_rows: Vec<Element<'a, Message>> = header.into_iter().chain(data).collect();

    if all_rows.is_empty() {
        return text("No matching rows").size(12).color(muted).into();
    }

    scrollable(column(all_rows).spacing(4).width(Fill))
        .width(Fill)
        .height(Fill)
        .into()
}

fn matches_selectors(
    row_data: &std::collections::BTreeMap<String, serde_json::Value>,
    selections: &std::collections::BTreeMap<String, String>,
) -> bool {
    selections
        .iter()
        .all(|(key, val)| val == "All" || row_data.get(key).and_then(|v| v.as_str()).map_or(true, |rv| rv == val))
}

fn header_row<'a, Message: 'a>(cols: &[&model::table::Column], muted: iced::Color) -> Element<'a, Message> {
    row(cols.iter().map(|col| {
        let label = col.label.as_deref().unwrap_or(&col.key);
        text(label.to_uppercase())
            .size(10)
            .font(Font {
                weight: font::Weight::Bold,
                ..Font::DEFAULT
            })
            .color(muted)
            .width(Fill)
            .into()
    }))
    .spacing(8)
    .into()
}

fn data_row<'a, Message: 'a>(
    cols: &[&model::table::Column],
    row_data: &std::collections::BTreeMap<String, serde_json::Value>,
    muted: iced::Color,
) -> Element<'a, Message> {
    row(cols.iter().map(|col| cell(col, row_data, muted))).spacing(8).into()
}

fn cell<'a, Message: 'a>(
    col: &model::table::Column,
    row_data: &std::collections::BTreeMap<String, serde_json::Value>,
    muted: iced::Color,
) -> Element<'a, Message> {
    let raw = row_data.get(&col.key).and_then(|v| v.as_str()).unwrap_or("");

    match col.kind {
        model::table::Kind::Tag => container(text(raw.to_string()).size(10).font(Font {
            weight: font::Weight::Bold,
            ..Font::DEFAULT
        }))
        .padding([2, 6])
        .style(style::tag)
        .width(60)
        .into(),

        model::table::Kind::Timestamp => text(theme::humanize(raw)).size(11).color(muted).into(),

        _ => text(raw.to_string()).size(12).width(Fill).into(),
    }
}

mod style {
    use iced::Theme;
    use iced::widget::container;

    pub fn tag(theme: &Theme) -> container::Style {
        let palette = theme.palette();
        container::Style {
            background: Some(palette.background.weak.color.into()),
            border: iced::Border {
                radius: 3.0.into(),
                ..Default::default()
            },
            ..Default::default()
        }
    }
}
