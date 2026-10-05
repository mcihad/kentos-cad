//! Alan işlemleri: the web's area tools (`apps/web/src/tools/areaTools.ts`)
//! on the desktop's selection-first base ([`crate::modify`]), step for step
//! (docs/adr/0065):
//!
//! - Alan birleştir (tevhit): the selected areas become one, the first
//!   picked lending its layer, colour and data; areas that do not touch stay
//!   apart;
//! - Alan kesiştir: their common part becomes a new area; Kaynakları sil (S)
//!   takes the sources away, and then the part keeps the first one's data;
//! - Alan çıkar: two selections, the areas to cut from, then the areas to
//!   take away; one wholly inside leaves a hole; Çıkarılanları sil (S);
//! - Alan böl: the areas, then a cut line clicked point by point (the pieces
//!   and their areas shown live) or an existing line (Çizgiyle kes, N); the
//!   first piece is the area itself, the others new, with its data;
//! - Alana çevir: closed objects become areas, and line work closing regions
//!   gives an area per region (the lines stay);
//! - Çizgiye çevir: an area becomes closed polylines, its holes ones of
//!   their own;
//! - Parçaları birleştir: the selected areas become one multi-part area in
//!   the first one's place, overlapping ones merged into one part, the parts
//!   from the largest to the smallest; Parçalara ayır: a multi-part area
//!   becomes an area a part, the first keeping its place (docs/adr/0143).
//!   Lines and points join and come apart likewise (`line_parts`, docs/adr/0174).
//!
//! Birleştir, kesiştir and çıkar take a multi-part area whole, as the union
//! of its parts; with Tek nesne (T) their result is one multi-part area
//! rather than an area a piece.
//!
//! Each writes one undo step named after the tool through the product
//! command `cad.entities.edit`, the objects by persistent id and the core's
//! geometry in its input, as the web's do (docs/adr/0069). Objects on locked
//! layers are left out before the command. The area algebra is the shared
//! core's (exact: arcs stay arcs, input corners keep their coordinates).

use kentos_contracts::{
    AreaPart, EditOperation, EntitiesEdited, Entity, EntityEdit, EntityGeometry,
};
use kentos_domain::{Document, Slot, Uuid};
use kentos_geometry_core::entity::{Entity as CoreEntity, Shape, polygon_ring};
use kentos_geometry_core::geom::arrangement::{Area, Source};
use kentos_geometry_core::geom::intersect::Edge;
use kentos_geometry_core::geom::region::{
    FaceIndex, intersect_area_sets, net_area, split_area, subtract_areas, union_areas,
};
use kentos_geometry_core::geometry::dist;
use kentos_geometry_core::ops::areas::{
    area_of_entity, areas_of_entity, line_source, polygon_of_area, polylines_of_polygon,
};
use kentos_geometry_core::ops::parts::one_area;
use kentos_geometry_core::tools::point_text::js_trim;
use kentos_native_application::geometry::{edit_geometry, shape};

use crate::Vec2;
use crate::edge::{self, Outline};
use crate::format::Format;
use crate::line_parts;
use crate::log::Level;
use crate::modify::{Modify, Stages};
use crate::prompt::{Prompt, upper_tr};
use crate::tool::{self, Context, Flow, Memory, Pointer, Preview, Stroke, Tag, Tone};

pub const UNION_ID: &str = "areaUnion";
pub const INTERSECT_ID: &str = "areaIntersect";
pub const SUBTRACT_ID: &str = "areaSubtract";
pub const SPLIT_ID: &str = "areaSplit";
pub const TO_AREA_ID: &str = "toArea";
pub const TO_POLYLINE_ID: &str = "toPolyline";
pub const PARTS_JOIN_ID: &str = "partsJoin";
pub const PARTS_SPLIT_ID: &str = "partsSplit";

/// The kinds that enclose an area, as the messages name them.
const AREA_KINDS: &str = "kapalı alan, daire, elips ya da kapalı eğri";

/// An object that encloses an area, and the area.
#[derive(Clone, Debug)]
struct Picked {
    slot: Slot,
    a: Area,
}

/// The objects at `slots` that enclose an area (the web's `areasOf`).
fn areas_of(slots: &[Slot], doc: &Document) -> Vec<Picked> {
    slots
        .iter()
        .filter_map(|&slot| {
            let a = area_of_entity(&shape(doc.get(slot)?))?;
            Some(Picked { slot, a })
        })
        .collect()
}

/// An object that encloses one area or more: a multi-part area's parts, or
/// the one area of anything else (docs/adr/0143).
#[derive(Clone, Debug)]
struct Whole {
    slot: Slot,
    e: Entity,
    parts: Vec<Area>,
}

/// The objects at `slots` that enclose areas, each with all of them.
fn wholes_of(slots: &[Slot], doc: &Document) -> Vec<Whole> {
    slots
        .iter()
        .filter_map(|&slot| {
            let e = doc.get(slot)?;
            let parts = areas_of_entity(&shape(e));
            (!parts.is_empty()).then(|| Whole {
                slot,
                e: e.clone(),
                parts,
            })
        })
        .collect()
}

/// New areas made from the object `from` names, as `add_areas` makes them;
/// with `one`, one multi-part area of them all (Tek nesne).
fn add_result(areas: &[Area], from: &str, keep_data: bool, one: bool) -> Vec<EntityEdit> {
    if !one || areas.len() < 2 {
        return add_areas(areas, from, keep_data);
    }
    one_area(areas)
        .and_then(|e| edit_geometry(e.shape))
        .map(|geometry| EntityEdit::add(from.to_owned(), geometry, Some(keep_data)))
        .into_iter()
        .collect()
}

