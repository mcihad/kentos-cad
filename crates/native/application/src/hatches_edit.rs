//! `cad.hatches.edit` v1 (docs/adr/0329 §4): a hatch's boundary written as objects again (HATCHEDIT's Recreate
//! boundary), its region found again around a point (Reselect), islands added and taken away, its tie broken; each as
//! one undo step named after the edit. A tied hatch is cut again from its objects by the document's own rule
//! (`Document::recut_hatch`); another has its region cut by the areas given. The web is paused (docs/adr/0228): the
//! desktop's handler alone, passing `fixtures/commands/v1/cad.hatches.edit.json`.

use std::collections::HashSet;

use kentos_contracts::{
    BoundaryForm, CommandResult, CommandWarning, EllipseEntity, Entity, EntityBase, EntityId,
    HatchAssoc, HatchEdit, HatchEntity, HatchRegion, HatchesEdit, HatchesEdited, HatchesPlan,
    NurbsEntity, PathEntity, RingGeometry, Vec2 as Point,
};
use kentos_domain::{Document, Slot, Uuid};
use kentos_geometry_core::entity::{HatchLoop, Shape, polygon_ring};
use kentos_geometry_core::geom::arrangement::Area;
use kentos_geometry_core::geom::hatch_boundary;
use kentos_geometry_core::geom::region::{net_area, subtract_areas};
use kentos_geometry_core::jsmath::TAU;
use kentos_geometry_core::ops::areas::areas_of_entity;
use kentos_geometry_core::ops::hatch_region::seed_of;
use kentos_geometry_core::store::Store;
use kentos_geometry_core::vec2::Vec2;

use crate::ExecutionContext;
use crate::checks::{self, Stop, error, is_uid_text};
pub use crate::codes;
use crate::geometry::shape;
use crate::geometry_store::store_of;
use crate::region::{RegionCache, hatch_ties_of, point_region_checks, regions};

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.to_owned())))
}

/// The undo step's name: the edit's.
fn label(e: &HatchEdit) -> &'static str {
    match e {
        HatchEdit::Recreate { .. } => "Sınırı yeniden oluştur",
        HatchEdit::Reselect { .. } => "Sınırı yeniden seç",
        HatchEdit::AddIslands { .. } => "Ada ekle",
        HatchEdit::RemoveIslands { .. } => "Ada çıkar",
        HatchEdit::Untie { .. } => "İlişkiyi kopar",
    }
}

fn bad_uid(uid: &str, path: &str) -> Stop {
    fail(
        codes::INVALID_UID,
        format!(
            "“{uid}” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir (01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi)."
        ),
        path,
    )
}

/// The object `uid` names: its slot and id.
fn object_at(doc: &Document, uid: &str, path: &str) -> Result<(Slot, Uuid), Stop> {
    if !is_uid_text(uid) {
        return Err(bad_uid(uid, path));
    }
    Uuid::parse_str(uid)
        .ok()
        .and_then(|u| doc.slot_of(u).map(|s| (s, u)))
        .ok_or_else(|| {
            fail(
                codes::ENTITY_NOT_FOUND,
                format!(
                    "“{uid}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Var olan bir nesnenin kimliğini verin."
                ),
                path,
            )
        })
}

/// The hatch `uid` names, on an unlocked layer.
fn hatch_at(doc: &Document, uid: &str, path: &str) -> Result<(Slot, Uuid, HatchEntity), Stop> {
    let (slot, u) = object_at(doc, uid, path)?;
    let Some(Entity::Hatch(h)) = doc.get(slot) else {
        return Err(fail(
            codes::NOT_A_HATCH,
            "Verilen nesne bir tarama değil; bu komut yalnız taramaları değiştirir.".into(),
            path,
        ));
    };
    if doc.layers().is_locked(&h.base.layer_id) {
        let layers = doc.layers();
        let name = layers
            .get(&h.base.layer_id)
            .map_or(h.base.layer_id.as_str(), |n| n.name.as_str());
        return Err(fail(
            codes::LAYER_LOCKED,
            format!(
                "“{name}” katmanı kilitli; üzerindeki tarama değişmez. Kilidi Katmanlar panelinden açın."
            ),
            path,
        ));
    }
    Ok((slot, u, h.clone()))
}

