use hyozu::field::{self, Mesh};
use hyozu::{Data, data, palette};

fn terrain() -> Mesh {
    let axis: Vec<_> = (-8..=8).map(|i| f64::from(i) / 4.).collect();
    let heights: Vec<_> = axis
        .iter()
        .flat_map(|&y| {
            axis.iter().map(move |&x| {
                2.0 * (-0.8 * ((x + 0.6).powi(2) + (y - 0.3).powi(2))).exp()
                    + 0.8 * (-2. * ((x - 1.).powi(2) + (y + 0.4).powi(2))).exp()
            })
        })
        .collect();
    Mesh::grid(&axis, &axis, &heights).expect("matching grid dimensions")
}
fn observations() -> Vec<[f64; 2]> {
    (0..500)
        .map(|i| {
            let a = f64::from(i) * 2.399963;
            let r = (f64::from(i) / 500.).sqrt() * (0.55 + 0.3 * (a * 4.).cos());
            [a.cos() * r, 0.7 * a.cos() * r + a.sin() * r]
        })
        .collect()
}
fn ramp() -> hyozu::Palette {
    palette::gradient([0xddeee9, 0x87beb6, 0x3c887e, 0x174f58])
}
pub fn contour() -> Data {
    data(terrain().contours(&[0.25, 0.5, 0.75, 1., 1.25, 1.5, 1.75]))
        .palette(ramp())
        .title("Elevation contours")
}
pub fn filled() -> Data {
    data(
        terrain()
            .contour_bands(&[0., 0.25, 0.5, 0.75, 1., 1.25, 1.5, 1.75, 2.5])
            .expect("increasing levels"),
    )
    .palette(ramp())
    .title("Elevation bands")
}
pub fn wireframe() -> Data {
    data(terrain().wireframe()).title("Triangulated measurement grid")
}
pub fn histogram() -> Data {
    let edges: Vec<_> = (-5..=5).map(|i| f64::from(i) / 5.).collect();
    data(field::histogram2d(&edges, &edges, &observations(), false).expect("valid axes"))
        .palette(ramp())
        .title("Observations per cell")
}
pub fn hexbin() -> Data {
    data(field::hexbin(&observations(), 0.10).expect("positive radius"))
        .palette(ramp())
        .title("Observation density")
}
pub fn tiles() -> Data {
    data(field::tiles(&[0., 1., 3., 4.], &[0., 2., 3.], &[2., 4., 6., 7., 5., 2.]).expect("cell values match grid"))
        .palette(ramp())
        .title("Variable cell widths")
}

fn wind() -> Vec<hyozu::vector::Vector> {
    (-3..=3)
        .flat_map(|j| {
            (-4..=4).map(move |i| {
                let (x, y) = (f64::from(i), f64::from(j));
                hyozu::vector::Vector {
                    x,
                    y,
                    u: 1. + 0.18 * y,
                    v: 0.45 * (x * 0.8).cos(),
                }
            })
        })
        .collect()
}
pub fn quiver() -> Data {
    data(hyozu::vector::quiver(&wind(), 0.55)).title("Velocity field")
}
pub fn barbs() -> Data {
    let mut vectors = wind();
    for v in &mut vectors {
        v.u *= 42.;
        v.v *= 42.;
    }
    data(hyozu::vector::wind_barbs(&vectors, 0.65)).title("Wind speed and direction")
}
pub fn streamlines() -> Data {
    let x: Vec<_> = (-10..=10).map(|i| f64::from(i) / 5.).collect();
    let uv: Vec<_> = x
        .iter()
        .flat_map(|&y| x.iter().map(move |&xx| [1., 0.45 * (xx + y * 0.3).cos()]))
        .collect();
    let field = hyozu::vector::Grid::new(x.clone(), x, uv).expect("matching vector grid");
    let seeds: Vec<_> = (-6..=6).map(|i| [0., f64::from(i) / 4.]).collect();
    data(field.streamlines(&seeds, 0.05, 120)).title("Flow trajectories")
}

