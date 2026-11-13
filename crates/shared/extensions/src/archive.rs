//! Opening a `.kek` package **without running any of its code**, and making one (plan/eklenti.md ADIM 16; PY-KEK-03…09,
//! P08): the same checks on the author's computer (`kentos kek pack`), in an agent's repair loop
//! (`extension.verify`), at install and in the catalog. Every broken rule is a [`Problem`]; no input panics.
//!
//! [`open`] checks, in order, and stops at the first stage that fails, so a hostile archive is never unpacked:
//! 1. the archive: its size, a ZIP from its first byte, its directory, a layout two readers read alike;
//! 2. every entry: safe ASCII paths, depth, no duplicate or case-folded twin, no link, special file, encryption or
//!    folder bytes, each entry's size and unpack ratio, the sum;
//! 3. nothing a package must not carry: environments, caches, version control, secrets, AI tool settings, native
//!    code (by name and by its first bytes), nested archives, a computer's junk;
//! 4. the content: `integrity.json` against every file, `manifest.json` and its rules, every file it names, each
//!    as what it is (an icon, a form, a tool descriptor, a GeoPackage …), the handlers' modules, the Python project
//!    against its lock, and every text file for invisible characters and a user's home path.
//!
//! Nothing is written anywhere: the installer takes [`Opened::files`] only when [`Inspection::problems`] is empty.

use std::collections::{BTreeMap, BTreeSet};

use kentos_zip::Pack;
use serde::Serialize;

use crate::icon;
use crate::integrity::{self, FileDigest, Integrity};
use crate::manifest::{self, FileRole, Manifest};
use crate::paths;
use crate::problem::Problem;
use crate::python;
use crate::text;

/// How much a package may hold.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    /// Bytes of the `.kek` file itself.
    pub archive: u64,
    /// Entries in it.
    pub entries: usize,
    /// Bytes unpacked in all.
    pub total: u64,
    /// Bytes one file may unpack to.
    pub file: u64,
    /// Bytes an entry may unpack to per byte packed: a ZIP bomb's tell.
    pub ratio: u64,
    /// Folders and the file in a path (`a/b/c.py` is 3).
    pub depth: usize,
}

impl Default for Limits {
    fn default() -> Limits {
        Limits {
            archive: 128 * 1024 * 1024,
            entries: 2000,
            total: 256 * 1024 * 1024,
            file: 64 * 1024 * 1024,
            ratio: 200,
            depth: 12,
        }
    }
}

impl Limits {
    fn zip(&self) -> kentos_zip::Limits {
        kentos_zip::Limits {
            entries: self.entries,
            total: self.total,
            ratio: self.ratio,
        }
    }
}

/// What the inspector found.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Inspection {
    /// The manifest, when it could be read.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manifest: Option<Manifest>,
    /// The content digest (`sha256:…`), when the integrity list holds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub digest: Option<String>,
    /// The content files as the integrity list gives them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub files: Vec<FileDigest>,
    /// What keeps it from being installed.
    pub problems: Vec<Problem>,
    /// What works but should be mended.
    pub warnings: Vec<Problem>,
}

impl Inspection {
    pub fn is_valid(&self) -> bool {
        self.problems.is_empty()
    }
}

/// A package opened: the inspection, and its files when it passed.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Opened {
    pub inspection: Inspection,
    /// Every file by its path, `integrity.json` too; empty unless the inspection passed.
    pub files: BTreeMap<String, Vec<u8>>,
}

/// Checks a `.kek` package's bytes; runs none of its code.
pub fn inspect(bytes: &[u8], limits: &Limits) -> Inspection {
    open(bytes, limits).inspection
}

/// Opens a `.kek` package's bytes: the inspection, and the files when nothing is wrong.
pub fn open(bytes: &[u8], limits: &Limits) -> Opened {
    let mut inspection = Inspection::default();
    let problems = &mut inspection.problems;
    if bytes.len() as u64 > limits.archive {
        problems.push(Problem::new(
            "archive_too_large",
            format!(
                "Paket {} MB; en çok {} MB olur.",
                bytes.len() / (1024 * 1024),
                limits.archive / (1024 * 1024)
            ),
        ));
        return Opened {
            inspection,
            files: BTreeMap::new(),
        };
    }
    let entries = match read_entries(bytes, limits, problems) {
        Some(entries) => entries,
        None => {
            return Opened {
                inspection,
                files: BTreeMap::new(),
            };
        }
    };
    let mut files = BTreeMap::new();
    for entry in entries {
        if let Some((code, message)) = native_code(&entry.data) {
            problems.push(Problem::new(code, message).file(entry.name.clone()));
        }
        files.insert(entry.name, entry.data);
    }
    if !inspection.problems.is_empty() {
        return Opened {
            inspection,
            files: BTreeMap::new(),
        };
    }
    check_tree(&files, Stage::Inspect, &mut inspection);
    if !inspection.is_valid() {
        files.clear();
    }
    Opened { inspection, files }
}

