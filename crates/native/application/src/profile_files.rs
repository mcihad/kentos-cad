//! An institution's own data profile (docs/adr/0558; plan/gis.md CBS-34's remainder).
//!
//! [`import`] is `cad.profiles.import`: a profile or a mapping the user brings, checked and kept as a file of the
//! project's `profiller` folder beside the drawing (written beside its place, read back, then moved there).
//! [`list`] is `cad.profiles.list`: KentOS's profiles and the folder's. [`migrate`] is `cad.profiles.migrate`: the
//! project's data moved from one release of a profile to a later one by the steps the profile lists, as one undo step
//! “Profil göçü”, the values that did not fit reported by value. The shared cases are
//! `fixtures/commands/v1/cad.profiles.{import,list,migrate}.json` (`scripts/fixtures/profile_files_cases.py`).

use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};

use kentos_commands::{
    BUILTIN_PROFILES, CommandResult, CommandWarning, ConvertFail, DataProfile, Entity, FieldDomain,
    LayerField, LossKind, MAPPING_FORMAT, MAPPING_SUFFIX, MAX_LOSS_UIDS, MAX_MAPPING_ITEMS,
    MULTIPLE_JOIN, MigratedStep, MigrationLoss, MigrationStep, PROFILE_FOLDER, PROFILE_FORMAT,
    PROFILE_SUFFIX, ProfileFileKind, ProfileImported, ProfileListed, ProfileMapping, ProfileSource,
    ProfilesImport, ProfilesList, ProfilesListed, ProfilesMigrate, ProfilesMigrated,
    builtin_profile, check_value, fold_name, follow_domains, js_trim, mapping_problem,
    profile_file_name_ok, profile_problem,
};
use kentos_domain::{Document, Slot};

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;
use crate::profiles::{self, Place};

/// The undo step's name.
pub const STEP: &str = "Profil göçü";

/// The largest profile or mapping file read.
const MOST_BYTES: u64 = 4 * 1024 * 1024;

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.into())))
}

fn warn(code: &str, message: String, path: Option<String>) -> CommandWarning {
    CommandWarning {
        code: code.into(),
        message,
        path,
    }
}

// ── The project's folder ───────────────────────────────────────────────────────────────────────────────────────

/// The `profiller` folder beside the drawing's file; none when the drawing has no (absolute) file.
pub(crate) fn folder_of(drawing: Option<&str>) -> Option<PathBuf> {
    let t = drawing?.trim();
    let p = Path::new(t);
    if t.is_empty() || !p.is_absolute() {
        return None;
    }
    Some(p.parent()?.join(PROFILE_FOLDER))
}

fn read_json(path: &Path) -> Result<serde_json::Value, String> {
    let len = std::fs::metadata(path)
        .map_err(|e| format!("{} okunamadı: {e}", path.display()))?
        .len();
    if len > MOST_BYTES {
        return Err(format!(
            "{} {} MB; en çok {} MB okunur.",
            path.display(),
            len / 1_048_576,
            MOST_BYTES / 1_048_576
        ));
    }
    let bytes = std::fs::read(path).map_err(|e| format!("{} okunamadı: {e}", path.display()))?;
    serde_json::from_slice(&bytes).map_err(|e| format!("JSON okunamadı: {e}"))
}

/// A profile that holds: its format, its version, and everything else `profile_problem` asks.
fn check_profile(p: &DataProfile) -> Result<(), (&'static str, String)> {
    if p.format != PROFILE_FORMAT {
        return Err((
            codes::UNKNOWN_PROFILE_FORMAT,
            format!("Profilin biçimi “{}”; “{PROFILE_FORMAT}” olmalı.", p.format),
        ));
    }
    if p.version != 1 {
        return Err((
            codes::UNSUPPORTED_PROFILE_VERSION,
            format!(
                "Profilin biçim sürümü {}; bu KentOS 1. sürümü okur.",
                p.version
            ),
        ));
    }
    match profile_problem(p) {
        Some(why) => Err((
            codes::INVALID_PROFILE,
            format!("Veri profili geçersiz: {why}"),
        )),
        None => Ok(()),
    }
}

/// What a mapping must hold without its profile at hand: its format and version, a plain name, a title, classes.
fn mapping_shape(m: &ProfileMapping) -> Result<(), (&'static str, String)> {
    if m.format != MAPPING_FORMAT {
        return Err((
            codes::UNKNOWN_PROFILE_FORMAT,
            format!(
                "Eşlemenin biçimi “{}”; “{MAPPING_FORMAT}” olmalı.",
                m.format
            ),
        ));
    }
    if m.version != 1 {
        return Err((
            codes::UNSUPPORTED_PROFILE_VERSION,
            format!(
                "Eşlemenin biçim sürümü {}; bu KentOS 1. sürümü okur.",
                m.version
            ),
        ));
    }
    let bad = |why: String| {
        Err((
            codes::INVALID_PROFILE_MAPPING,
            format!("Eşleme geçersiz: {why}"),
        ))
    };
    if !profile_file_name_ok(&m.name) {
        return bad(format!(
            "“{}” eşleme adı olamaz: küçük harf, rakam ve tire, en çok 64.",
            m.name
        ));
    }
    if m.title.trim().is_empty() {
        return bad("Eşlemenin başlığı boş olamaz.".into());
    }
    if m.classes.is_empty() || m.classes.len() > MAX_MAPPING_ITEMS {
        return bad(format!(
            "Eşlemede 1 ile {MAX_MAPPING_ITEMS} arası sınıf olur."
        ));
    }
    Ok(())
}

/// A JSON value read as a profile or a mapping, by its `format` and `version`.
enum Parsed {
    Profile(Box<DataProfile>),
    Mapping(ProfileMapping),
}

