//! An import's objects moved into the project's coordinate system (docs/adr/0222
//! §2): a GeoJSON, Shapefile or KML file whose system is known and is not the
//! project's comes in only when the user asks for the move, never silently
//! (CLAUDE.md §5). Every vertex moves the way Koordinat dönüştür moves a point
//! (docs/adr/0167, 0168: the projections, the datum shift, the project's datum
//! choices); elevations stay as they are (the move is 2D). The readers of these
//! files give points, lines and paths of straight edges only; any other kind,
//! and an object one of whose vertices the target system does not reach, is
//! left out and said (and no longer counted). The layers' boxes and the view
//! are taken again; a layer left without objects is not imported.

use std::collections::{BTreeMap, HashMap};

use kentos_contracts::{Bounds, Entity, ImportResult, ReportItem, Vec2};
use kentos_geometry_core::crs::{Choice, System, Unreached, transform_in};

use crate::import::{defining_bounds, view_bounds};

/// What a move did: the objects moved and the vertices, the objects left out
/// by why, and what the move rests on (the datum shift's accuracy in metres,
/// none when not known; its operations, empty when only the projection
/// changed; whether an unofficial operation was used).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Moved {
    pub objects: u32,
    pub vertices: u64,
    pub left: Vec<(Unreached, u32)>,
    pub kinds_left: u32,
    pub accuracy: Option<f64>,
    pub via: String,
    pub unofficial: bool,
}

/// Why a vertex was not moved, as a report says it.
pub fn why(u: Unreached) -> &'static str {
    match u {
        Unreached::Outside => "projenin sisteminin ulaştığı yerin dışında",
        Unreached::NoLink => "datumlardan birinin WGS 84'e dönüşümü yok",
        Unreached::NoGrid => "datum dönüşümünün ızgarası bu cihazda yok",
        Unreached::OutsideGrid => "datum dönüşümünün ızgarasının dışında",
    }
}

/// Every vertex of an object, mutably, in a fixed order; false for a kind
/// that is not points and straight paths.
fn vertices(e: &mut Entity, f: &mut dyn FnMut(&mut Vec2)) -> bool {
    match e {
        Entity::Point(p) => {
            f(&mut p.p);
            for q in p.parts.iter_mut().flatten() {
                f(&mut q.p);
            }
            true
        }
        Entity::Line(l) => {
            f(&mut l.a);
            f(&mut l.b);
            true
        }
        Entity::Polyline(p) | Entity::Polygon(p) => {
            let straight =
                |b: &Option<Vec<f64>>| b.as_ref().is_none_or(|b| b.iter().all(|x| *x == 0.0));
            let arcs = !straight(&p.bulges)
                || p.holes.iter().flatten().any(|h| !straight(&h.bulges))
                || p.parts.iter().flatten().any(|q| {
                    !straight(&q.bulges) || q.holes.iter().flatten().any(|h| !straight(&h.bulges))
                });
            if arcs {
                return false;
            }
            p.pts.iter_mut().for_each(&mut *f);
            for h in p.holes.iter_mut().flatten() {
                h.pts.iter_mut().for_each(&mut *f);
            }
            for q in p.parts.iter_mut().flatten() {
                q.pts.iter_mut().for_each(&mut *f);
                for h in q.holes.iter_mut().flatten() {
                    h.pts.iter_mut().for_each(&mut *f);
                }
            }
            true
        }
        _ => false,
    }
}

/// One object moved from `from` into `to` in place (a bound layer's object and its table's row, docs/adr/0504 §1).
/// `Err(None)`: a kind that is not points and straight paths (an arc does not move exactly); `Err(Some(why))`: a
/// vertex the target does not reach. The object is unchanged when it is refused.
pub fn move_entity(
    e: &mut Entity,
    from: &System,
    to: &System,
    choices: &[Choice],
) -> Result<(), Option<Unreached>> {
    let mut moved = e.clone();
    let mut miss: Option<Unreached> = None;
    let taken = vertices(&mut moved, &mut |p: &mut Vec2| {
        if miss.is_some() {
            return;
        }
        match transform_in(from, to, kentos_geometry_core::Vec2::new(p.x, p.y), choices) {
            Ok(t) => {
                *p = Vec2 {
                    x: t.point.x,
                    y: t.point.y,
                }
            }
            Err(u) => miss = Some(u),
        }
    });
    if !taken {
        return Err(None);
    }
    if let Some(u) = miss {
        return Err(Some(u));
    }
    *e = moved;
    Ok(())
}

