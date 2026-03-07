use hyozu::mark::pie;
use hyozu::{Palette, data, pie as pie_fn};
use iced::widget::{center, column, pick_list, row};
use iced::{Center, Fill, Subscription, Task, Theme, keyboard};

pub fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .window_size([700.0, 600.0])
        .title("hyozu • donut chart")
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
        let chart = pie_fn([
            pie::slice(42).name("Rent"),
            pie::slice(25).name("Food"),
            pie::slice(15).name("Transport"),
            pie::slice(10).name("Utilities"),
            pie::slice(8).name("Other"),
        ])
        .labels(pie::label::Label::percent())
        .hole(0.55)
        .gap(2.0);

        Self {
            data: data(chart).title("Monthly Expenses").palette(Palette::Sequential),
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
            pick_list(self.all_themes.clone(), Some(self.theme.clone()), Message::ThemeChanged)
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
