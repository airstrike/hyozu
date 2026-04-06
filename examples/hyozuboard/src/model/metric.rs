use serde::Deserialize;
use std::collections::BTreeMap;

use crate::model::Token;

#[derive(Deserialize)]
pub struct Body {
    pub data: Vec<Record>,
    #[serde(default)]
    pub chart: Option<Chart>,
}

/// One metric value for one selector combination.
///
/// Properties matching selector keys are captured via `#[serde(flatten)]`.
#[derive(Deserialize)]
pub struct Record {
    pub value: String,
    #[serde(default)]
    pub delta: Option<Delta>,
    #[serde(default)]
    pub series: Option<Vec<Series>>,
    #[serde(default)]
    pub labels: Option<Vec<String>>,
    #[serde(default)]
    pub geo_points: Option<Vec<GeoPoint>>,
    #[serde(flatten)]
    pub selectors: BTreeMap<String, serde_json::Value>,
}

impl Record {
    /// Whether this record matches the current selector choices.
    pub fn matches(&self, selections: &BTreeMap<String, String>) -> bool {
        selections.iter().all(|(k, v)| {
            self.selectors
                .get(k)
                .and_then(|sv| sv.as_str())
                .map_or(true, |sv| sv == v)
        })
    }
}

#[derive(Deserialize)]
pub struct Delta {
    pub display: String,
    pub direction: Direction,
    #[serde(default)]
    pub period: Option<String>,
}

#[derive(Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    Positive,
    Negative,
    Neutral,
}

#[derive(Deserialize)]
pub struct Series {
    #[serde(default)]
    pub label: Option<String>,
    pub data: Vec<f64>,
    #[serde(default)]
    pub color: Option<Token>,
}

#[derive(Deserialize)]
pub struct Chart {
    #[serde(rename = "type")]
    pub kind: Kind,
}

#[derive(Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Line,
    Bar,
    Map,
}

/// A geographic point for bubble-map charts.
#[derive(Deserialize)]
pub struct GeoPoint {
    pub lat: f64,
    pub lon: f64,
    pub value: f64,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub color: Option<Token>,
}
