use crate::core::widget::tree;
use crate::core::{Element, Layout, Length, Rectangle, Size, Widget, layout, mouse};
use crate::design;
use crate::widget::canvas::{Frame, Path, Stroke};
use crate::widget::renderer::geometry;

/// Which points to highlight.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Highlight {
    First,
    Last,
    Min,
    Max,
}

/// Rendering style: line chart or bar chart.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Kind {
    #[default]
    Line,
    Bar,
}

/// A single highlight rule: which point(s) and what color.
#[derive(Debug, Clone, Copy)]
struct HighlightRule {
    which: Highlight,
    color: crate::core::Color,
}

/// A minimal inline chart for embedding in table cells, dashboards, etc.
///
/// Supports line and bar rendering with highlighted points of interest.
pub struct Sparkline<'a, Message, Theme = crate::core::Theme, Renderer = crate::widget::Renderer> {
    values: &'a [f64],
    width: Length,
    height: Length,
    color: Option<crate::core::Color>,
    stroke_width: f32,
    kind: Kind,
    highlights: Vec<HighlightRule>,
    _marker: std::marker::PhantomData<(Message, Theme, Renderer)>,
}

/// Creates a sparkline widget from a slice of values.
pub fn sparkline<'a, Message, Theme>(values: &'a [f64]) -> Sparkline<'a, Message, Theme> {
    Sparkline {
        values,
        width: Length::Fill,
        height: Length::Fill,
        color: None,
        stroke_width: 1.5,
        kind: Kind::default(),
        highlights: Vec::new(),
        _marker: std::marker::PhantomData,
    }
}

impl<'a, Message, Theme, Renderer> Sparkline<'a, Message, Theme, Renderer> {
    /// Sets the width of the sparkline.
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// Sets the height of the sparkline.
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }

    /// Sets the base color. If not set, uses the first data color from the design.
    pub fn color(mut self, color: impl Into<crate::core::Color>) -> Self {
        self.color = Some(color.into());
        self
    }

    /// Sets the stroke width (line mode) or gap between bars (bar mode).
    pub fn stroke_width(mut self, width: f32) -> Self {
        self.stroke_width = width;
        self
    }

    /// Switches to bar rendering.
    pub fn bars(mut self) -> Self {
        self.kind = Kind::Bar;
        self
    }

    /// Highlights a point of interest with a specific color.
    ///
    /// In line mode, highlights are drawn as filled circles.
    /// In bar mode, the highlighted bar is drawn in the given color.
    ///
    /// Multiple highlights can be added; call this method repeatedly.
    pub fn highlight(mut self, which: Highlight, color: impl Into<crate::core::Color>) -> Self {
        self.highlights.push(HighlightRule {
            which,
            color: color.into(),
        });
        self
    }
}

// --- Internals ---

/// Precomputed data range and highlight indices for rendering.
struct Computed {
    min_y: f64,
    #[allow(dead_code)]
    max_y: f64,
    y_range: f64,
    min_idx: usize,
    max_idx: usize,
}

impl Computed {
    fn from_values(values: &[f64]) -> Self {
        let mut min_y = f64::INFINITY;
        let mut max_y = f64::NEG_INFINITY;
        let mut min_idx = 0;
        let mut max_idx = 0;
        for (i, &v) in values.iter().enumerate() {
            if v < min_y {
                min_y = v;
                min_idx = i;
            }
            if v > max_y {
                max_y = v;
                max_idx = i;
            }
        }
        let y_range = if (max_y - min_y).abs() < f64::EPSILON {
            1.0
        } else {
            max_y - min_y
        };
        Self {
            min_y,
            max_y,
            y_range,
            min_idx,
            max_idx,
        }
    }

    fn highlight_index(&self, which: Highlight, len: usize) -> usize {
        match which {
            Highlight::First => 0,
            Highlight::Last => len.saturating_sub(1),
            Highlight::Min => self.min_idx,
            Highlight::Max => self.max_idx,
        }
    }
}

