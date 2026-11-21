//! Angular and radial coordinates.
//!
//! [`Axes`] configures one polar coordinate system: the value ticks and their
//! format, the angular spokes, the start angle and the direction. Lines, bars,
//! scatter, error bars, radar series and radial bars are all plotted through
//! it. The free functions use [`Axes::default`], the mathematical convention.
use crate::mark::diagram::label::Label;
use crate::mark::diagram::{Entrance, Sample};
use crate::{Diagram, RadarStyle};
use std::f64::consts::{FRAC_PI_2, PI, TAU};
use std::sync::Arc;

/// Outer ring radius in units of the square plot side.
const RADIUS: f64 = 0.38;
/// Distance past the outer ring, in units of the square side, at which
/// angular labels start.
const GAP: f64 = 0.025;
/// Segments per full turn of a circular ring.
const SEGMENTS: usize = 96;
/// Radius, in units of the outer ring, at which the innermost radial bar starts.
const HUB: f64 = 0.2;

/// An angle normalized into one turn, `[0, 2π)` radians, measured
/// counterclockwise from three o'clock.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct Angle(f64);

impl Angle {
    /// Three o'clock.
    pub const EAST: Self = Self(0.);
    /// Twelve o'clock.
    pub const NORTH: Self = Self(FRAC_PI_2);
    /// Nine o'clock.
    pub const WEST: Self = Self(PI);
    /// Six o'clock.
    pub const SOUTH: Self = Self(3. * FRAC_PI_2);

    /// Normalizes `radians` into one turn. A nonfinite angle is [`Angle::EAST`].
    pub fn radians(radians: f64) -> Self {
        let turn = radians.rem_euclid(TAU);
        // A tiny negative input rounds up to exactly one turn.
        if turn.is_finite() && turn < TAU {
            Self(turn)
        } else {
            Self::EAST
        }
    }

    /// Normalizes `degrees` into one turn. A nonfinite angle is [`Angle::EAST`].
    pub fn degrees(degrees: f64) -> Self {
        Self::radians(degrees.to_radians())
    }

    /// The angle in radians, in `[0, 2π)`.
    pub fn to_radians(self) -> f64 {
        self.0
    }

    /// The angle in degrees, in `[0, 360)`.
    pub fn to_degrees(self) -> f64 {
        self.0.to_degrees()
    }
}

/// The way increasing data angles turn on screen.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Direction {
    #[default]
    Counterclockwise,
    Clockwise,
}

impl Direction {
    fn sign(self) -> f64 {
        match self {
            Self::Counterclockwise => 1.,
            Self::Clockwise => -1.,
        }
    }
}

/// Spokes for angle-valued data: lines, bars, scatter and error bars.
#[derive(Clone, Debug, PartialEq)]
pub enum Angular {
    /// Evenly spaced spokes labelled with their data angle in degrees.
    Degrees(usize),
    /// Evenly spaced named spokes. See [`Axes::spoke`] for their data angles.
    Categories(Vec<String>),
}

/// A polar coordinate system shared by every polar layout.
///
/// The value scale is radial for radar, lines, bars, scatter and error bars,
/// and angular for radial bars, whose sweep encodes the value. Radar and
/// radial bars name their own categories, so [`Angular`] spokes apply to
/// angle-valued data only.
///
/// ```ignore
/// use hyozu::polar::{Angle, Axes, Direction};
///
/// let wind = Axes::new()
///     .start(Angle::NORTH)
///     .direction(Direction::Clockwise)
///     .categories(["N", "E", "S", "W"])
///     .ticks(3)
///     .format(|v| format!("{v:.0} kn"))
///     .lines(&[("Gusts", samples)]);
/// ```
#[derive(Clone)]
pub struct Axes {
    start: Angle,
    direction: Direction,
    angular: Angular,
    ticks: usize,
    maximum: Option<f64>,
    format: Option<Arc<dyn Fn(f64) -> String + Send + Sync>>,
}

impl Default for Axes {
    fn default() -> Self {
        Self {
            start: Angle::EAST,
            direction: Direction::Counterclockwise,
            angular: Angular::Degrees(8),
            ticks: 4,
            maximum: None,
            format: None,
        }
    }
}

