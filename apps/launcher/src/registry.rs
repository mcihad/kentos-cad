//! The apps the launcher shows (docs/adr/0800 §2): the manifests bundled
//! with it (KentOS's modules and those still to come), then those in
//! `uygulamalar/` next to the launcher (what an installation brings), then
//! the user's own (`$XDG_DATA_HOME/kentos/uygulamalar`, what Uygulama ekle
//! writes). A later manifest with the same id takes the earlier one's place:
//! an installation or the user may point a KentOS module elsewhere. A broken
//! manifest never stops the launcher: it is left out and said, with its file.

use std::path::{Path, PathBuf};

use kentos_ui::attribute::text;

use crate::files;
use crate::manifest::{self, Folder, Manifest, Origin};

/// The bundled manifests, by file name.
const BUILTINS: [(&str, &str); 6] = [
    (
        "kentos-mimari.json",
        include_str!("../assets/apps/kentos-mimari.json"),
    ),
    (
        "kentos-harita.json",
        include_str!("../assets/apps/kentos-harita.json"),
    ),
    (
        "kentos-gis.json",
        include_str!("../assets/apps/kentos-gis.json"),
    ),
    (
        "kentos-yol.json",
        include_str!("../assets/apps/kentos-yol.json"),
    ),
    (
        "kentos-3d.json",
        include_str!("../assets/apps/kentos-3d.json"),
    ),
    (
        "kentos-afet.json",
        include_str!("../assets/apps/kentos-afet.json"),
    ),
];

/// A manifest file's size the launcher reads at most.
const LARGEST: u64 = 256 * 1024;

/// Something a manifest could not be, said with its file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Problem {
    pub file: String,
    pub reason: String,
    /// The manifest was left out (else a field fell back to its default).
    pub left_out: bool,
}

/// The folders manifests are read from.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Folders {
    pub beside: Option<PathBuf>,
    pub user: Option<PathBuf>,
}

impl Folders {
    /// `uygulamalar/` next to the running launcher and the user's own.
    pub fn system() -> Self {
        Self {
            beside: std::env::current_exe()
                .ok()
                .and_then(|exe| exe.parent().map(|d| d.join("uygulamalar"))),
            user: files::data_home().map(|d| d.join("kentos").join("uygulamalar")),
        }
    }
}

/// The apps and what was said reading them.
#[derive(Debug, Clone, Default)]
pub struct Registry {
    /// In the cards' order: `order`, then the name in the Turkish alphabet.
    pub apps: Vec<Manifest>,
    pub problems: Vec<Problem>,
}

impl Registry {
    pub fn get(&self, id: &str) -> Option<&Manifest> {
        self.apps.iter().find(|m| m.id == id)
    }

    /// Reads every folder.
    pub fn load(folders: &Folders) -> Self {
        let mut problems = Vec::new();
        let builtins = builtins(&mut problems);
        let beside = folders
            .beside
            .as_deref()
            .map(|d| read_folder(d, Folder::Beside, &mut problems))
            .unwrap_or_default();
        let user = folders
            .user
            .as_deref()
            .map(|d| read_folder(d, Folder::User, &mut problems))
            .unwrap_or_default();
        Self {
            apps: merge([builtins, beside, user]),
            problems,
        }
    }

    /// The bundled manifests alone (the snapshot).
    pub fn builtin() -> Self {
        let mut problems = Vec::new();
        let apps = merge([builtins(&mut problems), Vec::new(), Vec::new()]);
        Self { apps, problems }
    }
}

/// The bundled manifests; a broken one is said (a test keeps them whole).
fn builtins(problems: &mut Vec<Problem>) -> Vec<Manifest> {
    let mut apps = Vec::new();
    for (file, text) in BUILTINS {
        match manifest::read(text, Origin::Builtin) {
            Ok(read) => {
                problems.extend(read.warnings.into_iter().map(|reason| Problem {
                    file: file.to_owned(),
                    reason,
                    left_out: false,
                }));
                apps.push(read.manifest);
            }
            Err(reason) => problems.push(Problem {
                file: file.to_owned(),
                reason,
                left_out: true,
            }),
        }
    }
    apps
}

