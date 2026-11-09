//! A map project's work structure (plan/map-cad.md MAP-01, docs/adr/1200): its survey campaigns, reference sources,
//! working data, designs, check results and delivery packages as project items, and what the drawn objects were derived
//! from. An item holds layers (the layer profile: an object belongs to the item of its layer or of a group above it) and,
//! when it is a file the project rests on (a GNSS log, an orthophoto, a CAD reference), the file with the SHA-256 of the
//! content the project last accepted. A derivation names objects by persistent id, the item they came from, that item's
//! revision when they were made and the operation that made them: the derivation is out of date when the item's
//! revision moved on. A project setting (`.kcad` schema 188, `mapWork`), a map project's only.
//!
//! An item without a file has a content revision (schema 206, docs/adr/2080 §1): the digest of the objects its layers
//! hold, worked out when read (`kentos-native-application`'s `map_work::content_revision`), so objects can be derived
//! from a design and an approval can stamp what it approved.
//!
//! An item's status is the office's own record: “onaylı” is an approved delivery revision, never an institution's
//! approval, and a passed check does not set it. An approval keeps the revision it approved (`approved`, schema 206):
//! an item that moved on since shows it.

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
    /// An absolute path, as added or last found.
    pub path: String,
    /// The SHA-256 of its content when the project last accepted it, 64 lower-case hex digits: the item's revision.
    pub revision: String,
    /// Its path from the project file's folder, `/` between its parts, written when the project is saved (schema 193,
    /// docs/adr/1201 §1): a project moved with its data finds the file again; none on another drive.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub relative: Option<String>,
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
    /// The revision its approval stamped (schema 206, docs/adr/2080 §3), 64 lower-case hex digits: its file's revision
    /// or, without a file, its content revision then; only on an approved item. None on an item approved before 206.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub approved: Option<String>,
    /// The objects its approval stamped (schema 210, docs/adr/2880): `[persistent id, object digest]` by id, the very
    /// list whose compact JSON's SHA-256 is `approved`; only on an approved item without a file, of at most
    /// [`MAX_APPROVED_OBJECTS`]. Empty: approved before 210, an item with a file, or one of more objects.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<Vec<(EntityId, String)>>"))]
    pub approved_objects: Vec<(EntityId, String)>,
    /// Each approved object's parts' digests (schema 211, docs/adr/3280), in `approved_objects`' order: its geometry,
    /// attributes, layer and look ([`OBJECT_PARTS`], [`object_parts`]), so what changes after can be told part by part.
    /// Empty: approved before 211, or no `approved_objects`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<Vec<[String; 4]>>"))]
    pub approved_parts: Vec<[String; 4]>,
    /// Each approved object's value as approved (schema 213, docs/adr/3680), in `approved_objects`' order: the very text
    /// its object digest is the SHA-256 of (`kentos.object-digest/1`: its snapshot form without the slot id, keys
    /// sorted, compact JSON), so what an attribute was and what a removed object was can be said. Empty: approved before
    /// 213, no `approved_objects`, or their texts together over [`MAX_APPROVED_VALUES_BYTES`].
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<Vec<String>>"))]
    pub approved_values: Vec<String>,
}

/// The most objects an approval keeps a digest for (docs/adr/2880): an item of more is stamped whole only.
pub const MAX_APPROVED_OBJECTS: usize = 100_000;

/// The most bytes of its objects' values an approval keeps together (docs/adr/3680): an item of more keeps their digests
/// and parts only. KentOS's choice, no rule's: the file and each undo step of the work's settings stay small.
pub const MAX_APPROVED_VALUES_BYTES: usize = 4 * 1024 * 1024;

/// The parts of an object an approval keeps a digest of (docs/adr/3280), in their order.
pub const OBJECT_PARTS: [&str; 4] = ["geometry", "attributes", "layer", "look"];
/// The fields of an object's snapshot form that are its attributes part (docs/adr/3280).
pub const ATTRIBUTE_FIELDS: [&str; 2] = ["attrs", "attachments"];
/// The field that is its layer part.
pub const LAYER_FIELD: &str = "layerId";
/// The fields that are its look part; the rest but `id` (its `kind` and its own fields: places, corners, a text's
/// words …) are its geometry.
pub const LOOK_FIELDS: [&str; 12] = [
    "color",
    "label",
    "symbol",
    "lineWeight",
    "labelPins",
    "lineType",
    "linePattern",
    "lineTypeScale",
    "transparency",
    "order",
    "byLayer",
    "wipeout",
];

