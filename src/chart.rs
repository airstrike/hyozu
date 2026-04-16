pub mod guide;
pub(crate) mod hover;
pub mod legend;
pub mod plot_area;
mod scene;
pub mod title;
pub mod value;

use std::borrow::Cow;

use crate::core::widget::{Tree, tree};
use crate::core::{Element, Event, Layout, Length, Padding, Point, Rectangle, Size, Widget, layout, mouse};
use crate::data::tooltip::TooltipEntry;
use crate::widget::Renderer;

use crate::{Action, Data, design};
use scene::Scene;

const DEFAULT_PADDING: Padding = Padding::new(10.0);

/// Pixels to expand each legend entry's cached rect by when hit-testing,
/// so small clickable swatches are forgiving to hover/click at the edges.
const LEGEND_HIT_PADDING: f32 = 2.0;

/// Chart widget for displaying data visualizations.
pub struct Chart<'a, Message, Design = crate::core::Theme, Theme = crate::core::Theme>
where
    Design: design::Design + Clone,
{
    scene: Scene<'a, Message, Renderer>,
    generation: u64,
    padding: Padding,
    width: Length,
    height: Length,
    on_action: Option<Box<dyn Fn(Action) -> Message + 'a>>,
    design: Option<Cow<'a, Design>>,
    style: StyleFn<'a>,
    _theme: std::marker::PhantomData<Theme>,
}

/// Internal state for the chart widget.
#[derive(Default)]
struct State {
    generation: u64,
    is_pressed: bool,
    hover: Option<hover::State>,
    /// Names of series hidden via legend click toggles. Lives with the
    /// widget state and is reset when the tree is dropped.
    hidden_series: std::collections::HashSet<String>,
}

/// Creates a chart widget from data.
///
/// The Theme type is inferred from context (e.g., the Element type in your view).
/// Call `.design()` to use a custom design type separate from your Theme.
pub fn chart<'a, Message, Theme>(data: &'a Data) -> Chart<'a, Message, Theme, Theme>
where
    Theme: design::Design + Clone,
{
    Chart::new(data)
}

impl<'a, Message, Design, Theme> Chart<'a, Message, Design, Theme>
where
    Design: design::Design + Clone,
{
    /// Creates a new chart widget from data.
    ///
    /// Uses the application's theme for styling by default.
    pub fn new(data: &'a Data) -> Self {
        let scene = Scene::new(data);

        Chart {
            scene,
            generation: data.generation,
            padding: DEFAULT_PADDING,
            width: Length::Fill,
            height: Length::Fill,
            on_action: None,
            design: None,
            style: Box::new(default),
            _theme: std::marker::PhantomData,
        }
    }

    /// Sets the width of the chart.
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// Sets the height of the chart.
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }

    /// Sets the [`Padding`] of the chart.
    pub fn padding(mut self, padding: impl Into<Padding>) -> Self {
        self.padding = padding.into();
        self
    }

    /// Sets a custom design for this chart, changing the Design type.
    ///
    /// Accepts either a reference or an owned value.
    /// If not set, the chart will use the application's theme.
    pub fn design<D: design::Design + Clone + 'a>(
        self,
        design: impl design::IntoDesign<'a, D>,
    ) -> Chart<'a, Message, D, Theme> {
        Chart {
            scene: self.scene,
            generation: self.generation,
            padding: self.padding,
            width: self.width,
            height: self.height,
            on_action: self.on_action,
            design: Some(design.into_design()),
            style: self.style,
            _theme: std::marker::PhantomData,
        }
    }

    /// Sets the action handler for the chart.
    pub fn on_action(mut self, f: impl Fn(Action) -> Message + 'a) -> Self {
        self.on_action = Some(Box::new(f));
        self
    }

    /// Sets the style of the [`Chart`].
    pub fn style(mut self, style: impl Fn(&dyn design::Design) -> Style + 'a) -> Self {
        self.style = Box::new(style);
        self
    }
}

/// Compute the absolute plot bounds from chart bounds, padding, and plane.
fn compute_plot_bounds(
    chart_bounds: Rectangle,
    padding: Padding,
    plot_area_offset: Point,
    plane: &plot_area::Plane,
) -> Rectangle {
    Rectangle {
        x: chart_bounds.x + padding.left + plot_area_offset.x + plane.bounds.x,
        y: chart_bounds.y + padding.top + plot_area_offset.y + plane.bounds.y,
        width: plane.bounds.width,
        height: plane.bounds.height,
    }
}

