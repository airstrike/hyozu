use iced::widget::{center, column, pick_list, row};
use iced::{Center, Fill, Subscription, Task, Theme, keyboard};

use hyozu::data;
use hyozu::waterfall::EntryKind::*;
use hyozu::waterfall::{self};

pub fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .window_size([900.0, 600.0])
        .title("hyozu • waterfall chart")
        .theme(App::theme)
        .subscription(App::subscription)
        .run()
}

struct App {
    data: hyozu::Data,
    theme: Theme,
    all_themes: Vec<Theme>,
}

#[derive(Debug, Clone)]
enum Message {
    ThemeChanged(Theme),
    NextTheme,
    PreviousTheme,
    FirstTheme,
    LastTheme,
}

impl App {
    fn new() -> Self {
        let wf = waterfall::waterfall([
            waterfall::entry(2733, Total).text("$2,733M"),
            waterfall::entry(50, Increase).text("+$50M"),
            waterfall::entry(-30, Decrease).text("-$30M"),
            waterfall::entry(45, Increase).text("+$45M"),
            waterfall::entry(-25, Decrease).text("-$25M"),
            waterfall::entry(40, Increase).text("+$40M"),
            waterfall::entry(-20, Decrease).text("-$20M"),
            waterfall::entry(35, Increase).text("+$35M"),
            waterfall::entry(-75, Decrease).text("-$75M"),
            waterfall::entry(2753, Total).text("$2,753M"),
        ]);

        Self {
            data: data(wf).title("Cash Flow FY 2025").x_axis_labels([
                "Opening", "Q1 Ops", "Q1 CapEx", "Q2 Ops", "Q2 CapEx", "Q3 Ops", "Q3 CapEx", "Q4 Ops", "Q4 CapEx",
                "Closing",
            ]),
            theme: hyozu::theme::paper(),
            all_themes: hyozu::theme::all_themes().collect(),
        }
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ThemeChanged(theme) => {
                self.theme = theme;
            }
            Message::FirstTheme => {
                self.theme = self.all_themes.first().unwrap().clone();
            }
            Message::LastTheme => {
                self.theme = self.all_themes.last().unwrap().clone();
            }
            Message::NextTheme => {
                if let Some(next) = self
                    .all_themes
                    .iter()
                    .cycle()
                    .skip_while(|theme| *theme != &self.theme)
                    .nth(1)
                {
                    self.theme = next.clone();
                }
            }
            Message::PreviousTheme => {
                if let Some(prev) = self
                    .all_themes
                    .iter()
                    .rev()
                    .cycle()
                    .skip_while(|theme| *theme != &self.theme)
                    .nth(1)
                {
                    self.theme = prev.clone();
                }
            }
        }
        Task::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        use keyboard::key::{Key, Named};

        keyboard::listen().filter_map(|event| {
            let keyboard::Event::KeyPressed { key, .. } = event else {
                return None;
            };
            match key {
                Key::Named(Named::PageDown) | Key::Named(Named::ArrowRight) | Key::Named(Named::ArrowDown) => {
                    Some(Message::NextTheme)
                }
                Key::Named(Named::PageUp) | Key::Named(Named::ArrowLeft) | Key::Named(Named::ArrowUp) => {
                    Some(Message::PreviousTheme)
                }
                Key::Named(Named::Home) => Some(Message::FirstTheme),
                Key::Named(Named::End) => Some(Message::LastTheme),
                _ => None,
            }
        })
    }

    fn view(&self) -> iced::Element<'_, Message> {
        let theme_picker = row![
            "Theme:",
            pick_list(Some(self.theme.clone()), self.all_themes.clone(), |t: &Theme| t
                .to_string(),)
            .on_select(Message::ThemeChanged)
            .width(Fill)
            .placeholder("Paper (default)"),
        ]
        .align_y(Center)
        .spacing(10);

        center(
            column![theme_picker, hyozu::chart(&self.data).design(&self.theme).padding(20)]
                .align_x(Center)
                .spacing(20),
        )
        .padding(20)
        .into()
    }

    fn theme(&self) -> Theme {
        self.theme.clone()
    }
}
