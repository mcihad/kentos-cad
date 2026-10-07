//! Plan yolu çizimi (docs/adr/0198): the web's `PlanRoadTool`,
//! `RoadJunctionsTool` and `MedianCloseTool` (`apps/web/src/tools/planRoadTools.ts`).
//!
//! - **Plan yolu** (`planRoad`): the axis clicked (snapping) or typed; Enter
//!   or a right click ends it. The road is an area `Genişlik` wide around
//!   it; a Yol's kerbs (Kaldırım) leave a carriageway area inside, its median
//!   (Refüj) an area closed with half circles; Eksen writes the axis too. All
//!   on the active layer with their `Tür` and `Genişlik` attributes, one step
//!   “Plan yolu” through `cad.entities.create`.
//! - **Kavşak temizle** (`roadJunctions`), on the modify tools' base:
//!   selection first; the selected areas joined kind by kind (`Tür`; Refüj
//!   and Yol ekseni left out) and their inner corners rounded, Ada köşesi (A)
//!   for roads, Kaldırım köşesi (K) for carriageways; the result previewed
//!   dashed, Enter writes it in one step “Kavşak temizle” through
//!   `cad.entities.edit`.
//! - **Refüj kapat** (`medianClose`): two lines picked, closed into a median
//!   with half circles (Uç (U): Düz straight); the first becomes the area, the
//!   second goes, one step “Refüj kapat”.
//!
//! The kinds' widths, Kaldırım, Refüj, Eksen, the radii and Uç stay for as
//! long as the app lives ([`crate::tool::Memory`]). The geometry is the
//! shared core's (`ops::road`).

use std::collections::BTreeMap;

use kentos_contracts::{CreateOperation, EditOperation, EntityEdit, EntityGeometry};
use kentos_domain::Slot;
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::geom::arrangement::{Area, Ring};
use kentos_geometry_core::geom::bulge::bulge_path_outline;
use kentos_geometry_core::geom::parallel::clean_axis;
use kentos_geometry_core::geometry::dist;
use kentos_geometry_core::jsmath::TAU;
use kentos_geometry_core::ops::areas::areas_of_entity;
use kentos_geometry_core::ops::parts::one_area;
use kentos_geometry_core::ops::road::{RoadParts, median_ring, road_junctions, road_parts};
use kentos_geometry_core::tools::point_text::js_trim;
use kentos_native_application::geometry::{edit_geometry, shape};

use crate::Vec2;
use crate::edge;
use crate::format::{Format, js_number};
use crate::log::Level;
use crate::modify::{Modify, Stages};
use crate::points::{self, Taken, plain_number, wire_all};
use crate::prompt::{Prompt, upper_tr};
use crate::tool::{
    Context, Cursor, Flow, Memory, OptionChoice, Pointer, Preview, Stroke, Tag, Tool,
};

/// Plan yolu's id: its command is `tool.planRoad`.
pub const ROAD_ID: &str = "planRoad";
pub const ROAD_LABEL: &str = "Plan yolu";
/// Kavşak temizle's id: its command is `tool.roadJunctions`.
pub const JUNCTIONS_ID: &str = "roadJunctions";
pub const JUNCTIONS_LABEL: &str = "Kavşak temizle";
/// Refüj kapat's id: its command is `tool.medianClose`.
pub const MEDIAN_ID: &str = "medianClose";
pub const MEDIAN_LABEL: &str = "Refüj kapat";

/// The attribute naming a road's kind, and its width's (metres, as a number's text).
const KIND: &str = "Tür";
const WIDTH: &str = "Genişlik";
/// Plan yolu's kinds, by `Memory::road_kind` (§2).
pub const ROAD_KINDS: [&str; 3] = ["Yol", "Yaya yolu", "Bisiklet yolu"];
/// The kinds Kavşak temizle leaves out (§3), and the one whose corners are kerb corners.
const NOT_JOINED: [&str; 2] = ["Refüj", "Yol ekseni"];
const CARRIAGEWAY: &str = "Taşıt yolu";
/// How finely a preview draws an arc.
const SEGMENTS: f64 = 96.0;

/// The areas as one object's geometry, the largest first (the web's `oneArea`).
fn area_geometry(areas: &[Area]) -> Option<EntityGeometry> {
    one_area(areas).and_then(|e| edit_geometry(e.shape))
}

