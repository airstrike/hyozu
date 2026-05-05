//! Sundae Drive — fictional national ice cream chain.
//!
//! Demo of two geo-aware marks rendered against the same `geo::Plane`:
//!
//! 1. A US-states **choropleth** colored by per-capita monthly sales.
//! 2. An **`Xy::on_geo()` bubble overlay** for shop locations, sized by
//!    revenue and projected through the same Mercator plane.
//!
//! Run with: `cargo run --package ice_cream`

use std::sync::Arc;

use hyozu::ProjectionKind;
use hyozu::data::legend;
use hyozu::geo::{self, GeoData, MapScope};
use iced::widget::{center, column, container, text};
use iced::{Color, Element, Fill, Font, Task, Theme, color};

/// Hex of the choropleth's light-mode ocean fill (`#F2F7FA`). Used as
/// the chart widget's own background so its frame blends with the
/// rendered ocean rather than fighting it with a white border.
const OCEAN_FILL: Color = color!(0xF2F7FA);

/// Warm contrast color for the bubble overlay so flagship stores read
/// against the blue choropleth instead of disappearing into it.
const BUBBLE_FILL: Color = color!(0xE07A2D);

const STATES_URL: &str = "https://raw.githubusercontent.com/nvkelso/natural-earth-vector/master/geojson/ne_110m_admin_1_states_provinces.geojson";

// ── Data ───────────────────────────────────────────────────────────

/// Monthly state-level revenue in thousands of USD. Postal codes match
/// Natural Earth's `postal` property on US states.
///
/// The northern plains and intermountain west — MT, WY, ND, SD, NE,
/// KS, ID — are intentionally absent so the choropleth's
/// `FeatureState::Missing` rendering (theme `missing_fill`) is visible
/// alongside the gradient-filled states.
fn state_revenue() -> Vec<(&'static str, f64)> {
    vec![
        ("CA", 1450.0),
        ("TX", 1280.0),
        ("FL", 1100.0),
        ("NY", 950.0),
        ("PA", 720.0),
        ("IL", 680.0),
        ("OH", 590.0),
        ("GA", 580.0),
        ("NC", 510.0),
        ("MI", 470.0),
        ("NJ", 460.0),
        ("VA", 440.0),
        ("WA", 430.0),
        ("AZ", 420.0),
        ("MA", 400.0),
        ("TN", 380.0),
        ("IN", 370.0),
        ("MO", 350.0),
        ("MD", 340.0),
        ("WI", 330.0),
        ("CO", 320.0),
        ("MN", 310.0),
        ("SC", 300.0),
        ("AL", 290.0),
        ("LA", 280.0),
        ("KY", 270.0),
        ("OR", 260.0),
        ("OK", 250.0),
        ("CT", 240.0),
        ("UT", 230.0),
        ("IA", 220.0),
        ("NV", 210.0),
        ("AR", 200.0),
        ("MS", 190.0),
        ("NM", 180.0),
        ("WV", 100.0),
        ("NH", 95.0),
        ("ME", 85.0),
        ("RI", 80.0),
        ("DE", 75.0),
        ("VT", 60.0),
    ]
}

/// A flagship store in a major metro. `value` is monthly revenue in
/// thousands of USD; the bubble overlay sizes markers by it.
struct Shop {
    label: &'static str,
    lon: f64,
    lat: f64,
    revenue_k: f64,
}

fn shops() -> Vec<Shop> {
    vec![
        Shop {
            label: "Los Angeles",
            lon: -118.24,
            lat: 34.05,
            revenue_k: 412.0,
        },
        Shop {
            label: "New York",
            lon: -74.00,
            lat: 40.71,
            revenue_k: 388.0,
        },
        Shop {
            label: "Chicago",
            lon: -87.63,
            lat: 41.88,
            revenue_k: 245.0,
        },
        Shop {
            label: "Houston",
            lon: -95.37,
            lat: 29.76,
            revenue_k: 268.0,
        },
        Shop {
            label: "Phoenix",
            lon: -112.07,
            lat: 33.45,
            revenue_k: 198.0,
        },
        Shop {
            label: "Miami",
            lon: -80.19,
            lat: 25.76,
            revenue_k: 312.0,
        },
        Shop {
            label: "Dallas",
            lon: -96.80,
            lat: 32.78,
            revenue_k: 224.0,
        },
        Shop {
            label: "Seattle",
            lon: -122.33,
            lat: 47.61,
            revenue_k: 176.0,
        },
        Shop {
            label: "Denver",
            lon: -104.99,
            lat: 39.74,
            revenue_k: 152.0,
        },
        Shop {
            label: "Atlanta",
            lon: -84.39,
            lat: 33.75,
            revenue_k: 209.0,
        },
        Shop {
            label: "Boston",
            lon: -71.06,
            lat: 42.36,
            revenue_k: 187.0,
        },
        Shop {
            label: "Las Vegas",
            lon: -115.14,
            lat: 36.17,
            revenue_k: 234.0,
        },
        Shop {
            label: "Minneapolis",
            lon: -93.27,
            lat: 44.98,
            revenue_k: 121.0,
        },
        Shop {
            label: "Nashville",
            lon: -86.78,
            lat: 36.16,
            revenue_k: 143.0,
        },
        Shop {
            label: "New Orleans",
            lon: -90.07,
            lat: 29.95,
            revenue_k: 168.0,
        },
    ]
}