fn parse(value: serde_json::Value) -> Result<Parsed, (&'static str, String)> {
    let format = value
        .get("format")
        .and_then(|f| f.as_str())
        .map(str::to_owned);
    let version = value.get("version").and_then(serde_json::Value::as_u64);
    match format.as_deref() {
        Some(PROFILE_FORMAT) => {
            if version != Some(1) {
                return Err((
                    codes::UNSUPPORTED_PROFILE_VERSION,
                    format!(
                        "Profilin biçim sürümü {}; bu KentOS 1. sürümü okur.",
                        version.map_or("yok".to_owned(), |v| v.to_string())
                    ),
                ));
            }
            let p: DataProfile = serde_json::from_value(value).map_err(|e| {
                (
                    codes::INVALID_PROFILE,
                    format!("Veri profili geçersiz: {e}"),
                )
            })?;
            check_profile(&p)?;
            Ok(Parsed::Profile(Box::new(p)))
        }
        Some(MAPPING_FORMAT) => {
            if version != Some(1) {
                return Err((
                    codes::UNSUPPORTED_PROFILE_VERSION,
                    format!(
                        "Eşlemenin biçim sürümü {}; bu KentOS 1. sürümü okur.",
                        version.map_or("yok".to_owned(), |v| v.to_string())
                    ),
                ));
            }
            let m: ProfileMapping = serde_json::from_value(value).map_err(|e| {
                (
                    codes::INVALID_PROFILE_MAPPING,
                    format!("Eşleme geçersiz: {e}"),
                )
            })?;
            mapping_shape(&m)?;
            Ok(Parsed::Mapping(m))
        }
        _ => Err((
            codes::UNKNOWN_PROFILE_FORMAT,
            format!(
                "Dosyanın biçimi tanınmıyor: veri profili için “{PROFILE_FORMAT}”, eşleme için “{MAPPING_FORMAT}” yazılı olmalı."
            ),
        )),
    }
}

/// The project's profile of `name`: none when the drawing has no file or the folder no such file; why not, when the
/// file does not read or does not hold. The name inside is the file's.
pub(crate) fn load_profile(
    drawing: Option<&str>,
    name: &str,
) -> Result<Option<DataProfile>, String> {
    if !profile_file_name_ok(name) {
        return Ok(None);
    }
    let Some(folder) = folder_of(drawing) else {
        return Ok(None);
    };
    let path = folder.join(format!("{name}{PROFILE_SUFFIX}"));
    if !path.is_file() {
        return Ok(None);
    }
    match parse(read_json(&path)?).map_err(|(_, why)| why)? {
        Parsed::Profile(p) if p.name == name => Ok(Some(*p)),
        Parsed::Profile(p) => Err(format!(
            "{} dosyasının içindeki profilin adı “{}”; dosya adıyla aynı olmalı.",
            path.display(),
            p.name
        )),
        Parsed::Mapping(_) => Err(format!(
            "{} bir veri profili değil, eşleme.",
            path.display()
        )),
    }
}

/// The project's mapping of `name`, as [`load_profile`] reads a profile.
pub(crate) fn load_mapping(
    drawing: Option<&str>,
    name: &str,
) -> Result<Option<ProfileMapping>, String> {
    if !profile_file_name_ok(name) {
        return Ok(None);
    }
    let Some(folder) = folder_of(drawing) else {
        return Ok(None);
    };
    let path = folder.join(format!("{name}{MAPPING_SUFFIX}"));
    if !path.is_file() {
        return Ok(None);
    }
    match parse(read_json(&path)?).map_err(|(_, why)| why)? {
        Parsed::Mapping(m) if m.name == name => Ok(Some(m)),
        Parsed::Mapping(m) => Err(format!(
            "{} dosyasının içindeki eşlemenin adı “{}”; dosya adıyla aynı olmalı.",
            path.display(),
            m.name
        )),
        Parsed::Profile(_) => Err(format!(
            "{} bir eşleme değil, veri profili.",
            path.display()
        )),
    }
}

/// A profile by name: KentOS's, else the project's file beside the drawing.
pub fn find_profile(drawing: Option<&str>, name: &str) -> Option<DataProfile> {
    builtin_profile(name).or_else(|| load_profile(drawing, name).ok().flatten())
}

// ── List ────────────────────────────────────────────────────────────────────────────────────────────────────────

pub mod list {
    use super::*;

    /// The built-in profiles and the folder's files (a window reads them as the command does).
    pub fn run(input: &ProfilesList) -> ProfilesListed {
        let mut items = Vec::new();
        for (name, _) in BUILTIN_PROFILES {
            let Some(p) = builtin_profile(name) else {
                continue;
            };
            items.push(ProfileListed {
                kind: ProfileFileKind::Profile,
                source: ProfileSource::Builtin,
                name: (*name).to_owned(),
                title: p.title.clone(),
                release: p.release,
                classes: p.classes.len() as u32,
                migrations: p.migrations.len() as u32,
                profile: None,
                path: None,
                problem: None,
            });
        }
        let folder = folder_of(input.drawing.as_deref());
        if let Some(dir) = &folder
            && let Ok(read) = std::fs::read_dir(dir)
        {
            let mut found: Vec<(String, ProfileFileKind, PathBuf)> = Vec::new();
            for entry in read.flatten() {
                let file = entry.file_name().to_string_lossy().into_owned();
                let (name, kind) = if let Some(n) = file.strip_suffix(PROFILE_SUFFIX) {
                    (n, ProfileFileKind::Profile)
                } else if let Some(n) = file.strip_suffix(MAPPING_SUFFIX) {
                    (n, ProfileFileKind::Mapping)
                } else {
                    continue;
                };
                if name.is_empty() || !entry.path().is_file() {
                    continue;
                }
                found.push((name.to_owned(), kind, entry.path()));
            }
            found.sort_by(|a, b| {
                (a.0.as_str(), a.1 == ProfileFileKind::Mapping)
                    .cmp(&(b.0.as_str(), b.1 == ProfileFileKind::Mapping))
            });
            for (name, kind, path) in found {
                let loaded = match kind {
                    ProfileFileKind::Profile => load_profile(input.drawing.as_deref(), &name)
                        .and_then(|o| o.ok_or_else(|| "dosya okunamadı".to_owned()))
                        .map(|p| Parsed::Profile(Box::new(p))),
                    ProfileFileKind::Mapping => load_mapping(input.drawing.as_deref(), &name)
                        .and_then(|o| o.ok_or_else(|| "dosya okunamadı".to_owned()))
                        .map(Parsed::Mapping),
                };
                let mut item = ProfileListed {
                    kind,
                    source: ProfileSource::Project,
                    name: name.clone(),
                    title: String::new(),
                    release: 0,
                    classes: 0,
                    migrations: 0,
                    profile: None,
                    path: Some(path.display().to_string()),
                    problem: None,
                };
                match loaded {
                    Ok(Parsed::Profile(p)) => {
                        item.title = p.title.clone();
                        item.release = p.release;
                        item.classes = p.classes.len() as u32;
                        item.migrations = p.migrations.len() as u32;
                    }
                    Ok(Parsed::Mapping(m)) => {
                        item.title = m.title.clone();
                        item.classes = m.classes.len() as u32;
                        // A mapping that does not hold for its profile is listed with why.
                        item.problem = find_profile(input.drawing.as_deref(), &m.profile)
                            .and_then(|p| mapping_problem(&m, &p));
                        item.profile = Some(m.profile.clone());
                    }
                    Err(why) => item.problem = Some(why),
                }
                items.push(item);
            }
        }
        ProfilesListed {
            items,
            folder: folder.map(|f| f.display().to_string()),
        }
    }

