//! A map project's work structure (plan/map-cad.md MAP-01, docs/adr/1200): its survey campaigns, reference sources,
//! working data, designs, check results and delivery packages as project items, and what the drawn objects were derived
//! from. An item holds layers (the layer profile: an object belongs to the item of its layer or of a group above it) and,
//! when it is a file the project rests on (a GNSS log, an orthophoto, a CAD reference), the file with the SHA-256 of the
//! content the project last accepted. A derivation names objects by persistent id, the item they came from, that item's
//! revision when they were made and the operation that made them: the derivation is out of date when the item's
//! revision moved on. A project setting (`.kcad` schema 188, `mapWork`), a map project's only.
//!
//! An item's status is the office's own record: “onaylı” is an approved delivery revision, never an institution's
//! approval, and a passed check does not set it.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::EntityId;
use crate::sheets::is_sha256_text;

/// At most this many items in a project.
pub const MAX_WORK_ITEMS: usize = 1000;
/// At most this many derivations in a project.
pub const MAX_DERIVATIONS: usize = 10_000;
/// At most this many objects in one derivation.
pub const MAX_DERIVED_OBJECTS: usize = 1_000_000;
/// The longest item name and operation, in characters.
pub const MOST_WORK_TEXT: usize = 200;
/// At most this many layers in one item.
pub const MAX_ITEM_LAYERS: usize = 1000;

/// What an item is in the map's work.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum WorkKind {
    /// A survey campaign: GNSS, total station or scan data gathered in the field.
    Campaign,
    /// A reference source taken as it is: an orthophoto, a CAD reference, a base map.
    Reference,
    /// The working data drawn and edited in the project.
    Working,
    /// A design: a surface, a plan, an infrastructure layout.
    Design,
    /// The results of a check (topology, quality, preflight).
    Check,
    /// A delivery package.
    Delivery,
}

impl WorkKind {
    pub const ALL: [WorkKind; 6] = [
        WorkKind::Campaign,
        WorkKind::Reference,
        WorkKind::Working,
        WorkKind::Design,
        WorkKind::Check,
        WorkKind::Delivery,
    ];

    /// Whether its objects are source data (a campaign's, a reference's) rather than the project's own drawing.
    pub fn is_source(self) -> bool {
        matches!(self, WorkKind::Campaign | WorkKind::Reference)
    }
}

/// Where an item's work stands, as the office records it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum WorkStatus {
    /// Taslak.
    #[default]
    Draft,
    /// Denetlenecek: handed over to be checked.
    Review,
    /// Onaylı teslim revizyonu: the office approved it for delivery; not an institution's approval.
    Approved,
}

impl WorkStatus {
    fn is_draft(&self) -> bool {
        *self == WorkStatus::Draft
    }
}

/// The file an item is: where it is and the content the project last accepted.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct WorkFile {
    /// An absolute path.
    pub path: String,
    /// The SHA-256 of its content when the project last accepted it, 64 lower-case hex digits: the item's revision.
    pub revision: String,
}

/// An item of the map's work.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct WorkItem {
    /// `is-N`.
    pub id: String,
    pub kind: WorkKind,
    /// 1–200 characters, no space at either end.
    pub name: String,
    #[serde(default, skip_serializing_if = "WorkStatus::is_draft")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<WorkStatus>"))]
    pub status: WorkStatus,
    /// Layer or group ids; a layer is in one item at most.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<Vec<String>>"))]
    pub layers: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub file: Option<WorkFile>,
}

/// Objects made from an item: the item, its revision then, the operation that made them and the objects.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct WorkDerivation {
    /// `tr-N`.
    pub id: String,
    /// The item they came from: one with a file.
    pub source: String,
    /// The item's revision when they were made (64 lower-case hex digits).
    pub revision: String,
    /// What made them: a command's id or a step's name, 1–200 characters.
    pub operation: String,
    /// The objects, each in one derivation at most.
    pub uids: Vec<EntityId>,
}

