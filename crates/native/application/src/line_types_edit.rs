//! `cad.linetypes.edit` v1 (docs/adr/0313 §2): the project's line types with patterns. A LIN file's types taken in
//! (`load`: a type of a name the project has kept, or replaced with `replace`; the file's lines that do not read said),
//! one type added or replaced (`add`), one taken away when nothing draws with it (`remove`), the project's line type
//! scale set (`scale`, AutoCAD's LTSCALE), the types written as a LIN file's text (`export`, writing nothing). The
//! table is a project setting, as the text styles' (docs/adr/0183): writing it makes the drawing dirty, it is no undo
//! step. The shared cases are `fixtures/commands/v1/cad.linetypes.edit.json`
//! (`scripts/fixtures/line_type_cases.py`).

use kentos_commands::{
    CommandResult, LayerNode, LinLineProblem, LineTypeDef, LineTypesEdit, LineTypesEdited,
    LineTypesOperation, line_types_problem, same_line_type_name,
};
use kentos_domain::Document;

use crate::checks::{self, Stop, error};
use crate::{ExecutionContext, codes};

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.to_owned())))
}

fn find<'a>(types: &'a [LineTypeDef], name: &str) -> Option<&'a LineTypeDef> {
    types.iter().find(|t| same_line_type_name(&t.name, name))
}

/// How many layers, layer states' nodes and objects (the drawing's and its blocks') draw with `name`.
fn uses(doc: &Document, name: &str) -> (usize, usize, usize) {
    fn layers(nodes: &[LayerNode], name: &str) -> usize {
        nodes
            .iter()
            .map(|n| {
                usize::from(
                    n.style
                        .line_pattern
                        .as_deref()
                        .is_some_and(|p| same_line_type_name(p, name)),
                ) + layers(&n.children, name)
            })
            .sum()
    }
    let named = |p: Option<&str>| p.is_some_and(|p| same_line_type_name(p, name));
    let states = doc
        .settings()
        .layer_states
        .iter()
        .flat_map(|s| &s.nodes)
        .filter(|n| named(n.style.as_ref().and_then(|s| s.line_pattern.as_deref())))
        .count();
    let objects = doc
        .entities()
        .filter(|e| named(e.base().look.line_pattern.as_deref()))
        .count()
        + doc
            .blocks()
            .iter()
            .flat_map(|b| &b.entities)
            .filter(|e| named(e.base().look.line_pattern.as_deref()))
            .count();
    (layers(doc.layers().nodes(), name), states, objects)
}

fn check(doc: &Document, input: &LineTypesEdit) -> Result<(), Stop> {
    let types = &doc.settings().line_types;
    match input.operation {
        LineTypesOperation::Load => {
            if input.lin.as_deref().is_none_or(|t| t.trim().is_empty()) {
                return Err(fail(
                    codes::NO_LIN,
                    "Yüklenecek LIN dosyasının metnini verin (lin).".into(),
                    "lin",
                ));
            }
        }
        LineTypesOperation::Add => {
            let Some(t) = &input.line_type else {
                return Err(fail(
                    codes::NO_DEFINITION,
                    "Eklenecek çizgi tipini verin (lineType).".into(),
                    "lineType",
                ));
            };
            if let Some(problem) = line_types_problem(std::slice::from_ref(t)) {
                return Err(fail(
                    codes::INVALID_LINE_TYPE,
                    format!("Çizgi tipi geçersiz: {problem}."),
                    "lineType",
                ));
            }
            if !input.replace && find(types, &t.name).is_some() {
                return Err(fail(
                    codes::INVALID_LINE_TYPE,
                    format!(
                        "“{}” adlı çizgi tipi projede var. Değiştirmek için replace verin ya da başka bir ad seçin.",
                        t.name
                    ),
                    "lineType",
                ));
            }
        }
        LineTypesOperation::Remove => {
            let Some(name) = input.name.as_deref().filter(|n| !checks::is_blank(n)) else {
                return Err(fail(
                    codes::EMPTY_NAME,
                    "Silinecek çizgi tipinin adını verin (name).".into(),
                    "name",
                ));
            };
            if find(types, name).is_none() {
                return Err(fail(
                    codes::LINE_TYPE_NOT_FOUND,
                    format!("“{name}” adlı çizgi tipi projede yok."),
                    "name",
                ));
            }
            let (layers, states, objects) = uses(doc, name);
            if layers + states + objects > 0 {
                return Err(fail(
                    codes::LINE_TYPE_IN_USE,
                    format!(
                        "“{name}” çizgi tipi {layers} katmanda, {states} katman durumunda ve {objects} nesnede kullanılıyor; önce onları başka bir çizgi tipine geçirin."
                    ),
                    "name",
                ));
            }
        }
        LineTypesOperation::Scale => {
            if !input.scale.is_some_and(|s| {
                s.is_finite() && s > 0.0 && s <= kentos_commands::MAX_LINE_TYPE_SCALE
            }) {
                return Err(fail(
                    codes::INVALID_LINE_TYPE_SCALE,
                    "Çizgi tipi ölçeği 0'dan büyük, en çok 1000 olmalı (scale); 1 ölçeksizdir."
                        .into(),
                    "scale",
                ));
            }
        }
        LineTypesOperation::Export => {
            if let Some(missing) = input
                .names
                .iter()
                .flatten()
                .find(|n| find(types, n).is_none())
            {
                return Err(fail(
                    codes::LINE_TYPE_NOT_FOUND,
                    format!("“{missing}” adlı çizgi tipi projede yok."),
                    "names",
                ));
            }
        }
    }
    checks::revision(doc, input.expected_revision.as_deref())
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &LineTypesEdit) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(()) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// What the edit would do: the same write on a reading copy; the drawing stays.
pub fn plan(cx: &ExecutionContext<'_>, input: &LineTypesEdit) -> CommandResult<LineTypesEdited> {
    if let Err(stop) = check(cx.doc, input) {
        return stop.into();
    }
    let mut copy = cx.doc.reading_copy();
    let mut out = write(&mut copy, input);
    out.revision = cx.doc.revision().to_string();
    CommandResult::Completed {
        output: out,
        warnings: Vec::new(),
    }
}

