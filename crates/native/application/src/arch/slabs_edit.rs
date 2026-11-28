//! `cad.slabs.edit` v1 (docs/adr/0722 §4): slabs named by their persistent ids changed as one undo step “Döşemeyi
//! düzenle”, all or none. Değiştir sets their own fields (Öznitelikler writes through it); Kenarı ötele moves every
//! edge; Kes takes an area drawn or found from the walls out of each (a hole, a notch: a wet room's lowered slab), Ekle
//! adds one. Each slab's edge's runs are written again from the walls. The shared cases are
//! `fixtures/commands/v1/cad.slabs.edit.json`.

use kentos_commands::{
    CommandResult, Entity, SlabBound, SlabBoundary, SlabChange, SlabEdgeBindings, SlabEntity,
    SlabFields, SlabSourceKind, SlabsEdit, SlabsEdited, SlabsOperation, SpacePart, Vec2,
};
use kentos_domain::{Document, Slot};
use kentos_geometry_core::geom::region::{net_area, subtract_areas, union_areas};

use crate::ExecutionContext;
use crate::checks::{self, Stop};
use crate::codes;
use crate::slabs;
use crate::spaces::{self, fail};

pub const STEP: &str = "Döşemeyi düzenle";

fn changed(s: &SlabFields, c: &SlabChange) -> SlabFields {
    let mut f = s.clone();
    if let Some(v) = c.role {
        f.role = v;
    }
    if let Some(v) = c.thickness {
        f.thickness = v;
    }
    if let Some(v) = &c.layers {
        f.layers = v.clone();
    }
    if let Some(v) = c.offset {
        f.offset = v;
    }
    if let Some(v) = &c.level {
        f.arch.level.clone_from(v);
    }
    if let Some(v) = c.phase {
        f.arch.phase = v;
    }
    if let Some(v) = &c.material {
        f.material.clone_from(v);
        // A text of its own leaves the project material (docs/adr/0785 §2).
        f.material_id = None;
    }
    if let Some(v) = &c.edge {
        f.edge = v.as_ref().map(|e| {
            let mut edge = slabs::edge_of(e);
            // The same kind keeps its runs until the document writes them again.
            if let Some(old) = s.edge.as_ref().filter(|o| o.kind == edge.kind) {
                edge.runs.clone_from(&old.runs);
            }
            edge
        });
    }
    if let Some(v) = c.excluded {
        f.excluded = v;
    }
    // Its slope, or level again (docs/adr/8071 §6).
    if let Some(v) = c.slope {
        f.slope = v;
    }
    // A covering or a slab again (docs/adr/8070 §10).
    if let Some(v) = &c.covering {
        f.covering.clone_from(v);
    }
    f
}

/// The area an operation adds or takes out: drawn, or found from the walls round `seed` on the slab's level.
fn area_of(
    doc: &Document,
    parts: Option<&Vec<SpacePart>>,
    seed: Option<Vec2>,
    boundary: Option<SlabBoundary>,
    level: Option<&str>,
) -> Result<Vec<SpacePart>, Stop> {
    slabs::outline(doc, parts, seed, boundary, level, "operation")
}

