use crate::color::Color;
use crate::data::axis::{self, Axis, Kind, Orientation, Placement};
use crate::data::{Datum, IntoDatums};

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
}

impl Series {
    pub fn new(data: impl IntoDatums) -> Self {
        Self {
            points: data.into_datums(),
            color: None,
            stroke: Some(1.5),
            opacity: 0.4,
            name: None,
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

    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
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
        }
    }
}

impl<const N: usize> IntoAreas for [Series; N] {
    fn into_areas(self) -> Area {
        Area {
            series: self.into(),
            layout: Layout::default(),
        }
    }
}

impl IntoAreas for Vec<Series> {
    fn into_areas(self) -> Area {
        Area {
            series: self,
            layout: Layout::default(),
        }
    }
}

impl Area {
    pub fn from_series(series: Vec<Series>) -> Self {
        Self {
            series,
            layout: Layout::default(),
        }
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