    pub fn validate(_cx: &ExecutionContext<'_>, _input: &ProfilesList) -> CommandResult<()> {
        CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        }
    }

    pub fn plan(_cx: &ExecutionContext<'_>, input: &ProfilesList) -> CommandResult<ProfilesListed> {
        CommandResult::Completed {
            output: run(input),
            warnings: Vec::new(),
        }
    }

    pub fn execute(
        _cx: &mut ExecutionContext<'_>,
        input: ProfilesList,
    ) -> CommandResult<ProfilesListed> {
        CommandResult::Completed {
            output: run(&input),
            warnings: Vec::new(),
        }
    }
}

// ── Import ──────────────────────────────────────────────────────────────────────────────────────────────────────

pub mod import {
    use super::*;

    /// What the import will write and say.
    struct Planned {
        out: ProfileImported,
        bytes: Vec<u8>,
        target: PathBuf,
        warnings: Vec<CommandWarning>,
    }

    /// A path the input gives for a file to read: absolute, or from the drawing's folder.
    fn source_path(text: &str, drawing: Option<&str>) -> Result<PathBuf, Stop> {
        let t = text.trim();
        if t.is_empty() {
            return Err(fail(
                codes::INVALID_FILE_PATH,
                "Dosya yolu boş; bir JSON dosyasının yolunu verin.".into(),
                "source",
            ));
        }
        let p = Path::new(t);
        if p.is_absolute() {
            return Ok(p.to_path_buf());
        }
        match drawing
            .map(str::trim)
            .map(Path::new)
            .filter(|d| d.is_absolute())
            .and_then(Path::parent)
        {
            Some(folder) => Ok(crate::source_places::resolve(folder, &t.replace('\\', "/"))),
            None => Err(fail(
                codes::INVALID_FILE_PATH,
                format!(
                    "“{t}” göreli bir yol, ama çizimin dosyası yok; mutlak yolu verin ya da çizimi önce kaydedin."
                ),
                "source",
            )),
        }
    }

    fn drawing_folder(drawing: Option<&str>) -> Result<PathBuf, Stop> {
        let Some(folder) = folder_of(drawing) else {
            return Err(fail(
                codes::INVALID_FILE_PATH,
                format!(
                    "Profil çizimin dosyasının yanındaki “{PROFILE_FOLDER}” klasörüne yazılır; çizimin dosyası yok. Çizimi önce kaydedin ya da dosyasının mutlak yolunu (drawing) verin."
                ),
                "drawing",
            ));
        };
        match folder.parent() {
            Some(p) if p.is_dir() => Ok(folder),
            _ => Err(fail(
                codes::INVALID_FILE_PATH,
                "Çizimin klasörü yok; profil yazılamaz.".into(),
                "drawing",
            )),
        }
    }

