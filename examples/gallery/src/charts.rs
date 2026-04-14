mod area_basic;
mod area_stacked;
mod bar_basic;
mod bar_horizontal;
mod bar_labels;
mod bar_stacked;
mod boxplot_basic;
mod gauge_basic;
mod gauge_zones;
mod heatmap_basic;
mod heatmap_warm;
mod line_basic;
mod line_multi;
mod pie_basic;
mod pie_donut;
mod scatter_basic;
mod scatter_multi;
mod violin_basic;
mod waterfall_basic;

use hyozu::Data;

use crate::category::Category;

/// Identifies a chart variant by index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChartId(pub usize);

/// A gallery chart variant: metadata plus a factory function that builds the [`Data`].
pub struct Variant {
    pub id: ChartId,
    pub name: &'static str,
    pub category: Category,
    pub build: fn() -> Data,
    pub source: &'static str,
}

/// Returns the full list of chart variants shown in the gallery.
pub fn variants() -> Vec<Variant> {
    vec![
        Variant {
            id: ChartId(0),
            name: "Line",
            category: Category::Line,
            build: line_basic::data,
            source: include_str!("charts/line_basic.rs"),
        },
        Variant {
            id: ChartId(1),
            name: "Multi Line",
            category: Category::Line,
            build: line_multi::data,
            source: include_str!("charts/line_multi.rs"),
        },
        Variant {
            id: ChartId(2),
            name: "Bar",
            category: Category::Bar,
            build: bar_basic::data,
            source: include_str!("charts/bar_basic.rs"),
        },
        Variant {
            id: ChartId(3),
            name: "Horizontal Bar",
            category: Category::Bar,
            build: bar_horizontal::data,
            source: include_str!("charts/bar_horizontal.rs"),
        },
        Variant {
            id: ChartId(4),
            name: "Stacked Bar",
            category: Category::Bar,
            build: bar_stacked::data,
            source: include_str!("charts/bar_stacked.rs"),
        },
        Variant {
            id: ChartId(5),
            name: "Bar Labels",
            category: Category::Bar,
            build: bar_labels::data,
            source: include_str!("charts/bar_labels.rs"),
        },
        Variant {
            id: ChartId(6),
            name: "Pie",
            category: Category::Pie,
            build: pie_basic::data,
            source: include_str!("charts/pie_basic.rs"),
        },
        Variant {
            id: ChartId(7),
            name: "Donut",
            category: Category::Pie,
            build: pie_donut::data,
            source: include_str!("charts/pie_donut.rs"),
        },
        Variant {
            id: ChartId(8),
            name: "Scatter",
            category: Category::Scatter,
            build: scatter_basic::data,
            source: include_str!("charts/scatter_basic.rs"),
        },
        Variant {
            id: ChartId(9),
            name: "Scatter (Iris)",
            category: Category::Scatter,
            build: scatter_multi::data,
            source: include_str!("charts/scatter_multi.rs"),
        },
        Variant {
            id: ChartId(10),
            name: "Area",
            category: Category::Area,
            build: area_basic::data,
            source: include_str!("charts/area_basic.rs"),
        },
        Variant {
            id: ChartId(11),
            name: "Stacked Area",
            category: Category::Area,
            build: area_stacked::data,
            source: include_str!("charts/area_stacked.rs"),
        },
        Variant {
            id: ChartId(12),
            name: "Heatmap",
            category: Category::Heatmap,
            build: heatmap_basic::data,
            source: include_str!("charts/heatmap_basic.rs"),
        },
        Variant {
            id: ChartId(13),
            name: "Heatmap (Temp)",
            category: Category::Heatmap,
            build: heatmap_warm::data,
            source: include_str!("charts/heatmap_warm.rs"),
        },
        Variant {
            id: ChartId(14),
            name: "Gauge",
            category: Category::Gauge,
            build: gauge_basic::data,
            source: include_str!("charts/gauge_basic.rs"),
        },
        Variant {
            id: ChartId(15),
            name: "Gauge (Zones)",
            category: Category::Gauge,
            build: gauge_zones::data,
            source: include_str!("charts/gauge_zones.rs"),
        },
        Variant {
            id: ChartId(16),
            name: "Box Plot",
            category: Category::BoxPlot,
            build: boxplot_basic::data,
            source: include_str!("charts/boxplot_basic.rs"),
        },
        Variant {
            id: ChartId(17),
            name: "Violin",
            category: Category::Violin,
            build: violin_basic::data,
            source: include_str!("charts/violin_basic.rs"),
        },
        Variant {
            id: ChartId(18),
            name: "Waterfall",
            category: Category::Waterfall,
            build: waterfall_basic::data,
            source: include_str!("charts/waterfall_basic.rs"),
        },
    ]
}
