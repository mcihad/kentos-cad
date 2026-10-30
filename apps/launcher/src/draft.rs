//! Uygulama ekle and Düzenle (docs/adr/0800 §2): the dialog's draft of an
//! app, what is wrong with it, and the manifest it becomes in the user's
//! folder. What the dialog does not show (the picture, the version, the
//! files, the environment, the order) is kept from the manifest edited.

use std::path::{Path, PathBuf};

use crate::glyphs::Glyph;
use crate::manifest::{self, Folder, Icon, Launch, Manifest, Origin, State};
use crate::registry;
use crate::resolve;
use crate::tones::Tone;

/// What the app runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Kind {
    #[default]
    Program,
    Web,
}

impl std::fmt::Display for Kind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Kind::Program => "Program",
            Kind::Web => "Web adresi",
        })
    }
}

/// The dialog's fields.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Draft {
    /// The manifest edited; none for a new app.
    pub base: Option<Manifest>,
    pub name: String,
    pub note: String,
    pub kind: Kind,
    pub program: String,
    pub args: String,
    pub url: String,
    pub tone: Tone,
    pub glyph: Glyph,
    /// The edited manifest's own icon file: kept until a glyph is picked.
    pub icon_file: Option<PathBuf>,
    /// Ekle was pressed: the fields' problems are shown from then on.
    pub tried: bool,
    /// The file could not be written: why.
    pub refusal: Option<String>,
}

/// What is wrong with the fields, each under its own.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Problems {
    pub name: Option<String>,
    pub program: Option<String>,
    pub args: Option<String>,
    pub url: Option<String>,
}

impl Problems {
    pub fn none(&self) -> bool {
        *self == Problems::default()
    }
}

impl Draft {
    /// A new app's draft: a program, the generic tile in orange.
    pub fn new() -> Self {
        Self {
            tone: Tone::Orange,
            ..Self::default()
        }
    }

    /// An app's draft to edit.
    pub fn of(app: &Manifest) -> Self {
        let (kind, program, args, url) = match &app.launch {
            Some(Launch::Program { program, args, .. }) => (
                Kind::Program,
                program.clone(),
                resolve::join_args(args),
                String::new(),
            ),
            Some(Launch::Url(url)) => (Kind::Web, String::new(), String::new(), url.clone()),
            Some(Launch::Kentos(_)) | None => {
                (Kind::Program, String::new(), String::new(), String::new())
            }
        };
        Self {
            base: Some(app.clone()),
            name: app.name.clone(),
            note: app.note.clone(),
            kind,
            program,
            args,
            url,
            tone: app.tone,
            glyph: match &app.icon {
                Icon::Glyph(glyph) => *glyph,
                Icon::File(_) => Glyph::Uygulama,
            },
            icon_file: match &app.icon {
                Icon::File(path) => Some(path.clone()),
                Icon::Glyph(_) => None,
            },
            tried: false,
            refusal: None,
        }
    }

    pub fn problems(&self) -> Problems {
        let mut problems = Problems::default();
        let name = self.name.trim();
        if name.is_empty() {
            problems.name = Some("Kartta görünecek adı yazın.".to_owned());
        } else if name.chars().count() > 60 {
            problems.name = Some("Ad en çok 60 harf olur.".to_owned());
        }
        match self.kind {
            Kind::Program => {
                if self.program.trim().is_empty() {
                    problems.program = Some(
                        "Programın yolunu seçin ya da PATH'teki adını yazın (ör. python3)."
                            .to_owned(),
                    );
                }
                if let Err(e) = resolve::split_args(&self.args) {
                    problems.args = Some(e);
                }
            }
            Kind::Web => {
                if !manifest::is_web_address(self.url.trim()) {
                    problems.url = Some("Web adresi http:// ya da https:// ile başlar.".to_owned());
                }
            }
        }
        problems
    }

    /// The manifest the draft becomes in `folder`, the user's; `taken` says
    /// whether an id is in use (a new app gets one of its own).
    pub fn manifest(&self, folder: &Path, taken: impl Fn(&str) -> bool) -> Option<Manifest> {
        if !self.problems().none() {
            return None;
        }
        let id = match &self.base {
            Some(base) => base.id.clone(),
            None => registry::new_id(&self.name, taken),
        };
        let launch = match self.kind {
            Kind::Program => {
                let (env, cwd) = match self.base.as_ref().and_then(|b| b.launch.as_ref()) {
                    Some(Launch::Program { env, cwd, .. }) => (env.clone(), cwd.clone()),
                    _ => Default::default(),
                };
                Launch::Program {
                    program: self.program.trim().to_owned(),
                    args: resolve::split_args(&self.args).ok()?,
                    env,
                    cwd,
                }
            }
            Kind::Web => Launch::Url(self.url.trim().to_owned()),
        };
        let path = match &self.base {
            Some(Manifest {
                origin:
                    Origin::File {
                        path,
                        folder: Folder::User,
                    },
                ..
            }) => path.clone(),
            _ => folder.join(format!("{id}.json")),
        };
        let base = self.base.as_ref();
        let web = self.kind == Kind::Web;
        Some(Manifest {
            id,
            name: self.name.trim().to_owned(),
            note: self.note.trim().to_owned(),
            icon: match &self.icon_file {
                Some(path) => Icon::File(path.clone()),
                None => Icon::Glyph(self.glyph),
            },
            tone: self.tone,
            picture: base.and_then(|b| b.picture.clone()),
            version: base.and_then(|b| b.version.clone()),
            state: State::Ready,
            launch: Some(launch),
            order: base.map_or(manifest::ORDER_DEFAULT, |b| b.order),
            files: if web {
                Vec::new()
            } else {
                base.map(|b| b.files.clone()).unwrap_or_default()
            },
            origin: Origin::File {
                path,
                folder: Folder::User,
            },
        })
    }
}

