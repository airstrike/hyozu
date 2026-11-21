//! Hatched pattern fills for bars, areas and pie slices.
//!
//! iced fills a path with a solid color or a gradient only, so a pattern is
//! drawn as geometry: the shape is filled with the pattern's background, then
//! a family of parallel lines is clipped to the shape's outline and stroked
//! in the foreground. Lines are anchored to the frame rather than to each
//! shape, so neighboring bars or slices hatched alike line up.

use crate::color::Color;
use crate::core::{Point, Vector};
use crate::widget::canvas::path::lyon_path;
use crate::widget::canvas::{Frame, LineCap, Path, Stroke};
use crate::widget::renderer::geometry;

/// Curves in an outline are flattened to chords within this many pixels.
const TOLERANCE: f32 = 0.1;

/// A hatched fill: parallel lines over a background.
///
/// Unset, the lines take the series color and the background a light tint
/// of it, so a hatched series still reads as its color.
///
/// # Examples
///
/// ```
/// use hyozu::pattern::{Hatch, Pattern};
///
/// let striped = hyozu::bar([3, 5, 4]).pattern(Hatch::Upward);
/// let dense = hyozu::bar([3, 5, 4]).pattern(Pattern::new(Hatch::Grid).spacing(4.0).width(0.75));
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Pattern {
    hatch: Hatch,
    foreground: Option<Color>,
    background: Option<Color>,
    spacing: f32,
    width: f32,
}

/// The line families of a hatch, after Excel's pattern fills.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Hatch {
    /// Horizontal lines.
    Horizontal,
    /// Vertical lines.
    Vertical,
    /// Lines rising to the right (`/`).
    Upward,
    /// Lines falling to the right (`\`).
    Downward,
    /// Horizontal and vertical lines.
    Grid,
    /// Both diagonals.
    Crosshatch,
}

impl Pattern {
    /// The narrowest line spacing, in pixels.
    pub const MIN_SPACING: f32 = 2.0;

    /// A hatch with 6 px spacing and 1 px lines.
    pub fn new(hatch: Hatch) -> Self {
        Self {
            hatch,
            foreground: None,
            background: None,
            spacing: 6.0,
            width: 1.0,
        }
    }

    /// Sets the line color.
    pub fn foreground(mut self, color: impl Into<Color>) -> Self {
        self.foreground = Some(color.into());
        self
    }

    /// Sets the color under the lines.
    pub fn background(mut self, color: impl Into<Color>) -> Self {
        self.background = Some(color.into());
        self
    }

    /// Sets the distance between neighboring lines, at least
    /// [`MIN_SPACING`](Self::MIN_SPACING). Lines never grow wider than it.
    pub fn spacing(mut self, spacing: f32) -> Self {
        self.spacing = if spacing.is_finite() {
            spacing.max(Self::MIN_SPACING)
        } else {
            Self::MIN_SPACING
        };
        self.width = self.width.min(self.spacing);
        self
    }

    /// Sets the line width in pixels, between a quarter pixel and the spacing.
    pub fn width(mut self, width: f32) -> Self {
        self.width = if width.is_finite() { width } else { 1.0 }.clamp(0.25, self.spacing);
        self
    }

    /// Returns the hatch.
    pub fn hatch(&self) -> Hatch {
        self.hatch
    }

    /// Returns the line color, if set.
    pub fn foreground_value(&self) -> Option<&Color> {
        self.foreground.as_ref()
    }

    /// Returns the background color, if set.
    pub fn background_value(&self) -> Option<&Color> {
        self.background.as_ref()
    }

    /// Returns the line spacing in pixels.
    pub fn spacing_value(&self) -> f32 {
        self.spacing
    }

    /// Returns the line width in pixels.
    pub fn width_value(&self) -> f32 {
        self.width
    }

    /// The hatch lines inside `outline`, under the even-odd rule.
    pub(crate) fn segments(&self, outline: &Path) -> Vec<(Point, Point)> {
        let rings = rings(outline);
        self.hatch
            .directions()
            .iter()
            .flat_map(|&direction| clip(&rings, direction, self.spacing))
            .collect()
    }

    /// Fills `outline` with the pattern's background, then strokes its
    /// hatch lines clipped to the outline, in colors resolved by [`Ink`].
    pub(crate) fn fill<R: geometry::Renderer>(&self, frame: &mut Frame<R>, outline: &Path, ink: Ink) {
        frame.fill(outline, ink.background);
        let segments = self.segments(outline);
        if segments.is_empty() {
            return;
        }
        let lines = Path::new(|builder| {
            for (a, b) in &segments {
                builder.move_to(*a);
                builder.line_to(*b);
            }
        });
        frame.stroke(
            &lines,
            Stroke::default()
                .with_width(self.width)
                .with_color(ink.foreground)
                .with_line_cap(LineCap::Butt),
        );
    }
}

/// The resolved colors a pattern paints with.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Ink {
    pub(crate) foreground: crate::core::Color,
    pub(crate) background: crate::core::Color,
}

impl Ink {
    /// The tint an unset background takes, as a share of the series color.
    const TINT: f32 = 0.22;

