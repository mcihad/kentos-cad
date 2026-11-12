//! The forms designer's files on this device (docs/adr/0441 §1, §5): the section library (a tab, a group
//! or an item kept by name, each save its next version, with the fields it was bound to so another
//! layer's form takes it by key or name) and the drafts kept while a form is edited, so a closed
//! window or a crash does not lose unsaved work. Under `$XDG_DATA_HOME/kentos-cad/formlar`; nothing
//! of it is the project's.

use std::path::{Path, PathBuf};

use kentos_commands::{FormBody, FormItem, LayerField};
use serde::{Deserialize, Serialize};

/// The library's sections at most; the oldest version of a name goes first past it.
pub const MOST_SECTIONS: usize = 200;

/// A kept section: its name, its version (1, 2, … each save of the name), when, the item and the
/// fields of the layer it came from.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Section {
    pub name: String,
    pub version: u32,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub saved: String,
    pub item: FormItem,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<LayerField>,
}

#[derive(Serialize, Deserialize)]
struct Library {
    format: String,
    version: u32,
    sections: Vec<Section>,
}

const FORMAT: &str = "kentos.form-sections";

/// A draft kept while it was edited: the form it is of, its name typed and its body.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeptDraft {
    pub form: String,
    pub name: String,
    pub body: FormBody,
}

/// The folder; none (tests, a device without a data folder): nothing is kept.
#[derive(Clone, Debug, Default)]
pub struct FormFiles {
    dir: Option<PathBuf>,
}

impl FormFiles {
    /// The device's: `$XDG_DATA_HOME/kentos-cad/formlar`, else `~/.local/share/kentos-cad/formlar`.
    pub fn device() -> Self {
        let base = std::env::var_os("XDG_DATA_HOME")
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")));
        Self {
            dir: base.map(|b| b.join("kentos-cad").join("formlar")),
        }
    }

    #[cfg(test)]
    pub fn at(dir: &Path) -> Self {
        Self {
            dir: Some(dir.to_path_buf()),
        }
    }

    fn library_file(&self) -> Option<PathBuf> {
        self.dir.as_ref().map(|d| d.join("bolumler.json"))
    }

    /// The sections, newest version of each name first by name; an unreadable file is none (kept as it
    /// is: a save renames it aside first).
    pub fn sections(&self) -> Vec<Section> {
        let Some(path) = self.library_file() else {
            return Vec::new();
        };
        std::fs::read(&path)
            .ok()
            .and_then(|b| serde_json::from_slice::<Library>(&b).ok())
            .filter(|l| l.format == FORMAT)
            .map(|l| l.sections)
            .unwrap_or_default()
    }

    /// Keeps `item` as the next version of `name`; the version it took.
    pub fn save_section(
        &self,
        name: &str,
        item: &FormItem,
        fields: &[LayerField],
        saved: String,
    ) -> Result<u32, String> {
        let path = self
            .library_file()
            .ok_or("Bu cihazda uygulama veri klasörü yok (XDG_DATA_HOME ya da HOME).")?;
        let name = name.trim();
        if name.is_empty() || name.chars().count() > 64 {
            return Err("Bölümün adı boş olamaz, en çok 64 karakterdir.".into());
        }
        let raw = std::fs::read(&path).ok();
        let mut sections = match &raw {
            Some(b) => match serde_json::from_slice::<Library>(b) {
                Ok(l) if l.format == FORMAT => l.sections,
                // An unreadable library is set aside, never written over.
                _ => {
                    let aside = path
                        .with_file_name(format!("bolumler-okunamadi-{}.json", chrono_free_stamp()));
                    std::fs::rename(&path, aside).map_err(|e| e.to_string())?;
                    Vec::new()
                }
            },
            None => Vec::new(),
        };
        let version = sections
            .iter()
            .filter(|s| s.name == name)
            .map(|s| s.version)
            .max()
            .unwrap_or(0)
            + 1;
        // Only the fields the section binds travel with it.
        let bound = bound_keys(item);
        sections.push(Section {
            name: name.to_owned(),
            version,
            saved,
            item: item.clone(),
            fields: fields
                .iter()
                .filter(|f| bound.iter().any(|k| k == f.key()))
                .cloned()
                .collect(),
        });
        while sections.len() > MOST_SECTIONS {
            sections.remove(0);
        }
        let text = serde_json::to_vec_pretty(&Library {
            format: FORMAT.into(),
            version: 1,
            sections,
        })
        .map_err(|e| e.to_string())?;
        write_atomically(&path, &text)?;
        Ok(version)
    }

