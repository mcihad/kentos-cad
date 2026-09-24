//! Join (`apps/web/src/model/ops/join.ts`): lines, arcs and open polylines whose ends
//! meet become polylines with arc segments; a chain that returns to its
//! start becomes a closed polygon. Entities that connect to nothing are
//! left out.

use crate::api::Op;
use crate::api::json::{Json, ToJson, field};
use crate::entity::{Entity, Shape};
use crate::geom::arc::{ArcGeom, arc_end, arc_start, bulge_from_arc};
use crate::geom::bulge::{bulge_at, clean_bulge_path, reverse_bulge_path};
use crate::jsmath::js_hypot;
use crate::op;
use crate::vec2::Vec2;

/// An open run of vertices with segment bulges (bulges[i] for pts[i] → pts[i+1]).
#[derive(Clone)]
struct Chain {
    pts: Vec<Vec2>,
    bulges: Vec<f64>,
}

fn chain_of(e: &Shape) -> Option<Chain> {
    match e {
        Shape::Line { a, b } => Some(Chain {
            pts: vec![*a, *b],
            bulges: vec![0.0, 0.0],
        }),
        Shape::Arc { c, r, a0, a1 } => {
            let g = ArcGeom {
                c: *c,
                r: *r,
                a0: *a0,
                a1: *a1,
            };
            Some(Chain {
                pts: vec![arc_start(&g), arc_end(&g)],
                bulges: vec![bulge_from_arc(&g), 0.0],
            })
        }
        Shape::Polyline { pts, bulges, .. } => Some(Chain {
            pts: pts.clone(),
            bulges: (0..pts.len())
                .map(|i| bulge_at(bulges.as_deref(), i))
                .collect(),
        }),
        _ => None,
    }
}

fn near(a: Vec2, b: Vec2, tol: f64) -> bool {
    js_hypot(a.x - b.x, a.y - b.y) <= tol
}

// An empty polyline has no ends; the TypeScript throws reading them.
const NO_ENDS: &str = "Uçları olmayan boş bir çoklu çizgi birleştirilemez.";

fn first(c: &Chain) -> Result<Vec2, String> {
    c.pts.first().copied().ok_or_else(|| NO_ENDS.to_string())
}

fn last(c: &Chain) -> Result<Vec2, String> {
    c.pts.last().copied().ok_or_else(|| NO_ENDS.to_string())
}

fn reversed(c: &Chain) -> Chain {
    let r = reverse_bulge_path(&c.pts, Some(&c.bulges), false);
    Chain {
        pts: r.pts,
        bulges: r.bulges.unwrap_or_default(),
    }
}

/// a followed by b; b's first point is dropped (it meets a's last).
fn append(a: &Chain, b: &Chain) -> Chain {
    let mut pts = a.pts.clone();
    pts.extend(b.pts.iter().skip(1).copied());
    let mut bulges: Vec<f64> = a.bulges[..a.bulges.len().saturating_sub(1)].to_vec();
    bulges.extend_from_slice(&b.bulges);
    Chain { pts, bulges }
}

fn to_geometry(c: &Chain, tol: f64) -> Result<Entity, String> {
    let closed = c.pts.len() > 2 && near(first(c)?, last(c)?, tol);
    let (pts, bulges) = if closed {
        // Closed: the last segment (into the dropped duplicate) becomes the closing one.
        (
            &c.pts[..c.pts.len() - 1],
            &c.bulges[..c.bulges.len().saturating_sub(1)],
        )
    } else {
        (&c.pts[..], &c.bulges[..])
    };
    let clean = clean_bulge_path(pts, Some(bulges), closed, 1e-9);
    Ok(Entity::new(if closed {
        Shape::Polygon {
            pts: clean.pts,
            bulges: clean.bulges,
            holes: None,
        }
    } else {
        Shape::Polyline {
            pts: clean.pts,
            bulges: clean.bulges,
            holes: None,
        }
    }))
}

pub struct JoinGroup {
    pub geometry: Entity,
    /// Source ids in chain order; the first decides layer and attributes.
    pub sources: Vec<Json>,
}

pub struct Joined {
    pub groups: Vec<JoinGroup>,
    pub skipped: Vec<Json>,
}

impl ToJson for JoinGroup {
    fn write_json(&self, out: &mut String) {
        out.push('{');
        let mut first = true;
        field(out, &mut first, "geometry", &self.geometry);
        field(out, &mut first, "sources", &self.sources);
        out.push('}');
    }
}

impl ToJson for Joined {
    fn write_json(&self, out: &mut String) {
        out.push('{');
        let mut first = true;
        field(out, &mut first, "groups", &self.groups);
        field(out, &mut first, "skipped", &self.skipped);
        out.push('}');
    }
}

fn id_of(e: &Entity) -> Json {
    e.rest
        .iter()
        .find(|(k, _)| k == "id")
        .map(|(_, v)| v.clone())
        .unwrap_or(Json::Null)
}

/// Joins the entities whose ends meet within `tol`, in the order given.
pub fn join_entities(list: &[Entity], tol: f64) -> Result<Joined, String> {
    let items: Vec<(Json, Option<Chain>)> = list
        .iter()
        .map(|e| (id_of(e), chain_of(&e.shape)))
        .collect();
    let mut skipped: Vec<Json> = items
        .iter()
        .filter(|(_, c)| c.is_none())
        .map(|(id, _)| id.clone())
        .collect();
    let pool: Vec<(&Json, &Chain)> = items
        .iter()
        .filter_map(|(id, c)| c.as_ref().map(|c| (id, c)))
        .collect();
    let mut used: Vec<&Json> = Vec::new();
    let mut groups: Vec<JoinGroup> = Vec::new();

    for &(seed_id, seed) in &pool {
        if used.contains(&seed_id) {
            continue;
        }
        used.push(seed_id);
        let mut chain = seed.clone();
        let mut sources: Vec<Json> = vec![seed_id.clone()];
        let mut grew = true;
        while grew {
            grew = false;
            for &(id, c) in &pool {
                if used.contains(&id) {
                    continue;
                }
                let head = first(&chain)?;
                let tail = last(&chain)?;
                if near(head, tail, tol) && chain.pts.len() > 2 {
                    break; // already closed
                }
                let s = first(c)?;
                let f = last(c)?;
                let (next, at_tail) = if near(tail, s, tol) {
                    (c.clone(), true)
                } else if near(tail, f, tol) {
                    (reversed(c), true)
                } else if near(head, f, tol) {
                    (c.clone(), false)
                } else if near(head, s, tol) {
                    (reversed(c), false)
                } else {
                    continue;
                };
                if at_tail {
                    chain = append(&chain, &next);
                    sources.push(id.clone());
                } else {
                    chain = append(&next, &chain);
                    sources.insert(0, id.clone());
                }
                used.push(id);
                grew = true;
            }
        }
        if sources.len() < 2 {
            continue;
        }
        groups.push(JoinGroup {
            geometry: to_geometry(&chain, tol)?,
            sources,
        });
    }
    let joined: Vec<&Json> = groups.iter().flat_map(|g| g.sources.iter()).collect();
    for &(id, _) in &pool {
        if !joined.contains(&id) {
            skipped.push(id.clone());
        }
    }
    Ok(Joined { groups, skipped })
}

pub(crate) static OPS: &[Op] = &[op!("joinEntities", |list: Vec<Entity>, tol: f64| {
    join_entities(&list, tol)
})];
