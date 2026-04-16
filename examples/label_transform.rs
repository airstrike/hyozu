use hyozu::axis::Placement;
use hyozu::data::Action;
use hyozu::{Data, bar, bars, chart, item, props};
use iced::Alignment::End;
use iced::widget::{center, column, container, row, slider, text};
use iced::{Center, Element, Fill, Function, Task, Theme};

pub fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .title("hyozu • customizing spacing and axis labels")
        .settings(iced::Settings {
            default_text_size: 13.into(),
            ..Default::default()
        })
        .theme(iced::Theme::Light)
        .run()
}

struct App {
    simple: Data,
    with_placement: Data,
    with_transform: Data,
    full_control: Data,
    size: f32,
    spacing: f32,
}

#[derive(Debug, Clone)]
enum Message {
    Set(item::Item),
}

impl App {
    fn new() -> (Self, Task<Message>) {
        let size = 0.75; // Default bar length (75% of available width)
        let spacing = 0.0; // Default spacing (no spacing)

        let (simple, with_placement, with_transform, full_control) = Self::create_charts(size, spacing);

        (
            Self {
                simple,
                with_placement,
                with_transform,
                full_control,
                size,
                spacing,
            },
            Task::none(),
        )
    }

    fn create_charts(size: f32, spacing: f32) -> (Data, Data, Data, Data) {
        let months = ["January", "February", "March", "April", "May", "Jun"];

        // 1. Simple categorical labels
        let simple = Data::from(bars([1200, 1900, 1500, 2200]).with_size(size).with_spacing(spacing))
            .title("Simple")
            .x_axis_labels(["Q1", "Q2", "Q3", "Q4"])
            .y_axis_labels(|v| format!("${:.0}", v));

        // 2. Placement + labels with convenience methods
        let with_placement = Data::from(
            bars![[50, 75, 90, 60, 80, 95], [75, 40, 30, 90, 50, 60]]
                .with_size(size)
                .with_spacing(spacing)
                .data_labels(bar::label::Position::End),
        )
        .title("Grouped Bars, Labels on End")
        .x_axis_labels(Placement::BetweenTicks + months + |m: &str| m[..3].to_string())
        .y_axis_labels(|v| format!("{:.0}%", v));

        // 3. Transform labels with .map()
        let with_transform = Data::from(
            bars([2100, 1800, 2400, 2000])
                .with_size(size)
                .with_spacing(spacing)
                .data_labels(None),
        )
        .title("Simple label formatting")
        .x_axis_labels(Placement::OnTicks + months.map(|m| m[..1].to_uppercase()))
        .y_axis_labels(|v| format!("${:.0}k", v / 1000.0));

        // 4. Multiple axis properties (use .x_axis() as escape hatch)
        let full_control = Data::from(bars([45, 67, 52, 78]).with_size(size).with_spacing(spacing))
            .title("Different label API")
            .x_axis(|axis| axis.labels(Placement::OnTicks + ["North", "South", "East", "West"]));

        (simple, with_placement, with_transform, full_control)
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Set(ref item) => {
                // Update local state for display
                match item {
                    item::Item::Bars(_, props::bar::Property::Size(v)) => self.size = *v,
                    item::Item::Bars(_, props::bar::Property::Spacing(v)) => self.spacing = *v,
                    _ => {}
                }

                // Apply to all charts
                let action = Action::Set(item.clone());
                self.simple.perform(action.clone());
                self.with_placement.perform(action.clone());
                self.with_transform.perform(action.clone());
                self.full_control.perform(action);
            }
        }
        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        center(
            column![
                container(
                    row![
                        row![
                            text("Bar Length:"),
                            Element::from(slider(0.1..=1.0, self.size, props::bar::Size).step(0.01).width(200))
                                .map(item::Bars.with(0))
                                .map(Message::Set),
                            text!("{:.0}%", self.size * 100.0).width(40).align_x(End),
                        ]
                        .spacing(5)
                        .align_y(Center),
                        row![
                            text("Spacing:"),
                            Element::from(
                                slider(0.0..=1.0, self.spacing, props::bar::Spacing)
                                    .step(0.01)
                                    .width(200)
                            )
                            .map(item::Bars.with(0))
                            .map(Message::Set),
                            text!("{:.0}%", self.spacing * 100.0).width(40).align_x(End),
                        ]
                        .spacing(5)
                        .align_y(Center),
                    ]
                    .spacing(40)
                    .align_y(Center)
                )
                .center_x(Fill),
                row![
                    chart(&self.simple).design(&Theme::Dark),
                    chart(&self.with_placement).design(&Theme::Light),
                ]
                .padding(20)
                .spacing(20),
                row![
                    chart(&self.with_transform).design(&Theme::TokyoNightLight),
                    chart(&self.full_control).design(&Theme::TokyoNight),
                ]
                .padding(20)
                .spacing(20),
            ]
            .align_x(Center)
            .spacing(20),
        )
        .padding(20)
        .into()
    }
}
