//! Per-dim slicer pins plus a cache of each dim's top-level members.
//!
//! The slicer is the dashboard's unified filter-row UI. Every dim in the
//! schema renders a picker; pinned dims show their active member's path
//! (e.g. `"World/West/US/TX"`) with a clear button; unpinned dims show a
//! placeholder. Pins feed every panel's query at dispatch time.
//!
//! [`State::load_options`] fires one `cube.level_members` per dim to
//! populate the dropdowns; pickers render synchronously from the cache.
//! Drill clicks on panels call [`State::pin`] directly, which is legal
//! even at a deeper level than the picker's top-level options — the
//! selected-value display reads the pin's own path, not the option list.

use std::collections::HashMap;
use std::sync::Arc;

use iced::widget::{Column, Row, button, container, pick_list, row, text};
use iced::{Alignment, Element, Length, Padding, Task};

use tatami::query::{MemberRef, Tuple};
use tatami::schema::{Dimension, Schema};
use tatami_inmem::InMemoryCube;

/// Fixed picker width in logical pixels.
const PICKER_WIDTH: f32 = 180.0;
/// Blocks per grid row. Six schema dims typically fit as 2 × 3.
const BLOCKS_PER_ROW: usize = 3;
/// Horizontal spacing between blocks in a grid row.
const ROW_SPACING: f32 = 12.0;
/// Vertical spacing between stacked rows.
const COLUMN_SPACING: f32 = 6.0;
/// Spacing between children within a single dim block.
const BLOCK_SPACING: f32 = 4.0;
/// Padding around the slicer grid so chips don't hug the outer container.
const GRID_PADDING: Padding = Padding {
    top: 4.0,
    right: 8.0,
    bottom: 4.0,
    left: 8.0,
};
/// Font size for dim labels and hints.
const LABEL_SIZE: f32 = 12.0;
/// Font size for picker text.
const PICKER_SIZE: f32 = 12.0;
/// × glyph shown on the clear button.
const CLEAR_GLYPH: &str = "\u{00d7}";

/// A pick_list option for a slicer picker — a top-level member of some
/// dimension paired with its display label.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Choice {
    /// The underlying member reference.
    pub member: MemberRef,
    /// Display label — the member's path rendered with `/` separators.
    pub label: String,
}

impl std::fmt::Display for Choice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.label)
    }
}

/// Slicer state — pinned members plus a cache of each dim's top-level
/// options.
#[derive(Clone, Debug, Default)]
pub struct State {
    /// Pinned members keyed by dim index into `schema.dimensions`.
    pub pins: HashMap<usize, MemberRef>,
    /// Cached per-dim top-level members. Populated after schema arrival
    /// by one [`InMemoryCube::level_members`] call per dim.
    pub options: HashMap<usize, Vec<MemberRef>>,
}

/// Messages produced by the slicer row.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum Message {
    /// A per-dim top-level members query completed.
    MembersLoaded(usize, Result<Vec<MemberRef>, String>),
    /// User picked a member (`Some(choice)`) or cleared the pin (`None`).
    Picked(usize, Option<Choice>),
}

impl State {
    /// Apply a slicer message to the state. Returns nothing; the caller
    /// is responsible for re-dispatching any downstream queries.
    pub fn update(&mut self, msg: Message) {
        match msg {
            Message::MembersLoaded(dim_index, Ok(members)) => {
                self.options.insert(dim_index, members);
            }
            Message::MembersLoaded(_dim_index, Err(error)) => {
                // One dim's load failing is non-fatal; its picker stays
                // in the loading state, the rest of the UI runs.
                eprintln!("slicer members load failed: {error}");
            }
            Message::Picked(dim_index, choice) => match choice {
                Some(c) => {
                    self.pins.insert(dim_index, c.member);
                }
                None => {
                    self.pins.remove(&dim_index);
                }
            },
        }
    }

    /// Pin a member at `dim_index`, replacing any previous pin for that
    /// dim. Used both by the picker and by panel drill clicks (which
    /// pin at deeper levels than the picker's top-level options).
    pub fn pin(&mut self, dim_index: usize, member: MemberRef) {
        self.pins.insert(dim_index, member);
    }

    /// Replace `pins` to match a concrete [`Tuple`] against `schema`.
    /// Members whose `dim` is not present in `schema.dimensions` are
    /// skipped with a warning; duplicate dims in the tuple are
    /// impossible by `Tuple`'s construction invariant.
    pub fn sync_from_tuple(&mut self, tuple: &Tuple, schema: &Schema) {
        self.pins.clear();
        for member in tuple.members() {
            match schema.dimensions.iter().position(|d| d.name == member.dim) {
                Some(idx) => {
                    self.pins.insert(idx, member.clone());
                }
                None => {
                    eprintln!("slicer: tuple member references unknown dim {:?}", member.dim.as_str());
                }
            }
        }
    }

