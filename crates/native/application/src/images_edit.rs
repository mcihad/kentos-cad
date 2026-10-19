//! `cad.images.edit` v1 (docs/adr/0350 §1; plan/cad.md `CAD-REF-16`): an image's source changed. `embed` (Göm) reads
//! linked images' files, looked for as external references' sources are (`links::find`), into the project's library
//! as pictures named by their content (the desktop's `pictures::library_item`) and makes them embedded; `relink`
//! (Yeniden bağla) points images at another PNG or JPEG. Images on a locked layer are left out and counted
//! (docs/adr/0278); the files are read whole or the command writes nothing. The shared cases are
//! `fixtures/commands/v1/cad.images.edit.json`.

use std::path::Path;

use kentos_contracts::encoding::{base64_encode, sha256_hex};
use kentos_contracts::{
    CommandResult, CommandWarning, Entity, ImageSourced, ImagesEdit, ImagesEdited, ImagesOperation, MAX_IMAGE_BYTES,
    codes,
};
use kentos_domain::{Document, Slot};
use serde_json::{Value, json};

use crate::ExecutionContext;
use crate::checks::{self, Stop};
use crate::links::find;

/// A picture's format and its size in pixels, read from its header: a PNG's IHDR, a JPEG's frame header. None when
/// the bytes are neither, or say no size.
pub fn picture_size(bytes: &[u8]) -> Option<(&'static str, u32, u32)> {
    let be16 = |i: usize| -> Option<u32> { Some(u32::from(u16::from_be_bytes([*bytes.get(i)?, *bytes.get(i + 1)?]))) };
    let be32 = |i: usize| -> Option<u32> { Some(u32::from_be_bytes(bytes.get(i..i + 4)?.try_into().ok()?)) };
    if bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]) {
        if bytes.get(12..16)? != b"IHDR" {
            return None;
        }
        let (w, h) = (be32(16)?, be32(20)?);
        return (w > 0 && h > 0).then_some(("png", w, h));
    }
    if !bytes.starts_with(&[0xFF, 0xD8]) {
        return None;
    }
    let mut i = 2;
    loop {
        if *bytes.get(i)? != 0xFF {
            return None;
        }
        // Fill bytes before the marker.
        while *bytes.get(i + 1)? == 0xFF {
            i += 1;
        }
        let marker = *bytes.get(i + 1)?;
        if marker == 0x01 || (0xD0..=0xD7).contains(&marker) {
            i += 2;
            continue;
        }
        if marker == 0xD9 || marker == 0xDA {
            return None;
        }
        let len = be16(i + 2)? as usize;
        if len < 2 {
            return None;
        }
        if matches!(marker, 0xC0..=0xC3 | 0xC5..=0xC7 | 0xC9..=0xCB | 0xCD..=0xCF) {
            let (h, w) = (be16(i + 5)?, be16(i + 7)?);
            return (w > 0 && h > 0).then_some(("jpeg", w, h));
        }
        i += 2 + len;
    }
}

/// A picture's id in the project's library: `resim-` and its content's SHA-256's first sixteen hex digits, so the
/// same picture is kept once (docs/adr/0192 §2).
pub fn picture_id(bytes: &[u8]) -> String {
    format!("resim-{}", &sha256_hex(bytes)[..16])
}

/// A PNG's or JPEG's bytes as an image of the project's library, under Resimler, named by the file's stem.
pub fn library_item(name: &str, bytes: &[u8], format: &str, width: u32, height: u32) -> (String, Value) {
    let stem = name
        .rsplit_once('.')
        .map_or(name, |(stem, ext)| {
            if matches!(ext.to_ascii_lowercase().as_str(), "png" | "jpg" | "jpeg") && !stem.is_empty() {
                stem
            } else {
                name
            }
        })
        .to_owned();
    let id = picture_id(bytes);
    let item = json!({
        "kind": "asset",
        "id": id,
        "name": stem,
        "path": ["Resimler"],
        "format": format,
        "data": format!("data:image/{format};base64,{}", base64_encode(bytes)),
        "width": width,
        "height": height,
    });
    (id, item)
}

fn refuse(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(checks::error(code, message, Some(path.into())))
}

fn file_name(path: &str) -> String {
    Path::new(path).file_name().map_or_else(|| path.to_owned(), |n| n.to_string_lossy().into_owned())
}

/// A file read for an image: where from, its bytes, format and size.
struct Read {
    at: String,
    bytes: Vec<u8>,
    format: &'static str,
    width: u32,
    height: u32,
}

