use std::sync::Arc;

use crate::color::Color;
use crate::core::{Font, Pixels};
use crate::{scale, text};

/// Where on the plot area the legend attaches.
///
/// Corner variants align the legend to that corner; side variants
/// center-align along that edge. The [`Orientation`] decides which edge
/// the legend rides on, and [`Placement`] decides whether it overlays
/// the plot or pushes the plot area in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum Anchor {
    TopLeft,
    #[default]
    Top,
    TopRight,
    Right,
    BottomRight,
    Bottom,
    BottomLeft,
    Left,
}

/// Whether the legend draws on top of the plot area or outside it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum Placement {
    /// The legend is drawn on top of the plot area. The plot keeps its
    /// full size.
    Overlaid,
    /// The chart reserves space for the legend along the edge nearest the
    /// anchor. The plot area shrinks accordingly.
    #[default]
    Inset,
}

/// Legend layout direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum Orientation {
    /// Entries flow left-to-right; the legend rides the top or bottom edge.
    #[default]
    Horizontal,
    /// Entries stack top-to-bottom; the legend rides the left or right edge.
    Vertical,
}

/// Configuration for a chart legend.
///
/// A single `Config` drives both the series-key legend (swatches for each
/// data series) and the choropleth color-scale legend. Pick the edge with
/// [`Config::anchor`] + [`Config::orientation`] and whether the chart should
/// reserve space for it with [`Config::placement`].
///
/// # Examples
///
/// ```
/// use hyozu::legend;
///
/// // Short form — legend below the plot area (chart reserves space).
/// let config = legend::Config::below();
///
/// // Top-right overlay with vertical stacking.
/// let config = legend::Config::overlay(legend::Anchor::TopRight)
///     .orientation(legend::Orientation::Vertical);
/// ```
#[derive(Clone)]
pub struct Config {
    pub(crate) anchor: Anchor,
    pub(crate) placement: Placement,
    pub(crate) orientation: Orientation,
    /// Typography override. Any field left unset falls back to
    /// [`crate::Design::legend_text`].
    pub(crate) text: text::Style,
    /// Optional text color override.
    pub(crate) text_color: Option<Color>,
    /// Whether legend entries wrap to multiple rows (horizontal) or
    /// columns (vertical). Default true.
    pub(crate) wrap: bool,
    /// Whether clicking a legend entry toggles its series visibility.
    /// Defaults to `false` so existing charts are unaffected.
    pub(crate) interactive: bool,
    /// Highest-precedence override for the legend's value column.
    /// `None` falls through to the mark's `value_scale`, then the data
    /// scale, then the built-in default.
    pub(crate) value_format: Option<scale::Format<f64>>,
}

impl std::fmt::Debug for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Config")
            .field("anchor", &self.anchor)
            .field("placement", &self.placement)
            .field("orientation", &self.orientation)
            .field("text", &self.text)
            .field("text_color", &self.text_color)
            .field("wrap", &self.wrap)
            .field("interactive", &self.interactive)
            .field("value_format", &self.value_format.as_ref().map(|_| "<function>"))
            .finish()
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            anchor: Anchor::Bottom,
            placement: Placement::Inset,
            orientation: Orientation::Horizontal,
            text: text::Style::new(),
            text_color: None,
            wrap: true,
            interactive: false,
            value_format: None,
        }
    }
}

impl Config {
    /// Legend inset above the plot area (centered, horizontal).
    pub fn above() -> Self {
        Self {
            anchor: Anchor::Top,
            placement: Placement::Inset,
            orientation: Orientation::Horizontal,
            ..Self::default()
        }
    }

    /// Legend inset below the plot area (centered, horizontal).
    pub fn below() -> Self {
        Self {
            anchor: Anchor::Bottom,
            placement: Placement::Inset,
            orientation: Orientation::Horizontal,
            ..Self::default()
        }
    }

