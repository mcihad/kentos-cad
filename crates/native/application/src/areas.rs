//! `cad.areas.edit` v1 (docs/adr/0622 §9): the area operations computed from the objects and what the user gives (Alan
//! birleştir, kesiştir, çıkar, böl; Alana and Çizgiye çevir; Parçaları birleştir and Parçalara ayır for areas, lines
//! and points; Delik ekle, Deliği sil, Deliği doldur) with the shared core's exact area algebra (arcs stay arcs, input
//! corners keep their coordinates), written as `cad.entities.edit` writes them, one undo step named after the tool.
//! The shared cases are `fixtures/commands/v1/cad.areas.edit.json` (`scripts/fixtures/areas_edit_cases.py`).

use kentos_commands::{
    AreaPart, AreaWork, AreasEdit, AreasEdited, CommandResult, EditOperation, EntitiesEdit,
    EntitiesEditPlan, Entity, EntityEdit, EntityGeometry, PathEntity, PointPart, Vec2 as Wire,
};
use kentos_domain::{Document, Slot, Uuid};
use kentos_geometry_core::entity::{Entity as CoreEntity, Shape, area_parts, entity_length};
use kentos_geometry_core::geom::arrangement::{Area, Ring, Source};
use kentos_geometry_core::geom::intersect::Edge;
use kentos_geometry_core::geom::region::{
    FaceIndex, intersect_area_sets, net_area, split_area, subtract_areas, union_areas,
};
use kentos_geometry_core::geometry::dist;
use kentos_geometry_core::ops::areas::{
    area_of_entity, areas_of_entity, line_source, polygon_of_area, polylines_of_polygon,
};
use kentos_geometry_core::ops::holes::{HoleRefusal, hole_add, hole_remove, hole_ring};
use kentos_geometry_core::ops::parts::one_area;
use kentos_geometry_core::vec2::Vec2;

use crate::checks::{self, Stop, error, is_uid_text};
use crate::geometry::{edit_geometry, shape};
use crate::{ExecutionContext, codes, edit};

/// The kinds that enclose an area, as the messages name them.
const AREA_KINDS: &str = "kapalı alan, daire, elips ya da kapalı eğri";

fn refuse(code: &str, message: impl Into<String>, path: &str) -> Stop {
    Stop::failed(error(code, message.into(), Some(path.into())))
}

fn v(p: Wire) -> Vec2 {
    Vec2::new(p.x, p.y)
}

/// An object named by its persistent id: its slot and itself.
fn object<'d>(doc: &'d Document, uid: &str, path: &str) -> Result<(Slot, &'d Entity), Stop> {
    if !is_uid_text(uid) {
        return Err(refuse(
            codes::INVALID_UID,
            format!(
                "“{uid}” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir (01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi)."
            ),
            path,
        ));
    }
    Uuid::parse_str(uid)
        .ok()
        .and_then(|u| doc.slot_of(u))
        .and_then(|s| Some((s, doc.get(s)?)))
        .ok_or_else(|| {
            refuse(
                codes::ENTITY_NOT_FOUND,
                format!(
                    "“{uid}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Var olan bir nesnenin kimliğini verin."
                ),
                path,
            )
        })
}

/// The objects `ids` names, each once, in order.
fn objects<'d>(
    doc: &'d Document,
    ids: &[String],
    path: &str,
) -> Result<Vec<(String, Slot, &'d Entity)>, Stop> {
    let mut out: Vec<(String, Slot, &Entity)> = Vec::new();
    for (i, uid) in ids.iter().enumerate() {
        let (slot, e) = object(doc, uid, &format!("{path}[{i}]"))?;
        if !out.iter().any(|(_, s, _)| *s == slot) {
            out.push((uid.clone(), slot, e));
        }
    }
    Ok(out)
}

/// An object that encloses one area or more: a multi-part area's parts, or the one area of anything else.
struct Whole<'d> {
    uid: String,
    e: &'d Entity,
    parts: Vec<Area>,
}

fn wholes<'d>(list: Vec<(String, Slot, &'d Entity)>) -> Vec<Whole<'d>> {
    list.into_iter()
        .filter_map(|(uid, _, e)| {
            let parts = areas_of_entity(&shape(e));
            (!parts.is_empty()).then_some(Whole { uid, e, parts })
        })
        .collect()
}

fn total_area(list: &[Area]) -> f64 {
    list.iter().map(net_area).sum()
}

/// New areas made from the object `from` names: its layer and colour, and with `keep_data` its attributes and
/// label; never its symbol.
fn add_areas(areas: &[Area], from: &str, keep_data: bool) -> Vec<EntityEdit> {
    areas
        .iter()
        .filter_map(|a| edit_geometry(polygon_of_area(a).shape))
        .map(|geometry| EntityEdit::add(from.to_owned(), geometry, Some(keep_data)))
        .collect()
}

/// As [`add_areas`]; with `one`, one multi-part area of them all (Tek nesne).
fn add_result(areas: &[Area], from: &str, keep_data: bool, one: bool) -> Vec<EntityEdit> {
    if !one || areas.len() < 2 {
        return add_areas(areas, from, keep_data);
    }
    one_area(areas)
        .and_then(|e| edit_geometry(e.shape))
        .map(|geometry| EntityEdit::add(from.to_owned(), geometry, Some(keep_data)))
        .into_iter()
        .collect()
}

