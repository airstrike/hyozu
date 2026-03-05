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

        let legend = if entries.is_empty() {
            None
        } else {
            Some(Legend::new(entries))
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

        // Start with Scene's available space
        let available = limits.max();

        // Tree indices: [0: title, 1: legend, 2: top, 3: right, 4: bottom, 5: left, 6: plot]

        // Phase 1: Layout top decorations (title, legend)
        let (title_height, title_node, legend_height, legend_node) = {
            // Layout title if it exists
            let (title_height, title_node) = if let Some(title) = &self.title {
                if let Some(title_tree) = tree.children.get_mut(0) {
                    let node = title.layout(
                        title_tree,
                        renderer,
                        &crate::core::layout::Limits::new(Size::ZERO, available),
                    );
                    let height = node.size().height;
                    (height, Some(node))
                } else {
                    (0.0, None)
                }
            } else {
                (0.0, None)
            };

            // Layout legend if it exists
            let (legend_height, legend_node) = if let Some(legend) = &self.legend {
                if let Some(legend_tree) = tree.children.get_mut(1) {
                    let node = legend.layout(
                        legend_tree,
                        renderer,
                        &crate::core::layout::Limits::new(
                            Size::ZERO,
                            Size::new(available.width, available.height - title_height),
                        ),
                    );
                    let height = node.size().height;
                    (height, Some(node))
                } else {
                    (0.0, None)
                }
            } else {
                (0.0, None)
            };

            (title_height, title_node, legend_height, legend_node)
        };

        let top_height = 0.0; // TODO: top axis
        let right_width = 0.0; // TODO: right axis

        // Calculate remaining space after top decorations
        let remaining_height = available.height - title_height - legend_height - top_height;

        // Phase 2: Measurement pass for axes
        // Split at index 5 to access bottom (4) and left (5) independently
        let (bottom_height, left_width) = {
            let (first_children, second_children) = tree.children.split_at_mut(5);
            // first_children: [0:title, 1:legend, 2:top, 3:right, 4:bottom]
            // second_children: [5:left, 6:plot]

            let bottom_tree = &mut first_children[4];
            let left_tree = &mut second_children[0];

            // Measure bottom axis height with full width
            let bottom_height = if let Some(guide) = &self.bottom_axis {
                let node = guide.layout(
                    bottom_tree,
                    renderer,
                    &crate::core::layout::Limits::new(Size::ZERO, Size::new(available.width, remaining_height)),
                );
                node.size().height
            } else {
                0.0
            };

            // Measure left axis width with height accounting for bottom axis
            let vertical_space = remaining_height - bottom_height;
            let left_width = if let Some(guide) = &self.left_axis {
                let node = guide.layout(
                    left_tree,
                    renderer,
                    &crate::core::layout::Limits::new(Size::ZERO, Size::new(available.width, vertical_space)),
                );
                node.size().width
            } else {
                0.0
            };

            (bottom_height, left_width)
        }; // Phase 2 borrows end here

        // Calculate final dimensions for all pieces
        let vertical_space = remaining_height - bottom_height;
        let horizontal_width = available.width - left_width - right_width;

        // Phase 3: Final layout pass with correct dimensions
        // Split again to get fresh mutable borrows (previous phase borrows have ended)
        let (first_children, second_children) = tree.children.split_at_mut(5);

        let left_axis_node = self.left_axis.as_ref().map(|guide| {
            guide.layout(
                &mut second_children[0], // left axis
                renderer,
                &crate::core::layout::Limits::new(Size::ZERO, Size::new(available.width, vertical_space)),
            )
        });

        let bottom_axis_node = self.bottom_axis.as_ref().map(|guide| {
            guide.layout(
                &mut first_children[4], // bottom axis
                renderer,
                &crate::core::layout::Limits::new(Size::ZERO, Size::new(horizontal_width, remaining_height)),
            )
        });

        // 6: plot area - gets whatever space remains
        let plot_width = horizontal_width;
        let plot_height = vertical_space;

        // Read axis bounds from their states (after layout) to pass to plot area
        let axis_bounds = {
            use crate::chart::guide;

            let x_bounds = if self.bottom_axis.is_some() {
                let state = first_children[4]
                    .state
                    .downcast_ref::<guide::State<Renderer::Paragraph>>();
                Some(state.bounds)
            } else {
                None
            };

            let y_bounds = if self.left_axis.is_some() {
                let state = second_children[0]
                    .state
                    .downcast_ref::<guide::State<Renderer::Paragraph>>();
                Some(state.bounds)
            } else {
                None
            };

            // Combine X and Y bounds
            match (x_bounds, y_bounds) {
                (Some(x), Some(y)) => Some((x.min(), x.max(), y.min(), y.max())),
                _ => None, // Fall back to PlotArea computing its own bounds
            }
        };

        // Create axis layout info for obstacle computation
        let axis_layout = crate::chart::plot_area::AxisLayout {
            left_width,
            right_width,
            top_height,
            bottom_height,
        };

        // Layout plot area with its allocated space and axis bounds
        let plot_area_node = self.plot_area.layout(
            &mut second_children[1], // plot area tree
            renderer,
            &crate::core::layout::Limits::new(Size::ZERO, Size::new(plot_width, plot_height)),
            axis_bounds, // Pass axis bounds for coordinate system sync
            axis_layout, // Pass axis dimensions for obstacle computation
        );

        // Build layout tree with proper positioning
        let mut layout_children = Vec::new();

        // Title at (0, 0)
        if let Some(node) = title_node {
            layout_children.push(node.move_to(Point::new(0.0, 0.0)));
        }

        // Legend at (0, title_height)
        if let Some(node) = legend_node {
            layout_children.push(node.move_to(Point::new(0.0, title_height)));
        }

        // Left axis at (0, title_height + legend_height + top_height)
        if let Some(node) = left_axis_node {
            layout_children.push(node.move_to(Point::new(0.0, title_height + legend_height + top_height)));
        }

        // Plot area at (left_width, title_height + legend_height + top_height)
        self.plot_area_offset = Point::new(left_width, title_height + legend_height + top_height);
        layout_children.push(plot_area_node.move_to(self.plot_area_offset));

        // Bottom axis at (left_width, title + legend + top + plot_height)
        if let Some(node) = bottom_axis_node {
            layout_children.push(node.move_to(Point::new(
                left_width,
                title_height + legend_height + top_height + plot_height,
            )));
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

        // Bottom axis
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
    }
}
