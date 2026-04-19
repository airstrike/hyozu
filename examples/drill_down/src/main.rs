//! drill_down — Hewton Hotels multi-panel sales dashboard.
//!
//! Loads three assets in parallel at startup: a facts CSV parsed via Polars
//! on the blocking thread pool, plus a Natural Earth admin_1 GeoJSON
//! fetched over HTTPS. Per-state centroids are derived from the loaded
//! GeoJSON, so there are only two async asset slots to track. Once all
//! assets are in, the app constructs an `InMemoryCube` from the DataFrame
//! + schema and hands off to the `Dashboard`.

use std::sync::Arc;

use iced::widget::{column, container, text};
use iced::{Element, Length, Task};

use polars_core::prelude::DataFrame;
use tatami_inmem::InMemoryCube;

use drill_down::ui::dashboard::{self, Dashboard};
use drill_down::{data, ui};

/// Natural Earth 110m admin_1 states/provinces GeoJSON. Features carry a
/// `postal` property (two-letter USPS code) that becomes the feature id; the
/// choropleth entries and centroid lookup key off that code.
const STATES_URL: &str = "https://raw.githubusercontent.com/nvkelso/natural-earth-vector/master/geojson/ne_110m_admin_1_states_provinces.geojson";

fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .title("Drill Down — Hewton Hotels")
        .window_size((1400.0, 900.0))
        .run()
}

/// Top-level application state. The large variants are boxed so the enum's
/// stack footprint stays flat across state transitions.
enum App {
    /// Assets are loading in parallel. Each slot starts `None` and is
    /// populated by its `*Loaded` message.
    Loading(Box<LoadingState>),
    /// Both assets loaded and the cube was constructed.
    Loaded(Box<Dashboard>),
    /// An unrecoverable error occurred during load or cube construction.
    Failed(data::Error),
}

/// In-flight asset slots while loading.
struct LoadingState {
    facts: Option<DataFrame>,
    geo: Option<Arc<hyozu::GeoData>>,
}

/// Application-level messages.
#[derive(Debug)]
enum Message {
    /// Fact CSV finished loading (success or failure).
    FactsLoaded(Result<DataFrame, data::Error>),
    /// GeoJSON finished loading (success or failure).
    GeoLoaded(Result<Arc<hyozu::GeoData>, data::Error>),
    /// A panel's query finished.
    PanelDone(ui::Panel, Result<tatami::Results, String>),
    /// The user clicked a measure toggle.
    SetMeasure(ui::Measure),
    /// The user clicked a period toggle.
    #[allow(dead_code)]
    SetPeriod(ui::Period),
    /// Back button.
    Back,
    /// Forward button.
    Forward,
    /// Retry loading after a failure.
    #[allow(dead_code)]
    Retry,
}

