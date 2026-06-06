use iced::widget::center;
use iced::{Element, Font, Task, Theme, color};

use hyozu::mark::bar::label::Position;
use hyozu::{Tooltip, bar, bars, data, encoding};

pub fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .window_size([720.0, 420.0])
        .title("hyozu • horizontal bars")
        .run()
}

struct App {
    pipeline: hyozu::Data,
}

pub type Message = ();

// Stages bottom-to-top: horizontal bars place the first datum at the bottom,
// so listing Prospect last puts it on top.
const STAGES: [&str; 5] = ["Closed", "Negotiate", "Proposal", "Qualify", "Prospect"];

// Deep-to-light ramp keyed by stage name (order-independent).
const RAMP: [(&str, iced::Color); 5] = [
    ("Prospect", color!(0x111318)),
    ("Qualify", color!(0x3B3F4A)),
    ("Proposal", color!(0x4B49E0)),
    ("Negotiate", color!(0x8E8BF0)),
    ("Closed", color!(0xBFC0F7)),
];

impl App {
    fn new() -> Self {
        let series = bar([0.8, 1.5, 2.5, 3.9, 5.6])
            .with_labels(Position::End + (|v: f64| format!("{v:.1}")))
            .color_by(encoding::key(|i, _| STAGES[i]).manual(RAMP));

        Self {
            pipeline: data(bars([series]).horizontal().with_size(0.82))
                .title("Pipeline by Stage")
                // Category labels on the left, no spine and no grid. Hiding
                // the line also drops its ticks, and the hidden value spine
                // lets the bars round their baseline end rather than sitting
                // flush against a line. The value axis is dropped entirely.
                .y_axis(|a| a.labels(STAGES).show_line(false).show_grid(false))
                .x_axis(|a| a.none())
                // Hover annotation carries the stage name alongside the value.
                .tooltip(Tooltip::default().format(|e| {
                    let stage = STAGES.get(e.x as usize).copied().unwrap_or_default();
                    format!("{stage}   {:.1}", e.y)
                })),
        }
    }

    fn view(&self) -> Element<'_, Message> {
        center(
            hyozu::chart(&self.pipeline)
                .design(&Design)
                .padding(30)
                // A faint `track` rail behind each bar plus a moderate
                // `corners` radius gives the rounded-rectangle "fills to full
                // length" look (Recharts' `<Bar background>`).
                .style(|d| hyozu::chart::Style {
                    corners: 8.0.into(),
                    track: Some(color!(0x000000, 0.05)),
                    ..hyozu::chart::default(d)
                }),
        )
        .padding(20)
        .into()
    }

    fn update(&mut self, _: Message) -> Task<Message> {
        Task::none()
    }
}

// iced's Light theme with a larger base font across the whole chart.
#[derive(Clone)]
struct Design;

impl hyozu::Design for Design {
    fn background_color(&self) -> iced::Color {
        Theme::Light.background_color()
    }

    fn text_color(&self) -> hyozu::Color {
        Theme::Light.text_color()
    }

    fn text_pair(&self) -> hyozu::Pair {
        Theme::Light.text_pair()
    }

    fn seed(&self) -> hyozu::palette::Seed {
        // Disambiguate from iced::Theme's own inherent `seed()`.
        <Theme as hyozu::Design>::seed(&Theme::Light)
    }

    #[allow(deprecated)]
    fn data_colors(&self) -> Vec<hyozu::Color> {
        Theme::Light.data_colors()
    }

    fn divider_color(&self) -> hyozu::Color {
        Theme::Light.divider_color()
    }

    fn axis_color(&self) -> hyozu::Color {
        Theme::Light.axis_color()
    }

    fn font(&self) -> Font {
        Theme::Light.font()
    }

    fn font_size(&self) -> f32 {
        15.0
    }

    fn title_text(&self) -> hyozu::text::Style {
        hyozu::text::Style::new().font(self.font()).size(20.0)
    }
}
