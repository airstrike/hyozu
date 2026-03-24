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
    color_slots: usize,
    /// Plot area offset within the scene, computed during layout.
    plot_area_offset: crate::core::Point,
    /// Current selection (borrowed from Data).
    selection: &'a Option<crate::target::Target>,
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
            Tree::empty(), // top_axis (optional, not used yet)
            Tree::empty(), // right_axis (optional, not used yet)
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
            Some(config) if !entries.is_empty() => {
                Some(Legend::new(entries, config.position, config.font_size, config.wrap))
            }
            _ => None,
        };

        let marks = data.primary.marks();
        let palette_strategy = data.palette.clone().unwrap_or_else(|| Palette::default_for(marks));
        let color_slots = palette::count_color_slots(marks);

        Self {
            title: data.title.as_deref().map(Title::new),
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
            plot_area: PlotArea::new(marks),
            palette: palette_strategy,
            color_slots,
            plot_area_offset: crate::core::Point::ORIGIN,
            selection: &data.selection,
        }
    }

    /// Returns the plot area offset within the scene (stored during layout).
    pub(crate) fn plot_area_offset(&self) -> crate::core::Point {
        self.plot_area_offset
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

        let top_height = 0.0; // TODO: top axis
        let right_width = 0.0; // TODO: right axis

        // --- Phase 3: Measure axes ---
        let (bottom_height, left_width) = {
            let (first_children, second_children) = tree.children.split_at_mut(5);
            let bottom_tree = &mut first_children[4];
            let left_tree = &mut second_children[0];

            let remaining_h = available.height - title_height;
            let bottom_height = if let Some(guide) = &self.bottom_axis {
                let node = guide.layout(bottom_tree, renderer, &lim(available.width, remaining_h));
                node.size().height
            } else {
                0.0
            };

            let vertical = remaining_h - bottom_height;
            let left_width = if let Some(guide) = &self.left_axis {
                let node = guide.layout(left_tree, renderer, &lim(available.width, vertical));
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

        // --- Phase 7: Final layout pass ---
        let (first_children, second_children) = tree.children.split_at_mut(5);

        let left_axis_node = self
            .left_axis
            .as_ref()
            .map(|guide| guide.layout(&mut second_children[0], renderer, &lim(available.width, plot_height)));

        let bottom_axis_node = self
            .bottom_axis
            .as_ref()
            .map(|guide| guide.layout(&mut first_children[4], renderer, &lim(plot_width, remaining_height)));

        // Axis bounds for plot area coordinate sync
        let axis_bounds = {
            use crate::chart::guide;
            let x_bounds = if self.bottom_axis.is_some() {
                Some(
                    first_children[4]
                        .state
                        .downcast_ref::<guide::State<Renderer::Paragraph>>()
                        .bounds,
                )
            } else {
                None
            };
            let y_bounds = if self.left_axis.is_some() {
                Some(
                    second_children[0]
                        .state
                        .downcast_ref::<guide::State<Renderer::Paragraph>>()
                        .bounds,
                )
            } else {
                None
            };
            match (x_bounds, y_bounds) {
                (Some(x), Some(y)) => Some((x.min(), x.max(), y.min(), y.max())),
                _ => None,
            }
        };

        let axis_layout = crate::chart::plot_area::AxisLayout {
            left_width,
            right_width,
            top_height,
            bottom_height,
        };

        let plot_area_node = self.plot_area.layout(
            &mut second_children[1],
            renderer,
            &lim(plot_width, plot_height),
            axis_bounds,
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
            layout_children.push(node.move_to(pos));
        }

        // Left axis — widen by 1px for clean corner join
        if let Some(node) = left_axis_node {
            let size = node.size();
            let children: Vec<_> = node.children().to_vec();
            let wider = Node::with_children(Size::new(size.width + 1.0, size.height), children);
            layout_children.push(wider.move_to(Point::new(legend_left_x, plot_top)));
        }

        // Plot area
        self.plot_area_offset = Point::new(content_left, plot_top);
        layout_children.push(plot_area_node.move_to(self.plot_area_offset));

        // Bottom axis
        if let Some(node) = bottom_axis_node {
            layout_children.push(node.move_to(Point::new(content_left, plot_top + plot_height)));
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
    ) where
        D: design::Design + ?Sized,
    {
        // Resolve palette for this draw call
        let seed = design.palette_seed();
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
            );
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
        self.plot_area.draw(
            &tree.children[6],
            renderer,
            design,
            style,
            plot_layout,
            cursor,
            viewport,
            &resolved,
            self.selection,
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

        // Axis lines drawn last so they render on top of marks
        // Re-iterate layout children to find axis layouts for axis line drawing
        let mut line_layouts = layout.children();

        // Skip title
        if self.title.is_some() {
            line_layouts.next();
        }
        // Skip legend
        if self.legend.is_some() {
            line_layouts.next();
        }

        // Left axis line
        if let Some(guide) = &self.left_axis {
            let left_layout = line_layouts.next().expect("left axis layout must exist");
            guide.draw_axis_line(renderer, design, left_layout);
        }

        // Skip plot area
        line_layouts.next();

        // Bottom axis line
        if let Some(guide) = &self.bottom_axis {
            let bottom_layout = line_layouts.next().expect("bottom axis layout must exist");
            guide.draw_axis_line(renderer, design, bottom_layout);
        }
    }
}
