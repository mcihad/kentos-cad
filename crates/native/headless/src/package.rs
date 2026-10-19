//! Paketle (`file.package`, docs/adr/0506): the project with the files it uses, in one folder. The plan walks the
//! drawing's snapshot for every linked file (bound GeoPackages, linked images and rasters with their world files,
//! point cloud files, linked tables, attachments), names each copy once under `veri/`, rewrites the links to the
//! copies and encodes the project file; the write copies the files into a temporary folder, reads each back against
//! its SHA-256, writes the project file whole and renames the folder into place. Missing files, databases and
//! addresses stay as they are, each said.
//!
//! External references' sources go too (docs/adr/0350 §4), byte for byte so their digests hold; linked images and
//! rasters, and the sources, are looked for with the project's search paths (`links::find_in`). A text style drawn in a
//! substitute face is said as left out; KentOS's own faces come with it. `paket.json` lists all of it.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use kentos_contracts::encoding::{sha256_hex, sha256_reader_hex};
use kentos_contracts::{
    CommandWarning, DocumentSnapshotV2, FontMatch, FontsReport, LeftWhy, MAX_PACKAGE_NAME, PackageLeft,
    PackageProject, PackagedFile, PackagedKind, ProjectPackaged, SourceKind, codes,
};
use kentos_native_application::links::find_in;
use kentos_domain::Document;
use serde_json::Value;

use crate::file_commands::Refused;
use crate::files;

/// The folder the copies go into, in the package.
const DATA: &str = "veri";

/// The manifest's name in the package and its format (docs/adr/0350 §4).
pub const MANIFEST: &str = "paket.json";
pub const MANIFEST_FORMAT: &str = "kentos.package/1";

/// A raster's files beside it, by the extension added to its stem or to its whole name.
const WORLD_FILES: &[&str] = &[
    "wld", "pgw", "pngw", "jgw", "jpgw", "jpegw", "tfw", "tifw", "tiffw", "gfw", "bpw",
];
const WHOLE_NAME_SIDECARS: &[&str] = &["aux.xml", "ovr"];

fn refuse(code: &str, message: impl Into<String>, path: &str) -> Refused {
    Box::new(kentos_contracts::CommandError {
        code: code.to_owned(),
        message: message.into(),
        path: Some(path.to_owned()),
        ..Default::default()
    })
}

/// A package planned: what to write and where.
pub struct Planned {
    pub output: ProjectPackaged,
    pub warnings: Vec<CommandWarning>,
    target: PathBuf,
    name: String,
    project: Vec<u8>,
    manifest: Vec<u8>,
    /// Each copy: the file it is copied from and its path in the package.
    copies: Vec<(PathBuf, String)>,
}

/// The copies named so far.
#[derive(Default)]
struct Copies {
    /// A file copied (its path as found) → its index in `list`.
    by_source: BTreeMap<PathBuf, usize>,
    /// Names taken in `veri/`, folded to lower case (a case-insensitive disk keeps them apart too).
    names: BTreeSet<String>,
    list: Vec<(PathBuf, String, PackagedKind, u32)>,
}

impl Copies {
    /// The package path of `from`'s copy, named once: `veri/name`, `veri/name-2` … when the name is another file's.
    fn take(&mut self, from: &Path, kind: PackagedKind) -> String {
        if let Some(&i) = self.by_source.get(from) {
            self.list[i].3 += 1;
            return self.list[i].1.clone();
        }
        let file = from
            .file_name()
            .map_or_else(|| "dosya".to_owned(), |n| n.to_string_lossy().into_owned());
        let (stem, ext) = match file.rfind('.') {
            Some(i) if i > 0 => (file[..i].to_owned(), file[i..].to_owned()),
            _ => (file.clone(), String::new()),
        };
        let mut name = file.clone();
        let mut n = 2;
        while self.names.contains(&name.to_lowercase()) {
            name = format!("{stem}-{n}{ext}");
            n += 1;
        }
        self.names.insert(name.to_lowercase());
        let rel = format!("{DATA}/{name}");
        self.by_source.insert(from.to_path_buf(), self.list.len());
        self.list.push((from.to_path_buf(), rel.clone(), kind, 1));
        rel
    }

