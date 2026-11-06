//! A terrain surface's input gathered from the drawing (plan/road.md Y2.2, docs/adr/0900 §10): its definition names
//! layers and objects, this reads their geometry into a [`SurfaceInput`], says what it could not use and why, and
//! takes the digest of what it read, so a surface whose sources changed since it was last accepted is known out of
//! date (Civil 3D's “out of date”). The drawing's numbers are metres in the project's plane, as the surface's.
//!
//! What each source gives:
//! - a point object: each of its places with an elevation (a multi-point object's parts too); one without an
//!   elevation is left out (`no_height`);
//! - a breakline: a line with both ends' elevations; a polyline's each part, or a polygon's outer ring and holes,
//!   closed, with every vertex's elevation (`no_height` else); an arc edge is left out (`arc_edge`: Y2.2 takes straight
//!   breaklines only);
//! - the boundary and a void: a polygon's outer ring or a closed polyline, of straight edges; its heights are the
//!   surface's (drape), its own elevations are not read.
//!
//! An object named by its id that is not of a kind its list takes is left out (`wrong_kind`); a layer's objects of
//! other kinds are not its sources and are passed over in silence (a layer of points may hold their names' texts).

use std::collections::{BTreeMap, BTreeSet};

use kentos_contracts::{Entity, LayerNode, PathEntity, SurfaceEdit, TerrainSurfaceDef};
use serde::Serialize;

use crate::surface::{Breakline, EdgeMethod, ForcedEdge, RemovedAt, Ring, SpotPoint, SurfaceInput, ALGORITHM};

/// One object of the drawing as a source sees it.
#[derive(Clone, Copy, Debug)]
pub struct SourceObject<'a> {
    /// Its persistent id.
    pub uid: &'a str,
    pub entity: &'a Entity,
}

/// An object (or a layer) a surface names and could not use, and why.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Left {
    /// The object's id, or the layer's.
    pub id: String,
    /// The list that names it: `points`, `breaklines`, `boundary`, `voids`.
    pub role: &'static str,
    /// `missing` (not in the drawing), `no_height`, `arc_edge`, `wrong_kind`, `not_closed`, `too_few`.
    pub why: &'static str,
}

/// A surface's input read from the drawing.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Gathered {
    pub input: SurfaceInput,
    /// What it names and could not use, in the order it names them.
    pub left: Vec<Left>,
    /// The SHA-256 (64 lower-case hex digits) of the input and the method: what `built` keeps.
    pub digest: String,
    /// The point records left out by the definition's `excluded` (docs/adr/0916 §6), and the exclusions no record is at.
    pub excluded: usize,
    pub unmatched: Vec<[f64; 2]>,
}

/// A surface's state against its sources.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SurfaceState {
    /// Made from the sources it was last accepted from.
    Current,
    /// Its sources changed since it was last accepted.
    Stale,
    /// Never accepted.
    New,
}

impl Gathered {
    /// Its state against the definition's `built`.
    pub fn state(&self, def: &TerrainSurfaceDef) -> SurfaceState {
        match def.built.as_deref() {
            None => SurfaceState::New,
            Some(d) if d == self.digest => SurfaceState::Current,
            Some(_) => SurfaceState::Stale,
        }
    }
}

/// The layers (and the layers of the groups) named, and the names that are no node of the tree.
pub fn layer_set<'a>(tree: &'a [LayerNode], named: &'a [String]) -> (BTreeSet<&'a str>, Vec<&'a str>) {
    fn find<'t>(nodes: &'t [LayerNode], id: &str) -> Option<&'t LayerNode> {
        nodes
            .iter()
            .find_map(|n| if n.id == id { Some(n) } else { find(&n.children, id) })
    }
    fn under<'t>(node: &'t LayerNode, out: &mut BTreeSet<&'t str>) {
        out.insert(&node.id);
        for c in &node.children {
            under(c, out);
        }
    }
    let mut set = BTreeSet::new();
    let mut missing = Vec::new();
    for id in named {
        match find(tree, id) {
            Some(n) => under(n, &mut set),
            None => missing.push(id.as_str()),
        }
    }
    (set, missing)
}

