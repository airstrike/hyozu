//! `drill_down` — Hewton Hotels multi-panel sales dashboard example.
//!
//! Validates the `tatami` OLAP cube + `hyozu::tatami` adapter end-to-end.
//! This crate is primarily a binary; the library exists so
//! `cargo test --lib -p drill_down` can reach internal types.

pub mod data;
pub mod ui;
