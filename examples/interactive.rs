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
        .window_size([1050.0, 700.0])
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

const SERIES_NAMES: [&str; 4] = ["North", "South", "East", "West"];

const PRESET_COLORS: [(&str, Option<iced::Color>); 7] = [
    ("Auto", None),
    ("Red", Some(iced::Color::from_rgb(0.85, 0.2, 0.2))),
    ("Blue", Some(iced::Color::from_rgb(0.2, 0.4, 0.85))),
    ("Green", Some(iced::Color::from_rgb(0.2, 0.7, 0.3))),
    ("Orange", Some(iced::Color::from_rgb(0.9, 0.55, 0.1))),
    ("Purple", Some(iced::Color::from_rgb(0.6, 0.3, 0.75))),
    ("Teal", Some(iced::Color::from_rgb(0.15, 0.65, 0.6))),
];

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
        // 4 series (regions) × 4 categories (quarters)
        let bars = bars([
            bar([4200, 3800, 4500, 5100]).with_name("North"),
            bar([2800, 3200, 2600, 3400]).with_name("South"),
            bar([3100, 2900, 3600, 3300]).with_name("East"),
            bar([1900, 2400, 2100, 2800]).with_name("West"),
        ])
        .data_labels(bar::label::Position::End + currency);

        let data = Data::from(bars)
            .title("Quarterly Revenue by Region")
            .x_axis_labels(Placement::OnTicks + ["Q1", "Q2", "Q3", "Q4"])
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
                    // Sentinel = clicked empty area
                    if *target == Target::Mark(usize::MAX) {
                        self.data.deselect();
                        return Task::none();
                    }

                    // Two-level selection:
                    // 1) Nothing/different-series selected -> select series
                    // 2) Same series selected -> select specific entry
                    // 3) Same entry selected -> deselect
                    if let Target::Entry {
                        mark,
                        series,
                        index,
                    } = target
                    {
                        match self.data.selection() {
                            Some(Target::Entry {
                                mark: m,
                                series: s,
                                index: i,
                            }) if m == mark && s == series && i == index => {
                                self.data.deselect();
                            }
                            Some(Target::Series { mark: m, series: s })
                                if m == mark && s == series =>
                            {
                                self.data.select(target.clone());
                            }
                            _ => {
                                self.data.select(Target::Series {
                                    mark: *mark,
                                    series: *series,
                                });
                            }
                        }
                    } else if self.data.selection() == Some(target) {
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

        let x_placement = (|| self.data.x_axis_ref()?.placement())()
            .unwrap_or(Placement::OnTicks);

        // Selection info
        let selected_series_idx = match self.data.selection() {
            Some(Target::Series { series, .. }) => Some(*series),
            Some(Target::Entry { series, .. }) => Some(*series),
            _ => None,
        };

        let selection_text = match self.data.selection() {
            None => "Click a bar to select".to_string(),
            Some(Target::Series { series, .. }) => {
                let name = SERIES_NAMES.get(*series).unwrap_or(&"?");
                format!("{name} (series)")
            }
            Some(Target::Entry { series, index, .. }) => {
                let name = SERIES_NAMES.get(*series).unwrap_or(&"?");
                let quarter = index + 1;
                format!("{name} Q{quarter}")
            }
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

        let current_palette = match self.data.get_palette() {
            Some(Palette::Categorical) => "Categorical",
            Some(Palette::Sequential) => "Sequential",
            Some(Palette::Gradient(_)) => "Gradient",
            None => "Auto",
        };

        // --- Sidebar sections ---

        let selection_section =
            column![text("Selection").size(14), text(selection_text).size(12),]
                .spacing(4);

        // Series color section (only shown when a series is selected)
        let color_section: iced::Element<'_, Message> =
            if let Some(si) = selected_series_idx {
                // Get current color for this series
                let current_color = self
                    .data
                    .bars(0)
                    .and_then(|b| b.series(si))
                    .and_then(|s| s.color())
                    .and_then(|c| match c {
                        hyozu::Color::Fixed(fc) => Some(*fc),
                        _ => None,
                    });

                let current_label = PRESET_COLORS
                    .iter()
                    .find(|(_, c)| *c == current_color)
                    .map(|(name, _)| *name)
                    .unwrap_or("Custom");

                let color_radios = PRESET_COLORS.iter().fold(
                    column![].spacing(3),
                    |col, (name, _)| {
                        col.push(radio(
                            *name,
                            *name,
                            Some(current_label),
                            move |selected_name: &str| {
                                let color_opt = PRESET_COLORS
                                    .iter()
                                    .find(|(n, _)| *n == selected_name)
                                    .and_then(|(_, c)| *c)
                                    .map(hyozu::Color::Fixed);
                                props::bar::series::Color(color_opt)
                                    .map(props::bar::Series(si))
                                    .map(item::Bars.with(0))
                                    .map(Message::Set)
                            },
                        ))
                    },
                );

                column![text("Series Color").size(14), color_radios,]
                    .spacing(4)
                    .into()
            } else {
                column![].into()
            };

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
                color_section,
                size_section,
                spacing_section,
                layout_section,
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