    /// Resolves a pattern's colors against the theme, defaulting to the
    /// series color for the lines and a tint of it for the background.
    pub(crate) fn resolve(
        pattern: &Pattern,
        series: crate::core::Color,
        background: crate::core::Color,
        text: crate::color::Pair,
        seed: &crate::palette::Seed,
    ) -> Self {
        let resolve = |c: &Color| c.resolve(background, text, seed, None);
        Self {
            foreground: pattern.foreground.as_ref().map_or(series, resolve),
            background: pattern
                .background
                .as_ref()
                .map_or(series.scale_alpha(Self::TINT), resolve),
        }
    }
}

impl From<Hatch> for Pattern {
    fn from(hatch: Hatch) -> Self {
        Self::new(hatch)
    }
}

impl Hatch {
    /// Unit directions of the line families, in screen space (y down).
    fn directions(self) -> &'static [Vector] {
        const H: Vector = Vector::new(1.0, 0.0);
        const V: Vector = Vector::new(0.0, 1.0);
        const UP: Vector = Vector::new(std::f32::consts::FRAC_1_SQRT_2, -std::f32::consts::FRAC_1_SQRT_2);
        const DOWN: Vector = Vector::new(std::f32::consts::FRAC_1_SQRT_2, std::f32::consts::FRAC_1_SQRT_2);
        match self {
            Self::Horizontal => &[H],
            Self::Vertical => &[V],
            Self::Upward => &[UP],
            Self::Downward => &[DOWN],
            Self::Grid => &[H, V],
            Self::Crosshatch => &[UP, DOWN],
        }
    }
}

/// Flattens a path into closed polygons, one per subpath.
fn rings(path: &Path) -> Vec<Vec<Point>> {
    use lyon_path::PathEvent;
    use lyon_path::iterator::PathIterator;

    let mut rings = Vec::new();
    let mut ring = Vec::new();
    for event in path.raw().iter().flattened(TOLERANCE) {
        match event {
            PathEvent::Begin { at } => {
                ring.clear();
                ring.push(Point::new(at.x, at.y));
            }
            PathEvent::Line { to, .. } => ring.push(Point::new(to.x, to.y)),
            PathEvent::End { .. } => {
                if ring.len() >= 3 {
                    rings.push(std::mem::take(&mut ring));
                }
                ring.clear();
            }
            PathEvent::Quadratic { .. } | PathEvent::Cubic { .. } => {}
        }
    }
    rings
}

