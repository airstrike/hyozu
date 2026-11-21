//! Renders [`crate::mark::Annotation`]: subject, connector and note.
//!
//! Layout runs in two steps. [`Annotation::layout`] shapes the note text,
//! which needs only the renderer. Once every other series has laid out,
//! [`Annotation::place`] resolves the target to a pixel anchor (through the
//! domain's per-axis mappings, or through the rendered geometry of the
//! targeted mark) and computes a [`Placement`]: the subject, the connector
//! route and the note box. `draw` only paints the cached placement.
use super::{Domain, Series, to_pixel};
use crate::core::text::{self, paragraph};
use crate::core::widget::{Tree, tree};
use crate::core::{Point, Rectangle, Size, Vector};
use crate::data::Datum;
use crate::mark::annotation::{Connector, End, Subject, Target};
use crate::mark::text::TextAlign;
use crate::widget::canvas::{Frame, Path, Stroke};
use crate::widget::renderer::geometry;

/// Body text size in pixels.
const BODY_SIZE: f32 = 11.0;
/// Title text size in pixels.
const TITLE_SIZE: f32 = 12.0;
/// Gap between the title and the body.
const LINE_GAP: f32 = 2.0;
/// Note padding with a background plate.
const PLATE_PADDING: f32 = 6.0;
/// Note padding without a background, so the connector doesn't touch text.
const BARE_PADDING: f32 = 3.0;
/// Arrowhead length along the connector.
const ARROW_LENGTH: f32 = 7.0;
/// Arrowhead half-width across the connector.
const ARROW_HALF_WIDTH: f32 = 3.5;
/// Radius of the dot end.
const DOT_RADIUS: f32 = 2.5;
/// Tolerance for an anchor sitting exactly on the data rectangle's edge.
const EDGE_TOLERANCE: f32 = 0.5;

/// State for an annotation: the shaped note text and its placement.
pub struct State<P>
where
    P: text::Paragraph,
{
    /// The note title, shaped in `layout`. Empty without a title.
    pub title: paragraph::Plain<P>,
    /// The note body, shaped in `layout`. Empty without a body.
    pub body: paragraph::Plain<P>,
    /// Where everything lands, or `None` when the target is hidden.
    pub placement: Option<Placement>,
}

/// The resolved pixel geometry of one annotation, in plot-local
/// coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placement {
    /// The target's pixel position.
    pub anchor: Point,
    /// The highlight drawn at the anchor.
    pub subject: Subject,
    /// The connector from the subject edge to the note, if any.
    pub route: Option<Route>,
    /// The note box, padding included.
    pub note: Rectangle,
    /// Top-left of the title paragraph.
    pub title: Point,
    /// Top-left of the body paragraph.
    pub body: Point,
    /// The data rectangle the subject is clipped to.
    pub content: Rectangle,
}

/// The path a connector takes. Every route starts on the subject edge and
/// ends where it meets the note.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Route {
    /// A straight segment.
    Line { from: Point, to: Point },
    /// Two segments meeting at `corner`.
    Elbow { from: Point, corner: Point, to: Point },
    /// A quadratic Bézier through `control`.
    Curve { from: Point, control: Point, to: Point },
}

/// The shaped note's text extents, which [`Placement::new`] lays out.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Text {
    /// Title extent, zero without a title.
    pub title: Size,
    /// Body extent, zero without a body.
    pub body: Size,
}

/// An annotation series.
pub struct Annotation<'a, Message, Renderer>
where
    Message: 'a,
    Renderer: text::Renderer + geometry::Renderer,
{
    pub(super) data: &'a crate::mark::Annotation,
    _marker: std::marker::PhantomData<(Message, Renderer)>,
}

impl Route {
    /// Where the route leaves the subject.
    pub fn from(&self) -> Point {
        match *self {
            Route::Line { from, .. } | Route::Elbow { from, .. } | Route::Curve { from, .. } => from,
        }
    }

    /// Where the route meets the note.
    pub fn to(&self) -> Point {
        match *self {
            Route::Line { to, .. } | Route::Elbow { to, .. } | Route::Curve { to, .. } => to,
        }
    }

