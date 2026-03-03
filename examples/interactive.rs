use hyozu::axis::Placement;
use hyozu::data::Action;
use hyozu::target::Target;
use hyozu::{Data, Map, Palette, bar, bars, chart, item, props};
use iced::widget::{
    center, column, container, pick_list, radio, row, slider, text,
};
use iced::{Center, Fill, Function, Subscription, Task, Theme, keyboard};

pub fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .window_size([1000.0, 650.0])
        .settings(iced::Settings {
            default_text_size: 13.into(),
            default_font: iced::Font::with_name("GT Pressura Mono"),
            ..Default::default()
        })
        .title("Hyozu - Interactive Chart")
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
    Action(Action),
    ThemeChanged(Theme),
    NextTheme,
    PreviousTheme,
    FirstTheme,
    LastTheme,
}

fn currency(value: f64) -> String {
    let whole = value as i64;
    let s = whole.abs().to_string();
    let mut result = String::new();
    let chars: Vec<_> = s.chars().collect();

    for (i, ch) in chars.iter().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.insert(0, ',');
        }
        result.insert(0, *ch);
    }

    if whole < 0 {
        format!("-${result}")
    } else {
        format!("${result}")
    }
}

impl App {
    fn new() -> (Self, Task<Message>) {
        let bars = bars!([1500, 1600, 1800, 1900, 2100, 2000], [
            800, 1200, 1000, 1400, 900, 1100
        ],)
        .data_labels(bar::label::Position::End + currency);

        let data = Data::from(bars)
            .title("Monthly Revenue by Channel")
            .x_axis_labels(
                Placement::OnTicks + ["Jan", "Feb", "Mar", "Apr", "May", "Jun"],
            )
            .y_axis_labels(|v: f64| currency(v));

        (
            Self {
                data,
                theme: hyozu::theme::paper(),
                all_themes: hyozu::theme::all_themes().collect(),
            },
            Task::none(),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Set(item) => {
                self.data.perform(Action::Set(item));
            }
            Message::Action(action) => match &action {
                Action::Clicked(target) => {
                    // Toggle selection: click same or empty → deselect
                    let should_deselect = *target == Target::Mark(usize::MAX)
                        || self.data.selection() == Some(target);

                    if should_deselect {
                        self.data.deselect();
                    } else {
                        self.data.select(target.clone());
                    }
                }
                Action::Set(item) => {
                    self.data.perform(Action::Set(item.clone()));
                }
            },
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
                Key::Named(Named::ArrowRight)
                | Key::Named(Named::ArrowDown) => Some(Message::NextTheme),
                Key::Named(Named::ArrowLeft) | Key::Named(Named::ArrowUp) => {
                    Some(Message::PreviousTheme)
                }
                Key::Named(Named::Home) => Some(Message::FirstTheme),
                Key::Named(Named::End) => Some(Message::LastTheme),
                _ => None,
            }
        })
    }

    fn view(&self) -> iced::Element<'_, Message> {
        // Read current property values from data
        let size = self.data.bars(0).map(|b| b.size()).unwrap_or(0.75);
        let spacing = self.data.bars(0).map(|b| b.spacing()).unwrap_or(0.0);

        let layout = self
            .data
            .bars(0)
            .map(|b| b.layout())
            .unwrap_or(bar::Layout::Grouped);

        let label_position =
            (|| Some(self.data.bars(0)?.series(0)?.label()?.position()))()
                .unwrap_or(bar::label::Position::End);

        let x_placement = (|| self.data.x_axis_ref()?.placement())()
            .unwrap_or(Placement::OnTicks);

        // Selection display
        let selection_text = match self.data.selection() {
            None => "None".to_string(),
            Some(Target::Entry {
                mark,
                series,
                index,
            }) => format!("Bar [{mark}][{series}][{index}]"),
            Some(Target::Series { mark, series }) => {
                format!("Series [{mark}][{series}]")
            }
            Some(Target::Mark(m)) => format!("Mark [{m}]"),
            Some(other) => format!("{other:?}"),
        };

        // --- Property change handlers ---

        let on_size = |v| {
            props::bar::Size(v)
                .map(item::Bars.with(0))
                .map(Message::Set)
        };

        let on_spacing = |v| {
            props::bar::Spacing(v)
                .map(item::Bars.with(0))
                .map(Message::Set)
        };

        let on_layout = |l| {
            props::bar::Layout(l)
                .map(item::Bars.with(0))
                .map(Message::Set)
        };

        let on_label_pos = |p| {
            props::bar::label::Position(p)
                .map(props::bar::Label)
                .map(item::Bars.with(0))
                .map(Message::Set)
        };

        let on_x_placement =
            |p| props::axis::Placement(p).map(item::XAxis).map(Message::Set);

        let on_palette = |p: String| {
            let palette = match p.as_str() {
                "Categorical" => Palette::Categorical,
                "Sequential" => Palette::Sequential,
                _ => return Message::Set(item::Palette(Palette::Categorical)),
            };
            Message::Set(item::Palette(palette))
        };

        let current_palette = self
            .data
            .primary()
            .marks()
            .first()
            .map(|_| {
                // Infer from data's palette or the default
                "Auto"
            })
            .unwrap_or("Auto");

        // --- Sidebar controls ---

        let selection_section =
            column![text("Selection").size(14), text(selection_text).size(12),]
                .spacing(4);

        let size_section = column![
            text("Bar Size").size(14),
            row![
                slider(0.1..=1.0, size, on_size).step(0.05).width(Fill),
                text(format!("{size:.2}")),
            ]
            .spacing(10)
            .align_y(Center),
        ]
        .spacing(4);

        let spacing_section = column![
            text("Bar Spacing").size(14),
            row![
                slider(0.0..=1.0, spacing, on_spacing)
                    .step(0.05)
                    .width(Fill),
                text(format!("{spacing:.2}")),
            ]
            .spacing(10)
            .align_y(Center),
        ]
        .spacing(4);

        let layout_section = column![
            text("Layout").size(14),
            radio("Grouped", bar::Layout::Grouped, Some(layout), on_layout),
            radio("Stacked", bar::Layout::Stacked, Some(layout), on_layout),
            radio("Overlaid", bar::Layout::Overlaid, Some(layout), on_layout),
        ]
        .spacing(4);

        let label_section = column![
            text("Label Position").size(14),
            radio(
                "Above",
                bar::label::Position::Above,
                Some(label_position),
                on_label_pos,
            ),
            radio(
                "End",
                bar::label::Position::End,
                Some(label_position),
                on_label_pos,
            ),
            radio(
                "Center",
                bar::label::Position::Center,
                Some(label_position),
                on_label_pos,
            ),
            radio(
                "Base",
                bar::label::Position::Base,
                Some(label_position),
                on_label_pos,
            ),
        ]
        .spacing(4);

        let axis_section = column![
            text("X-Axis Labels").size(14),
            radio(
                "On Ticks",
                Placement::OnTicks,
                Some(x_placement),
                on_x_placement,
            ),
            radio(
                "Between Ticks",
                Placement::BetweenTicks,
                Some(x_placement),
                on_x_placement,
            ),
        ]
        .spacing(4);

        let palette_section = column![
            text("Palette").size(14),
            pick_list(
                Some(current_palette.to_string()),
                vec![
                    "Auto".to_string(),
                    "Categorical".to_string(),
                    "Sequential".to_string(),
                ],
                |s: &String| s.clone(),
            )
            .on_select(on_palette)
            .width(Fill),
        ]
        .spacing(4);

        let theme_section = column![
            text("Theme").size(14),
            pick_list(
                Some(self.theme.clone()),
                self.all_themes.clone(),
                |t: &Theme| t.to_string(),
            )
            .on_select(Message::ThemeChanged)
            .width(Fill),
        ]
        .spacing(4);

        let sidebar = container(
            column![
                selection_section,
                size_section,
                spacing_section,
                layout_section,
                label_section,
                axis_section,
                palette_section,
                theme_section,
            ]
            .spacing(12)
            .width(200),
        )
        .padding(15);

        let chart_area = chart(&self.data)
            .design(&self.theme)
            .on_action(Message::Action)
            .padding(20);

        center(
            row![sidebar, chart_area]
                .spacing(0)
                .height(Fill)
                .width(Fill),
        )
        .padding(10)
        .into()
    }

    fn theme(&self) -> Theme {
        self.theme.clone()
    }
}
