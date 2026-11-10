//! `cad.parcels.record` v1 and `cad.parcels.records` v1 (plan/map-cad.md MAP-08, docs/adr/4220): Parsel kayıtları.
//! A document written to the drawing's parcel register makes parcels from areas and closes earlier ones (ArcGIS's
//! parcel fabric records); each new parcel's rings become a chain of boundaries, an edge with the same two corners (as
//! stored) and bulge as a current boundary is that boundary, and a boundary no current parcel uses is closed by the
//! document. The areas are never touched: the register keeps what the document said (identity, stated area, shape),
//! the drawing what is drawn now; `list` reads one against the other. The shared cases are
//! `fixtures/commands/v1/cad.parcels.record.json` and `cad.parcels.records.json`.

use std::collections::{BTreeSet, HashMap, HashSet};

use kentos_commands::{
    BOUNDARY_BULGE_TOLERANCE, CommandResult, Entity, MAX_PARCEL_DOCUMENTS, MAX_PARCEL_LABEL, MAX_PARCEL_NAME,
    MAX_RECORDED_PARCELS, ParcelDocument, ParcelDocumentRow, ParcelRegister, ParcelsRecord, ParcelsRecorded,
    ParcelsRecords, ParcelsRecordsListed, PathEntity, RecordedBoundary, RecordedBoundaryRow, RecordedParcel,
    RecordedParcelRow, RecordedRing, Vec2, parcel_register_problem, plain_decimal, review_label_ok, review_text_ok,
};
use kentos_domain::{Document, Uuid};
use kentos_geometry_core::entity::entity_area;
use kentos_geometry_core::geom::bulge::bulge_ring_area;

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;
use crate::geometry::shape;
use crate::parcels_check::RECORDED_FIELD;

/// The undo step's name.
pub const STEP: &str = "Parsel kaydı";

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.to_owned())))
}

/// One ring of an area: its part, whether it is a hole, its vertices and each edge's bulge (0 when none).
type AreaRing = (u32, bool, Vec<Vec2>, Vec<f64>);

/// An area's rings, part by part, each part's outline before its holes.
fn area_rings(a: &PathEntity) -> Vec<AreaRing> {
    let bulged = |pts: &[Vec2], bulges: Option<&Vec<f64>>| -> Vec<f64> {
        (0..pts.len()).map(|k| bulges.and_then(|b| b.get(k)).copied().unwrap_or(0.0)).collect()
    };
    let mut out = vec![(0, false, a.pts.clone(), bulged(&a.pts, a.bulges.as_ref()))];
    for h in a.holes.iter().flatten() {
        out.push((0, true, h.pts.clone(), bulged(&h.pts, h.bulges.as_ref())));
    }
    for (k, part) in a.parts.iter().flatten().enumerate() {
        let n = k as u32 + 1;
        out.push((n, false, part.pts.clone(), bulged(&part.pts, part.bulges.as_ref())));
        for h in part.holes.iter().flatten() {
            out.push((n, true, h.pts.clone(), bulged(&h.pts, h.bulges.as_ref())));
        }
    }
    out
}

/// An edge in a boundary's own direction: the lesser corner (by east, then north) first, the bulge turned with it;
/// and whether the ring runs it the other way.
fn canonical(p: Vec2, q: Vec2, bulge: f64) -> (Vec2, Vec2, f64, bool) {
    if (p.x, p.y) <= (q.x, q.y) { (p, q, bulge, false) } else { (q, p, -bulge, true) }
}

fn corners_key(from: Vec2, to: Vec2) -> [u64; 4] {
    [from.x.to_bits(), from.y.to_bits(), to.x.to_bits(), to.y.to_bits()]
}

/// A trimmed optional text, empty when absent.
fn trimmed(text: Option<&String>) -> String {
    text.map(|t| t.trim().to_owned()).unwrap_or_default()
}

/// A parcel the document makes, before its boundaries are numbered.
struct Made {
    parcel: RecordedParcel,
    rings: Vec<AreaRing>,
}

