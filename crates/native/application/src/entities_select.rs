//! `cad.entities.select` v1 (docs/adr/0275): the selection as one explicit query. A region (a window, a crossing
//! box, a fence, a polygon, a circle, the closed shapes around a point, a click's candidates, every object, objects
//! given, their inverse, the objects like given ones) through the geometry store's queries (`Store::in_rect`,
//! `in_fence`, `in_polygon`, `in_circle`, `containing`, `hits`), the rule every selection tool selects by; the
//! selection filter's kinds and a list of layers after it; only objects shown. The tools ask [`select`] with the
//! store they keep; the command asks it with the host's store, or one of its own. It writes nothing. The shared
//! cases are `fixtures/commands/v1/cad.entities.select.json` (`scripts/fixtures/entities_select_cases.py`).

use std::collections::HashSet;

use kentos_contracts::{
    CommandError, CommandResult, EntitiesSelect, EntitiesSelected, Entity, LayerNodeType, MAX_SELECT_ITEMS,
    PolygonSelectMode, SelectKind, SelectLeftOut, SelectRegion, SimilarCriteria, Vec2 as Point,
};
use kentos_domain::{Document, Slot};
use kentos_geometry_core::geometry::Bounds;
use kentos_geometry_core::store::Store;
use kentos_geometry_core::store::polygon::{PolygonMode, ring_problem};
use kentos_geometry_core::vec2::Vec2;
use kentos_domain::Uuid;

use crate::ExecutionContext;
use crate::checks::{self, Stop, error, is_uid_text};
use crate::codes;

/// What a query gives: the objects in the region's order and what was left out.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Selected {
    pub slots: Vec<Slot>,
    pub left_out: SelectLeftOut,
}

/// Checks `input`.
pub fn validate(cx: &ExecutionContext<'_>, input: &EntitiesSelect) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(()) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// The objects; the same answer as [`execute`].
pub fn plan(cx: &ExecutionContext<'_>, input: &EntitiesSelect) -> CommandResult<EntitiesSelected> {
    answer(cx.doc, cx.store, input)
}

/// The objects. Writes nothing.
pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: EntitiesSelect,
) -> CommandResult<EntitiesSelected> {
    answer(cx.doc, cx.store, &input)
}