    /// Legend inset to the left of the plot area (centered, vertical).
    pub fn left() -> Self {
        Self {
            anchor: Anchor::Left,
            placement: Placement::Inset,
            orientation: Orientation::Vertical,
            ..Self::default()
        }
    }

    /// Legend inset to the right of the plot area (centered, vertical).
    pub fn right() -> Self {
        Self {
            anchor: Anchor::Right,
            placement: Placement::Inset,
            orientation: Orientation::Vertical,
            ..Self::default()
        }
    }

    /// Overlay legend at an arbitrary anchor point.
    ///
    /// The orientation is picked from the anchor: top/bottom side anchors
    /// and all corner anchors default to [`Orientation::Horizontal`];
    /// left/right side anchors default to [`Orientation::Vertical`]. Use
    /// [`Legend::orientation`] to override.
    pub fn overlay(anchor: Anchor) -> Self {
        let orientation = match anchor {
            Anchor::Left | Anchor::Right => Orientation::Vertical,
            Anchor::Top
            | Anchor::Bottom
            | Anchor::TopLeft
            | Anchor::TopRight
            | Anchor::BottomLeft
            | Anchor::BottomRight => Orientation::Horizontal,
        };
        Self {
            anchor,
            placement: Placement::Overlaid,
            orientation,
            ..Self::default()
        }
    }

    /// Sets the anchor position.
    pub fn anchor(mut self, anchor: Anchor) -> Self {
        self.anchor = anchor;
        self
    }

    /// Sets the placement (overlay vs inset).
    pub fn placement(mut self, placement: Placement) -> Self {
        self.placement = placement;
        self
    }

    /// Sets the orientation.
    pub fn orientation(mut self, orientation: Orientation) -> Self {
        self.orientation = orientation;
        self
    }

    /// Sets the font size for legend text. Accepts anything convertible to
    /// [`Pixels`] (`14.0`, `Pixels(14.0)`).
    ///
    /// Unset (the default) falls back to the legend renderer's built-in
    /// size.
    pub fn size(mut self, size: impl Into<Pixels>) -> Self {
        self.text.size = Some(size.into());
        self
    }

    /// Sets the font family for legend text.
    ///
    /// Accepts anything convertible to a [`Font`] — `"Inter"` works.
    pub fn font(mut self, font: impl Into<Font>) -> Self {
        self.text.family = Some(font.into());
        self
    }

    /// Overrides the full text style for legend labels. Any field left
    /// unset falls back to [`crate::Design::legend_text`].
    pub fn with_text(mut self, style: text::Style) -> Self {
        self.text = style;
        self
    }

    /// Sets the text color for legend labels.
    pub fn text_color(mut self, color: impl Into<Color>) -> Self {
        self.text_color = Some(color.into());
        self
    }

    /// Whether legend entries wrap to multiple rows/columns (default true).
    pub fn wrap(mut self, wrap: bool) -> Self {
        self.wrap = wrap;
        self
    }

    /// Enable or disable click-to-toggle on legend entries.
    ///
    /// When enabled, clicking a legend entry hides or shows its
    /// corresponding series. Visibility state lives inside the chart
    /// widget and is reset when the widget is destroyed.
    pub fn interactive(mut self, enabled: bool) -> Self {
        self.interactive = enabled;
        self
    }

    /// Sets a legend-only format closure for the value column.
    ///
    /// Highest precedence in the value-format chain — overrides any
    /// mark- or data-level scale for this legend's column. Other text
    /// sites (in-mark labels, hover tooltip) keep their own resolution.
    pub fn value_format(mut self, f: impl Fn(&f64) -> String + Send + Sync + 'static) -> Self {
        self.value_format = Some(Arc::new(f));
        self
    }

    /// Returns the legend-level value-format override, if any.
    pub fn value_format_ref(&self) -> Option<&scale::Format<f64>> {
        self.value_format.as_ref()
    }

    /// Returns the current anchor.
    pub fn anchor_value(&self) -> Anchor {
        self.anchor
    }

