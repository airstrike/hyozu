use iced::widget::center;
use iced::{Task, Theme};

use hyozu::{Palette, bar, bars, data, encoding};

const MONTHS: [&str; 6] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun"];

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
            sales: data(
                bars([bar([1200, 1900, 1500, 2200, 1800, 2400]).color_by(encoding::key(|i, _| MONTHS[i]))])
                    .corner_radius(5.0),
            )
            .palette(Palette::Categorical)
            .x_axis_labels(MONTHS)
            .y_axis(|a| a.show_grid(true).show_minor_grid(true)),
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
