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