/// A surface's input from the drawing's layer tree and objects (in the drawing's order).
pub fn gather<'a>(
    def: &TerrainSurfaceDef,
    tree: &[LayerNode],
    objects: impl IntoIterator<Item = SourceObject<'a>>,
) -> Gathered {
    let objects: Vec<SourceObject<'a>> = objects.into_iter().collect();
    let by_uid: BTreeMap<&str, &SourceObject<'a>> = objects.iter().map(|o| (o.uid, o)).collect();
    let mut left = Vec::new();
    let leave = |left: &mut Vec<Left>, id: &str, role: &'static str, why: &'static str| {
        left.push(Left {
            id: id.to_string(),
            role,
            why,
        })
    };

    let mut points = Vec::new();
    let mut breaklines = Vec::new();
    let place = |x: f64, y: f64| ((x + 0.0).to_bits(), (y + 0.0).to_bits());
    let out: BTreeSet<(u64, u64)> = def.excluded.iter().map(|e| place(e.x, e.y)).collect();
    let mut hit: BTreeSet<(u64, u64)> = BTreeSet::new();
    let mut excluded = 0usize;
    for (role, sources) in [("points", &def.points), ("breaklines", &def.breaklines)] {
        let (layers, missing) = layer_set(tree, &sources.layers);
        for id in missing {
            leave(&mut left, id, role, "missing");
        }
        for uid in &sources.uids {
            if !by_uid.contains_key(uid.as_str()) {
                leave(&mut left, uid, role, "missing");
            }
        }
        let named: BTreeSet<&str> = sources.uids.iter().map(String::as_str).collect();
        for o in &objects {
            let by_name = named.contains(o.uid);
            if !by_name && !layers.contains(o.entity.base().layer_id.as_str()) {
                continue;
            }
            let source = Some(o.uid.to_string());
            if role == "points" {
                match point_places(o.entity) {
                    Some(places) => {
                        let mut lost = false;
                        for (p, z) in places {
                            match z {
                                Some(_) if out.contains(&place(p[0], p[1])) => {
                                    excluded += 1;
                                    hit.insert(place(p[0], p[1]));
                                }
                                Some(z) => points.push(SpotPoint {
                                    x: p[0],
                                    y: p[1],
                                    z,
                                    source: source.clone(),
                                }),
                                None => lost = true,
                            }
                        }
                        if lost {
                            leave(&mut left, o.uid, role, "no_height");
                        }
                    }
                    None if by_name => leave(&mut left, o.uid, role, "wrong_kind"),
                    None => {}
                }
            } else {
                match lines_of(o.entity) {
                    Some(Ok(lines)) => breaklines.extend(lines.into_iter().map(|(pts, closed)| Breakline {
                        points: pts,
                        closed,
                        source: source.clone(),
                    })),
                    Some(Err(why)) => leave(&mut left, o.uid, role, why),
                    None if by_name => leave(&mut left, o.uid, role, "wrong_kind"),
                    None => {}
                }
            }
        }
    }

    // The edits (ADR 1560 §2), in their order: a point where no record is; a height for every point record and
    // breakline vertex at the place; the edges and the triangles taken out go to the core, which says back the ones
    // it cannot do.
    let mut edges = Vec::new();
    let mut removed = Vec::new();
    for (k, e) in def.edits.iter().enumerate() {
        let id = format!("edits[{k}]");
        let edit = u32::try_from(k).unwrap_or(u32::MAX);
        let at = |x: f64, y: f64| place(x, y);
        match *e {
            SurfaceEdit::AddPoint { x, y, z } => {
                let here = at(x, y);
                let taken = points.iter().any(|p| at(p.x, p.y) == here)
                    || breaklines.iter().flat_map(|b| &b.points).any(|p| at(p[0], p[1]) == here)
                    || out.contains(&here);
                if taken {
                    leave(&mut left, &id, "edits", "occupied");
                } else {
                    points.push(SpotPoint { x, y, z, source: Some(id) });
                }
            }
            SurfaceEdit::SetHeight { x, y, z } => {
                let here = at(x, y);
                let mut n = 0;
                for p in points.iter_mut().filter(|p| at(p.x, p.y) == here) {
                    p.z = z;
                    n += 1;
                }
                for p in breaklines.iter_mut().flat_map(|b| b.points.iter_mut()).filter(|p| at(p[0], p[1]) == here) {
                    p[2] = z;
                    n += 1;
                }
                if n == 0 {
                    leave(&mut left, &id, "edits", "no_point");
                }
            }
            SurfaceEdit::AddEdge { ax, ay, bx, by } => edges.push(ForcedEdge { from: [ax, ay], to: [bx, by], edit }),
            SurfaceEdit::RemoveTriangle { x, y } => removed.push(RemovedAt { at: [x, y], edit }),
        }
    }

    let mut ring = |uid: &str, role: &'static str| -> Option<Ring> {
        let Some(o) = by_uid.get(uid) else {
            leave(&mut left, uid, role, "missing");
            return None;
        };
        match ring_of(o.entity) {
            Ok(points) => Some(Ring {
                points,
                source: Some(uid.to_string()),
            }),
            Err(why) => {
                leave(&mut left, uid, role, why);
                None
            }
        }
    };
    let boundary = def.boundary.as_deref().and_then(|b| ring(b, "boundary"));
    let voids: Vec<Ring> = def.voids.iter().filter_map(|v| ring(v, "voids")).collect();

    let input = SurfaceInput {
        points,
        breaklines,
        boundary,
        voids,
        tolerance: def.tolerance,
        edge: match def.max_edge {
            Some(length) => EdgeMethod::MaxEdge { length },
            None => EdgeMethod::None,
        },
        edges,
        removed,
    };
    let digest = digest_of(&input);
    let unmatched = def.excluded.iter().filter(|e| !hit.contains(&place(e.x, e.y))).map(|e| [e.x, e.y]).collect();
    Gathered {
        input,
        left,
        digest,
        excluded,
        unmatched,
    }
}