impl std::fmt::Debug for Axes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Axes")
            .field("start", &self.start)
            .field("direction", &self.direction)
            .field("angular", &self.angular)
            .field("ticks", &self.ticks)
            .field("maximum", &self.maximum)
            .field("format", &self.format.as_ref().map(|_| "<function>"))
            .finish()
    }
}

/// The value scale after resolving its maximum against the data.
#[derive(Clone, Debug, PartialEq)]
struct Scale {
    maximum: f64,
    /// Positive tick values in increasing order, none beyond `maximum`.
    ticks: Vec<f64>,
}

#[derive(Clone, Copy)]
enum Rings {
    Circles,
    /// Straight segments between consecutive spokes, as on a radar.
    Polygons,
}

impl Axes {
    /// Zero at three o'clock, growing counterclockwise, with eight spokes
    /// labelled in degrees and about four value ticks.
    pub fn new() -> Self {
        Self::default()
    }

    /// Zero at twelve o'clock, growing clockwise, like a compass or a clock.
    pub fn compass() -> Self {
        Self {
            start: Angle::NORTH,
            direction: Direction::Clockwise,
            ..Self::default()
        }
    }

    /// Screen angle at which data angle zero points.
    pub fn start(mut self, start: Angle) -> Self {
        self.start = start;
        self
    }

    /// The way increasing data angles turn.
    pub fn direction(mut self, direction: Direction) -> Self {
        self.direction = direction;
        self
    }

    /// Evenly spaced spokes labelled in degrees, between 1 and 360 of them.
    pub fn spokes(mut self, count: usize) -> Self {
        self.angular = Angular::Degrees(count.clamp(1, 360));
        self
    }

    /// Evenly spaced named spokes in place of degrees.
    pub fn categories<S: Into<String>>(mut self, names: impl IntoIterator<Item = S>) -> Self {
        self.angular = Angular::Categories(names.into_iter().map(Into::into).collect());
        self
    }

    /// The approximate number of value ticks. Steps snap to 1, 2 or 5 times
    /// a power of ten.
    pub fn ticks(mut self, count: usize) -> Self {
        self.ticks = count.max(1);
        self
    }

    /// Fixes the outer value of the scale. A nonpositive or nonfinite
    /// maximum restores the default, which rounds the data maximum up to the
    /// next tick. Values beyond a fixed maximum extend past the outer ring,
    /// except on radar and radial bars, which stop at it.
    pub fn maximum(mut self, maximum: f64) -> Self {
        self.maximum = (maximum.is_finite() && maximum > 0.).then_some(maximum);
        self
    }

    /// Formats value tick labels.
    pub fn format(mut self, format: impl Fn(f64) -> String + Send + Sync + 'static) -> Self {
        self.format = Some(Arc::new(format));
        self
    }

    /// The data angle, in radians, of spoke `index`. Spokes divide the turn
    /// evenly, so with categories this is the angle at which category
    /// `index` sits.
    pub fn spoke(&self, index: usize) -> f64 {
        let count = match &self.angular {
            Angular::Degrees(count) => *count,
            Angular::Categories(names) => names.len(),
        };
        index as f64 * TAU / count.max(1) as f64
    }

    /// Plot named series of [angle in radians, nonnegative radius] pairs.
    /// All series share the value scale. Missing samples break the line, and
    /// hover snaps to the observations.
    pub fn lines<S: AsRef<str>, V: AsRef<[[f64; 2]]>>(&self, series: &[(S, V)]) -> Diagram {
        let valid = |p: &[f64; 2]| p[0].is_finite() && p[1].is_finite() && p[1] >= 0.;
        let maximum = series
            .iter()
            .flat_map(|(_, v)| v.as_ref())
            .filter(|p| valid(p))
            .map(|p| p[1])
            .fold(0., f64::max);
        let scale = self.scale(maximum);
        let mut d = self.frame(&scale, &self.angles(), Rings::Circles);
        for (name, values) in series {
            let id = d.series(name.as_ref());
            for run in values.as_ref().split(|p| !valid(p)).filter(|run| run.len() >= 2) {
                let points: Vec<_> = run.iter().map(|&[a, r]| self.place(a, r / scale.maximum)).collect();
                let samples = points
                    .iter()
                    .zip(run)
                    .map(|(&at, &[angle, radius])| (at, Sample::Polar { angle, radius }))
                    .collect();
                d.trace(id, points, samples);
            }
        }
        d
    }

