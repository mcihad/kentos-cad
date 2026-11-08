//! Teslim paketinin devir dosyaları (`file.package`'s `exchange`; plan/map-cad.md MAP-17, docs/adr/2780): the package's
//! own project written by KentOS's writer in each exchange format asked, into the plan's scratch folder
//! (`devir/<name>.<extension>`), and read back from there as `file.exchange.check` reads its file, so the losses said are
//! those of the very file the package delivers. The checks give the support matrix (`exchange_matrix::rows`); for people
//! the matrix goes in words into `devir/devir-raporu.csv`. The files are then copied into the package byte for byte like
//! every other copy, each against its SHA-256.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use kentos_commands::exchange_words::{aspect_word, cell_words};
use kentos_commands::{
    CommandError, CommandResult, CommandWarning, EXCHANGE_TOLERANCE, ExchangeCheck, ExchangeFormat, ExchangeMatrix,
    ExchangeMatrixed, ExchangeState, codes,
};
use kentos_domain::Document;
use serde_json::Value;

use crate::file_commands::Refused;

/// The open files' folder in the package.
pub const FOLDER: &str = "devir";
/// The matrix in words, in the folder.
pub const REPORT: &str = "devir-raporu.csv";
/// The command the report is said to come from.
const MATRIX: &str = "file.exchange.matrix";

/// The open files made: the matrix and its checks, and each file (where it was made, its path in the package, the
/// command that wrote it), in the order written, the report last.
pub(crate) struct Made {
    pub(crate) matrix: ExchangeMatrixed,
    pub(crate) files: Vec<(PathBuf, String, String)>,
}

/// A check's refusal, its path under `under`.
fn failed(result: CommandResult<Value>, under: &str) -> Refused {
    let mut error = match result {
        CommandResult::Failed { error } | CommandResult::Conflict { error } | CommandResult::NeedsInput { error } => error,
        _ => CommandError {
            code: codes::EXCHANGE_UNREADABLE.into(),
            message: "Devir dosyası denetlenemedi.".into(),
            ..CommandError::default()
        },
    };
    error.path = Some(match error.path.take() {
        Some(p) if !p.is_empty() => format!("{under}.{p}"),
        _ => under.to_owned(),
    });
    Box::new(error)
}

fn unwritable(message: String) -> Refused {
    Box::new(CommandError {
        code: codes::FILE_UNWRITABLE.into(),
        message,
        path: Some("path".into()),
        ..CommandError::default()
    })
}

/// The formats asked, once each, in their order; none: all six.
fn formats(ask: &ExchangeMatrix) -> Vec<ExchangeFormat> {
    let mut out: Vec<ExchangeFormat> = Vec::new();
    for f in if ask.formats.is_empty() { ExchangeFormat::ALL.to_vec() } else { ask.formats.clone() } {
        if !out.contains(&f) {
            out.push(f);
        }
    }
    out
}

fn check_of(ask: &ExchangeMatrix, format: ExchangeFormat) -> ExchangeCheck {
    ExchangeCheck { format, layers: ask.layers.clone(), tolerance: ask.tolerance }
}

/// The question's own refusals as the check says them (the tolerance, then the layers), under `exchange.`.
pub(crate) fn asked(doc: &Document, ask: &ExchangeMatrix) -> Result<(), Refused> {
    let Some(&f) = formats(ask).first() else {
        return Ok(());
    };
    crate::exchange_check::asked(doc, &check_of(ask, f)).map_err(|r| failed(r, "exchange"))
}

