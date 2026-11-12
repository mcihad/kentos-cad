//! `cad.attachments.inspect` v1 (docs/adr/0442 §7): what each attachment of some objects is, read from its
//! bytes, writing nothing. The shared cases are `fixtures/commands/v1/cad.attachments.inspect.json`.
//!
//! The checks, in order: one to 1 000 objects; each a UUID text of an object of the drawing; the folder
//! (when given) an absolute path. A photo of the library is read from its data, a file from the disk (a
//! relative path from the folder), a web address by its name only.

use std::path::{Path, PathBuf};

use kentos_commands::{
    Attachment, AttachmentFacts, AttachmentSource, AttachmentsInspect, AttachmentsInspected,
    CommandResult, InspectedObject, MOST_INSPECTED,
};
use kentos_domain::Document;

use crate::ExecutionContext;
use crate::checks::{Stop, error, is_uid_text};
use crate::codes;

/// The largest file whose bytes are read and hashed; a larger one gives its size alone.
const MOST_READ: u64 = 256 * 1024 * 1024;

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.to_owned())))
}

fn check(doc: &Document, input: &AttachmentsInspect) -> Result<(), Stop> {
    if input.uids.is_empty() || input.uids.len() > MOST_INSPECTED {
        return Err(fail(
            codes::INVALID_COUNT,
            format!(
                "Bir çağrıda 1 ile {MOST_INSPECTED} arasında nesne incelenir; {} verildi.",
                input.uids.len()
            ),
            "uids",
        ));
    }
    for (i, uid) in input.uids.iter().enumerate() {
        if !is_uid_text(uid) {
            return Err(fail(
                codes::INVALID_UID,
                format!(
                    "“{uid}” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir (01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi)."
                ),
                &format!("uids/{i}"),
            ));
        }
        let found = kentos_domain::Uuid::parse_str(uid)
            .ok()
            .and_then(|u| doc.slot_of(u))
            .is_some();
        if !found {
            return Err(fail(
                codes::ENTITY_NOT_FOUND,
                format!(
                    "“{uid}” kimlikli nesne çizimde yok; silinmiş olabilir. Çizimdeki bir nesnenin kimliğini verin."
                ),
                &format!("uids/{i}"),
            ));
        }
    }
    if let Some(folder) = &input.folder
        && !Path::new(folder).is_absolute()
    {
        return Err(fail(
            codes::INVALID_FOLDER,
            format!("“{folder}” mutlak bir yol değil; çizimin klasörünü tam yoluyla verin."),
            "folder",
        ));
    }
    Ok(())
}

/// The media type of `bytes` by their first bytes, else by `name`'s extension.
pub fn media_type(bytes: Option<&[u8]>, name: &str) -> Option<String> {
    if let Some(b) = bytes {
        let sniffed = if b.starts_with(b"\x89PNG\r\n\x1a\n") {
            Some("image/png")
        } else if b.starts_with(&[0xFF, 0xD8, 0xFF]) {
            Some("image/jpeg")
        } else if b.starts_with(b"GIF87a") || b.starts_with(b"GIF89a") {
            Some("image/gif")
        } else if b.starts_with(b"II*\0") || b.starts_with(b"MM\0*") {
            Some("image/tiff")
        } else if b.starts_with(b"%PDF-") {
            Some("application/pdf")
        } else if b.starts_with(b"PK\x03\x04") {
            Some(match extension(name).as_str() {
                "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
                "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
                "ods" => "application/vnd.oasis.opendocument.spreadsheet",
                "kmz" => "application/vnd.google-earth.kmz",
                _ => "application/zip",
            })
        } else {
            None
        };
        if let Some(m) = sniffed {
            return Some(m.to_owned());
        }
        return Some(
            by_extension(name)
                .unwrap_or("application/octet-stream")
                .to_owned(),
        );
    }
    by_extension(name).map(str::to_owned)
}

/// A name's extension, lowercase, without a query or fragment.
fn extension(name: &str) -> String {
    let end = name.find(['?', '#']).unwrap_or(name.len());
    let path = &name[..end];
    let file = path.rsplit('/').next().unwrap_or(path);
    file.rsplit_once('.')
        .map(|(_, e)| e.to_lowercase())
        .unwrap_or_default()
}