/// A ring's outline, its arcs drawn as the preview draws them.
fn ring_points(r: &Ring) -> Vec<Vec2> {
    bulge_path_outline(&r.pts, r.bulges.as_deref(), true, TAU / SEGMENTS)
}

/// An area's rings, each drawn by `stroke`.
fn area_strokes(a: &Area, stroke: impl Fn(Vec<Vec2>) -> Stroke) -> Vec<Stroke> {
    std::iter::once(&a.outer)
        .chain(&a.holes)
        .map(|r| stroke(ring_points(r)))
        .collect()
}

fn attrs(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
        .collect()
}

/// A path's length.
fn length(pts: &[Vec2]) -> f64 {
    pts.windows(2).map(|w| dist(w[0], w[1])).sum()
}

// ── Plan yolu ───────────────────────────────────────────────────────────

/// A value Plan yolu asks for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Asking {
    Width,
    Kerb,
    Median,
}

/// Plan yolu.
#[derive(Clone, Debug, Default)]
pub struct PlanRoad {
    d: Taken,
    asking: Option<Asking>,
    seen: Option<(Memory, Format)>,
}

impl PlanRoad {
    pub fn new() -> Self {
        Self::default()
    }

    fn see(&mut self, cx: &Context<'_>) {
        self.seen = Some((*cx.memory, cx.format()));
    }

    fn memory(&self) -> Memory {
        self.seen.map(|(m, _)| m).unwrap_or_default()
    }

    fn format(&self) -> Format {
        self.seen.map(|(_, f)| f).unwrap_or_default()
    }

    fn kind(&self) -> usize {
        usize::from(self.memory().road_kind).min(ROAD_KINDS.len() - 1)
    }

    fn is_road(&self) -> bool {
        self.kind() == 0
    }

    fn width(&self) -> f64 {
        self.memory().road_widths[self.kind()]
    }

    /// The kerbs and the median the kind takes (only a Yol has them).
    fn kerb_median(&self) -> (f64, f64) {
        let m = self.memory();
        if self.is_road() {
            (m.road_kerb, m.road_median)
        } else {
            (0.0, 0.0)
        }
    }

    /// Why the widths cannot make the road, or none.
    fn misfit(&self) -> Option<String> {
        if !self.is_road() {
            return None;
        }
        let f = self.format();
        let (w, (k, md)) = (self.width(), self.kerb_median());
        if 2.0 * k >= w {
            return Some(format!(
                "Kaldırımlar (2 × {}) yolun genişliğine ({}) sığmıyor; Kaldırım'ı ya da Genişlik'i değiştirin.",
                f.length(k),
                f.length(w)
            ));
        }
        if md >= w - 2.0 * k {
            return Some(format!(
                "Refüj ({}) taşıt yoluna ({}) sığmıyor; Refüj'ü ya da Genişlik'i değiştirin.",
                f.length(md),
                f.length(w - 2.0 * k)
            ));
        }
        None
    }

    /// The road's areas along `axis` at the kept values.
    fn parts(&self, axis: &[Vec2]) -> Option<RoadParts> {
        if self.misfit().is_some() {
            return None;
        }
        let (k, md) = self.kerb_median();
        road_parts(axis, self.width(), k, md)
    }

    fn options(&self, prompt: Prompt) -> Prompt {
        let (m, f) = (self.memory(), self.format());
        let shown = |v: f64| {
            if v > 0.0 {
                f.length(v)
            } else {
                "yok".to_owned()
            }
        };
        let prompt = prompt
            .option_with("Tür", "T", ROAD_KINDS[self.kind()])
            .option_with("Genişlik", "G", f.length(self.width()));
        let prompt = if self.is_road() {
            prompt
                .option_with("Kaldırım", "K", shown(m.road_kerb))
                .option_with("Refüj", "R", shown(m.road_median))
        } else {
            prompt
        };
        prompt.toggle("Eksen", "E", m.road_axis)
    }