/// An option's value as the prompt says it.
fn yes_no(on: bool) -> &'static str {
    if on { "evet" } else { "hayır" }
}

/// The prompt with Tek nesne (T): whether the result is one multi-part area.
fn one_object(prompt: Prompt, memory: &Memory) -> Prompt {
    prompt.option_with("Tek nesne", "T", yes_no(memory.area_one_object))
}

/// An object's parts as the contract has them, each with its size: a
/// polygon's own fields and parts (elevations kept), else the one area it
/// encloses as a polygon.
fn contract_parts(x: &Whole) -> Vec<(f64, AreaPart)> {
    let Entity::Polygon(p) = &x.e else {
        return x
            .parts
            .iter()
            .filter_map(|a| match edit_geometry(polygon_of_area(a).shape)? {
                EntityGeometry::Polygon {
                    pts, bulges, holes, ..
                } => Some((
                    net_area(a),
                    AreaPart {
                        pts,
                        bulges,
                        holes,
                        zs: None,
                    },
                )),
                _ => None,
            })
            .collect();
    };
    let own = AreaPart {
        pts: p.pts.clone(),
        bulges: p.bulges.clone(),
        holes: p.holes.clone(),
        zs: p.zs.clone(),
    };
    std::iter::once(own)
        .chain(p.parts.iter().flatten().cloned())
        .map(|part| {
            let size = area_of_entity(&shape(&Entity::Polygon(kentos_contracts::PathEntity {
                base: p.base.clone(),
                pts: part.pts.clone(),
                bulges: part.bulges.clone(),
                holes: part.holes.clone(),
                zs: None,
                parts: None,
            })))
            .map_or(0.0, |a| net_area(&a));
            (size, part)
        })
        .collect()
}

/// How the result of a tool with Tek nesne is said: “n parçalı tek alan”.
fn one_said(pieces: usize) -> String {
    format!("{pieces} parçalı tek alan")
}

fn total_area(list: &[Area]) -> f64 {
    list.iter().map(net_area).sum()
}

/// New areas made from the object `from` names (`add`): its layer and
/// colour, and with `keep_data` its attributes and label; never its symbol
/// (the web's `addAreas`).
fn add_areas(areas: &[Area], from: &str, keep_data: bool) -> Vec<EntityEdit> {
    areas
        .iter()
        .filter_map(|a| edit_geometry(polygon_of_area(a).shape))
        .map(|geometry| EntityEdit::add(from.to_owned(), geometry, Some(keep_data)))
        .collect()
}

/// The object at `slot` goes (`remove`).
fn removal(slot: Slot, doc: &Document) -> EntityEdit {
    EntityEdit::Remove {
        uid: edge::uid(doc, slot),
    }
}

/// The object at `slot` becomes the area `a` in its place, keeping its
/// data (`replace` with keepData).
fn replacement(slot: Slot, a: &Area, doc: &Document) -> Option<EntityEdit> {
    Some(EntityEdit::Replace {
        uid: edge::uid(doc, slot),
        geometry: edit_geometry(polygon_of_area(a).shape)?,
        keep_data: Some(true),
    })
}

/// Each object followed by the `n` new ones made from it, in order: what a
/// split or a polygon's holes select (the web's `flatMap` over `made`).
fn interleaved(each: &[(Slot, usize)], made: &[Slot]) -> Vec<Slot> {
    let mut out = Vec::with_capacity(each.len() + made.len());
    let mut rest = made.iter();
    for &(slot, n) in each {
        out.push(slot);
        out.extend(rest.by_ref().take(n));
    }
    out
}

/// The slots of the objects an edit made, in its order (the web's `createdIds`).
fn created(out: &EntitiesEdited, doc: &Document) -> Vec<Slot> {
    out.created
        .iter()
        .filter_map(|uid| Uuid::parse_str(uid).ok())
        .filter_map(|uid| doc.slot_of(uid))
        .collect()
}

/// An area's rings as the preview draws them: the outer ring, then the holes.
fn rings(a: &Area) -> Vec<Vec<Vec2>> {
    std::iter::once(&a.outer)
        .chain(&a.holes)
        .map(|r| polygon_ring(&r.pts, r.bulges.as_deref()))
        .collect()
}

/// An area outlined in the accent colour, 2 px (the web's `drawArea(…, { width: 2 })`).
fn outlined(a: &Area) -> tool::Area {
    tool::Area {
        rings: rings(a),
        fill: 0.0,
        width: 2.0,
        dash: None,
        fill_tone: Tone::Accent,
    }
}

/// A straight cut line through the clicked points (the web's `pathSource`).
fn path_source(pts: &[Vec2]) -> Source {
    Source {
        edges: pts
            .windows(2)
            .map(|w| Edge::Seg { a: w[0], b: w[1] })
            .collect(),
        points: Some(pts.to_vec()),
        cut: Some(true),
    }
}

/// The selection's objects not on a locked layer; how many were left out
/// is said with `noun` (“nesne”, “alan”).
fn editable(slots: Vec<Slot>, noun: &str, cx: &mut Context<'_>) -> Vec<Slot> {
    let doc = &*cx.doc;
    let (keep, locked): (Vec<Slot>, Vec<Slot>) = slots.into_iter().partition(|&s| {
        doc.get(s)
            .is_some_and(|e| !doc.layers().is_locked(&e.base().layer_id))
    });
    if !locked.is_empty() {
        cx.say(
            Level::Warn,
            format!(
                "{} {noun} kilitli katmanda olduğu için atlandı.",
                locked.len()
            ),
        );
    }
    keep
}

