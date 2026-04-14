# Changelog

All notable changes to hyozu are documented in this file.

The format is loosely based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project will follow [Semantic Versioning](https://semver.org/spec/v2.0.0.html)
once it stabilizes past 0.x.

## [0.2.0] — 2026-04-14

First substantive release. Hyozu is an iced-based grammar-of-graphics charting
library: declarative `Mark` data types compose into a `Data` value that a
single `chart()` widget renders.

### Chart types

- **Area** — overlaid or stacked, optional vertical gradient fill.
- **Bar** — vertical or horizontal, grouped or stacked, `corner_radius` for
  rounded tops, `color_by` fill encoding (see *Grammar of graphics* below),
  per-point color and label overrides.
- **Box plot** — vertical or horizontal, with whisker style controls.
- **Bubble map** — geo-projected bubbles at lat/lon with size and color
  channels.
- **Choropleth** — geo-projected polygon fills driven by a sequential color
  scale and a configurable normalization (linear / sqrt / log).
- **Gauge** — radial value indicator with zones, needle, subtitle, and
  zone-tinted gradient arcs.
- **Heatmap** — categorical row/column matrix with continuous color stops.
- **Line** — solid / dashed / dotted / custom-dash styles, optional markers,
  data labels.
- **Pie** — donut-mode supported, configurable label placement.
- **Reference band** — horizontal or vertical shaded value range with optional
  edge label.
- **Rule** — single horizontal or vertical reference line.
- **Tick** — single-axis tick marks for distribution displays.
- **Treemap** — squarified layout with pre-truncated labels.
- **Violin** — vertical density distributions.
- **Waterfall** — running-total bar chart with positive / negative / total
  semantic colors.
- **XY scatter** — two continuous axes with optional grouping.

### Sparkline widget

- Standalone `Sparkline` widget for inline charts in tables and dashboards,
  with line and bar rendering plus `Highlight::{First, Last, Min, Max}`
  rules in arbitrary colors.

### Grammar of graphics

- `crate::encoding` module with `Encoding<C>` and `channel::Fill`. Marks bind
  a visual channel to a closure over their data points; the resulting plan is
  computed once in layout and materialized against the active theme in draw.
  See `GOG.md` for the full spec.
- `Bars::color_by(encoding::key(...))` — derive each bar's color from a key
  extracted from its data point. The expensive distinct-key walk happens once
  per layout pass and is cached on `bars::State::series_fill_plans`.
- `bar::Series::resolved_color_at(...)` exposes the full color priority chain
  (`point_colors > color_by > series.color > palette`) so callers like the
  hover tooltip can match the actual displayed color.
- `Scene::user_palette()` lets encoded series inherit the chart's explicit
  palette flavor when one is set (GOG.md D17).

### Color & palette

- Adaptive `Color` enum with literal RGB, contrast-aware variants, and
  semantic seed slots (`Color::Primary`, `Color::Secondary`, `Color::Success`,
  `Color::Warning`, `Color::Danger`) that resolve against the active theme's
  `PaletteSeed` at draw time. Charts styled with `Color::Success` follow
  theme changes automatically.
- `Color::resolve(background, text_pair, &seed, fallback)` for theme-aware
  color resolution at the draw site.
- `Palette` with `Categorical` and `Sequential(Color)` variants — the inner
  `Color` is the hue source, so `Sequential(Color::Success)` produces a green
  sequence and `Sequential(Color::Fixed(rgb))` honors an explicit hex.
- `palette::categorical()`, `palette::sequential(...)`, and
  `palette::gradient(stops)` constructors. Gradient stops are wrapper
  `Color`s, so `palette::gradient([Color::Success, Color::Danger])` works
  with semantic colors.
- OKLCh-based perceptually uniform interpolation across the sequential and
  gradient palettes, with chroma taper at high lightness.

### Themes

- `theme::paper()` / `theme::paper_dark()` — warm, paper-like aesthetic.
- `theme::hyozu()` / `theme::hyozu_dark()` — icy mountains and cherry
  blossoms.
- `theme::basic()` / `theme::basic_light()` — anchored to shadcn/ui's Nova
  palette: vivid violet primary plus saturated green, amber, and red
  semantic accents.

### Charts & interactivity

- **Title, legend, plot area, axis** orchestration through one `chart()`
  widget. Legend supports left / right / above / below positioning, wrapping,
  and centering.
- **Interactive legend** with click-to-toggle series visibility.
- **Dual-axis** with independent secondary y-axis bounds.
- **Hover tooltips** with composable atoms (`Swatch`, `ColoredText`) that
  combine via `+`. Type-safe hover annotations through `chart::hover`.
- **Maps**: geo data model, projections, choropleth + bubble-map marks, and
  an interactive maps example.
- **Dashboard**: data-driven inboard dashboard example with interactive
  drill-down.
- **Gallery**: 19 chart variants in a v2 gallery example with detail view
  and icon set.

### Axis & layout

- `axis::Kind::{Index, Categorical, Scalar, ScalarAnchored}` for natural
  bounds inference per mark type.
- `axis::Placement::{OnTicks, BetweenTicks}` for label positioning.
- `axis::label::Overflow::{Ellipsize, Wrap}` strategy for horizontal axis
  labels that exceed their column. `Ellipsize` (default) truncates with `…`;
  `Wrap` wraps at word boundaries and grows the axis vertically.
- `axis::tick::Ticks::{continuous, discrete}` for tick generation.
- Major and minor gridlines configurable per axis.
- Categorical-axis natural padding crediting in edge-label inset
  calculation: short labels on `Kind::Categorical` axes don't needlessly
  shrink the chart.
- Series-driven inset floor: bars reserve plot-area inset space so labels
  that extend past bar ends don't clip against the boundary.
- `Axis::none()` builder that preserves axis `Kind` for downstream
  inference.

### Data labels

- `Label` configuration shared by line and area marks (re-exported as
  `area::label`), with `Position::{Auto, Above, Below, Left, Right}`,
  `Show::{Any, FirstOnly, LastOnly, FirstAndLast, MinMax*}`, custom format
  closures, font weight / style overrides, and background fill.
- Default `Position::Auto` runs a placement search that avoids the line
  itself, axis obstacles, and previously-placed labels — for multi-series
  area charts the search pools segments and rects across all series so
  labels flow around each other.
- `clamp_rect_to_bounds` guarantees first/last-point labels stay visible
  inside the plot area even when the centered ideal rect would bleed off
  the edge (`190` no longer renders as `19`).
- Bar labels respect a per-orientation extent: text width for horizontal
  bars, label height for vertical bars, so vertical-bar labels don't
  over-clip against the plot top.

### Architecture

- Layout-vs-draw discipline: per-element walks, allocations, and string
  formatting happen in `layout`; `draw` is O(1) per element. Choropleth
  pre-computes normalized values and legend strings; treemap pre-computes
  truncated labels; bars pre-compute fill encoding plans; line and area
  pre-compute label placement rects.
- All marks render via canvas `Frame + Path + draw_geometry` rather than
  per-element `fill_quad`, so a series with N elements emits one geometry
  call.
