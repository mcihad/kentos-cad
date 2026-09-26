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
//!   their own.
//!
//! Each writes one undo step named after the tool, into the document as the
//! web's do (no product command yet). The area algebra is the shared core's
//! (exact: arcs stay arcs, input corners keep their coordinates).

use std::collections::BTreeMap;

use kentos_contracts::{Entity, EntityBase};
use kentos_domain::{Document, Slot, SlotsExhausted};
use kentos_geometry_core::entity::{Entity as CoreEntity, Shape, polygon_ring};
use kentos_geometry_core::geom::arrangement::{Area, Source};
use kentos_geometry_core::geom::intersect::Edge;
use kentos_geometry_core::geom::region::{
    FaceIndex, intersect_areas, net_area, split_area, subtract_areas, union_areas,
};
use kentos_geometry_core::geometry::dist;
use kentos_geometry_core::ops::areas::{
    area_of_entity, line_source, polygon_of_area, polylines_of_polygon,
};
use kentos_geometry_core::tools::point_text::js_trim;
use kentos_native_application::geometry::{edit_geometry, entity_of, shape};

use crate::Vec2;
use crate::edge::{self, Outline};
use crate::format::Format;
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

/// The kinds that enclose an area, as the messages name them.
const AREA_KINDS: &str = "kapalı alan, daire, elips ya da kapalı eğri";

/// An object that encloses an area, and the area.
#[derive(Clone, Debug)]
struct Picked {
    slot: Slot,
    e: Entity,
    a: Area,
}

/// The objects at `slots` that enclose an area (the web's `areasOf`).
fn areas_of(slots: &[Slot], doc: &Document) -> Vec<Picked> {
    slots
        .iter()
        .filter_map(|&slot| {
            let e = doc.get(slot)?;
            let a = area_of_entity(&shape(e))?;
            Some(Picked {
                slot,
                e: e.clone(),
                a,
            })
        })
        .collect()
}

fn total_area(list: &[Area]) -> f64 {
    list.iter().map(net_area).sum()
}

/// A polygon for `a` on the layer of `from`, with its colour and, when
/// `keep_data`, its attributes and label (the web's `areaEntity`).
fn area_entity(a: &Area, from: &Entity, keep_data: bool) -> Option<Entity> {
    let geometry = edit_geometry(polygon_of_area(a).shape)?;
    let base = from.base();
    Some(entity_of(
        &geometry,
        EntityBase {
            id: 0,
            layer_id: base.layer_id.clone(),
            color: base.color.clone(),
            attrs: if keep_data {
                base.attrs.clone()
            } else {
                BTreeMap::new()
            },
            label: if keep_data { base.label.clone() } else { None },
            symbol: None,
        },
    ))
}

