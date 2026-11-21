//! Interactive x-domain selection. Applications can share a selection across charts.
use crate::core::{Color, Event, Length, Point, Rectangle, Size, mouse, touch};
use crate::widget::canvas::{Frame, Geometry, Path, Stroke};
use crate::widget::{self, canvas};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Selection {
    domain: [f64; 2],
    bounds: [f64; 2],
}

impl Selection {
    pub fn new(domain: [f64; 2], bounds: [f64; 2]) -> Option<Self> {
        let [lo, hi] = domain;
        (domain.into_iter().chain(bounds).all(f64::is_finite)
            && (hi - lo).is_finite()
            && lo <= bounds[0]
            && bounds[0] < bounds[1]
            && bounds[1] <= hi)
            .then_some(Self { domain, bounds })
    }

    pub fn bounds(self) -> [f64; 2] {
        self.bounds
    }

    /// Use the same selection for a detail chart and any linked charts.
    pub fn apply(self, data: &mut crate::Data) {
        for axis in [data.primary.x_axis_mut(), data.secondary.x_axis_mut()]
            .into_iter()
            .flatten()
        {
            axis.lower_bound = Some(self.bounds[0]);
            axis.upper_bound = Some(self.bounds[1]);
        }
        data.invalidate();
    }

    fn moved(self, part: Part, delta: f64, min_span: f64) -> Self {
        let [a, b] = self.bounds;
        let [lo, hi] = self.domain;
        let bounds = match part {
            Part::Left => [(a + delta).clamp(lo, (b - min_span).max(lo)), b],
            Part::Right => [a, (b + delta).clamp((a + min_span).min(hi), hi)],
            Part::Window => {
                let shift = delta.clamp(lo - a, hi - b);
                [a + shift, b + shift]
            }
        };
        Self { bounds, ..self }
    }
}

#[derive(Clone, Copy)]
enum Part {
    Left,
    Right,
    Window,
}
#[derive(Clone, Copy)]
struct Drag {
    original: Selection,
    part: Part,
    start: f64,
    finger: Option<touch::Finger>,
}
#[derive(Default)]
pub struct State {
    drag: Option<Drag>,
}

pub struct Brush<'a, M> {
    selection: Selection,
    overview: &'a [[f64; 2]],
    on_change: Box<dyn Fn(Selection) -> M + 'a>,
    color: Color,
}

/// A two-handle brush with a full-domain overview. Drag a handle to resize,
/// drag the selected interval to pan, or click outside to move the nearest handle.
pub fn brush<'a, M, T>(
    selection: Selection,
    overview: &'a [[f64; 2]],
    color: Color,
    on_change: impl Fn(Selection) -> M + 'a,
) -> crate::widget::Canvas<Brush<'a, M>, M, T> {
    crate::widget::Canvas::new(Brush {
        selection,
        overview,
        on_change: Box::new(on_change),
        color,
    })
    .width(Length::Fill)
    .height(38)
}