/// The archive's file entries, unpacked, when every rule on the archive and its paths holds.
fn read_entries(
    bytes: &[u8],
    limits: &Limits,
    problems: &mut Vec<Problem>,
) -> Option<Vec<kentos_zip::Entry>> {
    if !kentos_zip::sniff(bytes) {
        problems.push(Problem::new(
            "archive_not_zip",
            "Dosya bir ZIP arşivi olarak başlamıyor (başka bir dosya ya da başına veri eklenmiş bir arşiv). \
             Paketi “kentos kek pack” ile oluşturun.",
        ));
        return None;
    }
    let listed = match kentos_zip::list_every(bytes, &limits.zip()) {
        Ok(listed) => listed,
        Err(message) => {
            problems.push(Problem::new("archive_broken", message));
            return None;
        }
    };
    if let Some(message) = kentos_zip::layout_problem(bytes, &listed) {
        problems.push(Problem::new("archive_layout", message));
        return None;
    }
    let mut total: u64 = 0;
    let mut seen: BTreeMap<String, String> = BTreeMap::new();
    for l in &listed {
        let shown = l.name.clone();
        let mut refuse = |code: &'static str, message: String| {
            problems.push(Problem::new(code, message).file(shown.clone()));
        };
        let path = l.name.strip_suffix('/').unwrap_or(&l.name);
        if l.had_backslash() {
            refuse(
                "path_unsafe",
                "Yolda ters bölü (\\) var; paket içinde klasörler “/” ile ayrılır.".to_owned(),
            );
            continue;
        }
        if let Some(problem) = paths::problem(path) {
            refuse("path_unsafe", problem);
            continue;
        }
        if paths::depth(path) > limits.depth {
            refuse(
                "path_depth",
                format!(
                    "Yol {} klasör derinliğinde; en çok {} olur.",
                    paths::depth(path),
                    limits.depth
                ),
            );
        }
        if l.is_encrypted() {
            refuse(
                "archive_encrypted",
                "Girdi şifreli; paket şifresiz olmalı.".to_owned(),
            );
        }
        if l.is_link() {
            refuse(
                "archive_link",
                "Girdi bir sembolik bağ; paket yalnız gerçek dosyalar taşır (bağ paketin dışını gösterebilir)."
                    .to_owned(),
            );
        } else if let Some(mode) = l.unix_mode()
            && !matches!(mode & 0o170_000, 0 | 0o100_000 | 0o040_000)
        {
            refuse(
                "archive_special",
                format!(
                    "Girdi olağan bir dosya değil (Unix türü {:o}).",
                    mode & 0o170_000
                ),
            );
        }
        if l.is_folder() {
            if l.size() != 0 {
                refuse("archive_folder", "Klasör girdisi bayt taşıyor.".to_owned());
            }
            continue;
        }
        if let Some((code, message)) = forbidden(path) {
            refuse(code, message);
        }
        if l.size() > limits.file {
            refuse(
                "archive_file_too_large",
                format!(
                    "Dosya açılınca {} MB; bir dosya en çok {} MB olur.",
                    l.size() / (1024 * 1024),
                    limits.file / (1024 * 1024)
                ),
            );
        }
        if l.size() > l.packed().saturating_mul(limits.ratio).max(1024) {
            refuse(
                "archive_ratio",
                format!(
                    "Dosya olağan dışı sıkıştırılmış ({} bayttan {} bayta); ZIP bombası olabileceği için açılmadı.",
                    l.packed(),
                    l.size()
                ),
            );
        }
        total = total.saturating_add(l.size());
        let folded = paths::folded(path);
        if let Some(first) = seen.insert(folded, path.to_owned()) {
            if first == path {
                refuse(
                    "path_duplicate",
                    "Aynı adla iki girdi var; hangisinin açılacağı belirsiz.".to_owned(),
                );
            } else {
                refuse(
                    "path_case",
                    format!(
                        "“{first}” ile yalnız büyük/küçük harfle ayrılıyor; Windows ve macOS ikisini aynı dosya sayar."
                    ),
                );
            }
        }
    }
    if total > limits.total {
        problems.push(Problem::new(
            "archive_total",
            format!(
                "Paketteki dosyalar açılınca {} MB; en çok {} MB olur.",
                total / (1024 * 1024),
                limits.total / (1024 * 1024)
            ),
        ));
    }
    if !problems.is_empty() {
        return None;
    }
    match kentos_zip::unpack(bytes, &listed, &limits.zip(), |l| !l.is_folder()) {
        Ok(entries) => Some(entries),
        Err(message) => {
            problems.push(Problem::new("archive_broken", message));
            None
        }
    }
}