    /// The road along the points in, written in one step; the points start over.
    fn write(&mut self, cx: &mut Context<'_>) {
        self.see(cx);
        let axis = clean_axis(&self.d.pts, false);
        if axis.len() < 2 {
            self.d.reset();
            return;
        }
        if let Some(why) = self.misfit() {
            cx.say(Level::Warn, why);
            return;
        }
        let Some(parts) = self.parts(&axis) else {
            self.d.reset();
            return;
        };
        let m = self.memory();
        let (kind, width) = (ROAD_KINDS[self.kind()], self.width());
        let (kerb, median) = self.kerb_median();
        let mut objects = Vec::new();
        let width_text = js_number(width);
        objects.extend(
            area_geometry(std::slice::from_ref(&parts.road))
                .map(|g| (g, Some(attrs(&[(KIND, kind), (WIDTH, &width_text)])))),
        );
        if let Some(c) = &parts.carriageway {
            let text = js_number(width - 2.0 * kerb);
            objects.extend(
                area_geometry(std::slice::from_ref(c))
                    .map(|g| (g, Some(attrs(&[(KIND, CARRIAGEWAY), (WIDTH, &text)])))),
            );
        }
        if let Some(md) = &parts.median {
            let text = js_number(median);
            objects.extend(
                area_geometry(std::slice::from_ref(md))
                    .map(|g| (g, Some(attrs(&[(KIND, "Refüj"), (WIDTH, &text)])))),
            );
        }
        if m.road_axis {
            objects.push((
                EntityGeometry::Polyline {
                    pts: wire_all(&axis),
                    bulges: None,
                    zs: None,
                    parts: None,
                },
                Some(attrs(&[(KIND, "Yol ekseni")])),
            ));
        }
        let f = cx.format();
        if points::write_objects_each(objects, Some(CreateOperation::PlanRoad), cx).is_some() {
            cx.say(
                Level::Success,
                format!(
                    "{kind} eklendi: genişlik {}, eksen {}.",
                    f.length(width),
                    f.length(length(&axis))
                ),
            );
        }
        self.d.reset();
    }

    /// A value typed for the value asked; false when it is not a number.
    fn value(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        let Some(n) = plain_number(text) else {
            return false;
        };
        let m = cx.format().to_metres(n);
        let kind = usize::from(cx.memory.road_kind).min(ROAD_KINDS.len() - 1);
        match self.asking {
            Some(Asking::Width) => {
                if m.is_nan() || m <= 0.0 {
                    cx.say(Level::Warn, "Yolun genişliği sıfırdan büyük olmalı.");
                    return true;
                }
                cx.memory.road_widths[kind] = m;
            }
            Some(asked) => {
                if m.is_nan() || m < 0.0 {
                    let what = if asked == Asking::Kerb {
                        "Kaldırım genişliği sıfır ya da pozitif olmalı."
                    } else {
                        "Refüj genişliği sıfır ya da pozitif olmalı."
                    };
                    cx.say(Level::Warn, what);
                    return true;
                }
                if asked == Asking::Kerb {
                    cx.memory.road_kerb = m;
                } else {
                    cx.memory.road_median = m;
                }
            }
            None => return false,
        }
        self.asking = None;
        self.see(cx);
        true
    }

    /// An option's letter: whether it was one.
    fn option(&mut self, key: &str, cx: &mut Context<'_>) -> bool {
        let road = usize::from(cx.memory.road_kind) == 0;
        match key {
            "T" => {
                cx.memory.road_kind = (cx.memory.road_kind + 1) % ROAD_KINDS.len() as u8;
            }
            "G" => self.asking = Some(Asking::Width),
            "K" if road => self.asking = Some(Asking::Kerb),
            "R" if road => self.asking = Some(Asking::Median),
            "E" => cx.memory.road_axis = !cx.memory.road_axis,
            _ => return false,
        }
        self.see(cx);
        true
    }
}

