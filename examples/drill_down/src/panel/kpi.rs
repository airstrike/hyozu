//! KPI panel — a 3×2 grid of scalar chips.
//!
//! Each chip is display-only: big formatted value with the metric's name
//! beneath. The panel emits one `tatami::Query` with [`tatami::Axes::Scalar`]
//! and one metric per pinned slot; the result's values array aligns to the
//! pinned-slot sequence (unpinned slots skipped). Slots are configured via
//! [`crate::DashboardSpec`]; there's no runtime picker.

use iced::widget::{Column, Renderer, Row, container, text};
use iced::{Alignment, Element, Length, Theme};

use tatami::query::{Options, Tuple};
use tatami::schema::{Name, Schema};

use crate::{dashboard, metric};

/// How many scalar chips the KPI panel renders. Laid out as 3 rows × 2 cols.
pub const SLOTS: usize = 6;
/// Chips per row in the panel body.
const CHIPS_PER_ROW: usize = 2;

/// Per-panel bindings — one metric pick per chip slot.
#[derive(Debug, Clone)]
pub struct State {
    /// Slot metric picks. Always length [`SLOTS`]; empty slots are `None`.
    pub metrics: Vec<Option<metric::Pick>>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            metrics: vec![None; SLOTS],
        }
    }
}

impl State {
    /// Build a state from an initial list of metric picks. The list is
    /// truncated or padded with `None` to length [`SLOTS`].
    #[must_use]
    pub fn new(metrics: Vec<Option<metric::Pick>>) -> Self {
        let mut m = metrics;
        m.resize(SLOTS, None);
        Self { metrics: m }
    }

    /// Build a [`tatami::Query`] from the pinned slots against `schema`,
    /// seeded with the trail's slicer. Returns `None` when no slot
    /// resolves to a metric name.
    #[must_use]
    pub fn query(&self, schema: &Schema, slicer: Tuple) -> Option<tatami::Query> {
        let names: Vec<Name> = self
            .metrics
            .iter()
            .filter_map(|pick| (*pick).and_then(|p| metric::resolve(schema, p)))
            .collect();
        if names.is_empty() {
            return None;
        }
        Some(tatami::Query {
            axes: tatami::Axes::Scalar,
            slicer,
            metrics: names,
            options: Options::default(),
        })
    }

    /// For each slot, the index into a scalar result's values array where
    /// that slot's value lands — or `None` if the slot is unpinned or the
    /// metric can't be resolved. Aligns to [`State::query`]'s metrics order.
    #[must_use]
    pub fn slot_result_indices(&self, schema: &Schema) -> Vec<Option<usize>> {
        let mut cursor: usize = 0;
        self.metrics
            .iter()
            .map(|pick| match pick.and_then(|p| metric::resolve(schema, p)) {
                Some(_) => {
                    let i = cursor;
                    cursor += 1;
                    Some(i)
                }
                None => None,
            })
            .collect()
    }
}

/// Render the 3×2 chip grid from the query's scalar result.
#[must_use]
pub fn render<'a>(
    results: &'a tatami::Results,
    schema: &'a Schema,
    state: &State,
) -> Element<'a, dashboard::Message, Theme, Renderer> {
    let values: &[tatami::Cell] = match results {
        tatami::Results::Scalar(s) => s.values(),
        _ => &[],
    };
    let slot_indices = state.slot_result_indices(schema);

    let mut row_children: Vec<Element<'a, dashboard::Message, Theme, Renderer>> = Vec::with_capacity(CHIPS_PER_ROW);
    let mut rows: Vec<Element<'a, dashboard::Message, Theme, Renderer>> = Vec::new();

    for (i, pick) in state.metrics.iter().enumerate() {
        let value_cell = slot_indices[i].and_then(|idx| values.get(idx));
        row_children.push(chip(*pick, schema, value_cell));
        if row_children.len() == CHIPS_PER_ROW {
            let finished = std::mem::replace(&mut row_children, Vec::with_capacity(CHIPS_PER_ROW));
            rows.push(
                Row::with_children(finished)
                    .spacing(12)
                    .align_y(Alignment::Start)
                    .into(),
            );
        }
    }
    if !row_children.is_empty() {
        rows.push(
            Row::with_children(row_children)
                .spacing(12)
                .align_y(Alignment::Start)
                .into(),
        );
    }

    container(Column::with_children(rows).spacing(12))
        .padding([8, 12])
        .into()
}