    /// Angular bars with independently supplied angles, widths and radial intervals.
    pub fn bars<S: AsRef<str>>(&self, bars: &[(S, Bar)]) -> Option<Diagram> {
        if bars.is_empty() || bars.iter().any(|(_, b)| !b.valid()) {
            return None;
        }
        let scale = self.scale(bars.iter().map(|(_, b)| b.outer).fold(0., f64::max));
        let mut d = self.frame(&scale, &self.angles(), Rings::Circles);
        for (name, b) in bars {
            let id = d.entry(name.as_ref(), b.outer - b.inner);
            let from = b.angle - b.width / 2.;
            let points = self.arc(from, from + b.width, b.inner / scale.maximum, b.outer / scale.maximum);
            d.path(id, points, true, 1.);
        }
        Some(d)
    }

    /// Scatter observations in radians and radius, sharing one scale across named groups.
    pub fn scatter<S: AsRef<str>, V: AsRef<[[f64; 2]]>>(&self, groups: &[(S, V)]) -> Diagram {
        let valid = |p: &[f64; 2]| p[0].is_finite() && p[1].is_finite() && p[1] >= 0.;
        let maximum = groups
            .iter()
            .flat_map(|(_, v)| v.as_ref())
            .filter(|p| valid(p))
            .map(|p| p[1])
            .fold(0., f64::max);
        let scale = self.scale(maximum);
        let mut d = self.frame(&scale, &self.angles(), Rings::Circles);
        for (name, values) in groups {
            let id = d.series(name.as_ref());
            for &[angle, radius] in values.as_ref().iter().filter(|p| valid(p)) {
                let at = self.place(angle, radius / scale.maximum);
                d.sampled(id, disc(at, 0.008, 20), true, 1., vec![(at, Sample::Polar {
                    angle,
                    radius,
                })]);
            }
        }
        d
    }

    /// Angular and radial uncertainty around each observation. See [`Uncertainty`].
    pub fn error_bars(&self, samples: &[Uncertainty]) -> Option<Diagram> {
        if samples.is_empty() || samples.iter().any(|p| !p.valid()) {
            return None;
        }
        let scale = self.scale(samples.iter().map(|p| p.radius + p.radial_error).fold(0., f64::max));
        let mut d = self.frame(&scale, &self.angles(), Rings::Circles);
        let point = |a: f64, r: f64| self.place(a, r / scale.maximum);
        for (i, p) in samples.iter().enumerate() {
            let id = d.entry(&format!("Observation {}", i + 1), p.radius);
            if p.radial_error > 0. {
                d.path(
                    id,
                    vec![
                        point(p.angle, (p.radius - p.radial_error).max(0.)),
                        point(p.angle, p.radius + p.radial_error),
                    ],
                    false,
                    1.,
                );
            }
            if p.angular_error > 0. {
                let n = (128. * p.angular_error / TAU).ceil().max(2.) as usize;
                d.path(
                    id,
                    (0..=n)
                        .map(|i| point(p.angle + p.angular_error * (2. * i as f64 / n as f64 - 1.), p.radius))
                        .collect(),
                    false,
                    1.,
                );
            }
            d.path(id, disc(point(p.angle, p.radius), 0.006, 16), true, 1.);
        }
        Some(d)
    }