impl Tool for PlanRoad {
    fn id(&self) -> &'static str {
        ROAD_ID
    }

    fn label(&self) -> &'static str {
        ROAD_LABEL
    }

    fn prompt(&self) -> Prompt {
        let (m, f) = (self.memory(), self.format());
        let unit = f.length_unit_label();
        match self.asking {
            Some(Asking::Width) => Prompt::new(
                ROAD_LABEL,
                format!(
                    "yolun genişliğini {unit} olarak yazın (Enter: {})",
                    f.plain(self.width())
                ),
            ),
            Some(Asking::Kerb) => Prompt::new(
                ROAD_LABEL,
                format!(
                    "kaldırım genişliğini {unit} olarak yazın; 0 kaldırımsız (Enter: {})",
                    f.plain(m.road_kerb)
                ),
            ),
            Some(Asking::Median) => Prompt::new(
                ROAD_LABEL,
                format!(
                    "refüj genişliğini {unit} olarak yazın; 0 refüjsüz (Enter: {})",
                    f.plain(m.road_median)
                ),
            ),
            None if self.d.pts.is_empty() => {
                self.options(Prompt::new(ROAD_LABEL, "eksenin ilk noktasını belirtin"))
            }
            None => self.options(Prompt::new(
                ROAD_LABEL,
                "eksenin sonraki noktasını belirtin; Enter ya da sağ tık bitirir",
            )),
        }
    }

    fn point_count(&self) -> usize {
        self.d.pts.len()
    }

    fn activate(&mut self, cx: &mut Context<'_>) -> Flow {
        self.see(cx);
        Flow::Stay
    }

    fn snap_from(&self) -> Option<Vec2> {
        self.d.last()
    }

    fn accepts_points(&self) -> bool {
        true
    }

    fn accept_point(&mut self, p: Vec2, cx: &mut Context<'_>) -> bool {
        self.asking = None;
        if self.d.last().is_some_and(|q| dist(q, p) < points::SAME) {
            return true;
        }
        self.d.begin(p, cx);
        self.d.pts.push(p);
        self.see(cx);
        true
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.see(cx);
        self.d.hover = Some(self.d.constrain(p, cx));
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        let point = self.d.constrain(p, cx);
        self.accept_point(point, cx);
    }

    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        self.see(cx);
        let typed = js_trim(text);
        if self.option(&upper_tr(typed), cx) {
            return true;
        }
        if self.asking.is_some() {
            return self.value(typed, cx);
        }
        let Some(p) = cx.typed_point(text, self.d.last(), self.d.hover) else {
            return false;
        };
        self.accept_point(p, cx)
    }

    fn option_choices(&self, key: &str) -> Vec<OptionChoice> {
        if key != "T" {
            return Vec::new();
        }
        ROAD_KINDS
            .iter()
            .enumerate()
            .map(|(i, kind)| OptionChoice {
                label: (*kind).to_owned(),
                typed: kind.to_lowercase(),
                icon: None,
                preview: None,
                checked: i == self.kind(),
                command: None,
            })
            .collect()
    }

    fn choose_option(&mut self, key: &str, typed: &str, cx: &mut Context<'_>) -> bool {
        let word = upper_tr(js_trim(typed));
        let Some(i) = ROAD_KINDS
            .iter()
            .position(|k| key == "T" && upper_tr(k) == word)
        else {
            return false;
        };
        cx.memory.road_kind = i as u8;
        self.see(cx);
        true
    }

    /// A value asked: it stays as it was. Else the road along the points in;
    /// none: the tool leaves.
    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow {
        if self.asking.take().is_some() {
            return Flow::Stay;
        }
        if self.d.pts.is_empty() {
            return Flow::Exit;
        }
        self.write(cx);
        Flow::Stay
    }

    fn cancel(&mut self, _cx: &mut Context<'_>) -> bool {
        if self.asking.take().is_some() {
            return true;
        }
        self.d.pts.pop().is_some()
    }

    fn undo_step(&mut self, cx: &mut Context<'_>) -> bool {
        self.cancel(cx)
    }

    /// The axis so far to the cursor, dashed; the road's areas along it, the
    /// kind and size beside the cursor.
    fn preview(&self, format: &Format) -> Preview {
        let mut preview = Preview {
            tracking: self.d.tracking,
            ..Preview::default()
        };
        let Some(h) = self.d.hover else {
            return preview;
        };
        if self.d.pts.is_empty() {
            return preview;
        }
        let axis = clean_axis(
            &self.d.pts.iter().copied().chain([h]).collect::<Vec<_>>(),
            false,
        );
        preview
            .strokes
            .push(Stroke::dashed(axis.clone(), false, [5.0, 3.0]));
        let Some(parts) = (axis.len() >= 2).then(|| self.parts(&axis)).flatten() else {
            return preview;
        };
        preview.strokes.extend(area_strokes(&parts.road, |pts| {
            Stroke::solid(pts, true).width(1.5)
        }));
        if let Some(c) = &parts.carriageway {
            preview
                .strokes
                .extend(area_strokes(c, |pts| Stroke::dashed(pts, true, [4.0, 3.0])));
        }
        if let Some(md) = &parts.median {
            preview
                .strokes
                .extend(area_strokes(md, |pts| Stroke::solid(pts, true)));
        }
        preview.tag = Some(Tag {
            at: h,
            lines: vec![
                ROAD_KINDS[self.kind()].to_owned(),
                format!(
                    "{} × {}",
                    format.length(self.width()),
                    format.length(length(&axis))
                ),
            ],
        });
        preview
    }
}

