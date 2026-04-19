//! [`tatami::Cell`] → plot-ready primitive converters.
//!
//! Two shapes:
//! - [`cell_f64`] — single `Cell` to `f64`. `Valid` returns the value;
//!   `Missing` returns [`f64::NAN`]; `Error` also returns [`f64::NAN`]. NaN
//!   is the universal "no value" signal hyozu marks tolerate (line charts
//!   break the line at NaN; bar charts drop the bar).
//! - [`cells_f64`] — vector shape, same per-cell rule.
//!
//! No `Option<f64>` returned — hyozu's mark builders consume bare `f64`, so
//! the adapter lifts NaN at this boundary rather than pushing `Option`
//! through every downstream call.

use tatami::Cell;

/// Convert a single [`Cell`] to `f64`. Missing and Error cells become
/// [`f64::NAN`], which hyozu's line / bar / choropleth marks interpret as
/// "no value at this point" (break the line, drop the bar, skip the fill).
#[must_use]
pub fn cell_f64(cell: &Cell) -> f64 {
    match cell {
        Cell::Valid { value, .. } => *value,
        Cell::Missing { .. } => f64::NAN,
        Cell::Error { .. } => f64::NAN,
        // `Cell` is `#[non_exhaustive]` — surface future variants as NaN so
        // chart construction never panics on an upstream tatami bump.
        _ => f64::NAN,
    }
}

/// Vector shape of [`cell_f64`]. Length preserved, element order preserved.
#[must_use]
pub fn cells_f64(cells: &[Cell]) -> Vec<f64> {
    cells.iter().map(cell_f64).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tatami::missing;

    #[test]
    fn valid_cell_returns_its_value() {
        let c = Cell::Valid {
            value: 42.0,
            unit: None,
            format: None,
        };
        assert_eq!(cell_f64(&c), 42.0);
    }

    #[test]
    fn missing_cell_returns_nan() {
        let c = Cell::Missing {
            reason: missing::Reason::NoFacts,
        };
        assert!(cell_f64(&c).is_nan());
    }

    #[test]
    fn error_cell_returns_nan() {
        let c = Cell::Error {
            message: "div/0".into(),
        };
        assert!(cell_f64(&c).is_nan());
    }

    #[test]
    fn cells_f64_preserves_length_and_order() {
        let cells = vec![
            Cell::Valid {
                value: 1.0,
                unit: None,
                format: None,
            },
            Cell::Missing {
                reason: missing::Reason::NoFacts,
            },
            Cell::Valid {
                value: 3.0,
                unit: None,
                format: None,
            },
        ];
        let out = cells_f64(&cells);
        assert_eq!(out.len(), 3);
        assert_eq!(out[0], 1.0);
        assert!(out[1].is_nan());
        assert_eq!(out[2], 3.0);
    }
}