/// The files of `folder`, by name.
fn listed(folder: &Path) -> BTreeSet<String> {
    fs::read_dir(folder)
        .map(|read| {
            read.flatten()
                .filter(|e| e.path().is_file())
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default()
}

/// Each format asked written into `scratch/devir/` as `stem.<extension>` and tried back there; the matrix of the checks
/// and its words. A writer's or the reader's refusal is the package's, under `exchange.formats[i]`.
pub(crate) fn make(doc: &Document, ask: &ExchangeMatrix, scratch: &Path, stem: &str) -> Result<Made, Refused> {
    let folder = scratch.join(FOLDER);
    fs::create_dir_all(&folder).map_err(|e| unwritable(format!("{} açılamadı: {e}", folder.display())))?;
    let mut checks = Vec::new();
    let mut files = Vec::new();
    for (i, f) in formats(ask).into_iter().enumerate() {
        let before = listed(&folder);
        let file = folder.join(format!("{stem}.{}", f.extension()));
        let checked =
            crate::exchange_check::trip(doc, &check_of(ask, f), &file).map_err(|r| failed(r, &format!("exchange.formats[{i}]")))?;
        checks.push(checked);
        // Every file the writer made, by name.
        for name in listed(&folder).difference(&before) {
            files.push((folder.join(name), format!("{FOLDER}/{name}"), f.commands().0.to_owned()));
        }
    }
    let tolerance = checks.first().map_or(ask.tolerance.unwrap_or(EXCHANGE_TOLERANCE), |c| c.tolerance);
    let matrix = ExchangeMatrixed { tolerance, rows: crate::exchange_matrix::rows(&checks), checks };
    let report = folder.join(REPORT);
    fs::write(&report, words(&matrix)).map_err(|e| unwritable(format!("{} yazılamadı: {e}", report.display())))?;
    files.push((report, format!("{FOLDER}/{REPORT}"), MATRIX.to_owned()));
    Ok(Made { matrix, files })
}

/// A cell of the report: in quotes when it holds the separator, a quote or a line break, its quotes doubled.
fn field(text: &str) -> String {
    if text.contains([';', '"', '\n', '\r']) {
        format!("\"{}\"", text.replace('"', "\"\""))
    } else {
        text.to_owned()
    }
}

/// The matrix in words (`devir-raporu.csv`, docs/adr/2780): a head naming the formats, then a row for each aspect, its
/// cells as Dışa aktarma kaybı denetimi says them; `;` between cells as a Turkish spreadsheet takes them, UTF-8.
pub(crate) fn words(m: &ExchangeMatrixed) -> String {
    let mut out = String::new();
    let head: Vec<String> =
        std::iter::once("Özellik".to_owned()).chain(m.checks.iter().map(|c| c.format.title().to_owned())).collect();
    out.push_str(&head.iter().map(|c| field(c)).collect::<Vec<_>>().join(";"));
    out.push('\n');
    for r in &m.rows {
        let cells: Vec<String> = std::iter::once(aspect_word(&r.aspect).to_owned())
            .chain(r.cells.iter().map(|c| cell_words(&r.aspect, c)))
            .collect();
        out.push_str(&cells.iter().map(|c| field(c)).collect::<Vec<_>>().join(";"));
        out.push('\n');
    }
    out
}

/// What the open files lose, said once (`export_losses`): each format with an aspect not kept, the aspects named.
pub(crate) fn losses(m: &ExchangeMatrixed) -> Option<CommandWarning> {
    let said: Vec<String> = m
        .checks
        .iter()
        .enumerate()
        .filter_map(|(i, c)| {
            let changed: Vec<&str> = m
                .rows
                .iter()
                .filter(|r| r.cells.get(i).is_some_and(|x| x.state != ExchangeState::Kept))
                .map(|r| aspect_word(&r.aspect))
                .collect();
            (!changed.is_empty()).then(|| format!("{}: {}", c.format.title(), changed.join(", ").to_lowercase()))
        })
        .collect();
    (!said.is_empty()).then(|| CommandWarning {
        code: codes::EXPORT_LOSSES.into(),
        message: format!(
            "Devir dosyaları projenin her şeyini taşımaz ({}). Ayrıntı paket.json'da ve {FOLDER}/{REPORT}'de; proje dosyası \
             kaynaktır.",
            said.join("; ")
        ),
        path: Some("exchange".into()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use kentos_commands::{ExchangeCell, ExchangeChecked, ExchangeNumeric, ExchangeRow};

    fn checked(format: ExchangeFormat) -> ExchangeChecked {
        ExchangeChecked {
            format,
            tolerance: 0.001,
            objects: 3,
            matched: 3,
            not_written: Vec::new(),
            numeric: ExchangeNumeric::default(),
            losses: Vec::new(),
            writer: Vec::new(),
            reader: Vec::new(),
        }
    }

    #[test]
    fn the_report_names_the_formats_and_says_each_cell() {
        let m = ExchangeMatrixed {
            tolerance: 0.001,
            rows: vec![ExchangeRow {
                aspect: "objects".into(),
                cells: vec![
                    ExchangeCell { format: ExchangeFormat::Gpkg, state: ExchangeState::Lost, count: 1, detail: Some("text 1".into()) },
                    ExchangeCell { format: ExchangeFormat::Dxf, state: ExchangeState::Kept, count: 0, detail: None },
                ],
            }],
            checks: vec![checked(ExchangeFormat::Gpkg), checked(ExchangeFormat::Dxf)],
        };
        assert_eq!(words(&m), "Özellik;GeoPackage;DXF\nNesneler;Kaybolur: yazı 1;Korunur\n");
        let w = losses(&m).expect("a loss");
        assert_eq!(w.code, "export_losses");
        assert!(w.message.contains("(GeoPackage: nesneler)"), "{}", w.message);
        assert_eq!(field("a;b \"c\""), "\"a;b \"\"c\"\"\"");
    }

    #[test]
    fn a_format_asked_twice_is_written_once() {
        let ask = ExchangeMatrix { formats: vec![ExchangeFormat::Dxf, ExchangeFormat::Gpkg, ExchangeFormat::Dxf], ..Default::default() };
        assert_eq!(formats(&ask), [ExchangeFormat::Dxf, ExchangeFormat::Gpkg]);
        assert_eq!(formats(&ExchangeMatrix::default()), ExchangeFormat::ALL);
    }
}