// ── Kavşak temizle ──────────────────────────────────────────────────────

/// One kind's work: its objects (the first keeps the result), the result and its counts.
#[derive(Clone, Debug)]
struct Group {
    slots: Vec<Slot>,
    geometry: Option<EntityGeometry>,
    areas: Vec<Area>,
    done: usize,
    skipped: usize,
}

/// What would be written for the drawing as it was and the radii it was worked out with.
struct Plan {
    key: (u64, u64, u64),
    groups: Vec<Group>,
}

/// Kavşak temizle.
#[derive(Default)]
pub struct RoadJunctions {
    slots: Vec<Slot>,
    /// A radius being typed: Ada köşesi (true) or Kaldırım köşesi.
    asking: Option<bool>,
    hover: Option<Vec2>,
    plan: Option<Plan>,
    seen: Option<(Memory, Format)>,
}

impl RoadJunctions {
    pub fn tool() -> Modify<Self> {
        Modify::with(Self::default())
    }

    fn memory(&self) -> Memory {
        self.seen.map(|(m, _)| m).unwrap_or_default()
    }

    fn format(&self) -> Format {
        self.seen.map(|(_, f)| f).unwrap_or_default()
    }

    fn groups(&self) -> &[Group] {
        self.plan.as_ref().map_or(&[], |p| &p.groups)
    }

    /// `3 alan → 1 alan, 4 köşe yuvarlanacak`, then `1 köşe sığmıyor` when some are left.
    fn finding(&self) -> Vec<String> {
        let groups = self.groups();
        let before: usize = groups.iter().map(|g| g.slots.len()).sum();
        let after = groups.iter().filter(|g| g.geometry.is_some()).count();
        let done: usize = groups.iter().map(|g| g.done).sum();
        let skipped: usize = groups.iter().map(|g| g.skipped).sum();
        let mut lines = vec![format!(
            "{before} alan → {after} alan, {done} köşe yuvarlanacak"
        )];
        if skipped > 0 {
            lines.push(format!("{skipped} köşe sığmıyor"));
        }
        lines
    }

    /// The result written in one step, and it is said.
    fn write(&mut self, cx: &mut Context<'_>) -> Flow {
        self.see(cx);
        let mut changes = Vec::new();
        for g in self.groups() {
            let (Some((first, others)), Some(geometry)) = (g.slots.split_first(), &g.geometry)
            else {
                continue;
            };
            if others.is_empty() && g.done == 0 {
                continue;
            }
            changes.push(EntityEdit::Update {
                uid: edge::uid(cx.doc, *first),
                geometry: geometry.clone(),
            });
            changes.extend(others.iter().map(|s| EntityEdit::Remove {
                uid: edge::uid(cx.doc, *s),
            }));
        }
        if changes.is_empty() {
            cx.say(Level::Info, "Kavşak temizle: değişecek bir şey yok.");
            return Flow::Exit;
        }
        let lines = self.finding();
        if edge::write(EditOperation::RoadJunctions, changes, cx).is_none() {
            return Flow::Stay;
        }
        let head = lines[0].replace(" yuvarlanacak", " yuvarlandı");
        let rest: String = lines[1..].iter().map(|l| format!("; {l}")).collect();
        cx.say(Level::Success, format!("Kavşak temizlendi: {head}{rest}."));
        Flow::Exit
    }
}

