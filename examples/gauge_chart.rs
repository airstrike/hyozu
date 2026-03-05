use hyozu::axis::Ticks;
use hyozu::{data, gauge};
use iced::widget::{center, column, pick_list, row};
use iced::{Center, Fill, Subscription, Task, Theme, keyboard};

pub fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .window_size([600.0, 550.0])
        .title("hyozu • gauge chart")
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
    fn new() -> (Self, Task<Message>) {
        let g = gauge(72.0)
            .range(0.0, 100.0)
            .zone(0.0, 40.0, 0x4CAF50)
            .zone(40.0, 70.0, 0xFFC107)
            .zone(70.0, 100.0, 0xF44336)
            .format(|v| format!("{:.0}%", v))
            .unit("efficiency")
            .ticks(Ticks::default())
            .gradient(true);

        (
            Self {
                data: data(g).title("System Performance"),
                theme: hyozu::theme::paper(),
                all_themes: hyozu::theme::all_themes().collect(),
            },
            Task::none(),
        )
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
