//! What a size does to a corner (the web's `Corner.plan` and `piece`): the
//! sides' new geometry and the arc or the cut, from the shared core
//! (`corner_of_path`, `fillet_lines`, `chamfer_lines`, `fillet_arc`,
//! `chamfer_line`).

use kentos_contracts::Entity;
use kentos_domain::{Document, Slot};
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::ops::fillet::{
    Chamfer, Corner as Joined, CornerOp, CornerResult, Fillet, Seg, chamfer_lines, corner_of_path,
    fillet_lines,
};
use kentos_geometry_core::tools::editing::{chamfer_line, fillet_arc};
use kentos_native_application::geometry::shape;

use super::site::{Corner, Site, line_of};
use crate::Vec2;

/// A size: a fillet's radius, or a chamfer's two distances.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Op {
    Radius(f64),
    Cut(f64, f64),
}

impl Op {
    pub fn core(self) -> CornerOp {
        match self {
            Op::Radius(r) => CornerOp::Radius(r),
            Op::Cut(d1, d2) => CornerOp::Chamfer(d1, d2),
        }
    }
}

/// What a size does to the corner: the sides' new geometry and the piece
/// added (the arc or the cut), made from the first side.
pub(super) struct Plan {
    pub updates: Vec<(Slot, Shape)>,
    pub add: Option<(Slot, Shape)>,
}

/// What `op` does to the corner (the web's `Corner.plan`), or why it cannot.
pub(super) fn plan(c: &Corner, op: Op, doc: &Document) -> Result<Plan, String> {
    match c.site {
        Site::Vertex { slot, vertex } => {
            let e = doc.get(slot).ok_or_else(String::new)?;
            let (Entity::Polyline(path) | Entity::Polygon(path)) = e else {
                return Err(String::new());
            };
            let closed = matches!(e, Entity::Polygon(_));
            let pts: Vec<Vec2> = path.pts.iter().map(|p| Vec2::new(p.x, p.y)).collect();
            match corner_of_path(&pts, path.bulges.as_deref(), closed, vertex, &op.core())? {
                CornerResult::Error(error) => Err(error),
                CornerResult::Path(p) => {
                    // The whole geometry is written (docs/adr/0047): a closed area keeps its holes.
                    let s = match shape(e) {
                        Shape::Polygon { holes, .. } => Shape::Polygon {
                            pts: p.pts,
                            bulges: p.bulges,
                            holes,
                        },
                        _ => Shape::Polyline {
                            pts: p.pts,
                            bulges: p.bulges,
                            holes: None,
                        },
                    };
                    Ok(Plan {
                        updates: vec![(slot, s)],
                        add: None,
                    })
                }
            }
        }
        Site::Lines {
            first,
            pick1,
            second,
            pick2,
        } => {
            let l1 = doc.get(first).and_then(line_of).ok_or_else(String::new)?;
            let l2 = doc.get(second).and_then(line_of).ok_or_else(String::new)?;
            let line = |s: Seg| Shape::Line { a: s.a, b: s.b };
            let (joined, piece) = match op {
                Op::Radius(r) => {
                    let Fillet(j) = fillet_lines(l1, pick1, l2, pick2, r);
                    match j {
                        Joined::Error(e) => return Err(e),
                        Joined::Ok { line1, line2, join } => (
                            (line1, line2),
                            join.map(|a| Shape::Arc {
                                c: a.c,
                                r: a.r,
                                a0: a.a0,
                                a1: a.a1,
                            }),
                        ),
                    }
                }
                Op::Cut(d1, d2) => {
                    let Chamfer(j) = chamfer_lines(l1, pick1, l2, pick2, d1, d2);
                    match j {
                        Joined::Error(e) => return Err(e),
                        Joined::Ok { line1, line2, join } => ((line1, line2), join.map(line)),
                    }
                }
            };
            Ok(Plan {
                updates: vec![(first, line(joined.0)), (second, line(joined.1))],
                add: piece.map(|s| (first, s)),
            })
        }
    }
}

/// The arc or the cut alone, for Kırp: hayır.
pub(super) fn piece(c: &Corner, op: Op) -> Option<Shape> {
    match op {
        Op::Radius(r) => fillet_arc(&c.geom, r).map(|a| Shape::Arc {
            c: a.c,
            r: a.r,
            a0: a.a0,
            a1: a.a1,
        }),
        Op::Cut(d1, d2) => chamfer_line(&c.geom, d1, d2).map(|p| Shape::Line { a: p.a, b: p.b }),
    }
}
