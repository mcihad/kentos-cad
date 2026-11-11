//! The parcel register a drawing carries (docs/adr/4220; plan/map-cad.md MAP-08), apart from its areas: the documents
//! that made and closed parcels, each parcel as its document made it, and the boundaries the parcels share. A document
//! (a deed, a plan, a survey) creates parcels and closes earlier ones, which stay as history; a parcel's boundaries are
//! shared with its neighbours and close once no current parcel uses them.
//!
//! Docs/adr/5020: a document states its boundaries' lengths as measured (`lengths`, the document's values, kept as
//! written); a document may first be a draft (`drafts`: what it would make and close, its areas' shape kept as drafted,
//! nothing current changed) and an approved document carries the approval as written outside (`approval`: its number,
//! date and who approved; KentOS never approves).
//!
//! A ring is a chain of boundaries: `edges` holds their numbers, negative where the ring runs a boundary from its
//! `to` to its `from` (TopoJSON's arcs do the same). Two parcels that use one boundary number share that boundary, by
//! its identity and not because their coordinates meet. The area that draws a current parcel is its checkable picture:
//! editing the area changes the drawing, never the register (the stated area, the recorded shape and the document
//! stay). The file reader checks the structure here; the commands check the drawing.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::Vec2;
use crate::review_notes::{review_id_ok, review_label_ok, review_text_ok};

/// The most documents a register holds.
pub const MAX_PARCEL_DOCUMENTS: usize = 10_000;
/// The most parcels, current and closed, a register holds.
pub const MAX_RECORDED_PARCELS: usize = 100_000;
/// The most boundaries, current and closed, a register holds.
pub const MAX_RECORDED_BOUNDARIES: usize = 1_000_000;
/// The most characters of a document's name and kind and of an identity's part.
pub const MAX_PARCEL_NAME: usize = 120;
/// The most characters of a document's date and of a stated area, as written.
pub const MAX_PARCEL_LABEL: usize = 64;
/// The most characters of a document's note.
pub const MAX_PARCEL_NOTE: usize = 4000;
/// Two bulges within this are one boundary's (docs/adr/2000's rule for the boundary graph).
pub const BOUNDARY_BULGE_TOLERANCE: f64 = 1e-9;

/// The most characters of an approval's number.
pub const MAX_APPROVAL_REFERENCE: usize = 64;

/// The register: documents, boundaries and parcels, each numbered 1, 2, 3 … in the order they were made; the lengths
/// the documents state; the drafts, numbered apart (docs/adr/5020).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ParcelRegister {
    pub documents: Vec<ParcelDocument>,
    pub boundaries: Vec<RecordedBoundary>,
    pub parcels: Vec<RecordedParcel>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<Vec<RecordedLength>>"))]
    pub lengths: Vec<RecordedLength>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<Vec<ParcelDraft>>"))]
    pub drafts: Vec<ParcelDraft>,
}

/// An approval written outside KentOS, as written (docs/adr/5020): its number (never empty), its date and who approved.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ParcelApproval {
    pub reference: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<String>"))]
    pub date: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<String>"))]
    pub by: String,
}

/// The length a document states for a boundary (docs/adr/5020): a plain decimal in metres as written, an arc's length
/// on an arc. One per boundary and document.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RecordedLength {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
    pub boundary: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
    pub document: u32,
    pub length: String,
}

/// A length stated for one edge of an area's ring (docs/adr/5020): the ring in the area's order (the outline, its holes,
/// then each further part's outline and holes; 0 first), the edge from the ring's vertex `edge` to the next, the length a
/// plain decimal in metres as written.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct StatedLength {
    #[serde(default, skip_serializing_if = "is_first")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<u32>"))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub ring: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub edge: u32,
    pub length: String,
}

/// A draft's ring as its area was when drafted: its part, whether a hole, its vertices and each edge's bulge (none: all
/// straight).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DraftRing {
    #[serde(default, skip_serializing_if = "is_first")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<u32>"))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub part: u32,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<bool>"))]
    pub hole: bool,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub pts: Vec<Vec2>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<Vec<f64>>"))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub bulges: Vec<f64>,
}

