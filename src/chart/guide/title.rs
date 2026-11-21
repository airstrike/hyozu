//! Axis titles: the name of an axis, set beyond its tick labels.
//!
//! Bottom and top titles read left to right under (or over) the tick
//! labels. Left and right titles turn a quarter counterclockwise and read
//! bottom to top, the way Excel and Matplotlib set them. Each title centers
//! on the axis's data span and shortens with an ellipsis when the axis is
//! shorter than the title.

use crate::axis::Orientation;
use crate::core::{Font, Pixels, Point, Size};

/// Space between the tick labels (or the plot edge) and the title.
const GAP: f32 = 6.0;

/// Appended to a title shortened to fit its axis.
const ELLIPSIS: char = '…';

/// An axis title shaped for its axis: the text that fits along the axis,
/// and the font and size it was measured in.
#[derive(Debug, Clone, PartialEq)]
pub struct Fitted {
    content: String,
    font: Font,
    size: Pixels,
    /// Width of the fitted text along its reading direction.
    width: f32,
    /// Line height of the full title. The band depends on it alone, so a
    /// title keeps the same band however much of it fits.
    line: f32,
    /// Room the title may take past the start and end of the axis, from
    /// the empty corners between neighboring gutters.
    reach: (f32, f32),
}

/// Where a fitted title draws inside its guide.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placement {
    /// Center of the title in guide-local coordinates.
    pub center: Point,
    /// Clockwise turn in radians to apply before drawing, as
    /// `Frame::rotate` takes it.
    pub rotation: f32,
}

impl Fitted {
    /// Fits `content` along an axis `length` pixels long, which the title
    /// may overrun by `reach` at its start and end. A title that's too long
    /// shortens with an ellipsis, down to nothing at all. `measure` shapes
    /// a candidate string in the title's font and returns its size.
    ///
    /// Returns `None` for an empty title.
    pub fn new(
        content: &str,
        font: Font,
        size: Pixels,
        length: f32,
        reach: (f32, f32),
        measure: impl Fn(&str) -> Size,
    ) -> Option<Self> {
        let content = content.trim();
        if content.is_empty() {
            return None;
        }
        let room = length + reach.0.max(0.0) + reach.1.max(0.0);
        let full = measure(content);
        let fitted = |content: String, width: f32| Self {
            content,
            font,
            size,
            width,
            line: full.height,
            reach: (reach.0.max(0.0), reach.1.max(0.0)),
        };
        if full.width <= room {
            return Some(fitted(content.to_owned(), full.width));
        }

        // The widest prefix that still fits with an ellipsis, by binary
        // search over char boundaries: shaping is monotone in prefix length.
        let boundaries: Vec<usize> = content.char_indices().map(|(index, _)| index).collect();
        let shortened = |count: usize| {
            let end = boundaries.get(count).copied().unwrap_or(content.len());
            let mut text = content[..end].trim_end().to_owned();
            text.push(ELLIPSIS);
            text
        };
        let (mut fits, mut over) = (0usize, boundaries.len().saturating_sub(1));
        let mut best = None;
        while fits < over {
            let count = (fits + over).div_ceil(2);
            let candidate = shortened(count);
            let width = measure(&candidate).width;
            if width <= room {
                fits = count;
                best = Some((candidate, width));
            } else {
                over = count - 1;
            }
        }
        let (content, width) = best.unwrap_or_else(|| {
            let ellipsis = ELLIPSIS.to_string();
            let width = measure(&ellipsis).width;
            if width <= room {
                (ellipsis, width)
            } else {
                (String::new(), 0.0)
            }
        });
        Some(fitted(content, width))
    }

    /// The text to draw, possibly shortened or empty.
    pub fn content(&self) -> &str {
        &self.content
    }

    /// The font the title was measured in.
    pub fn font(&self) -> Font {
        self.font
    }

    /// The size the title was measured at.
    pub fn size(&self) -> Pixels {
        self.size
    }

    /// Room the title may take past the start and end of its axis.
    pub fn reach(&self) -> (f32, f32) {
        self.reach
    }

    /// The thickness the title adds to its guide across the axis: one line
    /// plus the gap that separates it from the tick labels.
    pub fn band(&self) -> f32 {
        self.line + GAP
    }

