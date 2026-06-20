use hyozu::data::Action;
use hyozu::mark::bar::label::Position;
use hyozu::{Data, bar, bars, chart, item, props};
use iced::widget::{center, column, container, row, slider, text};
use iced::{Center, Element, Fill, Function, Task, Theme};

pub fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .window_size([800.0, 500.0])
        .settings(iced::Settings {
            default_text_size: 11.into(),
            ..Default::default()
        })
        .title("hyozu - label geometry")
        .theme(Theme::Light)
        .run()
}

struct App {
    data: Data,
    corner_radius: f32,
    bar_size: f32,
    label_size: f32,
}

#[derive(Debug, Clone)]
enum Message {
    Set(item::Item),
    CornerRadius(f32),
    LabelSize(f32),
}

const LABELS: [&str; 10] = [
    "Alpha", "Beta", "Gamma", "Delta", "Epsilon", "Zeta", "Eta", "Theta", "Iota", "Kappa",
];

impl App {
    fn new() -> (Self, Task<Message>) {
        let bar_size = 0.75;
        let label_size = 12.0;

        let data = Self::build_data(bar_size, label_size);

        (
            Self {
                data,
                corner_radius: 4.0,
                bar_size,
                label_size,
            },
            Task::none(),
        )
    }

    fn build_data(bar_size: f32, label_size: f32) -> Data {
        let series = bar([5000.0, 800.0, 400.0, 200.0, 100.0, 50.0, 30.0, 15.0, 5.0, 2.0]).with_labels(
            Position::End + hyozu::text::Style::new().size(label_size) + (|v: f64| format!("{}", v as i64)),
        );

        Data::from(bars([series]).horizontal().with_size(bar_size))
            .y_axis_labels(LABELS)
            .x_axis(|a| a.none())
            .y_axis(|a| a.show_line(false).show_grid(false))
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Set(ref item) => {
                if let item::Item::Bars(_, props::bar::Property::Size(v)) = item {
                    self.bar_size = *v;
                }
                self.data.perform(Action::Set(item.clone()));
            }
            Message::CornerRadius(v) => {
                self.corner_radius = v;
            }
            Message::LabelSize(v) => {
                self.label_size = v;
                self.data = Self::build_data(self.bar_size, v);
            }
        }
        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        let controls = container(
            row![
                row![
                    text("Corner Radius:"),
                    slider(0.0..=32.0, self.corner_radius, Message::CornerRadius)
                        .step(1.0f32)
                        .width(Fill),
                    text!("{:.0}", self.corner_radius).width(30),
                ]
                .spacing(5)
                .align_y(Center),
                row![
                    text("Bar Height:"),
                    Element::from(
                        slider(0.1..=1.0, self.bar_size, props::bar::Size)
                            .step(0.01f32)
                            .width(Fill)
                    )
                    .map(item::Bars.with(0))
                    .map(Message::Set),
                    text!("{:.0}%", self.bar_size * 100.0).width(40),
                ]
                .spacing(5)
                .align_y(Center),
                row![
                    text("Label Size:"),
                    slider(8.0..=24.0, self.label_size, Message::LabelSize)
                        .step(1.0f32)
                        .width(Fill),
                    text!("{:.0}", self.label_size).width(30),
                ]
                .spacing(5)
                .align_y(Center),
            ]
            .spacing(30)
            .align_y(Center),
        )
        .center_x(Fill);

        center(
            column![
                controls,
                chart(&self.data)
                    .design(&Theme::Light)
                    .padding(20)
                    .style(|d| hyozu::chart::Style {
                        corners: self.corner_radius.into(),
                        ..hyozu::chart::default(d)
                    }),
            ]
            .align_x(Center)
            .spacing(20),
        )
        .padding(20)
        .into()
    }
}