/// Render one chip: big value on top, metric name beneath.
fn chip<'a>(
    pick: Option<metric::Pick>,
    schema: &'a Schema,
    value_cell: Option<&tatami::Cell>,
) -> Element<'a, dashboard::Message, Theme, Renderer> {
    let value_text = match (pick, value_cell) {
        (Some(_), Some(cell)) => format_cell(cell),
        _ => "—".to_owned(),
    };
    let label = pick.and_then(|p| metric::label(schema, p)).unwrap_or("(unset)");

    Column::with_children([text(value_text).size(22).into(), text(label.to_owned()).size(11).into()])
        .spacing(2)
        .align_x(Alignment::Start)
        .width(Length::Fill)
        .into()
}

/// KPI has no title-bar controls — chips are display-only.
#[must_use]
pub fn chrome<'a>() -> Element<'a, dashboard::Message, Theme, Renderer> {
    Row::new().into()
}

fn format_cell(cell: &tatami::Cell) -> String {
    let v = hyozu::tatami::cell_f64(cell);
    if v.is_nan() {
        "—".into()
    } else if v.abs() >= 1_000_000.0 {
        format!("{:.1}M", v / 1_000_000.0)
    } else if v.abs() >= 1_000.0 {
        format!("{:.1}K", v / 1_000.0)
    } else {
        format!("{v:.2}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use tatami::schema::{Aggregation, Dimension, Hierarchy, Level, Measure, Name};

    fn name(s: &str) -> Name {
        Name::parse(s).expect("valid")
    }

    fn small_schema() -> Schema {
        let dim = Dimension::regular(name("Geography"))
            .hierarchy(Hierarchy::new(name("Default")).level(Level::new(name("World"), name("world"))));
        Schema::builder()
            .dimension(dim)
            .measure(Measure::new(name("amount"), Aggregation::sum()))
            .measure(Measure::new(name("count"), Aggregation::sum()))
            .build()
            .expect("builds")
    }

    #[test]
    fn state_new_truncates_and_pads_to_slots() {
        let three = State::new(vec![None, None, None]);
        assert_eq!(three.metrics.len(), SLOTS);
        let too_many = State::new(vec![None; SLOTS + 2]);
        assert_eq!(too_many.metrics.len(), SLOTS);
    }

    #[test]
    fn query_returns_none_when_all_slots_empty() {
        let schema = small_schema();
        let state = State::default();
        assert!(state.query(&schema, Tuple::empty()).is_none());
    }

    #[test]
    fn query_has_one_metric_per_pinned_slot() {
        let schema = small_schema();
        let mut state = State::default();
        state.metrics[0] = Some(metric::Pick::Measure(0));
        state.metrics[3] = Some(metric::Pick::Measure(1));
        let q = state.query(&schema, Tuple::empty()).expect("query");
        assert_eq!(q.metrics.len(), 2);
    }

    #[test]
    fn slot_result_indices_align_with_pinned_order() {
        let schema = small_schema();
        let mut state = State::default();
        state.metrics[0] = Some(metric::Pick::Measure(0));
        state.metrics[3] = Some(metric::Pick::Measure(1));
        let indices = state.slot_result_indices(&schema);
        assert_eq!(indices[0], Some(0));
        assert_eq!(indices[1], None);
        assert_eq!(indices[3], Some(1));
    }
}
