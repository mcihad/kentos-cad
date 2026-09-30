//! What the GIS readers (GeoJSON, Shapefile; docs/adr/0046) share: the four
//! kinds of object a GIS file holds (a point, a line, a path and an area
//! with holes, each with the heights the file gave its vertices) made into
//! the app's objects with their attributes, the layers in the order they are
//! met, the extent, and the object limit. Coordinates are kept as read:
//! never rounded, reprojected or reordered (CLAUDE.md §5, §23). A height is
//! a vertex's elevation (docs/adr/0142): one a vertex was not given is none,
//! not 0.

use std::collections::BTreeMap;

use kentos_contracts::{
    AreaPart, Bounds, DeclaredCrs, Entity, EntityBase, ImportLayer, ImportResult, LineEntity,
    LineType, PathEntity, PointEntity, RingGeometry, Vec2,
};

use crate::report::Report;

/// Objects a reader makes when the caller sets no limit.
pub const DEFAULT_LIMIT: usize = 1_000_000;

/// The elevation of each vertex of a run: none when no vertex has one, else
/// one entry per vertex, `None` for a vertex without.
pub type Zs = Option<Vec<Option<f64>>>;

/// A run of vertices and their elevations: a path, or a ring of an area
/// (without a repeated closing point).
#[derive(Clone, Debug, PartialEq)]
pub struct Ring {
    pub pts: Vec<Vec2>,
    pub zs: Zs,
}

impl Ring {
    /// A run of vertices with the height (if any) the file gave each: none at
    /// all leaves the run flat, as does a count that does not match.
    pub fn new(pts: Vec<Vec2>, heights: impl IntoIterator<Item = Option<f64>>) -> Ring {
        let zs: Vec<Option<f64>> = heights.into_iter().collect();
        let zs = (zs.len() == pts.len() && zs.iter().any(Option::is_some)).then_some(zs);
        Ring { pts, zs }
    }

    /// A run of vertices without heights.
    pub fn flat(pts: Vec<Vec2>) -> Ring {
        Ring { pts, zs: None }
    }
}

/// One object of a GIS file.
#[derive(Clone, Debug, PartialEq)]
pub enum Shape {
    Point {
        p: Vec2,
        z: Option<f64>,
    },
    Line {
        a: Vec2,
        b: Vec2,
        za: Option<f64>,
        zb: Option<f64>,
    },
    Polyline(Ring),
    /// An outline and its holes (rings without a repeated closing point).
    Polygon(Ring, Vec<Ring>),
    /// An area of two or more parts, each an outline and its holes, in the
    /// file's order: a MultiPolygon, a Shapefile record of several outlines
    /// (docs/adr/0143). Made by [`Shape::area`].
    Parts(Vec<(Ring, Vec<Ring>)>),
}

impl Shape {
    fn kind(&self) -> &'static str {
        match self {
            Shape::Point { .. } => "point",
            Shape::Line { .. } => "line",
            Shape::Polyline(_) => "polyline",
            Shape::Polygon(..) | Shape::Parts(_) => "polygon",
        }
    }

    /// One area of `parts`: none of none, an area of one, a multi-part area
    /// of more (docs/adr/0143).
    pub fn area(mut parts: Vec<(Ring, Vec<Ring>)>) -> Option<Shape> {
        match parts.len() {
            0 => None,
            1 => parts
                .pop()
                .map(|(outline, holes)| Shape::Polygon(outline, holes)),
            _ => Some(Shape::Parts(parts)),
        }
    }
}

/// An area's holes as the contract has them (none for none).
fn holes_of(holes: Vec<Ring>) -> Option<Vec<RingGeometry>> {
    (!holes.is_empty()).then(|| {
        holes
            .into_iter()
            .map(|h| RingGeometry {
                pts: h.pts,
                bulges: None,
                zs: h.zs,
            })
            .collect()
    })
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
            Shape::Line { a, b, .. } => {
                self.extend(*a);
                self.extend(*b);
            }
            Shape::Polyline(r) => r.pts.iter().for_each(|p| self.extend(*p)),
            // Holes lie inside their outline.
            Shape::Polygon(r, _) => r.pts.iter().for_each(|p| self.extend(*p)),
            Shape::Parts(parts) => {
                for (r, _) in parts {
                    r.pts.iter().for_each(|p| self.extend(*p));
                }
            }
        }
        let base = EntityBase {
            id: 0,
            layer_id: layer.to_string(),
            color: None,
            attrs: attrs.clone(),
            label: label.map(str::to_string),
            symbol: None,
            line_weight: None,
        };
        self.entities.push(match shape {
            Shape::Point { p, z } => Entity::Point(PointEntity { base, p, z }),
            Shape::Line { a, b, za, zb } => Entity::Line(LineEntity { base, a, b, za, zb }),
            Shape::Polyline(r) => Entity::Polyline(PathEntity {
                base,
                pts: r.pts,
                bulges: None,
                holes: None,
                zs: r.zs,
                parts: None,
            }),
            Shape::Polygon(r, holes) => Entity::Polygon(PathEntity {
                base,
                pts: r.pts,
                bulges: None,
                holes: holes_of(holes),
                zs: r.zs,
                parts: None,
            }),
            // The first part is the area's own fields, the others its parts.
            Shape::Parts(parts) => {
                let mut parts = parts.into_iter();
                let Some((r, holes)) = parts.next() else {
                    return;
                };
                Entity::Polygon(PathEntity {
                    base,
                    pts: r.pts,
                    bulges: None,
                    holes: holes_of(holes),
                    zs: r.zs,
                    parts: Some(
                        parts
                            .map(|(r, holes)| AreaPart {
                                pts: r.pts,
                                bulges: None,
                                holes: holes_of(holes),
                                zs: r.zs,
                            })
                            .collect(),
                    ),
                })
            }
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
            let base = Entity::base_mut(e);
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
                    kinds: BTreeMap::new(),
                    bounds: None,
                });
                layers.len() - 1
            });
            layers[i].count += 1;
        }
        let mut result = ImportResult {
            entities: self.entities,
            layers,
            report: self.report.import(),
            bounds: self.bounds,
            declared_crs: declared,
            view: None,
            blocks: Vec::new(),
        };
        crate::import::summarise(&mut result);
        result
    }
}

