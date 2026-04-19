//! Hewton Hotels dashboard binary.
//!
//! Loads the hewton CSV + Natural Earth admin_1 GeoJSON in parallel, builds
//! an `InMemoryCube`, resolves the cube's hewton-specific vocabulary into
//! schema indices once, and hands the whole thing to the schema-blind
//! `drill_down::Dashboard` as a `DashboardSpec`. Every name referenced
//! below lives either here or in `data::schema::hewton_schema`.

use std::num::NonZeroUsize;
use std::sync::Arc;

use iced::widget::{column, container, text};
use iced::{Element, Length, Task};

use hyozu::MapScope;
use polars_core::prelude::DataFrame;
use tatami::query::{MemberRef, Path};
use tatami::schema::{Name, Schema};
use tatami_inmem::InMemoryCube;

use drill_down::panel::{kpi, line, map, rail};
use drill_down::{DashboardSpec, axis, dashboard, data, metric};

/// Natural Earth 110m admin_1 states/provinces GeoJSON. Features carry a
/// `postal` property (two-letter USPS code) that becomes the feature id;
/// the choropleth entries and centroid lookup key off that code.
const STATES_URL: &str = "https://raw.githubusercontent.com/nvkelso/natural-earth-vector/master/geojson/ne_110m_admin_1_states_provinces.geojson";

fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .title("Drill Down \u{2014} Hewton Hotels")
        .window_size((1400.0, 900.0))
        .run()
}

enum App {
    Loading(Box<LoadingState>),
    Loaded(Box<dashboard::Dashboard>),
    Failed(data::Error),
}

struct LoadingState {
    facts: Option<DataFrame>,
    geo: Option<Arc<hyozu::GeoData>>,
}

#[derive(Debug)]
enum Message {
    FactsLoaded(Result<DataFrame, data::Error>),
    GeoLoaded(Result<Arc<hyozu::GeoData>, data::Error>),
    Dashboard(dashboard::Message),
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
            Message::Dashboard(msg) => {
                if let App::Loaded(dashboard) = self {
                    dashboard.update(msg).map(Message::Dashboard)
                } else {
                    Task::none()
                }
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        match self {
            App::Loading(_) => container(text("Loading\u{2026}").size(24))
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .into(),
            App::Failed(err) => {
                container(column![text("Failed to load").size(24), text(format!("{err}")).size(14),].spacing(8))
                    .center_x(Length::Fill)
                    .center_y(Length::Fill)
                    .into()
            }
            App::Loaded(dashboard) => dashboard.view().map(Message::Dashboard),
        }
    }

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
        let cube = match InMemoryCube::new(df, schema.clone()) {
            Ok(c) => Arc::new(c),
            Err(e) => {
                *self = App::Failed(e.into());
                return Task::none();
            }
        };
        let centroids = Arc::new(data::Centroids::from_geo(&geo));

        let spec = match build_spec(&schema) {
            Ok(s) => s,
            Err(e) => {
                *self = App::Failed(data::Error::Parse {
                    asset: "DashboardSpec",
                    detail: e,
                });
                return Task::none();
            }
        };
        let (dashboard, task) = dashboard::Dashboard::new(cube, schema, geo, centroids, spec);
        *self = App::Loaded(Box::new(dashboard));
        task.map(Message::Dashboard)
    }
}

// ── Hewton-specific spec binding ───────────────────────────────────────────

/// Resolve hewton's vocabulary against the schema and assemble the
/// concrete [`DashboardSpec`] for this binary. This is the only place
/// names appear — everything else is indices.
fn build_spec(schema: &Schema) -> Result<DashboardSpec, String> {
    let geo_dim = dim_index(schema, "Geography")?;
    let geo_state_level = level_index(schema, geo_dim, "Default", "State")?;
    let geo_country_level = level_index(schema, geo_dim, "Default", "Country")?;

    let brand_tier_dim = dim_index(schema, "BrandTier")?;
    let brand_tier_level = level_index(schema, brand_tier_dim, "Default", "Tier")?;

    let time_dim = dim_index(schema, "Time")?;
    let quarter_level = level_index(schema, time_dim, "Fiscal", "Quarter")?;

    let revenue = metric::Pick::Metric(metric_index(schema, "Revenue")?);
    let room_nights_sold = metric::Pick::Measure(measure_index(schema, "room_nights_sold")?);

    let initial_slicer = vec![
        MemberRef::new(
            parse_name("Time")?,
            parse_name("Fiscal")?,
            Path::of(parse_name("FY2026")?),
        ),
        MemberRef::scenario(parse_name("Actual")?),
    ];

    let kpi = kpi::State { metric: Some(revenue) };
    let map = map::State {
        rows: axis::Pick::Pick {
            dim: geo_dim,
            hierarchy: geo_state_level.hierarchy,
            level: geo_state_level.level,
        },
        fill: Some(revenue),
        size: Some(room_nights_sold),
        scope: MapScope::UnitedStates,
    };
    let rail = rail::State {
        rows: axis::Pick::Pick {
            dim: brand_tier_dim,
            hierarchy: brand_tier_level.hierarchy,
            level: brand_tier_level.level,
        },
        metric: Some(revenue),
        top_n: NonZeroUsize::new(10).expect("10 > 0"),
    };
    let line = line::State {
        rows: axis::Pick::Pick {
            dim: time_dim,
            hierarchy: quarter_level.hierarchy,
            level: quarter_level.level,
        },
        metric: Some(revenue),
    };

    let _ = geo_country_level;

    Ok(DashboardSpec {
        title: "Hewton Hotels".to_owned(),
        initial_slicer,
        kpi,
        map,
        rail,
        line,
    })
}

struct LevelIndex {
    hierarchy: usize,
    level: usize,
}

fn parse_name(s: &str) -> Result<Name, String> {
    Name::parse(s).map_err(|e| format!("name {s}: {e}"))
}

fn dim_index(schema: &Schema, name: &str) -> Result<usize, String> {
    schema
        .dimensions
        .iter()
        .position(|d| d.name.as_str() == name)
        .ok_or_else(|| format!("dimension {name} not in schema"))
}

fn level_index(schema: &Schema, dim: usize, hierarchy: &str, level: &str) -> Result<LevelIndex, String> {
    let d = schema
        .dimensions
        .get(dim)
        .ok_or_else(|| format!("dim index {dim} out of range"))?;
    let h_idx = d
        .hierarchies
        .iter()
        .position(|h| h.name.as_str() == hierarchy)
        .ok_or_else(|| format!("hierarchy {hierarchy} not under dim {}", d.name.as_str()))?;
    let h = &d.hierarchies[h_idx];
    let l_idx = h
        .levels
        .iter()
        .position(|l| l.name.as_str() == level)
        .ok_or_else(|| format!("level {level} not under {}/{hierarchy}", d.name.as_str()))?;
    Ok(LevelIndex {
        hierarchy: h_idx,
        level: l_idx,
    })
}

fn measure_index(schema: &Schema, name: &str) -> Result<usize, String> {
    schema
        .measures
        .iter()
        .position(|m| m.name.as_str() == name)
        .ok_or_else(|| format!("measure {name} not in schema"))
}

fn metric_index(schema: &Schema, name: &str) -> Result<usize, String> {
    schema
        .metrics
        .iter()
        .position(|m| m.name.as_str() == name)
        .ok_or_else(|| format!("metric {name} not in schema"))
}

// ── Async load helpers ─────────────────────────────────────────────────────

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
