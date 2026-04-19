//! Axis pick — a schema-blind selection of a `(dimension, hierarchy, level)`
//! triple, or none.
//!
//! Every axis-like panel field (rows, columns, …) is a [`Pick`]. Concrete
//! `Name` values flow out only at query-assembly time, via [`build_set`].

use std::fmt;

use tatami::query::Set;
use tatami::schema::Schema;

/// An axis choice, sourced entirely from schema indices.
///
/// `None` means the axis is absent; [`Pick::Pick`] carries three indices
/// into the schema — `dim` into `schema.dimensions`, `hierarchy` into
/// `schema.dimensions[dim].hierarchies`, `level` into
/// `schema.dimensions[dim].hierarchies[hierarchy].levels`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Pick {
    /// Axis absent.
    #[default]
    None,
    /// Axis present at the given `(dim, hierarchy, level)` indices.
    Pick {
        /// Index into `schema.dimensions`.
        dim: usize,
        /// Index into `schema.dimensions[dim].hierarchies`.
        hierarchy: usize,
        /// Index into `schema.dimensions[dim].hierarchies[hierarchy].levels`.
        level: usize,
    },
}

impl Pick {
    /// Materialize the three index coordinates if the axis is present.
    #[must_use]
    pub fn indices(self) -> Option<(usize, usize, usize)> {
        match self {
            Pick::None => None,
            Pick::Pick { dim, hierarchy, level } => Some((dim, hierarchy, level)),
        }
    }

    /// Dim index, if present.
    #[must_use]
    pub fn dim(self) -> Option<usize> {
        self.indices().map(|(d, _, _)| d)
    }
}

/// Build a [`Set::members`] from an axis pick against a schema. Returns
/// `None` when the pick is absent or the indices do not resolve.
#[must_use]
pub fn build_set(schema: &Schema, pick: &Pick) -> Option<Set> {
    let (dim, hierarchy, level) = pick.indices()?;
    let d = schema.dimensions.get(dim)?;
    let h = d.hierarchies.get(hierarchy)?;
    let l = h.levels.get(level)?;
    Some(Set::members(d.name.clone(), h.name.clone(), l.name.clone()))
}

/// A pick_list option for the dim slot of an axis picker.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DimChoice {
    /// Index into `schema.dimensions`.
    pub index: usize,
    /// Human-readable label — the dim's declared name.
    pub label: String,
}

impl fmt::Display for DimChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.label)
    }
}

/// Every dim in the schema, packaged for a pick_list.
#[must_use]
pub fn dim_choices(schema: &Schema) -> Vec<DimChoice> {
    schema
        .dimensions
        .iter()
        .enumerate()
        .map(|(i, d)| DimChoice {
            index: i,
            label: d.name.as_str().to_owned(),
        })
        .collect()
}

/// Resolve a dim choice against the current pick — returns the list entry
/// whose `index` matches, if any.
#[must_use]
pub fn current_dim(options: &[DimChoice], pick: &Pick) -> Option<DimChoice> {
    pick.dim().and_then(|d| options.iter().find(|c| c.index == d).cloned())
}

/// A pick_list option for the level slot of an axis picker — an indexed
/// `(hierarchy, level)` pair scoped to an already-chosen dim.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LevelChoice {
    /// Index into `schema.dimensions[dim].hierarchies`.
    pub hierarchy: usize,
    /// Index into `schema.dimensions[dim].hierarchies[hierarchy].levels`.
    pub level: usize,
    /// Human-readable label.
    pub label: String,
}

impl fmt::Display for LevelChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.label)
    }
}

/// Build the flat list of level choices for a given dim — every
/// `(hierarchy, level)` pair, labeled `"hierarchy / level"` when the dim
/// has more than one hierarchy, otherwise just `"level"`.
#[must_use]
pub fn level_choices(schema: &Schema, dim: usize) -> Vec<LevelChoice> {
    let Some(d) = schema.dimensions.get(dim) else {
        return Vec::new();
    };
    let multi = d.hierarchies.len() > 1;
    d.hierarchies
        .iter()
        .enumerate()
        .flat_map(|(h_idx, h)| {
            h.levels.iter().enumerate().map(move |(l_idx, l)| LevelChoice {
                hierarchy: h_idx,
                level: l_idx,
                label: if multi {
                    format!("{} / {}", h.name, l.name)
                } else {
                    l.name.as_str().to_owned()
                },
            })
        })
        .collect()
}

/// Seed an axis to a dim's first-hierarchy / first-level when the user
/// picks a dim. When the dim has no levels, returns [`Pick::None`].
#[must_use]
pub fn axis_for(schema: &Schema, choice: Option<DimChoice>) -> Pick {
    let Some(choice) = choice else {
        return Pick::None;
    };
    let Some(d) = schema.dimensions.get(choice.index) else {
        return Pick::None;
    };
    if d.hierarchies.is_empty() || d.hierarchies[0].levels.is_empty() {
        return Pick::None;
    }
    Pick::Pick {
        dim: choice.index,
        hierarchy: 0,
        level: 0,
    }
}

/// Update an axis pick with a level choice. When the pick is absent or
/// the choice is `None`, returns [`Pick::None`] / the current state as
/// appropriate.
#[must_use]
pub fn level_for(current: Pick, choice: Option<LevelChoice>) -> Pick {
    match (current, choice) {
        (Pick::Pick { dim, .. }, Some(c)) => Pick::Pick {
            dim,
            hierarchy: c.hierarchy,
            level: c.level,
        },
        (Pick::Pick { .. }, None) => Pick::None,
        (Pick::None, _) => Pick::None,
    }
}
