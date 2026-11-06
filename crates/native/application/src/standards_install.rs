//! `cad.standards.install` v1 (Standart kaynaklarını kur, docs/adr/1002): the bound office standards' named
//! resources (line types, hatch patterns, text, dimension, leader and table styles; `cad.standards.resolve`'s first-source
//! answer) installed into the drawing by the user's choices. Each resource is `missing`, `same` or `differs` against
//! the drawing's of that name (case aside, appearance compared as `cad.standards.check` does: names, ids,
//! descriptions and sources aside, numbers within 1e-9). A missing one is added by default; a differing one is kept by
//! default and never overwritten without the choice `replace` (in place: the drawing's name and local id kept, so what
//! names it follows) or `renameIncoming` (the standard's added under a new name). Nothing is removed or purged. One
//! undo step “Standart kaynaklarını kur”; the plan is the same answer, written nowhere. The shared cases are
//! `fixtures/commands/v1/cad.standards.install.json` (`scripts/fixtures/standards_install_cases.py`).
//!
//! Block library references and approved sheet template revisions (docs/adr/1100) are rows of the same table,
//! resolved against the libraries KentOS reads (`standards_install_library`, `standard_libraries`); their cases are
//! `cad.standards.install.library.json` (`scripts/fixtures/standards_install_library_cases.py`).

use std::collections::BTreeSet;

use kentos_commands::{
    CommandResult, StandardInstallAction as Action, StandardInstallRow, StandardOrigin, StandardResourceKind as Kind,
    StandardsInstall, StandardsInstalled, StandardsResolved, STANDARDS_INSTALL_STEP,
};
use kentos_domain::{Document, Uuid};
use serde_json::Value;

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;
use crate::standard_libraries::Libraries;
use crate::standards::{resolve_project, style_differences};
use crate::standards_install_library::{self as library, Incoming as LibraryIncoming};

fn fail(code: &str, message: String, path: String) -> Stop {
    Stop::failed(error(code, message, Some(path)))
}

fn key(name: &str) -> String {
    name.trim().to_lowercase()
}

fn kind_words(k: Kind) -> &'static str {
    match k {
        Kind::LineType => "çizgi tipi",
        Kind::HatchPattern => "tarama deseni",
        Kind::TextStyle => "yazı stili",
        Kind::DimensionStyle => "ölçü stili",
        Kind::LeaderStyle => "kılavuz stili",
        Kind::TableStyle => "tablo stili",
        Kind::Block => "blok",
        Kind::SheetTemplate => "pafta şablonu",
    }
}

/// A resource's appearance as the check compares it: its name, id, description and source aside.
fn appearance<T: serde::Serialize>(item: &T) -> Value {
    let mut v = serde_json::to_value(item).unwrap_or(Value::Null);
    if let Some(o) = v.as_object_mut() {
        for field in ["id", "name", "description", "source"] {
            o.remove(field);
        }
    }
    v
}

/// One incoming resource: its kind, name, appearance and source, and the drawing's of that name.
struct Incoming {
    kind: Kind,
    name: String,
    look: Value,
    local: Option<Value>,
    source: StandardOrigin,
}