/// Scan the plot area tree for the nearest pixel x, then collect all entries
/// at that x. Returns `None` if no hoverable points exist or the cursor is
/// too far from any point.
fn find_nearest_hover(local: Point, plot_area_tree: &Tree, plane: &plot_area::Plane) -> Option<hover::State> {
    let line_tag = tree::Tag::of::<plot_area::line::State>();
    let area_tag = tree::Tag::of::<plot_area::area::State>();
    let xy_tag = tree::Tag::of::<plot_area::xy::State>();
    let bars_tag = tree::Tag::of::<plot_area::bars::State>();

    let mut best_dist = f32::INFINITY;
    let mut best_pixel_x = 0.0f32;

    // Pass 1: find globally nearest pixel_x
    for child in &plot_area_tree.children {
        if child.tag == line_tag {
            let s = child.state.downcast_ref::<plot_area::line::State>();
            for pt in &s.pixel_points {
                let d = (pt.x - local.x).abs();
                if d < best_dist {
                    best_dist = d;
                    best_pixel_x = pt.x;
                }
            }
        } else if child.tag == area_tag {
            let s = child.state.downcast_ref::<plot_area::area::State>();
            for sub in &s.series_points {
                for pt in sub {
                    let d = (pt.x - local.x).abs();
                    if d < best_dist {
                        best_dist = d;
                        best_pixel_x = pt.x;
                    }
                }
            }
        } else if child.tag == xy_tag {
            let s = child.state.downcast_ref::<plot_area::xy::State>();
            for pt in &s.pixel_points {
                let d = (pt.x - local.x).abs();
                if d < best_dist {
                    best_dist = d;
                    best_pixel_x = pt.x;
                }
            }
        } else if child.tag == bars_tag {
            let s = child.state.downcast_ref::<plot_area::bars::State>();
            for rects in &s.series_rects {
                for rect in rects {
                    let center_x = rect.x + rect.width / 2.0;
                    let d = (center_x - local.x).abs();
                    if d < best_dist {
                        best_dist = d;
                        best_pixel_x = center_x;
                    }
                }
            }
        }
    }

    if best_dist == f32::INFINITY {
        return None;
    }

    // Snap threshold: 30px max distance
    if best_dist > 30.0 {
        return None;
    }

    let data_x = plane.to_data_x(best_pixel_x);

    // Pass 2: collect all entries at that pixel_x (within tolerance)
    let tolerance = 0.5f32;
    // Bars need a wider tolerance because bar center x can differ from line pixel x
    let bar_tolerance = 4.0f32;
    let mut entries = Vec::new();

    for (mark_idx, child) in plot_area_tree.children.iter().enumerate() {
        if child.tag == line_tag {
            let s = child.state.downcast_ref::<plot_area::line::State>();
            for (pt_idx, pt) in s.pixel_points.iter().enumerate() {
                if (pt.x - best_pixel_x).abs() <= tolerance {
                    entries.push((mark_idx, 0, pt_idx));
                    break; // one entry per line series
                }
            }
        } else if child.tag == area_tag {
            let s = child.state.downcast_ref::<plot_area::area::State>();
            for (ser_idx, sub) in s.series_points.iter().enumerate() {
                for (pt_idx, pt) in sub.iter().enumerate() {
                    if (pt.x - best_pixel_x).abs() <= tolerance {
                        entries.push((mark_idx, ser_idx, pt_idx));
                        break;
                    }
                }
            }
        } else if child.tag == xy_tag {
            let s = child.state.downcast_ref::<plot_area::xy::State>();
            for (pt_idx, pt) in s.pixel_points.iter().enumerate() {
                if (pt.x - best_pixel_x).abs() <= tolerance {
                    entries.push((mark_idx, 0, pt_idx));
                    break;
                }
            }
        } else if child.tag == bars_tag {
            let s = child.state.downcast_ref::<plot_area::bars::State>();
            for (ser_idx, rects) in s.series_rects.iter().enumerate() {
                for (bar_idx, rect) in rects.iter().enumerate() {
                    let center_x = rect.x + rect.width / 2.0;
                    if (center_x - best_pixel_x).abs() <= bar_tolerance {
                        entries.push((mark_idx, ser_idx, bar_idx));
                        break;
                    }
                }
            }
        }
    }

    if entries.is_empty() {
        return None;
    }
    Some(hover::State { data_x, entries })
}

