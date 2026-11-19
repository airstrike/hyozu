//! Data labels share the chart's drawing plane. Marks remain in their own
//! canvas frames, while glyphs can use the breathing room around the plot.
use crate::core::{Color, Point, Rectangle, Vector, text};

#[derive(Clone, Default)]
pub(crate) struct Space {
    outer: Rectangle,
    reserved: Vec<Rectangle>,
}

impl Space {
    pub(crate) fn new(outer: Rectangle) -> Self {
        Self {
            outer,
            reserved: Vec::new(),
        }
    }

    pub(crate) fn reserve(&mut self, bounds: Rectangle) {
        if bounds.width > 0. && bounds.height > 0. {
            self.reserved.push(bounds);
        }
    }

    fn clamp(&self, mut rect: Rectangle) -> Rectangle {
        rect.x = rect.x.clamp(
            self.outer.x,
            (self.outer.x + self.outer.width - rect.width).max(self.outer.x),
        );
        rect.y = rect.y.clamp(
            self.outer.y,
            (self.outer.y + self.outer.height - rect.height).max(self.outer.y),
        );
        rect
    }

    /// Keep the intended anchor whenever possible. Only labels that collide
    /// with chart chrome move to the closest available side of that obstacle.
    pub(crate) fn place(&self, rect: Rectangle) -> Rectangle {
        let mut placed = self.clamp(rect);
        for reserved in &self.reserved {
            if placed.intersection(reserved).is_none() {
                continue;
            }
            let candidates = [
                Rectangle {
                    y: reserved.y + reserved.height + 1.,
                    ..placed
                },
                Rectangle {
                    y: reserved.y - placed.height - 1.,
                    ..placed
                },
                Rectangle {
                    x: reserved.x + reserved.width + 1.,
                    ..placed
                },
                Rectangle {
                    x: reserved.x - placed.width - 1.,
                    ..placed
                },
            ];
            if let Some(best) = candidates
                .into_iter()
                .map(|r| self.clamp(r))
                .filter(|r| self.reserved.iter().all(|b| r.intersection(b).is_none()))
                .min_by(|a, b| {
                    let distance = |r: &Rectangle| (r.x - rect.x).powi(2) + (r.y - rect.y).powi(2);
                    distance(a).total_cmp(&distance(b))
                })
            {
                placed = best;
            }
        }
        placed
    }
}

pub(crate) struct Layer<'p, P: text::Paragraph> {
    space: Space,
    plates: Vec<(Rectangle, Color)>,
    outlines: Vec<(Rectangle, Color, Color)>,
    labels: Vec<(&'p P, Point, Color)>,
}