fn incoming(doc: &Document, r: &StandardsResolved) -> Vec<Incoming> {
    let s = doc.settings();
    let origin = |map: &std::collections::BTreeMap<String, StandardOrigin>, name: &str| {
        map.get(name).cloned().unwrap_or_else(|| r.provenance.rules.clone())
    };
    let mut out = Vec::new();
    let p = &r.provenance;
    for t in &r.resources.line_types {
        out.push(Incoming {
            kind: Kind::LineType,
            name: t.name.clone(),
            look: appearance(t),
            local: s.line_types.iter().find(|l| key(&l.name) == key(&t.name)).map(appearance),
            source: origin(&p.line_types, &t.name),
        });
    }
    for t in &r.resources.hatch_patterns {
        out.push(Incoming {
            kind: Kind::HatchPattern,
            name: t.name.clone(),
            look: appearance(t),
            local: s.hatch_patterns.iter().find(|l| key(&l.name) == key(&t.name)).map(appearance),
            source: origin(&p.hatch_patterns, &t.name),
        });
    }
    for t in &r.resources.text_style_definitions {
        out.push(Incoming {
            kind: Kind::TextStyle,
            name: t.name.clone(),
            look: appearance(t),
            local: s.text_styles.iter().find(|l| key(&l.name) == key(&t.name)).map(appearance),
            source: origin(&p.text_style_definitions, &t.name),
        });
    }
    for t in &r.resources.dimension_style_definitions {
        out.push(Incoming {
            kind: Kind::DimensionStyle,
            name: t.name.clone(),
            look: appearance(t),
            local: s.dimension_styles.iter().find(|l| key(&l.name) == key(&t.name)).map(appearance),
            source: origin(&p.dimension_style_definitions, &t.name),
        });
    }
    for t in &r.resources.leader_styles {
        out.push(Incoming {
            kind: Kind::LeaderStyle,
            name: t.name.clone(),
            look: appearance(t),
            local: s.leader_styles.iter().find(|l| key(&l.name) == key(&t.name)).map(appearance),
            source: origin(&p.leader_styles, &t.name),
        });
    }
    // Table styles (docs/adr/1126 §6): by name, every cell style compared.
    for t in &r.resources.table_style_definitions {
        out.push(Incoming {
            kind: Kind::TableStyle,
            name: t.name.clone(),
            look: appearance(t),
            local: s.table_styles.iter().find(|l| key(&l.name) == key(&t.name)).map(appearance),
            source: origin(&p.table_style_definitions, &t.name),
        });
    }
    out.sort_by_key(|i| i.kind);
    out
}

/// The names of `kind` the drawing has, folded.
fn local_names(doc: &Document, kind: Kind) -> BTreeSet<String> {
    let s = doc.settings();
    match kind {
        Kind::LineType => s.line_types.iter().map(|t| key(&t.name)).collect(),
        Kind::HatchPattern => s.hatch_patterns.iter().map(|t| key(&t.name)).collect(),
        Kind::TextStyle => s.text_styles.iter().map(|t| key(&t.name)).collect(),
        Kind::DimensionStyle => s.dimension_styles.iter().map(|t| key(&t.name)).collect(),
        Kind::LeaderStyle => s.leader_styles.iter().map(|t| key(&t.name)).collect(),
        Kind::TableStyle => s.table_styles.iter().map(|t| key(&t.name)).collect(),
        Kind::Block => doc.blocks().iter().map(|b| key(&b.name)).collect(),
        Kind::SheetTemplate => s.approved_sheet_templates.iter().map(|t| key(&t.template_id)).collect(),
    }
}

/// Every row with what a block or template row installs (style rows install from the resolved standard).
type Rows = Vec<(StandardInstallRow, Option<LibraryIncoming>)>;