fn check(doc: &Document, input: &SlabsEdit) -> Result<Vec<(Slot, SlabEntity)>, Stop> {
    checks::uids(&input.uids, "Düzenlenecek döşeme verilmedi.")?;
    let found = checks::named(doc, &input.uids)?;
    let mut list: Vec<(usize, Slot, &SlabEntity)> = Vec::with_capacity(found.len());
    for (at, slot, e, _) in &found {
        match e {
            Entity::Slab(s) => list.push((*at, *slot, s)),
            _ => {
                return Err(fail(
                    codes::NOT_A_SLAB,
                    "Nesne döşeme değil; Döşemeyi düzenle yalnız döşemeleri değiştirir.".into(),
                    &format!("uids[{at}]"),
                ));
            }
        }
    }
    let mut out = Vec::with_capacity(list.len());
    match &input.operation {
        SlabsOperation::Set(c) => {
            if c == &SlabChange::default() {
                return Err(fail(
                    codes::NOTHING_TO_SET,
                    "Değiştirilecek bir alan verilmedi: tür, kalınlık, kot farkı, kat, tadilat durumu, malzeme, kenar, katmanlar, kaplama, eğim ya da metraj dışı seçimlerinden birini verin.".into(),
                    "operation",
                ));
            }
            for (v, what, field) in [
                (c.thickness, "Kalınlık", "thickness"),
                (c.offset, "Kot farkı", "offset"),
            ] {
                if let Some(v) = v {
                    checks::finite(
                        v,
                        what,
                        "Sonlu bir sayı verin.",
                        &format!("operation.{field}"),
                    )?;
                }
            }
            if let Some(Some(e)) = &c.edge {
                slabs::edge_finite(e, "operation.edge")?;
            }
            for (at, slot, s) in &list {
                let f = changed(&s.slab, c);
                slabs::valid(&f, &format!("uids[{at}]"))?;
                spaces::level_known(doc, &f.arch, "operation.level")?;
                out.push((*slot, *at, f));
            }
        }
        SlabsOperation::Offset { distance } => {
            checks::finite(
                *distance,
                "Kenar ötelemesi",
                "Sonlu bir sayı verin.",
                "operation.distance",
            )?;
            if *distance == 0.0 {
                return Err(fail(
                    codes::NOTHING_TO_SET,
                    "Kenar ötelemesi sıfır; kenarları taşımak için sıfırdan farklı bir uzaklık verin.".into(),
                    "operation.distance",
                ));
            }
            for (at, slot, s) in &list {
                let mut f = s.slab.clone();
                if f.bound.as_ref().is_some_and(|b| b.edges.is_some()) {
                    return Err(fail(
                        codes::SLAB_BOUND,
                        "Bağlı kenarları tek tek öteleyin veya önce konturu ayırın.".into(),
                        "operation.distance",
                    ));
                }
                f.parts = slabs::moved(&f.parts, *distance, &format!("uids[{at}]"))?;
                // A bound slab keeps its offset with its source (docs/adr/0771 §4).
                if let Some(b) = &mut f.bound {
                    b.offset += *distance;
                }
                slabs::valid(&f, &format!("uids[{at}]"))?;
                out.push((*slot, *at, *f));
            }
        }
        SlabsOperation::Bind {
            seed,
            boundary,
            space,
            offset,
        } => {
            if let Some(p) = seed {
                checks::point(*p, "Döşemenin iç noktasının", "operation.seed")?;
            }
            if let Some(o) = offset {
                checks::finite(
                    *o,
                    "Kenar ötelemesi",
                    "Sonlu bir sayı verin.",
                    "operation.offset",
                )?;
            }
            if seed.is_some() == space.is_some() {
                return Err(fail(
                    codes::INVALID_SLAB,
                    "Döşeme ya duvarlara (seed: duvarlarla çevrili bir iç nokta) ya da bir mahale (space) bağlanır; birini verin.".into(),
                    "operation",
                ));
            }
            for (at, slot, s) in &list {
                let level = s.slab.arch.level.as_deref();
                if s.slab
                    .bound
                    .as_ref()
                    .and_then(|b| b.edges.as_ref())
                    .is_some_and(|e| e.links.iter().any(|l| l.locked))
                {
                    return Err(fail(
                        codes::SLAB_BOUND,
                        "Tüm konturu bağlamadan önce kenar kilitlerini açın.".into(),
                        &format!("uids[{at}]"),
                    ));
                }
                let parts = match space {
                    Some(uid) => slabs::space_parts(doc, uid, "operation.space")?,
                    None => slabs::outline(doc, None, *seed, *boundary, level, "operation")?,
                };
                let d = offset.unwrap_or(0.0);
                let mut f = s.slab.clone();
                f.parts = slabs::moved(&parts, d, "operation.offset")?;
                f.bound = Some(slabs::bound_of(*seed, *boundary, space.as_ref(), d));
                slabs::valid(&f, &format!("uids[{at}]"))?;
                out.push((*slot, *at, slabs::with_bound_uids(doc, *f)));
            }
        }
        SlabsOperation::Detach => {
            for (at, slot, s) in &list {
                if s.slab.bound.is_none() {
                    return Err(fail(
                        codes::SLAB_NOT_BOUND,
                        "Döşeme kaynağa bağlı değil; ayrılacak bağ yok.".into(),
                        &format!("uids[{at}]"),
                    ));
                }
                let mut f = s.slab.clone();
                if f.bound
                    .as_ref()
                    .and_then(|b| b.edges.as_ref())
                    .is_some_and(|e| e.links.iter().any(|l| l.locked))
                {
                    return Err(fail(
                        codes::SLAB_BOUND,
                        "Konturu ayırmadan önce bağlı kenarların kilidini açın.".into(),
                        &format!("uids[{at}]"),
                    ));
                }
                f.bound = None;
                out.push((*slot, *at, *f));
            }
        }
        SlabsOperation::BindEdge(link) => {
            let Some(wall) = kentos_domain::Uuid::parse_str(&link.wall).ok() else {
                return Err(fail(
                    codes::INVALID_SLAB,
                    "Geçerli bir kaynak duvar kimliği verin.".into(),
                    "operation.wall",
                ));
            };
            let mut link = link.clone();
            link.wall = wall.to_string();
            for (at, slot, s) in &list {
                let mut f = s.slab.clone();
                let field = format!("uids[{at}]");
                if f.bound
                    .as_ref()
                    .is_some_and(|b| b.source != SlabSourceKind::Edges)
                    || f.parts.len() != 1
                {
                    return Err(fail(
                        codes::SLAB_BOUND,
                        "Tekil bağ için bağımsız, tek parçalı döşeme konturu kullanın.".into(),
                        &field,
                    ));
                }
                let bound = f.bound.get_or_insert_with(|| SlabBound {
                    source: SlabSourceKind::Edges,
                    edges: Some(SlabEdgeBindings {
                        reference: f.parts[0].clone(),
                        links: Vec::new(),
                    }),
                    ..SlabBound::default()
                });
                let edges = bound.edges.as_mut().ok_or_else(|| {
                    fail(
                        codes::INVALID_SLAB,
                        "Döşemenin kenar bağı kaydı eksik.".into(),
                        &field,
                    )
                })?;
                if let Some(existing) = edges.links.iter_mut().find(|l| l.edge == link.edge) {
                    if existing.locked
                        && (existing.wall != link.wall
                            || existing.segment != link.segment
                            || existing.face != link.face
                            || existing.offset != link.offset)
                    {
                        return Err(fail(
                            codes::SLAB_BOUND,
                            "Kaynağı veya ötelemeyi değiştirmek için önce kenar kilidini açın."
                                .into(),
                            "operation.edge",
                        ));
                    }
                    *existing = link.clone();
                } else {
                    edges.links.push(link.clone());
                    edges.links.sort_by_key(|l| l.edge);
                }
                slabs::valid(&f, &field)?;
                out.push((*slot, *at, edge_result(doc, *f, &field)?));
            }
        }
        SlabsOperation::UnbindEdge { edge } => {
            for (at, slot, s) in &list {
                let mut f = s.slab.clone();
                let field = format!("uids[{at}]");
                let Some(edges) = f.bound.as_mut().and_then(|b| b.edges.as_mut()) else {
                    return Err(fail(
                        codes::SLAB_NOT_BOUND,
                        "Döşemede tekil kenar bağı yok.".into(),
                        &field,
                    ));
                };
                let Some(i) = edges.links.iter().position(|l| l.edge == *edge) else {
                    return Err(fail(
                        codes::SLAB_NOT_BOUND,
                        "Bu kenar zaten serbest.".into(),
                        "operation.edge",
                    ));
                };
                if edges.links[i].locked {
                    return Err(fail(
                        codes::SLAB_BOUND,
                        "Kenarı ayırmadan önce kilidini açın.".into(),
                        "operation.edge",
                    ));
                }
                edges.links.remove(i);
                if edges.links.is_empty() {
                    f.bound = None;
                } else {
                    edges.reference = f.parts[0].clone();
                }
                out.push((*slot, *at, *f));
            }
        }
        SlabsOperation::Subtract {
            parts,
            seed,
            boundary,
        }
        | SlabsOperation::Add {
            parts,
            seed,
            boundary,
        } => {
            let cut = matches!(input.operation, SlabsOperation::Subtract { .. });
            if let Some((at, ..)) = list.iter().find(|(_, _, s)| {
                s.slab
                    .bound
                    .as_ref()
                    .is_some_and(|b| !cut || b.edges.is_none())
            }) {
                return Err(fail(
                    codes::SLAB_BOUND,
                    "Döşemenin sınırı kaynağına bağlı; kesmek ya da alan eklemek için önce Ayır ile bağımsız kontur yapın.".into(),
                    &format!("uids[{at}]"),
                ));
            }
            if let Some(p) = seed {
                checks::point(*p, "Alanın iç noktasının", "operation.seed")?;
            }
            if let Some(ps) = parts {
                slabs::parts_finite(ps, "operation.parts")?;
            }
            // A cut found round a point is the room there: the outline of the walls' outer faces holds the whole slab
            // (docs/adr/0798 §3). An addition keeps the outline as Bind does.
            let boundary = if cut {
                Some(boundary.unwrap_or(SlabBoundary::Room))
            } else {
                *boundary
            };
            for (at, slot, s) in &list {
                let area = area_of(
                    doc,
                    parts.as_ref(),
                    *seed,
                    boundary,
                    s.slab.arch.level.as_deref(),
                )?;
                let mine = slabs::areas(&s.slab.parts);
                let other = slabs::areas(&area);
                let before: f64 = mine.iter().map(net_area).sum();
                let after = if cut {
                    subtract_areas(&mine, &other)
                } else {
                    union_areas(&[mine.clone(), other].concat())
                };
                let left: f64 = after.iter().map(net_area).sum();
                // A cut that takes nothing, or everything, misses (an area within the slab's tolerance).
                let same = (left - before).abs() <= 1e-9 * before.abs().max(1.0);
                if cut && (same || after.is_empty() || left <= 1e-12) {
                    return Err(fail(
                        codes::SLAB_CUT_MISSED,
                        if after.is_empty() || left <= 1e-12 {
                            "Kesilen alan döşemenin hepsini götürüyor; döşemenin içinde kalan bir alan verin ya da döşemeyi silin.".into()
                        } else {
                            "Kesilen alan döşemeye değmiyor; döşemenin üzerinde kalan bir alan verin.".into()
                        },
                        &format!("uids[{at}]"),
                    ));
                }
                let mut f = s.slab.clone();
                if let Some(edges) = f.bound.as_mut().and_then(|b| b.edges.as_mut()) {
                    if after.len() != 1
                        || (kentos_geometry_core::geom::region::ring_area(&after[0].outer).abs()
                            - kentos_geometry_core::geom::region::ring_area(&mine[0].outer).abs())
                        .abs()
                            > 1e-9 * before.max(1.0)
                    {
                        return Err(fail(
                            codes::SLAB_BOUND,
                            "Bağlı kenarlı döşemede şaft dış konturun içinde kalmalı.".into(),
                            &format!("uids[{at}]"),
                        ));
                    }
                    let mut cut_parts = slabs::parts_of(&after);
                    edges.reference.holes = std::mem::take(&mut cut_parts[0].holes);
                    *f = edge_result(doc, *f, &format!("uids[{at}]"))?;
                } else {
                    f.parts = slabs::parts_of(&after);
                }
                slabs::valid(&f, &format!("uids[{at}]"))?;
                out.push((*slot, *at, *f));
            }
        }
    }
    checks::revision(doc, input.expected_revision.as_deref())?;
    for (at, _, e, _) in &found {
        let layer = &e.base().layer_id;
        if doc.layers().is_locked(layer) {
            let name = doc
                .layers()
                .get(layer)
                .map_or(layer.as_str(), |n| n.name.as_str());
            return Err(fail(
                codes::LAYER_LOCKED,
                format!(
                    "“{name}” katmanı kilitli; üzerindeki döşeme düzenlenemez. Kilidi Katmanlar panelinden açın."
                ),
                &format!("uids[{at}]"),
            ));
        }
    }
    Ok(out
        .into_iter()
        .map(|(slot, at, f)| {
            let (_, _, s) = list
                .iter()
                .find(|(a, ..)| *a == at)
                .copied()
                .unwrap_or(list[0]);
            let mut e = s.clone();
            // A bound slab as its source gives it (docs/adr/0771 §4), then its edge's runs as the document writes them
            // (docs/adr/0722 §3).
            let f = if f.bound.is_some() && f.bound != s.slab.bound
                || f.arch.level != s.slab.arch.level
            {
                doc.slab_bound_again(&f)
            } else {
                f
            };
            e.slab = Box::new(doc.slab_with_runs(&f));
            (slot, e)
        })
        .collect())
}

