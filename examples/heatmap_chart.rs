use iced::widget::center;
use iced::{Task, Theme};

use hyozu::data;
use hyozu::mark::heatmap::heatmap;

pub fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .window_size([600.0, 500.0])
        .title("hyozu • heatmap")
        .run()
}

struct App {
    data: hyozu::Data,
}

pub type Message = ();

impl App {
    fn new() -> Self {
        // GitHub-style contribution activity — weeks × days
        #[rustfmt::skip]
        let values: Vec<f64> = vec![
            0.0, 0.0, 3.0, 1.0, 0.0, 2.0, 0.0,
            1.0, 0.0, 5.0, 8.0, 2.0, 0.0, 0.0,
            0.0, 4.0, 7.0,12.0, 6.0, 1.0, 0.0,
            2.0, 6.0,11.0,15.0, 9.0, 3.0, 0.0,
            0.0, 3.0, 8.0,14.0,10.0, 5.0, 1.0,
            1.0, 2.0, 6.0, 9.0, 7.0, 4.0, 0.0,
            0.0, 1.0, 4.0, 6.0, 3.0, 2.0, 0.0,
            0.0, 0.0, 2.0, 3.0, 1.0, 0.0, 0.0,
        ];

        let inferno = vec![
            iced::Color::from_rgb8(0x00, 0x00, 0x04), // near-black
            iced::Color::from_rgb8(0x42, 0x06, 0x58), // deep purple
            iced::Color::from_rgb8(0x93, 0x16, 0x54), // magenta
            iced::Color::from_rgb8(0xDD, 0x51, 0x29), // orange-red
            iced::Color::from_rgb8(0xFB, 0xA4, 0x0A), // amber
            iced::Color::from_rgb8(0xFC, 0xFE, 0xA4), // bright yellow
        ];

        Self {
            data: data(
                heatmap(values, 8, 7)
                    .col_names(["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"])
                    .row_names(["W1", "W2", "W3", "W4", "W5", "W6", "W7", "W8"])
                    .show_labels(true)
                    .label_format(|v| if v < 0.5 { String::new() } else { format!("{v:.0}") })
                    .color_stops(inferno),
            )
            .title("Commit Activity"),
        }
    }

    fn view(&self) -> iced::Element<'_, Message> {
        center(hyozu::chart(&self.data).design(&Theme::Dark).padding(40))
            .padding(20)
            .into()
    }

    fn update(&mut self, _: Message) -> Task<Message> {
        Task::none()
    }
}
