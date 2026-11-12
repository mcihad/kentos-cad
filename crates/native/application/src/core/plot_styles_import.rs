//! `cad.plotstyles.import` (“Baskı stili tablosu içe aktar”, docs/adr/1130): an AutoCAD `.ctb`/`.stb` read by the
//! sheet core's reader (`kentos_sheet::plot_profile::acad`) into a plot profile of the book, with its explicit pen or
//! style assignments and the sheets it is applied to, written as one change of the book through `cad.sheets.edit`'s
//! own path. The file is only read. The shared cases are `fixtures/commands/v1/cad.plotstyles.import.json`.

use std::path::{Path, PathBuf};

use kentos_commands::{CommandResult, CommandWarning, PlotStylesImport, PlotStylesImported, SheetsEdit};
use kentos_sheet::{
    ops::Op,
    plot_profile::{self, Assignment, SavePlotProfiles, Target, acad},
    print_setup::{AppliedPageSetup, SetSheetPlot},
};

use crate::{
    ExecutionContext,
    checks::{self, Stop, error},
    codes, sheets_edit,
};

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.to_owned())))
}

/// Where the file is: as given when absolute, else from `base`.
fn resolved(path: &str, base: Option<&str>) -> PathBuf {
    let p = Path::new(path);
    match base {
        Some(b) if !p.is_absolute() => Path::new(b).join(p),
        _ => p.to_path_buf(),
    }
}

/// What the import writes: the book's change, and what the reading said.
struct Prepared {
    edit: SheetsEdit,
    read: acad::Read,
    applied: Vec<String>,
}

fn prepare(cx: &ExecutionContext<'_>, input: &PlotStylesImport) -> Result<Prepared, Stop> {
    checks::revision(cx.doc, input.expected_revision.as_deref())?;
    if input.path.trim().is_empty() {
        return Err(fail(codes::INVALID_INPUT, "Okunacak .ctb ya da .stb dosyasının yolunu verin.".into(), "path"));
    }
    let at = resolved(&input.path, input.base.as_deref());
    let file = at
        .file_name()
        .map_or_else(|| input.path.clone(), |n| n.to_string_lossy().into_owned());
    let name = match &input.name {
        Some(n) => n.trim().to_owned(),
        None => at
            .file_stem()
            .map_or_else(|| file.clone(), |s| s.to_string_lossy().into_owned()),
    };
    if name.is_empty() || name.len() > 256 || name.chars().any(char::is_control) {
        return Err(fail(
            codes::INVALID_INPUT,
            "Profilin adı boş olmayan, tek satırlık ve en çok 256 baytlık olmalı.".into(),
            "name",
        ));
    }
    if !at.is_file() {
        return Err(fail(
            codes::FILE_NOT_FOUND,
            format!("“{}” yok; dosyanın yolunu denetleyin.", at.display()),
            "path",
        ));
    }
    let meta_len = std::fs::metadata(&at).map(|m| m.len()).unwrap_or(0);
    if meta_len > acad::MAX_FILE as u64 {
        return Err(fail(
            "plot_style_file_too_large",
            "Baskı stili tablosu 4 MiB sınırını aşıyor.".into(),
            "path",
        ));
    }
    let bytes = std::fs::read(&at).map_err(|e| {
        fail(
            codes::FILE_UNREADABLE,
            format!("“{}” okunamadı: {e}. Dosyanın izinlerini denetleyin.", at.display()),
            "path",
        )
    })?;
    // The profile's id comes from the file's bytes: the same file is the same profile.
    let digest = kentos_sheet::template::sha256_hex(&bytes);
    let format = if bytes.starts_with(b"PIAFILEVERSION_2.0,STBVER1") { "stb" } else { "ctb" };
    let id = format!("{format}-{}", &digest[..12]);
    let mut read = acad::read(&bytes, &id, &name, &file)
        .map_err(|e| fail(&e.code, e.message, "path"))?;
    let (book, _) = match cx.doc.sheets() {
        Some(s) => kentos_sheet::project::read_project_sheets(s)
            .map_err(|e| Stop::failed(error(codes::BOOK_UNREADABLE, e.message, None)))?,
        None => (Default::default(), Vec::new()),
    };
    if let Some(p) = book.plot_profiles.iter().find(|p| p.name == name) {
        return Err(fail(
            "duplicate_plot_profile",
            format!("Kitapta “{}” adlı bir baskı profili var; başka bir ad verin ya da önce onu kaldırın.", p.name),
            "name",
        ));
    }
    if let Some(p) = book.plot_profiles.iter().find(|p| p.id == id) {
        return Err(fail(
            "duplicate_plot_profile",
            format!("Bu dosya kitapta “{}” adıyla zaten var; aynı tablo ikinci kez alınmaz.", p.name),
            "path",
        ));
    }
    for (i, v) in input.assignments.iter().enumerate() {
        let path = format!("assignments[{i}]");
        let a: Assignment = serde_json::from_value(v.clone()).map_err(|e| {
            fail(codes::INVALID_INPUT, format!("Atama okunamadı: {e}"), &path)
        })?;
        match &a.target {
            Target::ModelLayer { id } => {
                let is_layer = cx
                    .doc
                    .layers()
                    .get(id)
                    .is_some_and(|n| n.kind == kentos_commands::LayerNodeType::Layer);
                if !is_layer {
                    return Err(fail(
                        codes::LAYER_NOT_FOUND,
                        format!("“{id}” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin."),
                        &format!("{path}.target.id"),
                    ));
                }
            }
            Target::PaperItem { id } => {
                if book.item(id).is_none() {
                    return Err(fail(
                        codes::UNKNOWN_ITEM,
                        format!("Kitapta “{id}” kimlikli kâğıt öğesi yok."),
                        &format!("{path}.target.id"),
                    ));
                }
            }
        }
        read.profile.assignments.push(a);
    }
    if let Err(e) = plot_profile::validate(&read.profile) {
        return Err(fail(&e.code, e.message, "assignments"));
    }
    let mut applied: Vec<String> = Vec::new();
    for (i, s) in input.sheets.iter().enumerate() {
        if book.sheet(s).is_none() {
            return Err(fail(
                codes::UNKNOWN_SHEET,
                format!("Kitapta “{s}” kimlikli pafta yok."),
                &format!("sheets[{i}]"),
            ));
        }
        if !applied.contains(s) {
            applied.push(s.clone());
        }
    }
    let mut profiles = book.plot_profiles.clone();
    profiles.push(read.profile.clone());
    let mut ops = vec![Op::SavePlotProfiles(SavePlotProfiles { profiles })];
    for id in &applied {
        let sheet = book.sheet(id);
        let mut settings = sheet
            .and_then(|s| s.plot.as_ref())
            .map(|p| p.settings.clone())
            .unwrap_or_default();
        settings.profile = Some(read.profile.id.clone());
        settings.profile_snapshot = Some(read.profile.clone());
        ops.push(Op::SetSheetPlot(SetSheetPlot {
            sheet: id.clone(),
            plot: Some(AppliedPageSetup {
                source: sheet.and_then(|s| s.plot.as_ref()).and_then(|p| p.source.clone()),
                settings,
            }),
        }));
    }
    let edit = SheetsEdit {
        ops: ops
            .iter()
            .map(|op| serde_json::to_value(op).unwrap_or_default())
            .collect(),
        pictures: Vec::new(),
        replace_pictures: false,
        expected_revision: input.expected_revision.clone(),
    };
    Ok(Prepared { edit, read, applied })
}

