//! `cad.relations.edit` v1 (docs/adr/0405 §3): the project's relations. A project setting
//! (`Document::set_settings`: an edit, no undo step), as the forms are. The shared cases are
//! `fixtures/commands/v1/cad.relations.edit.json`.

use kentos_contracts::{
    CommandResult, LayerNodeType, MOST_RELATIONS, ProjectRelation, ProjectSettings, RelationsEdit,
    RelationsEdited, RelationsOp, form_id_ok, relation_problem,
};
use kentos_domain::Document;

use crate::ExecutionContext;
use crate::checks::{self, Stop};
use crate::codes;
use crate::forms::fail;

/// The input's own checks: what needs no document.
fn check_input(input: &RelationsEdit) -> Result<(), Stop> {
    match &input.edit {
        RelationsOp::Create { relation } | RelationsOp::Update { relation } => {
            if let Some(problem) = relation_problem(relation) {
                return Err(fail(
                    codes::INVALID_RELATION,
                    format!("İlişki kurallarına uymuyor: {problem}"),
                    "edit.relation",
                ));
            }
        }
        RelationsOp::Remove { id } => {
            if !form_id_ok(id) {
                return Err(fail(
                    codes::INVALID_RELATION,
                    format!(
                        "“{id}” bir ilişki kimliği olamaz: 1–64 harf, rakam, -, _ ya da . olmalı."
                    ),
                    "edit.id",
                ));
            }
        }
    }
    Ok(())
}

/// The relation's layers and field in the drawing.
fn check_layers(doc: &Document, r: &ProjectRelation) -> Result<(), Stop> {
    for (id, path) in [
        (&r.parent, "edit.relation.parent"),
        (&r.child, "edit.relation.child"),
    ] {
        let Some(node) = doc.layers().get(id) else {
            return Err(fail(
                codes::LAYER_NOT_FOUND,
                format!(
                    "“{id}” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin."
                ),
                path,
            ));
        };
        if node.kind != LayerNodeType::Layer {
            return Err(fail(
                codes::NOT_A_LAYER,
                format!(
                    "“{}” bir katman grubu; ilişki katmanlar arasında olur. Grubun içinden bir katman seçin.",
                    node.name
                ),
                path,
            ));
        }
    }
    let child = doc.layers().get(&r.child);
    if !child.is_some_and(|n| n.fields.iter().any(|f| f.key() == r.field)) {
        return Err(fail(
            codes::RELATION_FIELD_UNKNOWN,
            format!(
                "“{}” katmanında “{}” alanı yok. Alt katmanın, üst nesnenin kimliğini tutacak alanını seçin.",
                child.map_or(r.child.as_str(), |n| n.name.as_str()),
                r.field
            ),
            "edit.relation.field",
        ));
    }
    Ok(())
}

fn check(doc: &Document, input: &RelationsEdit) -> Result<ProjectSettings, Stop> {
    check_input(input)?;
    checks::revision(doc, input.expected_revision.as_deref())?;
    let mut s = doc.settings().clone();
    let at = |s: &ProjectSettings, id: &str| s.relations.iter().position(|r| r.id == id);
    let unknown = |id: &str, path: &str| {
        fail(
            codes::UNKNOWN_RELATION,
            format!("“{id}” kimlikli bir ilişki yok. Var olan bir ilişkinin kimliğini verin."),
            path,
        )
    };
    match &input.edit {
        RelationsOp::Create { relation } => {
            if at(&s, &relation.id).is_some() {
                return Err(fail(
                    codes::RELATION_EXISTS,
                    format!(
                        "“{}” kimlikli başka bir ilişki var. Başka bir kimlik verin.",
                        relation.id
                    ),
                    "edit.relation.id",
                ));
            }
            if s.relations.len() >= MOST_RELATIONS {
                return Err(fail(
                    codes::TOO_MANY_RELATIONS,
                    "Projede en çok 100 ilişki olur. Kullanmadığınız birini silin.".into(),
                    "edit",
                ));
            }
            check_layers(doc, relation)?;
            s.relations.push(relation.clone());
        }
        RelationsOp::Update { relation } => {
            let Some(i) = at(&s, &relation.id) else {
                return Err(unknown(&relation.id, "edit.relation.id"));
            };
            check_layers(doc, relation)?;
            s.relations[i] = relation.clone();
        }
        RelationsOp::Remove { id } => {
            let Some(i) = at(&s, id) else {
                return Err(unknown(id, "edit.id"));
            };
            s.relations.remove(i);
        }
    }
    Ok(s)
}

fn answer(s: &ProjectSettings, changed: bool, revision: String) -> RelationsEdited {
    RelationsEdited {
        relations: s.relations.clone(),
        changed,
        revision,
    }
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &RelationsEdit) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// What execute would do; nothing is written.
pub fn plan(cx: &ExecutionContext<'_>, input: &RelationsEdit) -> CommandResult<RelationsEdited> {
    match check(cx.doc, input) {
        Ok(next) => {
            let changed = next != *cx.doc.settings();
            CommandResult::Completed {
                output: answer(&next, changed, cx.doc.revision().to_string()),
                warnings: Vec::new(),
            }
        }
        Err(stop) => stop.into(),
    }
}

/// Checks `input` again and writes the settings.
pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: RelationsEdit,
) -> CommandResult<RelationsEdited> {
    crate::call::recorded(
        cx,
        kentos_contracts::CAD_RELATIONS_EDIT,
        kentos_contracts::CAD_RELATIONS_EDIT_VERSION,
        input,
        write_execute,
    )
}

fn write_execute(
    cx: &mut ExecutionContext<'_>,
    input: RelationsEdit,
) -> CommandResult<RelationsEdited> {
    let next = match check(cx.doc, &input) {
        Ok(next) => next,
        Err(stop) => return stop.into(),
    };
    let changed = next != *cx.doc.settings();
    if changed {
        cx.doc.set_settings(next);
    }
    let settings = cx.doc.settings().clone();
    CommandResult::Completed {
        output: answer(&settings, changed, cx.doc.revision().to_string()),
        warnings: Vec::new(),
    }
}
