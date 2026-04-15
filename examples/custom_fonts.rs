//! Per-element typography with `text::Style`.
//!
//! Every chart text element — title, axis labels, data labels, legend —
//! accepts its own `text::Style` override. Any field left unset falls back
//! to the active `Design`'s defaults, so you can tweak just one thing
//! without rebuilding the whole style.

use iced::widget::{center, column, row, text};
use iced::{Font, Length, Task, Theme, font};

use hyozu::{LegendConfig, Palette, axis, bar, bars, chart, data, text as chart_text};

const MONTHS: [&str; 6] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun"];

pub fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .title("hyozu • custom fonts")
        .window_size([900.0, 620.0])
        .run()
}

struct App {
    sales: hyozu::Data,
}

pub type Message = ();

impl App {
    fn new() -> Self {
        // Bold italic monospace data labels, layered on top of the theme
        // default. `with_font` now takes `impl Into<Font>`, so passing
        // `Font::MONOSPACE` or a bare `"Inter"` both work.
        let label = bar::label::Label::default()
            .with_position(bar::label::Position::Above)
            .with_font(Font::MONOSPACE)
            .with_weight(font::Weight::Bold)
            .with_style(font::Style::Italic);

        let sales = data(
            bars([bar([1200, 1900, 1500, 2200, 1800, 2400]).with_name("Revenue")])
                .corner_radius(6.0)
                .data_labels(label),
        )
        .palette(Palette::Categorical)
        .title("Quarterly Revenue")
        // Big monospace title — unset fields fall back to the theme's
        // `title_text` default, so only font + size + weight are
        // overridden here.
        .title_style(
            chart_text::Style::new()
                .font(Font::MONOSPACE)
                .size(22.0)
                .weight(font::Weight::Semibold),
        )
        .x_axis_labels(axis::Placement::BetweenTicks + MONTHS)
        // Semibold, 14px x-axis — default family.
        .x_axis(|a| a.with_label_size(14.0).with_label_weight(font::Weight::Semibold))
        // Light italic y-axis — just because we can.
        .y_axis(|a| {
            a.show_grid(true)
                .with_label_size(11.0)
                .with_label_weight(font::Weight::Light)
                .with_label_italic()
        })
        // Monospace 11px legend. `font()` takes `impl Into<Font>` too.
        .legend(LegendConfig::below().font(Font::MONOSPACE).font_size(11.0));

        Self { sales }
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
            info_row("Title", "Font::MONOSPACE · 22 px · Semibold"),
            info_row("X axis", "default family · 14 px · Semibold"),
            info_row("Y axis", "default family · 11 px · Light italic"),
            info_row("Data labels", "Font::MONOSPACE · Bold italic"),
            info_row("Legend", "Font::MONOSPACE · 11 px"),
        ]
        .spacing(2);

        let chart = chart(&self.sales).design(&Theme::TokyoNightLight).padding(20);

        center(column![header, chart, key].spacing(16)).padding(20).into()
    }

    fn update(&mut self, _: Message) -> Task<Message> {
        Task::none()
    }
}

fn info_row<'a>(label: &'a str, value: &'a str) -> iced::Element<'a, Message> {
    row![
        text(label).size(12).width(Length::Fixed(100.0)),
        text(value).size(12).font(Font::MONOSPACE).color([0.4, 0.4, 0.4]),
    ]
    .spacing(12)
    .into()
}
