//! An app of the launcher is a manifest (docs/adr/0800 §2, docs/launcher/README.md):
//! a small JSON file, format `kentos.launcher-app/1`, that says what the
//! card shows and how the app starts. The program may be written in any
//! language: the launcher starts it as a process of its own, or opens a web
//! address in the system's browser. KentOS's own modules are manifests too,
//! bundled with the launcher (`assets/apps`).
//!
//! What would leave the app unstartable (the format, the id, the name, the
//! state, how it starts) refuses the manifest with a sentence saying which
//! field and what to write; what is only looks (the icon, the colour, the
//! picture, the version, the order, the files) falls back to its default
//! and is said as a warning. Keys the format does not know are left alone.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use kentos_launch::Module;
use serde::Serialize;
use serde_json::{Map, Value};

use crate::glyphs::Glyph;
use crate::pictures::Shot;
use crate::tones::Tone;

/// The format and its version, the first key of every manifest.
pub const FORMAT: &str = "kentos.launcher-app/1";

/// The longest name and version a card shows whole.
const NAME_MAX: usize = 60;
const VERSION_MAX: usize = 32;
/// The order of an app that does not give one: after KentOS's own.
pub const ORDER_DEFAULT: i64 = 100;

/// An app.
#[derive(Debug, Clone, PartialEq)]
pub struct Manifest {
    /// Lowercase ASCII letters, digits and hyphens; one app, one id.
    pub id: String,
    pub name: String,
    /// One line on what it is for; may be empty.
    pub note: String,
    pub icon: Icon,
    pub tone: Tone,
    pub picture: Option<Picture>,
    /// What the card writes as its version; KentOS's own are the launcher's.
    pub version: Option<String>,
    pub state: State,
    /// How it starts; none for an app that is not out yet.
    pub launch: Option<Launch>,
    /// Cards are in this order, then by name.
    pub order: i64,
    /// The extensions (lowercase, without the dot) of the files it opens.
    pub files: Vec<String>,
    pub origin: Origin,
}

/// The tile's mark.
#[derive(Debug, Clone, PartialEq)]
pub enum Icon {
    Glyph(Glyph),
    /// An SVG or PNG file of the app's own: drawn as it is, in place of the tile.
    File(PathBuf),
}

/// The card's picture.
#[derive(Debug, Clone, PartialEq)]
pub enum Picture {
    Builtin(Shot),
    /// A PNG file of the app's own.
    File(PathBuf),
}

/// Whether the app can be started.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Ready,
    /// Announced, not out yet: its card says so and starts nothing.
    Soon,
}

/// How an app starts.
#[derive(Debug, Clone, PartialEq)]
pub enum Launch {
    /// A KentOS module (docs/adr/0801, 0802): its own program.
    Kentos(Module),
    /// Any program: a path (relative to the manifest's folder) or a name on `PATH`.
    Program {
        program: String,
        /// `{dosya}` stands for a file it is asked to open.
        args: Vec<String>,
        env: BTreeMap<String, String>,
        /// Relative to the manifest's folder; the manifest's folder when none.
        cwd: Option<PathBuf>,
    },
    /// A web address, opened in the system's browser.
    Url(String),
}

/// Where a manifest came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Origin {
    /// Bundled with the launcher.
    Builtin,
    /// A file in one of the app folders.
    File { path: PathBuf, folder: Folder },
}

/// The folders manifests are read from, in the order they are read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Folder {
    /// `uygulamalar/` next to the launcher: what an installation brings.
    Beside,
    /// The user's own (`~/.local/share/kentos/uygulamalar`): what Uygulama ekle writes.
    User,
}

impl Origin {
    /// The folder relative paths of the manifest are read from.
    pub fn base(&self) -> Option<&Path> {
        match self {
            Origin::Builtin => None,
            Origin::File { path, .. } => path.parent(),
        }
    }

