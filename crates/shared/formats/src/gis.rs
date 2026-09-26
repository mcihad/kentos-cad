//! What the GIS readers (GeoJSON, Shapefile; docs/adr/0046) share: the four
//! kinds of object a GIS file holds (a point with its height, a line, a
//! path and an area with holes) made into the app's objects with their
//! attributes, the layers in the order they are met, the extent, and the
//! object limit. Coordinates are kept as read: never rounded, reprojected
//! or reordered (CLAUDE.md §5, §23).

use std::collections::BTreeMap;

use kentos_contracts::{
    Bounds, DeclaredCrs, Entity, EntityBase, ImportLayer, ImportResult, LineEntity, LineType,
    PathEntity, PointEntity, RingGeometry, Vec2,
};

use crate::report::Report;

/// Objects a reader makes when the caller sets no limit.
pub const DEFAULT_LIMIT: usize = 1_000_000;

/// One object of a GIS file.
#[derive(Clone, Debug, PartialEq)]
pub enum Shape {
    Point {
        p: Vec2,
        z: Option<f64>,
    },
    Line(Vec2, Vec2),
    Polyline(Vec<Vec2>),
    /// An outline and its holes (rings without a repeated closing point).
    Polygon(Vec<Vec2>, Vec<Vec<Vec2>>),
}

impl Shape {
    fn kind(&self) -> &'static str {
        match self {
            Shape::Point { .. } => "point",
            Shape::Line(..) => "line",
            Shape::Polyline(_) => "polyline",
            Shape::Polygon(..) => "polygon",
        }
    }
}

/// The objects read so far and what was said about them.
pub struct Collect {
    pub entities: Vec<Entity>,
    pub report: Report,
    bounds: Option<Bounds>,
    limit: usize,
    /// Objects left out for the limit.
    over: u32,
}

impl Collect {
    /// A collection of at most `max` objects (0: the default limit).
    pub fn new(max: u32) -> Collect {
        Collect {
            entities: Vec::new(),
            report: Report::default(),
            bounds: None,
            limit: if max == 0 {
                DEFAULT_LIMIT
            } else {
                max as usize
            },
            over: 0,
        }
    }

    fn extend(&mut self, p: Vec2) {
        let b = self.bounds.get_or_insert(Bounds {
            min_x: p.x,
            min_y: p.y,
            max_x: p.x,
            max_y: p.y,
        });
        b.min_x = b.min_x.min(p.x);
        b.min_y = b.min_y.min(p.y);
        b.max_x = b.max_x.max(p.x);
        b.max_y = b.max_y.max(p.y);
    }

    /// Adds an object on `layer` (an empty name: the reader's default
    /// layer, named when the file is read, see `finish`).
    pub fn add(
        &mut self,
        shape: Shape,
        layer: &str,
        attrs: &BTreeMap<String, String>,
        label: Option<&str>,
    ) {
        if self.entities.len() >= self.limit {
            self.over += 1;
            return;
        }
        self.report.count(shape.kind());
        match &shape {
            Shape::Point { p, .. } => self.extend(*p),
            Shape::Line(a, b) => {
                self.extend(*a);
                self.extend(*b);
            }
            Shape::Polyline(pts) => pts.iter().for_each(|p| self.extend(*p)),
            // Holes lie inside their outline.
            Shape::Polygon(pts, _) => pts.iter().for_each(|p| self.extend(*p)),
        }
        let base = EntityBase {
            id: 0,
            layer_id: layer.to_string(),
            color: None,
            attrs: attrs.clone(),
            label: label.map(str::to_string),
            symbol: None,
        };
        self.entities.push(match shape {
            Shape::Point { p, z } => Entity::Point(PointEntity { base, p, z }),
            Shape::Line(a, b) => Entity::Line(LineEntity { base, a, b }),
            Shape::Polyline(pts) => Entity::Polyline(PathEntity {
                base,
                pts,
                bulges: None,
                holes: None,
            }),
            Shape::Polygon(pts, holes) => Entity::Polygon(PathEntity {
                base,
                pts,
                bulges: None,
                holes: (!holes.is_empty()).then(|| {
                    holes
                        .into_iter()
                        .map(|pts| RingGeometry { pts, bulges: None })
                        .collect()
                }),
            }),
        });
    }

