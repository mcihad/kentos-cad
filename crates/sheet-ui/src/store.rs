//! Where the desktop keeps sheets until `.kcad` carries them (design §10,
//! integration.md §6): a folder of the host's data folder with the books by
//! the project's key (the web's keys: `bulut/…`, `proje/…`, `dosya/…`,
//! `oturum/…`), this device's templates and the pictures by their SHA-256.
//! A file is written beside its place and renamed over it, so a crash never
//! leaves half a book. Nothing here is the user's settings.
//!
//! ```text
//! pafta/
//!   kitaplar/<anahtar>.json      the book of a project
//!   sablonlar/<kimlik>.json      a template of this device, or the cached
//!                                copy of one of an account's cloud library
//!   resimler/<sha256>            a picture's bytes
//! ```
//!
//! A template file is the web's `StoredTemplate` (`kentos.sheets.deviceTemplate`
//! 1): the template and, for a copy of the cloud's, its place in the
//! account's library (`DeviceCloudState`: whose, which revision, changed
//! here, the role). The core reads the template; a file it refuses is left
//! as it is and listed with its reason, never dropped.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use kentos_sheet::cloud::DeviceCloudState;
use kentos_sheet::model::SheetBook;
use kentos_sheet::template::{Template, read_template, sha256_hex};
use serde::{Deserialize, Serialize};

/// A template file's format and version (the web's `TEMPLATE_FORMAT`, `TEMPLATE_VERSION`).
pub const TEMPLATE_FORMAT: &str = "kentos.sheets.deviceTemplate";
pub const TEMPLATE_VERSION: u32 = 1;

/// A template kept on this device.
#[derive(Clone, Debug, PartialEq)]
pub struct StoredTemplate {
    pub id: String,
    /// When it was written here, milliseconds since 1970.
    pub saved: i64,
    pub template: Template,
    /// Its place in an account's cloud library; none: this device's only.
    pub cloud: Option<DeviceCloudState>,
}

/// A template file that could not be read: its file and why (the gallery lists it).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Unreadable {
    pub file: String,
    pub reason: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct TemplateFile {
    format: String,
    version: u32,
    id: String,
    saved: i64,
    /// Read by the core only.
    template: serde_json::Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    cloud: Option<DeviceCloudState>,
}

/// Now, in milliseconds since 1970.
fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX))
}

/// A template file's text read: the web's record, or (written before it) a bare template of this device.
fn read_record(text: &str) -> Result<StoredTemplate, String> {
    if let Ok(file) = serde_json::from_str::<TemplateFile>(text) {
        if file.format != TEMPLATE_FORMAT {
            return Err(format!("“{}” bir şablon kaydı değil.", file.format));
        }
        if file.version > TEMPLATE_VERSION {
            return Err(format!(
                "kayıt bu sürümden yeni (sürüm {}); programı güncelleyin.",
                file.version
            ));
        }
        let template = read_template(&file.template.to_string()).map_err(|e| e.message)?;
        return Ok(StoredTemplate {
            id: file.id,
            saved: file.saved,
            template,
            cloud: file.cloud,
        });
    }
    let template = read_template(text).map_err(|e| e.message)?;
    Ok(StoredTemplate {
        id: template.meta.id.clone(),
        saved: 0,
        template,
        cloud: None,
    })
}

/// The folder of the sheets.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Store {
    dir: PathBuf,
}

/// A file name for a key: its letters kept where they are safe, and a short hash so two keys never meet.
fn file_name(key: &str) -> String {
    let safe: String = key
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' {
                c
            } else {
                '_'
            }
        })
        .take(80)
        .collect();
    let hash = sha256_hex(key.as_bytes());
    format!("{safe}-{}", hash.get(..12).unwrap_or(&hash))
}

/// Writes beside the file and renames over it.
fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let tmp = path.with_extension("yaziliyor");
    fs::write(&tmp, bytes)?;
    fs::rename(&tmp, path)
}

