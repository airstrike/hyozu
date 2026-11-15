//! Keyboard targets collected from the geometry prepared during chart layout.

use crate::chart::plot_area::{self, area, bars, line, xy};
use crate::core::keyboard::key::Named;
use crate::core::widget::Tree;
use crate::core::{Point, text};
use crate::data::Datum;
use crate::mark::xy::CoordKind;
use crate::widget::Renderer;

type Paragraph = <Renderer as text::Renderer>::Paragraph;

pub(super) fn targets<Message>(plot: &plot_area::PlotArea<'_, Message, Renderer>, tree: &Tree) -> Vec<Point> {
    let mut points = Vec::new();
    let mut append = |data: &[Datum], pixels: &[Point]| {
        points.extend(data.iter().zip(pixels).filter_map(|(datum, pixel)| {
            (datum.x.is_finite() && datum.y.is_finite() && pixel.x.is_finite() && pixel.y.is_finite())
                .then_some((datum.x, *pixel))
        }));
    };
    for (series, child) in plot.series.iter().zip(&tree.children) {
        match series {
            plot_area::Series::Line(mark) => {
                let state = child.state.downcast_ref::<line::State<Paragraph>>();
                append(&mark.data.points, &state.pixel_points);
            }
            plot_area::Series::Area(mark) => {
                let state = child.state.downcast_ref::<area::State<Paragraph>>();
                for (series, pixels) in mark.data.series.iter().zip(&state.series_points) {
                    append(&series.points, pixels);
                }
            }
            plot_area::Series::Xy(mark) if mark.data.coord_kind != CoordKind::Geo => {
                let state = child.state.downcast_ref::<xy::State>();
                append(&mark.data.points, &state.pixel_points);
            }
            plot_area::Series::Bars(mark) => {
                let state = child.state.downcast_ref::<bars::State<Paragraph>>();
                for (series, rects) in mark.data.series.iter().zip(&state.series_rects) {
                    let pixels: Vec<_> = rects.iter().map(|rect| rect.center()).collect();
                    append(&series.points, &pixels);
                }
            }
            _ => {}
        }
    }
    points.sort_by(|a, b| a.0.total_cmp(&b.0));
    points.dedup_by(|a, b| a.0 == b.0);
    points.into_iter().map(|(_, pixel)| pixel).collect()
}

pub(super) fn step(key: Named, current: Option<usize>, count: usize) -> Option<usize> {
    if count == 0 {
        return None;
    }
    let last = count - 1;
    match key {
        Named::Home => Some(0),
        Named::End => Some(last),
        Named::ArrowRight | Named::ArrowDown => Some(current.map_or(0, |index| index.saturating_add(1).min(last))),
        Named::ArrowLeft | Named::ArrowUp => Some(current.unwrap_or(0).saturating_sub(1).min(last)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{Named, step};

    #[test]
    fn navigation_stays_within_available_targets() {
        assert_eq!(step(Named::ArrowRight, None, 3), Some(0));
        assert_eq!(step(Named::ArrowRight, Some(0), 3), Some(1));
        assert_eq!(step(Named::ArrowRight, Some(2), 3), Some(2));
        assert_eq!(step(Named::ArrowLeft, Some(0), 3), Some(0));
        assert_eq!(step(Named::Home, Some(2), 3), Some(0));
        assert_eq!(step(Named::End, Some(0), 3), Some(2));
        assert_eq!(step(Named::End, None, 0), None);
        assert_eq!(step(Named::Escape, Some(1), 3), None);
    }
}
