//! Demonstrates `Data::y_log()` — a log-scaled y-axis on a line chart.
//!
//! Numeric-axis marks (Bar, Line, Area, Xy) honor `Scale::transform`.
//! Pie, Choropleth, TileGrid, and BubbleMap don't — pie is angle-based,
//! the others are categorical or geographic.
//!
//! With `.y_log()`, the y-axis ticks land on decade boundaries (1, 10,
//! 100, 1000, 10000) and the line's exponential growth becomes a
//! straight diagonal.

use iced::Task;
use iced::widget::center;

use hyozu::{chart, data, line};

pub fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .title("hyozu • log y-axis")
        .theme(iced::Theme::Light)
        .window_size([520.0, 400.0])
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
        // Five points along an exponential. On a linear y-axis the first
        // four would crush against the baseline; on a log axis they
        // spread evenly.
        let exponential = [1.0, 10.0, 100.0, 1000.0, 10000.0];
        Self {
            data: data(line(exponential).with_name("Growth"))
                .title("Exponential growth (log y-axis)")
                .y_log(),
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