    /// The point the first leg heads toward: the next vertex, or the
    /// control point of a curve.
    fn heading(&self) -> Point {
        match *self {
            Route::Line { to, .. } => to,
            Route::Elbow { corner, .. } => corner,
            Route::Curve { control, .. } => control,
        }
    }
}

impl Placement {
    /// Lays out an annotation at `anchor`, or returns `None` when the anchor
    /// is hidden: non-finite, or outside the `content` data rectangle.
    ///
    /// The note box sits `offset` pixels from the anchor, grows away from
    /// it, and is nudged back inside `bounds` (the plot area) when it would
    /// leave it; the connector follows the nudged box.
    pub fn new(
        data: &crate::mark::Annotation,
        anchor: Point,
        text: Text,
        content: Rectangle,
        bounds: Rectangle,
    ) -> Option<Self> {
        if !anchor.x.is_finite() || !anchor.y.is_finite() || !content.expand(EDGE_TOLERANCE).contains(anchor) {
            return None;
        }

        let padding = if data.note.background.is_some() {
            PLATE_PADDING
        } else {
            BARE_PADDING
        };
        let has_title = text.title.width > 0.0;
        let has_body = text.body.width > 0.0;
        let gap = if has_title && has_body { LINE_GAP } else { 0.0 };
        let inner = Size::new(
            text.title.width.max(text.body.width),
            text.title.height + gap + text.body.height,
        );
        let size = Size::new(inner.width + 2.0 * padding, inner.height + 2.0 * padding);

        let (dx, dy) = data.offset;
        let attach = Point::new(anchor.x + dx, anchor.y + dy);
        let x = side(attach.x, dx, size.width);
        let y = side(attach.y, dy, size.height);
        let desired = Rectangle::new(Point::new(x, y), size);
        let note = clamp(desired, bounds);
        let attach = attach + Vector::new(note.x - desired.x, note.y - desired.y);

        let align = |width: f32| match data.note.align {
            TextAlign::Left => 0.0,
            TextAlign::Center => (inner.width - width) / 2.0,
            TextAlign::Right => inner.width - width,
        };
        let title = Point::new(note.x + padding + align(text.title.width), note.y + padding);
        let body = Point::new(
            note.x + padding + align(text.body.width),
            note.y + padding + text.title.height + gap,
        );

        let route = if note.contains(anchor) {
            None
        } else {
            route(data.connector, data.subject, anchor, attach)
        };

        Some(Self {
            anchor,
            subject: data.subject,
            route,
            note,
            title,
            body,
            content,
        })
    }
}