impl<'a, Message, Design, Theme> Widget<Message, Theme, Renderer> for Chart<'a, Message, Design, Theme>
where
    Design: design::Design + Clone + 'a,
    Theme: design::Design,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn children(&self) -> Vec<Tree> {
        vec![self.scene.state()]
    }

    fn diff(&self, tree: &mut Tree) {
        let state = tree.state.downcast_mut::<State>();
        if state.generation != self.generation {
            state.generation = self.generation;
            tree.children = self.children();
            return;
        }
        self.scene.diff(&mut tree.children[0]);
    }

    fn size(&self) -> Size<Length> {
        Size {
            width: self.width,
            height: self.height,
        }
    }

    fn layout(&mut self, tree: &mut Tree, renderer: &Renderer, limits: &layout::Limits) -> layout::Node {
        use crate::core::Point;

        let size = limits.resolve(self.width, self.height, Size::ZERO);

        // Calculate inner size after padding
        let inner_size = Size::new(
            (size.width - self.padding.left - self.padding.right).max(0.0),
            (size.height - self.padding.top - self.padding.bottom).max(0.0),
        );

        // Get the scene's tree
        let scene_tree = &mut tree.children[0];

        // Delegate layout to scene with padded limits
        let scene_node = self
            .scene
            .layout(scene_tree, renderer, &layout::Limits::new(Size::ZERO, inner_size));

        // Position scene node with padding offset
        layout::Node::with_children(size, vec![
            scene_node.move_to(Point::new(self.padding.left, self.padding.top)),
        ])
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _renderer: &Renderer,
        shell: &mut crate::core::Shell<'_, Message>,
        _viewport: &Rectangle,
    ) {
        if shell.is_event_captured() {
            return;
        }

        let has_tooltip = self.scene.has_tooltip();
        let has_action = self.on_action.is_some();
        let legend_interactive = self.scene.legend().map(|l| l.interactive()).unwrap_or(false);

        // --- Interactive legend click-to-toggle ---
        //
        // Handled before plot-area actions so legend clicks take priority
        // even if an action handler is attached.
        // Hit-test before borrowing widget State mutably below.
        let legend_click =
            if legend_interactive && matches!(event, Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))) {
                self.legend_entry_at(tree, layout, cursor)
            } else {
                None
            };

        let state = tree.state.downcast_mut::<State>();

        if let Some(idx) = legend_click
            && let Some(legend) = self.scene.legend()
        {
            let entry = &legend.entries()[idx];
            if state.hidden_series.contains(&entry.name) {
                state.hidden_series.remove(&entry.name);
            } else {
                state.hidden_series.insert(entry.name.clone());
            }
            shell.capture_event();
            shell.request_redraw();
            return;
        }

        if !has_tooltip && !has_action {
            return;
        }

        // Compute absolute plot area bounds from the layout tree.
        let chart_bounds = layout.bounds();
        let plot_area_offset = self.scene.plot_area_offset();
        let scene_tree = &tree.children[0];
        let plot_area_state = scene_tree.children[6].state.downcast_ref::<plot_area::State>();
        let Some(plane) = &plot_area_state.plane else {
            return;
        };
        let plot_bounds = compute_plot_bounds(chart_bounds, self.padding, plot_area_offset, plane);

        match event {
            // === HOVER HANDLING ===
            Event::Mouse(mouse::Event::CursorMoved { .. }) if has_tooltip => match cursor.position_in(plot_bounds) {
                Some(local) => {
                    let plot_area_tree = &scene_tree.children[6];
                    let new_hover = find_nearest_hover(Point::new(local.x, local.y), plot_area_tree, plane);

                    let changed = match (&state.hover, &new_hover) {
                        (Some(old), Some(new_h)) => (old.data_x - new_h.data_x).abs() > f64::EPSILON,
                        (None, Some(_)) | (Some(_), None) => true,
                        (None, None) => false,
                    };

                    state.hover = new_hover;
                    if changed {
                        shell.request_redraw();
                    }
                }
                None => {
                    if state.hover.is_some() {
                        state.hover = None;
                        shell.request_redraw();
                    }
                }
            },
            Event::Mouse(mouse::Event::CursorLeft) if has_tooltip => {
                if state.hover.is_some() {
                    state.hover = None;
                    shell.request_redraw();
                }
            }

            // === CLICK HANDLING ===
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) if has_action => {
                if cursor.is_over(plot_bounds) {
                    state.is_pressed = true;
                    shell.capture_event();
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) if has_action => {
                let on_action = self.on_action.as_ref().unwrap();

                if state.is_pressed {
                    state.is_pressed = false;
                    shell.capture_event();

                    // Use position_in to get plot-area-local coordinates
                    let Some(local) = cursor.position_in(plot_bounds) else {
                        // Released outside plot area — deselect
                        shell.publish(on_action(Action::Clicked(crate::target::Target::Mark(usize::MAX))));
                        return;
                    };

                    let local = Point::new(local.x, local.y);

                    // Hit-test against bar/pie elements in the tree
                    let plot_area_tree = &scene_tree.children[6];
                    let bars_tag = tree::Tag::of::<plot_area::bars::State>();
                    let pie_tag = tree::Tag::of::<plot_area::pie::State>();
                    let treemap_tag = tree::Tag::of::<plot_area::treemap::State>();
                    let choropleth_tag = tree::Tag::of::<plot_area::choropleth::State>();

                    // First pass: hit-test labels (labels win when overlapping shapes)
                    for (mark_idx, mark_tree) in plot_area_tree.children.iter().enumerate() {
                        if mark_tree.tag == bars_tag {
                            let bars_state = mark_tree.state.downcast_ref::<plot_area::bars::State>();

                            for (series_idx, label_rects) in bars_state.label_rects.iter().enumerate() {
                                for (label_idx, maybe_rect) in label_rects.iter().enumerate() {
                                    if let Some(rect) = maybe_rect
                                        && rect.contains(local)
                                    {
                                        shell.publish(on_action(Action::Clicked(crate::target::Target::EntryLabel {
                                            mark: mark_idx,
                                            series: series_idx,
                                            index: label_idx,
                                        })));
                                        return;
                                    }
                                }
                            }
                        } else if mark_tree.tag == pie_tag {
                            let pie_state = mark_tree.state.downcast_ref::<plot_area::pie::State>();

                            for (label_idx, maybe_rect) in pie_state.label_rects.iter().enumerate() {
                                if let Some(rect) = maybe_rect
                                    && rect.contains(local)
                                {
                                    shell.publish(on_action(Action::Clicked(crate::target::Target::EntryLabel {
                                        mark: mark_idx,
                                        series: 0,
                                        index: label_idx,
                                    })));
                                    return;
                                }
                            }
                        }
                    }

                    // Second pass: hit-test shapes
                    for (mark_idx, mark_tree) in plot_area_tree.children.iter().enumerate() {
                        if mark_tree.tag == bars_tag {
                            let bars_state = mark_tree.state.downcast_ref::<plot_area::bars::State>();

                            for (series_idx, rects) in bars_state.series_rects.iter().enumerate() {
                                for (bar_idx, rect) in rects.iter().enumerate() {
                                    if rect.contains(local) {
                                        shell.publish(on_action(Action::Clicked(crate::target::Target::Entry {
                                            mark: mark_idx,
                                            series: series_idx,
                                            index: bar_idx,
                                        })));
                                        return;
                                    }
                                }
                            }
                        } else if mark_tree.tag == pie_tag {
                            let pie_state = mark_tree.state.downcast_ref::<plot_area::pie::State>();

                            let (cx, cy) = pie_state.center;
                            let dx = local.x - cx;
                            let dy = local.y - cy;
                            let dist = (dx * dx + dy * dy).sqrt();

                            if dist >= pie_state.inner_radius && dist <= pie_state.outer_radius {
                                let mut angle = dy.atan2(dx);
                                let first_start = pie_state.slice_angles.first().map(|(s, _)| *s).unwrap_or(0.0);
                                if angle < first_start {
                                    angle += std::f32::consts::TAU;
                                }

                                for (slice_idx, (start_angle, end_angle)) in pie_state.slice_angles.iter().enumerate() {
                                    if angle >= *start_angle && angle <= *end_angle {
                                        shell.publish(on_action(Action::Clicked(crate::target::Target::Entry {
                                            mark: mark_idx,
                                            series: 0,
                                            index: slice_idx,
                                        })));
                                        return;
                                    }
                                }
                            }
                        } else if mark_tree.tag == treemap_tag {
                            let tm_state = mark_tree.state.downcast_ref::<plot_area::treemap::State>();

                            for (item_idx, rect) in tm_state.item_rects.iter().enumerate() {
                                if rect.contains(local) {
                                    shell.publish(on_action(Action::Clicked(crate::target::Target::Entry {
                                        mark: mark_idx,
                                        series: 0,
                                        index: item_idx,
                                    })));
                                    return;
                                }
                            }
                        } else if mark_tree.tag == choropleth_tag {
                            let choro_state = mark_tree.state.downcast_ref::<plot_area::choropleth::State>();

                            // Bbox pre-filter then ray-cast point-in-polygon
                            for (feat_idx, bbox) in choro_state.feature_bboxes.iter().enumerate() {
                                if !bbox.contains(local) {
                                    continue;
                                }
                                // Ray-casting: count crossings of a horizontal ray to the right
                                let mut inside = false;
                                for ring in &choro_state.projected_polygons[feat_idx] {
                                    let n = ring.len();
                                    let mut j = n.wrapping_sub(1);
                                    for i in 0..n {
                                        let (xi, yi) = ring[i];
                                        let (xj, yj) = ring[j];
                                        if ((yi > local.y) != (yj > local.y))
                                            && (local.x < (xj - xi) * (local.y - yi) / (yj - yi) + xi)
                                        {
                                            inside = !inside;
                                        }
                                        j = i;
                                    }
                                }
                                if inside && let Some(id) = choro_state.filtered_ids.get(feat_idx) {
                                    shell.publish(on_action(Action::Clicked(crate::target::Target::Feature {
                                        mark: mark_idx,
                                        id: id.clone(),
                                    })));
                                    return;
                                }
                            }
                        }
                    }

                    // No element hit — report empty click
                    shell.publish(on_action(Action::Clicked(crate::target::Target::Mark(usize::MAX))));
                }
            }
            _ => {}
        }
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        defaults: &crate::core::renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        // Get the design to use (explicit or from theme)
        let design: &dyn design::Design = match &self.design {
            Some(d) => d.as_ref(),
            None => theme,
        };

        // Get style from the design and draw background
        let style = (self.style)(design);
        draw_background(renderer, &style, layout.bounds());

        // Draw scene contents (use child layout which accounts for padding)
        let scene_layout = layout.children().next().unwrap();

        let state = tree.state.downcast_ref::<State>();

        self.scene.draw(
            &tree.children[0],
            renderer,
            design,
            defaults,
            scene_layout,
            cursor,
            viewport,
            &state.hidden_series,
        );

        // Draw tooltip overlay
        if let Some(hover) = &state.hover
            && let Some(tooltip_config) = self.scene.tooltip()
        {
            let scene_tree = &tree.children[0];
            let plot_area_state = scene_tree.children[6].state.downcast_ref::<plot_area::State>();
            if let Some(plane) = &plot_area_state.plane {
                draw_tooltip_overlay(
                    renderer,
                    design,
                    layout.bounds(),
                    self.padding,
                    self.scene.plot_area_offset(),
                    plane,
                    hover,
                    tooltip_config,
                    &self.scene,
                    scene_tree,
                    viewport,
                );
            }
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        // Interactive legend entries — show a Pointer cursor when hovering
        // over a toggleable swatch so the affordance is clear.
        if self.legend_entry_at(tree, layout, cursor).is_some() {
            return mouse::Interaction::Pointer;
        }

        if self.scene.has_tooltip() {
            let chart_bounds = layout.bounds();
            let plot_area_offset = self.scene.plot_area_offset();
            let scene_tree = &tree.children[0];
            let plot_area_state = scene_tree.children[6].state.downcast_ref::<plot_area::State>();
            if let Some(plane) = &plot_area_state.plane {
                let plot_bounds = compute_plot_bounds(chart_bounds, self.padding, plot_area_offset, plane);
                if cursor.is_over(plot_bounds) {
                    return mouse::Interaction::Crosshair;
                }
            }
        }
        mouse::Interaction::None
    }
}