    fn draft_file(&self, project: &str, form: &str) -> Option<PathBuf> {
        let key: String = format!("{project}-{form}")
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        self.dir
            .as_ref()
            .map(|d| d.join("taslak").join(format!("{key}.json")))
    }

    /// Keeps the window's draft of `form` in `project` (written each step; small).
    pub fn keep_draft(&self, project: &str, draft: &KeptDraft) {
        if let Some(path) = self.draft_file(project, &draft.form)
            && let Ok(text) = serde_json::to_vec(draft)
        {
            let _ = write_atomically(&path, &text);
        }
    }

    /// The draft kept of `form`, if one is.
    pub fn kept_draft(&self, project: &str, form: &str) -> Option<KeptDraft> {
        let path = self.draft_file(project, form)?;
        let bytes = std::fs::read(path).ok()?;
        serde_json::from_slice(&bytes)
            .ok()
            .filter(|d: &KeptDraft| d.form == form)
    }

    /// Forgets it: saved or let go.
    pub fn drop_draft(&self, project: &str, form: &str) {
        if let Some(path) = self.draft_file(project, form) {
            let _ = std::fs::remove_file(path);
        }
    }
}

/// The layer field keys an item binds, nested ones too.
pub fn bound_keys(item: &FormItem) -> Vec<String> {
    let mut out = Vec::new();
    fn go(i: &FormItem, out: &mut Vec<String>) {
        if let FormItem::Field(f) = i
            && !out.contains(&f.field)
        {
            out.push(f.field.clone());
        }
        for x in i.items() {
            go(x, out);
        }
    }
    go(item, &mut out);
    out
}

/// How a section's fields meet a layer's (the binding shown before it goes in): each kept field with
/// the layer's field it binds to, by key, else by name; none: not on the layer.
pub fn bindings(section: &Section, to: &[LayerField]) -> Vec<(String, Option<String>)> {
    bound_keys(&section.item)
        .into_iter()
        .map(|key| {
            let from = section.fields.iter().find(|f| f.key() == key);
            let label = from.map_or(key.clone(), |f| f.label().to_owned());
            let hit = to
                .iter()
                .find(|t| t.key() == key)
                .or_else(|| from.and_then(|f| to.iter().find(|t| t.name == f.name)))
                .map(|t| t.label().to_owned());
            (label, hit)
        })
        .collect()
}

fn write_atomically(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let tmp = path.with_extension("json.yaziliyor");
    std::fs::write(&tmp, bytes).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, path).map_err(|e| e.to_string())
}

/// Seconds since the epoch: a set-aside file's name.
fn chrono_free_stamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use kentos_commands::{FormField, LayerFieldKind};

    fn group() -> FormItem {
        FormItem::Group {
            across: false,
            id: "adres".into(),
            label: Some("Adres".into()),
            columns: None,
            collapsed: false,
            visible_when: None,
            items: vec![
                FormItem::Field(FormField::new("il", "Il")),
                FormItem::Field(FormField::new("ilce", "Ilce")),
            ],
        }
    }

    #[test]
    fn a_section_s_name_takes_versions_and_its_fields_travel() {
        let dir = std::env::temp_dir().join(format!("kentos-bolum-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let files = FormFiles::at(&dir);
        let fields = vec![
            LayerField::new("Il", LayerFieldKind::Text),
            LayerField::new("Ilce", LayerFieldKind::Text),
            LayerField::new("Kat", LayerFieldKind::Integer),
        ];
        assert_eq!(
            files.save_section("Adres", &group(), &fields, String::new()),
            Ok(1)
        );
        assert_eq!(
            files.save_section("Adres", &group(), &fields, String::new()),
            Ok(2)
        );
        let kept = files.sections();
        assert_eq!(kept.len(), 2);
        assert_eq!(kept[1].version, 2);
        assert_eq!(kept[1].fields.len(), 2, "Kat is not bound");
        let to = vec![LayerField::new("Il", LayerFieldKind::Text)];
        assert_eq!(
            bindings(&kept[1], &to),
            [
                ("Il".to_owned(), Some("Il".to_owned())),
                ("Ilce".to_owned(), None)
            ]
        );
        // A draft is kept, read and dropped.
        let draft = KeptDraft {
            form: "f".into(),
            name: "F".into(),
            body: FormBody::default(),
        };
        files.keep_draft("proje", &draft);
        assert_eq!(files.kept_draft("proje", "f"), Some(draft));
        files.drop_draft("proje", "f");
        assert_eq!(files.kept_draft("proje", "f"), None);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