impl<'a, Message, Renderer> Annotation<'a, Message, Renderer>
where
    Message: 'a,
    Renderer: text::Renderer + geometry::Renderer,
{
    /// Create a new annotation series borrowing data.
    pub fn new(data: &'a crate::mark::Annotation) -> Self {
        Self {
            data,
            _marker: std::marker::PhantomData,
        }
    }

    /// Returns the initial tree state.
    pub(super) fn state(&self) -> Tree {
        Tree {
            tag: tree::Tag::of::<State<Renderer::Paragraph>>(),
            state: tree::State::new(State::<Renderer::Paragraph> {
                title: paragraph::Plain::default(),
                body: paragraph::Plain::default(),
                placement: None,
            }),
            children: Vec::new(),
            ..Tree::empty()
        }
    }

    /// Reconcile the tree with the current annotation.
    pub(super) fn diff(&self, _tree: &mut Tree) {
        // No children to diff
    }

    /// Shapes the note text. Placement waits for [`Annotation::place`],
    /// since an entry target reads geometry the other series lay out.
    pub fn layout(&self, tree: &mut Tree, renderer: &Renderer, design: Option<&dyn crate::design::Design>) {
        let state = tree.state.downcast_mut::<State<Renderer::Paragraph>>();
        let font = design
            .map(|d| d.data_label_text().resolved_font(renderer.font()))
            .unwrap_or_else(|| renderer.font());
        let mut title_font = font;
        title_font.weight = crate::core::font::Weight::Bold;
        let note = &self.data.note;
        shape(
            &mut state.title,
            note.title.as_deref(),
            TITLE_SIZE,
            title_font,
            note.wrap,
            note.align,
            renderer,
        );
        shape(
            &mut state.body,
            note.body.as_deref(),
            BODY_SIZE,
            font,
            note.wrap,
            note.align,
            renderer,
        );
        state.placement = None;
    }

    /// Resolves the target and caches the [`Placement`].
    ///
    /// `index` is this annotation's position in `series`, which shares
    /// indices with `children`. `area` is the range of `series` that forms
    /// this annotation's plotting area (primary or secondary), the space an
    /// entry target's mark index counts in.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn place(
        &self,
        children: &mut [Tree],
        index: usize,
        series: &[Series<'_, Message, Renderer>],
        area: std::ops::Range<usize>,
        domain: &Domain,
        content: Rectangle,
        bounds: Rectangle,
    ) {
        let anchor = match self.data.target {
            Target::Coordinates(datum) => Some(to_pixel(domain, content, datum)),
            Target::Entry {
                mark,
                series: series_index,
                index: datum,
            } => {
                let target = area.start + mark;
                (area.contains(&target) && target != index)
                    .then(|| entry_anchor(&series[target], &children[target], series_index, datum, domain, content))
                    .flatten()
            }
        };

        let state = children[index].state.downcast_mut::<State<Renderer::Paragraph>>();
        let text = Text {
            title: extent(&state.title, self.data.note.title.is_some()),
            body: extent(&state.body, self.data.note.body.is_some()),
        };
        state.placement = anchor.and_then(|anchor| Placement::new(self.data, anchor, text, content, bounds));
    }

    /// Paints the cached placement. Called after every other series and
    /// their data labels, so annotations sit on top.
    pub fn draw<Theme>(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        layout: crate::core::Layout,
        viewport: &Rectangle,
    ) where
        Theme: crate::design::Design + ?Sized,
    {
        let state = tree.state.downcast_ref::<State<Renderer::Paragraph>>();
        let Some(placement) = state.placement else {
            return;
        };

        let background = theme.background_color();
        let text_pair = theme.text_pair();
        let seed = theme.seed();
        let text_color = theme.text_color().resolve(background, text_pair, &seed, None);
        let color = self
            .data
            .color
            .map_or(text_color, |c| c.resolve(background, text_pair, &seed, None));

        let bounds = layout.bounds();
        let translation = Vector::new(bounds.x, bounds.y);

        // The subject stays inside the data rectangle.
        if let Some(clip) = (placement.content + translation).intersection(viewport) {
            let mut frame = Frame::new(renderer, bounds.size());
            draw_subject(&mut frame, &placement, color);
            let geometry = frame.into_geometry();
            renderer.with_layer(clip, |renderer| {
                renderer.with_translation(translation, |renderer| renderer.draw_geometry(geometry));
            });
        }

        // The connector and note may use the plot area's breathing room.
        let Some(clip) = bounds.intersection(viewport) else {
            return;
        };
        let mut frame = Frame::new(renderer, bounds.size());
        if let Some(route) = placement.route {
            draw_route(&mut frame, route, self.data.end, color);
        }
        if let Some(fill) = self.data.note.background {
            let fill = fill.resolve(background, text_pair, &seed, None);
            let plate = Path::rounded_rectangle(placement.note.position(), placement.note.size(), 4.0.into());
            frame.fill(&plate, fill);
            frame.stroke(
                &plate,
                Stroke::default().with_width(1.0).with_color(color.scale_alpha(0.35)),
            );
        }
        let geometry = frame.into_geometry();
        renderer.with_layer(clip, |renderer| {
            renderer.with_translation(translation, |renderer| renderer.draw_geometry(geometry));
            if self.data.note.title.is_some() {
                renderer.fill_paragraph(state.title.raw(), placement.title + translation, color, clip);
            }
            if self.data.note.body.is_some() {
                renderer.fill_paragraph(state.body.raw(), placement.body + translation, text_color, clip);
            }
        });
    }
}