/// The rows, every choice checked; why not.
fn rows(doc: &Document, input: &StandardsInstall, libraries: &Libraries) -> Result<(StandardsResolved, Rows), Stop> {
    let resolved = match resolve_project(doc) {
        CommandResult::Completed { output, .. } => output,
        CommandResult::Failed { error } => return Err(Stop::failed(error)),
        CommandResult::Conflict { error } => return Err(Stop::conflict(error)),
        _ => return Err(Stop::Cancelled),
    };
    let found = incoming(doc, &resolved);
    let mut rows: Rows = found
        .iter()
        .map(|i| {
            let (status, fields) = match &i.local {
                None => ("missing", Vec::new()),
                Some(local) => {
                    let fields = style_differences(local, &i.look);
                    if fields.is_empty() { ("same", fields) } else { ("differs", fields) }
                }
            };
            let row = StandardInstallRow {
                kind: i.kind,
                name: i.name.clone(),
                status: status.to_owned(),
                action: if status == "missing" { Action::Add } else { Action::KeepLocal },
                new_name: None,
                fields,
                library: None,
                uses: 0,
                source: i.source.clone(),
            };
            (row, None)
        })
        .collect();
    rows.extend(library::rows(doc, &resolved, libraries));
    let mut chosen = BTreeSet::new();
    let mut taken: Vec<(Kind, String)> = Vec::new();
    let library_names: Vec<(Kind, String)> =
        rows.iter().filter(|(r, _)| matches!(r.kind, Kind::Block | Kind::SheetTemplate)).map(|(r, _)| (r.kind, r.name.clone())).collect();
    for (n, c) in input.choices.iter().enumerate() {
        let at = format!("choices[{n}]");
        let Some((row, _)) = rows.iter_mut().find(|(r, _)| r.kind == c.kind && key(&r.name) == key(&c.name)) else {
            return Err(fail(
                codes::STANDARD_RESOURCE_NOT_FOUND,
                format!(
                    "Bağlı standartlarda “{}” adlı {} yok. Kaynağı standartların listesinden seçin.",
                    c.name,
                    kind_words(c.kind)
                ),
                format!("{at}.name"),
            ));
        };
        if !chosen.insert((c.kind, key(&c.name))) {
            return Err(fail(
                codes::INVALID_INSTALL_CHOICE,
                format!("“{}” {} için iki seçim verildi; her kaynağa bir seçim verin.", c.name, kind_words(c.kind)),
                at,
            ));
        }
        if let Some(why) = library::refused_action(doc, row, c.action) {
            return Err(fail(
                codes::INVALID_INSTALL_CHOICE,
                format!("“{}” {} {why}.", c.name, kind_words(c.kind)),
                format!("{at}.action"),
            ));
        }
        let allowed = match row.status.as_str() {
            "missing" => matches!(c.action, Action::Add | Action::KeepLocal),
            "same" | "unavailable" => matches!(c.action, Action::KeepLocal),
            _ => matches!(c.action, Action::Replace | Action::KeepLocal | Action::RenameIncoming),
        };
        if !allowed {
            let why = match row.status.as_str() {
                "missing" => "çizimde yok: eklenebilir ya da atlanabilir",
                "same" => "çizimdekiyle aynı: değişecek bir şey yok",
                _ => "çizimdekinden farklı: değiştirilebilir, yeni adla eklenebilir ya da çizimdeki korunabilir",
            };
            return Err(fail(
                codes::INVALID_INSTALL_CHOICE,
                format!("“{}” {} {why}.", c.name, kind_words(c.kind)),
                format!("{at}.action"),
            ));
        }
        match (c.action, &c.new_name) {
            (Action::RenameIncoming, Some(new)) if !checks::is_blank(new) && new.trim() == new => {
                let clash = local_names(doc, c.kind).contains(&key(new))
                    || (c.kind == Kind::Block && doc.block_named(new).is_some())
                    || taken.iter().any(|(k, t)| *k == c.kind && key(t) == key(new))
                    || rows_with_name(&found, c.kind, new)
                    || library_rows_with_name(&library_names, c.kind, new);
                if clash {
                    return Err(fail(
                        codes::DUPLICATE_NAME,
                        format!("“{new}” adlı {} çizimde ya da standartta var; başka bir ad verin.", kind_words(c.kind)),
                        format!("{at}.newName"),
                    ));
                }
                taken.push((c.kind, new.clone()));
            }
            (Action::RenameIncoming, _) => {
                return Err(fail(
                    codes::INVALID_INSTALL_CHOICE,
                    "Yeni adla eklemek için kenarı boşluksuz, boş olmayan bir ad verin (newName).".into(),
                    format!("{at}.newName"),
                ));
            }
            (_, Some(_)) => {
                return Err(fail(
                    codes::INVALID_INSTALL_CHOICE,
                    "Yeni ad yalnız yeni adla eklemede (renameIncoming) verilir.".into(),
                    format!("{at}.newName"),
                ));
            }
            _ => {}
        }
        row.action = c.action;
        row.new_name.clone_from(&c.new_name);
    }
    checks::revision(doc, input.expected_revision.as_deref())?;
    Ok((resolved, rows))
}

