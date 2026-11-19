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
