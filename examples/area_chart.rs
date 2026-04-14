use iced::Task;
use iced::widget::center;

use hyozu::mark::area::label::{Label, Position, Show};
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
                        .data_labels(Label::new().show(Show::FirstAndLast)),
                    area([150, 180, 160, 200, 170, 210, 190])
                        .with_name("Expenses")
                        .data_labels(Label::new().show(Show::Any) + Position::Below),
                ])
                .stacked()
                .gradient(true),
            )
            .title("Revenue vs Expenses")
            .palette(palette::categorical()),
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