/// What a package must not carry, by its path: the code and the reason.
fn forbidden(path: &str) -> Option<(&'static str, String)> {
    let name = path.rsplit('/').next().unwrap_or(path);
    let folders: Vec<&str> = path.split('/').collect();
    let folders = &folders[..folders.len() - 1];
    let lower = name.to_ascii_lowercase();
    let ext = paths::extension(path);
    let has = |any: &[&str]| {
        folders
            .iter()
            .any(|f| any.contains(&f.to_ascii_lowercase().as_str()))
    };
    if has(&["__macosx"])
        || name.starts_with("._")
        || matches!(lower.as_str(), ".ds_store" | "thumbs.db" | "desktop.ini")
    {
        return Some((
            "content_junk",
            "İşletim sisteminin ek dosyası; pakete girmez.".to_owned(),
        ));
    }
    if has(&[".venv", "site-packages"]) || lower == "pyvenv.cfg" {
        return Some((
            "content_environment",
            "Python ortamı (.venv) pakete girmez: ortamı kilitten kurulumda uv kurar.".to_owned(),
        ));
    }
    if has(&["__pycache__"]) || matches!(ext.as_str(), "pyc" | "pyo") {
        return Some((
            "content_cache",
            "Derlenmiş Python önbelleği pakete girmez; kaynak (.py) yeter.".to_owned(),
        ));
    }
    if has(&[".git", ".hg", ".svn"]) {
        return Some((
            "content_vcs",
            "Sürüm denetimi klasörü pakete girmez (geçmiş ve kimlik bilgisi taşıyabilir)."
                .to_owned(),
        ));
    }
    let env_template = matches!(
        lower.as_str(),
        ".env.example" | ".env.sample" | ".env.template"
    );
    let secret = ((lower == ".env" || lower.starts_with(".env.")) && !env_template)
        || matches!(
            ext.as_str(),
            "pem" | "key" | "p12" | "pfx" | "jks" | "keystore" | "ppk"
        )
        || ["id_rsa", "id_dsa", "id_ecdsa", "id_ed25519"]
            .iter()
            .any(|k| lower.starts_with(k))
        || matches!(
            lower.as_str(),
            ".netrc" | ".pypirc" | ".npmrc" | ".git-credentials" | "credentials.json"
        );
    if secret {
        return Some((
            "content_secret",
            "Gizli bilgi taşıyabilecek dosya (anahtar, parola, .env) pakete girmez; ayarı kurulumdan sonra kullanıcı \
             girer."
                .to_owned(),
        ));
    }
    let ai = matches!(
        lower.as_str(),
        "mcp.json"
            | ".mcp.json"
            | ".cursorrules"
            | ".windsurfrules"
            | ".clinerules"
            | ".aider.conf.yml"
    ) || lower.starts_with(".aider")
        || has(&[
            ".cursor",
            ".claude",
            ".windsurf",
            ".continue",
            ".codeium",
            ".vscode",
            ".idea",
        ]);
    if ai {
        return Some((
            "content_ai_config",
            "Yapay zekâ aracı ya da düzenleyici ayarı pakete girmez: çoğu zaman anahtar ve yerel yol taşır.".to_owned(),
        ));
    }
    if matches!(
        ext.as_str(),
        "so" | "dll"
            | "dylib"
            | "exe"
            | "pyd"
            | "msi"
            | "com"
            | "scr"
            | "bat"
            | "cmd"
            | "ps1"
            | "vbs"
            | "jar"
            | "class"
            | "node"
            | "a"
            | "lib"
            | "o"
            | "obj"
            | "wasm"
            | "apk"
            | "deb"
            | "rpm"
            | "dmg"
    ) {
        return Some((
            "content_binary",
            "Derlenmiş program ya da betik pakete girmez: v1 kaynak paketidir, platform tekerleklerini kataloğun onaylı \
             kilidi üretir."
                .to_owned(),
        ));
    }
    if matches!(
        ext.as_str(),
        "zip" | "whl" | "egg" | "kek" | "tar" | "gz" | "tgz" | "bz2" | "xz" | "7z" | "rar" | "zst"
    ) {
        return Some((
            "content_archive",
            "Paketin içinde arşiv olmaz: içindekiler denetlenemez. Dosyaları açık hâlde koyun."
                .to_owned(),
        ));
    }
    None
}