    /// One closed series per row of values, one value per spoke, with the
    /// spokes in input order. Polygonal rings mark the value ticks. Values
    /// clamp to the scale. Incomplete series and nonfinite values are
    /// omitted, and fewer than three spokes draw nothing.
    pub fn radar<S: AsRef<str>, T: AsRef<str>, V: AsRef<[f64]>>(
        &self,
        spokes: &[S],
        series: &[(T, V)],
        style: RadarStyle,
    ) -> Diagram {
        if spokes.len() < 3 {
            return Diagram {
                square: true,
                entrance: Entrance::Radial,
                ..Diagram::default()
            };
        }
        let complete = |v: &[f64]| v.len() == spokes.len() && v.iter().all(|v| v.is_finite());
        let maximum = series
            .iter()
            .map(|(_, v)| v.as_ref())
            .filter(|v| complete(v))
            .flatten()
            .fold(0., |m: f64, v| m.max(*v));
        let scale = self.scale(maximum);
        let angles: Vec<_> = spokes
            .iter()
            .enumerate()
            .map(|(i, s)| (i as f64 * TAU / spokes.len() as f64, s.as_ref().to_owned()))
            .collect();
        let mut d = self.frame(&scale, &angles, Rings::Polygons);
        d.spokes = angles.iter().map(|(_, name)| name.clone()).collect();
        for (name, values) in series {
            let values = values.as_ref();
            if !complete(values) {
                continue;
            }
            let id = d.series(name.as_ref());
            let mut points: Vec<_> = values
                .iter()
                .zip(&angles)
                .map(|(v, (angle, _))| self.place(*angle, (v / scale.maximum).clamp(0., 1.)))
                .collect();
            // Hover reports the spoke value under the cursor rather than
            // the whole series.
            let samples: Vec<_> = points
                .iter()
                .zip(values)
                .enumerate()
                .map(|(spoke, (&at, &value))| (at, Sample::Spoke { spoke, value }))
                .collect();
            if style == RadarStyle::Filled {
                d.sampled(id, points, true, 0.33, samples);
                continue;
            }
            if style == RadarStyle::Markers {
                for &(at, sample) in &samples {
                    d.sampled(id, disc(at, 0.009, 16), true, 1., vec![(at, sample)]);
                }
            }
            points.push(points[0]);
            d.trace(id, points, samples);
        }
        d
    }

    /// One concentric bar per row, innermost first. The swept angle encodes
    /// the value as a fraction of the scale's maximum, from the start angle
    /// in the configured direction, and stops at one full turn. Value ticks
    /// become labelled spokes, and each row is named just before its bar
    /// starts. Negative and nonfinite rows are omitted.
    pub fn radial_bars<S: AsRef<str>>(&self, rows: &[(S, f64)]) -> Diagram {
        let rows: Vec<_> = rows.iter().filter(|(_, v)| v.is_finite() && *v >= 0.).collect();
        let scale = self.scale(rows.iter().map(|(_, v)| *v).fold(0., f64::max));
        let mut d = Diagram {
            square: true,
            entrance: Entrance::Radial,
            ..Diagram::default()
        };
        let f32s = |p: [f64; 2]| p.map(|v| v as f32);
        d.guide(
            (0..=SEGMENTS)
                .map(|i| f32s(self.place(i as f64 * TAU / SEGMENTS as f64, 1.)))
                .collect(),
        );
        let values =
            std::iter::once(0.).chain(scale.ticks.iter().copied().filter(|t| *t < scale.maximum * (1. - 1e-9)));
        for value in values {
            let angle = value / scale.maximum * TAU;
            d.guide(vec![f32s(self.place(angle, HUB)), f32s(self.place(angle, 1.))]);
            d.labels.push(Label::toward(
                f32s(self.place(angle, 1. + GAP / RADIUS)),
                self.outward(angle),
                self.text(value),
            ));
        }
        let thickness = (1. - HUB) / rows.len().max(1) as f64;
        // Against the sweep at its start: the side a row name reads from.
        let phi = self.screen(0.);
        let back = [
            (self.direction.sign() * phi.sin()) as f32,
            (self.direction.sign() * phi.cos()) as f32,
        ];
        for (i, (name, v)) in rows.into_iter().enumerate() {
            let id = d.entry(name.as_ref(), *v);
            let inner = HUB + i as f64 * thickness;
            let outer = inner + 0.86 * thickness;
            let [x, y] = f32s(self.place(0., inner.midpoint(outer)));
            d.labels
                .push(Label::toward([x + back[0] * 0.01, y + back[1] * 0.01], back, name.as_ref()).of(id));
            if *v > 0. {
                let sweep = (v / scale.maximum).min(1.) * TAU;
                d.path(id, self.arc(0., sweep, inner, outer), true, 1.);
            }
        }
        d
    }