/// Edits the project's line types.
pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: LineTypesEdit,
) -> CommandResult<LineTypesEdited> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_LINE_TYPES_EDIT,
        kentos_commands::CAD_LINE_TYPES_EDIT_VERSION,
        input,
        |cx, input| {
            if let Err(stop) = check(cx.doc, &input) {
                return stop.into();
            }
            CommandResult::Completed {
                output: write(cx.doc, &input),
                warnings: Vec::new(),
            }
        },
    )
}

fn write(doc: &mut Document, input: &LineTypesEdit) -> LineTypesEdited {
    let mut settings = doc.settings().clone();
    let mut out = LineTypesEdited {
        changed: false,
        added: Vec::new(),
        replaced: Vec::new(),
        skipped: Vec::new(),
        removed: Vec::new(),
        problems: Vec::new(),
        lin: None,
        scale: settings.line_type_scale,
        revision: String::new(),
    };
    let put = |settings: &mut kentos_commands::ProjectSettings,
               t: LineTypeDef,
               replace: bool,
               out: &mut LineTypesEdited| {
        match settings
            .line_types
            .iter()
            .position(|p| same_line_type_name(&p.name, &t.name))
        {
            Some(at) if replace => {
                out.replaced.push(t.name.clone());
                settings.line_types[at] = t;
            }
            Some(_) => out.skipped.push(t.name),
            None => {
                out.added.push(t.name.clone());
                settings.line_types.push(t);
            }
        }
    };
    match input.operation {
        LineTypesOperation::Load => {
            let read = kentos_formats::lin::read_lin(
                input.lin.as_deref().unwrap_or_default(),
                input.source.as_deref().unwrap_or("dosya.lin"),
            );
            out.problems = read
                .problems
                .into_iter()
                .map(|p| LinLineProblem {
                    line: p.line,
                    message: p.message,
                })
                .collect();
            for name in input.names.iter().flatten() {
                if find(&read.types, name).is_none() {
                    out.problems.push(LinLineProblem {
                        line: 0,
                        message: format!("“{name}” dosyada yok"),
                    });
                }
            }
            for t in read.types {
                let wanted = input
                    .names
                    .as_ref()
                    .is_none_or(|names| names.iter().any(|n| same_line_type_name(n, &t.name)));
                if wanted {
                    put(&mut settings, t, input.replace, &mut out);
                }
            }
        }
        LineTypesOperation::Add => {
            if let Some(t) = input.line_type.clone() {
                put(&mut settings, t, true, &mut out);
            }
        }
        LineTypesOperation::Remove => {
            if let Some(name) = input.name.as_deref() {
                let before = settings.line_types.len();
                settings
                    .line_types
                    .retain(|t| !same_line_type_name(&t.name, name));
                if settings.line_types.len() < before {
                    out.removed.push(name.trim().to_owned());
                }
            }
        }
        LineTypesOperation::Scale => {
            settings.line_type_scale = input.scale.filter(|s| *s != 1.0);
        }
        LineTypesOperation::Export => {
            let chosen: Vec<LineTypeDef> = settings
                .line_types
                .iter()
                .filter(|t| {
                    input
                        .names
                        .as_ref()
                        .is_none_or(|names| names.iter().any(|n| same_line_type_name(n, &t.name)))
                })
                .cloned()
                .collect();
            out.lin = Some(kentos_formats::lin::write_lin(&chosen));
        }
    }
    out.scale = settings.line_type_scale;
    if settings != *doc.settings() {
        out.changed = true;
        doc.set_settings(settings);
    }
    out.revision = doc.revision().to_string();
    out
}
