//! Mixed simple features in a common coordinate reference system.
//! Geographic coordinates use degrees in longitude/latitude order. Projected
//! coordinates use their CRS units. Transform once when the data changes.
use crate::Diagram;
use georust::{BoundingRect, CoordsIter, InteriorPoint, MapCoords};
pub use georust::{
    Coord, Geometry, GeometryCollection, LineString, MultiLineString, MultiPoint, MultiPolygon, Point, Polygon,
};
use proj4rs::Proj;
use serde_json::{Map, Value};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid GeoJSON: {0}")]
    GeoJson(#[source] Box<geojson::Error>),
    #[error("coordinate reference system: {0}")]
    Projection(#[from] proj4rs::errors::Error),
    #[error("CRS {0} requires an external coordinate operation")]
    ExternalCrs(String),
    #[error("coordinate operation failed: {0}")]
    Operation(String),
    #[cfg(feature = "native-proj")]
    #[error(transparent)]
    NativeDefinition(#[from] proj::ProjCreateError),
    #[cfg(feature = "native-proj")]
    #[error(transparent)]
    NativeOperation(#[from] proj::ProjError),
    #[error("feature {index} contains a nonfinite coordinate")]
    Nonfinite { index: usize },
}
impl From<geojson::Error> for Error {
    fn from(e: geojson::Error) -> Self {
        Self::GeoJson(Box::new(e))
    }
}

#[derive(Clone)]
pub struct Crs {
    definition: String,
    projection: Option<Proj>,
}
impl Crs {
    /// Accept an EPSG identifier such as EPSG:4326, or a PROJ parameter string.
    pub fn parse(definition: &str) -> Result<Self, Error> {
        match Proj::from_user_string(definition) {
            Ok(projection) => Ok(Self {
                definition: definition.to_owned(),
                projection: Some(projection),
            }),
            #[cfg(feature = "native-proj")]
            Err(_) => {
                proj::Proj::new(definition)?;
                Ok(Self::external(definition))
            }
            #[cfg(not(feature = "native-proj"))]
            Err(error) => Err(error.into()),
        }
    }
    /// Describe a CRS handled by the caller's transform, e.g. a local WKT
    /// definition or an instrument coordinate frame.
    pub fn external(definition: impl Into<String>) -> Self {
        Self {
            definition: definition.into(),
            projection: None,
        }
    }
    pub fn definition(&self) -> &str {
        &self.definition
    }
    /// Convert one coordinate with the configured backend. For an entire
    /// dataset use Layer::reproject so native PROJ can reuse one operation.
    pub fn convert(&self, to: &Self, point: Coord) -> Result<Coord, Error> {
        #[cfg(feature = "native-proj")]
        {
            if !point.x.is_finite() || !point.y.is_finite() {
                return Err(Error::Operation("input coordinate is nonfinite".into()));
            }
            let operation = proj::Proj::new_known_crs(self.definition(), to.definition(), None)?;
            let (x, y) = operation.convert((point.x, point.y))?;
            Ok(Coord { x, y })
        }
        #[cfg(not(feature = "native-proj"))]
        {
            self.convert_portable(to, point)
        }
    }
    /// Use the pure-Rust engine for one supported pair, even in a native build.
    pub fn convert_portable(&self, to: &Self, point: Coord) -> Result<Coord, Error> {
        if !point.x.is_finite() || !point.y.is_finite() {
            return Err(Error::Operation("input coordinate is nonfinite".into()));
        }
        let src = self
            .projection
            .as_ref()
            .ok_or_else(|| Error::ExternalCrs(self.definition.clone()))?;
        let dst = to
            .projection
            .as_ref()
            .ok_or_else(|| Error::ExternalCrs(to.definition.clone()))?;
        let (x, y) = if src.is_latlong() {
            (point.x.to_radians(), point.y.to_radians())
        } else {
            (point.x, point.y)
        };
        let (x, y) = proj4rs::adaptors::transform_xy(src, dst, x, y)?;
        let (x, y) = if dst.is_latlong() {
            (x.to_degrees(), y.to_degrees())
        } else {
            (x, y)
        };
        if x.is_finite() && y.is_finite() {
            Ok(Coord { x, y })
        } else {
            Err(Error::Operation("transformed coordinate is nonfinite".into()))
        }
    }
}

#[derive(Clone, Debug)]
pub struct Feature {
    pub id: Option<String>,
    pub properties: Map<String, Value>,
    pub geometry: Option<Geometry>,
}
impl Feature {
    pub fn name(&self, key: &str) -> &str {
        self.properties
            .get(key)
            .and_then(Value::as_str)
            .or(self.id.as_deref())
            .unwrap_or("")
    }
    /// Point on the feature surface, including concave polygons and holes.
    pub fn label_point(&self) -> Option<Point> {
        self.geometry.as_ref()?.interior_point()
    }
}

#[derive(Clone)]
pub struct Layer {
    features: Vec<Feature>,
    crs: Crs,
}

#[derive(Clone, Copy, Debug)]
pub enum Labels {
    None,
    Text,
    Boxed {
        ink: crate::core::Color,
        fill: crate::core::Color,
        padding: f32,
    },
}

impl Layer {
    pub fn new(crs: Crs, features: Vec<Feature>) -> Result<Self, Error> {
        for (index, feature) in features.iter().enumerate() {
            if feature
                .geometry
                .iter()
                .flat_map(Geometry::coords_iter)
                .any(|p| !p.x.is_finite() || !p.y.is_finite())
            {
                return Err(Error::Nonfinite { index });
            }
        }
        Ok(Self { features, crs })
    }
    /// Read a geometry, feature or feature collection. The source CRS is explicit
    /// for legacy datasets. RFC 7946 GeoJSON uses EPSG:4326.
    pub fn from_geojson(source: &str, crs: Crs) -> Result<Self, Error> {
        let geojson: geojson::GeoJson = source.parse()?;
        let rows = match geojson {
            geojson::GeoJson::Geometry(g) => vec![geojson::Feature::from(g)],
            geojson::GeoJson::Feature(f) => vec![f],
            geojson::GeoJson::FeatureCollection(fc) => fc.features,
        };
        let features = rows
            .into_iter()
            .map(|f| {
                let id = f.id.map(|id| match id {
                    geojson::feature::Id::String(s) => s,
                    geojson::feature::Id::Number(n) => n.to_string(),
                });
                let geometry = f.geometry.map(|g| Geometry::try_from(g.value)).transpose()?;
                Ok(Feature {
                    id,
                    properties: f.properties.unwrap_or_default(),
                    geometry,
                })
            })
            .collect::<Result<Vec<_>, geojson::Error>>()?;
        Self::new(crs, features)
    }

    pub fn features(&self) -> &[Feature] {
        &self.features
    }
    pub fn crs(&self) -> &Crs {
        &self.crs
    }

    pub fn reproject(&self, destination: Crs) -> Result<Self, Error> {
        #[cfg(feature = "native-proj")]
        {
            self.reproject_native(destination)
        }
        #[cfg(not(feature = "native-proj"))]
        {
            let source = &self.crs;
            let to = destination.clone();
            self.try_transform(destination, |p| source.convert(&to, p))
        }
    }

    /// Native PROJ resolves EPSG, WKT and PROJ definitions. The operation is
    /// selected once for the layer. Geographic axes follow visualization XY order.
    #[cfg(feature = "native-proj")]
    pub fn reproject_native(&self, destination: Crs) -> Result<Self, Error> {
        let operation = proj::Proj::new_known_crs(self.crs.definition(), destination.definition(), None)?;
        self.try_transform(destination, |p| {
            let (x, y) = operation.convert((p.x, p.y))?;
            Ok(Coord { x, y })
        })
    }

    /// Accept a caller-provided transformation, including native PROJ pipelines
    /// requiring local datum grids. Failed conversions leave the source intact.
    pub fn try_transform(
        &self,
        destination: Crs,
        transform: impl Fn(Coord) -> Result<Coord, Error>,
    ) -> Result<Self, Error> {
        let records = self
            .features
            .iter()
            .map(|f| {
                Ok(Feature {
                    id: f.id.clone(),
                    properties: f.properties.clone(),
                    geometry: f.geometry.as_ref().map(|g| g.try_map_coords(&transform)).transpose()?,
                })
            })
            .collect::<Result<Vec<_>, Error>>()?;
        Self::new(destination, records)
    }

    /// Each feature keeps one legend and selection identity across all of its
    /// components. Coordinates retain equal scale on both axes at any widget size.
    pub fn diagram(&self, labels: Labels, name: impl Fn(&Feature) -> String) -> Diagram {
        self.diagram_with_anchors(labels, name, Feature::label_point)
    }
    /// Override the default interior point for e.g. a manually placed label.
    pub fn diagram_with_anchors(
        &self,
        labels: Labels,
        name: impl Fn(&Feature) -> String,
        anchor: impl Fn(&Feature) -> Option<Point>,
    ) -> Diagram {
        let mut diagram = Diagram {
            square: true,
            centered_labels: true,
            ..Diagram::default()
        };
        if let Labels::Boxed { ink, fill, padding } = labels {
            diagram.label_color = Some(ink);
            diagram.label_background = Some((fill, padding.max(0.)));
        }
        let bounds = self
            .features
            .iter()
            .filter_map(|f| f.geometry.as_ref()?.bounding_rect())
            .fold(None, |acc: Option<georust::Rect>, r| {
                Some(acc.map_or(r, |a| {
                    georust::Rect::new(
                        Coord {
                            x: a.min().x.min(r.min().x),
                            y: a.min().y.min(r.min().y),
                        },
                        Coord {
                            x: a.max().x.max(r.max().x),
                            y: a.max().y.max(r.max().y),
                        },
                    )
                }))
            });
        let Some(bounds) = bounds else {
            return diagram;
        };
        let mid = bounds.center();
        let extent = bounds.width().max(bounds.height());
        let scale = if extent > 0. { 0.91 / extent } else { 1. };
        let position = |p: Coord| [0.5 + (p.x - mid.x) * scale, 0.5 - (p.y - mid.y) * scale];
        for feature in &self.features {
            let Some(geometry) = &feature.geometry else {
                continue;
            };
            let name = name(feature);
            let id = diagram.series(&name);
            draw(&mut diagram, id, geometry, &position);
            if !matches!(labels, Labels::None)
                && !name.is_empty()
                && let Some(pt) = anchor(feature).filter(|p| p.x().is_finite() && p.y().is_finite())
            {
                diagram.labels.push((position(pt.0).map(|v| v as f32), name));
            }
        }
        diagram
    }
}

fn draw(d: &mut Diagram, id: usize, g: &Geometry, position: &impl Fn(Coord) -> [f64; 2]) {
    let line =
        |d: &mut Diagram, coords: &LineString| d.path(id, coords.0.iter().copied().map(position).collect(), false, 1.);
    let polygon = |d: &mut Diagram, p: &Polygon| {
        if p.exterior().0.len() < 3 {
            return;
        }
        d.path(id, p.exterior().0.iter().copied().map(position).collect(), true, 0.78);
        if let Some(shape) = d.shapes.last_mut() {
            shape.holes = p
                .interiors()
                .iter()
                .filter(|ring| ring.0.len() >= 3)
                .map(|ring| ring.0.iter().copied().map(position).collect())
                .collect();
        }
    };
    let point = |d: &mut Diagram, p: Point| {
        let [x, y] = position(p.0);
        d.path(
            id,
            (0..18)
                .map(|i| {
                    let t = f64::from(i) * std::f64::consts::TAU / 18.;
                    [x + 0.009 * t.cos(), y + 0.009 * t.sin()]
                })
                .collect(),
            true,
            1.,
        );
    };
    match g {
        Geometry::Point(p) => point(d, *p),
        Geometry::Line(l) => d.path(id, vec![position(l.start), position(l.end)], false, 1.),
        Geometry::LineString(l) => line(d, l),
        Geometry::Polygon(p) => polygon(d, p),
        Geometry::MultiPoint(ps) => ps.iter().for_each(|p| point(d, *p)),
        Geometry::MultiLineString(ls) => ls.iter().for_each(|l| line(d, l)),
        Geometry::MultiPolygon(ps) => ps.iter().for_each(|p| polygon(d, p)),
        Geometry::GeometryCollection(gs) => gs.iter().for_each(|g| draw(d, id, g, position)),
        Geometry::Rect(r) => polygon(d, &r.to_polygon()),
        Geometry::Triangle(t) => polygon(d, &t.to_polygon()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use georust::Contains;
    const SAMPLE: &str = include_str!("../../examples/gallery/src/charts/harbor.geojson");
    #[test]
    fn mercator_and_utm_match_published_projection_definitions() {
        let wgs = Crs::parse("EPSG:4326").expect("WGS 84");
        let merc = Crs::parse("EPSG:3857").expect("Web Mercator");
        let p = wgs
            .convert(&merc, Coord { x: -74., y: 40. })
            .expect("projected coordinate");
        assert!((p.x + 8237642.3187).abs() < 0.01 && (p.y - 4865942.2795).abs() < 0.01);
        let utm = Crs::parse("EPSG:32618").expect("UTM 18N");
        let center = wgs.convert(&utm, Coord { x: -75., y: 0. }).expect("central meridian");
        assert!((center.x - 500000.).abs() < 0.01 && center.y.abs() < 0.01);
        let round = merc.convert(&wgs, p).expect("inverse");
        assert!((round.x + 74.).abs() < 1e-7 && (round.y - 40.).abs() < 1e-7);
    }
    #[test]
    fn mixed_collections_keep_identity_holes_and_surface_labels() {
        let source = Layer::from_geojson(SAMPLE, Crs::parse("EPSG:4326").expect("WGS 84")).expect("features");
        let projected = source
            .reproject(Crs::parse("EPSG:32618").expect("UTM"))
            .expect("converted features");
        let feature = &projected.features()[0];
        assert_eq!(feature.name("name"), "Harbor district");
        let pt = feature.label_point().expect("interior label");
        assert!(feature.geometry.as_ref().expect("polygon").contains(&pt));
        let diagram = projected.diagram(Labels::Text, |f| f.name("name").to_owned());
        assert_eq!(diagram.entries().len(), 5);
        assert_eq!(diagram.labels.len(), 5);
        assert_eq!(diagram.shapes[0].holes.len(), 1);
        assert_eq!(diagram.shapes.iter().filter(|s| s.entry == Some(3)).count(), 4);
    }
    #[test]
    fn failed_external_transform_preserves_properties_and_source_coordinates() {
        let l = Layer::from_geojson(
            r#"{"type":"Feature","id":"a","properties":{"name":"A"},"geometry":{"type":"Point","coordinates":[1,2]}}"#,
            Crs::external("test source frame"),
        )
        .expect("input feature");
        let bad = l.try_transform(Crs::external("other frame"), |_| {
            Err(Error::Operation("missing calibration".into()))
        });
        assert!(bad.is_err());
        assert_eq!(l.features()[0].label_point(), Some(Point::new(1., 2.)));
        let transformed = l
            .try_transform(Crs::external("other frame"), |p| {
                Ok(Coord {
                    x: p.x + 10.,
                    y: p.y * 2.,
                })
            })
            .expect("callback operation");
        assert_eq!(transformed.features()[0].label_point(), Some(Point::new(11., 4.)));
        assert_eq!(transformed.features()[0].name("name"), "A");
    }

    #[cfg(feature = "native-proj")]
    #[test]
    fn native_proj_transforms_an_authority_free_wkt_definition() {
        let local=Crs::parse(r#"PROJCS["Example UTM zone 18N",GEOGCS["WGS 84",DATUM["WGS_1984",SPHEROID["WGS 84",6378137,298.257223563]],PRIMEM["Greenwich",0],UNIT["degree",0.0174532925199433]],PROJECTION["Transverse_Mercator"],PARAMETER["latitude_of_origin",0],PARAMETER["central_meridian",-75],PARAMETER["scale_factor",0.9996],PARAMETER["false_easting",500000],PARAMETER["false_northing",0],UNIT["metre",1]]"#).expect("valid WKT");
        let l = Layer::from_geojson(
            r#"{"type":"Point","coordinates":[-75,0]}"#,
            Crs::parse("EPSG:4326").expect("WGS 84"),
        )
        .expect("point");
        let q = l.reproject_native(local).expect("native WKT operation");
        let p = q.features()[0].label_point().expect("point");
        assert!((p.x() - 500000.).abs() < 1e-5 && p.y().abs() < 1e-5);
    }
}