/// The register after the document and the answer, or why not.
fn run(doc: &Document, input: &ParcelsRecord) -> Result<(ParcelRegister, ParcelsRecorded), Stop> {
    checks::revision(doc, input.expected_revision.as_deref())?;
    let d = &input.document;
    let name = d.name.trim().to_owned();
    if !review_label_ok(&name, MAX_PARCEL_NAME) {
        return Err(fail(
            codes::INVALID_INPUT,
            format!("Belgenin adı boş olmaz; en çok {MAX_PARCEL_NAME} karakterdir, denetim karakteri taşımaz."),
            "document.name",
        ));
    }
    let (kind, date, note) = (trimmed(d.kind.as_ref()), trimmed(d.date.as_ref()), trimmed(d.note.as_ref()));
    for (text, most, path, what) in
        [(&kind, MAX_PARCEL_NAME, "document.kind", "türü"), (&date, MAX_PARCEL_LABEL, "document.date", "tarihi")]
    {
        if !text.is_empty() && !review_label_ok(text, most) {
            return Err(fail(
                codes::INVALID_INPUT,
                format!("Belgenin {what} en çok {most} karakterdir, denetim karakteri taşımaz."),
                path,
            ));
        }
    }
    if !note.is_empty() && !review_text_ok(&note) {
        return Err(fail(
            codes::INVALID_INPUT,
            "Belgenin notu en çok 4000 karakterdir; satır sonu ve sekme dışında denetim karakteri taşımaz.".into(),
            "document.note",
        ));
    }
    if input.create.is_empty() && input.close.is_empty() {
        return Err(fail(
            codes::INVALID_INPUT,
            "Belge ne parsel oluşturuyor ne kapatıyor: oluşturacağı alanları (create) ya da kapatacağı parselleri \
             (close) verin."
                .into(),
            "create",
        ));
    }
    let mut reg = doc.parcel_register().cloned().unwrap_or_default();
    if reg.documents.iter().any(|x| x.name == name) {
        return Err(fail(
            codes::DOCUMENT_NAME_TAKEN,
            format!("Parsel kayıtlarında “{name}” adlı bir belge var; belgenin adı çizimde tektir, başka bir ad verin."),
            "document.name",
        ));
    }
    let mut closing: Vec<u32> = Vec::new();
    for (i, n) in input.close.iter().enumerate() {
        let path = format!("close[{i}]");
        if closing.contains(n) {
            return Err(fail(codes::INVALID_INPUT, format!("{n} numaralı parsel iki kez verildi."), &path));
        }
        let Some(p) = reg.parcel(*n) else {
            return Err(fail(codes::UNKNOWN_PARCEL_RECORD, format!("Parsel kayıtlarında {n} numaralı parsel yok."), &path));
        };
        if let Some(c) = p.closed_by {
            let by = reg.document(c).map(|x| x.name.clone()).unwrap_or_default();
            return Err(fail(
                codes::PARCEL_CLOSED,
                format!("{n} numaralı parsel ({}) “{by}” belgesiyle kapatılmış; ancak güncel bir parsel kapatılır.", p.label()),
                &path,
            ));
        }
        closing.push(*n);
    }
    let mut made: Vec<Made> = Vec::new();
    for (i, item) in input.create.iter().enumerate() {
        let path = format!("create[{i}].uid");
        let uid = item.uid.as_str();
        if !checks::is_uid_text(uid) {
            return Err(fail(
                codes::INVALID_UID,
                format!("“{uid}” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir."),
                &path,
            ));
        }
        let Some(e) = Uuid::parse_str(uid).ok().and_then(|u| doc.slot_of(u)).and_then(|s| doc.get(s)) else {
            return Err(fail(codes::ENTITY_NOT_FOUND, format!("“{uid}” kimlikli nesne çizimde yok."), &path));
        };
        let Entity::Polygon(area) = e else {
            return Err(fail(
                codes::NOT_A_PARCEL,
                format!("“{uid}” bir alan değil; parsel yalnız bir alandan kaydedilir."),
                &path,
            ));
        };
        let attr = |k: &str| e.base().attrs.get(k).map(|v| v.trim().to_owned()).unwrap_or_default();
        let (il, ilce, mahalle, ada, parsel) = (attr("İl"), attr("İlçe"), attr("Mahalle"), attr("Ada"), attr("Parsel"));
        if ada.is_empty() || parsel.is_empty() {
            return Err(fail(
                codes::NO_PARCEL_IDENTITY,
                format!(
                    "“{uid}” alanının Ada ya da Parsel'i yok; parselin kimliği İl, İlçe, Mahalle, Ada ve Parsel \
                     birliktedir, önce yazın."
                ),
                &path,
            ));
        }
        if made.iter().any(|m| m.parcel.uid.as_deref() == Some(uid)) {
            return Err(fail(codes::INVALID_INPUT, format!("“{uid}” alanı iki kez verildi."), &path));
        }
        for part in [&il, &ilce, &mahalle, &ada, &parsel] {
            if !part.is_empty() && !review_label_ok(part, MAX_PARCEL_NAME) {
                return Err(fail(
                    codes::INVALID_INPUT,
                    format!(
                        "“{uid}” alanının kimliğinin bir parçası en çok {MAX_PARCEL_NAME} karakterdir, denetim \
                         karakteri taşımaz."
                    ),
                    &path,
                ));
            }
        }
        let rings = area_rings(area);
        let doubled = rings.iter().any(|(_, _, pts, _)| (0..pts.len()).any(|k| pts[k] == pts[(k + 1) % pts.len()]));
        if doubled {
            return Err(fail(
                codes::INVALID_INPUT,
                format!(
                    "“{uid}” alanının aynı yerde iki ardışık köşesi var; sınır iki ayrı köşe arasındadır, önce \
                     yinelenen köşeyi silin."
                ),
                &path,
            ));
        }
        let area_path = format!("create[{i}].statedArea");
        let stated = match &item.stated_area {
            Some(text) => {
                let text = text.trim().to_owned();
                if !text.is_empty() && !(text.len() <= MAX_PARCEL_LABEL && plain_decimal(&text)) {
                    return Err(fail(
                        codes::INVALID_INPUT,
                        format!(
                            "Belgedeki alan “{text}” düz bir ondalık sayı değil: rakamlar, en çok bir nokta ya da \
                             virgül (118,5 ya da 723.525 gibi)."
                        ),
                        &area_path,
                    ));
                }
                text
            }
            None => {
                let text = attr(RECORDED_FIELD);
                if !text.is_empty() && !(text.len() <= MAX_PARCEL_LABEL && plain_decimal(&text)) {
                    return Err(fail(
                        codes::INVALID_INPUT,
                        format!(
                            "Alanın “{RECORDED_FIELD}” özniteliği “{text}”: düz bir ondalık sayı değil; belgedeki \
                             alanı statedArea ile yazın (belge alan vermiyorsa boş verin)."
                        ),
                        &area_path,
                    ));
                }
                text
            }
        };
        let parcel = RecordedParcel {
            number: 0,
            il,
            ilce,
            mahalle,
            ada,
            parsel,
            created_by: 0,
            closed_by: None,
            stated_area: stated,
            uid: Some(uid.to_owned()),
            rings: Vec::new(),
        };
        if let Some(p) = reg.parcels.iter().find(|p| {
            p.closed_by.is_none() && p.uid.as_deref() == Some(uid) && !closing.contains(&p.number)
        }) {
            return Err(fail(
                codes::PARCEL_RECORDED,
                format!(
                    "“{uid}” alanı {} numaralı güncel parseli ({}) çiziyor; belge önce o parseli kapatmalı (close).",
                    p.number,
                    p.label()
                ),
                &path,
            ));
        }
        let key = parcel.key();
        if let Some(p) = reg.parcels.iter().find(|p| p.closed_by.is_none() && !closing.contains(&p.number) && p.key() == key) {
            return Err(fail(
                codes::PARCEL_IDENTITY_TAKEN,
                format!("{key} kimlikli güncel bir parsel var ({} numaralı); belge önce onu kapatmalı (close).", p.number),
                &path,
            ));
        }
        if made.iter().any(|m| m.parcel.key() == key) {
            return Err(fail(codes::PARCEL_IDENTITY_TAKEN, format!("{key} kimliği bu belgede iki alanda var."), &path));
        }
        made.push(Made { parcel, rings });
    }
    if reg.documents.len() >= MAX_PARCEL_DOCUMENTS || reg.parcels.len() + made.len() > MAX_RECORDED_PARCELS {
        return Err(fail(
            codes::INVALID_INPUT,
            format!("Parsel kayıtları en çok {MAX_PARCEL_DOCUMENTS} belge ve {MAX_RECORDED_PARCELS} parsel taşır."),
            "create",
        ));
    }
    let number = reg.documents.iter().map(|x| x.number).max().unwrap_or(0) + 1;
    reg.documents.push(ParcelDocument { number, name, kind, date, note });

    // The boundaries current before this document, by their corners; a new edge on one of them is that boundary.
    let mut index: HashMap<[u64; 4], Vec<usize>> = HashMap::new();
    for (k, b) in reg.boundaries.iter().enumerate().filter(|(_, b)| b.closed_by.is_none()) {
        index.entry(corners_key(b.from, b.to)).or_default().push(k);
    }
    let earlier = reg.boundaries.len();
    let mut next_boundary = reg.boundaries.iter().map(|b| b.number).max().unwrap_or(0) + 1;
    let mut next_parcel = reg.parcels.iter().map(|p| p.number).max().unwrap_or(0) + 1;
    let mut shared = BTreeSet::new();
    let mut created = Vec::new();
    for m in made {
        let mut parcel = m.parcel;
        for (part, hole, pts, bulges) in m.rings {
            let n = pts.len();
            let mut edges = Vec::with_capacity(n);
            for k in 0..n {
                let (from, to, bulge, reversed) = canonical(pts[k], pts[(k + 1) % n], bulges[k]);
                let key = corners_key(from, to);
                let found = index.get(&key).and_then(|list| {
                    list.iter().copied().find(|&j| (reg.boundaries[j].bulge - bulge).abs() <= BOUNDARY_BULGE_TOLERANCE)
                });
                let at = match found {
                    Some(j) => {
                        if j < earlier {
                            shared.insert(reg.boundaries[j].number);
                        }
                        j
                    }
                    None => {
                        reg.boundaries.push(RecordedBoundary {
                            number: next_boundary,
                            from,
                            to,
                            bulge,
                            created_by: number,
                            closed_by: None,
                        });
                        next_boundary += 1;
                        index.entry(key).or_default().push(reg.boundaries.len() - 1);
                        reg.boundaries.len() - 1
                    }
                };
                let b = reg.boundaries[at].number as i32;
                edges.push(if reversed { -b } else { b });
            }
            parcel.rings.push(RecordedRing { part, hole, edges });
        }
        parcel.number = next_parcel;
        parcel.created_by = number;
        next_parcel += 1;
        created.push(parcel.number);
        reg.parcels.push(parcel);
    }
    let (boundaries_split, neighbours) = split_neighbours(&mut reg, earlier, &closing, &created, &index);
    for p in reg.parcels.iter_mut().filter(|p| closing.contains(&p.number)) {
        p.closed_by = Some(number);
        p.uid = None;
    }
    // A boundary no current parcel uses any more goes with the document (ArcGIS: lines go historic once no adjacent
    // parcel uses them).
    let used_now: HashSet<u32> = reg
        .parcels
        .iter()
        .filter(|p| p.closed_by.is_none())
        .flat_map(|p| p.rings.iter().flat_map(|r| r.edges.iter().map(|e| e.unsigned_abs())))
        .collect();
    let mut boundaries_closed = 0;
    for b in reg.boundaries.iter_mut().filter(|b| b.closed_by.is_none() && !used_now.contains(&b.number)) {
        b.closed_by = Some(number);
        boundaries_closed += 1;
    }
    if let Some(problem) = parcel_register_problem(&reg) {
        return Err(fail(codes::INVALID_INPUT, format!("Parsel kayıtları yazılamıyor: {problem}."), "create"));
    }
    let out = ParcelsRecorded {
        document: number,
        created,
        closed: closing,
        boundaries_created: (reg.boundaries.len() - earlier) as u32,
        boundaries_shared: shared.len() as u32,
        boundaries_closed,
        boundaries_split,
        neighbours,
        revision: doc.revision().to_string(),
    };
    Ok((reg, out))
}