    /// The user's own app: Düzenle and Kaldır are for these alone.
    pub fn is_users(&self) -> bool {
        matches!(
            self,
            Origin::File {
                folder: Folder::User,
                ..
            }
        )
    }

    /// The manifest file's name, for a sentence; none for a bundled one.
    pub fn file_name(&self) -> Option<String> {
        match self {
            Origin::Builtin => None,
            Origin::File { path, .. } => Some(path.file_name().map_or_else(
                || path.display().to_string(),
                |n| n.to_string_lossy().into_owned(),
            )),
        }
    }
}

/// A manifest read, with what was set aside of it.
#[derive(Debug, Clone, PartialEq)]
pub struct Read {
    pub manifest: Manifest,
    /// Fields that fell back to their defaults, each a sentence.
    pub warnings: Vec<String>,
}

/// Reads a manifest. A refusal is a sentence: which field, what is wrong, what to write.
pub fn read(text: &str, origin: Origin) -> Result<Read, String> {
    let value: Value = serde_json::from_str(text).map_err(|e| format!("JSON okunamadı ({e})."))?;
    let Value::Object(object) = value else {
        return Err("Bir JSON nesnesi olmalı: { \"format\": … } ile başlar.".to_owned());
    };
    let mut warnings = Vec::new();
    let base = origin.base().map(Path::to_path_buf);

    match object.get("format") {
        None => {
            return Err(format!(
                "“format” yok; ilk alan \"format\": \"{FORMAT}\" olmalı."
            ));
        }
        Some(Value::String(format)) if format == FORMAT => {}
        Some(other) => {
            return Err(format!(
                "“format” {}; bu sürüm yalnız {FORMAT} okur.",
                shown(other)
            ));
        }
    }

    let id = match object.get("id") {
        Some(Value::String(id)) if valid_id(id) => id.clone(),
        Some(other) => {
            return Err(format!(
                "“id” {}; küçük harf, rakam ve tireden oluşmalı (ör. rapor-betigi).",
                shown(other)
            ));
        }
        None => return Err("“id” yok; uygulamaya bir kimlik verin (ör. rapor-betigi).".to_owned()),
    };

    let state = match object.get("state") {
        None => State::Ready,
        Some(Value::String(s)) if s == "ready" => State::Ready,
        Some(Value::String(s)) if s == "soon" => State::Soon,
        Some(other) => {
            return Err(format!("“state” {}; ready ya da soon olur.", shown(other)));
        }
    };

    let launch = match object.get("launch") {
        None if state == State::Soon => None,
        None => {
            return Err(
                "“launch” yok: uygulamanın nasıl başlayacağını yazın (kind: program, url ya da kentos)."
                    .to_owned(),
            );
        }
        Some(Value::Object(launch)) => Some(read_launch(launch)?),
        Some(other) => {
            return Err(format!("“launch” {}; bir nesne olmalı.", shown(other)));
        }
    };
    let module = match &launch {
        Some(Launch::Kentos(module)) => Some(*module),
        _ => None,
    };

    let name = match object.get("name") {
        Some(Value::String(name)) if !name.trim().is_empty() => {
            let name = name.trim().to_owned();
            if name.chars().count() > NAME_MAX {
                return Err(format!("“name” en çok {NAME_MAX} harf olur."));
            }
            name
        }
        None if module.is_some() => module.map(Module::title).unwrap_or_default().to_owned(),
        Some(Value::String(_)) | None => {
            return Err("“name” yok ya da boş; kartta görünecek adı yazın.".to_owned());
        }
        Some(other) => return Err(format!("“name” {}; bir metin olmalı.", shown(other))),
    };

    let note = match object.get("note") {
        Some(Value::String(note)) => note.trim().to_owned(),
        None => module.map(Module::note).unwrap_or_default().to_owned(),
        Some(other) => {
            warnings.push(format!(
                "“note” {}; bir metin olmalı, yok sayıldı.",
                shown(other)
            ));
            String::new()
        }
    };

    let icon = match object.get("icon") {
        None => Icon::Glyph(Glyph::default()),
        Some(Value::String(word)) => match icon_of(word, base.as_deref()) {
            Ok(icon) => icon,
            Err(warning) => {
                warnings.push(warning);
                Icon::Glyph(Glyph::default())
            }
        },
        Some(other) => {
            warnings.push(format!(
                "“icon” {}; bir simge adı ya da dosya yolu olmalı, uygulama simgesi kullanıldı.",
                shown(other)
            ));
            Icon::Glyph(Glyph::default())
        }
    };

    let tone = match object.get("color") {
        None => Tone::default(),
        Some(Value::String(word)) => Tone::parse(word).unwrap_or_else(|| {
            warnings.push(format!(
                "“color” “{word}” bilinmiyor; bilinenler {}. Gri kullanıldı.",
                Tone::known()
            ));
            Tone::default()
        }),
        Some(other) => {
            warnings.push(format!(
                "“color” {}; bir renk adı olmalı, gri kullanıldı.",
                shown(other)
            ));
            Tone::default()
        }
    };

    let picture = match object.get("picture") {
        None | Some(Value::Null) => None,
        Some(Value::String(word)) => match picture_of(word, base.as_deref()) {
            Ok(picture) => Some(picture),
            Err(warning) => {
                warnings.push(warning);
                None
            }
        },
        Some(other) => {
            warnings.push(format!("“picture” {}; yok sayıldı.", shown(other)));
            None
        }
    };

    let version = match object.get("version") {
        None | Some(Value::Null) => None,
        Some(Value::String(v))
            if !v.trim().is_empty() && v.trim().chars().count() <= VERSION_MAX =>
        {
            Some(v.trim().to_owned())
        }
        Some(other) => {
            warnings.push(format!(
                "“version” {}; en çok {VERSION_MAX} harflik bir metin olmalı, yok sayıldı.",
                shown(other)
            ));
            None
        }
    };

    // KentOS's own programs are built with the launcher, of one version.
    let version = version.or_else(|| module.map(|_| env!("CARGO_PKG_VERSION").to_owned()));

    let order = match object.get("order") {
        None => ORDER_DEFAULT,
        Some(Value::Number(n)) if n.as_i64().is_some() => n.as_i64().unwrap_or(ORDER_DEFAULT),
        Some(other) => {
            warnings.push(format!(
                "“order” {}; bir tam sayı olmalı, yok sayıldı.",
                shown(other)
            ));
            ORDER_DEFAULT
        }
    };

    let mut files = match object.get("files") {
        None => Vec::new(),
        Some(Value::Array(items)) => {
            let mut files = Vec::new();
            for item in items {
                match item.as_str().and_then(extension) {
                    Some(ext) if !files.contains(&ext) => files.push(ext),
                    Some(_) => {}
                    None => warnings.push(format!(
                        "“files” içindeki {} bir uzantı değil (ör. \"csv\"); yok sayıldı.",
                        shown(item)
                    )),
                }
            }
            files
        }
        Some(other) => {
            warnings.push(format!(
                "“files” {}; uzantılardan oluşan bir dizi olmalı (ör. [\"csv\"]), yok sayıldı.",
                shown(other)
            ));
            Vec::new()
        }
    };
    if matches!(launch, Some(Launch::Url(_))) && !files.is_empty() {
        warnings.push("Web adresi dosya açmaz; “files” yok sayıldı.".to_owned());
        files.clear();
    }

    Ok(Read {
        manifest: Manifest {
            id,
            name,
            note,
            icon,
            tone,
            picture,
            version,
            state,
            launch,
            order,
            files,
            origin,
        },
        warnings,
    })
}

