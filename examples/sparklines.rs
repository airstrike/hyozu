use iced::Alignment::*;
use iced::widget::{Container, center, container, scrollable, table, text};
use iced::{Center, Color, Element, Fill, Font, Theme, color, font};

use hyozu::sparkline;
use hyozu::sparkline::Highlight;

const BOLD: Font = Font {
    weight: font::Weight::Bold,
    ..Font::DEFAULT
};

pub fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .title("hyozu • sparklines")
        .theme(Theme::Light)
        .default_font(Font::new("Geist"))
        .window_size([900.0, 500.0])
        .centered()
        .run()
}

struct App {
    products: Vec<Product>,
}

#[derive(Debug, Clone)]
enum Message {}

impl App {
    fn new() -> Self {
        Self {
            products: Product::list(),
        }
    }

    fn update(&mut self, _message: Message) {}

    fn view(&self) -> Element<'_, Message> {
        let columns = [
            table::column(header("PRODUCT"), |p: &Product| cell(&p.name, p.is_total)),
            table::column(header("NPW ($M)"), |p: &Product| {
                cell(format!("{:.1}", p.npw), p.is_total)
            })
            .align_x(End),
            table::column(header("GROWTH YOY"), |p: &Product| {
                signed_pct(p.growth_yoy as f64, 0, p.is_total)
            })
            .align_x(End),
            table::column(header("LOSS RATIO"), |p: &Product| {
                ratio_cell(p.loss_ratio, 45.0, 95.0, p.is_total)
            })
            .align_x(End),
            table::column(header("COMBINED"), |p: &Product| {
                ratio_cell(p.combined, 85.0, 130.0, p.is_total)
            })
            .align_x(End),
            table::column(header("RATE CHG."), |p: &Product| signed_pct(p.rate_chg, 1, p.is_total)).align_x(End),
            table::column(header("LOSS RATIO\n8Q"), |p: &Product| -> Element<'_, Message> {
                if p.is_total || p.loss_ratio_8q.is_empty() {
                    center("").into()
                } else {
                    sparkline(&p.loss_ratio_8q)
                        .width(80)
                        .color(color!(0x37474F))
                        .stroke_width(1.5)
                        .highlight(Highlight::Last, color!(0x37474F))
                        .into()
                }
            })
            .center()
            .width(100),
        ];

        let table = Element::from(
            table(columns, &self.products)
                .padding_x(12.0)
                .padding_y(8.0)
                .separator_x(0.0)
                .separator_y(1.0),
        );

        center(scrollable(container(table).padding(1).style(container::bordered_box)).spacing(10)).into()
    }
}

fn header<'a>(label: &str) -> Element<'a, Message> {
    container(
        text(label.to_string())
            .font(BOLD)
            .size(10)
            .align_x(Center)
            .style(|_: &Theme| text::Style {
                color: Some(color!(0x888888)),
            }),
    )
    .padding([2, 5])
    .center_y(Fill)
    .into()
}

fn cell_font(is_total: bool) -> Font {
    if is_total { BOLD } else { Font::DEFAULT }
}

fn cell<'a>(content: impl ToString, is_total: bool) -> Element<'a, Message> {
    container(text(content.to_string()).font(cell_font(is_total)).size(13))
        .center_y(Fill)
        .padding([2, 5])
        .into()
}

fn signed_pct<'a>(value: f64, decimals: usize, is_total: bool) -> Element<'a, Message> {
    let (sign, color) = if value >= 0.0 {
        ("+", color!(0x2E7D32))
    } else {
        ("", color!(0xC62828))
    };
    container(
        text(format!("{sign}{value:.decimals$}%"))
            .font(cell_font(is_total))
            .size(13)
            .align_x(End)
            .color(color),
    )
    .padding([2, 5])
    .center_y(Fill)
    .into()
}

fn ratio_cell<'a>(value: f64, low: f64, high: f64, is_total: bool) -> Container<'a, Message> {
    let bg = ratio_color(value, low, high);
    center(
        text(format!("{value:.1}%"))
            .font(cell_font(is_total))
            .size(13)
            .align_x(End),
    )
    .padding([2, 5])
    .align_x(End)
    .style(move |_| container::background(bg))
}

