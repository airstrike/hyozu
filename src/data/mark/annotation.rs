//! Rich annotations for chart data points.
//!
//! # Overview
//!
//! Annotations attach contextual information to specific data points in a chart.
//! They follow the **Subject-Connector-Note** pattern (established by d3-annotation
//! and adopted by Semiotic):
//!
//! - **Subject**: The thing being annotated — a point or a small region around
//!   it. Rendered as a circle, rectangle, crosshair, threshold or invisible
//!   anchor.
//! - **Connector**: A visual link from the subject to the note — a straight line,
//!   elbow, or curve, optionally with an arrowhead or dot at the subject end.
//! - **Note**: The label content — a title, body text, or both, with alignment,
//!   wrapping and an optional background. Positioned relative to the subject
//!   via a pixel offset.
//!
//! # Positioning
//!
//! Annotations are positioned in **data space**, not pixel space. The chart's
//! coordinate system resolves data values to screen positions through each
//! axis's [`crate::scale::Mapping`], so annotations land correctly on log and
//! reversed axes and survive resizing and axis rescaling.
//!
//! ```
//! use hyozu::annotation::{self, Subject};
//!
//! // Annotation at category index 2, value 95.0
//! let peak = hyozu::annotation(2.0, 95.0)
//!     .subject(Subject::circle(8.0))
//!     .note("Peak quarter")
//!     .offset(40.0, -30.0);
//! ```
//!
//! # Targeting
//!
//! Annotations target data in two ways:
//!
//! - **Coordinates**: Explicit `(x, y)` in data space, via [`annotation`].
//! - **Entry**: A datum index within a series of another mark in the same
//!   plotting area (e.g. "bar 3 of series 0 of mark 0"), via [`entry`]. The
//!   annotation reads the rendered position of that datum, so it follows
//!   stacking, grouping and the datum itself when the data changes.
//!
//! An entry target resolves against lines, areas, bars, scatter and tick
//! marks. Targets that don't resolve (an out-of-range index, or a mark
//! without addressable points) draw nothing.
//!
//! # Subjects
//!
//! The subject marks the region of interest:
//!
//! - [`Subject::Point`] — invisible anchor (just positions the connector origin)
//! - [`Subject::Circle`] — highlight circle around a datum
//! - [`Subject::Rect`] — highlight rectangle centered on a datum
//! - [`Subject::Crosshair`] — horizontal + vertical hairlines through the point
//! - [`Subject::Threshold`] — a short horizontal segment centered on the datum,
//!   scoped to one category band (unlike `Rule` which spans the whole axis)
//!
//! Subjects are rendered in the annotation's color, filled at reduced opacity.
//!
//! # Connectors
//!
//! The connector links the subject to the note. It starts on the subject's
//! edge, not its center:
//!
//! - [`Connector::None`] — note floats without a visible link
//! - [`Connector::Line`] — straight line from subject edge to note
//! - [`Connector::Elbow`] — right-angle bend (horizontal then vertical)
//! - [`Connector::Curve`] — smooth quadratic Bézier
//!
//! The subject end can be styled with [`End::None`], [`End::Arrow`] or
//! [`End::Dot`].
//!
//! # Notes
//!
//! The note is the text content, offset from the subject by `(dx, dy)` in
//! pixels. The offset point is where the connector meets the note; the note
//! box grows away from the subject from there.
//!
//! - `title` — bold heading (optional)
//! - `body` — descriptive text (optional)
//! - `wrap` — maximum text width before wrapping
//! - `align` — text alignment within the note box
//! - `background` — optional background fill with rounded corners
//!
//! A note that would leave the plot area is nudged back inside it, and the
//! connector follows.
//!
//! # Interaction with other marks
//!
//! Annotations render **above** all other marks and their data labels, but
//! below tooltips. An annotation whose target falls outside the visible data
//! range is clipped: it draws nothing. Subjects are clipped to the data
//! rectangle.
//!
//! Annotations do not contribute to axis bounds, palette slots, legends or
//! default axes. If an annotation's target is outside the data range, it won't
//! force the axis to expand.
//!
//! # Composing with Tick marks
//!
//! For the common case of per-datum reference lines (targets, thresholds),
//! prefer the `Tick` mark — it's simpler and data-driven. Use annotations
//! when you need a callout with text, a connector, or a highlight shape.
//!
//! ```
//! use hyozu::annotation::{self, Connector, Subject};
//!
//! // Simple: per-bar target lines → use Tick
//! let targets = hyozu::tick([(0, 80.0), (1, 95.0), (2, 75.0)]);
//!
//! // Rich: callout pointing at bar 1 of the first mark's first series
//! let callout = annotation::entry(0, 0, 1)
//!     .subject(Subject::circle(12.0))
//!     .connector(Connector::Elbow)
//!     .note("Record quarter")
//!     .offset(50.0, -40.0);
//! ```
//!
//! # Future work
//!
//! - Predicate targets (e.g. "the maximum of series 0")
//! - Draggable annotations (interactive repositioning via mouse)
//! - Annotation groups with shared styling
//! - Smart label placement to avoid overlapping annotations
//! - Animation support (annotations appearing/moving on data updates)
use crate::color::Color;
use crate::data::Datum;
use crate::mark::text::TextAlign;