/// Writes an app's manifest; the sentence says why when it cannot.
pub fn write(app: &Manifest) -> Result<PathBuf, String> {
    let Origin::File { path, .. } = &app.origin else {
        return Err("Yerleşik uygulama yazılmaz.".to_owned());
    };
    crate::files::write_whole(path, &manifest::to_json(app))
        .map(|()| path.clone())
        .map_err(|e| {
            format!(
                "“{}” yazılamadı ({e}). Klasörün iznini denetleyin.",
                path.display()
            )
        })
}

/// Removes a user's app: its manifest goes, the program it runs stays.
pub fn remove(app: &Manifest) -> Result<(), String> {
    match &app.origin {
        Origin::File {
            path,
            folder: Folder::User,
        } => std::fs::remove_file(path)
            .map_err(|e| format!("“{}” silinemedi ({e}).", path.display())),
        _ => Err("Yalnız eklediğiniz uygulamalar kaldırılır.".to_owned()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::{Folders, Registry};

    #[test]
    fn a_draft_says_what_is_missing() {
        let mut draft = Draft::new();
        let problems = draft.problems();
        assert!(problems.name.is_some() && problems.program.is_some());
        draft.name = "Parsel raporu".into();
        draft.program = "python3".into();
        draft.args = "rapor.py \"{dosya}".into();
        assert!(
            draft
                .problems()
                .args
                .expect("open quote")
                .contains("Kapanmayan tırnak")
        );
        draft.args = "rapor.py {dosya}".into();
        assert!(draft.problems().none());
        draft.kind = Kind::Web;
        draft.url = "e-plan.gov.tr".into();
        assert!(draft.problems().url.is_some());
        draft.url = "https://e-plan.example.org".into();
        assert!(
            draft.problems().none(),
            "the program is not asked of a web address"
        );
    }

    #[test]
    fn an_app_is_added_edited_and_removed_in_the_users_folder() {
        let root = crate::files::scratch("draft");
        let folder = root.join("uygulamalar");
        let folders = Folders {
            beside: None,
            user: Some(folder.clone()),
        };
        let mut draft = Draft::new();
        draft.name = "Kentos Mimari".into();
        draft.note = "Kurumun betiği".into();
        draft.program = "python3".into();
        draft.args = "rapor.py \"ada parsel.csv\"".into();
        draft.glyph = Glyph::Terminal;
        let registry = Registry::load(&folders);
        let app = draft
            .manifest(&folder, |id| registry.get(id).is_some())
            .expect("a manifest");
        assert_eq!(app.id, "kentos-mimari-2", "the bundled app keeps its id");
        write(&app).expect("written");

        let registry = Registry::load(&folders);
        assert!(registry.problems.is_empty(), "{:?}", registry.problems);
        let added = registry.get("kentos-mimari-2").expect("added").clone();
        assert_eq!(added, app, "what is written reads back");
        let Some(Launch::Program { args, .. }) = &added.launch else {
            panic!("a program");
        };
        assert_eq!(args, &["rapor.py", "ada parsel.csv"]);

        let mut edit = Draft::of(&added);
        assert_eq!(edit.args, "rapor.py \"ada parsel.csv\"");
        edit.kind = Kind::Web;
        edit.url = "https://kentos.app".into();
        let edited = edit.manifest(&folder, |_| true).expect("a manifest");
        assert_eq!(edited.id, added.id, "an edit keeps its id and file");
        assert_eq!(edited.origin, added.origin);
        write(&edited).expect("written");
        let registry = Registry::load(&folders);
        assert_eq!(
            registry.get(&added.id).and_then(|a| a.launch.clone()),
            Some(Launch::Url("https://kentos.app".into()))
        );

        // An icon file of its own stays until a glyph is picked.
        let own = Manifest {
            icon: Icon::File(folder.join("simge.svg")),
            ..edited.clone()
        };
        let mut edit = Draft::of(&own);
        edit.note = "Yeni not".into();
        let kept = edit.manifest(&folder, |_| true).expect("a manifest");
        assert_eq!(kept.icon, own.icon, "the file icon is kept");
        edit.glyph = Glyph::Web;
        edit.icon_file = None;
        let picked = edit.manifest(&folder, |_| true).expect("a manifest");
        assert_eq!(picked.icon, Icon::Glyph(Glyph::Web));

        remove(&edited).expect("removed");
        assert!(Registry::load(&folders).get(&added.id).is_none());
        let bundled = Registry::builtin()
            .get("kentos-gis")
            .cloned()
            .expect("bundled");
        assert!(remove(&bundled).is_err(), "a bundled app is not removed");
        assert!(write(&bundled).is_err());
        let _ = std::fs::remove_dir_all(&root);
    }
}
