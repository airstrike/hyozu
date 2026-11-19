# Chart coverage

The baseline is Hyozu `2892f6c`. A chart family is usable when callers can supply their own data, render through Hyozu and exercise a representative example. Mathematical transformations and coordinate layouts have separate acceptance criteria.

| Source | Families and capabilities present at baseline | Additional work identified |
|---|---|---|
| [Recharts 3](https://recharts.github.io/en-US/api/) | Line, area, bar, scatter/bubble, pie/donut, basic treemap and composed Cartesian charts | Radar, radial bar, funnel, Sankey, sunburst and hierarchical treemap |
| [Excel and PowerPoint](https://support.microsoft.com/en-us/excel/available-chart-types-in-office) | Column/bar, stacked area/bar, line, scatter/bubble, pie/donut, waterfall, box and whisker, basic geographic maps | Histogram, Pareto, stock, radar, funnel, sunburst, surface, 100% stacks, pie-of-pie/bar-of-pie, 3D variants |
| [ggplot2](https://ggplot2.tidyverse.org/reference/) | Points, bars, lines, areas, violins, box plots, basic heatmaps and reference marks | Statistical transforms including binning, density, ECDF, Q–Q, smoothing, contours and hex bins. Geometry includes ribbons, ranges, segments, error bars and steps. Coordinate systems, scales and facets are separate capabilities |
| [Matplotlib](https://matplotlib.org/stable/plot_types/index.html) | Most common 2D pairs, basic categorical, distribution and image-like grid displays | Stem, stairs, events, intervals, 2D histograms, meshes, contour lines and fills, vectors/streamlines, triangulated fields, spectral plots and genuine 3D projections |

## Implemented

| API | Mathematical contract | Gallery |
|---|---|---|
| `stats::Bins::histogram` | Count or probability density with arbitrary strictly increasing edges, inclusive final edge | Histogram |
| `stats::Bins::frequency_polygon` | Joins bin midpoints at their count or density | Frequency Polygon |
| `stats::density` | Gaussian KDE using the library's violin estimator | Kernel Density |
| `stats::ecdf` | Right-continuous steps with simultaneous jumps for ties | ECDF |
| `stats::qq` | Sample order statistics against caller-supplied theoretical quantiles | Q–Q Plot |
| `stats::pareto` | Stable descending counts and cumulative fraction on a second axis | Pareto |
| `radar`, `radial_bars` | Equal-angle spokes with a common extent, or concentric angular bars | Radar, Radial Bars |
| `funnel` | Stage widths scaled to the maximum | Funnel |
| `sunburst`, `hierarchical_treemap` | Derived branch weights control nested angular or rectangular partitions | Sunburst, Hierarchical Treemap |
| `sankey` | Weighted acyclic directed graph, dependency-level columns, conserved link thickness | Sankey |
| `cartesian::{candlesticks, ohlc}` | Explicit open/high/low/close values, data-space widths and rising/falling series colors | Candlesticks, OHLC |
| `cartesian::{error_bars,ribbon,steps,stem,events,segments}` | Cartesian line segments, capped ranges, filled bounds, piecewise constant steps, impulses and event groups | Error bars, Ribbon, Step, Stem, Event raster |
| `field::Mesh::{grid,new,wireframe,contours,contour_bands}` | Piecewise linear interpolation over explicit or rectilinear triangular meshes, masked nonfinite samples and band clipping | Contour lines, Filled contours, Triangular mesh |
| `field::{histogram2d,hexbin,tiles}` | Rectilinear bin/density calculations, cube-rounded axial hexagons and unequal grid widths | 2D histogram, Hexbin, Variable-width tiles |
| `vector::{quiver,wind_barbs,Grid::streamlines}` | Arrow endpoints, 50/10/5 feathering, bilinear interpolation and bounded bidirectional RK4 traces | Quiver, Wind barbs, Streamlines |
| `spatial::{Scene,Camera}` | Orthographic or perspective projection of 3D lines, points, faces, rectilinear/triangular surfaces and cuboids. Geometry sorts by camera-space depth | 3D surface, wireframe, scatter, line, bars and voxels with live orbit controls |
| `stats::{dotplot,survival,percent_bars,percent_areas}` | One mark per included observation, complementary empirical distribution and aligned category normalization | Stacked dots, Survival curve, 100% stacked bars and area |
| `stats::fit::{linear,loess,jitter}` | Least squares with mean confidence band, locally weighted linear regression and reproducible jitter | Linear regression, LOESS, Jitter |
| `spectral::{Transform,correlation}` | Windowed FFT, one-sided amplitude and phase, Welch PSD, cross-spectral magnitude and coherence, time/frequency power density, discrete lag correlation | Magnitude spectrum, Phase spectrum, Power spectral density, Spectrogram, Autocorrelation, Coherence |
| `parts::split_pie` | User-selected tail categories combine into one main wedge and a second weighted pie or stacked bar | Pie of pie, Bar of pie |

The spectral implementation uses the optional `spectral` feature and [RustFFT](https://docs.rs/rustfft/latest/rustfft/struct.FftPlanner.html). Sampling and scaling follow [SciPy’s Welch definition](https://docs.scipy.org/doc/scipy/reference/generated/scipy.signal.welch.html) and [Matplotlib’s spectrogram](https://matplotlib.org/stable/api/_as_gen/matplotlib.axes.Axes.specgram.html).

## Reference acceptance inventory

The library exposes the chart types below to callers supplying their own data. The gallery provides runnable fixtures and the source for each example. “Structural coverage” means the documented mark, transform or coordinate layout exists. It does not establish identical defaults, file formats or every rendering parameter.

| Ecosystem and reference surface | Coverage through native Hyozu APIs | Representative evidence |
|---|---|---|
| Recharts 3 chart containers | Line, area, bar, scatter/bubble, combined Cartesian, pie/doughnut, radial bars, radar, funnel, treemap, Sankey and sunburst. Mouse hover and selection, keyboard traversal for Cartesian, pie and category diagram marks, shared x-domain selection with a two-handle brush. | Radar, Sankey, hierarchical treemap, domain brush. [Recharts API](https://recharts.github.io/en-US/api/) |
| Excel categories | Column/bar, line/area and stacks, scatter/bubble, pie/doughnut and split pies, stock OHLC and HLC with optional volume, surface/wireframe, radar, treemap/sunburst, histogram/Pareto, box/whisker, waterfall, funnel, map and dual-axis combinations. A native spatial scene supports grouped/stacked/normalized columns and bars, depth-separated areas and lines, and extruded/exploded pie wedges. | Stacked lines, nested doughnuts, price and volume, 3D categories, extruded pie. [Microsoft chart types](https://support.microsoft.com/en-us/excel/available-chart-types-in-office) |
| PowerPoint chart families | Uses the same Microsoft category catalog. The native renderer covers the listed families and variations above. | The fixtures use ordinary Hyozu widgets. PowerPoint document import/export belongs to the host application. [Microsoft chart types](https://support.microsoft.com/en-us/excel/available-chart-types-in-office) |
| ggplot2 standard geoms and statistics | Bars and cumulative stacks, line/path, points/counts/jitter, intervals in both orientations, crossbars/point ranges, ribbon, polygons, segments/Bézier/spokes, rug, explicit stair steps, text, reference marks, tiles/raster, irregular/regular contours, histogram/frequency polygon/hex cells, Gaussian 1D and 2D KDE, ECDF, Q–Q and reference line, OLS, LOESS, conditional quantile fit, sampled functions and box/violin. | Quantile regression, KDE, rug, Q–Q reference, polygon groups and fixed/free-scale panel grids. [ggplot2 reference](https://ggplot2.tidyverse.org/reference/index.html) |
| Matplotlib pairwise and statistical plot types | Plot, scatter, bar, stem, fill-between, stackplot, explicit edge stairs, histogram, boxplot, horizontal/vertical error bars, violin, event rows, rectangular histogram, hex counts or a caller-supplied scalar reducer, pie and ECDF. | Interval stairs, unequal-width bins, hexagon scalar means and horizontal error bars. [Matplotlib plot types](https://matplotlib.org/stable/plot_types/index.html) |
| Matplotlib gridded and irregular fields | Native RGBA and scalar image textures, curvilinear quad mesh, contours/isobands, wind barbs, quiver, streamlines, explicit triangle connectivity, triangle face colors and wireframes. | Raster image, curvilinear cells, irregular survey contours and flat triangle colors. [Matplotlib plot types](https://matplotlib.org/stable/plot_types/index.html) |
| Matplotlib 3D and volumetric plot types | World-coordinate bars, ruled fills between 3D traces, lines, arrows, scatter, stems, grid and triangulated surfaces, voxels and wireframes. Projection supports camera orbit and perspective. | Triangulated survey surface, 3D quiver, stem and ribbon. [Matplotlib plot types](https://matplotlib.org/stable/plot_types/index.html) |
| Matplotlib signal analysis | Windowed magnitude and wrapped/unwrapped phase in radians, Welch PSD and cross-spectral magnitude/coherence, spectrogram and discrete lag correlation. | Signal gallery and numerical FFT tests. [Matplotlib signal example](https://matplotlib.org/stable/api/_as_gen/matplotlib.axes.Axes.specgram.html) |

Regression and arbitrary-label examples also cover covariance ellipses and boxed text anchored to a coordinate. Each of the 37 named commands in Matplotlib’s plot-types index has a native structural counterpart. All named Recharts chart containers and the Office top-level chart families in the cited chart-type catalog have native entry points. General artist styling, streamlines on arbitrary meshes, full ggplot2 grammar extension and Office file round-tripping are independent capabilities. The original native basemap supports Mercator, equirectangular, cylindrical equal-area and Natural Earth. Mixed-feature layers can also use an external CRS definition. Fixed/free facets synchronize finite numeric primary-axis bounds at construction. Facets do not infer a dataset’s grouping variable.

## Reproduce

From the checkout run `cargo run --locked -p gallery`. The optional `raster` feature enables iced image textures and `spectral` enables RustFFT. The gallery enables both. Each live example is searchable by name or family. Selecting a card opens its full-size chart and source. The 3D examples provide camera rotation and the domain brush changes the detail chart’s x-axis.

| Native Rawr evidence | Inspect |
|---|---|
| Gallery at desktop and narrow widths | [Before](chart-parity/gallery-before.png), [after](chart-parity/gallery-after.png), [narrow source view](chart-parity/narrow-source.png) |
| Revenue by Segment, Q6 top label | [Clipped](chart-parity/q6-before.png), [fully visible](chart-parity/q6-after.png), [compact card in the final build](chart-parity/q6-compact.png) |
| Independent live panels using one numeric domain | [Fixed facets](chart-parity/fixed-facets.png) |
| Data-space image and extruded native geometry | [Raster image](chart-parity/native-raster.png), [3D pie](chart-parity/extruded-pie.png) |
| Projected mixed geometry, polygon hole and anchored labels | [UTM harbor survey](chart-parity/harbor-features.png) |
| Domain selection changes the detail chart's x-axis | [Initial range](chart-parity/brush-before.png), [adjusted range](chart-parity/brush-after.png) |

### Microsoft presentation variants

The gallery also exposes an individually exploded flat pie. The gallery includes smooth scatter with optional observation markers, shaded 2D bubbles with a radial highlight, outlined radar with optional vertex markers, clustered bars/columns on one depth plane, and 3D surface colors clipped at numeric value boundaries. The [shaded bubble capture](chart-parity/shaded-bubbles.png) shows the native scatter renderer’s per-point radial gradient. These variants correspond to Microsoft’s scatter/bubble, radar, clustered and surface entries. [Microsoft chart types](https://support.microsoft.com/en-us/excel/available-chart-types-in-office)

### Mixed geographic features and reference systems

The optional `simple-features` feature provides native geometry, text and boxed labels for points, lines, polygons, all three multi-geometries and recursive geometry collections. Feature IDs and attributes survive transformation. Polygon holes use native compound geometry and label anchors default to a point on the feature surface. The gallery includes a [mixed harbor survey](../examples/gallery/src/charts/harbor.geojson) rendered in UTM zone 18N. These are the chart-authoring counterparts to `geom_sf()`, `geom_sf_text()`, `geom_sf_label()` and `stat_sf_coordinates()`. [ggplot2 simple features](https://ggplot2.tidyverse.org/reference/ggsf.html)

Proj4rs handles its supported EPSG and PROJ definitions in Rust. Enable `native-proj` for upstream PROJ’s coordinate-operation database, WKT definitions and locally available datum grids. The layer also accepts a caller-provided transform. Source ordinates follow visualization XY order, with longitude then latitude in degrees for geographic CRSs. Conversion tests exercise Web Mercator, a UTM central meridian and an authority-free WKT projection. A high-accuracy operation that requires an external datum grid needs that grid installed and discoverable by PROJ. [Proj4rs](https://docs.rs/proj4rs/latest/proj4rs/) [PROJ resources](https://proj.org/en/stable/resource_files.html)

### Additional chart-generating statistics

| ggplot2 reference | Hyozu chart-authoring API |
|---|---|
| `stat_summary()`, `stat_summary_bin()` | `stats::summary::{at,bins}` reduce finite values at exact or binned coordinates. Reducers can return arbitrary values, including interval bounds. `mean_se` draws mean and standard error directly. |
| `stat_summary_2d()`, `stat_summary_hex()` | `field::{summary2d,hexbin_values}` apply a caller-defined statistic to each occupied cell; missing observations and empty cells remain masked. |
| `stat_manual()`, `stat_unique()` | `stats::summary::{groups,unique}` apply arbitrary per-group transformations and retain the first observation for each unique key. |
| `stat_align()` | `stats::aligned_areas` linearly interpolates irregular series onto their common knots, respects missing intervals and stacks positive and negative contributions separately. |
| `stat_connect()` | `cartesian::{steps,connect}` support horizontal/vertical, vertical/horizontal, midpoint and user-defined normalized transition paths; standard line marks connect linearly. |
| `stat_ellipse()` | `stats::{ellipse,student_ellipse}` offer ordinary sample covariance and iteratively reweighted multivariate-t scatter at a supplied radius. |

These APIs work on Rust data and native marks. They cover the chart-generating actions without interpreting R formulas or requiring a ggplot2 runtime. [ggplot2 layer statistics](https://ggplot2.tidyverse.org/reference/index.html)

### Additional Matplotlib plotting commands

The pyplot drawing reference also lists commands outside its 37-entry plot-types index. The gallery includes these three additional chart constructors.

| Command | Native entry point | Data contract |
|---|---|---|
| [Polar](https://matplotlib.org/stable/api/_as_gen/matplotlib.pyplot.polar.html) | `polar::lines` | Angular observations in radians and nonnegative radii with a common radial scale. An irregular sequence of angles and gaps is supported. |
| [Broken horizontal bars](https://matplotlib.org/stable/api/_as_gen/matplotlib.pyplot.broken_barh.html) | `cartesian::broken_bars` | Independent start/width spans in a data-space lane, including idle gaps between activities. |
| [Matrix sparsity](https://matplotlib.org/stable/api/_as_gen/matplotlib.pyplot.spy.html) | `field::sparsity` | Sparse row/column coordinates with duplicate positions merged, square cells and the matrix origin at the top left. |

The [123-entry catalog](chart-parity/gallery-catalog.png), [polar line](chart-parity/gallery-polar.png), [interval bars](chart-parity/gallery-intervals.png) and [sparse matrix](chart-parity/gallery-sparsity.png) were drawn offscreen with iced’s software renderer through the gallery’s own browse and detail views at 1200 × 820. The earlier desktop captures above came from Rawr driving the native gallery window.

### ggplot2 index audit

The ggplot2 4.0.4 reference lists 51 geometry names and 30 statistic names, including aliases and no-op layers. Grouped below, the geometry names omit the shared `geom_` prefix and statistic names omit `stat_`. Each row identifies the corresponding chart-authoring operation. [ggplot2 reference](https://ggplot2.tidyverse.org/reference/index.html)

| Operation | Geometry names | Statistic names | Native counterpart |
|---|---|---|---|
| Reference lines and layer scaffolding | abline, hline, vline, blank | identity | Rules, data-space segments, unchanged marks and empty data |
| Categorical and accumulated values | bar, col, area, ribbon | count, align | Native bars, grouped counts, aligned stacked areas and bounded ribbons |
| Curves, guides and observations | path, line, step, segment, curve, spoke, point, count, jitter, rug, function | sum, connect, function | Native lines/points, count-sized points, reproducible jitter, Cartesian geometry and function sampling |
| Quantitative ranges | crossbar, errorbar, errorbarh, linerange, pointrange | summary, summary_bin | Cartesian ranges and point/binned reducers |
| Univariate distributions | histogram, freqpoly, dotplot, density, violin, boxplot, qq, qq_line | bin, density, ydensity, boxplot, qq, qq_line, ecdf | Histogram counts or densities, Gaussian KDE, order statistics, quartiles and empirical distribution steps |
| Conditional and multivariate estimates | smooth, quantile | smooth, quantile, ellipse | Linear and local smooths, conditional quantile regression and data ellipses |
| Gridded and irregular fields | bin2d, bin_2d, hex, contour, contour_filled, density_2d, density_2d_filled, rect, tile, raster | bin_2d, bin_hex, contour, contour_filled, density_2d, density_2d_filled, summary_2d, summary_hex | Rectangular/hexagonal aggregation, scalar meshes, isolines, isobands, 2D Gaussian KDE and native textures |
| Polygons, text and geography | polygon, text, label, map, sf, sf_text, sf_label | sf, sf_coordinates | Cartesian paths, text plates, maps, projected mixed geometry and point-on-surface labels |
| Caller-defined tabular transforms | | manual, unique | Per-group transforms and key-based deduplication |