/// The closed objects `uids` names, each once: their ids and shapes.
fn closed_at(doc: &Document, uids: &[String], path: &str) -> Result<Vec<(EntityId, Shape)>, Stop> {
    if uids.is_empty() {
        return Err(fail(
            codes::NO_ENTITIES,
            "Ada yok. En az bir kapalı nesnenin kalıcı kimliğini verin.".into(),
            path,
        ));
    }
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for (i, uid) in uids.iter().enumerate() {
        let at = format!("{path}[{i}]");
        let (slot, u) = object_at(doc, uid, &at)?;
        if !seen.insert(u) {
            continue;
        }
        let Some(e) = doc.get(slot) else { continue };
        let s = shape(e);
        if areas_of_entity(&s).is_empty() {
            return Err(fail(
                codes::NOT_CLOSED,
                "Nesne kapalı değil: ada alan, daire, tam elips ya da kapalı eğri olur.".into(),
                &at,
            ));
        }
        out.push((EntityId(*u.as_bytes()), s));
    }
    Ok(out)
}

/// The hatch's loops as the core holds them: its exact boundary, else its ring and holes.
fn loops_of(h: &HatchEntity) -> Vec<HatchLoop> {
    if let Some(b) = h.boundary.as_deref().and_then(kentos_formats::blocks::core_boundary) {
        return b;
    }
    let plain = |pts: &[Point]| HatchLoop {
        pts: pts.iter().map(|p| Vec2::new(p.x, p.y)).collect(),
        bulges: None,
        curves: None,
    };
    std::iter::once(plain(&h.ring))
        .chain(h.holes.iter().flatten().map(|r| plain(r)))
        .collect()
}

fn wire(p: Vec2) -> Point {
    Point { x: p.x, y: p.y }
}

/// The hatch with the region `area` (its boundary exact where `sources`' closed curves are found again).
fn with_area(h: &HatchEntity, area: &Area, sources: &[Shape]) -> HatchEntity {
    let ring: Vec<Point> = polygon_ring(&area.outer.pts, area.outer.bulges.as_deref())
        .into_iter()
        .map(wire)
        .collect();
    let holes: Vec<Vec<Point>> = area
        .holes
        .iter()
        .map(|r| {
            polygon_ring(&r.pts, r.bulges.as_deref())
                .into_iter()
                .map(wire)
                .collect()
        })
        .collect();
    let loops = hatch_boundary::of_area(area, sources);
    HatchEntity {
        ring,
        holes: (!holes.is_empty()).then_some(holes),
        boundary: hatch_boundary::curved(&loops)
            .then(|| kentos_formats::blocks::contract_boundary(&loops))
            .flatten(),
        ..h.clone()
    }
}

fn not_enclosed(path: &str) -> Stop {
    fail(
        codes::NOT_ENCLOSED,
        "Taranacak bölge kalmadı: adalar taramanın bütün bölgesini kaplıyor ya da nesneleri bir bölge kapatmıyor."
            .into(),
        path,
    )
}

fn base_on(layer: &str) -> EntityBase {
    EntityBase {
        look: Default::default(),
        id: 0,
        layer_id: layer.to_owned(),
        color: None,
        attrs: Default::default(),
        label: None,
        symbol: None,
        line_weight: None,
        label_pins: Vec::new(),
        attachments: Vec::new(),
    }
}

/// A loop chorded as a polygon's ring holds it: points, bulges (none when all straight), whether a curve was chorded.
fn ring_of(l: &HatchLoop) -> (Vec<Point>, Option<Vec<f64>>, bool) {
    let (pts, bulges, approximated) = hatch_boundary::chorded(l);
    let any = bulges.iter().any(|b| *b != 0.0);
    (pts.into_iter().map(wire).collect(), any.then_some(bulges), approximated)
}