// ── App ────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
enum Message {
    GeoLoaded(Result<GeoData, String>),
    ChartAction(#[allow(dead_code)] hyozu::Action),
}

struct App {
    states: Option<Arc<GeoData>>,
    chart_data: hyozu::Data,
    error: Option<String>,
}

impl App {
    fn new() -> (Self, Task<Message>) {
        let app = Self {
            states: None,
            chart_data: hyozu::Data::default(),
            error: None,
        };
        let task = Task::perform(fetch_states(), Message::GeoLoaded);
        (app, task)
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::GeoLoaded(Ok(geo)) => {
                self.states = Some(Arc::new(geo));
                self.chart_data = self.build_chart_data();
                Task::none()
            }
            Message::GeoLoaded(Err(err)) => {
                self.error = Some(err);
                Task::none()
            }
            Message::ChartAction(_) => Task::none(),
        }
    }

    fn view(&self) -> Element<'_, Message> {
        if let Some(err) = &self.error {
            return center(text(format!("Failed to load: {err}")).size(18))
                .width(Fill)
                .height(Fill)
                .into();
        }
        if self.states.is_none() {
            return center(text("Fetching US states…").size(18))
                .width(Fill)
                .height(Fill)
                .into();
        }

        let chart = hyozu::chart(&self.chart_data)
            .height(Fill)
            .on_action(Message::ChartAction)
            .style(|_design| hyozu::chart::Style {
                background: Some(OCEAN_FILL),
                ..hyozu::chart::Style::default()
            });
        column![header(), container(chart).padding(8).width(Fill).height(Fill)]
            .width(Fill)
            .height(Fill)
            .into()
    }

    /// Builds the Data: choropleth (states colored by revenue) + bubble
    /// overlay (shops sized by revenue), both projected through the
    /// same `Data::geo(...)` Mercator plane.
    fn build_chart_data(&self) -> hyozu::Data {
        let Some(states) = self.states.as_ref() else {
            return hyozu::Data::default();
        };

        let choropleth = hyozu::choropleth(state_revenue())
            .scheme(hyozu::palette::Scheme::Blues)
            .legend_title("Monthly revenue ($K)")
            .legend(legend::Config::overlay(legend::Anchor::BottomRight).orientation(legend::Orientation::Vertical))
            .linear();

        let shops_data = shops();
        let shop_points: Vec<hyozu::MapPoint> = shops_data
            .iter()
            .map(|s| hyozu::map_point(s.lat, s.lon, s.revenue_k as f32).label(s.label))
            .collect();

        // Build the bubble overlay manually instead of going through
        // `bubble_map_with_labels` so the warm bubble color is applied
        // directly. The helper covers the simple no-customization case.
        let bubbles = hyozu::bubble_map(shop_points)
            .with_name("Flagship store revenue ($K)")
            .color(BUBBLE_FILL);
        let label_items: Vec<hyozu::TextItem> = shops_data
            .iter()
            .map(|s| hyozu::TextItem {
                datum: hyozu::Datum::new(s.lon, s.lat),
                label: s.label.to_string(),
            })
            .collect();
        let labels = hyozu::text(label_items)
            .on_geo()
            .offset(0.0, -16.0)
            .align(hyozu::TextAlign::Center)
            .size(11.0);

        hyozu::data(vec![
            hyozu::Mark::Choropleth(choropleth),
            hyozu::Mark::Xy(bubbles),
            hyozu::Mark::Text(labels),
        ])
        .geo(states.clone(), MapScope::UnitedStates, ProjectionKind::Mercator)
        .title("Sundae Drive — Monthly Performance, CONUS")
    }
}

fn header() -> Element<'static, Message> {
    let title = text("Sundae Drive").size(22);
    let subtitle = text("Monthly performance · contiguous US")
        .size(13)
        .color(color!(0x666666));
    container(column![title, subtitle].spacing(2)).padding([10, 16]).into()
}

// ── Async fetch ────────────────────────────────────────────────────

async fn fetch_states() -> Result<GeoData, String> {
    let body = reqwest::get(STATES_URL)
        .await
        .map_err(|e| e.to_string())?
        .text()
        .await
        .map_err(|e| e.to_string())?;
    geo::parse_geojson(&body).map_err(|e| e.to_string())
}

// ── Main ───────────────────────────────────────────────────────────

fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .window_size([1100.0, 720.0])
        .title("Sundae Drive — Sales Map")
        .theme(|_state: &App| Theme::Light)
        .settings(iced::Settings {
            default_text_size: 13.into(),
            default_font: Font::DEFAULT,
            ..Default::default()
        })
        .run()
}
