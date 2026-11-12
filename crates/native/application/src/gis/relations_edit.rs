//! `cad.relations.edit` v1 (docs/adr/0405 §3): the project's relations. A project setting
//! (`Document::set_settings`: an edit, no undo step), as the forms are. The shared cases are
//! `fixtures/commands/v1/cad.relations.edit.json`.

use kentos_commands::{
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

/// The children of the child layer `r` names whose field holds a link (docs/adr/0440 §3).
pub(crate) fn links_of(doc: &Document, r: &ProjectRelation) -> u32 {
    let Some(name) = doc
        .layers()
        .get(&r.child)
        .and_then(|n| n.fields.iter().find(|f| f.key() == r.field))
        .map(|f| f.name.clone())
    else {
        return 0;
    };
    doc.by_layer(&r.child)
        .filter(|e| {
            e.base()
                .attrs
                .get(&name)
                .is_some_and(|v| !v.trim().is_empty())
        })
        .count() as u32
}

/// The most layers a chain of compositions crosses (docs/adr/0440 §3).
pub const MOST_CASCADE_DEPTH: usize = 8;

/// Compositions that delete each other in a circle through other layers, or a chain of them deeper
/// than [`MOST_CASCADE_DEPTH`] (a layer's own composition, a tree of its records, is neither): the
/// first such relation's name and why.
pub(crate) fn cascade_problem(relations: &[ProjectRelation]) -> Option<String> {
    use kentos_commands::RelationDelete;
    let edges: Vec<(&str, &str, &str)> = relations
        .iter()
        .filter(|r| r.on_delete == RelationDelete::Cascade && r.parent != r.child)
        .map(|r| (r.parent.as_str(), r.child.as_str(), r.name.as_str()))
        .collect();
    // The longest path from each layer, a circle found on the way.
    fn walk<'a>(
        at: &'a str,
        edges: &[(&'a str, &'a str, &'a str)],
        path: &mut Vec<&'a str>,
    ) -> Result<usize, &'a str> {
        let mut deepest = 0;
        for (from, to, name) in edges.iter().filter(|(f, ..)| *f == at) {
            let _ = from;
            if path.contains(to) {
                return Err(name);
            }
            path.push(to);
            let d = walk(to, edges, path)?;
            path.pop();
            deepest = deepest.max(d + 1);
        }
        Ok(deepest)
    }
    for (from, ..) in &edges {
        let mut path = vec![*from];
        match walk(from, &edges, &mut path) {
            Err(name) => {
                return Some(format!(
                    "“{name}” bileşik ilişkisi döngü kuruyor: üst kayıt silinince alt kayıtları onu yeniden silerdi. Döngüdeki bir ilişkinin silme kuralını değiştirin."
                ));
            }
            Ok(d) if d > MOST_CASCADE_DEPTH => {
                return Some(format!(
                    "Bileşik ilişkilerin zinciri {d} katman derinliğinde; en çok {MOST_CASCADE_DEPTH}. Zincirdeki bir ilişkinin silme kuralını değiştirin."
                ));
            }
            Ok(_) => {}
        }
    }
    None
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
            // A relation's key changed while children hold its links (docs/adr/0440 §3): the links would
            // be lost or misread.
            let old = &s.relations[i];
            if (old.child != relation.child || old.field != relation.field)
                && let n @ 1.. = links_of(doc, old)
            {
                return Err(fail(
                    codes::RELATION_IN_USE,
                    format!(
                        "{n} alt kayıt “{}” ilişkisinin bağını eski alanında tutuyor; alt katmanı ya da alanı değişince bağlar kaybolur. Önce değerleri yeni alana taşıyın (Veri aktar, Öznitelik hesapla) ya da bağları koparın.",
                        old.name
                    ),
                    "edit.relation.field",
                ));
            }
            s.relations[i] = relation.clone();
        }
        RelationsOp::Remove { id } => {
            let Some(i) = at(&s, id) else {
                return Err(unknown(id, "edit.id"));
            };
            s.relations.remove(i);
        }
    }
    if let Some(problem) = cascade_problem(&s.relations) {
        return Err(fail(codes::RELATION_CYCLE, problem, "edit"));
    }
    Ok(s)
}

/// The links the edit's relation holds: the edited or removed one's as it is, a new one's as given.
fn links_for(doc: &Document, input: &RelationsEdit) -> u32 {
    match &input.edit {
        RelationsOp::Create { relation } => links_of(doc, relation),
        RelationsOp::Update { relation } => doc
            .settings()
            .relations
            .iter()
            .find(|r| r.id == relation.id)
            .map_or(0, |r| links_of(doc, r)),
        RelationsOp::Remove { id } => doc
            .settings()
            .relations
            .iter()
            .find(|r| r.id == *id)
            .map_or(0, |r| links_of(doc, r)),
    }
}

fn answer(s: &ProjectSettings, changed: bool, links: u32, revision: String) -> RelationsEdited {
    RelationsEdited {
        relations: s.relations.clone(),
        changed,
        links,
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
                output: answer(
                    &next,
                    changed,
                    links_for(cx.doc, input),
                    cx.doc.revision().to_string(),
                ),
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
        kentos_commands::CAD_RELATIONS_EDIT,
        kentos_commands::CAD_RELATIONS_EDIT_VERSION,
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
    let links = links_for(cx.doc, &input);
    if changed {
        cx.doc.set_settings(next);
    }
    let settings = cx.doc.settings().clone();
    CommandResult::Completed {
        output: answer(&settings, changed, links, cx.doc.revision().to_string()),
        warnings: Vec::new(),
    }
}
