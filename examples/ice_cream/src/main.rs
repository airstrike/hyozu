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
use hyozu::geo::{self, GeoData, MapScope};
use iced::widget::{center, column, container, text};
use iced::{Element, Fill, Font, Task, Theme, color};

const STATES_URL: &str = "https://raw.githubusercontent.com/nvkelso/natural-earth-vector/master/geojson/ne_110m_admin_1_states_provinces.geojson";

// ── Data ───────────────────────────────────────────────────────────

/// Per-state monthly ice-cream sales per capita, in USD. Postal codes
/// match Natural Earth's `postal` property on US states.
fn sales_per_capita() -> Vec<(&'static str, f64)> {
    vec![
        ("CA", 24.10),
        ("TX", 19.80),
        ("FL", 22.50),
        ("NY", 18.30),
        ("PA", 14.20),
        ("IL", 15.90),
        ("OH", 13.40),
        ("GA", 17.60),
        ("NC", 16.40),
        ("MI", 12.80),
        ("NJ", 17.10),
        ("VA", 15.50),
        ("WA", 16.90),
        ("AZ", 21.30),
        ("MA", 18.90),
        ("TN", 14.70),
        ("IN", 12.30),
        ("MO", 13.10),
        ("MD", 16.80),
        ("WI", 14.60),
        ("CO", 17.20),
        ("MN", 13.80),
        ("SC", 16.10),
        ("AL", 15.20),
        ("LA", 18.10),
        ("KY", 12.10),
        ("OR", 15.30),
        ("OK", 14.40),
        ("CT", 16.30),
        ("UT", 14.90),
        ("IA", 11.70),
        ("NV", 20.40),
        ("AR", 13.50),
        ("MS", 14.80),
        ("KS", 11.90),
        ("NM", 16.60),
        ("NE", 11.40),
        ("ID", 13.20),
        ("WV", 10.80),
        ("NH", 15.70),
        ("ME", 14.30),
        ("MT", 12.40),
        ("RI", 16.70),
        ("DE", 17.40),
        ("SD", 11.20),
        ("ND", 10.90),
        ("VT", 14.10),
        ("WY", 12.60),
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
            .on_action(Message::ChartAction);

        column![header(), container(chart).padding(8).width(Fill).height(Fill)]
            .width(Fill)
            .height(Fill)
            .into()
    }

    /// Builds the Data: choropleth (states colored by sales/cap) +
    /// bubble overlay (shops sized by revenue), both projected through
    /// the same `Data::geo(...)` Mercator plane.
    fn build_chart_data(&self) -> hyozu::Data {
        let Some(states) = self.states.as_ref() else {
            return hyozu::Data::default();
        };

        let choropleth = hyozu::choropleth(sales_per_capita())
            .scheme(hyozu::palette::Scheme::Viridis)
            .legend_title("Monthly sales / capita ($)")
            .linear();

        let shop_points: Vec<hyozu::MapPoint> = shops()
            .into_iter()
            .map(|s| hyozu::map_point(s.lat, s.lon, s.revenue_k as f32).label(s.label))
            .collect();

        let mut marks = vec![hyozu::Mark::Choropleth(choropleth)];
        marks.extend(hyozu::bubble_map_with_labels(
            shop_points,
            "Flagship store revenue ($K)",
        ));

        hyozu::data(marks)
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