    /// The value scale resolved against the largest data value.
    fn scale(&self, data: f64) -> Scale {
        let count = self.ticks.max(1);
        let (maximum, step) = match self.maximum {
            Some(maximum) => (maximum, crate::scale::transform::nice_step(maximum, count)),
            None => {
                let top = if data.is_finite() && data > 0. { data } else { 1. };
                let step = crate::scale::transform::nice_step(top, count);
                // The epsilon keeps an exact multiple from rounding up a step.
                ((top / step - 1e-9).ceil() * step, step)
            }
        };
        let ticks = (1..=1000)
            .map(|k| k as f64 * step)
            .take_while(|v| *v <= maximum * (1. + 1e-9))
            .collect();
        Scale { maximum, ticks }
    }

    /// Screen angle (counterclockwise from east, y up) of a data angle.
    fn screen(&self, angle: f64) -> f64 {
        self.start.0 + self.direction.sign() * angle
    }

    /// Normalized position of a data angle at `unit` times the outer radius.
    fn place(&self, angle: f64, unit: f64) -> [f64; 2] {
        let phi = self.screen(angle);
        [0.5 + RADIUS * unit * phi.cos(), 0.5 - RADIUS * unit * phi.sin()]
    }

    /// Screen-space unit vector (y down) pointing away from the center.
    fn outward(&self, angle: f64) -> [f32; 2] {
        let phi = self.screen(angle);
        [phi.cos() as f32, -phi.sin() as f32]
    }

    /// Closed annular sector between two data angles.
    fn arc(&self, from: f64, to: f64, inner: f64, outer: f64) -> Vec<[f64; 2]> {
        let n = (SEGMENTS as f64 * 1.5 * (to - from).abs() / TAU).ceil().max(2.) as usize;
        let angle = |i: usize| from + (to - from) * i as f64 / n as f64;
        let mut points: Vec<_> = (0..=n).map(|i| self.place(angle(i), outer)).collect();
        points.extend((0..=n).rev().map(|i| self.place(angle(i), inner)));
        points
    }

    fn text(&self, value: f64) -> String {
        self.format
            .as_ref()
            .map_or_else(|| crate::scale::default_f64_format(value), |format| format(value))
    }

    /// Spokes for angle-valued data.
    fn angles(&self) -> Vec<(f64, String)> {
        match &self.angular {
            Angular::Degrees(count) => (0..*count)
                .map(|i| {
                    let angle = self.spoke(i);
                    (
                        angle,
                        format!("{}°", crate::scale::default_f64_format(angle.to_degrees())),
                    )
                })
                .collect(),
            Angular::Categories(names) => names
                .iter()
                .enumerate()
                .map(|(i, name)| (self.spoke(i), name.clone()))
                .collect(),
        }
    }

    /// Rings at every value tick plus the outer edge, spokes with outward
    /// labels, and tick labels along the bisector of the first spoke gap.
    fn frame(&self, scale: &Scale, spokes: &[(f64, String)], rings: Rings) -> Diagram {
        let mut d = Diagram {
            square: true,
            entrance: Entrance::Radial,
            ..Diagram::default()
        };
        let f32s = |p: [f64; 2]| p.map(|v| v as f32);
        let mut levels: Vec<_> = scale.ticks.iter().map(|t| t / scale.maximum).collect();
        if levels.last().is_none_or(|l| *l < 1. - 1e-9) {
            levels.push(1.);
        }
        for unit in levels {
            d.guide(match rings {
                Rings::Polygons if spokes.len() >= 3 => spokes
                    .iter()
                    .chain(spokes.first())
                    .map(|(angle, _)| f32s(self.place(*angle, unit)))
                    .collect(),
                _ => (0..=SEGMENTS)
                    .map(|i| f32s(self.place(i as f64 * TAU / SEGMENTS as f64, unit)))
                    .collect(),
            });
        }
        for (angle, name) in spokes {
            d.guide(vec![[0.5, 0.5], f32s(self.place(*angle, 1.))]);
            if !name.is_empty() {
                d.labels.push(Label::toward(
                    f32s(self.place(*angle, 1. + GAP / RADIUS)),
                    self.outward(*angle),
                    name.as_str(),
                ));
            }
        }
        let along = match spokes {
            [first, second, ..] => first.0.midpoint(second.0),
            _ => TAU / 16.,
        };
        // On the bisector a polygonal ring's edge runs inside the circle
        // through its corners.
        let edge = match rings {
            Rings::Polygons if spokes.len() >= 3 => (PI / spokes.len() as f64).cos(),
            _ => 1.,
        };
        // Each tick reads just outside its ring, clear of the ring's line.
        for &tick in &scale.ticks {
            let anchor = f32s(self.place(along, edge * tick / scale.maximum + 0.01 / RADIUS));
            d.labels
                .push(Label::toward(anchor, self.outward(along), self.text(tick)));
        }
        d
    }
}

