mod action;
mod category;
mod charts;
mod detail;
mod icon;

use category::Category;
use charts::{ChartId, Variant};

use hyozu::Data;
use iced::widget::{button, column, container, row, scrollable, text};
use iced::{Element, Fill, Font, Shrink, Task, Theme};

pub fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .window_size([1200.0, 800.0])
        .title("hyozu gallery")
        .settings(iced::Settings {
            default_text_size: 13.into(),
            default_font: Font::MONOSPACE,
            ..Default::default()
        })
        .font(icon::FONT)
        .theme(|state: &App| state.theme.clone())
        .run()
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

enum Screen {
    Gallery,
    Detail(detail::Detail),
}

struct App {
    theme: Theme,
    dark: bool,
    filter: Option<Category>,
    variants: Vec<Variant>,
    data: Vec<Data>,
    screen: Screen,
    active_chart: Option<ChartId>,
}

#[derive(Debug, Clone)]
enum Message {
    ToggleDark,
    FilterCategory(Option<Category>),
    SelectChart(ChartId),
    Detail(detail::Message),
}

// ---------------------------------------------------------------------------
// Logic
// ---------------------------------------------------------------------------

impl App {
    fn new() -> (Self, Task<Message>) {
        let variants = charts::variants();
        let data: Vec<Data> = variants.iter().map(|v| (v.build)()).collect();

        (
            Self {
                theme: hyozu::theme::paper(),
                dark: false,
                filter: None,
                variants,
                data,
                screen: Screen::Gallery,
                active_chart: None,
            },
            Task::none(),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ToggleDark => {
                self.dark = !self.dark;
                self.theme = if self.dark {
                    hyozu::theme::paper_dark()
                } else {
                    hyozu::theme::paper()
                };
            }
            Message::FilterCategory(cat) => {
                self.filter = cat;
            }
            Message::SelectChart(id) => {
                let variant = &self.variants[id.0];
                self.active_chart = Some(id);
                self.screen = Screen::Detail(detail::Detail::new(variant.source));
            }
            Message::Detail(msg) => {
                if let Screen::Detail(ref mut detail) = self.screen {
                    let action = detail.update(msg);
                    if let Some(detail::Instruction::Back) = action.instruction {
                        self.screen = Screen::Gallery;
                        self.active_chart = None;
                    }
                    return action.task.map(Message::Detail);
                }
            }
        }
        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        let sidebar = self.view_sidebar();

        let content: Element<'_, Message> = match &self.screen {
            Screen::Gallery => self.view_grid(),
            Screen::Detail(detail) => {
                let id = self.active_chart.unwrap();
                let variant = &self.variants[id.0];
                let data = &self.data[id.0];
                detail.view(variant, data, self.dark).map(Message::Detail)
            }
        };

        row![sidebar, content].into()
    }

    // -- Sidebar -----------------------------------------------------------

    fn view_sidebar(&self) -> Element<'_, Message> {
        let heading = text("hyozu").size(20);

        // "All" filter button
        let all_btn = sidebar_button("All", self.filter.is_none(), Message::FilterCategory(None));

        let mut sidebar_col = column![heading, all_btn].spacing(6);

        for &cat in Category::ALL {
            sidebar_col = sidebar_col.push(sidebar_button(
                cat.label(),
                self.filter == Some(cat),
                Message::FilterCategory(Some(cat)),
            ));
        }

        // Sun/moon toggle at bottom
        let theme_icon = if self.dark { icon::sun() } else { icon::moon() };

        let theme_btn = button(theme_icon.size(16))
            .padding([4, 8])
            .on_press(Message::ToggleDark)
            .style(button::text);

        let spacer = container(column![]).height(Fill);

        let sidebar_content = column![sidebar_col, spacer, theme_btn]
            .spacing(6)
            .padding(12)
            .height(Fill);

        container(sidebar_content)
            .width(180)
            .height(Fill)
            .style(|theme: &Theme| {
                let palette = theme.palette();
                container::Style {
                    background: Some(palette.background.weak.color.into()),
                    ..Default::default()
                }
            })
            .into()
    }

    // -- Chart grid --------------------------------------------------------

    fn view_grid(&self) -> Element<'_, Message> {
        let visible: Vec<&Variant> = self
            .variants
            .iter()
            .filter(|v| self.filter.is_none() || self.filter == Some(v.category))
            .collect();

        let mut grid_col = column![].spacing(16).padding(16);

        for pair in visible.chunks(2) {
            let mut chart_row = row![].spacing(16);

            for variant in pair {
                chart_row = chart_row.push(self.view_card(variant));
            }

            if pair.len() == 1 {
                chart_row = chart_row.push(container(column![]).width(Fill).height(Shrink));
            }

            grid_col = grid_col.push(chart_row);
        }

        scrollable(grid_col).width(Fill).height(Fill).into()
    }

    // -- Single chart card -------------------------------------------------

    fn view_card<'a>(&'a self, variant: &'a Variant) -> Element<'a, Message> {
        let id = variant.id;

        let chart_widget = hyozu::chart(&self.data[id.0]).height(220).padding(10);

        let label = text(variant.name).size(12);

        let card = button(column![chart_widget, label].spacing(4).width(Fill))
            .padding(8)
            .on_press(Message::SelectChart(id))
            .style(card_button_style);

        container(card).width(Fill).into()
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn sidebar_button(label: &str, active: bool, msg: Message) -> Element<'_, Message> {
    let btn = button(text(label).size(12))
        .padding([4, 8])
        .width(Fill)
        .on_press(msg)
        .style(move |theme, status| sidebar_button_style(theme, status, active));

    btn.into()
}

fn sidebar_button_style(theme: &Theme, status: button::Status, active: bool) -> button::Style {
    let palette = theme.palette();
    let muted = palette.background.base.text.scale_alpha(0.6);
    let bright = palette.background.base.text;
    let fill = palette.background.strong.color.scale_alpha(0.4);

    button::Style {
        background: if active {
            Some(fill.into())
        } else if matches!(status, button::Status::Hovered) {
            Some(fill.scale_alpha(0.5).into())
        } else {
            None
        },
        text_color: if active { bright } else { muted },
        border: iced::Border::default(),
        ..Default::default()
    }
}

fn card_button_style(theme: &Theme, status: button::Status) -> button::Style {
    let palette = theme.palette();
    let bg = palette.background.weak.color.scale_alpha(0.3);

    button::Style {
        background: match status {
            button::Status::Hovered => Some(palette.background.weak.color.scale_alpha(0.5).into()),
            _ => Some(bg.into()),
        },
        border: iced::Border {
            width: 1.0,
            radius: 6.0.into(),
            color: palette.background.strong.color.scale_alpha(0.2),
        },
        text_color: palette.background.base.text,
        ..Default::default()
    }
}