    /// The result: objects without a layer go to `default_layer`; the
    /// layers are listed in the order their first object was read.
    pub fn finish(mut self, default_layer: &str, declared: Option<DeclaredCrs>) -> ImportResult {
        if self.over > 0 {
            let limit = self.limit;
            self.report.skip_n(
                "Nesne sınırı",
                &format!("ilk {limit} nesne alındı; kalanlar alınmadı (dosyayı bölün)"),
                0,
                self.over,
            );
        }
        let mut layers: Vec<ImportLayer> = Vec::new();
        let mut index: BTreeMap<String, usize> = BTreeMap::new();
        for e in &mut self.entities {
            let base = entity_base(e);
            if base.layer_id.is_empty() {
                base.layer_id = default_layer.to_string();
            }
            let i = *index.entry(base.layer_id.clone()).or_insert_with(|| {
                layers.push(ImportLayer {
                    name: base.layer_id.clone(),
                    color: "ink".to_string(),
                    visible: true,
                    locked: false,
                    line_type: LineType::Continuous,
                    line_weight: None,
                    count: 0,
                });
                layers.len() - 1
            });
            layers[i].count += 1;
        }
        ImportResult {
            entities: self.entities,
            layers,
            report: self.report.import(),
            bounds: self.bounds,
            declared_crs: declared,
        }
    }
}

fn entity_base(e: &mut Entity) -> &mut EntityBase {
    match e {
        Entity::Point(x) => &mut x.base,
        Entity::Line(x) => &mut x.base,
        Entity::Polyline(x) | Entity::Polygon(x) => &mut x.base,
        Entity::Circle(x) => &mut x.base,
        Entity::Arc(x) => &mut x.base,
        Entity::Ellipse(x) => &mut x.base,
        Entity::Spline(x) => &mut x.base,
        Entity::Xline(x) | Entity::Ray(x) => &mut x.base,
        Entity::Text(x) => &mut x.base,
        Entity::Dimension(x) => &mut x.base,
        Entity::Hatch(x) => &mut x.base,
    }
}

/// Twice the signed area of a ring (the shoelace sum Σ xᵢ·yᵢ₊₁ − xᵢ₊₁·yᵢ,
/// without a repeated closing point): positive counter-clockwise.
pub fn shoelace(ring: &[Vec2]) -> f64 {
    let n = ring.len();
    let mut s = 0.0;
    for i in 0..n {
        let a = ring[i];
        let b = ring[(i + 1) % n];
        s += a.x * b.y - b.x * a.y;
    }
    s
}

/// A ring without its closing point (the last one, when it equals the first exactly).
pub fn open_ring(mut ring: Vec<Vec2>) -> (Vec<Vec2>, bool) {
    let closed = ring.len() > 1 && ring.first() == ring.last();
    if closed {
        ring.pop();
    }
    (ring, closed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(x: f64, y: f64) -> Vec2 {
        Vec2 { x, y }
    }

    #[test]
    fn layers_come_in_the_order_met_and_the_limit_is_reported() {
        let mut c = Collect::new(3);
        let none = BTreeMap::new();
        c.add(
            Shape::Point {
                p: v(1.0, 2.0),
                z: Some(3.0),
            },
            "b",
            &none,
            None,
        );
        c.add(Shape::Line(v(0.0, 0.0), v(5.0, -1.0)), "", &none, Some("L"));
        c.add(
            Shape::Point {
                p: v(9.0, 9.0),
                z: None,
            },
            "b",
            &none,
            None,
        );
        c.add(
            Shape::Point {
                p: v(99.0, 99.0),
                z: None,
            },
            "c",
            &none,
            None,
        );
        let r = c.finish("varsayılan", None);
        assert_eq!(
            r.layers
                .iter()
                .map(|l| (l.name.as_str(), l.count))
                .collect::<Vec<_>>(),
            vec![("b", 2), ("varsayılan", 1)]
        );
        let b = r.bounds.expect("bounds");
        assert_eq!((b.min_x, b.min_y, b.max_x, b.max_y), (0.0, -1.0, 9.0, 9.0));
        assert_eq!(r.report.skipped.len(), 1);
        assert_eq!(
            (r.report.skipped[0].what.as_str(), r.report.skipped[0].count),
            ("Nesne sınırı", 1)
        );
    }

    #[test]
    fn shoelace_is_positive_counter_clockwise_and_rings_lose_their_closing_point() {
        let ccw = [v(0.0, 0.0), v(2.0, 0.0), v(2.0, 2.0), v(0.0, 2.0)];
        assert_eq!(shoelace(&ccw), 8.0);
        let (ring, closed) = open_ring(vec![v(0.0, 0.0), v(1.0, 0.0), v(1.0, 1.0), v(0.0, 0.0)]);
        assert_eq!((ring.len(), closed), (3, true));
        let (ring, closed) = open_ring(vec![v(0.0, 0.0), v(1.0, 0.0), v(1.0, 1.0)]);
        assert_eq!((ring.len(), closed), (3, false));
    }
}