// ── Birleştir, kesiştir, alana ve çizgiye çevir ─────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Union,
    Intersect,
    ToArea,
    ToPolyline,
    PartsJoin,
    PartsSplit,
}

/// The tools that act on the selection at once (the web's `SelectionActionTool`s).
#[derive(Clone, Debug)]
pub struct AreaAction {
    kind: Kind,
    memory: Memory,
}

impl AreaAction {
    fn tool(kind: Kind) -> Modify<Self> {
        Modify::with(Self {
            kind,
            memory: Memory::default(),
        })
    }

    pub fn union() -> Modify<Self> {
        Self::tool(Kind::Union)
    }

    pub fn intersect() -> Modify<Self> {
        Self::tool(Kind::Intersect)
    }

    pub fn to_area() -> Modify<Self> {
        Self::tool(Kind::ToArea)
    }

    pub fn to_polyline() -> Modify<Self> {
        Self::tool(Kind::ToPolyline)
    }

    pub fn parts_join() -> Modify<Self> {
        Self::tool(Kind::PartsJoin)
    }

    pub fn parts_split() -> Modify<Self> {
        Self::tool(Kind::PartsSplit)
    }

    /// Parçaları birleştir: the areas as one in the first one's place. When
    /// no two overlap, every part is kept as it was, its elevations too;
    /// else the overlapping ones merge into one part (docs/adr/0143).
    fn parts_join_run(&self, targets: &[Slot], cx: &mut Context<'_>) {
        // Lines and points join as their own kind, kinds mixed are refused (docs/adr/0174 §4).
        let kinds = line_parts::kinds(targets, cx.doc);
        if kinds.present() > 1 {
            cx.say(Level::Warn, line_parts::MIXED);
            return;
        }
        match (kinds.lines.len(), kinds.points.len()) {
            (1, _) => return cx.say(Level::Warn, line_parts::ONE_LINE),
            (_, 1) => return cx.say(Level::Warn, line_parts::ONE_POINT),
            (n, _) if n > 1 => return line_parts::join_lines(&kinds.lines, cx),
            (_, n) if n > 1 => return line_parts::join_points(&kinds.points, cx),
            _ => {}
        }
        let list = wholes_of(targets, cx.doc);
        if list.len() < 2 {
            cx.say(
                Level::Warn,
                format!("Parçaları birleştirmek için en az iki alan seçin ({AREA_KINDS})."),
            );
            return;
        }
        let all: Vec<Area> = list.iter().flat_map(|x| x.parts.clone()).collect();
        let merged = union_areas(&all);
        let size = total_area(&all);
        let apart =
            merged.len() == all.len() && (total_area(&merged) - size).abs() <= 1e-9 * size.max(1.0);
        let geometry = if apart {
            // Each part as the contract has it, sized by its area, the largest first.
            let mut parts: Vec<(f64, AreaPart)> = list.iter().flat_map(contract_parts).collect();
            parts.sort_by(|a, b| b.0.total_cmp(&a.0));
            let mut parts = parts.into_iter().map(|(_, p)| p);
            parts.next().map(|first| EntityGeometry::Polygon {
                pts: first.pts,
                bulges: first.bulges,
                holes: first.holes,
                zs: first.zs,
                parts: Some(parts.collect()),
            })
        } else {
            one_area(&merged).and_then(|e| edit_geometry(e.shape))
        };
        let Some(geometry) = geometry else {
            return;
        };
        // The first picked lends its place, id, layer and data; a circle becomes the area.
        let doc = &*cx.doc;
        let first = &list[0];
        let uid = edge::uid(doc, first.slot);
        let mut changes = vec![match first.e {
            Entity::Polygon(_) => EntityEdit::Update { uid, geometry },
            _ => EntityEdit::Replace {
                uid,
                geometry,
                keep_data: Some(true),
            },
        }];
        changes.extend(list[1..].iter().map(|x| removal(x.slot, doc)));
        if edge::write(EditOperation::PartsJoin, changes, cx).is_none() {
            return;
        }
        cx.selection.set(vec![first.slot]);
        let (pieces, total, note) = if apart {
            (all.len(), size, "")
        } else {
            (merged.len(), total_area(&merged), "örtüşenler birleşti, ")
        };
        let total = cx.format().area(total);
        cx.say(
            Level::Success,
            format!(
                "{} alan tek alanda birleşti: {note}{pieces} parça, toplam {total}.",
                list.len()
            ),
        );
    }

