# Hyozu decisions

## D1 — Extend the native chart model

**Date:** 2026-11-18 · **Context:** chart coverage on feat/chart-catalog · **Status:** decided

### Issue

Adding missing chart families requires both sample transformations and layouts without Cartesian axes. Downstream applications construct charts with `chart(&Data)`.

### Investigation

At `2892f6c`, `src/data.rs:140` selects default axes from the mark. `src/chart/plot_area.rs` already routes layout, rendering and event handling for each native mark. The normal density curve can reuse the estimator in `src/data/mark/violin.rs:52`.

### Options

| | A — one widget per family | **B — native marks and transforms** | C — independent canvas gallery |
|---|---|---|---|
| API | Separate entry points per widget | **Existing chart model, themes and axes** | Owns an additional renderer |
| Geometry | Repeated scene and hit-test code | **Reuse existing mark geometry or add a shared geometry mark** | Requires separate interaction handling |
| Consumer reach | Special adapters for each widget | **Library users and gallery share one path** | Gallery visitors |

### Decision

Use B. Compute statistical transforms when observations change. Use native geometry for layouts requiring a different coordinate system.

### Why

This preserves the widget lifecycle and lets callers style, compose and inspect chart data through familiar APIs.

### Followups

Revisit layout ownership when a family requires a coordinate transform the shared mark cannot represent.

## D2 — Paint data labels in the chart scene

**Date:** 2026-11-18 · **Context:** stacked bar label clipping · **Status:** decided

### Issue

The stacked-bar gallery card clipped the top half of Q6's 260 label. The title band still had space for the glyphs. Several mark renderers used the same clipping boundary.

### Investigation

At `2892f6c`, `src/chart/plot_area/bars.rs:1325` supplied the plot rectangle to the renderer for each paragraph. The bars had already been rendered through a separate canvas frame. Line, area, pie, gauge, heatmap, waterfall, treemap and annotation renderers used the same paragraph pattern.

### Options

| | **A — chart-scoped label pass** | B — iced Overlay |
|---|---|---|
| Geometry | **Marks retain their own plot frames. Labels share chart bounds and the parent viewport** | Independent window-relative layout |
| Interaction | **Cached paragraphs and click targets use the same placement rule** | Requires additional event routing |
| Rendering | **Available to ordinary chart drawing and export renderers** | Depends on overlay processing |

### Decision

Use A. Defer paragraph drawing until every chart mark has drawn. Reserve title, legend and guide rectangles when positioning labels. Retain ordinary standalone mark drawing entry points.

### Why

Labels remain part of the chart's rendering and interaction lifecycle while gaining access to unused space at the plot edges.

### Followups

Revisit obstacle granularity if unusually dense layouts require placement between individual guide ticks.

## D3 — Compose facets from independent chart widgets

**Date:** 2026-11-18 · **Context:** ggplot2 facet coverage · **Status:** decided

### Issue

A facet group needs coordinated axis bounds while preserving each panel’s marks, legend interactions and keyboard targets.

### Investigation

Hyozu already calculates numeric bounds from each Data instance and routes interaction through its chart widget. iced rows and columns can assign each panel an equal share of the available space.

### Options

| | **A — compose ordinary charts** | B — add a facet mark to one plot area |
|---|---|---|
| Axes and titles | **Each panel owns its axes and title** | Requires multiple local transforms and guide sets |
| Interaction | **Existing chart handles each panel’s events** | The plot must identify the panel before each hit test |
| Shared scale | **Pool bounds at grid construction** | Resolve bounds within one compound scene |

### Decision

Use A. The Grid accepts pregrouped chart data and the caller chooses fixed or free scales on either axis. The Gallery maps panel indices back to the corresponding Data instance.

### Why

The same charts can appear independently or inside a grid. Each panel retains its established renderer, input behavior and event mapping.

### Followups

None.

## D4 — Keep geographic projection engines optional

**Date:** 2026-11-18 · **Context:** mixed geographic feature layers · **Status:** decided

### Issue

Geographic points, lines, polygons and their labels need one projected coordinate space. WKT and datum-grid operations may require the PROJ database. The native gallery and web-compatible chart core also need an installation that can use Rust dependencies alone.

### Investigation

GeoRust provides geometry traversal and interior-point algorithms. GeoJSON converts all seven geometry forms into those native types. Proj4rs resolves EPSG and PROJ strings in Rust. The PROJ bindings can build the upstream C++ library from source and run a WKT-defined coordinate operation.

### Options

| | A — require native PROJ | B — Rust-only transforms | **C — optional projection engines** |
|---|---|---|---|
| Deployment | Native build and database for every map | Rust-only | **Rust-only default for feature layers, native PROJ on request** |
| Coordinates | EPSG, WKT and installed datum grids | Subset implemented by Proj4rs | **Select the required operation through one layer API** |
| Existing maps | Would introduce a native dependency | Separate native model | **Existing map types and their feature requirements remain available** |

### Decision

Use C. The simple-features feature enables rich geometry, GeoJSON parsing and Proj4rs. The native-proj feature enables PROJ’s CRS resolver. Layer transformations happen ahead of widget layout. A caller can supply its own fallible transform and label-anchor function.

### Why

A geographic feature keeps its properties and identity through projection, drawing, hit testing and label placement. Polygon holes share one even–odd path with picking. Built-in chart families can still compile without either projection backend.

### Followups

None.
