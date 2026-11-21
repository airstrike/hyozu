//! Where a scale legend's ticks sit and what they read.
//!
//! Positions are unit-interval offsets along the key. A color bar places
//! each value through the color scale's transform, the same mapping the
//! marks sample their colors with, so a log heatmap's decades land at
//! evenly spaced positions. Labels that would overprint at the key's
//! nominal length are dropped, keeping both ends.

use super::{GLYPH_WIDTH_ESTIMATE, Tick};
use crate::data::legend::Orientation;
use crate::scale::{self, Transform};

/// Round values a color bar aims for between its ends.
const TARGET: usize = 5;
/// Interior ticks closer than this to an end (in unit position) are
/// dropped so they never crowd the end label.
const EDGE_GUARD: f32 = 0.05;
/// Key length the plan's overlap check assumes, per orientation: about the
/// shortest bar a roomy chart draws. A shorter bar on a small chart thins
/// its ticks again at draw time through [`fit`].
pub(crate) const HORIZONTAL_FIT_LENGTH: f32 = 150.0;
pub(crate) const VERTICAL_FIT_LENGTH: f32 = 135.0;
/// Height a vertical key's centered label occupies.
const LABEL_HEIGHT: f32 = 12.0;
/// Minimum clear space between neighboring labels, side by side on a
/// horizontal key and stacked on a vertical one.
const HORIZONTAL_LABEL_GAP: f32 = 6.0;
const VERTICAL_LABEL_GAP: f32 = 3.0;

/// How legend values become text.
///
/// An explicit closure (from [`crate::legend::Config::value_format`] or
/// the mark's color scale) formats each value on its own. The default
/// reads the values as a set: magnitudes of ten thousand and up shorten
/// to K/M/B, and smaller ones share the fewest decimals (at most three)
/// that write every value exactly, so contour edges such as `1.25` and
/// `1.75` read as themselves instead of rounding together.
#[derive(Clone, Default)]
pub(crate) struct Label(Option<scale::Format<f64>>);

impl Label {
    pub(crate) fn new(format: Option<scale::Format<f64>>) -> Self {
        Self(format)
    }

    pub(crate) fn all(&self, values: &[f64]) -> Vec<String> {
        if let Some(format) = &self.0 {
            return values.iter().map(|v| format(v)).collect();
        }
        let largest = values.iter().fold(0.0_f64, |m, v| m.max(v.abs()));
        let smallest = values
            .iter()
            .map(|v| v.abs())
            .filter(|v| *v > 0.0)
            .fold(f64::INFINITY, f64::min);
        if largest >= 10_000.0 || smallest < 0.001 {
            return values
                .iter()
                .map(|&v| crate::chart::plot_area::choropleth::format_legend_value(v))
                .collect();
        }
        let exact = |decimals: i32| {
            let scale = 10f64.powi(decimals);
            values
                .iter()
                .all(|v| ((v * scale).round() - v * scale).abs() < 1e-6 * scale.max(1.0))
        };
        let decimals = (0..=3).find(|&d| exact(d)).unwrap_or(3) as usize;
        values
            .iter()
            .map(|v| {
                let text = format!("{v:.decimals$}");
                // `-0.0` rounds to "-0"; a legend reads it as zero.
                if text.trim_start_matches('-').chars().all(|c| c == '0' || c == '.') {
                    text.trim_start_matches('-').to_owned()
                } else {
                    text
                }
            })
            .collect()
    }

    fn ticks(&self, marks: Vec<(f32, f64)>) -> Vec<Tick> {
        let values: Vec<f64> = marks.iter().map(|(_, v)| *v).collect();
        marks
            .into_iter()
            .zip(self.all(&values))
            .map(|((t, _), label)| Tick { t, label })
            .collect()
    }
}

/// Ticks along a color bar over `[lo, hi]`: both ends, plus round values
/// between them, each at `transform`'s unit position. A collapsed or
/// non-finite domain gets a single tick at the low end.
pub(crate) fn along(lo: f64, hi: f64, transform: Transform, label: &Label, orientation: Orientation) -> Vec<Tick> {
    if !(lo.is_finite() && hi.is_finite()) || hi <= lo {
        return label.ticks(vec![(0.0, lo)]);
    }
    let mut marks = vec![(0.0, lo)];
    marks.extend(
        round_values(lo, hi, transform, TARGET)
            .into_iter()
            .map(|v| (transform.map_to_unit(v, lo, hi) as f32, v))
            .filter(|(t, _)| *t > EDGE_GUARD && *t < 1.0 - EDGE_GUARD),
    );
    marks.push((1.0, hi));
    fit(label.ticks(marks), orientation, nominal(orientation))
}

