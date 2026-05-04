//! Demonstrates `Data::y_log()` — toggle a log-scaled y-axis on a line chart.
//!
//! Numeric-axis marks (Bar, Line, Area, Xy) honor `Scale::transform`.
//! Pie, Choropleth, TileGrid, and BubbleMap don't — pie is angle-based,
//! the others are categorical or geographic.
//!
//! With log on, the y-axis ticks land on decade boundaries (1, 10, 100,
//! 1000, 10000) and the line's exponential growth becomes a straight
//! diagonal. With log off, the first four points crush against the
//! baseline.

use iced::widget::{column, container};
use iced::{Center, Task};
use iced_widget::checkbox;

use hyozu::{Data, chart, data, line};

pub fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .title("hyozu • log y-axis")
        .theme(iced::Theme::Light)
        .window_size([520.0, 440.0])
        .centered()
        .run()
}

struct App {
    data: Data,
    log_y: bool,
}

#[derive(Debug, Clone)]
enum Message {
    ChartAction(hyozu::Action),
    LogYToggled(bool),
}

impl App {
    fn new() -> Self {
        let log_y = true;
        Self {
            data: build_data(log_y),
            log_y,
        }
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ChartAction(action) => {
                self.data.perform(action);
            }
            Message::LogYToggled(enabled) => {
                self.log_y = enabled;
                self.data = build_data(self.log_y);
            }
        }
        Task::none()
    }

    fn view(&self) -> iced::Element<'_, Message> {
        let toggle = checkbox(self.log_y).label("Log y-axis").on_toggle(Message::LogYToggled);

        container(
            column![toggle, chart(&self.data).on_action(Message::ChartAction).padding(40)]
                .spacing(12)
                .align_x(Center),
        )
        .padding(20)
        .into()
    }
}

fn build_data(log_y: bool) -> Data {
    // 41 samples along y = 10^(x/10). On a linear y-axis the curve hugs
    // the baseline for ~30 steps then explodes upward; on a log y-axis
    // the same curve becomes a straight diagonal across four decades.
    let exponential: Vec<f64> = (0..=40).map(|i| 10_f64.powf(i as f64 / 10.0)).collect();
    let mark = line(exponential).with_name("Growth");
    let d = data(mark).title("Exponential growth");
    if log_y { d.y_log() } else { d }
}
