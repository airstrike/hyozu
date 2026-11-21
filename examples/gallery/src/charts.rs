mod annotations;
mod area_basic;
mod area_stacked;
mod axes;
mod bar_basic;
mod bar_horizontal;
mod bar_labels;
mod bar_stacked;
mod boxplot_basic;
pub(super) mod brush;
mod cartesian;
mod diagrams;
mod facets;
mod features;
mod fields;
mod gauge_basic;
mod gauge_zones;
mod heatmap_basic;
mod heatmap_warm;
pub(super) mod interaction;
mod line_basic;
mod line_multi;
mod patterns;
mod pie_basic;
mod pie_donut;
mod scatter_basic;
mod scatter_multi;
mod series_styles;
mod spatial;
mod specialty;
mod spectra;
mod summaries;
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
    pub source: std::borrow::Cow<'static, str>,
}

pub enum Builder {
    Flat(fn() -> Data),
    Brush(fn() -> brush::Example),
    Facets(fn() -> hyozu::facet::Grid),
    Orbital(fn(hyozu::spatial::Camera) -> Data),
    Linked(fn() -> interaction::Linked),
    Zoom(fn() -> interaction::Zoomable),
}
pub enum Preview {
    Brush(Box<brush::Example>),
    Single(Box<Data>),
    Facets(hyozu::facet::Grid),
    Linked(Box<interaction::Linked>),
    Zoom(Box<interaction::Zoomable>),
}

impl Variant {
    pub fn render(&self, camera: hyozu::spatial::Camera) -> Preview {
        let chart = match self.build {
            Builder::Brush(build) => return Preview::Brush(Box::new(build())),
            Builder::Flat(build) => build(),
            Builder::Orbital(build) => build(camera),
            Builder::Facets(build) => return Preview::Facets(build()),
            Builder::Linked(build) => {
                let mut linked = build();
                for chart in &mut linked.charts {
                    *chart = chart.clone().palette(super::chart_colors(self.category));
                }
                return Preview::Linked(Box::new(linked));
            }
            Builder::Zoom(build) => return Preview::Zoom(Box::new(build())),
        };
        let chart = if chart.get_palette().is_some() {
            chart
        } else {
            chart.palette(super::chart_colors(self.category))
        };
        Preview::Single(Box::new(chart))
    }
}

impl Preview {
    pub fn title(&self) -> Option<&str> {
        match self {
            Self::Brush(ex) => ex.chart.get_title(),
            Self::Single(chart) => chart.get_title(),
            Self::Facets(_) => None,
            Self::Linked(linked) => linked.charts.first().and_then(Data::get_title),
            Self::Zoom(zoom) => zoom.chart.get_title(),
        }
    }
    pub fn perform(&mut self, panel: usize, action: hyozu::Action) {
        match self {
            Self::Brush(ex) => ex.chart.perform(action),
            Self::Single(chart) => chart.perform(action),
            Self::Facets(grid) => {
                if let Some(chart) = grid.panel_mut(panel) {
                    chart.perform(action);
                }
            }
            Self::Linked(linked) => linked.perform(&action),
            Self::Zoom(zoom) => zoom.perform(&action),
        }
    }
}