/// Clips the lines `{p : p·n = k·spacing}` (n normal to `direction`) to the
/// inside of `rings` under the even-odd rule.
///
/// Every edge crossing is found with a half-open test, so a line through a
/// vertex counts it once and the crossings along each line pair up into
/// inside spans.
fn clip(rings: &[Vec<Point>], direction: Vector, spacing: f32) -> Vec<(Point, Point)> {
    let normal = Vector::new(-direction.y, direction.x);
    let along = |p: Point| p.x * direction.x + p.y * direction.y;
    let across = |p: Point| p.x * normal.x + p.y * normal.y;
    let (lo, hi) = rings
        .iter()
        .flatten()
        .fold((f32::INFINITY, f32::NEG_INFINITY), |(lo, hi), &p| {
            (lo.min(across(p)), hi.max(across(p)))
        });
    if !(lo.is_finite() && hi.is_finite()) || spacing <= 0.0 {
        return Vec::new();
    }
    let first = (lo / spacing).ceil() as i64;
    let last = (hi / spacing).floor() as i64;
    let mut segments = Vec::new();
    let mut hits: Vec<Point> = Vec::new();
    for k in first..=last {
        let c = k as f32 * spacing;
        hits.clear();
        for ring in rings {
            for (i, &a) in ring.iter().enumerate() {
                let b = ring[(i + 1) % ring.len()];
                let (fa, fb) = (across(a) - c, across(b) - c);
                if (fa > 0.0) != (fb > 0.0) {
                    let t = fa / (fa - fb);
                    hits.push(Point::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t));
                }
            }
        }
        hits.sort_by(|p, q| along(*p).total_cmp(&along(*q)));
        segments.extend(
            hits.as_chunks::<2>()
                .0
                .iter()
                .filter(|[a, b]| along(*b) - along(*a) > f32::EPSILON)
                .map(|&[a, b]| (a, b)),
        );
    }
    segments
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::Size;

    const EPS: f32 = 1e-3;

    /// Even-odd point-in-polygon, counting the boundary as inside.
    fn inside(rings: &[Vec<Point>], p: Point) -> bool {
        let on_edge = rings.iter().any(|ring| {
            (0..ring.len()).any(|i| {
                let (a, b) = (ring[i], ring[(i + 1) % ring.len()]);
                let cross = (b.x - a.x) * (p.y - a.y) - (b.y - a.y) * (p.x - a.x);
                let len = ((b.x - a.x).powi(2) + (b.y - a.y).powi(2)).sqrt();
                let within = p.x >= a.x.min(b.x) - EPS
                    && p.x <= a.x.max(b.x) + EPS
                    && p.y >= a.y.min(b.y) - EPS
                    && p.y <= a.y.max(b.y) + EPS;
                within && cross.abs() <= EPS * len.max(1.0)
            })
        });
        let mut odd = false;
        for ring in rings {
            for i in 0..ring.len() {
                let (a, b) = (ring[i], ring[(i + 1) % ring.len()]);
                if (a.y > p.y) != (b.y > p.y) && p.x < a.x + (p.y - a.y) / (b.y - a.y) * (b.x - a.x) {
                    odd = !odd;
                }
            }
        }
        on_edge || odd
    }

    fn assert_clipped(outline: &Path, pattern: &Pattern) -> Vec<(Point, Point)> {
        let rings = rings(outline);
        let segments = pattern.segments(outline);
        assert!(!segments.is_empty());
        for &(a, b) in &segments {
            for t in [0.0, 0.25, 0.5, 0.75, 1.0] {
                let p = Point::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t);
                assert!(inside(&rings, p), "{p:?} on {a:?}-{b:?} escapes the outline");
            }
        }
        segments
    }

    #[test]
    fn horizontal_hatch_fills_a_rectangle_edge_to_edge() {
        let outline = Path::rectangle(Point::new(3.0, 2.0), Size::new(40.0, 30.0));
        let segments = assert_clipped(&outline, &Pattern::new(Hatch::Horizontal).spacing(5.0));
        // Lines at y = 5, 10, …, 30 cross the 2..32 band.
        assert_eq!(segments.len(), 6);
        for (a, b) in segments {
            assert!((a.x - 3.0).abs() < EPS && (b.x - 43.0).abs() < EPS);
            assert_eq!(a.y, b.y);
        }
    }

    #[test]
    fn diagonal_hatch_stays_inside_a_rectangle() {
        let outline = Path::rectangle(Point::new(0.0, 0.0), Size::new(50.0, 20.0));
        for hatch in [
            Hatch::Upward,
            Hatch::Downward,
            Hatch::Crosshatch,
            Hatch::Grid,
            Hatch::Vertical,
        ] {
            assert_clipped(&outline, &Pattern::new(hatch));
        }
    }

    #[test]
    fn concave_outline_splits_a_line_into_two_spans() {
        // A U: two prongs joined at the bottom.
        let outline = Path::new(|b| {
            b.move_to(Point::new(0.0, 0.0));
            b.line_to(Point::new(10.0, 0.0));
            b.line_to(Point::new(10.0, 20.0));
            b.line_to(Point::new(20.0, 20.0));
            b.line_to(Point::new(20.0, 0.0));
            b.line_to(Point::new(30.0, 0.0));
            b.line_to(Point::new(30.0, 30.0));
            b.line_to(Point::new(0.0, 30.0));
            b.close();
        });
        let segments = assert_clipped(&outline, &Pattern::new(Hatch::Horizontal).spacing(4.0));
        let at = |y: f32| segments.iter().filter(|(a, _)| (a.y - y).abs() < EPS).count();
        assert_eq!(at(8.0), 2, "the prongs are separate above the bend");
        assert_eq!(at(24.0), 1, "the base is one span");
    }

    #[test]
    fn hatch_respects_holes_and_curves() {
        let ring = Path::new(|b| {
            b.circle(Point::new(50.0, 50.0), 40.0);
            b.circle(Point::new(50.0, 50.0), 15.0);
        });
        let segments = assert_clipped(&ring, &Pattern::new(Hatch::Upward).spacing(3.0));
        let hole = |p: Point| (p.x - 50.0).hypot(p.y - 50.0) < 15.0 - 0.5;
        for (a, b) in segments {
            let mid = Point::new((a.x + b.x) / 2.0, (a.y + b.y) / 2.0);
            assert!(!hole(mid), "{a:?}-{b:?} crosses the hole");
        }
    }

    #[test]
    fn neighbouring_shapes_share_hatch_lines() {
        let left = Pattern::new(Hatch::Horizontal).segments(&Path::rectangle(Point::ORIGIN, Size::new(10.0, 30.0)));
        let right =
            Pattern::new(Hatch::Horizontal).segments(&Path::rectangle(Point::new(14.0, 3.0), Size::new(10.0, 30.0)));
        let rows = |s: &[(Point, Point)]| s.iter().map(|(a, _)| a.y).collect::<Vec<_>>();
        let (l, r) = (rows(&left), rows(&right));
        assert!(l.iter().filter(|y| r.contains(y)).count() >= 4);
    }

    #[test]
    fn spacing_and_width_hold_their_bounds() {
        let p = Pattern::new(Hatch::Grid).spacing(0.0).width(10.0);
        assert_eq!(p.spacing_value(), Pattern::MIN_SPACING);
        assert_eq!(p.width_value(), Pattern::MIN_SPACING);
        assert_eq!(
            Pattern::new(Hatch::Grid).spacing(f32::NAN).spacing_value(),
            Pattern::MIN_SPACING
        );
    }

    #[test]
    fn degenerate_outlines_yield_no_lines() {
        let flat = Path::rectangle(Point::ORIGIN, Size::new(30.0, 0.0));
        assert!(Pattern::new(Hatch::Vertical).segments(&flat).is_empty());
    }
}
