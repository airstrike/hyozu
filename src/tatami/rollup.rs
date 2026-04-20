//! [`tatami::rollup::Tree`] → [`crate::Data`] with [`crate::Mark::Choropleth`]
//! (and optional [`crate::Mark::BubbleMap`]).
//!
//! Rollups are hierarchical; choropleth renders one level (typically the
//! leaves, e.g. countries). The caller resolves each node's feature id
//! (the string that matches a polygon's `id` in the GeoJSON) via a closure.
//! The closure shape lets callers pick the right level — map world-level to
//! region codes, region-level to country codes, etc. — without the adapter
//! baking in a level policy.
//!
//! # Examples
//!
//! ```ignore
//! let data = hyozu::tatami::choropleth(&tree, |m| {
//!     // Use the deepest path segment as the feature id
//!     m.path.segments().last().map(|n| n.as_str().to_string())
//!            .unwrap_or_default()
//! });
//! ```

use tatami::rollup;

use crate::tatami::cell::cell_f64;
use crate::{ChoroplethEntry, Data, MapPoint, Mark, choropleth_entry, choropleth_entry_available, map_point};

/// Build [`Data`] with a single [`Mark::Choropleth`] from a rollup tree.
///
/// Walks the tree and collects one [`ChoroplethEntry`] per node whose
/// `feature_id` closure returned `Some`. Nodes whose closure returns `None`
/// are skipped — allowing the caller to restrict the choropleth to a
/// specific level (e.g. only leaves, only countries). [`tatami::Cell::Missing`]
/// and [`tatami::Cell::Error`] cells produce `NaN`-valued entries; hyozu's
/// choropleth renders NaN as unfilled.
#[must_use]
pub fn choropleth<F>(tree: &rollup::Tree, feature_id: F) -> Data
where
    F: Fn(&tatami::MemberRef) -> Option<String>,
{
    let entries = collect_choropleth_entries(tree, &feature_id);
    Mark::Choropleth(crate::choropleth(entries)).into()
}

/// Build [`Data`] with a single [`Mark::BubbleMap`] from a rollup tree.
///
/// Each node whose `centroid` closure returned `Some((lat, lon))` becomes a
/// bubble sized by the node's value. Nodes without a centroid are skipped.
/// Useful for centroid-marker overlays on choropleths (call both, combine
/// the marks into one `Data` via [`Vec<Mark>`] conversion).
#[must_use]
pub fn bubble_map<F>(tree: &rollup::Tree, centroid: F) -> Data
where
    F: Fn(&tatami::MemberRef) -> Option<(f64, f64)>,
{
    let points = collect_map_points(tree, &centroid);
    Mark::BubbleMap(crate::bubble_map(points)).into()
}

fn collect_choropleth_entries<F>(tree: &rollup::Tree, feature_id: &F) -> Vec<ChoroplethEntry>
where
    F: Fn(&tatami::MemberRef) -> Option<String>,
{
    let mut out = Vec::new();
    push_choropleth_nodes(tree, feature_id, &mut out);
    out
}

fn push_choropleth_nodes<F>(tree: &rollup::Tree, feature_id: &F, out: &mut Vec<ChoroplethEntry>)
where
    F: Fn(&tatami::MemberRef) -> Option<String>,
{
    if let Some(id) = feature_id(&tree.root) {
        let v = cell_f64(&tree.value);
        let entry = if v.is_finite() {
            choropleth_entry(id, v)
        } else {
            // Missing/Error cells become "available, no value" — the
            // feature is part of the active context but carries no
            // measurement to encode on the gradient.
            choropleth_entry_available(id)
        };
        out.push(entry);
    }
    for child in &tree.children {
        push_choropleth_nodes(child, feature_id, out);
    }
}

fn collect_map_points<F>(tree: &rollup::Tree, centroid: &F) -> Vec<MapPoint>
where
    F: Fn(&tatami::MemberRef) -> Option<(f64, f64)>,
{
    let mut out = Vec::new();
    push_map_points(tree, centroid, &mut out);
    out
}