/// A map project's work structure.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct MapWork {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<Vec<WorkItem>>"))]
    pub items: Vec<WorkItem>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<Vec<WorkDerivation>>"))]
    pub derivations: Vec<WorkDerivation>,
}

impl MapWork {
    /// Whether it says nothing (and is not written).
    pub fn is_empty(&self) -> bool {
        self.items.is_empty() && self.derivations.is_empty()
    }

    pub fn item(&self, id: &str) -> Option<&WorkItem> {
        self.items.iter().find(|i| i.id == id)
    }

    /// What is wrong with it, by the field's path and in words; none when it is right.
    pub fn problem(&self) -> Option<(String, String)> {
        map_work_problem(self)
    }
}

/// `prefix-N` with N a positive number written without leading zeros.
fn numbered(id: &str, prefix: &str) -> bool {
    id.strip_prefix(prefix).is_some_and(|n| {
        !n.is_empty() && !n.starts_with('0') && n.len() <= 9 && n.bytes().all(|b| b.is_ascii_digit())
    })
}

fn text_ok(s: &str) -> bool {
    let n = s.chars().count();
    n > 0 && n <= MOST_WORK_TEXT && s.trim() == s
}

/// The next free `prefix-N` among `ids`.
pub fn next_numbered<'a>(ids: impl Iterator<Item = &'a str>, prefix: &str) -> String {
    let most = ids
        .filter_map(|id| id.strip_prefix(prefix).and_then(|n| n.parse::<u64>().ok()))
        .max()
        .unwrap_or(0);
    format!("{prefix}{}", most + 1)
}

