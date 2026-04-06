use iced::{Element, Theme, color};
use std::collections::HashMap;
use sweeten::widget::tile_grid::{self, Configuration};

mod card;
mod dashboard;
mod model;
mod placeholder;
mod theme;

fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .theme(app_theme())
        .window_size((1400.0, 850.0))
        .title("hyozuboard")
        .run()
}

struct App {
    dashboard: dashboard::State,
}

impl App {
    fn new() -> Self {
        let theme = app_theme();

        let json = include_str!("../card-examples.json");
        let cards: Vec<model::card::Card> = serde_json::from_str(json).expect("Failed to parse card-examples.json");

        let mut cards_by_id: HashMap<String, model::card::Card> =
            cards.into_iter().map(|c| (c.id.clone(), c)).collect();

        let tile = |id: &str, cards: &mut HashMap<String, model::card::Card>| -> dashboard::Tile {
            let card_data = cards.remove(id).unwrap_or_else(|| panic!("missing card: {id}"));
            let state = card::State::new(&card_data, &theme);
            dashboard::Tile::Card {
                model: card_data,
                state,
            }
        };

        let config: Configuration<dashboard::Tile> = Configuration::new(theme::COLS)
            .float(true)
            .with_item(0, 0, 3, 1, tile("card_revenue", &mut cards_by_id))
            .with_item(0, 1, 3, 1, tile("card_leverage", &mut cards_by_id))
            .with_item(3, 0, 6, 2, dashboard::Tile::Placeholder)
            .with_item(9, 0, 3, 1, tile("card_arr", &mut cards_by_id))
            .with_item(9, 1, 3, 1, tile("card_burn", &mut cards_by_id))
            .with_item(0, 2, 6, 2, tile("card_markets", &mut cards_by_id))
            .with_item(6, 2, 6, 2, tile("card_news", &mut cards_by_id))
            .with_item(6, 4, 6, 2, tile("card_geo_profit", &mut cards_by_id));

        let mut grid = tile_grid::State::with_configuration(config);

        for (_, card_data) in cards_by_id {
            let state = card::State::new(&card_data, &theme);
            grid.add_auto(3, 1, dashboard::Tile::Card {
                model: card_data,
                state,
            });
        }

        let sizes: Vec<_> = grid
            .iter()
            .filter_map(|(id, _)| grid.get_item(id).map(|item| (id, item.w, item.h)))
            .collect();

        for (id, w, h) in sizes {
            if let Some(dashboard::Tile::Card {
                model: card_model,
                state,
            }) = grid.get_mut(id)
            {
                let (name, _) = card_model.sizing.resolve(w, h);
                state.active_variant = *name;
            }
        }

        App {
            dashboard: dashboard::State {
                grid,
                focus: None,
                theme,
            },
        }
    }

    fn update(&mut self, message: dashboard::Message) {
        dashboard::update(&mut self.dashboard, message);
    }

    fn view(&self) -> Element<'_, dashboard::Message> {
        dashboard::view(&self.dashboard)
    }
}

fn app_theme() -> Theme {
    use iced::theme::palette::Seed;

    Theme::custom("hyozuboard", Seed {
        background: color!(0xf5f5f5),
        text: color!(0x333333),
        primary: color!(0x0078d4),
        success: color!(0x17a589),
        warning: color!(0xf39c12),
        danger: color!(0xe74c3c),
    })
}