    /// Snapshot the current pins as a [`Tuple`]. Pins are keyed by dim
    /// index (one entry per dim), so the duplicate-dim check inside
    /// [`Tuple::of`] cannot fire; the defensive fallback preserves the
    /// no-crash contract regardless.
    #[must_use]
    pub fn to_tuple(&self) -> Tuple {
        Tuple::of(self.pins.values().cloned()).unwrap_or_else(|_| Tuple::empty())
    }

    /// Spawn one [`InMemoryCube::level_members`] call per dim, targeting
    /// the dim's first hierarchy + first level (the "top level" whose
    /// members seed the picker dropdowns).
    pub fn load_options(&self, cube: &Arc<InMemoryCube>, schema: &Schema) -> Vec<Task<Message>> {
        let mut tasks = Vec::new();
        for (dim_index, dim) in schema.dimensions.iter().enumerate() {
            let Some(hierarchy) = dim.hierarchies.first() else {
                continue;
            };
            let Some(level) = hierarchy.levels.first() else {
                continue;
            };
            let dim_name = dim.name.clone();
            let hierarchy_name = hierarchy.name.clone();
            let level_name = level.name.clone();
            let cube = cube.clone();
            tasks.push(Task::future(async move {
                let outcome = cube
                    .level_members(&dim_name, &hierarchy_name, &level_name)
                    .map_err(|e| e.to_string());
                Message::MembersLoaded(dim_index, outcome)
            }));
        }
        tasks
    }
}

/// Render the slicer grid — one block per schema dim, arranged with
/// [`BLOCKS_PER_ROW`] blocks per row so six dims land as a compact 2×3.
pub fn view<'a>(state: &'a State, schema: &'a Schema) -> Element<'a, Message> {
    let mut row_children: Vec<Element<'a, Message>> = Vec::with_capacity(BLOCKS_PER_ROW);
    let mut rows: Vec<Element<'a, Message>> = Vec::new();

    for (dim_index, dim) in schema.dimensions.iter().enumerate() {
        row_children.push(dim_block(dim_index, dim, state));
        if row_children.len() == BLOCKS_PER_ROW {
            let finished = std::mem::replace(&mut row_children, Vec::with_capacity(BLOCKS_PER_ROW));
            rows.push(
                Row::with_children(finished)
                    .spacing(ROW_SPACING)
                    .align_y(Alignment::Center)
                    .into(),
            );
        }
    }
    if !row_children.is_empty() {
        rows.push(
            Row::with_children(row_children)
                .spacing(ROW_SPACING)
                .align_y(Alignment::Center)
                .into(),
        );
    }

    container(Column::with_children(rows).spacing(COLUMN_SPACING))
        .padding(GRID_PADDING)
        .into()
}

fn dim_block<'a>(dim_index: usize, dim: &'a Dimension, state: &'a State) -> Element<'a, Message> {
    let label: Element<'a, Message> = text(dim.name.as_str()).size(LABEL_SIZE).into();

    let Some(members) = state.options.get(&dim_index) else {
        return row![label, hint("(loading\u{2026})")]
            .align_y(Alignment::Center)
            .spacing(BLOCK_SPACING)
            .into();
    };

    if members.is_empty() {
        return row![label, hint("(no members)")]
            .align_y(Alignment::Center)
            .spacing(BLOCK_SPACING)
            .into();
    }

    let options: Vec<Choice> = members
        .iter()
        .map(|m| Choice {
            member: m.clone(),
            label: m.path.to_string(),
        })
        .collect();

    let selected = state.pins.get(&dim_index).map(|pinned| Choice {
        member: pinned.clone(),
        label: pinned.path.to_string(),
    });

    let picker = pick_list(selected, options, |c: &Choice| c.label.clone())
        .on_select(move |c: Choice| Message::Picked(dim_index, Some(c)))
        .placeholder("(unbound)")
        .text_size(PICKER_SIZE)
        .width(Length::Fixed(PICKER_WIDTH));

    let clear: Element<'a, Message> = if state.pins.contains_key(&dim_index) {
        button(text(CLEAR_GLYPH).size(LABEL_SIZE))
            .padding([2, 6])
            .on_press(Message::Picked(dim_index, None))
            .into()
    } else {
        text("").into()
    };

    row![label, picker, clear]
        .align_y(Alignment::Center)
        .spacing(BLOCK_SPACING)
        .into()
}

