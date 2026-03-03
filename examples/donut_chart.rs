use iced::widget::center;
use iced::{Task, Theme};

use hyozu::{data, pie};

pub fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .window_size([600.0, 500.0])
        .title("hyozu • donut chart")
        .run()
}

struct App {
    chart: hyozu::Data,
}

pub type Message = ();

impl App {
    fn new() -> Self {
        Self {
            chart: data(pie([30, 50, 20, 40]).hole(0.6)),
        }
    }

    fn view(&self) -> iced::Element<'_, Message> {
        center(hyozu::chart(&self.chart).design(&Theme::Light).padding(20))
            .padding(20)
            .into()
    }

    fn update(&mut self, _: Message) -> Task<Message> {
        Task::none()
    }
}