fn read_launch(launch: &Map<String, Value>) -> Result<Launch, String> {
    let kind = match launch.get("kind") {
        Some(Value::String(kind)) => kind.as_str(),
        Some(other) => {
            return Err(format!(
                "“launch.kind” {}; program, url ya da kentos olur.",
                shown(other)
            ));
        }
        None => return Err("“launch.kind” yok; program, url ya da kentos yazın.".to_owned()),
    };
    match kind {
        "program" => {
            let program = match launch.get("program") {
                Some(Value::String(p)) if !p.trim().is_empty() => p.trim().to_owned(),
                _ => {
                    return Err(
                        "“launch.program” yok: çalışacak programın yolu ya da adı (ör. python3)."
                            .to_owned(),
                    );
                }
            };
            let args = match launch.get("args") {
                None => Vec::new(),
                Some(Value::Array(items)) if items.iter().all(Value::is_string) => items
                    .iter()
                    .filter_map(|v| v.as_str().map(str::to_owned))
                    .collect(),
                Some(_) => {
                    return Err(
                        "“launch.args” metinlerden oluşan bir dizi olmalı (ör. [\"rapor.py\"])."
                            .to_owned(),
                    );
                }
            };
            let env = match launch.get("env") {
                None => BTreeMap::new(),
                Some(Value::Object(env)) if env.values().all(Value::is_string) => env
                    .iter()
                    .filter_map(|(k, v)| Some((k.clone(), v.as_str()?.to_owned())))
                    .collect(),
                Some(_) => {
                    return Err(
                        "“launch.env” adları ve metin değerleri olan bir nesne olmalı (ör. {\"LANG\": \"tr_TR.UTF-8\"})."
                            .to_owned(),
                    );
                }
            };
            let cwd = match launch.get("cwd") {
                None | Some(Value::Null) => None,
                Some(Value::String(cwd)) if !cwd.trim().is_empty() => {
                    Some(PathBuf::from(cwd.trim()))
                }
                Some(_) => return Err("“launch.cwd” bir klasör yolu olmalı.".to_owned()),
            };
            Ok(Launch::Program {
                program,
                args,
                env,
                cwd,
            })
        }
        "url" => match launch.get("url") {
            Some(Value::String(url)) if is_web_address(url.trim()) => {
                Ok(Launch::Url(url.trim().to_owned()))
            }
            Some(other) => Err(format!(
                "“launch.url” {}; http:// ya da https:// ile başlamalı.",
                shown(other)
            )),
            None => Err("“launch.url” yok; açılacak web adresini yazın.".to_owned()),
        },
        "kentos" => match launch.get("module") {
            Some(Value::String(id)) => Module::from_id(id).map(Launch::Kentos).ok_or_else(|| {
                format!(
                    "“launch.module” “{id}” bir KentOS modülü değil; bilinenler {}.",
                    Module::ALL.map(Module::id).join(", ")
                )
            }),
            _ => Err(format!(
                "“launch.module” yok; KentOS modülünü yazın: {}.",
                Module::ALL.map(Module::id).join(", ")
            )),
        },
        other => Err(format!(
            "“launch.kind” “{other}”; program, url ya da kentos olur."
        )),
    }
}

