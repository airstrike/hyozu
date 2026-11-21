use crate::chart::guide::{self, Guide};
use crate::chart::legend::{self, Legend};
use crate::chart::plot_area::PlotArea;
use crate::chart::title::{self, Title};
use crate::core::widget::{Tree, tree};
use crate::data::mark::Mark;
use crate::design;
use crate::palette::{self, Palette};

use crate::core::text;
use crate::widget::renderer::geometry;

/// Returns the mark's per-mark value-format closure when set.
///
/// Currently only `Mark::Pie` carries a value scale override; other
/// marks return `None` and fall through to the data-level scale. As
/// more marks gain per-mark scale overrides this match grows.
fn mark_value_format(mark: &Mark) -> Option<crate::scale::Format<f64>> {
    match mark {
        Mark::Pie(pie) => pie.value_scale().and_then(|s| s.format.clone()),
        Mark::Area(_)
        | Mark::Band(_)
        | Mark::Bars(_)
        | Mark::BoxPlot(_)
        | Mark::Choropleth(_)
        | Mark::Line(_)
        | Mark::Gauge(_)
        | Mark::Diagram(_)
        | Mark::Treemap(_)
        | Mark::Waterfall(_)
        | Mark::Xy(_)
        | Mark::Rule(_)
        | Mark::Tick(_)
        | Mark::Heatmap(_)
        | Mark::Violin(_)
        | Mark::Text(_)
        | Mark::Annotation(_) => None,
    }
}

fn legend_entries_for(
    data: &crate::Data,
) -> (
    Vec<crate::data::mark::LegendEntry>,
    Vec<Option<crate::scale::Format<f64>>>,
) {
    let mut entries = Vec::new();
    let mut entry_value_format = Vec::new();
    for mark in data.primary.marks().iter().chain(data.secondary.marks()) {
        let mark_format = mark_value_format(mark).or_else(|| data.value_scale().format.clone());
        for entry in mark.legend_entries() {
            entries.push(entry);
            entry_value_format.push(mark_format.clone());
        }
    }
    (entries, entry_value_format)
}

