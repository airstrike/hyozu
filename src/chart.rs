pub mod guide;
pub mod legend;
pub mod plot_area;
mod scene;
pub mod title;
pub mod value;

use std::borrow::Cow;

use crate::core::widget::{Tree, tree};
use crate::core::{
    Element, Event, Layout, Length, Padding, Rectangle, Size, Widget, layout,
    mouse,
};
use crate::widget::Renderer;

use crate::{Action, Data, design};
use scene::Scene;

const DEFAULT_PADDING: Padding = Padding::new(10.0);

/// Chart widget for displaying data visualizations.
pub struct Chart<
    'a,
    Message,
    Design = crate::core::Theme,
    Theme = crate::core::Theme,
> where
    Design: design::Design + Clone,
{
    scene: Scene<'a, Message, Renderer>,
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
    is_pressed: bool,
}

/// Creates a chart widget from data.
///
/// The Theme type is inferred from context (e.g., the Element type in your view).
/// Call `.design()` to use a custom design type separate from your Theme.
pub fn chart<'a, Message, Theme>(
    data: &'a Data,
) -> Chart<'a, Message, Theme, Theme>
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
    pub fn style(
        mut self,
        style: impl Fn(&dyn design::Design) -> Style + 'a,
    ) -> Self {
        self.style = Box::new(style);
        self
    }
}

impl<'a, Message, Design, Theme> Widget<Message, Theme, Renderer>
    for Chart<'a, Message, Design, Theme>
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
        // Chart always has exactly one child (the scene)
        self.scene.diff(&mut tree.children[0]);
    }

    fn size(&self) -> Size<Length> {
        Size {
            width: self.width,
            height: self.height,
        }
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
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
        let scene_node = self.scene.layout(
            scene_tree,
            renderer,
            &layout::Limits::new(Size::ZERO, inner_size),
        );

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

        let Some(on_action) = &self.on_action else {
            return;
        };

        let state = tree.state.downcast_mut::<State>();

        // Compute absolute plot area bounds from the layout tree.
        // layout = chart widget (absolute), child[0] = scene (padded),
        // scene's plot_area_offset gives the position within the scene.
        let chart_bounds = layout.bounds();
        let plot_area_offset = self.scene.plot_area_offset();
        let scene_tree = &tree.children[0];
        let plot_area_state = scene_tree.children[6]
            .state
            .downcast_ref::<plot_area::State>();
        let plot_bounds = match &plot_area_state.plane {
            Some(plane) => Rectangle {
                x: chart_bounds.x
                    + self.padding.left
                    + plot_area_offset.x
                    + plane.bounds.x,
                y: chart_bounds.y
                    + self.padding.top
                    + plot_area_offset.y
                    + plane.bounds.y,
                width: plane.bounds.width,
                height: plane.bounds.height,
            },
            None => return,
        };

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if cursor.is_over(plot_bounds) {
                    state.is_pressed = true;
                    shell.capture_event();
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                if state.is_pressed {
                    state.is_pressed = false;
                    shell.capture_event();

                    // Use position_in to get plot-area-local coordinates
                    let Some(local) = cursor.position_in(plot_bounds) else {
                        // Released outside plot area — deselect
                        shell.publish(on_action(Action::Clicked(
                            crate::target::Target::Mark(usize::MAX),
                        )));
                        return;
                    };

                    let local = crate::core::Point::new(local.x, local.y);

                    // Hit-test against bar/pie elements in the tree
                    let plot_area_tree = &scene_tree.children[6];
                    let bars_tag = tree::Tag::of::<plot_area::bars::State>();
                    let pie_tag = tree::Tag::of::<plot_area::pie::State>();

                    for (mark_idx, mark_tree) in
                        plot_area_tree.children.iter().enumerate()
                    {
                        if mark_tree.tag == bars_tag {
                            let bars_state = mark_tree
                                .state
                                .downcast_ref::<plot_area::bars::State>(
                            );

                            for (series_idx, rects) in
                                bars_state.series_rects.iter().enumerate()
                            {
                                for (bar_idx, rect) in rects.iter().enumerate()
                                {
                                    if rect.contains(local) {
                                        shell.publish(on_action(
                                            Action::Clicked(
                                                crate::target::Target::Entry {
                                                    mark: mark_idx,
                                                    series: series_idx,
                                                    index: bar_idx,
                                                },
                                            ),
                                        ));
                                        return;
                                    }
                                }
                            }
                        } else if mark_tree.tag == pie_tag {
                            let pie_state = mark_tree
                                .state
                                .downcast_ref::<plot_area::pie::State>(
                            );

                            let (cx, cy) = pie_state.center;
                            let dx = local.x - cx;
                            let dy = local.y - cy;
                            let dist = (dx * dx + dy * dy).sqrt();

                            if dist >= pie_state.inner_radius
                                && dist <= pie_state.outer_radius
                            {
                                // Compute angle (atan2 gives -PI..PI, matching our -PI/2 start)
                                let mut angle = dy.atan2(dx);
                                // Normalize: our slices start at -PI/2 and go to ~3PI/2
                                // atan2 returns -PI..PI, so angles in top-left quadrant
                                // may need adjustment
                                let first_start = pie_state
                                    .slice_angles
                                    .first()
                                    .map(|(s, _)| *s)
                                    .unwrap_or(0.0);
                                if angle < first_start {
                                    angle += std::f32::consts::TAU;
                                }

                                for (slice_idx, (start_angle, end_angle)) in
                                    pie_state.slice_angles.iter().enumerate()
                                {
                                    if angle >= *start_angle
                                        && angle <= *end_angle
                                    {
                                        shell.publish(on_action(
                                            Action::Clicked(
                                                crate::target::Target::Entry {
                                                    mark: mark_idx,
                                                    series: 0,
                                                    index: slice_idx,
                                                },
                                            ),
                                        ));
                                        return;
                                    }
                                }
                            }
                        }
                    }

                    // No element hit — report empty click
                    shell.publish(on_action(Action::Clicked(
                        crate::target::Target::Mark(usize::MAX),
                    )));
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

        self.scene.draw(
            &tree.children[0],
            renderer,
            design,
            defaults,
            scene_layout,
            cursor,
            viewport,
        );
    }
}

impl<'a, Message, Design, Theme> From<Chart<'a, Message, Design, Theme>>
    for Element<'a, Message, Theme, Renderer>
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
pub fn draw_background(
    renderer: &mut Renderer,
    style: &Style,
    bounds: Rectangle,
) {
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
                None,
            ),
        },
    }
}
