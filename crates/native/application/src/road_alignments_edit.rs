//! `road.alignments.edit` v1 (plan/road.md Y3, docs/adr/0901 §9): an alignment's name, PIs, km or offset set, the
//! alignment reversed, or removed; one undo step “Aksı düzenle”, with what it did to the stations. The shared cases are
//! `fixtures/commands/v1/road.alignments.edit.json`.
//!
//! Reversing turns the PIs round and swaps each PI's clothoids; the start km stays and the equations go (they would
//! name places read the other way). The parallel axes that follow it change sides (their offsets' signs), so they lie
//! where they lay.

use kentos_commands::{
    AlignmentDef, AlignmentsEdit, AlignmentsEditOperation, AlignmentsWritten, CommandResult, CommandWarning,
    StationMapRow,
};

use crate::ExecutionContext;
use crate::checks::{self, Stop};
use crate::codes;
use crate::road_alignments as common;
use crate::road_terrains::fail;

pub(crate) const STEP: &str = "Aksı düzenle";

struct Checked {
    list: Vec<AlignmentDef>,
    target: Option<String>,
    mapping: Vec<StationMapRow>,
    warnings: Vec<CommandWarning>,
}

fn changed(a: &AlignmentDef, b: &AlignmentDef) -> bool {
    a.name != b.name || a.pis != b.pis || a.offset_of != b.offset_of || a.stationing != b.stationing
}

fn check(doc: &kentos_domain::Document, input: &AlignmentsEdit) -> Result<Checked, Stop> {
    common::road_project(doc)?;
    let own = common::alignments(doc);
    let (at, old) = common::find(own, &input.alignment, "alignment")?;
    let old_core = if old.offset_of.is_none() { common::build(own, old, "").ok().map(|b| b.core) } else { None };
    let mut list = own.to_vec();
    let mut warnings = Vec::new();
    let mut mapping = Vec::new();
    let target = match input.operation {
        AlignmentsEditOperation::Set => {
            if input.name.is_none() && input.pis.is_none() && input.stationing.is_none() && input.offset.is_none() {
                return Err(fail(
                    codes::INVALID_ALIGNMENT,
                    "Değiştirilecek bir şey yok: name, pis, stationing ya da offset verin.".into(),
                    "",
                ));
            }
            let mut def = old.clone();
            if let Some(n) = &input.name {
                def.name = n.clone();
            }
            if let Some(p) = &input.pis {
                if def.offset_of.is_some() {
                    return Err(fail(
                        codes::INVALID_ALIGNMENT,
                        "Paralel aksın PI'ı olmaz: kaynağını izler; kaynağı düzenleyin.".into(),
                        "pis",
                    ));
                }
                def.pis = p.clone();
            }
            if let Some(st) = &input.stationing {
                def.stationing = Some(st.clone()).filter(|s| !s.is_empty());
            }
            if let Some(d) = input.offset {
                match &mut def.offset_of {
                    Some(o) => o.offset = d,
                    None => {
                        return Err(fail(
                            codes::INVALID_ALIGNMENT,
                            "Öteleme yalnız paralel aksın olur.".into(),
                            "offset",
                        ));
                    }
                }
            }
            if changed(old, &def) {
                def.revision = old.revision.saturating_add(1);
            }
            list[at] = def;
            common::judged(&list, at, "")?;
            let built = common::build(&list, &list[at], "")?;
            common::dependents_hold(&list, &list[at].id, "pis")?;
            if list[at].offset_of.is_none() {
                mapping = common::mapping_rows(old_core.as_ref(), Some(&built.core));
            }
            Some(list[at].id.clone())
        }
        AlignmentsEditOperation::Reverse => {
            if old.offset_of.is_some() {
                return Err(fail(
                    codes::INVALID_ALIGNMENT,
                    "Paralel aks ters çevrilmez: kaynağını izler; kaynağı ters çevirin.".into(),
                    "alignment",
                ));
            }
            let mut def = old.clone();
            def.pis.reverse();
            for p in &mut def.pis {
                std::mem::swap(&mut p.spiral_in, &mut p.spiral_out);
            }
            if let Some(st) = &mut def.stationing
                && !st.equations.is_empty()
            {
                st.equations.clear();
                warnings.push(CommandWarning {
                    code: codes::EQUATIONS_DROPPED.into(),
                    message: "Aks ters çevrildi: km eşitlikleri kaldırıldı, başlangıç km'si kaldı.".into(),
                    path: Some("alignment".into()),
                });
            }
            def.stationing = def.stationing.filter(|s| !s.is_empty());
            def.revision = old.revision.saturating_add(1);
            let id = def.id.clone();
            list[at] = def;
            // The parallel axes lie where they lay: right becomes left.
            for d in list.iter_mut() {
                if let Some(o) = d.offset_of.as_mut().filter(|o| o.alignment == id) {
                    o.offset = -o.offset;
                    d.revision = d.revision.saturating_add(1);
                }
            }
            common::judged(&list, at, "")?;
            let built = common::build(&list, &list[at], "")?;
            common::dependents_hold(&list, &id, "alignment")?;
            let length = built.core.length;
            mapping = built
                .core
                .elements
                .iter()
                .map(|e| StationMapRow {
                    element: e.id.clone(),
                    change: "reversed".into(),
                    before: Some([length - e.end_station(), length - e.station]),
                    after: Some([e.station, e.end_station()]),
                })
                .collect();
            Some(id)
        }
        AlignmentsEditOperation::Remove => {
            let users = common::dependents(own, &old.id);
            if !users.is_empty() {
                return Err(fail(
                    codes::ALIGNMENT_IN_USE,
                    format!("“{}” aksını şu paralel akslar izliyor: {}. Önce onları silin.", old.name, users.join(", ")),
                    "alignment",
                ));
            }
            let profiles: Vec<&str> =
                doc.road_profiles().iter().filter(|p| p.alignment == old.id).map(|p| p.name.as_str()).collect();
            if !profiles.is_empty() {
                return Err(fail(
                    codes::ALIGNMENT_IN_USE,
                    format!("“{}” aksına şu boykesitler bağlı: {}. Önce onları silin.", old.name, profiles.join(", ")),
                    "alignment",
                ));
            }
            let sets: Vec<&str> =
                doc.road_sections().iter().filter(|s| s.alignment == old.id).map(|s| s.name.as_str()).collect();
            if !sets.is_empty() {
                return Err(fail(
                    codes::ALIGNMENT_IN_USE,
                    format!("“{}” aksına şu kesit takımları bağlı: {}. Önce onları silin.", old.name, sets.join(", ")),
                    "alignment",
                ));
            }
            let supers: Vec<&str> =
                doc.road_superelevations().iter().filter(|s| s.alignment == old.id).map(|s| s.name.as_str()).collect();
            if !supers.is_empty() {
                return Err(fail(
                    codes::ALIGNMENT_IN_USE,
                    format!("“{}” aksına şu dever çizelgeleri bağlı: {}. Önce onları silin.", old.name, supers.join(", ")),
                    "alignment",
                ));
            }
            let corridors: Vec<&str> =
                doc.road_corridors().iter().filter(|c| c.alignment == old.id).map(|c| c.name.as_str()).collect();
            if !corridors.is_empty() {
                return Err(fail(
                    codes::ALIGNMENT_IN_USE,
                    format!("“{}” aksına şu koridorlar bağlı: {}. Önce onları silin.", old.name, corridors.join(", ")),
                    "alignment",
                ));
            }
            mapping = common::mapping_rows(old_core.as_ref(), None);
            list.remove(at);
            None
        }
    };
    checks::revision(doc, input.expected_revision.as_deref())?;
    Ok(Checked {
        list,
        target,
        mapping,
        warnings,
    })
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &AlignmentsEdit) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(c) => CommandResult::Completed {
            output: (),
            warnings: c.warnings,
        },
        Err(stop) => stop.into(),
    }
}