// Keep the example the reader selected at the start of a shared module.
// Retain the imports and helper functions so Copy code has the full source.
fn example_source(module: &'static str, path: &str) -> std::borrow::Cow<'static, str> {
    use std::borrow::Cow;
    let name = path.rsplit("::").next().unwrap_or(path).trim();
    let needle = format!("pub fn {name}(");
    let Some(pos) = module
        .match_indices(&needle)
        .find_map(|(i, _)| (i == 0 || module.as_bytes()[i - 1] == b'\n').then_some(i))
    else {
        return Cow::Borrowed(module);
    };
    let Some(end) = module[pos..].find("\n}\n").map(|i| pos + i + 3) else {
        return Cow::Borrowed(module);
    };
    let first_item = module
        .split_inclusive('\n')
        .take_while(|line| {
            ![
                "fn ",
                "pub fn ",
                "const ",
                "pub const ",
                "pub struct ",
                "struct ",
                "pub(super) fn ",
            ]
            .iter()
            .any(|prefix| line.starts_with(prefix))
        })
        .map(str::len)
        .sum::<usize>();
    if first_item >= pos {
        return Cow::Borrowed(module);
    }
    Cow::Owned(format!(
        "{}{}\n{}{}",
        &module[..first_item],
        &module[pos..end],
        &module[first_item..pos],
        &module[end..]
    ))
}

/// Returns the full list of chart variants shown in the gallery.
pub fn variants() -> Vec<Variant> {
    macro_rules! variant {
        ($id:literal, $name:literal, $category:ident, $builder:ident, $function:path, $source:literal) => {
            Variant {
                id: ChartId($id),
                name: $name,
                category: Category::$category,
                build: Builder::$builder($function),
                source: example_source(include_str!($source), stringify!($function)),
            }
        };
    }
    vec![
        variant!(0, "Line", Line, Flat, line_basic::data, "charts/line_basic.rs"),
        variant!(1, "Multi Line", Line, Flat, line_multi::data, "charts/line_multi.rs"),
        variant!(2, "Bar", Bar, Flat, bar_basic::data, "charts/bar_basic.rs"),
        variant!(
            3,
            "Horizontal Bar",
            Bar,
            Flat,
            bar_horizontal::data,
            "charts/bar_horizontal.rs"
        ),
        variant!(4, "Stacked Bar", Bar, Flat, bar_stacked::data, "charts/bar_stacked.rs"),
        variant!(5, "Bar Labels", Bar, Flat, bar_labels::data, "charts/bar_labels.rs"),
        variant!(6, "Pie", Pie, Flat, pie_basic::data, "charts/pie_basic.rs"),
        variant!(7, "Donut", Pie, Flat, pie_donut::data, "charts/pie_donut.rs"),
        variant!(
            8,
            "Scatter",
            Scatter,
            Flat,
            scatter_basic::data,
            "charts/scatter_basic.rs"
        ),
        variant!(
            9,
            "Scatter (Iris)",
            Scatter,
            Flat,
            scatter_multi::data,
            "charts/scatter_multi.rs"
        ),
        variant!(10, "Area", Area, Flat, area_basic::data, "charts/area_basic.rs"),
        variant!(
            11,
            "Stacked Area",
            Area,
            Flat,
            area_stacked::data,
            "charts/area_stacked.rs"
        ),
        variant!(
            12,
            "Heatmap",
            Heatmap,
            Flat,
            heatmap_basic::data,
            "charts/heatmap_basic.rs"
        ),
        variant!(
            13,
            "Heatmap (Temp)",
            Heatmap,
            Flat,
            heatmap_warm::data,
            "charts/heatmap_warm.rs"
        ),
        variant!(14, "Gauge", Gauge, Flat, gauge_basic::data, "charts/gauge_basic.rs"),
        variant!(
            15,
            "Gauge (Zones)",
            Gauge,
            Flat,
            gauge_zones::data,
            "charts/gauge_zones.rs"
        ),
        variant!(
            16,
            "Box Plot",
            BoxPlot,
            Flat,
            boxplot_basic::data,
            "charts/boxplot_basic.rs"
        ),
        variant!(17, "Violin", Violin, Flat, violin_basic::data, "charts/violin_basic.rs"),
        variant!(
            18,
            "Waterfall",
            Waterfall,
            Flat,
            waterfall_basic::data,
            "charts/waterfall_basic.rs"
        ),
        variant!(
            19,
            "Histogram",
            Statistical,
            Flat,
            crate::distributions::histogram,
            "distributions.rs"
        ),
        variant!(
            20,
            "Frequency Polygon",
            Statistical,
            Flat,
            crate::distributions::frequency_polygon,
            "distributions.rs"
        ),
        variant!(
            21,
            "Kernel Density",
            Statistical,
            Flat,
            crate::distributions::density,
            "distributions.rs"
        ),
        variant!(
            22,
            "ECDF",
            Statistical,
            Flat,
            crate::distributions::ecdf,
            "distributions.rs"
        ),
        variant!(
            23,
            "Q–Q Plot",
            Statistical,
            Flat,
            crate::distributions::qq,
            "distributions.rs"
        ),
        variant!(
            24,
            "Pareto",
            Statistical,
            Flat,
            crate::distributions::pareto,
            "distributions.rs"
        ),
        variant!(25, "Radar", Diagram, Flat, diagrams::radar, "charts/diagrams.rs"),
        variant!(
            26,
            "Radial Bars",
            Diagram,
            Flat,
            diagrams::radial_bars,
            "charts/diagrams.rs"
        ),
        variant!(27, "Funnel", Diagram, Flat, diagrams::funnel, "charts/diagrams.rs"),
        variant!(28, "Sunburst", Diagram, Flat, diagrams::sunburst, "charts/diagrams.rs"),
        variant!(
            29,
            "Hierarchical Treemap",
            Diagram,
            Flat,
            diagrams::hierarchical_treemap,
            "charts/diagrams.rs"
        ),
        variant!(30, "Sankey", Diagram, Flat, diagrams::sankey, "charts/diagrams.rs"),
        variant!(
            31,
            "Candlesticks",
            Financial,
            Flat,
            cartesian::candlesticks,
            "charts/cartesian.rs"
        ),
        variant!(32, "OHLC", Financial, Flat, cartesian::ohlc, "charts/cartesian.rs"),
        variant!(
            33,
            "Ribbon",
            Statistical,
            Flat,
            cartesian::ribbon,
            "charts/cartesian.rs"
        ),
        variant!(
            34,
            "Error bars",
            Statistical,
            Flat,
            cartesian::error_bars,
            "charts/cartesian.rs"
        ),
        variant!(35, "Step", Signal, Flat, cartesian::step, "charts/cartesian.rs"),
        variant!(36, "Stem", Signal, Flat, cartesian::stem, "charts/cartesian.rs"),
        variant!(
            37,
            "Event raster",
            Signal,
            Flat,
            cartesian::events,
            "charts/cartesian.rs"
        ),
        variant!(38, "Contour lines", Field, Flat, fields::contour, "charts/fields.rs"),
        variant!(39, "Filled contours", Field, Flat, fields::filled, "charts/fields.rs"),
        variant!(
            40,
            "Triangular mesh",
            Field,
            Flat,
            fields::wireframe,
            "charts/fields.rs"
        ),
        variant!(41, "2D histogram", Field, Flat, fields::histogram, "charts/fields.rs"),
        variant!(42, "Hexbin", Field, Flat, fields::hexbin, "charts/fields.rs"),
        variant!(
            43,
            "Variable-width tiles",
            Field,
            Flat,
            fields::tiles,
            "charts/fields.rs"
        ),
        variant!(44, "Quiver", Field, Flat, fields::quiver, "charts/fields.rs"),
        variant!(45, "Wind barbs", Field, Flat, fields::barbs, "charts/fields.rs"),
        variant!(46, "Streamlines", Field, Flat, fields::streamlines, "charts/fields.rs"),
        variant!(
            47,
            "3D surface",
            Spatial,
            Orbital,
            spatial::surface,
            "charts/spatial.rs"
        ),
        variant!(
            48,
            "3D wireframe",
            Spatial,
            Orbital,
            spatial::wireframe,
            "charts/spatial.rs"
        ),
        variant!(
            49,
            "3D scatter",
            Spatial,
            Orbital,
            spatial::scatter,
            "charts/spatial.rs"
        ),
        variant!(50, "3D line", Spatial, Orbital, spatial::line, "charts/spatial.rs"),
        variant!(51, "3D bars", Spatial, Orbital, spatial::bars, "charts/spatial.rs"),
        variant!(52, "Voxels", Spatial, Orbital, spatial::voxels, "charts/spatial.rs"),
        variant!(
            53,
            "Stacked dots",
            Statistical,
            Flat,
            crate::distributions::dotplot,
            "distributions.rs"
        ),
        variant!(
            54,
            "Survival curve",
            Statistical,
            Flat,
            crate::distributions::survival,
            "distributions.rs"
        ),
        variant!(
            55,
            "Linear regression",
            Statistical,
            Flat,
            crate::distributions::fit,
            "distributions.rs"
        ),
        variant!(
            56,
            "LOESS",
            Statistical,
            Flat,
            crate::distributions::smooth,
            "distributions.rs"
        ),
        variant!(
            57,
            "Jitter",
            Scatter,
            Flat,
            crate::distributions::jitter,
            "distributions.rs"
        ),
        variant!(
            58,
            "100% stacked bars",
            Bar,
            Flat,
            crate::distributions::percent_bars,
            "distributions.rs"
        ),
        variant!(
            59,
            "100% stacked area",
            Area,
            Flat,
            crate::distributions::percent_areas,
            "distributions.rs"
        ),
        variant!(
            60,
            "Magnitude spectrum",
            Signal,
            Flat,
            spectra::amplitude,
            "charts/spectra.rs"
        ),
        variant!(61, "Phase spectrum", Signal, Flat, spectra::phase, "charts/spectra.rs"),
        variant!(
            62,
            "Power spectral density",
            Signal,
            Flat,
            spectra::psd,
            "charts/spectra.rs"
        ),
        variant!(
            63,
            "Spectrogram",
            Signal,
            Flat,
            spectra::spectrogram,
            "charts/spectra.rs"
        ),
        variant!(
            64,
            "Autocorrelation",
            Signal,
            Flat,
            spectra::autocorrelation,
            "charts/spectra.rs"
        ),
        variant!(65, "Coherence", Signal, Flat, spectra::coherence, "charts/spectra.rs"),
        variant!(66, "Pie of pie", Pie, Flat, diagrams::split_pie, "charts/diagrams.rs"),
        variant!(67, "Bar of pie", Pie, Flat, diagrams::split_bar, "charts/diagrams.rs"),
        variant!(68, "Small multiples", Facets, Facets, facets::fixed, "charts/facets.rs"),
        variant!(
            69,
            "Free-scale facets",
            Facets,
            Facets,
            facets::free,
            "charts/facets.rs"
        ),
        variant!(70, "Extruded pie", Spatial, Orbital, spatial::pie, "charts/spatial.rs"),
        variant!(
            71,
            "Stacked 3D columns",
            Spatial,
            Orbital,
            spatial::columns,
            "charts/spatial.rs"
        ),
        variant!(
            72,
            "100% stacked 3D bars",
            Spatial,
            Orbital,
            spatial::horizontal,
            "charts/spatial.rs"
        ),
        variant!(
            73,
            "3D ribbon lines",
            Spatial,
            Orbital,
            spatial::ribbons,
            "charts/spatial.rs"
        ),
        variant!(
            74,
            "3D grouped areas",
            Spatial,
            Orbital,
            spatial::areas,
            "charts/spatial.rs"
        ),
        variant!(
            75,
            "3D fill between",
            Spatial,
            Orbital,
            spatial::between,
            "charts/spatial.rs"
        ),
        variant!(76, "3D stems", Spatial, Orbital, spatial::stems, "charts/spatial.rs"),
        variant!(77, "3D quiver", Spatial, Orbital, spatial::quiver, "charts/spatial.rs"),
        variant!(78, "Curvilinear mesh", Field, Flat, fields::quad, "charts/fields.rs"),
        variant!(
            79,
            "Triangular face colors",
            Field,
            Flat,
            fields::tripcolor,
            "charts/fields.rs"
        ),
        variant!(
            80,
            "2D kernel density",
            Field,
            Flat,
            fields::density2d,
            "charts/fields.rs"
        ),
        variant!(81, "Raster image", Field, Flat, fields::image, "charts/fields.rs"),
        variant!(
            82,
            "Volume and OHLC",
            Financial,
            Flat,
            cartesian::stock_volume,
            "charts/cartesian.rs"
        ),
        variant!(
            83,
            "High low close",
            Financial,
            Flat,
            cartesian::high_low_close,
            "charts/cartesian.rs"
        ),
        variant!(
            84,
            "Horizontal error bars",
            Statistical,
            Flat,
            cartesian::horizontal_ranges,
            "charts/cartesian.rs"
        ),
        variant!(
            85,
            "Crossbars",
            Statistical,
            Flat,
            cartesian::crossbars,
            "charts/cartesian.rs"
        ),
        variant!(
            86,
            "Point ranges",
            Statistical,
            Flat,
            cartesian::points,
            "charts/cartesian.rs"
        ),
        variant!(
            87,
            "Interval stairs",
            Statistical,
            Flat,
            cartesian::stairs,
            "charts/cartesian.rs"
        ),
        variant!(
            88,
            "Sampled function",
            Statistical,
            Flat,
            cartesian::function,
            "charts/cartesian.rs"
        ),
        variant!(
            89,
            "Polygon groups",
            Statistical,
            Flat,
            cartesian::polygons,
            "charts/cartesian.rs"
        ),
        variant!(
            90,
            "Cubic curves",
            Statistical,
            Flat,
            cartesian::curves,
            "charts/cartesian.rs"
        ),
        variant!(
            91,
            "Spokes",
            Statistical,
            Flat,
            cartesian::spokes,
            "charts/cartesian.rs"
        ),
        variant!(92, "Rug", Statistical, Flat, cartesian::rug, "charts/cartesian.rs"),
        variant!(
            93,
            "Quantile regression",
            Statistical,
            Flat,
            crate::distributions::quantile_lines,
            "distributions.rs"
        ),
        variant!(
            94,
            "Q–Q reference",
            Statistical,
            Flat,
            crate::distributions::qq_reference,
            "distributions.rs"
        ),
        variant!(
            95,
            "Count by position",
            Statistical,
            Flat,
            crate::distributions::count,
            "distributions.rs"
        ),
        variant!(
            96,
            "100% stacked lines",
            Line,
            Flat,
            crate::distributions::stack_lines,
            "distributions.rs"
        ),
        variant!(97, "Nested doughnuts", Pie, Flat, diagrams::rings, "charts/diagrams.rs"),
        variant!(98, "Domain brush", Line, Brush, brush::example, "charts/brush.rs"),
        variant!(
            99,
            "Hexagon scalar means",
            Field,
            Flat,
            fields::hex_values,
            "charts/fields.rs"
        ),
        variant!(
            100,
            "Irregular contours",
            Field,
            Flat,
            fields::irregular,
            "charts/fields.rs"
        ),
        variant!(
            101,
            "Triangulated 3D surface",
            Spatial,
            Orbital,
            spatial::trisurf,
            "charts/spatial.rs"
        ),
        variant!(
            102,
            "Radar with markers",
            Diagram,
            Flat,
            diagrams::marker_radar,
            "charts/diagrams.rs"
        ),
        variant!(
            103,
            "Smooth scatter",
            Scatter,
            Flat,
            cartesian::smooth,
            "charts/cartesian.rs"
        ),
        variant!(
            104,
            "Shaded bubbles",
            Scatter,
            Flat,
            cartesian::spherical,
            "charts/cartesian.rs"
        ),
        variant!(
            105,
            "Clustered 3D columns",
            Spatial,
            Orbital,
            spatial::cluster,
            "charts/spatial.rs"
        ),
        variant!(
            106,
            "Banded 3D surface",
            Spatial,
            Orbital,
            spatial::banded,
            "charts/spatial.rs"
        ),
        variant!(
            107,
            "Data ellipse",
            Statistical,
            Flat,
            crate::distributions::ellipse,
            "distributions.rs"
        ),
        variant!(108, "Exploded pie", Pie, Flat, diagrams::exploded, "charts/diagrams.rs"),
        variant!(
            109,
            "Boxed annotations",
            Line,
            Flat,
            cartesian::boxed_labels,
            "charts/cartesian.rs"
        ),
        variant!(
            110,
            "Simple features",
            Geography,
            Flat,
            features::geometry,
            "charts/features.rs"
        ),
        variant!(
            111,
            "Feature text",
            Geography,
            Flat,
            features::names,
            "charts/features.rs"
        ),
        variant!(
            112,
            "Boxed feature labels",
            Geography,
            Flat,
            features::boxed,
            "charts/features.rs"
        ),
        variant!(
            113,
            "Mean and standard error",
            Statistical,
            Flat,
            summaries::point_range,
            "charts/summaries.rs"
        ),
        variant!(
            114,
            "Binned means",
            Statistical,
            Flat,
            summaries::binned,
            "charts/summaries.rs"
        ),
        variant!(
            115,
            "Grouped summaries",
            Statistical,
            Flat,
            summaries::grouped,
            "charts/summaries.rs"
        ),
        variant!(
            116,
            "Rectangular scalar means",
            Field,
            Flat,
            summaries::grid,
            "charts/summaries.rs"
        ),
        variant!(
            117,
            "Custom connection",
            Line,
            Flat,
            summaries::connector,
            "charts/summaries.rs"
        ),
        variant!(
            118,
            "Robust data ellipse",
            Statistical,
            Flat,
            summaries::robust,
            "charts/summaries.rs"
        ),
        variant!(
            119,
            "Aligned stacked area",
            Area,
            Flat,
            summaries::aligned,
            "charts/summaries.rs"
        ),
        variant!(120, "Polar line", Line, Flat, patterns::polar, "charts/patterns.rs"),
        variant!(
            121,
            "Broken horizontal bars",
            Bar,
            Flat,
            patterns::broken_bars,
            "charts/patterns.rs"
        ),
        variant!(
            122,
            "Matrix sparsity",
            Field,
            Flat,
            patterns::sparsity,
            "charts/patterns.rs"
        ),
        variant!(
            123,
            "Hinton diagram",
            Field,
            Flat,
            specialty::hinton,
            "charts/specialty.rs"
        ),
        variant!(
            124,
            "Ishikawa diagram",
            Diagram,
            Flat,
            specialty::fishbone,
            "charts/specialty.rs"
        ),
        variant!(
            125,
            "17-segment bullseye",
            Diagram,
            Flat,
            specialty::bullseye,
            "charts/specialty.rs"
        ),
        variant!(
            126,
            "Skew-T log-P",
            Field,
            Flat,
            specialty::skew_t,
            "charts/specialty.rs"
        ),
        variant!(
            127,
            "Streamgraph",
            Area,
            Flat,
            specialty::streamgraph,
            "charts/specialty.rs"
        ),
        variant!(
            128,
            "Polar bars",
            Bar,
            Flat,
            specialty::polar_bars,
            "charts/specialty.rs"
        ),
        variant!(
            129,
            "Polar scatter",
            Scatter,
            Flat,
            specialty::polar_scatter,
            "charts/specialty.rs"
        ),
        variant!(
            130,
            "Polar errors",
            Statistical,
            Flat,
            specialty::polar_errors,
            "charts/specialty.rs"
        ),
        variant!(
            131,
            "Hillshade",
            Field,
            Flat,
            specialty::hillshade,
            "charts/specialty.rs"
        ),
        variant!(
            132,
            "3D contours",
            Spatial,
            Orbital,
            specialty::contour_3d,
            "charts/specialty.rs"
        ),
        variant!(
            133,
            "3D contour bands",
            Spatial,
            Orbital,
            specialty::contour_bands_3d,
            "charts/specialty.rs"
        ),
        variant!(
            134,
            "Bihistogram",
            Statistical,
            Flat,
            specialty::bihistogram,
            "charts/specialty.rs"
        ),
        variant!(135, "Hat graph", Bar, Flat, specialty::hats, "charts/specialty.rs"),
        variant!(
            136,
            "Excel trendlines",
            Statistical,
            Flat,
            summaries::trendlines,
            "charts/summaries.rs"
        ),
        variant!(
            137,
            "Series error amounts",
            Statistical,
            Flat,
            summaries::error_amounts,
            "charts/summaries.rs"
        ),
        variant!(138, "Log–log", Scatter, Flat, axes::log_log, "charts/axes.rs"),
        variant!(
            139,
            "Reversed depth",
            Line,
            Flat,
            axes::reversed_depth,
            "charts/axes.rs"
        ),
        variant!(140, "Rotated labels", Bar, Flat, axes::rotated_labels, "charts/axes.rs"),
        variant!(141, "Origin axes", Line, Flat, axes::origin_axes, "charts/axes.rs"),
        variant!(
            142,
            "Line interpolation",
            Line,
            Flat,
            series_styles::interpolation,
            "charts/series_styles.rs"
        ),
        variant!(
            143,
            "Empty cells",
            Line,
            Flat,
            series_styles::empty_cells,
            "charts/series_styles.rs"
        ),
        variant!(
            144,
            "Hatched bars",
            Bar,
            Flat,
            series_styles::hatched_bars,
            "charts/series_styles.rs"
        ),
        variant!(
            145,
            "Hatched areas",
            Area,
            Flat,
            series_styles::hatched_area,
            "charts/series_styles.rs"
        ),
        variant!(
            146,
            "Hatched pie",
            Pie,
            Flat,
            series_styles::hatched_pie,
            "charts/series_styles.rs"
        ),
        variant!(
            147,
            "Drop and high-low lines",
            Line,
            Flat,
            series_styles::drop_and_high_low,
            "charts/series_styles.rs"
        ),
        variant!(
            148,
            "Up/down bars",
            Financial,
            Flat,
            series_styles::up_down_bars,
            "charts/series_styles.rs"
        ),
        variant!(
            149,
            "Bullet chart",
            Bar,
            Flat,
            series_styles::bullet_chart,
            "charts/series_styles.rs"
        ),
        variant!(
            150,
            "Wind rose",
            Diagram,
            Flat,
            diagrams::wind_rose,
            "charts/diagrams.rs"
        ),
        variant!(
            151,
            "Radar scale",
            Diagram,
            Flat,
            diagrams::radar_scale,
            "charts/diagrams.rs"
        ),
        variant!(
            152,
            "Polar categories",
            Line,
            Flat,
            diagrams::polar_months,
            "charts/diagrams.rs"
        ),
        variant!(
            153,
            "Linked hover",
            Line,
            Linked,
            interaction::linked,
            "charts/interaction.rs"
        ),
        variant!(154, "Facet wrap", Facets, Facets, facets::wrap, "charts/facets.rs"),
        variant!(155, "Facet grid", Facets, Facets, facets::grid, "charts/facets.rs"),
        variant!(
            156,
            "Zoom and pan",
            Scatter,
            Zoom,
            interaction::zoomable,
            "charts/interaction.rs"
        ),
        variant!(
            157,
            "Callouts",
            Bar,
            Flat,
            annotations::callouts,
            "charts/annotations.rs"
        ),
        variant!(
            158,
            "Log-axis callouts",
            Scatter,
            Flat,
            annotations::log_callouts,
            "charts/annotations.rs"
        ),
        variant!(
            159,
            "Log color scale",
            Heatmap,
            Flat,
            heatmap_warm::log_scale,
            "charts/heatmap_warm.rs"
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_entry_resolves_to_its_own_selection_and_builds() {
        for (index, v) in variants().iter().enumerate() {
            assert_eq!(v.id, ChartId(index));
            assert!(!v.source.is_empty());
            let _ = v.render(hyozu::spatial::Camera::default());
        }
    }
    #[test]
    fn source_opens_at_the_selected_constructor_and_preserves_helpers() {
        let v = variants();
        let src = v
            .iter()
            .find(|v| v.name == "3D quiver")
            .expect("fixture")
            .source
            .as_ref();
        assert!(src.find("pub fn quiver(") < src.find("fn project("));
        assert!(src.contains("use hyozu::spatial"));
    }
}
