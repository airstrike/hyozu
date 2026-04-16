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
//! is rebuilt with the new families so the text snaps to the real typeface.
//!
//! Note: we intentionally don't set a font family until it has finished
//! loading. iced's text buffer cache keys on (content, font, size, …) but
//! not the font-system version, so a buffer rendered with a fallback font
//! would persist even after the real font is registered. By deferring the
//! family name until the font is ready, we avoid that stale-cache path
//! entirely.

use std::collections::HashSet;

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
    loaded: HashSet<&'static str>,
    status: String,
}

#[derive(Debug, Clone)]
enum Message {
    FontLoaded(&'static str, Result<(), String>),
}

impl App {
    fn new() -> (Self, Task<Message>) {
        let mut app = Self {
            sales: hyozu::Data::default(),
            loaded: HashSet::new(),
            status: "Loading fonts\u{2026}".into(),
        };
        app.sales = build_data(&app.loaded);

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
                self.loaded.insert(name);
                self.sales = build_data(&self.loaded);
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

/// (Re)build chart data, setting font families only for fonts that
/// have already been registered with iced. This avoids poisoning
/// iced's text buffer cache with fallback-rendered entries.
fn build_data(loaded: &HashSet<&str>) -> hyozu::Data {
    let title_font = loaded.contains(TITLE_FAMILY).then_some(TITLE_FAMILY);
    let axis_font = loaded.contains(AXIS_FAMILY).then_some(AXIS_FAMILY);
    let mono_font = loaded.contains(MONO_FAMILY).then_some(MONO_FAMILY);

    // Data labels — only set family once the font is ready.
    let mut label = bar::label::Label::default()
        .with_position(bar::label::Position::Above)
        .with_weight(font::Weight::Bold)
        .with_style(font::Style::Italic);
    if let Some(f) = mono_font {
        label = label.with_font(f);
    }

    let mut title_style = chart_text::Style::new().size(26.0).weight(font::Weight::Semibold);
    if let Some(f) = title_font {
        title_style = title_style.font(f);
    }

    data(
        bars([bar([1200, 1900, 1500, 2200, 1800, 2400]).with_name("Revenue")])
            .corner_radius(6.0)
            .data_labels(label),
    )
    .palette(Palette::Categorical)
    .title("Quarterly Revenue")
    .title_style(title_style)
    .x_axis_labels(axis::Placement::BetweenTicks + MONTHS)
    .x_axis(|a| {
        let a = a.with_label_size(14.0).with_label_weight(font::Weight::Semibold);
        if let Some(f) = axis_font { a.with_font(f) } else { a }
    })
    .y_axis(|a| {
        let a = a
            .show_grid(true)
            .with_label_size(11.0)
            .with_label_weight(font::Weight::Light)
            .with_label_italic();
        if let Some(f) = axis_font { a.with_font(f) } else { a }
    })
    .legend({
        let l = LegendConfig::below().font_size(11.0);
        if let Some(f) = mono_font { l.font(f) } else { l }
    })
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