/// A parcel a draft would make: its area, identity and stated area as read when drafted, its rings as drawn then and the
/// lengths the document states.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DraftParcel {
    pub uid: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<String>"))]
    pub il: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<String>"))]
    pub ilce: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<String>"))]
    pub mahalle: String,
    pub ada: String,
    pub parsel: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<String>"))]
    pub stated_area: String,
    pub rings: Vec<DraftRing>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<Vec<StatedLength>>"))]
    pub lengths: Vec<StatedLength>,
}

/// A document not yet approved (docs/adr/5020): its fields as a document's, the parcels it would make and the recorded
/// parcels it would close. Approving it writes it as a document with the next number; nothing current changes before.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ParcelDraft {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
    pub number: u32,
    pub name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<String>"))]
    pub kind: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<String>"))]
    pub date: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<String>"))]
    pub note: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<Vec<DraftParcel>>"))]
    pub create: Vec<DraftParcel>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<Vec<u32>>"))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
    pub close: Vec<u32>,
}

/// A document that made or closed parcels: its name, unique in the drawing; its kind as the user
/// writes it (a plan, a deed …); the date it gives, as written; a note.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ParcelDocument {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
    pub number: u32,
    pub name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<String>"))]
    pub kind: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<String>"))]
    pub date: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<String>"))]
    pub note: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub approval: Option<ParcelApproval>,
}

/// A boundary: from one corner to another (`from` the lesser by east, then north), its bulge from `from` to `to` (0:
/// straight), the document that made it and the one that closed it once no current parcel used it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RecordedBoundary {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
    pub number: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub from: Vec2,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub to: Vec2,
    #[serde(default, skip_serializing_if = "is_straight")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<f64>"))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub bulge: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
    pub created_by: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
    pub closed_by: Option<u32>,
}

fn is_straight(bulge: &f64) -> bool {
    *bulge == 0.0
}

fn is_first(part: &u32) -> bool {
    *part == 0
}

/// One ring of a parcel: its part (0 for the first), whether it is a hole, and its boundaries in order (negative: run
/// from `to` to `from`). The ring's vertices are each boundary's start as the ring runs it.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RecordedRing {
    #[serde(default, skip_serializing_if = "is_first")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<u32>"))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub part: u32,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<bool>"))]
    pub hole: bool,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
    pub edges: Vec<i32>,
}

/// A parcel as its document made it: the identity's parts as written (İl, İlçe, Mahalle, Ada, Parsel; Ada and Parsel
/// never empty), the document that made it and the one that closed it, the area the document states (a plain decimal
/// in square metres, as written; empty: none), the area that draws it while it is current, and its rings.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RecordedParcel {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
    pub number: u32,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<String>"))]
    pub il: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<String>"))]
    pub ilce: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<String>"))]
    pub mahalle: String,
    pub ada: String,
    pub parsel: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
    pub created_by: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
    pub closed_by: Option<u32>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<String>"))]
    pub stated_area: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub uid: Option<String>,
    pub rings: Vec<RecordedRing>,
}

impl RecordedParcel {
    /// The identity's key, `İl/İlçe/Mahalle/Ada/Parsel` (docs/adr/1400: never the parcel number alone).
    pub fn key(&self) -> String {
        format!("{}/{}/{}/{}/{}", self.il, self.ilce, self.mahalle, self.ada, self.parsel)
    }

    /// `Ada/Parsel`, the short name.
    pub fn label(&self) -> String {
        format!("{}/{}", self.ada, self.parsel)
    }
}

/// A ring's vertices and bulges, each edge's as the ring runs it.
pub type RingPath = (Vec<Vec2>, Vec<f64>);

impl ParcelRegister {
    pub fn is_empty(&self) -> bool {
        self.documents.is_empty() && self.boundaries.is_empty() && self.parcels.is_empty() && self.drafts.is_empty()
    }

    /// Whether the register holds what schema 241 brought (docs/adr/5020): stated lengths, drafts or an approval.
    pub fn has_drafts(&self) -> bool {
        !self.lengths.is_empty() || !self.drafts.is_empty() || self.documents.iter().any(|d| d.approval.is_some())
    }

    pub fn draft(&self, number: u32) -> Option<&ParcelDraft> {
        self.drafts.iter().find(|d| d.number == number)
    }

    pub fn document(&self, number: u32) -> Option<&ParcelDocument> {
        self.documents.iter().find(|d| d.number == number)
    }

    pub fn boundary(&self, number: u32) -> Option<&RecordedBoundary> {
        self.boundaries.iter().find(|b| b.number == number)
    }

