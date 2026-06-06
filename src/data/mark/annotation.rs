//! Rich annotations for chart data points.
//!
//! # Overview
//!
//! Annotations attach contextual information to specific data points in a chart.
//! They follow the **Subject-Connector-Note** pattern (established by d3-annotation
//! and adopted by Semiotic):
//!
//! - **Subject**: The thing being annotated — a point, region, or threshold in
//!   data space. Rendered as a circle, rectangle, crosshair, or invisible anchor.
//! - **Connector**: A visual link from the subject to the note — a straight line,
//!   elbow, or curve, optionally with arrowheads or dot endpoints.
//! - **Note**: The label content — a title, body text, or both, with alignment
//!   and wrapping options. Positioned relative to the subject via a pixel offset.
//!
//! # Positioning
//!
//! Annotations are positioned in **data space**, not pixel space. The chart's
//! coordinate system resolves data values to screen positions automatically.
//! This means annotations survive resizing, zooming, and axis rescaling.
//!
//! ```ignore
//! // Annotation at category index 2, value 95.0
//! annotation(2.0, 95.0)
//!     .subject(Subject::circle(8.0))
//!     .note("Peak quarter")
//!     .offset(40.0, -30.0)
//! ```
//!
//! # Targeting
//!
//! Annotations can target data in three ways:
//!
//! - **Coordinates**: Explicit `(x, y)` in data space.
//! - **Index**: A datum index within a specific series (e.g. "bar 3 of series 0").
//!   The annotation follows the datum if data is reordered.
//! - **Predicate**: A filter function over data points (e.g. the maximum value).
//!   Useful for "annotate the outlier" without hardcoding positions.
//!
//! # Subjects
//!
//! The subject marks the region of interest:
//!
//! - `Subject::Point` — invisible anchor (just positions the connector origin)
//! - `Subject::Circle { radius }` — highlight circle around a datum
//! - `Subject::Rect { width, height }` — highlight rectangle
//! - `Subject::Crosshair` — horizontal + vertical hairlines through the point
//! - `Subject::Threshold { span }` — a short line segment scoped to one category
//!   band (unlike `Rule` which spans the whole axis)
//!
//! Subjects are rendered in the annotation's color at reduced opacity.
//!
//! # Connectors
//!
//! The connector links the subject to the note:
//!
//! - `Connector::None` — note floats without a visible link
//! - `Connector::Line` — straight line from subject edge to note
//! - `Connector::Elbow` — right-angle bend (horizontal then vertical)
//! - `Connector::Curve` — smooth quadratic Bézier
//!
//! Connector endpoints can be styled:
//!
//! - `End::None` — plain line termination
//! - `End::Arrow` — arrowhead
//! - `End::Dot` — small filled circle
//!
//! # Notes
//!
//! The note is the text content, offset from the subject by `(dx, dy)` in pixels:
//!
//! - `title` — bold/larger heading (optional)
//! - `body` — descriptive text (optional)
//! - `wrap` — maximum text width before wrapping
//! - `align` — text alignment within the note box
//! - `background` — optional background fill with rounded corners
//!
//! # Interaction with other marks
//!
//! Annotations render **above** all other marks (bars, lines, areas) but below
//! tooltips. They participate in the chart's data-to-pixel coordinate mapping and
//! respect axis bounds — annotations outside the visible range are clipped.
//!
//! Annotations do not contribute to axis bounds calculation. If an annotation's
//! target is outside the data range, it won't force the axis to expand.
//!
//! # Composing with Tick marks
//!
//! For the common case of per-datum reference lines (targets, thresholds),
//! prefer the `Tick` mark — it's simpler and data-driven. Use annotations
//! when you need a callout with text, a connector, or a highlight shape.
//!
//! ```ignore
//! // Simple: per-bar target lines → use Tick
//! let targets = tick([(0, 80.0), (1, 95.0), (2, 75.0)]);
//!
//! // Rich: callout pointing at a specific bar → use Annotation
//! let callout = annotation(1, 110.0)
//!     .subject(Subject::circle(12.0))
//!     .connector(Connector::Elbow)
//!     .note("Record quarter")
//!     .offset(50.0, -40.0);
//! ```
//!
//! # Future work
//!
//! - Draggable annotations (interactive repositioning via mouse)
//! - Annotation groups with shared styling
//! - Smart label placement to avoid overlapping annotations
//! - Animation support (annotations appearing/moving on data updates)