impl<'a, Message, Design, Theme> Chart<'a, Message, Design, Theme>
where
    Design: design::Design + Clone + 'a,
    Theme: design::Design,
{
    /// Hit-tests the cursor against legend entry rectangles.
    ///
    /// Returns `Some(idx)` if the legend is interactive and the cursor is
    /// over the `idx`-th entry swatch+label, else `None`. Shared between
    /// click handling in `update` and cursor feedback in `mouse_interaction`
    /// so both use the exact same hit region.
    fn legend_entry_at(&self, tree: &Tree, layout: Layout<'_>, cursor: mouse::Cursor) -> Option<usize> {
        let legend = self.scene.legend()?;
        if !legend.interactive() {
            return None;
        }
        let legend_rect = self.scene.legend_bounds()?;
        let chart_bounds = layout.bounds();
        let abs_legend = Rectangle {
            x: chart_bounds.x + self.padding.left + legend_rect.x,
            y: chart_bounds.y + self.padding.top + legend_rect.y,
            width: legend_rect.width,
            height: legend_rect.height,
        };
        let local = cursor.position_in(abs_legend)?;
        let scene_tree = &tree.children[0];
        let legend_tree = &scene_tree.children[1];
        let legend_state = legend_tree
            .state
            .downcast_ref::<legend::State<<Renderer as crate::core::text::Renderer>::Paragraph>>();
        let probe = Point::new(local.x, local.y);
        legend_state.entry_rects.iter().enumerate().find_map(|(i, maybe_rect)| {
            maybe_rect.and_then(|r| r.expand(LEGEND_HIT_PADDING).contains(probe).then_some(i))
        })
    }
}

