use hyozu::{Data, bars, chart};
use iced::widget::center;
use iced::{Task, Theme};

pub fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .title("hyozu • categorical axis labels")
        .theme(App::theme)
        .run()
}

struct App {
    data: Data,
}

#[derive(Debug, Clone)]
enum Message {}

impl App {
    fn new() -> (Self, Task<Message>) {
        // Create bar data with month labels
        let data = Data::from(bars([1200, 1900, 1500, 2200, 1800, 2400]))
            .title("Monthly Sales")
            .x_axis_labels(["Jan", "Feb", "Mar", "Apr", "May", "Jun"])
            .y_axis_labels(|v| format!("${:.0}k", v / 1000.0));

        (Self { data }, Task::none())
    }

    fn theme(&self) -> Theme {
        Theme::Dark
    }

    fn update(&mut self, _message: Message) -> Task<Message> {
        Task::none()
    }

    fn view(&self) -> iced::Element<'_, Message> {
        center(chart(&self.data).design(&Theme::Dark).width(800).height(600)).into()
    }
}