/// Ticks at a banded field's bin edges, evenly spaced: edge `i` of `n`
/// bins sits at `i / n`, so every band gets the same share of the key.
pub(crate) fn edges(edges: &[f64], label: &Label, orientation: Orientation) -> Vec<Tick> {
    let bins = edges.len().saturating_sub(1).max(1) as f32;
    let marks = edges.iter().enumerate().map(|(i, &v)| (i as f32 / bins, v)).collect();
    fit(label.ticks(marks), orientation, nominal(orientation))
}

/// Round values inside `[lo, hi]`, ascending.
///
/// Linear and square-root scales use multiples of the same nice step the
/// axis ticks use. Log scales use decades, filled in with 2× and 5×
/// multiples when the domain spans fewer than three decades. A log
/// domain that reaches zero or below has no round values.
pub(crate) fn round_values(lo: f64, hi: f64, transform: Transform, target: usize) -> Vec<f64> {
    if !(lo.is_finite() && hi.is_finite()) || hi <= lo {
        return Vec::new();
    }
    match transform {
        Transform::Log => {
            if lo <= 0.0 {
                return Vec::new();
            }
            let decades = |multiples: &[f64]| {
                let first = lo.log10().floor() as i32;
                let last = hi.log10().ceil() as i32;
                (first..=last)
                    .flat_map(|k| multiples.iter().map(move |m| m * 10f64.powi(k)))
                    .filter(|v| *v >= lo * (1.0 - 1e-9) && *v <= hi * (1.0 + 1e-9))
                    .collect::<Vec<_>>()
            };
            let whole = decades(&[1.0]);
            if whole.len() >= 3 {
                whole
            } else {
                decades(&[1.0, 2.0, 5.0])
            }
        }
        Transform::Linear | Transform::Sqrt => {
            let step = scale::transform::nice_step(hi - lo, target);
            if !(step.is_finite() && step > 0.0) {
                return Vec::new();
            }
            let first = (lo / step - 1e-9).ceil() as i64;
            let last = (hi / step + 1e-9).floor() as i64;
            (first..=last)
                .map(|k| {
                    let v = k as f64 * step;
                    // Snap float noise such as 0.30000000000000004.
                    let snapped = (v / step).round() * step;
                    if snapped.abs() < step * 1e-9 { 0.0 } else { snapped }
                })
                .collect()
        }
    }
}

fn nominal(orientation: Orientation) -> f32 {
    match orientation {
        Orientation::Horizontal => HORIZONTAL_FIT_LENGTH,
        Orientation::Vertical => VERTICAL_FIT_LENGTH,
    }
}

/// Keeps the ticks whose labels clear their kept neighbors on a key
/// `length` pixels long. Both ends always stay.
///
/// Horizontally the ends align inward (the first label starts at the
/// bar's left edge, the last ends at its right edge) and interior labels
/// center on their ticks; vertically every label centers on its tick.
pub(crate) fn fit(ticks: Vec<Tick>, orientation: Orientation, length: f32) -> Vec<Tick> {
    if ticks.len() <= 2 {
        return ticks;
    }
    let width = |tick: &Tick| tick.label.chars().count() as f32 * GLYPH_WIDTH_ESTIMATE;
    let last = ticks.len() - 1;
    // Each label's extent along the key, in pixels.
    let extent = |i: usize, tick: &Tick| -> (f32, f32) {
        match orientation {
            Orientation::Horizontal => {
                let x = tick.t * length;
                let w = width(tick);
                if i == 0 {
                    (x, x + w)
                } else if i == last {
                    (x - w, x)
                } else {
                    (x - w / 2.0, x + w / 2.0)
                }
            }
            Orientation::Vertical => {
                let y = tick.t * length;
                (y - LABEL_HEIGHT / 2.0, y + LABEL_HEIGHT / 2.0)
            }
        }
    };
    let gap = match orientation {
        Orientation::Horizontal => HORIZONTAL_LABEL_GAP,
        Orientation::Vertical => VERTICAL_LABEL_GAP,
    };
    let end = extent(last, &ticks[last]);
    let mut kept_end = extent(0, &ticks[0]).1;
    let mut kept = Vec::with_capacity(ticks.len());
    for (i, tick) in ticks.into_iter().enumerate() {
        if i == 0 || i == last {
            kept.push(tick);
            continue;
        }
        let (start, stop) = extent(i, &tick);
        if start >= kept_end + gap && stop + gap <= end.0 {
            kept_end = stop;
            kept.push(tick);
        }
    }
    kept
}

#[cfg(test)]
mod tests {
    use super::*;

