//! `file.export.shp` (docs/adr/0514): the drawing's layers as Shapefiles into a `.zip`, beside a `.shp`, or into a new
//! or empty folder, whole or not at all. The bytes are made first (`plan` stops there); `execute` writes them:
//!
//! - an archive through `files::write_bytes` (a temporary file, read back, renamed);
//! - a `.shp`'s parts each staged and read back, the old parts (and a stale index or .prj of the same name) moved
//!   aside, the new ones renamed in, the old ones removed; a failed rename puts the old ones back;
//! - a folder staged whole beside the target and renamed into its place, as Paketle does (docs/adr/0506).

use std::fs;
use std::path::{Path, PathBuf};

use kentos_contracts::encoding::sha256_hex;
use kentos_contracts::{
    CommandWarning, ExportShp, ShpColumnAsText, ShpColumnRenamed, ShpExported, ShpFileWritten,
    ShpSetExported, codes,
};
use kentos_domain::Document;

use crate::file_commands::Refused;
use crate::files;

/// Where the Shapefiles go.
pub enum Target {
    Zip(PathBuf),
    /// The one Shapefile's `.shp`: its folder and its name without extension.
    Shp(PathBuf, String),
    Folder(PathBuf),
}

/// The parts a Shapefile of a name may have beside it, old indexes among them: an overwrite removes those it does
/// not write, so no stale .prj or spatial index describes the new file.
const PARTS: [&str; 9] = [
    "shp", "shx", "dbf", "prj", "cpg", "qix", "sbn", "sbx", "shp.xml",
];

/// The files to write and what they are.
pub struct Made {
    pub into: Target,
    /// (name, bytes): the archive's entries, or the files.
    pub files: Vec<(String, Vec<u8>)>,
    pub output: ShpExported,
    pub warnings: Vec<CommandWarning>,
    /// The archive's bytes.
    pub zip: Option<Vec<u8>>,
}

fn refuse(code: &str, message: impl Into<String>, path: &str) -> Refused {
    Box::new(kentos_contracts::CommandError {
        code: code.to_owned(),
        message: message.into(),
        path: Some(path.to_owned()),
        ..kentos_contracts::CommandError::default()
    })
}

/// The target from the path: absolute; a `.zip` or `.shp` in a folder that is there (taken only with `overwrite`
/// when there); else a folder, new (its parent there) or empty.
fn target(path: &str, overwrite: bool) -> Result<Target, Refused> {
    let p = Path::new(path);
    if path.trim().is_empty() || !p.is_absolute() {
        return Err(refuse(
            codes::INVALID_FILE_PATH,
            format!("“{path}” mutlak bir yol değil; .zip, .shp ya da klasörün tam yolunu verin."),
            "path",
        ));
    }
    let ext = p
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    let parent_there = p.parent().is_some_and(Path::is_dir);
    if ext == "zip" || ext == "shp" {
        if p.is_dir() {
            return Err(refuse(
                codes::INVALID_FILE_PATH,
                format!(
                    "{path} bir klasör; yazılacak dosyanın adını verin ya da klasörün yolunu .{ext}'siz verin."
                ),
                "path",
            ));
        }
        if !parent_there {
            return Err(refuse(
                codes::INVALID_FILE_PATH,
                format!(
                    "{path} dosyasının klasörü yok; önce klasörü açın ya da başka bir yol verin."
                ),
                "path",
            ));
        }
        let dir = p.parent().map(Path::to_path_buf).unwrap_or_default();
        if ext == "zip" {
            if p.exists() && !overwrite {
                return Err(refuse(
                    codes::FILE_EXISTS,
                    format!("{path} var; üzerine yazmak için overwrite: true verin."),
                    "path",
                ));
            }
            return Ok(Target::Zip(p.to_path_buf()));
        }
        let stem = p
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        let there: Vec<String> = PARTS
            .iter()
            .map(|x| format!("{stem}.{x}"))
            .filter(|n| dir.join(n).exists())
            .collect();
        if !there.is_empty() && !overwrite {
            return Err(refuse(
                codes::FILE_EXISTS,
                format!(
                    "{} klasöründe {} var; üzerine yazmak için overwrite: true verin.",
                    dir.display(),
                    there.join(", ")
                ),
                "path",
            ));
        }
        return Ok(Target::Shp(dir, stem));
    }
    if p.is_file() {
        return Err(refuse(
            codes::INVALID_FILE_PATH,
            format!("{path} bir dosya; .zip, .shp ya da yeni ya da boş bir klasörün yolunu verin."),
            "path",
        ));
    }
    if p.is_dir() {
        let empty = fs::read_dir(p)
            .map(|mut d| d.next().is_none())
            .unwrap_or(false);
        if !empty {
            return Err(refuse(
                codes::FILE_EXISTS,
                format!(
                    "{path} klasörü boş değil; yeni ya da boş bir klasör verin ya da .zip olarak yazın."
                ),
                "path",
            ));
        }
    } else if !parent_there {
        return Err(refuse(
            codes::INVALID_FILE_PATH,
            format!("{path} klasörünün üst klasörü yok; önce onu açın ya da başka bir yol verin."),
            "path",
        ));
    }
    Ok(Target::Folder(p.to_path_buf()))
}

