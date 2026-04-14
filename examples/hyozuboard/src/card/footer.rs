use iced::widget::{container, row, space, text, tooltip};
use iced::{Center, Element, Theme};

use crate::{model, theme};

pub fn view<'a, Message: 'a>(card: &'a model::card::Card, theme: &'a Theme) -> Element<'a, Message> {
    let muted = theme::muted(theme);

    row(pills(card, muted)
        .chain(std::iter::once(space::horizontal().into()))
        .chain(meta(card, muted))
        .chain(source_icon(card, muted)))
    .spacing(4)
    .align_y(Center)
    .into()
}

fn pills<'a, Message: 'a>(
    card: &'a model::card::Card,
    muted: iced::Color,
) -> impl Iterator<Item = Element<'a, Message>> {
    card.context.values().map(move |value| {
        container(text!("{value}").size(10).color(muted))
            .padding([2, 6])
            .style(style::pill)
            .into()
    })
}

fn meta<'a, Message: 'a>(
    card: &'a model::card::Card,
    muted: iced::Color,
) -> impl Iterator<Item = Element<'a, Message>> {
    let author_name = card
        .footer
        .author
        .display_name
        .as_deref()
        .unwrap_or(&card.footer.author.user_id);

    let time = theme::humanize(&card.footer.updated);

    [
        text(author_name).size(9).color(muted).into(),
        text(time).size(9).color(muted).into(),
    ]
    .into_iter()
}

fn source_icon<'a, Message: 'a>(card: &'a model::card::Card, muted: iced::Color) -> Option<Element<'a, Message>> {
    let source = card.footer.data_source.as_ref()?;

    let initial = source.system.chars().next().unwrap_or('?').to_uppercase().to_string();

    let icon = container(text(initial).size(8).color(muted).center())
        .width(16)
        .height(16)
        .center(16)
        .style(style::source_icon);

    let tip = match &source.agent {
        Some(agent) => format!("{} — {}", source.system, agent),
        None => source.system.clone(),
    };

    Some(tooltip(icon, text(tip).size(10), tooltip::Position::Top).gap(4).into())
}

mod style {
    use iced::Theme;
    use iced::widget::container;

    pub fn pill(theme: &Theme) -> container::Style {
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

    pub fn source_icon(theme: &Theme) -> container::Style {
        let palette = theme.palette();
        container::Style {
            background: Some(palette.background.neutral.color.into()),
            border: iced::Border {
                radius: 8.0.into(),
                ..Default::default()
            },
            ..Default::default()
        }
    }
}