impl Stages for RoadJunctions {
    fn id(&self) -> &'static str {
        JUNCTIONS_ID
    }

    fn label(&self) -> &'static str {
        JUNCTIONS_LABEL
    }

    /// The radii, and the groups for the drawing as it is: worked out again
    /// when the drawing or a radius changed.
    fn see(&mut self, cx: &Context<'_>) {
        self.seen = Some((*cx.memory, cx.format()));
        if self.slots.is_empty() {
            return;
        }
        let m = *cx.memory;
        let key = (
            cx.doc.generation(),
            m.junction_ada.to_bits(),
            m.junction_kerb.to_bits(),
        );
        if self.plan.as_ref().is_some_and(|p| p.key == key) {
            return;
        }
        // In the drawing's order: a kind's first area keeps the result.
        let mut kinds: Vec<(String, Vec<Slot>, Vec<Area>)> = Vec::new();
        for e in cx.doc.entities() {
            let slot = Slot(e.base().id);
            if !self.slots.contains(&slot) {
                continue;
            }
            let kind = e.base().attrs.get(KIND).cloned().unwrap_or_default();
            let areas = areas_of_entity(&shape(e));
            match kinds.iter_mut().find(|(k, ..)| *k == kind) {
                Some((_, slots, all)) => {
                    slots.push(slot);
                    all.extend(areas);
                }
                None => kinds.push((kind, vec![slot], areas)),
            }
        }
        let groups = kinds
            .into_iter()
            .map(|(kind, slots, areas)| {
                let radius = if kind == CARRIAGEWAY {
                    m.junction_kerb
                } else {
                    m.junction_ada
                };
                let joined = road_junctions(&areas, radius);
                Group {
                    slots,
                    geometry: area_geometry(&joined.areas),
                    areas: joined.areas,
                    done: joined.done,
                    skipped: joined.skipped,
                }
            })
            .collect();
        self.plan = Some(Plan { key, groups });
    }

    fn begin(&mut self, cx: &mut Context<'_>) -> Flow {
        self.asking = None;
        self.plan = None;
        self.slots = cx
            .selection
            .ids()
            .iter()
            .copied()
            .filter(|&s| {
                cx.doc.get(s).is_some_and(|e| {
                    e.kind() == "polygon"
                        && !NOT_JOINED
                            .contains(&e.base().attrs.get(KIND).map_or("", String::as_str))
                })
            })
            .collect();
        if self.slots.is_empty() {
            cx.say(Level::Warn, "Kavşak temizle: seçimde yol alanı yok.");
            return Flow::Exit;
        }
        Flow::Stay
    }

    fn snaps(&self) -> bool {
        false
    }

    fn anchor(&self) -> Option<Vec2> {
        None
    }

    fn prompt(&self, _n: usize) -> Prompt {
        let (m, f) = (self.memory(), self.format());
        let unit = f.length_unit_label();
        match self.asking {
            Some(true) => Prompt::new(
                JUNCTIONS_LABEL,
                format!(
                    "ada köşesi yarıçapını {unit} olarak yazın; 0 yuvarlamaz (Enter: {})",
                    f.plain(m.junction_ada)
                ),
            ),
            Some(false) => Prompt::new(
                JUNCTIONS_LABEL,
                format!(
                    "kaldırım köşesi yarıçapını {unit} olarak yazın; 0 yuvarlamaz (Enter: {})",
                    f.plain(m.junction_kerb)
                ),
            ),
            None => Prompt::new(JUNCTIONS_LABEL, self.finding().join("; "))
                .option_with("Ada köşesi", "A", f.length(m.junction_ada))
                .option_with("Kaldırım köşesi", "K", f.length(m.junction_kerb))
                .option("Uygula", "Enter"),
        }
    }

    /// The cursor as it is; a click places nothing (Enter, Uygula or a quick
    /// right click writes).
    fn pointer(&mut self, p: &Pointer, down: bool, _cx: &mut Context<'_>) -> Option<Flow> {
        if down {
            return Some(Flow::Stay);
        }
        self.hover = Some(p.raw);
        None
    }

    fn point(&mut self, _p: Vec2, _cx: &mut Context<'_>) -> Flow {
        Flow::Stay
    }

    /// A and K ask a radius; a number answers it.
    fn typed(&mut self, text: &str, cx: &mut Context<'_>) -> Option<Flow> {
        let typed = js_trim(text);
        match upper_tr(typed).as_str() {
            "A" => {
                self.asking = Some(true);
                return Some(Flow::Stay);
            }
            "K" => {
                self.asking = Some(false);
                return Some(Flow::Stay);
            }
            _ => {}
        }
        let ada = self.asking?;
        let n = plain_number(typed)?;
        let m = cx.format().to_metres(n);
        if m.is_nan() || m < 0.0 {
            cx.say(Level::Warn, "Yarıçap sıfır ya da pozitif olmalı.");
            return Some(Flow::Stay);
        }
        if ada {
            cx.memory.junction_ada = m;
        } else {
            cx.memory.junction_kerb = m;
        }
        self.asking = None;
        self.see(cx);
        Some(Flow::Stay)
    }

    fn typed_points(&self) -> bool {
        false
    }

    fn takes_points(&self) -> bool {
        false
    }

    /// A radius asked keeps its value; else the result is written.
    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow {
        if self.asking.take().is_some() {
            return Flow::Stay;
        }
        self.write(cx)
    }

    fn back(&mut self, _cx: &mut Context<'_>) -> bool {
        self.asking.take().is_some()
    }

    fn preview(&self, _hover: Vec2) -> Option<kentos_geometry_core::geom::affine::Affine> {
        None
    }

    /// The joined and rounded areas, dashed; the finding beside the cursor.
    fn stage_preview(&self, hover: Option<Vec2>, _format: &Format) -> Option<Preview> {
        let strokes = self
            .groups()
            .iter()
            .flat_map(|g| g.areas.iter())
            .flat_map(|a| area_strokes(a, |pts| Stroke::dashed(pts, true, [5.0, 3.0]).width(1.5)))
            .collect();
        let tag = hover.map(|at| Tag {
            at,
            lines: self
                .finding()
                .into_iter()
                .chain(["Enter: uygula".to_owned()])
                .collect(),
        });
        Some(Preview {
            strokes,
            tag,
            ..Preview::default()
        })
    }
}

