use crate::data::IntoDatums;
use crate::data::axis::{self, Axis, Kind, Orientation, Placement};

pub mod label;
pub mod layout;
pub mod series;

pub use label::Label;
pub use layout::Layout;
pub use series::Series;

/// Creates a bar chart from one or more series.
///
/// # Examples
///
/// ```
/// use hyozu::{bars, bar};
///
/// // Single series
/// let chart = bars!([1200, 1900, 1500]);
///
/// // Multiple series (grouped by default)
/// let chart = bars!(
///     [1200, 1900, 1500],
///     [800, 1200, 1000],
/// );
///
/// // Multiple series with individual styling
/// let chart = bars!(
///     bar([1200, 1900, 1500]).with_color(0xFF5733),
///     bar([800, 1200, 1000]).with_color(0x33C3FF),
/// ).stacked();
/// ```
#[macro_export]
macro_rules! bars {
    // Single series
    ($data:expr) => {
        $crate::bars($data)
    };
    // Multiple series - each gets converted to Series via From
    ($($x:expr),+ $(,)?) => {
        $crate::Bars::from_series(vec![$($crate::bar::Series::from($x)),+])
    };
}

/// Direction of bar growth.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Direction {
    /// Bars grow upward from a horizontal baseline.
    #[default]
    Vertical,
    /// Bars grow rightward from a vertical baseline.
    Horizontal,
}

/// Proportional bar length - determines how much of available width bars occupy.
///
/// Value is clamped to [0.1, 1.0]:
/// - 1.0 = bars take 100% of available width (no padding)
/// - 0.75 = bars take 75% of available width (default)
/// - 0.5 = bars take 50% of available width
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Size(f32);

impl Size {
    /// Create a new bar Size, clamping value to [0.1, 1.0]
    pub fn new(value: f32) -> Self {
        Self(value.clamp(0.1, 1.0))
    }

    /// Get the inner value
    pub fn get(&self) -> f32 {
        self.0
    }
}

impl Default for Size {
    fn default() -> Self {
        Self(0.75)
    }
}

impl From<f32> for Size {
    fn from(value: f32) -> Self {
        Self::new(value)
    }
}

/// Spacing between bars in a group, expressed as proportion of bar width.
///
/// Value is clamped to [0.0, 1.0]:
/// - 0.0 = no spacing between bars
/// - 0.5 = half a bar's width of spacing between bars
/// - 1.0 = full bar's width of spacing between bars (default)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Spacing(f32);

impl Spacing {
    /// Create a new Spacing, clamping value to [0.0, 1.0]
    pub fn new(value: f32) -> Self {
        Self(value.clamp(0.0, 1.0))
    }

    /// Get the inner value
    pub fn get(&self) -> f32 {
        self.0
    }
}

impl Default for Spacing {
    fn default() -> Self {
        Self(0.0)
    }
}

impl From<f32> for Spacing {
    fn from(value: f32) -> Self {
        Self::new(value)
    }
}

/// Bar chart with support for multiple series, grouping, and stacking.
#[derive(Debug, Clone)]
pub struct Bars {
    /// The bar series to display.
    pub(crate) series: Vec<Series>,
    /// Layout strategy for multiple series.
    pub(crate) layout: Layout,
    /// Proportional bar length [0.1, 1.0] - how much of available width bars occupy
    pub(crate) size: Size,
    /// Proportional spacing [0.0, 1.0] - spacing between bars as proportion of bar width (grouped layout only)
    pub(crate) spacing: Spacing,
    /// Direction of bar growth (vertical or horizontal).
    pub(crate) direction: Direction,
}

/// Creates a single bar series with styling options.
///
/// Accepts various natural data formats:
/// - Point tuples: `[(0, 100), (1, 200)]`
/// - Point arrays: `[[0, 100], [1, 200]]`
/// - Just values (auto-enumerated): `[100, 200, 300]`
/// - Works with integers, floats, usize - whatever makes sense
///
/// # Examples
///
/// ```
/// use hyozu::bar;
///
/// let series = bar([1200, 1900, 1500]);
/// let styled = bar([800, 1200, 1000]).with_color(0xFF0000);
/// ```
pub fn bar(data: impl IntoDatums) -> Series {
    Series::new(data)
}

