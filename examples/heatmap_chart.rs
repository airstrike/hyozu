use iced::widget::center;
use iced::{Task, Theme};

use hyozu::data;
use hyozu::mark::heatmap::heatmap;

pub fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .window_size([500.0, 500.0])
        .title("hyozu • heatmap")
        .run()
}

struct App {
    data: hyozu::Data,
}

pub type Message = ();

impl App {
    fn new() -> Self {
        // Correlation matrix
        let values = vec![
            1.0, 0.8, 0.3, -0.2, 0.8, 1.0, 0.5, 0.1, 0.3, 0.5, 1.0, 0.7, -0.2, 0.1, 0.7, 1.0,
        ];
        Self {
            data: data(
                heatmap(values, 4, 4)
                    .row_names(["A", "B", "C", "D"])
                    .col_names(["A", "B", "C", "D"])
                    .show_labels(true)
                    .label_format(|v| format!("{v:.1}"))
                    .value_range(-1.0, 1.0),
            )
            .title("Correlation Matrix"),
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