/// Pixel position of datum `index` in `series_index` of `target`, read from
/// the geometry that series laid out. Bars anchor at their value end.
fn entry_anchor<Message, Renderer>(
    target: &Series<'_, Message, Renderer>,
    tree: &Tree,
    series_index: usize,
    index: usize,
    domain: &Domain,
    content: Rectangle,
) -> Option<Point>
where
    Renderer: text::Renderer + geometry::Renderer,
{
    match target {
        Series::Line(_) if series_index == 0 => tree
            .state
            .downcast_ref::<super::line::State<Renderer::Paragraph>>()
            .pixel_points
            .get(index)
            .copied(),
        Series::Xy(_) if series_index == 0 => tree
            .state
            .downcast_ref::<super::xy::State>()
            .pixel_points
            .get(index)
            .copied(),
        Series::Area(_) => tree
            .state
            .downcast_ref::<super::area::State<Renderer::Paragraph>>()
            .series_points
            .get(series_index)?
            .get(index)
            .copied(),
        Series::Bars(bars) => {
            let rect = *tree
                .state
                .downcast_ref::<super::bars::State<Renderer::Paragraph>>()
                .series_rects
                .get(series_index)?
                .get(index)?;
            let zero = to_pixel(domain, content, Datum::ORIGIN);
            Some(value_end(rect, zero, bars.data.direction))
        }
        Series::Tick(tick) if series_index == 0 => {
            let point = tick.data.points.get(index)?;
            let datum = match tick.data.orientation {
                crate::mark::tick::Orientation::Horizontal => *point,
                crate::mark::tick::Orientation::Vertical => Datum::new(point.y, point.x),
            };
            Some(to_pixel(domain, content, datum))
        }
        _ => None,
    }
}

/// The midpoint of the bar edge farther from the zero line: the end a
/// value grows toward, whatever the axis direction.
fn value_end(rect: Rectangle, zero: Point, direction: crate::mark::bar::Direction) -> Point {
    let center = rect.center();
    match direction {
        crate::mark::bar::Direction::Vertical => {
            let (top, bottom) = (rect.y, rect.y + rect.height);
            let y = if (top - zero.y).abs() >= (bottom - zero.y).abs() {
                top
            } else {
                bottom
            };
            Point::new(center.x, y)
        }
        crate::mark::bar::Direction::Horizontal => {
            let (left, right) = (rect.x, rect.x + rect.width);
            let x = if (right - zero.x).abs() >= (left - zero.x).abs() {
                right
            } else {
                left
            };
            Point::new(x, center.y)
        }
    }
}

/// Start coordinate of a note box of `extent` whose attachment point is
/// `attach`, growing away from the subject along an offset of `delta`.
fn side(attach: f32, delta: f32, extent: f32) -> f32 {
    if delta > 0.0 {
        attach
    } else if delta < 0.0 {
        attach - extent
    } else {
        attach - extent / 2.0
    }
}

/// Moves `rect` inside `bounds`, pinning to the top-left when it can't fit.
fn clamp(rect: Rectangle, bounds: Rectangle) -> Rectangle {
    let x = rect.x.min(bounds.x + bounds.width - rect.width).max(bounds.x);
    let y = rect.y.min(bounds.y + bounds.height - rect.height).max(bounds.y);
    Rectangle::new(Point::new(x, y), rect.size())
}

/// Routes `connector` from the edge of `subject` at `anchor` to `attach`.
fn route(connector: Connector, subject: Subject, anchor: Point, attach: Point) -> Option<Route> {
    let route = match connector {
        Connector::None => return None,
        Connector::Line => Route::Line {
            from: edge(subject, anchor, attach),
            to: attach,
        },
        Connector::Elbow | Connector::Curve => {
            let bend = Point::new(attach.x, anchor.y);
            // A note straight above or below has no horizontal leg.
            if bend.distance(anchor) < 1.0 || bend.distance(attach) < 1.0 {
                Route::Line {
                    from: edge(subject, anchor, attach),
                    to: attach,
                }
            } else if connector == Connector::Elbow {
                Route::Elbow {
                    from: edge(subject, anchor, bend),
                    corner: bend,
                    to: attach,
                }
            } else {
                Route::Curve {
                    from: edge(subject, anchor, bend),
                    control: bend,
                    to: attach,
                }
            }
        }
    };
    // A note overlapping its subject needs no connector.
    (route.from().distance(route.to()) >= 1.0 && route.from().distance(route.heading()) > 0.0).then_some(route)
}