fn removal(uid: &str) -> EntityEdit {
    EntityEdit::Remove {
        uid: uid.to_owned(),
    }
}

/// The object becomes the area `a` in its place, keeping its data.
fn replacement(uid: &str, a: &Area) -> Option<EntityEdit> {
    Some(EntityEdit::Replace {
        uid: uid.to_owned(),
        geometry: edit_geometry(polygon_of_area(a).shape)?,
        keep_data: Some(true),
    })
}

/// An object's parts as the contract has them, each with its size: a polygon's own fields and parts (elevations
/// kept), else the one area it encloses as a polygon.
fn contract_parts(x: &Whole<'_>) -> Vec<(f64, AreaPart)> {
    let Entity::Polygon(p) = x.e else {
        return x
            .parts
            .iter()
            .filter_map(|a| match edit_geometry(polygon_of_area(a).shape)? {
                EntityGeometry::Polygon {
                    pts, bulges, holes, ..
                } => Some((
                    net_area(a),
                    AreaPart {
                        ms: None,
                        pts,
                        bulges,
                        holes,
                        zs: None,
                    },
                )),
                _ => None,
            })
            .collect();
    };
    let own = AreaPart {
        ms: None,
        pts: p.pts.clone(),
        bulges: p.bulges.clone(),
        holes: p.holes.clone(),
        zs: p.zs.clone(),
    };
    std::iter::once(own)
        .chain(p.parts.iter().flatten().cloned())
        .map(|part| {
            let size = area_of_entity(&shape(&Entity::Polygon(PathEntity {
                ms: None,
                base: p.base.clone(),
                pts: part.pts.clone(),
                bulges: part.bulges.clone(),
                holes: part.holes.clone(),
                zs: None,
                parts: None,
                closed: false,
                widths: None,
            })))
            .map_or(0.0, |a| net_area(&a));
            (size, part)
        })
        .collect()
}

/// Which objects the result is, in order: an object the edit keeps, or the next `n` the edit creates.
#[derive(Clone, Debug)]
enum Res {
    Own(String),
    New(usize),
}

/// What an operation writes and how it is said.
struct Prepared {
    operation: EditOperation,
    changes: Vec<EntityEdit>,
    result: Vec<Res>,
    objects: usize,
    pieces: usize,
    size: f64,
    more: usize,
    sizes: Vec<f64>,
}

impl Prepared {
    fn new(operation: EditOperation, changes: Vec<EntityEdit>) -> Self {
        Prepared {
            operation,
            changes,
            result: vec![Res::New(usize::MAX)],
            objects: 0,
            pieces: 0,
            size: 0.0,
            more: 0,
            sizes: Vec::new(),
        }
    }
}

fn too_few(message: String, path: &str) -> Stop {
    refuse(codes::TOO_FEW_OBJECTS, message, path)
}

fn kinds_of_areas<'d>(
    list: Vec<(String, Slot, &'d Entity)>,
    needs: usize,
    message: impl FnOnce() -> String,
    path: &str,
) -> Result<Vec<Whole<'d>>, Stop> {
    let list = wholes(list);
    if list.len() < needs {
        return Err(too_few(message(), path));
    }
    Ok(list)
}

fn union(doc: &Document, ids: &[String], one: bool) -> Result<Prepared, Stop> {
    let list = kinds_of_areas(
        objects(doc, ids, "work.ids")?,
        2,
        || format!("Birleştirmek için en az iki alan seçin ({AREA_KINDS})."),
        "work.ids",
    )?;
    let all: Vec<Area> = list.iter().flat_map(|x| x.parts.clone()).collect();
    let result = union_areas(&all);
    let mut changes: Vec<EntityEdit> = list.iter().map(|x| removal(&x.uid)).collect();
    changes.extend(add_result(&result, &list[0].uid, true, one));
    let mut p = Prepared::new(EditOperation::AreaUnion, changes);
    p.objects = list.len();
    p.pieces = result.len();
    p.size = total_area(&result);
    Ok(p)
}

fn intersect(doc: &Document, ids: &[String], erase: bool, one: bool) -> Result<Prepared, Stop> {
    let list = kinds_of_areas(
        objects(doc, ids, "work.ids")?,
        2,
        || format!("Kesiştirmek için en az iki alan seçin ({AREA_KINDS})."),
        "work.ids",
    )?;
    let result = intersect_area_sets(&list.iter().map(|x| x.parts.clone()).collect::<Vec<_>>());
    if result.is_empty() {
        return Err(refuse(
            codes::NO_OVERLAP,
            "Seçili alanların ortak bir parçası yok.",
            "work.ids",
        ));
    }
    // Kept sources keep their data; the overlap is a new, blank area. Erased, it takes the first one's data.
    let mut changes: Vec<EntityEdit> = if erase {
        list.iter().map(|x| removal(&x.uid)).collect()
    } else {
        Vec::new()
    };
    changes.extend(add_result(&result, &list[0].uid, erase, one));
    let mut p = Prepared::new(EditOperation::AreaIntersect, changes);
    p.objects = list.len();
    p.pieces = result.len();
    p.size = total_area(&result);
    Ok(p)
}