fn disc([x, y]: [f64; 2], radius: f64, segments: u32) -> Vec<[f64; 2]> {
    (0..segments)
        .map(|i| {
            let t = f64::from(i) * TAU / f64::from(segments);
            [x + radius * t.cos(), y + radius * t.sin()]
        })
        .collect()
}

fn project(angle: f64, radius: f64) -> [f64; 2] {
    [0.5 + radius * angle.cos(), 0.5 - radius * angle.sin()]
}

/// Plot named series of [angle in radians, nonnegative radius] pairs.
/// Zero points east and angles grow counterclockwise. All series share the
/// same linear radial scale. Missing samples break the line.
pub fn lines<S: AsRef<str>, V: AsRef<[[f64; 2]]>>(series: &[(S, V)]) -> Diagram {
    Axes::default().lines(series)
}

/// An annular sector centered on an angle (radians). Angular width is also in radians.
#[derive(Clone, Copy, Debug)]
pub struct Bar {
    pub angle: f64,
    pub width: f64,
    pub inner: f64,
    pub outer: f64,
}
impl Bar {
    fn valid(self) -> bool {
        self.angle.is_finite()
            && self.width.is_finite()
            && self.width > 0.
            && self.width <= TAU
            && self.inner.is_finite()
            && self.inner >= 0.
            && self.outer.is_finite()
            && self.outer > self.inner
    }
    fn polygon(self, scale: f64) -> Vec<[f64; 2]> {
        let n = (96. * self.width / TAU).ceil().max(2.) as usize;
        let angle = |i| self.angle + self.width * (i as f64 / n as f64 - 0.5);
        let mut points: Vec<_> = (0..=n).map(|i| project(angle(i), self.outer * scale)).collect();
        points.extend((0..=n).rev().map(|i| project(angle(i), self.inner * scale)));
        points
    }
}
/// Angular bars with independently supplied angles, widths and radial intervals.
pub fn bars<S: AsRef<str>>(bars: &[(S, Bar)]) -> Option<Diagram> {
    Axes::default().bars(bars)
}

/// Scatter observations in radians and radius, sharing one scale across named groups.
pub fn scatter<S: AsRef<str>, V: AsRef<[[f64; 2]]>>(groups: &[(S, V)]) -> Diagram {
    Axes::default().scatter(groups)
}

/// Fixed 17-segment left-ventricle layout. Values control color independently
/// of wedge areas. Segments 1, 7 and 13 start at the top, increasing
/// counterclockwise within each ring, and segment 17 occupies the center.
pub fn bullseye(values: &[f64; 17]) -> Option<Diagram> {
    if values.iter().any(|v| !v.is_finite()) {
        return None;
    }
    let mut d = Diagram {
        square: true,
        entrance: Entrance::Radial,
        ..Diagram::default()
    };
    for (offset, n, inner, outer) in [
        (0, 6, 11. / 15., 1.),
        (6, 6, 7. / 15., 11. / 15.),
        (12, 4, 0.2, 7. / 15.),
        (16, 1, 0., 0.2),
    ] {
        for i in 0..n {
            let idx = offset + i;
            let id = d.entry(&format!("Segment {}", idx + 1), values[idx]);
            let width = TAU / n as f64;
            let b = Bar {
                angle: TAU / 4. + width * i as f64,
                width,
                inner,
                outer,
            };
            d.path(id, b.polygon(0.45), true, 1.);
        }
    }
    d.scalar_colors();
    Some(d)
}

/// Angular and radial uncertainties around an observation. Both half-widths
/// are nonnegative. Angles are radians, and lower radial bounds stop at zero.
#[derive(Clone, Copy, Debug)]
pub struct Uncertainty {
    pub angle: f64,
    pub radius: f64,
    pub angular_error: f64,
    pub radial_error: f64,
}