/// Sınırı yeniden oluştur's objects: in `form` on `layer`, outer loop first; and whether a curve was chorded.
fn recreated(loops: &[HatchLoop], form: BoundaryForm, layer: &str) -> (Vec<Entity>, bool) {
    let mut approximated = false;
    match form {
        BoundaryForm::Area => {
            let mut rings = loops.iter().map(ring_of);
            let Some((pts, bulges, a)) = rings.next() else {
                return (Vec::new(), false);
            };
            approximated |= a;
            let holes: Vec<RingGeometry> = rings
                .map(|(pts, bulges, a)| {
                    approximated |= a;
                    RingGeometry {
                        pts,
                        bulges,
                        zs: None,
                    }
                })
                .collect();
            let area = Entity::Polygon(PathEntity {
                base: base_on(layer),
                pts,
                bulges,
                holes: (!holes.is_empty()).then_some(holes),
                zs: None,
                ms: None,
                parts: None,
                closed: false,
                widths: None,
            });
            (vec![area], approximated)
        }
        BoundaryForm::Polyline => {
            let out = loops
                .iter()
                .map(|l| match hatch_boundary::whole_curve(l) {
                    Some(Shape::Ellipse {
                        c, major, ratio, ..
                    }) => Entity::Ellipse(EllipseEntity {
                        base: base_on(layer),
                        c: wire(c),
                        major: wire(major),
                        ratio,
                        t0: 0.0,
                        t1: TAU,
                    }),
                    Some(Shape::Nurbs {
                        degree,
                        ctrl,
                        knots,
                        weights,
                        ..
                    }) => Entity::Nurbs(NurbsEntity {
                        base: base_on(layer),
                        degree: degree as u32,
                        ctrl: ctrl.into_iter().map(wire).collect(),
                        knots,
                        weights,
                        closed: true,
                        periodic: false,
                        fit: None,
                        fit_tolerance: None,
                        tangents: None,
                    }),
                    _ => {
                        let (pts, bulges, a) = ring_of(l);
                        approximated |= a;
                        Entity::Polyline(PathEntity {
                            base: base_on(layer),
                            pts,
                            bulges,
                            holes: None,
                            zs: None,
                            ms: None,
                            parts: None,
                            closed: true,
                            widths: None,
                        })
                    }
                })
                .collect();
            (out, approximated)
        }
    }
}

/// What the checks found: the hatches as they will be written, the objects to create (with the hatch to tie to
/// them, and how), the warnings.
struct Checked {
    changed: Vec<(Slot, Uuid, HatchEntity)>,
    created: Vec<Entity>,
    tie_to_created: Option<(Slot, BoundaryForm, Point)>,
    warnings: Vec<CommandWarning>,
}

