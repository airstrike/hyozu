use hyozu::{Data, data, field, polar, specialty};
use std::f64::consts::TAU;

pub fn hinton() -> Data {
    let w: Vec<_> = (0..80)
        .map(|i| (f64::from(i) * 0.8).sin() * (f64::from(i / 10) * 0.9).cos())
        .collect();
    data(specialty::hinton(8, 10, &w).expect("rectangular matrix"))
        .title("Connection weights · teal positive, rust negative")
}
pub fn fishbone() -> Data {
    let groups = [
        ("Equipment", ["Pump", "Valve"]),
        ("Materials", ["Gasket", "Seal"]),
        ("Process", ["Timing", "Pressure"]),
        ("Environment", ["Dust", "Heat"]),
    ];
    data(specialty::fishbone("Leak", &groups)).title("Possible causes of a leak")
}
pub fn bullseye() -> Data {
    let v = std::array::from_fn(|i| 50. + 20. * (i as f64 * 0.75).sin());
    data(polar::bullseye(&v).expect("17 segment values"))
        .palette(hyozu::palette::gradient([0xe2f0e9, 0x72b69c, 0x176e67]))
        .title("Synthetic segment intensities")
}
pub fn skew_t() -> Data {
    let p = [1000., 925., 850., 700., 600., 500., 400., 300., 200., 100.];
    let dry = [25., 19., 15., 4., -2., -13., -26., -43., -54., -62.];
    let dew = [20., 17., 8., -3., -8., -22., -29., -51., -60., -75.];
    let at = |temps: [f64; 10]| temps.into_iter().zip(p).map(|(t, p)| [t, p]).collect::<Vec<_>>();
    specialty::skew_t(
        &[("Temperature", at(dry)), ("Dew point", at(dew))],
        [-48., 40.],
        [1000., 100.],
        48.,
    )
    .expect("positive pressure")
    .legend(hyozu::legend::Anchor::Bottom)
    .title("Atmospheric sounding")
}
pub fn streamgraph() -> Data {
    let x: Vec<_> = (0..80).map(f64::from).collect();
    let bands: Vec<_> = ["North", "East", "South", "West"]
        .into_iter()
        .enumerate()
        .map(|(i, s)| {
            (
                s,
                x.iter()
                    .map(|&x| {
                        let t = x / 79.;
                        0.4 + ((TAU * t + i as f64 * 1.2).sin() + 1.) * (std::f64::consts::PI * t).sin() * 3.
                    })
                    .collect::<Vec<_>>(),
            )
        })
        .collect();
    data(specialty::streamgraph(&x, &bands, specialty::Baseline::WeightedWiggle).expect("aligned flows"))
        .title("Regional activity")
        .y_axis(|a| a.none())
}
pub fn polar_bars() -> Data {
    let bars: Vec<_> = (0..16)
        .map(|i| {
            let angle = f64::from(i) * TAU / 16.;
            (format!("Bearing {:03}°", i * 360 / 16), polar::Bar {
                angle,
                width: TAU / 18.,
                inner: 0.,
                outer: 5. + 3. * (angle * 2.).cos() + angle.sin(),
            })
        })
        .collect();
    data(polar::bars(&bars).expect("positive radii")).title("Directional frequency")
}
pub fn polar_scatter() -> Data {
    let samples: Vec<_> = (0..100)
        .map(|i| {
            let a = f64::from(i) * 2.4;
            [a, 5. + 3. * (a * 3.).sin() + (a * 0.3).cos()]
        })
        .collect();
    data(polar::scatter(&[("Echo", samples)])).title("Echo returns")
}
pub fn polar_errors() -> Data {
    let rows: Vec<_> = (0..7)
        .map(|i| polar::Uncertainty {
            angle: f64::from(i) * 0.89,
            radius: 3. + (f64::from(i) * 1.6).sin(),
            angular_error: 0.21,
            radial_error: 0.6,
        })
        .collect();
    data(polar::error_bars(&rows).expect("valid uncertainties")).title("Bearing and range uncertainty")
}
fn terrain() -> field::Mesh {
    let axis: Vec<_> = (-25..=25).map(|v| f64::from(v) / 10.).collect();
    let z: Vec<_> = axis
        .iter()
        .flat_map(|&y| {
            axis.iter()
                .map(move |&x| 2. * (-0.8 * (x * x + y * y)).exp() + 0.4 * (2.3 * x + 0.7 * y).sin())
        })
        .collect();
    field::Mesh::grid(&axis, &axis, &z).expect("matching terrain grid")
}
pub fn hillshade() -> Data {
    data(terrain().hillshade(305., 38.).expect("valid light")).title("Terrain under northwestern light")
}
pub fn contour_3d(camera: hyozu::spatial::Camera) -> Data {
    let mesh = terrain();
    data(
        mesh.contours_3d(&[-0.3, 0., 0.3, 0.6, 0.9, 1.2, 1.5, 1.8], None)
            .project(camera)
            .expect("valid camera"),
    )
    .title("Elevation isolines")
}
pub fn contour_bands_3d(camera: hyozu::spatial::Camera) -> Data {
    data(
        terrain()
            .contour_bands_3d(&[-0.5, 0., 0.5, 1., 1.5, 2., 2.5], None)
            .expect("valid levels")
            .project(camera)
            .expect("valid camera"),
    )
    .title("Elevation bands")
}
pub fn bihistogram() -> Data {
    let samples = |shift: f64| {
        (0..300)
            .map(|i| {
                let t = f64::from(i) * 2.399;
                t.sin() + 0.6 * (t * 3.3).cos() + shift
            })
            .collect::<Vec<_>>()
    };
    let edges: Vec<_> = (-10..=10).map(|v| f64::from(v) / 4.).collect();
    data(
        hyozu::stats::bihistogram(
            &edges,
            ("Upstream", &samples(-0.6)),
            ("Downstream", &samples(0.6)),
            false,
        )
        .expect("shared edges"),
    )
    .legend(hyozu::legend::Anchor::Bottom)
    .title("Upstream and downstream measurements")
}
pub fn hats() -> Data {
    data(
        hyozu::cartesian::hat_graph(&[("Reference", [14., 22., 16., 19.]), ("Trial", [19., 16., 21., 20.])])
            .expect("matched categories"),
    )
    .legend(hyozu::legend::Anchor::Bottom)
    .title("Trial scores against reference")
}