    fn refuse_parsed((code, why): (&'static str, String), path: &str) -> Stop {
        fail(code, why, path)
    }

    fn check(input: &ProfilesImport) -> Result<Planned, Stop> {
        let given = usize::from(input.source.is_some())
            + usize::from(input.profile.is_some())
            + usize::from(input.mapping.is_some());
        if given != 1 {
            return Err(fail(
                codes::PROFILE_SOURCE_MISSING,
                "İçe aktarılacak tek bir kaynak verin: bir JSON dosyası (source), profil (profile) ya da eşleme (mapping).".into(),
                "source",
            ));
        }
        let drawing = input.drawing.as_deref();
        let source = match &input.source {
            Some(t) => Some(source_path(t, drawing)?),
            None => None,
        };
        let folder = drawing_folder(drawing)?;
        let parsed = if let Some(path) = &source {
            if !path.is_file() {
                return Err(fail(
                    codes::FILE_NOT_FOUND,
                    format!("{} dosyası yok.", path.display()),
                    "source",
                ));
            }
            let value =
                read_json(path).map_err(|why| fail(codes::INVALID_PROFILE, why, "source"))?;
            parse(value).map_err(|e| refuse_parsed(e, "source"))?
        } else if let Some(p) = &input.profile {
            check_profile(p).map_err(|e| refuse_parsed(e, "profile"))?;
            Parsed::Profile(Box::new(p.clone()))
        } else if let Some(m) = &input.mapping {
            mapping_shape(m).map_err(|e| refuse_parsed(e, "mapping"))?;
            Parsed::Mapping(m.clone())
        } else {
            unreachable!("one source is given")
        };
        let overwrite = input.overwrite == Some(true);
        let mut warnings = Vec::new();
        match parsed {
            Parsed::Profile(p) => {
                if !profile_file_name_ok(&p.name) {
                    return Err(fail(
                        codes::INVALID_PROFILE,
                        format!(
                            "“{}” projenin profilinin adı olamaz: küçük harf, rakam ve tire, en çok 64 (dosyanın adı olur).",
                            p.name
                        ),
                        "profile",
                    ));
                }
                if builtin_profile(&p.name).is_some() {
                    return Err(fail(
                        codes::PROFILE_NAME_TAKEN,
                        format!(
                            "“{}” KentOS'un kendi profilinin adı; projenin profiline başka bir ad verin.",
                            p.name
                        ),
                        "profile",
                    ));
                }
                let target = folder.join(format!("{}{PROFILE_SUFFIX}", p.name));
                let mut previous = None;
                if target.is_file() {
                    let held = load_profile(drawing, &p.name).ok().flatten();
                    previous = held.as_ref().map(|h| h.release);
                    if !overwrite {
                        match &held {
                            Some(h) if h.release > p.release => {
                                return Err(fail(
                                    codes::PROFILE_DOWNGRADE,
                                    format!(
                                        "Projedeki “{}” profili {}. sürümde; içe aktarılan {}. sürüm daha eski. Üzerine yazmak için overwrite verin.",
                                        p.name, h.release, p.release
                                    ),
                                    "profile",
                                ));
                            }
                            Some(h) if h.release < p.release => {}
                            _ => {
                                return Err(fail(
                                    codes::FILE_EXISTS,
                                    format!(
                                        "{} var; aynı sürümün üzerine yazmak için overwrite verin.",
                                        target.display()
                                    ),
                                    "profile",
                                ));
                            }
                        }
                    }
                    warnings.push(warn(
                        codes::PROFILE_REPLACED,
                        match previous {
                            Some(r) => format!(
                                "Projedeki “{}” profili {r}. sürümden {}. sürüme değiştirildi.",
                                p.name, p.release
                            ),
                            None => format!(
                                "Projedeki “{}” profilinin okunamayan dosyası yenisiyle değiştirildi.",
                                p.name
                            ),
                        },
                        Some("profile".into()),
                    ));
                }
                let bytes = pretty(&p);
                Ok(Planned {
                    out: ProfileImported {
                        kind: ProfileFileKind::Profile,
                        name: p.name.clone(),
                        title: p.title.clone(),
                        release: p.release,
                        previous_release: previous,
                        classes: p.classes.len() as u32,
                        migrations: p.migrations.len() as u32,
                        path: target.display().to_string(),
                        replaced: target.is_file(),
                        bytes: bytes.len() as u64,
                    },
                    bytes,
                    target,
                    warnings,
                })
            }
            Parsed::Mapping(m) => {
                match find_profile(drawing, &m.profile) {
                    Some(p) => {
                        if let Some(why) = mapping_problem(&m, &p) {
                            return Err(fail(
                                codes::INVALID_PROFILE_MAPPING,
                                format!("Eşleme geçersiz: {why}"),
                                "mapping",
                            ));
                        }
                        let through = kentos_commands::through(&p, &m);
                        if let Some(why) = profile_problem(&through) {
                            return Err(fail(
                                codes::INVALID_PROFILE_MAPPING,
                                format!(
                                    "Eşleme geçersiz: profil eşlemeyle okununca tutmuyor: {why}"
                                ),
                                "mapping",
                            ));
                        }
                    }
                    None => warnings.push(warn(
                        codes::MAPPING_UNCHECKED,
                        format!(
                            "“{}” profili bulunamadı; eşlemenin adları profille denetlenmedi.",
                            m.profile
                        ),
                        Some("mapping".into()),
                    )),
                }
                let target = folder.join(format!("{}{MAPPING_SUFFIX}", m.name));
                if target.is_file() {
                    if !overwrite {
                        return Err(fail(
                            codes::FILE_EXISTS,
                            format!(
                                "{} var; üzerine yazmak için overwrite verin.",
                                target.display()
                            ),
                            "mapping",
                        ));
                    }
                    warnings.push(warn(
                        codes::PROFILE_REPLACED,
                        format!("Projedeki “{}” eşlemesi değiştirildi.", m.name),
                        Some("mapping".into()),
                    ));
                }
                let bytes = pretty(&m);
                Ok(Planned {
                    out: ProfileImported {
                        kind: ProfileFileKind::Mapping,
                        name: m.name.clone(),
                        title: m.title.clone(),
                        release: 0,
                        previous_release: None,
                        classes: m.classes.len() as u32,
                        migrations: 0,
                        path: target.display().to_string(),
                        replaced: target.is_file(),
                        bytes: bytes.len() as u64,
                    },
                    bytes,
                    target,
                    warnings,
                })
            }
        }
    }

    /// A file's text: indented JSON ending in a newline.
    fn pretty<T: serde::Serialize>(value: &T) -> Vec<u8> {
        let mut text = serde_json::to_string_pretty(value).unwrap_or_default();
        text.push('\n');
        text.into_bytes()
    }

    pub fn validate(_cx: &ExecutionContext<'_>, input: &ProfilesImport) -> CommandResult<()> {
        match check(input) {
            Ok(p) => CommandResult::Completed {
                output: (),
                warnings: p.warnings,
            },
            Err(stop) => stop.into(),
        }
    }

    /// What execute would write; writes nothing.
    pub fn plan(
        _cx: &ExecutionContext<'_>,
        input: &ProfilesImport,
    ) -> CommandResult<ProfileImported> {
        match check(input) {
            Ok(p) => CommandResult::Completed {
                output: p.out,
                warnings: p.warnings,
            },
            Err(stop) => stop.into(),
        }
    }

    /// The file's bytes written at `target` whole or not at all: beside it, read back as the readers will, then moved
    /// over it. The folder is made when it is not there.
    fn write_file(target: &Path, bytes: &[u8]) -> Result<(), String> {
        let folder = target.parent().ok_or("klasörü yok")?;
        std::fs::create_dir_all(folder)
            .map_err(|e| format!("{} açılamadı: {e}", folder.display()))?;
        let mut tmp = target.as_os_str().to_owned();
        tmp.push(".kentos-yaziliyor");
        let tmp = PathBuf::from(tmp);
        let done = (|| -> Result<(), String> {
            std::fs::write(&tmp, bytes)
                .map_err(|e| format!("{} yazılamadı: {e}", tmp.display()))?;
            let back = std::fs::read(&tmp)
                .map_err(|e| format!("{} geri okunamadı: {e}", tmp.display()))?;
            if back != bytes {
                return Err("yazılan dosya geri okununca aynı çıkmadı".into());
            }
            std::fs::rename(&tmp, target)
                .map_err(|e| format!("{} yerine konamadı: {e}", target.display()))
        })();
        if done.is_err() {
            let _ = std::fs::remove_file(&tmp);
        }
        done
    }

    /// Checks `input` again and writes the file.
    pub fn execute(
        cx: &mut ExecutionContext<'_>,
        input: ProfilesImport,
    ) -> CommandResult<ProfileImported> {
        let p = match check(&input) {
            Ok(p) => p,
            Err(stop) => return stop.into(),
        };
        if !cx.dry()
            && let Err(why) = write_file(&p.target, &p.bytes)
        {
            return fail(
                codes::FILE_UNWRITABLE,
                format!("Profil yazılamadı: {why}. Dosya olduğu gibi kaldı."),
                "drawing",
            )
            .into();
        }
        CommandResult::Completed {
            output: p.out,
            warnings: p.warnings,
        }
    }
}

// ── Migrate ─────────────────────────────────────────────────────────────────────────────────────────────────────

pub mod migrate {
    use super::*;

