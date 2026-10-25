//! One joined boundary network for many room queries and bounded void discovery.
//! Arc geometry stays analytic. Wall material is classified by the directed
//! faces/caps of the same joined network, never by the tessellated render fill.
use crate::entity::{Entity, Shape};
use crate::geom::arrangement::{Area, WindingIndex, reverse_edge, translate_edge};
use crate::geom::intersect::Edge;
use crate::geom::region::FaceIndex;
use crate::geom::space::{self, Method};
use crate::geom::wall_join::{WallIn, solve};
use crate::vec2::Vec2;

/// A bounded void and its analytic interior probe. `net` is always the free
/// face; `boundary` uses the requested measurement method.
pub struct RoomFace {
    pub seed: Vec2,
    pub net: Area,
    pub boundary: Area,
}

/// Derived and immutable: callers must rebuild after any source change.
pub struct BoundaryIndex {
    faces: Vec<(Shape, f64)>,
    index: FaceIndex,
    material: Option<WindingIndex<'static>>,
    origin: Vec2,
    caps_known: bool,
}

fn near(a: Vec2, b: Vec2) -> bool {
    (a.x - b.x).abs() <= space::ON_FACE && (a.y - b.y).abs() <= space::ON_FACE
}

impl BoundaryIndex {
    pub fn new(walls: &[WallIn<'_>]) -> Self {
        Self::build(walls, true)
    }

    /// Repeated legacy seed queries do not need material classification.
    pub fn for_queries(walls: &[WallIn<'_>]) -> Self {
        Self::build(walls, false)
    }

    fn build(walls: &[WallIn<'_>], discovery: bool) -> Self {
        let solved = solve(walls, None);
        let mut faces = Vec::new();
        let mut edges = Vec::new();
        let mut caps_known = true;
        for (wall, drawn) in walls.iter().zip(solved.drawn) {
            let Some(drawn) = drawn else {
                caps_known = false;
                continue;
            };
            let last = drawn.lines.len().saturating_sub(1);
            for (k, strokes) in drawn.lines.iter().enumerate() {
                if k != 0 && k != last {
                    continue;
                }
                for stroke in strokes {
                    let shape = stroke.line.shape(stroke.closed);
                    // Material is right of the left face and left of the right
                    // face. Reversing the former leaves material on the left.
                    if discovery {
                        edges.extend(
                            crate::ops::edges::entity_edges(&shape)
                                .iter()
                                .map(|e| if k == 0 { reverse_edge(e) } else { *e }),
                        );
                    }
                    faces.push((shape, wall.spec.width));
                }
            }
            for (a, b) in drawn.caps {
                faces.push((Shape::Line { a, b }, 0.0));
                if !discovery {
                    continue;
                }
                let left = drawn.lines.first().into_iter().flatten();
                let direction = left.filter(|s| !s.closed).find_map(|s| {
                    let (start, end) = (*s.line.pts.first()?, *s.line.pts.last()?);
                    if near(a, start) || near(b, end) {
                        Some((a, b))
                    } else if near(b, start) || near(a, end) {
                        Some((b, a))
                    } else {
                        None
                    }
                });
                if let Some((a, b)) = direction {
                    edges.push(Edge::Seg { a, b });
                } else {
                    caps_known = false;
                }
            }
        }
        let lines: Vec<_> = faces.iter().map(|(s, _)| Entity::new(s.clone())).collect();
        let index = FaceIndex::new(&[crate::ops::areas::line_source(&lines)]);
        let origin = walls
            .first()
            .and_then(|w| w.pts.first())
            .copied()
            .unwrap_or_default();
        let edges = edges
            .iter()
            .map(|e| translate_edge(e, -origin.x, -origin.y))
            .collect();
        Self {
            faces,
            index,
            material: discovery.then(|| WindingIndex::owned(edges)),
            origin,
            caps_known,
        }
    }

    /// Existing seed semantics, including a seed deliberately inside a wall.
    /// Building the index does not impose new classification on old callers.
    pub fn found(&self, seed: Vec2, method: Method) -> Option<Area> {
        self.index
            .at(seed, true)
            .map(|net| space::by_method(&net, &self.faces, method))
    }

    /// A free-room seed query for explicit new source scopes. Legacy `found`
    /// intentionally still permits a point inside physical wall material.
    pub fn room_at(&self, seed: Vec2, method: Method) -> Result<Option<Area>, &'static str> {
        let Some(material)=&self.material else {
            return Err("Bu dizin oda sınıflandırması için hazırlanmadı.");
        };
        if !self.caps_known { return Err("Duvar sınırları çözülemedi."); }
        let local=Vec2::new(seed.x-self.origin.x,seed.y-self.origin.y);
        if material.winding(local)!=0. { return Ok(None); }
        Ok(self.found(seed,method))
    }

    /// Every bounded free face. Unknown cap direction is an explicit failure,
    /// so discovery cannot silently turn wall material into a room.
    pub fn rooms(&self, method: Method) -> Result<Vec<RoomFace>, &'static str> {
        let Some(material) = &self.material else {
            return Err("Bu dizin yalnız nokta sorgusu için hazırlandı.");
        };
        if !self.caps_known {
            return Err("Duvar sınırları çözülemedi; toplu mahal bulunamadı.");
        }
        Ok(self
            .index
            .faces_with_probes()
            .into_iter()
            .filter_map(|(net, seed)| {
                let local = Vec2::new(seed.x - self.origin.x, seed.y - self.origin.y);
                if material.winding(local) != 0.0 {
                    return None;
                }
                let boundary = space::by_method(&net, &self.faces, method);
                Some(RoomFace {
                    seed,
                    net,
                    boundary,
                })
            })
            .collect())
    }
}
