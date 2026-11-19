use hyozu::geo::features::{Crs, Labels, Layer};
use hyozu::{Data, data};
fn harbor() -> Layer {
    let layer = Layer::from_geojson(include_str!("harbor.geojson"), Crs::parse("EPSG:4326").expect("WGS 84"))
        .expect("harbor features");
    layer
        .reproject(Crs::parse("EPSG:32618").expect("UTM zone 18N"))
        .expect("projected harbor")
}
pub fn geometry() -> Data {
    data(harbor().diagram(Labels::None, |f| f.name("name").to_owned())).title("Harbor survey")
}
pub fn names() -> Data {
    data(harbor().diagram(Labels::Text, |f| f.name("name").to_owned())).title("Harbor features")
}
pub fn boxed() -> Data {
    let labels = Labels::Boxed {
        ink: hyozu::core::Color::WHITE,
        fill: hyozu::core::Color::from_rgb8(34, 79, 80),
        padding: 3.,
    };
    data(harbor().diagram(labels, |f| f.name("name").to_owned())).title("Harbor features in UTM zone 18N")
}