    /// A raster's world and auxiliary files beside `from`, copied under the names its copy `rel` gives them.
    fn sidecars(&mut self, from: &Path, rel: &str) {
        let Some(dir) = from.parent() else {
            return;
        };
        let file = from
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let stem = Path::new(&file)
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        let copy = rel.rsplit('/').next().unwrap_or(rel).to_owned();
        let copy_stem = Path::new(&copy)
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        let mut found: Vec<(PathBuf, String)> = Vec::new();
        for ext in WORLD_FILES {
            for e in [ext.to_string(), ext.to_uppercase()] {
                let p = dir.join(format!("{stem}.{e}"));
                if p.is_file() {
                    found.push((p, format!("{copy_stem}.{ext}")));
                    break;
                }
            }
        }
        for ext in WHOLE_NAME_SIDECARS {
            let p = dir.join(format!("{file}.{ext}"));
            if p.is_file() {
                found.push((p, format!("{copy}.{ext}")));
            }
        }
        for (p, name) in found {
            if self.by_source.contains_key(&p) || !self.names.insert(name.to_lowercase()) {
                continue;
            }
            self.by_source.insert(p.clone(), self.list.len());
            self.list
                .push((p, format!("{DATA}/{name}"), PackagedKind::Sidecar, 1));
        }
    }
}

/// The project file's name: `name`, else the drawing's, with `.kcad`.
fn file_name(input: &PackageProject, doc: &Document) -> Result<String, Refused> {
    let given = input.name.as_deref().map(str::trim);
    let base = match given {
        Some(n) => {
            if n.is_empty() || n == "." || n == ".." || n.contains(['/', '\\', '\0']) {
                return Err(refuse(
                    codes::INVALID_FILE_PATH,
                    format!("“{n}” bir dosya adı değil; klasör ayırıcısı olmadan bir ad verin."),
                    "name",
                ));
            }
            n.to_owned()
        }
        None => {
            let cleaned: String = doc
                .name()
                .chars()
                .map(|c| {
                    if matches!(c, '/' | '\\' | ':' | '\0') || c.is_control() {
                        '-'
                    } else {
                        c
                    }
                })
                .collect();
            let cleaned = cleaned.trim().trim_matches('.').to_owned();
            if cleaned.is_empty() {
                "proje".to_owned()
            } else {
                cleaned
            }
        }
    };
    let name = if base.to_lowercase().ends_with(".kcad") {
        base
    } else {
        format!("{base}.kcad")
    };
    if name.chars().count() > MAX_PACKAGE_NAME {
        return Err(refuse(
            codes::INVALID_FILE_PATH,
            format!("Proje dosyasının adı {MAX_PACKAGE_NAME} harften uzun."),
            "name",
        ));
    }
    Ok(name)
}

/// The folder: absolute, its parent there, itself not there or an empty folder.
fn target(path: &str) -> Result<PathBuf, Refused> {
    let p = Path::new(path);
    if path.trim().is_empty() || !p.is_absolute() {
        return Err(refuse(
            codes::INVALID_FILE_PATH,
            format!("“{path}” mutlak bir klasör yolu değil; paketin klasörünün tam yolunu verin."),
            "path",
        ));
    }
    if !p.parent().is_some_and(Path::is_dir) {
        return Err(refuse(
            codes::INVALID_FILE_PATH,
            format!(
                "{path} klasörünün bulunacağı klasör yok; önce onu açın ya da başka bir yol verin."
            ),
            "path",
        ));
    }
    if p.exists() {
        let empty = p.is_dir() && fs::read_dir(p).is_ok_and(|mut d| d.next().is_none());
        if !empty {
            return Err(refuse(
                codes::FILE_EXISTS,
                format!(
                    "{path} var ve boş bir klasör değil; paket boş ya da olmayan bir klasöre yapılır."
                ),
                "path",
            ));
        }
    }
    Ok(p.to_path_buf())
}

/// A linked file's place: absolute as kept, else from the project's folder; none when relative and no folder.
fn place(kept: &str, from: Option<&Path>) -> Option<PathBuf> {
    let p = Path::new(kept);
    if p.is_absolute() {
        return Some(p.to_path_buf());
    }
    from.map(|f| f.join(p))
}