impl Uncertainty {
    fn valid(&self) -> bool {
        self.angle.is_finite()
            && self.radius.is_finite()
            && self.radius >= 0.
            && (0. ..=TAU / 2.).contains(&self.angular_error)
            && self.radial_error.is_finite()
            && self.radial_error >= 0.
            && (self.radius + self.radial_error).is_finite()
    }
}

/// Angular and radial uncertainty around each observation.
pub fn error_bars(samples: &[Uncertainty]) -> Option<Diagram> {
    Axes::default().error_bars(samples)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: [f64; 2], b: [f64; 2]) -> bool {
        (a[0] - b[0]).abs() < 1e-9 && (a[1] - b[1]).abs() < 1e-9
    }

    #[test]
    fn unequal_angles_preserve_radius_and_missing_samples_break_the_line() {
        let points = [[0., 2.], [TAU / 8., 1.], [0., f64::NAN], [TAU / 4., 2.], [TAU / 2., 2.]];
        let d = lines(&[("Sensor", points)]);
        let paths: Vec<_> = d.shapes.iter().filter(|s| s.entry.is_some()).collect();
        assert_eq!(paths.len(), 2);
        let diagonal = RADIUS / 2. / 2f64.sqrt();
        for (got, want) in paths.iter().flat_map(|s| &s.points).zip([
            [0.5 + RADIUS, 0.5],
            [0.5 + diagonal, 0.5 - diagonal],
            [0.5, 0.5 - RADIUS],
            [0.5 - RADIUS, 0.5],
        ]) {
            assert!(close(*got, want), "{got:?} != {want:?}");
        }
    }

    #[test]
    fn angles_normalize_into_one_turn() {
        assert!((Angle::degrees(-90.).to_radians() - Angle::SOUTH.to_radians()).abs() < 1e-12);
        assert!((Angle::degrees(450.).to_radians() - FRAC_PI_2).abs() < 1e-12);
        assert_eq!(Angle::radians(-1e-18), Angle::EAST);
        assert_eq!(Angle::radians(f64::NAN), Angle::EAST);
        assert!((0. ..TAU).contains(&Angle::radians(-7. * TAU - 0.5).to_radians()));
    }

    #[test]
    fn ticks_round_the_data_maximum_up_to_a_nice_step() {
        let scale = Axes::new().scale(4.3);
        assert_eq!(scale.maximum, 5.);
        assert_eq!(scale.ticks, [1., 2., 3., 4., 5.]);
        let scale = Axes::new().ticks(3).scale(0.3);
        assert!((scale.maximum - 0.3).abs() < 1e-12);
        assert_eq!(scale.ticks.len(), 3);
        assert_eq!(Axes::new().scale(0.).maximum, 1.);
        assert_eq!(Axes::new().scale(f64::NAN).maximum, 1.);
    }

    #[test]
    fn a_fixed_maximum_bounds_the_ticks_without_rounding() {
        let scale = Axes::new().maximum(7.).scale(100.);
        assert_eq!(scale.maximum, 7.);
        assert_eq!(scale.ticks, [2., 4., 6.]);
        let scale = Axes::new().maximum(100.).ticks(5).scale(3.);
        assert_eq!(scale.ticks, [20., 40., 60., 80., 100.]);
        // An invalid maximum falls back to the data.
        assert_eq!(Axes::new().maximum(-1.).scale(3.).maximum, 3.);
    }

    #[test]
    fn start_and_direction_rotate_the_angular_axis() {
        let east = Axes::new();
        assert!(close(east.place(0., 1.), [0.5 + RADIUS, 0.5]));
        assert!(close(east.place(FRAC_PI_2, 1.), [0.5, 0.5 - RADIUS]));
        let compass = Axes::compass();
        assert!(close(compass.place(0., 1.), [0.5, 0.5 - RADIUS]));
        assert!(close(compass.place(FRAC_PI_2, 1.), [0.5 + RADIUS, 0.5]));
        let west = Axes::new().start(Angle::WEST).direction(Direction::Clockwise);
        assert!(close(west.place(FRAC_PI_2, 0.5), [0.5, 0.5 - RADIUS / 2.]));
        assert!(close(west.place(PI, 1.), [0.5 + RADIUS, 0.5]));
    }

    #[test]
    fn category_spokes_are_evenly_spaced_and_labelled_outward() {
        let axes = Axes::compass().categories(["N", "E", "S", "W"]);
        assert!((axes.spoke(1) - FRAC_PI_2).abs() < 1e-12);
        let d = axes.lines(&[("Wind", [[0., 1.], [axes.spoke(1), 1.]])]);
        let names: Vec<_> = d.labels.iter().map(|l| l.text.as_str()).collect();
        assert_eq!(&names[..4], ["N", "E", "S", "W"]);
        // "E" sits to the right of the ring and reads away from it.
        let east = &d.labels[1];
        assert!(east.anchor[0] > 0.5 + RADIUS as f32);
        assert!(matches!(east.place, crate::mark::diagram::label::Place::Toward([x, _]) if x > 0.99));
        // Ticks follow the four categories, along the bisector of N and E.
        assert_eq!(&names[4..], ["0.2", "0.4", "0.6", "0.8", "1"]);
    }

    #[test]
    fn polar_line_vertices_carry_their_observations() {
        let d = Axes::compass().lines(&[("Gust", [[0., 3.], [FRAC_PI_2, 6.]])]);
        let path = d.shapes.iter().find(|s| s.entry.is_some()).expect("traced path");
        assert_eq!(path.samples.len(), 2);
        assert_eq!(path.samples[1].1, Sample::Polar {
            angle: FRAC_PI_2,
            radius: 6.
        });
        assert!(close(path.samples[1].0, path.points[1]));
    }

    #[test]
    fn radial_bars_sweep_from_the_start_and_label_value_ticks() {
        let d = Axes::compass()
            .maximum(100.)
            .radial_bars(&[("Done", 50.), ("Idle", 0.)]);
        assert_eq!(d.entries.len(), 2);
        let bar = d.shapes.iter().find(|s| s.entry == Some(0)).expect("bar");
        // Half a turn clockwise from twelve o'clock ends at six o'clock.
        let lowest = bar.points.iter().map(|p| p[1]).fold(0., f64::max);
        assert!(lowest > 0.5 && bar.points.iter().all(|p| p[0] >= 0.5 - 1e-9));
        let text: Vec<_> = d.labels.iter().map(|l| l.text.as_str()).collect();
        assert_eq!(text, ["0", "20", "40", "60", "80", "Done", "Idle"]);
    }
}

