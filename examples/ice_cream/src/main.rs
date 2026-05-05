//! Sundae Drive — fictional national ice cream chain.
//!
//! Demo of two geo-aware marks rendered against the same `geo::Plane`:
//!
//! 1. A US-states **choropleth** colored by each state's monthly
//!    revenue (sum of its flagship stores).
//! 2. An **`Xy::on_geo()` bubble overlay** for individual shop
//!    locations, sized by revenue and projected through the same
//!    Mercator plane.
//!
//! States without a flagship store render in the theme's missing-fill
//! gray, so the chart's coverage and the bubble overlay describe the
//! same business footprint.
//!
//! Run with: `cargo run --package ice_cream`

use std::collections::HashMap;
use std::sync::Arc;

use hyozu::ProjectionKind;
use hyozu::data::legend;
use hyozu::geo::{self, GeoData, MapScope};
use iced::widget::{center, column, container, text};
use iced::{Color, Element, Fill, Font, Task, Theme, color};

/// Warm contrast color for the bubble overlay so flagship stores read
/// against the blue choropleth instead of disappearing into it.
const BUBBLE_FILL: Color = color!(0xE07A2D);

const STATES_URL: &str = "https://raw.githubusercontent.com/nvkelso/natural-earth-vector/master/geojson/ne_110m_admin_1_states_provinces.geojson";

// ── Data ───────────────────────────────────────────────────────────

/// Sums each shop's revenue into its state, so the choropleth covers
/// only states with a flagship store and matches the bubble overlay's
/// footprint (states without shops fall through to `FeatureState::
/// Missing` and render in the theme's missing-fill gray).
fn state_revenue(shops: &[Shop]) -> Vec<(&'static str, f64)> {
    let mut by_state: HashMap<&'static str, f64> = HashMap::new();
    for shop in shops {
        *by_state.entry(shop.state).or_insert(0.0) += shop.revenue_k;
    }
    by_state.into_iter().collect()
}

/// A flagship store in a major metro. `revenue_k` is July 2025 revenue
/// in thousands of USD; the bubble overlay sizes markers by it, and
/// the choropleth aggregates these by `state` per
/// [`state_revenue`].
struct Shop {
    label: &'static str,
    state: &'static str,
    lon: f64,
    lat: f64,
    revenue_k: f64,
}

fn shops() -> Vec<Shop> {
    vec![
        Shop {
            label: "Los Angeles",
            state: "CA",
            lon: -118.24,
            lat: 34.05,
            revenue_k: 412.0,
        },
        Shop {
            label: "New York",
            state: "NY",
            lon: -74.00,
            lat: 40.71,
            revenue_k: 388.0,
        },
        Shop {
            label: "Chicago",
            state: "IL",
            lon: -87.63,
            lat: 41.88,
            revenue_k: 245.0,
        },
        Shop {
            label: "Houston",
            state: "TX",
            lon: -95.37,
            lat: 29.76,
            revenue_k: 268.0,
        },
        Shop {
            label: "Phoenix",
            state: "AZ",
            lon: -112.07,
            lat: 33.45,
            revenue_k: 198.0,
        },
        Shop {
            label: "Miami",
            state: "FL",
            lon: -80.19,
            lat: 25.76,
            revenue_k: 312.0,
        },
        Shop {
            label: "Dallas",
            state: "TX",
            lon: -96.80,
            lat: 32.78,
            revenue_k: 224.0,
        },
        Shop {
            label: "Seattle",
            state: "WA",
            lon: -122.33,
            lat: 47.61,
            revenue_k: 176.0,
        },
        Shop {
            label: "Denver",
            state: "CO",
            lon: -104.99,
            lat: 39.74,
            revenue_k: 152.0,
        },
        Shop {
            label: "Atlanta",
            state: "GA",
            lon: -84.39,
            lat: 33.75,
            revenue_k: 209.0,
        },
        Shop {
            label: "Boston",
            state: "MA",
            lon: -71.06,
            lat: 42.36,
            revenue_k: 187.0,
        },
        Shop {
            label: "Las Vegas",
            state: "NV",
            lon: -115.14,
            lat: 36.17,
            revenue_k: 234.0,
        },
        Shop {
            label: "Minneapolis",
            state: "MN",
            lon: -93.27,
            lat: 44.98,
            revenue_k: 121.0,
        },
        Shop {
            label: "Nashville",
            state: "TN",
            lon: -86.78,
            lat: 36.16,
            revenue_k: 143.0,
        },
        Shop {
            label: "New Orleans",
            state: "LA",
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
            .style(|design| hyozu::chart::Style {
                background: Some(design.ocean_fill()),
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

        let shops_data = shops();
        let choropleth = hyozu::choropleth(state_revenue(&shops_data))
            .scheme(hyozu::palette::Scheme::Blues)
            .legend_title("July 2025 revenue ($K)")
            .legend(
                legend::Config::overlay(legend::Anchor::BottomRight)
                    .orientation(legend::Orientation::Vertical)
                    .value_format(|v| format!("${v:.0}K")),
            )
            .linear();

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

        // Per-shop label offset sized to each bubble's rendered radius
        // so labels sit just above the bubble's top edge regardless of
        // magnitude. `bubble_map`'s `radius_for` reports the same
        // formula the bubble layer renders, and the +8 px padding
        // keeps labels clear of the hover ring (radius + 2 px).
        let value_range = {
            let values: Vec<f64> = shops_data.iter().map(|s| s.revenue_k).collect();
            let lo = values.iter().copied().fold(f64::INFINITY, f64::min);
            let hi = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
            (lo, hi)
        };
        const LABEL_PADDING: f32 = 8.0;
        let label_items: Vec<hyozu::TextItem> = shops_data
            .iter()
            .map(|s| {
                let r = hyozu::mark::bubble_map::radius_for(s.revenue_k, value_range);
                hyozu::TextItem {
                    datum: hyozu::Datum::new(s.lon, s.lat),
                    label: s.label.to_string(),
                    offset: Some((0.0, -(r + LABEL_PADDING))),
                }
            })
            .collect();
        let labels = hyozu::text(label_items)
            .on_geo()
            .align(hyozu::TextAlign::Center)
            .size(11.0);

        hyozu::data(vec![
            hyozu::Mark::Choropleth(choropleth),
            hyozu::Mark::Xy(bubbles),
            hyozu::Mark::Text(labels),
        ])
        .geo(states.clone(), MapScope::UnitedStates, ProjectionKind::Mercator)
        .value_format(|v| format!("${v:.0}K"))
        .title("Sundae Drive — July 2025, CONUS")
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
