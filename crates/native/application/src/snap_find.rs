//! `cad.snap.find` v1 (docs/adr/0272): the object snap at a point read as a command reads it — the kind, the
//! point, the object's persistent id, its place on the object and the piece's path inside an insert — with the
//! drawing's revision. The snap is the geometry store's (`Store::snap_ex`, `snap_candidates`), the rule every
//! tool snaps by; the place is the shared core's (`ops::subref::place_at`, `place_in`), the one the dimension
//! tool binds with. It writes nothing. The shared cases are `fixtures/commands/v1/cad.snap.find.json`
//! (`scripts/fixtures/snap_find_cases.py`).

use kentos_contracts::{
    CommandResult, FoundSnap, SnapFind, SnapFound, SnapKindName, Vec2 as Point,
};
use kentos_domain::dimension_ties::{contract_place, core_blocks};
use kentos_domain::{Document, Slot};
use kentos_geometry_core::ops::subref::{place_at, place_in};
use kentos_geometry_core::store::Store;
use kentos_geometry_core::store::snap::{SnapExtras, SnapHit, SnapKind};
use kentos_geometry_core::vec2::Vec2;

use crate::ExecutionContext;
use crate::blocks::core_entity;
use crate::checks::{self, Stop, error};
use crate::codes;

/// The kinds on at first, as the settings schema has them (all but Yakın and the additions; docs/adr/0163).
const DEFAULT_KINDS: [SnapKind; 9] = [
    SnapKind::Endpoint,
    SnapKind::Midpoint,
    SnapKind::Center,
    SnapKind::Node,
    SnapKind::Quadrant,
    SnapKind::Intersection,
    SnapKind::Perpendicular,
    SnapKind::Tangent,
    SnapKind::Insertion,
];