impl Store {
    /// The store in `dir`, its folders made.
    pub fn open(dir: &Path) -> io::Result<Store> {
        for sub in ["kitaplar", "sablonlar", "resimler"] {
            fs::create_dir_all(dir.join(sub))?;
        }
        Ok(Store {
            dir: dir.to_owned(),
        })
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    fn book_path(&self, key: &str) -> PathBuf {
        self.dir
            .join("kitaplar")
            .join(format!("{}.json", file_name(key)))
    }

    /// The book kept under `key`; none when there is none or it does not read (a broken one is kept aside, not lost).
    pub fn load_book(&self, key: &str) -> Option<SheetBook> {
        let path = self.book_path(key);
        let text = fs::read_to_string(&path).ok()?;
        match kentos_sheet::validate::read_book(&text) {
            Ok(book) => Some(book),
            Err(_) => {
                let _ = fs::rename(&path, path.with_extension("bozuk.json"));
                None
            }
        }
    }

    pub fn save_book(&self, key: &str, book: &SheetBook) -> io::Result<()> {
        let json = serde_json::to_string_pretty(book).map_err(io::Error::other)?;
        write_atomic(&self.book_path(key), json.as_bytes())
    }

    fn template_path(&self, id: &str) -> PathBuf {
        self.dir
            .join("sablonlar")
            .join(format!("{}.json", file_name(id)))
    }

    /// The templates kept on this device, by name, and the files that do not read.
    pub fn records(&self) -> (Vec<StoredTemplate>, Vec<Unreadable>) {
        let Ok(entries) = fs::read_dir(self.dir.join("sablonlar")) else {
            return (Vec::new(), Vec::new());
        };
        let mut files: Vec<PathBuf> = entries
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x == "json"))
            .collect();
        files.sort();
        let (mut out, mut bad) = (Vec::new(), Vec::new());
        for path in files {
            let read = fs::read_to_string(&path)
                .map_err(|e| e.to_string())
                .and_then(|t| read_record(&t));
            match read {
                Ok(r) => out.push(r),
                Err(reason) => bad.push(Unreadable {
                    file: path
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_default(),
                    reason,
                }),
            }
        }
        out.sort_by(|a, b| {
            a.template
                .meta
                .name
                .cmp(&b.template.meta.name)
                .then_with(|| a.id.cmp(&b.id))
        });
        (out, bad)
    }

    /// The templates kept on this device, by name (the ones that do not read are left out).
    pub fn templates(&self) -> Vec<StoredTemplate> {
        self.records().0
    }

    /// One template kept here.
    pub fn template(&self, id: &str) -> Option<StoredTemplate> {
        let text = fs::read_to_string(self.template_path(id)).ok()?;
        read_record(&text).ok().filter(|r| r.id == id)
    }

    /// Keeps a template under `id` (its content's own id may differ: the cloud's comes later), with its place in the cloud.
    pub fn save_template(
        &self,
        id: &str,
        template: &Template,
        cloud: Option<&DeviceCloudState>,
    ) -> io::Result<()> {
        let file = TemplateFile {
            format: TEMPLATE_FORMAT.to_owned(),
            version: TEMPLATE_VERSION,
            id: id.to_owned(),
            saved: now_ms(),
            template: serde_json::to_value(template).map_err(io::Error::other)?,
            cloud: cloud.cloned(),
        };
        let json = serde_json::to_string_pretty(&file).map_err(io::Error::other)?;
        write_atomic(&self.template_path(id), json.as_bytes())
    }

    pub fn remove_template(&self, id: &str) -> io::Result<()> {
        match fs::remove_file(self.template_path(id)) {
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
            other => other,
        }
    }

    /// A picture's bytes by its SHA-256.
    pub fn asset(&self, sha256: &str) -> Option<Vec<u8>> {
        if !sha256.chars().all(|c| c.is_ascii_hexdigit()) {
            return None;
        }
        fs::read(self.dir.join("resimler").join(sha256)).ok()
    }

    /// Keeps a picture's bytes (once: the name is their hash).
    pub fn put_asset(&self, sha256: &str, bytes: &[u8]) {
        if !sha256.chars().all(|c| c.is_ascii_hexdigit()) || sha256_hex(bytes) != sha256 {
            return;
        }
        let path = self.dir.join("resimler").join(sha256);
        if !path.exists() {
            let _ = write_atomic(&path, bytes);
        }
    }
}

/// Which project a book belongs to (the web's `projectKeyOf`, app/sheet/projectKey.ts).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectKey {
    /// The store's key.
    pub id: String,
    /// Whether the key names the project again after a restart (a session's does not).
    pub lasting: bool,
    /// What the key is, in words (the tab strip's tooltip).
    pub label: String,
}

/// A cloud project, by its workspace and id.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CloudProject {
    pub tenant: String,
    pub project: String,
    pub name: String,
}

/// The key of a project: a cloud project's, else the drawing's lasting project id, else its file's name, else this session's.
pub fn project_key(
    cloud: Option<&CloudProject>,
    project_id: Option<&str>,
    file_name: Option<&str>,
    session: &str,
) -> ProjectKey {
    if let Some(c) = cloud {
        return ProjectKey {
            id: format!("bulut/{}/{}", c.tenant, c.project),
            lasting: true,
            label: format!("bulut projesi “{}”", c.name),
        };
    }
    if let Some(p) = project_id.filter(|p| !p.is_empty()) {
        return ProjectKey {
            id: format!("proje/{p}"),
            lasting: true,
            label: "çizimin proje kimliği".to_owned(),
        };
    }
    if let Some(f) = file_name.filter(|f| !f.is_empty()) {
        return ProjectKey {
            id: format!("dosya/{f}"),
            lasting: true,
            label: format!("“{f}” dosyası"),
        };
    }
    ProjectKey {
        id: format!("oturum/{session}"),
        lasting: false,
        label: "bu oturum (çizim kaydedilmedi)".to_owned(),
    }
}

/// What to do with the book when the key may have changed (the web's `keyChange`):
/// another drawing loads its own; a new drawing that got its lasting name takes
/// its book along; one that got another lasting name takes a copy along.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyChange {
    Keep,
    Move,
    Copy,
    Load,
}

