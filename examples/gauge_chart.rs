use hyozu::{Action, Function, Map, data, gauge, item, props};
use iced::widget::{center, column, pick_list, row};
use iced::{Center, Fill, Subscription, Task, Theme, keyboard, window};
use std::time::Instant;

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
    value: f64,
    direction: f64,
    last_frame: Instant,
    theme: Theme,
    all_themes: Vec<Theme>,
}

#[derive(Debug, Clone)]
enum Message {
    Tick(Instant),
    ThemeChanged(Theme),
    NextTheme,
    PreviousTheme,
    FirstTheme,
    LastTheme,
}

fn build_gauge(_theme: &Theme) -> hyozu::Data {
    data(
        gauge(0.0)
            .range(0.0, 130.0)
            .zones([
                (50.0, 0x4CAF50),  // green
                (90.0, 0xFFC107),  // yellow
                (100.0, 0xFF9800), // amber
                (130.0, 0xF44336), // red
            ])
            .format(|v| format!("{:.0}%", (v / 130.0 * 100.0)))
            .subtitle("Budget: 97% · Prior Year: 96%")
            .sweep(180.0)
            .ticks([0.0, 50.0, 90.0, 100.0, 130.0])
            .dim_by(0.0)
            .needle(true),
    )
    .title("System Performance")
}

impl App {
    fn new() -> (Self, Task<Message>) {
        let theme = hyozu::theme::paper();
        (
            Self {
                data: build_gauge(&theme),
                value: 0.0,
                direction: 1.0,
                last_frame: Instant::now(),
                theme,
                all_themes: hyozu::theme::all_themes().collect(),
            },
            Task::none(),
        )
    }

    fn set_theme(&mut self, theme: Theme) {
        self.theme = theme;
        self.data = build_gauge(&self.theme);
        // Re-apply current animated value
        let item = props::gauge::Value(self.value).map(item::Gauge.with(0));
        self.data.perform(Action::Set(item));
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Tick(now) => {
                let dt = (now - self.last_frame).as_secs_f64();
                self.last_frame = now;

                self.value += self.direction * 40.0 * dt;
                if self.value >= 130.0 {
                    self.value = 130.0;
                    self.direction = -1.0;
                } else if self.value <= 0.0 {
                    self.value = 0.0;
                    self.direction = 1.0;
                }

                let item = props::gauge::Value(self.value).map(item::Gauge.with(0));
                self.data.perform(Action::Set(item));
            }
            Message::ThemeChanged(theme) => self.set_theme(theme),
            Message::FirstTheme => {
                let t = self.all_themes.first().unwrap().clone();
                self.set_theme(t);
            }
            Message::LastTheme => {
                let t = self.all_themes.last().unwrap().clone();
                self.set_theme(t);
            }
            Message::NextTheme => {
                if let Some(next) = self
                    .all_themes
                    .iter()
                    .cycle()
                    .skip_while(|theme| *theme != &self.theme)
                    .nth(1)
                {
                    let t = next.clone();
                    self.set_theme(t);
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
                    let t = prev.clone();
                    self.set_theme(t);
                }
            }
        }
        Task::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        use keyboard::key::{Key, Named};

        Subscription::batch([
            window::frames().map(Message::Tick),
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
            }),
        ])
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
