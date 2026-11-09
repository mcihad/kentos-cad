//! The parcel register a drawing carries (docs/adr/4220; plan/map-cad.md MAP-08), apart from its areas: the documents
//! that made and closed parcels, each parcel as its document made it, and the boundaries the parcels share. After
//! ArcGIS's parcel fabric: a record (a deed, a plan, a survey) creates parcels and retires earlier ones, which stay as
//! history; a parcel's lines are shared with its neighbours and go historic once no current parcel uses them.
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

/// The register: documents, boundaries and parcels, each numbered 1, 2, 3 … in the order they were made.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ParcelRegister {
    pub documents: Vec<ParcelDocument>,
    pub boundaries: Vec<RecordedBoundary>,
    pub parcels: Vec<RecordedParcel>,
}

/// A document that made or closed parcels (ArcGIS's record): its name, unique in the drawing; its kind as the user
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
        self.documents.is_empty() && self.boundaries.is_empty() && self.parcels.is_empty()
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
}
