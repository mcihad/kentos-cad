//! `cad.joins.edit` v1 (docs/adr/0527): the project's join views. A project setting (`Document::set_settings`:
//! an edit, no undo step), as the relations are. The shared cases are `fixtures/commands/v1/cad.joins.edit.json`.
//!
//! The checks, in order: the view by its shape (`join_view_problem`); the revision; the id (unknown, taken),
//! the count; the relation a view is made of; each layer the drawing's and a layer; each key and field the
//! layer's (`$uid` always; a layer without typed fields takes any attribute).

use kentos_commands::{
    CommandResult, JoinView, JoinsEdit, JoinsEdited, JoinsOp, LayerNodeType, MOST_JOIN_VIEWS,
    ProjectSettings, UID_KEY, form_id_ok, join_view_problem,
};
use kentos_domain::Document;

use crate::ExecutionContext;
use crate::checks::{self, Stop};
use crate::codes;
use crate::forms::fail;

/// The view an op writes, made of a relation for `fromRelation`.
fn view_of(doc: &Document, op: &JoinsOp) -> Result<Option<JoinView>, Stop> {
    match op {
        JoinsOp::Create { view } | JoinsOp::Update { view } => Ok(Some(view.clone())),
        JoinsOp::FromRelation {
            relation,
            id,
            name,
            keep,
        } => {
            let Some(r) = doc.settings().relations.iter().find(|r| r.id == *relation) else {
                return Err(fail(
                    codes::UNKNOWN_RELATION,
                    format!(
                        "“{relation}” kimlikli bir ilişki yok. Var olan bir ilişkinin kimliğini verin."
                    ),
                    "edit.relation",
                ));
            };
            Ok(Some(JoinView {
                id: id.clone(),
                name: name.clone(),
                layer: r.parent.clone(),
                key: UID_KEY.into(),
                with: r.child.clone(),
                with_key: r.field.clone(),
                keep: *keep,
                fields: Vec::new(),
            }))
        }
        JoinsOp::Remove { .. } => Ok(None),
    }
}

fn check_shape(op: &JoinsOp) -> Result<(), Stop> {
    let (problem, path) = match op {
        JoinsOp::Create { view } | JoinsOp::Update { view } => {
            (join_view_problem(view), "edit.view")
        }
        JoinsOp::FromRelation { id, name, .. } => {
            let probe = JoinView {
                id: id.clone(),
                name: name.clone(),
                layer: "-".into(),
                key: "-".into(),
                with: "-".into(),
                with_key: "-".into(),
                keep: Default::default(),
                fields: Vec::new(),
            };
            (join_view_problem(&probe), "edit")
        }
        JoinsOp::Remove { id } => (
            (!form_id_ok(id)).then(|| {
                format!("“{id}” bir görünüm kimliği olamaz: 1–64 harf, rakam, -, _ ya da . olmalı.")
            }),
            "edit.id",
        ),
    };
    match problem {
        Some(p) => Err(fail(
            codes::INVALID_JOIN_VIEW,
            format!("Birleşik görünüm kurallarına uymuyor: {p}"),
            path,
        )),
        None => Ok(()),
    }
}

/// The view's layers, keys and fields in the drawing.
fn check_drawing(doc: &Document, v: &JoinView, path: &str) -> Result<(), Stop> {
    let sides = [
        (&v.layer, &v.key, "layer", "key"),
        (&v.with, &v.with_key, "with", "withKey"),
    ];
    for (id, key, at, key_at) in sides {
        let Some(node) = doc.layers().get(id) else {
            return Err(fail(
                codes::LAYER_NOT_FOUND,
                format!(
                    "“{id}” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin."
                ),
                &format!("{path}.{at}"),
            ));
        };
        if node.kind != LayerNodeType::Layer {
            return Err(fail(
                codes::NOT_A_LAYER,
                format!(
                    "“{}” bir katman grubu; görünüm katmanlar arasında olur. Grubun içinden bir katman seçin.",
                    node.name
                ),
                &format!("{path}.{at}"),
            ));
        }
        let known = |k: &str| node.fields.is_empty() || node.fields.iter().any(|f| f.key() == k);
        if key != UID_KEY && !known(key) {
            return Err(fail(
                codes::FIELD_NOT_FOUND,
                format!(
                    "“{}” katmanında “{key}” alanı yok. Katmanın bir alanını ya da nesnenin kimliği için $uid'i verin.",
                    node.name
                ),
                &format!("{path}.{key_at}"),
            ));
        }
        if at == "with"
            && let Some((i, f)) = v.fields.iter().enumerate().find(|(_, f)| !known(f))
        {
            return Err(fail(
                codes::FIELD_NOT_FOUND,
                format!(
                    "“{}” katmanında “{f}” alanı yok. Gösterilecek alanları katmanın alanlarından seçin.",
                    node.name
                ),
                &format!("{path}.fields.{i}"),
            ));
        }
    }
    Ok(())
}

