//! Metric pick — a schema-blind selection of a measure or metric by index
//! into the schema's `measures` / `metrics` arrays.

use std::fmt;

use tatami::schema::{Name, Schema};

/// A metric choice — either an index into `schema.measures` or an index
/// into `schema.metrics`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Pick {
    /// Index into `schema.measures`.
    Measure(usize),
    /// Index into `schema.metrics`.
    Metric(usize),
}

/// A pick_list option — a [`Pick`] with a pre-resolved display label.
///
/// Built once per view via [`choices`] and handed to every panel's metric
/// picker. The label is cloned from the schema, so nothing in the UI
/// layer has to reach back to the schema on every re-render.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Choice {
    /// The underlying pick.
    pub pick: Pick,
    /// Display label — the declared measure/metric name.
    pub label: String,
}

impl fmt::Display for Choice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.label)
    }
}

/// Build the full list of metric options for a schema — measures first,
/// then metrics, in declared order. Intended to be built once per render.
#[must_use]
pub fn choices(schema: &Schema) -> Vec<Choice> {
    schema
        .measures
        .iter()
        .enumerate()
        .map(|(i, m)| Choice {
            pick: Pick::Measure(i),
            label: m.name.as_str().to_owned(),
        })
        .chain(schema.metrics.iter().enumerate().map(|(i, m)| Choice {
            pick: Pick::Metric(i),
            label: m.name.as_str().to_owned(),
        }))
        .collect()
}

/// Resolve a pick to its declared [`Name`] against the schema. Returns
/// `None` when the index is out of range.
#[must_use]
pub fn resolve(schema: &Schema, pick: Pick) -> Option<Name> {
    match pick {
        Pick::Measure(i) => schema.measures.get(i).map(|m| m.name.clone()),
        Pick::Metric(i) => schema.metrics.get(i).map(|m| m.name.clone()),
    }
}

/// Borrow the declared label (the `Name` string) for a pick. Returns
/// `None` when the index is out of range.
#[must_use]
pub fn label(schema: &Schema, pick: Pick) -> Option<&str> {
    match pick {
        Pick::Measure(i) => schema.measures.get(i).map(|m| m.name.as_str()),
        Pick::Metric(i) => schema.metrics.get(i).map(|m| m.name.as_str()),
    }
}
