use hyozu::axis::Placement;
use hyozu::data::Action;
use hyozu::{Data, Map, bar, bars, chart, item, props};
use iced::widget::{center, column, pick_list, radio, row};
use iced::{Center, Fill, Function, Subscription, Task, Theme, keyboard};
use iced_widget::checkbox;

pub fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .window_size([800.0, 600.0])
        .settings(iced::Settings {
            default_text_size: 13.into(),
            default_font: iced::Font::with_name("GT Pressura Mono"),
            ..Default::default()
        })
        .title("Hyozu - Bar Chart with Labels")
        .theme(App::theme)
        .subscription(App::subscription)
        .run()
}

struct App {
    data: Data,
    theme: Theme,
    all_themes: Vec<Theme>,
}

#[derive(Debug, Clone)]
enum Message {
    Set(item::Item),
    ThemeChanged(Theme),
    NextTheme,
    PreviousTheme,
    FirstTheme,
    LastTheme,
    LoadData,
}

// Helper function to format numbers with thousands separator
fn currency(value: f64) -> String {
    let whole = value as i32;
    let s = whole.to_string();
    let mut result = String::new();
    let chars: Vec<_> = s.chars().collect();

    for (i, ch) in chars.iter().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.insert(0, ',');
        }
        result.insert(0, *ch);
    }

    format!("${}", result)
}

impl App {
    fn new() -> (Self, Task<Message>) {
        (
            Self {
                data: Default::default(),
                theme: hyozu::theme::paper(),
                all_themes: hyozu::theme::all_themes().collect(),
            },
            Task::done(Message::LoadData),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::LoadData => {
                let bars = bars!([1500, 1600, 1800, 1900, 2100, 2000], [
                    800, 1200, 1000, 1400, 900, 1100
                ],)
                .data_labels(bar::label::Position::End + currency)
                .stacked();

                let data = Data::from(bars)
                    .title("Monthly Sales with Labels")
                    .x_axis_labels(
                        Placement::OnTicks
                            + ["Jan", "Feb", "Mar", "Apr", "May", "Jun"],
                    );

                self.data = data;
            }
            Message::Set(item) => {
                self.data.perform(Action::Set(item));
            }
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
                Key::Named(Named::PageDown)
                | Key::Named(Named::ArrowRight)
                | Key::Named(Named::ArrowDown) => Some(Message::NextTheme),
                Key::Named(Named::PageUp)
                | Key::Named(Named::ArrowLeft)
                | Key::Named(Named::ArrowUp) => Some(Message::PreviousTheme),
                Key::Named(Named::Home) => Some(Message::FirstTheme),
                Key::Named(Named::End) => Some(Message::LastTheme),
                _ => None,
            }
        })
    }

    fn view(&self) -> iced::Element<'_, Message> {
        // Read current values from data using ? chains
        let position =
            (|| Some(self.data.bars(0)?.series(0)?.label()?.position()))()
                .unwrap_or(bar::label::Position::Above);

        let x_axis_placement = (|| self.data.x_axis_ref()?.placement())()
            .unwrap_or(Placement::OnTicks);

        let stacked = self
            .data
            .bars(0)
            .map(|b| b.layout() == bar::Layout::Stacked)
            .unwrap_or(false);

        // Property change handlers
        let on_position = |p| {
            props::bar::label::Position(p)
                .map(props::bar::Label)
                .map(item::Bars.with(0))
                .map(Message::Set)
        };

        let on_placement =
            |p| props::axis::Placement(p).map(item::XAxis).map(Message::Set);

        let on_layout = |stacked: bool| {
            props::bar::Layout(if stacked {
                bar::Layout::Stacked
            } else {
                bar::Layout::Grouped
            })
            .map(item::Bars.with(0))
            .map(Message::Set)
        };

        let position_controls = column![
            row![
                "X-Axis Labels:",
                radio(
                    "On Ticks",
                    Placement::OnTicks,
                    Some(x_axis_placement),
                    on_placement
                ),
                radio(
                    "Between Ticks",
                    Placement::BetweenTicks,
                    Some(x_axis_placement),
                    on_placement
                ),
            ]
            .align_y(Center)
            .spacing(10),
            row![
                "Bar Labels:",
                radio(
                    "Above",
                    bar::label::Position::Above,
                    Some(position),
                    on_position
                ),
                radio(
                    "End",
                    bar::label::Position::End,
                    Some(position),
                    on_position
                ),
                radio(
                    "Center",
                    bar::label::Position::Center,
                    Some(position),
                    on_position
                ),
                radio(
                    "Base",
                    bar::label::Position::Base,
                    Some(position),
                    on_position
                ),
            ]
            .align_y(Center)
            .spacing(10)
        ]
        .spacing(10);

        let layout_controls =
            row![checkbox(stacked).label("Stacked").on_toggle(on_layout)]
                .align_y(Center)
                .spacing(10);

        let theme_picker = row![
            "Theme:",
            pick_list(
                Some(self.theme.clone()),
                self.all_themes.clone(),
                |t: &Theme| t.to_string(),
            )
            .on_select(Message::ThemeChanged)
            .width(Fill)
            .placeholder("Paper (default)"),
        ]
        .align_y(Center)
        .spacing(10);

        let controls = row![position_controls, layout_controls, theme_picker]
            .spacing(40)
            .align_y(Center);

        center(
            column![
                controls,
                chart(&self.data).design(&self.theme).padding(20)
            ]
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
