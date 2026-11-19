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

Regression and arbitrary-label examples also cover covariance ellipses and boxed text anchored to a coordinate. Each of the 37 named commands in Matplotlib’s plot-types index has a native structural counterpart. All named Recharts chart containers and the Office top-level chart families in the cited chart-type catalog have native entry points. General artist styling, streamlines on arbitrary meshes, arbitrary coordinate reference systems, full ggplot2 grammar extension and automatic Office file round-tripping are independent capabilities. Geographic rendering currently supports Mercator, equirectangular, cylindrical equal-area and Natural Earth projections. Fixed/free facets synchronize finite numeric primary-axis bounds at construction. Facets do not infer a dataset’s grouping variable.

## Reproduce

From the checkout run `cargo run --locked -p gallery`. The optional `raster` feature enables iced image textures and `spectral` enables RustFFT. The gallery enables both. Each live example is searchable by name or family. Selecting a card opens its full-size chart and source. The 3D examples provide camera rotation and the domain brush changes the detail chart’s x-axis.

| Native Rawr evidence | Inspect |
|---|---|
| Gallery at desktop and narrow widths | [Before](chart-parity/gallery-before.png), [after](chart-parity/gallery-after.png), [narrow source view](chart-parity/narrow-source.png) |
| Revenue by Segment, Q6 top label | [Clipped](chart-parity/q6-before.png), [fully visible](chart-parity/q6-after.png) |
| Independent live panels using one numeric domain | [Fixed facets](chart-parity/fixed-facets.png) |
| Data-space image and extruded native geometry | [Raster image](chart-parity/native-raster.png), [3D pie](chart-parity/extruded-pie.png) |
| Domain selection changes the detail chart's x-axis | [Initial range](chart-parity/brush-before.png), [adjusted range](chart-parity/brush-after.png) |

### Microsoft presentation variants

The gallery also exposes an individually exploded flat pie. The gallery includes smooth scatter with optional observation markers, shaded 2D bubbles with a radial highlight, outlined radar with optional vertex markers, clustered bars/columns on one depth plane, and 3D surface colors clipped at numeric value boundaries. The [shaded bubble capture](chart-parity/shaded-bubbles.png) shows the native scatter renderer’s per-point radial gradient. These variants correspond to Microsoft’s scatter/bubble, radar, clustered and surface entries. [Microsoft chart types](https://support.microsoft.com/en-us/excel/available-chart-types-in-office)

### Remaining interoperability differences

| | A — native geographic layers | **B — accept arbitrary source coordinate systems** |
|---|---|---|
| Current coverage | Polygon and multipolygon GeoJSON ingestion, native geographic points and text, four built-in projections | Requires CRS metadata, coordinate transformation and simple-feature dispatch for points, lines, polygons and geometry collections |
| Relevant comparison | Existing map and bubble-map families | ggplot2’s `geom_sf()`, `geom_sf_text()` and `geom_sf_label()` accept richer source geometry and projection pipelines |

**Recommend B for a future geographic interoperability change.** The four existing projections cover native map families. Reproducing `sf` behavior requires an explicit CRS-aware import contract and tests for mixed geometry collections. [ggplot2 simple features](https://ggplot2.tidyverse.org/reference/ggsf.html)

The remaining ggplot2 differences extend beyond chart families. There are no direct declarative equivalents to `stat_summary()`, `stat_summary_bin()`, `stat_summary_2d()`, `stat_manual()`, `stat_connect()`, or `stat_sf_coordinates()`. Callers can supply precomputed data, and the hexbin API accepts an arbitrary reducer. The covariance ellipse uses an ordinary sample covariance matrix. ggplot2 also offers a robust multivariate-t estimator. [ggplot2 statistics](https://ggplot2.tidyverse.org/reference/index.html)
