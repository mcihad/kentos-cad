//! `cad.styles.edit` v1 (docs/adr/0248 §4): one of the project's named text
//! or dimension styles (docs/adr/0183) created, updated, renamed or
//! removed. The table that results is saved as the Yazı stilleri and Ölçü
//! stilleri windows save theirs ([`crate::style_tables`]): the table is a
//! project setting (an edit, no undo step), the objects that follow are
//! written in one undo step “Yazı stili” or “Ölçü stili”, those on locked
//! layers left and counted. The shared cases are in
//! `fixtures/commands/v1/cad.styles.edit.json`.

use kentos_contracts::{
    AnnotationStyleKind, CommandResult, DimensionStyleDef, StyleEditOperation, StylesEdit,
    StylesEdited, TextStyleDef, dimension_styles_problem, text_styles_problem,
};
use kentos_domain::{Document, Uuid};

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;
use crate::style_tables;

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::Failed(error(code, message, Some(path.to_owned())))
}

/// One table, text or dimension, as the command changes it.
enum Table {
    Text(Vec<TextStyleDef>),
    Dimension(Vec<DimensionStyleDef>),
}

impl Table {
    fn ids(&self) -> Vec<&str> {
        match self {
            Table::Text(t) => t.iter().map(|s| s.id.as_str()).collect(),
            Table::Dimension(t) => t.iter().map(|s| s.id.as_str()).collect(),
        }
    }

    fn problem(&self) -> Option<String> {
        match self {
            Table::Text(t) => text_styles_problem(t),
            Table::Dimension(t) => dimension_styles_problem(t),
        }
    }
}

/// The table the input would leave, the style's id and the input's path of
/// its definition; or why not. A new style without an id is given `fresh`.
fn next_table(doc: &Document, input: &StylesEdit, fresh: &str) -> Result<(Table, String), Stop> {
    use StyleEditOperation as Op;
    let settings = doc.settings();
    let (path, what) = match input.kind {
        AnnotationStyleKind::Text => ("text", "yazı stili"),
        AnnotationStyleKind::Dimension => ("dimension", "ölçü stili"),
    };
    let mut table = match input.kind {
        AnnotationStyleKind::Text => Table::Text(settings.text_styles.clone()),
        AnnotationStyleKind::Dimension => Table::Dimension(settings.dimension_styles.clone()),
    };
    let not_found = |id: &str, at: &str| {
        fail(
            codes::STYLE_NOT_FOUND,
            format!("“{id}” kimlikli {what} projede yok."),
            at,
        )
    };
    let given_id = match input.operation {
        Op::Create | Op::Update => match input.kind {
            AnnotationStyleKind::Text => input.text.as_ref().map(|s| s.id.clone()),
            AnnotationStyleKind::Dimension => input.dimension.as_ref().map(|s| s.id.clone()),
        }
        .unwrap_or_default(),
        Op::Rename | Op::Remove => input.style.clone().unwrap_or_default(),
    };
    let known = table.ids().contains(&given_id.as_str());
    let id = match input.operation {
        Op::Create if given_id.is_empty() => fresh.to_owned(),
        Op::Create if known => {
            return Err(fail(
                codes::DUPLICATE_STYLE,
                format!(
                    "“{given_id}” kimlikli {what} zaten var; yeni stile başka bir kimlik verin ya da kimliği boş bırakın."
                ),
                &format!("{path}.id"),
            ));
        }
        Op::Update if !known => return Err(not_found(&given_id, &format!("{path}.id"))),
        Op::Rename | Op::Remove if !known => return Err(not_found(&given_id, "style")),
        _ => given_id,
    };
    let name = input.name.as_deref().unwrap_or_default().trim().to_owned();
    match (&mut table, input.operation) {
        (Table::Text(t), Op::Create) => {
            if let Some(mut s) = input.text.clone() {
                s.id.clone_from(&id);
                t.push(s);
            }
        }
        (Table::Dimension(t), Op::Create) => {
            if let Some(mut s) = input.dimension.clone() {
                s.id.clone_from(&id);
                t.push(s);
            }
        }
        (Table::Text(t), Op::Update) => {
            if let (Some(s), Some(at)) = (input.text.clone(), t.iter().position(|s| s.id == id)) {
                t[at] = s;
            }
        }
        (Table::Dimension(t), Op::Update) => {
            if let (Some(s), Some(at)) =
                (input.dimension.clone(), t.iter().position(|s| s.id == id))
            {
                t[at] = s;
            }
        }
        (Table::Text(t), Op::Rename) => {
            if let Some(s) = t.iter_mut().find(|s| s.id == id) {
                s.name.clone_from(&name);
            }
        }
        (Table::Dimension(t), Op::Rename) => {
            if let Some(s) = t.iter_mut().find(|s| s.id == id) {
                s.name.clone_from(&name);
            }
        }
        (Table::Text(t), Op::Remove) => t.retain(|s| s.id != id),
        (Table::Dimension(t), Op::Remove) => t.retain(|s| s.id != id),
    }
    if let Some(problem) = table.problem() {
        let at = match input.operation {
            Op::Create | Op::Update => path,
            Op::Rename => "name",
            Op::Remove => "style",
        };
        let kind = match input.kind {
            AnnotationStyleKind::Text => "Yazı stilleri",
            AnnotationStyleKind::Dimension => "Ölçü stilleri",
        };
        return Err(fail(
            codes::INVALID_STYLE,
            format!("{kind}: {problem}."),
            at,
        ));
    }
    Ok((table, id))
}