fn subtract(
    doc: &Document,
    from: &[String],
    cut: &[String],
    erase: bool,
    one: bool,
) -> Result<Prepared, Stop> {
    let from = kinds_of_areas(
        objects(doc, from, "work.from")?,
        1,
        || format!("Önce kesilecek alanları seçin ({AREA_KINDS})."),
        "work.from",
    )?;
    let cutters: Vec<Whole<'_>> = wholes(objects(doc, cut, "work.cut")?)
        .into_iter()
        .filter(|x| !from.iter().any(|f| f.uid == x.uid))
        .collect();
    if cutters.is_empty() {
        return Err(too_few(
            format!("Çıkarılacak en az bir alan seçin ({AREA_KINDS})."),
            "work.cut",
        ));
    }
    let cut: Vec<Area> = cutters.iter().flat_map(|x| x.parts.clone()).collect();
    let mut changes = Vec::new();
    let (mut changed, mut gone) = (0usize, 0usize);
    let mut left = 0.0;
    let mut pieces = 0;
    for t in &from {
        let rest = subtract_areas(&t.parts, &cut);
        // Untouched areas stay as they are (a circle is not turned into a polygon for nothing).
        let size = total_area(&t.parts);
        if rest.len() == t.parts.len() && (total_area(&rest) - size).abs() <= 1e-9 * size.max(1.0) {
            continue;
        }
        changes.push(removal(&t.uid));
        changes.extend(add_result(&rest, &t.uid, true, one));
        changed += 1;
        left += total_area(&rest);
        pieces += if one && rest.len() > 1 { 1 } else { rest.len() };
        if rest.is_empty() {
            gone += 1;
        }
    }
    if changed == 0 {
        return Err(refuse(
            codes::NO_OVERLAP,
            "Çıkarılan alanlar kesilecek alanlarla örtüşmüyor; hiçbir alan değişmedi.",
            "work.cut",
        ));
    }
    // The cutters go too when asked, but not those on a locked layer.
    if erase {
        changes.extend(
            cutters
                .iter()
                .filter(|x| !doc.layers().is_locked(&x.e.base().layer_id))
                .map(|x| removal(&x.uid)),
        );
    }
    let mut p = Prepared::new(EditOperation::AreaSubtract, changes);
    p.objects = changed;
    p.pieces = pieces;
    p.size = left;
    p.more = gone;
    Ok(p)
}

/// A straight cut line through the points.
fn path_source(pts: &[Vec2]) -> Source {
    Source {
        edges: pts
            .windows(2)
            .map(|w| Edge::Seg { a: w[0], b: w[1] })
            .collect(),
        points: Some(pts.to_vec()),
        cut: Some(true),
    }
}

fn split(
    doc: &Document,
    ids: &[String],
    line: Option<&[Wire]>,
    cutter: Option<&str>,
) -> Result<Prepared, Stop> {
    let list = objects(doc, ids, "work.ids")?;
    let (cut, cutter_uid) = match (line, cutter) {
        (Some(pts), None) => {
            for (i, p) in pts.iter().enumerate() {
                checks::point(
                    *p,
                    "Kesme çizgisinin noktasının",
                    &format!("work.line[{i}]"),
                )?;
            }
            let mut clean: Vec<Vec2> = Vec::new();
            for p in pts.iter().copied().map(v) {
                if clean.last().is_none_or(|&q| dist(q, p) > 1e-9) {
                    clean.push(p);
                }
            }
            if clean.len() < 2 {
                return Err(refuse(
                    codes::DEGENERATE_SHAPE,
                    "Kesme çizgisi için en az iki ayrı nokta gösterin.",
                    "work.line",
                ));
            }
            (path_source(&clean), None)
        }
        (None, Some(uid)) => {
            let (_, e) = object(doc, uid, "work.cutter")?;
            (line_source(&[CoreEntity::new(shape(e))]), Some(uid))
        }
        _ => {
            return Err(refuse(
                codes::INVALID_INPUT,
                "Kesme çizgisini noktalarıyla (line) ya da kesici nesneyle (cutter) verin; ikisi birden değil.",
                "work.line",
            ));
        }
    };
    let areas: Vec<(String, Area)> = list
        .into_iter()
        .filter(|(uid, _, _)| Some(uid.as_str()) != cutter_uid)
        .filter_map(|(uid, _, e)| Some((uid, area_of_entity(&shape(e))?)))
        .collect();
    if areas.is_empty() {
        return Err(refuse(
            codes::NOT_AN_AREA,
            format!("Bölünecek bir alan seçin ({AREA_KINDS})."),
            "work.ids",
        ));
    }
    let mut changes = Vec::new();
    let mut result = Vec::new();
    let mut sizes = Vec::new();
    let mut changed = 0;
    for (uid, a) in &areas {
        let pieces = split_area(a, &cut);
        if pieces.len() < 2 {
            continue;
        }
        // The first piece is the area itself: it keeps its slot and persistent id; the rest are new.
        let Some(first) = replacement(uid, &pieces[0]) else {
            continue;
        };
        changes.push(first);
        changes.extend(add_areas(&pieces[1..], uid, true));
        result.push(Res::Own(uid.clone()));
        result.push(Res::New(pieces.len() - 1));
        sizes.extend(pieces.iter().map(net_area));
        changed += 1;
    }
    if changed == 0 {
        return Err(refuse(
            codes::NO_OVERLAP,
            "Kesme çizgisi alanı baştan başa geçmiyor; çizgi alanın sınırını iki yerden kesmeli.",
            "work.line",
        ));
    }
    let mut p = Prepared::new(EditOperation::AreaSplit, changes);
    p.result = result;
    p.objects = changed;
    p.pieces = sizes.len();
    p.size = sizes.iter().sum();
    p.sizes = sizes;
    Ok(p)
}