    /// Parçalara ayır: each multi-part area an area a part; the first keeps
    /// its place and id, the others are new with its data, each part as it
    /// was (docs/adr/0143); a multi-part polyline and a multi-point object
    /// likewise (docs/adr/0174).
    fn parts_split_run(&self, targets: &[Slot], cx: &mut Context<'_>) {
        let doc = &*cx.doc;
        let areas: Vec<(Slot, &kentos_contracts::PathEntity)> = targets
            .iter()
            .filter_map(|&s| match doc.get(s) {
                Some(Entity::Polygon(p)) if p.parts.as_ref().is_some_and(|q| !q.is_empty()) => {
                    Some((s, p))
                }
                _ => None,
            })
            .collect();
        let others: Vec<(Slot, (Vec<EntityEdit>, usize))> = targets
            .iter()
            .filter_map(|&s| Some((s, line_parts::split(doc.get(s)?, &edge::uid(doc, s))?)))
            .collect();
        if areas.is_empty() && others.is_empty() {
            cx.say(
                Level::Warn,
                "Parçalarına ayrılacak çok parçalı bir nesne seçin: alan, çoklu çizgi ya da çok noktalı nesne.",
            );
            return;
        }
        let mut changes = Vec::new();
        let mut each = Vec::new();
        for (slot, p) in &areas {
            let uid = edge::uid(doc, *slot);
            changes.push(EntityEdit::Update {
                uid: uid.clone(),
                geometry: EntityGeometry::Polygon {
                    pts: p.pts.clone(),
                    bulges: p.bulges.clone(),
                    holes: p.holes.clone(),
                    zs: p.zs.clone(),
                    parts: None,
                },
            });
            let parts = p.parts.iter().flatten();
            changes.extend(parts.map(|part| {
                EntityEdit::add(
                    uid.clone(),
                    EntityGeometry::Polygon {
                        pts: part.pts.clone(),
                        bulges: part.bulges.clone(),
                        holes: part.holes.clone(),
                        zs: part.zs.clone(),
                        parts: None,
                    },
                    Some(true),
                )
            }));
            each.push((*slot, p.parts.as_ref().map_or(0, Vec::len)));
        }
        // The lines' and the points' after the areas', in the selection's order.
        let only_areas = others.is_empty();
        for (slot, (more, n)) in others {
            changes.extend(more);
            each.push((slot, n));
        }
        let n = each.len();
        let Some(out) = edge::write(EditOperation::PartsSplit, changes, cx) else {
            return;
        };
        let made = interleaved(&each, &created(&out, cx.doc));
        let count = made.len();
        cx.selection.set(made);
        let noun = if only_areas { "alan" } else { "nesne" };
        cx.say(
            Level::Success,
            format!("{n} {noun} parçalarına ayrıldı ({count} {noun})."),
        );
    }

    fn union_run(&self, targets: &[Slot], cx: &mut Context<'_>) {
        let list = wholes_of(targets, cx.doc);
        if list.len() < 2 {
            cx.say(
                Level::Warn,
                format!("Birleştirmek için en az iki alan seçin ({AREA_KINDS})."),
            );
            return;
        }
        let all: Vec<Area> = list.iter().flat_map(|x| x.parts.clone()).collect();
        let result = union_areas(&all);
        let one = cx.memory.area_one_object;
        // The first picked area lends its layer, colour and data (tevhit: the parcel kept).
        let doc = &*cx.doc;
        let mut changes: Vec<EntityEdit> = list.iter().map(|x| removal(x.slot, doc)).collect();
        changes.extend(add_result(
            &result,
            &edge::uid(doc, list[0].slot),
            true,
            one,
        ));
        let Some(out) = edge::write(EditOperation::AreaUnion, changes, cx) else {
            return;
        };
        let made = created(&out, cx.doc);
        cx.selection.set(made);
        let parts = if result.len() == 1 {
            "tek alan".to_owned()
        } else if one {
            one_said(result.len())
        } else {
            format!(
                "{} ayrı alan (birbirine değmeyenler ayrı kalır)",
                result.len()
            )
        };
        let total = cx.format().area(total_area(&result));
        cx.say(
            Level::Success,
            format!(
                "{} alan birleştirildi: {parts}, toplam {total}.",
                list.len()
            ),
        );
    }

    fn intersect_run(&self, targets: &[Slot], cx: &mut Context<'_>) {
        let list = wholes_of(targets, cx.doc);
        if list.len() < 2 {
            cx.say(
                Level::Warn,
                format!("Kesiştirmek için en az iki alan seçin ({AREA_KINDS})."),
            );
            return;
        }
        let result = intersect_area_sets(&list.iter().map(|x| x.parts.clone()).collect::<Vec<_>>());
        if result.is_empty() {
            cx.say(Level::Warn, "Seçili alanların ortak bir parçası yok.");
            return;
        }
        let (erase, one) = (cx.memory.area_intersect_erase, cx.memory.area_one_object);
        // Kept sources keep their data; the overlap is a new, blank area. Erased, it takes the first one's data.
        let doc = &*cx.doc;
        let mut changes: Vec<EntityEdit> = if erase {
            list.iter().map(|x| removal(x.slot, doc)).collect()
        } else {
            Vec::new()
        };
        changes.extend(add_result(
            &result,
            &edge::uid(doc, list[0].slot),
            erase,
            one,
        ));
        let Some(out) = edge::write(EditOperation::AreaIntersect, changes, cx) else {
            return;
        };
        let made = created(&out, cx.doc);
        cx.selection.set(made);
        let pieces = match result.len() {
            1 => String::new(),
            n if one => format!(" ({})", one_said(n)),
            n => format!(" ({n} parça)"),
        };
        let erased = if erase { "; kaynaklar silindi" } else { "" };
        let total = cx.format().area(total_area(&result));
        cx.say(
            Level::Success,
            format!("Ortak alan: {total}{pieces}{erased}."),
        );
    }