/// A corner lies inside a straight boundary when it is within this of its line and farther than this from both its
/// ends (m): docs/adr/2000's `missingVertex` and docs/adr/2600's inner vertices.
const INSIDE: f64 = 1e-6;

/// Docs/adr/4620 (ArcGIS: a bounding line shared with a current adjacent parcel is split so that it has coincident
/// current lines, the original saved as historic): an earlier current straight boundary that a parcel current before
/// the document, and not closed by it, uses; inside which corners of the parcels the document made lie; and whose
/// pieces between them (from its `from` to its `to`) are current straight boundaries, is split: each such parcel's
/// rings take the pieces in order (each signed by its own direction; reversed and negated where the ring ran the
/// boundary backwards). The old boundary is then used by no current parcel and closes with the document. How many
/// were split, and the parcels whose rings changed.
fn split_neighbours(
    reg: &mut ParcelRegister,
    earlier: usize,
    closing: &[u32],
    created: &[u32],
    index: &HashMap<[u64; 4], Vec<usize>>,
) -> (u32, Vec<u32>) {
    let made: HashSet<u32> = created.iter().copied().collect();
    let shut: HashSet<u32> = closing.iter().copied().collect();
    // The new parcels' corners, each once.
    let mut corners: Vec<Vec2> = Vec::new();
    let by_number: HashMap<u32, usize> = reg.boundaries.iter().enumerate().map(|(k, b)| (b.number, k)).collect();
    for p in reg.parcels.iter().filter(|p| made.contains(&p.number)) {
        for e in p.rings.iter().flat_map(|r| &r.edges) {
            if let Some(&k) = by_number.get(&e.unsigned_abs()) {
                for c in [reg.boundaries[k].from, reg.boundaries[k].to] {
                    if !corners.contains(&c) {
                        corners.push(c);
                    }
                }
            }
        }
    }
    // The boundaries the neighbours (current before the document, not closed by it) use, and who uses each.
    let mut users: HashMap<u32, Vec<usize>> = HashMap::new();
    for (i, p) in reg.parcels.iter().enumerate() {
        if p.closed_by.is_some() || shut.contains(&p.number) || made.contains(&p.number) {
            continue;
        }
        for e in p.rings.iter().flat_map(|r| &r.edges) {
            let list = users.entry(e.unsigned_abs()).or_default();
            if !list.contains(&i) {
                list.push(i);
            }
        }
    }
    let mut splits: Vec<(u32, Vec<i32>, Vec<usize>)> = Vec::new();
    for b in reg.boundaries[..earlier].iter().filter(|b| b.closed_by.is_none() && b.bulge == 0.0) {
        let Some(holders) = users.get(&b.number) else { continue };
        let (dx, dy) = (b.to.x - b.from.x, b.to.y - b.from.y);
        let l = dx.hypot(dy);
        if !(l > 0.0) {
            continue;
        }
        let mut inside: Vec<(f64, Vec2)> = corners
            .iter()
            .filter_map(|&c| {
                let along = ((c.x - b.from.x) * dx + (c.y - b.from.y) * dy) / l;
                let off = ((c.x - b.from.x) * dy - (c.y - b.from.y) * dx).abs() / l;
                (off <= INSIDE && along > INSIDE && l - along > INSIDE).then_some((along, c))
            })
            .collect();
        if inside.is_empty() {
            continue;
        }
        inside.sort_by(|a, b| a.0.total_cmp(&b.0));
        let chain: Vec<Vec2> = std::iter::once(b.from).chain(inside.iter().map(|&(_, c)| c)).chain(std::iter::once(b.to)).collect();
        let pieces: Option<Vec<i32>> = chain
            .windows(2)
            .map(|w| {
                let (from, to, _, reversed) = canonical(w[0], w[1], 0.0);
                index.get(&corners_key(from, to)).and_then(|list| {
                    list.iter()
                        .copied()
                        .find(|&k| reg.boundaries[k].bulge == 0.0 && reg.boundaries[k].closed_by.is_none())
                        .map(|k| {
                            let n = reg.boundaries[k].number as i32;
                            if reversed { -n } else { n }
                        })
                })
            })
            .collect();
        if let Some(pieces) = pieces {
            splits.push((b.number, pieces, holders.clone()));
        }
    }
    let mut neighbours: BTreeSet<u32> = BTreeSet::new();
    for (old, pieces, holders) in &splits {
        let (old, back): (i32, Vec<i32>) = (*old as i32, pieces.iter().rev().map(|e| -e).collect());
        for &i in holders {
            let p = &mut reg.parcels[i];
            for r in &mut p.rings {
                let mut edges = Vec::with_capacity(r.edges.len() + pieces.len());
                for &e in &r.edges {
                    if e == old {
                        edges.extend_from_slice(pieces);
                    } else if e == -old {
                        edges.extend_from_slice(&back);
                    } else {
                        edges.push(e);
                    }
                }
                r.edges = edges;
            }
            neighbours.insert(p.number);
        }
    }
    (splits.len() as u32, neighbours.into_iter().collect())
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &ParcelsRecord) -> CommandResult<()> {
    match run(cx.doc, input) {
        Ok(_) => CommandResult::Completed { output: (), warnings: Vec::new() },
        Err(stop) => stop.into(),
    }
}