fn to_area(doc: &Document, ids: &[String]) -> Result<Prepared, Stop> {
    let list = objects(doc, ids, "work.ids")?;
    let closed: Vec<(String, Area)> = list
        .iter()
        .filter(|(_, _, e)| !matches!(e, Entity::Polygon(_)))
        .filter_map(|(uid, _, e)| Some((uid.clone(), area_of_entity(&shape(e))?)))
        .collect();
    let lines: Vec<(String, &Entity)> = list
        .iter()
        .filter(|(_, _, e)| {
            area_of_entity(&shape(e)).is_none()
                && matches!(
                    e,
                    Entity::Line(_)
                        | Entity::Arc(_)
                        | Entity::Polyline(_)
                        | Entity::Spline(_)
                        | Entity::Nurbs(_)
                        | Entity::Ellipse(_)
                )
        })
        .map(|(uid, _, e)| (uid.clone(), *e))
        .collect();
    let faces = if lines.is_empty() {
        Vec::new()
    } else {
        let work: Vec<CoreEntity> = lines
            .iter()
            .map(|(_, e)| CoreEntity::new(shape(e)))
            .collect();
        FaceIndex::new(&[line_source(&work)]).all()
    };
    if closed.is_empty() && faces.is_empty() {
        let already = list.iter().any(|(_, _, e)| matches!(e, Entity::Polygon(_)));
        return Err(refuse(
            codes::NOT_AN_AREA,
            if already {
                "Seçili nesneler zaten alan."
            } else {
                "Alana çevrilecek kapalı nesne ya da kapalı bölge oluşturan çizgi bulunamadı. Çizgilerin uçları birleşmeli ya da kesişmeli."
            },
            "work.ids",
        ));
    }
    // The object itself becomes an area: it keeps its slot and persistent id. Line work stays; the regions it
    // closes become new, blank areas on its layer.
    let mut changes: Vec<EntityEdit> = closed
        .iter()
        .filter_map(|(uid, a)| replacement(uid, a))
        .collect();
    if let Some((first, _)) = lines.first()
        && !faces.is_empty()
    {
        changes.extend(add_areas(&faces, first, false));
    }
    let mut p = Prepared::new(EditOperation::ToArea, changes);
    p.result = closed
        .iter()
        .map(|(uid, _)| Res::Own(uid.clone()))
        .chain(std::iter::once(Res::New(usize::MAX)))
        .collect();
    p.objects = closed.len();
    p.more = faces.len();
    p.pieces = closed.len() + faces.len();
    p.size = total_area(&faces);
    p.sizes = faces.iter().map(net_area).collect();
    Ok(p)
}

fn to_polyline(doc: &Document, ids: &[String]) -> Result<Prepared, Stop> {
    let polys: Vec<(String, &Entity)> = objects(doc, ids, "work.ids")?
        .into_iter()
        .filter(|(_, _, e)| matches!(e, Entity::Polygon(_)))
        .map(|(uid, _, e)| (uid, e))
        .collect();
    if polys.is_empty() {
        return Err(refuse(
            codes::NOT_AN_AREA,
            "Çizgiye çevrilecek bir kapalı alan seçin.",
            "work.ids",
        ));
    }
    // The outer ring is the area itself, now a polyline (its slot, persistent id and data kept); holes become new,
    // blank polylines from it.
    let mut changes = Vec::new();
    let mut result = Vec::new();
    let mut pieces = 0;
    for (uid, e) in &polys {
        let Ok(rings) = polylines_of_polygon(&shape(e)) else {
            continue;
        };
        let mut more = 0;
        for (i, ring) in rings.into_iter().enumerate() {
            let Some(geometry) = edit_geometry(ring.shape) else {
                continue;
            };
            if i == 0 {
                changes.push(EntityEdit::Replace {
                    uid: uid.clone(),
                    geometry,
                    keep_data: Some(true),
                });
            } else {
                changes.push(EntityEdit::add(uid.clone(), geometry, Some(false)));
                more += 1;
            }
            pieces += 1;
        }
        result.push(Res::Own(uid.clone()));
        result.push(Res::New(more));
    }
    let mut p = Prepared::new(EditOperation::ToPolyline, changes);
    p.result = result;
    p.objects = polys.len();
    p.pieces = pieces;
    Ok(p)
}