fn edge_result(doc: &Document, f: SlabFields, field: &str) -> Result<SlabFields, Stop> {
    let f = doc.slab_bound_again(&f);
    if let Some(broken) = f.bound.as_ref().and_then(|b| b.broken) {
        return Err(fail(codes::INVALID_SLAB, broken.words().into(), field));
    }
    slabs::valid(&f, field)?;
    Ok(f)
}

pub fn validate(cx: &ExecutionContext<'_>, input: &SlabsEdit) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

pub fn plan(cx: &ExecutionContext<'_>, input: &SlabsEdit) -> CommandResult<SlabsEdited> {
    plan_in(cx.doc, input)
}

/// [`plan`] on a document: what the slab tool draws as its preview.
pub fn plan_in(doc: &Document, input: &SlabsEdit) -> CommandResult<SlabsEdited> {
    match check(doc, input) {
        Ok(work) => CommandResult::Completed {
            output: SlabsEdited {
                slabs: work.into_iter().map(|(_, e)| Entity::Slab(e)).collect(),
                revision: doc.revision().to_string(),
            },
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

pub fn execute(cx: &mut ExecutionContext<'_>, input: SlabsEdit) -> CommandResult<SlabsEdited> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_SLABS_EDIT,
        kentos_commands::CAD_SLABS_EDIT_VERSION,
        input,
        write_execute,
    )
}

fn write_execute(cx: &mut ExecutionContext<'_>, input: SlabsEdit) -> CommandResult<SlabsEdited> {
    let work = match check(cx.doc, &input) {
        Ok(work) => work,
        Err(stop) => return stop.into(),
    };
    let doc = &mut *cx.doc;
    doc.update_many(
        work.iter()
            .map(|(s, e)| (*s, Entity::Slab(e.clone())))
            .collect(),
        STEP,
    );
    let slabs = work
        .iter()
        .filter_map(|(s, _)| doc.get(*s).cloned())
        .collect();
    CommandResult::Completed {
        output: SlabsEdited {
            slabs,
            revision: doc.revision().to_string(),
        },
        warnings: Vec::new(),
    }
}