fn check(doc: &Document, input: &JoinsEdit) -> Result<ProjectSettings, Stop> {
    check_shape(&input.edit)?;
    checks::revision(doc, input.expected_revision.as_deref())?;
    let mut s = doc.settings().clone();
    let at = |s: &ProjectSettings, id: &str| s.join_views.iter().position(|v| v.id == id);
    let path = match &input.edit {
        JoinsOp::FromRelation { .. } => "edit",
        _ => "edit.view",
    };
    match &input.edit {
        JoinsOp::Create {
            view: JoinView { id, .. },
        }
        | JoinsOp::FromRelation { id, .. } => {
            if at(&s, id).is_some() {
                return Err(fail(
                    codes::JOIN_VIEW_EXISTS,
                    format!(
                        "“{id}” kimlikli başka bir birleşik görünüm var. Başka bir kimlik verin."
                    ),
                    &format!("{path}.id"),
                ));
            }
            if s.join_views.len() >= MOST_JOIN_VIEWS {
                return Err(fail(
                    codes::TOO_MANY_JOIN_VIEWS,
                    format!(
                        "Projede en çok {MOST_JOIN_VIEWS} birleşik görünüm olur. Kullanmadığınız birini silin."
                    ),
                    "edit",
                ));
            }
            if let Some(v) = view_of(doc, &input.edit)? {
                check_drawing(doc, &v, path)?;
                s.join_views.push(v);
            }
        }
        JoinsOp::Update { view } => {
            let Some(i) = at(&s, &view.id) else {
                return Err(unknown(&view.id, "edit.view.id"));
            };
            check_drawing(doc, view, path)?;
            s.join_views[i] = view.clone();
        }
        JoinsOp::Remove { id } => {
            let Some(i) = at(&s, id) else {
                return Err(unknown(id, "edit.id"));
            };
            s.join_views.remove(i);
        }
    }
    Ok(s)
}

fn unknown(id: &str, path: &str) -> Stop {
    fail(
        codes::UNKNOWN_JOIN_VIEW,
        format!(
            "“{id}” kimlikli bir birleşik görünüm yok. Var olan bir görünümün kimliğini verin."
        ),
        path,
    )
}

fn answer(s: &ProjectSettings, changed: bool, revision: String) -> JoinsEdited {
    JoinsEdited {
        views: s.join_views.clone(),
        changed,
        revision,
    }
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &JoinsEdit) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// What execute would do; nothing is written.
pub fn plan(cx: &ExecutionContext<'_>, input: &JoinsEdit) -> CommandResult<JoinsEdited> {
    match check(cx.doc, input) {
        Ok(next) => CommandResult::Completed {
            output: answer(
                &next,
                next != *cx.doc.settings(),
                cx.doc.revision().to_string(),
            ),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// Checks `input` again and writes the settings.
pub fn execute(cx: &mut ExecutionContext<'_>, input: JoinsEdit) -> CommandResult<JoinsEdited> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_JOINS_EDIT,
        kentos_commands::CAD_JOINS_EDIT_VERSION,
        input,
        write_execute,
    )
}

fn write_execute(cx: &mut ExecutionContext<'_>, input: JoinsEdit) -> CommandResult<JoinsEdited> {
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
