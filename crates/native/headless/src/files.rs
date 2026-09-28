//! `.kcad` files, read and written as the desktop does (docs/adr/0025,
//! 0030): a v2 file or an old v1 JSON one is read, whatever its name says;
//! a save writes v2 whose bytes were read back to the same drawing
//! (`encode_verified`), into a new temporary file beside the target that is
//! flushed to the disk and only then renamed over it. A save that fails
//! leaves the previous file as it was.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use kentos_contracts::DocumentSnapshotV1;
use kentos_domain::Document;
use kentos_kcad::Sniff;

use crate::HeadlessError;

/// Reads a drawing from a file's bytes; `legacy` is true for a v1 JSON file.
pub fn read(path: &Path) -> Result<(Document, bool), HeadlessError> {
    let bytes = fs::read(path).map_err(|e| {
        HeadlessError::new(
            "file_unreadable",
            format!(
                "{} okunamadı: {e}. Yolu ve izinleri denetleyin.",
                path.display()
            ),
        )
    })?;
    from_bytes(&bytes)
        .map_err(|e| HeadlessError::new(e.code, format!("{}: {}", path.display(), e.message)))
}

/// Reads a drawing from bytes: `.kcad` v2, or v1 JSON.
pub fn from_bytes(bytes: &[u8]) -> Result<(Document, bool), HeadlessError> {
    match kentos_kcad::sniff(bytes) {
        Sniff::Kcad => {
            let snapshot = kentos_kcad::decode(bytes).map_err(|e| {
                HeadlessError::new("file_unreadable", format!("KCAD v2 dosyası okunamadı: {e}"))
            })?;
            let doc = Document::from_snapshot_v2(snapshot)
                .map_err(|e| HeadlessError::new("file_unreadable", e))?;
            Ok((doc, false))
        }
        Sniff::Json => {
            let text = std::str::from_utf8(bytes)
                .map_err(|_| HeadlessError::new("file_unreadable", "JSON dosyası UTF-8 değil."))?;
            let snapshot = DocumentSnapshotV1::from_json(text)
                .map_err(|e| HeadlessError::new("file_unreadable", e))?;
            let doc = Document::from_snapshot(snapshot)
                .map_err(|e| HeadlessError::new("file_unreadable", e))?;
            Ok((doc, true))
        }
        Sniff::KcadDamaged => Err(HeadlessError::new(
            "file_unreadable",
            "KCAD imzası bozuk: dosya metin olarak aktarılırken bozulmuş olabilir. Özgün kopyayı kullanın.",
        )),
        Sniff::Empty => Err(HeadlessError::new("file_unreadable", "Dosya boş.")),
        Sniff::Foreign => Err(HeadlessError::new(
            "file_unreadable",
            "Bu bir KentOS çizimi (.kcad) değil. DXF, GeoJSON gibi dosyalar içe aktarılır, açılmaz.",
        )),
    }
}

/// Writes `doc` as `.kcad` v2 to `path`; the bytes written.
pub fn write(doc: &Document, path: &Path) -> Result<usize, HeadlessError> {
    let fail = |what: String| HeadlessError::new("file_unwritable", what);
    let bytes = kentos_kcad::encode_verified(&doc.to_snapshot_v2())
        .map_err(|e| fail(format!("çizim KCAD v2'ye yazılamadı: {e}")))?;
    let dir = match path.parent() {
        Some(d) if !d.as_os_str().is_empty() => d.to_path_buf(),
        _ => PathBuf::from("."),
    };
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "cizim.kcad".to_owned());
    let temp = dir.join(format!(".{name}.{}.yaziliyor", std::process::id()));
    let result = (|| {
        let mut file = fs::File::create(&temp)
            .map_err(|e| fail(format!("{} yazılamadı: {e}", temp.display())))?;
        file.write_all(&bytes)
            .and_then(|()| file.sync_all())
            .map_err(|e| fail(format!("{} yazılamadı: {e}", temp.display())))?;
        drop(file);
        // What reached the disk is what was meant.
        let back =
            fs::read(&temp).map_err(|e| fail(format!("{} geri okunamadı: {e}", temp.display())))?;
        if back != bytes {
            return Err(fail(format!(
                "{} diske yazılanla aynı değil; kayıt yapılmadı.",
                temp.display()
            )));
        }
        fs::rename(&temp, path).map_err(|e| {
            fail(format!(
                "{} yerine konamadı: {e}. Hedef klasörün yazılabilir olduğunu denetleyin.",
                path.display()
            ))
        })
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result.map(|()| bytes.len())
}