    fn to_area_run(&self, targets: &[Slot], cx: &mut Context<'_>) {
        let doc = &*cx.doc;
        let not_polygons: Vec<Slot> = targets
            .iter()
            .copied()
            .filter(|&s| !matches!(doc.get(s), Some(Entity::Polygon(_))))
            .collect();
        let closed = areas_of(&not_polygons, doc);
        let lines: Vec<(Slot, Entity)> = targets
            .iter()
            .filter_map(|&s| doc.get(s).map(|e| (s, e.clone())))
            .filter(|(_, e)| {
                area_of_entity(&shape(e)).is_none()
                    && matches!(
                        e,
                        Entity::Line(_)
                            | Entity::Arc(_)
                            | Entity::Polyline(_)
                            | Entity::Spline(_)
                            | Entity::Ellipse(_)
                    )
            })
            .collect();
        let faces = if lines.is_empty() {
            Vec::new()
        } else {
            let work: Vec<CoreEntity> = lines
                .iter()
                .map(|(_, e)| CoreEntity::new(shape(e)))
                .collect();
            FaceIndex::new(&[line_source(&work)]).all()
        };
        if closed.is_empty() && faces.is_empty() {
            let already = targets
                .iter()
                .any(|&s| matches!(doc.get(s), Some(Entity::Polygon(_))));
            cx.say(
                Level::Warn,
                if already {
                    "Seçili nesneler zaten alan."
                } else {
                    "Alana çevrilecek kapalı nesne ya da kapalı bölge oluşturan çizgi bulunamadı. Çizgilerin uçları birleşmeli ya da kesişmeli."
                },
            );
            return;
        }
        // The object itself becomes an area: it keeps its slot and persistent id (docs/adr/0014).
        // Line work stays; the regions it closes become new, blank areas on its layer.
        let mut changes: Vec<EntityEdit> = closed
            .iter()
            .filter_map(|x| replacement(x.slot, &x.a, doc))
            .collect();
        if let Some((first, _)) = lines.first()
            && !faces.is_empty()
        {
            changes.extend(add_areas(&faces, &edge::uid(doc, *first), false));
        }
        let Some(out) = edge::write(EditOperation::ToArea, changes, cx) else {
            return;
        };
        let mut made: Vec<Slot> = closed.iter().map(|x| x.slot).collect();
        made.extend(created(&out, cx.doc));
        cx.selection.set(made);
        let mut parts = Vec::new();
        if !closed.is_empty() {
            parts.push(format!("{} nesne alana çevrildi", closed.len()));
        }
        if !faces.is_empty() {
            parts.push(format!(
                "çizgilerden {} alan oluştu ({})",
                faces.len(),
                cx.format().area(total_area(&faces))
            ));
        }
        cx.say(Level::Success, format!("{}.", parts.join("; ")));
    }

    fn to_polyline_run(&self, targets: &[Slot], cx: &mut Context<'_>) {
        let polys: Vec<(Slot, Entity)> = targets
            .iter()
            .filter_map(|&s| match cx.doc.get(s) {
                Some(e @ Entity::Polygon(_)) => Some((s, e.clone())),
                _ => None,
            })
            .collect();
        if polys.is_empty() {
            cx.say(Level::Warn, "Çizgiye çevrilecek bir kapalı alan seçin.");
            return;
        }
        // The outer ring is the area itself, now a polyline (its slot, persistent id and
        // data kept, docs/adr/0014); holes become new, blank polylines from it.
        let doc = &*cx.doc;
        let mut changes = Vec::new();
        let mut holes = Vec::new();
        for (slot, e) in &polys {
            let Ok(rings) = polylines_of_polygon(&shape(e)) else {
                continue;
            };
            let uid = edge::uid(doc, *slot);
            let mut more = 0;
            for (i, ring) in rings.into_iter().enumerate() {
                let Some(geometry) = edit_geometry(ring.shape) else {
                    continue;
                };
                if i == 0 {
                    changes.push(EntityEdit::Replace {
                        uid: uid.clone(),
                        geometry,
                        keep_data: Some(true),
                    });
                } else {
                    changes.push(EntityEdit::add(uid.clone(), geometry, Some(false)));
                    more += 1;
                }
            }
            holes.push((*slot, more));
        }
        let Some(out) = edge::write(EditOperation::ToPolyline, changes, cx) else {
            return;
        };
        let made = interleaved(&holes, &created(&out, cx.doc));
        let count = made.len();
        cx.selection.set(made);
        cx.say(
            Level::Success,
            format!(
                "{} alan kapalı çoklu çizgiye çevrildi ({count} çizgi).",
                polys.len()
            ),
        );
    }
}