fn hint<'a>(label: &'a str) -> Element<'a, Message> {
    text(label).size(LABEL_SIZE).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    use tatami::query::Path;
    use tatami::schema::{Aggregation, Calendar, Hierarchy, Level, Measure, Name};

    fn n(s: &str) -> Name {
        Name::parse(s).expect("valid name")
    }

    fn test_schema() -> Schema {
        let time = Dimension::time(n("Time"), vec![Calendar::gregorian(n("Fiscal"))])
            .hierarchy(Hierarchy::new(n("Fiscal")).level(Level::new(n("Year"), n("year_key"))));
        let geography = Dimension::regular(n("Geography")).hierarchy(
            Hierarchy::new(n("Default"))
                .level(Level::new(n("World"), n("world_key")))
                .level(Level::new(n("Region"), n("region_key")))
                .level(Level::new(n("Country"), n("country_key")))
                .level(Level::new(n("State"), n("state_key"))),
        );
        let scenario = Dimension::scenario(n("Scenario"))
            .hierarchy(Hierarchy::new(n("Default")).level(Level::new(n("Scenario"), n("scenario_key"))));
        Schema::builder()
            .dimension(time)
            .dimension(geography)
            .dimension(scenario)
            .measure(Measure::new(n("amount"), Aggregation::sum()))
            .build()
            .expect("schema builds")
    }

    fn geo_deep_ref(leaf: &str) -> MemberRef {
        MemberRef::new(
            n("Geography"),
            n("Default"),
            Path::with(n("World"), vec![n("West"), n("US"), n(leaf)]),
        )
    }

    #[test]
    fn pin_then_to_tuple_returns_every_pinned_member() {
        let mut state = State::default();
        state.pin(0, MemberRef::new(n("Time"), n("Fiscal"), Path::of(n("FY2026"))));
        state.pin(1, geo_deep_ref("TX"));

        let tuple = state.to_tuple();
        assert_eq!(tuple.len(), 2);
        let members = tuple.members();
        assert!(members.iter().any(|m| m.dim == n("Time")));
        assert!(members.iter().any(|m| m.dim == n("Geography")));
    }

    #[test]
    fn sync_from_tuple_replaces_pins_with_schema_indices() {
        let schema = test_schema();
        let tuple = Tuple::of([
            MemberRef::new(n("Time"), n("Fiscal"), Path::of(n("FY2026"))),
            geo_deep_ref("CA"),
        ])
        .expect("valid tuple");

        let mut state = State::default();
        state.pin(2, MemberRef::scenario(n("Actual")));

        state.sync_from_tuple(&tuple, &schema);

        assert_eq!(state.pins.len(), 2);
        assert_eq!(state.pins[&0].dim, n("Time"));
        assert_eq!(state.pins[&1].dim, n("Geography"));
        assert_eq!(state.pins[&1].path.to_string(), "World/West/US/CA");
        assert!(!state.pins.contains_key(&2), "prior Scenario pin was cleared");
    }

    #[test]
    fn sync_from_tuple_skips_unknown_dims() {
        let schema = test_schema();
        let stray = MemberRef::new(n("BrandTier"), n("Default"), Path::of(n("Luxury")));
        let tuple = Tuple::of([stray]).expect("valid tuple");

        let mut state = State::default();
        state.sync_from_tuple(&tuple, &schema);

        assert!(state.pins.is_empty());
    }

    #[test]
    fn pin_then_to_tuple_then_sync_round_trips() {
        let schema = test_schema();
        let mut state = State::default();
        state.pin(0, MemberRef::new(n("Time"), n("Fiscal"), Path::of(n("FY2026"))));
        state.pin(1, geo_deep_ref("CA"));

        let tuple = state.to_tuple();

        let mut rebuilt = State::default();
        rebuilt.sync_from_tuple(&tuple, &schema);

        assert_eq!(rebuilt.pins.len(), state.pins.len());
        assert_eq!(rebuilt.pins[&0].dim, n("Time"));
        assert_eq!(rebuilt.pins[&1].path.to_string(), "World/West/US/CA");
    }

    #[test]
    fn update_picked_some_sets_pin() {
        let mut state = State::default();
        let member = MemberRef::scenario(n("Actual"));
        state.update(Message::Picked(
            3,
            Some(Choice {
                member: member.clone(),
                label: "Actual".into(),
            }),
        ));
        assert_eq!(state.pins[&3], member);
    }

    #[test]
    fn update_picked_none_clears_pin() {
        let mut state = State::default();
        state.pin(3, MemberRef::scenario(n("Actual")));
        state.update(Message::Picked(3, None));
        assert!(state.pins.is_empty());
    }

    #[test]
    fn update_members_loaded_populates_cache() {
        let mut state = State::default();
        let members = vec![MemberRef::scenario(n("Actual"))];
        state.update(Message::MembersLoaded(0, Ok(members.clone())));
        assert_eq!(state.options[&0], members);
    }
}