/// The digest of what a surface is made from and how: the method's name and the input, as canonical JSON.
pub fn digest_of(input: &SurfaceInput) -> String {
    let value = serde_json::json!({ "algorithm": ALGORITHM, "input": input });
    kentos_contracts::canonical::digest_of(&value, 64)
}

/// A point object's places and their elevations; none for another kind.
fn point_places(e: &Entity) -> Option<Vec<([f64; 2], Option<f64>)>> {
    let Entity::Point(p) = e else { return None };
    let mut out = vec![([p.p.x, p.p.y], p.z)];
    out.extend(p.parts.iter().flatten().map(|q| ([q.p.x, q.p.y], q.z)));
    Some(out)
}

type Lines = Vec<(Vec<[f64; 3]>, bool)>;

/// A breakline object's paths with their vertices' elevations (and whether each is closed); none for another kind.
fn lines_of(e: &Entity) -> Option<Result<Lines, &'static str>> {
    let path = |pts: &[kentos_contracts::Vec2], zs: &Option<Vec<Option<f64>>>, bulges: &Option<Vec<f64>>| {
        if bulges.iter().flatten().any(|b| *b != 0.0) {
            return Err("arc_edge");
        }
        let Some(zs) = zs else { return Err("no_height") };
        if zs.len() != pts.len() || zs.iter().any(Option::is_none) {
            return Err("no_height");
        }
        Ok(pts.iter().zip(zs).map(|(p, z)| [p.x, p.y, z.unwrap_or_default()]).collect::<Vec<_>>())
    };
    Some(match e {
        Entity::Line(l) => match (l.za, l.zb) {
            (Some(za), Some(zb)) => Ok(vec![(vec![[l.a.x, l.a.y, za], [l.b.x, l.b.y, zb]], false)]),
            _ => Err("no_height"),
        },
        Entity::Polyline(p) => (|| {
            let mut out = vec![(path(&p.pts, &p.zs, &p.bulges)?, p.closed)];
            for part in p.parts.iter().flatten() {
                out.push((path(&part.pts, &part.zs, &part.bulges)?, false));
            }
            Ok(out)
        })(),
        Entity::Polygon(p) => (|| {
            let mut out = Vec::new();
            for (pts, zs, bulges, holes) in areas(p) {
                out.push((path(pts, zs, bulges)?, true));
                for h in holes {
                    out.push((path(&h.pts, &h.zs, &h.bulges)?, true));
                }
            }
            Ok(out)
        })(),
        _ => return None,
    })
}

type AreaRef<'a> = (
    &'a [kentos_contracts::Vec2],
    &'a Option<Vec<Option<f64>>>,
    &'a Option<Vec<f64>>,
    &'a [kentos_contracts::RingGeometry],
);

/// An area's parts: the first is its own fields'.
fn areas(p: &PathEntity) -> Vec<AreaRef<'_>> {
    let mut out = vec![(&p.pts[..], &p.zs, &p.bulges, p.holes.as_deref().unwrap_or_default())];
    out.extend(
        p.parts
            .iter()
            .flatten()
            .map(|q| (&q.pts[..], &q.zs, &q.bulges, q.holes.as_deref().unwrap_or_default())),
    );
    out
}

/// A boundary's or a void's ring: an area's single outer ring or a closed polyline's path, straight edges only.
fn ring_of(e: &Entity) -> Result<Vec<[f64; 2]>, &'static str> {
    let (pts, bulges) = match e {
        Entity::Polygon(p) if p.parts.as_ref().is_none_or(Vec::is_empty) => (&p.pts, &p.bulges),
        Entity::Polygon(_) => return Err("wrong_kind"),
        Entity::Polyline(p) if p.closed => (&p.pts, &p.bulges),
        Entity::Polyline(_) => return Err("not_closed"),
        _ => return Err("wrong_kind"),
    };
    if bulges.iter().flatten().any(|b| *b != 0.0) {
        return Err("arc_edge");
    }
    if pts.len() < 3 {
        return Err("too_few");
    }
    Ok(pts.iter().map(|p| [p.x, p.y]).collect())
}