/// What Parçaları birleştir joins: the areas, the lines (a line, an open polyline, a multi-part polyline) and the
/// points, in order.
#[derive(Default)]
struct Kinds<'d> {
    areas: Vec<(String, Slot, &'d Entity)>,
    lines: Vec<(String, &'d Entity)>,
    points: Vec<(String, &'d Entity)>,
}

fn kinds<'d>(list: Vec<(String, Slot, &'d Entity)>) -> Kinds<'d> {
    let mut k = Kinds::default();
    for (uid, slot, e) in list {
        match e {
            Entity::Point(_) => k.points.push((uid, e)),
            Entity::Line(_) => k.lines.push((uid, e)),
            Entity::Polyline(p) if p.parts.is_some() || areas_of_entity(&shape(e)).is_empty() => {
                k.lines.push((uid, e))
            }
            _ if !areas_of_entity(&shape(e)).is_empty() => k.areas.push((uid, slot, e)),
            _ => {}
        }
    }
    k
}

fn parts_join(doc: &Document, ids: &[String]) -> Result<Prepared, Stop> {
    let k = kinds(objects(doc, ids, "work.ids")?);
    let present = usize::from(!k.areas.is_empty())
        + usize::from(!k.lines.is_empty())
        + usize::from(!k.points.is_empty());
    if present > 1 {
        return Err(refuse(
            codes::MIXED_KINDS,
            "Parçaları birleştir aynı türden nesneleri birleştirir: alanları, çizgileri ya da noktaları.",
            "work.ids",
        ));
    }
    match (k.lines.len(), k.points.len()) {
        (1, _) => {
            return Err(too_few(
                "Parçaları birleştirmek için en az iki çizgi ya da çoklu çizgi seçin.".into(),
                "work.ids",
            ));
        }
        (_, 1) => {
            return Err(too_few(
                "Parçaları birleştirmek için en az iki nokta seçin.".into(),
                "work.ids",
            ));
        }
        (n, _) if n > 1 => return Ok(join_lines(&k.lines)),
        (_, n) if n > 1 => return Ok(join_points(&k.points)),
        _ => {}
    }
    let list = wholes(k.areas);
    if list.len() < 2 {
        return Err(too_few(
            format!("Parçaları birleştirmek için en az iki alan seçin ({AREA_KINDS})."),
            "work.ids",
        ));
    }
    let all: Vec<Area> = list.iter().flat_map(|x| x.parts.clone()).collect();
    let merged = union_areas(&all);
    let size = total_area(&all);
    let apart =
        merged.len() == all.len() && (total_area(&merged) - size).abs() <= 1e-9 * size.max(1.0);
    let geometry = if apart {
        // Each part as the contract has it, sized by its area, the largest first.
        let mut parts: Vec<(f64, AreaPart)> = list.iter().flat_map(contract_parts).collect();
        parts.sort_by(|a, b| b.0.total_cmp(&a.0));
        let mut parts = parts.into_iter().map(|(_, p)| p);
        parts.next().map(|first| EntityGeometry::Polygon {
            pts: first.pts,
            bulges: first.bulges,
            holes: first.holes,
            zs: first.zs,
            parts: Some(parts.collect()),
        })
    } else {
        one_area(&merged).and_then(|e| edit_geometry(e.shape))
    };
    let Some(geometry) = geometry else {
        return Err(refuse(
            codes::DEGENERATE_SHAPE,
            "Alanların birleşimi yazılabilir bir alan vermedi.",
            "work.ids",
        ));
    };
    // The first picked lends its place, id, layer and data; a circle becomes the area.
    let first = &list[0];
    let uid = first.uid.clone();
    let mut changes = vec![match first.e {
        Entity::Polygon(_) => EntityEdit::Update {
            uid: uid.clone(),
            geometry,
        },
        _ => EntityEdit::Replace {
            uid: uid.clone(),
            geometry,
            keep_data: Some(true),
        },
    }];
    changes.extend(list[1..].iter().map(|x| removal(&x.uid)));
    let mut p = Prepared::new(EditOperation::PartsJoin, changes);
    p.result = vec![Res::Own(uid)];
    p.objects = list.len();
    if apart {
        p.pieces = all.len();
        p.size = size;
    } else {
        p.pieces = merged.len();
        p.size = total_area(&merged);
        p.more = 1;
    }
    Ok(p)
}

/// The lines as one multi-part polyline in the first one's place: their parts in order, each object's own first; a
/// line becomes a polyline (`replace`, its data kept); the others go.
fn join_lines(lines: &[(String, &Entity)]) -> Prepared {
    let mut parts: Vec<AreaPart> = Vec::new();
    let mut length = 0.0;
    for (_, e) in lines {
        length += entity_length(&shape(e)).unwrap_or(0.0);
        match e {
            Entity::Line(l) => parts.push(AreaPart {
                ms: None,
                pts: vec![l.a, l.b],
                bulges: None,
                holes: None,
                zs: (l.za.is_some() || l.zb.is_some()).then(|| vec![l.za, l.zb]),
            }),
            Entity::Polyline(p) => {
                parts.push(AreaPart {
                    ms: None,
                    pts: p.pts.clone(),
                    bulges: p.bulges.clone(),
                    holes: None,
                    zs: p.zs.clone(),
                });
                parts.extend(p.parts.iter().flatten().map(|q| AreaPart {
                    holes: None,
                    ..q.clone()
                }));
            }
            _ => {}
        }
    }
    let pieces = parts.len();
    let mut parts = parts.into_iter();
    let first = parts.next().unwrap_or(AreaPart {
        ms: None,
        pts: Vec::new(),
        bulges: None,
        holes: None,
        zs: None,
    });
    let geometry = EntityGeometry::Polyline {
        ms: None,
        pts: first.pts,
        bulges: first.bulges,
        zs: first.zs,
        parts: Some(parts.collect()),
        closed: false,
        widths: None,
    };
    let (uid, e) = &lines[0];
    let mut changes = vec![match e {
        Entity::Polyline(_) => EntityEdit::Update {
            uid: uid.clone(),
            geometry,
        },
        _ => EntityEdit::Replace {
            uid: uid.clone(),
            geometry,
            keep_data: Some(true),
        },
    }];
    changes.extend(lines[1..].iter().map(|(u, _)| removal(u)));
    let mut p = Prepared::new(EditOperation::PartsJoin, changes);
    p.result = vec![Res::Own(uid.clone())];
    p.objects = lines.len();
    p.pieces = pieces;
    p.size = length;
    p
}

