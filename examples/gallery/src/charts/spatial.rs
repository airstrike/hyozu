use hyozu::spatial::{Camera, Scene};
use hyozu::{Data, data};

fn project(scene: Scene, camera: Camera, title: &str) -> Data {
    data(scene.project(camera).expect("gallery camera fits the scene")).title(title)
}
fn terrain(camera: Camera, wireframe: bool) -> Data {
    let a: Vec<_> = (-7..=7).map(|i| f64::from(i) / 3.).collect();
    let heights: Vec<_> = a
        .iter()
        .flat_map(|&y| a.iter().map(move |&x| 0.6 * x.sin() + 0.4 * (y * 1.7).cos()))
        .collect();
    let mut s = Scene::default();
    let id = s.series("Height");
    s.surface(id, &a, &a, &heights, wireframe);
    project(s, camera, "Measured surface")
}
pub fn surface(camera: Camera) -> Data {
    terrain(camera, false)
}
pub fn wireframe(camera: Camera) -> Data {
    terrain(camera, true)
}
pub fn scatter(camera: Camera) -> Data {
    let mut s = Scene::default();
    for j in 0..3 {
        let id = s.series(format!("Group {}", j + 1));
        let points: Vec<_> = (0..60)
            .map(|i| {
                let t = f64::from(i) * 2.3999;
                let r = (f64::from(i) / 60.).sqrt();
                [r * t.cos(), r * t.sin(), f64::from(j) * 0.7 + 0.2 * (t * 2.).cos()]
            })
            .collect();
        s.scatter(id, &points);
    }
    project(s, camera, "Clusters in three dimensions")
}
pub fn line(camera: Camera) -> Data {
    let mut s = Scene::default();
    let id = s.series("Path");
    let p: Vec<_> = (0..=160)
        .map(|i| {
            let t = f64::from(i) / 16.;
            [t.cos(), t.sin(), t]
        })
        .collect();
    s.line(id, &p);
    project(s, camera, "Helical trajectory")
}
pub fn bars(camera: Camera) -> Data {
    let mut s = Scene::default();
    for j in 0..3 {
        let id = s.series(format!("Group {}", j + 1));
        for i in 0..4 {
            let (x, y) = (f64::from(i), f64::from(j));
            s.cuboid(id, [x, y, 0.], [x + 0.65, y + 0.65, (x + 2. * y).sin() + 1.5]);
        }
    }
    project(s, camera, "Volume by group")
}
pub fn voxels(camera: Camera) -> Data {
    let mut s = Scene::default();
    for k in 0..3 {
        let id = s.series(format!("Layer {}", k + 1));
        for j in 0..3 - k {
            for i in 0..3 - k {
                let (x, y, z) = (f64::from(i), f64::from(j), f64::from(k));
                s.cuboid(id, [x, y, z], [x + 0.94, y + 0.94, z + 0.94]);
            }
        }
    }
    project(s, camera, "Occupied voxels")
}

pub fn pie(camera: Camera) -> Data {
    project(
        hyozu::spatial::pie(
            &[("Direct", 52., 0.), ("Partners", 31., 0.), ("Other", 17., 0.16)],
            0.32,
        )
        .expect("positive wedges"),
        camera,
        "Orders by channel",
    )
}

const REVENUE: [(&str, [f64; 5]); 3] = [
    ("Direct", [12., 15., 14., 18., 20.]),
    ("Partners", [8., 10., 12., 11., 15.]),
    ("Retail", [5., 6., 4., 8., 9.]),
];
pub fn columns(camera: Camera) -> Data {
    use hyozu::spatial::{CategoryMark, Stack, categories};
    project(
        categories(&REVENUE, CategoryMark::Column, Stack::Sum).expect("aligned categories"),
        camera,
        "Stacked quarterly revenue",
    )
}
pub fn horizontal(camera: Camera) -> Data {
    use hyozu::spatial::{CategoryMark, Stack, categories};
    project(
        categories(&REVENUE, CategoryMark::Bar, Stack::Percent).expect("positive totals"),
        camera,
        "Share by channel",
    )
}
pub fn ribbons(camera: Camera) -> Data {
    use hyozu::spatial::{CategoryMark, Stack, categories};
    project(
        categories(&REVENUE, CategoryMark::Line, Stack::Grouped).expect("aligned categories"),
        camera,
        "Series across reporting periods",
    )
}
pub fn areas(camera: Camera) -> Data {
    use hyozu::spatial::{CategoryMark, Stack, categories};
    project(
        categories(&REVENUE, CategoryMark::Area, Stack::Grouped).expect("aligned categories"),
        camera,
        "Channel volume by period",
    )
}
pub fn between(camera: Camera) -> Data {
    let t: Vec<_> = (0..=50).map(|i| f64::from(i) / 8.).collect();
    let edge = |h: f64| {
        t.iter()
            .map(|&v| [v, v.cos() + h * 0.3, v.sin() + h * (0.5 + 0.2 * v.cos())])
            .collect::<Vec<_>>()
    };
    let mut s = Scene::default();
    let id = s.series("Interval");
    s.fill_between(id, &edge(0.), &edge(1.));
    project(s, camera, "Uncertainty along a trajectory")
}
pub fn stems(camera: Camera) -> Data {
    let p: Vec<_> = (0..28)
        .map(|i| {
            let a = f64::from(i) * 0.45;
            [a.cos(), a.sin(), 0.6 * a.cos() + 0.32 * (2. * a).sin()]
        })
        .collect();
    let mut s = Scene::default();
    let id = s.series("Samples");
    s.stems(id, &p, 0.);
    project(s, camera, "Signed impulse response")
}
pub fn quiver(camera: Camera) -> Data {
    let mut s = Scene::default();
    let id = s.series("Velocity");
    for j in -2..=2 {
        for i in -2..=2 {
            let (x, y) = (f64::from(i), f64::from(j));
            s.arrow(id, [x, y, 0.], [-0.25 * y, 0.25 * x, 0.32]);
        }
    }
    project(s, camera, "Three-dimensional flow")
}

pub fn trisurf(camera: Camera) -> Data {
    let (pts, faces) = super::fields::survey();
    let mut s = Scene::default();
    let id = s.series("Elevation");
    s.mesh(id, &pts, &faces, false);
    project(s, camera, "Triangulated survey surface")
}

pub fn cluster(camera: Camera) -> Data {
    use hyozu::spatial::{CategoryMark, Stack, categories};
    project(
        categories(&REVENUE, CategoryMark::Column, Stack::Clustered).expect("aligned categories"),
        camera,
        "Clustered quarterly revenue",
    )
}
pub fn banded(camera: Camera) -> Data {
    let (points, faces) = super::fields::survey();
    let mesh = hyozu::field::Mesh::new(points, faces).expect("survey triangles");
    let scene = mesh
        .banded_surface(&[-0.3, 0., 0.25, 0.5, 0.75, 1., 1.3])
        .expect("ordered intervals");
    project(scene, camera, "Elevation bands")
        .palette(hyozu::palette::gradient([0x224e57, 0x468980, 0x87b493, 0xe0cf83]))
}
