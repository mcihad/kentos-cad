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
//!   Lines and points join and come apart likewise (docs/adr/0174).
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

use kentos_commands::{AreaWork, AreasEdit, AreasEdited, Entity};
use kentos_domain::{Document, Slot, Uuid};
use kentos_geometry_core::entity::{Shape, polygon_ring};
use kentos_geometry_core::geom::arrangement::{Area, Source};
use kentos_geometry_core::geom::intersect::Edge;
use kentos_geometry_core::geom::region::{net_area, split_area};
use kentos_geometry_core::geometry::dist;
use kentos_geometry_core::ops::areas::{area_of_entity, areas_of_entity};
use kentos_geometry_core::tools::point_text::js_trim;
use kentos_native_application::geometry::shape;
use kentos_native_application::{ExecutionContext, areas};

use crate::Vec2;
use crate::edge::{self, Outline};
use crate::format::Format;
use crate::log::Level;
use crate::modify::{Modify, Stages};
use crate::points;
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
    parts: Vec<Area>,
}

/// The objects at `slots` that enclose areas, each with all of them.
fn wholes_of(slots: &[Slot], doc: &Document) -> Vec<Whole> {
    slots
        .iter()
        .filter_map(|&slot| {
            let e = doc.get(slot)?;
            let parts = areas_of_entity(&shape(e));
            (!parts.is_empty()).then_some(Whole { slot, parts })
        })
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

/// How the result of a tool with Tek nesne is said: “n parçalı tek alan”.
fn one_said(pieces: usize) -> String {
    format!("{pieces} parçalı tek alan")
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

/// A straight cut line through the clicked points, as the preview cuts with it (the command cuts with the same).
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

/// The objects' persistent ids, in order.
fn uids(slots: &[Slot], doc: &Document) -> Vec<String> {
    slots.iter().map(|&s| edge::uid(doc, s)).collect()
}

/// The objects at the persistent ids, in order.
fn slots(uids: &[String], doc: &Document) -> Vec<Slot> {
    uids.iter()
        .filter_map(|u| Uuid::parse_str(u).ok())
        .filter_map(|u| doc.slot_of(u))
        .collect()
}

/// The work through `cad.areas.edit` (docs/adr/0622 §9), one undo step named after the tool; a refusal is said.
pub(crate) fn write_work(work: AreaWork, cx: &mut Context<'_>) -> Option<AreasEdited> {
    let input = AreasEdit {
        work,
        expected_revision: None,
    };
    let result = areas::execute(&mut ExecutionContext::new(cx.doc), input);
    points::written(result, cx)
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

    /// Parçaları birleştir: the areas as one multi-part area in the first one's place, the lines one multi-part
    /// polyline, the points one multi-point object; overlapping areas merge into one part (docs/adr/0143, 0174).
    fn parts_join_run(&self, targets: &[Slot], cx: &mut Context<'_>) {
        let work = AreaWork::PartsJoin {
            ids: uids(targets, cx.doc),
        };
        let Some(out) = write_work(work, cx) else {
            return;
        };
        let first = slots(&out.result, cx.doc);
        let kind = first.first().and_then(|&s| cx.doc.get(s)).map(Entity::kind);
        cx.selection.set(first);
        let (n, pieces) = (out.objects, out.pieces);
        let text = match kind {
            Some("point") => format!("{n} nokta tek nesnede birleşti: {pieces} nokta."),
            Some("polyline") => format!(
                "{n} çizgi tek çoklu çizgide birleşti: {pieces} parça, toplam {}.",
                cx.format().length(out.size)
            ),
            _ => {
                let note = if out.more == 1 {
                    "örtüşenler birleşti, "
                } else {
                    ""
                };
                format!(
                    "{n} alan tek alanda birleşti: {note}{pieces} parça, toplam {}.",
                    cx.format().area(out.size)
                )
            }
        };
        cx.say(Level::Success, text);
    }

    /// Parçalara ayır: each multi-part area an area a part, the first keeping its place and id, the others new with
    /// its data (docs/adr/0143); a multi-part polyline and a multi-point object likewise (docs/adr/0174).
    fn parts_split_run(&self, targets: &[Slot], cx: &mut Context<'_>) {
        let work = AreaWork::PartsSplit {
            ids: uids(targets, cx.doc),
        };
        let Some(out) = write_work(work, cx) else {
            return;
        };
        let made = slots(&out.result, cx.doc);
        let count = made.len();
        cx.selection.set(made);
        let noun = if out.more == 1 { "alan" } else { "nesne" };
        cx.say(
            Level::Success,
            format!(
                "{} {noun} parçalarına ayrıldı ({count} {noun}).",
                out.objects
            ),
        );
    }

    fn union_run(&self, targets: &[Slot], cx: &mut Context<'_>) {
        let one = cx.memory.area_one_object;
        let work = AreaWork::Union {
            ids: uids(targets, cx.doc),
            single: one,
        };
        let Some(out) = write_work(work, cx) else {
            return;
        };
        cx.selection.set(slots(&out.result, cx.doc));
        let parts = if out.pieces == 1 {
            "tek alan".to_owned()
        } else if one {
            one_said(out.pieces as usize)
        } else {
            format!(
                "{} ayrı alan (birbirine değmeyenler ayrı kalır)",
                out.pieces
            )
        };
        let total = cx.format().area(out.size);
        cx.say(
            Level::Success,
            format!(
                "{} alan birleştirildi: {parts}, toplam {total}.",
                out.objects
            ),
        );
    }

    fn intersect_run(&self, targets: &[Slot], cx: &mut Context<'_>) {
        let (erase, one) = (cx.memory.area_intersect_erase, cx.memory.area_one_object);
        let work = AreaWork::Intersect {
            ids: uids(targets, cx.doc),
            erase,
            single: one,
        };
        let Some(out) = write_work(work, cx) else {
            return;
        };
        cx.selection.set(slots(&out.result, cx.doc));
        let pieces = match out.pieces {
            1 => String::new(),
            n if one => format!(" ({})", one_said(n as usize)),
            n => format!(" ({n} parça)"),
        };
        let erased = if erase { "; kaynaklar silindi" } else { "" };
        let total = cx.format().area(out.size);
        cx.say(
            Level::Success,
            format!("Ortak alan: {total}{pieces}{erased}."),
        );
    }

    fn to_area_run(&self, targets: &[Slot], cx: &mut Context<'_>) {
        let work = AreaWork::ToArea {
            ids: uids(targets, cx.doc),
        };
        let Some(out) = write_work(work, cx) else {
            return;
        };
        cx.selection.set(slots(&out.result, cx.doc));
        let mut parts = Vec::new();
        if out.objects > 0 {
            parts.push(format!("{} nesne alana çevrildi", out.objects));
        }
        if out.more > 0 {
            parts.push(format!(
                "çizgilerden {} alan oluştu ({})",
                out.more,
                cx.format().area(out.size)
            ));
        }
        cx.say(Level::Success, format!("{}.", parts.join("; ")));
    }

    fn to_polyline_run(&self, targets: &[Slot], cx: &mut Context<'_>) {
        let work = AreaWork::ToPolyline {
            ids: uids(targets, cx.doc),
        };
        let Some(out) = write_work(work, cx) else {
            return;
        };
        let made = slots(&out.result, cx.doc);
        let count = made.len();
        cx.selection.set(made);
        cx.say(
            Level::Success,
            format!(
                "{} alan kapalı çoklu çizgiye çevrildi ({count} çizgi).",
                out.objects
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
        let (erase, one) = (cx.memory.area_subtract_erase, cx.memory.area_one_object);
        let doc = &*cx.doc;
        let work = AreaWork::Subtract {
            from: from.iter().map(|x| edge::uid(doc, x.slot)).collect(),
            cut: cutters.iter().map(|x| edge::uid(doc, x.slot)).collect(),
            erase,
            single: one,
        };
        let Some(out) = write_work(work, cx) else {
            return;
        };
        cx.selection.set(slots(&out.result, cx.doc));
        let left = cx.format().area(out.size);
        let gone = if out.more > 0 {
            format!(", {} alan tamamen silindi", out.more)
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
            format!(
                "{} alandan çıkarıldı; kalan {left}{gone}{erased}.",
                out.objects
            ),
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

    /// The pieces the line (its points, or the object `cutter`) cuts from the areas, with the area of each.
    fn split(
        &mut self,
        line: Option<Vec<Vec2>>,
        cutter: Option<Slot>,
        cx: &mut Context<'_>,
    ) -> Flow {
        let doc = &*cx.doc;
        let work = AreaWork::Split {
            ids: self.list.iter().map(|t| edge::uid(doc, t.slot)).collect(),
            line: line.map(|pts| pts.into_iter().map(points::wire).collect()),
            cutter: cutter.map(|s| edge::uid(doc, s)),
        };
        let Some(out) = write_work(work, cx) else {
            self.pts.clear();
            return Flow::Stay;
        };
        let created = slots(&out.result, cx.doc);
        let count = created.len();
        cx.selection.set(created);
        let format = cx.format();
        let list = if out.sizes.len() <= 4 {
            format!(
                ": {}",
                out.sizes
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
                "{} alan {count} parçaya bölündü{list}. Parçalar özgün alanın özniteliklerini taşır; parsel numaralarını güncelleyin.",
                out.objects
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
        let _ = e;
        Some(self.split(None, Some(slot), cx))
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
                let line = self.pts.clone();
                self.split(Some(line), None, cx)
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
