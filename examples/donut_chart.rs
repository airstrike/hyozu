use hyozu::mark::pie;
use hyozu::{Color, data, donut, legend, palette};
use iced::widget::{center, column, container, pick_list, row, space, text};
use iced::{Border, Center, Fill, Font, Subscription, Task, Theme, keyboard};

pub fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .window_size([900.0, 620.0])
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
        let chart = pie([
            pie::slice(86.2).name("Enterprise Subscription"),
            pie::slice(40.5).name("Mid-Market Subscription"),
            pie::slice(12.8).name("SMB Subscription"),
            pie::slice(42.1).name("Professional Services"),
            pie::slice(19.8).name("Support & Training"),
        ])
        .gap(2.0);

        Self {
            data: data(chart)
                .palette(palette::sequential(Color::Primary))
                .legend(legend::Config::right())
                .value_format(|v: &f64| format!("${v:.1}M")),
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
                if let Some(first) = self.all_themes.first() {
                    self.theme = first.clone();
                }
            }
            Message::LastTheme => {
                if let Some(last) = self.all_themes.last() {
                    self.theme = last.clone();
                }
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

        let total: f64 = self
            .data
            .pie(0)
            .map(|pie| pie.slices().iter().map(|s| s.value()).sum())
            .unwrap_or(0.0);
        let total_text = self.data.format_value(total);

        let header = row![
            column![
                text("Product Mix").size(18),
                text("FY 2026 revenue · by offering").size(13),
            ]
            .spacing(2),
            space::horizontal(),
            container(text(total_text.clone()).size(14))
                .padding([4, 10])
                .style(badge_style),
        ]
        .align_y(Center);

        let donut_widget = donut(&self.data)
            .hole(0.6)
            .design(&self.theme)
            .padding(8)
            .center(center(
                column![text(total_text).size(48), text("FY 2026").size(13)]
                    .spacing(2)
                    .align_x(Center),
            ))
            .hover(|entry| match entry {
                hyozu::hover::Entry::Pie {
                    label, value, total, ..
                } => {
                    let name = label.unwrap_or("Slice").to_string();
                    let share = if *total > 0.0 { 100.0 * value / total } else { 0.0 };
                    let semibold = Font {
                        weight: iced::font::Weight::Semibold,
                        ..Font::DEFAULT
                    };
                    hyozu::hover::Annotation::new(
                        column![
                            text(name).size(14).font(semibold),
                            text(format!("{share:.1}%  ·  ${value:.1}M")).size(12),
                        ]
                        .spacing(2),
                    )
                }
                _ => hyozu::hover::Annotation::new(text("")),
            });

        let card = container(column![header, donut_widget].spacing(16))
            .padding(24)
            .style(container::rounded_box);

        center(column![theme_picker, card].spacing(20).align_x(Center))
            .padding(20)
            .into()
    }

    fn theme(&self) -> Theme {
        self.theme.clone()
    }
}

fn badge_style(theme: &Theme) -> container::Style {
    let palette = theme.palette();
    container::Style {
        background: Some(palette.background.weakest.color.into()),
        text_color: Some(palette.background.weakest.text),
        border: Border {
            width: 1.0,
            color: palette.background.weak.color,
            radius: 6.0.into(),
        },
        ..container::Style::default()
    }
}
