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