/// Why `work` cannot be a project's, by the field's path and in words; none when it can.
pub fn map_work_problem(work: &MapWork) -> Option<(String, String)> {
    let fail = |path: String, words: &str| Some((path, words.to_owned()));
    if work.items.len() > MAX_WORK_ITEMS {
        return fail("items".into(), &format!("en çok {MAX_WORK_ITEMS} iş öğesi olur"));
    }
    let mut ids = std::collections::BTreeSet::new();
    let mut layers = std::collections::BTreeSet::new();
    for (i, item) in work.items.iter().enumerate() {
        let at = |field: &str| format!("items[{i}].{field}");
        if !numbered(&item.id, "is-") {
            return fail(at("id"), "öğenin kimliği is-1, is-2 … biçiminde olmalı");
        }
        if !ids.insert(item.id.as_str()) {
            return fail(at("id"), "bu kimlikte başka bir öğe var");
        }
        if !text_ok(&item.name) {
            return fail(
                at("name"),
                &format!("öğenin adı boşluksuz başlayıp biten 1–{MOST_WORK_TEXT} karakter olmalı"),
            );
        }
        if item.layers.len() > MAX_ITEM_LAYERS {
            return fail(at("layers"), &format!("bir öğede en çok {MAX_ITEM_LAYERS} katman olur"));
        }
        for layer in &item.layers {
            if layer.is_empty() {
                return fail(at("layers"), "katmanın kimliği boş olamaz");
            }
            if !layers.insert(layer.as_str()) {
                return fail(at("layers"), &format!("“{layer}” katmanı iki öğede ya da iki kez yazılmış"));
            }
        }
        if let Some(file) = &item.file {
            if file.path.trim().is_empty() || file.path.trim() != file.path {
                return fail(at("file.path"), "dosyanın yolu boş olamaz, başında ve sonunda boşluk olmaz");
            }
            if !is_sha256_text(&file.revision) {
                return fail(at("file.revision"), "dosyanın revizyonu 64 küçük onaltılık haneli SHA-256 olmalı");
            }
        }
    }
    if work.derivations.len() > MAX_DERIVATIONS {
        return fail("derivations".into(), &format!("en çok {MAX_DERIVATIONS} türetme kaydı olur"));
    }
    let mut dids = std::collections::BTreeSet::new();
    let mut objects = std::collections::HashSet::new();
    for (i, d) in work.derivations.iter().enumerate() {
        let at = |field: &str| format!("derivations[{i}].{field}");
        if !numbered(&d.id, "tr-") {
            return fail(at("id"), "türetmenin kimliği tr-1, tr-2 … biçiminde olmalı");
        }
        if !dids.insert(d.id.as_str()) {
            return fail(at("id"), "bu kimlikte başka bir türetme var");
        }
        match work.item(&d.source) {
            None => return fail(at("source"), "türetmenin kaynağı projenin bir öğesi olmalı"),
            Some(item) if item.file.is_none() => {
                return fail(at("source"), "türetmenin kaynağı dosyası olan bir öğe olmalı");
            }
            Some(_) => {}
        }
        if !is_sha256_text(&d.revision) {
            return fail(at("revision"), "kaynağın revizyonu 64 küçük onaltılık haneli SHA-256 olmalı");
        }
        if !text_ok(&d.operation) {
            return fail(
                at("operation"),
                &format!("üretim işlemi boşluksuz başlayıp biten 1–{MOST_WORK_TEXT} karakter olmalı"),
            );
        }
        if d.uids.is_empty() || d.uids.len() > MAX_DERIVED_OBJECTS {
            return fail(at("uids"), &format!("türetmede 1 ile {MAX_DERIVED_OBJECTS} arasında nesne olur"));
        }
        for u in &d.uids {
            if u.is_nil() {
                return fail(at("uids"), "nesnenin kimliği boş olamaz");
            }
            if !objects.insert(*u) {
                return fail(at("uids"), &format!("“{u}” nesnesi iki türetmede ya da iki kez yazılmış"));
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn uid(n: u8) -> EntityId {
        let mut b = [0u8; 16];
        b[15] = n;
        EntityId(b)
    }

    fn work() -> MapWork {
        MapWork {
            items: vec![
                WorkItem {
                    id: "is-1".into(),
                    kind: WorkKind::Campaign,
                    name: "GNSS 2026-10".into(),
                    status: WorkStatus::Draft,
                    layers: vec!["gnss".into()],
                    file: Some(WorkFile { path: "/veri/kampanya.nmea".into(), revision: "a".repeat(64) }),
                },
                WorkItem {
                    id: "is-2".into(),
                    kind: WorkKind::Design,
                    name: "Tasarım yüzeyi".into(),
                    status: WorkStatus::Review,
                    layers: vec!["tasarim".into()],
                    file: None,
                },
            ],
            derivations: vec![WorkDerivation {
                id: "tr-1".into(),
                source: "is-1".into(),
                revision: "a".repeat(64),
                operation: "file.import.gnss".into(),
                uids: vec![uid(1), uid(2)],
            }],
        }
    }

    #[test]
    fn a_work_structure_keeps_its_rules() {
        assert_eq!(work().problem(), None);
        let mut w = work();
        w.items[1].layers = vec!["gnss".into()];
        assert_eq!(w.problem().map(|p| p.0), Some("items[1].layers".into()));
        let mut w = work();
        w.items[1].id = "is-01".into();
        assert_eq!(w.problem().map(|p| p.0), Some("items[1].id".into()));
        let mut w = work();
        w.derivations[0].source = "is-2".into();
        assert_eq!(w.problem().map(|p| p.0), Some("derivations[0].source".into()));
        let mut w = work();
        w.derivations[0].uids.push(uid(1));
        assert_eq!(w.problem().map(|p| p.0), Some("derivations[0].uids".into()));
        let mut w = work();
        w.items[0].file.as_mut().map(|f| f.revision = "A".repeat(64));
        assert_eq!(w.problem().map(|p| p.0), Some("items[0].file.revision".into()));
        let mut w = work();
        w.items[0].name = " GNSS".into();
        assert_eq!(w.problem().map(|p| p.0), Some("items[0].name".into()));
    }

    #[test]
    fn the_next_id_follows_the_largest() {
        assert_eq!(next_numbered(["is-1", "is-7", "x"].into_iter(), "is-"), "is-8");
        assert_eq!(next_numbered(std::iter::empty(), "tr-"), "tr-1");
    }
}
