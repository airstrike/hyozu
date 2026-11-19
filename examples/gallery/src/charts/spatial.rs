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
