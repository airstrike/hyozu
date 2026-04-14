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
            Self::Waterfall => "Waterfall",
        }
    }
}