/// Where a ray from `anchor` toward `toward` leaves `subject`.
fn edge(subject: Subject, anchor: Point, toward: Point) -> Point {
    let d = toward - anchor;
    let length = (d.x * d.x + d.y * d.y).sqrt();
    if length == 0.0 {
        return anchor;
    }
    let u = Vector::new(d.x / length, d.y / length);
    let reach = match subject {
        Subject::Point | Subject::Crosshair | Subject::Threshold { .. } => 0.0,
        Subject::Circle { radius } => radius,
        Subject::Rect { width, height } => {
            let x = if u.x == 0.0 {
                f32::INFINITY
            } else {
                width / 2.0 / u.x.abs()
            };
            let y = if u.y == 0.0 {
                f32::INFINITY
            } else {
                height / 2.0 / u.y.abs()
            };
            x.min(y)
        }
    };
    let reach = reach.min(length);
    anchor + Vector::new(u.x * reach, u.y * reach)
}

/// Shapes `content` into `paragraph`, or clears it.
fn shape<P: text::Paragraph, R: text::Renderer<Paragraph = P>>(
    paragraph: &mut paragraph::Plain<P>,
    content: Option<&str>,
    size: f32,
    font: crate::core::Font,
    wrap: Option<f32>,
    align: TextAlign,
    renderer: &R,
) {
    let content = content.unwrap_or_default();
    let text = |bounds: Size, align_x: text::Alignment| text::Text {
        content,
        bounds,
        size: size.into(),
        line_height: text::LineHeight::default(),
        font,
        align_x,
        align_y: crate::core::alignment::Vertical::Top,
        shaping: text::Shaping::Advanced,
        wrapping: if wrap.is_some() {
            text::Wrapping::Word
        } else {
            text::Wrapping::None
        },
        ellipsis: text::Ellipsis::default(),
        hint_factor: renderer.hint_factor(),
        font_features: Vec::new(),
        font_variations: Vec::new(),
        letter_spacing: Default::default(),
        weight: None,
    };
    let bounds = Size::new(wrap.unwrap_or(f32::INFINITY), f32::INFINITY);
    let _ = paragraph.update(text(bounds, text::Alignment::Left));
    // Wrapped lines align within the paragraph's own width; a second pass
    // at that width applies the alignment line by line.
    if wrap.is_some() && align != TextAlign::Left {
        let width = paragraph.min_bounds().width;
        let align_x = match align {
            TextAlign::Left => text::Alignment::Left,
            TextAlign::Center => text::Alignment::Center,
            TextAlign::Right => text::Alignment::Right,
        };
        let _ = paragraph.update(text(Size::new(width, f32::INFINITY), align_x));
    }
}

/// The drawn extent of a shaped paragraph, zero when it holds no text.
fn extent<P: text::Paragraph>(paragraph: &paragraph::Plain<P>, present: bool) -> Size {
    if present { paragraph.min_bounds() } else { Size::ZERO }
}

fn draw_subject<Renderer: geometry::Renderer>(
    frame: &mut Frame<Renderer>,
    placement: &Placement,
    color: crate::core::Color,
) {
    let anchor = placement.anchor;
    let content = placement.content;
    let outline = Stroke::default().with_width(1.5).with_color(color);
    match placement.subject {
        Subject::Point => {}
        Subject::Circle { radius } => {
            let circle = Path::circle(anchor, radius);
            frame.fill(&circle, color.scale_alpha(0.15));
            frame.stroke(&circle, outline);
        }
        Subject::Rect { width, height } => {
            let rect = Path::rectangle(
                Point::new(anchor.x - width / 2.0, anchor.y - height / 2.0),
                Size::new(width, height),
            );
            frame.fill(&rect, color.scale_alpha(0.15));
            frame.stroke(&rect, outline);
        }
        Subject::Crosshair => {
            let hairlines = Path::new(|b| {
                b.move_to(Point::new(content.x, anchor.y));
                b.line_to(Point::new(content.x + content.width, anchor.y));
                b.move_to(Point::new(anchor.x, content.y));
                b.line_to(Point::new(anchor.x, content.y + content.height));
            });
            frame.stroke(
                &hairlines,
                Stroke::default().with_width(1.0).with_color(color.scale_alpha(0.6)),
            );
            frame.fill(&Path::circle(anchor, DOT_RADIUS), color);
        }
        Subject::Threshold { span } => {
            let segment = Path::line(
                Point::new(anchor.x - span / 2.0, anchor.y),
                Point::new(anchor.x + span / 2.0, anchor.y),
            );
            frame.stroke(&segment, Stroke::default().with_width(2.0).with_color(color));
        }
    }
}