/// The Shapefiles' bytes and what they hold, or the refusal.
pub fn make(doc: &Document, input: &ExportShp) -> Result<Made, Refused> {
    let into = target(&input.path, input.overwrite)?;
    let ids = kentos_native_application::shapefile::layer_ids(doc, &input.layers).map_err(
        |(i, id)| {
            refuse(
                codes::LAYER_NOT_FOUND,
                format!(
                    "“{id}” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin."
                ),
                &format!("layers[{i}]"),
            )
        },
    )?;
    let written =
        kentos_native_application::shapefile::write(doc, &ids, input.encoding).map_err(|why| {
            refuse(
                codes::FILE_UNWRITABLE,
                format!("Shapefile yazılamadı: {why}"),
                "path",
            )
        })?;
    if written.sets.is_empty() {
        return Err(refuse(
            codes::NOTHING_TO_EXPORT,
            "Seçilen katmanlarda Shapefile'a yazılacak nesne yok: yazı, ölçü, tablo, resim, raster ve nokta bulutunun Shapefile'da geometrisi yok.",
            if input.layers.is_empty() {
                "path"
            } else {
                "layers"
            },
        ));
    }
    if let Target::Shp(..) = into
        && written.sets.len() > 1
    {
        let names: Vec<&str> = written.sets.iter().map(|s| s.name.as_str()).collect();
        return Err(refuse(
            codes::SEVERAL_SHAPEFILES,
            format!(
                "Dışa aktarma {} Shapefile yapıyor ({}); bir .shp yalnız birini alır. Bir klasör ya da .zip verin.",
                names.len(),
                names.join(", ")
            ),
            "path",
        ));
    }
    let mut files: Vec<(String, Vec<u8>)> = Vec::new();
    let mut sets = Vec::new();
    for set in &written.sets {
        let name = match &into {
            Target::Shp(_, stem) => stem.clone(),
            _ => set.name.clone(),
        };
        for (ext, bytes) in &set.files {
            files.push((format!("{name}.{ext}"), bytes.clone()));
        }
        sets.push(ShpSetExported {
            name,
            layer: set.layer.clone(),
            rows: set.rows,
            shape: set.shape.map(str::to_owned),
        });
    }
    let zip = match &into {
        Target::Zip(_) => {
            let entries: Vec<(&str, &[u8])> = files
                .iter()
                .map(|(n, b)| (n.as_str(), b.as_slice()))
                .collect();
            Some(
                kentos_formats::zip::write(&entries)
                    .map_err(|why| refuse(codes::FILE_UNWRITABLE, why, "path"))?,
            )
        }
        _ => None,
    };
    let listed: Vec<ShpFileWritten> = match (&into, &zip) {
        (Target::Zip(p), Some(z)) => vec![ShpFileWritten {
            name: p
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default(),
            bytes: z.len() as u64,
            sha256: sha256_hex(z),
        }],
        _ => files
            .iter()
            .map(|(n, b)| ShpFileWritten {
                name: n.clone(),
                bytes: b.len() as u64,
                sha256: sha256_hex(b),
            })
            .collect(),
    };
    let mut warnings = Vec::new();
    let said = losses(doc, &written);
    if !said.is_empty() {
        warnings.push(CommandWarning {
            code: codes::EXPORT_LOSSES.into(),
            message: said.join(" "),
            path: Some("layers".into()),
        });
    }
    let prj = written
        .sets
        .iter()
        .any(|s| s.files.iter().any(|(x, _)| *x == "prj"));
    let output = ShpExported {
        path: input.path.clone(),
        into: match into {
            Target::Zip(_) => "zip",
            Target::Shp(..) => "shp",
            Target::Folder(_) => "folder",
        }
        .into(),
        sets,
        files: listed,
        prj,
        left: written.left.clone(),
        approximated: written.approximated,
        renamed: written
            .renamed
            .iter()
            .map(|r| ShpColumnRenamed {
                set: r.set.clone(),
                attribute: r.attribute.clone(),
                column: r.column.clone(),
            })
            .collect(),
        as_text: written
            .as_text
            .iter()
            .map(|a| ShpColumnAsText {
                set: a.set.clone(),
                column: a.column.clone(),
                reason: a.reason.to_owned(),
            })
            .collect(),
        cut: written.cut,
        replaced: written.replaced,
        blank_text: written.blank_text,
        report: written.report,
    };
    Ok(Made {
        into,
        files,
        output,
        warnings,
        zip,
    })
}