/// The plan: the copies named, the links rewritten, the project file encoded.
pub fn plan(doc: &Document, input: &PackageProject) -> Result<Planned, Refused> {
    let target = target(&input.path)?;
    let name = file_name(input, doc)?;
    let from = match input.from.as_deref() {
        Some(f) if !(Path::new(f).is_absolute() && Path::new(f).is_dir()) => {
            return Err(refuse(
                codes::INVALID_FILE_PATH,
                format!(
                    "“{f}” var olan bir klasörün mutlak yolu değil; proje dosyasının klasörünü verin."
                ),
                "from",
            ));
        }
        Some(f) => Some(PathBuf::from(f)),
        None => None,
    };
    let snapshot = doc.to_snapshot_v2();
    let mut value = serde_json::to_value(&snapshot)
        .map_err(|e| refuse(codes::FILE_UNWRITABLE, e.to_string(), "path"))?;
    let mut copies = Copies::default();
    let mut left: Vec<PackageLeft> = Vec::new();
    let from_ref = from.as_deref();
    let search = doc.settings().xref_paths.clone();
    // Objects, the drawing's (named by their lasting ids) and the blocks' (by the block's id).
    let uids: Vec<String> = snapshot.uids.iter().map(ToString::to_string).collect();
    if let Some(Value::Array(list)) = value.get_mut("entities") {
        for (k, e) in list.iter_mut().enumerate() {
            let at = uids.get(k).cloned().unwrap_or_default();
            walk_entity(e, &at, from_ref, &search, &mut copies, &mut left);
        }
    }
    if let Some(Value::Array(blocks)) = value.get_mut("blocks") {
        for b in blocks.iter_mut() {
            // An external reference: its source copied, its link the copy; its content is the source's.
            if b.get("xref").is_some_and(Value::is_object) {
                let name = b.get("name").and_then(Value::as_str).unwrap_or_default().to_owned();
                if let Some(x) = b.get_mut("xref") {
                    xref(x, &name, from_ref, &search, &target, &mut copies, &mut left);
                }
                continue;
            }
            let at = format!(
                "blok:{}",
                b.get("id").and_then(Value::as_str).unwrap_or_default()
            );
            if let Some(Value::Array(list)) = b.get_mut("entities") {
                for e in list.iter_mut() {
                    walk_entity(e, &at, from_ref, &search, &mut copies, &mut left);
                }
            }
        }
    }
    // Bound layers' sources.
    if let Some(Value::Array(sources)) = value.pointer_mut("/settings/sources") {
        for s in sources.iter_mut() {
            let id = s
                .get("id")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned();
            let kind: Option<SourceKind> = s
                .get("kind")
                .cloned()
                .and_then(|k| serde_json::from_value(k).ok());
            let path = s
                .get("path")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned();
            if kind == Some(SourceKind::Postgis) {
                left.push(PackageLeft {
                    kind: PackagedKind::Source,
                    path,
                    why: LeftWhy::Database,
                    at: id,
                });
                continue;
            }
            let relative = s.get("relative").and_then(Value::as_str).map(str::to_owned);
            let found = find_in(&path, relative.as_deref(), from_ref, &[]).at;
            let Some(at) = found else {
                left.push(PackageLeft {
                    kind: PackagedKind::Source,
                    path,
                    why: LeftWhy::Missing,
                    at: id,
                });
                continue;
            };
            let rel = copies.take(&at, PackagedKind::Source);
            let abs = rel.split('/').fold(target.clone(), |p, part| p.join(part));
            s["path"] = Value::String(abs.display().to_string());
            s["relative"] = Value::String(rel);
        }
    }
    // The typefaces: KentOS's own faces come with it; a style naming another is drawn in a substitute.
    let (fonts, _) = kentos_native_application::fonts_report::report(doc, &FontsReport::default());
    for f in &fonts.styles {
        if matches!(f.how, FontMatch::Shx | FontMatch::Missing) {
            left.push(PackageLeft {
                kind: PackagedKind::Font,
                path: f.file.clone().unwrap_or_default(),
                why: LeftWhy::Substituted,
                at: format!("stil:{}", f.name),
            });
        }
    }
    let packed: DocumentSnapshotV2 = serde_json::from_value(value).map_err(|e| {
        refuse(
            codes::FILE_UNWRITABLE,
            format!("paketin projesi kurulamadı: {e}"),
            "path",
        )
    })?;
    let project = kentos_kcad::encode_verified(&packed).map_err(|e| {
        refuse(
            codes::FILE_UNWRITABLE,
            format!("paketin projesi KCAD v2'ye yazılamadı: {e}"),
            "path",
        )
    })?;
    let mut files_out = Vec::with_capacity(copies.list.len());
    let mut bytes = project.len() as u64;
    let mut planned = Vec::with_capacity(copies.list.len());
    for (from, rel, kind, links) in &copies.list {
        let read = fs::File::open(from).and_then(sha256_reader_hex);
        let (size, sha) = read.map_err(|e| {
            refuse(
                codes::FILE_UNWRITABLE,
                format!("{} okunamadı: {e}", from.display()),
                "path",
            )
        })?;
        bytes += size;
        files_out.push(PackagedFile {
            path: rel.clone(),
            from: from.display().to_string(),
            kind: *kind,
            bytes: size,
            sha256: sha,
            links: *links,
        });
        planned.push((from.clone(), rel.clone()));
    }
    let mut warnings = Vec::new();
    if !left.is_empty() {
        let said: Vec<String> = left
            .iter()
            .map(|l| {
                let why = match l.why {
                    LeftWhy::Missing => "yerinde yok",
                    LeftWhy::Database => "veritabanı",
                    LeftWhy::Address => "adres",
                    LeftWhy::Substituted => "yerine başka yüzle çizilir",
                };
                format!("{} ({why})", l.path)
            })
            .collect();
        warnings.push(CommandWarning {
            code: codes::PACKAGE_LEFT.into(),
            message: format!(
                "{} bağ pakete kopyalanmadı, olduğu gibi kaldı: {}.",
                left.len(),
                said.join(", ")
            ),
            path: None,
        });
    }
    let project_path = target.join(&name);
    let project_sha256 = sha256_hex(&project);
    let manifest = manifest_of(&name, project.len() as u64, &project_sha256, &files_out, &left, &fonts.faces);
    bytes += manifest.len() as u64;
    Ok(Planned {
        output: ProjectPackaged {
            path: target.display().to_string(),
            project: project_path.display().to_string(),
            project_bytes: project.len() as u64,
            project_sha256,
            files: files_out,
            left,
            manifest: target.join(MANIFEST).display().to_string(),
            manifest_sha256: sha256_hex(&manifest),
            faces: fonts.faces,
            bytes,
        },
        warnings,
        target,
        name,
        project,
        manifest,
        copies: planned,
    })
}

