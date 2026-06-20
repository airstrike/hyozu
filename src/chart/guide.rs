use crate::axis::{Alignment, Axis, Bounds, Kind, Orientation, TextAlign};
use crate::core::layout::{Limits, Node};
use crate::core::text::{self, paragraph};
use crate::core::widget::{Tree, tree};
use crate::core::{Point, Size};

/// Wrapper for f64 to make it orderable (NaN compares as less than everything)
#[derive(Debug, Clone, Copy)]
struct OrderedFloat(f64);

impl PartialEq for OrderedFloat {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0 || (self.0.is_nan() && other.0.is_nan())
    }
}

impl Eq for OrderedFloat {}

impl PartialOrd for OrderedFloat {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for OrderedFloat {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.partial_cmp(&other.0).unwrap_or(std::cmp::Ordering::Less)
    }
}

/// Format a tick value with an explicit decimal precision derived from
/// the tick step size. This is the D3-style approach: if the step is 0.2
/// we need 1 decimal place; if 0.05, 2 places; if 5, 0 places.
fn format_with_precision(value: f64, precision: Option<usize>) -> String {
    match precision {
        Some(p) => format!("{:.prec$}", value, prec = p),
        None => {
            // Fallback: guess from the value itself
            if value == 0.0 || (value.round() - value).abs() < 1e-9 {
                format!("{:.0}", value)
            } else {
                // Show enough decimals to be meaningful, but not floating-point garbage
                let s = format!("{:.10}", value);
                s.trim_end_matches('0').trim_end_matches('.').to_string()
            }
        }
    }
}

/// Compute the number of decimal places needed to cleanly display a
/// given tick step. E.g. step=0.2 → 1, step=0.05 → 2, step=5 → 0.
fn precision_for_step(step: f64) -> usize {
    if step <= 0.0 || !step.is_finite() {
        return 1;
    }
    let p = (-step.log10()).ceil() as i32;
    p.max(0) as usize
}

/// Minimum pixel spacing between adjacent tick labels, in the axis's
/// major direction. Drives the target tick count so short axes get
/// fewer labels and long ones get more.
///
/// Per-label footprints are smaller on vertical axes (labels stack
/// top-to-bottom, single-line height) than horizontal ones (labels sit
/// left-to-right, full content width + breathing room), and time labels
/// are wider than numeric ones.
fn min_label_spacing(kind: Kind, orientation: Orientation) -> f32 {
    let is_vertical = matches!(orientation, Orientation::Left | Orientation::Right);
    match (kind, is_vertical) {
        (Kind::Time, true) => 40.0,
        (Kind::Time, false) => 90.0,
        (_, true) => 36.0,
        (_, false) => 70.0,
    }
}

/// Derive a target tick count from the axis's pixel extent.
///
/// Returns `6` (the historical default) when the extent is unknown,
/// zero, non-finite, or negative — this is hit during `state`/`diff`,
/// before layout has a size to work with. Otherwise divides the extent
/// by the per-label pixel footprint and clamps the result to `[2, 12]`
/// so sparse ranges don't collapse to a single tick and wide axes don't
/// explode into unreadable gridline forests.
fn target_tick_count(axis_length: Option<f32>, kind: Kind, orientation: Orientation) -> usize {
    const DEFAULT: usize = 6;
    const MIN: usize = 2;
    const MAX: usize = 12;

    let Some(length) = axis_length else {
        return DEFAULT;
    };
    if !length.is_finite() || length <= 0.0 {
        return DEFAULT;
    }

    // Categorical axes don't route through nice_ticks in the main code
    // path (positions come from the data itself), but if a caller ever
    // does use target_tick_count with one, keep the historical default
    // rather than derive a potentially surprising count.
    if matches!(kind, Kind::Categorical) {
        return DEFAULT;
    }

    let spacing = min_label_spacing(kind, orientation);
    let raw = (length / spacing).floor() as i64;
    raw.clamp(MIN as i64, MAX as i64) as usize
}

/// Snap a numeric domain outward to decade boundaries for log-axis use.
/// Non-positive bounds clamp to `f64::EPSILON` first.
fn log_nice_bounds(min: f64, max: f64) -> (f64, f64) {
    let lo = min.max(f64::EPSILON);
    let hi = max.max(lo * 10.0);
    let lo_dec = 10_f64.powi(lo.log10().floor() as i32);
    let hi_dec = 10_f64.powi(hi.log10().ceil() as i32);
    (lo_dec, hi_dec)
}

/// Generate log-axis ticks at integer powers of 10 within `[min, max]`.
///
/// Non-positive lower bounds are nonsensical on a log axis (the
/// transform clamps to `f64::EPSILON ≈ 2.22e-16`); falling back to that
/// for tick generation would emit a tick for every decade down to
/// `10^-15`, most rounding to `"0"` in the label formatter and sitting
/// in pure empty space. Instead, when `min ≤ 0` we use `hi / 10^4` as
/// the effective lower so the axis shows roughly four decades of ticks.
///
/// For any range, we cap the tick count at ~8 by striding decades when
/// the total span exceeds that — wider domains stay readable instead of
/// crowding labels.
///
/// The result always contains at least one entry — when the domain
/// spans less than a full decade the function falls back to
/// `[min, max]` so the axis still renders labelled endpoints.
fn log_ticks(min: f64, max: f64) -> Vec<f64> {
    if !(min.is_finite() && max.is_finite()) || min >= max {
        return vec![min];
    }
    let hi = max.max(f64::EPSILON);
    let lo = if min > 0.0 { min } else { hi / 1e4 };
    let lo_exp = lo.log10().ceil() as i32;
    let hi_exp = hi.log10().floor() as i32;
    if lo_exp > hi_exp {
        return vec![min, max];
    }
    const MAX_TICKS: usize = 8;
    let count = (hi_exp - lo_exp + 1) as usize;
    let stride = count.div_ceil(MAX_TICKS).max(1);
    (lo_exp..=hi_exp).step_by(stride).map(|k| 10_f64.powi(k)).collect()
}

use jiff::Timestamp;
use jiff::tz::TimeZone;

/// Time unit for hierarchical time intervals (like D3.js/Chart.js)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TimeUnit {
    Second,
    Minute,
    Hour,
    Day,
    Month,
    Year,
}

impl TimeUnit {
    /// Get the format string for labels at this time scale
    fn format_str(self) -> &'static str {
        match self {
            TimeUnit::Second => "%H:%M:%S",
            TimeUnit::Minute | TimeUnit::Hour => "%H:%M",
            TimeUnit::Day => "%b %d",   // "Jun 15"
            TimeUnit::Month => "%b %Y", // "Jun 2024"
            TimeUnit::Year => "%Y",     // "2024"
        }
    }
}

/// A time interval with unit and multiplier (e.g., 15 minutes, 2 hours, 3 months)
#[derive(Debug, Clone, Copy)]
struct TimeInterval {
    unit: TimeUnit,
    count: i64,
}

impl TimeInterval {
    const fn new(unit: TimeUnit, count: i64) -> Self {
        Self { unit, count }
    }

    /// Approximate duration in seconds (for comparison/selection)
    fn approx_seconds(&self) -> f64 {
        let base = match self.unit {
            TimeUnit::Second => 1.0,
            TimeUnit::Minute => 60.0,
            TimeUnit::Hour => 3600.0,
            TimeUnit::Day => 86400.0,
            TimeUnit::Month => 30.0 * 86400.0, // approximate
            TimeUnit::Year => 365.0 * 86400.0, // approximate
        };
        base * self.count as f64
    }

    /// Floor a unix timestamp (seconds) to this interval's boundary using jiff
    fn floor(&self, timestamp_secs: i64) -> i64 {
        let ts = Timestamp::from_second(timestamp_secs).unwrap_or(Timestamp::UNIX_EPOCH);
        let zoned = ts.to_zoned(TimeZone::UTC);

        match self.unit {
            TimeUnit::Second => {
                // Floor to interval multiple of seconds
                let step = self.count;
                timestamp_secs - (timestamp_secs % step)
            }
            TimeUnit::Minute => {
                // Floor to interval multiple of minutes from midnight
                let midnight = zoned.start_of_day().unwrap_or(zoned).timestamp().as_second();
                let secs_since_midnight = timestamp_secs - midnight;
                let step = self.count * 60;
                let aligned = (secs_since_midnight / step) * step;
                midnight + aligned
            }
            TimeUnit::Hour => {
                // Floor to interval multiple of hours from midnight
                let midnight = zoned.start_of_day().unwrap_or(zoned).timestamp().as_second();
                let secs_since_midnight = timestamp_secs - midnight;
                let step = self.count * 3600;
                let aligned = (secs_since_midnight / step) * step;
                midnight + aligned
            }
            TimeUnit::Day => {
                // Floor to start of day, then align to interval
                let midnight = zoned.start_of_day().unwrap_or(zoned);
                let days_since_epoch = midnight.timestamp().as_second() / 86400;
                let aligned_days = (days_since_epoch / self.count) * self.count;
                aligned_days * 86400
            }
            TimeUnit::Month => {
                // Floor to start of month, aligned to interval
                let dt = zoned.datetime();
                let month = dt.month();
                let aligned_month = ((month - 1) / self.count as i8) * self.count as i8 + 1;
                jiff::civil::date(dt.year(), aligned_month, 1)
                    .to_zoned(TimeZone::UTC)
                    .unwrap()
                    .timestamp()
                    .as_second()
            }
            TimeUnit::Year => {
                // Floor to start of year, aligned to interval
                let year = zoned.datetime().year();
                let aligned_year = (year / self.count as i16) * self.count as i16;
                jiff::civil::date(aligned_year, 1, 1)
                    .to_zoned(TimeZone::UTC)
                    .unwrap()
                    .timestamp()
                    .as_second()
            }
        }
    }