pub fn key_change(
    before: Option<&ProjectKey>,
    after: &ProjectKey,
    drawing_replaced: bool,
) -> KeyChange {
    match before {
        None => KeyChange::Load,
        Some(_) if drawing_replaced => KeyChange::Load,
        Some(b) if b.id == after.id => KeyChange::Keep,
        Some(b) if b.lasting => KeyChange::Copy,
        Some(_) => KeyChange::Move,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    pub(crate) fn scratch(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("kentos-sheet-ui-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn a_book_and_a_template_are_kept_and_read_back() {
        let dir = scratch("store");
        let store = Store::open(&dir).unwrap();
        assert!(store.load_book("dosya/ada.kcad").is_none());
        let mut book = SheetBook::default();
        book.variables.push(kentos_sheet::model::Variable {
            name: "proje_no".into(),
            label: "Proje no".into(),
            kind: kentos_sheet::model::VarKind::Text,
            value: kentos_sheet::model::VarValue::Text("12".into()),
        });
        store.save_book("dosya/ada.kcad", &book).unwrap();
        assert_eq!(store.load_book("dosya/ada.kcad"), Some(book));
        // A key with odd letters is a safe name; two keys never share one.
        assert_ne!(file_name("dosya/a b"), file_name("dosya/a_b"));
        // A broken book is set aside.
        fs::write(store.book_path("bozuk"), "{").unwrap();
        assert!(store.load_book("bozuk").is_none());
        assert!(!store.book_path("bozuk").exists());

        let mut t = kentos_sheet::template::system_template("sys:genel-a4-dikey")
            .unwrap()
            .clone();
        t.meta.id = "u:benim".into();
        t.meta.name = "Benim paftam".into();
        store.save_template("u:benim", &t, None).unwrap();
        assert_eq!(
            store
                .templates()
                .iter()
                .map(|t| t.template.meta.name.as_str())
                .collect::<Vec<_>>(),
            ["Benim paftam"]
        );
        // A copy of the cloud's keeps its place in the account's library.
        let mut cloud = DeviceCloudState::to_upload("ayse");
        cloud.revision = 3;
        cloud.changed = false;
        store.save_template("c1", &t, Some(&cloud)).unwrap();
        let kept = store.template("c1").unwrap();
        assert_eq!(
            (kept.id.as_str(), kept.cloud.as_ref()),
            ("c1", Some(&cloud))
        );
        assert!(kept.saved > 0);
        store.remove_template("c1").unwrap();
        // A template written before the record (a bare template file) is read as this device's.
        let mut old = t.clone();
        old.meta.id = "u:eski".into();
        fs::write(
            store.template_path("u:eski"),
            serde_json::to_string(&old).unwrap(),
        )
        .unwrap();
        let eski = store.template("u:eski").unwrap();
        assert_eq!((eski.id.as_str(), eski.cloud), ("u:eski", None));
        let (_, bad) = store.records();
        assert!(bad.is_empty());
        // A file the core refuses is listed with its reason, and kept.
        fs::write(store.template_path("bozuk"), "{\"format\": 1}").unwrap();
        let (good, bad) = store.records();
        assert_eq!(good.len(), 2);
        assert_eq!(bad.len(), 1);
        assert!(store.template_path("bozuk").exists());
        store.remove_template("u:benim").unwrap();
        store.remove_template("u:eski").unwrap();
        store.remove_template("bozuk").unwrap();
        assert!(store.templates().is_empty());

        let bytes = b"\x89PNG not really".to_vec();
        let sha = sha256_hex(&bytes);
        store.put_asset(&sha, &bytes);
        store.put_asset("00", b"x");
        assert_eq!(store.asset(&sha), Some(bytes));
        assert_eq!(store.asset("../kitaplar"), None);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_key_is_the_web_s() {
        let cloud = CloudProject {
            tenant: "t".into(),
            project: "p".into(),
            name: "Ada".into(),
        };
        assert_eq!(
            project_key(Some(&cloud), Some("x"), None, "s").id,
            "bulut/t/p"
        );
        assert_eq!(
            project_key(None, Some("x"), Some("a.kcad"), "s").id,
            "proje/x"
        );
        assert_eq!(
            project_key(None, None, Some("a.kcad"), "s").id,
            "dosya/a.kcad"
        );
        let session = project_key(None, None, None, "s1");
        assert_eq!((session.id.as_str(), session.lasting), ("oturum/s1", false));
        let file = project_key(None, None, Some("a.kcad"), "s1");
        assert_eq!(key_change(Some(&session), &file, false), KeyChange::Move);
        assert_eq!(
            key_change(
                Some(&file),
                &project_key(None, None, Some("b.kcad"), "s1"),
                false
            ),
            KeyChange::Copy
        );
        assert_eq!(key_change(Some(&file), &file, false), KeyChange::Keep);
        assert_eq!(key_change(Some(&file), &file, true), KeyChange::Load);
    }
}