    /// The steps to run, in order, with the migration each belongs to, after the checks that need no drawing.
    struct Chain {
        profile: DataProfile,
        from: u32,
        to: u32,
        steps: Vec<(u32, u32, MigrationStep)>,
    }

    fn chain(input: &ProfilesMigrate) -> Result<Chain, Stop> {
        let profile = profiles::profile_only(
            input.name.as_deref(),
            input.profile.as_ref(),
            input.drawing.as_deref(),
        )?;
        let from = input.from;
        let to = input.to.unwrap_or(profile.release);
        if from == 0 || to <= from || to > profile.release {
            return Err(fail(
                codes::INVALID_MIGRATION,
                format!(
                    "{from}. sürümden {to}. sürüme göç olmaz: hedef kaynaktan büyük ve en çok profilin sürümü ({}) olmalı.",
                    profile.release
                ),
                if input.to.is_some() { "to" } else { "from" },
            ));
        }
        let mut steps = Vec::new();
        for release in from..to {
            let Some(m) = profile.migrations.iter().find(|m| m.from == release) else {
                return Err(fail(
                    codes::MIGRATION_GAP,
                    format!(
                        "“{}” profilinde {release}. sürümden {}. sürüme göç yok; {from}. ile {to}. sürüm arasında kesintisiz yol kurulamaz.",
                        profile.name,
                        release + 1
                    ),
                    "from",
                ));
            };
            for (i, s) in m.steps.iter().enumerate() {
                steps.push((release, i as u32, s.clone()));
            }
        }
        Ok(Chain {
            profile,
            from,
            to,
            steps,
        })
    }

    fn check(doc: &Document, input: &ProfilesMigrate) -> Result<Chain, Stop> {
        let c = chain(input)?;
        checks::revision(doc, input.expected_revision.as_deref())?;
        Ok(c)
    }

    /// A step as it was run.
    struct Outcome {
        layer: Option<String>,
        applied: bool,
        objects: u32,
        note: String,
        warning: Option<(&'static str, String)>,
        losses: Vec<Loss>,
    }

    impl Outcome {
        fn skipped(layer: Option<String>, note: String) -> Self {
            Outcome {
                layer,
                applied: false,
                objects: 0,
                note,
                warning: None,
                losses: Vec::new(),
            }
        }

        fn missing(layer: Option<String>, note: String) -> Self {
            Outcome {
                warning: Some((codes::MIGRATION_STEP_SKIPPED, note.clone())),
                ..Outcome::skipped(layer, note)
            }
        }

        fn locked(layer: &str, name: &str) -> Self {
            let note = format!(
                "“{name}” katmanı kilitli; adım atlandı. Kilidi Katmanlar panelinden açın."
            );
            Outcome {
                warning: Some((codes::PROFILE_LAYER_SKIPPED, note.clone())),
                ..Outcome::skipped(Some(layer.to_owned()), note)
            }
        }
    }

    /// Objects whose value did not fit, by layer, field, how it ended and the value.
    struct Loss {
        layer: String,
        layer_name: String,
        field: String,
        kind: LossKind,
        value: String,
        uids: Vec<String>,
        count: u32,
    }

    fn refuse(why: impl Into<String>) -> Stop {
        fail(codes::LAYER_REFUSED, why.into(), "profile")
    }

    fn here(doc: &Document, path: &[String]) -> Result<Place, Stop> {
        Ok(profiles::find(doc.layers().nodes(), path))
    }

    /// The name of a layer, for a message.
    fn name_of(doc: &Document, id: &str) -> String {
        doc.layers()
            .get(id)
            .map_or_else(|| id.to_owned(), |n| n.name.clone())
    }

    fn words(path: &[String]) -> String {
        profiles::path_words(path)
    }

    fn run_step(
        doc: &mut Document,
        profile: &DataProfile,
        step: &MigrationStep,
        touched: &mut HashSet<Slot>,
    ) -> Result<Outcome, Stop> {
        match step {
            MigrationStep::RenameLayer { layer, to } => rename_layer(doc, layer, to),
            MigrationStep::RenameField { layer, from, to } => {
                rename_field(doc, layer, from, to, touched)
            }
            MigrationStep::ChangeFieldType {
                layer,
                field,
                on_fail,
            } => change_type(doc, profile, layer, field, *on_fail, touched),
            MigrationStep::MapValues { layer, field, map } => {
                map_values(doc, layer, field, map, touched)
            }
            MigrationStep::RetireCode {
                domain,
                code,
                replace_with,
            } => retire_code(doc, domain, code, replace_with.as_deref(), touched),
        }
    }

    fn rename_layer(doc: &mut Document, path: &[String], to: &str) -> Result<Outcome, Stop> {
        let to = to.trim();
        let id = match here(doc, path)? {
            Place::Layer(id) => id,
            Place::Blocked(_) | Place::Missing(..) => {
                let mut renamed: Vec<String> = path.to_vec();
                if let Some(last) = renamed.last_mut() {
                    *last = to.to_owned();
                }
                return Ok(match here(doc, &renamed)? {
                    Place::Layer(id) => Outcome::skipped(
                        Some(id),
                        format!("“{}” katmanı zaten “{to}” adıyla duruyor.", words(path)),
                    ),
                    _ => Outcome::missing(
                        None,
                        format!("“{}” katmanı çizimde yok; adım atlandı.", words(path)),
                    ),
                });
            }
        };
        let current = name_of(doc, &id);
        if current.trim() == to {
            return Ok(Outcome::skipped(
                Some(id),
                format!("“{current}” katmanı zaten bu adda."),
            ));
        }
        let mut renamed: Vec<String> = path.to_vec();
        if let Some(last) = renamed.last_mut() {
            *last = to.to_owned();
        }
        match here(doc, &renamed)? {
            Place::Layer(other) | Place::Blocked(other) if other != id => {
                return Err(refuse(format!(
                    "“{current}” katmanı “{to}” adını alamaz: aynı yerde bu adda bir katman ya da grup var."
                )));
            }
            _ => {}
        }
        doc.rename_layer(&id, to);
        Ok(Outcome {
            layer: Some(id),
            applied: true,
            objects: 0,
            note: format!("“{current}” katmanı “{to}” oldu."),
            warning: None,
            losses: Vec::new(),
        })
    }

