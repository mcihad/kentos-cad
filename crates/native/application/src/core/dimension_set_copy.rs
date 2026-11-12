//! A dimension recipe copied and moved with its objects (docs/adr/0730 §4): a copy, an array's items and a paste carry
//! each recipe whose every source and every member they copy, moving its rows with the copies; a move, turn, scale or
//! mirror of every source of a recipe moves its rows with them. A recipe partly copied is refused, as before.

use std::collections::{HashMap, HashSet};

use kentos_commands::{DimensionSet, DimensionSetId, Entity, EntityId, Vec2};
use kentos_domain::{Document, Uuid};
use kentos_geometry_core::geom::affine::{Affine, apply, apply_linear, is_reflection};
use kentos_geometry_core::jsmath::js_hypot;
use kentos_geometry_core::vec2::Vec2 as CoreVec2;

use crate::checks::{Stop, error};

fn id_of(uid: &str) -> Option<EntityId> {
    Uuid::parse_str(uid).ok().map(|u| EntityId(u.into_bytes()))
}

/// The recipes `uids` take whole: each one a member of which is among them, refused (`dimension_set_owned`) when one of
/// its sources or members is not.
pub(crate) fn whole(doc: &Document, uids: &[String]) -> Result<Vec<DimensionSet>, Stop> {
    let chosen: HashSet<EntityId> = uids.iter().filter_map(|u| id_of(u)).collect();
    let mut out = Vec::new();
    for set in doc.dimension_sets() {
        let Some(first) = set.members.iter().find(|m| chosen.contains(&m.uid)) else {
            continue;
        };
        let members = set.members.iter().filter(|m| !chosen.contains(&m.uid)).count();
        let sources = set.sources().iter().filter(|s| !chosen.contains(s)).count();
        if members + sources > 0 {
            let at = uids
                .iter()
                .position(|u| id_of(u) == Some(first.uid))
                .unwrap_or(0);
            return Err(Stop::failed(error(
                "dimension_set_owned",
                format!(
                    "Seçimde otomatik ölçü grubunun yalnız bir kısmı var: {sources} kaynağı ve {members} ölçüsü seçilmemiş. Grubu kopyalamak ya da taşımak için bütün kaynaklarını ve ölçülerini seçin, ya da önce Özellikler panelinden “Diziyi bağımsız ölçülere dönüştür”ü kullanın."
                ),
                Some(format!("uids[{at}]")),
            )));
        }
        out.push((**set).clone());
    }
    Ok(out)
}

fn moved(m: &Affine, p: Vec2) -> Vec2 {
    let q = apply(m, CoreVec2::new(p.x, p.y));
    Vec2 { x: q.x, y: q.y }
}

/// A direction turned by `m`'s linear part, of unit length, made to read (docs/adr/0729 §1).
fn turned(m: &Affine, d: Vec2) -> Vec2 {
    let v = apply_linear(m, CoreVec2::new(d.x, d.y));
    let l = js_hypot(v.x, v.y);
    crate::dimension_sets::readable(Vec2 {
        x: v.x / l,
        y: v.y / l,
    })
}

/// A wall's edge after a mirror (docs/adr/0730 §4): the mirrored wall's path runs back (docs/adr/0339 §1), so edge `k`
/// of an open path of `points` vertices is its edge `points − 2 − k`, of a closed one `(points − 2 − k) mod points`.
pub(crate) fn mirrored_edge(k: u32, points: usize, closed: bool) -> u32 {
    let (k, n) = (i64::from(k), points as i64);
    let j = if closed { (n - 2 - k).rem_euclid(n.max(1)) } else { n - 2 - k };
    u32::try_from(j.max(0)).unwrap_or(0)
}

/// The number of a wall's path vertices and whether it is closed.
fn wall_path(doc: &Document, wall: EntityId) -> Option<(usize, bool)> {
    match doc.get(doc.slot_of(Uuid::from_bytes(wall.0))?)? {
        Entity::Wall(w) => Some((w.wall.pts.len(), w.wall.closed)),
        _ => None,
    }
}