    /// Places the title in a guide of `area` size. `span` is the start and
    /// end of the axis's data range along the axis, in guide pixels; the
    /// title centers on it and slides inward rather than run past its
    /// reach.
    pub fn place(&self, orientation: Orientation, area: Size, span: (f32, f32)) -> Placement {
        let half = self.width / 2.0;
        let along = |length: f32| {
            let (lo, hi) = (half - self.reach.0, length + self.reach.1 - half);
            let center = (span.0 + span.1) / 2.0;
            if lo > hi { (lo + hi) / 2.0 } else { center.clamp(lo, hi) }
        };
        let across = self.line / 2.0;
        let quarter = -std::f32::consts::FRAC_PI_2;
        match orientation {
            Orientation::Bottom => Placement {
                center: Point::new(along(area.width), area.height - across),
                rotation: 0.0,
            },
            Orientation::Top => Placement {
                center: Point::new(along(area.width), across),
                rotation: 0.0,
            },
            Orientation::Left => Placement {
                center: Point::new(across, along(area.height)),
                rotation: quarter,
            },
            Orientation::Right => Placement {
                center: Point::new(area.width - across, along(area.height)),
                rotation: quarter,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Seven pixels per char, fourteen tall.
    fn mono(text: &str) -> Size {
        Size::new(7.0 * text.chars().count() as f32, 14.0)
    }

    fn fit(content: &str, length: f32) -> Option<Fitted> {
        Fitted::new(content, Font::DEFAULT, Pixels(12.0), length, (0.0, 0.0), mono)
    }

    #[test]
    fn short_title_keeps_its_text() {
        let title = fit("Revenue", 200.0).expect("fits");
        assert_eq!(title.content(), "Revenue");
        assert_eq!(title.band(), 14.0 + GAP);
    }

    #[test]
    fn long_title_shortens_with_an_ellipsis_to_fit() {
        let title = fit("Quarterly revenue by region", 70.0).expect("fits");
        assert_eq!(title.content(), "Quarterly…");
        assert!(mono(title.content()).width <= 70.0);
    }

    #[test]
    fn shortened_title_drops_trailing_space_before_the_ellipsis() {
        let title = fit("Net income", 35.0).expect("fits");
        assert_eq!(title.content(), "Net…");
    }

    #[test]
    fn empty_titles_vanish_and_unfittable_ones_keep_their_band() {
        assert_eq!(fit("   ", 100.0), None);
        let squeezed = fit("Revenue", 5.0).expect("titled");
        assert_eq!(squeezed.content(), "");
        assert_eq!(squeezed.band(), 14.0 + GAP);
        assert_eq!(
            fit("Revenue", 7.0).map(|t| t.content().to_owned()),
            Some("…".to_owned())
        );
    }

    #[test]
    fn title_may_reach_into_the_corners_past_its_axis() {
        let title = Fitted::new("Distance (km)", Font::DEFAULT, Pixels(12.0), 50.0, (20.0, 25.0), mono).expect("fits");
        assert_eq!(title.content(), "Distance (km)");
        // 91 px wide on a 50 px axis: it may span -20 to 75, so it centers
        // between 25.5 and 29.5.
        let area = Size::new(40.0, 50.0);
        assert_eq!(title.place(Orientation::Left, area, (0.0, 50.0)).center.y, 25.5);
        assert_eq!(title.place(Orientation::Left, area, (0.0, 55.0)).center.y, 27.5);
        assert_eq!(title.place(Orientation::Left, area, (40.0, 60.0)).center.y, 29.5);
    }

    #[test]
    fn horizontal_titles_sit_on_the_outer_edge_centered_on_the_span() {
        let title = fit("Hour", 400.0).expect("fits");
        let area = Size::new(400.0, 40.0);
        let bottom = title.place(Orientation::Bottom, area, (20.0, 380.0));
        assert_eq!(bottom.center, Point::new(200.0, 33.0));
        assert_eq!(bottom.rotation, 0.0);
        let top = title.place(Orientation::Top, area, (0.0, 300.0));
        assert_eq!(top.center, Point::new(150.0, 7.0));
    }

    #[test]
    fn vertical_titles_turn_to_read_upward_on_the_outer_edge() {
        let title = fit("Cells", 400.0).expect("fits");
        let area = Size::new(50.0, 300.0);
        let left = title.place(Orientation::Left, area, (0.0, 300.0));
        assert_eq!(left.center, Point::new(7.0, 150.0));
        assert_eq!(left.rotation, -std::f32::consts::FRAC_PI_2);
        let right = title.place(Orientation::Right, area, (0.0, 300.0));
        assert_eq!(right.center, Point::new(43.0, 150.0));
    }

    #[test]
    fn title_slides_inward_rather_than_leave_the_guide() {
        let title = fit("Temperature", 400.0).expect("fits");
        let placed = title.place(Orientation::Bottom, Size::new(100.0, 30.0), (60.0, 100.0));
        // 77 px wide: centered on 80 it would reach 118.5, so it stops at 61.5.
        assert_eq!(placed.center.x, 100.0 - 38.5);
    }
}
