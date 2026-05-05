//! Text-overlay mark.
//!
//! [`Text`] renders a per-point label layer composed alongside other marks
//! (typically a bubble map). Each item bundles its [`Datum`] and label so
//! the layer enforces 1:1 alignment by construction, sidestepping the
//! parallel-vector smell that two arguments would introduce.
//!
//! Text doesn't extend axis ranges, claim a palette slot, or contribute
//! to the legend — it's a passive label layer that projects through the
//! same plane (cartesian or geo) as the underlying point mark.

use crate::data::Datum;
use crate::data::mark::xy::CoordKind;

/// One labeled point. Bundling the datum with its label enforces a 1:1
/// pairing by construction — callers can't desync two parallel vectors.
#[derive(Debug, Clone)]
pub struct TextItem {
    pub datum: Datum,
    pub label: String,
}

impl TextItem {
    /// Convenience constructor.
    pub fn new(datum: Datum, label: impl Into<String>) -> Self {
        Self {
            datum,
            label: label.into(),
        }
    }
}

/// Horizontal alignment of each label relative to its projected point.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextAlign {
    Left,
    #[default]
    Center,
    Right,
}

/// Per-point text overlay. Projects through the same plane as the
/// underlying point mark (cartesian by default; flip with [`Text::on_geo`]).
#[derive(Debug, Clone)]
pub struct Text {
    pub(crate) items: Vec<TextItem>,
    pub(crate) coord_kind: CoordKind,
    /// Pixel offset applied AFTER projection. `(dx, dy)` in screen pixels;
    /// positive `dy` shifts down, negative `dy` shifts up (matches iced's
    /// pixel-y-down convention).
    pub(crate) offset: (f32, f32),
    pub(crate) align: TextAlign,
    pub(crate) size: f32,
    pub(crate) color: Option<crate::core::Color>,
}

/// Creates a [`Text`] mark from labeled items.
///
/// # Example
///
/// ```
/// use hyozu::Datum;
/// use hyozu::mark::text::{TextItem, text};
///
/// let labels = text([
///     TextItem::new(Datum::new(1.0, 2.0), "A"),
///     TextItem::new(Datum::new(3.0, 4.0), "B"),
/// ]);
/// ```
pub fn text(items: impl IntoIterator<Item = TextItem>) -> Text {
    Text {
        items: items.into_iter().collect(),
        coord_kind: CoordKind::Cartesian,
        offset: (0.0, 0.0),
        align: TextAlign::Center,
        size: 11.0,
        color: None,
    }
}

impl Text {
    /// Interpret each item's `(x, y)` as `(longitude, latitude)`. Renders
    /// through the chart's geo plane (configured via [`crate::Data::geo`]).
    /// When no geo plane is configured, the renderer emits no labels —
    /// matching [`crate::mark::xy::Xy::on_geo`]'s misconfiguration behavior.
    pub fn on_geo(mut self) -> Self {
        self.coord_kind = CoordKind::Geo;
        self
    }

    /// Pixel offset applied after projection. Use to nudge labels off
    /// their underlying marker (e.g. `(0.0, -16.0)` for "above the bubble").
    pub fn offset(mut self, dx: f32, dy: f32) -> Self {
        self.offset = (dx, dy);
        self
    }

    /// Sets horizontal alignment of each label relative to its projected
    /// (and offset) pixel position.
    pub fn align(mut self, align: TextAlign) -> Self {
        self.align = align;
        self
    }

    /// Sets label font size in pixels.
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    /// Sets label fill color. When unset, the renderer falls back to the
    /// design's text color so labels read against the active theme.
    pub fn color(mut self, color: impl Into<crate::core::Color>) -> Self {
        self.color = Some(color.into());
        self
    }

    /// Returns the items.
    pub fn items(&self) -> &[TextItem] {
        &self.items
    }

    /// Returns the coordinate-kind interpretation.
    pub fn coord_kind(&self) -> CoordKind {
        self.coord_kind
    }
}

impl From<Text> for crate::Data {
    fn from(t: Text) -> Self {
        use crate::data::IntoData;
        t.into_data()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_defaults_to_cartesian() {
        let t = text([TextItem::new(Datum::new(1.0, 2.0), "A")]);
        assert_eq!(t.coord_kind, CoordKind::Cartesian);
        assert_eq!(t.align, TextAlign::Center);
        assert_eq!(t.size, 11.0);
        assert_eq!(t.offset, (0.0, 0.0));
        assert!(t.color.is_none());
    }

    #[test]
    fn on_geo_flips_coord_kind() {
        let t = text([TextItem::new(Datum::new(1.0, 2.0), "A")]).on_geo();
        assert_eq!(t.coord_kind, CoordKind::Geo);
    }

    #[test]
    fn builders_set_fields() {
        let t = text([TextItem::new(Datum::new(0.0, 0.0), "A")])
            .offset(2.0, -4.0)
            .align(TextAlign::Right)
            .size(14.0);
        assert_eq!(t.offset, (2.0, -4.0));
        assert_eq!(t.align, TextAlign::Right);
        assert_eq!(t.size, 14.0);
    }

    #[test]
    fn item_pairs_datum_and_label() {
        let item = TextItem::new(Datum::new(3.0, 4.0), "label");
        assert_eq!(item.datum.x, 3.0);
        assert_eq!(item.datum.y, 4.0);
        assert_eq!(item.label, "label");
    }
}
