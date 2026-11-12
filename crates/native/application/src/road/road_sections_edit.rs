//! `road.sections.edit` v1 (plan/road.md Y5, docs/adr/0903 §1–§2): a sample line set written (new, or the given fields
//! replacing its), rebound to its alignment's present revision, or removed; one undo step “Kesit takımı”. The shared
//! cases are `fixtures/commands/v1/road.sections.edit.json`.

use kentos_commands::{CommandResult, RoadEditOperation, SectionSetDef, SectionsEdit, SectionsWritten};

use crate::ExecutionContext;
use crate::checks::{self, Stop};
use crate::codes;
use crate::road_alignments as alignments;
use crate::road_profiles as profiles;
use crate::road_sections as common;
use crate::road_terrains::fail;

pub(crate) const STEP: &str = "Kesit takımı";

struct Checked {
    list: Vec<SectionSetDef>,
    target: Option<String>,
}

/// A name given for a profile, template or surface: its id, or none for an empty one.
fn named(
    doc: &kentos_domain::Document,
    given: Option<&str>,
    kind: &str,
) -> Result<Option<Option<String>>, Stop> {
    let Some(name) = given else { return Ok(None) };
    if name.is_empty() {
        return Ok(Some(None));
    }
    let id = match kind {
        "profile" => profiles::find(doc.road_profiles(), name, "profile")?.1.id.clone(),
        "template" => common::find_template(doc.road_templates(), name, "template")?.1.id.clone(),
        "superelevation" => crate::road_superelevations::find(doc.road_superelevations(), name, "superelevation")?.1.id.clone(),
        _ => crate::road_profiles::surface_id(doc, name, "surface")?,
    };
    Ok(Some(Some(id)))
}