fn rows_with_name(found: &[Incoming], kind: Kind, name: &str) -> bool {
    found.iter().any(|i| i.kind == kind && key(&i.name) == key(name))
}

fn library_rows_with_name(names: &[(Kind, String)], kind: Kind, name: &str) -> bool {
    names.iter().any(|(k, n)| *k == kind && key(n) == key(name))
}

fn counted(rows: Rows, revision: String) -> StandardsInstalled {
    let rows: Vec<StandardInstallRow> = rows.into_iter().map(|(r, _)| r).collect();
    let n = |a: Action| rows.iter().filter(|r| r.action == a && r.status != "same").count() as u32;
    StandardsInstalled {
        added: n(Action::Add),
        replaced: n(Action::Replace),
        renamed: n(Action::RenameIncoming),
        kept: rows.iter().filter(|r| r.action == Action::KeepLocal).count() as u32,
        unavailable: rows.iter().filter(|r| r.status == "unavailable").count() as u32,
        rows,
        revision,
    }
}

fn fresh() -> kentos_commands::BlockId {
    kentos_commands::BlockId(*Uuid::now_v7().as_bytes())
}

/// The rows and the blocks they would write, every refusal checked.
fn checked(doc: &Document, input: &StandardsInstall, libraries: &Libraries) -> Result<(StandardsResolved, Rows, library::BlockWrite), Stop> {
    let libraries = given_libraries(input, libraries)?;
    let (resolved, rows) = rows(doc, input, &libraries)?;
    let blocks = library::plan_blocks(doc, &rows, fresh)?;
    Ok((resolved, rows, blocks))
}

/// The largest block library file an install reads.
const LIBRARY_LIMIT: u64 = 16 * 1024 * 1024;

/// `libraries` with the input's own block libraries in front, the first given searched first (docs/adr/1019): each
/// an absolute `.kblok` path, read now; refused by the first that is not one, is missing or does not read.
fn given_libraries(input: &StandardsInstall, libraries: &Libraries) -> Result<Libraries, Stop> {
    let mut out = libraries.clone();
    let fail = |code: &str, message: String, i: usize| Stop::failed(error(code, message, Some(format!("libraries[{i}]"))));
    let mut read = Vec::new();
    for (i, path) in input.libraries.iter().enumerate() {
        let at = std::path::Path::new(path);
        if !at.is_absolute() || at.extension().is_none_or(|e| !e.eq_ignore_ascii_case("kblok")) {
            return Err(fail(
                codes::INVALID_FILE_PATH,
                format!("“{path}” bir blok kitaplığının (.kblok) mutlak yolu değil; tam yolu verin."),
                i,
            ));
        }
        let meta = match std::fs::metadata(at) {
            Ok(m) if m.is_file() => m,
            _ => return Err(fail(codes::FILE_NOT_FOUND, format!("“{path}” yok; kitaplığın yolunu denetleyin."), i)),
        };
        if meta.len() > LIBRARY_LIMIT {
            return Err(fail(
                codes::BLOCK_LIBRARY_UNREADABLE,
                format!("“{path}” 16 MiB'tan büyük; kitaplığı bölün."),
                i,
            ));
        }
        let name = at.file_stem().map_or_else(|| path.clone(), |s| s.to_string_lossy().into_owned());
        let library = std::fs::read_to_string(at)
            .map_err(|e| e.to_string())
            .and_then(|text| crate::standard_libraries::read_block_library(&name, &text))
            .map_err(|why| fail(codes::BLOCK_LIBRARY_UNREADABLE, format!("“{path}” okunamadı: {why}."), i))?;
        read.push(library);
    }
    for library in read.into_iter().rev() {
        out = out.with_blocks(library);
    }
    Ok(out)
}