/// The points as one multi-point object in the first one's place, each with its elevation, in order.
fn join_points(list: &[(String, &Entity)]) -> Prepared {
    let mut points: Vec<PointPart> = Vec::new();
    for (_, e) in list {
        if let Entity::Point(p) = e {
            points.push(PointPart { p: p.p, z: p.z });
            points.extend(p.parts.iter().flatten().cloned());
        }
    }
    let n = points.len();
    let mut rest = points.into_iter();
    let first = rest.next();
    let geometry = EntityGeometry::Point {
        p: first.as_ref().map_or(Wire { x: 0.0, y: 0.0 }, |f| f.p),
        z: first.as_ref().and_then(|f| f.z),
        parts: Some(rest.collect()),
    };
    let uid = list[0].0.clone();
    let mut changes = vec![EntityEdit::Update {
        uid: uid.clone(),
        geometry,
    }];
    changes.extend(list[1..].iter().map(|(u, _)| removal(u)));
    let mut p = Prepared::new(EditOperation::PartsJoin, changes);
    p.result = vec![Res::Own(uid)];
    p.objects = list.len();
    p.pieces = n;
    p
}

/// A multi-part polyline's, point's or area's changes for Parçalara ayır: its own first part in its place, each
/// other one new with its data; with how many are new. None for anything else.
fn split_parts(e: &Entity, uid: &str) -> Option<(Vec<EntityEdit>, usize)> {
    let add = |geometry| EntityEdit::add(uid.to_owned(), geometry, Some(true));
    let update = |geometry| EntityEdit::Update {
        uid: uid.to_owned(),
        geometry,
    };
    match e {
        Entity::Polygon(p) if p.parts.as_ref().is_some_and(|q| !q.is_empty()) => {
            let parts = p.parts.as_deref().unwrap_or(&[]);
            let mut changes = vec![update(EntityGeometry::Polygon {
                pts: p.pts.clone(),
                bulges: p.bulges.clone(),
                holes: p.holes.clone(),
                zs: p.zs.clone(),
                parts: None,
            })];
            changes.extend(parts.iter().map(|q| {
                add(EntityGeometry::Polygon {
                    pts: q.pts.clone(),
                    bulges: q.bulges.clone(),
                    holes: q.holes.clone(),
                    zs: q.zs.clone(),
                    parts: None,
                })
            }));
            Some((changes, parts.len()))
        }
        Entity::Polyline(p) if p.parts.as_ref().is_some_and(|q| !q.is_empty()) => {
            let parts = p.parts.as_deref().unwrap_or(&[]);
            let polyline =
                |pts: &Vec<Wire>, bulges: &Option<Vec<f64>>, zs| EntityGeometry::Polyline {
                    ms: None,
                    pts: pts.clone(),
                    bulges: bulges.clone(),
                    zs,
                    parts: None,
                    closed: false,
                    widths: None,
                };
            let mut changes = vec![update(polyline(&p.pts, &p.bulges, p.zs.clone()))];
            changes.extend(
                parts
                    .iter()
                    .map(|q| add(polyline(&q.pts, &q.bulges, q.zs.clone()))),
            );
            Some((changes, parts.len()))
        }
        Entity::Point(p) if p.parts.as_ref().is_some_and(|q| !q.is_empty()) => {
            let parts = p.parts.as_deref().unwrap_or(&[]);
            let mut changes = vec![update(EntityGeometry::Point {
                p: p.p,
                z: p.z,
                parts: None,
            })];
            changes.extend(parts.iter().map(|q| {
                add(EntityGeometry::Point {
                    p: q.p,
                    z: q.z,
                    parts: None,
                })
            }));
            Some((changes, parts.len()))
        }
        _ => None,
    }
}