impl<'a, Message, Theme, Renderer> Widget<Message, Theme, Renderer> for Sparkline<'a, Message, Theme, Renderer>
where
    Theme: design::Design + 'a,
    Renderer: geometry::Renderer,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::stateless()
    }

    fn state(&self) -> tree::State {
        tree::State::None
    }

    fn size(&self) -> Size<Length> {
        Size {
            width: self.width,
            height: self.height,
        }
    }

    fn layout(
        &mut self,
        _tree: &mut crate::core::widget::Tree,
        _renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        layout::Node::new(limits.resolve(self.width, self.height, Size::new(60.0, 20.0)))
    }

    fn update(
        &mut self,
        _tree: &mut crate::core::widget::Tree,
        _event: &crate::core::Event,
        _layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _renderer: &Renderer,
        _shell: &mut crate::core::Shell<'_, Message>,
        _viewport: &Rectangle,
    ) {
    }

    fn draw(
        &self,
        _tree: &crate::core::widget::Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        _defaults: &crate::core::renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        if self.values.len() < 2 {
            return;
        }

        let bounds = layout.bounds();
        if bounds.width <= 0.0 || bounds.height <= 0.0 {
            return;
        }

        let base_color = self.color.unwrap_or_else(|| theme.seed().primary);

        let comp = Computed::from_values(self.values);

        match self.kind {
            Kind::Line => self.draw_line(renderer, bounds, base_color, &comp),
            Kind::Bar => self.draw_bars(renderer, bounds, base_color, &comp),
        }
    }
}

impl<'a, Message, Theme, Renderer> Sparkline<'a, Message, Theme, Renderer>
where
    Renderer: geometry::Renderer,
{
    fn draw_line(&self, renderer: &mut Renderer, bounds: Rectangle, base_color: crate::core::Color, comp: &Computed) {
        let inset = self.stroke_width + 2.0; // room for stroke + dots
        let plot_w = bounds.width - inset * 2.0;
        let plot_h = bounds.height - inset * 2.0;
        let n = self.values.len() as f32;

        let px = |i: usize, v: f64| -> crate::core::Point {
            let x = inset + (i as f32 / (n - 1.0)) * plot_w;
            let y = inset + plot_h - ((v - comp.min_y) / comp.y_range) as f32 * plot_h;
            crate::core::Point::new(x, y)
        };

        let mut frame = Frame::new(renderer, bounds.size());

        // Draw line path
        let path = Path::new(|builder| {
            for (i, &v) in self.values.iter().enumerate() {
                let p = px(i, v);
                if i == 0 {
                    builder.move_to(p);
                } else {
                    builder.line_to(p);
                }
            }
        });

        frame.stroke(
            &path,
            Stroke::default().with_width(self.stroke_width).with_color(base_color),
        );

        // Draw highlight dots
        let dot_radius = self.stroke_width + 1.0;
        for rule in &self.highlights {
            let idx = comp.highlight_index(rule.which, self.values.len());
            let p = px(idx, self.values[idx]);
            let dot = Path::new(|builder| builder.circle(p, dot_radius));
            frame.fill(&dot, rule.color);
        }

        renderer.with_translation(crate::core::Vector::new(bounds.x, bounds.y), |renderer| {
            renderer.draw_geometry(frame.into_geometry());
        });
    }

    fn draw_bars(&self, renderer: &mut Renderer, bounds: Rectangle, base_color: crate::core::Color, comp: &Computed) {
        let n = self.values.len() as f32;
        let gap = 1.0_f32;
        let bar_w = ((bounds.width - gap * (n - 1.0)) / n).max(1.0);
        let inset_y = 1.0_f32;
        let plot_h = bounds.height - inset_y * 2.0;

        // Build a color lookup for highlighted indices
        let mut highlight_colors: Vec<Option<crate::core::Color>> = vec![None; self.values.len()];
        for rule in &self.highlights {
            let idx = comp.highlight_index(rule.which, self.values.len());
            highlight_colors[idx] = Some(rule.color);
        }

        let mut frame = Frame::new(renderer, bounds.size());

        for (i, &v) in self.values.iter().enumerate() {
            let bar_h = ((v - comp.min_y) / comp.y_range) as f32 * plot_h;
            let bar_h = bar_h.max(1.0); // minimum 1px so flat values are visible
            let x = (bar_w + gap) * i as f32;
            let y = inset_y + plot_h - bar_h;

            let color = highlight_colors[i].unwrap_or(base_color);

            let rect = Path::new(|builder| {
                builder.rectangle(crate::core::Point::new(x, y), Size::new(bar_w, bar_h));
            });
            frame.fill(&rect, color);
        }

        renderer.with_translation(crate::core::Vector::new(bounds.x, bounds.y), |renderer| {
            renderer.draw_geometry(frame.into_geometry());
        });
    }
}

impl<'a, Message, Theme, Renderer> From<Sparkline<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: design::Design + 'a,
    Renderer: geometry::Renderer + 'a,
{
    fn from(sparkline: Sparkline<'a, Message, Theme, Renderer>) -> Self {
        Element::new(sparkline)
    }
}