    /// The layer a step works on when it can: its id and name; the outcome that leaves the step out, otherwise.
    fn layer_of(
        doc: &Document,
        path: &[String],
    ) -> Result<Result<(String, String), Outcome>, Stop> {
        Ok(match here(doc, path)? {
            Place::Layer(id) => {
                let name = name_of(doc, &id);
                if doc.layers().is_locked(&id) {
                    Err(Outcome::locked(&id, &name))
                } else {
                    Ok((id, name))
                }
            }
            _ => Err(Outcome::missing(
                None,
                format!("“{}” katmanı çizimde yok; adım atlandı.", words(path)),
            )),
        })
    }

    fn rename_field(
        doc: &mut Document,
        path: &[String],
        from: &str,
        to: &str,
        touched: &mut HashSet<Slot>,
    ) -> Result<Outcome, Stop> {
        let (id, name) = match layer_of(doc, path)? {
            Ok(x) => x,
            Err(o) => return Ok(o),
        };
        let fields = doc
            .layers()
            .get(&id)
            .map(|n| n.fields.clone())
            .unwrap_or_default();
        let Some(at) = fields.iter().position(|f| f.name == from) else {
            return Ok(if fields.iter().any(|f| f.name == to) {
                Outcome::skipped(
                    Some(id),
                    format!("“{name}” katmanının “{from}” alanı zaten “{to}” adıyla duruyor."),
                )
            } else {
                Outcome::missing(
                    Some(id),
                    format!("“{name}” katmanında “{from}” alanı yok; adım atlandı."),
                )
            });
        };
        if fields.iter().any(|f| f.name == to) {
            return Err(refuse(format!(
                "“{name}” katmanında “{from}” alanı “{to}” adını alamaz: bu adda bir alan zaten var."
            )));
        }
        let mut next = fields;
        next[at].name = to.to_owned();
        let holders: Vec<Slot> = doc
            .by_layer(&id)
            .filter(|e| e.base().attrs.contains_key(from))
            .map(|e| Slot(e.base().id))
            .collect();
        doc.set_layer_fields(&id, next, &[(from.to_owned(), to.to_owned())])
            .map_err(|r| refuse(r.0))?;
        let objects = holders.len() as u32;
        touched.extend(holders);
        Ok(Outcome {
            layer: Some(id),
            applied: true,
            objects,
            note: format!(
                "“{name}” katmanında “{from}” alanı “{to}” oldu; {objects} nesnenin değeri taşındı."
            ),
            warning: None,
            losses: Vec::new(),
        })
    }

    fn change_type(
        doc: &mut Document,
        profile: &DataProfile,
        path: &[String],
        field: &LayerField,
        on_fail: ConvertFail,
        touched: &mut HashSet<Slot>,
    ) -> Result<Outcome, Stop> {
        let (id, name) = match layer_of(doc, path)? {
            Ok(x) => x,
            Err(o) => return Ok(o),
        };
        let fields = doc
            .layers()
            .get(&id)
            .map(|n| n.fields.clone())
            .unwrap_or_default();
        let Some(at) = fields.iter().position(|f| f.name == field.name) else {
            return Ok(Outcome::missing(
                Some(id),
                format!(
                    "“{name}” katmanında “{}” alanı yok; adım atlandı.",
                    field.name
                ),
            ));
        };
        // A field that links a list the project lacks takes the profile's list of that id, in this step.
        let mut added_domain = None;
        if let Some(d) = &field.domain
            && !doc.settings().domains.iter().any(|x| x.id == *d)
            && let Some(own) = profile.domains.iter().find(|x| x.id == *d)
        {
            let mut all = doc.settings().domains.clone();
            all.push(own.clone());
            doc.set_domains(all, STEP).map_err(|r| refuse(r.0))?;
            added_domain = Some(own.name.clone());
        }
        let mut new = field.clone();
        if new.id.is_none() {
            new.id = fields[at].id.clone();
        }
        let (followed, _) = follow_domains(std::slice::from_ref(&new), &doc.settings().domains);
        let shown = followed.into_iter().next().unwrap_or_else(|| new.clone());
        if fields[at] == shown {
            return Ok(Outcome::skipped(
                Some(id),
                format!(
                    "“{name}” katmanının “{}” alanı zaten bu tanımda.",
                    field.name
                ),
            ));
        }
        // Each value through the new rule (a required field does not turn a blank away: blanks stay blank).
        let loose = LayerField {
            required: false,
            ..shown.clone()
        };
        let mut writes: Vec<(Slot, Entity)> = Vec::new();
        let mut losses: BTreeMap<(String, bool), (Vec<String>, u32)> = BTreeMap::new();
        for (uid, e) in doc.by_layer_with_uids(&id) {
            let Some(v) = e.base().attrs.get(&field.name) else {
                continue;
            };
            if js_trim(v).is_empty() {
                continue;
            }
            match check_value(&loose, v) {
                Ok(c) if c == *v => {}
                Ok(c) => {
                    let mut next = e.clone();
                    next.base_mut().attrs.insert(field.name.clone(), c);
                    writes.push((Slot(e.base().id), next));
                }
                Err(_) => {
                    let clear = on_fail == ConvertFail::Clear;
                    let entry = losses.entry((v.clone(), clear)).or_default();
                    entry.1 += 1;
                    if entry.0.len() < MAX_LOSS_UIDS {
                        entry.0.push(uid.to_string());
                    }
                    if clear {
                        let mut next = e.clone();
                        next.base_mut()
                            .attrs
                            .insert(field.name.clone(), String::new());
                        writes.push((Slot(e.base().id), next));
                    }
                }
            }
        }
        let mut next_fields = fields;
        next_fields[at] = new;
        doc.set_layer_fields(&id, next_fields, &[])
            .map_err(|r| refuse(r.0))?;
        let objects = writes.len() as u32;
        touched.extend(writes.iter().map(|(s, _)| *s));
        if !writes.is_empty() {
            doc.update_many(writes, STEP);
        }
        let unfit: u32 = losses.values().map(|(_, n)| *n).sum();
        let losses = losses
            .into_iter()
            .map(|((value, clear), (uids, count))| Loss {
                layer: id.clone(),
                layer_name: name.clone(),
                field: field.name.clone(),
                kind: if clear {
                    LossKind::Cleared
                } else {
                    LossKind::Kept
                },
                value,
                uids,
                count,
            })
            .collect();
        let mut note = format!(
            "“{name}” katmanının “{}” alanı {} oldu; {objects} nesnenin değeri yazıldı",
            field.name,
            kind_text(&shown)
        );
        if unfit > 0 {
            note.push_str(&format!(", {unfit} nesnenin değeri yeni kurala uymadı"));
        }
        if let Some(d) = added_domain {
            note.push_str(&format!("; projeye “{d}” değer listesi eklendi"));
        }
        note.push('.');
        Ok(Outcome {
            layer: Some(id),
            applied: true,
            objects,
            note,
            warning: None,
            losses,
        })
    }