/// An id: 1–64 lowercase ASCII letters, digits and hyphens, not starting or ending with one.
pub fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        && !id.starts_with('-')
        && !id.ends_with('-')
}

/// A web address the browser is given: http or https, something after.
pub fn is_web_address(url: &str) -> bool {
    let rest = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"));
    rest.is_some_and(|r| !r.is_empty() && !r.chars().any(char::is_whitespace))
}

/// A file extension as a manifest lists it: without the dot, lowercase, letters and digits.
pub fn extension(text: &str) -> Option<String> {
    let ext = text.trim().trim_start_matches('.').to_lowercase();
    (!ext.is_empty() && ext.len() <= 16 && ext.chars().all(|c| c.is_ascii_alphanumeric()))
        .then_some(ext)
}

fn icon_of(word: &str, base: Option<&Path>) -> Result<Icon, String> {
    if let Some(glyph) = Glyph::parse(word) {
        return Ok(Icon::Glyph(glyph));
    }
    if looks_like_path(word) {
        let ext = Path::new(word)
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase());
        return match (ext.as_deref(), base) {
            (Some("svg" | "png"), Some(base)) => Ok(Icon::File(base.join(word))),
            (Some("svg" | "png"), None) => Err(format!(
                "“icon” “{word}”: yerleşik uygulama dosya simgesi kullanmaz."
            )),
            _ => Err(format!(
                "“icon” “{word}” bir SVG ya da PNG dosyası değil; uygulama simgesi kullanıldı."
            )),
        };
    }
    Err(format!(
        "“icon” “{word}” bilinmiyor; bilinenler {} ya da bir SVG/PNG dosyası. Uygulama simgesi kullanıldı.",
        Glyph::known()
    ))
}

