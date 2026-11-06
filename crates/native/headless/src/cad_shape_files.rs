//! SHX dependencies beside an exported DXF/DWG. Only embedded, checked bytes
//! can be written; source filenames never authorize outside path lookups.
use crate::{
    file_commands::{Refused, refuse},
    files,
};
use kentos_commands::{codes, encoding::sha256_hex, shape_resource};
use kentos_domain::Document;
use serde_json::{Value, json};
use std::{
    collections::BTreeSet,
    fs,
    io::Read,
    path::{Path, PathBuf},
};

pub(crate) struct Source {
    file: String,
    path: PathBuf,
    bytes: Option<Vec<u8>>,
    reused: bool,
}

/// Equal existing resources are reused. A different file, directory or symlink
/// is never replaced by a CAD export, even when replacing the drawing itself.
fn existing(path: &Path, bytes: &[u8]) -> Result<bool, Refused> {
    let meta = match fs::symlink_metadata(path) {
        Ok(m) => m,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(e) => return Err(refuse(codes::FILE_UNREADABLE, e.to_string(), "path")),
    };
    if meta.is_file() && !meta.file_type().is_symlink() && meta.len() == bytes.len() as u64 {
        let mut actual = Vec::new();
        fs::File::open(path)
            .and_then(|f| {
                f.take(shape_resource::MAX_BYTES as u64 + 1)
                    .read_to_end(&mut actual)
            })
            .map_err(|e| refuse(codes::FILE_UNREADABLE, e.to_string(), "path"))?;
        if actual == bytes {
            return Ok(true);
        }
    }
    Err(refuse(
        "shape_resource_conflict",
        format!(
            "“{}” SHX kaynağıyla aynı dosya değil; mevcut dosya değiştirilmedi. Çizimi başka bir klasöre aktarın.",
            path.display()
        ),
        "path",
    ))
}

pub(crate) fn plan(doc: &Document, target: &Path) -> Result<Vec<Source>, Refused> {
    let parent = target
        .parent()
        .ok_or_else(|| refuse(codes::INVALID_FILE_PATH, "Çıktı klasörü yok.", "path"))?;
    let mut out = Vec::new();
    let mut seen = BTreeSet::new();
    let mut total = 0usize;
    let resource_of = |id: &str| {
        doc.styles()
            .items
            .iter()
            .find(|i| shape_resource::is_resource(i) && i["id"].as_str() == Some(id))
    };
    // The SHX fonts and Big Fonts of the drawing's text styles (ADR 1125 §5): every one the project embeds goes
    // beside the drawing its STYLE table names it in. One not embedded is the receiver's font, as before: not a loss
    // of this export, and not looked up on disk.
    let fonts: Vec<&str> = doc
        .settings()
        .text_styles
        .iter()
        .flat_map(|s| {
            let font = s.font_file.as_deref().filter(|_| kentos_native_application::shx_text::is_shx(s));
            let big = s.big_font_file.as_deref().filter(|f| f.to_ascii_lowercase().ends_with(".shx"));
            [font, big]
        })
        .flatten()
        .filter(|f| resource_of(&shape_resource::id(shape_resource::source_name(f))).is_some())
        .collect();
    let shapes = doc
        .settings()
        .line_types
        .iter()
        .flat_map(|p| &p.elements)
        .filter_map(|e| e.shape.as_ref())
        .map(|s| s.file.as_str());
    for named in shapes.chain(fonts) {
        let id = shape_resource::id(shape_resource::source_name(named));
        if !seen.insert(id.clone()) {
            continue;
        }
        let Some(resource) = resource_of(&id) else {
            out.push(Source {
                file: named.to_owned(),
                path: PathBuf::new(),
                bytes: None,
                reused: false,
            });
            continue;
        };
        let bytes = shape_resource::read(resource)
            .map_err(|e| refuse("invalid_shape_resource", e, "styles"))?;
        let file = kentos_geometry_core::text::shx::ShapeFile::read(&bytes)
            .map_err(|e| refuse("invalid_shape_resource", e, "styles"))?;
        // Check every used name, not just the first name in this file.
        for other in doc
            .settings()
            .line_types
            .iter()
            .flat_map(|p| &p.elements)
            .filter_map(|e| e.shape.as_ref())
            .filter(|s| shape_resource::id(shape_resource::source_name(&s.file)) == id)
        {
            if file.number(&other.name).is_none() {
                return Err(refuse(
                    "shape_not_found",
                    format!(
                        "“{}” şekli “{}” kaynağında yok; CAD dosyası yazılmadı.",
                        other.name, other.file
                    ),
                    "lineTypes",
                ));
            }
        }
        total = total.saturating_add(bytes.len());
        if total > 64 * 1024 * 1024 {
            return Err(refuse(
                "shape_resource_limit",
                "CAD aktarımındaki SHX kaynakları 64 MiB sınırını aşıyor.",
                "styles",
            ));
        }
        let name = resource["file"].as_str().unwrap_or_default().to_owned();
        let path = parent.join(&name);
        if target
            .file_name()
            .is_some_and(|n| n.to_string_lossy().to_lowercase() == name.to_lowercase())
        {
            return Err(refuse(
                "shape_resource_conflict",
                "Çizim ve SHX kaynağı aynı dosya yoluna yazılamaz.",
                "path",
            ));
        }
        let reused = existing(&path, &bytes)?;
        out.push(Source {
            file: name,
            path,
            bytes: Some(bytes),
            reused,
        });
    }
    Ok(out)
}