fn check(doc: &Document, store: &Store, input: &HatchesEdit) -> Result<Checked, Stop> {
    let mut warnings = Vec::new();
    let mut created = Vec::new();
    let mut tie_to_created = None;
    let mut changed: Vec<(Slot, Uuid, HatchEntity)> = Vec::new();
    match &input.edit {
        HatchEdit::Recreate {
            uid,
            form,
            layer_id,
            associative,
        } => {
            let (slot, u, h) = hatch_at(doc, uid, "edit.uid")?;
            let layer = layer_id.clone().unwrap_or_else(|| h.base.layer_id.clone());
            warnings.extend(checks::layer_at(doc, &layer, "edit.layerId")?);
            checks::revision(doc, input.expected_revision.as_deref())?;
            let form = form.unwrap_or_default();
            let (objects, approximated) = recreated(&loops_of(&h), form, &layer);
            if approximated {
                warnings.push(CommandWarning {
                    code: codes::CURVES_APPROXIMATED.into(),
                    message: "Sınırın elips ya da eğri kenarları 0,1 mm'lik kirişleriyle yazıldı; alan yalnız doğru ve yay tutar.".into(),
                    path: Some("edit.form".into()),
                });
            }
            created = objects;
            if associative.unwrap_or(true) {
                // The point it follows from: its tie's, else one inside its region.
                let seed = h.assoc.as_ref().map(|a| a.seed).or_else(|| {
                    hatch_boundary::area_of(&loops_of(&h))
                        .and_then(|a| seed_of(&a))
                        .map(wire)
                });
                if let Some(seed) = seed {
                    tie_to_created = Some((slot, form, seed));
                    changed.push((slot, u, HatchEntity { assoc: None, ..h }));
                }
            }
        }
        HatchEdit::Reselect { uid, region } => {
            let (slot, u, h) = hatch_at(doc, uid, "edit.uid")?;
            point_region_checks(doc, region, "edit.region")?;
            checks::revision(doc, input.expected_revision.as_deref())?;
            let (found, said) = regions(doc, store, region, &mut RegionCache::default())
                .map_err(|stop| reroot(stop, "region", "edit.region"))?;
            warnings.extend(said.into_iter().map(|mut w| {
                w.path = w.path.map(|p| p.replacen("region", "edit.region", 1));
                w
            }));
            let p = found.first().ok_or_else(|| not_enclosed("edit.region.at"))?;
            warnings.extend(crate::region::gaps_said(doc, p).map(|mut w| {
                w.path = Some("edit.region.gap".into());
                w
            }));
            let associative = match region {
                HatchRegion::Point { associative, .. } => associative.unwrap_or(true),
                HatchRegion::Objects { .. } => false,
            };
            let (assoc, boundary) = hatch_ties_of(doc, store, p, region, associative);
            let next = with_area(&h, &p.area, &[]);
            changed.push((
                slot,
                u,
                HatchEntity {
                    assoc,
                    boundary: boundary.or(next.boundary.clone()),
                    ..next
                },
            ));
        }
        HatchEdit::AddIslands { uid, uids } => {
            let (slot, u, h) = hatch_at(doc, uid, "edit.uid")?;
            let islands = closed_at(doc, uids, "edit.uids")?;
            checks::revision(doc, input.expected_revision.as_deref())?;
            let next = match &h.assoc {
                Some(a) => {
                    let mut tie = a.clone();
                    for (id, _) in &islands {
                        if tie.outer != Some(*id) && !tie.bounds.contains(id) && !tie.islands.contains(id) {
                            tie.islands.push(*id);
                        }
                    }
                    // Nothing new: nothing to write.
                    if tie == *a {
                        return Ok(Checked {
                            changed: Vec::new(),
                            created: Vec::new(),
                            tie_to_created: None,
                            warnings,
                        });
                    }
                    let tied = HatchEntity {
                        assoc: Some(tie),
                        ..h.clone()
                    };
                    doc.recut_hatch(&tied).ok_or_else(|| not_enclosed("edit.uids"))?
                }
                None => {
                    let loops = loops_of(&h);
                    let area = hatch_boundary::area_of(&loops).ok_or_else(|| not_enclosed("edit.uid"))?;
                    let cutters: Vec<Area> = islands.iter().flat_map(|(_, s)| areas_of_entity(s)).collect();
                    let parts = subtract_areas(std::slice::from_ref(&area), &cutters);
                    let largest = parts
                        .iter()
                        .filter(|a| net_area(a) > 0.0)
                        .max_by(|a, b| net_area(a).total_cmp(&net_area(b)))
                        .ok_or_else(|| not_enclosed("edit.uids"))?;
                    let mut sources: Vec<Shape> = islands.iter().map(|(_, s)| s.clone()).collect();
                    sources.extend(loops.iter().filter_map(hatch_boundary::whole_curve));
                    with_area(&h, largest, &sources)
                }
            };
            changed.push((slot, u, next));
        }
        HatchEdit::RemoveIslands { uid, uids, holes } => {
            let (slot, u, h) = hatch_at(doc, uid, "edit.uid")?;
            if uids.is_empty() && holes.is_empty() {
                return Err(fail(
                    codes::NO_ENTITIES,
                    "Çıkarılacak ada yok. Adaların nesnelerini (uids) ya da deliklerin sırasını (holes) verin.".into(),
                    "edit.uids",
                ));
            }
            for (i, uid) in uids.iter().enumerate() {
                if !is_uid_text(uid) {
                    return Err(bad_uid(uid, &format!("edit.uids[{i}]")));
                }
            }
            let next = match &h.assoc {
                Some(a) => {
                    if !holes.is_empty() {
                        return Err(fail(
                            codes::ISLAND_NOT_FOUND,
                            "İlişkili taramanın adası nesnesiyle (uids) verilir; deliğin sırası onu izlemez.".into(),
                            "edit.holes",
                        ));
                    }
                    let mut tie = a.clone();
                    for (i, uid) in uids.iter().enumerate() {
                        let id = Uuid::parse_str(uid).map(|u| EntityId(*u.as_bytes())).ok();
                        let Some(k) = id.and_then(|id| tie.islands.iter().position(|x| *x == id)) else {
                            return Err(fail(
                                codes::ISLAND_NOT_FOUND,
                                "Verilen nesne taramanın adalarından biri değil; ilişkideki adaları Öznitelikler gösterir.".into(),
                                &format!("edit.uids[{i}]"),
                            ));
                        };
                        tie.islands.remove(k);
                    }
                    checks::revision(doc, input.expected_revision.as_deref())?;
                    let tied = HatchEntity {
                        assoc: Some(tie),
                        ..h.clone()
                    };
                    doc.recut_hatch(&tied).ok_or_else(|| not_enclosed("edit.uids"))?
                }
                None => {
                    if !uids.is_empty() {
                        return Err(fail(
                            codes::ISLAND_NOT_FOUND,
                            "İlişkisiz taramanın adası deliğinin sırasıyla (holes) verilir; nesneleri yoktur.".into(),
                            "edit.uids",
                        ));
                    }
                    let loops = loops_of(&h);
                    let count = loops.len().saturating_sub(1);
                    for (i, k) in holes.iter().enumerate() {
                        if *k as usize >= count {
                            return Err(fail(
                                codes::ISLAND_NOT_FOUND,
                                format!("Taramanın {count} deliği var; {k}. sırada delik yok (0 ilki)."),
                                &format!("edit.holes[{i}]"),
                            ));
                        }
                    }
                    checks::revision(doc, input.expected_revision.as_deref())?;
                    let gone: HashSet<usize> = holes.iter().map(|k| *k as usize + 1).collect();
                    let kept: Vec<HatchLoop> = loops
                        .into_iter()
                        .enumerate()
                        .filter(|(i, _)| !gone.contains(i))
                        .map(|(_, l)| l)
                        .collect();
                    let area = hatch_boundary::area_of(&kept).ok_or_else(|| not_enclosed("edit.holes"))?;
                    let sources: Vec<Shape> = kept.iter().filter_map(hatch_boundary::whole_curve).collect();
                    with_area(&h, &area, &sources)
                }
            };
            changed.push((slot, u, next));
        }
        HatchEdit::Untie { uids } => {
            if uids.is_empty() {
                return Err(fail(
                    codes::NO_ENTITIES,
                    "Tarama yok. En az bir taramanın kalıcı kimliğini verin.".into(),
                    "edit.uids",
                ));
            }
            let mut seen = HashSet::new();
            for (i, uid) in uids.iter().enumerate() {
                let (slot, u, h) = hatch_at(doc, uid, &format!("edit.uids[{i}]"))?;
                if seen.insert(u) && h.assoc.is_some() {
                    changed.push((slot, u, HatchEntity { assoc: None, ..h }));
                }
            }
            checks::revision(doc, input.expected_revision.as_deref())?;
        }
    }
    changed.retain(|(slot, _, h)| doc.get(*slot) != Some(&Entity::Hatch(h.clone())));
    Ok(Checked {
        changed,
        created,
        tie_to_created,
        warnings,
    })
}