    pub fn parcel(&self, number: u32) -> Option<&RecordedParcel> {
        self.parcels.iter().find(|p| p.number == number)
    }

    /// The vertices and bulges of a ring as it runs its boundaries; none when a boundary is missing.
    pub fn ring_path(&self, ring: &RecordedRing) -> Option<RingPath> {
        let by: HashMap<u32, &RecordedBoundary> = self.boundaries.iter().map(|b| (b.number, b)).collect();
        ring_path_with(&by, ring)
    }
}

fn ring_path_with(by: &HashMap<u32, &RecordedBoundary>, ring: &RecordedRing) -> Option<RingPath> {
    let mut pts = Vec::with_capacity(ring.edges.len());
    let mut bulges = Vec::with_capacity(ring.edges.len());
    for &e in &ring.edges {
        let b = by.get(&e.unsigned_abs())?;
        if e > 0 {
            pts.push(b.from);
            bulges.push(b.bulge);
        } else {
            pts.push(b.to);
            bulges.push(-b.bulge);
        }
    }
    Some((pts, bulges))
}

/// Whether `text` is a plain decimal (digits, at most one point or comma, digits: “118,5”, “723.525”), the only form a
/// stated area is kept in (CLAUDE.md §23.1: a decimal string, never a float).
pub fn plain_decimal(text: &str) -> bool {
    let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    match text.split_once(['.', ',']) {
        Some((whole, fraction)) => digits(whole) && digits(fraction),
        None => digits(text),
    }
}

fn finite(p: Vec2) -> bool {
    p.x.is_finite() && p.y.is_finite()
}

fn optional_label(text: &str, most: usize) -> bool {
    text.is_empty() || review_label_ok(text, most)
}

