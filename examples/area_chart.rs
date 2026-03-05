use iced::Task;
use iced::widget::center;

use hyozu::{area, areas, chart, data};

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
                    area([2, 4, 3, 5, 4, 6, 5]).with_name("Revenue"),
                    area([1, 2, 2, 3, 2, 4, 3]).with_name("Expenses"),
                ])
                .stacked(),
            )
            .title("Revenue vs Expenses"),
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