fn picture_of(word: &str, base: Option<&Path>) -> Result<Picture, String> {
    if let Some(shot) = Shot::parse(word) {
        return Ok(Picture::Builtin(shot));
    }
    if looks_like_path(word) {
        let png = Path::new(word)
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("png"));
        return match (png, base) {
            (true, Some(base)) => Ok(Picture::File(base.join(word))),
            (true, None) => Err(format!(
                "“picture” “{word}”: yerleşik uygulama dosya resmi kullanmaz."
            )),
            (false, _) => Err(format!(
                "“picture” “{word}” bir PNG dosyası değil; resim yok sayıldı."
            )),
        };
    }
    Err(format!(
        "“picture” “{word}” bilinmiyor; bilinenler {} ya da bir PNG dosyası. Resim yok sayıldı.",
        Shot::known()
    ))
}

fn looks_like_path(word: &str) -> bool {
    word.contains('/') || word.contains('\\') || word.contains('.')
}

/// A value as a sentence quotes it: a text in quotes, anything else as JSON.
fn shown(value: &Value) -> String {
    match value {
        Value::String(s) => format!("“{s}”"),
        Value::Null => "boş".to_owned(),
        other => {
            let text = other.to_string();
            if text.chars().count() > 40 {
                format!("{}…", text.chars().take(40).collect::<String>())
            } else {
                text
            }
        }
    }
}