/// The report's warning: how many lines lose or approximate what AutoCAD plots.
fn warnings_of(read: &acad::Read) -> Vec<CommandWarning> {
    let count = |s: acad::Severity| read.losses.iter().filter(|l| l.severity == s).count();
    let (lost, near) = (count(acad::Severity::Lost), count(acad::Severity::Approximate));
    if lost + near == 0 {
        return Vec::new();
    }
    vec![CommandWarning {
        code: "plot_style_losses".into(),
        message: format!(
            "{} stilin {}'i aynen alındı; {lost} alan türü KentOS'ta basılmıyor, {near} alan türü yaklaşık basılıyor. Ayrıntı losses'ta.",
            read.styles, read.exact
        ),
        path: None,
    }]
}

fn imported(result: CommandResult<kentos_commands::SheetsEdited>, p: Prepared) -> CommandResult<PlotStylesImported> {
    match result {
        CommandResult::Completed { output, mut warnings } => {
            warnings.extend(warnings_of(&p.read));
            CommandResult::Completed {
                output: PlotStylesImported {
                    edited: output,
                    format: match p.read.profile.source.as_ref().map(|s| s.format) {
                        Some(acad::StyleFormat::Stb) => "stb",
                        _ => "ctb",
                    }
                    .into(),
                    profile: serde_json::to_value(&p.read.profile).unwrap_or_default(),
                    description: p.read.description.clone(),
                    styles: p.read.styles,
                    exact: p.read.exact,
                    losses: p
                        .read
                        .losses
                        .iter()
                        .map(|l| serde_json::to_value(l).unwrap_or_default())
                        .collect(),
                    applied: p.applied,
                },
                warnings,
            }
        }
        CommandResult::Queued { job_id } => CommandResult::Queued { job_id },
        CommandResult::NeedsInput { error } => CommandResult::NeedsInput { error },
        CommandResult::Conflict { error } => CommandResult::Conflict { error },
        CommandResult::Cancelled => CommandResult::Cancelled,
        CommandResult::Failed { error } => CommandResult::Failed { error },
    }
}

pub fn validate(cx: &ExecutionContext<'_>, input: &PlotStylesImport) -> CommandResult<()> {
    match prepare(cx, input) {
        Ok(p) => sheets_edit::validate(cx, &p.edit),
        Err(e) => e.into(),
    }
}

pub fn plan(cx: &ExecutionContext<'_>, input: &PlotStylesImport) -> CommandResult<PlotStylesImported> {
    match prepare(cx, input) {
        Ok(p) => {
            let planned = sheets_edit::plan(cx, &p.edit);
            imported(planned, p)
        }
        Err(e) => e.into(),
    }
}

pub fn execute(cx: &mut ExecutionContext<'_>, input: PlotStylesImport) -> CommandResult<PlotStylesImported> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_PLOTSTYLES_IMPORT,
        kentos_commands::CAD_PLOTSTYLES_IMPORT_VERSION,
        input,
        |cx, input| match prepare(cx, &input) {
            Ok(p) => {
                let edit = p.edit.clone();
                let done = sheets_edit::execute(cx, edit);
                imported(done, p)
            }
            Err(e) => e.into(),
        },
    )
}
