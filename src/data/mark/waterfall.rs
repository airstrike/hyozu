use crate::color::Color;
use crate::data::axis::{self, Axis, Kind, Orientation, Placement};

pub mod label;

pub use label::Label;

/// The kind of waterfall entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryKind {
    /// A positive delta (adds to running total)
    Increase,
    /// A negative delta (subtracts from running total)
    Decrease,
    /// An absolute total (resets running total)
    Total,
}

/// A single entry in a waterfall chart.
#[derive(Debug, Clone)]
pub struct Entry {
    pub(crate) value: f64,
    pub(crate) kind: EntryKind,
    /// Static text override displayed instead of the chart-wide label format.
    pub(crate) text: Option<String>,
    pub(crate) color: Option<Color>,
}

/// Creates a waterfall entry.
pub fn entry(value: impl Into<f64>, kind: EntryKind) -> Entry {
    Entry {
        value: value.into(),
        kind,
        text: None,
        color: None,
    }
}

impl Entry {
    /// Sets a static text override for this entry's label.
    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.text = Some(text.into());
        self
    }

    /// Sets the color for this entry.
    pub fn color(mut self, color: impl Into<Color>) -> Self {
        self.color = Some(color.into());
        self
    }

    /// Returns the entry's value.
    pub fn value(&self) -> f64 {
        self.value
    }

    /// Returns the entry's kind.
    pub fn kind(&self) -> EntryKind {
        self.kind
    }

    /// Returns the entry's static text override, if any.
    pub fn text_value(&self) -> Option<&str> {
        self.text.as_deref()
    }
}

/// Waterfall chart specification.
///
/// Shows how an initial value is affected by intermediate positive and negative
/// values, leading to a final value.
#[derive(Debug, Clone)]
pub struct Waterfall {
    pub(crate) entries: Vec<Entry>,
    /// Whether to draw connector lines between bars
    pub(crate) connector: bool,
    /// Chart-wide data label configuration.
    pub(crate) label: Option<Label>,
}

/// Creates a waterfall chart from entries.
///
/// # Examples
///
/// ```
/// use hyozu::waterfall::{self, EntryKind::*};
///
/// let chart = waterfall::waterfall([
///     waterfall::entry(100, Total).text("Start"),
///     waterfall::entry(30, Increase).text("+Sales"),
///     waterfall::entry(-20, Decrease).text("-Costs"),
///     waterfall::entry(110, Total).text("End"),
/// ]);
/// ```
pub fn waterfall(data: impl IntoWaterfall) -> Waterfall {
    data.into_waterfall()
}

/// Trait for converting inputs into a Waterfall.
pub trait IntoWaterfall {
    fn into_waterfall(self) -> Waterfall;
}

// From array of entries
impl<const N: usize> IntoWaterfall for [Entry; N] {
    fn into_waterfall(self) -> Waterfall {
        Waterfall {
            entries: self.into(),
            connector: true,
            label: None,
        }
    }
}

// From Vec of entries
impl IntoWaterfall for Vec<Entry> {
    fn into_waterfall(self) -> Waterfall {
        Waterfall {
            entries: self,
            connector: true,
            label: None,
        }
    }
}

impl Waterfall {
    /// Sets whether to draw connector lines between bars.
    pub fn connector(mut self, show: bool) -> Self {
        self.connector = show;
        self
    }

    /// Configures chart-wide data labels.
    pub fn data_labels(mut self, label: impl Into<Option<Label>>) -> Self {
        self.label = label.into();
        self
    }

    /// Replaces the chart-wide data label configuration in place.
    pub fn set_label(&mut self, label: Option<Label>) {
        self.label = label;
    }

    /// Returns the entries.
    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    /// Returns the chart-wide label configuration, if any.
    pub fn label(&self) -> Option<&Label> {
        self.label.as_ref()
    }

    /// Returns a mutable reference to the chart-wide label, if set.
    pub fn label_mut(&mut self) -> Option<&mut Label> {
        self.label.as_mut()
    }

    // === Chart-wide label setters (lazily create a default Label if absent) ===

    pub fn set_label_position(&mut self, position: label::Position) {
        if let Some(lbl) = &mut self.label {
            lbl.set_position(position);
        } else {
            self.label = Some(Label::default().with_position(position));
        }
    }

    pub fn set_label_show(&mut self, show: label::Show) {
        if let Some(lbl) = &mut self.label {
            lbl.set_show(show);
        } else {
            self.label = Some(Label::default().with_show(show));
        }
    }

    pub fn set_label_color(&mut self, color: Option<Color>) {
        if let Some(lbl) = &mut self.label {
            lbl.set_color(color);
        } else {
            let mut lbl = Label::default();
            lbl.set_color(color);
            self.label = Some(lbl);
        }
    }

    pub fn set_label_size(&mut self, size: Option<crate::core::Pixels>) {
        if let Some(lbl) = &mut self.label {
            lbl.set_size(size);
        } else {
            let mut lbl = Label::default();
            lbl.set_size(size);
            self.label = Some(lbl);
        }
    }

    pub fn set_label_weight(&mut self, weight: Option<crate::core::font::Weight>) {
        if let Some(lbl) = &mut self.label {
            lbl.set_weight(weight);
        } else {
            let mut lbl = Label::default();
            lbl.set_weight(weight);
            self.label = Some(lbl);
        }
    }

    pub fn set_label_style(&mut self, style: Option<crate::core::font::Style>) {
        if let Some(lbl) = &mut self.label {
            lbl.set_style(style);
        } else {
            let mut lbl = Label::default();
            lbl.set_style(style);
            self.label = Some(lbl);
        }
    }

    pub fn set_label_fill(&mut self, fill: Option<Color>) {
        if let Some(lbl) = &mut self.label {
            lbl.set_fill(fill);
        } else {
            let mut lbl = Label::default();
            lbl.set_fill(fill);
            self.label = Some(lbl);
        }
    }

    /// Creates the appropriate x-axis for a waterfall chart (categorical).
    pub fn x_axis() -> Axis {
        Axis::new(Orientation::Bottom)
            .with_kind(Kind::Categorical)
            .labels(Placement::OnTicks)
            .with_ticks(axis::tick::Ticks::categorical())
    }

    /// Creates the appropriate y-axis for a waterfall chart.
    ///
    /// Uses `Scalar` rather than `ScalarAnchored` so that when steps are small
    /// relative to totals, the axis can zoom into the step range instead of
    /// anchoring at zero.  The zero-anchor still happens naturally when the
    /// data range returned by `find_range` includes zero.
    pub fn y_axis() -> Axis {
        Axis::new(Orientation::Left)
            .with_kind(Kind::Scalar)
            .with_ticks(axis::tick::Ticks::continuous())
    }
}

impl<Message, Theme, Renderer> From<Waterfall> for crate::Data<Message, Theme, Renderer>
where
    Message: 'static,
    Theme: 'static,
    Renderer: 'static,
{
    fn from(waterfall: Waterfall) -> Self {
        <Waterfall as crate::data::IntoData<Message, Theme, Renderer>>::into_data(waterfall)
    }
}