/// Native code by its first bytes, whatever its name.
fn native_code(data: &[u8]) -> Option<(&'static str, String)> {
    let pe = data.starts_with(b"MZ")
        && data
            .get(0x3c..0x40)
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize)
            .and_then(|at| data.get(at..at.checked_add(4)?))
            .is_some_and(|signature| signature == b"PE\0\0");
    let mach_o: [&[u8]; 5] = [
        b"\xfe\xed\xfa\xce",
        b"\xfe\xed\xfa\xcf",
        b"\xce\xfa\xed\xfe",
        b"\xcf\xfa\xed\xfe",
        b"\xca\xfe\xba\xbe",
    ];
    let what = if data.starts_with(b"\x7fELF") {
        "Linux programı (ELF)"
    } else if pe {
        "Windows programı (PE)"
    } else if mach_o.iter().any(|magic| data.starts_with(magic)) {
        "macOS programı (Mach-O) ya da Java sınıfı"
    } else if data.starts_with(b"\0asm") {
        "WebAssembly modülü"
    } else {
        return None;
    };
    Some((
        "content_binary",
        format!("Dosya bir {what}; derlenmiş kod pakete girmez."),
    ))
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Stage {
    /// A package: its integrity list must hold.
    Inspect,
    /// A folder being packed: the packer writes the list.
    Pack,
}

/// The content's checks, the same for a package and for a folder being packed.
fn check_tree(files: &BTreeMap<String, Vec<u8>>, stage: Stage, out: &mut Inspection) {
    if stage == Stage::Inspect {
        match files.get(integrity::FILE).map(|b| std::str::from_utf8(b)) {
            None => out.problems.push(
                Problem::new(
                    "integrity_missing_file",
                    "Pakette integrity.json yok; paketi “kentos kek pack” ile oluşturun.",
                )
                .file(integrity::FILE),
            ),
            Some(Err(_)) => out.problems.push(
                Problem::new("integrity_json", "integrity.json UTF-8 değil.").file(integrity::FILE),
            ),
            Some(Ok(text)) => match Integrity::read(text) {
                Err(problem) => out.problems.push(problem),
                Ok(list) => {
                    let found = list.verify(files);
                    if found.is_empty() {
                        out.digest = Some(list.digest());
                        out.files = list.files;
                    }
                    out.problems.extend(found);
                }
            },
        }
    }
    let Some(manifest_bytes) = files.get(manifest::FILE) else {
        out.problems.push(
            Problem::new(
                "manifest_missing",
                "Pakette manifest.json yok; paketin kökünde olmalı (bir klasörün içinde değil).",
            )
            .file(manifest::FILE),
        );
        return;
    };
    let Ok(manifest_text) = std::str::from_utf8(manifest_bytes) else {
        out.problems
            .push(Problem::new("text_encoding", "manifest.json UTF-8 değil.").file(manifest::FILE));
        return;
    };
    let manifest = match Manifest::read(manifest_text) {
        Ok(manifest) => manifest,
        Err(problem) => {
            out.problems.push(problem);
            return;
        }
    };
    let (problems, warnings) = manifest.check();
    out.problems.extend(problems);
    out.warnings.extend(warnings);
    check_references(&manifest, files, out);
    for (path, data) in files {
        text::scan(path, data, &mut out.problems, &mut out.warnings);
    }
    if manifest.changelog_file.is_none() {
        out.warnings.push(
            Problem::new(
                "catalog_changelog",
                "Değişiklik notu (changelogFile) yok: güncellemeden önce kullanıcıya gösterilir ve genel katalogda \
                 zorunludur.",
            )
            .file(manifest::FILE)
            .at("/changelogFile"),
        );
    }
    out.manifest = Some(manifest);
}