fn parts_split(doc: &Document, ids: &[String]) -> Result<Prepared, Stop> {
    let list = objects(doc, ids, "work.ids")?;
    // The areas first, then the lines and the points, each in order.
    let (areas, others): (Vec<_>, Vec<_>) = list
        .into_iter()
        .partition(|(_, _, e)| matches!(e, Entity::Polygon(_)));
    let mut changes = Vec::new();
    let mut result = Vec::new();
    let (mut n, mut pieces) = (0, 0);
    let mut only_areas = true;
    for (i, (uid, _, e)) in areas.iter().chain(&others).enumerate() {
        let Some((more, count)) = split_parts(e, uid) else {
            continue;
        };
        if i >= areas.len() {
            only_areas = false;
        }
        changes.extend(more);
        result.push(Res::Own(uid.clone()));
        result.push(Res::New(count));
        n += 1;
        pieces += count + 1;
    }
    if n == 0 {
        return Err(refuse(
            codes::NOT_MULTI_PART,
            "Parçalarına ayrılacak çok parçalı bir nesne seçin: alan, çoklu çizgi ya da çok noktalı nesne.",
            "work.ids",
        ));
    }
    let mut p = Prepared::new(EditOperation::PartsSplit, changes);
    p.result = result;
    p.objects = n;
    p.pieces = pieces;
    p.more = usize::from(only_areas);
    Ok(p)
}

/// What a hole's refusal says, in the tools' words.
fn hole_refusal(why: &HoleRefusal, path: &str) -> Stop {
    match why {
        HoleRefusal::NotArea => refuse(
            codes::NOT_AN_AREA,
            "Delik yalnız kapalı alana eklenir.",
            path,
        ),
        HoleRefusal::Degenerate => refuse(
            codes::DEGENERATE_SHAPE,
            "Delik en az üç köşeli, alanı olan bir halka olmalı.",
            "work.ring",
        ),
        HoleRefusal::Outside => refuse(
            codes::INVALID_HOLE,
            "Delik alanın içinde kalmalı; sınırı aşan bölümü çıkarmak için Alan çıkar'ı kullanın.",
            "work.ring",
        ),
        HoleRefusal::Splits => refuse(
            codes::INVALID_HOLE,
            "Delik alanı parçalara ayırırdı; alanı bölmek için Alan böl'ü kullanın.",
            "work.ring",
        ),
        HoleRefusal::NotInHole => refuse(codes::NO_HOLE, "Bir deliğin içine tıklayın.", "work.at"),
    }
}

/// An area's net size.
fn net_of(s: &Shape) -> f64 {
    total_area(&areas_of_entity(s))
}

/// How many holes an area has, in all its parts.
fn holes_of(s: &Shape) -> usize {
    area_parts(s)
        .iter()
        .map(|part| match part {
            Shape::Polygon { holes: Some(h), .. } => h.len(),
            _ => 0,
        })
        .sum()
}

fn hole_add_of(
    doc: &Document,
    id: &str,
    ring: &[Wire],
    bulges: Option<&[f64]>,
) -> Result<Prepared, Stop> {
    let (_, e) = object(doc, id, "work.id")?;
    for (i, p) in ring.iter().enumerate() {
        checks::point(*p, "Deliğin köşesinin", &format!("work.ring[{i}]"))?;
    }
    if let Some(b) = bulges {
        for (i, x) in b.iter().enumerate() {
            checks::finite(
                *x,
                "Büküm",
                "Sonlu bir sayı verin.",
                &format!("work.bulges[{i}]"),
            )?;
        }
    }
    let before = shape(e);
    let hole = Ring {
        pts: ring.iter().copied().map(v).collect(),
        bulges: bulges.map(<[f64]>::to_vec),
    };
    let after = hole_add(&before, &hole).map_err(|why| hole_refusal(&why, "work.id"))?;
    let merged = holes_of(&after) <= holes_of(&before);
    let geometry = edit_geometry(after.clone()).ok_or_else(|| {
        refuse(
            codes::DEGENERATE_SHAPE,
            "Delikli alan yazılamadı.",
            "work.ring",
        )
    })?;
    let mut p = Prepared::new(
        EditOperation::HoleAdd,
        vec![EntityEdit::Update {
            uid: id.to_owned(),
            geometry,
        }],
    );
    p.result = vec![Res::Own(id.to_owned())];
    p.objects = 1;
    p.pieces = 1;
    p.size = net_of(&after);
    p.more = usize::from(merged);
    Ok(p)
}

fn hole_remove_of(doc: &Document, id: &str, at: Wire) -> Result<Prepared, Stop> {
    let (_, e) = object(doc, id, "work.id")?;
    checks::point(at, "Noktanın", "work.at")?;
    let after = hole_remove(&shape(e), v(at)).map_err(|why| hole_refusal(&why, "work.id"))?;
    let geometry = edit_geometry(after.clone())
        .ok_or_else(|| refuse(codes::DEGENERATE_SHAPE, "Alan yazılamadı.", "work.id"))?;
    let mut p = Prepared::new(
        EditOperation::HoleRemove,
        vec![EntityEdit::Update {
            uid: id.to_owned(),
            geometry,
        }],
    );
    p.result = vec![Res::Own(id.to_owned())];
    p.objects = 1;
    p.pieces = 1;
    p.size = net_of(&after);
    Ok(p)
}

fn hole_fill_of(doc: &Document, id: &str, at: Wire) -> Result<Prepared, Stop> {
    let (_, e) = object(doc, id, "work.id")?;
    checks::point(at, "Noktanın", "work.at")?;
    let ring = hole_ring(&shape(e), v(at)).map_err(|why| hole_refusal(&why, "work.id"))?;
    let filled = Shape::Polygon {
        pts: ring.pts.clone(),
        bulges: ring.bulges.clone(),
        holes: None,
        parts: None,
    };
    let size = net_of(&filled);
    let geometry = edit_geometry(filled)
        .ok_or_else(|| refuse(codes::DEGENERATE_SHAPE, "Delik doldurulamadı.", "work.at"))?;
    let mut p = Prepared::new(
        EditOperation::HoleFill,
        vec![EntityEdit::add(id.to_owned(), geometry, None)],
    );
    p.objects = 1;
    p.pieces = 1;
    p.size = size;
    Ok(p)
}

