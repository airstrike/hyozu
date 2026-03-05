use iced::Task;
use iced::widget::center;

use hyozu::data;
use hyozu::mark::violin::{violin, violin_from_data};

pub fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .title("hyozu - violin chart")
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
        // Generate sample data
        let group_a: Vec<f64> = (0..50)
            .map(|i| 5.0 + (i as f64 * 0.1).sin() * 3.0 + (i as f64 * 0.07))
            .collect();
        let group_b: Vec<f64> = (0..50)
            .map(|i| 8.0 + (i as f64 * 0.15).cos() * 4.0 + (i as f64 * 0.05))
            .collect();
        let group_c: Vec<f64> = (0..50)
            .map(|i| 3.0 + (i as f64 * 0.2).sin() * 2.0 + (i as f64 * 0.03))
            .collect();

        Self {
            data: data(violin([
                violin_from_data(&group_a).with_name("Treatment A"),
                violin_from_data(&group_b).with_name("Treatment B"),
                violin_from_data(&group_c).with_name("Control"),
            ]))
            .title("Treatment Response Distribution")
            .x_axis_labels(["Treatment A", "Treatment B", "Control"]),
        }
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ChartAction(action) => self.data.perform(action),
        }
        Task::none()
    }

    fn view(&self) -> iced::Element<'_, Message> {
        center(hyozu::chart(&self.data).on_action(Message::ChartAction).padding(40))
            .padding(20)
            .into()
    }
}