impl Stages for AreaAction {
    fn id(&self) -> &'static str {
        match self.kind {
            Kind::Union => UNION_ID,
            Kind::Intersect => INTERSECT_ID,
            Kind::ToArea => TO_AREA_ID,
            Kind::ToPolyline => TO_POLYLINE_ID,
            Kind::PartsJoin => PARTS_JOIN_ID,
            Kind::PartsSplit => PARTS_SPLIT_ID,
        }
    }

    fn label(&self) -> &'static str {
        match self.kind {
            Kind::Union => "Alan birleştir",
            Kind::Intersect => "Alan kesiştir",
            Kind::ToArea => "Alana çevir",
            Kind::ToPolyline => "Çizgiye çevir",
            Kind::PartsJoin => "Parçaları birleştir",
            Kind::PartsSplit => "Parçalara ayır",
        }
    }

    fn see(&mut self, cx: &Context<'_>) {
        self.memory = *cx.memory;
    }

    /// The selection, locked layers left out, acted on at once; the tool
    /// leaves (the web's `SelectionActionTool.begin`).
    fn begin(&mut self, cx: &mut Context<'_>) -> Flow {
        let targets = editable(cx.selection.ids().to_vec(), "nesne", cx);
        if !targets.is_empty() {
            match self.kind {
                Kind::Union => self.union_run(&targets, cx),
                Kind::Intersect => self.intersect_run(&targets, cx),
                Kind::ToArea => self.to_area_run(&targets, cx),
                Kind::ToPolyline => self.to_polyline_run(&targets, cx),
                Kind::PartsJoin => self.parts_join_run(&targets, cx),
                Kind::PartsSplit => self.parts_split_run(&targets, cx),
            }
        }
        Flow::Exit
    }

    fn picking_hint(&self, prompt: Prompt) -> Prompt {
        match self.kind {
            Kind::Union => one_object(prompt, &self.memory),
            Kind::Intersect => one_object(
                prompt.option_with(
                    "Kaynakları sil",
                    "S",
                    yes_no(self.memory.area_intersect_erase),
                ),
                &self.memory,
            ),
            _ => prompt,
        }
    }

    fn picking_input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        let key = upper_tr(js_trim(text));
        if self.kind == Kind::Intersect && key == "S" {
            cx.memory.area_intersect_erase = !cx.memory.area_intersect_erase;
        } else if matches!(self.kind, Kind::Union | Kind::Intersect) && key == "T" {
            cx.memory.area_one_object = !cx.memory.area_one_object;
        } else {
            return false;
        }
        self.see(cx);
        true
    }

    fn anchor(&self) -> Option<Vec2> {
        None
    }

    fn prompt(&self, _n: usize) -> Prompt {
        Prompt::new(self.label(), "")
    }

    fn point(&mut self, _p: Vec2, _cx: &mut Context<'_>) -> Flow {
        Flow::Stay
    }

    fn typed(&mut self, _text: &str, _cx: &mut Context<'_>) -> Option<Flow> {
        None
    }

    fn preview(&self, _hover: Vec2) -> Option<kentos_geometry_core::geom::affine::Affine> {
        None
    }
}

// ── Çıkar ───────────────────────────────────────────────────────────────

/// Two selections: the areas to cut from, then the areas to take away (the web's `AreaSubtractTool`).
#[derive(Clone, Debug)]
pub struct AreaSubtract {
    from: Option<Vec<Whole>>,
    memory: Memory,
}

impl AreaSubtract {
    pub fn tool() -> Modify<Self> {
        Modify::with(Self {
            from: None,
            memory: Memory::default(),
        })
    }

    fn apply(&self, from: &[Whole], cutters: &[Whole], cx: &mut Context<'_>) {
        let cut: Vec<Area> = cutters.iter().flat_map(|x| x.parts.clone()).collect();
        let (erase, one) = (cx.memory.area_subtract_erase, cx.memory.area_one_object);
        let doc = &*cx.doc;
        let mut changes = Vec::new();
        let (mut changed, mut gone) = (0usize, 0usize);
        for t in from {
            let rest = subtract_areas(&t.parts, &cut);
            // Untouched areas stay as they are (a circle is not turned into a polygon for nothing).
            let size = total_area(&t.parts);
            if rest.len() == t.parts.len()
                && (total_area(&rest) - size).abs() <= 1e-9 * size.max(1.0)
            {
                continue;
            }
            changes.push(removal(t.slot, doc));
            changes.extend(add_result(&rest, &edge::uid(doc, t.slot), true, one));
            changed += 1;
            if rest.is_empty() {
                gone += 1;
            }
        }
        if changed == 0 {
            cx.say(
                Level::Warn,
                "Çıkarılan alanlar kesilecek alanlarla örtüşmüyor; hiçbir alan değişmedi.",
            );
            return;
        }
        // The cutters go too when asked, but not those on a locked layer: they are left out here.
        if erase {
            changes.extend(
                cutters
                    .iter()
                    .filter(|x| !doc.layers().is_locked(&x.e.base().layer_id))
                    .map(|x| removal(x.slot, doc)),
            );
        }
        let Some(out) = edge::write(EditOperation::AreaSubtract, changes, cx) else {
            return;
        };
        let created = created(&out, cx.doc);
        let left: Vec<Area> = created
            .iter()
            .filter_map(|&s| cx.doc.get(s))
            .flat_map(|e| areas_of_entity(&shape(e)))
            .collect();
        cx.selection.set(created);
        let left = cx.format().area(total_area(&left));
        let gone = if gone > 0 {
            format!(", {gone} alan tamamen silindi")
        } else {
            String::new()
        };
        let erased = if erase {
            "; çıkarılan alanlar silindi"
        } else {
            ""
        };
        cx.say(
            Level::Success,
            format!("{changed} alandan çıkarıldı; kalan {left}{gone}{erased}."),
        );
    }
}