/// A callout attached to a datum: subject, connector and note.
///
/// Build one with [`annotation`] (data coordinates) or [`entry`] (a datum of
/// another mark), then add it to the chart's marks alongside the data it
/// describes.
#[derive(Debug, Clone)]
pub struct Annotation {
    pub(crate) target: Target,
    pub(crate) subject: Subject,
    pub(crate) connector: Connector,
    pub(crate) end: End,
    pub(crate) note: Note,
    pub(crate) offset: (f32, f32),
    pub(crate) color: Option<Color>,
}

/// What an [`Annotation`] points at.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Target {
    /// Explicit data coordinates, mapped through the plot's axes.
    Coordinates(Datum),
    /// A datum of another mark in the same plotting area (primary or
    /// secondary), addressed the way [`crate::Target::Entry`] addresses it.
    Entry {
        /// Index of the mark in its area's marks list.
        mark: usize,
        /// Series within the mark (0 for single-series marks).
        series: usize,
        /// Datum within the series.
        index: usize,
    },
}

/// The highlight drawn at the annotated datum.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Subject {
    /// Invisible anchor. The connector starts at the datum itself.
    #[default]
    Point,
    /// A circle of `radius` pixels around the datum.
    Circle {
        /// Radius in pixels.
        radius: f32,
    },
    /// A `width` × `height` pixel rectangle centered on the datum.
    Rect {
        /// Width in pixels.
        width: f32,
        /// Height in pixels.
        height: f32,
    },
    /// Hairlines through the datum spanning the whole data rectangle.
    Crosshair,
    /// A horizontal segment `span` pixels wide, centered on the datum.
    Threshold {
        /// Width in pixels.
        span: f32,
    },
}

/// The link drawn between the subject and the note.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Connector {
    /// No visible link.
    None,
    /// A straight line from the subject edge to the note.
    #[default]
    Line,
    /// A right-angle bend: horizontal from the subject, then vertical into
    /// the note.
    Elbow,
    /// A quadratic Bézier that leaves the subject horizontally and arrives
    /// at the note vertically.
    Curve,
}

/// Decoration at the subject end of the connector.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum End {
    /// Plain line termination.
    #[default]
    None,
    /// An arrowhead pointing at the subject.
    Arrow,
    /// A small filled circle.
    Dot,
}

/// The text content of an annotation.
#[derive(Debug, Clone, Default)]
pub(crate) struct Note {
    pub(crate) title: Option<String>,
    pub(crate) body: Option<String>,
    pub(crate) wrap: Option<f32>,
    pub(crate) align: TextAlign,
    pub(crate) background: Option<Color>,
}