#[cfg(test)]
mod layouts {
    use super::*;
    #[test]
    fn fixed_ring_sector_geometry_is_independent_of_its_value() {
        let d = bullseye(&std::array::from_fn(|i| i as f64)).expect("seventeen segments");
        assert_eq!(d.color_domain, Some((0., 16.)));
        assert_eq!(d.shapes.len(), 17);
        let span = |i: usize| {
            d.shapes[i]
                .points
                .iter()
                .map(|p| (p[0] - 0.5).hypot(p[1] - 0.5))
                .fold(0., f64::max)
        };
        assert!((span(0) - 0.45).abs() < 1e-10);
        assert!((span(16) - 0.09).abs() < 1e-10);
        assert_eq!(d.entries[0].name, "Segment 1");
        let top = &d.shapes[0].points[8];
        assert!((top[0] - 0.5).abs() < 1e-10 && top[1] < 0.1);
    }
    #[test]
    fn polar_observations_have_independent_angle_width_radius_and_uncertainty() {
        let b = Bar {
            angle: TAU / 8.,
            width: TAU / 8.,
            inner: 2.,
            outer: 4.,
        };
        let d = bars(&[("sample", b)]).expect("valid bar");
        assert_eq!(d.entries[0].value, 2.);
        let s = scatter(&[("sample", [[0., 2.], [TAU / 4., 1.]])]);
        assert_eq!(s.shapes.iter().filter(|s| s.closed).count(), 2);
        let e = error_bars(&[Uncertainty {
            angle: 0.,
            radius: 2.,
            angular_error: TAU / 8.,
            radial_error: 1.,
        }])
        .expect("uncertainties");
        let shapes: Vec<_> = e.shapes.iter().filter(|s| s.entry.is_some()).collect();
        assert_eq!(shapes.len(), 3);
        assert_eq!(shapes[0].points.len(), 2);
        assert!(shapes[1].points.len() > 2 && shapes[2].closed);
    }
}
