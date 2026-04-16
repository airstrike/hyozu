//! Per-element typography with `text::Style`.
//!
//! Every chart text element — title, axis labels, data labels, legend —
//! accepts its own `text::Style` override. Any field left unset falls back
//! to the active `Design`'s defaults, so you can tweak just one thing
//! without rebuilding the whole style.
//!
//! At startup we pull three Google Fonts via the `fount` crate — Playfair
//! Display for the title, Inter for the axes, JetBrains Mono for data
//! labels and legend — and register them with iced. Once loaded, the chart
//! text snaps to the new families on the next redraw.

use iced::widget::{center, column, row, text};
use iced::{Font, Length, Task, Theme, font};

use hyozu::{LegendConfig, Palette, axis, bar, bars, chart, data, text as chart_text};

const MONTHS: [&str; 6] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun"];

const TITLE_FAMILY: &str = "Playfair Display";
const AXIS_FAMILY: &str = "Inter";
const MONO_FAMILY: &str = "JetBrains Mono";

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
    FontLoaded(String, Result<(), String>),
}

impl App {
    fn new() -> (Self, Task<Message>) {
        // Bold italic data labels in JetBrains Mono. `with_font` takes
        // `impl Into<Font>`, so a bare `&str` works — iced looks the
        // family up by name once it's been registered.
        let label = bar::label::Label::default()
            .with_position(bar::label::Position::Above)
            .with_font(MONO_FAMILY)
            .with_weight(font::Weight::Bold)
            .with_style(font::Style::Italic);

        let sales = data(
            bars([bar([1200, 1900, 1500, 2200, 1800, 2400]).with_name("Revenue")])
                .corner_radius(6.0)
                .data_labels(label),
        )
        .palette(Palette::Categorical)
        .title("Quarterly Revenue")
        // Playfair Display title. Any field unset on the Style falls back
        // to `Design::title_text` (theme font at 16 px).
        .title_style(
            chart_text::Style::new()
                .font(TITLE_FAMILY)
                .size(26.0)
                .weight(font::Weight::Semibold),
        )
        .x_axis_labels(axis::Placement::BetweenTicks + MONTHS)
        // Inter semibold x-axis.
        .x_axis(|a| {
            a.with_font(AXIS_FAMILY)
                .with_label_size(14.0)
                .with_label_weight(font::Weight::Semibold)
        })
        // Inter light italic y-axis.
        .y_axis(|a| {
            a.show_grid(true)
                .with_font(AXIS_FAMILY)
                .with_label_size(11.0)
                .with_label_weight(font::Weight::Light)
                .with_label_italic()
        })
        // JetBrains Mono 11 px legend.
        .legend(LegendConfig::below().font(MONO_FAMILY).font_size(11.0));

        let app = Self {
            sales,
            status: "Loading fonts…".into(),
        };

        // Fire off three Google Fonts downloads in parallel. Each family
        // can ship multiple variant files (regular / bold / italic / …),
        // so `fount::google::load` returns a `Vec<Vec<u8>>` that we feed
        // into `iced::font::load` one variant at a time.
        let init = Task::batch([
            load_family(TITLE_FAMILY),
            load_family(AXIS_FAMILY),
            load_family(MONO_FAMILY),
        ]);

        (app, init)
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
            info_row("Title", "Playfair Display · 26 px · Semibold"),
            info_row("X axis", "Inter · 14 px · Semibold"),
            info_row("Y axis", "Inter · 11 px · Light italic"),
            info_row("Data labels", "JetBrains Mono · Bold italic"),
            info_row("Legend", "JetBrains Mono · 11 px"),
        ]
        .spacing(2);

        let status = text(&self.status).size(11).color([0.5, 0.5, 0.5]);

        let chart = chart(&self.sales).design(&Theme::TokyoNightLight).padding(20);

        center(column![header, chart, key, status].spacing(16))
            .padding(20)
            .into()
    }
}

/// Download a Google Fonts family via `fount` and register each variant
/// with iced. Emits one `FontLoaded` message per family (success once
/// every variant has been registered, or the first error).
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
                    Message::FontLoaded(name.into(), combined)
                })
        }
        Err(e) => Task::done(Message::FontLoaded(name.into(), Err(format!("{e:?}")))),
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