fn ratio_color(value: f64, low: f64, high: f64) -> Color {
    let t = ((value - low) / (high - low)).clamp(0.0, 1.0) as f32;
    // Green → transparent → Red, fading alpha through the midpoint
    let green = Color::from_rgb8(180, 230, 180);
    let red = Color::from_rgb8(245, 190, 190);
    let s = (t - 0.5).abs() * 2.0; // 0 at midpoint, 1 at extremes
    let base = if t <= 0.5 { green } else { red };
    Color { a: s * 0.8, ..base }
}

struct Product {
    name: String,
    npw: f64,
    growth_yoy: i32,
    loss_ratio: f64,
    combined: f64,
    rate_chg: f64,
    loss_ratio_8q: Vec<f64>,
    is_total: bool,
}

impl Product {
    fn list() -> Vec<Self> {
        vec![
            Product {
                name: "Property".into(),
                npw: 42.3,
                growth_yoy: 9,
                loss_ratio: 78.4,
                combined: 107.2,
                rate_chg: 11.4,
                loss_ratio_8q: vec![72.0, 74.0, 71.0, 76.0, 79.0, 81.0, 78.0, 78.4],
                is_total: false,
            },
            Product {
                name: "Marine".into(),
                npw: 28.7,
                growth_yoy: 5,
                loss_ratio: 62.1,
                combined: 98.7,
                rate_chg: 6.2,
                loss_ratio_8q: vec![58.0, 60.0, 59.0, 61.0, 63.0, 62.0, 61.5, 62.1],
                is_total: false,
            },
            Product {
                name: "Energy".into(),
                npw: 19.1,
                growth_yoy: 2,
                loss_ratio: 55.8,
                combined: 91.4,
                rate_chg: 4.8,
                loss_ratio_8q: vec![60.0, 58.0, 54.0, 52.0, 53.0, 55.0, 56.0, 55.8],
                is_total: false,
            },
            Product {
                name: "Liability".into(),
                npw: 22.4,
                growth_yoy: 1,
                loss_ratio: 71.3,
                combined: 108.9,
                rate_chg: 3.1,
                loss_ratio_8q: vec![68.0, 69.0, 70.0, 70.5, 71.0, 71.0, 71.2, 71.3],
                is_total: false,
            },
            Product {
                name: "Commercial".into(),
                npw: 18.6,
                growth_yoy: -4,
                loss_ratio: 64.7,
                combined: 101.2,
                rate_chg: 1.9,
                loss_ratio_8q: vec![62.0, 63.0, 63.5, 64.0, 64.0, 64.5, 64.5, 64.7],
                is_total: false,
            },
            Product {
                name: "Auto".into(),
                npw: 14.2,
                growth_yoy: -11,
                loss_ratio: 89.6,
                combined: 128.4,
                rate_chg: -1.5,
                loss_ratio_8q: vec![75.0, 78.0, 82.0, 86.0, 90.0, 91.0, 90.0, 89.6],
                is_total: false,
            },
            Product {
                name: "A&H".into(),
                npw: 7.8,
                growth_yoy: 14,
                loss_ratio: 48.2,
                combined: 87.6,
                rate_chg: 0.7,
                loss_ratio_8q: vec![55.0, 52.0, 50.0, 48.0, 47.0, 47.5, 48.0, 48.2],
                is_total: false,
            },
            Product {
                name: "Credit Lines".into(),
                npw: 4.0,
                growth_yoy: -6,
                loss_ratio: 57.3,
                combined: 95.1,
                rate_chg: -0.4,
                loss_ratio_8q: vec![56.0, 57.0, 58.0, 57.5, 57.0, 57.0, 57.2, 57.3],
                is_total: false,
            },
            Product {
                name: "Total".into(),
                npw: 157.1,
                growth_yoy: 1,
                loss_ratio: 72.4,
                combined: 115.9,
                rate_chg: 3.2,
                loss_ratio_8q: vec![],
                is_total: true,
            },
        ]
    }
}