/// A region's refusal said at the edit's region.
fn reroot(stop: Stop, from: &str, to: &str) -> Stop {
    let mut stop = stop;
    let (Stop::Failed(e) | Stop::Conflict(e)) = &mut stop;
    if let Some(p) = &mut e.path
        && p.starts_with(from)
    {
        *p = p.replacen(from, to, 1);
    }
    stop
}

fn store_for<'s>(cx: Option<&'s Store>, own: &'s mut Option<Store>, doc: &Document) -> &'s Store {
    match cx {
        Some(s) => s,
        None => own.insert(store_of(doc)),
    }
}

pub fn validate(cx: &ExecutionContext<'_>, input: &HatchesEdit) -> CommandResult<()> {
    let mut own = None;
    let store = store_for(cx.store, &mut own, cx.doc);
    match check(cx.doc, store, input) {
        Ok(c) => CommandResult::Completed {
            output: (),
            warnings: c.warnings,
        },
        Err(stop) => stop.into(),
    }
}

pub fn plan(cx: &ExecutionContext<'_>, input: &HatchesEdit) -> CommandResult<HatchesPlan> {
    plan_in(cx.doc, cx.store, input)
}

/// [`plan`] over the document (a tool's preview).
pub fn plan_in(doc: &Document, store: Option<&Store>, input: &HatchesEdit) -> CommandResult<HatchesPlan> {
    let mut own = None;
    let store = store_for(store, &mut own, doc);
    match check(doc, store, input) {
        Ok(c) => CommandResult::Completed {
            output: HatchesPlan {
                hatches: c
                    .changed
                    .into_iter()
                    .map(|(_, _, h)| Entity::Hatch(h))
                    .collect(),
                created: c.created,
                revision: doc.revision().to_string(),
            },
            warnings: c.warnings,
        },
        Err(stop) => stop.into(),
    }
}