/// What execute would write; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &ParcelsRecord) -> CommandResult<ParcelsRecorded> {
    match run(cx.doc, input) {
        Ok((_, out)) => CommandResult::Completed { output: out, warnings: Vec::new() },
        Err(stop) => stop.into(),
    }
}

fn write(cx: &mut ExecutionContext<'_>, input: ParcelsRecord) -> CommandResult<ParcelsRecorded> {
    let (reg, mut out) = match run(cx.doc, &input) {
        Ok(done) => done,
        Err(stop) => return stop.into(),
    };
    cx.doc.set_parcel_register(Some(reg), STEP);
    out.revision = cx.doc.revision().to_string();
    CommandResult::Completed { output: out, warnings: Vec::new() }
}

pub fn execute(cx: &mut ExecutionContext<'_>, input: ParcelsRecord) -> CommandResult<ParcelsRecorded> {
    crate::call::recorded(cx, kentos_commands::CAD_PARCELS_RECORD, kentos_commands::CAD_PARCELS_RECORD_VERSION, input, write)
}

fn core(p: Vec2) -> kentos_geometry_core::Vec2 {
    kentos_geometry_core::Vec2::new(p.x, p.y)
}

/// The area of a parcel's recorded rings: each part's outline less its holes (arcs' segments included).
fn recorded_area(paths: &[(u32, bool, Vec<Vec2>, Vec<f64>)]) -> f64 {
    paths
        .iter()
        .map(|(_, hole, pts, bulges)| {
            let pts: Vec<_> = pts.iter().copied().map(core).collect();
            let a = bulge_ring_area(&pts, Some(bulges.as_slice())).abs();
            if *hole { -a } else { a }
        })
        .sum()
}

