//! An extension's translations (plan/eklenti.md, the package: "`l10n/<dil>.json` manifest, şerit, panel ve form
//! metinlerinin çevirileri"; ADIM 22's lesson "`l10n` anahtarları panel ve form metinlerine bağlanır"; docs/adr/8057
//! §106).
//!
//! - **A translation** is a flat JSON object, as MEBİS's reference package writes it: each key a text's place, each
//!   value the text in the file's language (`"entrypoints.commands.init-schema.title": "Set up the MEBİS layers"`). The
//!   manifest's language (`l10n.default`) is the package's own and has no file; a text no key names stays as written.
//! - **A place** is the dotted path to the text: an object's key, a list's item by its `id`, else its `name`, else its
//!   `value` (a choice's), else its number. The manifest's places stand as they are (`name`, `summary`,
//!   `contributions.ribbonTabs.mebis.label`); a panel's begin `ui.<panel>`, an İşlemler tool's `tools.<tool>`, the
//!   layer schema's `schema` and a form's `forms.<form>`, each by its local id ([`files`]).
//! - **The texts** are what the user reads: a file's own `name` at its root (the manifest's, a form package's), and
//!   every [`TEXT_KEYS`] text wherever it stands. An id, a command, a value, an expression is never one.
//! - **When:** KentOS's interface speaks Turkish ([`INTERFACE`]): an extension written in another language and
//!   translated to Turkish is shown in Turkish; one written in Turkish is shown as written.
//! - **Gate 1** refuses a file of another shape and warns of a place no text stands at (the text moved, the
//!   translation was left behind). A text left untranslated is no mistake: it is shown as written, and a translation
//!   grows a step at a time (MEBİS's English names four of its 57 texts).

use std::collections::BTreeMap;

use serde_json::Value;

use crate::manifest::Manifest;
use crate::problem::Problem;

/// The language KentOS's interface speaks.
pub const INTERFACE: &str = "tr";

/// The keys whose texts the user reads, wherever they stand in a package's JSON.
pub const TEXT_KEYS: [&str; 10] = [
    "label",
    "title",
    "summary",
    "description",
    "help",
    "text",
    "reason",
    "alias",
    "placeholder",
    "tooltip",
];

/// The places gate 1 names; the rest are counted.
const NAMED: usize = 5;

/// A translation: each text's place and the text shown there.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Texts(BTreeMap<String, String>);

impl Texts {
    /// A translation file read; why not, in Turkish. An empty translation is no translation of its text.
    pub fn read(bytes: &[u8]) -> Result<Texts, String> {
        let value: Value = serde_json::from_slice(bytes).map_err(|e| {
            format!(
                "geçerli JSON değil (satır {}, sütun {})",
                e.line(),
                e.column()
            )
        })?;
        let Some(map) = value.as_object() else {
            return Err(
                "bir JSON nesnesi değil: her metnin yeri ve çevirisi (\"yer\": \"çeviri\")"
                    .to_owned(),
            );
        };
        let mut out = BTreeMap::new();
        for (place, shown) in map {
            let Some(shown) = shown.as_str() else {
                return Err(format!("“{place}” yerinin çevirisi bir metin değil"));
            };
            if !shown.trim().is_empty() {
                out.insert(place.clone(), shown.to_owned());
            }
        }
        Ok(Texts(out))
    }

    /// The places it translates.
    pub fn places(&self) -> impl Iterator<Item = &String> {
        self.0.keys()
    }

    /// The text shown at `place`, when it translates it.
    pub fn at(&self, place: &str) -> Option<&str> {
        self.0.get(place).map(String::as_str)
    }

    /// `value`'s texts translated, its places beginning with `prefix` (`ui.search-dock`; empty for the manifest).
    pub fn apply(&self, value: &mut Value, prefix: &str) {
        walk_mut(value, prefix, true, &mut |place, text| {
            if let Some(shown) = self.0.get(place) {
                *text = shown.clone();
            }
        });
    }
}

/// The places' join: `a` then `b`, a dot between.
fn join(a: &str, b: &str) -> String {
    if a.is_empty() {
        b.to_owned()
    } else {
        format!("{a}.{b}")
    }
}

/// A list item's segment: its `id`, else its `name`, else its `value` (a choice's), else its number.
fn segment(item: &Value, at: usize) -> String {
    ["id", "name", "value"]
        .iter()
        .find_map(|k| item.get(*k).and_then(Value::as_str))
        .map_or_else(|| at.to_string(), str::to_owned)
}