pub fn quad() -> Data {
    let n = 9;
    let vertices: Vec<_> = (0..n)
        .flat_map(|j| {
            (0..n).map(move |i| {
                let (x, y) = (f64::from(i), f64::from(j));
                [x + 0.48 * (y * 0.7).sin(), y + 0.35 * (x * 0.6).cos()]
            })
        })
        .collect();
    let values: Vec<_> = (0..n - 1)
        .flat_map(|j| (0..n - 1).map(move |i| f64::from(i + j)))
        .collect();
    data(field::quad_mesh(n as usize, n as usize, &vertices, &values).expect("grid corners"))
        .palette(ramp())
        .title("Curvilinear measurement cells")
}
pub fn tripcolor() -> Data {
    data(terrain().face_colors())
        .palette(ramp())
        .title("Mean elevation by triangle")
}
pub fn density2d() -> Data {
    let x: Vec<_> = (-12..=12).map(|i| f64::from(i) / 10.).collect();
    let mesh = Mesh::density(&x, &x, &observations(), [0.19, 0.17]).expect("finite observations");
    data(
        mesh.contour_bands(&[0., 0.2, 0.4, 0.6, 0.8, 1., 1.2, 1.4, 1.6, 2.5])
            .expect("sorted density levels"),
    )
    .palette(ramp())
    .title("Bivariate observation density")
}
pub fn image() -> Data {
    let (width, height) = (72, 48);
    let rgba: Vec<_> = (0..height)
        .flat_map(|j| {
            (0..width).flat_map(move |i| {
                let (x, y) = (f64::from(i) / 18., f64::from(j) / 14.);
                let v = ((x * 1.7 + y.sin()).cos() * y.sin() + 1.) * 0.5;
                [
                    (24. + 132. * v) as u8,
                    (68. + 139. * v) as u8,
                    (89. + 96. * v) as u8,
                    255,
                ]
            })
        })
        .collect();
    data(
        field::image(
            width,
            height,
            rgba,
            [0., 12.],
            [0., 8.],
            hyozu::core::image::FilterMethod::Linear,
        )
        .expect("matching image dimensions"),
    )
    .x_axis_bounds(0., 12.)
    .y_axis_bounds(0., 8.)
    .title("Calibrated intensity image")
}

pub fn hex_values() -> Data {
    let values: Vec<_> = observations()
        .into_iter()
        .map(|[x, y]| [x, y, (4. * x).sin() + (3. * y).cos()])
        .collect();
    data(field::hexbin_values(&values, 0.13, |v| v.iter().sum::<f64>() / v.len() as f64).expect("positive radius"))
        .palette(ramp())
        .title("Mean measurement per hexagon")
}

pub(super) fn survey() -> (Vec<field::Sample>, Vec<[usize; 3]>) {
    let mut samples = vec![field::Sample {
        x: 0.,
        y: 0.,
        value: 1.,
    }];
    let n = 13;
    for ring in 1..=3 {
        for i in 0..n {
            let a = std::f64::consts::TAU * (i as f64 + 0.17 * (ring % 2) as f64) / n as f64;
            let r = ring as f64 * (1. + 0.10 * (3. * a).sin());
            let (x, y) = (r * a.cos(), r * 0.8 * a.sin());
            samples.push(field::Sample {
                x,
                y,
                value: (-0.22 * (x * x + y * y)).exp() + 0.2 * (x - y).sin(),
            });
        }
    }
    let mut faces = Vec::new();
    for i in 0..n {
        let j = (i + 1) % n;
        faces.push([0, 1 + i, 1 + j]);
        for ring in 0..2 {
            let a = 1 + ring * n;
            faces.extend([[a + i, a + n + i, a + n + j], [a + i, a + n + j, a + j]]);
        }
    }
    (samples, faces)
}
pub fn irregular() -> Data {
    let (points, triangles) = survey();
    data(
        Mesh::new(points, triangles)
            .expect("survey triangles")
            .contour_bands(&[-0.3, 0., 0.2, 0.4, 0.6, 0.8, 1., 1.3])
            .expect("ordered contour levels"),
    )
    .palette(ramp())
    .title("Irregular survey contours")
}
