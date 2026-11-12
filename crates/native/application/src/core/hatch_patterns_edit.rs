//! `cad.hatchpatterns.edit` v1 (docs/adr/0331 §1): the project's own hatch patterns. A PAT file's patterns taken in
//! (`load`: its numbers in `unit`; a pattern of a name the project has kept, or replaced with `replace`; the file's
//! lines that do not read said), one pattern added or replaced (`add`), one taken away (`remove`: a hatch carries its
//! pattern's definition, so none changes), the patterns written as a PAT file's text (`export`, writing nothing). The
//! table is a project setting, as the line types' (docs/adr/0313): writing it makes the drawing dirty, it is no undo
//! step. The shared cases are `fixtures/commands/v1/cad.hatchpatterns.edit.json`
//! (`scripts/fixtures/hatch_pattern_file_cases.py`).

use kentos_commands::{
    CommandResult, HatchPatternDef, HatchPatternsEdit, HatchPatternsEdited, HatchPatternsOperation,
    PatLineProblem, hatch_patterns_problem, same_hatch_pattern_name,
};
use kentos_domain::Document;

use crate::checks::{self, Stop, error};
use crate::{ExecutionContext, codes};

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.to_owned())))
}

fn find<'a>(defs: &'a [HatchPatternDef], name: &str) -> Option<&'a HatchPatternDef> {
    defs.iter().find(|d| same_hatch_pattern_name(&d.name, name))
}

fn check(doc: &Document, input: &HatchPatternsEdit) -> Result<(), Stop> {
    let defs = &doc.settings().hatch_patterns;
    match input.operation {
        HatchPatternsOperation::Load => {
            if input.pat.as_deref().is_none_or(|t| t.trim().is_empty()) {
                return Err(fail(
                    codes::NO_PAT,
                    "Yüklenecek PAT dosyasının metnini verin (pat).".into(),
                    "pat",
                ));
            }
        }
        HatchPatternsOperation::Add => {
            let Some(d) = &input.pattern else {
                return Err(fail(
                    codes::NO_DEFINITION,
                    "Eklenecek tarama desenini verin (pattern).".into(),
                    "pattern",
                ));
            };
            if let Some(problem) = hatch_patterns_problem(std::slice::from_ref(d)) {
                return Err(fail(
                    codes::INVALID_HATCH_PATTERN,
                    format!("Tarama deseni geçersiz: {problem}."),
                    "pattern",
                ));
            }
            if !input.replace && find(defs, &d.name).is_some() {
                return Err(fail(
                    codes::INVALID_HATCH_PATTERN,
                    format!(
                        "“{}” adlı tarama deseni projede var. Değiştirmek için replace verin ya da başka bir ad seçin.",
                        d.name
                    ),
                    "pattern",
                ));
            }
        }
        HatchPatternsOperation::Remove => {
            let Some(name) = input.name.as_deref().filter(|n| !checks::is_blank(n)) else {
                return Err(fail(
                    codes::EMPTY_NAME,
                    "Silinecek tarama deseninin adını verin (name).".into(),
                    "name",
                ));
            };
            if find(defs, name).is_none() {
                return Err(fail(
                    codes::HATCH_PATTERN_NOT_FOUND,
                    format!("“{name}” adlı tarama deseni projede yok."),
                    "name",
                ));
            }
        }
        HatchPatternsOperation::Export => {
            if let Some(missing) = input
                .names
                .iter()
                .flatten()
                .find(|n| find(defs, n).is_none())
            {
                return Err(fail(
                    codes::HATCH_PATTERN_NOT_FOUND,
                    format!("“{missing}” adlı tarama deseni projede yok."),
                    "names",
                ));
            }
        }
    }
    checks::revision(doc, input.expected_revision.as_deref())
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &HatchPatternsEdit) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(()) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// What the edit would do: the same write on a reading copy; the drawing stays.
pub fn plan(
    cx: &ExecutionContext<'_>,
    input: &HatchPatternsEdit,
) -> CommandResult<HatchPatternsEdited> {
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

/// Edits the project's hatch patterns.
pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: HatchPatternsEdit,
) -> CommandResult<HatchPatternsEdited> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_HATCH_PATTERNS_EDIT,
        kentos_commands::CAD_HATCH_PATTERNS_EDIT_VERSION,
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

fn write(doc: &mut Document, input: &HatchPatternsEdit) -> HatchPatternsEdited {
    let mut settings = doc.settings().clone();
    let mut out = HatchPatternsEdited {
        changed: false,
        added: Vec::new(),
        replaced: Vec::new(),
        skipped: Vec::new(),
        removed: Vec::new(),
        problems: Vec::new(),
        pat: None,
        revision: String::new(),
    };
    let put = |settings: &mut kentos_commands::ProjectSettings,
               d: HatchPatternDef,
               replace: bool,
               out: &mut HatchPatternsEdited| {
        match settings
            .hatch_patterns
            .iter()
            .position(|p| same_hatch_pattern_name(&p.name, &d.name))
        {
            Some(at) if replace => {
                out.replaced.push(d.name.clone());
                settings.hatch_patterns[at] = d;
            }
            Some(_) => out.skipped.push(d.name),
            None => {
                out.added.push(d.name.clone());
                settings.hatch_patterns.push(d);
            }
        }
    };
    match input.operation {
        HatchPatternsOperation::Load => {
            let read = kentos_formats::pat::read_pat(
                input.pat.as_deref().unwrap_or_default(),
                input.unit.unwrap_or_default(),
                input.source.as_deref().unwrap_or("dosya.pat"),
            );
            out.problems = read
                .problems
                .into_iter()
                .map(|p| PatLineProblem {
                    line: p.line,
                    message: p.message,
                })
                .collect();
            for name in input.names.iter().flatten() {
                if find(&read.patterns, name).is_none() {
                    out.problems.push(PatLineProblem {
                        line: 0,
                        message: format!("“{name}” dosyada yok"),
                    });
                }
            }
            for d in read.patterns {
                let wanted = input
                    .names
                    .as_ref()
                    .is_none_or(|names| names.iter().any(|n| same_hatch_pattern_name(n, &d.name)));
                if wanted {
                    put(&mut settings, d, input.replace, &mut out);
                }
            }
        }
        HatchPatternsOperation::Add => {
            if let Some(d) = input.pattern.clone() {
                put(&mut settings, d, true, &mut out);
            }
        }
        HatchPatternsOperation::Remove => {
            if let Some(name) = input.name.as_deref() {
                let before = settings.hatch_patterns.len();
                settings
                    .hatch_patterns
                    .retain(|d| !same_hatch_pattern_name(&d.name, name));
                if settings.hatch_patterns.len() < before {
                    out.removed.push(name.trim().to_owned());
                }
            }
        }
        HatchPatternsOperation::Export => {
            let chosen: Vec<HatchPatternDef> = settings
                .hatch_patterns
                .iter()
                .filter(|d| {
                    input.names.as_ref().is_none_or(|names| {
                        names.iter().any(|n| same_hatch_pattern_name(n, &d.name))
                    })
                })
                .cloned()
                .collect();
            out.pat = Some(kentos_formats::pat::write_pat(&chosen));
        }
    }
    if settings != *doc.settings() {
        out.changed = true;
        doc.set_settings(settings);
    }
    out.revision = doc.revision().to_string();
    out
}