pub(crate) fn report(sources: &[Source]) -> Value {
    json!(sources.iter().map(|s|json!({"file":s.file,"available":s.bytes.is_some(),
        "path":s.bytes.as_ref().map(|_|s.path.display().to_string()),"bytes":s.bytes.as_ref().map(Vec::len),
        "sha256":s.bytes.as_ref().map(|b|sha256_hex(b)),"reused":s.reused})).collect::<Vec<_>>())
}

/// Resources commit first, then the drawing. On failure only newly created,
/// still matching resources are removed; prior resources are never touched.
pub(crate) fn commit(
    sources: &[Source],
    drawing: impl FnOnce() -> Result<(), Refused>,
) -> Result<(), Refused> {
    let mut created: Vec<&Source> = Vec::new();
    let result = (|| {
        for source in sources {
            let Some(bytes) = &source.bytes else {
                continue;
            };
            if existing(&source.path, bytes)? {
                continue;
            }
            match files::write_bytes_with_overwrite(bytes, &source.path, false) {
                Ok(_) => created.push(source),
                Err(e) => {
                    if !existing(&source.path, bytes)? {
                        return Err(refuse(e.code, e.message, "path"));
                    }
                }
            }
        }
        drawing()
    })();
    if result.is_err() {
        for source in created {
            if let Some(bytes) = &source.bytes
                && existing(&source.path, bytes).unwrap_or(false)
            {
                let _ = fs::remove_file(&source.path);
            }
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    fn doc() -> Document {
        let mut snap: kentos_commands::DocumentSnapshotV2 = serde_json::from_str(include_str!(
            "../../../../fixtures/kcad/v2/shape-resources.json"
        ))
        .unwrap();
        snap.settings.line_types=serde_json::from_value(json!([{"name":"P","unit":"paper","elements":[{"length":5.},{"length":-1.},{"length":0.,"shape":{"name":"FENCE","file":"FENCE.SHX"}},{"length":-2.}]}])).unwrap();
        Document::from_snapshot_v2(snap).unwrap()
    }
    /// ADR 1125 §5: a text style's embedded SHX font and Big Font go beside the drawing, byte for byte; a style
    /// naming a font the project does not embed adds nothing (the receiver's font, never a disk lookup).
    #[test]
    fn a_text_styles_embedded_shx_fonts_are_written_beside_the_drawing() {
        let mut snap: kentos_commands::DocumentSnapshotV2 =
            serde_json::from_str(include_str!("../../../../fixtures/kcad/v2/shx-fonts.json")).unwrap();
        snap.settings.text_styles.push(
            serde_json::from_value(json!({"id":"rom","name":"Romans","font":"barlow","fontFile":"romans.shx"}))
                .unwrap(),
        );
        let doc = Document::from_snapshot_v2(snap).unwrap();
        let root = std::env::temp_dir().join(format!("kentos-shx-fonts-{}", kentos_domain::Uuid::now_v7()));
        fs::create_dir(&root).unwrap();
        let target = root.join("out.dxf");
        let sources = plan(&doc, &target).unwrap();
        let report = report(&sources);
        let files: Vec<_> = report.as_array().unwrap().iter().map(|s| s["file"].as_str().unwrap()).collect();
        assert_eq!(files, ["kentos-uni.shx", "kentos-big.shx"], "{report}");
        assert!(report.as_array().unwrap().iter().all(|s| s["available"] == true && s["reused"] == false));
        assert_eq!(fs::read_dir(&root).unwrap().count(), 0, "a plan writes nothing");
        commit(&sources, || {
            files::write_bytes_with_overwrite(b"CAD", &target, false)
                .map(|_| ())
                .map_err(|e| refuse(e.code, e.message, "path"))
        })
        .unwrap();
        for name in ["kentos-uni.shx", "kentos-big.shx"] {
            let want = std::fs::read(
                std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/shx/v1").join(name),
            )
            .unwrap();
            assert_eq!(fs::read(root.join(name)).unwrap(), want, "{name}");
        }
        assert!(!root.join("romans.shx").exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn source_transaction_is_read_only_until_commit_and_rolls_back_only_new_files() {
        let root = std::env::temp_dir().join(format!(
            "kentos-shx-files-{}",
            kentos_domain::Uuid::now_v7()
        ));
        fs::create_dir(&root).unwrap();
        let target = root.join("out.dxf");
        let doc = doc();
        let sources = plan(&doc, &target).unwrap();
        assert_eq!(fs::read_dir(&root).unwrap().count(), 0);
        assert!(commit(&sources, || Err(refuse("test", "test", "path"))).is_err());
        assert_eq!(fs::read_dir(&root).unwrap().count(), 0);
        commit(&sources, || {
            files::write_bytes_with_overwrite(b"CAD", &target, false)
                .map(|_| ())
                .map_err(|e| refuse(e.code, e.message, "path"))
        })
        .unwrap();
        let saved = fs::read(root.join("fence.shx")).unwrap();
        assert_eq!(saved, *sources[0].bytes.as_ref().unwrap());
        let sources = plan(&doc, &target).unwrap();
        assert!(sources[0].reused);
        assert!(commit(&sources, || Err(refuse("test", "test", "path"))).is_err());
        assert_eq!(fs::read(root.join("fence.shx")).unwrap(), saved);
        fs::write(root.join("fence.shx"), b"Other user's source").unwrap();
        assert!(
            commit(&sources, || {
                panic!("conflicting source must prevent drawing commit")
            })
            .is_err()
        );
        assert_eq!(
            fs::read(root.join("fence.shx")).unwrap(),
            b"Other user's source"
        );
        assert_eq!(fs::read(&target).unwrap(), b"CAD");
        assert!(plan(&doc, &root.join("fence.shx")).is_err());
        fs::remove_dir_all(root).unwrap();
    }
}