pub fn execute(cx: &mut ExecutionContext<'_>, input: HatchesEdit) -> CommandResult<HatchesEdited> {
    crate::call::recorded(
        cx,
        kentos_contracts::CAD_HATCHES_EDIT,
        kentos_contracts::CAD_HATCHES_EDIT_VERSION,
        input,
        write_execute,
    )
}

fn write_execute(cx: &mut ExecutionContext<'_>, input: HatchesEdit) -> CommandResult<HatchesEdited> {
    let mut own = None;
    let c = {
        let store = store_for(cx.store, &mut own, cx.doc);
        match check(cx.doc, store, &input) {
            Ok(c) => c,
            Err(stop) => return stop.into(),
        }
    };
    let doc = &mut *cx.doc;
    let name = label(&input.edit);
    let mut created_ids: Vec<String> = Vec::new();
    let mut changed_ids: Vec<String> = c.changed.iter().map(|(_, u, _)| u.to_string()).collect();
    let warnings = c.warnings;
    let result: Result<(), Stop> = doc.transact(name, |doc| {
        let mut made: Vec<EntityId> = Vec::new();
        for e in c.created {
            let (_, uid, _) = checks::add(doc, e)?;
            if let Ok(u) = Uuid::parse_str(&uid) {
                made.push(EntityId(*u.as_bytes()));
            }
            created_ids.push(uid);
        }
        let mut changes: Vec<(Slot, Entity)> = c
            .changed
            .into_iter()
            .map(|(s, _, h)| (s, Entity::Hatch(h)))
            .collect();
        // The hatch then follows what was written (docs/adr/0329 §4).
        if let Some((slot, form, seed)) = c.tie_to_created
            && let Some((_, Entity::Hatch(h))) = changes.iter_mut().find(|(s, _)| *s == slot)
            && let Some((first, rest)) = made.split_first()
        {
            h.assoc = Some(match form {
                BoundaryForm::Area => HatchAssoc {
                    gap: None,
                    outer: Some(*first),
                    bounds: Vec::new(),
                    islands: Vec::new(),
                    cutouts: h.assoc.as_ref().map(|a| a.cutouts.clone()).unwrap_or_default(),
                    seed,
                },
                BoundaryForm::Polyline => HatchAssoc {
                    gap: None,
                    outer: None,
                    bounds: vec![*first],
                    islands: rest.to_vec(),
                    cutouts: Vec::new(),
                    seed,
                },
            });
        }
        if !changes.is_empty() {
            doc.update_many(changes, name);
        }
        Ok(())
    });
    if let Err(stop) = result {
        changed_ids.clear();
        return stop.into();
    }
    CommandResult::Completed {
        output: HatchesEdited {
            changed: changed_ids,
            created: created_ids,
            revision: doc.revision().to_string(),
        },
        warnings,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kentos_geometry_core::entity::HatchCurve;

    #[test]
    fn a_whole_ellipse_loop_is_written_back_as_an_ellipse() {
        let l = HatchLoop {
            pts: vec![Vec2::new(5.0, 0.0)],
            bulges: None,
            curves: Some(vec![Some(HatchCurve::Ellipse {
                c: Vec2::new(0.0, 0.0),
                major: Vec2::new(5.0, 0.0),
                ratio: 0.5,
                t0: 0.0,
                t1: -TAU,
            })]),
        };
        let (out, approximated) = recreated(std::slice::from_ref(&l), BoundaryForm::Polyline, "a");
        assert!(!approximated);
        assert!(matches!(&out[0], Entity::Ellipse(e) if e.ratio == 0.5));
        let (out, approximated) = recreated(&[l], BoundaryForm::Area, "a");
        assert!(approximated);
        assert!(matches!(&out[0], Entity::Polygon(p) if p.pts.len() > 8));
    }
}