/// Creates a bar chart from one or more series.
///
/// # Examples
///
/// ```
/// use hyozu::{bars, bar};
///
/// // Single series (pass data directly)
/// let mark = bars([1200, 1900, 1500]);
///
/// // Multiple series (grouped by default)
/// let mark = bars([
///     bar([1200, 1900, 1500]),
///     bar([800, 1200, 1000]).with_color(0xFF0000),
/// ]);
///
/// // Stacked layout
/// let mark = bars([
///     bar([1200, 1900, 1500]),
///     bar([800, 1200, 1000]),
/// ]).stacked();
/// ```
pub fn bars(data: impl IntoBars) -> Bars {
    data.into_bars()
}

/// Trait for converting various inputs into Bars.
pub trait IntoBars {
    fn into_bars(self) -> Bars;
}

// Single series: bars([1200, 1900, 1500])
impl<T: IntoDatums> IntoBars for T {
    fn into_bars(self) -> Bars {
        Bars {
            series: vec![Series::new(self)],
            layout: Layout::default(),
            size: Size::default(),
            spacing: Spacing::default(),
            direction: Direction::default(),
        }
    }
}

// Multiple series: bars([bar(...), bar(...)])
impl<const N: usize> IntoBars for [Series; N] {
    fn into_bars(self) -> Bars {
        Bars {
            series: self.into(),
            layout: Layout::default(),
            size: Size::default(),
            spacing: Spacing::default(),
            direction: Direction::default(),
        }
    }
}

// Multiple series from Vec: bars(vec![bar(...), bar(...)])
impl IntoBars for Vec<Series> {
    fn into_bars(self) -> Bars {
        Bars {
            series: self,
            layout: Layout::default(),
            size: Size::default(),
            spacing: Spacing::default(),
            direction: Direction::default(),
        }
    }
}

impl Bars {
    /// Create a Bars from a vector of series (used by the macro).
    #[doc(hidden)]
    pub fn from_series(series: Vec<Series>) -> Self {
        Self {
            series,
            layout: Layout::default(),
            size: Size::default(),
            spacing: Spacing::default(),
            direction: Direction::default(),
        }
    }

    /// Set the layout to grouped (side-by-side).
    pub fn grouped(mut self) -> Self {
        self.layout = Layout::Grouped;
        self
    }

    /// Set the layout to stacked (cumulative).
    pub fn stacked(mut self) -> Self {
        self.layout = Layout::Stacked;
        self
    }

    /// Set the layout to overlaid (directly on top).
    pub fn overlaid(mut self) -> Self {
        self.layout = Layout::Overlaid;
        self
    }

    /// Sets the layout strategy explicitly.
    pub fn with_layout(mut self, layout: Layout) -> Self {
        self.layout = layout;
        self
    }

