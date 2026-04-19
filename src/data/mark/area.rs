use crate::color::Color;
use crate::data::axis::{self, Axis, Kind, Orientation, Placement};
use crate::data::{Datum, IntoDatums};

pub mod label {
    pub use crate::data::mark::line::label::*;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Layout {
    #[default]
    Overlaid,
    Stacked,
}

#[derive(Debug, Clone)]
pub struct Series {
    pub(crate) points: Vec<Datum>,
    pub(crate) color: Option<Color>,
    pub(crate) stroke: Option<f32>,
    pub(crate) opacity: f32,
    pub(crate) name: Option<String>,
    /// Data label configuration. Reuses the line chart label type since the
    /// placement semantics are identical (labels sit above/around points on a
    /// polyline envelope).
    pub(crate) label: Option<label::Label>,
}

impl Series {
    pub fn new(data: impl IntoDatums) -> Self {
        Self {
            points: data.into_datums(),
            color: None,
            stroke: Some(1.5),
            opacity: 0.4,
            name: None,
            label: None,
        }
    }

    pub fn color(mut self, color: impl Into<Color>) -> Self {
        self.color = Some(color.into());
        self
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn stroke(mut self, width: f32) -> Self {
        self.stroke = Some(width);
        self
    }

    pub fn no_stroke(mut self) -> Self {
        self.stroke = None;
        self
    }

    pub fn opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity.clamp(0.0, 1.0);
        self
    }

    /// Configure data labels for this area series.
    pub fn data_labels(mut self, label: impl Into<Option<label::Label>>) -> Self {
        self.label = label.into();
        self
    }

    // === Property setters (in-place) ===

    /// Sets the series color in place.
    pub fn set_color(&mut self, color: Option<Color>) {
        self.color = color;
    }

    /// Sets the stroke width in place. `None` removes the stroke.
    pub fn set_stroke(&mut self, stroke: Option<f32>) {
        self.stroke = stroke;
    }

    /// Sets the fill opacity in place (clamped to `[0.0, 1.0]`).
    pub fn set_opacity(&mut self, opacity: f32) {
        self.opacity = opacity.clamp(0.0, 1.0);
    }

    /// Replaces the entire label configuration.
    pub fn set_label(&mut self, label: Option<label::Label>) {
        self.label = label;
    }

    /// Sets the label position. Lazily creates a default label if none exists.
    pub fn set_label_position(&mut self, position: label::Position) {
        if let Some(lbl) = &mut self.label {
            lbl.set_position(position);
        } else {
            self.label = Some(label::Label::default().position(position));
        }
    }

    /// Sets which points show labels. Lazily creates a default label if none exists.
    pub fn set_label_show(&mut self, show: label::Show) {
        if let Some(lbl) = &mut self.label {
            lbl.set_show(show);
        } else {
            self.label = Some(label::Label::default().show(show));
        }
    }

    /// Sets the label color. Lazily creates a default label if none exists.
    pub fn set_label_color(&mut self, color: Option<Color>) {
        if let Some(lbl) = &mut self.label {
            lbl.set_color(color);
        } else {
            let mut lbl = label::Label::default();
            lbl.set_color(color);
            self.label = Some(lbl);
        }
    }

    /// Sets the label size. Lazily creates a default label if none exists.
    pub fn set_label_size(&mut self, size: Option<crate::core::Pixels>) {
        if let Some(lbl) = &mut self.label {
            lbl.set_size(size);
        } else {
            let mut lbl = label::Label::default();
            lbl.set_size(size);
            self.label = Some(lbl);
        }
    }

    /// Sets the label font weight. Lazily creates a default label if none exists.
    pub fn set_label_weight(&mut self, weight: Option<crate::core::font::Weight>) {
        if let Some(lbl) = &mut self.label {
            lbl.set_weight(weight);
        } else {
            let mut lbl = label::Label::default();
            lbl.set_weight(weight);
            self.label = Some(lbl);
        }
    }

    /// Sets the label font style. Lazily creates a default label if none exists.
    pub fn set_label_style(&mut self, style: Option<crate::core::font::Style>) {
        if let Some(lbl) = &mut self.label {
            lbl.set_style(style);
        } else {
            let mut lbl = label::Label::default();
            lbl.set_style(style);
            self.label = Some(lbl);
        }
    }

    /// Sets the label background fill. Lazily creates a default label if none exists.
    pub fn set_label_fill(&mut self, fill: Option<Color>) {
        if let Some(lbl) = &mut self.label {
            lbl.set_fill(fill);
        } else {
            let mut lbl = label::Label::default();
            lbl.set_fill(fill);
            self.label = Some(lbl);
        }
    }

    pub fn label_mut(&mut self) -> Option<&mut label::Label> {
        self.label.as_mut()
    }

    // === Property getters ===

    /// Returns the series name, if any.
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Returns the series color, if any.
    pub fn color_value(&self) -> Option<&Color> {
        self.color.as_ref()
    }

    /// Returns the data points.
    pub fn points(&self) -> &[Datum] {
        &self.points
    }