/// An object's parts' digests (docs/adr/3280) from its snapshot form: each part's fields as compact JSON with sorted
/// keys (an object of the fields it has, maybe none), the first 16 lower-case hex digits of its SHA-256.
pub fn object_parts(object: &serde_json::Value) -> [String; 4] {
    let mut parts: [serde_json::Map<String, serde_json::Value>; 4] = Default::default();
    if let Some(fields) = object.as_object() {
        for (k, v) in fields {
            let at = match k.as_str() {
                "id" => continue,
                f if ATTRIBUTE_FIELDS.contains(&f) => 1,
                LAYER_FIELD => 2,
                f if LOOK_FIELDS.contains(&f) => 3,
                _ => 0,
            };
            parts[at].insert(k.clone(), v.clone());
        }
    }
    parts.map(|m| {
        let text = serde_json::to_vec(&serde_json::Value::Object(m)).unwrap_or_default();
        crate::encoding::sha256_hex(&text)[..16].to_owned()
    })
}

/// Whether a part's digest is 16 lower-case hex digits.
fn is_part_digest(d: &str) -> bool {
    d.len() == 16 && d.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
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
    /// The item they came from: one with a file or, from schema 206 (docs/adr/2080 §2), one without (a design).
    pub source: String,
    /// The item's revision when they were made (64 lower-case hex digits): its file's, or its content revision.
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

    /// Whether an item's file keeps its relative path (schema 193, docs/adr/1201 §1).
    pub fn has_relative(&self) -> bool {
        self.items.iter().any(|i| i.file.as_ref().is_some_and(|f| f.relative.is_some()))
    }

    /// Whether an approval keeps its objects' digests (schema 210, docs/adr/2880).
    pub fn has_approved_objects(&self) -> bool {
        self.items.iter().any(|i| !i.approved_objects.is_empty())
    }

    /// Whether an approval keeps its objects' parts' digests (schema 211, docs/adr/3280).
    pub fn has_approved_parts(&self) -> bool {
        self.items.iter().any(|i| !i.approved_parts.is_empty())
    }

    /// Whether an approval keeps its objects' values (schema 213, docs/adr/3680).
    pub fn has_approved_values(&self) -> bool {
        self.items.iter().any(|i| !i.approved_values.is_empty())
    }

    /// Whether it says what schema 206 added (docs/adr/2080): an approval's stamp, or a derivation from an item without
    /// a file.
    pub fn has_stamps(&self) -> bool {
        self.items.iter().any(|i| i.approved.is_some())
            || self.derivations.iter().any(|d| self.item(&d.source).is_some_and(|i| i.file.is_none()))
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
            if let Some(why) = file.relative.as_deref().and_then(crate::sources::relative_problem) {
                return fail(at("file.relative"), &format!("dosyanın göreli yolu {why}"));
            }
        }
        if let Some(stamp) = &item.approved {
            if item.status != WorkStatus::Approved {
                return fail(at("approved"), "onay damgası yalnız onaylı öğede olur");
            }
            if !is_sha256_text(stamp) {
                return fail(at("approved"), "onay damgası 64 küçük onaltılık haneli SHA-256 olmalı");
            }
        }
        // Docs/adr/2880: the objects an approval stamped, the list its stamp is the digest of.
        if !item.approved_objects.is_empty() {
            let objects = at("approvedObjects");
            let Some(stamp) = &item.approved else {
                return fail(objects, "nesne başına özet yalnız onay damgasıyla olur");
            };
            if item.file.is_some() {
                return fail(objects, "dosyası olan öğe nesne başına özet tutmaz; onayı dosyanın revizyonudur");
            }
            if item.approved_objects.len() > MAX_APPROVED_OBJECTS {
                return fail(objects, &format!("onay en çok {MAX_APPROVED_OBJECTS} nesnenin özetini tutar"));
            }
            for (k, (uid, digest)) in item.approved_objects.iter().enumerate() {
                if uid.is_nil() || !is_sha256_text(digest) {
                    return fail(objects, "her nesne boş olmayan bir kimlik ve 64 küçük onaltılık haneli özettir");
                }
                if k > 0 && item.approved_objects[k - 1].0 >= *uid {
                    return fail(objects, "nesneler kimliğe göre sıralı ve birer kez yazılır");
                }
            }
            let text = serde_json::to_vec(&item.approved_objects).unwrap_or_default();
            if crate::encoding::sha256_hex(&text) != *stamp {
                return fail(objects, "nesnelerin listesi onay damgasını vermiyor (damga listenin SHA-256'sı olmalı)");
            }
        }
        // Docs/adr/3280: each approved object's parts, one to one with the list.
        if !item.approved_parts.is_empty() {
            let parts = at("approvedParts");
            if item.approved_parts.len() != item.approved_objects.len() {
                return fail(parts, "nesnelerin parçaları onaylanan nesnelerle bire birdir (aynı sayı, aynı sıra)");
            }
            if item.approved_parts.iter().flatten().any(|d| !is_part_digest(d)) {
                return fail(parts, "her parçanın özeti 16 küçük onaltılık hanedir");
            }
        }
        // Docs/adr/3680: each approved object's value, one to one with the list, its SHA-256 the object's digest.
        if !item.approved_values.is_empty() {
            let values = at("approvedValues");
            if item.approved_values.len() != item.approved_objects.len() {
                return fail(values, "onaylı değerler onaylanan nesnelerle bire birdir (aynı sayı, aynı sıra)");
            }
            if item.approved_values.iter().map(String::len).sum::<usize>() > MAX_APPROVED_VALUES_BYTES {
                return fail(values, &format!("onay nesnelerin değerlerinden en çok {MAX_APPROVED_VALUES_BYTES} bayt tutar"));
            }
            for (value, (_, digest)) in item.approved_values.iter().zip(&item.approved_objects) {
                if crate::encoding::sha256_hex(value.as_bytes()) != *digest {
                    return fail(values, "her onaylı değerin SHA-256'sı nesnenin onaylanan özetidir");
                }
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
        if work.item(&d.source).is_none() {
            return fail(at("source"), "türetmenin kaynağı projenin bir öğesi olmalı");
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
                    file: Some(WorkFile { path: "/veri/kampanya.nmea".into(), revision: "a".repeat(64), relative: None }),
                    approved: None,
                    approved_objects: Vec::new(),
                    approved_parts: Vec::new(),
                    approved_values: Vec::new(),
                },
                WorkItem {
                    id: "is-2".into(),
                    kind: WorkKind::Design,
                    name: "Tasarım yüzeyi".into(),
                    status: WorkStatus::Review,
                    layers: vec!["tasarim".into()],
                    file: None,
                    approved: None,
                    approved_objects: Vec::new(),
                    approved_parts: Vec::new(),
                    approved_values: Vec::new(),
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
        // From schema 206 an item without a file is a source too (docs/adr/2080 §2); an item that is none is not.
        let mut w = work();
        w.derivations[0].source = "is-2".into();
        assert_eq!(w.problem(), None);
        assert!(w.has_stamps());
        w.derivations[0].source = "is-3".into();
        assert_eq!(w.problem().map(|p| p.0), Some("derivations[0].source".into()));
        // An approval's stamp: only on an approved item, a SHA-256 (docs/adr/2080 §3).
        let mut w = work();
        assert!(!w.has_stamps());
        w.items[1].approved = Some("b".repeat(64));
        assert_eq!(w.problem().map(|p| p.0), Some("items[1].approved".into()));
        w.items[1].status = WorkStatus::Approved;
        assert_eq!(w.problem(), None);
        assert!(w.has_stamps());
        w.items[1].approved = Some("B".repeat(64));
        assert_eq!(w.problem().map(|p| p.0), Some("items[1].approved".into()));
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