    fn kind_text(f: &LayerField) -> String {
        kentos_commands::kind_word(f.kind)
    }

    /// A value with each of its codes recoded by `change` (a multiple field's parts one by one); none when none changes.
    fn recode(
        f: Option<&LayerField>,
        value: &str,
        change: &dyn Fn(&str) -> Option<String>,
    ) -> Option<String> {
        if f.is_some_and(LayerField::holds_many) {
            let parts: Vec<&str> = value
                .split(';')
                .map(js_trim)
                .filter(|p| !p.is_empty())
                .collect();
            let mut changed = false;
            let mut out: Vec<String> = Vec::new();
            for p in parts {
                let n = match change(p) {
                    Some(n) => {
                        changed = true;
                        n
                    }
                    None => p.to_owned(),
                };
                if !n.is_empty() && !out.contains(&n) {
                    out.push(n);
                }
            }
            changed.then(|| out.join(MULTIPLE_JOIN))
        } else {
            change(value)
        }
    }

    fn map_values(
        doc: &mut Document,
        path: &[String],
        field: &str,
        map: &[kentos_commands::ValueMap],
        touched: &mut HashSet<Slot>,
    ) -> Result<Outcome, Stop> {
        let (id, name) = match layer_of(doc, path)? {
            Ok(x) => x,
            Err(o) => return Ok(o),
        };
        let held = doc
            .layers()
            .get(&id)
            .and_then(|n| n.fields.iter().find(|f| f.name == field).cloned());
        let Some(held) = held else {
            return Ok(Outcome::missing(
                Some(id),
                format!("“{name}” katmanında “{field}” alanı yok; adım atlandı."),
            ));
        };
        let mut writes: Vec<(Slot, Entity)> = Vec::new();
        for e in doc.by_layer(&id) {
            let attrs = &e.base().attrs;
            let Some(v) = attrs.get(field) else { continue };
            let change = |code: &str| -> Option<String> {
                // A condition is more specific than none: the conditional recodes are tried first.
                let hit = |conditional: bool| {
                    map.iter().find(|m| {
                        m.from == code
                            && m.when_field.is_some() == conditional
                            && match (&m.when_field, &m.when_equals) {
                                (Some(f), Some(eq)) => attrs.get(f).is_some_and(|x| x == eq),
                                _ => true,
                            }
                    })
                };
                hit(true).or_else(|| hit(false)).map(|m| m.to.clone())
            };
            if let Some(n) = recode(Some(&held), v, &change)
                && n != *v
            {
                let mut next = e.clone();
                next.base_mut().attrs.insert(field.to_owned(), n);
                writes.push((Slot(e.base().id), next));
            }
        }
        let objects = writes.len() as u32;
        if writes.is_empty() {
            return Ok(Outcome::skipped(
                Some(id),
                format!("“{name}” katmanının “{field}” alanında değişecek değer yok."),
            ));
        }
        touched.extend(writes.iter().map(|(s, _)| *s));
        doc.update_many(writes, STEP);
        Ok(Outcome {
            layer: Some(id),
            applied: true,
            objects,
            note: format!(
                "“{name}” katmanının “{field}” alanında {objects} nesnenin değeri yeniden kodlandı."
            ),
            warning: None,
            losses: Vec::new(),
        })
    }

    fn retire_code(
        doc: &mut Document,
        domain: &str,
        code: &str,
        replace_with: Option<&str>,
        touched: &mut HashSet<Slot>,
    ) -> Result<Outcome, Stop> {
        let domains = doc.settings().domains.clone();
        let key = fold_name(domain.trim());
        let Some(d) = domains
            .iter()
            .find(|d| d.id == domain)
            .or_else(|| domains.iter().find(|d| fold_name(d.name.trim()) == key))
        else {
            return Ok(Outcome::missing(
                None,
                format!("Projede “{domain}” değer listesi yok; adım atlandı."),
            ));
        };
        let Some(values) = &d.values else {
            return Ok(Outcome::missing(
                None,
                format!("“{}” değer listesinin kodları yok; adım atlandı.", d.name),
            ));
        };
        let Some(old) = values.iter().find(|v| v.code == code) else {
            return Ok(Outcome::missing(
                None,
                format!(
                    "“{}” değer listesinde “{code}” kodu yok; adım atlandı.",
                    d.name
                ),
            ));
        };
        if let Some(r) = replace_with
            && !values.iter().any(|v| v.code == r)
        {
            return Ok(Outcome::missing(
                None,
                format!(
                    "“{}” değer listesinde yerine konacak “{r}” kodu yok; adım atlandı.",
                    d.name
                ),
            ));
        }
        // The values of the fields that link the list.
        let mut writes: Vec<(Slot, Entity)> = Vec::new();
        let mut locked: Vec<String> = Vec::new();
        if let Some(r) = replace_with {
            let linked: Vec<(String, String, LayerField)> = doc
                .layers()
                .leaves()
                .into_iter()
                .flat_map(|n| {
                    n.fields
                        .iter()
                        .filter(|f| f.domain.as_deref() == Some(d.id.as_str()))
                        .map(|f| (n.id.clone(), n.name.clone(), f.clone()))
                        .collect::<Vec<_>>()
                })
                .collect();
            for (layer, layer_name, f) in &linked {
                let change = |c: &str| (c == code).then(|| r.to_owned());
                let mut layer_writes = Vec::new();
                for e in doc.by_layer(layer) {
                    let Some(v) = e.base().attrs.get(&f.name) else {
                        continue;
                    };
                    if let Some(n) = recode(Some(f), v, &change)
                        && n != *v
                    {
                        let mut next = e.clone();
                        next.base_mut().attrs.insert(f.name.clone(), n);
                        layer_writes.push((Slot(e.base().id), next));
                    }
                }
                if layer_writes.is_empty() {
                    continue;
                }
                if doc.layers().is_locked(layer) {
                    locked.push(layer_name.clone());
                } else {
                    writes.extend(layer_writes);
                }
            }
        }
        let name = d.name.clone();
        let retired_already = old.retired;
        let mut next: Vec<FieldDomain> = domains.clone();
        for x in &mut next {
            if x.id == d.id {
                kentos_commands::retire_in(x, code);
            }
        }
        let objects = writes.len() as u32;
        touched.extend(writes.iter().map(|(s, _)| *s));
        if !writes.is_empty() {
            doc.update_many(writes, STEP);
        }
        doc.set_domains(next, STEP).map_err(|r| refuse(r.0))?;
        let applied = objects > 0 || !retired_already;
        let mut note = if retired_already && objects == 0 {
            format!("“{name}” listesinin “{code}” kodu zaten kullanımdan kalkmış.")
        } else if retired_already {
            format!("“{name}” listesinin “{code}” kodu zaten kullanımdan kalkmıştı.")
        } else {
            format!("“{name}” listesinin “{code}” kodu kullanımdan kaldırıldı.")
        };
        if let Some(r) = replace_with
            && objects > 0
        {
            note.push_str(&format!(" {objects} nesnenin değeri “{r}” oldu."));
        }
        let warning = (!locked.is_empty()).then(|| {
            (
                codes::PROFILE_LAYER_SKIPPED,
                format!(
                    "{} katmanı kilitli; değerleri “{}” koduyla kaldı.",
                    locked.join(", "),
                    code
                ),
            )
        });
        Ok(Outcome {
            layer: None,
            applied,
            objects,
            note,
            warning,
            losses: Vec::new(),
        })
    }

