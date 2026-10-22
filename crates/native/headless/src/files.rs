//! `.kcad` files, read and written as the desktop does (docs/adr/0025,
//! 0030): a v2 file or an old v1 JSON one is read, whatever its name says;
//! a save writes v2 whose bytes were read back to the same drawing
//! (`encode_verified`), into a new temporary file beside the target that is
//! flushed to the disk and only then renamed over it. A save that fails
//! leaves the previous file as it was.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use kentos_commands::DocumentSnapshotV1;
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

/// Writes `doc` as `.kcad` v2 to `path`; the bytes written. Bound GeoPackages get their path from the file's folder
/// (docs/adr/0506), external references theirs (docs/adr/0348).
pub fn write(doc: &Document, path: &Path) -> Result<usize, HeadlessError> {
    let mut snapshot = doc.to_snapshot_v2();
    kentos_native_application::source_places::with_relative(&mut snapshot.settings, path);
    kentos_native_application::xrefs::with_relative(&mut snapshot.blocks, path);
    let bytes = kentos_kcad::encode_verified(&snapshot).map_err(|e| {
        HeadlessError::new(
            "file_unwritable",
            format!("çizim KCAD v2'ye yazılamadı: {e}"),
        )
    })?;
    write_bytes(&bytes, path)
}

/// Writes `bytes` to `path` through a temporary file beside it, read back
/// before it takes the name: a file is whole or not there. The bytes written.
pub fn write_bytes(bytes: &[u8], path: &Path) -> Result<usize, HeadlessError> {
    write_bytes_with_overwrite(bytes, path, true)
}

/// Commit with an explicit replacement policy. A no-replace commit is atomic
/// even if another process creates the destination after the plan was made.
pub fn write_bytes_with_overwrite(
    bytes: &[u8], path: &Path, overwrite: bool,
) -> Result<usize, HeadlessError> {
    let fail = |what: String| HeadlessError::new("file_unwritable", what);
    let dir = match path.parent() {
        Some(d) if !d.as_os_str().is_empty() => d.to_path_buf(),
        _ => PathBuf::from("."),
    };
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "cizim".to_owned());
    let temp = dir.join(format!(".{name}.{}.yaziliyor", kentos_domain::Uuid::now_v7()));
    let result = (|| {
        let mut file = fs::OpenOptions::new().write(true).create_new(true).open(&temp)
            .map_err(|e| fail(format!("{} yazılamadı: {e}", temp.display())))?;
        file.write_all(bytes)
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
        let committed = if overwrite {
            fs::rename(&temp, path)
        } else {
            // A hard-link publishes the fully flushed inode under a name that
            // must not exist. Unlike exists()+rename(), it cannot overwrite a
            // destination won by another writer. Both names are on one volume.
            fs::hard_link(&temp, path)
        };
        committed.map_err(|e| {
            if !overwrite && e.kind() == std::io::ErrorKind::AlreadyExists {
                return HeadlessError::new("file_exists", format!(
                    "{} başka bir işlem tarafından oluşturuldu; üzerine yazılmadı.", path.display()));
            }
            fail(format!(
                "{} yerine konamadı: {e}. Hedef klasörün yazılabilir olduğunu denetleyin.",
                path.display()
            ))
        })?;
        if !overwrite { let _ = fs::remove_file(&temp); }
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result.map(|()| bytes.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A folder of its own under the system's temporary one.
    fn folder(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("kentos-yaz-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).expect("a folder");
        d
    }

    fn names(d: &Path) -> Vec<String> {
        let mut n: Vec<String> = fs::read_dir(d)
            .expect("lists")
            .map(|e| {
                e.expect("an entry")
                    .file_name()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect();
        n.sort();
        n
    }

    /// A write that fails after its bytes are down (the name cannot be
    /// taken) leaves what was there whole and no temporary file.
    #[test]
    fn a_write_that_fails_leaves_no_half_file() {
        let d = folder("yarim");
        let target = d.join("hedef.dxf");
        fs::create_dir(&target).expect("a folder in the way");
        fs::write(target.join("icinde"), b"eski").expect("writes");
        let e = write_bytes(b"yeni", &target).expect_err("a folder's name is not taken");
        assert_eq!(e.code, "file_unwritable");
        assert_eq!(fs::read(target.join("icinde")).expect("reads"), b"eski");
        assert_eq!(names(&d), ["hedef.dxf"], "no temporary file left");
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn competing_no_replace_writers_have_one_winner_and_no_partial_file() {
        let d = folder("yarisan");
        let target = d.join("dosya.dwg");
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(8));
        let jobs: Vec<_> = (0..8).map(|i| {
            let (path, barrier) = (target.clone(), barrier.clone());
            std::thread::spawn(move || {
                let bytes = vec![i as u8; 1024 * 64];
                barrier.wait();
                (i as u8, write_bytes_with_overwrite(&bytes, &path, false))
            })
        }).collect();
        let done: Vec<_> = jobs.into_iter().map(|j| j.join().expect("writer")).collect();
        let winners: Vec<_> = done.iter().filter(|(_, r)| r.is_ok()).collect();
        assert_eq!(winners.len(), 1);
        assert_eq!(fs::read(&target).expect("complete file"), vec![winners[0].0; 1024 * 64]);
        for (_, result) in done { if let Err(e) = result { assert_eq!(e.code, "file_exists"); } }
        assert_eq!(names(&d), ["dosya.dwg"]);
        fs::remove_dir_all(d).expect("cleanup");
    }

    /// A file there is replaced whole; the bytes read back are the ones meant.
    #[test]
    fn a_file_there_is_replaced_whole() {
        let d = folder("yerine");
        let target = d.join("dosya.dxf");
        fs::write(&target, b"eski icerik, daha uzun").expect("writes");
        assert_eq!(write_bytes(b"yeni", &target).expect("written"), 4);
        assert_eq!(fs::read(&target).expect("reads"), b"yeni");
        assert_eq!(names(&d), ["dosya.dxf"]);
        let _ = fs::remove_dir_all(&d);
    }

    /// A folder it may not write to: nothing made, nothing left.
    #[cfg(unix)]
    #[test]
    fn a_folder_it_may_not_write_to_gets_nothing() {
        use std::os::unix::fs::PermissionsExt;
        let d = folder("salt-okunur");
        fs::set_permissions(&d, fs::Permissions::from_mode(0o555)).expect("read only");
        let e = write_bytes(b"yeni", &d.join("dosya.dxf"));
        fs::set_permissions(&d, fs::Permissions::from_mode(0o755)).expect("writable again");
        // Root writes anyway; then the file is whole, which is the rule too.
        if let Err(e) = e {
            assert_eq!(e.code, "file_unwritable");
            assert!(names(&d).is_empty(), "nothing made");
        }
        let _ = fs::remove_dir_all(&d);
    }
}