    /// Returns the stroke width, if any.
    pub fn stroke_value(&self) -> Option<f32> {
        self.stroke
    }

    /// Returns the fill opacity.
    pub fn opacity_value(&self) -> f32 {
        self.opacity
    }

    /// Returns a reference to the label configuration, if any.
    pub fn label(&self) -> Option<&label::Label> {
        self.label.as_ref()
    }
}

impl<T: IntoDatums> From<T> for Series {
    fn from(data: T) -> Self {
        Series::new(data)
    }
}

#[derive(Debug, Clone)]
pub struct Area {
    pub(crate) series: Vec<Series>,
    pub(crate) layout: Layout,
    /// When true, each series' fill is rendered as a vertical linear gradient
    /// (fill color at the top, transparent at the baseline) instead of a flat
    /// translucent color.
    pub(crate) gradient: bool,
}

/// Creates a single area series.
pub fn area(data: impl IntoDatums) -> Series {
    Series::new(data)
}

/// Creates an area chart from multiple series.
pub fn areas(series: impl IntoAreas) -> Area {
    series.into_areas()
}

pub trait IntoAreas {
    fn into_areas(self) -> Area;
}

impl IntoAreas for Series {
    fn into_areas(self) -> Area {
        Area {
            series: vec![self],
            layout: Layout::default(),
            gradient: true,
        }
    }
}

impl<const N: usize> IntoAreas for [Series; N] {
    fn into_areas(self) -> Area {
        Area {
            series: self.into(),
            layout: Layout::default(),
            gradient: true,
        }
    }
}

impl IntoAreas for Vec<Series> {
    fn into_areas(self) -> Area {
        Area {
            series: self,
            layout: Layout::default(),
            gradient: true,
        }
    }
}

impl Area {
    pub fn from_series(series: Vec<Series>) -> Self {
        Self {
            series,
            layout: Layout::default(),
            gradient: true,
        }
    }

    /// Enable or disable a vertical linear gradient fill.
    ///
    /// When enabled, each series is filled with a gradient from the series
    /// color at the top to fully transparent at the baseline. Falls back to
    /// the default translucent solid fill when `false`.
    pub fn gradient(mut self, enabled: bool) -> Self {
        self.gradient = enabled;
        self
    }

    pub fn stacked(mut self) -> Self {
        self.layout = Layout::Stacked;
        self
    }

    pub fn overlaid(mut self) -> Self {
        self.layout = Layout::Overlaid;
        self
    }

    pub fn with_layout(mut self, layout: Layout) -> Self {
        self.layout = layout;
        self
    }

    pub fn layout(&self) -> Layout {
        self.layout
    }

    pub fn all_series(&self) -> &[Series] {
        &self.series
    }

    pub fn series(&self, index: usize) -> Option<&Series> {
        self.series.get(index)
    }

    pub fn series_mut(&mut self) -> &mut Vec<Series> {
        &mut self.series
    }

    /// Applies the given label configuration to every series in the chart.
    ///
    /// Mirrors `Bars::data_labels` so callers can set a single label spec at
    /// the chart level instead of repeating it per series.
    pub fn data_labels(mut self, label: impl Into<Option<label::Label>>) -> Self {
        let label_config = label.into();
        for series in &mut self.series {
            series.label = label_config.clone();
        }
        self
    }

    // === Property setters (in-place) ===

    /// Sets the layout strategy in place.
    pub fn set_layout(&mut self, layout: Layout) {
        self.layout = layout;
    }

    /// Toggles the vertical gradient fill in place.
    pub fn set_gradient(&mut self, enabled: bool) {
        self.gradient = enabled;
    }

    /// Returns whether the vertical gradient fill is enabled.
    pub fn is_gradient(&self) -> bool {
        self.gradient
    }

    /// Creates the appropriate x-axis for an area chart.
    ///
    /// Area charts (when using auto-enumerated data) have index x-axes with:
    /// - `Kind::Index` for proper bounds (integer-aligned, no fractional padding)
    /// - Labels placed on ticks
    /// - Continuous tick style
    pub fn x_axis() -> Axis {
        Axis::new(Orientation::Bottom)
            .with_kind(Kind::Index)
            .labels(Placement::OnTicks)
            .with_ticks(axis::tick::Ticks::continuous())
    }

    /// Creates the appropriate y-axis for an area chart.
    ///
    /// Area charts have scalar y-axes anchored at zero:
    /// - `Kind::ScalarAnchored` for proper bounds (anchored at 0 for fill baseline)
    /// - Continuous tick style
    pub fn y_axis() -> Axis {
        Axis::new(Orientation::Left)
            .with_kind(Kind::ScalarAnchored)
            .with_ticks(axis::tick::Ticks::continuous())
    }
}

impl From<Area> for crate::Data {
    fn from(area: Area) -> Self {
        use crate::data::IntoData;
        area.into_data()
    }
}

#[macro_export]
macro_rules! areas {
    ($data:expr) => {
        $crate::areas($data)
    };
    ($($x:expr),+ $(,)?) => {
        $crate::mark::area::Area::from_series(vec![$($crate::mark::area::Series::from($x)),+])
    };
}
