//! `cad.sheets.edit` v1 (docs/adr/0252 §2): the sheet core's own operations
//! (`kentos_sheet::ops`, the ones the sheet designer makes) applied to the
//! project's sheet book as one change. The book is read as a project keeps
//! it (`kentos_sheet::project`), changed by the core's `apply`, checked by
//! it, and written back with its pictures' bytes; a book left empty is no
//! book. The book is the paper's: an edit of the project, no undo step of
//! the drawing (docs/adr/0251 §3); the answer's `inverse` takes it back. The
//! shared cases are in `fixtures/commands/v1/cad.sheets.edit.json`.

use kentos_contracts::{CommandResult, SheetInfo, SheetMapInfo, SheetsEdit, SheetsEdited};
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
}

fn check(doc: &Document, input: &SheetsEdit) -> Result<Next, Stop> {
    if input.ops.is_empty() {
        return Err(Stop::Failed(error(
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
                return Err(Stop::Failed(error(
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
    let (mut book, pictures) = match doc.sheets() {
        None => (SheetBook::default(), Vec::new()),
        Some(kept) => read_project_sheets(kept).map_err(|e| {
            Stop::Failed(error(
                codes::BOOK_UNREADABLE,
                format!(
                    "Bu projenin pafta kitabı okunamadı: {} Kitap olduğu gibi duruyor; ona işlem yazılmaz.",
                    e.message
                ),
                None,
            ))
        })?,
    };
    // One by one, so a refusal names its operation; the inverse undoes them all, last first.
    let mut inverse: Vec<Op> = Vec::new();
    let mut labels = Vec::new();
    for (i, op) in parsed.iter().enumerate() {
        let applied = ops::apply(&book, op).map_err(|e| {
            let at = match &e.path {
                Some(p) => format!("ops[{i}].{p}"),
                None => format!("ops[{i}]"),
            };
            Stop::Failed(error(&e.code, e.message, Some(at)))
        })?;
        book = applied.book;
        labels.push(applied.label);
        let mut back = applied.inverse;
        back.extend(inverse);
        inverse = back;
    }
    let label = match labels.as_slice() {
        [one] => one.clone(),
        _ => format!("{} işlem", labels.len()),
    };
    Ok(Next {
        book,
        pictures,
        label,
        inverse,
    })
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
                            MapView::Fixed(f) => Some(f.scale),
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
        Some(kept) => read_project_sheets(kept).ok().map(|(book, _)| sheets_of(&book)),
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
        .map_err(|e| Stop::Failed(error(&e.code, e.message, e.path)))?;
    doc.set_sheets(kept)
        .map_err(|e| Stop::Failed(error(codes::BOOK_UNREADABLE, e, None)))?;
    Ok(SheetsEdited {
        label: next.label,
        inverse: next
            .inverse
            .iter()
            .filter_map(|op| serde_json::to_value(op).ok())
            .collect(),
        sheets: sheets_of(&next.book),
        revision: doc.revision().to_string(),
    })
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