impl Stages for AreaSubtract {
    fn id(&self) -> &'static str {
        SUBTRACT_ID
    }

    fn label(&self) -> &'static str {
        "Alan çıkar"
    }

    fn see(&mut self, cx: &Context<'_>) {
        self.memory = *cx.memory;
    }

    /// Each confirmed selection: the areas to cut from, then those to take
    /// away, which act and leave (the web's `begin`).
    fn begin(&mut self, cx: &mut Context<'_>) -> Flow {
        let picked = wholes_of(cx.selection.ids(), cx.doc);
        cx.selection.clear();
        let Some(from) = self.from.clone() else {
            let slots = editable(picked.iter().map(|x| x.slot).collect(), "alan", cx);
            let editable: Vec<Whole> = picked
                .into_iter()
                .filter(|x| slots.contains(&x.slot))
                .collect();
            if editable.is_empty() {
                cx.say(
                    Level::Warn,
                    format!("Önce kesilecek alanları seçin ({AREA_KINDS})."),
                );
            } else {
                self.from = Some(editable);
            }
            return Flow::Stay;
        };
        let cutters: Vec<Whole> = picked
            .into_iter()
            .filter(|x| !from.iter().any(|f| f.slot == x.slot))
            .collect();
        if cutters.is_empty() {
            cx.say(
                Level::Warn,
                format!("Çıkarılacak en az bir alan seçin ({AREA_KINDS})."),
            );
            return Flow::Stay;
        }
        self.apply(&from, &cutters, cx);
        Flow::Exit
    }

    /// Always back to picking: the second selection, or the first again.
    fn repick(&self) -> bool {
        true
    }

    fn picking_prompt(&self, n: usize) -> Option<Prompt> {
        Some(if self.from.is_some() {
            let prompt = Prompt::new(
                "Alan çıkar",
                format!("çıkarılacak alanları seçin, bitince sağ tıklayın ({n} seçili)"),
            )
            .option_with(
                "Çıkarılanları sil",
                "S",
                yes_no(self.memory.area_subtract_erase),
            );
            one_object(prompt, &self.memory)
        } else {
            Prompt::new(
                "Alan çıkar",
                format!("kesilecek alanları seçin, bitince sağ tıklayın ({n} seçili)"),
            )
        })
    }

    fn picking_input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        if self.from.is_none() {
            return false;
        }
        match upper_tr(js_trim(text)).as_str() {
            "S" => cx.memory.area_subtract_erase = !cx.memory.area_subtract_erase,
            "T" => cx.memory.area_one_object = !cx.memory.area_one_object,
            _ => return false,
        }
        self.see(cx);
        true
    }

    /// The areas to cut from, outlined while the others are picked.
    fn picking_preview(&self) -> Preview {
        Preview {
            areas: self
                .from
                .iter()
                .flatten()
                .flat_map(|f| f.parts.iter().map(outlined))
                .collect(),
            ..Preview::default()
        }
    }

    fn anchor(&self) -> Option<Vec2> {
        None
    }

    fn prompt(&self, _n: usize) -> Prompt {
        Prompt::new("Alan çıkar", "")
    }

    fn point(&mut self, _p: Vec2, _cx: &mut Context<'_>) -> Flow {
        Flow::Stay
    }

    fn typed(&mut self, _text: &str, _cx: &mut Context<'_>) -> Option<Flow> {
        None
    }

    fn preview(&self, _hover: Vec2) -> Option<kentos_geometry_core::geom::affine::Affine> {
        None
    }
}

// ── Böl ─────────────────────────────────────────────────────────────────

/// The areas, then the cut line (the web's `AreaSplitTool`).
#[derive(Clone, Debug, Default)]
pub struct AreaSplit {
    list: Vec<Picked>,
    pts: Vec<Vec2>,
    /// Çizgiyle kes: an existing line cuts.
    by_object: bool,
    /// The line under the cursor in Çizgiyle kes.
    cutter: Option<Slot>,
    cutter_shape: Option<Shape>,
}

impl AreaSplit {
    pub fn tool() -> Modify<Self> {
        Modify::with(Self::default())
    }

    /// The pieces the line cuts from the areas, with the area of each (the web's `split`).
    fn split(&mut self, cut: &Source, cutter: Option<Slot>, cx: &mut Context<'_>) -> Flow {
        let doc = &*cx.doc;
        let mut changes = Vec::new();
        // Each split area and how many new pieces come from it, in order.
        let mut split: Vec<(Slot, usize)> = Vec::new();
        let mut sizes = Vec::new();
        for t in &self.list {
            if Some(t.slot) == cutter {
                continue;
            }
            let pieces = split_area(&t.a, cut);
            if pieces.len() < 2 {
                continue;
            }
            // The first piece is the area itself, split: it keeps its slot and
            // persistent id; the rest are new (docs/adr/0014).
            let Some(first) = replacement(t.slot, &pieces[0], doc) else {
                continue;
            };
            changes.push(first);
            changes.extend(add_areas(&pieces[1..], &edge::uid(doc, t.slot), true));
            split.push((t.slot, pieces.len() - 1));
            sizes.extend(pieces.iter().map(net_area));
        }
        let changed = split.len();
        if changed == 0 {
            self.pts.clear();
            cx.say(
                Level::Warn,
                "Kesme çizgisi alanı baştan başa geçmiyor; çizgi alanın sınırını iki yerden kesmeli.",
            );
            return Flow::Stay;
        }
        let Some(out) = edge::write(EditOperation::AreaSplit, changes, cx) else {
            return Flow::Stay;
        };
        let created = interleaved(&split, &created(&out, cx.doc));
        let count = created.len();
        cx.selection.set(created);
        let format = cx.format();
        let list = if sizes.len() <= 4 {
            format!(
                ": {}",
                sizes
                    .iter()
                    .map(|&s| format.area(s))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        } else {
            String::new()
        };
        cx.say(
            Level::Success,
            format!(
                "{changed} alan {count} parçaya bölündü{list}. Parçalar özgün alanın özniteliklerini taşır; parsel numaralarını güncelleyin."
            ),
        );
        Flow::Exit
    }
}

/// Every object's edge can be picked as the cutter (the web's `pickEdge`).
fn any(_: &Entity, _: &Document) -> bool {
    true
}

impl Stages for AreaSplit {
    fn id(&self) -> &'static str {
        SPLIT_ID
    }

