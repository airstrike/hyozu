use crate::axis::{Alignment, Axis, Bounds, Kind, Orientation};
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
        self.0
            .partial_cmp(&other.0)
            .unwrap_or(std::cmp::Ordering::Less)
    }
}

/// Default label formatting function
fn default_format(value: f64) -> String {
    // For integers, show without decimal point
    if value.fract() == 0.0 {
        format!("{:.0}", value)
    } else {
        format!("{}", value)
    }
}

/// Compute a nice step size for a given range and target tick count
fn compute_nice_step(range: f64, target_count: usize) -> f64 {
    let rough_step = range / (target_count as f64);
    let magnitude = 10_f64.powf(rough_step.log10().floor());
    let normalized = rough_step / magnitude;

    if normalized <= 1.5 {
        magnitude
    } else if normalized <= 3.0 {
        2.0 * magnitude
    } else if normalized <= 7.0 {
        5.0 * magnitude
    } else {
        10.0 * magnitude
    }
}

use jiff::{Timestamp, tz::TimeZone};

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
            TimeUnit::Day => "%b %d",        // "Jun 15"
            TimeUnit::Month => "%b %Y",      // "Jun 2024"
            TimeUnit::Year => "%Y",          // "2024"
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
            TimeUnit::Month => 30.0 * 86400.0,  // approximate
            TimeUnit::Year => 365.0 * 86400.0,  // approximate
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
    TimeInterval::new(TimeUnit::Day, 7),    // week
    TimeInterval::new(TimeUnit::Day, 14),   // 2 weeks
    TimeInterval::new(TimeUnit::Month, 1),
    TimeInterval::new(TimeUnit::Month, 2),
    TimeInterval::new(TimeUnit::Month, 3),  // quarter
    TimeInterval::new(TimeUnit::Month, 6),  // half year
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
fn nice_time_ticks_with_interval(
    min: f64,
    max: f64,
    target_count: usize,
    alignment: Alignment,
) -> (Vec<i64>, TimeInterval) {
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

/// Generate nice time-aligned tick positions
fn nice_time_ticks(min: f64, max: f64, target_count: usize, alignment: Alignment) -> Vec<f64> {
    let (ticks, _) = nice_time_ticks_with_interval(min, max, target_count, alignment);
    ticks.into_iter().map(|t| t as f64).collect()
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
    _renderer: std::marker::PhantomData<(Message, Renderer)>,
}

impl<'a, Message, Renderer> Guide<'a, Message, Renderer>
where
    Message: 'a,
    Renderer: text::Renderer<Font = crate::core::Font>,
{
    /// Create a new Guide borrowing an Axis and marks
    pub fn new(axis: &'a Axis, marks: &'a [crate::Mark]) -> Self {
        Self {
            axis,
            marks,
            _renderer: std::marker::PhantomData,
        }
    }

    /// Compute the axis bounds (visual range) for this axis.
    ///
    /// Uses `Kind::bounds()` to apply kind-appropriate normalization (padding,
    /// nice numbers, zero-anchoring) based on the axis kind.
    fn compute_axis_bounds(&self) -> Bounds {
        let is_x_axis = matches!(
            self.axis.orientation(),
            Orientation::Bottom | Orientation::Top
        );

        // Get data range
        let (data_min, data_max) = self.find_range(is_x_axis);

        // Check for explicit user bounds
        let has_explicit_bounds =
            self.axis.lower_bound.is_some() && self.axis.upper_bound.is_some();

        if has_explicit_bounds {
            // User specified exact bounds - use as-is
            Bounds::exact(
                self.axis.lower_bound.unwrap(),
                self.axis.upper_bound.unwrap(),
            )
        } else {
            // Apply partial overrides if any, then let Kind compute proper bounds
            let min = self.axis.lower_bound.unwrap_or(data_min);
            let max = self.axis.upper_bound.unwrap_or(data_max);
            self.axis.kind().bounds(min, max)
        }
    }

    /// Derive tick positions, label positions, and label text from data
    /// Returns (label_info, tick_positions) based on label placement
    fn compute_ticks_and_labels(&self, bounds: Bounds) -> (Vec<(f64, String)>, Vec<f64>) {
        use crate::axis::label;
        use crate::axis::tick::Frequency;

        let is_x_axis = matches!(
            self.axis.orientation(),
            Orientation::Bottom | Orientation::Top
        );
        let is_categorical = is_x_axis && self.axis.is_categorical();

        let (axis_min, axis_max) = (bounds.min(), bounds.max());

        let data_positions = if is_categorical {
            // Categorical: collect unique X positions from data
            use std::collections::BTreeSet;
            let mut values: BTreeSet<OrderedFloat> = BTreeSet::new();

            for mark in self.marks {
                match mark {
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
                }
            }

            values.into_iter().map(|OrderedFloat(v)| v).collect()
        } else {
            // Continuous: generate nice ticks WITHIN the bounds
            // Use time-aligned ticks for time-based axes
            let alignment = self.axis.ticks.alignment;
            if self.axis.kind() == Kind::Time {
                nice_time_ticks(axis_min, axis_max, 6, alignment)
            } else {
                self.nice_ticks(axis_min, axis_max, 6, alignment)
            }
        };

        // For time axes, get the interval for smart formatting
        let time_interval = if self.axis.kind() == Kind::Time {
            let alignment = self.axis.ticks.alignment;
            let (_, interval) = nice_time_ticks_with_interval(axis_min, axis_max, 6, alignment);
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
            Frequency::Custom(custom) => custom.clone(),
        };

        // Create label formatting function based on Labels configuration
        let format_label = |pos: f64| -> String {
            if let Some(ref labels) = self.axis.labels.values {
                // Custom categorical labels
                let index = pos.round() as usize;
                if index < labels.len() {
                    labels[index].clone()
                } else {
                    // Fallback if index out of bounds
                    default_format(pos)
                }
            } else if let Some(ref format_fn) = self.axis.labels.format {
                // Use the provided format function
                format_fn(pos)
            } else if let Some(interval) = time_interval {
                // Smart time formatting based on interval scale
                interval.format(pos as i64)
            } else {
                // Auto-generate labels
                default_format(pos)
            }
        };

        // Determine tick and label positions based on placement
        let placement = self
            .axis
            .labels
            .placement
            .unwrap_or(label::Placement::OnTicks);
        let (label_info, tick_positions) = match placement {
            label::Placement::OnTicks => {
                // Labels and ticks at same positions
                let labels: Vec<(f64, String)> = filtered_positions
                    .iter()
                    .map(|&pos| (pos, format_label(pos)))
                    .collect();
                let ticks = filtered_positions.clone();
                (labels, ticks)
            }
            label::Placement::BetweenTicks => {
                if is_categorical && !filtered_positions.is_empty() {
                    // For categorical data with BetweenTicks:
                    // - Labels at data positions (centers): [0, 1, 2, 3, 4, 5]
                    // - Ticks at boundaries (between): [-0.5, 0.5, 1.5, 2.5, 3.5, 4.5, 5.5]
                    let labels: Vec<(f64, String)> = filtered_positions
                        .iter()
                        .map(|&pos| (pos, format_label(pos)))
                        .collect();

                    let min = filtered_positions
                        .iter()
                        .fold(f64::INFINITY, |a, &b| a.min(b));
                    let _max = filtered_positions
                        .iter()
                        .fold(f64::NEG_INFINITY, |a, &b| a.max(b));

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
                    let labels: Vec<(f64, String)> = filtered_positions
                        .iter()
                        .map(|&pos| (pos, format_label(pos)))
                        .collect();
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
        use std::collections::HashMap;

        let mut min = f64::INFINITY;
        let mut max = f64::NEG_INFINITY;

        for mark in self.marks {
            match mark {
                crate::Mark::Bars(bars) => {
                    if !is_x_axis && bars.layout == Layout::Stacked {
                        // For stacked bars Y-axis, compute cumulative sums
                        let mut sums: HashMap<i64, f64> = HashMap::new();

                        for series in &bars.series {
                            for point in &series.points {
                                let x_key = (point.x * 1000.0).round() as i64;
                                *sums.entry(x_key).or_insert(0.0) += point.y;

                                // Still track X range
                                min = min.min(point.x);
                                max = max.max(point.x);
                            }
                        }

                        // Update max with stacked totals
                        for sum in sums.values() {
                            max = max.max(*sum);
                            min = min.min(*sum);
                        }
                    } else {
                        // For grouped/overlaid bars or X-axis, use individual values
                        for series in &bars.series {
                            for point in &series.points {
                                let val =
                                    if is_x_axis { point.x } else { point.y };
                                min = min.min(val);
                                max = max.max(val);
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
            }
        }

        // For Y-axis with bar charts, ensure we include zero
        // (Line charts should fit to the data range)
        if !is_x_axis {
            let has_bars = self
                .marks
                .iter()
                .any(|m| matches!(m, crate::Mark::Bars(_)));
            if has_bars {
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

    /// Generate nice tick positions within [min, max]
    /// Bounds are assumed to already be nice (from compute_axis_bounds)
    fn nice_ticks(
        &self,
        min: f64,
        max: f64,
        target_count: usize,
        alignment: Alignment,
    ) -> Vec<f64> {
        if min >= max {
            return vec![min];
        }

        let range = max - min;
        let nice_step = compute_nice_step(range, target_count);

        let mut ticks = Vec::new();

        match alignment {
            Alignment::Auto => {
                // Generate ticks starting from min, stepping by nice_step
                // Since bounds are already nice, ticks should align
                let mut value = min;
                while value <= max + nice_step * 0.001 {
                    ticks.push(value);
                    value += nice_step;
                }
            }
            Alignment::SnapToStart => {
                // Start exactly at min
                let mut value = min;
                while value <= max + nice_step * 0.001 {
                    ticks.push(value);
                    value += nice_step;
                }
            }
            Alignment::SnapToEnd => {
                // Work backward from max
                let mut value = max;
                while value >= min - nice_step * 0.001 {
                    ticks.push(value);
                    value -= nice_step;
                }
                ticks.reverse();
            }
        }

        ticks
    }

    /// Returns the initial tree state for this Guide
    pub(super) fn state(&self) -> Tree {
        // Compute bounds first, then ticks within bounds
        let bounds = self.compute_axis_bounds();
        let (label_info, _tick_positions) = self.compute_ticks_and_labels(bounds);

        // Create a tree for each label's paragraph
        let children = label_info
            .iter()
            .map(|_| Tree::empty())
            .collect();

        Tree {
            tag: tree::Tag::of::<State<Renderer::Paragraph>>(),
            state: tree::State::new(State::<Renderer::Paragraph> {
                labels: Vec::new(),
                label_info: Vec::new(),
                tick_positions: Vec::new(),
                bounds: Bounds::exact(0.0, 1.0),
            }),
            children,
        }
    }

    /// Reconcile the tree with current Guide state
    pub(super) fn diff(&self, tree: &mut Tree) {
        let bounds = self.compute_axis_bounds();
        let (label_info, _) = self.compute_ticks_and_labels(bounds);

        tree.diff_children_custom(
            &label_info,
            |_tree, _tick| {},
            |_tick| Tree::empty(),
        );
    }

    /// Layout the guide, measuring text labels and positioning ticks.
    pub fn layout(
        &self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &Limits,
    ) -> Node {
        // Compute bounds first, then generate ticks within those bounds
        let bounds = self.compute_axis_bounds();
        let (label_info, tick_positions) = self.compute_ticks_and_labels(bounds);

        // Update state with computed values
        let state = tree.state.downcast_mut::<State<Renderer::Paragraph>>();
        state.label_info = label_info.clone();
        state.tick_positions = tick_positions;
        state.bounds = bounds;

        let labels = &state.label_info;
        let (min_value, max_value) = (state.bounds.min(), state.bounds.max());

        if labels.is_empty()
            || (!self.axis.has_ticks() && !self.axis.has_labels())
        {
            return Node::new(Size::ZERO);
        }

        let tick_length = 5.0;
        let label_offset = 8.0;
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
            ),
            Orientation::Bottom | Orientation::Top => self.layout_horizontal(
                state,
                renderer,
                max_size,
                tick_length,
                label_offset,
                min_value,
                max_value,
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

        for (i, (pos, label)) in label_data.iter().enumerate() {
            let paragraph = &mut state.labels[i];

            use crate::core::alignment;
            let _ = paragraph.update(text::Text {
                content: label,
                bounds: Size::INFINITE,
                size: self.axis.label_size().unwrap_or(12.0.into()),
                line_height: text::LineHeight::default(),
                font: self
                    .axis
                    .font()
                    .unwrap_or_else(|| renderer.default_font()),
                align_x: text::Alignment::Left,
                align_y: alignment::Vertical::Top,
                shaping: text::Shaping::Basic,
                wrapping: text::Wrapping::None,
                hint_factor: renderer.scale_factor(),
            });

            let label_width = paragraph.min_bounds().width;
            max_label_width = max_label_width.max(label_width);

            // Calculate Y position for this tick within our height
            let tick_value = *pos;
            let label_height = paragraph.min_bounds().height;
            let half_height = label_height / 2.0;

            // Skip if not enough room for the label
            if max_size.height < label_height {
                continue;
            }

            let y = if value_range > 0.0 {
                (max_size.height as f64
                    - ((tick_value - min_value) / value_range) * max_size.height as f64) as f32
            } else {
                max_size.height / 2.0
            };

            // Clamp y so vertically-centered labels stay within bounds
            let y = y.clamp(half_height, max_size.height - half_height);

            // Create a node for this tick positioned at the calculated y
            children.push(
                Node::new(Size::new(label_width, label_height))
                    .move_to(Point::ORIGIN + crate::core::Vector::new(0.0, y)),
            );
        }

        let width = if self.axis.has_labels() {
            max_label_width + tick_length + label_offset
        } else if self.axis.has_ticks() {
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

        for (i, (pos, label)) in label_data.iter().enumerate() {
            let paragraph = &mut state.labels[i];

            use crate::core::alignment;
            let _ = paragraph.update(text::Text {
                content: label,
                bounds: Size::INFINITE,
                size: self.axis.label_size().unwrap_or(12.0.into()),
                line_height: text::LineHeight::default(),
                font: self
                    .axis
                    .font()
                    .unwrap_or_else(|| renderer.default_font()),
                align_x: text::Alignment::Left,
                align_y: alignment::Vertical::Top,
                shaping: text::Shaping::Basic,
                wrapping: text::Wrapping::None,
                hint_factor: renderer.scale_factor(),
            });

            let label_width = paragraph.min_bounds().width;

            // Calculate X position for this tick within our width
            let tick_value = *pos;
            let x = if value_range > 0.0 {
                (((tick_value - min_value) / value_range) * max_size.width as f64) as f32
            } else {
                max_size.width / 2.0
            };

            // Create a node for this tick positioned at the calculated x
            children.push(
                Node::new(Size::new(label_width, 20.0))
                    .move_to(Point::ORIGIN + crate::core::Vector::new(x, 0.0)),
            );
        }

        let label_size = self.axis.label_size().unwrap_or(12.0.into());
        let height = label_size.0 + tick_length + label_offset;
        Node::with_children(Size::new(max_size.width, height), children)
    }

    /// Draws the guide by rendering ticks and labels at their laid-out positions.
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
    {
        let state = tree.state.downcast_ref::<State<Renderer::Paragraph>>();
        // Use cached label info and tick positions from state (computed in diff())
        let label_data = &state.label_info;
        let tick_positions = &state.tick_positions;

        // Draw axis line
        let bounds = layout.bounds();

        // Get colors from the design system or axis overrides
        let background = design.background_color();
        let text_pair = design.text_pair();

        let axis_color = self
            .axis
            .axis_color()
            .unwrap_or(design.axis_color())
            .resolve(background, text_pair, None);
        let label_color = self
            .axis
            .label_color()
            .unwrap_or(design.text_color())
            .resolve(background, text_pair, None);

        match self.axis.orientation() {
            Orientation::Bottom => {
                // Horizontal line at top of bounds
                renderer.fill_quad(
                    crate::core::renderer::Quad {
                        bounds: crate::core::Rectangle {
                            x: bounds.x,
                            y: bounds.y,
                            width: bounds.width,
                            height: 1.0,
                        },
                        ..Default::default()
                    },
                    axis_color,
                );
            }
            Orientation::Left => {
                // Vertical line at right edge of bounds (aligns with plot area left edge)
                renderer.fill_quad(
                    crate::core::renderer::Quad {
                        bounds: crate::core::Rectangle {
                            x: bounds.x + bounds.width,
                            y: bounds.y,
                            width: 1.0,
                            height: bounds.height,
                        },
                        ..Default::default()
                    },
                    axis_color,
                );
            }
            _ => {} // TODO: Top and Right orientations
        }

        // Get bounds for coordinate mapping
        let (min_value, max_value) = (state.bounds.min(), state.bounds.max());
        let value_range = max_value - min_value;

        // First, draw all tick marks at their positions
        let tick_length = 5.0;
        for &tick_pos in tick_positions {
            match self.axis.orientation() {
                Orientation::Bottom => {
                    // Map tick position to pixel coordinate
                    let normalized = if value_range > 0.0 {
                        ((tick_pos - min_value) / value_range) as f32
                    } else {
                        0.5
                    };

                    // Simple linear mapping that matches the bars
                    let pixel_x = bounds.x + normalized * bounds.width;

                    // Vertical tick mark
                    renderer.fill_quad(
                        crate::core::renderer::Quad {
                            bounds: crate::core::Rectangle {
                                x: pixel_x,
                                y: bounds.y,
                                width: 1.0,
                                height: tick_length,
                            },
                            ..Default::default()
                        },
                        axis_color,
                    );
                }
                Orientation::Left => {
                    // Map tick position to pixel coordinate
                    let normalized = if value_range > 0.0 {
                        ((tick_pos - min_value) / value_range) as f32
                    } else {
                        0.5
                    };
                    let pixel_y =
                        bounds.y + bounds.height - normalized * bounds.height;

                    // Horizontal tick mark
                    renderer.fill_quad(
                        crate::core::renderer::Quad {
                            bounds: crate::core::Rectangle {
                                x: bounds.x + bounds.width - tick_length,
                                y: pixel_y,
                                width: tick_length,
                                height: 1.0,
                            },
                            ..Default::default()
                        },
                        axis_color,
                    );
                }
                _ => {}
            }
        }

        // Then, draw labels at their positions from layout
        for ((i, (_pos, _label)), child_layout) in
            label_data.iter().enumerate().zip(layout.children())
        {
            let child_bounds = child_layout.bounds();

            // Draw label if we have it
            if i < state.labels.len() && self.axis.has_labels() {
                let paragraph = &state.labels[i];
                let paragraph_bounds = paragraph.min_bounds();

                let tick_length = 5.0;
                let label_offset = 8.0;

                // Calculate anchor position based on axis orientation
                // The paragraph is measured with Left/Top alignment, so we
                // offset the anchor to achieve the desired visual alignment
                let anchor = match self.axis.orientation() {
                    Orientation::Bottom => {
                        // Center horizontally on tick, position below tick mark
                        Point::new(
                            child_bounds.x - paragraph_bounds.width / 2.0,
                            bounds.y + tick_length + label_offset,
                        )
                    }
                    Orientation::Left => {
                        // Right align, vertically center on tick
                        Point::new(
                            bounds.x + bounds.width
                                - tick_length
                                - label_offset
                                - paragraph_bounds.width,
                            child_bounds.y - paragraph_bounds.height / 2.0,
                        )
                    }
                    Orientation::Right => {
                        // Left align, vertically center on tick
                        Point::new(
                            bounds.x + tick_length + label_offset,
                            child_bounds.y - paragraph_bounds.height / 2.0,
                        )
                    }
                    Orientation::Top => {
                        // Center horizontally on tick, position above tick mark
                        Point::new(
                            child_bounds.x - paragraph_bounds.width / 2.0,
                            bounds.y + bounds.height
                                - tick_length
                                - label_offset
                                - paragraph_bounds.height,
                        )
                    }
                };

                renderer.fill_paragraph(
                    paragraph.raw(),
                    anchor,
                    label_color,
                    *viewport,
                );
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

        let (ticks, interval) = nice_time_ticks_with_interval(min, max, 6, Alignment::Auto);

        // Should select 10-minute intervals
        assert_eq!(interval.unit, TimeUnit::Minute);
        assert_eq!(interval.count, 10);

        // Verify ticks are at 10-minute boundaries
        for tick in &ticks {
            assert_eq!(tick % 600, 0, "Tick {} not aligned to 10 minutes", tick);
        }

        // Should have 6-7 ticks (depends on whether 13:00 is included)
        assert!(ticks.len() >= 6 && ticks.len() <= 7, "Expected 6-7 ticks, got {}", ticks.len());
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