/// Creates an annotation at data coordinates `(x, y)`.
///
/// # Examples
///
/// ```
/// use hyozu::annotation::Subject;
///
/// let peak = hyozu::annotation(2.0, 95.0)
///     .subject(Subject::circle(8.0))
///     .title("Peak")
///     .note("Best quarter on record");
/// ```
pub fn annotation(x: impl Into<f64>, y: impl Into<f64>) -> Annotation {
    Annotation::new(Target::Coordinates(Datum::new(x.into(), y.into())))
}

/// Creates an annotation targeting datum `index` of `series` within the
/// `mark`-th mark of the same plotting area.
///
/// # Examples
///
/// ```
/// use hyozu::annotation;
///
/// // The fourth bar of the first series of the first mark.
/// let callout = annotation::entry(0, 0, 3).note("Launch");
/// ```
pub fn entry(mark: usize, series: usize, index: usize) -> Annotation {
    Annotation::new(Target::Entry { mark, series, index })
}

impl Subject {
    /// A highlight circle of `radius` pixels.
    pub fn circle(radius: f32) -> Self {
        Self::Circle { radius }
    }

    /// A highlight rectangle of `width` × `height` pixels.
    pub fn rect(width: f32, height: f32) -> Self {
        Self::Rect { width, height }
    }

    /// A horizontal threshold segment `span` pixels wide.
    pub fn threshold(span: f32) -> Self {
        Self::Threshold { span }
    }
}

impl Annotation {
    fn new(target: Target) -> Self {
        Self {
            target,
            subject: Subject::default(),
            connector: Connector::default(),
            end: End::default(),
            note: Note::default(),
            offset: (30.0, -30.0),
            color: None,
        }
    }

    /// Sets the subject drawn at the target.
    pub fn subject(mut self, subject: Subject) -> Self {
        self.subject = subject;
        self
    }

    /// Sets how the subject links to the note.
    pub fn connector(mut self, connector: Connector) -> Self {
        self.connector = connector;
        self
    }

    /// Sets the decoration at the subject end of the connector.
    pub fn end(mut self, end: End) -> Self {
        self.end = end;
        self
    }

    /// Sets the note's body text.
    pub fn note(mut self, body: impl Into<String>) -> Self {
        self.note.body = Some(body.into());
        self
    }

    /// Sets the note's bold title, drawn above the body.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.note.title = Some(title.into());
        self
    }

    /// Wraps note text at `width` pixels.
    pub fn wrap(mut self, width: f32) -> Self {
        self.note.wrap = Some(width);
        self
    }

    /// Aligns note text within the note box. Defaults to center.
    pub fn align(mut self, align: TextAlign) -> Self {
        self.note.align = align;
        self
    }

    /// Fills the note box with `color` behind rounded corners.
    pub fn background(mut self, color: impl Into<Color>) -> Self {
        self.note.background = Some(color.into());
        self
    }

    /// Places the note `(dx, dy)` pixels from the target. Positive `dy`
    /// points down. Defaults to `(30, -30)`: up and to the right.
    pub fn offset(mut self, dx: f32, dy: f32) -> Self {
        self.offset = (dx, dy);
        self
    }

    /// Sets the color of the subject, connector and title. Defaults to the
    /// design's text color.
    pub fn color(mut self, color: impl Into<Color>) -> Self {
        self.color = Some(color.into());
        self
    }

    /// Returns what this annotation points at.
    pub fn target(&self) -> Target {
        self.target
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coordinates_target_keeps_data_values() {
        let a = annotation(2, 95.5);
        assert_eq!(a.target(), Target::Coordinates(Datum::new(2.0, 95.5)));
    }

    #[test]
    fn entry_target_addresses_mark_series_and_index() {
        let a = entry(1, 2, 3);
        assert_eq!(a.target(), Target::Entry {
            mark: 1,
            series: 2,
            index: 3
        });
    }
}
