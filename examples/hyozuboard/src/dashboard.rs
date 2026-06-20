use iced::widget::{container, pick_list, row, scrollable, text, tooltip};
use iced::{Center, Element, Fill, Font, Length, font};

use sweeten::widget::tile_grid::{self, CellHeight, grid_content, title_bar};

use crate::{card, model, placeholder, theme};

pub enum Tile {
    Card {
        model: model::card::Card,
        state: card::State,
    },
    Placeholder,
}

pub struct State {
    pub grid: tile_grid::State<Tile>,
    pub focus: Option<tile_grid::ItemId>,
    pub theme: iced::Theme,
}

#[derive(Debug, Clone)]
pub enum Message {
    GridAction(tile_grid::Action),
    SelectorChanged(tile_grid::ItemId, String, String),
}

pub fn update(state: &mut State, message: Message) {
    match message {
        Message::GridAction(action) => {
            if action.is_click() {
                state.focus = Some(action.id());
            }

            if let tile_grid::Action::Resize {
                id,
                w,
                h,
                phase: tile_grid::DragPhase::Ended,
            } = &action
            {
                if let Some(Tile::Card {
                    model: card,
                    state: card_state,
                }) = state.grid.get_mut(*id)
                {
                    let (name, _) = card.sizing.resolve(*w, *h);
                    card_state.active_variant = *name;
                }
            }

            state.grid.perform(action, |_, tile| matches!(tile, Tile::Placeholder));
        }

        Message::SelectorChanged(id, key, value) => {
            if let Some(Tile::Card { model: m, state: s }) = state.grid.get_mut(id) {
                s.selections.insert(key, value);
                s.rebuild_chart(m, &state.theme);
            }
        }
    }
}

pub fn view(state: &State) -> Element<'_, Message> {
    let theme = &state.theme;

    let grid = sweeten::tile_grid(&state.grid, |id, tile| match tile {
        Tile::Placeholder => grid_content(placeholder::view())
            .draggable(false)
            .resizable(false)
            .held(true)
            .style(style::placeholder),

        Tile::Card {
            model: card_model,
            state: card_state,
        } => {
            let is_focused = state.focus == Some(id);

            grid_content(card::view(card_model, card_state, theme))
                .title_bar(
                    title_bar(label(card_model, theme))
                        .controls(selectors(id, card_model, card_state))
                        .padding(10)
                        .always_show_controls(),
                )
                .style(if is_focused { style::panel_focused } else { style::panel })
        }
    })
    .width(Fill)
    .spacing(theme::GRID_SPACING)
    .cell_height(CellHeight::Fixed(theme::CELL_H))
    .on_action(Message::GridAction);

    container(scrollable(grid).width(Fill).spacing(5))
        .padding(8)
        .width(Fill)
        .height(Fill)
        .style(style::page)
        .into()
}

fn label<'a>(card: &'a model::card::Card, theme: &'a iced::Theme) -> iced::widget::Row<'a, Message> {
    let muted = theme::muted(theme);

    let title = text(card.header.label.to_uppercase())
        .size(11)
        .font(Font {
            weight: font::Weight::Bold,
            ..Font::DEFAULT
        })
        .color(muted);

    let info = card.meta.as_ref().and_then(|m| m.description.as_deref()).map(|desc| {
        tooltip(
            container(text("i").size(8).color(muted).center())
                .width(16)
                .height(16)
                .center(16)
                .style(style::info_icon),
            container(text(desc).size(11))
                .width(Length::Fit.max(260))
                .padding(8)
                .style(style::tooltip),
            tooltip::Position::Bottom,
        )
        .gap(4)
        .into()
    });

    row(std::iter::once(title.into()).chain(info))
        .spacing(6)
        .align_y(Center)
}

fn selectors<'a>(
    tile_id: tile_grid::ItemId,
    card: &'a model::card::Card,
    state: &'a card::State,
) -> iced::widget::Row<'a, Message> {
    row(card.selectors.iter().map(|(key, selector)| {
        let selected: Option<String> = state.selections.get(key).cloned();
        let key = key.clone();

        pick_list(selected, selector.options.as_slice(), String::to_string)
            .on_select(move |val| Message::SelectorChanged(tile_id, key.clone(), val))
            .text_size(10)
            .padding([3, 8])
            .into()
    }))
    .spacing(6)
    .align_y(Center)
}

mod style {
    use iced::Theme;
    use iced::widget::container;

    pub fn panel(theme: &Theme) -> container::Style {
        let palette = theme.palette();
        container::Style {
            background: Some(palette.background.base.color.into()),
            border: iced::Border {
                width: 1.0,
                color: palette.background.strong.color,
                radius: 6.0.into(),
            },
            ..Default::default()
        }
    }

    pub fn panel_focused(theme: &Theme) -> container::Style {
        let palette = theme.palette();
        container::Style {
            background: Some(palette.background.base.color.into()),
            border: iced::Border {
                width: 2.0,
                color: palette.primary.base.color,
                radius: 6.0.into(),
            },
            ..Default::default()
        }
    }

    pub fn placeholder(_theme: &Theme) -> container::Style {
        container::Style::default()
    }

    pub fn tooltip(theme: &Theme) -> container::Style {
        let palette = theme.palette();
        container::Style {
            background: Some(palette.background.base.color.into()),
            border: iced::Border {
                width: 1.0,
                color: palette.background.strong.color,
                radius: 4.0.into(),
            },
            ..Default::default()
        }
    }

    pub fn info_icon(theme: &Theme) -> container::Style {
        let palette = theme.palette();
        container::Style {
            border: iced::Border {
                width: 1.0,
                color: palette.background.strong.color,
                radius: 99.0.into(),
            },
            ..Default::default()
        }
    }

    pub fn page(theme: &Theme) -> container::Style {
        let palette = theme.palette();
        container::Style {
            background: Some(palette.background.weak.color.into()),
            ..Default::default()
        }
    }
}