/// [`execute`] over the document and the host's store of it, when it has one.
pub fn answer(
    doc: &Document,
    store: Option<&Store>,
    input: &EntitiesSelect,
) -> CommandResult<EntitiesSelected> {
    let own;
    let store = match store {
        Some(s) => s,
        None => {
            own = crate::geometry_store::store_of(doc);
            &own
        }
    };
    match run(doc, store, input) {
        Ok(found) => CommandResult::Completed {
            output: EntitiesSelected {
                uids: found
                    .slots
                    .iter()
                    .filter_map(|s| doc.uid(*s))
                    .map(|u| u.to_string())
                    .collect(),
                left_out: found.left_out,
                revision: doc.revision().to_string(),
            },
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// The query over a store of the document (the tools': the one they keep in step with it). Refused as the command
/// refuses.
pub fn select(
    doc: &Document,
    store: &Store,
    input: &EntitiesSelect,
) -> Result<Selected, Box<CommandError>> {
    run(doc, store, input).map_err(|stop| Box::new(stop.into_error()))
}

fn run(doc: &Document, store: &Store, input: &EntitiesSelect) -> Result<Selected, Stop> {
    check(doc, input)?;
    let mut left_out = SelectLeftOut::default();
    let ids = region(doc, store, &input.region, &mut left_out);
    let kinds: Option<HashSet<&str>> = input
        .kinds
        .as_ref()
        .map(|k| k.iter().map(|k| k.name()).collect());
    let layers: Option<HashSet<&str>> = input
        .layers
        .as_ref()
        .map(|l| l.iter().map(String::as_str).collect());
    let mut slots = Vec::with_capacity(ids.len());
    for slot in ids {
        let Some(e) = doc.get(slot) else {
            continue;
        };
        if let Some(k) = &kinds
            && !k.contains(filter_kind(e))
        {
            left_out.kinds += 1;
            continue;
        }
        if let Some(l) = &layers
            && !l.contains(e.base().layer_id.as_str())
        {
            left_out.layers += 1;
            continue;
        }
        slots.push(slot);
    }
    Ok(Selected { slots, left_out })
}

/// An object's kind as the selection filter knows it: a NURBS curve is an Eğri (docs/adr/0259 §6); a kind the
/// filter does not hold passes only without it.
pub fn filter_kind(e: &Entity) -> &'static str {
    match e.kind() {
        "nurbs" => "spline",
        k => k,
    }
}

/// The selection filter's kinds of a mask of [`SelectKind::ALL`]'s bits (the tools' `Draft::select_kinds`).
pub fn kinds_of_mask(mask: u32) -> Vec<SelectKind> {
    SelectKind::ALL
        .iter()
        .enumerate()
        .filter(|(i, _)| mask & (1 << i) != 0)
        .map(|(_, k)| *k)
        .collect()
}

fn v(p: Point) -> Vec2 {
    Vec2::new(p.x, p.y)
}

fn slots(ids: Vec<f64>) -> Vec<Slot> {
    ids.into_iter()
        .filter(|id| (0.0..=f64::from(u32::MAX)).contains(id))
        .map(|id| Slot(id as u32))
        .collect()
}

/// Whether the store shows the object: its layer visible, a layer's filter and the time window passing it.
fn shown(store: &Store, slot: Slot) -> bool {
    store.get(f64::from(slot.0)).is_some_and(|it| store.shown(it))
}

/// The objects shown, in the drawing's order.
fn every(doc: &Document, store: &Store) -> impl Iterator<Item = Slot> {
    doc.entities()
        .map(|e| Slot(e.base().id))
        .filter(move |s| shown(store, *s))
}

/// The objects the given ids name, in their order, each once (checked by [`check`]).
fn given(doc: &Document, uids: &[String]) -> Vec<Slot> {
    let mut seen = HashSet::new();
    uids.iter()
        .filter_map(|u| Uuid::parse_str(u).ok().and_then(|u| doc.slot_of(u)))
        .filter(|s| seen.insert(*s))
        .collect()
}

fn region(
    doc: &Document,
    store: &Store,
    region: &SelectRegion,
    left_out: &mut SelectLeftOut,
) -> Vec<Slot> {
    match region {
        SelectRegion::Window { a, b } | SelectRegion::Crossing { a, b } => {
            let r = Bounds {
                min_x: a.x.min(b.x),
                min_y: a.y.min(b.y),
                max_x: a.x.max(b.x),
                max_y: a.y.max(b.y),
            };
            slots(store.in_rect(&r, matches!(region, SelectRegion::Crossing { .. })))
        }
        SelectRegion::Fence { points, tolerance } => {
            let fence: Vec<Vec2> = points.iter().copied().map(v).collect();
            slots(store.in_fence(&fence, *tolerance))
        }
        SelectRegion::Polygon { points, mode } => {
            let ring: Vec<Vec2> = points.iter().copied().map(v).collect();
            let mode = match mode {
                PolygonSelectMode::Inside => PolygonMode::Inside,
                PolygonSelectMode::Crossing => PolygonMode::Crossing,
                PolygonSelectMode::Outside => PolygonMode::Outside,
            };
            slots(store.in_polygon(&ring, mode))
        }
        SelectRegion::Circle {
            center,
            radius,
            crossing,
        } => slots(store.in_circle(v(*center), *radius, *crossing)),
        SelectRegion::Containing { at } => {
            slots(store.containing(v(*at)).into_iter().map(|(id, _)| id).collect())
        }
        SelectRegion::Point { at, tolerance } => slots(store.hits(v(*at), *tolerance)),
        SelectRegion::All => every(doc, store).collect(),
        SelectRegion::Objects { uids } => {
            let all = given(doc, uids);
            let n = all.len();
            let out: Vec<Slot> = all.into_iter().filter(|s| shown(store, *s)).collect();
            left_out.hidden = (n - out.len()) as u32;
            out
        }
        SelectRegion::Invert { uids } => {
            let skip: HashSet<Slot> = given(doc, uids).into_iter().collect();
            every(doc, store).filter(|s| !skip.contains(s)).collect()
        }
        SelectRegion::Similar { uids, criteria } => {
            let examples: Vec<Facts> = given(doc, uids)
                .into_iter()
                .filter_map(|s| doc.get(s).map(|e| Facts::of(s, e)))
                .collect();
            let objects: Vec<Facts> = every(doc, store)
                .filter_map(|s| doc.get(s).map(|e| Facts::of(s, e)))
                .collect();
            similar_to(&objects, &examples, criteria.unwrap_or_default())
        }
    }
}

/// The facts the criteria that are on compare, `None` where one is off: the kind and an insert's block, the
/// layer, the colour, the symbol.
type Key<'a> = (
    Option<(&'a str, Option<&'a str>)>,
    Option<&'a str>,
    Option<Option<&'a str>>,
    Option<Option<&'a str>>,
);

/// What an object is compared by (Benzerini seç, docs/adr/0187 §4).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Facts {
    pub id: Slot,
    pub kind: &'static str,
    /// An insert's block, none for any other object.
    pub block: Option<String>,
    pub layer: String,
    pub color: Option<String>,
    pub symbol: Option<String>,
}