/// What is wrong with a register, in words; none when it reads. The numbers are unique; a document's name too; every
/// boundary and parcel names documents that exist, closed by a later one; a parcel has Ada and Parsel, a plain decimal
/// stated area or none, an area while current and none once closed; its rings run existing boundaries end to end, a
/// part's outline first and its holes after it; the current parcels hold different areas and identities and use only
/// current boundaries; every boundary is some parcel's, and a current one a current parcel's.
pub fn parcel_register_problem(r: &ParcelRegister) -> Option<String> {
    if r.documents.len() > MAX_PARCEL_DOCUMENTS
        || r.parcels.len() > MAX_RECORDED_PARCELS
        || r.boundaries.len() > MAX_RECORDED_BOUNDARIES
    {
        return Some(format!(
            "Parsel kayıtları en çok {MAX_PARCEL_DOCUMENTS} belge, {MAX_RECORDED_PARCELS} parsel ve \
             {MAX_RECORDED_BOUNDARIES} sınır taşır"
        ));
    }
    let mut documents = HashSet::new();
    let mut names = HashSet::new();
    for d in &r.documents {
        if d.number == 0 || !documents.insert(d.number) {
            return Some(format!("{}. parsel belgesinin numarası sıfır ya da yinelenmiş", d.number));
        }
        if !review_label_ok(&d.name, MAX_PARCEL_NAME)
            || !optional_label(&d.kind, MAX_PARCEL_NAME)
            || !optional_label(&d.date, MAX_PARCEL_LABEL)
            || !(d.note.is_empty() || review_text_ok(&d.note) && d.note.chars().count() <= MAX_PARCEL_NOTE)
        {
            return Some(format!("{}. parsel belgesinin adı, türü, tarihi ya da notu geçersiz", d.number));
        }
        if !names.insert(d.name.as_str()) {
            return Some(format!("“{}” adlı parsel belgesi iki kez yazılmış", d.name));
        }
        if let Some(a) = &d.approval
            && let Some(problem) = approval_problem(a)
        {
            return Some(format!("{}. parsel belgesinin onayı geçersiz: {problem}", d.number));
        }
    }
    let known = |n: u32| documents.contains(&n);
    let lifespan = |created: u32, closed: Option<u32>| {
        known(created) && closed.is_none_or(|c| known(c) && c > created)
    };
    let mut boundaries: HashMap<u32, &RecordedBoundary> = HashMap::new();
    for b in &r.boundaries {
        if b.number == 0 || boundaries.insert(b.number, b).is_some() {
            return Some(format!("{}. sınırın numarası sıfır ya da yinelenmiş", b.number));
        }
        if !finite(b.from) || !finite(b.to) || b.from == b.to || !b.bulge.is_finite() {
            return Some(format!("{}. sınırın uçları ya da yayı geçersiz", b.number));
        }
        if (b.from.x, b.from.y) > (b.to.x, b.to.y) {
            return Some(format!("{}. sınırın başı doğu, sonra kuzey sırasıyla küçük uç değil", b.number));
        }
        if !lifespan(b.created_by, b.closed_by) {
            return Some(format!("{}. sınırın belgeleri yok ya da kapatan belge önce gelmiş", b.number));
        }
    }
    let mut numbers = HashSet::new();
    let mut areas = HashSet::new();
    let mut keys = HashSet::new();
    let mut used: HashSet<u32> = HashSet::new();
    let mut used_now: HashSet<u32> = HashSet::new();
    for p in &r.parcels {
        let n = p.number;
        if n == 0 || !numbers.insert(n) {
            return Some(format!("{n}. parsel kaydının numarası sıfır ya da yinelenmiş"));
        }
        if !review_label_ok(&p.ada, MAX_PARCEL_NAME)
            || !review_label_ok(&p.parsel, MAX_PARCEL_NAME)
            || ![&p.il, &p.ilce, &p.mahalle].iter().all(|t| optional_label(t, MAX_PARCEL_NAME))
        {
            return Some(format!("{n}. parsel kaydının kimliği geçersiz: Ada ve Parsel boş olmaz"));
        }
        if !lifespan(p.created_by, p.closed_by) {
            return Some(format!("{n}. parsel kaydının belgeleri yok ya da kapatan belge önce gelmiş"));
        }
        if !(p.stated_area.is_empty() || p.stated_area.len() <= MAX_PARCEL_LABEL && plain_decimal(&p.stated_area)) {
            return Some(format!("{n}. parsel kaydının belgedeki alanı düz bir ondalık sayı değil"));
        }
        let current = p.closed_by.is_none();
        match (&p.uid, current) {
            (Some(uid), true) if review_id_ok(uid) => {
                if !areas.insert(uid.as_str()) {
                    return Some(format!("“{uid}” alanı iki güncel parsel kaydını çiziyor"));
                }
            }
            (None, false) => {}
            _ => return Some(format!("{n}. parsel kaydının alanı yok, geçersiz ya da kapanmışken duruyor")),
        }
        if current && !keys.insert(p.key()) {
            return Some(format!("{} kimlikli iki güncel parsel kaydı var", p.key()));
        }
        if let Some(problem) = rings_problem(n, &p.rings, &boundaries) {
            return Some(problem);
        }
        for e in p.rings.iter().flat_map(|r| &r.edges) {
            let b = e.unsigned_abs();
            used.insert(b);
            if current {
                if boundaries.get(&b).is_some_and(|b| b.closed_by.is_some()) {
                    return Some(format!("{n}. güncel parsel kaydı kapanmış {b}. sınırı kullanıyor"));
                }
                used_now.insert(b);
            }
        }
    }
    for b in &r.boundaries {
        if !used.contains(&b.number) || b.closed_by.is_none() && !used_now.contains(&b.number) {
            return Some(format!("{}. sınırı hiçbir parsel ya da güncelken hiçbir güncel parsel kullanmıyor", b.number));
        }
    }
    if r.lengths.len() > MAX_RECORDED_BOUNDARIES {
        return Some(format!("Parsel kayıtları en çok {MAX_RECORDED_BOUNDARIES} kenar ölçüsü taşır"));
    }
    let mut stated = HashSet::new();
    for l in &r.lengths {
        let Some(b) = boundaries.get(&l.boundary) else {
            return Some(format!("{}. sınırın ölçüsü olmayan bir sınırın", l.boundary));
        };
        if !known(l.document) || l.document < b.created_by {
            return Some(format!("{}. sınırın ölçüsünün belgesi yok ya da sınırdan önce gelmiş", l.boundary));
        }
        if !positive_decimal(&l.length) {
            return Some(format!("{}. sınırın belgedeki ölçüsü sıfırdan büyük düz bir ondalık sayı değil", l.boundary));
        }
        if !stated.insert((l.boundary, l.document)) {
            return Some(format!("{}. sınırın {}. belgedeki ölçüsü iki kez yazılmış", l.boundary, l.document));
        }
    }
    if r.documents.len() + r.drafts.len() > MAX_PARCEL_DOCUMENTS {
        return Some(format!("Parsel kayıtları taslaklarıyla en çok {MAX_PARCEL_DOCUMENTS} belge taşır"));
    }
    let mut drafts = HashSet::new();
    for d in &r.drafts {
        if d.number == 0 || !drafts.insert(d.number) {
            return Some(format!("{}. belge taslağının numarası sıfır ya da yinelenmiş", d.number));
        }
        if !review_label_ok(&d.name, MAX_PARCEL_NAME)
            || !optional_label(&d.kind, MAX_PARCEL_NAME)
            || !optional_label(&d.date, MAX_PARCEL_LABEL)
            || !(d.note.is_empty() || review_text_ok(&d.note) && d.note.chars().count() <= MAX_PARCEL_NOTE)
        {
            return Some(format!("{}. belge taslağının adı, türü, tarihi ya da notu geçersiz", d.number));
        }
        if !names.insert(d.name.as_str()) {
            return Some(format!("“{}” adlı belge ya da taslak iki kez yazılmış", d.name));
        }
        if let Some(problem) = draft_problem(d, &numbers) {
            return Some(format!("{}. belge taslağı geçersiz: {problem}", d.number));
        }
    }
    None
}