/// A local id `id` does not clash with: `id`, else `id-2`, `id-3` …
fn fresh_id(taken: &[&str], id: &str) -> String {
    if !taken.contains(&id) {
        return id.to_owned();
    }
    (2..).map(|n| format!("{id}-{n}")).find(|c| !taken.contains(&c.as_str())).unwrap_or_default()
}

/// The style table after the rows of `kind`.
fn styles<T: Clone>(
    table: &[T],
    incoming: &[T],
    rows: &[&StandardInstallRow],
    kind: Kind,
    name: impl Fn(&T) -> &str,
    id: impl Fn(&T) -> &str,
    with: impl Fn(&T, Option<String>, Option<String>) -> T,
) -> Vec<T> {
    let mut out = table.to_vec();
    for row in rows.iter().filter(|r| r.kind == kind && r.status != "same") {
        let Some(new) = incoming.iter().find(|t| key(name(t)) == key(&row.name)) else {
            continue;
        };
        match row.action {
            Action::Add | Action::RenameIncoming => {
                let ids: Vec<String> = out.iter().map(|t| id(t).to_owned()).collect();
                let ids: Vec<&str> = ids.iter().map(String::as_str).collect();
                let fresh = fresh_id(&ids, id(new));
                out.push(with(new, row.new_name.clone(), Some(fresh)));
            }
            Action::Replace => {
                if let Some(at) = out.iter().position(|t| key(name(t)) == key(&row.name)) {
                    let local_name = name(&out[at]).to_owned();
                    let local_id = id(&out[at]).to_owned();
                    out[at] = with(new, Some(local_name), Some(local_id));
                }
            }
            Action::KeepLocal => {}
        }
    }
    out
}

fn write_rows(
    doc: &mut Document,
    resolved: &StandardsResolved,
    all: &Rows,
    blocks: library::BlockWrite,
) -> Result<(), String> {
    let rows: Vec<&StandardInstallRow> = all.iter().map(|(r, _)| r).collect();
    let rows = rows.as_slice();
    let r = &resolved.resources;
    let s = doc.settings().clone();
    // Line types and hatch patterns have no id: the name is the reference.
    let named = |t: &str| t.to_owned();
    let line_types = styles(
        &s.line_types,
        &r.line_types,
        rows,
        Kind::LineType,
        |t| t.name.as_str(),
        |t| t.name.as_str(),
        |t, name, _| kentos_commands::LineTypeDef { name: name.map_or_else(|| named(&t.name), |n| n), ..t.clone() },
    );
    let hatch_patterns = styles(
        &s.hatch_patterns,
        &r.hatch_patterns,
        rows,
        Kind::HatchPattern,
        |t| t.name.as_str(),
        |t| t.name.as_str(),
        |t, name, _| kentos_commands::HatchPatternDef { name: name.map_or_else(|| named(&t.name), |n| n), ..t.clone() },
    );
    let text = styles(
        &s.text_styles,
        &r.text_style_definitions,
        rows,
        Kind::TextStyle,
        |t| t.name.as_str(),
        |t| t.id.as_str(),
        |t, name, id| kentos_commands::TextStyleDef {
            name: name.unwrap_or_else(|| t.name.clone()),
            id: id.unwrap_or_else(|| t.id.clone()),
            ..t.clone()
        },
    );
    let dimension = styles(
        &s.dimension_styles,
        &r.dimension_style_definitions,
        rows,
        Kind::DimensionStyle,
        |t| t.name.as_str(),
        |t| t.id.as_str(),
        |t, name, id| kentos_commands::DimensionStyleDef {
            name: name.unwrap_or_else(|| t.name.clone()),
            id: id.unwrap_or_else(|| t.id.clone()),
            ..t.clone()
        },
    );
    let leader = styles(
        &s.leader_styles,
        &r.leader_styles,
        rows,
        Kind::LeaderStyle,
        |t| t.name.as_str(),
        |t| t.id.as_str(),
        |t, name, id| kentos_commands::LeaderStyleDef {
            name: name.unwrap_or_else(|| t.name.clone()),
            id: id.unwrap_or_else(|| t.id.clone()),
            ..t.clone()
        },
    );
    // Table styles (docs/adr/1126 §6): replaced in place keep the drawing's name and id, so its tables follow.
    let table = styles(
        &s.table_styles,
        &r.table_style_definitions,
        rows,
        Kind::TableStyle,
        |t| t.name.as_str(),
        |t| t.id.as_str(),
        |t, name, id| kentos_commands::TableStyleDef {
            name: name.unwrap_or_else(|| t.name.clone()),
            id: id.unwrap_or_else(|| t.id.clone()),
            ..t.clone()
        },
    );
    let approved = library::approved_after(doc, all);
    doc.name_step(STANDARDS_INSTALL_STEP);
    doc.transact(STANDARDS_INSTALL_STEP, |doc| {
        doc.with_style_tables(STANDARDS_INSTALL_STEP, |doc| {
            let mut next = doc.settings().clone();
            next.text_styles = text;
            next.dimension_styles = dimension;
            next.leader_styles = leader;
            next.table_styles = table;
            doc.set_settings(next);
        });
        doc.set_pattern_tables(line_types, hatch_patterns, STANDARDS_INSTALL_STEP);
        doc.set_approved_sheet_templates(approved, STANDARDS_INSTALL_STEP);
        library::write_blocks(doc, blocks)
    })
}

