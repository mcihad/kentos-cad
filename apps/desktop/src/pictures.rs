//! Pictures in the drawing (docs/adr/0192): their bytes kept in the project's
//! library as PNG or JPEG images, named by their content (the same picture
//! once), or read from a linked file beside the drawing; what Öznitelikler
//! says of a picture's source; Göm, which embeds a linked picture. The
//! drawing's textures come from here too (`fetch`), each decoded once and
//! kept by the atlas's `Images`.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use kentos_contracts::Entity;
use kentos_domain::{Document as Model, Slot};
use kentos_render_wgpu::styled::Bitmap;
use serde_json::{Value, json};

/// The largest file a picture is read from (docs/adr/0192 §2).
pub const MOST_BYTES: usize = 32 * 1024 * 1024;

/// A picture's id in the project's library: `resim-` and its content's
/// SHA-256's first sixteen hex digits, so the same picture is kept once.
pub fn picture_id(bytes: &[u8]) -> String {
    let hex = kentos_sheet::template::sha256_hex(bytes);
    format!("resim-{}", &hex[..16])
}

/// A PNG or JPEG file as an image of the project's library: its id from its
/// content, its name the file's stem, under Resimler; why not when it is
/// neither or cannot be read.
pub fn library_item(name: &str, bytes: &[u8]) -> Result<(String, Value), String> {
    if bytes.len() > MOST_BYTES {
        return Err(format!(
            "“{name}” {} MB; resim en çok {} MB olabilir.",
            bytes.len() / (1024 * 1024),
            MOST_BYTES / (1024 * 1024)
        ));
    }
    let item = match crate::style::manager::files::raster_asset(name, bytes) {
        Some(Ok(item)) => item,
        Some(Err(why)) => return Err(why),
        None => {
            return Err(format!(
                "“{name}” PNG ya da JPEG değil; resim olarak eklenemez."
            ));
        }
    };
    let id = picture_id(bytes);
    let mut item = item;
    item["id"] = json!(id);
    item["path"] = json!(["Resimler"]);
    Ok((id, item))
}

/// Adds the image to the project's library unless it is there (an edit, no undo step; docs/adr/0092).
pub fn keep_in_library(model: &mut Model, id: &str, item: Value) {
    if kentos_native_application::edit::has_picture(model, id) {
        return;
    }
    let mut styles = model.styles().clone();
    styles.items.push(item);
    model.set_styles(styles);
}

/// A linked picture's file: as written when absolute, else beside the drawing.
pub fn resolve(file: &str, folder: Option<&Path>) -> PathBuf {
    let path = Path::new(file);
    match folder {
        Some(dir) if path.is_relative() => dir.join(path),
        _ => path.to_path_buf(),
    }
}

/// Öznitelikler's Kaynak for an embedded picture: its name and size in pixels.
pub fn asset_words(model: &Model, id: &str) -> String {
    let item = model.styles().items.iter().find(|it| {
        it.get("kind").and_then(Value::as_str) == Some("asset")
            && it.get("id").and_then(Value::as_str) == Some(id)
    });
    match item {
        Some(it) => {
            let name = it.get("name").and_then(Value::as_str).unwrap_or(id);
            let (w, h) = (
                it.get("width").and_then(Value::as_f64).unwrap_or(0.0),
                it.get("height").and_then(Value::as_f64).unwrap_or(0.0),
            );
            format!("Gömülü: {name} ({w}×{h} piksel)")
        }
        None => "Gömülü (kitaplıkta yok)".to_owned(),
    }
}

/// Öznitelikler's Kaynak for a linked picture: its file's name, and whether it was found.
pub fn file_words(file: &str, drawing: Option<&Path>) -> String {
    let path = resolve(file, drawing.and_then(Path::parent));
    let name = path
        .file_name()
        .map_or_else(|| file.to_owned(), |n| n.to_string_lossy().into_owned());
    if path.is_file() {
        format!("Bağlı: {name}")
    } else {
        format!("Bağlı: {name} (bulunamadı)")
    }
}

/// Göm: the linked picture at `slot` read and kept in the project's library,
/// the picture made embedded in one step “Değiştir”; why not otherwise.
pub fn embed(model: &mut Model, slot: Slot, folder: Option<&Path>) -> Vec<String> {
    let Some(Entity::Image(im)) = model.get(slot).cloned() else {
        return Vec::new();
    };
    let Some(file) = im.image.file.clone() else {
        return Vec::new();
    };
    let path = resolve(&file, folder);
    let name = path
        .file_name()
        .map_or_else(|| file.clone(), |n| n.to_string_lossy().into_owned());
    let bytes = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(e) => {
            return vec![format!(
                "“{name}” okunamadı: {e}. Dosyanın yerini denetleyin."
            )];
        }
    };
    let (id, item) = match library_item(&name, &bytes) {
        Ok(it) => it,
        Err(why) => return vec![why],
    };
    keep_in_library(model, &id, item);
    let mut picture = im.clone();
    picture.image.asset = Some(id);
    picture.image.file = None;
    kentos_interaction::properties::set_geometry(model, slot, &Entity::Image(picture))
}

/// A PNG's or a JPEG's pixels for the drawing.
pub fn decode(bytes: &[u8]) -> Option<Bitmap> {
    use kentos_render_wgpu::styled::picture::Picture;
    let picture = if bytes.starts_with(&[0x89, b'P', b'N', b'G']) {
        crate::style::images::png(bytes)
    } else if bytes.starts_with(&[0xFF, 0xD8]) {
        crate::style::images::jpeg(bytes)
    } else {
        None
    }?;
    match picture {
        Picture::Bitmap {
            width,
            height,
            rgba,
        } => Some(Bitmap {
            width,
            height,
            rgba,
        }),
        Picture::Vector { .. } => None,
    }
}

/// A `data:` address's bytes.
fn data_bytes(url: &str) -> Option<Vec<u8>> {
    let (_, data) = url.split_once(";base64,")?;
    kentos_sheet::template::base64_decode(data)
}

/// The pixels of the picture `key` names (docs/adr/0192 §3): an embedded
/// one from the project's library, a linked one from its file beside the
/// drawing (`folder`); none when it cannot be read.
pub fn fetch(key: &str, model: &Model, folder: Option<&Path>) -> Option<Arc<Bitmap>> {
    let bytes = if let Some(id) = key.strip_prefix("asset:") {
        model
            .styles()
            .items
            .iter()
            .find(|it| {
                it.get("kind").and_then(Value::as_str) == Some("asset")
                    && it.get("id").and_then(Value::as_str) == Some(id)
            })
            .and_then(|it| it.get("data").and_then(Value::as_str))
            .and_then(data_bytes)
    } else if let Some(file) = key.strip_prefix("file:") {
        std::fs::read(resolve(file, folder))
            .ok()
            .filter(|b| b.len() <= MOST_BYTES)
    } else {
        None
    };
    bytes.as_deref().and_then(decode).map(Arc::new)
}