/// A boundary's length: the chord's, an arc's c·θ / (2 sin(θ/2)).
fn length(b: &RecordedBoundary) -> f64 {
    let c = (b.to.x - b.from.x).hypot(b.to.y - b.from.y);
    let theta = (4.0 * b.bulge.atan()).abs();
    if theta <= 1e-12 { c } else { c * theta / (2.0 * (theta / 2.0).sin()) }
}

/// Whether one of an area's ring edges is the boundary, either way round.
fn draws(rings: &[AreaRing], b: &RecordedBoundary) -> bool {
    rings.iter().any(|(_, _, pts, bulges)| {
        let n = pts.len();
        (0..n).any(|k| {
            let (p, q, bulge) = (pts[k], pts[(k + 1) % n], bulges[k]);
            (p == b.from && q == b.to && (bulge - b.bulge).abs() <= BOUNDARY_BULGE_TOLERANCE)
                || (p == b.to && q == b.from && (bulge + b.bulge).abs() <= BOUNDARY_BULGE_TOLERANCE)
        })
    })
}

/// Whether an area draws a parcel as recorded: each of its boundaries is an edge of the area (either way round, from any
/// vertex) and the area has no other edge. The vertices' order and the rings' direction do not matter.
fn drawn_as(rings: &[AreaRing], p: &RecordedParcel, by: &HashMap<u32, &RecordedBoundary>) -> bool {
    let edges: usize = rings.iter().map(|r| r.2.len()).sum();
    let recorded: usize = p.rings.iter().map(|r| r.edges.len()).sum();
    edges == recorded
        && p.rings.iter().flat_map(|r| &r.edges).all(|e| by.get(&e.unsigned_abs()).is_some_and(|b| draws(rings, b)))
}

