use iced::widget::center;
use iced::{Task, Theme};

use hyozu::{Mark, bars, data, line};

pub fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .window_size([800.0, 500.0])
        .title("hyozu • dual axis")
        .run()
}

struct App {
    chart: hyozu::Data,
}

pub type Message = ();

impl App {
    fn new() -> Self {
        let revenue = bars([1200, 1900, 1500, 2200, 1800, 2400]);
        let conversion_rate = line([0.12, 0.18, 0.15, 0.22, 0.19, 0.25]);

        Self {
            chart: data(revenue)
                .x_axis_labels(["Jan", "Feb", "Mar", "Apr", "May", "Jun"])
                .secondary(vec![Mark::Line(conversion_rate)])
                .right_axis(|a| a.title("Conversion"))
                .title("Revenue with Conversion Rate"),
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
