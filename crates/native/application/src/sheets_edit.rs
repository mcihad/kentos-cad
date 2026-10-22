//! `cad.sheets.edit` v1 (docs/adr/0252 §2): the sheet core's own operations
//! (`kentos_sheet::ops`, the ones the sheet designer makes) applied to the
//! project's sheet book as one change. The book is read as a project keeps
//! it (`kentos_sheet::project`), changed by the core's `apply`, checked by
//! it, and written back with its pictures' bytes; a book left empty is no
//! book. The book is the paper's: an edit of the project, no undo step of
//! the drawing when standalone (docs/adr/0251 §3); an explicit group or change
//! set includes it in model undo/redo (ADR 0590). The answer's `inverse` is
//! also available. The
//! shared cases are in `fixtures/commands/v1/cad.sheets.edit.json`.

use kentos_commands::{CommandResult, SheetInfo, SheetMapInfo, SheetsEdit, SheetsEdited};
use kentos_domain::Document;
use kentos_sheet::kinds::{ItemKind, MapView};
use kentos_sheet::model::SheetBook;
use kentos_sheet::ops::{self, Op};
use kentos_sheet::project::{Pictures, project_sheets, read_project_sheets};

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;

/// The book after the operations, the pictures it keeps, and the core's
/// answer (its label and inverse); or why not.
struct Next {
    book: SheetBook,
    pictures: Pictures,
    label: String,
    inverse: Vec<Op>,
    previous_pictures: Pictures,
}

fn check(doc: &Document, input: &SheetsEdit) -> Result<Next, Stop> {
    if input.ops.is_empty() && input.pictures.is_empty() && !input.replace_pictures {
        return Err(Stop::failed(error(
            codes::NO_OPERATIONS,
            "Uygulanacak işlem yok: ops'a en az bir pafta işlemi verin.".into(),
            Some("ops".into()),
        )));
    }
    let mut parsed = Vec::with_capacity(input.ops.len());
    for (i, raw) in input.ops.iter().enumerate() {
        match serde_json::from_value::<Op>(raw.clone()) {
            Ok(op) => parsed.push(op),
            Err(e) => {
                return Err(Stop::failed(error(
                    codes::INVALID_OPERATION,
                    format!(
                        "{}. işlem pafta çekirdeğinin bir işlemi olarak okunamadı: {e}. İşlemin op adını ve alanlarını kataloğun örneğine göre denetleyin.",
                        i + 1
                    ),
                    Some(format!("ops[{i}]")),
                )));
            }
        }
    }
    checks::revision(doc, input.expected_revision.as_deref())?;
    let (mut book, mut pictures) = match doc.sheets() {
        None => (SheetBook::default(), Vec::new()),
        Some(kept) => read_project_sheets(kept).map_err(|e| {
            Stop::failed(error(
                codes::BOOK_UNREADABLE,
                format!(
                    "Bu projenin pafta kitabı okunamadı: {} Kitap olduğu gibi duruyor; ona işlem yazılmaz.",
                    e.message
                ),
                None,
            ))
        })?,
    };
    let previous_pictures = pictures.clone();
    if input.replace_pictures {
        pictures.clear();
    }
    // One by one, so a refusal names its operation; the inverse undoes them all, last first.
    let mut inverse: Vec<Op> = Vec::new();
    let mut labels = Vec::new();
    for (i, op) in parsed.iter().enumerate() {
        let applied = ops::apply(&book, op).map_err(|e| {
            let at = match &e.path {
                Some(p) => format!("ops[{i}].{p}"),
                None => format!("ops[{i}]"),
            };
            Stop::failed(error(&e.code, e.message, Some(at)))
        })?;
        book = applied.book;
        if let Op::ApplyTemplate(t) = op {
            for asset in &t.template.assets {
                let bytes =
                    kentos_commands::encoding::base64_decode(&asset.data).ok_or_else(|| {
                        Stop::failed(error(
                            "bad_asset",
                            "Şablon resminin base64 verisi bozuk.".into(),
                            Some(format!("ops[{i}].template.assets")),
                        ))
                    })?;
                put_picture(&mut pictures, asset.meta.sha256.clone(), bytes);
            }
        }
        labels.push(applied.label);
        let mut back = applied.inverse;
        back.extend(inverse);
        inverse = back;
    }
    let mut seen = std::collections::BTreeSet::new();
    for (i, p) in input.pictures.iter().enumerate() {
        let meta = book.asset(&p.sha256);
        if !seen.insert(&p.sha256)
            || meta.is_none_or(|a| a.bytes as usize != p.bytes.len())
            || kentos_commands::encoding::sha256_hex(&p.bytes) != p.sha256
        {
            return Err(Stop::failed(error("bad_asset", "Resim baytları kitapta tek bir kayda, SHA-256'ya ve byte boyuna uymalı. addAssets ile bilgilerini ekleyip doğru baytları verin.".into(), Some(format!("pictures[{i}]")))));
        }
        put_picture(&mut pictures, p.sha256.clone(), p.bytes.clone());
    }
    let label = match labels.as_slice() {
        [one] => one.clone(),
        [] => "Resim baytları".into(),
        _ => format!("{} işlem", labels.len()),
    };
    Ok(Next {
        book,
        pictures,
        label,
        inverse,
        previous_pictures,
    })
}

