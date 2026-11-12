//! `road.templates.edit` v1 (plan/road.md Y5, docs/adr/0903 §3): a template written (new, or the given fields
//! replacing its) or removed; one undo step “Tip kesit”. The shared cases are
//! `fixtures/commands/v1/road.templates.edit.json`.

use kentos_commands::{CommandResult, RoadEditOperation, TemplateDef, TemplatesEdit, TemplatesWritten};

use crate::ExecutionContext;
use crate::checks::{self, Stop};
use crate::codes;
use crate::road_sections as common;
use crate::road_terrains::fail;

pub(crate) const STEP: &str = "Tip kesit";

struct Checked {
    list: Vec<TemplateDef>,
    target: Option<String>,
}

fn check(doc: &kentos_domain::Document, input: &TemplatesEdit) -> Result<Checked, Stop> {
    common::road_project(doc)?;
    if input.pavement.is_some() && input.remove_pavement {
        return Err(fail(
            codes::INVALID_ROAD_TEMPLATE,
            "pavement ile removePavement birlikte verilmez: üstyapı ya yazılır ya kaldırılır.".into(),
            "removePavement",
        ));
    }
    let own = doc.road_templates();
    let mut list = own.to_vec();
    let found = input.template.as_deref().and_then(|t| common::find_template(own, t, "template").ok());
    let target = match (input.operation, found) {
        (RoadEditOperation::Remove | RoadEditOperation::Rebind, None) => {
            let name = input.template.clone().unwrap_or_default();
            return Err(common::find_template(own, &name, "template").err().unwrap_or(Stop::Cancelled));
        }
        (RoadEditOperation::Rebind, Some(_)) => {
            return Err(fail(
                codes::INVALID_ROAD_TEMPLATE,
                "Tip kesit yeniden bağlanmaz: rebind yalnız kesit takımınındır.".into(),
                "operation",
            ));
        }
        (RoadEditOperation::Remove, Some((at, t))) => {
            let users: Vec<&str> =
                doc.road_sections().iter().filter(|s| s.template.as_deref() == Some(&t.id)).map(|s| s.name.as_str()).collect();
            if !users.is_empty() {
                return Err(fail(
                    codes::ROAD_TEMPLATE_IN_USE,
                    format!("“{}” tip kesitini şu kesit takımları kullanıyor: {}. Önce takımlardan kaldırın.", t.name, users.join(", ")),
                    "template",
                ));
            }
            let corridors: Vec<&str> = doc
                .road_corridors()
                .iter()
                .filter(|c| c.regions.iter().any(|r| r.template == t.id))
                .map(|c| c.name.as_str())
                .collect();
            if !corridors.is_empty() {
                return Err(fail(
                    codes::ROAD_TEMPLATE_IN_USE,
                    format!("“{}” tip kesitini şu koridorlar kullanıyor: {}. Önce koridorlardan kaldırın.", t.name, corridors.join(", ")),
                    "template",
                ));
            }
            list.remove(at);
            None
        }
        (RoadEditOperation::Set, found) => match found {
            Some((at, old)) => {
                if input.name.is_none()
                    && input.left.is_none()
                    && input.right.is_none()
                    && !input.mirror
                    && input.pavement.is_none()
                    && !input.remove_pavement
                {
                    return Err(fail(
                        codes::INVALID_ROAD_TEMPLATE,
                        "Değiştirilecek bir şey yok: name, left, right, mirror, pavement ya da removePavement verin.".into(),
                        "",
                    ));
                }
                let mut t = old.clone();
                if let Some(n) = &input.name {
                    t.name = n.clone();
                }
                if let Some(l) = &input.left {
                    t.left = l.clone();
                }
                if let Some(r) = &input.right {
                    t.right = Some(r.clone());
                }
                if input.mirror {
                    t.right = None;
                }
                if let Some(p) = &input.pavement {
                    t.pavement = Some(p.clone());
                }
                if input.remove_pavement {
                    t.pavement = None;
                }
                if t != *old {
                    t.revision = old.revision.saturating_add(1);
                }
                list[at] = t;
                Some(list[at].id.clone())
            }
            None => {
                let (Some(name), Some(left)) = (&input.name, &input.left) else {
                    return Err(fail(
                        codes::INVALID_ROAD_TEMPLATE,
                        "Yeni tip kesitin adı (name) ve sol yanı (left) verilmeli.".into(),
                        if input.name.is_none() { "name" } else { "left" },
                    ));
                };
                let id = input.template.clone().unwrap_or_else(|| {
                    (1..).map(|n| format!("tk-{n}")).find(|id| own.iter().all(|t| &t.id != id)).unwrap_or_default()
                });
                list.push(TemplateDef {
                    id: id.clone(),
                    name: name.clone(),
                    revision: 1,
                    left: left.clone(),
                    right: if input.mirror { None } else { input.right.clone() },
                    pavement: if input.remove_pavement { None } else { input.pavement.clone() },
                });
                Some(id)
            }
        },
    };
    common::judged(&common::road_with(doc, Some(&list), None), codes::INVALID_ROAD_TEMPLATE, "Tip kesit")?;
    checks::revision(doc, input.expected_revision.as_deref())?;
    Ok(Checked { list, target })
}

fn write(cx: &mut ExecutionContext<'_>, c: Checked) -> CommandResult<TemplatesWritten> {
    let doc = &mut *cx.doc;
    if let Err(r) = doc.set_road_templates(c.list.clone(), STEP) {
        return fail(codes::INVALID_ROAD_TEMPLATE, format!("Tip kesit kurallarına uymuyor: {}.", r.0), "").into();
    }
    CommandResult::Completed {
        output: TemplatesWritten {
            template: c.target.as_ref().and_then(|id| c.list.iter().find(|t| &t.id == id)).cloned(),
            templates: common::listed_templates(doc, &c.list),
            revision: doc.revision().to_string(),
        },
        warnings: vec![],
    }
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &TemplatesEdit) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(_) => CommandResult::Completed { output: (), warnings: vec![] },
        Err(stop) => stop.into(),
    }
}

/// What execute would write now; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &TemplatesEdit) -> CommandResult<TemplatesWritten> {
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

/// Writes the template: one undo step.
pub fn execute(cx: &mut ExecutionContext<'_>, input: TemplatesEdit) -> CommandResult<TemplatesWritten> {
    crate::call::recorded(
        cx,
        kentos_commands::ROAD_TEMPLATES_EDIT,
        kentos_commands::ROAD_TEMPLATES_EDIT_VERSION,
        input,
        |cx, input| match check(cx.doc, &input) {
            Ok(c) => write(cx, c),
            Err(stop) => stop.into(),
        },
    )
}
