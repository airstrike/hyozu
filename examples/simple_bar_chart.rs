use iced::widget::center;
use iced::{Task, Theme};

use hyozu::{bars, data};

pub fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .window_size([800.0, 500.0])
        .title("hyozu • simple bar chart")
        .run()
}

struct App {
    sales: hyozu::Data,
}

pub type Message = ();

impl App {
    fn new() -> Self {
        Self {
            sales: data(bars([1200, 1900, 1500, 2200, 1800, 2400]))
                .x_axis_labels(["Jan", "Feb", "Mar", "Apr", "May", "Jun"]),
        }
    }

    fn view(&self) -> iced::Element<'_, Message> {
        center(hyozu::chart(&self.sales).design(&Theme::Light).padding(20))
            .padding(20)
            .into()
    }

    fn update(&mut self, _: Message) -> Task<Message> {
        Task::none()
    }
}