/// The import's objects moved from `from` into `to`, the project's datum
/// choices taken; those not moved are left out and said in the report.
pub fn reproject(
    result: &mut ImportResult,
    from: &System,
    to: &System,
    choices: &[Choice],
) -> Moved {
    let mut moved = Moved::default();
    let mut left: HashMap<&'static str, (Unreached, u32)> = HashMap::new();
    let mut kinds_left = 0u32;
    let entities = std::mem::take(&mut result.entities);
    let mut kept = Vec::with_capacity(entities.len());
    let mut first = true;
    for mut e in entities {
        let mut miss: Option<Unreached> = None;
        let mut count = 0u64;
        let taken = vertices(&mut e, &mut |p: &mut Vec2| {
            if miss.is_some() {
                return;
            }
            match transform_in(from, to, kentos_geometry_core::Vec2::new(p.x, p.y), choices) {
                Ok(t) => {
                    if first {
                        moved.accuracy = t.accuracy;
                        moved.via = t.via.clone();
                        moved.unofficial = t.unofficial;
                        first = false;
                    }
                    *p = Vec2 {
                        x: t.point.x,
                        y: t.point.y,
                    };
                    count += 1;
                }
                Err(u) => miss = Some(u),
            }
        });
        if !taken {
            kinds_left += 1;
            uncount(result, &e);
            continue;
        }
        if let Some(u) = miss {
            left.entry(u.as_str()).or_insert((u, 0)).1 += 1;
            uncount(result, &e);
            continue;
        }
        moved.objects += 1;
        moved.vertices += count;
        kept.push(e);
    }
    result.entities = kept;
    let mut reasons: Vec<(Unreached, u32)> = left.into_values().collect();
    reasons.sort_by_key(|(u, _)| u.as_str());
    for (u, n) in &reasons {
        result.report.skipped.push(ReportItem {
            what: "Çevrilemeyen nesne".into(),
            count: *n,
            reason: format!("bir köşesi {}; alınmadı", why(*u)),
            lines: Vec::new(),
        });
    }
    if kinds_left > 0 {
        result.report.skipped.push(ReportItem {
            what: "Eğrili ya da başka türden nesne".into(),
            count: kinds_left,
            reason: "projenin sistemine yalnız noktalar ve düz kenarlı yollar çevrilir; alınmadı"
                .into(),
            lines: Vec::new(),
        });
    }
    moved.left = reasons;
    moved.kinds_left = kinds_left;
    rebox(result);
    moved
}

/// An object left out is not among those the report counts.
fn uncount(result: &mut ImportResult, e: &Entity) {
    if let Some(n) = result.report.counts.get_mut(e.kind()) {
        *n = n.saturating_sub(1);
        if *n == 0 {
            result.report.counts.remove(e.kind());
        }
    }
}

/// The layers' objects by kind and their boxes, the import's box and its
/// view, taken again; a layer left without objects is not imported.
fn rebox(result: &mut ImportResult) {
    #[derive(Default)]
    struct Layer {
        kinds: BTreeMap<String, u32>,
        bounds: Option<Bounds>,
    }
    let mut per: HashMap<&str, Layer> = HashMap::new();
    let mut boxes = Vec::with_capacity(result.entities.len());
    for e in &result.entities {
        let layer = per.entry(e.base().layer_id.as_str()).or_default();
        *layer.kinds.entry(e.kind().to_owned()).or_default() += 1;
        let Some(b) = defining_bounds(e) else {
            continue;
        };
        layer.bounds = Some(match layer.bounds {
            Some(o) => Bounds {
                min_x: o.min_x.min(b.min_x),
                min_y: o.min_y.min(b.min_y),
                max_x: o.max_x.max(b.max_x),
                max_y: o.max_y.max(b.max_y),
            },
            None => b,
        });
        boxes.push(b);
    }
    for layer in &mut result.layers {
        let l = per.remove(layer.name.as_str()).unwrap_or_default();
        layer.count = l.kinds.values().sum();
        layer.kinds = l.kinds;
        layer.bounds = l.bounds;
    }
    result.layers.retain(|l| l.count > 0);
    result.bounds = boxes.iter().copied().reduce(|a, b| Bounds {
        min_x: a.min_x.min(b.min_x),
        min_y: a.min_y.min(b.min_y),
        max_x: a.max_x.max(b.max_x),
        max_y: a.max_y.max(b.max_y),
    });
    result.view = view_bounds(&boxes);
}