    fn label(&self) -> &'static str {
        "Alan böl"
    }

    fn begin(&mut self, cx: &mut Context<'_>) -> Flow {
        let picked = areas_of(cx.selection.ids(), cx.doc);
        let slots = editable(picked.iter().map(|x| x.slot).collect(), "alan", cx);
        cx.selection.clear();
        *self = Self::default();
        self.list = picked
            .into_iter()
            .filter(|x| slots.contains(&x.slot))
            .collect();
        if self.list.is_empty() {
            cx.say(
                Level::Warn,
                format!("Bölünecek bir alan seçin ({AREA_KINDS})."),
            );
        }
        Flow::Stay
    }

    /// No area to cut: picking again.
    fn repick(&self) -> bool {
        self.list.is_empty()
    }

    fn anchor(&self) -> Option<Vec2> {
        if self.by_object {
            None
        } else {
            self.pts.last().copied()
        }
    }

    fn prompt(&self, _n: usize) -> Prompt {
        if self.by_object {
            return Prompt::new(
                "Alan böl",
                "kesici çizgiye tıklayın: çizgi, çoklu çizgi, yay ya da daire",
            )
            .option("Noktalarla kes", "N");
        }
        let step = match self.pts.len() {
            0 => "kesme çizgisinin ilk noktasını gösterin",
            1 => "sonraki noktayı gösterin",
            _ => "sonraki noktayı gösterin ya da bitirmek için sağ tıklayın",
        };
        let prompt = Prompt::new("Alan böl", step);
        let prompt = if self.pts.is_empty() {
            prompt
        } else {
            prompt.option("Geri", "G")
        };
        prompt.option("Çizgiyle kes", "N")
    }

    fn point(&mut self, p: Vec2, _cx: &mut Context<'_>) -> Flow {
        if self.pts.last().is_none_or(|&last| dist(last, p) > 1e-9) {
            self.pts.push(p);
        }
        Flow::Stay
    }

    /// Çizgiyle kes: the line under the cursor is highlighted, a press cuts with it.
    fn pointer(&mut self, p: &Pointer, down: bool, cx: &mut Context<'_>) -> Option<Flow> {
        if !self.by_object {
            return None;
        }
        if !down {
            let hovered = edge::hover(p, cx, any).map(|h| h.slot);
            self.cutter = hovered;
            self.cutter_shape = hovered.and_then(|s| cx.doc.get(s)).map(shape);
            return Some(Flow::Stay);
        }
        let Some((slot, e)) =
            edge::pick(p, cx, any).and_then(|s| cx.doc.get(s).map(|e| (s, e.clone())))
        else {
            cx.say(
                Level::Warn,
                "Kesici olarak bir çizgiye, çoklu çizgiye, yaya ya da daireye tıklayın.",
            );
            return Some(Flow::Stay);
        };
        let cut = line_source(&[CoreEntity::new(shape(&e))]);
        Some(self.split(&cut, Some(slot), cx))
    }

    fn typed(&mut self, text: &str, cx: &mut Context<'_>) -> Option<Flow> {
        match upper_tr(js_trim(text)).as_str() {
            "N" => {
                self.by_object = !self.by_object;
                self.cutter = None;
                self.cutter_shape = None;
                cx.selection.set_hover(None);
                Some(Flow::Stay)
            }
            "G" if !self.pts.is_empty() => {
                self.pts.pop();
                Some(Flow::Stay)
            }
            _ => None,
        }
    }

    /// Two points or more cut; one is not a line; none leaves.
    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow {
        match self.pts.len() {
            0 => Flow::Exit,
            1 => {
                cx.say(Level::Warn, "Kesme çizgisi için en az iki nokta gösterin.");
                Flow::Stay
            }
            _ => {
                let cut = path_source(&self.pts);
                self.split(&cut, None, cx)
            }
        }
    }

    fn preview(&self, _hover: Vec2) -> Option<kentos_geometry_core::geom::affine::Affine> {
        None
    }

    /// The areas outlined; the cut line in the danger colour with the pieces
    /// it would leave and their areas, or the line under the cursor.
    fn stage_preview(&self, hover: Option<Vec2>, format: &Format) -> Option<Preview> {
        let mut preview = Preview {
            areas: self.list.iter().map(|t| outlined(&t.a)).collect(),
            ..Preview::default()
        };
        if self.by_object {
            if let Some(s) = &self.cutter_shape {
                preview
                    .strokes
                    .extend(Outline::of(s, None, 2.0, Tone::Danger).strokes);
            }
            return Some(preview);
        }
        let line: Vec<Vec2> = self.pts.iter().copied().chain(hover).collect();
        if line.len() >= 2 {
            preview.strokes.push(
                Stroke::solid(line.clone(), false)
                    .width(1.5)
                    .tone(Tone::Danger),
            );
        }
        // Live pieces: what the cut would leave, with their areas.
        if let (true, Some(at)) = (line.len() >= 2, hover) {
            let cut = path_source(&line);
            let pieces: Vec<Area> = self
                .list
                .iter()
                .flat_map(|t| {
                    let r = split_area(&t.a, &cut);
                    if r.len() > 1 { r } else { Vec::new() }
                })
                .collect();
            for (i, a) in pieces.iter().enumerate() {
                preview.areas.push(tool::Area {
                    rings: rings(a),
                    fill: 0.18,
                    width: 1.0,
                    dash: Some([4.0, 3.0]),
                    fill_tone: if i % 2 == 1 { Tone::Snap } else { Tone::Accent },
                });
            }
            if !pieces.is_empty() {
                preview.tag = Some(Tag {
                    at,
                    lines: pieces
                        .iter()
                        .take(4)
                        .enumerate()
                        .map(|(i, a)| format!("{}: {}", i + 1, format.area(net_area(a))))
                        .collect(),
                });
            }
        }
        Some(preview)
    }
}