impl Facts {
    pub fn of(id: Slot, e: &Entity) -> Facts {
        let b = e.base();
        Facts {
            id,
            kind: e.kind(),
            block: match e {
                Entity::Insert(i) => Some(i.block.to_text()),
                _ => None,
            },
            layer: b.layer_id.clone(),
            color: b.color.clone(),
            symbol: b.symbol.clone(),
        }
    }

    fn key(&self, c: SimilarCriteria) -> Key<'_> {
        (
            c.kind.then_some((self.kind, self.block.as_deref())),
            c.layer.then_some(self.layer.as_str()),
            c.color.then_some(self.color.as_deref()),
            c.symbol.then_some(self.symbol.as_deref()),
        )
    }
}

/// The objects similar to at least one of `examples`, in the objects' order.
pub fn similar_to(objects: &[Facts], examples: &[Facts], c: SimilarCriteria) -> Vec<Slot> {
    let wanted: Vec<_> = examples.iter().map(|e| e.key(c)).collect();
    objects
        .iter()
        .filter(|o| wanted.contains(&o.key(c)))
        .map(|o| o.id)
        .collect()
}

fn check(doc: &Document, input: &EntitiesSelect) -> Result<(), Stop> {
    let finite = |value: f64, what: &str, path: &str| {
        checks::finite(value, what, "Metre olarak bir sayı verin.", path)
    };
    let too_many = |n: usize, what: &str, path: &str| {
        if n > MAX_SELECT_ITEMS {
            Err(Stop::failed(error(
                codes::TOO_MANY_POINTS,
                format!("{what} en çok 100 000 olur; {n} verildi. Bölgeyi ya da listeyi parçalara ayırın."),
                Some(path.into()),
            )))
        } else {
            Ok(())
        }
    };
    let points = |pts: &[Point]| -> Result<(), Stop> {
        too_many(pts.len(), "Bölgenin noktaları", "region.points")?;
        for (i, p) in pts.iter().enumerate() {
            checks::point(*p, &format!("{}. noktanın", i + 1), &format!("region.points[{i}]"))?;
        }
        Ok(())
    };
    let tolerance = |t: f64| -> Result<(), Stop> {
        finite(t, "Tolerans", "region.tolerance")?;
        if t < 0.0 {
            return Err(Stop::failed(error(
                codes::INVALID_DISTANCE,
                "Tolerans sıfırdan küçük olamaz: tıklamanın ya da çitin yakınında aranacak uzaklığı metre olarak verin.".into(),
                Some("region.tolerance".into()),
            )));
        }
        Ok(())
    };
    let uids = |uids: &[String]| -> Result<(), Stop> {
        too_many(uids.len(), "Verilen nesneler", "region.uids")?;
        for (i, uid) in uids.iter().enumerate() {
            let path = Some(format!("region.uids[{i}]"));
            if !is_uid_text(uid) {
                return Err(Stop::failed(error(
                    codes::INVALID_UID,
                    format!("“{uid}” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir."),
                    path,
                )));
            }
            if Uuid::parse_str(uid).ok().and_then(|u| doc.slot_of(u)).is_none() {
                return Err(Stop::failed(error(
                    codes::ENTITY_NOT_FOUND,
                    format!("“{uid}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir."),
                    path,
                )));
            }
        }
        Ok(())
    };
    match &input.region {
        SelectRegion::Window { a, b } | SelectRegion::Crossing { a, b } => {
            checks::point(*a, "Kutunun ilk köşesinin", "region.a")?;
            checks::point(*b, "Kutunun öbür köşesinin", "region.b")?;
        }
        SelectRegion::Fence {
            points: pts,
            tolerance: t,
        } => {
            points(pts)?;
            tolerance(*t)?;
            if pts.len() < 2 {
                return Err(Stop::failed(error(
                    codes::TOO_FEW_POINTS,
                    "Çitin en az iki noktası olmalı.".into(),
                    Some("region.points".into()),
                )));
            }
        }
        SelectRegion::Polygon { points: pts, .. } => {
            points(pts)?;
            if pts.len() < 3 {
                return Err(Stop::failed(error(
                    codes::TOO_FEW_CORNERS,
                    "Seçim çokgeninin en az üç köşesi olmalı.".into(),
                    Some("region.points".into()),
                )));
            }
            let ring: Vec<Vec2> = pts.iter().copied().map(v).collect();
            if let Some(why) = ring_problem(&ring) {
                return Err(Stop::failed(error(
                    codes::INVALID_RING,
                    why.into(),
                    Some("region.points".into()),
                )));
            }
        }
        SelectRegion::Circle { center, radius, .. } => {
            checks::point(*center, "Merkezin", "region.center")?;
            finite(*radius, "Yarıçap", "region.radius")?;
            if *radius <= 0.0 {
                return Err(Stop::failed(error(
                    codes::INVALID_RADIUS,
                    "Seçim dairesinin yarıçapı sıfırdan büyük olmalı.".into(),
                    Some("region.radius".into()),
                )));
            }
        }
        SelectRegion::Containing { at } => checks::point(*at, "Noktanın", "region.at")?,
        SelectRegion::Point { at, tolerance: t } => {
            checks::point(*at, "Tıklamanın", "region.at")?;
            tolerance(*t)?;
        }
        SelectRegion::All => {}
        SelectRegion::Objects { uids: list } | SelectRegion::Invert { uids: list } => uids(list)?,
        SelectRegion::Similar { uids: list, .. } => {
            uids(list)?;
            if list.is_empty() {
                return Err(Stop::failed(error(
                    codes::NO_OBJECTS,
                    "Benzerini seç için en az bir örnek nesnenin kimliğini verin.".into(),
                    Some("region.uids".into()),
                )));
            }
        }
    }
    if let Some(layers) = &input.layers {
        let tree = doc.layers();
        for (i, id) in layers.iter().enumerate() {
            let path = Some(format!("layers[{i}]"));
            match tree.get(id) {
                None => {
                    return Err(Stop::failed(error(
                        codes::LAYER_NOT_FOUND,
                        format!("“{id}” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin."),
                        path,
                    )));
                }
                Some(node) if node.kind != LayerNodeType::Layer => {
                    return Err(Stop::failed(error(
                        codes::NOT_A_LAYER,
                        format!("“{}” bir katman grubu; süzgeç katmanları alır. Grubun katmanlarını verin.", node.name),
                        path,
                    )));
                }
                Some(_) => {}
            }
        }
    }
    Ok(())
}

