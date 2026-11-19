//! Data labels share the chart's drawing plane. Marks remain in their own
//! canvas frames, while glyphs can use the breathing room around the plot.
use crate::core::{Color, Point, Rectangle, Size, Vector, alignment, text};

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
        let Size { width, height } = paragraph.min_bounds();
        let x = match paragraph.align_x() {
            text::Alignment::Center => width / 2.,
            text::Alignment::Right => width,
            _ => 0.,
        };
        let y = match paragraph.align_y() {
            alignment::Vertical::Top => 0.,
            alignment::Vertical::Center => height / 2.,
            alignment::Vertical::Bottom => height,
        };
        let rect = Rectangle {
            x: anchor.x - x,
            y: anchor.y - y,
            width,
            height,
        };
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
        for (bounds, color) in self.plates {
            if let Some(bounds) = bounds.intersection(&clip) {
                renderer.fill_quad(
                    crate::core::renderer::Quad {
                        bounds,
                        ..Default::default()
                    },
                    color,
                );
            }
        }
        if !self.outlines.is_empty() {
            renderer.with_layer(clip, |renderer| {
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
            });
        }
        for (paragraph, anchor, color) in self.labels {
            renderer.fill_paragraph(paragraph, anchor, color, clip);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