/// Checks the choices, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &StandardsInstall) -> CommandResult<()> {
    validate_with(cx, input, &Libraries::builtin())
}

/// [`validate`] reading the references from `libraries`.
pub fn validate_with(cx: &ExecutionContext<'_>, input: &StandardsInstall, libraries: &Libraries) -> CommandResult<()> {
    match checked(cx.doc, input, libraries) {
        Ok(_) => CommandResult::Completed { output: (), warnings: Vec::new() },
        Err(stop) => stop.into(),
    }
}

/// Every resource and what the install would do with it; the drawing stays.
pub fn plan(cx: &ExecutionContext<'_>, input: &StandardsInstall) -> CommandResult<StandardsInstalled> {
    plan_with(cx, input, &Libraries::builtin())
}

/// [`plan`] reading the references from `libraries`.
pub fn plan_with(
    cx: &ExecutionContext<'_>,
    input: &StandardsInstall,
    libraries: &Libraries,
) -> CommandResult<StandardsInstalled> {
    match checked(cx.doc, input, libraries) {
        Ok((_, rows, _)) => {
            CommandResult::Completed { output: counted(rows, cx.doc.revision().to_string()), warnings: Vec::new() }
        }
        Err(stop) => stop.into(),
    }
}

/// Installs as one undo step “Standart kaynaklarını kur”.
pub fn execute(cx: &mut ExecutionContext<'_>, input: StandardsInstall) -> CommandResult<StandardsInstalled> {
    execute_with(cx, input, &Libraries::builtin())
}

/// [`execute`] reading the references from `libraries` (a host's project or office library in front of KentOS's).
pub fn execute_with(
    cx: &mut ExecutionContext<'_>,
    input: StandardsInstall,
    libraries: &Libraries,
) -> CommandResult<StandardsInstalled> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_STANDARDS_INSTALL,
        kentos_commands::CAD_STANDARDS_INSTALL_VERSION,
        input,
        |cx, input| {
            let (resolved, rows, blocks) = match checked(cx.doc, &input, libraries) {
                Ok(done) => done,
                Err(stop) => return stop.into(),
            };
            if let Err(message) = write_rows(cx.doc, &resolved, &rows, blocks) {
                return Stop::failed(error(codes::BLOCK_REFUSED, message, Some("choices".into()))).into();
            }
            CommandResult::Completed {
                output: counted(rows, cx.doc.revision().to_string()),
                warnings: Vec::new(),
            }
        },
    )
}