fn push_map_points<F>(tree: &rollup::Tree, centroid: &F, out: &mut Vec<MapPoint>)
where
    F: Fn(&tatami::MemberRef) -> Option<(f64, f64)>,
{
    if let Some((lat, lon)) = centroid(&tree.root) {
        let value = cell_f64(&tree.value);
        // Bubble builders want non-NaN magnitudes; NaN becomes a zero-value
        // point (hyozu clamps to `min_radius`) so the bubble still renders
        // at the centroid but minimally.
        let magnitude = if value.is_nan() { 0.0 } else { value };
        out.push(map_point(lat, lon, magnitude));
    }
    for child in &tree.children {
        push_map_points(child, centroid, out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tatami::query::{MemberRef, Path};
    use tatami::schema::Name;
    use tatami::{Cell, rollup};

    fn name(s: &str) -> Name {
        Name::parse(s).expect("valid")
    }

    fn mr(head: &str) -> MemberRef {
        MemberRef::new(name("Geography"), name("Default"), Path::of(name(head)))
    }

    fn valid(v: f64) -> Cell {
        Cell::Valid {
            value: v,
            unit: None,
            format: None,
        }
    }

    fn leaf(head: &str, v: f64) -> rollup::Tree {
        rollup::Tree {
            root: mr(head),
            value: valid(v),
            children: Vec::new(),
        }
    }

    /// Build a node at a two-segment path like `["World", head]` to mimic a
    /// real drill result where leaves sit under the root's path prefix.
    fn under_world(head: &str, v: f64) -> rollup::Tree {
        rollup::Tree {
            root: MemberRef::new(
                name("Geography"),
                name("Default"),
                Path::with(name("World"), [name(head)]),
            ),
            value: valid(v),
            children: Vec::new(),
        }
    }

    #[test]
    fn choropleth_collects_one_entry_per_mapped_node() {
        let tree = rollup::Tree {
            root: mr("World"),
            value: valid(6.0),
            children: vec![under_world("US", 2.0), under_world("UK", 3.0), under_world("JP", 1.0)],
        };
        // Leaf-only filter: return id for nodes with a non-empty tail; root
        // (path = ["World"], empty tail) returns None and is skipped.
        let data = choropleth(&tree, |m| {
            if m.path.tail().is_empty() {
                None
            } else {
                m.path.segments().last().map(|n| n.as_str().to_string())
            }
        });
        assert_eq!(data.marks().len(), 1);
        if let Mark::Choropleth(c) = &data.marks()[0] {
            assert_eq!(c.entries().len(), 3);
        } else {
            panic!("expected Mark::Choropleth");
        }
    }

    #[test]
    fn bubble_map_skips_nodes_without_centroids() {
        let tree = rollup::Tree {
            root: mr("World"),
            value: valid(6.0),
            children: vec![leaf("US", 2.0), leaf("UK", 3.0)],
        };
        // Only provide a centroid for "US"; UK and World should be skipped.
        let data = bubble_map(&tree, |m| {
            if m.path.segments().last().map(|n| n.as_str()) == Some("US") {
                Some((39.8, -98.6))
            } else {
                None
            }
        });
        assert_eq!(data.marks().len(), 1);
        if let Mark::BubbleMap(bm) = &data.marks()[0] {
            assert_eq!(bm.points().len(), 1);
        } else {
            panic!("expected Mark::BubbleMap");
        }
    }

    #[test]
    fn choropleth_nan_flows_through_for_missing_cells() {
        let tree = rollup::Tree {
            root: mr("World"),
            value: Cell::Missing {
                reason: tatami::missing::Reason::NoFacts,
            },
            children: Vec::new(),
        };
        // Root-only: include it regardless of path depth.
        let data = choropleth(&tree, |m| m.path.head().as_str().to_string().into());
        if let Mark::Choropleth(c) = &data.marks()[0] {
            let entries = c.entries();
            assert_eq!(entries.len(), 1);
            // Missing cell lifts to NaN through the adapter's cell_f64 hop.
            // Missing cells are now lifted to "available" entries
            // (value = None) so the choropleth renderer treats them as
            // muted neutral fill, not gradient-encoded NaN.
            assert!(entries[0].value.is_none());
        } else {
            panic!("expected Mark::Choropleth");
        }
    }
}