    /// Runs the chain's steps on `doc`.
    fn run(doc: &mut Document, c: &Chain) -> Result<(ProfilesMigrated, Vec<CommandWarning>), Stop> {
        let mut steps = Vec::new();
        let mut losses: Vec<MigrationLoss> = Vec::new();
        let mut warnings = Vec::new();
        let mut touched: HashSet<Slot> = HashSet::new();
        let mut unfit = 0u32;
        let mut samples: Vec<String> = Vec::new();
        let group = doc.begin_group(STEP);
        for (migration, index, step) in &c.steps {
            let out = match run_step(doc, &c.profile, step, &mut touched) {
                Ok(o) => o,
                Err(stop) => {
                    doc.cancel_group(group);
                    return Err(stop);
                }
            };
            if let Some((code, message)) = &out.warning {
                warnings.push(warn(
                    code,
                    message.clone(),
                    Some(format!("migrations/{migration}/steps/{index}")),
                ));
            }
            for l in out.losses {
                unfit += l.count;
                if samples.len() < 5 {
                    samples.push(format!("“{}”", l.value));
                }
                let verb = match l.kind {
                    LossKind::Kept => "olduğu gibi bırakıldı",
                    LossKind::Cleared => "boşaltıldı",
                };
                losses.push(MigrationLoss {
                    migration: *migration,
                    step: *index,
                    layer: l.layer.clone(),
                    field: l.field.clone(),
                    kind: l.kind,
                    message: format!(
                        "“{}” katmanının “{}” alanındaki “{}” değeri yeni kurala uymuyor ({} nesne); {verb}.",
                        l.layer_name, l.field, l.value, l.count
                    ),
                    value: l.value,
                    uids: l.uids,
                    count: l.count,
                });
            }
            steps.push(MigratedStep {
                migration: *migration,
                index: *index,
                kind: step.kind().to_owned(),
                layer: out.layer,
                applied: out.applied,
                objects: out.objects,
                note: out.note,
            });
        }
        doc.end_group(group);
        if unfit > 0 {
            warnings.push(warn(
                codes::VALUES_UNFIT,
                format!(
                    "{unfit} nesnenin değeri yeni kurala uymadı ({}); göçün raporunda değer değer yazılı. Formla düzenle ya da Öznitelik tablosuyla düzeltin.",
                    samples.join(", ")
                ),
                Some("losses".into()),
            ));
        }
        let changed = steps.iter().any(|s| s.applied);
        Ok((
            ProfilesMigrated {
                changed,
                from: c.from,
                to: c.to,
                steps,
                losses,
                objects: touched.len() as u32,
                revision: doc.revision().to_string(),
            },
            warnings,
        ))
    }

    pub fn validate(cx: &ExecutionContext<'_>, input: &ProfilesMigrate) -> CommandResult<()> {
        match check(cx.doc, input) {
            Ok(_) => CommandResult::Completed {
                output: (),
                warnings: Vec::new(),
            },
            Err(stop) => stop.into(),
        }
    }

    /// The preview: the same run on a reading copy; the drawing stays.
    pub fn plan(
        cx: &ExecutionContext<'_>,
        input: &ProfilesMigrate,
    ) -> CommandResult<ProfilesMigrated> {
        plan_in(cx.doc, input)
    }

    pub fn plan_in(doc: &Document, input: &ProfilesMigrate) -> CommandResult<ProfilesMigrated> {
        let c = match check(doc, input) {
            Ok(c) => c,
            Err(stop) => return stop.into(),
        };
        let mut copy = doc.reading_copy();
        match run(&mut copy, &c) {
            Ok((mut output, warnings)) => {
                output.revision = doc.revision().to_string();
                CommandResult::Completed { output, warnings }
            }
            Err(stop) => stop.into(),
        }
    }

    pub fn execute(
        cx: &mut ExecutionContext<'_>,
        input: ProfilesMigrate,
    ) -> CommandResult<ProfilesMigrated> {
        crate::call::recorded(
            cx,
            kentos_commands::CAD_PROFILES_MIGRATE,
            kentos_commands::CAD_PROFILES_MIGRATE_VERSION,
            input,
            write_execute,
        )
    }

    fn write_execute(
        cx: &mut ExecutionContext<'_>,
        input: ProfilesMigrate,
    ) -> CommandResult<ProfilesMigrated> {
        let c = match check(cx.doc, &input) {
            Ok(c) => c,
            Err(stop) => return stop.into(),
        };
        match run(cx.doc, &c) {
            Ok((output, warnings)) => CommandResult::Completed { output, warnings },
            Err(stop) => stop.into(),
        }
    }
}
