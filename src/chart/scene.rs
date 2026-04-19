use crate::chart::guide::{self, Guide};
use crate::chart::legend::{self, Legend};
use crate::chart::plot_area::PlotArea;
use crate::chart::title::{self, Title};
use crate::core::widget::{Tree, tree};
use crate::design;
use crate::palette::{self, Palette};

use crate::core::text;
use crate::widget::renderer::geometry;

/// Scene organizes the 7 pieces of a chart.
/// Borrows everything from Data for lifetime 'a.
/// Follows the pane_grid::Content pattern from iced.
pub struct Scene<'a, Message, Renderer>
where
    Renderer: text::Renderer<Font = crate::core::Font> + geometry::Renderer,
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
    /// Legend bounds (position + size) within the scene, computed during layout.
    /// Only set when a legend is configured.
    legend_bounds: Option<crate::core::Rectangle>,
    /// Current selection (borrowed from Data).
    selection: &'a Option<crate::target::Target>,
    /// Optional tooltip configuration (borrowed from Data).
    tooltip: Option<&'a crate::data::tooltip::Tooltip>,
}

impl<'a, Message, Renderer> Scene<'a, Message, Renderer>
where
    Renderer: text::Renderer<Font = crate::core::Font> + geometry::Renderer,
{
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
    pub(super) fn diff(&self, tree: &mut Tree) {
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
        // Extract legend entries from all marks in the primary area
        let entries: Vec<crate::data::mark::LegendEntry> =
            data.primary.marks().iter().flat_map(|m| m.legend_entries()).collect();

        let legend = match data.legend.as_ref() {
            Some(config) if !entries.is_empty() => Some(Legend::new(
                entries,
                config.position,
                config.text,
                config.wrap,
                config.interactive,
            )),
            _ => None,
        };

        // Cycle the palette across *all* marks in one shared sequence —
        // primary first, then secondary — so series on different axes
        // don't both resolve to slot 0 and collide. This matches the
        // convention in ggplot / plotly / matplotlib / vega / d3: one
        // global categorical color cycle, indexed in draw order,
        // regardless of which axis a series belongs to.
        let primary_marks = data.primary.marks();
        let secondary_marks = data.secondary.marks();
        let color_slots = palette::count_color_slots(primary_marks) + palette::count_color_slots(secondary_marks);
        let palette_strategy = data.palette.clone().unwrap_or_else(|| {
            let total_marks = primary_marks.len() + secondary_marks.len();
            if total_marks >= 2 {
                // Two or more marks → categorical, same rule the
                // multi-mark branch of `Palette::default_for` applies.
                Palette::Categorical
            } else if primary_marks.is_empty() {
                Palette::default_for(secondary_marks)
            } else {
                Palette::default_for(primary_marks)
            }
        });

        Self {
            title: data.title.as_deref().map(|t| Title::new(t, data.title_text)),
            legend,
            top_axis: data
                .secondary
                .x_axis
                .as_ref()
                .map(|axis| Guide::new(axis, data.secondary.marks())),
            right_axis: data
                .secondary
                .y_axis
                .as_ref()
                .map(|axis| Guide::new(axis, data.secondary.marks())),
            bottom_axis: data
                .primary
                .x_axis
                .as_ref()
                .map(|axis| Guide::new(axis, data.primary.marks())),
            left_axis: data
                .primary
                .y_axis
                .as_ref()
                .map(|axis| Guide::new(axis, data.primary.marks())),
            plot_area: if data.secondary.is_empty() {
                PlotArea::new(primary_marks)
            } else {
                PlotArea::new(primary_marks).with_secondary(secondary_marks)
            },
            palette: palette_strategy,
            user_palette: data.palette.clone(),
            color_slots,
            plot_area_offset: crate::core::Point::ORIGIN,
            legend_bounds: None,
            selection: &data.selection,
            tooltip: data.tooltip.as_ref(),
        }
    }

    /// Returns the plot area offset within the scene (stored during layout).
    pub(crate) fn plot_area_offset(&self) -> crate::core::Point {
        self.plot_area_offset
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
    ) -> crate::core::layout::Node {
        use crate::core::layout::Node;
        use crate::core::{Point, Size};
        use crate::data::legend::Position as LPos;

        let available = limits.max();
        let lim = |w, h| crate::core::layout::Limits::new(Size::ZERO, Size::new(w, h));

        // Tree indices: [0: title, 1: legend, 2: top, 3: right, 4: bottom, 5: left, 6: plot]

        let legend_pos = self.legend.as_ref().map(|l| l.position());
        let legend_is_side = matches!(legend_pos, Some(LPos::Left | LPos::Right));
        let legend_is_above = matches!(legend_pos, Some(LPos::Above));

        // --- Phase 1: Title (always at top, full width) ---
        let (title_height, title_node) = if let Some(title) = &self.title {
            if let Some(title_tree) = tree.children.get_mut(0) {
                let node = title.layout(title_tree, renderer, &lim(available.width, available.height));
                let h = node.size().height;
                (h, Some(node))
            } else {
                (0.0, None)
            }
        } else {
            (0.0, None)
        };

        // --- Phase 2: Side legend measurement (Left/Right) ---
        let (legend_side_width, legend_side_node) = if legend_is_side {
            if let (Some(legend), Some(legend_tree)) = (&self.legend, tree.children.get_mut(1)) {
                let remaining_h = available.height - title_height;
                let node = legend.layout(legend_tree, renderer, &lim(available.width, remaining_h));
                let w = node.size().width;
                (w, Some(node))
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
                let node = guide.layout(
                    top_tree,
                    renderer,
                    &lim(available.width, remaining_h),
                    (0.0, 0.0),
                    (0.0, 0.0),
                );
                node.size().height
            } else {
                0.0
            };
            let right_tree = &mut tree.children[3];
            let vertical = remaining_h - top_height;
            let right_width = if let Some(guide) = &self.right_axis {
                let node = guide.layout(
                    right_tree,
                    renderer,
                    &lim(available.width, vertical),
                    (0.0, 0.0),
                    (0.0, 0.0),
                );
                node.size().width
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
                let node = guide.layout(
                    bottom_tree,
                    renderer,
                    &lim(available.width, remaining_h),
                    (0.0, 0.0),
                    (0.0, 0.0),
                );
                node.size().height
            } else {
                0.0
            };

            let vertical = remaining_h - bottom_height;
            let left_width = if let Some(guide) = &self.left_axis {
                let node = guide.layout(
                    left_tree,
                    renderer,
                    &lim(available.width, vertical),
                    (0.0, 0.0),
                    (0.0, 0.0),
                );
                node.size().width
            } else {
                0.0
            };

            (bottom_height, left_width)
        };

        // --- Phase 4: Compute plot_width (horizontal space) ---
        let plot_width = available.width - left_width - right_width - legend_side_width;

        // --- Phase 5: Above/Below legend measurement (needs plot_width for wrapping) ---
        let (legend_tb_height, legend_tb_node) = if !legend_is_side {
            if let (Some(legend), Some(legend_tree)) = (&self.legend, tree.children.get_mut(1)) {
                let node = legend.layout(legend_tree, renderer, &lim(plot_width, available.height));
                let h = node.size().height;
                (h, Some(node))
            } else {
                (0.0, None)
            }
        } else {
            (0.0, None)
        };

        let above_legend_h = if legend_is_above { legend_tb_height } else { 0.0 };
        let below_legend_h = if matches!(legend_pos, Some(LPos::Below)) {
            legend_tb_height
        } else {
            0.0
        };

        // --- Phase 6: Compute plot_height (vertical space) ---
        let remaining_height = available.height - title_height - above_legend_h - top_height;
        let vertical_space = remaining_height - bottom_height - below_legend_h;
        let plot_height = vertical_space;

        // --- Phase 6.5: Series-driven inset floor ---
        // Ask the plot area how much inset its series need past the data
        // mapping region (e.g., for data labels that extend past bar ends).
        // This becomes a floor for the axis insets so ticks & bars stay aligned.
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
            self.plot_area
                .compute_series_insets(crate::core::Size::new(plot_width, plot_height), xb, yb)
        };

        // --- Phase 7: Final layout pass ---
        let (first_children, second_children) = tree.children.split_at_mut(5);

        // Overflow budgets: sibling axis columns absorb edge-label overhang.
        // Vertical axis: (top_overflow, bottom_overflow) = (top axis, bottom axis).
        // Horizontal axis: (left_overflow, right_overflow) = (left axis, right axis).
        let left_overflow = (top_height, bottom_height);
        let bottom_overflow = (left_width, right_width);

        // Min-inset floors from series (data labels).
        // Vertical axis: (top, bottom) of Plane. Horizontal axis: (left, right).
        let left_min_inset = (series_insets.top, series_insets.bottom);
        let bottom_min_inset = (series_insets.left, series_insets.right);

        let left_axis_node = self.left_axis.as_ref().map(|guide| {
            guide.layout(
                &mut second_children[0],
                renderer,
                &lim(available.width, plot_height),
                left_overflow,
                left_min_inset,
            )
        });

        let bottom_axis_node = self.bottom_axis.as_ref().map(|guide| {
            guide.layout(
                &mut first_children[4],
                renderer,
                &lim(plot_width, remaining_height),
                bottom_overflow,
                bottom_min_inset,
            )
        });

        // Secondary axes — share X/Y overflow with their siblings.
        let top_axis_node = self.top_axis.as_ref().map(|guide| {
            guide.layout(
                &mut first_children[2],
                renderer,
                &lim(plot_width, remaining_height),
                bottom_overflow,
                bottom_min_inset,
            )
        });

        let right_axis_node = self.right_axis.as_ref().map(|guide| {
            guide.layout(
                &mut first_children[3],
                renderer,
                &lim(available.width, plot_height),
                left_overflow,
                left_min_inset,
            )
        });

        // Axis bounds for plot area coordinate sync
        let (axis_bounds, secondary_bounds, plot_insets) = {
            use crate::chart::guide;
            use crate::chart::plot_area::PlotInsets;

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

            let insets = PlotInsets {
                left: h_insets.0,
                right: h_insets.1,
                top: v_insets.0,
                bottom: v_insets.1,
            };
            (bounds, secondary_bounds, insets)
        };

        let axis_layout = crate::chart::plot_area::AxisLayout {
            left_width,
            right_width,
            top_height,
            bottom_height,
            insets: plot_insets,
        };

        let plot_area_node = self.plot_area.layout(
            &mut second_children[1],
            renderer,
            &lim(plot_width, plot_height),
            axis_bounds,
            secondary_bounds,
            axis_layout,
        );

        // --- Phase 8: Position all nodes ---
        let mut layout_children = Vec::new();

        // Compute vertical offsets
        let legend_left_x = if legend_pos == Some(LPos::Left) {
            legend_side_width
        } else {
            0.0
        };
        let content_left = legend_left_x + left_width;
        let plot_top = title_height + above_legend_h + top_height;

        // Title at (0, 0)
        if let Some(node) = title_node {
            layout_children.push(node.move_to(Point::new(0.0, 0.0)));
        }

        // Legend — position depends on config
        let legend_node = legend_tb_node.or(legend_side_node);
        if let Some(node) = legend_node {
            let pos = match legend_pos.unwrap_or(LPos::Above) {
                LPos::Above => Point::new(content_left, title_height),
                LPos::Below => Point::new(content_left, plot_top + plot_height + bottom_height),
                LPos::Left => Point::new(0.0, plot_top),
                LPos::Right => Point::new(content_left + plot_width, plot_top),
            };
            let size = node.size();
            self.legend_bounds = Some(crate::core::Rectangle {
                x: pos.x,
                y: pos.y,
                width: size.width,
                height: size.height,
            });
            layout_children.push(node.move_to(pos));
        } else {
            self.legend_bounds = None;
        }

        // Top axis — above plot area
        if let Some(node) = top_axis_node {
            layout_children.push(node.move_to(Point::new(content_left, plot_top - top_height)));
        }

        // Left axis. The axis border line itself is drawn inside the plot
        // area frame by `plot_area::draw_axis_borders`, so the left axis
        // frame contains only the labels and tick marks — no +1 width
        // hack needed to bridge the gap to the plot area.
        if let Some(node) = left_axis_node {
            layout_children.push(node.move_to(Point::new(legend_left_x, plot_top)));
        }

        // Plot area
        self.plot_area_offset = Point::new(content_left, plot_top);
        layout_children.push(plot_area_node.move_to(self.plot_area_offset));

        // Bottom axis
        if let Some(node) = bottom_axis_node {
            layout_children.push(node.move_to(Point::new(content_left, plot_top + plot_height)));
        }

        // Right axis — to the right of plot area
        if let Some(node) = right_axis_node {
            layout_children.push(node.move_to(Point::new(content_left + plot_width, plot_top)));
        }

        Node::with_children(available, layout_children)
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
        layout: crate::core::Layout<'_>,
        cursor: crate::core::mouse::Cursor,
        viewport: &crate::core::Rectangle,
        hidden_series: &std::collections::HashSet<String>,
    ) where
        D: design::Design + ?Sized,
    {
        // Resolve palette for this draw call
        let seed = design.seed();
        let resolved = palette::Resolved::resolve(&self.palette, &seed, self.color_slots);

        // Navigate layout children in the same order as layout()
        // Layout tree structure: [title?, left_axis?, plot_area, bottom_axis?]
        let mut children_layouts = layout.children();

        // Tree indices:
        // 0: title, 1: legend, 2: top_axis, 3: right_axis
        // 4: bottom_axis, 5: left_axis, 6: plot_area

        // Title
        if let Some(title) = &self.title {
            let title_layout = children_layouts.next().expect("title layout must exist");
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
            let legend_layout = children_layouts.next().expect("legend layout must exist");
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

        // Top axis (secondary X)
        if let Some(guide) = &self.top_axis {
            let top_layout = children_layouts.next().expect("top axis layout must exist");
            guide.draw(&tree.children[2], renderer, design, style, top_layout, cursor, viewport);
        }

        // Left axis
        if let Some(guide) = &self.left_axis {
            let left_layout = children_layouts.next().expect("left axis layout must exist");
            guide.draw(
                &tree.children[5],
                renderer,
                design,
                style,
                left_layout,
                cursor,
                viewport,
            );
        }

        // Plot area
        let plot_layout = children_layouts.next().expect("plot layout must exist");

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

        self.plot_area.draw(
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
        );

        // Bottom axis (labels and ticks)
        if let Some(guide) = &self.bottom_axis {
            let bottom_layout = children_layouts.next().expect("bottom axis layout must exist");
            guide.draw(
                &tree.children[4],
                renderer,
                design,
                style,
                bottom_layout,
                cursor,
                viewport,
            );
        }

        // Right axis (secondary Y)
        if let Some(guide) = &self.right_axis {
            let right_layout = children_layouts.next().expect("right axis layout must exist");
            guide.draw(
                &tree.children[3],
                renderer,
                design,
                style,
                right_layout,
                cursor,
                viewport,
            );
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
