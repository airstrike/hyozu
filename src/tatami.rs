//! Adapter — `tatami::Results` → hyozu marks and iced widgets.
//!
//! Feature-gated behind `tatami`; pulled in by downstream dashboards as
//! `hyozu = { ..., features = ["tatami"] }`. The adapter only depends on the
//! `tatami` core crate (schema + query + results + cube trait), not on
//! `tatami-inmem` — so consumers pay no Polars cost for the type conversions
//! alone. They plug their own `impl Cube` (in-memory, Polars, DuckDB, HTTP)
//! at the call site.
//!
//! ## What each converter does
//!
//! - [`scalar::card`] — build a KPI-card iced `Element` from
//!   [`tatami::scalar::Result`]. Composes title / primary value / optional
//!   delta / optional caller-supplied sparkline data.
//! - [`series::line`] / [`series::bars`] — build [`crate::Data`] from
//!   [`tatami::series::Result`]. One [`crate::Mark::Line`] (or
//!   [`crate::Mark::Bars`]) per `series::Row`, x-axis values from the member
//!   path tails.
//! - [`rollup::choropleth`] — build [`crate::Data`] from [`tatami::rollup::Tree`]
//!   with [`crate::Mark::Choropleth`]; caller supplies a `MemberRef → feature_id`
//!   closure.
//! - [`rollup::bubble_map`] — overlay [`crate::Mark::BubbleMap`] on a rollup;
//!   caller supplies a `MemberRef → Option<(lat, lon)>` closure.
//! - [`pivot::table`] — render [`tatami::pivot::Result`] as an iced
//!   `column`/`row` table with per-cell `Cell::Valid::format` honored.
//!
//! All converters are total: [`tatami::Cell::Missing`] renders as `—` or
//! `f64::NAN` (per site); [`tatami::Cell::Error`] renders as `⚠` text. No
//! variant ever silently drops to zero.

pub mod cell;
pub mod pivot;
pub mod rollup;
pub mod scalar;
pub mod series;

pub use cell::{cell_f64, cells_f64};
pub use pivot::table;
pub use rollup::{bubble_map, choropleth};
pub use scalar::{KpiLayout, card};
pub use series::{bars, line};
