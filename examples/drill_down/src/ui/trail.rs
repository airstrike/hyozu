//! Navigation trail — a linear history of `tatami::Query` entries with a
//! cursor for back/forward.
//!
//! The `push` discipline is truncate-on-diverge (pushing while the cursor
//! is not at the end discards the future) and adjacent-duplicate no-op
//! (pushing a query identical to the current one leaves the trail
//! unchanged). These match icebreaker's `History` semantics and mirror the
//! pre-tatami draft of this example.

/// Ordered stack of queries plus a cursor pointing at the currently-active
/// entry.
///
/// Invariant: `entries` is never empty; `cursor < entries.len()`.
#[derive(Debug, Clone)]
pub struct Trail {
    entries: Vec<tatami::Query>,
    cursor: usize,
}

impl Trail {
    /// Construct a trail with a single initial entry; cursor at 0.
    #[must_use]
    pub fn new(initial: tatami::Query) -> Self {
        Self {
            entries: vec![initial],
            cursor: 0,
        }
    }

    /// The query at the cursor.
    #[must_use]
    pub fn current(&self) -> &tatami::Query {
        &self.entries[self.cursor]
    }

    /// Push a new query. If the cursor is not at the end, discards the
    /// forward history first. Adjacent-duplicate push (query equals
    /// `current()`) is a no-op.
    pub fn push(&mut self, q: tatami::Query) {
        if self.entries[self.cursor] == q {
            // Adjacent-duplicate: no-op. Drop any forward history too,
            // matching icebreaker's "committing to the branch" semantics.
            self.entries.truncate(self.cursor + 1);
            return;
        }
        self.entries.truncate(self.cursor + 1);
        self.entries.push(q);
        self.cursor += 1;
    }

    /// Move the cursor back one step. Returns `true` if the cursor moved.
    pub fn back(&mut self) -> bool {
        if self.cursor == 0 {
            return false;
        }
        self.cursor -= 1;
        true
    }

    /// Move the cursor forward one step. Returns `true` if the cursor moved.
    pub fn forward(&mut self) -> bool {
        if self.cursor + 1 >= self.entries.len() {
            return false;
        }
        self.cursor += 1;
        true
    }

    /// Whether `back()` would move the cursor.
    #[must_use]
    pub fn can_back(&self) -> bool {
        self.cursor > 0
    }

    /// Whether `forward()` would move the cursor.
    #[must_use]
    pub fn can_forward(&self) -> bool {
        self.cursor + 1 < self.entries.len()
    }

    /// Rebuild a trail from persisted entries. Empty input yields an
    /// empty-cursor panic-guarded default — callers should prefer
    /// [`Trail::new`] when they have a valid initial query.
    ///
    /// Returns `None` if `entries` is empty.
    #[must_use]
    pub fn from_data(entries: Vec<tatami::Query>) -> Option<Self> {
        if entries.is_empty() {
            return None;
        }
        let cursor = entries.len() - 1;
        Some(Self { entries, cursor })
    }

    /// Return the trail's entries for persistence.
    #[must_use]
    pub fn to_data(&self) -> Vec<tatami::Query> {
        self.entries.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tatami::query::{Options, Tuple};
    use tatami::schema::Name;
    use tatami::{Axes, Query};

    fn q(metric: &str) -> Query {
        Query {
            axes: Axes::Scalar,
            slicer: Tuple::empty(),
            metrics: vec![Name::parse(metric).expect("valid name")],
            options: Options::default(),
        }
    }

    #[test]
    fn new_sets_cursor_at_zero_and_returns_initial() {
        let trail = Trail::new(q("Revenue"));
        assert_eq!(trail.current().metrics[0].as_str(), "Revenue");
        assert!(!trail.can_back());
        assert!(!trail.can_forward());
    }

    #[test]
    fn push_advances_cursor_and_enables_back() {
        let mut trail = Trail::new(q("Revenue"));
        trail.push(q("Occupancy"));
        assert_eq!(trail.current().metrics[0].as_str(), "Occupancy");
        assert!(trail.can_back());
        assert!(!trail.can_forward());
    }

    #[test]
    fn push_adjacent_duplicate_is_a_noop() {
        let mut trail = Trail::new(q("Revenue"));
        trail.push(q("Revenue"));
        assert!(!trail.can_back(), "duplicate should not advance cursor");
    }

    #[test]
    fn push_after_back_truncates_forward_entries() {
        let mut trail = Trail::new(q("A"));
        trail.push(q("B"));
        trail.push(q("C"));
        assert!(trail.back());
        // cursor at B; push D → should drop C and land on D
        trail.push(q("D"));
        assert_eq!(trail.current().metrics[0].as_str(), "D");
        assert!(!trail.can_forward());
        assert!(trail.can_back());
    }

    #[test]
    fn back_forward_cursor_bookkeeping() {
        let mut trail = Trail::new(q("A"));
        trail.push(q("B"));
        trail.push(q("C"));

        assert!(trail.back());
        assert_eq!(trail.current().metrics[0].as_str(), "B");
        assert!(trail.back());
        assert_eq!(trail.current().metrics[0].as_str(), "A");
        assert!(!trail.back(), "already at start");

        assert!(trail.forward());
        assert_eq!(trail.current().metrics[0].as_str(), "B");
        assert!(trail.forward());
        assert_eq!(trail.current().metrics[0].as_str(), "C");
        assert!(!trail.forward(), "already at end");
    }

    #[test]
    fn from_data_roundtrips() {
        let mut t = Trail::new(q("A"));
        t.push(q("B"));
        let data = t.to_data();
        let rebuilt = Trail::from_data(data).expect("non-empty");
        assert_eq!(rebuilt.current().metrics[0].as_str(), "B");
    }

    #[test]
    fn from_data_empty_returns_none() {
        assert!(Trail::from_data(Vec::new()).is_none());
    }
}