    /// Advance a timestamp by this interval using jiff
    fn advance(&self, timestamp_secs: i64) -> i64 {
        let ts = Timestamp::from_second(timestamp_secs).unwrap_or(Timestamp::UNIX_EPOCH);
        let zoned = ts.to_zoned(TimeZone::UTC);

        let span = match self.unit {
            TimeUnit::Second => jiff::Span::new().seconds(self.count),
            TimeUnit::Minute => jiff::Span::new().minutes(self.count),
            TimeUnit::Hour => jiff::Span::new().hours(self.count),
            TimeUnit::Day => jiff::Span::new().days(self.count),
            TimeUnit::Month => jiff::Span::new().months(self.count),
            TimeUnit::Year => jiff::Span::new().years(self.count),
        };

        let advanced = zoned.checked_add(span).unwrap_or(zoned);
        advanced.timestamp().as_second()
    }

    /// Move backward by one interval step
    fn retreat(&self, timestamp_secs: i64) -> i64 {
        let ts = Timestamp::from_second(timestamp_secs).unwrap_or(Timestamp::UNIX_EPOCH);
        let zoned = ts.to_zoned(TimeZone::UTC);

        let span = match self.unit {
            TimeUnit::Second => jiff::Span::new().seconds(self.count),
            TimeUnit::Minute => jiff::Span::new().minutes(self.count),
            TimeUnit::Hour => jiff::Span::new().hours(self.count),
            TimeUnit::Day => jiff::Span::new().days(self.count),
            TimeUnit::Month => jiff::Span::new().months(self.count),
            TimeUnit::Year => jiff::Span::new().years(self.count),
        };

        let retreated = zoned.checked_sub(span).unwrap_or(zoned);
        retreated.timestamp().as_second()
    }

    /// Format a timestamp using the appropriate format for this interval's unit
    fn format(&self, timestamp_secs: i64) -> String {
        let ts = Timestamp::from_second(timestamp_secs).unwrap_or(Timestamp::UNIX_EPOCH);
        let zoned = ts.to_zoned(TimeZone::UTC);
        zoned.strftime(self.unit.format_str()).to_string()
    }
}

/// Standard time intervals following D3.js conventions
const TIME_INTERVALS: &[TimeInterval] = &[
    TimeInterval::new(TimeUnit::Second, 1),
    TimeInterval::new(TimeUnit::Second, 5),
    TimeInterval::new(TimeUnit::Second, 10),
    TimeInterval::new(TimeUnit::Second, 15),
    TimeInterval::new(TimeUnit::Second, 30),
    TimeInterval::new(TimeUnit::Minute, 1),
    TimeInterval::new(TimeUnit::Minute, 2),
    TimeInterval::new(TimeUnit::Minute, 5),
    TimeInterval::new(TimeUnit::Minute, 10),
    TimeInterval::new(TimeUnit::Minute, 15),
    TimeInterval::new(TimeUnit::Minute, 30),
    TimeInterval::new(TimeUnit::Hour, 1),
    TimeInterval::new(TimeUnit::Hour, 2),
    TimeInterval::new(TimeUnit::Hour, 3),
    TimeInterval::new(TimeUnit::Hour, 4),
    TimeInterval::new(TimeUnit::Hour, 6),
    TimeInterval::new(TimeUnit::Hour, 12),
    TimeInterval::new(TimeUnit::Day, 1),
    TimeInterval::new(TimeUnit::Day, 2),
    TimeInterval::new(TimeUnit::Day, 7),  // week
    TimeInterval::new(TimeUnit::Day, 14), // 2 weeks
    TimeInterval::new(TimeUnit::Month, 1),
    TimeInterval::new(TimeUnit::Month, 2),
    TimeInterval::new(TimeUnit::Month, 3), // quarter
    TimeInterval::new(TimeUnit::Month, 6), // half year
    TimeInterval::new(TimeUnit::Year, 1),
    TimeInterval::new(TimeUnit::Year, 2),
    TimeInterval::new(TimeUnit::Year, 5),
    TimeInterval::new(TimeUnit::Year, 10),
    TimeInterval::new(TimeUnit::Year, 20),
    TimeInterval::new(TimeUnit::Year, 50),
    TimeInterval::new(TimeUnit::Year, 100),
];

/// Select the best time interval for a given range and target tick count
fn select_time_interval(range_seconds: f64, target_count: usize) -> TimeInterval {
    let target_step = range_seconds / target_count as f64;

    for interval in TIME_INTERVALS {
        if interval.approx_seconds() >= target_step {
            return *interval;
        }
    }

    // Fall back to centuries for very large ranges
    *TIME_INTERVALS.last().unwrap()
}

/// Generate nice time-aligned tick positions using hierarchical intervals.
/// Returns (tick_timestamps_i64, selected_interval) for proper formatting.
fn nice_time_ticks(min: f64, max: f64, target_count: usize, alignment: Alignment) -> (Vec<i64>, TimeInterval) {
    if min >= max {
        return (vec![min as i64], TIME_INTERVALS[0]);
    }

    let range = max - min;
    let interval = select_time_interval(range, target_count);

    // Use i64 throughout for precision
    let min_i64 = min as i64;
    let max_i64 = max as i64;

    let mut ticks = Vec::new();

    match alignment {
        Alignment::Auto => {
            // Floor min to interval boundary, then iterate
            let mut value = interval.floor(min_i64);

            // Advance until we're >= min
            while value < min_i64 {
                value = interval.advance(value);
            }

            // Collect ticks until we exceed max
            while value <= max_i64 {
                ticks.push(value);
                value = interval.advance(value);
            }
        }
        Alignment::SnapToStart => {
            // Start exactly at min, advance by interval
            let mut value = min_i64;
            while value <= max_i64 {
                ticks.push(value);
                value = interval.advance(value);
            }
        }
        Alignment::SnapToEnd => {
            // Work backward from max
            let mut value = max_i64;
            while value >= min_i64 {
                ticks.push(value);
                value = interval.retreat(value);
            }
            ticks.reverse();
        }
    }

    // Ensure we have at least one tick
    if ticks.is_empty() {
        ticks.push(min_i64);
    }

    (ticks, interval)
}

/// State for a Guide - stores paragraphs for text measurement
pub struct State<P>
where
    P: text::Paragraph,
{
    /// Paragraphs for each text label in the guide
    pub labels: Vec<paragraph::Plain<P>>,
    /// Cached label info (position, text) - computed once at diff
    pub label_info: Vec<(f64, String)>,
    /// Cached tick positions - computed once at diff
    pub tick_positions: Vec<f64>,
    /// Cached bounds from tick marks - for coordinate system
    pub bounds: Bounds,
    /// Edge label insets to prevent overhang at axis boundaries.
    /// For horizontal axes: (left, right). For vertical axes: (top, bottom).
    pub label_insets: (f32, f32),
    pub tick_space: f32,
    pub label_offset: f32,
}

/// A Guide wraps an Axis and handles UI layout with proper text measurement.
///
/// Similar to pane_grid::Content, this borrows the data (Axis) and is
/// widget-like but doesn't implement Widget. Keeps UI code separate from data viz.
pub struct Guide<'a, Message, Renderer>
where
    Message: 'a,
    Renderer: text::Renderer<Font = crate::core::Font>,
{
    axis: &'a Axis,
    marks: &'a [crate::Mark],
    /// Transform applied to the numeric domain when generating ticks.
    /// Categorical and time axes ignore this; only Scalar axes route
    /// through it.
    transform: crate::scale::Transform,
    /// Whether numeric-axis bounds should round outward to nice numbers
    /// when no explicit bounds are set on the axis. `true` matches
    /// historical behavior (`Kind::Scalar.bounds()`); `false` keeps the
    /// raw data range so the user's reading lines up byte-for-byte with
    /// the numbers they passed in.
    nice: bool,
    _renderer: std::marker::PhantomData<(Message, Renderer)>,
}