impl<'a, Message, Design, Theme> From<Chart<'a, Message, Design, Theme>> for Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Design: design::Design + Clone + 'a,
    Theme: design::Design + 'a,
{
    fn from(chart: Chart<'a, Message, Design, Theme>) -> Self {
        Element::new(chart)
    }
}

/// Draws the background of a [`Chart`] given its [`Style`] and its `bounds`.
pub fn draw_background(renderer: &mut Renderer, style: &Style, bounds: Rectangle) {
    use crate::core::renderer::Renderer as _;

    if style.background.is_some() || style.border.width > 0.0 {
        renderer.fill_quad(
            crate::core::renderer::Quad {
                bounds,
                border: style.border,
                ..Default::default()
            },
            style.background.unwrap_or(crate::core::Color::TRANSPARENT),
        );
    }
}

/// Draws the tooltip overlay (tracking line, markers, tooltip box).
///
/// Wrapped in `renderer.with_layer()` so the entire overlay composites
/// above chart content (lines, bars, axes, etc.).
#[allow(clippy::too_many_arguments, clippy::unit_arg)]
fn draw_tooltip_overlay<Message>(
    renderer: &mut Renderer,
    design: &dyn design::Design,
    chart_bounds: Rectangle,
    padding: Padding,
    plot_area_offset: Point,
    plane: &plot_area::Plane,
    hover: &hover::State,
    tooltip_config: &crate::data::tooltip::Tooltip,
    scene: &scene::Scene<'_, Message, Renderer>,
    scene_tree: &Tree,
    viewport: &Rectangle,
) {
    use crate::core::renderer::Renderer as _;
    use crate::core::text::Renderer as _;
    use crate::widget::canvas::{Frame, Path, Stroke};
    use crate::widget::renderer::geometry;

    let plot_bounds = compute_plot_bounds(chart_bounds, padding, plot_area_offset, plane);
    let plot_area = scene.plot_area();
    let plot_area_tree = &scene_tree.children[6];

    let background = design.background_color();
    let text_pair = design.text_pair();
    let seed = design.palette_seed();
    let text_color = design.text_color().resolve(background, text_pair, &seed, None);
    let palette = scene.resolve_palette(design);

    // Compute the tracking pixel x from data_x
    let tracking_pixel_x = {
        let t = if plane.x_max > plane.x_min {
            ((hover.data_x - plane.x_min) / (plane.x_max - plane.x_min)) as f32
        } else {
            0.5
        };
        plane.bounds.x + t * plane.bounds.width
    };

    // Build tooltip entries and collect pixel positions
    let line_tag = tree::Tag::of::<plot_area::line::State>();
    let area_tag = tree::Tag::of::<plot_area::area::State>();
    let xy_tag = tree::Tag::of::<plot_area::xy::State>();
    let bars_tag = tree::Tag::of::<plot_area::bars::State>();

    let mut entries: Vec<hover::Entry> = Vec::new();

    for &(mark_idx, series_idx, pt_idx) in &hover.entries {
        let series = &plot_area.series[mark_idx];
        let child = &plot_area_tree.children[mark_idx];

        // Bars resolve their color through the full priority chain
        // (point_colors > color_by > series.color > palette). When this is
        // `Some`, the generic explicit_color/palette-fallback logic below is
        // bypassed. See GOG.md § 8b.
        let mut bars_resolved_color: Option<crate::core::Color> = None;

        let (datum, name, explicit_color, annotation) = if child.tag == line_tag {
            if let plot_area::Series::Line(line) = series {
                let pt = &line.data.points[pt_idx];
                let pixel = plane.to_pixel(crate::data::Datum { x: pt.x, y: pt.y });
                (
                    crate::data::Datum { x: pt.x, y: pt.y },
                    line.data.name().map(|s| s.to_string()),
                    line.data.color,
                    hover::Annotation::PointMarker { pixel, radius: 4.0 },
                )
            } else {
                continue;
            }
        } else if child.tag == area_tag {
            if let plot_area::Series::Area(area) = series {
                let ser = &area.data.series[series_idx];
                let pt = &ser.points[pt_idx];
                let pixel = plane.to_pixel(crate::data::Datum { x: pt.x, y: pt.y });
                (
                    crate::data::Datum { x: pt.x, y: pt.y },
                    ser.name().map(|s| s.to_string()),
                    ser.color,
                    hover::Annotation::PointMarker { pixel, radius: 4.0 },
                )
            } else {
                continue;
            }
        } else if child.tag == xy_tag {
            if let plot_area::Series::Xy(xy) = series {
                let pt = &xy.data.points[pt_idx];
                let pixel = plane.to_pixel(crate::data::Datum { x: pt.x, y: pt.y });
                (
                    crate::data::Datum { x: pt.x, y: pt.y },
                    xy.data.name.as_deref().map(|s| s.to_string()),
                    None,
                    hover::Annotation::PointMarker { pixel, radius: 4.0 },
                )
            } else {
                continue;
            }
        } else if child.tag == bars_tag {
            if let plot_area::Series::Bars(bars) = series {
                let bar_series = &bars.data.series[series_idx];
                let pt = &bar_series.points[pt_idx];

                // Resolve the bar's displayed color via the full priority
                // chain (point_colors > color_by > series.color > palette)
                // so the tooltip swatch matches the actual bar color.
                let color_idx = plot_area.color_offset_for(mark_idx, series_idx);
                let fallback = palette.get(color_idx);
                let resolved = bar_series.resolved_color_at(pt_idx, &seed, scene.user_palette(), fallback);
                bars_resolved_color = Some(resolved.resolve(background, text_pair, &seed, None));

                (
                    crate::data::Datum { x: pt.x, y: pt.y },
                    bar_series.name().map(|s| s.to_string()),
                    None,
                    hover::Annotation::None,
                )
            } else {
                continue;
            }
        } else {
            continue;
        };

        let anchor = plane.to_pixel(datum);

        // Resolve series color: bars pre-compute through the priority chain;
        // other marks use explicit mark color if set, else palette fallback.
        let series_color = if let Some(c) = bars_resolved_color {
            c
        } else if let Some(c) = explicit_color {
            c.resolve(background, text_pair, &seed, None)
        } else {
            let color_idx = plot_area.color_offset_for(mark_idx, series_idx);
            palette.get(color_idx).resolve(background, text_pair, &seed, None)
        };

        entries.push(hover::Entry {
            tooltip: TooltipEntry {
                x: datum.x,
                y: datum.y,
                series_name: name,
                series_index: series_idx,
                mark_index: mark_idx,
                color: series_color,
            },
            anchor,
            color: series_color,
            annotation,
        });
    }

    if entries.is_empty() {
        return;
    }

    // Wrap everything in a layer so it composites above chart content
    renderer.with_layer(*viewport, |renderer| {
        // --- Draw tracking line and markers via canvas Frame ---
        let frame_size = crate::core::Size::new(plane.bounds.width, plane.bounds.height);
        let mut frame = Frame::new(renderer, frame_size);

        // Tracking line — only for continuous marks (line/area/xy), not discrete (bars)
        let has_continuous = entries
            .iter()
            .any(|re| matches!(re.annotation, hover::Annotation::PointMarker { .. }));
        if tooltip_config.tracking_line && has_continuous {
            let line_x = tracking_pixel_x - plane.bounds.x;
            let tracking_color = crate::core::Color { a: 0.18, ..text_color };
            let path = Path::line(Point::new(line_x, 0.0), Point::new(line_x, plane.bounds.height));
            let dash_pattern: [f32; 2] = [4.0, 3.0];
            let stroke = Stroke {
                line_dash: crate::widget::canvas::LineDash {
                    segments: &dash_pattern,
                    offset: 0,
                },
                ..Stroke::default().with_width(1.0).with_color(tracking_color)
            };
            frame.stroke(&path, stroke);
        }

        // Hover annotations — each mark type declares its own visual
        if tooltip_config.markers {
            for re in &entries {
                match &re.annotation {
                    hover::Annotation::PointMarker { pixel, radius } => {
                        let cx = pixel.x - plane.bounds.x;
                        let cy = pixel.y - plane.bounds.y;

                        let circle = Path::circle(Point::new(cx, cy), *radius);
                        frame.fill(&circle, re.color);

                        let ring = Path::circle(Point::new(cx, cy), *radius);
                        frame.stroke(
                            &ring,
                            Stroke::default().with_width(1.5).with_color(crate::core::Color::WHITE),
                        );
                    }
                    hover::Annotation::None => {}
                }
            }
        }

        renderer.with_translation(crate::core::Vector::new(plot_bounds.x, plot_bounds.y), |renderer| {
            geometry::Renderer::draw_geometry(renderer, frame.into_geometry());
        });

        // --- Draw tooltip box using renderer fill_quad/fill_text ---
        let font = renderer.default_font();
        let font_size: f32 = 12.0;
        let line_height_px: f32 = 18.0;
        let swatch_size: f32 = 8.0;
        let swatch_gap: f32 = 6.0;
        let box_padding: f32 = 8.0;
        let box_gap: f32 = 8.0;

        let formatted: Vec<String> = entries.iter().map(|re| (tooltip_config.format)(&re.tooltip)).collect();

        let char_width = font_size * 0.58;
        let max_text_width: f32 = formatted
            .iter()
            .map(|s| s.len() as f32 * char_width)
            .fold(0.0f32, f32::max);

        let swatch_space = if tooltip_config.swatch {
            swatch_size + swatch_gap
        } else {
            0.0
        };
        let box_width = box_padding * 2.0 + swatch_space + max_text_width;
        let box_height = box_padding * 2.0 + entries.len() as f32 * line_height_px;

        // Position tooltip box: right of tracking line if in left half, else left
        let half_x = plot_bounds.x + plane.bounds.width / 2.0;
        let abs_tracking_x = plot_bounds.x + tracking_pixel_x - plane.bounds.x;
        let box_x = if abs_tracking_x < half_x {
            abs_tracking_x + box_gap
        } else {
            abs_tracking_x - box_width - box_gap
        };

        // Vertically center on mean y of entries, clamped within chart bounds
        let mean_y: f32 = entries.iter().map(|re| re.anchor.y).sum::<f32>() / entries.len() as f32;
        let abs_mean_y = plot_bounds.y + mean_y - plane.bounds.y;
        let box_y = (abs_mean_y - box_height / 2.0)
            .max(chart_bounds.y + 2.0)
            .min(chart_bounds.y + chart_bounds.height - box_height - 2.0);

        // Tooltip background — opaque by default (ensures readability on transparent charts)
        let tooltip_bg = crate::core::Color { a: 1.0, ..background };
        let divider_color = design.divider_color().resolve(background, text_pair, &seed, None);

        renderer.fill_quad(
            crate::core::renderer::Quad {
                bounds: Rectangle {
                    x: box_x,
                    y: box_y,
                    width: box_width,
                    height: box_height,
                },
                border: crate::core::Border {
                    width: 1.0,
                    radius: 4.0.into(),
                    color: divider_color,
                },
                ..Default::default()
            },
            tooltip_bg,
        );

        // Draw each entry line
        for (i, (re, text)) in entries.iter().zip(formatted.iter()).enumerate() {
            let row_y = box_y + box_padding + i as f32 * line_height_px;
            let mut text_x = box_x + box_padding;

            // Colored swatch circle (if enabled)
            if tooltip_config.swatch {
                let swatch_y = row_y + (line_height_px - swatch_size) / 2.0;
                renderer.fill_quad(
                    crate::core::renderer::Quad {
                        bounds: Rectangle {
                            x: text_x,
                            y: swatch_y,
                            width: swatch_size,
                            height: swatch_size,
                        },
                        border: crate::core::Border {
                            radius: (swatch_size / 2.0).into(),
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                    re.color,
                );
                text_x += swatch_size + swatch_gap;
            }

            // Text label — optionally colored to match series
            let label_color = if tooltip_config.colored_text {
                re.color
            } else {
                text_color
            };

            renderer.fill_text(
                crate::core::text::Text {
                    content: text.clone(),
                    bounds: Size::new(max_text_width + 10.0, line_height_px),
                    size: font_size.into(),
                    font,
                    align_x: crate::core::alignment::Horizontal::Left.into(),
                    align_y: crate::core::alignment::Vertical::Top,
                    line_height: crate::core::text::LineHeight::default(),
                    shaping: crate::core::text::Shaping::Basic,
                    wrapping: crate::core::text::Wrapping::None,
                    ellipsis: crate::core::text::Ellipsis::default(),
                    hint_factor: renderer.scale_factor(),
                    font_features: Vec::new(),
                    font_variations: Vec::new(),
                    letter_spacing: Default::default(),
                    weight: None,
                },
                Point::new(text_x, row_y + (line_height_px - font_size) / 2.0),
                label_color,
                *viewport,
            );
        }
    });
}

/// The appearance of a chart.
#[derive(Debug, Clone, Copy, Default)]
pub struct Style {
    /// The background color of the chart.
    pub background: Option<crate::core::Color>,
    /// The border of the chart.
    pub border: crate::core::Border,
}

/// A styling function for a [`Chart`].
pub type StyleFn<'a> = Box<dyn Fn(&dyn design::Design) -> Style + 'a>;

/// The default [`Chart`] style with background color and thin rounded border.
pub fn default(design: &dyn design::Design) -> Style {
    Style {
        background: Some(design.background_color()),
        border: crate::core::Border {
            width: 1.0,
            radius: 5.0.into(),
            color: design.divider_color().resolve(
                design.background_color(),
                design.text_pair(),
                &design.palette_seed(),
                None,
            ),
        },
    }
}

/// A transparent [`Chart`] with no background or border.
pub fn transparent(_design: &dyn design::Design) -> Style {
    Style::default()
}

/// A [`Chart`] with thin square border and no background.
pub fn bordered(design: &dyn design::Design) -> Style {
    Style {
        background: None,
        border: crate::core::Border {
            width: 1.0,
            radius: 0.0.into(),
            color: design.divider_color().resolve(
                design.background_color(),
                design.text_pair(),
                &design.palette_seed(),
                None,
            ),
        },
    }
}

/// A [`Chart`] with a solid background color and thin rounded border.
pub fn filled(design: &dyn design::Design) -> Style {
    Style {
        background: Some(design.background_color()),
        border: crate::core::Border {
            width: 1.0,
            radius: 5.0.into(),
            color: design.divider_color().resolve(
                design.background_color(),
                design.text_pair(),
                &design.palette_seed(),
                None,
            ),
        },
    }
}