    /// Sets the proportional bar size - how much of available space bars occupy.
    ///
    /// Value is clamped to [0.1, 1.0]:
    /// - 1.0 = bars take 100% of available space (no padding)
    /// - 0.75 = bars take 75% of available space (default)
    /// - 0.5 = bars take 50% of available space
    ///
    /// # Examples
    /// ```ignore
    /// bars([100, 200, 300]).with_size(0.8)  // Bars take 80% of space
    /// ```
    pub fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }

    /// Sets the spacing between bars within a group (for grouped layout only).
    ///
    /// Value is clamped to [0.0, 1.0] and represents spacing as a proportion of bar size:
    /// - 0.0 = no spacing between bars (default)
    /// - 0.5 = half a bar's size of spacing between bars
    /// - 1.0 = full bar's size of spacing between bars
    ///
    /// For single-series bar charts, this has no effect since there's only one bar per category.
    /// For stacked and overlaid layouts, spacing has no effect.
    ///
    /// # Examples
    /// ```ignore
    /// bars![[100, 200], [150, 250]].with_spacing(0.2)  // 20% of bar width between bars
    /// ```
    pub fn with_spacing(mut self, spacing: impl Into<Spacing>) -> Self {
        self.spacing = spacing.into();
        self
    }

    /// Sets the bar size in place (for updating existing Bars).
    pub fn set_size(&mut self, size: impl Into<Size>) {
        self.size = size.into();
    }

    /// Sets the bar spacing in place (for updating existing Bars).
    pub fn set_spacing(&mut self, spacing: impl Into<Spacing>) {
        self.spacing = spacing.into();
    }

    /// Sets the direction to horizontal (bars grow rightward).
    pub fn horizontal(mut self) -> Self {
        self.direction = Direction::Horizontal;
        self
    }

    /// Sets the direction to vertical (bars grow upward).
    pub fn vertical(mut self) -> Self {
        self.direction = Direction::Vertical;
        self
    }

    /// Sets the bar direction explicitly.
    pub fn with_direction(mut self, direction: Direction) -> Self {
        self.direction = direction;
        self
    }

    /// Applies data label configuration to all series.
    pub fn data_labels(mut self, label: impl Into<Option<Label>>) -> Self {
        let label_config = label.into();
        for series in &mut self.series {
            series.label = label_config.clone();
        }
        self
    }

    /// Access all series (for iteration).
    pub fn all_series(&self) -> &[Series] {
        &self.series
    }

    /// Mutably access the series (for applying operations to all).
    pub fn series_mut(&mut self) -> &mut Vec<Series> {
        &mut self.series
    }

    // === Property getters ===

    /// Returns the layout strategy.
    pub fn layout(&self) -> Layout {
        self.layout
    }

    /// Returns the bar size (proportion of available width).
    pub fn size(&self) -> f32 {
        self.size.get()
    }

    /// Returns the spacing between bars in a group.
    pub fn spacing(&self) -> f32 {
        self.spacing.get()
    }

    /// Returns the bar direction.
    pub fn direction(&self) -> Direction {
        self.direction
    }

    /// Returns a reference to a specific series by index.
    pub fn series(&self, index: usize) -> Option<&Series> {
        self.series.get(index)
    }

    // === Axis factory methods ===

    /// Creates the appropriate x-axis for a bar chart.
    ///
    /// Bar charts have categorical x-axes with:
    /// - `Kind::Categorical` for proper bounds (±0.5 centering)
    /// - Labels placed on ticks (at bar centers)
    /// - Categorical tick style
    pub fn x_axis() -> Axis {
        Axis::new(Orientation::Bottom)
            .with_kind(Kind::Categorical)
            .labels(Placement::OnTicks)
            .with_ticks(axis::tick::Ticks::categorical())
    }

    /// Creates the appropriate y-axis for a bar chart.
    ///
    /// Bar charts have scalar y-axes with:
    /// - `Kind::ScalarAnchored` for proper bounds (anchored at 0 for bar baseline)
    /// - Continuous tick style
    pub fn y_axis() -> Axis {
        Axis::new(Orientation::Left)
            .with_kind(Kind::ScalarAnchored)
            .with_ticks(axis::tick::Ticks::continuous())
    }

    /// Creates axes appropriate for the given direction.
    ///
    /// For vertical bars: categorical x-axis, scalar y-axis (default).
    /// For horizontal bars: scalar x-axis (values), categorical y-axis (categories).
    pub fn axes(direction: Direction) -> (Axis, Axis) {
        match direction {
            Direction::Vertical => (Self::x_axis(), Self::y_axis()),
            Direction::Horizontal => (
                // x becomes scalar (values), y becomes categorical (categories)
                Axis::new(Orientation::Bottom)
                    .with_kind(Kind::ScalarAnchored)
                    .with_ticks(axis::tick::Ticks::continuous()),
                Axis::new(Orientation::Left)
                    .with_kind(Kind::Categorical)
                    .labels(Placement::OnTicks)
                    .with_ticks(axis::tick::Ticks::categorical()),
            ),
        }
    }
}

impl From<Bars> for crate::Data {
    fn from(bars: Bars) -> Self {
        use crate::data::IntoData;
        bars.into_data()
    }
}

impl<const N: usize> From<[Bars; N]> for crate::Data {
    fn from(bars: [Bars; N]) -> Self {
        crate::Data::from(bars.into_iter().map(crate::Mark::Bars).collect::<Vec<_>>())
    }
}