    /// Returns the current placement.
    pub fn placement_value(&self) -> Placement {
        self.placement
    }

    /// Returns the current orientation.
    pub fn orientation_value(&self) -> Orientation {
        self.orientation
    }

    /// Which edge of the plot area this legend rides.
    ///
    /// Decided by [`Self::orientation`] first (horizontal → top/bottom,
    /// vertical → left/right), then tie-broken by the anchor.
    pub fn edge(&self) -> Edge {
        match self.orientation {
            Orientation::Horizontal => match self.anchor {
                Anchor::Top | Anchor::TopLeft | Anchor::TopRight => Edge::Top,
                Anchor::Bottom | Anchor::BottomLeft | Anchor::BottomRight => Edge::Bottom,
                Anchor::Left => Edge::Bottom,
                Anchor::Right => Edge::Bottom,
            },
            Orientation::Vertical => match self.anchor {
                Anchor::Left | Anchor::TopLeft | Anchor::BottomLeft => Edge::Left,
                Anchor::Right | Anchor::TopRight | Anchor::BottomRight => Edge::Right,
                Anchor::Top => Edge::Right,
                Anchor::Bottom => Edge::Right,
            },
        }
    }
}

/// The edge of the plot area the legend attaches to, derived from the
/// combination of [`Anchor`] and [`Orientation`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Edge {
    Top,
    Bottom,
    Left,
    Right,
}

impl From<Anchor> for Config {
    fn from(anchor: Anchor) -> Self {
        Self::overlay(anchor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn above_resolves_to_top_edge_inset_horizontal() {
        let l = Config::above();
        assert_eq!(l.anchor_value(), Anchor::Top);
        assert_eq!(l.placement_value(), Placement::Inset);
        assert_eq!(l.orientation_value(), Orientation::Horizontal);
        assert_eq!(l.edge(), Edge::Top);
    }

    #[test]
    fn below_resolves_to_bottom_edge() {
        assert_eq!(Config::below().edge(), Edge::Bottom);
    }

    #[test]
    fn left_and_right_resolve_to_vertical_edges() {
        assert_eq!(Config::left().edge(), Edge::Left);
        assert_eq!(Config::right().edge(), Edge::Right);
        assert_eq!(Config::left().orientation_value(), Orientation::Vertical);
        assert_eq!(Config::right().orientation_value(), Orientation::Vertical);
    }

    #[test]
    fn overlay_picks_horizontal_for_corners() {
        let l = Config::overlay(Anchor::BottomRight);
        assert_eq!(l.orientation_value(), Orientation::Horizontal);
        assert_eq!(l.placement_value(), Placement::Overlaid);
    }

    #[test]
    fn overlay_picks_vertical_for_left_and_right_sides() {
        assert_eq!(Config::overlay(Anchor::Left).orientation_value(), Orientation::Vertical);
        assert_eq!(
            Config::overlay(Anchor::Right).orientation_value(),
            Orientation::Vertical
        );
    }

    #[test]
    fn builder_methods_override_defaults() {
        let l = Config::above()
            .anchor(Anchor::TopRight)
            .orientation(Orientation::Vertical)
            .placement(Placement::Overlaid);
        assert_eq!(l.anchor_value(), Anchor::TopRight);
        assert_eq!(l.orientation_value(), Orientation::Vertical);
        assert_eq!(l.placement_value(), Placement::Overlaid);
        // TopRight + Vertical => right edge
        assert_eq!(l.edge(), Edge::Right);
    }

    #[test]
    fn anchor_converts_into_overlay_config() {
        let l: Config = Anchor::TopLeft.into();
        assert_eq!(l.placement_value(), Placement::Overlaid);
        assert_eq!(l.anchor_value(), Anchor::TopLeft);
    }

    #[test]
    fn size_setter_overrides_text_size() {
        let l = Config::below().size(14.0);
        assert_eq!(l.text.size, Some(Pixels(14.0)));
    }
}