/// `file` looked for (with the project's search paths when `search` is given) and read as a PNG or JPEG.
fn read(file: &str, drawing: Option<&str>, search: &[String], at: &str) -> Result<Read, Stop> {
    let text = file.trim();
    if text.is_empty() {
        return Err(refuse(codes::INVALID_FILE_PATH, "Resmin dosya yolu boş; bir PNG ya da JPEG dosyasının yolunu verin.".into(), at));
    }
    if Path::new(text).is_relative() && drawing.is_none() {
        return Err(refuse(
            codes::INVALID_FILE_PATH,
            format!("“{text}” göreli bir yol, ama çizimin dosyası yok; mutlak yolu verin ya da çizimi önce kaydedin."),
            at,
        ));
    }
    let found = find(text, None, drawing, search);
    let Some(p) = found.at else {
        return Err(refuse(
            codes::FILE_NOT_FOUND,
            format!("{} bulunamadı. Bakılan yerler: {}.", file_name(text), found.tried.join(", ")),
            at,
        ));
    };
    let shown = p.display().to_string();
    let size = std::fs::metadata(&p).map(|m| m.len()).unwrap_or(0);
    if size > MAX_IMAGE_BYTES {
        return Err(refuse(
            codes::IMPORT_UNREADABLE,
            format!(
                "{} {} MB; resim en çok {} MB olabilir.",
                file_name(text),
                size / (1024 * 1024),
                MAX_IMAGE_BYTES / (1024 * 1024)
            ),
            at,
        ));
    }
    let bytes = std::fs::read(&p)
        .map_err(|e| refuse(codes::FILE_NOT_FOUND, format!("{shown} okunamadı: {e}."), at))?;
    let Some((format, width, height)) = picture_size(&bytes) else {
        return Err(refuse(
            codes::IMPORT_UNREADABLE,
            format!("{} PNG ya da JPEG değil ya da bozuk; resim olarak okunamadı.", file_name(text)),
            at,
        ));
    };
    Ok(Read { at: shown, bytes, format, width, height })
}

/// What the command writes: the library's new pictures, then each image as it will be.
struct Planned {
    assets: Vec<(String, Value)>,
    images: Vec<(Slot, Entity, ImageSourced)>,
    warnings: Vec<CommandWarning>,
}

fn check(doc: &Document, input: &ImagesEdit) -> Result<Planned, Stop> {
    let drawing = input.drawing.as_deref();
    let search = doc.settings().xref_paths.clone();
    // The images named, or (Göm with none) every linked image of the drawing.
    let all: Vec<String>;
    let uids: &[String] = match &input.operation {
        ImagesOperation::Embed { uids: Some(u) } | ImagesOperation::Relink { uids: u, .. } => {
            checks::uids(u, "Kaynağı değişecek resim verilmedi.")?;
            u
        }
        ImagesOperation::Embed { uids: None } => {
            all = doc
                .entities()
                .filter(|e| matches!(e, Entity::Image(i) if i.image.file.is_some()))
                .filter_map(|e| doc.uid(Slot(e.base().id)).map(|u| u.to_string()))
                .collect();
            &all
        }
    };
    let named = checks::named(doc, uids)?;
    for (at, _, e, uid) in &named {
        if !matches!(e, Entity::Image(_)) {
            return Err(refuse(
                codes::NOT_AN_IMAGE,
                format!("“{uid}” kimlikli nesne bir resim değil; yalnız resmin kaynağı değişir."),
                &format!("uids[{at}]"),
            ));
        }
    }
    let mut warnings = Vec::new();
    let open = if named.is_empty() {
        named
    } else {
        let (open, warning) = checks::skip_locked(doc, named)?;
        warnings.extend(warning);
        open
    };
    let mut assets: Vec<(String, Value)> = Vec::new();
    let mut images = Vec::new();
    match &input.operation {
        ImagesOperation::Embed { uids: given } => {
            let linked: Vec<_> = open
                .into_iter()
                .filter(|(_, _, e, _)| matches!(e, Entity::Image(i) if i.image.file.is_some()))
                .collect();
            if linked.is_empty() {
                let path = if given.is_some() { "uids" } else { "operation" };
                return Err(refuse(
                    codes::NOTHING_TO_EMBED,
                    if given.is_some() {
                        "Verilen resimler zaten gömülü; gömülecek bağlı resim yok.".into()
                    } else {
                        "Çizimde gömülecek bağlı resim yok.".into()
                    },
                    path,
                ));
            }
            for (at, slot, e, uid) in linked {
                let Entity::Image(im) = e else { continue };
                let file = im.image.file.clone().unwrap_or_default();
                let place = if given.is_some() { format!("uids[{at}]") } else { "operation".to_owned() };
                let r = read(&file, drawing, &search, &place)?;
                let (id, item) = library_item(&file_name(&file), &r.bytes, r.format, r.width, r.height);
                if !crate::edit::has_picture(doc, &id) && !assets.iter().any(|(a, _)| *a == id) {
                    assets.push((id.clone(), item));
                }
                let mut next = im.clone();
                next.image.asset = Some(id.clone());
                next.image.file = None;
                images.push((
                    slot,
                    Entity::Image(next),
                    ImageSourced {
                        uid: uid.clone(),
                        asset: Some(id),
                        file: None,
                        read: r.at,
                        width: r.width,
                        height: r.height,
                    },
                ));
            }
        }
        ImagesOperation::Relink { file, .. } => {
            let r = read(file, drawing, &[], "operation.file")?;
            let kept = file.trim().to_owned();
            for (_, slot, e, uid) in open {
                let Entity::Image(im) = e else { continue };
                let mut next = im.clone();
                next.image.asset = None;
                next.image.file = Some(kept.clone());
                images.push((
                    slot,
                    Entity::Image(next),
                    ImageSourced {
                        uid: uid.clone(),
                        asset: None,
                        file: Some(kept.clone()),
                        read: r.at.clone(),
                        width: r.width,
                        height: r.height,
                    },
                ));
            }
        }
    }
    checks::revision(doc, input.expected_revision.as_deref())?;
    Ok(Planned { assets, images, warnings })
}