/// The manifests of a folder, by file name; a folder that is not there has none.
pub fn read_folder(dir: &Path, folder: Folder, problems: &mut Vec<Problem>) -> Vec<Manifest> {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Vec::new(),
        Err(e) => {
            problems.push(Problem {
                file: dir.display().to_string(),
                reason: format!("Klasör okunamadı ({e})."),
                left_out: true,
            });
            return Vec::new();
        }
    };
    let mut paths: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| {
            let name = p
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            !name.starts_with('.')
                && p.extension()
                    .is_some_and(|e| e.eq_ignore_ascii_case("json"))
                && p.is_file()
        })
        .collect();
    paths.sort();
    let mut apps: Vec<Manifest> = Vec::new();
    for path in paths {
        let file = path.display().to_string();
        let mut say = |reason: String, left_out: bool| {
            problems.push(Problem {
                file: file.clone(),
                reason,
                left_out,
            });
        };
        let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        if size > LARGEST {
            say(
                format!(
                    "Dosya {} KB; bir manifest en çok {} KB olur.",
                    size / 1024,
                    LARGEST / 1024
                ),
                true,
            );
            continue;
        }
        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(e) => {
                say(format!("Dosya okunamadı ({e})."), true);
                continue;
            }
        };
        let origin = Origin::File {
            path: path.clone(),
            folder,
        };
        match manifest::read(&text, origin) {
            Ok(read) => {
                for warning in read.warnings {
                    say(warning, false);
                }
                if let Some(first) = apps.iter().find(|m| m.id == read.manifest.id) {
                    say(
                        format!(
                            "“{}” kimliği {} dosyasında da var; bu dosya yok sayıldı.",
                            first.id,
                            first.origin.file_name().unwrap_or_default()
                        ),
                        true,
                    );
                    continue;
                }
                apps.push(read.manifest);
            }
            Err(reason) => say(reason, true),
        }
    }
    apps
}

/// The layers in order: a later manifest takes the place of the earlier one
/// with its id. The cards' order: `order`, then the name.
fn merge(layers: [Vec<Manifest>; 3]) -> Vec<Manifest> {
    let mut apps: Vec<Manifest> = Vec::new();
    for layer in layers {
        for app in layer {
            match apps.iter_mut().find(|a| a.id == app.id) {
                Some(slot) => *slot = app,
                None => apps.push(app),
            }
        }
    }
    apps.sort_by(|a, b| {
        a.order
            .cmp(&b.order)
            .then_with(|| text::compare(&a.name, &b.name))
    });
    apps
}