/// Whether `text` is a plain decimal above zero (a stated length).
pub fn positive_decimal(text: &str) -> bool {
    text.len() <= MAX_PARCEL_LABEL && plain_decimal(text) && text.bytes().any(|b| (b'1'..=b'9').contains(&b))
}

/// What is wrong with an approval: its number empty or too long, its date or approver too long.
pub fn approval_problem(a: &ParcelApproval) -> Option<&'static str> {
    if !review_label_ok(&a.reference, MAX_APPROVAL_REFERENCE) {
        return Some("onay sayısı boş ya da 64 karakterden uzun");
    }
    if !optional_label(&a.date, MAX_PARCEL_LABEL) || !optional_label(&a.by, MAX_PARCEL_NAME) {
        return Some("onay tarihi ya da onaylayan çok uzun");
    }
    None
}

/// What is wrong with a draft's content: nothing to make or close, a closed number twice or not a recorded parcel, a
/// parcel without Ada and Parsel, an area given twice or not a UUID, a ring of fewer than three vertices, a vertex or
/// bulge not finite, two vertices in one place, bulges not one per edge, parts out of order, a length on a missing edge,
/// twice, or not a plain decimal above zero.
fn draft_problem(d: &ParcelDraft, parcels: &HashSet<u32>) -> Option<&'static str> {
    if d.create.is_empty() && d.close.is_empty() {
        return Some("ne parsel oluşturuyor ne kapatıyor");
    }
    let mut closing = HashSet::new();
    if d.close.iter().any(|n| !parcels.contains(n) || !closing.insert(*n)) {
        return Some("kapatacağı parsel kayıtlarda yok ya da iki kez");
    }
    let mut uids = HashSet::new();
    for p in &d.create {
        if !review_id_ok(&p.uid) || !uids.insert(p.uid.as_str()) {
            return Some("alanın kimliği geçersiz ya da iki kez");
        }
        if !review_label_ok(&p.ada, MAX_PARCEL_NAME)
            || !review_label_ok(&p.parsel, MAX_PARCEL_NAME)
            || ![&p.il, &p.ilce, &p.mahalle].iter().all(|t| optional_label(t, MAX_PARCEL_NAME))
        {
            return Some("parselin kimliği geçersiz");
        }
        if !(p.stated_area.is_empty() || p.stated_area.len() <= MAX_PARCEL_LABEL && plain_decimal(&p.stated_area)) {
            return Some("belgedeki alan düz bir ondalık sayı değil");
        }
        if p.rings.is_empty() {
            return Some("parselin halkası yok");
        }
        let mut part = 0;
        for (i, ring) in p.rings.iter().enumerate() {
            let opens = if i == 0 { ring.part == 0 } else { ring.part == part + 1 };
            if !(opens && !ring.hole || i > 0 && ring.part == part && ring.hole) {
                return Some("parselin parçaları sırasız");
            }
            part = ring.part;
            let n = ring.pts.len();
            if n < 3
                || !(ring.bulges.is_empty() || ring.bulges.len() == n)
                || ring.pts.iter().any(|v| !finite(*v))
                || ring.bulges.iter().any(|b| !b.is_finite())
                || (0..n).any(|k| ring.pts[k] == ring.pts[(k + 1) % n])
            {
                return Some("bir halkası üç köşeden az, sonlu değil ya da aynı yerde iki ardışık köşe taşıyor");
            }
        }
        let mut edges = HashSet::new();
        for l in &p.lengths {
            let ring = p.rings.get(l.ring as usize);
            if ring.is_none_or(|r| l.edge as usize >= r.pts.len()) || !edges.insert((l.ring, l.edge)) {
                return Some("bir kenar ölçüsü olmayan bir kenarın ya da iki kez");
            }
            if !positive_decimal(&l.length) {
                return Some("bir kenar ölçüsü sıfırdan büyük düz bir ondalık sayı değil");
            }
        }
    }
    None
}

