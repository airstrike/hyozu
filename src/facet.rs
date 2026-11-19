//! Small multiples with independent chart state and optionally shared axes.
use crate::Data;
use crate::core::{Element, Length};
use crate::widget::{Column, Row, Widget};

#[derive(Clone, Copy, Debug)]
pub enum Scales {
    Fixed,
    FreeX,
    FreeY,
    Free,
}

#[derive(Clone, Debug)]
pub struct Grid {
    panels: Vec<Data>,
    columns: usize,
}

impl Grid {
    /// Panels retain their titles, palettes and secondary axes. Shared bounds
    /// are calculated from the pooled primary data once, at construction.
    pub fn new(mut panels: Vec<Data>, columns: usize, scales: Scales) -> Option<Self> {
        if columns == 0 {
            return None;
        }
        let mut ranges: Option<crate::data::area::Bounds> = None;
        for panel in panels
            .iter()
            .filter(|p| !p.marks().is_empty() && (p.x_axis_ref().is_some() || p.y_axis_ref().is_some()))
        {
            let b = panel.primary_bounds();
            let pooled = ranges.get_or_insert(b);
            pooled.x_min = pooled.x_min.min(b.x_min);
            pooled.x_max = pooled.x_max.max(b.x_max);
            pooled.y_min = pooled.y_min.min(b.y_min);
            pooled.y_max = pooled.y_max.max(b.y_max);
        }
        if let Some(b) = ranges {
            for panel in &mut panels {
                if matches!(scales, Scales::Fixed | Scales::FreeY)
                    && let Some(axis) = panel.x_axis_mut()
                {
                    *axis = axis.clone().with_bounds(b.x_min, b.x_max);
                }
                if matches!(scales, Scales::Fixed | Scales::FreeX)
                    && let Some(axis) = panel.y_axis_mut()
                {
                    *axis = axis.clone().with_bounds(b.y_min, b.y_max);
                }
                panel.invalidate();
            }
        }
        Some(Self { panels, columns })
    }

    pub fn panels(&self) -> &[Data] {
        &self.panels
    }
    pub fn panel_mut(&mut self, index: usize) -> Option<&mut Data> {
        self.panels.get_mut(index)
    }

    /// Each panel uses the caller's ordinary chart widget and event mapping.
    pub fn view_with<'a, M: 'a, T: 'a>(
        &'a self,
        mut render: impl FnMut(usize, &'a Data) -> Element<'a, M, T, crate::widget::Renderer>,
    ) -> Element<'a, M, T, crate::widget::Renderer> {
        let mut column = Column::<Element<'a, M, T, crate::widget::Renderer>>::new().spacing(6);
        for (row, chunk) in self.panels.chunks(self.columns).enumerate() {
            let mut line = Row::<Element<'a, M, T, crate::widget::Renderer>>::new()
                .spacing(6)
                .height(Length::Fill);
            for (i, panel) in chunk.iter().enumerate() {
                line = line.push(render(row * self.columns + i, panel));
            }
            for _ in chunk.len()..self.columns {
                line = line.push(crate::widget::space().width(Length::Fill).boxed());
            }
            column = column.push(line.boxed());
        }
        column.width(Length::Fill).height(Length::Fill).boxed()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pooling_only_replaces_shared_dimensions() {
        let make = || {
            vec![
                crate::data(crate::line([(1., 1.), (2., 3.)])),
                crate::data(crate::line([(5., 20.), (6., 30.)])),
            ]
        };
        let g = Grid::new(make(), 2, Scales::FreeY).expect("two facets");
        for panel in g.panels() {
            let x = panel.x_axis_ref().expect("x axis");
            assert_eq!(x.lower_bound, Some(1.));
            assert_eq!(x.upper_bound, Some(6.));
            assert_eq!(panel.y_axis_ref().expect("y axis").upper_bound, None);
        }
        let fixed = Grid::new(make(), 1, Scales::Fixed).expect("fixed axes");
        assert_eq!(fixed.panels()[0].y_axis_ref().expect("y axis").upper_bound, Some(30.));
    }
}
