use iced::Task;
use iced::widget::center;

use hyozu::data;
use hyozu::mark::boxplot::{boxplot, entry, entry_from_data};

pub fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .title("hyozu • box plot")
        .theme(iced::Theme::Light)
        .window_size([500.0, 400.0])
        .centered()
        .run()
}

struct App {
    data: hyozu::Data,
}

#[derive(Debug, Clone)]
enum Message {
    ChartAction(hyozu::Action),
}

impl App {
    fn new() -> Self {
        Self {
            data: data(boxplot([
                entry(2.0, 5.0, 7.0, 9.0, 12.0)
                    .with_name("Group A")
                    .with_outliers(vec![0.5, 14.0]),
                entry(3.0, 6.0, 8.0, 11.0, 15.0).with_name("Group B"),
                entry_from_data(&[
                    1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 20.0,
                ])
                .with_name("Group C"),
            ]))
            .title("Distribution Comparison")
            .y_axis_labels(|v: f64| format!("{:.0}", v))
            .x_axis_labels(["A", "B", "C"]),
        }
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ChartAction(action) => self.data.perform(action),
        }
        Task::none()
    }

    fn view(&self) -> iced::Element<'_, Message> {
        center(
            hyozu::chart(&self.data)
                .on_action(Message::ChartAction)
                .padding(40),
        )
        .padding(20)
        .into()
    }
}
