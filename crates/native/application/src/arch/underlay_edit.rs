//! `cad.underlay.edit` v1 (“Altlık model”, docs/adr/8101; plan/ifc-uyum.md IFC-09, B18): the drawing's underlay models
//! (`settings.arch.underlays`) as one undo step: an IFC file added (read for its SHA-256, size and schema, placed by its
//! map conversion onto the project's system or at the origin when asked), its labels, placement or showing set, its
//! file read again, the underlay taken away. The shared cases are `fixtures/commands/v1/cad.underlay.edit.json`
//! (`scripts/fixtures/underlay_cases.py`).
//!
//! The checks, in order (the first that fails answers): the ids' form (`invalid_uid`); the underlay named exists
//! (`model_underlay_not_found`), an added id does not (`duplicate_model_underlay`); the file's path is absolute
//! (`invalid_file_path`), it is there (`file_not_found`) and reads as IFC (`model_underlay_unreadable`); it is placed
//! (`model_underlay_not_placed`); a setting says something (`nothing_to_set`); the list's own checks
//! (`invalid_model_underlay`, its field's path); the count (`too_many_model_underlays`); the expected revision.

use std::path::Path;

use kentos_commands::encoding::sha256_hex;
use kentos_commands::{
    CommandResult, MAX_UNDERLAYS, ModelUnderlay, UnderlayChange, UnderlayEdit, UnderlayEdited,
    UnderlayPlacement,
};
use kentos_domain::{Document, Uuid};
use kentos_ifc::underlay::{self, MapFrame};

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;

/// The undo step's name.
pub const STEP: &str = "Altlık model";

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.into())))
}

/// A text as given: its edges trimmed, an empty one none.
fn said(t: &Option<String>) -> Option<String> {
    t.as_deref()
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .map(str::to_owned)
}

fn is_id(text: &str) -> bool {
    Uuid::parse_str(text).is_ok_and(|u| u.to_string() == text)
}

/// A file as read: its digest, size and schema.
struct Read {
    sha256: String,
    bytes: u64,
    schema: String,
}

/// The file at `path` read and placed `placement` in `doc`; refused at `at` (the path's field) or at `change.placement`.
fn read(doc: &Document, path: &str, placement: UnderlayPlacement, at: &str) -> Result<Read, Stop> {
    let file = Path::new(path);
    if path.trim().is_empty() || !file.is_absolute() {
        return Err(fail(
            codes::INVALID_FILE_PATH,
            format!(
                "“{path}” mutlak bir dosya yolu değil; dosyanın tam yolunu verin (/klasör/ad.ifc)."
            ),
            at,
        ));
    }
    let bytes = std::fs::read(file).map_err(|_| {
        fail(
            codes::FILE_NOT_FOUND,
            format!("{path} okunamadı: dosya yok ya da açılamıyor."),
            at,
        )
    })?;
    let name = file
        .file_name()
        .map_or_else(|| path.to_owned(), |n| n.to_string_lossy().into_owned());
    let model = kentos_ifc::read::read(&bytes, &kentos_ifc::read::Limits::default(), &|| false)
        .map_err(|e| {
            fail(
                codes::MODEL_UNDERLAY_UNREADABLE,
                format!("{name} bir IFC dosyası olarak okunamadı: {e}"),
                at,
            )
        })?;
    let unit = model
        .summary
        .units
        .length
        .as_ref()
        .map_or(f64::NAN, |u| u.to_si);
    let frame = model
        .summary
        .location
        .as_ref()
        .and_then(kentos_ifc::read::Location::frame);
    let project = crate::arch_ifc::crs(doc).map(|c| MapFrame::from(&c));
    let core = match placement {
        UnderlayPlacement::Map => underlay::Placement::Map,
        UnderlayPlacement::Origin => underlay::Placement::Origin,
    };
    underlay::place(
        frame.as_ref(),
        unit,
        project.as_ref().map_err(String::as_str),
        core,
    )
    .map_err(|why| {
        fail(
            codes::MODEL_UNDERLAY_NOT_PLACED,
            format!("{name}: {why}"),
            "change.placement",
        )
    })?;
    Ok(Read {
        sha256: sha256_hex(&bytes),
        bytes: bytes.len() as u64,
        schema: model.summary.schema.clone(),
    })
}