/// An id for a new app from its name: Turkish letters to ASCII, words joined
/// by hyphens; a number after it while `taken` says it is in use.
pub fn new_id(name: &str, taken: impl Fn(&str) -> bool) -> String {
    let folded = text::fold(name);
    let mut id = String::new();
    for c in folded.chars() {
        if c.is_ascii_alphanumeric() {
            id.push(c.to_ascii_lowercase());
        } else if !id.is_empty() && !id.ends_with('-') {
            id.push('-');
        }
    }
    let mut id: String = id.trim_end_matches('-').chars().take(48).collect();
    while id.ends_with('-') {
        id.pop();
    }
    if id.is_empty() {
        id = "uygulama".to_owned();
    }
    if !taken(&id) {
        return id;
    }
    (2..)
        .map(|n| format!("{id}-{n}"))
        .find(|candidate| !taken(candidate))
        .unwrap_or(id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::{Icon, Launch, Picture, State};
    use kentos_launch::Module;

    fn write(dir: &Path, name: &str, text: &str) {
        std::fs::create_dir_all(dir).expect("a folder");
        std::fs::write(dir.join(name), text).expect("written");
    }

    #[test]
    fn the_bundled_apps_are_whole_and_in_order() {
        let registry = Registry::builtin();
        assert!(registry.problems.is_empty(), "{:#?}", registry.problems);
        let ids: Vec<&str> = registry.apps.iter().map(|a| a.id.as_str()).collect();
        assert_eq!(
            ids,
            [
                "kentos-mimari",
                "kentos-harita",
                "kentos-gis",
                "kentos-yol",
                "kentos-3d",
                "kentos-afet"
            ]
        );
        for module in Module::ALL {
            let app = registry
                .apps
                .iter()
                .find(|a| a.launch == Some(Launch::Kentos(module)))
                .expect("each module is an app");
            assert_eq!(app.name, module.title());
            assert_eq!(app.state, State::Ready);
            assert!(app.files.contains(&"kcad".to_owned()));
        }
        let soon: Vec<&str> = registry
            .apps
            .iter()
            .filter(|a| a.state == State::Soon)
            .map(|a| a.name.as_str())
            .collect();
        assert_eq!(soon, ["KentOS YOL", "KentOS 3D", "KentOS Afet"]);
        // Each card has the picture and the glyph of its own, and a tone no other has.
        let mut tones = Vec::new();
        for app in &registry.apps {
            let key = app.id.trim_start_matches("kentos-");
            let picture = match &app.picture {
                Some(Picture::Builtin(shot)) => shot.key(),
                other => panic!("{}: {other:?}", app.id),
            };
            let glyph = match &app.icon {
                Icon::Glyph(glyph) => glyph.key(),
                other => panic!("{}: {other:?}", app.id),
            };
            let own = if key == "gis" { "cbs" } else { key };
            assert_eq!((picture, glyph), (own, own), "{}", app.id);
            assert!(!tones.contains(&app.tone), "{}: {:?}", app.id, app.tone);
            tones.push(app.tone);
        }
    }

    #[test]
    fn folders_add_apps_replace_by_id_and_say_what_is_broken() {
        let root = files::scratch("registry");
        let (beside, user) = (root.join("kurulum"), root.join("kullanici"));
        write(
            &beside,
            "kentos-gis.json",
            r#"{"format": "kentos.launcher-app/1", "id": "kentos-gis", "name": "Kurumun GIS'i",
                "color": "teal", "launch": {"kind": "program", "program": "/opt/kurum/gis"}, "order": 30}"#,
        );
        write(
            &user,
            "a-rapor.json",
            r#"{"format": "kentos.launcher-app/1", "id": "rapor", "name": "Rapor betiği",
                "color": "mor", "launch": {"kind": "program", "program": "python3", "args": ["r.py"]}}"#,
        );
        write(
            &user,
            "b-rapor.json",
            r#"{"format": "kentos.launcher-app/1", "id": "rapor", "name": "İkinci",
                "launch": {"kind": "url", "url": "https://example.org"}}"#,
        );
        write(&user, "bozuk.json", "{\"format\": ");
        write(&user, "notlar.txt", "not a manifest");
        write(&user, ".gizli.json", "{}");
        let registry = Registry::load(&Folders {
            beside: Some(beside.clone()),
            user: Some(user.clone()),
        });
        let ids: Vec<&str> = registry.apps.iter().map(|a| a.id.as_str()).collect();
        assert_eq!(
            ids,
            [
                "kentos-mimari",
                "kentos-harita",
                "kentos-gis",
                "kentos-yol",
                "kentos-3d",
                "kentos-afet",
                "rapor"
            ]
        );
        let gis = registry.get("kentos-gis").expect("gis");
        assert_eq!(
            gis.name, "Kurumun GIS'i",
            "the installation's manifest takes the place"
        );
        assert!(matches!(
            gis.origin,
            Origin::File {
                folder: Folder::Beside,
                ..
            }
        ));
        let rapor = registry.get("rapor").expect("rapor");
        assert_eq!(rapor.name, "Rapor betiği", "the first file by name wins");
        assert!(rapor.origin.is_users());
        let said: Vec<(&str, bool)> = registry
            .problems
            .iter()
            .map(|p| (p.reason.as_str(), p.left_out))
            .collect();
        assert_eq!(said.len(), 3, "{said:#?}");
        assert!(said[0].0.contains("“color” “mor”") && !said[0].1);
        assert!(said[1].0.contains("a-rapor.json dosyasında da var") && said[1].1);
        assert!(said[2].0.contains("JSON okunamadı") && said[2].1);
        assert!(registry.problems[2].file.ends_with("bozuk.json"));

        // A folder that is not there is no problem.
        let none = Registry::load(&Folders {
            beside: Some(root.join("yok")),
            user: None,
        });
        assert_eq!(none.apps.len(), 6);
        assert!(none.problems.is_empty());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_new_id_is_ascii_and_free() {
        let taken = |id: &str| ["parsel-raporu", "parsel-raporu-2"].contains(&id);
        assert_eq!(new_id("Parsel Raporu", taken), "parsel-raporu-3");
        assert_eq!(
            new_id("İmar Uygulaması (Ankara)", |_| false),
            "imar-uygulamasi-ankara"
        );
        assert_eq!(new_id("  ---  ", |_| false), "uygulama");
        assert!(manifest::valid_id(&new_id(
            &"çok uzun ad ".repeat(10),
            |_| false
        )));
    }
}
