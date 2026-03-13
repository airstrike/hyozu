use iced::widget::{center, column, container, pick_list, radio, row, scrollable, slider, text};
use iced::{Center, Fill, Function, Pixels, Subscription, Task, Theme, keyboard};

use hyozu::axis::Placement;
use hyozu::data::Action;
use hyozu::target::Target;
use hyozu::{Data, Map, Palette, bar, bars, chart, item, pie, props};

pub fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .window_size([1050.0, 700.0])
        .antialiasing(true)
        .settings(iced::Settings {
            default_text_size: 13.into(),
            default_font: iced::Font::with_family("GT Pressura Mono"),
            ..Default::default()
        })
        .title("Hyozu - Interactive Chart")
        .theme(App::theme)
        .subscription(App::subscription)
        .run()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChartType {
    Bar,
    Pie,
}

impl std::fmt::Display for ChartType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChartType::Bar => write!(f, "Bar"),
            ChartType::Pie => write!(f, "Pie"),
        }
    }
}

struct App {
    chart_type: ChartType,
    bar_data: Data,
    pie_data: Data,
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
    ChartTypeChanged(ChartType),
}

const SERIES_NAMES: [&str; 4] = ["North", "South", "East", "West"];

const SLICE_NAMES: [&str; 5] = ["Product A", "Product B", "Product C", "Product D", "Product E"];

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
        let bar_mark = bars([
            bar([4200, 3800, 4500, 5100]).with_name("North"),
            bar([2800, 3200, 2600, 3400]).with_name("South"),
            bar([3100, 2900, 3600, 3300]).with_name("East"),
            bar([1900, 2400, 2100, 2800]).with_name("West"),
        ])
        .data_labels(bar::label::Position::End + currency);

        let bar_data = Data::from(bar_mark)
            .title("Quarterly Revenue by Region")
            .x_axis_labels(Placement::OnTicks + ["Q1", "Q2", "Q3", "Q4"])
            .y_axis_labels(|v: f64| currency(v));

        // Pie data: market share
        let pie_mark = pie([
            pie::slice(35.0).name("Product A"),
            pie::slice(25.0).name("Product B"),
            pie::slice(20.0).name("Product C"),
            pie::slice(12.0).name("Product D"),
            pie::slice(8.0).name("Product E"),
        ])
        .hole(0.4)
        .labels(pie::Label::percent());

        let pie_data = Data::from(pie_mark).title("Market Share by Product");

        (
            Self {
                chart_type: ChartType::Bar,
                bar_data,
                pie_data,
                theme: hyozu::theme::paper(),
                all_themes: hyozu::theme::all_themes().collect(),
            },
            Task::none(),
        )
    }

    fn data(&self) -> &Data {
        match self.chart_type {
            ChartType::Bar => &self.bar_data,
            ChartType::Pie => &self.pie_data,
        }
    }

    fn data_mut(&mut self) -> &mut Data {
        match self.chart_type {
            ChartType::Bar => &mut self.bar_data,
            ChartType::Pie => &mut self.pie_data,
        }
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Set(item) => {
                self.data_mut().perform(Action::Set(item));
            }
            Message::Action(action) => match &action {
                Action::Clicked(target) => {
                    // Sentinel = clicked empty area
                    if *target == Target::Mark(usize::MAX) {
                        self.data_mut().deselect();
                        return Task::none();
                    }

                    let ct = self.chart_type;
                    let data = self.data_mut();

                    match ct {
                        ChartType::Bar => {
                            // Two-level selection for bars:
                            // 1) Nothing/different-series selected -> select series
                            // 2) Same series selected -> select specific entry
                            // 3) Same entry selected -> deselect
                            if let Target::Entry { mark, series, index } = target {
                                match data.selection() {
                                    Some(Target::Entry {
                                        mark: m,
                                        series: s,
                                        index: i,
                                    }) if m == mark && s == series && i == index => {
                                        data.deselect();
                                    }
                                    Some(Target::Series { mark: m, series: s }) if m == mark && s == series => {
                                        data.select(target.clone());
                                    }
                                    _ => {
                                        data.select(Target::Series {
                                            mark: *mark,
                                            series: *series,
                                        });
                                    }
                                }
                            } else if let Target::EntryLabel { mark, series, index } = target {
                                // Two-level selection for bar labels:
                                // 1) Nothing/different-series -> SeriesLabel
                                // 2) SeriesLabel same series -> EntryLabel
                                // 3) EntryLabel same -> deselect
                                // 4) EntryLabel different label, same series -> new EntryLabel
                                // 5) Different series -> SeriesLabel of new series
                                match data.selection() {
                                    Some(Target::EntryLabel {
                                        mark: m,
                                        series: s,
                                        index: i,
                                    }) if m == mark && s == series && i == index => {
                                        data.deselect();
                                    }
                                    Some(Target::EntryLabel { mark: m, series: s, .. })
                                    | Some(Target::SeriesLabel { mark: m, series: s })
                                        if m == mark && s == series =>
                                    {
                                        data.select(target.clone());
                                    }
                                    _ => {
                                        data.select(Target::SeriesLabel {
                                            mark: *mark,
                                            series: *series,
                                        });
                                    }
                                }
                            } else if data.selection() == Some(target) {
                                data.deselect();
                            } else {
                                data.select(target.clone());
                            }
                        }
                        ChartType::Pie => {
                            // Pie: direct entry/label selection (no series level)
                            if data.selection() == Some(target) {
                                data.deselect();
                            } else {
                                data.select(target.clone());
                            }
                        }
                    }
                }
                Action::Set(item) => {
                    self.data_mut().perform(Action::Set(item.clone()));
                }
            },
            Message::ThemeChanged(theme) => {
                self.theme = theme;
            }
            Message::ChartTypeChanged(ct) => {
                self.chart_type = ct;
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
                Key::Named(Named::ArrowRight) | Key::Named(Named::ArrowDown) => Some(Message::NextTheme),
                Key::Named(Named::ArrowLeft) | Key::Named(Named::ArrowUp) => Some(Message::PreviousTheme),
                Key::Named(Named::Home) => Some(Message::FirstTheme),
                Key::Named(Named::End) => Some(Message::LastTheme),
                _ => None,
            }
        })
    }

    fn view(&self) -> iced::Element<'_, Message> {
        let data = self.data();

        // Selection info
        let selection_text = match (self.chart_type, data.selection()) {
            (_, None) => match self.chart_type {
                ChartType::Bar => "Click a bar to select".to_string(),
                ChartType::Pie => "Click a slice to select".to_string(),
            },
            (ChartType::Bar, Some(Target::Series { series, .. })) => {
                let name = SERIES_NAMES.get(*series).unwrap_or(&"?");
                format!("{name} (series)")
            }
            (ChartType::Bar, Some(Target::Entry { series, index, .. })) => {
                let name = SERIES_NAMES.get(*series).unwrap_or(&"?");
                let quarter = index + 1;
                format!("{name} Q{quarter}")
            }
            (ChartType::Bar, Some(Target::SeriesLabel { series, .. })) => {
                let name = SERIES_NAMES.get(*series).unwrap_or(&"?");
                format!("{name} labels")
            }
            (ChartType::Bar, Some(Target::EntryLabel { series, index, .. })) => {
                let name = SERIES_NAMES.get(*series).unwrap_or(&"?");
                let quarter = index + 1;
                format!("{name} Q{quarter} label")
            }
            (ChartType::Pie, Some(Target::Entry { index, .. })) => {
                let name = SLICE_NAMES.get(*index).unwrap_or(&"?");
                format!("{name} (slice)")
            }
            (ChartType::Pie, Some(Target::EntryLabel { index, .. })) => {
                let name = SLICE_NAMES.get(*index).unwrap_or(&"?");
                format!("{name} label")
            }
            (_, Some(other)) => format!("{other:?}"),
        };

        // --- Chart type toggle ---

        let chart_type_section = column![
            text("Chart Type").size(14),
            radio("Bar", ChartType::Bar, Some(self.chart_type), Message::ChartTypeChanged,),
            radio("Pie", ChartType::Pie, Some(self.chart_type), Message::ChartTypeChanged,),
        ]
        .spacing(4);

        let selection_section = column![text("Selection").size(14), text(selection_text).size(12),].spacing(4);

        // --- Color section ---

        let color_section: iced::Element<'_, Message> = match self.chart_type {
            ChartType::Bar => self.bar_color_section(),
            ChartType::Pie => self.pie_color_section(),
        };

        // --- Bar-specific controls ---

        let bar_controls: iced::Element<'_, Message> = if self.chart_type == ChartType::Bar {
            let size = self.bar_data.bars(0).map(|b| b.size()).unwrap_or(0.75);
            let spacing = self.bar_data.bars(0).map(|b| b.spacing()).unwrap_or(0.0);
            let layout = self
                .bar_data
                .bars(0)
                .map(|b| b.layout())
                .unwrap_or(bar::Layout::Grouped);
            let x_placement = (|| self.bar_data.x_axis_ref()?.placement())().unwrap_or(Placement::OnTicks);

            let on_size = |v| props::bar::Size(v).map(item::Bars.with(0)).map(Message::Set);
            let on_spacing = |v| props::bar::Spacing(v).map(item::Bars.with(0)).map(Message::Set);
            let on_layout = |l| props::bar::Layout(l).map(item::Bars.with(0)).map(Message::Set);
            let on_x_placement = |p| props::axis::Placement(p).map(item::XAxis).map(Message::Set);

            column![
                column![
                    text("Bar Size").size(14),
                    row![
                        slider(0.1..=1.0, size, on_size).step(0.05).width(Fill),
                        text(format!("{size:.2}")),
                    ]
                    .spacing(10)
                    .align_y(Center),
                ]
                .spacing(4),
                column![
                    text("Bar Spacing").size(14),
                    row![
                        slider(0.0..=1.0, spacing, on_spacing).step(0.05).width(Fill),
                        text(format!("{spacing:.2}")),
                    ]
                    .spacing(10)
                    .align_y(Center),
                ]
                .spacing(4),
                column![
                    text("Layout").size(14),
                    radio("Grouped", bar::Layout::Grouped, Some(layout), on_layout),
                    radio("Stacked", bar::Layout::Stacked, Some(layout), on_layout),
                    radio("Overlaid", bar::Layout::Overlaid, Some(layout), on_layout),
                ]
                .spacing(4),
                column![
                    text("X-Axis Labels").size(14),
                    radio("On Ticks", Placement::OnTicks, Some(x_placement), on_x_placement,),
                    radio(
                        "Between Ticks",
                        Placement::BetweenTicks,
                        Some(x_placement),
                        on_x_placement,
                    ),
                ]
                .spacing(4),
            ]
            .spacing(12)
            .into()
        } else {
            // Pie-specific controls
            let hole = self.pie_data.pie(0).map(|p| p.hole_value()).unwrap_or(0.0);

            let on_hole = |v| props::pie::Hole(v).map(item::Pie.with(0)).map(Message::Set);

            column![
                column![
                    text("Hole Size").size(14),
                    row![
                        slider(0.0..=0.9, hole, on_hole).step(0.05).width(Fill),
                        text(format!("{hole:.2}")),
                    ]
                    .spacing(10)
                    .align_y(Center),
                ]
                .spacing(4),
            ]
            .spacing(12)
            .into()
        };

        // --- Palette and theme ---

        let on_palette = |p: String| {
            let palette = match p.as_str() {
                "Categorical" => Palette::Categorical,
                "Sequential" => Palette::Sequential,
                _ => return Message::Set(item::Palette(Palette::Categorical)),
            };
            Message::Set(item::Palette(palette))
        };

        let current_palette = match data.get_palette() {
            Some(Palette::Categorical) => "Categorical",
            Some(Palette::Sequential) => "Sequential",
            Some(Palette::Gradient(_)) => "Gradient",
            None => "Auto",
        };

        let palette_section = column![
            text("Palette").size(14),
            pick_list(
                Some(current_palette.to_string()),
                vec!["Auto".to_string(), "Categorical".to_string(), "Sequential".to_string(),],
                |s: &String| s.clone(),
            )
            .on_select(on_palette)
            .width(Fill),
        ]
        .spacing(4);

        let theme_section = column![
            text("Theme").size(14),
            pick_list(Some(self.theme.clone()), self.all_themes.clone(), |t: &Theme| t
                .to_string(),)
            .on_select(Message::ThemeChanged)
            .width(Fill),
        ]
        .spacing(4);

        // --- Label controls ---

        let label_section: iced::Element<'_, Message> = match self.chart_type {
            ChartType::Bar => self.bar_label_controls_section(),
            ChartType::Pie => self.pie_label_controls_section(),
        };

        let sidebar = scrollable(
            container(
                column![
                    chart_type_section,
                    selection_section,
                    color_section,
                    label_section,
                    bar_controls,
                    palette_section,
                    theme_section,
                ]
                .spacing(12)
                .width(200),
            )
            .padding(15),
        )
        .direction(scrollable::Direction::Vertical(
            scrollable::Scrollbar::new().width(0).margin(0).scroller_width(1),
        ))
        .spacing(0);

        let chart_area = chart(data).design(&self.theme).on_action(Message::Action).padding(20);

        center(row![sidebar, chart_area].spacing(0).height(Fill).width(Fill))
            .padding(10)
            .into()
    }

    fn bar_color_section(&self) -> iced::Element<'_, Message> {
        // Determine what's selected
        let selected_series_idx = match self.bar_data.selection() {
            Some(Target::Series { series, .. }) => Some(*series),
            Some(Target::Entry { series, .. }) => Some(*series),
            _ => None,
        };

        let selected_entry = match self.bar_data.selection() {
            Some(Target::Entry { series, index, .. }) => Some((*series, *index)),
            _ => None,
        };

        let Some(si) = selected_series_idx else {
            return column![].into();
        };

        if let Some((series_idx, point_idx)) = selected_entry {
            // Entry selected — show per-point color picker
            let current_color = self
                .bar_data
                .bars(0)
                .and_then(|b| b.series(series_idx))
                .and_then(|s| s.point_color(point_idx))
                .and_then(|c| match c {
                    hyozu::Color::Fixed(fc) => Some(*fc),
                    _ => None,
                });

            let current_label = PRESET_COLORS
                .iter()
                .find(|(_, c)| *c == current_color)
                .map(|(name, _)| *name)
                .unwrap_or("Custom");

            let color_radios = PRESET_COLORS.iter().fold(column![].spacing(3), |col, (name, _)| {
                col.push(radio(*name, *name, Some(current_label), move |selected_name: &str| {
                    let color_opt = PRESET_COLORS
                        .iter()
                        .find(|(n, _)| *n == selected_name)
                        .and_then(|(_, c)| *c)
                        .map(hyozu::Color::Fixed);
                    props::bar::series::PointColor(point_idx, color_opt)
                        .map(props::bar::Series(series_idx))
                        .map(item::Bars.with(0))
                        .map(Message::Set)
                }))
            });

            column![text("Bar Color").size(14), color_radios,].spacing(4).into()
        } else {
            // Series selected — show series color picker
            let current_color = self
                .bar_data
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

            let color_radios = PRESET_COLORS.iter().fold(column![].spacing(3), |col, (name, _)| {
                col.push(radio(*name, *name, Some(current_label), move |selected_name: &str| {
                    let color_opt = PRESET_COLORS
                        .iter()
                        .find(|(n, _)| *n == selected_name)
                        .and_then(|(_, c)| *c)
                        .map(hyozu::Color::Fixed);
                    props::bar::series::Color(color_opt)
                        .map(props::bar::Series(si))
                        .map(item::Bars.with(0))
                        .map(Message::Set)
                }))
            });

            column![text("Series Color").size(14), color_radios,].spacing(4).into()
        }
    }

    fn pie_color_section(&self) -> iced::Element<'_, Message> {
        let selected_slice = match self.pie_data.selection() {
            Some(Target::Entry { index, .. }) => Some(*index),
            _ => None,
        };

        let Some(slice_idx) = selected_slice else {
            return column![].into();
        };

        // Get current slice color
        let current_color = self
            .pie_data
            .pie(0)
            .and_then(|p| p.slices().get(slice_idx))
            .and_then(|s| match s.get_color() {
                Some(hyozu::Color::Fixed(fc)) => Some(*fc),
                _ => None,
            });

        let current_label = PRESET_COLORS
            .iter()
            .find(|(_, c)| *c == current_color)
            .map(|(name, _)| *name)
            .unwrap_or("Custom");

        let color_radios = PRESET_COLORS.iter().fold(column![].spacing(3), |col, (name, _)| {
            col.push(radio(*name, *name, Some(current_label), move |selected_name: &str| {
                let color_opt = PRESET_COLORS
                    .iter()
                    .find(|(n, _)| *n == selected_name)
                    .and_then(|(_, c)| *c)
                    .map(hyozu::Color::Fixed);
                props::pie::SliceColor(slice_idx, color_opt)
                    .map(item::Pie.with(0))
                    .map(Message::Set)
            }))
        });

        column![text("Slice Color").size(14), color_radios,].spacing(4).into()
    }

    fn bar_label_controls_section(&self) -> iced::Element<'_, Message> {
        use iced::font::{Style, Weight};

        // Get the label-selected series/entry
        let (series_idx, entry_idx) = match self.bar_data.selection() {
            Some(Target::SeriesLabel { series, .. }) => (*series, None),
            Some(Target::EntryLabel { series, index, .. }) => (*series, Some(*index)),
            _ => return column![].into(),
        };

        // Read current label state from the series
        let series = match self.bar_data.bars(0).and_then(|b| b.series(series_idx)) {
            Some(s) => s,
            None => return column![].into(),
        };

        // Resolve current values: point override -> series label
        let series_label = series.label();
        let point_label = entry_idx.and_then(|i| series.point_label(i));

        let current_size = point_label
            .and_then(|l| l.size())
            .or_else(|| series_label.and_then(|l| l.size()))
            .map(|p| p.0)
            .unwrap_or(12.0);
        let current_weight = point_label
            .and_then(|l| l.weight())
            .or_else(|| series_label.and_then(|l| l.weight()))
            .unwrap_or(Weight::Normal);
        let current_style = point_label
            .and_then(|l| l.style())
            .or_else(|| series_label.and_then(|l| l.style()))
            .unwrap_or(Style::Normal);

        let current_label_color = point_label
            .and_then(|l| l.color().copied())
            .or_else(|| series_label.and_then(|l| l.color().copied()));
        let current_fill = point_label
            .and_then(|l| l.fill().copied())
            .or_else(|| series_label.and_then(|l| l.fill().copied()));

        // Build controls - route through appropriate props based on selection level
        let on_size = move |v: f32| {
            let size = Some(Pixels(v));
            if let Some(idx) = entry_idx {
                props::bar::series::PointLabel(idx, props::bar::series::PointLabelProperty::Size(size))
                    .map(props::bar::Series(series_idx))
                    .map(item::Bars.with(0))
                    .map(Message::Set)
            } else {
                props::bar::series::Property::LabelSize(size)
                    .map(props::bar::Series(series_idx))
                    .map(item::Bars.with(0))
                    .map(Message::Set)
            }
        };

        let on_weight = move |w: Weight| {
            let weight = Some(w);
            if let Some(idx) = entry_idx {
                props::bar::series::PointLabel(idx, props::bar::series::PointLabelProperty::Weight(weight))
                    .map(props::bar::Series(series_idx))
                    .map(item::Bars.with(0))
                    .map(Message::Set)
            } else {
                props::bar::series::Property::LabelWeight(weight)
                    .map(props::bar::Series(series_idx))
                    .map(item::Bars.with(0))
                    .map(Message::Set)
            }
        };

        let on_style = move |s: Style| {
            let style = Some(s);
            if let Some(idx) = entry_idx {
                props::bar::series::PointLabel(idx, props::bar::series::PointLabelProperty::Style(style))
                    .map(props::bar::Series(series_idx))
                    .map(item::Bars.with(0))
                    .map(Message::Set)
            } else {
                props::bar::series::Property::LabelStyle(style)
                    .map(props::bar::Series(series_idx))
                    .map(item::Bars.with(0))
                    .map(Message::Set)
            }
        };

        // Label color radios
        let current_color_iced = current_label_color.and_then(|c| match c {
            hyozu::Color::Fixed(fc) => Some(fc),
            _ => None,
        });
        let color_label = PRESET_COLORS
            .iter()
            .find(|(_, c)| *c == current_color_iced)
            .map(|(name, _)| *name)
            .unwrap_or("Custom");
        let color_radios = PRESET_COLORS.iter().fold(column![].spacing(3), |col, (name, _)| {
            col.push(radio(*name, *name, Some(color_label), move |selected_name: &str| {
                let color_opt = PRESET_COLORS
                    .iter()
                    .find(|(n, _)| *n == selected_name)
                    .and_then(|(_, c)| *c)
                    .map(hyozu::Color::Fixed);
                if let Some(idx) = entry_idx {
                    props::bar::series::PointLabel(idx, props::bar::series::PointLabelProperty::Color(color_opt))
                        .map(props::bar::Series(series_idx))
                        .map(item::Bars.with(0))
                        .map(Message::Set)
                } else {
                    props::bar::series::Property::LabelColor(color_opt)
                        .map(props::bar::Series(series_idx))
                        .map(item::Bars.with(0))
                        .map(Message::Set)
                }
            }))
        });

        // Fill color radios
        let current_fill_iced = current_fill.and_then(|c| match c {
            hyozu::Color::Fixed(fc) => Some(fc),
            _ => None,
        });
        let fill_label = PRESET_COLORS
            .iter()
            .find(|(_, c)| *c == current_fill_iced)
            .map(|(name, _)| *name)
            .unwrap_or("Custom");
        let fill_radios = PRESET_COLORS.iter().fold(column![].spacing(3), |col, (name, _)| {
            col.push(radio(*name, *name, Some(fill_label), move |selected_name: &str| {
                let color_opt = PRESET_COLORS
                    .iter()
                    .find(|(n, _)| *n == selected_name)
                    .and_then(|(_, c)| *c)
                    .map(hyozu::Color::Fixed);
                if let Some(idx) = entry_idx {
                    props::bar::series::PointLabel(idx, props::bar::series::PointLabelProperty::Fill(color_opt))
                        .map(props::bar::Series(series_idx))
                        .map(item::Bars.with(0))
                        .map(Message::Set)
                } else {
                    props::bar::series::Property::LabelFill(color_opt)
                        .map(props::bar::Series(series_idx))
                        .map(item::Bars.with(0))
                        .map(Message::Set)
                }
            }))
        });

        column![
            column![
                text("Font Size").size(14),
                row![
                    slider(8.0..=24.0, current_size, on_size).step(1.0).width(Fill),
                    text(format!("{current_size:.0}")),
                ]
                .spacing(10)
                .align_y(Center),
            ]
            .spacing(4),
            column![
                text("Bold").size(14),
                radio("Normal", Weight::Normal, Some(current_weight), on_weight),
                radio("Bold", Weight::Bold, Some(current_weight), on_weight),
            ]
            .spacing(4),
            column![
                text("Italic").size(14),
                radio("Normal", Style::Normal, Some(current_style), on_style),
                radio("Italic", Style::Italic, Some(current_style), on_style),
            ]
            .spacing(4),
            column![text("Label Color").size(14), color_radios,].spacing(4),
            column![text("Fill").size(14), fill_radios,].spacing(4),
        ]
        .spacing(12)
        .into()
    }

    fn pie_label_controls_section(&self) -> iced::Element<'_, Message> {
        use iced::font::{Style, Weight};

        let slice_idx = match self.pie_data.selection() {
            Some(Target::EntryLabel { index, .. }) => *index,
            _ => return column![].into(),
        };

        // Read current label state from the slice
        let label = self
            .pie_data
            .pie(0)
            .and_then(|p| p.slices().get(slice_idx))
            .and_then(|s| s.get_label());

        let Some(label) = label else {
            return column![].into();
        };

        let current_size = label.size().map(|p| p.0).unwrap_or(12.0);
        let current_weight = label.weight().unwrap_or(Weight::Normal);
        let current_style = label.style().unwrap_or(Style::Normal);

        let on_size = move |v: f32| {
            props::pie::SliceLabel(slice_idx, props::pie::label::Property::Size(Some(Pixels(v))))
                .map(item::Pie.with(0))
                .map(Message::Set)
        };

        let on_weight = move |w: Weight| {
            props::pie::SliceLabel(slice_idx, props::pie::label::Property::Weight(Some(w)))
                .map(item::Pie.with(0))
                .map(Message::Set)
        };

        let on_style = move |s: Style| {
            props::pie::SliceLabel(slice_idx, props::pie::label::Property::Style(Some(s)))
                .map(item::Pie.with(0))
                .map(Message::Set)
        };

        // Label color radios
        let current_color_iced = label.color().copied().and_then(|c| match c {
            hyozu::Color::Fixed(fc) => Some(fc),
            _ => None,
        });
        let color_label_str = PRESET_COLORS
            .iter()
            .find(|(_, c)| *c == current_color_iced)
            .map(|(name, _)| *name)
            .unwrap_or("Custom");
        let color_radios = PRESET_COLORS.iter().fold(column![].spacing(3), |col, (name, _)| {
            col.push(radio(
                *name,
                *name,
                Some(color_label_str),
                move |selected_name: &str| {
                    let color_opt = PRESET_COLORS
                        .iter()
                        .find(|(n, _)| *n == selected_name)
                        .and_then(|(_, c)| *c)
                        .map(hyozu::Color::Fixed);
                    props::pie::SliceLabel(slice_idx, props::pie::label::Property::Color(color_opt))
                        .map(item::Pie.with(0))
                        .map(Message::Set)
                },
            ))
        });

        // Fill color radios
        let current_fill_iced = label.fill().copied().and_then(|c| match c {
            hyozu::Color::Fixed(fc) => Some(fc),
            _ => None,
        });
        let fill_label_str = PRESET_COLORS
            .iter()
            .find(|(_, c)| *c == current_fill_iced)
            .map(|(name, _)| *name)
            .unwrap_or("Custom");
        let fill_radios = PRESET_COLORS.iter().fold(column![].spacing(3), |col, (name, _)| {
            col.push(radio(*name, *name, Some(fill_label_str), move |selected_name: &str| {
                let color_opt = PRESET_COLORS
                    .iter()
                    .find(|(n, _)| *n == selected_name)
                    .and_then(|(_, c)| *c)
                    .map(hyozu::Color::Fixed);
                props::pie::SliceLabel(slice_idx, props::pie::label::Property::Fill(color_opt))
                    .map(item::Pie.with(0))
                    .map(Message::Set)
            }))
        });

        column![
            column![
                text("Font Size").size(14),
                row![
                    slider(8.0..=24.0, current_size, on_size).step(1.0).width(Fill),
                    text(format!("{current_size:.0}")),
                ]
                .spacing(10)
                .align_y(Center),
            ]
            .spacing(4),
            column![
                text("Bold").size(14),
                radio("Normal", Weight::Normal, Some(current_weight), on_weight),
                radio("Bold", Weight::Bold, Some(current_weight), on_weight),
            ]
            .spacing(4),
            column![
                text("Italic").size(14),
                radio("Normal", Style::Normal, Some(current_style), on_style),
                radio("Italic", Style::Italic, Some(current_style), on_style),
            ]
            .spacing(4),
            column![text("Label Color").size(14), color_radios,].spacing(4),
            column![text("Fill").size(14), fill_radios,].spacing(4),
        ]
        .spacing(12)
        .into()
    }

    fn theme(&self) -> Theme {
        self.theme.clone()
    }
}
