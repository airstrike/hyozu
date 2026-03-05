use iced::widget::center;
use iced::{Task, Theme};

use hyozu::{bars, data};

pub fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .window_size([600.0, 400.0])
        .title("hyozu • horizontal bars")
        .run()
}

struct App {
    data: hyozu::Data,
}

pub type Message = ();

impl App {
    fn new() -> Self {
        Self {
            data: data(bars([120, 190, 150, 80, 210]).horizontal())
                .title("Sales by Region")
                .x_axis_labels(|v: f64| format!("{:.0}", v))
                .y_axis_labels(["North", "South", "East", "West", "Central"]),
        }
    }

    fn view(&self) -> iced::Element<'_, Message> {
        center(hyozu::chart(&self.data).design(&Theme::Light).padding(40))
            .padding(20)
            .into()
    }

    fn update(&mut self, _: Message) -> Task<Message> {
        Task::none()
    }
}