/// Every file the manifest names is in the package and is what its field says; every handler's module is there.
fn check_references(manifest: &Manifest, files: &BTreeMap<String, Vec<u8>>, out: &mut Inspection) {
    let text_of = |path: &str| files.get(path).and_then(|b| std::str::from_utf8(b).ok());
    let mut python: (Option<String>, Option<String>) = (None, None);
    let mut checked = BTreeSet::new();
    for reference in manifest.references() {
        let path = reference.path.as_str();
        if paths::problem(path).is_some() {
            // Said already, by the manifest's own rules.
            continue;
        }
        let missing = |out: &mut Inspection| {
            out.problems.push(
                Problem::new(
                    "reference_missing",
                    format!("Manifest “{path}” dosyasını gösteriyor ama dosya pakette yok."),
                )
                .file(manifest::FILE)
                .at(reference.at.clone()),
            );
        };
        let Some(data) = files.get(path) else {
            missing(out);
            continue;
        };
        if !checked.insert((path.to_owned(), reference.role)) {
            continue;
        }
        let problem =
            |code: &'static str, message: String| Problem::new(code, message).file(path.to_owned());
        let json = |data: &[u8]| -> Result<serde_json::Value, String> {
            let text = std::str::from_utf8(data).map_err(|_| "UTF-8 değil".to_owned())?;
            let value: serde_json::Value = serde_json::from_str(text).map_err(|e| {
                format!(
                    "geçerli JSON değil (satır {}, sütun {})",
                    e.line(),
                    e.column()
                )
            })?;
            if value.is_object() {
                Ok(value)
            } else {
                Err("bir JSON nesnesi ({…}) değil".to_owned())
            }
        };
        match reference.role {
            FileRole::License | FileRole::Changelog => {
                if text_of(path).is_none_or(|t| t.trim().is_empty()) {
                    out.problems.push(problem(
                        "reference_empty",
                        "Dosya boş ya da UTF-8 metin değil.".to_owned(),
                    ));
                }
            }
            FileRole::Icon => match text_of(path).map(icon::problem) {
                None => out
                    .problems
                    .push(problem("icon_unsafe", "İkon UTF-8 metin değil.".to_owned())),
                Some(Some(why)) => out.problems.push(problem("icon_unsafe", why)),
                Some(None) => {}
            },
            FileRole::Schema | FileRole::Translation | FileRole::PanelDefinition => {
                if let Err(why) = json(data) {
                    out.problems
                        .push(problem("reference_json", format!("Dosya {why}.")));
                }
            }
            FileRole::ToolDescriptor => {
                let id = manifest
                    .entrypoints
                    .processing
                    .iter()
                    .find(|p| p.descriptor == path)
                    .map(|p| p.id.as_str())
                    .unwrap_or_default();
                match json(data) {
                    Err(why) => out
                        .problems
                        .push(problem("tool_descriptor", format!("Araç tanımı {why}."))),
                    Ok(value) => {
                        let field = |k: &str| value.get(k);
                        if field("format").and_then(|f| f.as_str()) != Some("kentos.processingTool")
                            || field("schemaVersion").and_then(serde_json::Value::as_u64) != Some(1)
                        {
                            out.problems.push(problem(
                                "tool_descriptor",
                                "Araç tanımı “kentos.processingTool” sürüm 1 değil (format, schemaVersion).".to_owned(),
                            ));
                        } else if field("id").and_then(|f| f.as_str()) != Some(id) {
                            out.problems.push(problem(
                                "tool_descriptor",
                                format!("Araç tanımının kimliği (id) manifestteki “{id}” ile aynı olmalı."),
                            ));
                        }
                    }
                }
            }
            FileRole::Form => {
                let package = std::str::from_utf8(data)
                    .map_err(|_| "UTF-8 değil".to_owned())
                    .and_then(|t| {
                        serde_json::from_str::<kentos_commands::FormPackage>(t).map_err(|e| {
                            format!(
                                "okunamadı: {} (satır {})",
                                e.to_string().split(" at line ").next().unwrap_or_default(),
                                e.line()
                            )
                        })
                    });
                match package {
                    Err(why) => out
                        .problems
                        .push(problem("form_invalid", format!("Form paketi {why}."))),
                    Ok(form) => {
                        if let Some((place, why)) = form.problem() {
                            out.problems.push(problem("form_invalid", why).at(place));
                        } else {
                            form_preflight(path, &form, out);
                        }
                    }
                }
            }
            FileRole::GeoPackage => {
                if !data.starts_with(b"SQLite format 3\0") {
                    out.problems.push(problem(
                        "geopackage_invalid",
                        "Dosya bir GeoPackage (SQLite) değil.".to_owned(),
                    ));
                }
            }
            FileRole::PythonProject => python.0 = Some(path.to_owned()),
            FileRole::PythonLock => python.1 = Some(path.to_owned()),
        }
    }
    if let (Some(project_path), Some(lock_path)) = (python.0.as_deref(), python.1.as_deref()) {
        match (text_of(project_path), text_of(lock_path)) {
            (Some(project), Some(lock)) => python::check(
                manifest,
                project_path,
                project,
                lock_path,
                lock,
                &mut out.problems,
                &mut out.warnings,
            ),
            _ => out.problems.push(
                Problem::new("text_encoding", "pyproject.toml ya da uv.lock UTF-8 değil.")
                    .file(project_path.to_owned()),
            ),
        }
    }
    let base = manifest
        .python
        .as_ref()
        .and_then(|p| p.project_file.rsplit_once('/'))
        .map(|(folder, _)| format!("{folder}/"))
        .unwrap_or_default();
    for (at, handler) in manifest.handlers() {
        let Some((module, _)) = handler.split_once(':') else {
            continue;
        };
        let module = module.replace('.', "/");
        let candidates = [
            format!("{base}src/{module}.py"),
            format!("{base}src/{module}/__init__.py"),
            format!("{base}{module}.py"),
            format!("{base}{module}/__init__.py"),
        ];
        if !candidates.iter().any(|c| files.contains_key(c)) {
            out.problems.push(
                Problem::new(
                    "handler_missing",
                    format!(
                        "“{handler}” işlevinin modülü pakette yok: {} ya da {} beklendi.",
                        candidates[0], candidates[2]
                    ),
                )
                .file(manifest::FILE)
                .at(at),
            );
        }
    }
}