/// `paket.json` (docs/adr/0350 §4): the project file, every copy and every link left out, the faces drawn; keys in a
/// fixed order, no time, so the same drawing gives the same manifest.
fn manifest_of(
    name: &str,
    bytes: u64,
    sha256: &str,
    files: &[PackagedFile],
    left: &[PackageLeft],
    faces: &[kentos_contracts::DrawingFont],
) -> Vec<u8> {
    let manifest = serde_json::json!({
        "format": MANIFEST_FORMAT,
        "project": { "path": name, "bytes": bytes, "sha256": sha256 },
        "files": files,
        "left": left,
        "faces": faces,
    });
    let mut out = serde_json::to_vec_pretty(&manifest).unwrap_or_default();
    out.push(b'\n');
    out
}

/// An external reference's source (its block's `xref`), copied byte for byte (its digest holds) and its link made the
/// copy: `path` the copy in the package, `relative` `veri/…`; or left out, missing.
fn xref(
    x: &mut Value,
    name: &str,
    from: Option<&Path>,
    search: &[String],
    target: &Path,
    copies: &mut Copies,
    left: &mut Vec<PackageLeft>,
) {
    let path = x.get("path").and_then(Value::as_str).unwrap_or_default().to_owned();
    let relative = x.get("relative").and_then(Value::as_str).map(str::to_owned);
    match find_in(&path, relative.as_deref(), from, search).at {
        Some(at) => {
            let rel = copies.take(&at, PackagedKind::Xref);
            let abs = rel.split('/').fold(target.to_path_buf(), |p, part| p.join(part));
            x["path"] = Value::String(abs.display().to_string());
            x["relative"] = Value::String(rel);
        }
        None => left.push(PackageLeft {
            kind: PackagedKind::Xref,
            path,
            why: LeftWhy::Missing,
            at: format!("xref:{name}"),
        }),
    }
}

