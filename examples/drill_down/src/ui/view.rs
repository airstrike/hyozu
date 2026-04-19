//! View mode — the measure and period the dashboard currently displays.
//!
//! Held on [`crate::ui::Dashboard`] outside the [`crate::ui::Trail`] so
//! back-navigation through drill/filter history preserves the current view.

/// The metric family on display.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum Measure {
    /// Revenue (USD). Default.
    #[default]
    Revenue,
    /// Occupancy ratio.
    Occupancy,
    /// Average daily rate (USD).
    Adr,
    /// Revenue per available room (USD).
    RevPar,
}

/// The time-axis granularity for the time-series panel.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum Period {
    /// Daily grain.
    Day,
    /// Weekly grain. Default.
    #[default]
    Week,
    /// Monthly grain.
    Month,
}

/// The paired view state — measure and period.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct View {
    /// The currently-selected measure family.
    pub measure: Measure,
    /// The currently-selected time-axis grain.
    pub period: Period,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_view_is_revenue_and_week() {
        let v = View::default();
        assert_eq!(v.measure, Measure::Revenue);
        assert_eq!(v.period, Period::Week);
    }
}
