//! Domains a logarithmic axis can map.
//!
//! [`Transform::Log`](super::Transform::Log) is undefined at and below
//! zero. Rather than let a nonpositive bound reach `log10` (and come back
//! as NaN or a sixteen-decade axis), log axes derive their range through
//! [`Domain`], whose constructor rejects such bounds with an [`Error`] the
//! caller handles explicitly.

/// A range a log axis can map: both ends finite, strictly positive and
/// ordered (`lo <= hi`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Domain {
    lo: f64,
    hi: f64,
}

/// Why a pair of bounds can't be a log [`Domain`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Error {
    /// A bound is NaN or infinite. Also returned when there is no positive
    /// datum at all, since the running minimum stays at infinity.
    NonFinite { lo: f64, hi: f64 },
    /// The lower bound is zero or negative.
    NonPositive { lo: f64 },
    /// The lower bound exceeds the upper bound.
    Inverted { lo: f64, hi: f64 },
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NonFinite { lo, hi } => write!(f, "log domain bounds must be finite, got [{lo}, {hi}]"),
            Self::NonPositive { lo } => write!(f, "log domain lower bound must be positive, got {lo}"),
            Self::Inverted { lo, hi } => write!(f, "log domain is inverted: {lo} > {hi}"),
        }
    }
}

impl std::error::Error for Error {}

impl Domain {
    /// Validates `[lo, hi]` as a log domain. A degenerate `lo == hi`
    /// domain is accepted; [`Domain::nice`] widens it to a full decade.
    pub fn new(lo: f64, hi: f64) -> Result<Self, Error> {
        if !lo.is_finite() || !hi.is_finite() {
            Err(Error::NonFinite { lo, hi })
        } else if lo <= 0.0 {
            Err(Error::NonPositive { lo })
        } else if lo > hi {
            Err(Error::Inverted { lo, hi })
        } else {
            Ok(Self { lo, hi })
        }
    }

    /// Fallback range for a log axis with no positive data and no usable
    /// explicit bounds: one decade, `[1, 10]`.
    pub fn unit_decade() -> Self {
        Self { lo: 1.0, hi: 10.0 }
    }

    /// Rounds the domain outward to decade boundaries: `lo` down to
    /// `10^floor(log10(lo))` and `hi` up to `10^ceil(log10(hi))`. A domain
    /// that would collapse to a single power of ten grows upward by one
    /// decade so the axis keeps a visible span.
    pub fn nice(self) -> Self {
        let lo = 10f64.powi(self.lo.log10().floor() as i32);
        let mut hi = 10f64.powi(self.hi.log10().ceil() as i32);
        if hi <= lo {
            hi = lo * 10.0;
        }
        Self { lo, hi }
    }

    /// The lower bound (always positive).
    pub fn lo(&self) -> f64 {
        self.lo
    }

    /// The upper bound.
    pub fn hi(&self) -> f64 {
        self.hi
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9 * a.abs().max(b.abs()).max(1.0)
    }

    #[test]
    fn rejects_nonpositive_lower_bound() {
        assert_eq!(Domain::new(0.0, 10.0), Err(Error::NonPositive { lo: 0.0 }));
        assert_eq!(Domain::new(-3.0, 10.0), Err(Error::NonPositive { lo: -3.0 }));
    }

    #[test]
    fn rejects_non_finite_and_inverted_bounds() {
        assert!(matches!(Domain::new(f64::INFINITY, 10.0), Err(Error::NonFinite { .. })));
        assert!(matches!(Domain::new(1.0, f64::NAN), Err(Error::NonFinite { .. })));
        assert_eq!(Domain::new(100.0, 10.0), Err(Error::Inverted { lo: 100.0, hi: 10.0 }));
    }

    #[test]
    fn nice_snaps_outward_to_decades() {
        let d = Domain::new(2.5, 8500.0).unwrap().nice();
        assert!(close(d.lo(), 1.0));
        assert!(close(d.hi(), 10_000.0));
    }

    #[test]
    fn nice_handles_sub_unit_domains() {
        let d = Domain::new(0.003, 0.4).unwrap().nice();
        assert!(close(d.lo(), 0.001), "lo was {}", d.lo());
        assert!(close(d.hi(), 1.0), "hi was {}", d.hi());
    }

    #[test]
    fn nice_is_idempotent_at_decades() {
        let d = Domain::new(10.0, 1e6).unwrap().nice();
        assert!(close(d.lo(), 10.0));
        assert!(close(d.hi(), 1e6));
        assert_eq!(d.nice(), d);
    }

    #[test]
    fn nice_widens_a_single_power_of_ten() {
        let d = Domain::new(100.0, 100.0).unwrap().nice();
        assert!(close(d.lo(), 100.0));
        assert!(close(d.hi(), 1000.0));
    }
}