/// Adds polygons for `areas` (the web's `addAreas`).
fn add_areas(
    doc: &mut Document,
    areas: &[Area],
    from: &Entity,
    keep_data: bool,
) -> Result<Vec<Slot>, SlotsExhausted> {
    areas
        .iter()
        .filter_map(|a| area_entity(a, from, keep_data))
        .map(|e| doc.add(e))
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

/// Says a failed write: the drawing has no ids left.
fn said(result: Result<Vec<Slot>, SlotsExhausted>, cx: &mut Context<'_>) -> Option<Vec<Slot>> {
    match result {
        Ok(slots) => Some(slots),
        Err(e) => {
            cx.say(Level::Error, e.to_string());
            None
        }
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

    fn union_run(&self, targets: &[Slot], cx: &mut Context<'_>) {
        let list = areas_of(targets, cx.doc);
        if list.len() < 2 {
            cx.say(
                Level::Warn,
                format!("Birleştirmek için en az iki alan seçin ({AREA_KINDS})."),
            );
            return;
        }
        let result = union_areas(&list.iter().map(|x| x.a.clone()).collect::<Vec<_>>());
        let written = cx.doc.transact("Alan birleştir", |doc| {
            doc.remove(&list.iter().map(|x| x.slot).collect::<Vec<_>>());
            // The first picked area lends its layer, colour and data (tevhit: the parcel kept).
            add_areas(doc, &result, &list[0].e, true)
        });
        let Some(created) = said(written, cx) else {
            return;
        };
        cx.selection.set(created);
        let parts = if result.len() == 1 {
            "tek alan".to_owned()
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
        let list = areas_of(targets, cx.doc);
        if list.len() < 2 {
            cx.say(
                Level::Warn,
                format!("Kesiştirmek için en az iki alan seçin ({AREA_KINDS})."),
            );
            return;
        }
        let result = intersect_areas(&list.iter().map(|x| x.a.clone()).collect::<Vec<_>>());
        if result.is_empty() {
            cx.say(Level::Warn, "Seçili alanların ortak bir parçası yok.");
            return;
        }
        let erase = cx.memory.area_intersect_erase;
        let written = cx.doc.transact("Alan kesiştir", |doc| {
            if erase {
                doc.remove(&list.iter().map(|x| x.slot).collect::<Vec<_>>());
            }
            // Kept sources keep their data; the overlap is a new, blank area.
            add_areas(doc, &result, &list[0].e, erase)
        });
        let Some(created) = said(written, cx) else {
            return;
        };
        cx.selection.set(created);
        let pieces = if result.len() > 1 {
            format!(" ({} parça)", result.len())
        } else {
            String::new()
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
        let written = cx.doc.transact("Alana çevir", |doc| {
            let mut created = Vec::new();
            for x in &closed {
                // The object itself becomes an area: it keeps its slot and persistent id (docs/adr/0014).
                if let Some(e) = area_entity(&x.a, &x.e, true) {
                    doc.update(x.slot, e);
                    created.push(x.slot);
                }
            }
            // Line work stays; the regions it closes become new areas on its layer.
            if let Some((_, first)) = lines.first()
                && !faces.is_empty()
            {
                created.extend(add_areas(doc, &faces, first, false)?);
            }
            Ok(created)
        });
        let Some(created) = said(written, cx) else {
            return;
        };
        cx.selection.set(created);
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
        let written = cx.doc.transact("Çizgiye çevir", |doc| {
            let mut created = Vec::new();
            for (slot, e) in &polys {
                let Ok(rings) = polylines_of_polygon(&shape(e)) else {
                    continue;
                };
                for (i, ring) in rings.into_iter().enumerate() {
                    let Some(geometry) = edit_geometry(ring.shape) else {
                        continue;
                    };
                    let base = e.base();
                    let init = entity_of(
                        &geometry,
                        EntityBase {
                            id: 0,
                            layer_id: base.layer_id.clone(),
                            color: base.color.clone(),
                            attrs: if i == 0 {
                                base.attrs.clone()
                            } else {
                                BTreeMap::new()
                            },
                            label: if i == 0 { base.label.clone() } else { None },
                            symbol: None,
                        },
                    );
                    // The outer ring is the area itself, now a polyline (its slot and
                    // persistent id kept, docs/adr/0014); holes become new objects.
                    if i == 0 {
                        doc.update(*slot, init);
                        created.push(*slot);
                    } else {
                        created.push(doc.add(init)?);
                    }
                }
            }
            Ok(created)
        });
        let Some(created) = said(written, cx) else {
            return;
        };
        let count = created.len();
        cx.selection.set(created);
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
        }
    }

    fn label(&self) -> &'static str {
        match self.kind {
            Kind::Union => "Alan birleştir",
            Kind::Intersect => "Alan kesiştir",
            Kind::ToArea => "Alana çevir",
            Kind::ToPolyline => "Çizgiye çevir",
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
            }
        }
        Flow::Exit
    }

    fn picking_hint(&self, prompt: Prompt) -> Prompt {
        match self.kind {
            Kind::Intersect => prompt.option_with(
                "Kaynakları sil",
                "S",
                if self.memory.area_intersect_erase {
                    "evet"
                } else {
                    "hayır"
                },
            ),
            _ => prompt,
        }
    }

    fn picking_input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        if self.kind == Kind::Intersect && upper_tr(js_trim(text)) == "S" {
            cx.memory.area_intersect_erase = !cx.memory.area_intersect_erase;
            self.see(cx);
            return true;
        }
        false
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
    from: Option<Vec<Picked>>,
    memory: Memory,
}

impl AreaSubtract {
    pub fn tool() -> Modify<Self> {
        Modify::with(Self {
            from: None,
            memory: Memory::default(),
        })
    }

    fn apply(&self, from: &[Picked], cutters: &[Picked], cx: &mut Context<'_>) {
        let cut: Vec<Area> = cutters.iter().map(|x| x.a.clone()).collect();
        let erase = cx.memory.area_subtract_erase;
        let written = cx.doc.transact("Alan çıkar", |doc| {
            let mut created = Vec::new();
            let (mut changed, mut gone) = (0usize, 0usize);
            for t in from {
                let rest = subtract_areas(std::slice::from_ref(&t.a), &cut);
                // Untouched areas stay as they are (a circle is not turned into a polygon for nothing).
                let size = net_area(&t.a);
                if rest.len() == 1 && (total_area(&rest) - size).abs() <= 1e-9 * size.max(1.0) {
                    continue;
                }
                doc.remove(&[t.slot]);
                created.extend(add_areas(doc, &rest, &t.e, true)?);
                changed += 1;
                if rest.is_empty() {
                    gone += 1;
                }
            }
            if erase && changed > 0 {
                let unlocked: Vec<Slot> = cutters
                    .iter()
                    .filter(|x| !doc.layers().is_locked(&x.e.base().layer_id))
                    .map(|x| x.slot)
                    .collect();
                doc.remove(&unlocked);
            }
            Ok::<_, SlotsExhausted>((created, changed, gone))
        });
        let (created, changed, gone) = match written {
            Ok(done) => done,
            Err(e) => {
                cx.say(Level::Error, e.to_string());
                return;
            }
        };
        if changed == 0 {
            cx.say(
                Level::Warn,
                "Çıkarılan alanlar kesilecek alanlarla örtüşmüyor; hiçbir alan değişmedi.",
            );
            return;
        }
        let left: Vec<Area> = created
            .iter()
            .filter_map(|&s| cx.doc.get(s).and_then(|e| area_of_entity(&shape(e))))
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
        let picked = areas_of(cx.selection.ids(), cx.doc);
        cx.selection.clear();
        let Some(from) = self.from.clone() else {
            let slots = editable(picked.iter().map(|x| x.slot).collect(), "alan", cx);
            let editable: Vec<Picked> = picked
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
        let cutters: Vec<Picked> = picked
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
            Prompt::new(
                "Alan çıkar",
                format!("çıkarılacak alanları seçin, bitince sağ tıklayın ({n} seçili)"),
            )
            .option_with(
                "Çıkarılanları sil",
                "S",
                if self.memory.area_subtract_erase {
                    "evet"
                } else {
                    "hayır"
                },
            )
        } else {
            Prompt::new(
                "Alan çıkar",
                format!("kesilecek alanları seçin, bitince sağ tıklayın ({n} seçili)"),
            )
        })
    }

    fn picking_input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        if self.from.is_some() && upper_tr(js_trim(text)) == "S" {
            cx.memory.area_subtract_erase = !cx.memory.area_subtract_erase;
            self.see(cx);
            return true;
        }
        false
    }

    /// The areas to cut from, outlined while the others are picked.
    fn picking_preview(&self) -> Preview {
        Preview {
            areas: self
                .from
                .iter()
                .flatten()
                .map(|f| outlined(&f.a))
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
        let list = self.list.clone();
        let written = cx.doc.transact("Alan böl", |doc| {
            let (mut created, mut sizes, mut changed) = (Vec::new(), Vec::new(), 0usize);
            for t in &list {
                if Some(t.slot) == cutter {
                    continue;
                }
                let pieces = split_area(&t.a, cut);
                if pieces.len() < 2 {
                    continue;
                }
                // The first piece is the area itself, split: it keeps its slot and
                // persistent id; the rest are new (docs/adr/0014).
                if let Some(e) = area_entity(&pieces[0], &t.e, true) {
                    doc.update(t.slot, e);
                }
                created.push(t.slot);
                created.extend(add_areas(doc, &pieces[1..], &t.e, true)?);
                sizes.extend(pieces.iter().map(net_area));
                changed += 1;
            }
            Ok::<_, SlotsExhausted>((created, sizes, changed))
        });
        let (created, sizes, changed) = match written {
            Ok(done) => done,
            Err(e) => {
                cx.say(Level::Error, e.to_string());
                return Flow::Stay;
            }
        };
        if changed == 0 {
            self.pts.clear();
            cx.say(
                Level::Warn,
                "Kesme çizgisi alanı baştan başa geçmiyor; çizgi alanın sınırını iki yerden kesmeli.",
            );
            return Flow::Stay;
        }
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
