use hyozu::data::Mark;
use hyozu::{data, xy};
use iced::widget::{center, column, pick_list, row};
use iced::{Center, Fill, Subscription, Task, Theme, keyboard};

pub fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .window_size([800.0, 600.0])
        .title("hyozu • xy scatter chart")
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
        // Iris dataset — Petal Length (x) vs Petal Width (y)
        let setosa = xy([
            (1.4, 0.2),
            (1.4, 0.2),
            (1.3, 0.2),
            (1.5, 0.2),
            (1.4, 0.2),
            (1.7, 0.4),
            (1.4, 0.3),
            (1.5, 0.2),
            (1.4, 0.2),
            (1.5, 0.1),
            (1.5, 0.2),
            (1.6, 0.2),
            (1.4, 0.1),
            (1.1, 0.1),
            (1.2, 0.2),
            (1.5, 0.4),
            (1.3, 0.4),
            (1.4, 0.3),
            (1.7, 0.3),
            (1.5, 0.3),
            (1.7, 0.2),
            (1.5, 0.4),
            (1.0, 0.2),
            (1.7, 0.5),
            (1.9, 0.2),
            (1.6, 0.2),
            (1.6, 0.4),
            (1.5, 0.2),
            (1.4, 0.2),
            (1.6, 0.2),
        ])
        .with_name("Setosa");

        let versicolor = xy([
            (4.7, 1.4),
            (4.5, 1.5),
            (4.9, 1.5),
            (4.0, 1.3),
            (4.6, 1.5),
            (4.5, 1.3),
            (4.7, 1.6),
            (3.3, 1.0),
            (4.6, 1.3),
            (3.9, 1.4),
            (3.5, 1.0),
            (4.2, 1.5),
            (4.0, 1.0),
            (4.7, 1.4),
            (3.6, 1.3),
            (4.4, 1.4),
            (4.5, 1.5),
            (4.1, 1.0),
            (4.5, 1.5),
            (3.9, 1.1),
            (4.8, 1.8),
            (4.0, 1.3),
            (4.9, 1.5),
            (4.7, 1.2),
            (4.3, 1.3),
            (4.4, 1.2),
            (4.8, 1.4),
            (5.0, 1.7),
            (4.5, 1.3),
            (3.5, 1.0),
        ])
        .with_name("Versicolor");

        let virginica = xy([
            (6.0, 2.5),
            (5.1, 1.9),
            (5.9, 2.1),
            (5.6, 1.8),
            (5.8, 2.2),
            (6.6, 2.1),
            (4.5, 1.7),
            (6.3, 1.8),
            (5.8, 1.8),
            (6.1, 2.5),
            (5.1, 2.0),
            (5.3, 1.9),
            (5.5, 2.1),
            (5.0, 2.0),
            (5.1, 2.4),
            (5.3, 2.3),
            (5.5, 1.8),
            (6.7, 2.2),
            (6.9, 2.3),
            (5.0, 1.5),
            (5.7, 2.3),
            (4.9, 2.0),
            (6.7, 2.0),
            (4.9, 1.8),
            (5.7, 2.1),
            (6.0, 1.8),
            (4.8, 1.8),
            (4.9, 1.8),
            (5.6, 2.1),
            (5.8, 1.6),
        ])
        .with_name("Virginica");

        let marks: Vec<Mark> = vec![Mark::from(setosa), Mark::from(versicolor), Mark::from(virginica)];

        (
            Self {
                data: data(marks)
                    .title("Iris Dataset (Petal)")
                    .x_axis_labels(|v| format!("{v:.1} cm"))
                    .y_axis_labels(|v| format!("{v:.1} cm")),
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
