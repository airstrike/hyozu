//! Per-element typography with `text::Style`.
//!
//! Every chart text element — title, axis labels, data labels, legend —
//! accepts its own `text::Style` override. Any field left unset falls back
//! to the active `Design`'s defaults.
//!
//! At startup we pull three Google Fonts via `fount` and register them with
//! iced. The chart references the families immediately — once a font
//! finishes loading, `Data::invalidate()` forces a re-measure and the text
//! snaps to the real typeface.

use iced::widget::{center, column, row, text};
use iced::{Font, Length, Task, Theme};

use hyozu::axis::Placement::BetweenTicks;
use hyozu::bar::label::Above;
use hyozu::text::{Bold, Italic, Light, Semibold, font};
use hyozu::{Palette, bar, bars, chart, data, legend};

const MONTHS: [&str; 6] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun"];

pub fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .title("hyozu • custom fonts")
        .window_size([900.0, 640.0])
        .run()
}

struct App {
    sales: hyozu::Data,
    status: String,
}

#[derive(Debug, Clone)]
enum Message {
    FontLoaded(&'static str, Result<(), String>),
}

impl App {
    fn new() -> (Self, Task<Message>) {
        let sales = data(
            bars([bar([1200, 1900, 1500, 2200, 1800, 2400]).with_name("Revenue")])
                .corner_radius(6.0)
                .data_labels(Above + font("JetBrains Mono") + Bold + Italic),
        )
        .palette(Palette::Categorical)
        .title("Quarterly Revenue")
        .title_style(font("Playfair Display") + 26.0 + Semibold)
        .x_axis_labels(BetweenTicks + MONTHS + font("Inter") + 14.0 + Semibold)
        .y_axis(|a| a.show_grid(true).labels(font("Inter") + 11.0 + Light + Italic))
        .legend(legend::Config::below().font("JetBrains Mono").size(11.0));

        let init = Task::batch([
            load_family("Playfair Display"),
            load_family("Inter"),
            load_family("JetBrains Mono"),
        ]);

        (
            Self {
                sales,
                status: "Loading fonts\u{2026}".into(),
            },
            init,
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::FontLoaded(name, Ok(())) => {
                self.status = format!("Loaded {name}");
                self.sales.invalidate();
            }
            Message::FontLoaded(name, Err(e)) => {
                self.status = format!("Failed to load {name}: {e}");
            }
        }
        Task::none()
    }

    fn view(&self) -> iced::Element<'_, Message> {
        let header = column![
            text("Custom fonts in hyozu").size(24),
            text("Title, axes, data labels, and legend each carry their own text::Style.")
                .size(13)
                .color([0.5, 0.5, 0.5]),
        ]
        .spacing(4);

        let key = column![
            info_row("Title", "Playfair Display \u{b7} 26 px \u{b7} Semibold"),
            info_row("X axis", "Inter \u{b7} 14 px \u{b7} Semibold"),
            info_row("Y axis", "Inter \u{b7} 11 px \u{b7} Light italic"),
            info_row("Data labels", "JetBrains Mono \u{b7} Bold italic"),
            info_row("Legend", "JetBrains Mono \u{b7} 11 px"),
        ]
        .spacing(2);

        let status = text(&self.status).size(11).color([0.5, 0.5, 0.5]);
        let chart = chart(&self.sales).design(&Theme::TokyoNightLight).padding(20);

        center(column![header, chart, key, status].spacing(16))
            .padding(20)
            .into()
    }
}

fn load_family(name: &'static str) -> Task<Message> {
    Task::future(async move { fount::google::load(name, None).await }).then(move |result| match result {
        Ok(variants) => {
            let register = variants.into_iter().map(|bytes| {
                iced::font::load(bytes).map(move |r: Result<(), iced::font::Error>| r.map_err(|e| format!("{e:?}")))
            });
            Task::batch(register)
                .collect()
                .map(move |results: Vec<Result<(), String>>| {
                    let combined = results.into_iter().find(Result::is_err).unwrap_or(Ok(()));
                    Message::FontLoaded(name, combined)
                })
        }
        Err(e) => Task::done(Message::FontLoaded(name, Err(format!("{e:?}")))),
    })
}

fn info_row<'a>(label: &'a str, value: &'a str) -> iced::Element<'a, Message> {
    row![
        text(label).size(12).width(Length::Fixed(100.0)),
        text(value).size(12).font(Font::MONOSPACE).color([0.4, 0.4, 0.4]),
    ]
    .spacing(12)
    .into()
}