/// The register read against the drawing (`cad.parcels.records`); empty lists without one.
pub fn list(doc: &Document, input: &ParcelsRecords) -> ParcelsRecordsListed {
    let Some(reg) = doc.parcel_register() else { return ParcelsRecordsListed::default() };
    let documents = reg
        .documents
        .iter()
        .map(|d| ParcelDocumentRow {
            number: d.number,
            name: d.name.clone(),
            kind: d.kind.clone(),
            date: d.date.clone(),
            note: d.note.clone(),
            created: reg.parcels.iter().filter(|p| p.created_by == d.number).map(|p| p.number).collect(),
            closed: reg.parcels.iter().filter(|p| p.closed_by == Some(d.number)).map(|p| p.number).collect(),
        })
        .collect();
    // Each parcel's boundaries in ring order, its recorded rings, and its area's rings and geometric area now.
    let by: HashMap<u32, &RecordedBoundary> = reg.boundaries.iter().map(|b| (b.number, b)).collect();
    let mut drawn: HashMap<u32, (Vec<AreaRing>, f64)> = HashMap::new();
    let mut rows = Vec::new();
    for p in &reg.parcels {
        let recorded: Vec<AreaRing> = p
            .rings
            .iter()
            .filter_map(|r| reg.ring_path(r).map(|(pts, bulges)| (r.part, r.hole, pts, bulges)))
            .collect();
        let mut boundaries = Vec::new();
        for e in p.rings.iter().flat_map(|r| &r.edges) {
            let b = e.unsigned_abs();
            if !boundaries.contains(&b) {
                boundaries.push(b);
            }
        }
        let area_now = p.uid.as_deref().and_then(|uid| {
            let e = Uuid::parse_str(uid).ok().and_then(|u| doc.slot_of(u)).and_then(|s| doc.get(s))?;
            let Entity::Polygon(a) = e else { return None };
            Some((area_rings(a), entity_area(&shape(e)).unwrap_or(0.0)))
        });
        let state = match (&p.closed_by, &area_now) {
            (Some(_), _) => "closed",
            (None, None) => "missing",
            (None, Some((rings, _))) if drawn_as(rings, p, &by) => "current",
            (None, Some(_)) => "changed",
        };
        if let Some(now) = &area_now {
            drawn.insert(p.number, now.clone());
        }
        rows.push(RecordedParcelRow {
            number: p.number,
            il: p.il.clone(),
            ilce: p.ilce.clone(),
            mahalle: p.mahalle.clone(),
            ada: p.ada.clone(),
            parsel: p.parsel.clone(),
            created_by: p.created_by,
            closed_by: p.closed_by,
            stated_area: (!p.stated_area.is_empty()).then(|| p.stated_area.clone()),
            recorded_area: recorded_area(&recorded),
            uid: p.uid.clone(),
            area: area_now.map(|(_, a)| a),
            state: state.to_owned(),
            boundaries,
            neighbours: Vec::new(),
            before: reg.parcels.iter().filter(|q| q.closed_by == Some(p.created_by)).map(|q| q.number).collect(),
            after: p
                .closed_by
                .map(|c| reg.parcels.iter().filter(|q| q.created_by == c).map(|q| q.number).collect())
                .unwrap_or_default(),
        });
    }
    // The current parcels that use each boundary; a current parcel's neighbours are those it shares one with.
    let mut users: HashMap<u32, Vec<u32>> = HashMap::new();
    let mut all_users: HashMap<u32, Vec<u32>> = HashMap::new();
    for row in &rows {
        for b in &row.boundaries {
            all_users.entry(*b).or_default().push(row.number);
            if row.closed_by.is_none() {
                users.entry(*b).or_default().push(row.number);
            }
        }
    }
    for row in rows.iter_mut().filter(|r| r.closed_by.is_none()) {
        let mut neighbours: Vec<u32> = row
            .boundaries
            .iter()
            .flat_map(|b| users.get(b).into_iter().flatten().copied())
            .filter(|n| *n != row.number)
            .collect();
        neighbours.sort_unstable();
        neighbours.dedup();
        row.neighbours = neighbours;
    }
    let boundaries: Vec<RecordedBoundaryRow> = reg
        .boundaries
        .iter()
        .map(|b| {
            let current = b.closed_by.is_none();
            let parcels = if current { users.get(&b.number) } else { all_users.get(&b.number) }.cloned().unwrap_or_default();
            let is_drawn = current
                && parcels.iter().all(|n| drawn.get(n).is_some_and(|(rings, _)| draws(rings, b)));
            RecordedBoundaryRow {
                number: b.number,
                from: [b.from.x, b.from.y],
                to: [b.to.x, b.to.y],
                bulge: b.bulge,
                length: length(b),
                created_by: b.created_by,
                closed_by: b.closed_by,
                parcels,
                drawn: is_drawn,
            }
        })
        .collect();
    let count = |f: &dyn Fn(&RecordedParcelRow) -> bool| rows.iter().filter(|r| f(r)).count() as u32;
    let moved = boundaries.iter().filter(|b| b.closed_by.is_none() && !b.drawn).count() as u32;
    ParcelsRecordsListed {
        current: count(&|r| r.closed_by.is_none()),
        closed: count(&|r| r.closed_by.is_some()),
        changed: count(&|r| r.state == "changed"),
        missing: count(&|r| r.state == "missing"),
        moved,
        documents,
        parcels: rows.into_iter().filter(|r| input.history || r.closed_by.is_none()).collect(),
        boundaries: boundaries.into_iter().filter(|b| input.history || b.closed_by.is_none()).collect(),
    }
}

