mod area_basic;
mod area_stacked;
mod bar_basic;
mod bar_horizontal;
mod bar_labels;
mod bar_stacked;
mod boxplot_basic;
mod cartesian;
mod diagrams;
mod fields;
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
mod spatial;
mod spectra;
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
    pub build: Builder,
    pub source: &'static str,
}

pub enum Builder {
    Flat(fn() -> Data),
    Orbital(fn(hyozu::spatial::Camera) -> Data),
}
impl Variant {
    pub fn render(&self, camera: hyozu::spatial::Camera) -> Data {
        match self.build {
            Builder::Flat(build) => build(),
            Builder::Orbital(build) => build(camera),
        }
    }
}

/// Returns the full list of chart variants shown in the gallery.
pub fn variants() -> Vec<Variant> {
    vec![
        Variant {
            id: ChartId(0),
            name: "Line",
            category: Category::Line,
            build: Builder::Flat(line_basic::data),
            source: include_str!("charts/line_basic.rs"),
        },
        Variant {
            id: ChartId(1),
            name: "Multi Line",
            category: Category::Line,
            build: Builder::Flat(line_multi::data),
            source: include_str!("charts/line_multi.rs"),
        },
        Variant {
            id: ChartId(2),
            name: "Bar",
            category: Category::Bar,
            build: Builder::Flat(bar_basic::data),
            source: include_str!("charts/bar_basic.rs"),
        },
        Variant {
            id: ChartId(3),
            name: "Horizontal Bar",
            category: Category::Bar,
            build: Builder::Flat(bar_horizontal::data),
            source: include_str!("charts/bar_horizontal.rs"),
        },
        Variant {
            id: ChartId(4),
            name: "Stacked Bar",
            category: Category::Bar,
            build: Builder::Flat(bar_stacked::data),
            source: include_str!("charts/bar_stacked.rs"),
        },
        Variant {
            id: ChartId(5),
            name: "Bar Labels",
            category: Category::Bar,
            build: Builder::Flat(bar_labels::data),
            source: include_str!("charts/bar_labels.rs"),
        },
        Variant {
            id: ChartId(6),
            name: "Pie",
            category: Category::Pie,
            build: Builder::Flat(pie_basic::data),
            source: include_str!("charts/pie_basic.rs"),
        },
        Variant {
            id: ChartId(7),
            name: "Donut",
            category: Category::Pie,
            build: Builder::Flat(pie_donut::data),
            source: include_str!("charts/pie_donut.rs"),
        },
        Variant {
            id: ChartId(8),
            name: "Scatter",
            category: Category::Scatter,
            build: Builder::Flat(scatter_basic::data),
            source: include_str!("charts/scatter_basic.rs"),
        },
        Variant {
            id: ChartId(9),
            name: "Scatter (Iris)",
            category: Category::Scatter,
            build: Builder::Flat(scatter_multi::data),
            source: include_str!("charts/scatter_multi.rs"),
        },
        Variant {
            id: ChartId(10),
            name: "Area",
            category: Category::Area,
            build: Builder::Flat(area_basic::data),
            source: include_str!("charts/area_basic.rs"),
        },
        Variant {
            id: ChartId(11),
            name: "Stacked Area",
            category: Category::Area,
            build: Builder::Flat(area_stacked::data),
            source: include_str!("charts/area_stacked.rs"),
        },
        Variant {
            id: ChartId(12),
            name: "Heatmap",
            category: Category::Heatmap,
            build: Builder::Flat(heatmap_basic::data),
            source: include_str!("charts/heatmap_basic.rs"),
        },
        Variant {
            id: ChartId(13),
            name: "Heatmap (Temp)",
            category: Category::Heatmap,
            build: Builder::Flat(heatmap_warm::data),
            source: include_str!("charts/heatmap_warm.rs"),
        },
        Variant {
            id: ChartId(14),
            name: "Gauge",
            category: Category::Gauge,
            build: Builder::Flat(gauge_basic::data),
            source: include_str!("charts/gauge_basic.rs"),
        },
        Variant {
            id: ChartId(15),
            name: "Gauge (Zones)",
            category: Category::Gauge,
            build: Builder::Flat(gauge_zones::data),
            source: include_str!("charts/gauge_zones.rs"),
        },
        Variant {
            id: ChartId(16),
            name: "Box Plot",
            category: Category::BoxPlot,
            build: Builder::Flat(boxplot_basic::data),
            source: include_str!("charts/boxplot_basic.rs"),
        },
        Variant {
            id: ChartId(17),
            name: "Violin",
            category: Category::Violin,
            build: Builder::Flat(violin_basic::data),
            source: include_str!("charts/violin_basic.rs"),
        },
        Variant {
            id: ChartId(18),
            name: "Waterfall",
            category: Category::Waterfall,
            build: Builder::Flat(waterfall_basic::data),
            source: include_str!("charts/waterfall_basic.rs"),
        },
        Variant {
            id: ChartId(19),
            name: "Histogram",
            category: Category::Statistical,
            build: Builder::Flat(crate::distributions::histogram),
            source: include_str!("distributions.rs"),
        },
        Variant {
            id: ChartId(20),
            name: "Frequency Polygon",
            category: Category::Statistical,
            build: Builder::Flat(crate::distributions::frequency_polygon),
            source: include_str!("distributions.rs"),
        },
        Variant {
            id: ChartId(21),
            name: "Kernel Density",
            category: Category::Statistical,
            build: Builder::Flat(crate::distributions::density),
            source: include_str!("distributions.rs"),
        },
        Variant {
            id: ChartId(22),
            name: "ECDF",
            category: Category::Statistical,
            build: Builder::Flat(crate::distributions::ecdf),
            source: include_str!("distributions.rs"),
        },
        Variant {
            id: ChartId(23),
            name: "Q–Q Plot",
            category: Category::Statistical,
            build: Builder::Flat(crate::distributions::qq),
            source: include_str!("distributions.rs"),
        },
        Variant {
            id: ChartId(24),
            name: "Pareto",
            category: Category::Statistical,
            build: Builder::Flat(crate::distributions::pareto),
            source: include_str!("distributions.rs"),
        },
        Variant {
            id: ChartId(25),
            name: "Radar",
            category: Category::Diagram,
            build: Builder::Flat(diagrams::radar),
            source: include_str!("charts/diagrams.rs"),
        },
        Variant {
            id: ChartId(26),
            name: "Radial Bars",
            category: Category::Diagram,
            build: Builder::Flat(diagrams::radial_bars),
            source: include_str!("charts/diagrams.rs"),
        },
        Variant {
            id: ChartId(27),
            name: "Funnel",
            category: Category::Diagram,
            build: Builder::Flat(diagrams::funnel),
            source: include_str!("charts/diagrams.rs"),
        },
        Variant {
            id: ChartId(28),
            name: "Sunburst",
            category: Category::Diagram,
            build: Builder::Flat(diagrams::sunburst),
            source: include_str!("charts/diagrams.rs"),
        },
        Variant {
            id: ChartId(29),
            name: "Hierarchical Treemap",
            category: Category::Diagram,
            build: Builder::Flat(diagrams::hierarchical_treemap),
            source: include_str!("charts/diagrams.rs"),
        },
        Variant {
            id: ChartId(30),
            name: "Sankey",
            category: Category::Diagram,
            build: Builder::Flat(diagrams::sankey),
            source: include_str!("charts/diagrams.rs"),
        },
        Variant {
            id: ChartId(31),
            name: "Candlesticks",
            category: Category::Financial,
            build: Builder::Flat(cartesian::candlesticks),
            source: include_str!("charts/cartesian.rs"),
        },
        Variant {
            id: ChartId(32),
            name: "OHLC",
            category: Category::Financial,
            build: Builder::Flat(cartesian::ohlc),
            source: include_str!("charts/cartesian.rs"),
        },
        Variant {
            id: ChartId(33),
            name: "Ribbon",
            category: Category::Statistical,
            build: Builder::Flat(cartesian::ribbon),
            source: include_str!("charts/cartesian.rs"),
        },
        Variant {
            id: ChartId(34),
            name: "Error bars",
            category: Category::Statistical,
            build: Builder::Flat(cartesian::error_bars),
            source: include_str!("charts/cartesian.rs"),
        },
        Variant {
            id: ChartId(35),
            name: "Step",
            category: Category::Signal,
            build: Builder::Flat(cartesian::step),
            source: include_str!("charts/cartesian.rs"),
        },
        Variant {
            id: ChartId(36),
            name: "Stem",
            category: Category::Signal,
            build: Builder::Flat(cartesian::stem),
            source: include_str!("charts/cartesian.rs"),
        },
        Variant {
            id: ChartId(37),
            name: "Event raster",
            category: Category::Signal,
            build: Builder::Flat(cartesian::events),
            source: include_str!("charts/cartesian.rs"),
        },
        Variant {
            id: ChartId(38),
            name: "Contour lines",
            category: Category::Field,
            build: Builder::Flat(fields::contour),
            source: include_str!("charts/fields.rs"),
        },
        Variant {
            id: ChartId(39),
            name: "Filled contours",
            category: Category::Field,
            build: Builder::Flat(fields::filled),
            source: include_str!("charts/fields.rs"),
        },
        Variant {
            id: ChartId(40),
            name: "Triangular mesh",
            category: Category::Field,
            build: Builder::Flat(fields::wireframe),
            source: include_str!("charts/fields.rs"),
        },
        Variant {
            id: ChartId(41),
            name: "2D histogram",
            category: Category::Field,
            build: Builder::Flat(fields::histogram),
            source: include_str!("charts/fields.rs"),
        },
        Variant {
            id: ChartId(42),
            name: "Hexbin",
            category: Category::Field,
            build: Builder::Flat(fields::hexbin),
            source: include_str!("charts/fields.rs"),
        },
        Variant {
            id: ChartId(43),
            name: "Variable-width tiles",
            category: Category::Field,
            build: Builder::Flat(fields::tiles),
            source: include_str!("charts/fields.rs"),
        },
        Variant {
            id: ChartId(44),
            name: "Quiver",
            category: Category::Field,
            build: Builder::Flat(fields::quiver),
            source: include_str!("charts/fields.rs"),
        },
        Variant {
            id: ChartId(45),
            name: "Wind barbs",
            category: Category::Field,
            build: Builder::Flat(fields::barbs),
            source: include_str!("charts/fields.rs"),
        },
        Variant {
            id: ChartId(46),
            name: "Streamlines",
            category: Category::Field,
            build: Builder::Flat(fields::streamlines),
            source: include_str!("charts/fields.rs"),
        },
        Variant {
            id: ChartId(47),
            name: "3D surface",
            category: Category::Spatial,
            build: Builder::Orbital(spatial::surface),
            source: include_str!("charts/spatial.rs"),
        },
        Variant {
            id: ChartId(48),
            name: "3D wireframe",
            category: Category::Spatial,
            build: Builder::Orbital(spatial::wireframe),
            source: include_str!("charts/spatial.rs"),
        },
        Variant {
            id: ChartId(49),
            name: "3D scatter",
            category: Category::Spatial,
            build: Builder::Orbital(spatial::scatter),
            source: include_str!("charts/spatial.rs"),
        },
        Variant {
            id: ChartId(50),
            name: "3D line",
            category: Category::Spatial,
            build: Builder::Orbital(spatial::line),
            source: include_str!("charts/spatial.rs"),
        },
        Variant {
            id: ChartId(51),
            name: "3D bars",
            category: Category::Spatial,
            build: Builder::Orbital(spatial::bars),
            source: include_str!("charts/spatial.rs"),
        },
        Variant {
            id: ChartId(52),
            name: "Voxels",
            category: Category::Spatial,
            build: Builder::Orbital(spatial::voxels),
            source: include_str!("charts/spatial.rs"),
        },
        Variant {
            id: ChartId(53),
            name: "Stacked dots",
            category: Category::Statistical,
            build: Builder::Flat(crate::distributions::dotplot),
            source: include_str!("distributions.rs"),
        },
        Variant {
            id: ChartId(54),
            name: "Survival curve",
            category: Category::Statistical,
            build: Builder::Flat(crate::distributions::survival),
            source: include_str!("distributions.rs"),
        },
        Variant {
            id: ChartId(55),
            name: "Linear regression",
            category: Category::Statistical,
            build: Builder::Flat(crate::distributions::fit),
            source: include_str!("distributions.rs"),
        },
        Variant {
            id: ChartId(56),
            name: "LOESS",
            category: Category::Statistical,
            build: Builder::Flat(crate::distributions::smooth),
            source: include_str!("distributions.rs"),
        },
        Variant {
            id: ChartId(57),
            name: "Jitter",
            category: Category::Scatter,
            build: Builder::Flat(crate::distributions::jitter),
            source: include_str!("distributions.rs"),
        },
        Variant {
            id: ChartId(58),
            name: "100% stacked bars",
            category: Category::Bar,
            build: Builder::Flat(crate::distributions::percent_bars),
            source: include_str!("distributions.rs"),
        },
        Variant {
            id: ChartId(59),
            name: "100% stacked area",
            category: Category::Area,
            build: Builder::Flat(crate::distributions::percent_areas),
            source: include_str!("distributions.rs"),
        },
        Variant {
            id: ChartId(60),
            name: "Magnitude spectrum",
            category: Category::Signal,
            build: Builder::Flat(spectra::amplitude),
            source: include_str!("charts/spectra.rs"),
        },
        Variant {
            id: ChartId(61),
            name: "Phase spectrum",
            category: Category::Signal,
            build: Builder::Flat(spectra::phase),
            source: include_str!("charts/spectra.rs"),
        },
        Variant {
            id: ChartId(62),
            name: "Power spectral density",
            category: Category::Signal,
            build: Builder::Flat(spectra::psd),
            source: include_str!("charts/spectra.rs"),
        },
        Variant {
            id: ChartId(63),
            name: "Spectrogram",
            category: Category::Signal,
            build: Builder::Flat(spectra::spectrogram),
            source: include_str!("charts/spectra.rs"),
        },
        Variant {
            id: ChartId(64),
            name: "Autocorrelation",
            category: Category::Signal,
            build: Builder::Flat(spectra::autocorrelation),
            source: include_str!("charts/spectra.rs"),
        },
        Variant {
            id: ChartId(65),
            name: "Coherence",
            category: Category::Signal,
            build: Builder::Flat(spectra::coherence),
            source: include_str!("charts/spectra.rs"),
        },
        Variant {
            id: ChartId(66),
            name: "Pie of pie",
            category: Category::Pie,
            build: Builder::Flat(diagrams::split_pie),
            source: include_str!("charts/diagrams.rs"),
        },
        Variant {
            id: ChartId(67),
            name: "Bar of pie",
            category: Category::Pie,
            build: Builder::Flat(diagrams::split_bar),
            source: include_str!("charts/diagrams.rs"),
        },
    ]
}