/// What the format could not hold, sentence by sentence, as `export_losses` says it (and the window shows it first).
pub fn losses(doc: &Document, written: &kentos_formats::shp::write::Written) -> Vec<String> {
    let mut said: Vec<String> = Vec::new();
    if !written.left.is_empty() {
        let kinds: Vec<String> = written
            .left
            .iter()
            .map(|(k, n)| format!("{n} {k}"))
            .collect();
        said.push(format!(
            "Shapefile'da karşılığı olmayan nesneler yazılmadı: {}.",
            kinds.join(", ")
        ));
    }
    if written.approximated > 0 {
        said.push(format!(
            "{} eğrili nesne (daire, yay, elips, eğri, yaylı yol) eğriden en çok 0,1 mm sapan kirişlerle yazıldı.",
            written.approximated
        ));
    }
    for r in &written.renamed {
        said.push(format!(
            "“{}” Shapefile'ında “{}” özniteliği “{}” sütununa yazıldı (dBASE adı en çok 10 bayt, büyük küçük harf ayrılmaz).",
            r.set, r.attribute, r.column
        ));
    }
    for a in &written.as_text {
        said.push(format!(
            "“{}” Shapefile'ının “{}” sütunu: {}.",
            a.set, a.column, a.reason
        ));
    }
    if written.cut > 0 {
        said.push(format!(
            "{} metin dBASE'in 254 baytına sığmadı, kesildi.",
            written.cut
        ));
    }
    if written.replaced > 0 {
        said.push(format!(
            "Windows-1254'te olmayan {} harf ? olarak yazıldı; UTF-8 seçin.",
            written.replaced
        ));
    }
    if written.blank_text > 0 {
        said.push(format!(
            "{} boş metin boşlukla yazıldı: dBASE'te boş metin ile değerin yokluğu ayrılamaz.",
            written.blank_text
        ));
    }
    match kentos_native_application::shapefile::prj(doc) {
        None => said.push(
            "Projenin koordinat sistemi yok (yerel proje) ya da adlandırılamıyor: .prj yazılmadı; okuyan program sistemi sorar."
                .to_owned(),
        ),
        Some((_, kentos_native_application::shapefile::PrjKind::Definition)) => said.push(
            "Projenin kendi koordinat sistemi tanımı .prj'ye OGC WKT'siyle yazıldı; okuyan program onu özel bir sistem olarak tanır."
                .to_owned(),
        ),
        Some(_) => {}
    }
    if written.columns_left > 0 {
        said.push(format!(
            "dBASE en çok 255 sütun alır; {} sütun yazılmadı.",
            written.columns_left
        ));
    }
    said
}

/// What the journal keeps of the write: the path, every byte, and the SHA-256 of the files' names and digests (an
/// archive: its own).
pub fn effect(made: &Made) -> (String, u64, String) {
    let bytes = made.output.files.iter().map(|f| f.bytes).sum();
    let sha = match &made.zip {
        Some(z) => sha256_hex(z),
        None => {
            let list: String = made
                .output
                .files
                .iter()
                .map(|f| format!("{} {}\n", f.sha256, f.name))
                .collect();
            sha256_hex(list.as_bytes())
        }
    };
    (made.output.path.clone(), bytes, sha)
}