// ── Refüj kapat ─────────────────────────────────────────────────────────

/// A line's or an open, one-part polyline's path, as the core takes a ring's.
fn path_of(e: &kentos_contracts::Entity) -> Option<Ring> {
    match shape(e) {
        Shape::Line { a, b } => Some(Ring {
            pts: vec![a, b],
            bulges: None,
        }),
        Shape::Polyline {
            pts, bulges, parts, ..
        } if parts.as_ref().is_none_or(Vec::is_empty) => Some(Ring { pts, bulges }),
        _ => None,
    }
}

/// A line or a polyline under the cursor, the first one aside.
fn line_under(p: &Pointer, cx: &Context<'_>, besides: Option<Slot>) -> Option<Slot> {
    cx.spatial.pick_edge(p.raw, cx.pick_tolerance(), |s| {
        Some(s) != besides
            && cx
                .doc
                .get(s)
                .is_some_and(|e| matches!(e.kind(), "line" | "polyline"))
    })
}

/// Refüj kapat.
#[derive(Clone, Debug, Default)]
pub struct MedianClose {
    /// The first edge: its object, its path and where it was clicked.
    first: Option<(Slot, Ring, Vec2)>,
    /// The line under the cursor and its path.
    under: Option<(Slot, Option<Ring>)>,
    round: bool,
}

impl MedianClose {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Tool for MedianClose {
    fn id(&self) -> &'static str {
        MEDIAN_ID
    }

