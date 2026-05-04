use iced::Task;
use iced::widget::center;

use hyozu::mark::area::LineStyle;
use hyozu::mark::area::label::{Label, Position, Show};
use hyozu::mark::area::marker::Shape;
use hyozu::{area, areas, chart, data, palette};

pub fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .title("hyozu \u{2022} area chart")
        .theme(iced::Theme::Light)
        .window_size([500.0, 400.0])
        .centered()
        .run()
}

struct App {
    data: data::Data,
}

#[derive(Debug, Clone)]
enum Message {
    ChartAction(hyozu::Action),
}

impl App {
    fn new() -> Self {
        Self {
            data: data(
                areas([
                    area([218, 250, 230, 270, 240, 280, 260])
                        .with_name("Revenue")
                        .markers(Shape::Circle)
                        .data_labels(Label::new().show(Show::FirstAndLast)),
                    area([150, 180, 160, 200, 170, 210, 190])
                        .with_name("Expenses")
                        .style(LineStyle::Dashed)
                        .data_labels(Label::new().show(Show::Any) + Position::Below),
                ])
                .stacked(),
            )
            .title("Revenue vs Expenses")
            .palette(palette::categorical())
            .y_axis(|axis| axis.with_bounds(100.0, 500.0)),
        }
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ChartAction(action) => self.data.perform(action),
        }
        Task::none()
    }

    fn view(&self) -> iced::Element<'_, Message> {
        center(chart(&self.data).on_action(Message::ChartAction).padding(40))
            .padding(20)
            .into()
    }
}