fn fail(what: String) -> Refused {
    refuse(codes::FILE_UNWRITABLE, what, "path")
}

/// `bytes` at `at` through a sync, read back.
fn stage(bytes: &[u8], at: &Path) -> Result<(), Refused> {
    use std::io::Write as _;
    let mut f =
        fs::File::create(at).map_err(|e| fail(format!("{} yazılamadı: {e}", at.display())))?;
    f.write_all(bytes)
        .and_then(|()| f.sync_all())
        .map_err(|e| fail(format!("{} yazılamadı: {e}", at.display())))?;
    drop(f);
    let back = fs::read(at).map_err(|e| fail(format!("{} geri okunamadı: {e}", at.display())))?;
    if back != bytes {
        return Err(fail(format!(
            "{} diske yazılanla aynı değil; Shapefile yazılmadı.",
            at.display()
        )));
    }
    Ok(())
}

/// Writes what [`make`] made, whole or not at all.
pub fn write(made: &Made) -> Result<(), Refused> {
    match &made.into {
        Target::Zip(p) => {
            let bytes = made.zip.as_deref().unwrap_or_default();
            files::write_bytes(bytes, p)
                .map(|_| ())
                .map_err(|e| fail(e.message))
        }
        Target::Shp(dir, stem) => write_beside(dir, stem, &made.files),
        Target::Folder(p) => write_folder(p, &made.files),
    }
}

fn write_beside(dir: &Path, stem: &str, files: &[(String, Vec<u8>)]) -> Result<(), Refused> {
    let pid = std::process::id();
    let temp = |n: &str| dir.join(format!(".{n}.{pid}.yaziliyor"));
    let aside = |n: &str| dir.join(format!(".{n}.{pid}.eski"));
    let staged = (|| {
        for (n, b) in files {
            stage(b, &temp(n))?;
        }
        Ok(())
    })();
    if let Err(e) = staged {
        for (n, _) in files {
            let _ = fs::remove_file(temp(n));
        }
        return Err(e);
    }
    // The old parts of the name, and stale ones the new set does not write, move aside.
    let old: Vec<String> = PARTS
        .iter()
        .map(|x| format!("{stem}.{x}"))
        .filter(|n| dir.join(n).exists())
        .collect();
    let mut moved: Vec<&String> = Vec::new();
    let mut placed: Vec<&String> = Vec::new();
    let result = (|| {
        for n in &old {
            fs::rename(dir.join(n), aside(n))
                .map_err(|e| fail(format!("{} kenara alınamadı: {e}", n)))?;
            moved.push(n);
        }
        for (n, _) in files {
            fs::rename(temp(n), dir.join(n))
                .map_err(|e| fail(format!("{} yerine konamadı: {e}", dir.join(n).display())))?;
            placed.push(n);
        }
        Ok(())
    })();
    match result {
        Ok(()) => {
            for n in moved {
                let _ = fs::remove_file(aside(n));
            }
            Ok(())
        }
        Err(e) => {
            for n in placed {
                let _ = fs::remove_file(dir.join(n));
            }
            for n in moved {
                let _ = fs::rename(aside(n), dir.join(n));
            }
            for (n, _) in files {
                let _ = fs::remove_file(temp(n));
            }
            Err(e)
        }
    }
}

fn write_folder(target: &Path, files: &[(String, Vec<u8>)]) -> Result<(), Refused> {
    let parent = target.parent().unwrap_or(Path::new("."));
    let leaf = target.file_name().map_or_else(
        || "shapefile".to_owned(),
        |n| n.to_string_lossy().into_owned(),
    );
    let temp = parent.join(format!(".{leaf}.yaziliyor-{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp);
    let result = (|| {
        fs::create_dir_all(&temp)
            .map_err(|e| fail(format!("{} açılamadı: {e}", temp.display())))?;
        for (n, b) in files {
            stage(b, &temp.join(n))?;
        }
        if target.is_dir() {
            fs::remove_dir(target)
                .map_err(|e| fail(format!("{} kaldırılamadı: {e}", target.display())))?;
        }
        fs::rename(&temp, target)
            .map_err(|e| fail(format!("{} yerine konamadı: {e}", target.display())))
    })();
    if result.is_err() {
        let _ = fs::remove_dir_all(&temp);
    }
    result
}
