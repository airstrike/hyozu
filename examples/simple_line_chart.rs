use iced::Task;
use iced::widget::center;

use hyozu::{chart, data, line};

pub fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .title("hyozu • simple line chart")
        .theme(iced::Theme::Light)
        .window_size([400.0, 400.0])
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
        use line::label::*;

        Self {
            data: data(
                line(std::array::from_fn::<usize, 10, _>(|i| (i * 7) % 13))
                    .data_labels(Position::Auto + Show::FirstAndLast),
            )
            .title("Temperature Over Time"),
        }
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ChartAction(action) => {
                self.data.perform(action);
            }
        }
        Task::none()
    }

    fn view(&self) -> iced::Element<'_, Message> {
        center(chart(&self.data).on_action(Message::ChartAction).padding(40))
            .padding(20)
            .into()
    }
}