/// A recipe as `m` moves it with its objects (docs/adr/0730 §4): its row places through `m`, a façade's or a space's
/// direction turned and made to read. A mirrored wall's path runs back, so a wall's recipe keeps its face (the body
/// stays on the side of its path it was on) and its edge is renumbered.
pub(crate) fn moved_recipe(doc: &Document, set: &DimensionSet, m: &Affine) -> DimensionSet {
    let mut out = set.clone();
    out.at = moved(m, set.at);
    for place in &mut out.row_places {
        place.at = moved(m, place.at);
    }
    match &mut out.span {
        Some(span) => span.direction = turned(m, span.direction),
        None if is_reflection(m) => {
            if let (Some(k), Some((points, closed))) = (set.edge, wall_path(doc, set.wall)) {
                out.edge = Some(mirrored_edge(k, points, closed));
            }
        }
        None => {}
    }
    out
}

/// A recipe as its copy (docs/adr/0730 §4): moved by `m`, its objects the copies `map` names (an object not copied stays
/// itself), its members the copies of its members, a new id; on `layer` when the copies go there.
pub(crate) fn copied_recipe(
    doc: &Document,
    set: &DimensionSet,
    map: &HashMap<EntityId, EntityId>,
    m: &Affine,
    layer: Option<&str>,
) -> DimensionSet {
    let to = |id: EntityId| map.get(&id).copied().unwrap_or(id);
    let mut out = moved_recipe(doc, set, m);
    out.id = DimensionSetId(Uuid::now_v7().into_bytes());
    out.wall = to(set.wall);
    if let Some(span) = &mut out.span {
        span.walls = span.walls.iter().map(|w| to(*w)).collect();
    }
    for member in &mut out.members {
        member.uid = to(member.uid);
        member.a.uid = to(member.a.uid);
        member.b.uid = to(member.b.uid);
    }
    for segment in &mut out.suppressed {
        segment.a.uid = to(segment.a.uid);
        segment.b.uid = to(segment.b.uid);
    }
    if let Some(layer) = layer {
        out.layer_id = layer.to_owned();
    }
    out.broken = None;
    out
}

/// The copies' recipes added after the copies (docs/adr/0730 §4): each source's copy by the same place in `copies` as
/// its original in `sources`, `m` the copy's transform. Inside the copy's step.
pub(crate) fn add_copies(
    doc: &mut Document,
    sets: &[DimensionSet],
    sources: &[String],
    copies: &[String],
    m: &Affine,
    layer: Option<&str>,
) -> Result<(), Stop> {
    if sets.is_empty() {
        return Ok(());
    }
    let map: HashMap<EntityId, EntityId> = sources
        .iter()
        .zip(copies)
        .filter_map(|(a, b)| Some((id_of(a)?, id_of(b)?)))
        .collect();
    for set in sets {
        let copy = copied_recipe(doc, set, &map, m, layer);
        doc.add_dimension_set_copy(copy)
            .map_err(|why| {
                Stop::failed(error(
                    "dimension_set_owned",
                    format!("Ölçü grubu kopyalanamadı: {why}"),
                    Some("uids".into()),
                ))
            })?;
    }
    Ok(())
}

/// Objects changed by `m` with the recipes they hold whole, as one change (docs/adr/0730 §4): the recipes' rows go with
/// their sources and their members keep their intervals.
pub(crate) fn move_whole(
    doc: &mut Document,
    changes: Vec<(kentos_domain::Slot, kentos_commands::Entity)>,
    sets: &[DimensionSet],
    m: &Affine,
    label: &str,
) -> Result<(), Stop> {
    let moved: Vec<DimensionSet> = sets.iter().map(|s| moved_recipe(doc, s, m)).collect();
    doc.update_with_recipes(changes, moved, label)
        .map(|_| ())
        .map_err(|why| {
            Stop::failed(error(
                "dimension_set_owned",
                format!("Ölçü grubu kaynaklarıyla taşınamadı: {why}"),
                Some("uids".into()),
            ))
        })
}
