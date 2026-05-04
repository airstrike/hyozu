use crate::core::Element;
use crate::widget::Renderer;

/// Default hole radius as a proportion of the outer radius for a donut.
const DEFAULT_HOLE: f32 = 0.5;

/// Donut-variant configuration carried inside the chart widget's `Kind`.
///
/// Holds the donut-only knobs (hole radius, optional center overlay)
/// so they don't leak into the generic `chart()` builder or the
/// data-layer `pie::Pie` spec.
pub struct Chart<'a, Message, Theme> {
    /// Inner hole radius as a proportion of the outer radius.
    pub(crate) hole: f32,
    /// Optional center overlay rendered inside the donut hole.
    pub(crate) center: Option<Center<'a, Message, Theme>>,
}

impl<'a, Message, Theme> Chart<'a, Message, Theme> {
    /// Creates a donut variant with the default hole and no center overlay.
    pub(crate) fn new() -> Self {
        Self {
            hole: DEFAULT_HOLE,
            center: None,
        }
    }
}

/// Center overlay configuration for a donut chart.
///
/// Pairs an iced `Element` with a `Placement` describing how it sizes
/// inside the chart's plot area. Constructed by the `Chart::center`
/// and `Chart::center_inset` builders on the chart widget.
pub struct Center<'a, Message, Theme> {
    /// The element to render inside the donut hole.
    pub(crate) element: Element<'a, Message, Theme, Renderer>,
    /// How the element is sized and positioned within the plot area.
    pub(crate) placement: Placement,
}

impl<'a, Message, Theme> Center<'a, Message, Theme> {
    /// Creates a center overlay with the given element and placement.
    pub(crate) fn new(element: Element<'a, Message, Theme, Renderer>, placement: Placement) -> Self {
        Self { element, placement }
    }
}

/// How a donut center overlay sizes itself within the chart's plot area.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Placement {
    /// The center element fills the entire plot area, stacking over the
    /// donut. Useful for cards where the overlay carries its own
    /// padding and shouldn't be clipped to the hole.
    Stack,
    /// The center element is inscribed inside the donut hole — sized as
    /// the largest axis-aligned square that fits in a circle of radius
    /// `inner_radius`.
    Inset,
}