/// Scene organizes the 7 pieces of a chart.
/// Borrows everything from Data for lifetime 'a.
/// Follows the pane_grid::Content pattern from iced.
pub struct Scene<'a, Message, Renderer>
where
    Renderer: text::Renderer + geometry::Renderer,
{
    // The 7 pieces (6 optional, plot area required)
    title: Option<Title<'a, Message, Renderer>>,
    legend: Option<Legend<'a, Message, Renderer>>,
    top_axis: Option<Guide<'a, Message, Renderer>>,    // x_axis_secondary
    right_axis: Option<Guide<'a, Message, Renderer>>,  // y_axis_secondary
    bottom_axis: Option<Guide<'a, Message, Renderer>>, // x_axis_primary
    left_axis: Option<Guide<'a, Message, Renderer>>,   // y_axis_primary
    plot_area: PlotArea<'a, Message, Renderer>,        // plot area with series
    palette: Palette,
    /// The user's explicit `Data::palette` choice, if any. Preserved
    /// separately from `palette` (which carries the effective, defaulted
    /// strategy) so encoded series can distinguish "user set it" from "we
    /// defaulted it". See GOG.md D17.
    user_palette: Option<Palette>,
    color_slots: usize,
    /// Plot area offset within the scene, computed during layout.
    plot_area_offset: crate::core::Point,
    /// Plot area size, computed during layout.
    plot_area_size: crate::core::Size,
    /// Legend bounds (position + size) within the scene, computed during layout.
    /// Only set when a legend is configured.
    legend_bounds: Option<crate::core::Rectangle>,
    /// Which edge of the plot area the series-key legend rides, cached
    /// once from the legend config at scene construction. `None` when no
    /// legend is configured or the legend has no entries.
    legend_edge: Option<crate::data::legend::Edge>,
    /// Per-edge pixel budget reserved for `Placement::Inset` scale-legends
    /// across all marks. Computed from `Data` at scene construction and
    /// consumed by `layout` to shrink the plot area. Horizontal edges
    /// reserve height; vertical edges reserve width.
    scale_legend_budgets: std::collections::HashMap<crate::data::legend::Edge, f32>,
    /// Pairs of (series index in `plot_area.series`, reservation) for
    /// every mark with a `Placement::Inset` scale legend. Iteration order
    /// is the plot-area series order — stacking of multiple legends on
    /// one edge follows this order innermost-first.
    scale_legend_entries: Vec<(usize, crate::data::legend::Edge, f32)>,
    /// Per-mark strip rectangle for `Placement::Inset` scale-legends,
    /// keyed by the mark's index inside `plot_area.series`. Populated in
    /// `layout` and consumed by `draw` to position each mark's legend.
    scale_legend_strips: std::collections::HashMap<usize, crate::core::Rectangle>,
    /// Every mark carrying a scale legend (inset or overlaid), as (index in
    /// `plot_area.series`, mark, first palette slot it paints with).
    scale_legend_marks: Vec<(usize, &'a crate::Mark, usize)>,
    /// Scale legends planned in `layout` from `scale_legend_marks`, drawn
    /// after the marks.
    scale_legends: Vec<crate::chart::scale_legend::Legend>,
    /// Current selection (borrowed from Data).
    selection: &'a Option<crate::target::Target>,
    /// Optional tooltip configuration (borrowed from Data).
    tooltip: Option<&'a crate::data::tooltip::Tooltip>,
    /// Per-mark value-format closure: mark override → data scale; `None`
    /// at index `i` means walk to [`crate::scale::default_f64_format`].
    /// Indexed by primary-area mark index. The pie tooltip overlay
    /// reads this when [`Tooltip::format_is_default`] is true so the
    /// hover string stays consistent with the legend column.
    primary_mark_value_format: Vec<Option<crate::scale::Format<f64>>>,
    /// Per-axis transform and direction of the primary plane: x from
    /// `Data::x_scale` and the bottom axis's order, y from `Data::y_scale`
    /// and the left axis's order. The matching guides carry the same
    /// mappings, so ticks and marks agree.
    mappings: crate::chart::plot_area::Mappings,
    /// Per-axis mappings of the secondary plane: y from
    /// `Data::secondary_y_scale` and the right axis, x shared with the
    /// primary transform and the top axis's order. An absent secondary
    /// axis falls back to the primary mapping, as its bounds do.
    secondary_mappings: crate::chart::plot_area::Mappings,
    /// Chart-level geographic configuration sourced from
    /// [`crate::Data`]'s `geo_data` / `geo_scope` / `geo_projection` /
    /// `geo_basemap` fields. Threaded into `PlotArea::layout` for
    /// geo-aware marks (Choropleth, geo-Xy).
    geo_config: crate::chart::plot_area::GeoConfig,
    /// Whether a zoom window narrows any axis. Marks then clip to the data
    /// rectangle, since the window cuts through them.
    zoomed: bool,
}

impl<'a, Message, Renderer> Scene<'a, Message, Renderer>
where
    Renderer: text::Renderer + geometry::Renderer,
{
    /// Format the category independently of the axis label's visual truncation.
    pub(super) fn category_label(&self, x: f64) -> String {
        let horizontal = self.plot_area.series.iter().any(|series| {
            matches!(series, super::plot_area::Series::Bars(bars)
                if bars.data.direction == crate::mark::bar::Direction::Horizontal)
        });
        let guide = if horizontal { &self.left_axis } else { &self.bottom_axis };
        if let Some(guide) = guide {
            let labels = &guide.axis().labels;
            if let Some(label) = labels.values.as_ref().and_then(|values| values.get(x.round() as usize)) {
                return label.clone();
            }
            if let Some(format) = &labels.format {
                return format(x);
            }
        }
        crate::scale::default_f64_format(x)
    }

    /// Returns tree state for Scene's 7 children (like Content::state)
    pub(super) fn state(&self) -> Tree {
        let children = vec![
            self.title.as_ref().map_or(Tree::empty(), |title| title.state()),
            self.legend.as_ref().map_or(Tree::empty(), |legend| legend.state()),
            self.top_axis.as_ref().map_or(Tree::empty(), |guide| guide.state()),
            self.right_axis.as_ref().map_or(Tree::empty(), |guide| guide.state()),
            self.bottom_axis.as_ref().map_or(Tree::empty(), |guide| guide.state()),
            self.left_axis.as_ref().map_or(Tree::empty(), |guide| guide.state()),
            self.plot_area.state(),
        ];

        Tree {
            children,
            ..Tree::empty()
        }
    }

    /// Reconcile tree with current Scene state (like Content::diff)
    ///
    /// Key insight from iced's tree.rs: if tags don't match, recreate the tree.
    /// This handles None→Some transitions where the tree was Tree::empty().
    pub(super) fn diff(&mut self, tree: &mut Tree) {
        // Scene always has exactly 7 children (some may be empty Trees)
        // For each component, check if the tree matches what we expect.
        // If not (e.g., was None/empty, now Some), recreate the child tree.

        // Title at index 0
        match &self.title {
            Some(title) => {
                let expected_tag = tree::Tag::of::<title::State<Renderer::Paragraph>>();
                if tree.children[0].tag == expected_tag {
                    title.diff(&mut tree.children[0]);
                } else {
                    tree.children[0] = title.state();
                }
            }
            None => {
                tree.children[0] = Tree::empty();
            }
        }

        // Legend at index 1
        match &self.legend {
            Some(legend) => {
                let expected_tag = tree::Tag::of::<legend::State<Renderer::Paragraph>>();
                if tree.children[1].tag == expected_tag {
                    legend.diff(&mut tree.children[1]);
                } else {
                    tree.children[1] = legend.state();
                }
            }
            None => {
                tree.children[1] = Tree::empty();
            }
        }

        // Top axis at index 2
        match &self.top_axis {
            Some(guide) => {
                let expected_tag = tree::Tag::of::<guide::State<Renderer::Paragraph>>();
                if tree.children[2].tag == expected_tag {
                    guide.diff(&mut tree.children[2]);
                } else {
                    tree.children[2] = guide.state();
                }
            }
            None => {
                tree.children[2] = Tree::empty();
            }
        }

        // Right axis at index 3
        match &self.right_axis {
            Some(guide) => {
                let expected_tag = tree::Tag::of::<guide::State<Renderer::Paragraph>>();
                if tree.children[3].tag == expected_tag {
                    guide.diff(&mut tree.children[3]);
                } else {
                    tree.children[3] = guide.state();
                }
            }
            None => {
                tree.children[3] = Tree::empty();
            }
        }

        // Bottom axis at index 4
        match &self.bottom_axis {
            Some(guide) => {
                let expected_tag = tree::Tag::of::<guide::State<Renderer::Paragraph>>();
                if tree.children[4].tag == expected_tag {
                    guide.diff(&mut tree.children[4]);
                } else {
                    tree.children[4] = guide.state();
                }
            }
            None => {
                tree.children[4] = Tree::empty();
            }
        }

        // Left axis at index 5
        match &self.left_axis {
            Some(guide) => {
                let expected_tag = tree::Tag::of::<guide::State<Renderer::Paragraph>>();
                if tree.children[5].tag == expected_tag {
                    guide.diff(&mut tree.children[5]);
                } else {
                    tree.children[5] = guide.state();
                }
            }
            None => {
                tree.children[5] = Tree::empty();
            }
        }

        // Plot area at index 6 (always present)
        let expected_tag = tree::Tag::of::<crate::chart::plot_area::State>();
        if tree.children[6].tag == expected_tag {
            self.plot_area.diff(&mut tree.children[6]);
        } else {
            tree.children[6] = self.plot_area.state();
        }
    }

    /// Create a new scene from Data, borrowing everything for lifetime 'a.
    pub fn new(data: &'a crate::Data) -> Self {
        // Extract legend entries from primary and secondary-area marks,
        // paired with each entry's resolved value-format closure (mark
        // override falls through to the chart-level data scale). The
        // legend layout walks the legend-config override on top of
        // these per-entry slots.
        let (entries, entry_value_format) = legend_entries_for(data);

        let legend = match data.legend.as_ref() {
            Some(config) if !entries.is_empty() => Some(Legend::new(
                entries,
                config.anchor,
                config.orientation,
                config.text,
                config.wrap,
                config.interactive,
                config.value_format_ref().cloned(),
                entry_value_format,
            )),
            _ => None,
        };
        let legend_edge = match data.legend.as_ref() {
            Some(config) if legend.is_some() => Some(config.edge()),
            _ => None,
        };

        // Cycle the palette across *all* marks in one shared sequence —
        // primary first, then secondary — so series on different axes
        // don't both resolve to slot 0 and collide. This matches the
        // convention in ggplot / plotly / matplotlib / vega / d3: one
        // global data-color cycle, indexed in draw order,
        // regardless of which axis a series belongs to.
        let primary_marks = data.primary.marks();
        let secondary_marks = data.secondary.marks();
        let color_slots = palette::count_color_slots(primary_marks) + palette::count_color_slots(secondary_marks);
        let palette_strategy = data.palette.clone().unwrap_or_else(|| {
            let total_marks = primary_marks.len() + secondary_marks.len();
            if total_marks >= 2 {
                // Two or more marks → tonal, same rule the
                // multi-mark branch of `Palette::default_for` applies.
                Palette::TONAL
            } else if primary_marks.is_empty() {
                Palette::default_for(secondary_marks)
            } else {
                Palette::default_for(primary_marks)
            }
        });

        // Marks with a scale legend, in `plot_area.series` order (primary
        // then secondary) with the palette slot each starts at, which is
        // how the plot area hands out color offsets.
        let scale_legend_marks: Vec<(usize, &'a crate::Mark, usize)> = primary_marks
            .iter()
            .chain(secondary_marks)
            .scan(0, |offset, mark| {
                let start = *offset;
                *offset += palette::color_slots(mark);
                Some((mark, start))
            })
            .enumerate()
            .filter(|(_, (mark, _))| mark.scale_legend_config().is_some())
            .map(|(i, (mark, offset))| (i, mark, offset))
            .collect();

        let y_transform = data.y_scale().transform;
        let y_nice = data.y_scale().nice;
        let secondary_y_transform = data.secondary_y_scale_ref().transform;
        let secondary_y_nice = data.secondary_y_scale_ref().nice;
        // X-axis transform and nicing flow from whichever `Scale` the x-axis
        // is tagged with. Only numeric x scales carry a transform; category
        // and time axes map linearly and keep their own tick alignment.
        let x_transform = data.x_scale_ref().transform();
        let x_nice = data.x_scale_ref().nice();

        let mapping = |transform, axis: Option<&crate::Axis>| {
            crate::scale::Mapping::new(transform, axis.map(crate::Axis::order).unwrap_or_default())
        };
        let primary_x = mapping(x_transform, data.primary.x_axis.as_ref());
        let primary_y = mapping(y_transform, data.primary.y_axis.as_ref());
        let mappings = crate::chart::plot_area::Mappings {
            x: primary_x,
            y: primary_y,
        };
        let secondary_mappings = crate::chart::plot_area::Mappings {
            x: data
                .secondary
                .x_axis
                .as_ref()
                .map_or(primary_x, |axis| mapping(x_transform, Some(axis))),
            y: data
                .secondary
                .y_axis
                .as_ref()
                .map_or(primary_y, |axis| mapping(secondary_y_transform, Some(axis))),
        };
        Self {
            title: data.title.as_deref().map(|t| Title::new(t, data.title_text)),
            legend,
            top_axis: data.secondary.x_axis.as_ref().map(|axis| {
                Guide::new(axis, data.secondary.marks())
                    .with_transform(x_transform)
                    .with_nice(x_nice)
            }),
            right_axis: data.secondary.y_axis.as_ref().map(|axis| {
                Guide::new(axis, data.secondary.marks())
                    .with_transform(secondary_y_transform)
                    .with_nice(secondary_y_nice)
            }),
            bottom_axis: data.primary.x_axis.as_ref().map(|axis| {
                Guide::new(axis, data.primary.marks())
                    .with_transform(x_transform)
                    .with_nice(x_nice)
            }),
            left_axis: data.primary.y_axis.as_ref().map(|axis| {
                Guide::new(axis, data.primary.marks())
                    .with_transform(y_transform)
                    .with_nice(y_nice)
            }),
            plot_area: {
                // Per-series value-format vector mirrors the order
                // PlotArea::new + with_secondary appends series, so a
                // single `with_value_formats` call covers both axis
                // sides.
                let primary_chain: Vec<_> = primary_marks
                    .iter()
                    .map(|m| mark_value_format(m).or_else(|| data.value_scale().format.clone()))
                    .collect();
                let secondary_chain: Vec<_> = secondary_marks
                    .iter()
                    .map(|m| mark_value_format(m).or_else(|| data.value_scale().format.clone()))
                    .collect();
                let mut chain = primary_chain;
                chain.extend(secondary_chain);

                let area = if data.secondary.is_empty() {
                    PlotArea::new(primary_marks)
                } else {
                    PlotArea::new(primary_marks).with_secondary(secondary_marks)
                };
                area.with_value_formats(chain).with_animate(data.animate)
            },
            palette: palette_strategy,
            user_palette: data.palette.clone(),
            color_slots,
            plot_area_offset: crate::core::Point::ORIGIN,
            plot_area_size: crate::core::Size::ZERO,
            legend_bounds: None,
            legend_edge,
            scale_legend_budgets: data.scale_legend_reservations(),
            scale_legend_entries: scale_legend_marks
                .iter()
                .filter_map(|&(i, mark, _)| {
                    let scale = mark.scale_legend_config()?;
                    let (edge, budget) = crate::chart::scale_legend::reservation(&scale)?;
                    Some((i, edge, budget))
                })
                .collect(),
            scale_legend_strips: std::collections::HashMap::new(),
            scale_legend_marks,
            scale_legends: Vec::new(),
            selection: &data.selection,
            tooltip: data.tooltip.as_ref(),
            primary_mark_value_format: data
                .primary
                .marks()
                .iter()
                .map(|mark| mark_value_format(mark).or_else(|| data.value_scale().format.clone()))
                .collect(),
            mappings,
            secondary_mappings,
            geo_config: crate::chart::plot_area::GeoConfig {
                geo: data.geo_data.clone(),
                scope: data.geo_scope,
                projection: data.geo_projection,
                basemap: data.geo_basemap,
            },
            zoomed: false,
        }
    }

    /// Shows the zoom windows on the axes: x on the bottom and top guides,
    /// y on the left and right ones. Call before [`Self::layout`].
    pub(crate) fn zoom(&mut self, zoom: &crate::chart::zoom::State) {
        for guide in [&mut self.bottom_axis, &mut self.top_axis].into_iter().flatten() {
            guide.zoom(zoom.x);
        }
        for guide in [&mut self.left_axis, &mut self.right_axis].into_iter().flatten() {
            guide.zoom(zoom.y);
        }
        self.zoomed = zoom.is_zoomed();
    }

    /// The primary axes that can zoom, with their full domains as of the
    /// last layout: `(bottom, left)`.
    pub(crate) fn zoom_axes(
        &self,
        tree: &Tree,
    ) -> (Option<crate::chart::zoom::Axis>, Option<crate::chart::zoom::Axis>) {
        let axis = |guide: &Option<Guide<'a, Message, Renderer>>, index: usize| {
            let guide = guide.as_ref().filter(|guide| guide.zoomable())?;
            let state = tree
                .children
                .get(index)
                .filter(|child| child.tag == tree::Tag::of::<guide::State<Renderer::Paragraph>>())?
                .state
                .downcast_ref::<guide::State<Renderer::Paragraph>>();
            Some(crate::chart::zoom::Axis {
                home: [state.home.min(), state.home.max()],
                mapping: guide.mapping(),
            })
        };
        (axis(&self.bottom_axis, 4), axis(&self.left_axis, 5))
    }

    /// Returns the resolved value-format closure for the given primary
    /// mark index — `mark.value_scale.format` if set, else
    /// `Data::value_scale.format`. Returns `None` when neither is set
    /// (callers fall back to [`crate::scale::default_f64_format`]).
    pub(crate) fn primary_mark_value_format(&self, mark_idx: usize) -> Option<&crate::scale::Format<f64>> {
        self.primary_mark_value_format.get(mark_idx).and_then(|f| f.as_ref())
    }

    /// Returns the plot area offset within the scene (stored during layout).
    pub(crate) fn plot_area_offset(&self) -> crate::core::Point {
        self.plot_area_offset
    }

    /// Returns the plot area size (stored during layout).
    #[allow(dead_code)]
    pub(crate) fn plot_area_size(&self) -> crate::core::Size {
        self.plot_area_size
    }

    /// Returns the legend bounds within the scene, if a legend is configured.
    pub(crate) fn legend_bounds(&self) -> Option<crate::core::Rectangle> {
        self.legend_bounds
    }

    /// Returns whether a tooltip is configured.
    pub(crate) fn has_tooltip(&self) -> bool {
        self.tooltip.is_some()
    }

    /// Returns the tooltip configuration, if any.
    pub(crate) fn tooltip(&self) -> Option<&'a crate::data::tooltip::Tooltip> {
        self.tooltip
    }

    /// Returns a reference to the plot area.
    pub(crate) fn plot_area(&self) -> &PlotArea<'a, Message, Renderer> {
        &self.plot_area
    }

    /// Returns a mutable reference to the plot area.
    ///
    /// Used by the chart widget to mutate variant-specific renderer
    /// state (e.g. the donut hole on the first `Series::Pie`) before
    /// `Scene::layout` runs.
    pub(crate) fn plot_area_mut(&mut self) -> &mut PlotArea<'a, Message, Renderer> {
        &mut self.plot_area
    }

    /// Returns a reference to the legend, if one is configured.
    pub(crate) fn legend(&self) -> Option<&Legend<'a, Message, Renderer>> {
        self.legend.as_ref()
    }

    /// Resolves the palette using the given design.
    pub(crate) fn resolve_palette(&self, design: &dyn crate::design::Design) -> palette::Resolved {
        let seed = design.seed();
        palette::Resolved::resolve(&self.palette, &seed, self.color_slots)
    }

    /// Returns the user's explicit `Data::palette` choice (if any), used by
    /// encoded series to inherit the chart's flavor. See GOG.md D17.
    pub(crate) fn user_palette(&self) -> Option<&Palette> {
        self.user_palette.as_ref()
    }

    /// Layout the scene with proper tree delegation to 7 pieces.
    /// This is where the actual recursive layout happens.
    pub(crate) fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &crate::core::layout::Limits,
        design: Option<&dyn crate::design::Design>,
    ) {
        use crate::core::{Point, Size, Vector};
        use crate::data::legend::Edge;

        let available = limits.bounds();
        let lim = |w, h| crate::core::layout::Limits::new(Size::ZERO, Size::new(w, h));

        // Tree indices: [0: title, 1: legend, 2: top, 3: right, 4: bottom, 5: left, 6: plot]

        let legend_edge = self.legend_edge;
        let legend_is_side = matches!(legend_edge, Some(Edge::Left | Edge::Right));
        let legend_is_above = matches!(legend_edge, Some(Edge::Top));

        // Inset scale-legend budgets: aggregated from the marks at scene
        // construction. Horizontal edges reserve vertical space; vertical
        // edges reserve horizontal space. Stacked outside the axes but
        // inside the series-key legend (see edge-order docs on Scene).
        let scale_top = self.scale_legend_budgets.get(&Edge::Top).copied().unwrap_or(0.0);
        let scale_bottom = self.scale_legend_budgets.get(&Edge::Bottom).copied().unwrap_or(0.0);
        let scale_left = self.scale_legend_budgets.get(&Edge::Left).copied().unwrap_or(0.0);
        let scale_right = self.scale_legend_budgets.get(&Edge::Right).copied().unwrap_or(0.0);

        // --- Phase 1: Title (always at top, full width) ---
        let (title_height, title_laid_out) = if let Some(title) = &self.title {
            if let Some(title_tree) = tree.children.get_mut(0) {
                title.layout(title_tree, renderer, &lim(available.width, available.height), design);
                (title_tree.size.height, true)
            } else {
                (0.0, false)
            }
        } else {
            (0.0, false)
        };

        // --- Phase 2: Side legend measurement (Left/Right) ---
        let (legend_side_width, legend_side_size) = if legend_is_side {
            if let (Some(legend), Some(legend_tree)) = (&self.legend, tree.children.get_mut(1)) {
                let remaining_h = available.height - title_height;
                legend.layout(legend_tree, renderer, &lim(available.width, remaining_h), design);
                (legend_tree.size.width, Some(legend_tree.size))
            } else {
                (0.0, None)
            }
        } else {
            (0.0, None)
        };

        // --- Phase 3a: Measure secondary axes (top and right) ---
        let (top_height, right_width) = {
            let remaining_h = available.height - title_height;
            let top_tree = &mut tree.children[2];
            let top_height = if let Some(guide) = &self.top_axis {
                guide.layout(
                    top_tree,
                    renderer,
                    &lim(available.width, remaining_h),
                    (0.0, 0.0),
                    (0.0, 0.0),
                    design,
                );
                top_tree.size.height
            } else {
                0.0
            };
            let right_tree = &mut tree.children[3];
            let vertical = remaining_h - top_height;
            let right_width = if let Some(guide) = &self.right_axis {
                guide.layout(
                    right_tree,
                    renderer,
                    &lim(available.width, vertical),
                    (0.0, 0.0),
                    (0.0, 0.0),
                    design,
                );
                right_tree.size.width
            } else {
                0.0
            };
            (top_height, right_width)
        };

        // --- Phase 3: Measure axes ---
        let (bottom_height, left_width) = {
            let (first_children, second_children) = tree.children.split_at_mut(5);
            let bottom_tree = &mut first_children[4];
            let left_tree = &mut second_children[0];

            let remaining_h = available.height - title_height;
            // Phase 3 is a measurement pass; sibling axis sizes aren't known
            // yet, so we pass (0, 0) overflow and (0, 0) min_inset. Insets
            // don't affect the returned width/height, so the measurement
            // remains correct. Data bounds are written to state.
            let bottom_height = if let Some(guide) = &self.bottom_axis {
                guide.layout(
                    bottom_tree,
                    renderer,
                    &lim(available.width, remaining_h),
                    (0.0, 0.0),
                    (0.0, 0.0),
                    design,
                );
                bottom_tree.size.height
            } else {
                0.0
            };

            let vertical = remaining_h - bottom_height;
            let left_width = if let Some(guide) = &self.left_axis {
                guide.layout(
                    left_tree,
                    renderer,
                    &lim(available.width, vertical),
                    (0.0, 0.0),
                    (0.0, 0.0),
                    design,
                );
                left_tree.size.width
            } else {
                0.0
            };

            (bottom_height, left_width)
        };

        // --- Phase 4: Compute plot_width (horizontal space) ---
        let plot_width = available.width - left_width - right_width - legend_side_width - scale_left - scale_right;

        // --- Phase 5: Above/Below legend measurement (needs plot_width for wrapping) ---
        let (legend_tb_height, legend_tb_size) = if !legend_is_side {
            if let (Some(legend), Some(legend_tree)) = (&self.legend, tree.children.get_mut(1)) {
                legend.layout(legend_tree, renderer, &lim(plot_width, available.height), design);
                (legend_tree.size.height, Some(legend_tree.size))
            } else {
                (0.0, None)
            }
        } else {
            (0.0, None)
        };

        let above_legend_h = if legend_is_above { legend_tb_height } else { 0.0 };
        let below_legend_h = if matches!(legend_edge, Some(Edge::Bottom)) {
            legend_tb_height
        } else {
            0.0
        };

        // --- Phase 6: Compute plot_height (vertical space) ---
        let remaining_height = available.height - title_height - above_legend_h - top_height - scale_top;
        let vertical_space = remaining_height - bottom_height - below_legend_h - scale_bottom;
        let plot_height = vertical_space;

        // --- Phase 6.5: Series-driven inset floor ---
        // Ask the plot area how much inset its series need past the data
        // mapping region (e.g., for data labels that extend past bar ends).
        // Labels are shaped here, in the measurement pass, so the inset
        // measure reads real paragraph metrics and the final `plot_area.layout`
        // reuses the same shaped paragraphs. This inset becomes a floor for the
        // axis insets so ticks & bars stay aligned.
        let series_insets = {
            use crate::chart::guide;
            let x_bounds = self.bottom_axis.as_ref().map(|_| {
                let st = tree.children[4]
                    .state
                    .downcast_ref::<guide::State<Renderer::Paragraph>>();
                (st.bounds.min(), st.bounds.max())
            });
            let y_bounds = self.left_axis.as_ref().map(|_| {
                let st = tree.children[5]
                    .state
                    .downcast_ref::<guide::State<Renderer::Paragraph>>();
                (st.bounds.min(), st.bounds.max())
            });
            let xb = x_bounds.unwrap_or((0.0, 1.0));
            let yb = y_bounds.unwrap_or((0.0, 1.0));
            let plot_tree = &mut tree.children[6];
            self.plot_area.shape_labels(plot_tree, renderer, design);
            self.plot_area
                .min_insets(plot_tree, crate::core::Size::new(plot_width, plot_height), xb, yb)
        };

        // --- Phase 7: Final layout pass ---
        let (first_children, second_children) = tree.children.split_at_mut(5);

        // Overflow budgets: sibling axis columns absorb edge-label overhang.
        // Vertical axis: (top_overflow, bottom_overflow) = (top axis, bottom axis).
        // Horizontal axis: (left_overflow, right_overflow) = (left axis, right axis).
        let left_overflow = (top_height, bottom_height);
        let bottom_overflow = (left_width, right_width);

        // Min-inset floors from series (data labels).
        // Vertical axis: (top, bottom) of the content rect. Horizontal axis: (left, right).
        let left_min_inset = (series_insets.top, series_insets.bottom);
        let bottom_min_inset = (series_insets.left, series_insets.right);

        if let Some(guide) = &self.left_axis {
            guide.layout(
                &mut second_children[0],
                renderer,
                &lim(available.width, plot_height),
                left_overflow,
                left_min_inset,
                design,
            );
        }

        if let Some(guide) = &self.bottom_axis {
            guide.layout(
                &mut first_children[4],
                renderer,
                &lim(plot_width, remaining_height),
                bottom_overflow,
                bottom_min_inset,
                design,
            );
        }

        // Secondary axes — share X/Y overflow with their siblings.
        if let Some(guide) = &self.top_axis {
            guide.layout(
                &mut first_children[2],
                renderer,
                &lim(plot_width, remaining_height),
                bottom_overflow,
                bottom_min_inset,
                design,
            );
        }

        if let Some(guide) = &self.right_axis {
            guide.layout(
                &mut first_children[3],
                renderer,
                &lim(available.width, plot_height),
                left_overflow,
                left_min_inset,
                design,
            );
        }

        // Axis bounds for plot area coordinate sync
        let (axis_bounds, secondary_bounds, plot_insets) = {
            use crate::chart::guide;
            use crate::chart::plot_area::Insets;

            let (x_bounds, h_insets) = if self.bottom_axis.is_some() {
                let st = first_children[4]
                    .state
                    .downcast_ref::<guide::State<Renderer::Paragraph>>();
                (Some(st.bounds), st.label_insets)
            } else {
                (None, (0.0, 0.0))
            };
            let (y_bounds, v_insets) = if self.left_axis.is_some() {
                let st = second_children[0]
                    .state
                    .downcast_ref::<guide::State<Renderer::Paragraph>>();
                (Some(st.bounds), st.label_insets)
            } else {
                (None, (0.0, 0.0))
            };
            let bounds = match (x_bounds, y_bounds) {
                (Some(x), Some(y)) => Some((x.min(), x.max(), y.min(), y.max())),
                _ => None,
            };

            // Secondary axes: share primary X when top_axis is absent.
            let sec_x = if self.top_axis.is_some() {
                let st = first_children[2]
                    .state
                    .downcast_ref::<guide::State<Renderer::Paragraph>>();
                Some(st.bounds)
            } else {
                x_bounds
            };
            let sec_y = if self.right_axis.is_some() {
                let st = first_children[3]
                    .state
                    .downcast_ref::<guide::State<Renderer::Paragraph>>();
                Some(st.bounds)
            } else {
                None
            };
            let secondary_bounds = match (sec_x, sec_y) {
                (Some(x), Some(y)) => Some((x.min(), x.max(), y.min(), y.max())),
                _ => None,
            };

            // The data-rect inset is the axis guides' final `label_insets`,
            // which already fold the series `min_inset` floor in via the
            // guide's own max — so ticks and the content rect consume one
            // shared per-edge value. Horizontal edges come from the bottom
            // axis, vertical from the left.
            let insets = Insets {
                left: h_insets.0,
                right: h_insets.1,
                top: v_insets.0,
                bottom: v_insets.1,
            };
            (bounds, secondary_bounds, insets)
        };

        // Label-avoidance obstacle bands: the sibling axis gutters expressed
        // as plot-local rectangles (left axis at negative x, bottom axis below
        // the plot, etc.), sourced from the laid-out axis sizes. Marks
        // that place data labels via pathfinding (area, line) keep their labels
        // out of these regions.
        let mut axis_obstacles = Vec::new();
        if left_width > 0.0 {
            axis_obstacles.push(crate::core::Rectangle {
                x: -left_width,
                y: 0.0,
                width: left_width,
                height: plot_height,
            });
        }
        if right_width > 0.0 {
            axis_obstacles.push(crate::core::Rectangle {
                x: plot_width,
                y: 0.0,
                width: right_width,
                height: plot_height,
            });
        }
        if top_height > 0.0 {
            axis_obstacles.push(crate::core::Rectangle {
                x: 0.0,
                y: -top_height,
                width: plot_width,
                height: top_height,
            });
        }
        if bottom_height > 0.0 {
            axis_obstacles.push(crate::core::Rectangle {
                x: 0.0,
                y: plot_height,
                width: plot_width,
                height: bottom_height,
            });
        }

        self.plot_area.layout(
            &mut second_children[1],
            renderer,
            &lim(plot_width, plot_height),
            axis_bounds,
            secondary_bounds,
            plot_insets,
            &axis_obstacles,
            self.mappings,
            self.secondary_mappings,
            &self.geo_config,
            design,
        );

        // --- Phase 8: Position all children ---

        // Compute vertical offsets. Layer order from the chart edge inward:
        //   series-key legend → scale-legend strip → axis → plot area.
        let legend_left_x = if legend_edge == Some(Edge::Left) {
            legend_side_width
        } else {
            0.0
        };
        let scale_left_x = legend_left_x;
        let axis_left_x = scale_left_x + scale_left;
        let content_left = axis_left_x + left_width;
        let plot_top = title_height + above_legend_h + top_height + scale_top;

        // Title at (0, 0)
        if title_laid_out {
            tree.children[0].translation = Vector::ZERO;
        }

        // Legend — position depends on config. Layer order from edge
        // inward is [series-key legend, scale-legend strip, axis, plot],
        // so the series-key legend clears the full outer margin (axis +
        // scale strip) on its edge.
        if let Some(size) = legend_tb_size.or(legend_side_size) {
            // Side legends center vertically against the plot area so a
            // short legend doesn't pin to the top of a tall chart.
            let side_y = plot_top + ((plot_height - size.height) / 2.0).max(0.0);
            let pos = match legend_edge.unwrap_or(Edge::Top) {
                Edge::Top => Point::new(content_left, title_height),
                Edge::Bottom => Point::new(content_left, plot_top + plot_height + bottom_height + scale_bottom),
                Edge::Left => Point::new(0.0, side_y),
                Edge::Right => Point::new(content_left + plot_width + right_width + scale_right, side_y),
            };
            self.legend_bounds = Some(crate::core::Rectangle {
                x: pos.x,
                y: pos.y,
                width: size.width,
                height: size.height,
            });
            tree.children[1].translation = Vector::new(pos.x, pos.y);
        } else {
            self.legend_bounds = None;
        }

        // Top axis — above plot area
        if self.top_axis.is_some() {
            tree.children[2].translation = Vector::new(content_left, plot_top - top_height);
        }

        // Left axis. The axis border line itself is drawn inside the plot
        // area frame by `plot_area::draw_axis_borders`, so the left axis
        // frame contains only the labels and tick marks — no +1 width
        // hack needed to bridge the gap to the plot area.
        if self.left_axis.is_some() {
            tree.children[5].translation = Vector::new(axis_left_x, plot_top);
        }

        // Plot area
        self.plot_area_offset = Point::new(content_left, plot_top);
        self.plot_area_size = Size::new(plot_width, plot_height);
        tree.children[6].translation = Vector::new(content_left, plot_top);

        // Bottom axis
        if self.bottom_axis.is_some() {
            tree.children[4].translation = Vector::new(content_left, plot_top + plot_height);
        }

        // Right axis — to the right of plot area
        if self.right_axis.is_some() {
            tree.children[3].translation = Vector::new(content_left + plot_width, plot_top);
        }

        // Scale-legend strip rects. Stack marks on the same edge along
        // the edge's normal: on Bottom they pile downward innermost-first
        // (first reservation sits flush with the axis), on Top they pile
        // upward, and so on. Widths/heights along the edge span the plot
        // area so the gradient bar centers over the plot.
        self.scale_legend_strips.clear();
        let mut top_cursor = 0.0;
        let mut bottom_cursor = 0.0;
        let mut left_cursor = 0.0;
        let mut right_cursor = 0.0;
        for &(series_idx, edge, budget) in &self.scale_legend_entries {
            let rect = match edge {
                Edge::Top => {
                    // First entry flush with the axis (innermost), then stack upward.
                    let y = plot_top - top_height - top_cursor - budget;
                    top_cursor += budget;
                    crate::core::Rectangle {
                        x: content_left,
                        y,
                        width: plot_width,
                        height: budget,
                    }
                }
                Edge::Bottom => {
                    let y = plot_top + plot_height + bottom_height + bottom_cursor;
                    bottom_cursor += budget;
                    crate::core::Rectangle {
                        x: content_left,
                        y,
                        width: plot_width,
                        height: budget,
                    }
                }
                Edge::Left => {
                    // First entry flush with the axis, then stack outward.
                    let x = axis_left_x - left_cursor - budget;
                    left_cursor += budget;
                    crate::core::Rectangle {
                        x,
                        y: plot_top,
                        width: budget,
                        height: plot_height,
                    }
                }
                Edge::Right => {
                    let x = content_left + plot_width + right_width + right_cursor;
                    right_cursor += budget;
                    crate::core::Rectangle {
                        x,
                        y: plot_top,
                        width: budget,
                        height: plot_height,
                    }
                }
            };
            self.scale_legend_strips.insert(series_idx, rect);
        }

        // Scale-legend plans: tick labels, color steps and circle sizes are
        // theme-free, so they're computed here, fitted to the strips and
        // plot just laid out, and `draw` only resolves colors.
        self.scale_legends = self
            .scale_legend_marks
            .iter()
            .filter_map(|&(i, mark, offset)| {
                let mut legend = crate::chart::scale_legend::Legend::of(i, mark, offset)?;
                legend.fit(self.plot_area_size, self.scale_legend_strips.get(&i).map(|r| r.size()));
                Some(legend)
            })
            .collect();

        tree.size = available;
    }

    pub(crate) fn label_at(
        &self,
        tree: &Tree,
        layout: crate::core::Layout,
        position: crate::core::Point,
        hidden: &std::collections::HashSet<String>,
    ) -> Option<crate::Target> {
        if !layout.bounds().contains(position) {
            return None;
        }
        let space = self.label_space(tree, layout);
        let (plot, plot_tree) = layout.iter(&tree.children).nth(6)?;
        let shift = plot.bounds().position() - crate::core::Point::ORIGIN;
        let hit = |rect: &Option<crate::core::Rectangle>| {
            rect.is_some_and(|rect| space.place(rect + shift).contains(position))
        };
        for (mark, (series, child)) in self.plot_area.series.iter().zip(&plot_tree.children).enumerate().rev() {
            match series {
                crate::chart::plot_area::Series::Bars(bars) => {
                    let state = child
                        .state
                        .downcast_ref::<crate::chart::plot_area::bars::State<Renderer::Paragraph>>();
                    for (series, rects) in state.label_rects.iter().enumerate() {
                        if bars
                            .data
                            .series(series)
                            .and_then(|s| s.name())
                            .is_some_and(|name| hidden.contains(name))
                        {
                            continue;
                        }
                        if let Some(index) = rects.iter().position(hit) {
                            return Some(crate::Target::EntryLabel { mark, series, index });
                        }
                    }
                }
                crate::chart::plot_area::Series::Pie(_) => {
                    let state = child
                        .state
                        .downcast_ref::<crate::chart::plot_area::pie::State<Renderer::Paragraph>>();
                    if let Some(index) = state.label_rects.iter().position(hit) {
                        return Some(crate::Target::EntryLabel { mark, series: 0, index });
                    }
                }
                _ => {}
            }
        }
        None
    }

    pub(crate) fn label_space(&self, tree: &Tree, layout: crate::core::Layout) -> crate::chart::labels::Space {
        let children_layouts: Vec<_> = layout.iter(&tree.children).map(|(l, _)| l).collect();
        let mut label_space = crate::chart::labels::Space::new(layout.bounds());
        if self.title.is_some() {
            let bounds = children_layouts[0].bounds();
            let title = tree.children[0]
                .state
                .downcast_ref::<title::State<Renderer::Paragraph>>();
            let size = title.paragraph.min_bounds();
            label_space.reserve(crate::core::Rectangle {
                x: bounds.center_x() - size.width / 2.,
                y: bounds.y + 10.,
                width: size.width,
                height: size.height,
            });
        }
        for (index, active) in [
            (1, self.legend.is_some()),
            (2, self.top_axis.is_some()),
            (3, self.right_axis.is_some()),
            (4, self.bottom_axis.is_some()),
            (5, self.left_axis.is_some()),
        ] {
            if active {
                label_space.reserve(children_layouts[index].bounds());
            }
        }
        label_space
    }

    /// Draws the scene by delegating to its 7 pieces.
    /// Follows the same order as layout() - navigates layout tree and delegates.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn draw<D>(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        design: &D,
        style: &crate::core::renderer::Style,
        layout: crate::core::Layout,
        cursor: crate::core::mouse::Cursor,
        viewport: &crate::core::Rectangle,
        hidden_series: &std::collections::HashSet<String>,
        corners: crate::core::border::Radius,
        track: Option<crate::core::Color>,
    ) where
        D: design::Design + ?Sized,
    {
        // Resolve palette for this draw call
        let seed = design.seed();
        let resolved = palette::Resolved::resolve(&self.palette, &seed, self.color_slots);

        // Child layouts follow the tree indices:
        // 0: title, 1: legend, 2: top_axis, 3: right_axis
        // 4: bottom_axis, 5: left_axis, 6: plot_area
        let children_layouts: Vec<crate::core::Layout> =
            layout.iter(&tree.children).map(|(layout, _)| layout).collect();

        // Title
        if let Some(title) = &self.title {
            let title_layout = children_layouts[0];
            title.draw(
                &tree.children[0],
                renderer,
                design,
                style,
                title_layout,
                cursor,
                viewport,
            );
        }

        // Legend
        if let Some(legend) = &self.legend {
            let legend_layout = children_layouts[1];
            legend.draw(
                &tree.children[1],
                renderer,
                design,
                style,
                legend_layout,
                cursor,
                viewport,
                &resolved,
                hidden_series,
            );
        }

        // Axes that cross the plot at a data value (`Axis::crosses_at`)
        // draw their ticks and labels shifted from their gutter onto the
        // crossing, the same offset the plot area applies to their spine.
        let plot_tree = &tree.children[6];
        let draw_guide = |guide: &Guide<'a, Message, Renderer>, index: usize, renderer: &mut Renderer| {
            let offset = self.plot_area.crossing_offset(plot_tree, guide.axis());
            renderer.with_translation(offset, |renderer| {
                guide.draw(
                    &tree.children[index],
                    renderer,
                    design,
                    style,
                    children_layouts[index],
                    cursor,
                    viewport,
                );
            });
        };

        // Top axis (secondary X)
        if let Some(guide) = &self.top_axis {
            draw_guide(guide, 2, renderer);
        }

        // Left axis
        if let Some(guide) = &self.left_axis {
            draw_guide(guide, 5, renderer);
        }

        // Plot area
        let plot_layout = children_layouts[6];

        // Draw gridlines before marks so marks render on top.
        // Tick positions come from guide state (data coordinates); the plot
        // area converts them to pixels via its own plane.
        {
            use crate::chart::guide;
            let x_ticks: Vec<f64> = self
                .bottom_axis
                .as_ref()
                .map(|_| {
                    tree.children[4]
                        .state
                        .downcast_ref::<guide::State<Renderer::Paragraph>>()
                        .tick_positions
                        .clone()
                })
                .unwrap_or_default();
            let y_ticks: Vec<f64> = self
                .left_axis
                .as_ref()
                .map(|_| {
                    tree.children[5]
                        .state
                        .downcast_ref::<guide::State<Renderer::Paragraph>>()
                        .tick_positions
                        .clone()
                })
                .unwrap_or_default();

            self.plot_area.draw_gridlines(
                &tree.children[6],
                renderer,
                design,
                plot_layout,
                self.bottom_axis.as_ref().map(|g| g.axis()),
                self.left_axis.as_ref().map(|g| g.axis()),
                &x_ticks,
                &y_ticks,
            );
        }

        // Whether the baseline axis spines are drawn. Bars round their
        // baseline end only when the spine they'd sit on is hidden; the plot
        // area selects the left or bottom flag per bar direction.
        let bottom_axis_line = self
            .bottom_axis
            .as_ref()
            .map(|g| g.axis().shows_line())
            .unwrap_or(false);
        let left_axis_line = self.left_axis.as_ref().map(|g| g.axis().shows_line()).unwrap_or(false);

        let mut labels = crate::chart::labels::Layer::new(self.label_space(tree, layout));
        let draw_marks = |renderer: &mut Renderer| {
            self.plot_area.draw_with_labels(
                &tree.children[6],
                renderer,
                design,
                style,
                plot_layout,
                cursor,
                viewport,
                &resolved,
                self.user_palette.as_ref(),
                self.selection,
                hidden_series,
                corners,
                track,
                bottom_axis_line,
                left_axis_line,
                &mut labels,
            );
            labels.draw(renderer, viewport);
            self.plot_area
                .draw_annotations(&tree.children[6], renderer, design, plot_layout, viewport);
        };
        if self.zoomed {
            let content = tree.children[6]
                .state
                .downcast_ref::<crate::chart::plot_area::State>()
                .content_rect;
            let clip = content + (plot_layout.bounds().position() - crate::core::Point::ORIGIN);
            if let Some(clip) = clip.intersection(viewport) {
                renderer.with_layer(clip, draw_marks);
            }
        } else {
            draw_marks(renderer);
        }

        // Scale legends. Overlaid panels float inside the plot area;
        // inset strips live outside the plot area in rectangles the scene
        // reserved during layout. Drawing here (after the plot area, before
        // axis borders) puts legends on top of the marks but below axes
        // framing.
        //
        // Strip rects were computed in scene-local coordinates; promote them
        // to absolute coordinates here using the scene's own absolute
        // position so the legend lands in the right place when the chart
        // widget isn't at the screen origin.
        let scene_origin = layout.bounds().position() - crate::core::Point::ORIGIN;
        for legend in &self.scale_legends {
            let strip = self
                .scale_legend_strips
                .get(&legend.series())
                .map(|rect| *rect + scene_origin);
            legend.draw(renderer, design, &resolved, plot_layout.bounds(), strip);
        }

        // Bottom axis (labels and ticks)
        if let Some(guide) = &self.bottom_axis {
            draw_guide(guide, 4, renderer);
        }

        // Right axis (secondary Y)
        if let Some(guide) = &self.right_axis {
            draw_guide(guide, 3, renderer);
        }

        // Axis border lines drawn last so they render on top of marks.
        // Drawn inside the plot area frame using the same pixel-snap as
        // gridlines, so they coincide structurally with the extreme
        // gridlines (no per-frame "+1 px" hacks needed). Each side is
        // gated by its axis's `shows_line()` flag.
        self.plot_area.draw_axis_borders(
            &tree.children[6],
            renderer,
            design,
            plot_layout,
            self.bottom_axis.as_ref().map(|g| g.axis()),
            self.left_axis.as_ref().map(|g| g.axis()),
            self.top_axis.as_ref().map(|g| g.axis()),
            self.right_axis.as_ref().map(|g| g.axis()),
        );
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn legend_entries_include_secondary_marks() {
        let bars = crate::bars([
            crate::bar([100.0, 120.0]).with_name("Prior year"),
            crate::bar([110.0, 130.0]).with_name("Current year"),
        ]);
        let line = crate::line([0.2, 0.25]).with_name("Operating Profit Margin %");
        let data = crate::data(bars)
            .secondary(vec![crate::Mark::Line(line)])
            .legend(crate::legend::Config::above());

        let (entries, formats) = super::legend_entries_for(&data);
        let names: Vec<_> = entries.iter().map(|entry| entry.name.as_str()).collect();

        assert_eq!(names, vec!["Prior year", "Current year", "Operating Profit Margin %"],);
        assert_eq!(formats.len(), entries.len());
    }

    /// Lays a scene out at `size` and returns the plot rectangle, the
    /// reserved scale-legend strips and the series-legend bounds.
    fn laid_out(
        data: &crate::Data,
        size: crate::core::Size,
    ) -> (
        crate::core::Rectangle,
        Vec<crate::core::Rectangle>,
        Option<crate::core::Rectangle>,
    ) {
        let mut scene = super::Scene::<(), ()>::new(data);
        let mut tree = scene.state();
        scene.layout(
            &mut tree,
            &(),
            &crate::core::layout::Limits::new(crate::core::Size::ZERO, size),
            None,
        );
        let plot = crate::core::Rectangle::new(scene.plot_area_offset, scene.plot_area_size);
        let mut strips: Vec<_> = scene.scale_legend_strips.values().copied().collect();
        strips.sort_by(|a, b| a.x.total_cmp(&b.x).then(a.y.total_cmp(&b.y)));
        assert_eq!(
            scene.scale_legends.len(),
            scene.scale_legend_marks.len(),
            "every legend plans"
        );
        (plot, strips, scene.legend_bounds)
    }

    fn overlaps(a: crate::core::Rectangle, b: crate::core::Rectangle) -> bool {
        a.intersection(&b).is_some_and(|r| r.width * r.height > 1e-3)
    }

    #[test]
    fn inset_scale_legend_strips_never_overlap_the_plot() {
        use crate::legend::Config;
        use crate::{Mark, encoding};

        let size = crate::core::Size::new(640.0, 420.0);
        let scene = crate::core::Rectangle::with_size(size);
        let heatmap = || {
            crate::heatmap(vec![1.0, 5.0, 9.0, 3.0, 7.0, 2.0], 2, 3)
                .col_names(["a", "b", "c"])
                .row_names(["x", "y"])
        };
        let bands = || {
            let axis: Vec<f64> = (0..=4).map(f64::from).collect();
            let heights: Vec<f64> = axis.iter().flat_map(|&y| axis.iter().map(move |&x| x + y)).collect();
            crate::field::Mesh::grid(&axis, &axis, &heights)
                .expect("grid")
                .contour_bands(&[0.0, 2.0, 4.0, 6.0, 8.0])
                .expect("levels")
        };
        let bubbles = || {
            crate::xy([(1.0, 2.0), (2.0, 4.0), (3.0, 3.0)])
                .with_name("Accounts")
                .size_by(encoding::size_by(|_, d| d.y).range(4.0..=36.0))
        };
        let cases = [
            (
                "heatmap right",
                crate::data(heatmap().scale_legend(Config::right()).legend_title("Count")),
                1,
            ),
            ("heatmap above", crate::data(heatmap().scale_legend(Config::above())), 1),
            ("bands left", crate::data(bands().scale_legend(Config::left())), 1),
            (
                "bubbles below with a series legend below",
                crate::data(bubbles().size_legend(Config::below()).size_legend_title("Orders")).legend(Config::below()),
                1,
            ),
            (
                "two legends stacked on the right",
                crate::data(vec![
                    Mark::Diagram(bands().scale_legend(Config::right())),
                    Mark::Xy(bubbles().size_legend(Config::right())),
                ])
                .legend(Config::right()),
                2,
            ),
        ];
        for (name, data, expected) in cases {
            let (plot, strips, legend) = laid_out(&data, size);
            assert_eq!(strips.len(), expected, "{name}: one strip per inset legend");
            assert!(plot.width > 0.0 && plot.height > 0.0, "{name}: plot keeps room");
            for (i, strip) in strips.iter().enumerate() {
                assert!(
                    !overlaps(*strip, plot),
                    "{name}: strip {strip:?} overlaps plot {plot:?}"
                );
                assert!(
                    strip.x >= -0.01
                        && strip.y >= -0.01
                        && strip.x + strip.width <= scene.width + 0.01
                        && strip.y + strip.height <= scene.height + 0.01,
                    "{name}: strip {strip:?} leaves the scene"
                );
                for other in &strips[i + 1..] {
                    assert!(!overlaps(*strip, *other), "{name}: stacked strips overlap");
                }
                if let Some(legend) = legend {
                    assert!(!overlaps(*strip, legend), "{name}: strip overlaps the series legend");
                }
            }
        }
    }

    #[test]
    fn inset_scale_legend_shrinks_the_plot_by_its_budget() {
        use crate::legend::Config;
        let size = crate::core::Size::new(640.0, 420.0);
        let base = crate::heatmap(vec![1.0, 5.0, 9.0, 3.0], 2, 2);
        let (plain, _, _) = laid_out(&crate::data(base.clone()), size);
        let (inset, strips, _) = laid_out(&crate::data(base.clone().scale_legend(Config::right())), size);
        let (overlay, overlay_strips, _) = laid_out(
            &crate::data(base.scale_legend(Config::overlay(crate::legend::Anchor::TopRight))),
            size,
        );
        let strip = strips[0];
        assert!((plain.width - inset.width - strip.width).abs() < 0.01);
        assert_eq!(plain.height, inset.height);
        assert!(overlay_strips.is_empty(), "overlays reserve nothing");
        assert_eq!(plain, overlay);
    }
}
