use serde::Deserialize;
use std::collections::BTreeMap;

use crate::model::{footer, metric, sizing, table};

/// A self-contained data packet pinned to the home screen.
#[derive(Deserialize)]
pub struct Card {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: Kind,
    pub header: Header,
    #[serde(default)]
    pub meta: Option<Meta>,
    #[serde(default)]
    pub selectors: BTreeMap<String, Selector>,
    #[serde(default)]
    pub context: BTreeMap<String, String>,
    pub body: Body,
    pub footer: footer::Footer,
    pub sizing: sizing::Sizing,
}

#[derive(Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Metric,
    Table,
}

#[derive(Deserialize)]
pub struct Header {
    pub label: String,
}

#[derive(Deserialize)]
pub struct Selector {
    pub options: Vec<String>,
    pub default: String,
}

#[derive(Deserialize)]
pub struct Meta {
    #[serde(default)]
    pub description: Option<String>,
}

/// Card body — metric or table, discriminated structurally.
#[derive(Deserialize)]
#[serde(untagged)]
pub enum Body {
    Metric(metric::Body),
    Table(table::Body),
}