fn by_extension(name: &str) -> Option<&'static str> {
    Some(match extension(name).as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "tif" | "tiff" => "image/tiff",
        "pdf" => "application/pdf",
        "txt" => "text/plain",
        "csv" => "text/csv",
        "json" => "application/json",
        "xml" => "application/xml",
        "dxf" => "image/vnd.dxf",
        "kcad" => "application/vnd.kentos.kcad",
        "doc" => "application/msword",
        "xls" => "application/vnd.ms-excel",
        "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "ods" => "application/vnd.oasis.opendocument.spreadsheet",
        "kmz" => "application/vnd.google-earth.kmz",
        "zip" => "application/zip",
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "mp4" => "video/mp4",
        _ => return None,
    })
}

/// A library photo's bytes, from its data URL.
fn asset_bytes(doc: &Document, id: &str) -> Option<Vec<u8>> {
    let item = doc.styles().items.iter().find(|it| {
        it.get("kind").and_then(|k| k.as_str()) == Some("asset")
            && it.get("id").and_then(|k| k.as_str()) == Some(id)
    })?;
    let data = item.get("data")?.as_str()?;
    let (_, b64) = data.split_once(";base64,")?;
    kentos_commands::encoding::base64_decode(b64)
}

/// What one attachment is.
fn facts(doc: &Document, a: &Attachment, folder: Option<&Path>) -> AttachmentFacts {
    let mut out = AttachmentFacts {
        name: a.name.clone(),
        source: AttachmentSource::Url,
        mime: None,
        size: None,
        sha256: None,
        taken: None,
        latitude: None,
        longitude: None,
        missing: false,
    };
    let (bytes, label): (Option<Vec<u8>>, String) = if let Some(id) = &a.asset {
        out.source = AttachmentSource::Asset;
        let b = asset_bytes(doc, id);
        out.missing = b.is_none();
        (b, a.name.clone())
    } else if let Some(file) = &a.file {
        out.source = AttachmentSource::File;
        let path: Option<PathBuf> = if Path::new(file).is_absolute() {
            Some(PathBuf::from(file))
        } else {
            folder.map(|f| f.join(file))
        };
        let read = path.as_ref().and_then(|p| {
            let meta = std::fs::metadata(p).ok().filter(|m| m.is_file())?;
            if meta.len() > MOST_READ {
                out.size = Some(meta.len());
                return None;
            }
            std::fs::read(p).ok()
        });
        out.missing = read.is_none() && out.size.is_none();
        (read, file.clone())
    } else {
        (None, a.url.clone().unwrap_or_default())
    };
    out.mime = match (&bytes, out.missing, out.source) {
        // A file or photo not read says nothing of itself; a large file is named by its extension.
        (None, true, _) => None,
        (Some(b), _, _) => media_type(Some(b), &label),
        (None, false, _) => media_type(None, &label),
    };
    if let Some(b) = &bytes {
        out.size = Some(b.len() as u64);
        out.sha256 = Some(kentos_commands::encoding::sha256_hex(b));
        if matches!(out.mime.as_deref(), Some("image/jpeg" | "image/tiff")) {
            let exif = kentos_formats::exif::read(b);
            out.taken = exif.taken;
            out.latitude = exif.latitude;
            out.longitude = exif.longitude;
        }
    }
    out
}

/// Checks `input`.
pub fn validate(cx: &ExecutionContext<'_>, input: &AttachmentsInspect) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(()) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// The facts; the same answer as [`execute`].
pub fn plan(
    cx: &ExecutionContext<'_>,
    input: &AttachmentsInspect,
) -> CommandResult<AttachmentsInspected> {
    answer(cx.doc, input)
}

/// The facts. Writes nothing.
pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: AttachmentsInspect,
) -> CommandResult<AttachmentsInspected> {
    answer(cx.doc, &input)
}

fn answer(doc: &Document, input: &AttachmentsInspect) -> CommandResult<AttachmentsInspected> {
    if let Err(stop) = check(doc, input) {
        return stop.into();
    }
    let folder = input.folder.as_deref().map(Path::new);
    let objects = input
        .uids
        .iter()
        .map(|uid| {
            let attachments = kentos_domain::Uuid::parse_str(uid)
                .ok()
                .and_then(|u| doc.slot_of(u))
                .and_then(|s| doc.get(s))
                .map(|e| {
                    e.base()
                        .attachments
                        .iter()
                        .map(|a| facts(doc, a, folder))
                        .collect()
                })
                .unwrap_or_default();
            InspectedObject {
                uid: uid.clone(),
                attachments,
            }
        })
        .collect();
    CommandResult::Completed {
        output: AttachmentsInspected { objects },
        warnings: Vec::new(),
    }
}