/// The form core's preflight over the package's own fields (docs/adr/0401 §4): its errors are problems, but those
/// that need the project the form is installed into (a list from another layer, a relation) and its warnings are
/// warnings; the same check runs again there.
fn form_preflight(path: &str, form: &kentos_commands::FormPackage, out: &mut Inspection) {
    use kentos_forms::codes;
    for issue in kentos_forms::preflight(&form.body, &form.fields, &[]) {
        let needs_project = matches!(
            issue.code.as_str(),
            codes::LOOKUP_LAYER
                | codes::LOOKUP_FIELD
                | codes::LOOKUP_TYPE
                | codes::RELATION_UNKNOWN
        );
        let error = issue.severity == kentos_commands::FormSeverity::Error && !needs_project;
        let at: Vec<&str> = [issue.item.as_deref(), issue.rule.as_deref()]
            .into_iter()
            .flatten()
            .collect();
        let mut problem = Problem::new(
            if error {
                "form_preflight"
            } else {
                "form_preflight_warning"
            },
            format!("{} ({})", issue.message, issue.code),
        )
        .file(path.to_owned());
        if !at.is_empty() {
            problem = problem.at(at.join("/"));
        }
        if error {
            out.problems.push(problem);
        } else {
            out.warnings.push(problem);
        }
    }
}

/// A package made: its bytes, its content digest, what was left out and what should be mended.
#[derive(Clone, Debug, PartialEq)]
pub struct Packed {
    pub bytes: Vec<u8>,
    pub digest: String,
    /// The folder's files that never go into a package (an environment, caches, secrets …), so the author sees them.
    pub left_out: Vec<String>,
    pub warnings: Vec<Problem>,
}

/// Whether the packer leaves a folder's file out without asking: what every working folder has and no package carries.
fn left_out(path: &str) -> bool {
    let segments: Vec<&str> = path.split('/').collect();
    let folders = &segments[..segments.len() - 1];
    let dev = folders.iter().any(|f| {
        matches!(
            *f,
            ".pytest_cache"
                | ".mypy_cache"
                | ".ruff_cache"
                | ".tox"
                | ".nox"
                | ".ipynb_checkpoints"
        ) || f.ends_with(".egg-info")
    }) || folders
        .first()
        .is_some_and(|f| matches!(*f, "build" | "dist"));
    path == integrity::FILE
        || paths::extension(path) == "kek"
        || dev
        || forbidden(path).is_some_and(|(code, _)| {
            matches!(
                code,
                "content_junk"
                    | "content_environment"
                    | "content_cache"
                    | "content_vcs"
                    | "content_secret"
                    | "content_ai_config"
            )
        })
}