/// What execute would write now; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &AlignmentsEdit) -> CommandResult<AlignmentsWritten> {
    let c = match check(cx.doc, input) {
        Ok(c) => c,
        Err(stop) => return stop.into(),
    };
    let mut copy = cx.doc.reading_copy();
    let mut planned = write(&mut ExecutionContext::new(&mut copy), c);
    if let CommandResult::Completed { output, .. } = &mut planned {
        output.revision = cx.doc.revision().to_string();
    }
    planned
}

/// Writes the edit: one undo step.
pub fn execute(cx: &mut ExecutionContext<'_>, input: AlignmentsEdit) -> CommandResult<AlignmentsWritten> {
    crate::call::recorded(
        cx,
        kentos_commands::ROAD_ALIGNMENTS_EDIT,
        kentos_commands::ROAD_ALIGNMENTS_EDIT_VERSION,
        input,
        write_execute,
    )
}

fn write(cx: &mut ExecutionContext<'_>, c: Checked) -> CommandResult<AlignmentsWritten> {
    common::write(cx, c.list, STEP, c.target.as_deref(), c.mapping, c.warnings)
}

fn write_execute(cx: &mut ExecutionContext<'_>, input: AlignmentsEdit) -> CommandResult<AlignmentsWritten> {
    match check(cx.doc, &input) {
        Ok(c) => write(cx, c),
        Err(stop) => stop.into(),
    }
}