fn prepare(doc: &Document, input: &AreasEdit) -> Result<Prepared, Stop> {
    checks::revision(doc, input.expected_revision.as_deref())?;
    match &input.work {
        AreaWork::Union { ids, single } => union(doc, ids, *single),
        AreaWork::Intersect { ids, erase, single } => intersect(doc, ids, *erase, *single),
        AreaWork::Subtract {
            from,
            cut,
            erase,
            single,
        } => subtract(doc, from, cut, *erase, *single),
        AreaWork::Split { ids, line, cutter } => {
            split(doc, ids, line.as_deref(), cutter.as_deref())
        }
        AreaWork::ToArea { ids } => to_area(doc, ids),
        AreaWork::ToPolyline { ids } => to_polyline(doc, ids),
        AreaWork::PartsJoin { ids } => parts_join(doc, ids),
        AreaWork::PartsSplit { ids } => parts_split(doc, ids),
        AreaWork::HoleAdd { id, ring, bulges } => hole_add_of(doc, id, ring, bulges.as_deref()),
        AreaWork::HoleRemove { id, at } => hole_remove_of(doc, id, *at),
        AreaWork::HoleFill { id, at } => hole_fill_of(doc, id, *at),
    }
}

/// `cad.entities.edit`'s answer in this command's words: a refusal of one of its changes (`changes[2].uid`) names
/// the work (`work`), since the changes are the command's own.
fn ours<T>(result: CommandResult<T>) -> CommandResult<T> {
    fn path(error: &mut kentos_commands::CommandError) {
        if error
            .path
            .as_deref()
            .is_some_and(|p| p.starts_with("changes["))
        {
            error.path = Some("work".to_owned());
        }
    }
    match result {
        CommandResult::Failed { mut error } => {
            path(&mut error);
            CommandResult::Failed { error }
        }
        CommandResult::Conflict { mut error } => {
            path(&mut error);
            CommandResult::Conflict { error }
        }
        CommandResult::NeedsInput { mut error } => {
            path(&mut error);
            CommandResult::NeedsInput { error }
        }
        other => other,
    }
}

fn edit_input(p: &Prepared, input: &AreasEdit) -> EntitiesEdit {
    EntitiesEdit {
        operation: p.operation,
        changes: p.changes.clone(),
        expected_revision: input.expected_revision.clone(),
    }
}

/// The operation's checks and `cad.entities.edit`'s.
pub fn validate(cx: &ExecutionContext<'_>, input: &AreasEdit) -> CommandResult<()> {
    match prepare(cx.doc, input) {
        Ok(p) => ours(edit::validate(cx, &edit_input(&p, input))),
        Err(stop) => stop.into(),
    }
}

/// The objects as they would be written.
pub fn plan(cx: &ExecutionContext<'_>, input: &AreasEdit) -> CommandResult<EntitiesEditPlan> {
    match prepare(cx.doc, input) {
        Ok(p) => ours(edit::plan(cx, &edit_input(&p, input))),
        Err(stop) => stop.into(),
    }
}

/// The result as a tool's preview draws it: what the command would write, or its refusal.
pub fn plan_in(doc: &Document, input: &AreasEdit) -> CommandResult<EntitiesEditPlan> {
    match prepare(doc, input) {
        Ok(p) => ours(edit::plan_in(doc, &edit_input(&p, input))),
        Err(stop) => stop.into(),
    }
}

/// Edits the areas, one undo step named after the tool.
pub fn execute(cx: &mut ExecutionContext<'_>, input: AreasEdit) -> CommandResult<AreasEdited> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_AREAS_EDIT,
        kentos_commands::CAD_AREAS_EDIT_VERSION,
        input,
        write_execute,
    )
}

fn write_execute(cx: &mut ExecutionContext<'_>, input: AreasEdit) -> CommandResult<AreasEdited> {
    let p = match prepare(cx.doc, &input) {
        Ok(p) => p,
        Err(stop) => return stop.into(),
    };
    match ours(edit::write_in(cx, edit_input(&p, &input))) {
        CommandResult::Completed { output, warnings } => {
            let mut created = output.created.iter();
            let mut result = Vec::new();
            for r in &p.result {
                match r {
                    Res::Own(uid) => result.push(uid.clone()),
                    Res::New(n) => result.extend(created.by_ref().take(*n).cloned()),
                }
            }
            CommandResult::Completed {
                output: AreasEdited {
                    changed: output.changed,
                    created: output.created.clone(),
                    removed: output.removed,
                    result,
                    objects: count(p.objects),
                    pieces: count(p.pieces),
                    size: p.size,
                    more: count(p.more),
                    sizes: p.sizes,
                    revision: output.revision,
                },
                warnings,
            }
        }
        other => crate::trim::relay(other),
    }
}

fn count(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}