impl<M, T> canvas::Program<M, T> for Brush<'_, M> {
    type State = State;
    fn update(
        &self,
        state: &mut State,
        event: &Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<widget::Action<M>> {
        let (phase, pos, finger) = match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => (0, cursor.position()?, None),
            Event::Mouse(mouse::Event::CursorMoved { position }) => (1, *position, None),
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                (2, cursor.position().unwrap_or(Point::ORIGIN), None)
            }
            Event::Touch(touch::Event::FingerPressed { id, position, .. }) => (0, *position, Some(*id)),
            Event::Touch(touch::Event::FingerMoved { id, position, .. }) => (1, *position, Some(*id)),
            Event::Touch(
                touch::Event::FingerLifted { id, position, .. } | touch::Event::FingerLost { id, position, .. },
            ) => (2, *position, Some(*id)),
            _ => return None,
        };
        if bounds.width <= 0. {
            return None;
        }
        let [lo, hi] = self.selection.domain;
        let unit = (hi - lo) / f64::from(bounds.width);
        let x = (lo + f64::from(pos.x - bounds.x) * unit).clamp(lo, hi);
        if phase == 0 {
            if !bounds.contains(pos) || state.drag.is_some() {
                return None;
            }
            let [a, b] = self.selection.bounds;
            let distance = (x - a).abs().min((x - b).abs());
            let part = if x > a && x < b && distance > 7. * unit {
                Part::Window
            } else if (x - a).abs() <= (x - b).abs() {
                Part::Left
            } else {
                Part::Right
            };
            let start = if distance > 7. * unit {
                match part {
                    Part::Left => a,
                    Part::Right => b,
                    Part::Window => x,
                }
            } else {
                x
            };
            state.drag = Some(Drag {
                original: self.selection,
                part,
                start,
                finger,
            });
        }
        let drag = state.drag.filter(|d| d.finger == finger)?;
        if phase == 2 {
            state.drag = None;
            return Some(widget::Action::capture());
        }
        let minimum = (6. * unit).min(drag.original.bounds[1] - drag.original.bounds[0]);
        let value = drag.original.moved(drag.part, x - drag.start, minimum);
        Some(widget::Action::publish((self.on_change)(value)).and_capture())
    }
    fn draw(
        &self,
        _: &State,
        renderer: &crate::widget::Renderer,
        _: &T,
        bounds: Rectangle,
        _: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let w = bounds.width;
        let h = bounds.height;
        let [lo, hi] = self.selection.domain;
        let px = |v: f64| ((v - lo) / (hi - lo)) as f32 * w;
        frame.fill_rectangle(Point::ORIGIN, bounds.size(), self.color.scale_alpha(0.08));
        let finite = self.overview.iter().filter(|v| v.iter().all(|x| x.is_finite()));
        let (ymin, ymax) = finite.clone().fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), p| {
            (a.min(p[1]), b.max(p[1]))
        });
        if ymax >= ymin {
            let path = Path::new(|b| {
                let mut first = true;
                for &[x, y] in self.overview {
                    if !x.is_finite() || !y.is_finite() {
                        first = true;
                        continue;
                    }
                    let t = if ymax > ymin {
                        ((y - ymin) / (ymax - ymin)) as f32
                    } else {
                        0.5
                    };
                    let p = Point::new(px(x), h - 4. - (h - 8.) * t);
                    if first {
                        b.move_to(p);
                        first = false;
                    } else {
                        b.line_to(p);
                    }
                }
            });
            frame.stroke(
                &path,
                Stroke::default()
                    .with_width(1.5)
                    .with_color(self.color.scale_alpha(0.7)),
            );
        }
        let [a, b] = self.selection.bounds.map(px);
        frame.fill_rectangle(Point::new(a, 0.), Size::new(b - a, h), self.color.scale_alpha(0.22));
        for x in [a, b] {
            frame.fill_rectangle(
                Point::new((x - 1.5).clamp(0., (w - 3.).max(0.)), 1.),
                Size::new(3., (h - 2.).max(0.)),
                self.color,
            );
        }
        vec![frame.into_geometry()]
    }
    fn mouse_interaction(&self, state: &State, bounds: Rectangle, cursor: mouse::Cursor) -> mouse::Interaction {
        if state.drag.is_some() || cursor.position_over(bounds).is_some() {
            mouse::Interaction::ResizingHorizontally
        } else {
            mouse::Interaction::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pan_and_resize_stop_at_domain_and_opposite_handle() {
        let s = Selection::new([0., 10.], [2., 6.]).expect("selection");
        assert_eq!(s.moved(Part::Window, 100., 1.).bounds(), [6., 10.]);
        assert_eq!(s.moved(Part::Left, 100., 1.).bounds(), [5., 6.]);
        assert_eq!(s.moved(Part::Right, -100., 1.).bounds(), [2., 3.]);
        assert!(Selection::new([0., 1.], [1., 0.]).is_none());
    }
}
