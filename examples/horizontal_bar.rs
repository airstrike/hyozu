use iced::widget::center;
use iced::{Element, Theme};

use hyozu::{bar, bars, data, encoding};

const REGIONS: [&str; 5] = ["North", "South", "East", "West", "Central"];

pub fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .window_size([600.0, 400.0])
        .title("hyozu • horizontal bars")
        .run()
}

struct App {
    data: hyozu::Data,
}

pub type Message = ();

impl App {
    fn new() -> Self {
        Self {
            data: data(bars([bar([120, 190, 150, 80, 210]).color_by(encoding::key(|i, _| REGIONS[i]))]).horizontal())
                .palette(hyozu::Palette::Categorical)
                .title("Sales by Region")
                .x_axis_labels(|v: f64| format!("{:.0}", v))
                .y_axis_labels(REGIONS),
        }
    }

    fn view(&self) -> Element<'_, Message> {
        center(hyozu::chart(&self.data).design(&Theme::Light))
            .padding(20)
            .into()
    }

    fn update(&mut self, _: Message) {}
}