/// One object's links: its file (an image or a raster, looked for with the search paths too), a point cloud's files,
/// a linked table, its attachments.
fn walk_entity(
    e: &mut Value,
    at: &str,
    from: Option<&Path>,
    search: &[String],
    copies: &mut Copies,
    left: &mut Vec<PackageLeft>,
) {
    let at = at.to_owned();
    let raster = e.get("kind").and_then(Value::as_str) == Some("raster");
    let picture = matches!(e.get("kind").and_then(Value::as_str), Some("image" | "raster"));
    let link = |slot: &mut Value,
                kind: PackagedKind,
                sidecars: bool,
                looks: &[String],
                copies: &mut Copies,
                left: &mut Vec<PackageLeft>| {
        let Some(kept) = slot.as_str().map(str::to_owned) else {
            return;
        };
        let found = if looks.is_empty() { place(&kept, from) } else { find_in(&kept, None, from, looks).at };
        match found {
            Some(p) if p.is_file() => {
                let rel = copies.take(&p, kind);
                if sidecars {
                    copies.sidecars(&p, &rel);
                }
                *slot = Value::String(rel);
            }
            _ => left.push(PackageLeft {
                kind,
                path: kept,
                why: LeftWhy::Missing,
                at: at.clone(),
            }),
        }
    };
    if let Some(slot) = e.get_mut("file") {
        let looks: &[String] = if picture { search } else { &[] };
        link(slot, PackagedKind::Picture, raster, looks, copies, left);
    }
    if let Some(url) = e.get("url").and_then(Value::as_str) {
        left.push(PackageLeft {
            kind: PackagedKind::Picture,
            path: url.to_owned(),
            why: LeftWhy::Address,
            at: at.clone(),
        });
    }
    if let Some(Value::Array(parts)) = e.get_mut("sources") {
        for part in parts.iter_mut() {
            if let Some(slot) = part.get_mut("file") {
                link(slot, PackagedKind::Picture, false, &[], copies, left);
            }
            if let Some(url) = part.get("url").and_then(Value::as_str) {
                left.push(PackageLeft {
                    kind: PackagedKind::Picture,
                    path: url.to_owned(),
                    why: LeftWhy::Address,
                    at: at.clone(),
                });
            }
        }
    }
    if let Some(slot) = e.pointer_mut("/source/path") {
        link(slot, PackagedKind::Table, false, &[], copies, left);
    }
    if let Some(Value::Array(list)) = e.get_mut("attachments") {
        for a in list.iter_mut() {
            if let Some(slot) = a.get_mut("file") {
                link(slot, PackagedKind::Attachment, false, &[], copies, left);
            }
        }
    }
}

/// Writes the package planned: the copies and the project file into a temporary folder beside the target, each copy
/// read back against its SHA-256, then the folder renamed into place. Nothing is left on a failure.
pub fn write(planned: &Planned) -> Result<(), Refused> {
    let target = &planned.target;
    let parent = target.parent().unwrap_or(Path::new("."));
    let leaf = target
        .file_name()
        .map_or_else(|| "paket".to_owned(), |n| n.to_string_lossy().into_owned());
    let temp = parent.join(format!(".{leaf}.paketleniyor-{}", std::process::id()));
    let fail = |what: String| refuse(codes::FILE_UNWRITABLE, what, "path");
    let _ = fs::remove_dir_all(&temp);
    let result = (|| {
        fs::create_dir_all(&temp)
            .map_err(|e| fail(format!("{} açılamadı: {e}", temp.display())))?;
        if !planned.copies.is_empty() {
            fs::create_dir_all(temp.join(DATA))
                .map_err(|e| fail(format!("{} açılamadı: {e}", temp.display())))?;
        }
        for ((from, rel), meant) in planned.copies.iter().zip(&planned.output.files) {
            let to = rel.split('/').fold(temp.clone(), |p, part| p.join(part));
            fs::copy(from, &to)
                .map_err(|e| fail(format!("{} kopyalanamadı: {e}", from.display())))?;
            let (size, sha) = fs::File::open(&to)
                .and_then(sha256_reader_hex)
                .map_err(|e| fail(format!("{} geri okunamadı: {e}", to.display())))?;
            if size != meant.bytes || sha != meant.sha256 {
                return Err(fail(format!(
                    "{} kopyalanırken değişti ya da kopyası aynı değil; paket yapılmadı. Dosyayı kullanan programı kapatıp yeniden deneyin.",
                    from.display()
                )));
            }
        }
        files::write_bytes(&planned.project, &temp.join(&planned.name))
            .map_err(|e| fail(e.message))?;
        let manifest = temp.join(MANIFEST);
        fs::write(&manifest, &planned.manifest)
            .map_err(|e| fail(format!("{} yazılamadı: {e}", manifest.display())))?;
        let back = fs::read(&manifest).map_err(|e| fail(format!("{} geri okunamadı: {e}", manifest.display())))?;
        if sha256_hex(&back) != planned.output.manifest_sha256 {
            return Err(fail(format!("{} yazılırken değişti; paket yapılmadı.", manifest.display())));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_taken_once_each() {
        let mut c = Copies::default();
        assert_eq!(
            c.take(Path::new("/a/kadastro.gpkg"), PackagedKind::Source),
            "veri/kadastro.gpkg"
        );
        assert_eq!(
            c.take(Path::new("/b/KADASTRO.gpkg"), PackagedKind::Source),
            "veri/KADASTRO-2.gpkg"
        );
        assert_eq!(
            c.take(Path::new("/a/kadastro.gpkg"), PackagedKind::Source),
            "veri/kadastro.gpkg"
        );
        assert_eq!(c.list[0].3, 2);
        assert_eq!(
            c.take(Path::new("/c/README"), PackagedKind::Attachment),
            "veri/README"
        );
    }
}
