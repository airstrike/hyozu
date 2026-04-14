use iced::widget::center;
use iced::{Element, Font, Task, color};

use hyozu::mark::bar::label::Position;
use hyozu::{Mark, bar, bars, data, encoding, tick};

pub fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .window_size([720.0, 480.0])
        .title("hyozu • goal dashboard")
        .run()
}

struct App {
    data: hyozu::Data,
    design: Design,
}

type Message = ();

struct Country {
    name: &'static str,
    actual: f64,
    target: f64,
}

const COUNTRIES: [Country; 6] = [
    Country {
        name: "Argentina",
        actual: 72.0,
        target: 80.0,
    },
    Country {
        name: "Brazil",
        actual: 110.0,
        target: 95.0,
    },
    Country {
        name: "Chile",
        actual: 68.0,
        target: 75.0,
    },
    Country {
        name: "Colombia",
        actual: 91.0,
        target: 85.0,
    },
    Country {
        name: "Mexico",
        actual: 105.0,
        target: 100.0,
    },
    Country {
        name: "Peru",
        actual: 80.0,
        target: 90.0,
    },
];

// Blues from lighter (below target) to deeper (above target)
const BLUE_BELOW: iced::Color = color!(0x93C5E8);
const BLUE_ABOVE: iced::Color = color!(0x1A6DAA);

impl App {
    fn new() -> Self {
        let actuals: Vec<f64> = COUNTRIES.iter().map(|c| c.actual).collect();
        let targets: Vec<f64> = COUNTRIES.iter().map(|c| c.target).collect();
        let series = bar(actuals)
            .with_labels(Position::Above + (|v: f64| format!("${v:.0}M")))
            .color_by(
                encoding::key(move |i, d| if d.y >= targets[i] { "above" } else { "below" })
                    .manual([("above", BLUE_ABOVE), ("below", BLUE_BELOW)]),
            );

        let chart_bars = bars([series]).horizontal();

        // Per-bar target ticks: (category_index, target_value)
        let tick_points: Vec<(f64, f64)> = COUNTRIES
            .iter()
            .enumerate()
            .map(|(i, c)| (i as f64, c.target))
            .collect();

        let target_ticks = tick(tick_points).vertical().color(color!(0xD4A843)).width(2.0);

        let marks: Vec<Mark> = vec![chart_bars.into(), target_ticks.into()];
        let labels: Vec<&str> = COUNTRIES.iter().map(|c| c.name).collect();

        Self {
            data: data(marks)
                .title("Q1 Revenue vs. Target — Latin America")
                .x_axis_labels(|v: f64| format!("${v:.0}M"))
                .y_axis_labels(labels),
            design: Design,
        }
    }

    fn view(&self) -> Element<'_, Message> {
        center(hyozu::chart(&self.data).design(&self.design).padding(40))
            .padding(20)
            .into()
    }

    fn update(&mut self, _: Message) -> Task<Message> {
        Task::none()
    }
}

// Corporate-style palette (inspired by custom_design Ocean)
#[derive(Clone)]
struct Design;

impl hyozu::Design for Design {
    fn background_color(&self) -> iced::Color {
        color!(0xF2F6FA)
    }

    fn text_color(&self) -> hyozu::Color {
        color!(0x1A334D).into()
    }

    fn text_pair(&self) -> hyozu::Pair {
        hyozu::Pair::new(color!(0x1A334D), color!(0xE8EEF4))
    }

    fn data_colors(&self) -> Vec<hyozu::Color> {
        vec![
            color!(0x0078BF).into(),
            color!(0x00A6D9).into(),
            color!(0x33CCCC).into(),
            color!(0x66E6B3).into(),
        ]
    }

    fn divider_color(&self) -> hyozu::Color {
        color!(0x1A334D, 0.12).into()
    }

    fn axis_color(&self) -> hyozu::Color {
        color!(0x1A334D, 0.6).into()
    }

    fn font(&self) -> Font {
        Font::default()
    }

    fn font_size(&self) -> f32 {
        12.0
    }
}