fn draw_route<Renderer: geometry::Renderer>(
    frame: &mut Frame<Renderer>,
    route: Route,
    end: End,
    color: crate::core::Color,
) {
    let from = route.from();
    let heading = route.heading() - from;
    let length = (heading.x * heading.x + heading.y * heading.y).sqrt();
    let u = Vector::new(heading.x / length, heading.y / length);

    // The line starts behind an arrowhead so its cap doesn't poke through
    // the tip.
    let start = if end == End::Arrow {
        from + Vector::new(u.x * ARROW_LENGTH.min(length), u.y * ARROW_LENGTH.min(length))
    } else {
        from
    };
    let path = Path::new(|b| {
        b.move_to(start);
        match route {
            Route::Line { to, .. } => b.line_to(to),
            Route::Elbow { corner, to, .. } => {
                b.line_to(corner);
                b.line_to(to);
            }
            Route::Curve { control, to, .. } => b.quadratic_curve_to(control, to),
        }
    });
    frame.stroke(&path, Stroke::default().with_width(1.0).with_color(color));

    match end {
        End::None => {}
        End::Dot => frame.fill(&Path::circle(from, DOT_RADIUS), color),
        End::Arrow => {
            let base = from + Vector::new(u.x * ARROW_LENGTH, u.y * ARROW_LENGTH);
            let normal = Vector::new(-u.y * ARROW_HALF_WIDTH, u.x * ARROW_HALF_WIDTH);
            let head = Path::new(|b| {
                b.move_to(from);
                b.line_to(base + normal);
                b.line_to(base - normal);
                b.close();
            });
            frame.fill(&head, color);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mark::annotation::{self, Connector, Subject};

    const CONTENT: Rectangle = Rectangle {
        x: 40.0,
        y: 10.0,
        width: 300.0,
        height: 200.0,
    };
    const BOUNDS: Rectangle = Rectangle {
        x: 0.0,
        y: 0.0,
        width: 360.0,
        height: 240.0,
    };
    const TEXT: Text = Text {
        title: Size::new(0.0, 0.0),
        body: Size::new(60.0, 14.0),
    };

    fn place(a: &crate::mark::Annotation, anchor: Point) -> Option<Placement> {
        Placement::new(a, anchor, TEXT, CONTENT, BOUNDS)
    }

    #[test]
    fn line_connector_starts_on_the_circle_edge() {
        let a = annotation::annotation(0, 0)
            .subject(Subject::circle(10.0))
            .offset(40.0, -30.0);
        let anchor = Point::new(150.0, 120.0);
        let p = place(&a, anchor).expect("visible");
        let route = p.route.expect("connector");
        assert!((route.from().distance(anchor) - 10.0).abs() < 1e-4);
        // Toward the attachment point at (+40, -30): the 3-4-5 direction.
        assert!((route.from().x - 158.0).abs() < 1e-4 && (route.from().y - 114.0).abs() < 1e-4);
        assert_eq!(route.to(), Point::new(190.0, 90.0));
    }

    #[test]
    fn line_connector_starts_on_the_rect_edge() {
        let a = annotation::annotation(0, 0)
            .subject(Subject::rect(40.0, 20.0))
            .offset(-60.0, -60.0);
        let anchor = Point::new(200.0, 150.0);
        let route = place(&a, anchor).and_then(|p| p.route).expect("connector");
        // A 45° ray leaves a 40×20 box through its top edge, 10 px up.
        assert!((route.from().y - 140.0).abs() < 1e-4);
        assert!((route.from().x - 190.0).abs() < 1e-4);
    }

    #[test]
    fn elbow_leaves_the_subject_horizontally_then_turns_into_the_note() {
        let a = annotation::annotation(0, 0)
            .subject(Subject::circle(8.0))
            .connector(Connector::Elbow)
            .offset(50.0, -40.0);
        let anchor = Point::new(100.0, 150.0);
        let Some(Route::Elbow { from, corner, to }) = place(&a, anchor).and_then(|p| p.route) else {
            panic!("expected an elbow");
        };
        assert_eq!(from, Point::new(108.0, 150.0));
        assert_eq!(corner, Point::new(150.0, 150.0));
        assert_eq!(to, Point::new(150.0, 110.0));
    }

    #[test]
    fn curve_heads_along_its_control_point() {
        let a = annotation::annotation(0, 0)
            .subject(Subject::circle(5.0))
            .connector(Connector::Curve)
            .offset(-50.0, 40.0);
        let anchor = Point::new(200.0, 60.0);
        let Some(Route::Curve { from, control, to }) = place(&a, anchor).and_then(|p| p.route) else {
            panic!("expected a curve");
        };
        assert_eq!(from, Point::new(195.0, 60.0));
        assert_eq!(control, Point::new(150.0, 60.0));
        assert_eq!(to, Point::new(150.0, 100.0));
    }

    #[test]
    fn note_grows_away_from_the_subject() {
        let a = annotation::annotation(0, 0).offset(30.0, -30.0);
        let p = place(&a, Point::new(150.0, 120.0)).expect("visible");
        // Up-right offset: the attachment is the note's bottom-left corner.
        assert_eq!(p.note.x, 180.0);
        assert_eq!(p.note.y + p.note.height, 90.0);
    }

    #[test]
    fn anchor_outside_the_data_rect_is_clipped() {
        let a = annotation::annotation(0, 0).subject(Subject::circle(6.0));
        assert!(place(&a, Point::new(CONTENT.x - 5.0, 100.0)).is_none());
        assert!(place(&a, Point::new(150.0, CONTENT.y + CONTENT.height + 3.0)).is_none());
        assert!(place(&a, Point::new(f32::NAN, 100.0)).is_none());
        // The edge itself still shows.
        assert!(place(&a, Point::new(CONTENT.x, 100.0)).is_some());
    }

    #[test]
    fn note_past_the_plot_edge_is_nudged_inside_and_the_connector_follows() {
        let a = annotation::annotation(0, 0).offset(80.0, -100.0);
        let anchor = Point::new(320.0, 40.0);
        let p = place(&a, anchor).expect("visible");
        assert!(p.note.x + p.note.width <= BOUNDS.x + BOUNDS.width);
        assert!(p.note.y >= BOUNDS.y);
        let route = p.route.expect("connector");
        assert_eq!(route.to(), Point::new(p.note.x, p.note.y + p.note.height));
    }

    #[test]
    fn connector_none_draws_only_subject_and_note() {
        let a = annotation::annotation(0, 0).connector(Connector::None);
        assert!(place(&a, Point::new(150.0, 120.0)).expect("visible").route.is_none());
    }

    #[test]
    fn coordinates_land_on_log_and_reversed_axes() {
        let domain = Domain::new(
            super::super::Range { min: 1.0, max: 1000.0 },
            super::super::Range { min: 0.0, max: 100.0 },
            super::super::Mappings {
                x: crate::scale::Mapping::new(crate::scale::Transform::Log, crate::axis::Order::Forward),
                y: crate::scale::Mapping::new(crate::scale::Transform::Linear, crate::axis::Order::Reverse),
            },
        );
        let a = annotation::annotation(10.0, 25.0).subject(Subject::circle(6.0));
        let crate::mark::annotation::Target::Coordinates(datum) = a.target() else {
            panic!("coordinates");
        };
        let anchor = to_pixel(&domain, CONTENT, datum);
        // One decade of three: a third of the width. Reversed y: 25 sits a
        // quarter of the way down from the top.
        assert!((anchor.x - (CONTENT.x + CONTENT.width / 3.0)).abs() < 1e-3);
        assert!((anchor.y - (CONTENT.y + CONTENT.height * 0.25)).abs() < 1e-3);
        let p = place(&a, anchor).expect("visible");
        assert_eq!(p.anchor, anchor);
        assert!((p.route.expect("connector").from().distance(anchor) - 6.0).abs() < 1e-4);
    }

    #[test]
    fn bar_value_end_follows_the_axis_direction() {
        use crate::mark::bar::Direction;
        let rect = Rectangle::new(Point::new(10.0, 40.0), Size::new(20.0, 100.0));
        // Forward axis: zero at the bottom, the value end is the top.
        assert_eq!(
            value_end(rect, Point::new(0.0, 140.0), Direction::Vertical),
            Point::new(20.0, 40.0)
        );
        // Reversed axis: zero at the top, the value end is the bottom.
        assert_eq!(
            value_end(rect, Point::new(0.0, 40.0), Direction::Vertical),
            Point::new(20.0, 140.0)
        );
        assert_eq!(
            value_end(rect, Point::new(10.0, 0.0), Direction::Horizontal),
            Point::new(30.0, 90.0)
        );
    }
}

#[cfg(test)]
mod entry_tests {
    use super::*;
    use crate::chart::plot_area::{AxisSide, Mappings, PlotArea, Range};

    type Renderer = crate::widget::Renderer;
    type Paragraph = <Renderer as text::Renderer>::Paragraph;

    const CONTENT: Rectangle = Rectangle {
        x: 40.0,
        y: 10.0,
        width: 300.0,
        height: 200.0,
    };

    fn domain() -> Domain {
        Domain::new(
            Range { min: 0.0, max: 3.0 },
            Range { min: 0.0, max: 10.0 },
            Mappings::default(),
        )
    }

    /// Lays out the trees by hand: the line's pixel points as its layout
    /// would map them, then the annotations' placements.
    fn placements(marks: &[crate::Mark], secondary: &[crate::Mark]) -> Vec<Option<Placement>> {
        let plot = PlotArea::<(), Renderer>::new(marks).with_secondary(secondary);
        let mut tree = plot.state();
        let domain = domain();
        for (i, series) in plot.series.iter().enumerate() {
            if let Series::Line(line) = series {
                let state = tree.children[i]
                    .state
                    .downcast_mut::<super::super::line::State<Paragraph>>();
                state.pixel_points = line
                    .data
                    .points
                    .iter()
                    .map(|p| to_pixel(&domain, CONTENT, *p))
                    .collect();
            }
        }
        let primary = plot.axis_side.iter().filter(|s| **s == AxisSide::Primary).count();
        let bounds = Rectangle::new(Point::ORIGIN, Size::new(360.0, 240.0));
        for (i, series) in plot.series.iter().enumerate() {
            if let Series::Annotation(a) = series {
                let area = match plot.axis_side[i] {
                    AxisSide::Primary => 0..primary,
                    AxisSide::Secondary => primary..plot.series.len(),
                };
                a.place(&mut tree.children, i, &plot.series, area, &domain, CONTENT, bounds);
            }
        }
        tree.children
            .iter()
            .zip(&plot.series)
            .map(|(child, series)| match series {
                Series::Annotation(_) => child.state.downcast_ref::<State<Paragraph>>().placement,
                _ => None,
            })
            .collect()
    }

    #[test]
    fn entry_target_anchors_on_the_rendered_datum() {
        let marks = vec![
            crate::line([2.0, 8.0, 5.0, 6.0]).into(),
            crate::mark::annotation::entry(0, 0, 1).into(),
        ];
        let placed = placements(&marks, &[]);
        let anchor = placed[1].expect("placed").anchor;
        assert_eq!(anchor, to_pixel(&domain(), CONTENT, Datum::new(1.0, 8.0)));
    }

    #[test]
    fn entry_target_counts_marks_within_its_own_area() {
        let primary = vec![crate::line([1.0, 1.0, 1.0, 1.0]).into()];
        let secondary = vec![
            crate::line([2.0, 4.0, 6.0, 8.0]).into(),
            crate::mark::annotation::entry(0, 0, 2).into(),
        ];
        let placed = placements(&primary, &secondary);
        let anchor = placed[2].expect("placed").anchor;
        assert_eq!(anchor, to_pixel(&domain(), CONTENT, Datum::new(2.0, 6.0)));
    }

    #[test]
    fn unresolvable_entry_draws_nothing() {
        let marks = vec![
            crate::line([2.0, 8.0]).into(),
            crate::mark::annotation::entry(0, 0, 9).into(),
            crate::mark::annotation::entry(0, 3, 0).into(),
            crate::mark::annotation::entry(7, 0, 0).into(),
        ];
        let placed = placements(&marks, &[]);
        assert!(placed[1..].iter().all(Option::is_none));
    }
}