pub mod list_cmd {
    //! `cad.parcels.records` v1's handlers: a query, the same answer for each stage.
    use super::*;

    pub fn validate(_cx: &ExecutionContext<'_>, _input: &ParcelsRecords) -> CommandResult<()> {
        CommandResult::Completed { output: (), warnings: Vec::new() }
    }

    pub fn plan(cx: &ExecutionContext<'_>, input: &ParcelsRecords) -> CommandResult<ParcelsRecordsListed> {
        CommandResult::Completed { output: list(cx.doc, input), warnings: Vec::new() }
    }

    pub fn execute(cx: &mut ExecutionContext<'_>, input: ParcelsRecords) -> CommandResult<ParcelsRecordsListed> {
        CommandResult::Completed { output: list(cx.doc, &input), warnings: Vec::new() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(x: f64, y: f64) -> Vec2 {
        Vec2 { x, y }
    }

    #[test]
    fn an_edge_runs_its_boundary_from_the_lesser_corner() {
        assert_eq!(canonical(v(10.0, 0.0), v(0.0, 0.0), 0.25), (v(0.0, 0.0), v(10.0, 0.0), -0.25, true));
        assert_eq!(canonical(v(0.0, 5.0), v(0.0, 9.0), 0.0), (v(0.0, 5.0), v(0.0, 9.0), 0.0, false));
    }

    #[test]
    fn a_semicircle_s_length_is_half_its_circle() {
        let b = RecordedBoundary { number: 1, from: v(0.0, 0.0), to: v(20.0, 0.0), bulge: 1.0, created_by: 1, closed_by: None };
        assert!((length(&b) - std::f64::consts::PI * 10.0).abs() < 1e-12);
    }
}