fn put_picture(pictures: &mut Pictures, sha: String, bytes: Vec<u8>) {
    if let Some(p) = pictures.iter_mut().find(|(s, _)| *s == sha) {
        p.1 = bytes;
    } else {
        pictures.push((sha, bytes));
    }
}

/// The book's sheets as the answer lists them: page size in millimetres, map frames with their scales.
pub fn sheets_of(book: &SheetBook) -> Vec<SheetInfo> {
    book.sheets
        .iter()
        .map(|s| SheetInfo {
            id: s.id.clone(),
            name: s.name.clone(),
            width: f64::from(s.page.size.width) / 1000.0,
            height: f64::from(s.page.size.height) / 1000.0,
            items: u32::try_from(s.items.len()).unwrap_or(u32::MAX),
            maps: s
                .items
                .iter()
                .filter_map(|item| match &item.kind {
                    ItemKind::Map(m) => Some(SheetMapInfo {
                        item: item.id.clone(),
                        name: item.name.clone(),
                        scale: match &m.view {
                            MapView::Fixed(f) => Some(f.denominator()),
                            MapView::Atlas(_) => None,
                        },
                    }),
                    _ => None,
                })
                .collect(),
        })
        .collect()
}

/// The project's sheets as [`sheets_of`] lists them: none for a book the core cannot read, an
/// empty list for a project without one.
pub fn sheets_in(doc: &Document) -> Option<Vec<SheetInfo>> {
    match doc.sheets() {
        None => Some(Vec::new()),
        Some(kept) => read_project_sheets(kept)
            .ok()
            .map(|(book, _)| sheets_of(&book)),
    }
}

fn write(doc: &mut Document, next: Next) -> Result<SheetsEdited, Stop> {
    let bytes = |sha: &str| {
        next.pictures
            .iter()
            .find(|(s, _)| s == sha)
            .map(|(_, b)| b.clone())
    };
    let kept = project_sheets(&next.book, bytes)
        .map_err(|e| Stop::failed(error(&e.code, e.message, e.path)))?;
    let previous: Vec<_> = next
        .previous_pictures
        .into_iter()
        .map(|(sha256, bytes)| kentos_commands::SheetPicture { sha256, bytes })
        .collect();
    let now = kept
        .as_ref()
        .map(|s| s.pictures.as_slice())
        .unwrap_or_default();
    let inverse_pictures = (previous != now).then_some(previous);
    doc.set_sheets(kept)
        .map_err(|e| Stop::failed(error(codes::BOOK_UNREADABLE, e, None)))?;
    Ok(SheetsEdited {
        label: next.label,
        inverse: next
            .inverse
            .iter()
            .filter_map(|op| serde_json::to_value(op).ok())
            .collect(),
        inverse_pictures,
        sheets: sheets_of(&next.book),
        revision: doc.revision().to_string(),
    })
}

/// The sheet core's operations `ops` applied to the project's book, as `cad.sheets.edit` applies them, inside another
/// command's write (Mimari şablonu's sheets, docs/adr/0346 §1).
pub(crate) fn apply_ops(
    doc: &mut Document,
    ops: Vec<serde_json::Value>,
) -> Result<SheetsEdited, Stop> {
    let next = check(
        doc,
        &SheetsEdit {
            ops,
            pictures: Vec::new(),
            replace_pictures: false,
            expected_revision: None,
        },
    )?;
    write(doc, next)
}

pub fn validate(cx: &ExecutionContext<'_>, input: &SheetsEdit) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// What execute would do, on a reading copy; the drawing is left as it is.
pub fn plan(cx: &ExecutionContext<'_>, input: &SheetsEdit) -> CommandResult<SheetsEdited> {
    let next = match check(cx.doc, input) {
        Ok(next) => next,
        Err(stop) => return stop.into(),
    };
    let mut copy = cx.doc.reading_copy();
    match write(&mut copy, next) {
        Ok(mut output) => {
            output.revision = cx.doc.revision().to_string();
            CommandResult::Completed {
                output,
                warnings: Vec::new(),
            }
        }
        Err(stop) => stop.into(),
    }
}

/// Writes the changed book into the project.
pub fn execute(cx: &mut ExecutionContext<'_>, input: SheetsEdit) -> CommandResult<SheetsEdited> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_SHEETS_EDIT,
        kentos_commands::CAD_SHEETS_EDIT_VERSION,
        input,
        write_execute,
    )
}

/// [`execute`]'s write; `call::recorded` runs it as one step of the
/// document, all or nothing, and journals it (docs/adr/0253 §1, §3).
fn write_execute(cx: &mut ExecutionContext<'_>, input: SheetsEdit) -> CommandResult<SheetsEdited> {
    let next = match check(cx.doc, &input) {
        Ok(next) => next,
        Err(stop) => return stop.into(),
    };
    match write(cx.doc, next) {
        Ok(output) => CommandResult::Completed {
            output,
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}