/// Packs a folder's files (path → bytes, `/` between folders) into a `.kek` package: the same checks as [`open`],
/// `integrity.json` written, entries in a fixed order (the manifest, the list, then by path) with a fixed date, so the
/// same files always make the same bytes. What every working folder has (an environment, caches, version control,
/// secrets, AI tool settings, earlier packages) is left out and listed.
pub fn pack(folder: &BTreeMap<String, Vec<u8>>, limits: &Limits) -> Result<Packed, Vec<Problem>> {
    let mut kept: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    let mut skipped = Vec::new();
    for (path, data) in folder {
        if left_out(path) {
            skipped.push(path.clone());
        } else {
            kept.insert(path.clone(), data.clone());
        }
    }
    let mut problems = Vec::new();
    let mut seen: BTreeMap<String, &str> = BTreeMap::new();
    let mut total: u64 = 0;
    if kept.len() + 1 > limits.entries {
        problems.push(Problem::new(
            "archive_entries",
            format!(
                "Klasörde {} dosya var; bir paket en çok {} dosya taşır.",
                kept.len(),
                limits.entries.saturating_sub(1)
            ),
        ));
    }
    for (path, data) in &kept {
        let refuse = |problems: &mut Vec<Problem>, code: &'static str, message: String| {
            problems.push(Problem::new(code, message).file(path.clone()));
        };
        if let Some(problem) = paths::problem(path) {
            refuse(&mut problems, "path_unsafe", problem);
            continue;
        }
        if paths::depth(path) > limits.depth {
            refuse(
                &mut problems,
                "path_depth",
                format!(
                    "Yol {} klasör derinliğinde; en çok {} olur.",
                    paths::depth(path),
                    limits.depth
                ),
            );
        }
        if let Some(first) = seen.insert(paths::folded(path), path) {
            refuse(
                &mut problems,
                "path_case",
                format!(
                    "“{first}” ile yalnız büyük/küçük harfle ayrılıyor; Windows ve macOS ikisini aynı dosya sayar."
                ),
            );
        }
        if let Some((code, message)) = forbidden(path).or_else(|| native_code(data)) {
            refuse(&mut problems, code, message);
        }
        if data.len() as u64 > limits.file {
            refuse(
                &mut problems,
                "archive_file_too_large",
                format!(
                    "Dosya {} MB; bir dosya en çok {} MB olur.",
                    data.len() / (1024 * 1024),
                    limits.file / (1024 * 1024)
                ),
            );
        }
        total = total.saturating_add(data.len() as u64);
    }
    if total > limits.total {
        problems.push(Problem::new(
            "archive_total",
            format!(
                "Dosyalar toplam {} MB; en çok {} MB olur.",
                total / (1024 * 1024),
                limits.total / (1024 * 1024)
            ),
        ));
    }
    if !problems.is_empty() {
        return Err(problems);
    }
    let mut checked = Inspection::default();
    check_tree(&kept, Stage::Pack, &mut checked);
    if !checked.is_valid() {
        return Err(checked.problems);
    }
    let list = Integrity::of(kept.iter().map(|(p, d)| (p.as_str(), d.as_slice())));
    let list_json = list.to_json();
    let mut entries: Vec<(&str, &[u8], Pack)> = Vec::with_capacity(kept.len() + 1);
    if let Some(manifest) = kept.get(manifest::FILE) {
        entries.push((manifest::FILE, manifest, Pack::Best));
    }
    entries.push((integrity::FILE, list_json.as_bytes(), Pack::Best));
    entries.extend(
        kept.iter()
            .filter(|(p, _)| p.as_str() != manifest::FILE)
            .map(|(p, d)| (p.as_str(), d.as_slice(), Pack::Best)),
    );
    let bytes =
        kentos_zip::write_with(&entries).map_err(|e| vec![Problem::new("archive_write", e)])?;
    // What is packed opens: the packer's output is the inspector's input, checked once more.
    let opened = inspect(&bytes, limits);
    if !opened.is_valid() {
        return Err(opened.problems);
    }
    Ok(Packed {
        bytes,
        digest: list.digest(),
        left_out: skipped,
        warnings: checked.warnings,
    })
}

#[cfg(test)]
mod tests;