/// The underlays the change leaves and the changed one's id; refused with the first check that fails.
fn check(doc: &Document, input: &UnderlayEdit) -> Result<(Vec<ModelUnderlay>, String), Stop> {
    let mut next = doc.underlays().to_vec();
    let find = |list: &[ModelUnderlay], id: &str| -> Result<usize, Stop> {
        if !is_id(id) {
            return Err(fail(
                codes::INVALID_UID,
                format!("“{id}” küçük harfli, tireli bir UUID değil."),
                "change.id",
            ));
        }
        list.iter().position(|u| u.id == id).ok_or_else(|| {
            fail(
                codes::MODEL_UNDERLAY_NOT_FOUND,
                format!("“{id}” kimlikli altlık model çizimde yok."),
                "change.id",
            )
        })
    };
    let id = match &input.change {
        UnderlayChange::Add {
            id,
            file,
            name,
            discipline,
            revision,
            placement,
        } => {
            if let Some(id) = id {
                if !is_id(id) {
                    return Err(fail(
                        codes::INVALID_UID,
                        format!("“{id}” küçük harfli, tireli bir UUID değil."),
                        "change.id",
                    ));
                }
                if next.iter().any(|u| &u.id == id) {
                    return Err(fail(
                        codes::DUPLICATE_MODEL_UNDERLAY,
                        format!("“{id}” kimlikli bir altlık model çizimde zaten var."),
                        "change.id",
                    ));
                }
            }
            let placement = placement.unwrap_or_default();
            let path = file.trim();
            let got = read(doc, path, placement, "change.file")?;
            let id = id.clone().unwrap_or_else(|| Uuid::now_v7().to_string());
            let stem = Path::new(path)
                .file_name()
                .map_or_else(|| path.to_owned(), |n| n.to_string_lossy().into_owned());
            next.push(ModelUnderlay {
                id: id.clone(),
                name: said(name).unwrap_or(stem),
                file: path.to_owned(),
                sha256: got.sha256,
                bytes: got.bytes,
                schema: got.schema,
                discipline: said(discipline),
                revision: said(revision),
                placement,
                shown: true,
            });
            id
        }
        UnderlayChange::Set {
            id,
            name,
            discipline,
            revision,
            placement,
            shown,
        } => {
            let i = find(&next, id)?;
            if name.is_none()
                && discipline.is_none()
                && revision.is_none()
                && placement.is_none()
                && shown.is_none()
            {
                return Err(fail(
                    codes::NOTHING_TO_SET,
                    "Değiştirilecek bir şey verilmedi: ad, disiplin, revizyon, yerleşim ya da gösterim verin.".into(),
                    "change",
                ));
            }
            let u = &mut next[i];
            if let Some(n) = name {
                u.name = n.trim().to_owned();
            }
            if discipline.is_some() {
                u.discipline = said(discipline);
            }
            if revision.is_some() {
                u.revision = said(revision);
            }
            if let Some(s) = shown {
                u.shown = *s;
            }
            if let Some(p) = placement
                && *p != u.placement
            {
                let got = read(doc, &u.file.clone(), *p, "change.placement")?;
                (u.placement, u.sha256, u.bytes, u.schema) =
                    (*p, got.sha256, got.bytes, got.schema);
            }
            u.id.clone()
        }
        UnderlayChange::Reload { id, file } => {
            let i = find(&next, id)?;
            let path = said(file).unwrap_or_else(|| next[i].file.clone());
            let at = if file.is_some() {
                "change.file"
            } else {
                "change.id"
            };
            let got = read(doc, &path, next[i].placement, at)?;
            let u = &mut next[i];
            (u.file, u.sha256, u.bytes, u.schema) = (path, got.sha256, got.bytes, got.schema);
            u.id.clone()
        }
        UnderlayChange::Remove { id } => {
            let i = find(&next, id)?;
            next.remove(i).id
        }
    };
    if let Some((path, why)) = ModelUnderlay::list_problem(&next) {
        let i = next.iter().position(|u| u.id == id);
        let field = path.splitn(3, '.').nth(2).unwrap_or("");
        let path = match (i, field) {
            (Some(_), f) if !f.is_empty() => format!("change.{f}"),
            _ => "change".to_owned(),
        };
        let code = if next.len() > MAX_UNDERLAYS {
            codes::TOO_MANY_MODEL_UNDERLAYS
        } else {
            codes::INVALID_MODEL_UNDERLAY
        };
        return Err(fail(code, why, &path));
    }
    checks::revision(doc, input.expected_revision.as_deref())?;
    Ok((next, id))
}

pub fn validate(cx: &ExecutionContext<'_>, input: &UnderlayEdit) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// What it would write, nothing written.
pub fn plan(cx: &ExecutionContext<'_>, input: &UnderlayEdit) -> CommandResult<UnderlayEdited> {
    match check(cx.doc, input) {
        Ok((underlays, id)) => CommandResult::Completed {
            output: UnderlayEdited {
                id,
                underlays,
                revision: cx.doc.revision().to_string(),
            },
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: UnderlayEdit,
) -> CommandResult<UnderlayEdited> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_UNDERLAY_EDIT,
        kentos_commands::CAD_UNDERLAY_EDIT_VERSION,
        input,
        write_execute,
    )
}

fn write_execute(
    cx: &mut ExecutionContext<'_>,
    input: UnderlayEdit,
) -> CommandResult<UnderlayEdited> {
    let (underlays, id) = match check(cx.doc, &input) {
        Ok(next) => next,
        Err(stop) => return stop.into(),
    };
    let doc = &mut *cx.doc;
    if let Err(e) = doc.set_underlays(underlays.clone(), STEP) {
        return CommandResult::Failed {
            error: error(codes::INVALID_MODEL_UNDERLAY, e.0, Some("change".into())),
        };
    }
    CommandResult::Completed {
        output: UnderlayEdited {
            id,
            underlays,
            revision: doc.revision().to_string(),
        },
        warnings: Vec::new(),
    }
}