/// Checks `input`.
pub fn validate(_cx: &ExecutionContext<'_>, input: &SnapFind) -> CommandResult<()> {
    match check(input) {
        Ok(()) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// The snaps; the same answer as [`execute`].
pub fn plan(cx: &ExecutionContext<'_>, input: &SnapFind) -> CommandResult<SnapFound> {
    answer(cx.doc, cx.store, input)
}

/// The snaps. Writes nothing.
pub fn execute(cx: &mut ExecutionContext<'_>, input: SnapFind) -> CommandResult<SnapFound> {
    answer(cx.doc, cx.store, &input)
}

/// [`execute`] over the document and the host's store of it, when it has one.
pub fn answer(doc: &Document, store: Option<&Store>, input: &SnapFind) -> CommandResult<SnapFound> {
    if let Err(stop) = check(input) {
        return stop.into();
    }
    let own;
    let store = match store {
        Some(s) => s,
        None => {
            own = crate::geometry_store::store_of(doc);
            &own
        }
    };
    let kinds = match &input.kinds {
        Some(names) => names.iter().fold(0, |m, k| m | core_kind(*k).bit()),
        None => DEFAULT_KINDS.iter().fold(0, |m, k| m | k.bit()),
    };
    let at = Vec2::new(input.at.x, input.at.y);
    let from = input.from.map(|f| Vec2::new(f.x, f.y));
    let extras = SnapExtras::default();
    let hits = if input.all {
        store.snap_candidates(at, input.aperture, kinds, from, &extras)
    } else {
        store
            .snap_ex(at, input.aperture, kinds, from, &extras)
            .into_iter()
            .collect()
    };
    let snaps: Vec<_> = hits.into_iter().map(|h| found(doc, h)).collect();
    // A point on a PDF underlay's lines is read off paper (docs/adr/0351 §6): said, never passed as CAD geometry.
    let paper = snaps.iter().filter(|s| !s.path.is_empty() && s.uid.as_deref().is_some_and(|u| on_underlay(doc, u))).count();
    let warnings = if paper > 0 {
        vec![kentos_contracts::CommandWarning {
            code: codes::UNDERLAY_MEASURE.into(),
            message: format!(
                "{paper} kenet bir PDF altlığının çizgisinde: yeri kâğıttan okunmuştur (çizimin doğruluğu kâğıdınkidir), kanıtlanmış CAD geometrisi değildir."
            ),
            path: Some("snaps".into()),
        }]
    } else {
        Vec::new()
    };
    CommandResult::Completed {
        output: SnapFound {
            snaps,
            revision: doc.revision().to_string(),
        },
        warnings,
    }
}

/// Whether the object `uid` is an insert of a PDF underlay (an external reference with a page).
pub fn on_underlay(doc: &Document, uid: &str) -> bool {
    kentos_domain::Uuid::parse_str(uid)
        .ok()
        .and_then(|u| doc.slot_of(u))
        .and_then(|s| doc.get(s))
        .and_then(|e| match e {
            kentos_contracts::Entity::Insert(i) => doc.block(i.block),
            _ => None,
        })
        .is_some_and(|b| b.xref.as_ref().is_some_and(|x| x.page.is_some()))
}

fn check(input: &SnapFind) -> Result<(), Stop> {
    checks::point(input.at, "Tıklamanın", "at")?;
    checks::finite(
        input.aperture,
        "Kenet açıklığı",
        "Metre olarak bir sayı verin.",
        "aperture",
    )?;
    if let Some(f) = input.from {
        checks::point(f, "Son noktanın", "from")?;
    }
    if input.aperture <= 0.0 {
        return Err(Stop::failed(error(
            codes::INVALID_APERTURE,
            "Kenet açıklığı sıfırdan büyük olmalı: tıklamanın yakınında aranacak uzaklığı metre olarak verin.".into(),
            Some("aperture".into()),
        )));
    }
    if input.kinds.as_ref().is_some_and(Vec::is_empty) {
        return Err(Stop::failed(error(
            codes::NO_SNAP_KINDS,
            "Hiç kenet türü verilmedi: kinds'i kaldırın (varsayılan türler) ya da en az bir tür verin.".into(),
            Some("kinds".into()),
        )));
    }
    Ok(())
}

fn core_kind(k: SnapKindName) -> SnapKind {
    match k {
        SnapKindName::Endpoint => SnapKind::Endpoint,
        SnapKindName::Midpoint => SnapKind::Midpoint,
        SnapKindName::Center => SnapKind::Center,
        SnapKindName::Node => SnapKind::Node,
        SnapKindName::Quadrant => SnapKind::Quadrant,
        SnapKindName::Intersection => SnapKind::Intersection,
        SnapKindName::Perpendicular => SnapKind::Perpendicular,
        SnapKindName::Tangent => SnapKind::Tangent,
        SnapKindName::Nearest => SnapKind::Nearest,
        SnapKindName::Centroid => SnapKind::Centroid,
        SnapKindName::Insertion => SnapKind::Insertion,
    }
}

/// The wire's name of a kind the store gives; the kinds a command does not ask for never come.
fn kind_name(k: SnapKind) -> SnapKindName {
    match k {
        SnapKind::Endpoint => SnapKindName::Endpoint,
        SnapKind::Midpoint => SnapKindName::Midpoint,
        SnapKind::Center => SnapKindName::Center,
        SnapKind::Node => SnapKindName::Node,
        SnapKind::Quadrant => SnapKindName::Quadrant,
        SnapKind::Intersection => SnapKindName::Intersection,
        SnapKind::Perpendicular => SnapKindName::Perpendicular,
        SnapKind::Tangent => SnapKindName::Tangent,
        SnapKind::Centroid => SnapKindName::Centroid,
        SnapKind::Insertion => SnapKindName::Insertion,
        SnapKind::Nearest | SnapKind::Extension | SnapKind::Parallel | SnapKind::Grid => {
            SnapKindName::Nearest
        }
    }
}

/// A snap as the wire says it: its object's id and place (a crossing and a centroid have no place).
fn found(doc: &Document, h: SnapHit) -> FoundSnap {
    let slot = (0.0..=f64::from(u32::MAX))
        .contains(&h.id)
        .then_some(Slot(h.id as u32));
    let uid = slot.and_then(|s| doc.uid(s)).map(|u| u.to_string());
    let placed = slot.and_then(|s| doc.get(s)).and_then(|e| {
        let core = core_entity(e);
        // An insert's own insertion point is its place; its pieces' points are inside it.
        if matches!(e, kentos_contracts::Entity::Insert(_)) && h.kind != SnapKind::Insertion {
            place_in(&core_blocks(doc.blocks()), &core, h.kind, h.point)
        } else {
            place_at(&core, h.kind, h.point).map(|p| (Vec::new(), p))
        }
    });
    let (path, place) = match placed {
        Some((path, p)) => (path, Some(contract_place(p))),
        None => (Vec::new(), None),
    };
    FoundSnap {
        kind: kind_name(h.kind),
        point: Point {
            x: h.point.x,
            y: h.point.y,
        },
        uid,
        place,
        path,
    }
}