impl<'a, Message, Renderer> Guide<'a, Message, Renderer>
where
    Message: 'a,
    Renderer: text::Renderer<Font = crate::core::Font>,
{
    /// Create a new Guide borrowing an Axis and marks. Defaults to
    /// [`crate::scale::Transform::Linear`]; use [`Guide::with_transform`]
    /// to override for log-axis support.
    pub fn new(axis: &'a Axis, marks: &'a [crate::Mark]) -> Self {
        Self {
            axis,
            marks,
            transform: crate::scale::Transform::default(),
            nice: true,
            _renderer: std::marker::PhantomData,
        }
    }

    /// Sets the numeric-domain transform used for tick generation. For
    /// [`crate::scale::Transform::Log`], ticks land on integer powers of
    /// 10 within the domain.
    pub fn with_transform(mut self, transform: crate::scale::Transform) -> Self {
        self.transform = transform;
        self
    }

    /// Sets whether the auto-derived axis bounds should round outward to
    /// nice numbers. Mirrors [`crate::scale::Scale::nice`]; `true` is the
    /// historical default. Has no effect when both bounds on the axis
    /// are explicit.
    pub fn with_nice(mut self, nice: bool) -> Self {
        self.nice = nice;
        self
    }

    /// Returns a reference to the underlying axis.
    pub fn axis(&self) -> &'a Axis {
        self.axis
    }

    /// Compute the axis bounds (visual range) for this axis.
    ///
    /// Uses `Kind::bounds()` to apply kind-appropriate normalization (padding,
    /// nice numbers, zero-anchoring) based on the axis kind. For log-transformed
    /// axes, falls back to decade-aligned bounds instead so the nice-step path
    /// doesn't snap the lower bound to zero (which has no place on a log axis).
    fn compute_axis_bounds(&self) -> Bounds {
        let is_x_axis = matches!(self.axis.orientation(), Orientation::Bottom | Orientation::Top);

        // Get data range
        let (data_min, data_max) = self.find_range(is_x_axis);

        // Check for explicit user bounds
        let has_explicit_bounds = self.axis.lower_bound.is_some() && self.axis.upper_bound.is_some();

        if has_explicit_bounds {
            let lower = self.axis.lower_bound.unwrap();
            let upper = self.axis.upper_bound.unwrap();
            // Log axes can't show non-positive values; the transform
            // clamps to `f64::EPSILON` for mapping anyway, which spans
            // ~16 decades of empty space below any real data. Clamp the
            // user's lower to a positive default derived from `upper`
            // so tick positioning, gridlines, and the rendered plane
            // all agree on the same range.
            if matches!(self.transform, crate::scale::Transform::Log) && lower <= 0.0 && upper > 0.0 {
                Bounds::exact(upper / 1e4, upper)
            } else {
                Bounds::exact(lower, upper)
            }
        } else {
            // Apply partial overrides if any, then let Kind compute proper bounds
            let min = self.axis.lower_bound.unwrap_or(data_min);
            let max = self.axis.upper_bound.unwrap_or(data_max);
            if matches!(self.transform, crate::scale::Transform::Log) {
                if self.nice {
                    let (lo, hi) = log_nice_bounds(min, max);
                    Bounds::exact(lo, hi)
                } else {
                    Bounds::exact(min, max)
                }
            } else if self.nice {
                self.axis.kind().bounds(min, max)
            } else {
                Bounds::exact(min, max)
            }
        }
    }

    /// Derive tick positions, label positions, and label text from data
    /// Returns (label_info, tick_positions) based on label placement.
    ///
    /// `axis_length` is the pixel extent available along the axis when
    /// known (during `layout`). Used to derive a comfortable target tick
    /// count from the axis footprint. `None` during `state`/`diff`, which
    /// run before layout — we fall back to a fixed default in that case.
    fn ticks_and_labels(&self, bounds: Bounds, axis_length: Option<f32>) -> (Vec<(f64, String)>, Vec<f64>) {
        use crate::axis::label;
        use crate::axis::tick::Frequency;

        let is_categorical = self.axis.is_categorical();

        let (axis_min, axis_max) = (bounds.min(), bounds.max());

        let (data_positions, nice_step) = if is_categorical {
            // Categorical: collect unique X positions from data
            use std::collections::BTreeSet;
            let mut values: BTreeSet<OrderedFloat> = BTreeSet::new();

            for mark in self.marks {
                match mark {
                    crate::Mark::Area(area) => {
                        for series in &area.series {
                            for point in &series.points {
                                values.insert(OrderedFloat(point.x));
                            }
                        }
                    }
                    crate::Mark::Bars(bars) => {
                        for series in &bars.series {
                            for point in &series.points {
                                values.insert(OrderedFloat(point.x));
                            }
                        }
                    }
                    crate::Mark::Line(line) => {
                        for point in &line.points {
                            values.insert(OrderedFloat(point.x));
                        }
                    }
                    crate::Mark::Waterfall(wf) => {
                        for (i, _) in wf.entries.iter().enumerate() {
                            values.insert(OrderedFloat(i as f64));
                        }
                    }
                    crate::Mark::Xy(xy) => {
                        for point in &xy.points {
                            values.insert(OrderedFloat(point.x));
                        }
                    }
                    crate::Mark::Heatmap(hm) => {
                        let is_x = matches!(self.axis.orientation(), Orientation::Bottom | Orientation::Top);
                        let count = if is_x { hm.cols() } else { hm.rows() };
                        for i in 0..count {
                            values.insert(OrderedFloat(i as f64));
                        }
                    }
                    crate::Mark::BoxPlot(bp) => {
                        for (i, _) in bp.entries.iter().enumerate() {
                            values.insert(OrderedFloat(i as f64));
                        }
                    }
                    crate::Mark::Violin(v) => {
                        for (i, _) in v.entries.iter().enumerate() {
                            values.insert(OrderedFloat(i as f64));
                        }
                    }
                    crate::Mark::Rule(_)
                    | crate::Mark::Band(_)
                    | crate::Mark::Tick(_)
                    | crate::Mark::Pie(_)
                    | crate::Mark::Gauge(_)
                    | crate::Mark::Treemap(_)
                    | crate::Mark::Choropleth(_)
                    | crate::Mark::Text(_) => {}
                }
            }

            (values.into_iter().map(|OrderedFloat(v)| v).collect(), None)
        } else {
            // Continuous: generate nice ticks WITHIN the bounds
            // Use time-aligned ticks for time-based axes
            let alignment = self.axis.ticks.alignment;
            let target = target_tick_count(axis_length, self.axis.kind(), self.axis.orientation());
            if self.axis.kind() == Kind::Time {
                (
                    {
                        let (ticks, _) = nice_time_ticks(axis_min, axis_max, target, alignment);
                        ticks.into_iter().map(|t| t as f64).collect::<Vec<f64>>()
                    },
                    None,
                )
            } else if matches!(self.transform, crate::scale::Transform::Log) {
                // Log axes use decade-boundary ticks. The label format
                // helper uses None precision so the value is rendered
                // as a clean integer (10, 100, 1000…).
                (log_ticks(axis_min, axis_max), None)
            } else {
                let (ticks, step) = self.nice_ticks(axis_min, axis_max, target, alignment);
                (ticks, Some(step))
            }
        };

        // Derive format precision from the tick step (D3-style)
        let step_precision = nice_step.map(precision_for_step);

        // For time axes, get the interval for smart formatting
        let time_interval = if self.axis.kind() == Kind::Time {
            let alignment = self.axis.ticks.alignment;
            let target = target_tick_count(axis_length, self.axis.kind(), self.axis.orientation());
            let (_, interval) = nice_time_ticks(axis_min, axis_max, target, alignment);
            Some(interval)
        } else {
            None
        };

        // Apply frequency filter
        let filtered_positions: Vec<f64> = match &self.axis.ticks.frequency {
            Frequency::EveryItem => data_positions,
            Frequency::EveryNthItem(n) => data_positions
                .into_iter()
                .enumerate()
                .filter(|(i, _)| i % n == 0)
                .map(|(_, v)| v)
                .collect(),
            Frequency::FirstAndLast if data_positions.len() >= 2 => {
                vec![data_positions[0], *data_positions.last().unwrap()]
            }
            Frequency::FirstAndLast => data_positions,
            Frequency::Custom(custom) => custom.clone(),
        };

        // Create label formatting function based on Labels configuration.
        // When no custom format is provided, use step-derived precision
        // so that floating-point noise is hidden (e.g. 0.6 not 0.600000001).
        let format_label = |pos: f64| -> String {
            if let Some(ref labels) = self.axis.labels.values {
                let index = pos.round() as usize;
                if index < labels.len() {
                    labels[index].clone()
                } else {
                    format_with_precision(pos, step_precision)
                }
            } else if let Some(ref format_fn) = self.axis.labels.format {
                format_fn(pos)
            } else if let Some(interval) = time_interval {
                interval.format(pos as i64)
            } else {
                format_with_precision(pos, step_precision)
            }
        };

        // Determine tick and label positions based on placement
        let placement = self.axis.labels.placement.unwrap_or(label::Placement::OnTicks);
        let (label_info, tick_positions) = match placement {
            label::Placement::OnTicks => {
                // Labels and ticks at same positions
                let labels: Vec<(f64, String)> =
                    filtered_positions.iter().map(|&pos| (pos, format_label(pos))).collect();
                let ticks = filtered_positions.clone();
                (labels, ticks)
            }
            label::Placement::BetweenTicks => {
                if is_categorical && !filtered_positions.is_empty() {
                    // For categorical data with BetweenTicks:
                    // - Labels at data positions (centers): [0, 1, 2, 3, 4, 5]
                    // - Ticks at boundaries (between): [-0.5, 0.5, 1.5, 2.5, 3.5, 4.5, 5.5]
                    let labels: Vec<(f64, String)> =
                        filtered_positions.iter().map(|&pos| (pos, format_label(pos))).collect();

                    let min = filtered_positions.iter().fold(f64::INFINITY, |a, &b| a.min(b));
                    let _max = filtered_positions.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));

                    // Generate boundary ticks
                    let mut ticks = Vec::new();
                    ticks.push(min - 0.5);
                    for &pos in &filtered_positions {
                        ticks.push(pos + 0.5);
                    }
                    // Note: first boundary is min-0.5, last is max+0.5 (which is filtered_positions.last() + 0.5)

                    (labels, ticks)
                } else {
                    // For continuous data, fall back to OnTicks
                    let labels: Vec<(f64, String)> =
                        filtered_positions.iter().map(|&pos| (pos, format_label(pos))).collect();
                    let ticks = filtered_positions.clone();
                    (labels, ticks)
                }
            }
        };

        (label_info, tick_positions)
    }

    /// Find the min/max range for this axis dimension
    fn find_range(&self, is_x_axis: bool) -> (f64, f64) {
        use crate::bar::Layout;
        use crate::mark::area::Layout as AreaLayout;
        use std::collections::HashMap;

        let mut min = f64::INFINITY;
        let mut max = f64::NEG_INFINITY;

        for mark in self.marks {
            match mark {
                crate::Mark::Area(area) => {
                    if !is_x_axis && area.layout == AreaLayout::Stacked {
                        let mut sums: HashMap<i64, f64> = HashMap::new();
                        for series in &area.series {
                            for point in &series.points {
                                let x_key = (point.x * 1000.0).round() as i64;
                                *sums.entry(x_key).or_insert(0.0) += point.y;
                                min = min.min(point.x);
                                max = max.max(point.x);
                            }
                        }
                        for sum in sums.values() {
                            max = max.max(*sum);
                            min = min.min(*sum);
                        }
                    } else {
                        for series in &area.series {
                            for point in &series.points {
                                let val = if is_x_axis { point.x } else { point.y };
                                min = min.min(val);
                                max = max.max(val);
                            }
                        }
                    }
                }
                crate::Mark::Bars(bars) => {
                    use crate::mark::bar::Direction;

                    // For horizontal bars, the axes are swapped:
                    // - x-axis (bottom) shows values (point.y)
                    // - y-axis (left) shows categories (point.x)
                    let is_horizontal = bars.direction == Direction::Horizontal;

                    // Determine which field is the "category" and which is the "value"
                    // based on direction and which axis we're computing for
                    let is_value_axis = (is_x_axis && is_horizontal) || (!is_x_axis && !is_horizontal);

                    if is_value_axis && bars.layout == Layout::Stacked {
                        // For stacked bars on the value axis, compute cumulative sums
                        let mut sums: HashMap<i64, f64> = HashMap::new();

                        for series in &bars.series {
                            for point in &series.points {
                                let cat_key = (point.x * 1000.0).round() as i64;
                                *sums.entry(cat_key).or_insert(0.0) += point.y;
                            }
                        }

                        // Update with stacked totals
                        for sum in sums.values() {
                            max = max.max(*sum);
                            min = min.min(*sum);
                        }
                    } else if is_value_axis {
                        // Value axis (non-stacked): use point.y (the values)
                        for series in &bars.series {
                            for point in &series.points {
                                min = min.min(point.y);
                                max = max.max(point.y);
                            }
                        }
                    } else {
                        // Category axis: use point.x (the category indices)
                        for series in &bars.series {
                            for point in &series.points {
                                min = min.min(point.x);
                                max = max.max(point.x);
                            }
                        }
                    }
                }
                crate::Mark::Line(line) => {
                    for point in &line.points {
                        let val = if is_x_axis { point.x } else { point.y };
                        min = min.min(val);
                        max = max.max(val);
                    }
                }
                crate::Mark::Waterfall(wf) => {
                    if is_x_axis {
                        for (i, _) in wf.entries.iter().enumerate() {
                            min = min.min(i as f64);
                            max = max.max(i as f64);
                        }
                    } else {
                        // Track the envelope of all running totals (steps + totals)
                        // and decide whether to zoom into the step range.
                        let mut running: f64 = 0.0;
                        let mut envelope_min: f64 = f64::INFINITY;
                        let mut envelope_max: f64 = f64::NEG_INFINITY;

                        for entry in &wf.entries {
                            match entry.kind {
                                crate::mark::waterfall::EntryKind::Total => {
                                    running = entry.value;
                                }
                                _ => {
                                    running += entry.value;
                                }
                            }
                            envelope_min = envelope_min.min(running);
                            envelope_max = envelope_max.max(running);
                        }

                        let envelope_range = envelope_max - envelope_min;
                        let full_range = envelope_max.max(0.0) - envelope_min.min(0.0);

                        if full_range > 0.0 && envelope_range / full_range < 0.4 {
                            // Steps are small relative to the full 0-based range.
                            // Zoom in so the steps occupy ~50 % of the chart height.
                            let padding = envelope_range * 0.5;
                            min = min.min(envelope_min - padding);
                            max = max.max(envelope_max + padding);
                        } else {
                            // Steps are large enough — use normal 0-based range.
                            min = min.min(envelope_min).min(0.0);
                            max = max.max(envelope_max);
                        }
                    }
                }
                crate::Mark::Xy(xy) => {
                    for point in &xy.points {
                        let val = if is_x_axis { point.x } else { point.y };
                        min = min.min(val);
                        max = max.max(val);
                    }
                }
                crate::Mark::Rule(rule) => match rule.orientation {
                    crate::mark::rule::RuleOrientation::Horizontal if !is_x_axis => {
                        min = min.min(rule.value);
                        max = max.max(rule.value);
                    }
                    crate::mark::rule::RuleOrientation::Vertical if is_x_axis => {
                        min = min.min(rule.value);
                        max = max.max(rule.value);
                    }
                    _ => {}
                },
                crate::Mark::Band(band) => match band.orientation {
                    crate::mark::band::BandOrientation::Horizontal if !is_x_axis => {
                        min = min.min(band.lower);
                        max = max.max(band.upper);
                    }
                    crate::mark::band::BandOrientation::Vertical if is_x_axis => {
                        min = min.min(band.lower);
                        max = max.max(band.upper);
                    }
                    _ => {}
                },
                crate::Mark::Tick(tick) => {
                    for point in &tick.points {
                        match tick.orientation {
                            crate::mark::tick::Orientation::Vertical => {
                                if is_x_axis {
                                    min = min.min(point.y);
                                    max = max.max(point.y);
                                } else {
                                    min = min.min(point.x);
                                    max = max.max(point.x);
                                }
                            }
                            crate::mark::tick::Orientation::Horizontal => {
                                if is_x_axis {
                                    min = min.min(point.x);
                                    max = max.max(point.x);
                                } else {
                                    min = min.min(point.y);
                                    max = max.max(point.y);
                                }
                            }
                        }
                    }
                }
                crate::Mark::Heatmap(hm) => {
                    if is_x_axis {
                        min = min.min(0.0);
                        max = max.max((hm.cols() as f64 - 1.0).max(0.0));
                    } else {
                        min = min.min(0.0);
                        max = max.max((hm.rows() as f64 - 1.0).max(0.0));
                    }
                }
                crate::Mark::BoxPlot(bp) => match bp.direction {
                    crate::mark::boxplot::Direction::Vertical => {
                        if is_x_axis {
                            for (i, _) in bp.entries.iter().enumerate() {
                                min = min.min(i as f64);
                                max = max.max(i as f64);
                            }
                        } else {
                            for e in &bp.entries {
                                min = min.min(e.min);
                                max = max.max(e.max);
                                for &o in &e.outliers {
                                    min = min.min(o);
                                    max = max.max(o);
                                }
                            }
                        }
                    }
                    crate::mark::boxplot::Direction::Horizontal => {
                        if is_x_axis {
                            for e in &bp.entries {
                                min = min.min(e.min);
                                max = max.max(e.max);
                                for &o in &e.outliers {
                                    min = min.min(o);
                                    max = max.max(o);
                                }
                            }
                        } else {
                            for (i, _) in bp.entries.iter().enumerate() {
                                min = min.min(i as f64);
                                max = max.max(i as f64);
                            }
                        }
                    }
                },
                crate::Mark::Violin(v) => match v.direction {
                    crate::mark::violin::Direction::Vertical => {
                        if is_x_axis {
                            for (i, _) in v.entries.iter().enumerate() {
                                min = min.min(i as f64);
                                max = max.max(i as f64);
                            }
                        } else {
                            for e in &v.entries {
                                for &(val, _) in &e.density {
                                    min = min.min(val);
                                    max = max.max(val);
                                }
                            }
                        }
                    }
                    crate::mark::violin::Direction::Horizontal => {
                        if is_x_axis {
                            for e in &v.entries {
                                for &(val, _) in &e.density {
                                    min = min.min(val);
                                    max = max.max(val);
                                }
                            }
                        } else {
                            for (i, _) in v.entries.iter().enumerate() {
                                min = min.min(i as f64);
                                max = max.max(i as f64);
                            }
                        }
                    }
                },
                crate::Mark::Pie(_)
                | crate::Mark::Gauge(_)
                | crate::Mark::Treemap(_)
                | crate::Mark::Choropleth(_)
                | crate::Mark::Text(_) => {}
            }
        }

        // For the value axis of bar/waterfall/area charts, ensure we include zero
        // (Line charts should fit to the data range)
        // For vertical bars: value axis is Y; for horizontal bars: value axis is X
        {
            let has_vertical_bars = self.marks.iter().any(|m| {
                matches!(m, crate::Mark::Area(_))
                    || matches!(m, crate::Mark::Bars(b) if b.direction() == crate::mark::bar::Direction::Vertical)
            });
            let has_horizontal_bars = self
                .marks
                .iter()
                .any(|m| matches!(m, crate::Mark::Bars(b) if b.direction() == crate::mark::bar::Direction::Horizontal));

            let should_include_zero = (!is_x_axis && has_vertical_bars) || (is_x_axis && has_horizontal_bars);
            if should_include_zero {
                min = min.min(0.0);
            }
        }

        // Ensure valid range
        if min.is_infinite() || max.is_infinite() {
            (0.0, 1.0)
        } else {
            (min, max)
        }
    }

    /// Generate nice tick positions within [min, max].
    ///
    /// Returns (ticks, step) so callers can derive format precision from the step.
    /// Uses multiplication-based positioning (`start + i * step`) instead of
    /// repeated addition to avoid floating-point error accumulation.
    fn nice_ticks(&self, min: f64, max: f64, target_count: usize, alignment: Alignment) -> (Vec<f64>, f64) {
        if min >= max {
            return (vec![min], 0.0);
        }

        let range = max - min;
        let nice_step = crate::scale::transform::nice_step(range, target_count);

        let mut ticks = Vec::new();

        match alignment {
            Alignment::Auto | Alignment::SnapToStart => {
                // Use multiplication from the starting value to avoid
                // floating-point drift: start + i * step
                let n = ((max - min) / nice_step + 0.001).floor() as usize + 1;
                for i in 0..n {
                    ticks.push(min + i as f64 * nice_step);
                }
            }
            Alignment::SnapToEnd => {
                // Work backward from max using multiplication
                let n = ((max - min) / nice_step + 0.001).floor() as usize + 1;
                for i in (0..n).rev() {
                    ticks.push(max - i as f64 * nice_step);
                }
            }
        }

        (ticks, nice_step)
    }

    /// Returns the initial tree state for this Guide
    pub(super) fn state(&self) -> Tree {
        // Compute bounds first, then ticks within bounds
        let bounds = self.compute_axis_bounds();
        let (label_info, _tick_positions) = self.ticks_and_labels(bounds, None);

        // Create a tree for each label's paragraph
        let children = label_info.iter().map(|_| Tree::empty()).collect();

        Tree {
            tag: tree::Tag::of::<State<Renderer::Paragraph>>(),
            state: tree::State::new(State::<Renderer::Paragraph> {
                labels: Vec::new(),
                label_info: Vec::new(),
                tick_positions: Vec::new(),
                bounds: Bounds::exact(0.0, 1.0),
                label_insets: (0.0, 0.0),
                tick_space: 0.0,
                label_offset: 0.0,
            }),
            children,
        }
    }

    /// Reconcile the tree with current Guide state
    pub(super) fn diff(&self, tree: &mut Tree) {
        let bounds = self.compute_axis_bounds();
        let (mut label_info, _) = self.ticks_and_labels(bounds, None);

        tree.diff_children_custom(&mut label_info, |_tree, _tick| {}, |_tick| Tree::empty());
    }

    /// Layout the guide, measuring text labels and positioning ticks.
    ///
    /// `overflow` is the pixel budget available outside each end of the axis
    /// for edge-label overhang (e.g. the sibling axis column). `.0` is the
    /// start (left for horizontal, top for vertical); `.1` is the end.
    ///
    /// `min_inset` is a floor imposed by other chart elements (e.g. series
    /// data labels that extend past bar ends). The final inset is computed
    /// as `max(0, half_label - overflow, min_inset)`.
    pub fn layout(
        &self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &Limits,
        overflow: (f32, f32),
        min_inset: (f32, f32),
        design: Option<&dyn crate::design::Design>,
    ) -> Node {
        // Compute bounds first, then generate ticks within those bounds.
        // The target tick count scales with the axis's pixel extent along
        // its major direction (height for left/right, width for top/bottom).
        let bounds = self.compute_axis_bounds();
        let axis_length = match self.axis.orientation() {
            Orientation::Left | Orientation::Right => limits.max().height,
            Orientation::Bottom | Orientation::Top => limits.max().width,
        };
        let (label_info, tick_positions) = self.ticks_and_labels(bounds, Some(axis_length));

        // Update state with computed values
        let state = tree.state.downcast_mut::<State<Renderer::Paragraph>>();
        state.label_info = label_info.clone();
        state.tick_positions = tick_positions;
        state.bounds = bounds;

        let labels = &state.label_info;
        let (min_value, max_value) = (state.bounds.min(), state.bounds.max());

        if labels.is_empty() || (!self.axis.shows_ticks() && !self.axis.has_labels()) {
            // Hidden axes (e.g. a y-axis muted via `.none()` to show only
            // gridlines) still need to propagate the `min_inset` floor so
            // series-driven insets — bar data labels extending past the
            // plot area top, etc. — reach the plot area. Without this the
            // scene reads `(0, 0)` from `state.label_insets` and lays the
            // plot area flush against the chart edge, clipping the labels.
            state.label_insets = min_inset;
            return Node::new(Size::ZERO);
        }

        let tick_length = 5.0;
        let has_chrome = self.axis.shows_ticks() || self.axis.shows_line();
        let tick_space = if self.axis.shows_ticks() { tick_length } else { 0.0 };
        let label_offset = if has_chrome {
            8.0
        } else {
            let base = design.and_then(|d| d.axis_text().size).map(|p| p.0).unwrap_or(12.0);
            let resolved = self.axis.labels.text.resolved_size(base);
            resolved * 0.5
        };
        state.tick_space = tick_space;
        state.label_offset = label_offset;
        let max_size = limits.max();

        match self.axis.orientation() {
            Orientation::Left | Orientation::Right => self.layout_vertical(
                state,
                renderer,
                max_size,
                tick_length,
                label_offset,
                min_value,
                max_value,
                overflow,
                min_inset,
                design,
            ),
            Orientation::Bottom | Orientation::Top => self.layout_horizontal(
                state,
                renderer,
                max_size,
                tick_length,
                label_offset,
                min_value,
                max_value,
                overflow,
                min_inset,
                design,
            ),
        }
    }

    /// Layout a vertical axis (left/right)
    #[allow(clippy::too_many_arguments)]
    fn layout_vertical(
        &self,
        state: &mut State<Renderer::Paragraph>,
        renderer: &Renderer,
        max_size: Size,
        tick_length: f32,
        label_offset: f32,
        min_value: f64,
        max_value: f64,
        overflow: (f32, f32),
        min_inset: (f32, f32),
        design: Option<&dyn crate::design::Design>,
    ) -> Node {
        // Use cached label info from state
        let label_data = &state.label_info;
        let value_range = max_value - min_value;

        // Ensure we have enough paragraphs for all labels
        while state.labels.len() < label_data.len() {
            state.labels.push(paragraph::Plain::default());
        }

        // Calculate width and position each label
        let mut children = Vec::new();
        let mut max_label_width = 0.0f32;

        // First pass: measure all labels
        let base_font = design
            .map(|d| d.axis_text().resolved_font(renderer.default_font()))
            .unwrap_or_else(|| renderer.default_font());
        let base_size = design.and_then(|d| d.axis_text().size).map(|p| p.0).unwrap_or(12.0);
        let axis_text = self.axis.labels.text.or(self.axis.text());
        let label_font = axis_text.resolved_font(base_font);
        let label_size_px: crate::core::Pixels = axis_text.resolved_size(base_size).into();
        for (i, (_pos, label)) in label_data.iter().enumerate() {
            let paragraph = &mut state.labels[i];

            use crate::core::alignment;
            let _ = paragraph.update(text::Text {
                content: label,
                bounds: Size::INFINITE,
                size: label_size_px,
                line_height: text::LineHeight::default(),
                font: label_font,
                align_x: text::Alignment::Left,
                align_y: alignment::Vertical::Top,
                shaping: text::Shaping::Basic,
                wrapping: text::Wrapping::None,
                ellipsis: text::Ellipsis::default(),
                hint_factor: renderer.scale_factor(),
                font_features: Vec::new(),
                font_variations: Vec::new(),
                letter_spacing: Default::default(),
                weight: None,
            });

            max_label_width = max_label_width.max(paragraph.min_bounds().width);
        }

        // Compute edge label insets (half-height of top/bottom center-aligned labels),
        // reduced by the overflow budget available on each side, floored by min_inset.
        // labels[last] = max value = top of axis, labels[0] = min value = bottom.
        //
        // The first/last tick is not necessarily at the plot's outer edge: a
        // categorical axis with OnTicks placement and `Kind::Categorical` bounds
        // (±0.5 padding) sits the first tick at `k_bottom * usable_height` above
        // the plot's bottom edge, giving the bottom label that much free space
        // inside the plot before any inset is needed. We credit that natural
        // space against the half-label overhang so short categorical labels
        // don't needlessly shrink the chart. For scalar axes the ticks sit at
        // the data extrema (`k_* == 0`), so the formula collapses to the
        // previous behavior.
        let n = label_data.len();
        let top_half = if n > 0 {
            state.labels[n - 1].min_bounds().height / 2.0
        } else {
            0.0
        };
        let bottom_half = if n > 0 {
            state.labels[0].min_bounds().height / 2.0
        } else {
            0.0
        };
        let (k_top, k_bottom) = if value_range > 0.0 && n > 0 {
            let bottom_tick = label_data[0].0;
            let top_tick = label_data[n - 1].0;
            (
                (1.0 - self.transform.map_to_unit(top_tick, min_value, max_value)) as f32,
                self.transform.map_to_unit(bottom_tick, min_value, max_value) as f32,
            )
        } else {
            (0.0, 0.0)
        };
        let natural_top = k_top * max_size.height;
        let natural_bottom = k_bottom * max_size.height;
        let top_inset = (top_half - natural_top - overflow.0).max(0.0).max(min_inset.0);
        let bottom_inset = (bottom_half - natural_bottom - overflow.1).max(0.0).max(min_inset.1);
        let usable_height = (max_size.height - top_inset - bottom_inset).max(0.0);

        state.label_insets = (top_inset, bottom_inset);

        // Second pass: position labels within the inset range
        for (i, (pos, _label)) in label_data.iter().enumerate() {
            let label_width = state.labels[i].min_bounds().width;
            let label_height = state.labels[i].min_bounds().height;

            if max_size.height < label_height {
                continue;
            }

            let tick_value = *pos;
            let y = if value_range > 0.0 {
                let normalized = self.transform.map_to_unit(tick_value, min_value, max_value) as f32;
                top_inset + usable_height - normalized * usable_height
            } else {
                max_size.height / 2.0
            };

            children.push(
                Node::new(Size::new(label_width, label_height))
                    .move_to(Point::ORIGIN + crate::core::Vector::new(0.0, y)),
            );
        }

        let tick_space = if self.axis.shows_ticks() { tick_length } else { 0.0 };
        let width = if self.axis.has_labels() {
            max_label_width + tick_space + label_offset
        } else if self.axis.shows_ticks() {
            tick_length
        } else {
            0.0
        };

        Node::with_children(Size::new(width, max_size.height), children)
    }

    /// Layout a horizontal axis (top/bottom)
    #[allow(clippy::too_many_arguments)]
    fn layout_horizontal(
        &self,
        state: &mut State<Renderer::Paragraph>,
        renderer: &Renderer,
        max_size: Size,
        tick_length: f32,
        label_offset: f32,
        min_value: f64,
        max_value: f64,
        overflow: (f32, f32),
        min_inset: (f32, f32),
        design: Option<&dyn crate::design::Design>,
    ) -> Node {
        // Use cached label info from state
        let label_data = &state.label_info;
        let value_range = max_value - min_value;

        // Ensure we have enough paragraphs for all labels
        while state.labels.len() < label_data.len() {
            state.labels.push(paragraph::Plain::default());
        }

        // Position each label along the x-axis
        let mut children = Vec::new();
        let n = label_data.len();

        // First pass: measure all labels.
        //
        // Compute the allotted column width per label: the pixel distance
        // between adjacent ticks in the axis's natural domain. The overflow
        // strategy constrains labels to this width so they don't crowd each
        // other. Using `max_size.width` (not post-inset `usable_width`) is a
        // slight over-allocation when insets end up non-zero, but it lets us
        // avoid a fixed-point iteration between label widths and inset sizes.
        let tick_stride: f64 = if n >= 2 {
            let mut min_stride = f64::INFINITY;
            for pair in label_data.windows(2) {
                let s = pair[1].0 - pair[0].0;
                if s > 0.0 && s < min_stride {
                    min_stride = s;
                }
            }
            if min_stride.is_finite() {
                min_stride
            } else {
                value_range.max(1.0)
            }
        } else {
            value_range.max(1.0)
        };
        let column_width: f32 = if value_range > 0.0 {
            (max_size.width as f64 * tick_stride / value_range) as f32
        } else {
            max_size.width
        };

        // Pick wrapping + ellipsis from the overflow strategy.
        let (wrapping, ellipsis) = match self.axis.labels.overflow {
            crate::data::axis::label::Overflow::Ellipsize => (text::Wrapping::None, text::Ellipsis::End),
            crate::data::axis::label::Overflow::Wrap => (text::Wrapping::Word, text::Ellipsis::None),
        };

        // Measure all labels with bounds constrained to the column width so
        // the overflow strategy engages automatically inside iced's paragraph
        // update. If intrinsic < column_width, the paragraph's `min_bounds`
        // shrinks to intrinsic (the bound is an upper limit). If intrinsic >
        // column_width, the paragraph either wraps or ellipsizes at
        // column_width.
        let base_font = design
            .map(|d| d.axis_text().resolved_font(renderer.default_font()))
            .unwrap_or_else(|| renderer.default_font());
        let base_size = design.and_then(|d| d.axis_text().size).map(|p| p.0).unwrap_or(12.0);
        let axis_text = self.axis.labels.text.or(self.axis.text());
        let label_font = axis_text.resolved_font(base_font);
        let label_size_px: crate::core::Pixels = axis_text.resolved_size(base_size).into();
        for (i, (_pos, label)) in label_data.iter().enumerate() {
            let paragraph = &mut state.labels[i];

            use crate::core::alignment;
            let _ = paragraph.update(text::Text {
                content: label,
                bounds: Size::new(column_width, f32::INFINITY),
                size: label_size_px,
                line_height: text::LineHeight::default(),
                font: label_font,
                align_x: text::Alignment::Left,
                align_y: alignment::Vertical::Top,
                shaping: text::Shaping::Basic,
                wrapping,
                ellipsis,
                hint_factor: renderer.scale_factor(),
                font_features: Vec::new(),
                font_variations: Vec::new(),
                letter_spacing: Default::default(),
                weight: None,
            });
        }

        // Compute edge label insets (half-width of first/last center-aligned labels),
        // reduced by the overflow budget available on each side, floored by min_inset.
        //
        // The first/last tick is not necessarily at the plot's outer edge: a
        // categorical axis with OnTicks placement and `Kind::Categorical` bounds
        // (±0.5 padding) sits the first tick at `k_left * usable_width` inside
        // the plot's left edge, giving the first label that much free space to
        // extend leftward before any inset is needed. We credit that natural
        // space against the half-label overhang so short categorical labels
        // don't needlessly shrink the chart. For scalar axes the ticks sit at
        // the data extrema (`k_* == 0`), so the formula collapses to the
        // previous behavior.
        let left_half = state.labels.first().map(|p| p.min_bounds().width / 2.0).unwrap_or(0.0);
        let right_half = state
            .labels
            .get(label_data.len().saturating_sub(1))
            .map(|p| p.min_bounds().width / 2.0)
            .unwrap_or(0.0);
        let (k_left, k_right) = if value_range > 0.0 && n > 0 {
            let first_tick = label_data[0].0;
            let last_tick = label_data[n - 1].0;
            (
                self.transform.map_to_unit(first_tick, min_value, max_value) as f32,
                (1.0 - self.transform.map_to_unit(last_tick, min_value, max_value)) as f32,
            )
        } else {
            (0.0, 0.0)
        };
        let natural_left = k_left * max_size.width;
        let natural_right = k_right * max_size.width;
        let left_inset = (left_half - natural_left - overflow.0).max(0.0).max(min_inset.0);
        let right_inset = (right_half - natural_right - overflow.1).max(0.0).max(min_inset.1);
        let usable_width = (max_size.width - left_inset - right_inset).max(0.0);

        state.label_insets = (left_inset, right_inset);

        // Track the tallest label so the axis area can grow vertically when
        // `Wrap` produces multi-line labels. Single-line (Ellipsize) keeps the
        // historical height (label_size + tick_length + label_offset).
        let mut max_label_height: f32 = label_size_px.0;

        // Second pass: position labels within the inset range
        for (i, (pos, _label)) in label_data.iter().enumerate() {
            let label_width = state.labels[i].min_bounds().width;
            let label_height = state.labels[i].min_bounds().height;
            max_label_height = max_label_height.max(label_height);

            let tick_value = *pos;
            let x = if value_range > 0.0 {
                left_inset + (self.transform.map_to_unit(tick_value, min_value, max_value) * usable_width as f64) as f32
            } else {
                max_size.width / 2.0
            };

            children.push(
                Node::new(Size::new(label_width, label_height))
                    .move_to(Point::ORIGIN + crate::core::Vector::new(x, 0.0)),
            );
        }

        let tick_space = if self.axis.shows_ticks() { tick_length } else { 0.0 };
        let height = max_label_height + tick_space + label_offset;
        Node::with_children(Size::new(max_size.width, height), children)
    }

    /// Draws tick marks and labels at their laid-out positions.
    /// Call before the plot area so labels appear behind marks.
    #[allow(clippy::too_many_arguments)]
    pub fn draw<D>(
        &self,
        tree: &crate::core::widget::Tree,
        renderer: &mut Renderer,
        design: &D,
        _style: &crate::core::renderer::Style,
        layout: crate::core::Layout<'_>,
        _cursor: crate::core::mouse::Cursor,
        viewport: &crate::core::Rectangle,
    ) where
        D: crate::design::Design + ?Sized,
        Renderer: crate::widget::renderer::geometry::Renderer,
    {
        use crate::widget::canvas::{Frame, Path, Stroke};

        let state = tree.state.downcast_ref::<State<Renderer::Paragraph>>();
        let label_data = &state.label_info;
        let tick_positions = &state.tick_positions;

        let bounds = layout.bounds();

        let background = design.background_color();
        let text_pair = design.text_pair();
        let seed = design.seed();

        let axis_color = self
            .axis
            .axis_color()
            .unwrap_or(design.axis_color())
            .resolve(background, text_pair, &seed, None);
        let label_color = self
            .axis
            .label_color()
            .unwrap_or(design.text_color())
            .resolve(background, text_pair, &seed, None);

        // Get bounds for coordinate mapping
        let (min_value, max_value) = (state.bounds.min(), state.bounds.max());
        let value_range = max_value - min_value;

        // Draw all tick marks using canvas geometry (same pipeline as marks)
        let tick_length = 5.0;
        let (inset_start, inset_end) = state.label_insets;
        if !tick_positions.is_empty() && self.axis.shows_ticks() {
            let mut frame = Frame::new(renderer, bounds.size());
            // Pixel-snap perpendicular coordinates to a half-pixel row so a
            // 1 px stroke renders as one crisp physical pixel (instead of
            // straddling two rows at 50 % coverage) and clamp 0.5 px inside
            // the frame. Guard against degenerate frames (`bounds < 1.0`)
            // by floor-ing `hi` to `lo` so the clamp never panics with
            // `min > max`.
            let snap_h = |x: f32| {
                let hi = (bounds.width - 0.5).max(0.5);
                (x.round() + 0.5).clamp(0.5, hi)
            };
            let snap_v = |y: f32| {
                let hi = (bounds.height - 0.5).max(0.5);
                (y.round() + 0.5).clamp(0.5, hi)
            };
            let path = Path::new(|builder| {
                for &tick_pos in tick_positions {
                    let normalized = if value_range > 0.0 {
                        self.transform.map_to_unit(tick_pos, min_value, max_value) as f32
                    } else {
                        0.5
                    };

                    match self.axis.orientation() {
                        Orientation::Bottom => {
                            let usable = bounds.width - inset_start - inset_end;
                            let x = snap_h(inset_start + normalized * usable);
                            // The plot area's bottom border is drawn at
                            // scene y = plot_top + plot_height - 0.5,
                            // which is `frame y = -0.5` here. Start the
                            // tick at that border so it visually meets
                            // it, not 0.5 px below.
                            builder.move_to(Point::new(x, -0.5));
                            builder.line_to(Point::new(x, tick_length - 0.5));
                        }
                        Orientation::Top => {
                            let usable = bounds.width - inset_start - inset_end;
                            let x = snap_h(inset_start + normalized * usable);
                            // The plot area's top border is drawn at
                            // scene y = plot_top + 0.5, which is
                            // `frame y = bounds.height + 0.5` here (just
                            // past the bottom edge of the top-axis
                            // frame). The tick extends upward from the
                            // border into the axis frame, away from the
                            // plot area.
                            builder.move_to(Point::new(x, bounds.height + 0.5));
                            builder.line_to(Point::new(x, bounds.height - tick_length + 0.5));
                        }
                        Orientation::Left => {
                            let usable = bounds.height - inset_start - inset_end;
                            let y = snap_v(inset_start + usable - normalized * usable);
                            // The plot area's left border is drawn at
                            // scene x = content_left + 0.5, which is
                            // `frame x = bounds.width + 0.5` here. Extend
                            // the tick to that border so it visually
                            // meets it, not 0.5 px short.
                            builder.move_to(Point::new(bounds.width - tick_length + 0.5, y));
                            builder.line_to(Point::new(bounds.width + 0.5, y));
                        }
                        Orientation::Right => {
                            let usable = bounds.height - inset_start - inset_end;
                            let y = snap_v(inset_start + usable - normalized * usable);
                            // The plot area's right border is drawn at
                            // scene x = content_left + plot_width - 0.5,
                            // which is `frame x = -0.5` here (just past
                            // the left edge of the right-axis frame).
                            // Tick extends rightward from the border.
                            builder.move_to(Point::new(-0.5, y));
                            builder.line_to(Point::new(tick_length - 0.5, y));
                        }
                    }
                }
            });
            frame.stroke(&path, Stroke::default().with_width(1.0).with_color(axis_color));
            let geometry = frame.into_geometry();
            renderer.with_translation(crate::core::Vector::new(bounds.x, bounds.y), |renderer| {
                renderer.draw_geometry(geometry);
            });
        }

        // Then, draw labels at their positions from layout
        for ((i, (_pos, _label)), child_layout) in label_data.iter().enumerate().zip(layout.children()) {
            let child_bounds = child_layout.bounds();

            // Draw label if we have it
            if i < state.labels.len() && self.axis.has_labels() {
                let paragraph = &state.labels[i];
                let paragraph_bounds = paragraph.min_bounds();

                let tick_space = state.tick_space;
                let label_offset = state.label_offset;

                let orientation = self.axis.orientation();
                let text_align = self.axis.labels.align.unwrap_or(match orientation {
                    Orientation::Bottom | Orientation::Top => TextAlign::Center,
                    Orientation::Left => TextAlign::Right,
                    Orientation::Right => TextAlign::Left,
                });

                let w = paragraph_bounds.width;

                let anchor = match orientation {
                    Orientation::Bottom => {
                        let x = match text_align {
                            TextAlign::Left => child_bounds.x,
                            TextAlign::Center => child_bounds.x - w / 2.0,
                            TextAlign::Right => child_bounds.x - w,
                        };
                        Point::new(x, bounds.y + tick_space + label_offset)
                    }
                    Orientation::Left => {
                        let col_w = bounds.width - tick_space - label_offset;
                        let x = match text_align {
                            TextAlign::Left => bounds.x,
                            TextAlign::Center => bounds.x + (col_w - w) / 2.0,
                            TextAlign::Right => bounds.x + col_w - w,
                        };
                        Point::new(x, child_bounds.y - paragraph_bounds.height / 2.0)
                    }
                    Orientation::Right => {
                        let col_start = bounds.x + tick_space + label_offset;
                        let col_w = bounds.width - tick_space - label_offset;
                        let x = match text_align {
                            TextAlign::Left => col_start,
                            TextAlign::Center => col_start + (col_w - w) / 2.0,
                            TextAlign::Right => col_start + col_w - w,
                        };
                        Point::new(x, child_bounds.y - paragraph_bounds.height / 2.0)
                    }
                    Orientation::Top => {
                        let x = match text_align {
                            TextAlign::Left => child_bounds.x,
                            TextAlign::Center => child_bounds.x - w / 2.0,
                            TextAlign::Right => child_bounds.x - w,
                        };
                        Point::new(
                            x,
                            bounds.y + bounds.height - tick_space - label_offset - paragraph_bounds.height,
                        )
                    }
                };

                renderer.fill_paragraph(paragraph.raw(), anchor, label_color, *viewport);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to create a timestamp from date components using jiff
    fn ymd_to_timestamp(year: i16, month: i8, day: i8) -> i64 {
        jiff::civil::date(year, month, day)
            .to_zoned(TimeZone::UTC)
            .unwrap()
            .timestamp()
            .as_second()
    }

    #[test]
    fn test_interval_floor_minutes() {
        // 12:07:30 should floor to 12:00:00 for 5-minute intervals
        // (jiff floors to the unit, not to multiples)
        let base = ymd_to_timestamp(2024, 6, 15) + 12 * 3600; // 12:00:00
        let ts_12_07_30 = base + 7 * 60 + 30; // 12:07:30
        let interval = TimeInterval::new(TimeUnit::Minute, 5);
        let floored = interval.floor(ts_12_07_30);

        // Should floor to 12:07:00 (minute boundary)
        // Note: jiff rounds to the unit, so 5-min interval floors to minute
        assert_eq!(floored % 60, 0, "Should be on minute boundary");
    }

    #[test]
    fn test_interval_floor_hours() {
        let base = ymd_to_timestamp(2024, 6, 15);
        let ts_14_37 = base + 14 * 3600 + 37 * 60; // 14:37:00
        let interval = TimeInterval::new(TimeUnit::Hour, 1);
        let floored = interval.floor(ts_14_37);

        // Should be 14:00:00
        let expected = base + 14 * 3600;
        assert_eq!(floored, expected);
    }

    #[test]
    fn test_interval_floor_days() {
        // Any time on 2024-06-15 should floor to midnight UTC
        let midnight = ymd_to_timestamp(2024, 6, 15);
        let ts = midnight + 12 * 3600; // 12:00:00
        let interval = TimeInterval::new(TimeUnit::Day, 1);
        let floored = interval.floor(ts);

        assert_eq!(floored, midnight);
    }

    #[test]
    fn test_interval_floor_months() {
        // 2024-06-15 should floor to 2024-06-01 for 1-month intervals
        let june_15 = ymd_to_timestamp(2024, 6, 15);
        let interval = TimeInterval::new(TimeUnit::Month, 1);
        let floored = interval.floor(june_15);

        let expected = ymd_to_timestamp(2024, 6, 1);
        assert_eq!(floored, expected);

        // For 3-month (quarter) intervals, June should floor to April (Q2 start)
        let interval_q = TimeInterval::new(TimeUnit::Month, 3);
        let floored_q = interval_q.floor(june_15);
        let expected_q = ymd_to_timestamp(2024, 4, 1);
        assert_eq!(floored_q, expected_q);
    }

    #[test]
    fn test_interval_advance() {
        let base = ymd_to_timestamp(2024, 6, 15) + 12 * 3600; // 12:00:00

        // Advance by 15 minutes
        let interval = TimeInterval::new(TimeUnit::Minute, 15);
        let next = interval.advance(base);
        assert_eq!(next, base + 15 * 60);

        // Advance by 1 month from 2024-06-01
        let june_1 = ymd_to_timestamp(2024, 6, 1);
        let interval_m = TimeInterval::new(TimeUnit::Month, 1);
        let july_1 = interval_m.advance(june_1);
        assert_eq!(july_1, ymd_to_timestamp(2024, 7, 1));

        // Advance by 1 year
        let jan_2024 = ymd_to_timestamp(2024, 1, 1);
        let interval_y = TimeInterval::new(TimeUnit::Year, 1);
        let jan_2025 = interval_y.advance(jan_2024);
        assert_eq!(jan_2025, ymd_to_timestamp(2025, 1, 1));
    }

    #[test]
    fn test_select_interval() {
        // 1 hour range, target 6 ticks -> ~10 min per tick
        let interval = select_time_interval(3600.0, 6);
        assert_eq!(interval.unit, TimeUnit::Minute);
        assert_eq!(interval.count, 10);

        // 6 hour range, target 6 ticks -> ~1 hour per tick
        let interval = select_time_interval(6.0 * 3600.0, 6);
        assert_eq!(interval.unit, TimeUnit::Hour);
        assert_eq!(interval.count, 1);

        // 1 day range, target 6 ticks -> ~4 hours per tick
        let interval = select_time_interval(24.0 * 3600.0, 6);
        assert_eq!(interval.unit, TimeUnit::Hour);
        assert_eq!(interval.count, 4);

        // 1 year range, target 6 ticks -> ~2 months per tick -> selects 3 months
        // (365*24*3600)/6 = 5,256,000 secs; 3 months ≈ 7,776,000 is first >= target
        let interval = select_time_interval(365.0 * 24.0 * 3600.0, 6);
        assert_eq!(interval.unit, TimeUnit::Month);
        assert_eq!(interval.count, 3);
    }

    #[test]
    fn test_nice_time_ticks_one_hour() {
        // 1 hour from 12:00 to 13:00
        let base = ymd_to_timestamp(2024, 6, 15) + 12 * 3600;
        let min = base as f64;
        let max = min + 3600.0;

        let (ticks, interval) = nice_time_ticks(min, max, 6, Alignment::Auto);

        // Should select 10-minute intervals
        assert_eq!(interval.unit, TimeUnit::Minute);
        assert_eq!(interval.count, 10);

        // Verify ticks are at 10-minute boundaries
        for tick in &ticks {
            assert_eq!(tick % 600, 0, "Tick {} not aligned to 10 minutes", tick);
        }

        // Should have 6-7 ticks (depends on whether 13:00 is included)
        assert!(
            ticks.len() >= 6 && ticks.len() <= 7,
            "Expected 6-7 ticks, got {}",
            ticks.len()
        );
    }

    #[test]
    fn test_target_tick_count_short_extent() {
        // 100 px vertical numeric axis: 100 / 36 = 2.77 -> floor 2
        let n = target_tick_count(Some(100.0), Kind::Scalar, Orientation::Left);
        assert!((2..=3).contains(&n), "short y-axis should produce 2-3 ticks, got {}", n);
    }

    #[test]
    fn test_target_tick_count_tall_extent() {
        // 500 px vertical numeric axis: 500 / 36 = 13.88 -> clamped to 12
        let n = target_tick_count(Some(500.0), Kind::Scalar, Orientation::Left);
        assert!(
            (6..=12).contains(&n),
            "tall y-axis should produce 6-12 ticks, got {}",
            n
        );
    }

    #[test]
    fn test_target_tick_count_wide_x_axis() {
        // 700 px horizontal numeric axis: 700 / 70 = 10
        let n = target_tick_count(Some(700.0), Kind::Scalar, Orientation::Bottom);
        assert_eq!(n, 10, "wide x-axis at 700px with 70px spacing should yield 10");
    }

    #[test]
    fn test_target_tick_count_narrow_x_time_axis() {
        // 180 px horizontal time axis: 180 / 90 = 2
        let n = target_tick_count(Some(180.0), Kind::Time, Orientation::Bottom);
        assert_eq!(n, 2, "narrow x-time axis at 180px should yield 2");
    }

    #[test]
    fn test_target_tick_count_zero_extent_falls_back() {
        assert_eq!(target_tick_count(Some(0.0), Kind::Scalar, Orientation::Left), 6);
        assert_eq!(target_tick_count(Some(-50.0), Kind::Scalar, Orientation::Left), 6);
        assert_eq!(target_tick_count(Some(f32::NAN), Kind::Scalar, Orientation::Left), 6);
        assert_eq!(target_tick_count(None, Kind::Scalar, Orientation::Left), 6);
    }

    #[test]
    fn test_target_tick_count_clamped_max() {
        // Arbitrarily huge extent should still clamp to 12.
        let n = target_tick_count(Some(100_000.0), Kind::Scalar, Orientation::Left);
        assert_eq!(n, 12, "huge extent should clamp to 12");
        let n = target_tick_count(Some(100_000.0), Kind::Time, Orientation::Bottom);
        assert_eq!(n, 12, "huge x-time extent should clamp to 12");
    }

    #[test]
    fn test_target_tick_count_clamped_min() {
        // Tiny extent (below 2 * spacing) should still produce at least 2.
        let n = target_tick_count(Some(20.0), Kind::Scalar, Orientation::Left);
        assert_eq!(n, 2, "tiny y-axis should clamp up to 2");
        let n = target_tick_count(Some(20.0), Kind::Time, Orientation::Bottom);
        assert_eq!(n, 2, "tiny x-time axis should clamp up to 2");
    }

    #[test]
    fn test_target_tick_count_categorical_fallback() {
        // Categorical axes don't use this path in practice, but if they do,
        // keep the historical default rather than derive a surprising count.
        let n = target_tick_count(Some(400.0), Kind::Categorical, Orientation::Bottom);
        assert_eq!(n, 6);
    }

    #[test]
    fn test_interval_format() {
        let ts = ymd_to_timestamp(2024, 6, 15) + 14 * 3600 + 30 * 60; // 14:30:00

        let minute_interval = TimeInterval::new(TimeUnit::Minute, 10);
        assert_eq!(minute_interval.format(ts), "14:30");

        let day_interval = TimeInterval::new(TimeUnit::Day, 1);
        assert_eq!(day_interval.format(ts), "Jun 15");

        let month_interval = TimeInterval::new(TimeUnit::Month, 1);
        assert_eq!(month_interval.format(ts), "Jun 2024");

        let year_interval = TimeInterval::new(TimeUnit::Year, 1);
        assert_eq!(year_interval.format(ts), "2024");
    }
}
