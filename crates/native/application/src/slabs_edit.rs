//! `cad.slabs.edit` v1 (docs/adr/0722 §4): slabs named by their persistent ids changed as one undo step “Döşemeyi
//! düzenle”, all or none. Değiştir sets their own fields (Öznitelikler writes through it); Kenarı ötele moves every
//! edge; Kes takes an area drawn or found from the walls out of each (a hole, a notch: a wet room's lowered slab), Ekle
//! adds one. Each slab's edge's runs are written again from the walls. The shared cases are
//! `fixtures/commands/v1/cad.slabs.edit.json`.

use kentos_commands::{
    CommandResult, Entity, SlabBoundary, SlabChange, SlabEntity, SlabFields, SlabsEdit, SlabsEdited, SlabsOperation,
    SpacePart, Vec2,
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
                    "Değiştirilecek bir alan verilmedi: tür, kalınlık, kot farkı, kat, evre, malzeme, kenar ya da metraj dışı seçimlerinden birini verin.".into(),
                    "operation",
                ));
            }
            for (v, what, field) in [(c.thickness, "Kalınlık", "thickness"), (c.offset, "Kot farkı", "offset")] {
                if let Some(v) = v {
                    checks::finite(v, what, "Sonlu bir sayı verin.", &format!("operation.{field}"))?;
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
            checks::finite(*distance, "Kenar ötelemesi", "Sonlu bir sayı verin.", "operation.distance")?;
            if *distance == 0.0 {
                return Err(fail(
                    codes::NOTHING_TO_SET,
                    "Kenar ötelemesi sıfır; kenarları taşımak için sıfırdan farklı bir uzaklık verin.".into(),
                    "operation.distance",
                ));
            }
            for (at, slot, s) in &list {
                let mut f = s.slab.clone();
                f.parts = slabs::moved(&f.parts, *distance, &format!("uids[{at}]"))?;
                slabs::valid(&f, &format!("uids[{at}]"))?;
                out.push((*slot, *at, f));
            }
        }
        SlabsOperation::Subtract { parts, seed, boundary } | SlabsOperation::Add { parts, seed, boundary } => {
            let cut = matches!(input.operation, SlabsOperation::Subtract { .. });
            if let Some(p) = seed {
                checks::point(*p, "Alanın iç noktasının", "operation.seed")?;
            }
            if let Some(ps) = parts {
                slabs::parts_finite(ps, "operation.parts")?;
            }
            for (at, slot, s) in &list {
                let area = area_of(doc, parts.as_ref(), *seed, *boundary, s.slab.arch.level.as_deref())?;
                let mine = slabs::areas(&s.slab.parts);
                let other = slabs::areas(&area);
                let before: f64 = mine.iter().map(net_area).sum();
                let after = if cut { subtract_areas(&mine, &other) } else { union_areas(&[mine.clone(), other].concat()) };
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
                f.parts = slabs::parts_of(&after);
                slabs::valid(&f, &format!("uids[{at}]"))?;
                out.push((*slot, *at, f));
            }
        }
    }
    checks::revision(doc, input.expected_revision.as_deref())?;
    for (at, _, e, _) in &found {
        let layer = &e.base().layer_id;
        if doc.layers().is_locked(layer) {
            let name = doc.layers().get(layer).map_or(layer.as_str(), |n| n.name.as_str());
            return Err(fail(
                codes::LAYER_LOCKED,
                format!("“{name}” katmanı kilitli; üzerindeki döşeme düzenlenemez. Kilidi Katmanlar panelinden açın."),
                &format!("uids[{at}]"),
            ));
        }
    }
    Ok(out
        .into_iter()
        .map(|(slot, at, f)| {
            let (_, _, s) = list.iter().find(|(a, ..)| *a == at).copied().unwrap_or(list[0]);
            let mut e = s.clone();
            // Its edge's runs as the document writes them (docs/adr/0722 §3).
            e.slab = doc.slab_with_runs(&f);
            (slot, e)
        })
        .collect())
}

pub fn validate(cx: &ExecutionContext<'_>, input: &SlabsEdit) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(_) => CommandResult::Completed { output: (), warnings: Vec::new() },
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
    doc.update_many(work.iter().map(|(s, e)| (*s, Entity::Slab(e.clone()))).collect(), STEP);
    let slabs = work
        .iter()
        .filter_map(|(s, _)| doc.get(*s).cloned())
        .collect();
    CommandResult::Completed {
        output: SlabsEdited { slabs, revision: doc.revision().to_string() },
        warnings: Vec::new(),
    }
}