    fn label(&self) -> &'static str {
        MEDIAN_LABEL
    }

    fn prompt(&self) -> Prompt {
        let step = if self.first.is_some() {
            "refüjün ikinci kenarına tıklayın; Esc birinciyi bıraktırır"
        } else {
            "refüjün birinci kenarına tıklayın"
        };
        let end = if self.round { "Yarım daire" } else { "Düz" };
        Prompt::new(MEDIAN_LABEL, step).option_with("Uç", "U", end)
    }

    fn point_count(&self) -> usize {
        usize::from(self.first.is_some())
    }

    fn activate(&mut self, cx: &mut Context<'_>) -> Flow {
        self.round = cx.memory.median_round;
        Flow::Stay
    }

    fn snaps(&self) -> bool {
        false
    }

    fn cursor(&self) -> Cursor {
        Cursor::Pick
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.round = cx.memory.median_round;
        let slot = line_under(p, cx, self.first.as_ref().map(|(s, ..)| *s));
        self.under = slot.map(|s| (s, cx.doc.get(s).and_then(path_of)));
        cx.selection.set_hover(slot);
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        let Some(slot) = line_under(p, cx, self.first.as_ref().map(|(s, ..)| *s)) else {
            cx.say(
                Level::Warn,
                "Tıklanan yerde çizgi ya da çoklu çizgi yok. Refüjün kenarına tıklayın.",
            );
            return;
        };
        let Some(path) = cx.doc.get(slot).and_then(path_of) else {
            cx.say(
                Level::Warn,
                "Çok parçalı çizgi refüjün kenarı olamaz; parçalarına ayırın.",
            );
            return;
        };
        let Some((first, first_path, _)) = self.first.take() else {
            self.first = Some((slot, path, p.raw));
            self.under = None;
            cx.selection.set_hover(None);
            return;
        };
        self.under = None;
        cx.selection.set_hover(None);
        let Some(ring) = median_ring(
            &first_path.pts,
            first_path.bulges.as_deref(),
            &path.pts,
            path.bulges.as_deref(),
            cx.memory.median_round,
        ) else {
            cx.say(Level::Warn, "Bu iki çizgi refüj olarak kapatılamaz.");
            return;
        };
        let changes = vec![
            EntityEdit::Replace {
                uid: edge::uid(cx.doc, first),
                geometry: EntityGeometry::Polygon {
                    pts: wire_all(&ring.pts),
                    bulges: ring.bulges,
                    holes: None,
                    zs: None,
                    parts: None,
                },
                keep_data: Some(true),
            },
            EntityEdit::Remove {
                uid: edge::uid(cx.doc, slot),
            },
        ];
        if edge::write(EditOperation::MedianClose, changes, cx).is_some() {
            cx.say(Level::Success, "Refüj kapatıldı.");
        }
    }

    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        if upper_tr(js_trim(text)) != "U" {
            return false;
        }
        cx.memory.median_round = !cx.memory.median_round;
        self.round = cx.memory.median_round;
        true
    }

    fn option_choices(&self, key: &str) -> Vec<OptionChoice> {
        if key != "U" {
            return Vec::new();
        }
        [("Yarım daire", true), ("Düz", false)]
            .into_iter()
            .map(|(label, round)| OptionChoice {
                label: label.to_owned(),
                typed: label.to_lowercase(),
                icon: None,
                preview: None,
                checked: round == self.round,
                command: None,
            })
            .collect()
    }

    fn choose_option(&mut self, key: &str, typed: &str, cx: &mut Context<'_>) -> bool {
        let word = upper_tr(js_trim(typed));
        let round = match word.as_str() {
            "YARIM DAIRE" | "YARIM DAİRE" => true,
            "DÜZ" => false,
            _ => return false,
        };
        if key != "U" {
            return false;
        }
        cx.memory.median_round = round;
        self.round = round;
        true
    }

    /// The first let go; without one the tool leaves.
    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow {
        if self.first.take().is_some() {
            cx.selection.set_hover(None);
            return Flow::Stay;
        }
        Flow::Exit
    }

    fn cancel(&mut self, cx: &mut Context<'_>) -> bool {
        cx.selection.set_hover(None);
        self.under = None;
        self.first.take().is_some()
    }

    fn undo_step(&mut self, cx: &mut Context<'_>) -> bool {
        self.cancel(cx)
    }

    /// Where the first was picked; over a second, the median it closes, dashed.
    fn preview(&self, _format: &Format) -> Preview {
        let mut preview = Preview::default();
        let Some((_, first, at)) = &self.first else {
            return preview;
        };
        preview.squares.push(*at);
        let Some((_, Some(second))) = &self.under else {
            return preview;
        };
        if let Some(ring) = median_ring(
            &first.pts,
            first.bulges.as_deref(),
            &second.pts,
            second.bulges.as_deref(),
            self.round,
        ) {
            preview
                .strokes
                .push(Stroke::dashed(ring_points(&ring), true, [5.0, 3.0]).width(1.5));
        }
        preview
    }
}