/// Twice the signed area of a ring (the shoelace sum Σ xᵢ·yᵢ₊₁ − xᵢ₊₁·yᵢ,
/// without a repeated closing point): positive counter-clockwise. Taken
/// relative to the first point: on raw TM coordinates the products cancel
/// away the area of a small ring (docs/adr/0122).
pub fn shoelace(ring: &[Vec2]) -> f64 {
    let Some(&o) = ring.first() else {
        return 0.0;
    };
    let n = ring.len();
    let mut s = 0.0;
    for i in 0..n {
        let a = ring[i];
        let b = ring[(i + 1) % n];
        s += (a.x - o.x) * (b.y - o.y) - (b.x - o.x) * (a.y - o.y);
    }
    s
}

/// What the readers say of a ring whose closing point held another elevation
/// than the first point (`open_ring`): the report's item and its reason.
pub const CLOSING_Z: (&str, &str) = (
    "Halka kapanışının Z'si",
    "kapanış konumu ilkiyle x ve y'de aynı ama Z'si farklı; alanın köşesi ilk konumun Z'sini aldı",
);

/// A ring without its closing point (the last one, when it equals the first
/// in x and y exactly): whether it had one, and whether that point held
/// another elevation than the first (the vertex keeps the first's).
pub fn open_ring(mut ring: Ring) -> (Ring, bool, bool) {
    let closed = ring.pts.len() > 1 && ring.pts.first() == ring.pts.last();
    let mut differs = false;
    if closed {
        ring.pts.pop();
        if let Some(zs) = ring.zs.as_mut() {
            let closing = zs.pop().flatten();
            differs = closing.is_some() && closing != zs.first().copied().flatten();
            if zs.iter().all(Option::is_none) {
                ring.zs = None;
            }
        }
    }
    (ring, closed, differs)
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
        c.add(
            Shape::Line {
                a: v(0.0, 0.0),
                b: v(5.0, -1.0),
                za: None,
                zb: None,
            },
            "",
            &none,
            Some("L"),
        );
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
        let flat = |pts: Vec<Vec2>| Ring::flat(pts);
        let (ring, closed, differs) = open_ring(flat(vec![
            v(0.0, 0.0),
            v(1.0, 0.0),
            v(1.0, 1.0),
            v(0.0, 0.0),
        ]));
        assert_eq!((ring.pts.len(), closed, differs), (3, true, false));
        let (ring, closed, _) = open_ring(flat(vec![v(0.0, 0.0), v(1.0, 0.0), v(1.0, 1.0)]));
        assert_eq!((ring.pts.len(), closed), (3, false));
    }

    #[test]
    fn a_ring_keeps_the_elevations_it_was_given_and_the_firsts_at_its_closing() {
        // Only vertices with a height make a run elevated; a count that does not match makes it flat.
        let pts = || vec![v(0.0, 0.0), v(1.0, 0.0), v(1.0, 1.0), v(0.0, 0.0)];
        assert_eq!(Ring::new(pts(), [None; 4]).zs, None);
        assert_eq!(Ring::new(pts(), [Some(1.0)]).zs, None);
        let (ring, closed, differs) =
            open_ring(Ring::new(pts(), [Some(10.0), None, Some(12.0), Some(10.0)]));
        assert_eq!(
            (ring.zs, closed, differs),
            (Some(vec![Some(10.0), None, Some(12.0)]), true, false)
        );
        // The closing position with another height goes, and it is said; without one it goes unsaid.
        let (ring, _, differs) = open_ring(Ring::new(pts(), [Some(10.0), None, None, Some(9.0)]));
        assert_eq!(
            (ring.zs, differs),
            (Some(vec![Some(10.0), None, None]), true)
        );
        let (_, _, differs) = open_ring(Ring::new(pts(), [Some(10.0), None, None, None]));
        assert!(!differs);
        // A height only the closing position had leaves the ring flat, and that is said.
        let (ring, _, differs) = open_ring(Ring::new(pts(), [None, None, None, Some(9.0)]));
        assert_eq!((ring.zs, differs), (None, true));
    }
}