fn check_input(input: &StylesEdit) -> Result<(), Stop> {
    use StyleEditOperation as Op;
    let op = input.operation;
    let has = match input.kind {
        AnnotationStyleKind::Text => input.text.is_some(),
        AnnotationStyleKind::Dimension => input.dimension.is_some(),
    };
    if matches!(op, Op::Create | Op::Update) && !has {
        let path = match input.kind {
            AnnotationStyleKind::Text => "text",
            AnnotationStyleKind::Dimension => "dimension",
        };
        return Err(fail(
            codes::NO_DEFINITION,
            format!("Stilin tanımını {path} ile verin."),
            path,
        ));
    }
    if matches!(op, Op::Rename | Op::Remove) && input.style.is_none() {
        return Err(fail(
            codes::NO_STYLE,
            "Stili kimliğiyle verin (style).".into(),
            "style",
        ));
    }
    if op == Op::Rename && input.name.as_deref().is_none_or(checks::is_blank) {
        return Err(fail(
            codes::EMPTY_NAME,
            "Stilin adı boş olamaz; bir ad yazın.".into(),
            "name",
        ));
    }
    Ok(())
}

fn check(doc: &Document, input: &StylesEdit, fresh: &str) -> Result<(Table, String), Stop> {
    check_input(input)?;
    checks::revision(doc, input.expected_revision.as_deref())?;
    next_table(doc, input, fresh)
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &StylesEdit) -> CommandResult<()> {
    match check(cx.doc, input, &Uuid::now_v7().to_string()) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// What execute would do, on a reading copy; the drawing is left as it is.
/// A new style without an id is shown with an id of its own that execute
/// will not give again.
pub fn plan(cx: &ExecutionContext<'_>, input: &StylesEdit) -> CommandResult<StylesEdited> {
    let mut copy = cx.doc.reading_copy();
    match check(&copy, input, &Uuid::now_v7().to_string()) {
        Ok((table, id)) => {
            let mut output = write(&mut copy, table, id);
            output.revision = cx.doc.revision().to_string();
            CommandResult::Completed {
                output,
                warnings: Vec::new(),
            }
        }
        Err(stop) => stop.into(),
    }
}

/// Saves the table, then the objects that follow, in one undo step.
pub fn execute(cx: &mut ExecutionContext<'_>, input: StylesEdit) -> CommandResult<StylesEdited> {
    let fresh = Uuid::now_v7().to_string();
    match check(cx.doc, &input, &fresh) {
        Ok((table, id)) => CommandResult::Completed {
            output: write(cx.doc, table, id),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

fn write(doc: &mut Document, table: Table, id: String) -> StylesEdited {
    let before = doc.revision();
    let saved = match table {
        Table::Text(t) => style_tables::save_text_styles(doc, t),
        Table::Dimension(t) => style_tables::save_dimension_styles(doc, t),
    };
    StylesEdited {
        style: id,
        changed: doc.revision() != before,
        followed: count(saved.changed),
        locked: count(saved.locked),
        revision: doc.revision().to_string(),
    }
}

fn count(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}
