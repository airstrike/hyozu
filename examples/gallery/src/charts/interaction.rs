use hyozu::brush::Region;
use hyozu::chart::zoom::Viewport;
use hyozu::sync::Cursor;
use hyozu::{Action, Data, Target, bars, data, line, xy};

/// Three charts over the same twelve months, hovered together.
pub struct Linked {
    pub charts: Vec<Data>,
    pub cursor: Cursor,
}

impl Linked {
    pub fn perform(&mut self, action: &Action) {
        self.cursor.update(action);
    }
}

pub fn linked() -> Linked {
    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let sessions: [f64; 12] = [42., 45., 51., 49., 58., 63., 61., 68., 72., 70., 79., 86.];
    let conversion = [2.1, 2.3, 2.2, 2.6, 2.8, 2.7, 3.0, 3.2, 3.1, 3.4, 3.6, 3.9];
    let orders: Vec<f64> = sessions
        .iter()
        .zip(conversion)
        .map(|(s, c)| (s * c * 10.).round())
        .collect();
    Linked {
        charts: vec![
            data(line(sessions)).title("Sessions (k)").x_axis_labels(MONTHS),
            data(line(conversion)).title("Conversion (%)").x_axis_labels(MONTHS),
            data(bars(orders)).title("Orders").x_axis_labels(MONTHS),
        ],
        cursor: Cursor::IDLE,
    }
}

/// A scatter on a log axis that zooms, pans and selects.
pub struct Zoomable {
    pub chart: Data,
    points: Vec<(f64, f64)>,
    pub viewport: Viewport,
    pub selected: Option<Region>,
    /// Whether a plain drag selects instead of panning.
    pub selecting: bool,
}

impl Zoomable {
    pub fn perform(&mut self, action: &Action) {
        match action {
            Action::Viewport(viewport) => self.viewport = *viewport,
            Action::Selected(region) => {
                // Highlight the selected points of the base scatter with a
                // second, darker mark on top.
                let picked: Vec<(f64, f64)> = region
                    .targets()
                    .iter()
                    .filter_map(|target| match target {
                        Target::Entry { mark: 0, index, .. } => self.points.get(*index).copied(),
                        _ => None,
                    })
                    .collect();
                self.chart = scatter(&self.points, &picked);
                self.selected = Some(region.clone());
            }
            _ => {}
        }
    }

    pub fn status(&self) -> String {
        let range = |r: Option<[f64; 2]>| r.map_or("full".to_owned(), |[lo, hi]| format!("{lo:.1} to {hi:.1}"));
        let selected = self.selected.as_ref().map_or(0, |r| {
            r.targets()
                .iter()
                .filter(|t| matches!(t, Target::Entry { mark: 0, .. }))
                .count()
        });
        format!(
            "x {} · y {} · {selected} selected — scroll to zoom, double-click to reset",
            range(self.viewport.x()),
            range(self.viewport.y()),
        )
    }
}

fn scatter(points: &[(f64, f64)], picked: &[(f64, f64)]) -> Data {
    let mut marks = vec![hyozu::Mark::Xy(xy(points.to_vec()).with_name("Cells").color(0x7fb7ae))];
    if !picked.is_empty() {
        let ring = hyozu::mark::xy::Marker::new()
            .size(9.0)
            .stroke(0xffffff)
            .stroke_width(1.5);
        marks.push(hyozu::Mark::Xy(
            xy(picked.to_vec()).with_name("Selected").color(0xc2410c).markers(ring),
        ));
    }
    data(marks)
        .title("Colony growth")
        .y_log()
        .x_axis(|a| a.with_kind(hyozu::axis::Kind::Scalar).title("Hour"))
        .y_axis(|a| a.title("Cells"))
}

pub fn zoomable() -> Zoomable {
    // Exponential growth with deterministic jitter spans four decades.
    let points: Vec<(f64, f64)> = (0..120)
        .map(|i| {
            let t = f64::from(i) * 0.4;
            let jitter = ((f64::from(i) * 12.9898).sin() * 43_758.545).fract() - 0.5;
            (t, 3.0 * (0.19 * t + 0.6 * jitter).exp())
        })
        .collect();
    Zoomable {
        chart: scatter(&points, &[]),
        points,
        viewport: Viewport::default(),
        selected: None,
        selecting: false,
    }
}