/// The manifest an app added in the launcher is written as (Uygulama ekle):
/// the format's keys in its order.
pub fn to_json(manifest: &Manifest) -> String {
    #[derive(Serialize)]
    struct Out<'a> {
        format: &'a str,
        id: &'a str,
        name: &'a str,
        #[serde(skip_serializing_if = "str::is_empty")]
        note: &'a str,
        icon: String,
        color: &'a str,
        #[serde(skip_serializing_if = "Option::is_none")]
        picture: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        version: Option<&'a str>,
        state: &'a str,
        #[serde(skip_serializing_if = "Option::is_none")]
        launch: Option<LaunchOut<'a>>,
        order: i64,
        #[serde(skip_serializing_if = "<[String]>::is_empty")]
        files: &'a [String],
    }
    #[derive(Serialize)]
    #[serde(tag = "kind", rename_all = "lowercase")]
    enum LaunchOut<'a> {
        Kentos {
            module: &'a str,
        },
        Program {
            program: &'a str,
            #[serde(skip_serializing_if = "<[String]>::is_empty")]
            args: &'a [String],
            #[serde(skip_serializing_if = "BTreeMap::is_empty")]
            env: &'a BTreeMap<String, String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            cwd: Option<String>,
        },
        Url {
            url: &'a str,
        },
    }
    let base = manifest.origin.base();
    let relative = |path: &Path| -> String {
        base.and_then(|b| path.strip_prefix(b).ok())
            .unwrap_or(path)
            .to_string_lossy()
            .into_owned()
    };
    let out = Out {
        format: FORMAT,
        id: &manifest.id,
        name: &manifest.name,
        note: &manifest.note,
        icon: match &manifest.icon {
            Icon::Glyph(glyph) => glyph.key().to_owned(),
            Icon::File(path) => relative(path),
        },
        color: manifest.tone.key(),
        picture: manifest.picture.as_ref().map(|picture| match picture {
            Picture::Builtin(shot) => shot.key().to_owned(),
            Picture::File(path) => relative(path),
        }),
        version: manifest.version.as_deref(),
        state: match manifest.state {
            State::Ready => "ready",
            State::Soon => "soon",
        },
        launch: manifest.launch.as_ref().map(|launch| match launch {
            Launch::Kentos(module) => LaunchOut::Kentos {
                module: module.id(),
            },
            Launch::Program {
                program,
                args,
                env,
                cwd,
            } => LaunchOut::Program {
                program,
                args,
                env,
                cwd: cwd.as_deref().map(|c| c.to_string_lossy().into_owned()),
            },
            Launch::Url(url) => LaunchOut::Url { url },
        }),
        order: manifest.order,
        files: &manifest.files,
    };
    let mut text = serde_json::to_string_pretty(&out).unwrap_or_default();
    text.push('\n');
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    fn user(name: &str) -> Origin {
        Origin::File {
            path: PathBuf::from("/ev/uygulamalar").join(name),
            folder: Folder::User,
        }
    }

    fn refused(text: &str) -> String {
        read(text, user("x.json")).expect_err("refused")
    }

    #[test]
    fn a_python_script_reads_whole() {
        let read = read(
            r#"{
              "format": "kentos.launcher-app/1",
              "id": "parsel-raporu",
              "name": "Parsel raporu",
              "note": "Python ile ada ve parsel raporu",
              "icon": "terminal",
              "color": "orange",
              "picture": "resimler/rapor.png",
              "version": "1.2",
              "launch": {"kind": "program", "program": "python3", "args": ["rapor.py", "{dosya}"],
                         "env": {"LANG": "tr_TR.UTF-8"}, "cwd": "betikler"},
              "order": 40,
              "files": ["CSV", ".xlsx", "csv"],
              "bilinmeyen": true
            }"#,
            user("parsel-raporu.json"),
        )
        .expect("reads");
        assert!(read.warnings.is_empty(), "{:?}", read.warnings);
        let m = read.manifest;
        assert_eq!(m.id, "parsel-raporu");
        assert_eq!(m.icon, Icon::Glyph(Glyph::Terminal));
        assert_eq!(m.tone, Tone::Orange);
        assert_eq!(
            m.picture,
            Some(Picture::File(PathBuf::from(
                "/ev/uygulamalar/resimler/rapor.png"
            )))
        );
        assert_eq!(m.files, ["csv", "xlsx"], "lowercase, without the dot, once");
        assert_eq!(m.order, 40);
        let Some(Launch::Program {
            program,
            args,
            env,
            cwd,
        }) = m.launch
        else {
            panic!("a program");
        };
        assert_eq!((program.as_str(), args.len()), ("python3", 2));
        assert_eq!(env.get("LANG").map(String::as_str), Some("tr_TR.UTF-8"));
        assert_eq!(cwd, Some(PathBuf::from("betikler")));
    }

    #[test]
    fn a_kentos_module_takes_its_name_and_note_from_the_module() {
        let m = read(
            r#"{"format": "kentos.launcher-app/1", "id": "kentos-gis",
                "launch": {"kind": "kentos", "module": "cbs"}}"#,
            Origin::Builtin,
        )
        .expect("reads")
        .manifest;
        assert_eq!(m.name, "KentOS GIS");
        assert_eq!(m.note, Module::Gis.note());
        assert_eq!(
            m.version.as_deref(),
            Some(env!("CARGO_PKG_VERSION")),
            "the launcher's"
        );
        assert_eq!(m.launch, Some(Launch::Kentos(Module::Gis)));
        assert_eq!(m.order, ORDER_DEFAULT);
    }

    #[test]
    fn a_web_address_and_a_soon_app() {
        let web = read(
            r#"{"format": "kentos.launcher-app/1", "id": "e-plan", "name": "e-Plan",
                "launch": {"kind": "url", "url": "https://e-plan.example.org/giris"},
                "files": ["pdf"]}"#,
            user("e-plan.json"),
        )
        .expect("reads");
        assert_eq!(
            web.manifest.launch,
            Some(Launch::Url("https://e-plan.example.org/giris".into()))
        );
        assert!(web.manifest.files.is_empty());
        assert_eq!(web.warnings.len(), 1, "files left out, said");

        let soon = read(
            r#"{"format": "kentos.launcher-app/1", "id": "kentos-yol", "name": "KentOS YOL", "state": "soon"}"#,
            Origin::Builtin,
        )
        .expect("reads")
        .manifest;
        assert_eq!((soon.state, soon.launch), (State::Soon, None));
    }

    #[test]
    fn what_cannot_start_is_refused_with_its_field() {
        let cases = [
            ("{bozuk", "JSON okunamadı"),
            ("[1, 2]", "JSON nesnesi"),
            (r#"{"id": "a", "name": "A"}"#, "“format” yok"),
            (
                r#"{"format": "kentos.launcher-app/2", "id": "a", "name": "A"}"#,
                "yalnız kentos.launcher-app/1",
            ),
            (
                r#"{"format": "kentos.launcher-app/1", "name": "A"}"#,
                "“id” yok",
            ),
            (
                r#"{"format": "kentos.launcher-app/1", "id": "Rapor Betiği", "name": "A"}"#,
                "“id” “Rapor Betiği”",
            ),
            (
                r#"{"format": "kentos.launcher-app/1", "id": "-a", "name": "A", "state": "soon"}"#,
                "“id”",
            ),
            (
                r#"{"format": "kentos.launcher-app/1", "id": "a", "name": "  ", "state": "soon"}"#,
                "“name” yok ya da boş",
            ),
            (
                r#"{"format": "kentos.launcher-app/1", "id": "a", "name": "A"}"#,
                "“launch” yok",
            ),
            (
                r#"{"format": "kentos.launcher-app/1", "id": "a", "name": "A", "state": "beta"}"#,
                "ready ya da soon",
            ),
            (
                r#"{"format": "kentos.launcher-app/1", "id": "a", "name": "A", "launch": {"kind": "exe"}}"#,
                "“launch.kind” “exe”",
            ),
            (
                r#"{"format": "kentos.launcher-app/1", "id": "a", "name": "A", "launch": {"kind": "program"}}"#,
                "“launch.program” yok",
            ),
            (
                r#"{"format": "kentos.launcher-app/1", "id": "a", "name": "A", "launch": {"kind": "program", "program": "x", "args": "-v"}}"#,
                "“launch.args”",
            ),
            (
                r#"{"format": "kentos.launcher-app/1", "id": "a", "name": "A", "launch": {"kind": "program", "program": "x", "env": {"A": 1}}}"#,
                "“launch.env”",
            ),
            (
                r#"{"format": "kentos.launcher-app/1", "id": "a", "name": "A", "launch": {"kind": "url", "url": "ftp://x"}}"#,
                "http:// ya da https://",
            ),
            (
                r#"{"format": "kentos.launcher-app/1", "id": "a", "launch": {"kind": "kentos", "module": "yol"}}"#,
                "“yol” bir KentOS modülü değil",
            ),
        ];
        for (text, want) in cases {
            let reason = refused(text);
            assert!(reason.contains(want), "{text}: {reason}");
        }
    }

    #[test]
    fn looks_fall_back_and_are_said() {
        let read = read(
            r#"{"format": "kentos.launcher-app/1", "id": "a", "name": "A",
                "icon": "uzay-gemisi", "color": "mavi", "picture": "ekran.jpg",
                "version": 3, "order": "ilk", "files": ["c s v"],
                "launch": {"kind": "program", "program": "a"}}"#,
            user("a.json"),
        )
        .expect("reads anyway");
        let m = &read.manifest;
        assert_eq!(m.icon, Icon::Glyph(Glyph::Uygulama));
        assert_eq!(m.tone, Tone::Gray);
        assert_eq!(m.picture, None);
        assert_eq!(m.version, None);
        assert_eq!(m.order, ORDER_DEFAULT);
        assert!(m.files.is_empty());
        assert_eq!(read.warnings.len(), 6, "{:#?}", read.warnings);
        assert!(read.warnings[1].contains("“color” “mavi” bilinmiyor; bilinenler blue"));
        // A bundled manifest has no folder for a file of its own.
        let builtin = super::read(
            r#"{"format": "kentos.launcher-app/1", "id": "a", "name": "A", "icon": "a.svg", "state": "soon"}"#,
            Origin::Builtin,
        )
        .expect("reads");
        assert_eq!(builtin.manifest.icon, Icon::Glyph(Glyph::Uygulama));
        assert_eq!(builtin.warnings.len(), 1);
    }

    #[test]
    fn what_is_written_reads_back_the_same() {
        let read = read(
            r#"{"format": "kentos.launcher-app/1", "id": "parsel-raporu", "name": "Parsel raporu",
                "note": "Python", "icon": "simge.svg", "color": "teal", "picture": "mimari",
                "launch": {"kind": "program", "program": "/usr/bin/python3", "args": ["r.py"]},
                "order": 7, "files": ["csv"]}"#,
            user("parsel-raporu.json"),
        )
        .expect("reads");
        let text = to_json(&read.manifest);
        assert!(
            text.starts_with("{\n  \"format\": \"kentos.launcher-app/1\",\n  \"id\""),
            "{text}"
        );
        assert!(
            text.contains("\"icon\": \"simge.svg\""),
            "relative again: {text}"
        );
        let again = super::read(&text, user("parsel-raporu.json")).expect("reads back");
        assert_eq!(again.manifest, read.manifest);
        assert!(again.warnings.is_empty());
    }

    /// The documentation's examples (docs/launcher/ornekler) read whole.
    #[test]
    fn the_documented_examples_read_without_a_word() {
        let examples = [
            (
                "parsel-raporu.json",
                include_str!("../../../docs/launcher/ornekler/parsel-raporu.json"),
            ),
            (
                "kot-hesabi.json",
                include_str!("../../../docs/launcher/ornekler/kot-hesabi.json"),
            ),
            (
                "kurum-portali.json",
                include_str!("../../../docs/launcher/ornekler/kurum-portali.json"),
            ),
            (
                "e-plan.json",
                include_str!("../../../docs/launcher/ornekler/e-plan.json"),
            ),
            (
                "kentos-gis-kurum.json",
                include_str!("../../../docs/launcher/ornekler/kentos-gis-kurum.json"),
            ),
        ];
        let mut kinds = Vec::new();
        for (name, text) in examples {
            let read = read(text, user(name)).unwrap_or_else(|e| panic!("{name}: {e}"));
            assert!(read.warnings.is_empty(), "{name}: {:?}", read.warnings);
            kinds.push(match read.manifest.launch {
                Some(Launch::Program { program, .. }) => program,
                Some(Launch::Url(_)) => "url".to_owned(),
                Some(Launch::Kentos(m)) => m.id().to_owned(),
                None => "soon".to_owned(),
            });
        }
        assert_eq!(
            kinds,
            [
                "python3",
                "java",
                "/opt/KurumPortali/kurum-portali",
                "url",
                "cbs"
            ]
        );
    }

    #[test]
    fn ids_extensions_and_addresses() {
        assert!(valid_id("kentos-gis") && valid_id("a1"));
        for id in ["", "A", "a b", "a_b", "a-", "ğ", &"a".repeat(65)] {
            assert!(!valid_id(id), "{id}");
        }
        assert_eq!(extension(".KCAD").as_deref(), Some("kcad"));
        assert_eq!(extension("tar.gz"), None);
        assert!(is_web_address("https://kentos.app"));
        assert!(!is_web_address("https://") && !is_web_address("kentos.app"));
        assert!(!is_web_address("http://a b"));
    }
}