/// What is wrong with a parcel's rings: none, a ring of fewer than three boundaries or one that names a missing
/// boundary, a ring whose boundaries do not meet end to end, parts out of order or a part that does not open with its
/// outline.
fn rings_problem(n: u32, rings: &[RecordedRing], by: &HashMap<u32, &RecordedBoundary>) -> Option<String> {
    if rings.is_empty() {
        return Some(format!("{n}. parsel kaydının halkası yok"));
    }
    let mut part = 0;
    for (i, ring) in rings.iter().enumerate() {
        let opens = if i == 0 { ring.part == 0 } else { ring.part == part + 1 };
        let continues = i > 0 && ring.part == part;
        if !(opens && !ring.hole || continues && ring.hole) {
            return Some(format!("{n}. parsel kaydının parçaları sırasız ya da bir parça dış halkasıyla açılmıyor"));
        }
        part = ring.part;
        if ring.edges.len() < 3 || ring.edges.iter().any(|e| *e == 0) {
            return Some(format!("{n}. parsel kaydının bir halkası üç sınırdan az ya da sıfır numaralı sınır taşıyor"));
        }
        let Some((pts, _)) = ring_path_with(by, ring) else {
            return Some(format!("{n}. parsel kaydının bir halkası olmayan bir sınırı kullanıyor"));
        };
        let ends = ring.edges.iter().map(|&e| {
            let b = by[&e.unsigned_abs()];
            if e > 0 { b.to } else { b.from }
        });
        if ends.zip(pts.iter().cycle().skip(1)).any(|(end, next)| end != *next) {
            return Some(format!("{n}. parsel kaydının bir halkasının sınırları uç uca değmiyor"));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    const UID: &str = "0192f5c0-4220-7000-8000-000000000001";

    fn v(x: f64, y: f64) -> Vec2 {
        Vec2 { x, y }
    }

    fn boundary(number: u32, from: Vec2, to: Vec2) -> RecordedBoundary {
        RecordedBoundary { number, from, to, bulge: 0.0, created_by: 1, closed_by: None }
    }

    /// A 10 m square: boundaries 1 south, 2 east, 3 north (run west), 4 west (run south).
    fn square() -> ParcelRegister {
        ParcelRegister {
            documents: vec![ParcelDocument { number: 1, name: "Kadastro 1985".into(), ..Default::default() }],
            boundaries: vec![
                boundary(1, v(0.0, 0.0), v(10.0, 0.0)),
                boundary(2, v(10.0, 0.0), v(10.0, 10.0)),
                boundary(3, v(0.0, 10.0), v(10.0, 10.0)),
                boundary(4, v(0.0, 0.0), v(0.0, 10.0)),
            ],
            parcels: vec![RecordedParcel {
                number: 1,
                ada: "101".into(),
                parsel: "7".into(),
                created_by: 1,
                stated_area: "100".into(),
                uid: Some(UID.into()),
                rings: vec![RecordedRing { part: 0, hole: false, edges: vec![1, 2, -3, -4] }],
                ..Default::default()
            }],
            ..Default::default()
        }
    }

    #[test]
    fn a_square_reads_and_its_ring_runs_its_boundaries() {
        let r = square();
        assert_eq!(parcel_register_problem(&r), None);
        let (pts, bulges) = r.ring_path(&r.parcels[0].rings[0]).unwrap();
        assert_eq!(pts, vec![v(0.0, 0.0), v(10.0, 0.0), v(10.0, 10.0), v(0.0, 10.0)]);
        assert_eq!(bulges, vec![0.0; 4]);
    }

    #[test]
    fn what_a_register_may_not_hold() {
        let broken = |f: &dyn Fn(&mut ParcelRegister)| {
            let mut r = square();
            f(&mut r);
            parcel_register_problem(&r)
        };
        // A ring that does not close, a boundary the wrong way round, a missing identity, a float-like stated area.
        assert!(broken(&|r| r.parcels[0].rings[0].edges = vec![1, 2, 3, -4]).is_some());
        assert!(broken(&|r| {
            let b = &mut r.boundaries[0];
            (b.from, b.to) = (b.to, b.from);
        })
        .is_some());
        assert!(broken(&|r| r.parcels[0].ada.clear()).is_some());
        assert!(broken(&|r| r.parcels[0].stated_area = "1.250,50".into()).is_some());
        // Closed without a later document, closed but still drawn, a current parcel on a closed boundary.
        assert!(broken(&|r| r.parcels[0].closed_by = Some(1)).is_some());
        assert!(broken(&|r| {
            r.documents.push(ParcelDocument { number: 2, name: "Yenileme".into(), ..Default::default() });
            r.parcels[0].closed_by = Some(2);
        })
        .is_some());
        assert!(broken(&|r| {
            r.documents.push(ParcelDocument { number: 2, name: "Yenileme".into(), ..Default::default() });
            r.boundaries[0].closed_by = Some(2);
        })
        .is_some());
        // A boundary nobody uses; two documents of one name.
        assert!(broken(&|r| r.boundaries.push(boundary(5, v(0.0, 0.0), v(5.0, 5.0)))).is_some());
        assert!(broken(&|r| {
            r.documents.push(ParcelDocument { number: 2, name: "Kadastro 1985".into(), ..Default::default() })
        })
        .is_some());
        // Closed properly: the later document closes the parcel and its boundaries, the area goes.
        assert_eq!(
            broken(&|r| {
                r.documents.push(ParcelDocument { number: 2, name: "Yenileme".into(), ..Default::default() });
                r.parcels[0].closed_by = Some(2);
                r.parcels[0].uid = None;
                for b in &mut r.boundaries {
                    b.closed_by = Some(2);
                }
            }),
            None
        );
    }

    #[test]
    fn lengths_drafts_and_approvals_read_and_what_they_may_not_hold() {
        let mut r = square();
        r.lengths.push(RecordedLength { boundary: 1, document: 1, length: "10,00".into() });
        r.documents[0].approval = Some(ParcelApproval { reference: "2026/145".into(), ..Default::default() });
        r.drafts.push(ParcelDraft {
            number: 1,
            name: "2026/146 ifraz".into(),
            close: vec![1],
            create: vec![DraftParcel {
                uid: UID.into(),
                ada: "101".into(),
                parsel: "8".into(),
                rings: vec![DraftRing { pts: vec![v(0.0, 0.0), v(10.0, 0.0), v(0.0, 10.0)], ..Default::default() }],
                lengths: vec![StatedLength { ring: 0, edge: 0, length: "10".into() }],
                ..Default::default()
            }],
            ..Default::default()
        });
        assert_eq!(parcel_register_problem(&r), None);
        assert!(r.has_drafts());
        let broken = |f: &dyn Fn(&mut ParcelRegister)| {
            let mut x = r.clone();
            f(&mut x);
            parcel_register_problem(&x)
        };
        // A length of zero, on a missing boundary, twice; an approval without its number.
        assert!(broken(&|x| x.lengths[0].length = "0,00".into()).is_some());
        assert!(broken(&|x| x.lengths[0].boundary = 9).is_some());
        assert!(broken(&|x| x.lengths.push(x.lengths[0].clone())).is_some());
        assert!(broken(&|x| x.documents[0].approval.as_mut().unwrap().reference.clear()).is_some());
        // A draft named as a document, closing a parcel twice, a length on an edge the ring lacks, two rings in one
        // place.
        assert!(broken(&|x| x.drafts[0].name = "Kadastro 1985".into()).is_some());
        assert!(broken(&|x| x.drafts[0].close = vec![1, 1]).is_some());
        assert!(broken(&|x| x.drafts[0].create[0].lengths[0].edge = 3).is_some());
        assert!(broken(&|x| x.drafts[0].create[0].rings[0].pts[1] = v(0.0, 0.0)).is_some());
    }
}