    fn positions(ticks: &[Tick]) -> Vec<(f32, &str)> {
        ticks.iter().map(|t| (t.t, t.label.as_str())).collect()
    }

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-5
    }

    #[test]
    fn linear_color_bar_places_round_values_proportionally() {
        let ticks = along(0.0, 100.0, Transform::Linear, &Label::default(), Orientation::Vertical);
        let expected = [
            (0.0, "0"),
            (0.2, "20"),
            (0.4, "40"),
            (0.6, "60"),
            (0.8, "80"),
            (1.0, "100"),
        ];
        assert_eq!(ticks.len(), expected.len(), "{:?}", positions(&ticks));
        for (tick, (t, label)) in ticks.iter().zip(expected) {
            assert!(close(tick.t, t), "{} at {} not {t}", tick.label, tick.t);
            assert_eq!(tick.label, label);
        }
    }

    #[test]
    fn log_color_bar_places_decades_evenly() {
        let ticks = along(1.0, 10_000.0, Transform::Log, &Label::default(), Orientation::Vertical);
        let expected = [(0.0, "1"), (0.25, "10"), (0.5, "100"), (0.75, "1K"), (1.0, "10K")];
        assert_eq!(ticks.len(), expected.len(), "{:?}", positions(&ticks));
        for (tick, (t, label)) in ticks.iter().zip(expected) {
            assert!(close(tick.t, t), "{} at {} not {t}", tick.label, tick.t);
            assert_eq!(tick.label, label);
        }
    }

    #[test]
    fn log_ticks_sit_where_the_transform_maps_their_values() {
        // A raw data range: ends off the decades, interior decades placed
        // by the log transform rather than linearly.
        let (lo, hi) = (2.5, 8_500.0);
        let ticks = along(lo, hi, Transform::Log, &Label::default(), Orientation::Vertical);
        let values = round_values(lo, hi, Transform::Log, TARGET);
        assert_eq!(values, vec![10.0, 100.0, 1000.0]);
        for (tick, v) in ticks[1..ticks.len() - 1].iter().zip(values) {
            assert!(close(tick.t, Transform::Log.map_to_unit(v, lo, hi) as f32));
        }
        let linear = (100.0 - lo) / (hi - lo);
        assert!(ticks[2].t > 0.4 && (ticks[2].t as f64 - linear).abs() > 0.3);
    }

    #[test]
    fn narrow_log_domain_fills_in_two_and_five() {
        assert_eq!(round_values(1.0, 60.0, Transform::Log, TARGET), vec![
            1.0, 2.0, 5.0, 10.0, 20.0, 50.0
        ]);
        assert!(round_values(0.0, 60.0, Transform::Log, TARGET).is_empty());
    }

    #[test]
    fn sqrt_color_bar_places_values_through_the_root() {
        let ticks = along(0.0, 100.0, Transform::Sqrt, &Label::default(), Orientation::Vertical);
        let tick = ticks.iter().find(|t| t.label == "40").expect("40 survives");
        assert!(close(tick.t, (0.4_f32).sqrt()));
    }

    #[test]
    fn crowded_horizontal_labels_drop_but_keep_both_ends() {
        let label = Label::new(Some(std::sync::Arc::new(|v: &f64| format!("${v:.0},000"))));
        let ticks = along(0.0, 100.0, Transform::Linear, &label, Orientation::Horizontal);
        assert_eq!(ticks.first().map(|t| t.t), Some(0.0));
        assert_eq!(ticks.last().map(|t| t.t), Some(1.0));
        assert!(ticks.len() < 6, "wide labels must thin: {:?}", positions(&ticks));
        for pair in ticks.windows(2) {
            assert!(pair[1].t > pair[0].t);
        }
    }

    #[test]
    fn edges_space_bins_evenly_and_read_exactly() {
        let ticks = edges(&[0.0, 0.25, 0.5, 2.5], &Label::default(), Orientation::Vertical);
        assert_eq!(positions(&ticks), vec![
            (0.0, "0.00"),
            (1.0 / 3.0, "0.25"),
            (2.0 / 3.0, "0.50"),
            (1.0, "2.50")
        ]);
    }

    #[test]
    fn default_labels_share_the_fewest_exact_decimals() {
        let label = Label::default();
        assert_eq!(label.all(&[0.0, 0.5, 1.0]), vec!["0.0", "0.5", "1.0"]);
        assert_eq!(label.all(&[0.0, 5.0, 10.0]), vec!["0", "5", "10"]);
        assert_eq!(label.all(&[-0.0, 20_000.0]), vec!["0", "20K"]);
    }
}
