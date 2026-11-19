#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Category {
    Line,
    Bar,
    Pie,
    Scatter,
    Area,
    Heatmap,
    Gauge,
    BoxPlot,
    Violin,
    Waterfall,
    Statistical,
    Diagram,
    Financial,
    Signal,
    Field,
    Spatial,
}

impl Category {
    pub const ALL: &[Self] = &[
        Self::Line,
        Self::Bar,
        Self::Pie,
        Self::Scatter,
        Self::Area,
        Self::Heatmap,
        Self::Gauge,
        Self::BoxPlot,
        Self::Violin,
        Self::Waterfall,
        Self::Statistical,
        Self::Diagram,
        Self::Financial,
        Self::Signal,
        Self::Field,
        Self::Spatial,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Line => "Line",
            Self::Bar => "Bar",
            Self::Pie => "Pie",
            Self::Scatter => "Scatter",
            Self::Area => "Area",
            Self::Heatmap => "Heatmap",
            Self::Gauge => "Gauge",
            Self::BoxPlot => "Box Plot",
            Self::Violin => "Violin",
            Self::Financial => "Financial",
            Self::Spatial => "3D",
            Self::Field => "Scientific fields",
            Self::Signal => "Signal",
            Self::Diagram => "Diagram",
            Self::Statistical => "Statistical",
            Self::Waterfall => "Waterfall",
        }
    }
}