/// The images in the drawing's order.
fn in_order(mut images: Vec<(Slot, Entity, ImageSourced)>) -> Vec<(Slot, Entity, ImageSourced)> {
    images.sort_by_key(|(slot, ..)| slot.0);
    images
}

pub fn validate(cx: &ExecutionContext<'_>, input: &ImagesEdit) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(p) => CommandResult::Completed { output: (), warnings: p.warnings },
        Err(stop) => stop.into(),
    }
}

pub fn plan(cx: &ExecutionContext<'_>, input: &ImagesEdit) -> CommandResult<ImagesEdited> {
    match check(cx.doc, input) {
        Ok(p) => CommandResult::Completed {
            output: ImagesEdited {
                images: in_order(p.images).into_iter().map(|(.., s)| s).collect(),
                assets: p.assets.into_iter().map(|(id, _)| id).collect(),
                revision: cx.doc.revision().to_string(),
            },
            warnings: p.warnings,
        },
        Err(stop) => stop.into(),
    }
}

/// Checks `input` again and writes it: the library's new pictures (a library edit, not an undo step, as Göm's always
/// were), then the images as one undo step “Göm” or “Yeniden bağla”.
pub fn execute(cx: &mut ExecutionContext<'_>, input: ImagesEdit) -> CommandResult<ImagesEdited> {
    crate::call::recorded(
        cx,
        kentos_contracts::CAD_IMAGES_EDIT,
        kentos_contracts::CAD_IMAGES_EDIT_VERSION,
        input,
        write_execute,
    )
}

fn write_execute(cx: &mut ExecutionContext<'_>, input: ImagesEdit) -> CommandResult<ImagesEdited> {
    let Planned { assets, images, warnings } = match check(cx.doc, &input) {
        Ok(p) => p,
        Err(stop) => return stop.into(),
    };
    let label = match input.operation {
        ImagesOperation::Embed { .. } => "Göm",
        ImagesOperation::Relink { .. } => "Yeniden bağla",
    };
    if !assets.is_empty() {
        let mut styles = cx.doc.styles().clone();
        styles.items.extend(assets.iter().map(|(_, item)| item.clone()));
        cx.doc.set_styles(styles);
    }
    let images = in_order(images);
    let mut out = Vec::with_capacity(images.len());
    let writes: Vec<(Slot, Entity)> = images
        .into_iter()
        .map(|(slot, e, s)| {
            out.push(s);
            (slot, e)
        })
        .collect();
    let _ = cx.doc.transact(label, |doc| -> Result<(), ()> {
        for (slot, e) in writes {
            doc.update(slot, e);
        }
        Ok(())
    });
    CommandResult::Completed {
        output: ImagesEdited {
            images: out,
            assets: assets.into_iter().map(|(id, _)| id).collect(),
            revision: cx.doc.revision().to_string(),
        },
        warnings,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes_come_from_the_headers() {
        let mut png = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0, 0, 0, 13];
        png.extend_from_slice(b"IHDR");
        png.extend_from_slice(&640u32.to_be_bytes());
        png.extend_from_slice(&480u32.to_be_bytes());
        assert_eq!(picture_size(&png), Some(("png", 640, 480)));
        // A JPEG: SOI, an APP0 of four bytes, then a baseline frame header.
        let jpeg = [
            0xFF, 0xD8, 0xFF, 0xE0, 0, 4, 0, 0, 0xFF, 0xC0, 0, 11, 8, 0x01, 0x2C, 0x01, 0x90, 3,
        ];
        assert_eq!(picture_size(&jpeg), Some(("jpeg", 400, 300)));
        assert_eq!(picture_size(b"GIF89a"), None);
        assert_eq!(picture_size(&jpeg[..9]), None);
    }
}
