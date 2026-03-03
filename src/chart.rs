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
    /// Absolute screen-space bounds of the plot area (stored during layout).
    plot_area_bounds: Option<Rectangle>,
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

        // Store plot area bounds from the plane (in plot-area-local coords)
        // and offset to screen space using the scene layout
        let state = tree.state.downcast_mut::<State>();
        let plot_area_state = scene_tree.children[6]
            .state
            .downcast_ref::<plot_area::State>();
        state.plot_area_bounds = plot_area_state.plane.as_ref().map(|plane| {
            // Find the plot area position within the scene layout
            // The plot area node in the scene is the one matching the plane bounds size
            let plot_area_offset = self.scene.plot_area_offset();

            Rectangle {
                x: self.padding.left + plot_area_offset.x + plane.bounds.x,
                y: self.padding.top + plot_area_offset.y + plane.bounds.y,
                width: plane.bounds.width,
                height: plane.bounds.height,
            }
        });

        // Position scene node with padding offset
        layout::Node::with_children(size, vec![
            scene_node.move_to(Point::new(self.padding.left, self.padding.top)),
        ])
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        _layout: Layout<'_>,
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

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if let Some(plot_bounds) = &state.plot_area_bounds
                    && cursor.is_over(*plot_bounds)
                {
                    state.is_pressed = true;
                    shell.capture_event();
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                if state.is_pressed {
                    state.is_pressed = false;
                    shell.capture_event();

                    let Some(plot_bounds) = state.plot_area_bounds else {
                        return;
                    };

                    let Some(cursor_pos) = cursor.position() else {
                        return;
                    };

                    if !plot_bounds.contains(cursor_pos) {
                        // Clicked outside plot area — report empty click
                        shell.publish(on_action(Action::Clicked(
                            crate::target::Target::Mark(usize::MAX),
                        )));
                        return;
                    }

                    // Translate cursor to plot-area-local coordinates
                    let local = crate::core::Point::new(
                        cursor_pos.x - plot_bounds.x,
                        cursor_pos.y - plot_bounds.y,
                    );

                    // Hit-test against bar rects in the tree
                    // Tree: chart[0] → scene[6] → plot_area children (per mark)
                    let scene_tree = &tree.children[0];
                    let plot_area_tree = &scene_tree.children[6];

                    // Walk each mark's series looking for a hit
                    let bars_tag = tree::Tag::of::<plot_area::bars::State>();

                    for (mark_idx, series_tree) in
                        plot_area_tree.children.iter().enumerate()
                    {
                        // Check if this is a bars series by matching tree tag
                        if series_tree.tag == bars_tag {
                            let bars_state = series_tree
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
                        }
                    }

                    // No specific element hit — report empty click
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