fn check(doc: &kentos_domain::Document, input: &SectionsEdit) -> Result<Checked, Stop> {
    common::road_project(doc)?;
    let own = doc.road_sections();
    let mut list = own.to_vec();
    let found = input.set.as_deref().and_then(|t| common::find_set(own, t, "set").ok());
    if matches!(input.operation, RoadEditOperation::Remove | RoadEditOperation::Rebind) && found.is_none() {
        let name = input.set.clone().unwrap_or_default();
        return Err(common::find_set(own, &name, "set").err().unwrap_or(Stop::Cancelled));
    }
    let alignment = match input.alignment.as_deref() {
        Some(a) => Some(alignments::find(doc.road_alignments(), a, "alignment")?.1.clone()),
        None => None,
    };
    let profile = named(doc, input.profile.as_deref(), "profile")?;
    let template = named(doc, input.template.as_deref(), "template")?;
    let superelevation = named(doc, input.superelevation.as_deref(), "superelevation")?;
    let surface = named(doc, input.surface.as_deref(), "surface")?;
    let target = match (input.operation, found) {
        (RoadEditOperation::Remove | RoadEditOperation::Rebind, None) => {
            let name = input.set.clone().unwrap_or_default();
            return Err(common::find_set(own, &name, "set").err().unwrap_or(Stop::Cancelled));
        }
        (RoadEditOperation::Remove, Some((at, _))) => {
            list.remove(at);
            None
        }
        (RoadEditOperation::Rebind, Some((at, old))) => {
            let now = doc.road_alignments().iter().find(|a| a.id == old.alignment).map_or(0, |a| a.revision);
            let mut d = old.clone();
            if d.alignment_revision != now {
                d.alignment_revision = now;
                d.revision = old.revision.saturating_add(1);
            }
            list[at] = d;
            Some(list[at].id.clone())
        }
        (RoadEditOperation::Set, found) => {
            let given = input.name.is_some()
                || alignment.is_some()
                || profile.is_some()
                || template.is_some()
                || surface.is_some()
                || superelevation.is_some()
                || input.left.is_some()
                || input.right.is_some()
                || input.stations.is_some()
                || input.clip.is_some();
            match found {
                Some((at, old)) => {
                    if !given {
                        return Err(fail(
                            codes::INVALID_SECTION_SET,
                            "Değiştirilecek bir şey yok: name, alignment, profile, template, surface, superelevation, left, right, stations ya da clip verin."
                                .into(),
                            "",
                        ));
                    }
                    let mut d = old.clone();
                    if let Some(n) = &input.name {
                        d.name = n.clone();
                    }
                    if let Some(a) = &alignment {
                        d.alignment = a.id.clone();
                        d.alignment_revision = a.revision;
                    }
                    if let Some(p) = profile {
                        d.profile = p;
                    }
                    if let Some(t) = template {
                        d.template = t;
                    }
                    if let Some(s) = surface {
                        d.surface = s;
                    }
                    if let Some(e) = superelevation {
                        d.superelevation = e;
                    }
                    if let Some(l) = input.left {
                        d.left = l;
                    }
                    if let Some(r) = input.right {
                        d.right = r;
                    }
                    if let Some(st) = &input.stations {
                        d.stations = st.clone();
                    }
                    if let Some(c) = input.clip {
                        d.clip = c;
                    }
                    if d != *old {
                        d.revision = old.revision.saturating_add(1);
                    }
                    list[at] = d;
                    Some(list[at].id.clone())
                }
                None => {
                    let missing = [
                        ("name", input.name.is_none()),
                        ("alignment", alignment.is_none()),
                        ("left", input.left.is_none()),
                        ("right", input.right.is_none()),
                        ("stations", input.stations.is_none()),
                    ]
                    .into_iter()
                    .find(|(_, m)| *m);
                    if let Some((field, _)) = missing {
                        return Err(fail(
                            codes::INVALID_SECTION_SET,
                            "Yeni kesit takımı için name, alignment, left, right ve stations verilmeli.".into(),
                            field,
                        ));
                    }
                    let (Some(a), Some(stations)) = (alignment, input.stations.clone()) else {
                        return Err(Stop::Cancelled);
                    };
                    let id = input.set.clone().unwrap_or_else(|| {
                        (1..).map(|n| format!("k-{n}")).find(|id| own.iter().all(|s| &s.id != id)).unwrap_or_default()
                    });
                    list.push(SectionSetDef {
                        id: id.clone(),
                        name: input.name.clone().unwrap_or_default(),
                        alignment: a.id.clone(),
                        alignment_revision: a.revision,
                        revision: 1,
                        profile: profile.flatten(),
                        template: template.flatten(),
                        surface: surface.flatten(),
                        superelevation: superelevation.flatten(),
                        left: input.left.unwrap_or_default(),
                        right: input.right.unwrap_or_default(),
                        stations,
                        clip: input.clip.unwrap_or(false),
                    });
                    Some(id)
                }
            }
        }
    };
    common::judged(&common::road_with(doc, None, Some(&list)), codes::INVALID_SECTION_SET, "Kesit takımı")?;
    if let Some(d) = target.as_ref().and_then(|id| list.iter().find(|s| &s.id == id)) {
        let built = common::alignment_of(doc, d)?;
        common::stations_of(doc, d, &built)?;
    }
    checks::revision(doc, input.expected_revision.as_deref())?;
    Ok(Checked { list, target })
}

fn write(cx: &mut ExecutionContext<'_>, c: Checked) -> CommandResult<SectionsWritten> {
    let doc = &mut *cx.doc;
    if let Err(r) = doc.set_road_sections(c.list.clone(), STEP) {
        return fail(codes::INVALID_SECTION_SET, format!("Kesit takımı kurallarına uymuyor: {}.", r.0), "").into();
    }
    let sets = common::reports(doc, &c.list);
    CommandResult::Completed {
        output: SectionsWritten {
            set: c.target.as_ref().and_then(|id| sets.iter().find(|s| &s.id == id)).cloned(),
            sets,
            revision: doc.revision().to_string(),
        },
        warnings: vec![],
    }
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &SectionsEdit) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(_) => CommandResult::Completed { output: (), warnings: vec![] },
        Err(stop) => stop.into(),
    }
}

/// What execute would write now; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &SectionsEdit) -> CommandResult<SectionsWritten> {
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

/// Writes the set: one undo step.
pub fn execute(cx: &mut ExecutionContext<'_>, input: SectionsEdit) -> CommandResult<SectionsWritten> {
    crate::call::recorded(
        cx,
        kentos_commands::ROAD_SECTIONS_EDIT,
        kentos_commands::ROAD_SECTIONS_EDIT_VERSION,
        input,
        |cx, input| match check(cx.doc, &input) {
            Ok(c) => write(cx, c),
            Err(stop) => stop.into(),
        },
    )
}