/// Each text of `value` the user reads, with its place, to be changed; at the file's `root`, its own `name` is one.
fn walk_mut(value: &mut Value, at: &str, root: bool, f: &mut dyn FnMut(&str, &mut String)) {
    match value {
        Value::Object(map) => {
            for (key, item) in map.iter_mut() {
                let place = join(at, key);
                match item {
                    Value::String(text)
                        if TEXT_KEYS.contains(&key.as_str()) || (root && key == "name") =>
                    {
                        f(&place, text);
                    }
                    other => walk_mut(other, &place, false, f),
                }
            }
        }
        Value::Array(items) => {
            for (i, item) in items.iter_mut().enumerate() {
                let place = join(at, &segment(item, i));
                walk_mut(item, &place, false, f);
            }
        }
        _ => {}
    }
}

/// The texts of `value` the user reads, by their places.
pub fn texts_at(value: &Value, prefix: &str) -> BTreeMap<String, String> {
    let mut copy = value.clone();
    let mut out = BTreeMap::new();
    walk_mut(&mut copy, prefix, true, &mut |place, text| {
        if !text.trim().is_empty() {
            out.insert(place.to_owned(), text.clone());
        }
    });
    out
}

/// The package's files whose texts a translation reaches, each with its places' prefix: its panels (`ui.<id>`), its
/// İşlemler tools (`tools.<id>`), its layer schema (`schema`) and its forms (`forms.<id>`). The manifest's are its own.
pub fn files(manifest: &Manifest) -> Vec<(String, String)> {
    let e = &manifest.entrypoints;
    let mut out: Vec<(String, String)> = Vec::new();
    for p in &e.panels {
        out.push((p.definition.clone(), format!("ui.{}", p.id)));
    }
    for t in &e.processing {
        out.push((t.descriptor.clone(), format!("tools.{}", t.id)));
    }
    for f in &e.forms {
        out.push((f.definition.clone(), format!("forms.{}", f.id)));
    }
    if let Some(schema) = &manifest.schema_file {
        out.push((schema.clone(), "schema".to_owned()));
    }
    out
}

/// The translation into the interface's language `manifest` is shown with, its file read by `read` (a package's file by
/// its path): none when the manifest is written in that language, has no translation to it, or the file does not read.
pub fn for_interface(manifest: &Manifest, read: impl Fn(&str) -> Option<Vec<u8>>) -> Option<Texts> {
    let l10n = manifest.l10n.as_ref()?;
    if l10n.default == INTERFACE {
        return None;
    }
    let path = l10n.files.get(INTERFACE)?;
    Texts::read(&read(path)?).ok()
}

/// `manifest` as the interface shows it: its texts translated when it has a translation to the interface's language.
pub fn localized(manifest: Manifest, read: impl Fn(&str) -> Option<Vec<u8>>) -> Manifest {
    let Some(texts) = for_interface(&manifest, read) else {
        return manifest;
    };
    let Ok(mut value) = serde_json::to_value(&manifest) else {
        return manifest;
    };
    texts.apply(&mut value, "");
    serde_json::from_value(value).unwrap_or(manifest)
}

/// A file of the package as the interface shows it (a panel's, a tool's: [`files`]'s prefix), `bytes` as they are
/// without a translation or when they do not read as JSON.
pub fn localized_json(bytes: Vec<u8>, texts: Option<&Texts>, prefix: &str) -> Vec<u8> {
    let Some(texts) = texts else {
        return bytes;
    };
    let Ok(mut value) = serde_json::from_slice::<Value>(&bytes) else {
        return bytes;
    };
    texts.apply(&mut value, prefix);
    serde_json::to_vec(&value).unwrap_or(bytes)
}

/// What gate 1 says of the translation at `path` of a package whose texts stand at `texts`' places: the places no text
/// stands at (a warning).
pub fn check(path: &str, translation: &Texts, texts: &BTreeMap<String, String>) -> Option<Problem> {
    let stale: Vec<&String> = translation
        .places()
        .filter(|p| !texts.contains_key(*p))
        .collect();
    (!stale.is_empty()).then(|| {
        Problem::new(
            "l10n_stale",
            format!(
                "{} çevirinin yerinde metin yok (yeri değişmiş ya da kaldırılmış): {}. Yeni yeriyle yazın; \
                 çevrilmemiş metin yazıldığı gibi gösterilir.",
                stale.len(),
                named(&stale)
            ),
        )
        .file(path.to_owned())
    })
}

/// The first few places, quoted, and how many more.
fn named(places: &[&String]) -> String {
    let mut words: Vec<String> = places
        .iter()
        .take(NAMED)
        .map(|p| format!("“{p}”"))
        .collect();
    if places.len() > NAMED {
        words.push(format!("ve {} tane daha", places.len() - NAMED));
    }
    words.join(", ")
}

#[cfg(test)]
mod tests;