impl<'p, P: text::Paragraph> Layer<'p, P> {
    pub(crate) fn new(space: Space) -> Self {
        Self {
            space,
            labels: Vec::new(),
            plates: Vec::new(),
            outlines: Vec::new(),
        }
    }
    pub(crate) fn push(&mut self, paragraph: &'p P, anchor: Point, color: Color) {
        self.push_backed(paragraph, anchor, color, None);
    }
    pub(crate) fn push_backed(
        &mut self,
        paragraph: &'p P,
        anchor: Point,
        color: Color,
        background: Option<(Color, f32)>,
    ) {
        // fill_paragraph receives the glyph box's top-left corner. Mark
        // renderers have already converted their data anchor to this origin.
        let rect = Rectangle::new(anchor, paragraph.min_bounds());
        let rect = background.map_or(rect, |(_, padding)| rect.expand(padding));
        let placed = self.space.place(rect);
        if let Some((fill, _)) = background {
            self.plates.push((placed, fill));
        }
        self.labels.push((
            paragraph,
            anchor + Vector::new(placed.x - rect.x, placed.y - rect.y),
            color,
        ));
    }
    pub(crate) fn outline(&mut self, rect: Rectangle, outer: Color, inner: Color) {
        self.outlines.push((self.space.place(rect), outer, inner));
    }
    pub(crate) fn background(&mut self, rect: Rectangle, color: Color) {
        self.plates.push((self.space.place(rect), color));
    }
    pub(crate) fn draw<R: text::Renderer<Paragraph = P>>(self, renderer: &mut R, viewport: &Rectangle) {
        let Some(clip) = self.space.outer.intersection(viewport) else {
            return;
        };
        if self.plates.is_empty() && self.outlines.is_empty() && self.labels.is_empty() {
            return;
        }
        // Keep plates and glyphs in the same renderer layer above the plot.
        // A separate child layer for plates would composite over the glyphs.
        renderer.with_layer(clip, |renderer| {
            for (bounds, color) in self.plates {
                renderer.fill_quad(
                    crate::core::renderer::Quad {
                        bounds,
                        ..Default::default()
                    },
                    color,
                );
            }
            for (bounds, outer, inner) in self.outlines {
                for (bounds, color) in [(bounds.expand(1.), outer), (bounds, inner)] {
                    renderer.fill_quad(
                        crate::core::renderer::Quad {
                            bounds,
                            border: crate::core::Border {
                                color,
                                width: 1.,
                                ..Default::default()
                            },
                            ..Default::default()
                        },
                        Color::TRANSPARENT,
                    );
                }
            }
            for (paragraph, anchor, color) in self.labels {
                renderer.fill_paragraph(paragraph, anchor, color, clip);
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn backed_labels_paint_visible_glyphs_inside_their_plate() {
        use crate::core::renderer::{Headless, Settings};
        use crate::core::{Font, Size};
        type Renderer = crate::widget::Renderer;
        type Paragraph = <Renderer as text::Renderer>::Paragraph;
        let mut renderer = iced::futures::executor::block_on(Renderer::new(Settings::default(), Some("tiny-skia")))
            .expect("software renderer");
        let mut paragraph = text::paragraph::Plain::<Paragraph>::default();
        paragraph.update(text::Text {
            content: "Harbor",
            bounds: Size::INFINITE,
            size: 22.into(),
            font: Font::DEFAULT,
            line_height: Default::default(),
            align_x: text::Alignment::Left,
            align_y: crate::core::alignment::Vertical::Top,
            shaping: text::Shaping::Basic,
            wrapping: text::Wrapping::None,
            hint_factor: None,
            ellipsis: text::Ellipsis::None,
            letter_spacing: Default::default(),
            font_features: Default::default(),
            font_variations: Default::default(),
            weight: None,
        });
        let clip = Rectangle::with_size(Size::new(180., 70.));
        let mut layer = Layer::new(Space::new(clip));
        layer.push_backed(
            paragraph.raw(),
            Point::new(40., 15.),
            Color::from_rgb(1., 0., 0.),
            Some((Color::from_rgb(0., 0., 1.), 4.)),
        );
        let plate = layer.plates[0].0;
        layer.draw(&mut renderer, &clip);
        let pixels = renderer.screenshot(Size::new(180, 70), 1., Color::WHITE);
        let mut red = 0;
        let mut blue = 0;
        for (i, rgba) in pixels.as_chunks::<4>().0.iter().enumerate() {
            if rgba[0] > 160 && rgba[1] < 90 && rgba[2] < 90 {
                red += 1;
                assert!(plate.contains(Point::new((i % 180) as f32, (i / 180) as f32)));
            }
            if rgba[0] < 30 && rgba[1] < 30 && rgba[2] > 220 {
                blue += 1;
            }
        }
        assert!(
            red > 20 && blue > 100,
            "visible glyph pixels {red}, background pixels {blue}"
        );
    }

    #[test]
    fn four_edges_stay_within_card_and_clear_title() {
        let mut space = Space::new(Rectangle {
            x: 100.,
            y: 20.,
            width: 150.,
            height: 80.,
        });
        space.reserve(Rectangle {
            x: 145.,
            y: 21.,
            width: 45.,
            height: 10.,
        });
        for (x, y) in [(90., 50.), (245., 60.), (150., 13.), (170., 97.)] {
            let r = space.place(Rectangle {
                x,
                y,
                width: 20.,
                height: 12.,
            });
            assert!(r.x >= 100. && r.x + r.width <= 250. && r.y >= 20. && r.y + r.height <= 100.);
            assert!(r.intersection(&space.reserved[0]).is_none());
        }
    }
}