impl App {
    fn new() -> (Self, Task<Message>) {
        let load = Task::batch([
            Task::future(load_facts()).map(Message::FactsLoaded),
            Task::future(load_geo(STATES_URL)).map(Message::GeoLoaded),
        ]);
        (App::Loading(Box::new(LoadingState { facts: None, geo: None })), load)
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::FactsLoaded(Ok(df)) => {
                if let App::Loading(state) = self {
                    state.facts = Some(df);
                }
                self.try_transition()
            }
            Message::FactsLoaded(Err(e)) => {
                *self = App::Failed(e);
                Task::none()
            }
            Message::GeoLoaded(Ok(g)) => {
                if let App::Loading(state) = self {
                    state.geo = Some(g);
                }
                self.try_transition()
            }
            Message::GeoLoaded(Err(e)) => {
                *self = App::Failed(e);
                Task::none()
            }
            Message::PanelDone(panel, outcome) => {
                if let App::Loaded(dashboard) = self {
                    dashboard
                        .update(dashboard::Message::PanelDone(panel, outcome))
                        .map(lift_dashboard_message)
                } else {
                    Task::none()
                }
            }
            Message::SetMeasure(m) => {
                if let App::Loaded(dashboard) = self {
                    let v = ui::View {
                        measure: m,
                        ..dashboard.view
                    };
                    dashboard.set_view(v).map(lift_dashboard_message)
                } else {
                    Task::none()
                }
            }
            Message::SetPeriod(p) => {
                if let App::Loaded(dashboard) = self {
                    let v = ui::View {
                        period: p,
                        ..dashboard.view
                    };
                    dashboard.set_view(v).map(lift_dashboard_message)
                } else {
                    Task::none()
                }
            }
            Message::Back => {
                if let App::Loaded(dashboard) = self {
                    dashboard.back().map(lift_dashboard_message)
                } else {
                    Task::none()
                }
            }
            Message::Forward => {
                if let App::Loaded(dashboard) = self {
                    dashboard.forward().map(lift_dashboard_message)
                } else {
                    Task::none()
                }
            }
            Message::Retry => {
                let (fresh, task) = App::new();
                *self = fresh;
                task
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        match self {
            App::Loading(_) => container(text("Loading…").size(24))
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .into(),
            App::Failed(err) => {
                container(column![text("Failed to load").size(24), text(format!("{err}")).size(14),].spacing(8))
                    .center_x(Length::Fill)
                    .center_y(Length::Fill)
                    .into()
            }
            App::Loaded(dashboard) => dashboard.view().map(lift_dashboard_message),
        }
    }

    /// Check whether both assets are loaded; if so, derive centroids, build
    /// the cube, and transition to `Loaded`. On cube-build failure,
    /// transition to `Failed`.
    ///
    /// Checks presence *before* consuming any slot — a naive destructure via
    /// `facts.take()` / `geo.take()` drains those slots even when the match
    /// fails, which permanently strands the loader if the other asset hasn't
    /// arrived yet.
    fn try_transition(&mut self) -> Task<Message> {
        let App::Loading(state) = self else {
            return Task::none();
        };
        if state.facts.is_none() || state.geo.is_none() {
            return Task::none();
        }
        let df = state.facts.take().expect("presence checked");
        let geo = state.geo.take().expect("presence checked");

        let schema = match data::hewton_schema() {
            Ok(s) => s,
            Err(e) => {
                *self = App::Failed(e.into());
                return Task::none();
            }
        };
        let cube = match InMemoryCube::new(df, schema) {
            Ok(c) => Arc::new(c),
            Err(e) => {
                *self = App::Failed(e.into());
                return Task::none();
            }
        };
        let centroids = Arc::new(data::Centroids::from_geo(&geo));

        let initial_query = ui::queries::nation_default();
        let (dashboard, task) = Dashboard::new(cube, geo, centroids, initial_query, ui::View::default());
        *self = App::Loaded(Box::new(dashboard));
        task.map(lift_dashboard_message)
    }
}

/// Lift a `dashboard::Message` into the outer `Message`. Every variant of
/// `dashboard::Message` has a matching outer variant; when dashboard grows
/// a new variant, extend this match in lockstep.
fn lift_dashboard_message(msg: dashboard::Message) -> Message {
    match msg {
        dashboard::Message::PanelDone(p, r) => Message::PanelDone(p, r),
        dashboard::Message::SetMeasure(m) => Message::SetMeasure(m),
        dashboard::Message::SetPeriod(p) => Message::SetPeriod(p),
        dashboard::Message::Back => Message::Back,
        dashboard::Message::Forward => Message::Forward,
    }
}

// ── Async load helpers ────────────────────────────────────────────────────

async fn load_bytes(asset: &'static str) -> Result<Vec<u8>, data::Error> {
    let path = format!("{}/assets/{asset}", env!("CARGO_MANIFEST_DIR"));
    tokio::fs::read(&path)
        .await
        .map_err(|e| data::Error::Io { path, source: e })
}

async fn load_facts() -> Result<DataFrame, data::Error> {
    let bytes = load_bytes("hewton.csv").await?;
    tokio::task::spawn_blocking(move || {
        use polars_io::prelude::{CsvReadOptions, SerReader};
        use std::io::Cursor;
        CsvReadOptions::default()
            .with_has_header(true)
            .into_reader_with_file_handle(Cursor::new(bytes))
            .finish()
            .map_err(|e| data::Error::Parse {
                asset: "hewton.csv",
                detail: e.to_string(),
            })
    })
    .await?
}

async fn load_geo(url: &'static str) -> Result<Arc<hyozu::GeoData>, data::Error> {
    let body = reqwest::get(url)
        .await
        .map_err(|e| data::Error::Fetch {
            url: url.to_owned(),
            detail: e.to_string(),
        })?
        .text()
        .await
        .map_err(|e| data::Error::Fetch {
            url: url.to_owned(),
            detail: e.to_string(),
        })?;
    let data = hyozu::geo::parse_geojson(&body).map_err(|e| data::Error::Parse {
        asset: "states.geojson",
        detail: e.to_string(),
    })?;
    Ok(Arc::new(data))
}
