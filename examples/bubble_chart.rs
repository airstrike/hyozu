use hyozu::{Color, LegendConfig, chart, data, encoding, xy};
use iced::Task;
use iced::widget::center;

pub fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .title("hyozu \u{2022} bubble chart")
        .theme(iced::Theme::Light)
        .window_size([720.0, 540.0])
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
        // Eight cities. Tuple layout is (x, y, population_millions).
        // x = average July temperature (°C)
        // y = average annual rainfall (cm)
        // size encoding maps the population (third tuple slot, captured below)
        let cities: [(&str, f64, f64, f64); 8] = [
            ("Tokyo", 27.0, 156.0, 37.4),
            ("Delhi", 35.0, 79.0, 32.9),
            ("Shanghai", 28.0, 119.0, 29.2),
            ("São Paulo", 19.0, 144.0, 22.6),
            ("Mexico City", 17.0, 84.0, 22.3),
            ("Cairo", 28.0, 2.5, 22.2),
            ("Dhaka", 29.0, 220.0, 22.0),
            ("Mumbai", 28.0, 240.0, 21.3),
        ];

        // Parallel population vec, captured by the size_by closure so the
        // encoding can look it up by point index. The Datum itself stays 2D
        // (x, y); the size channel reaches outside via the closure.
        let pops: Vec<f64> = cities.iter().map(|c| c.3).collect();
        let points: Vec<(f64, f64)> = cities.iter().map(|c| (c.1, c.2)).collect();

        Self {
            data: data(
                xy(points)
                    .with_name("Megacities")
                    .color(Color::from_rgb8(0x35, 0x70, 0xB0))
                    // Sqrt scale (default) so the bubble area is perceptually
                    // proportional to population. Diameter 8–48 px.
                    .size_by(encoding::size_by(move |i, _| pops[i]).range(8.0..=48.0)),
            )
            .title("Megacities: temperature, rainfall, population")
            .x_axis_labels(|v| format!("{v:.0}\u{00b0}C"))
            .y_axis_labels(|v| format!("{v:.0} cm"))
            .legend(LegendConfig::right()),
        }
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ChartAction(action) => self.data.perform(action),
        }
        Task::none()
    }

    fn view(&self) -> iced::Element<'_, Message> {
        center(chart(&self.data).on_action(Message::ChartAction).padding(40))
            .padding(20)
            .into()
    }
}
