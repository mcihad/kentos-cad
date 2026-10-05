//! Toplu alan (docs/adr/0151): every region the line work closes becomes an
//! area in one undo step, the text or the named point inside each its
//! attribute. The finding is the shared core's (`polygonize`); the web's
//! tool is `apps/web/src/tools/polygonizeTool.ts`, and both play
//! `fixtures/interaction/v1/polygonize.json`.
//!
//! - **Input**, taken when it starts (§2): the selection's line work and
//!   labels, else every visible layer's. Line work is lines, polylines,
//!   arcs, circles, ellipses, curves and areas; a label is a text (its value
//!   its text, its place its box's middle) or a point with a label.
//! - **Adalar (A)** and **the attribute's name (Ö)** are kept for as long as
//!   the app lives ([`crate::tool::Memory`]).
//! - **Shown first** (§8): the areas to be written (one label: the accent,
//!   filled; none: the accent, dashed; two or more: the danger colour,
//!   filled), a cross at every free end, a ring at every label on a
//!   boundary, the counts beside the cursor. Enter, the Uygula button or a
//!   quick right click writes them through `cad.entities.create`
//!   (`polygonize`) on the active layer, each with its elevations carried
//!   from the line work (ADR 0142) and its label as the attribute; Esc leaves.

use std::collections::{BTreeMap, HashSet};

use kentos_contracts::{CreateOperation, Entity, EntityGeometry, NewObject};
use kentos_domain::{Document, Slot};
use kentos_geometry_core::entity::{Entity as CoreEntity, TextPlace, polygon_ring};
use kentos_geometry_core::ops::areas::polygon_of_area;
use kentos_geometry_core::ops::elevation::{Carry, Elevated, carry_elevations};
use kentos_geometry_core::ops::polygonize::{PolyLabel, PolyResult, polygonize};
use kentos_native_application::elevation::{elevated, paths as elevated_paths};
use kentos_native_application::geometry::{drawing_font, edit_geometry, shape};

use crate::Vec2;
use crate::format::Format;
use crate::log::Level;
use crate::modify::MAX_GHOSTS;
use crate::prompt::Prompt;
use crate::tool::{
    Area, Context, Cursor, Flow, Marker, MarkerShape, Preview, Stroke, Tag, TextField, Tone, Tool,
    ViewChange,
};

/// Toplu alan's id: its command is `tool.polygonize`.
pub const ID: &str = "polygonize";
pub const LABEL: &str = "Toplu alan";
/// The attribute's name before any is typed (docs/adr/0151 §4).
pub const FIRST_ATTRIBUTE: &str = "Ad";
/// The details said at most (regions with many labels, labels on a boundary).
const MAX_DETAILS: usize = 20;
/// The free ends marked at most.
const MAX_MARKS: usize = 2000;

/// How an area to be written looks: one label, none, or many.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Look {
    One,
    None,
    Many,
}

/// An area to be written: its geometry, the label's value when it has one, how it looks.
#[derive(Clone, Debug)]
struct ToWrite {
    geometry: EntityGeometry,
    /// Its rings as drawn (arcs made points).
    rings: Vec<Vec<Vec2>>,
    value: Option<String>,
    look: Look,
}

/// What the tool would write for the drawing as it is, and what it was worked out from.
#[derive(Clone, Debug)]
struct Plan {
    generation: u64,
    islands: bool,
    attribute: String,
    result: PolyResult,
    labels: Vec<PolyLabel>,
    areas: Vec<ToWrite>,
    unlabelled: usize,
    many: usize,
    existing: usize,
    drawn: Preview,
}

/// The tool.
#[derive(Clone, Debug, Default)]
pub struct Polygonize {
    /// The objects it reads, taken when it starts.
    lines: HashSet<Slot>,
    labels: HashSet<Slot>,
    whole: bool,
    /// Ö: the attribute's name is being typed in its field.
    typing: bool,
    plan: Option<Plan>,
    /// The finding last said, so a change that finds the same says nothing again.
    said: String,
    hover: Option<Vec2>,
    done: bool,
}

/// The kinds that close regions (docs/adr/0151 §2): construction lines,
/// leaders and blocks do not.
fn line_work(e: &Entity) -> bool {
    matches!(
        e,
        Entity::Line(_)
            | Entity::Polyline(_)
            | Entity::Arc(_)
            | Entity::Circle(_)
            | Entity::Ellipse(_)
            | Entity::Spline(_)
            | Entity::Polygon(_)
    )
}

/// A label's value: a text's (trimmed), a point's label; none for anything
/// else or an empty one (the web's `labelValue`).
pub fn label_value(e: &Entity) -> Option<String> {
    let v = match e {
        Entity::Text(t) => t.text.trim(),
        Entity::Point(p) => p.base.label.as_deref().map_or("", str::trim),
        _ => "",
    };
    (!v.is_empty()).then(|| v.to_owned())
}

/// Where a label is: a text's box middle (its alignment and the drawing's
/// typeface counted), a point itself (the web's `labelAt`).
pub fn label_at(e: &Entity, doc: &Document) -> Option<Vec2> {
    match e {
        Entity::Point(p) => Some(Vec2::new(p.p.x, p.p.y)),
        Entity::Text(_) => {
            let s = shape(e);
            let b = TextPlace::of(&s)?.outline(drawing_font(doc.settings().drawing_font));
            if b.len() < 4 {
                return None;
            }
            Some(Vec2::new(
                (b[0].x + b[1].x + b[2].x + b[3].x) / 4.0,
                (b[0].y + b[1].y + b[2].y + b[3].y) / 4.0,
            ))
        }
        _ => None,
    }
}

impl Polygonize {
    pub fn new() -> Self {
        Self::default()
    }

    /// The line work and the labels (docs/adr/0151 §2); false when there is no line work.
    fn take_input(&mut self, cx: &mut Context<'_>) -> bool {
        let doc = &*cx.doc;
        let layers = doc.layers();
        self.whole = cx.selection.is_empty();
        let chosen: Vec<&Entity> = if self.whole {
            doc.entities().collect()
        } else {
            cx.selection
                .ids()
                .iter()
                .filter_map(|slot| doc.get(*slot))
                .collect()
        };
        let seen = chosen
            .into_iter()
            .filter(|e| layers.is_visible(&e.base().layer_id));
        self.lines.clear();
        self.labels.clear();
        for e in seen {
            if line_work(e) {
                self.lines.insert(Slot(e.base().id));
            }
            if label_value(e).is_some() {
                self.labels.insert(Slot(e.base().id));
            }
        }
        if self.lines.is_empty() {
            cx.say(
                Level::Warn,
                format!(
                    "{LABEL}: bölge kapatacak çizgi yok; çizgi, çoklu çizgi, yay ya da alan seçin."
                ),
            );
        }
        !self.lines.is_empty()
    }

    /// The plan for the drawing as it is, kept until the drawing, Adalar or
    /// the attribute's name change. Whether it was worked out again.
    fn refresh(&mut self, cx: &Context<'_>) -> bool {
        let (islands, attribute) = (
            cx.memory.polygonize_islands,
            cx.memory.polygonize_attribute.as_str(),
        );
        let generation = cx.doc.generation();
        if self.plan.as_ref().is_some_and(|p| {
            p.generation == generation && p.islands == islands && p.attribute == attribute
        }) {
            return false;
        }
        self.plan = Some(self.work_out(cx.doc, islands, attribute.to_owned(), generation));
        true
    }

    fn work_out(&self, doc: &Document, islands: bool, attribute: String, generation: u64) -> Plan {
        // In the drawing's order: an input area is named by its place there.
        let mut lines: Vec<&Entity> = Vec::new();
        let mut labels: Vec<PolyLabel> = Vec::new();
        for e in doc.entities() {
            let slot = Slot(e.base().id);
            if self.lines.contains(&slot) {
                lines.push(e);
            }
            if self.labels.contains(&slot)
                && let (Some(value), Some(at)) = (label_value(e), label_at(e, doc))
            {
                labels.push(PolyLabel { at, value });
            }
        }
        let core: Vec<CoreEntity> = lines.iter().map(|e| CoreEntity::new(shape(e))).collect();
        let result = polygonize(&core, &labels, islands);
        let sources: Vec<Elevated> = lines.iter().flat_map(|e| elevated_paths(e)).collect();
        let carry = elevated(&sources);
        let mut areas = Vec::new();
        let (mut unlabelled, mut many, mut existing) = (0, 0, 0);
        for r in &result.regions {
            if r.existing.is_some() {
                existing += 1;
                continue;
            }
            let look = match r.labels.len() {
                1 => Look::One,
                0 => Look::None,
                _ => Look::Many,
            };
            match look {
                Look::None => unlabelled += 1,
                Look::Many => many += 1,
                Look::One => {}
            }
            let Some(mut geometry) = edit_geometry(polygon_of_area(&r.area).shape) else {
                continue;
            };
            if carry {
                with_elevations(&mut geometry, &sources);
            }
            let rings = std::iter::once(&r.area.outer)
                .chain(&r.area.holes)
                .map(|ring| polygon_ring(&ring.pts, ring.bulges.as_deref()))
                .collect();
            areas.push(ToWrite {
                geometry,
                rings,
                value: (look == Look::One).then(|| labels[r.labels[0]].value.clone()),
                look,
            });
        }
        let mut plan = Plan {
            generation,
            islands,
            attribute,
            result,
            labels,
            areas,
            unlabelled,
            many,
            existing,
            drawn: Preview::default(),
        };
        plan.drawn = drawn(&plan);
        plan
    }

    /// Says the finding when it changed; when the tool starts, with what it
    /// read and what to do (the web's `tell`).
    fn tell(&mut self, cx: &mut Context<'_>, scope: Option<String>) {
        let Some(plan) = &self.plan else { return };
        let text = finding(plan);
        if scope.is_none() && text == self.said {
            return;
        }
        let message = match (&scope, plan.areas.is_empty()) {
            (Some(scope), false) => format!("{LABEL}: {text}. Enter ile uygulayın ({scope})."),
            (Some(scope), true) => format!("{LABEL}: {text} ({scope})."),
            (None, _) => format!("{LABEL}: {text}."),
        };
        let level = if plan.areas.is_empty() {
            Level::Warn
        } else {
            Level::Info
        };
        cx.say(level, message);
        self.said = text;
    }

    /// What to look at: free ends (with Topolojik temizlik's offer), regions
    /// with many labels, labels on a boundary (the web's `details`).
    fn details(&self, cx: &mut Context<'_>) {
        let Some(plan) = &self.plan else { return };
        let free = plan.result.free_ends.len();
        if free > 0 {
            cx.say(
                Level::Warn,
                format!(
                    "{LABEL}: {free} çizgi ucu boşta; kapanmayan bölge alan olmaz. Önce Topolojik temizlik'i deneyin."
                ),
            );
        }
        let quote = |i: &usize| format!("“{}”", plan.labels[*i].value);
        let mut lines: Vec<String> = plan
            .result
            .regions
            .iter()
            .filter(|r| r.existing.is_none() && r.labels.len() > 1)
            .map(|r| {
                format!(
                    "{LABEL}: bir bölgede {} etiket: {}.",
                    r.labels.len(),
                    r.labels.iter().map(quote).collect::<Vec<_>>().join(", ")
                )
            })
            .collect();
        lines.extend(plan.result.on_boundary.iter().map(|i| {
            format!(
                "{LABEL}: {} etiketi bir sınırın üstünde; hiçbir bölgeye verilmedi.",
                quote(i)
            )
        }));
        let more = lines.len().saturating_sub(MAX_DETAILS);
        for line in lines.into_iter().take(MAX_DETAILS) {
            cx.say(Level::Warn, line);
        }
        if more > 0 {
            cx.say(Level::Warn, format!("{LABEL}: {more} ayrıntı daha."));
        }
    }

    /// Ö: a text field by the cursor, as Yazı's and Kılavuz's (a name is no
    /// command: letters typed over the drawing would start one), the name in
    /// it. Enter keeps what is typed, Esc (or an empty Enter) the old name.
    fn ask_name(&mut self, cx: &mut Context<'_>) {
        let b = cx.view.visible();
        let at = self.hover.unwrap_or(Vec2::new(
            (b.min_x + b.max_x) / 2.0,
            (b.min_y + b.max_y) / 2.0,
        ));
        self.typing = true;
        cx.view_changes.push(ViewChange::Text(TextField {
            at,
            // Readable at any zoom: 14 px on the screen.
            height: cx.view.world_length(14.0),
            rotation: 0.0,
            align: None,
            width_factor: 1.0,
            initial: Some(cx.memory.polygonize_attribute.as_str().to_owned()),
            placeholder: Some("Öznitelik adı"),
            hint: Some("Enter: kaydet · Esc: vazgeç"),
            empty: false,
        }));
    }

    /// Writes the areas in one step and says what came of it.
    fn write(&mut self, cx: &mut Context<'_>) -> Flow {
        self.refresh(cx);
        let Some(plan) = self.plan.clone() else {
            return Flow::Exit;
        };
        if plan.areas.is_empty() {
            cx.say(
                Level::Warn,
                format!("{LABEL}: yazılacak alan yok; hiçbir şey değişmedi."),
            );
            return Flow::Exit;
        }
        let attribute = plan.attribute.clone();
        let input = kentos_contracts::EntitiesCreate {
            layer_id: cx.doc.layers().active().to_owned(),
            objects: plan
                .areas
                .iter()
                .map(|a| NewObject {
                    line_weight: a
                        .geometry
                        .draws_lines()
                        .then_some(cx.draft.line_weight)
                        .flatten(),
                    geometry: a.geometry.clone(),
                    color: cx.draft.color.map(str::to_owned),
                    attrs: a
                        .value
                        .as_ref()
                        .map(|v| BTreeMap::from([(attribute.clone(), v.clone())])),
                    label: None,
                    label_of: None,
                    label_scale: None,
                    symbol: None,
                })
                .collect(),
            operation: Some(CreateOperation::Polygonize),
            expected_revision: None,
        };
        let result = kentos_native_application::create::execute(
            &mut kentos_native_application::ExecutionContext::new(cx.doc),
            input,
        );
        if crate::points::written(result, cx).is_none() {
            return Flow::Stay;
        }
        let named = plan.areas.iter().filter(|a| a.value.is_some()).count();
        let mut empty = Vec::new();
        if plan.unlabelled > 0 {
            empty.push(format!("{} etiketsiz", plan.unlabelled));
        }
        if plan.many > 0 {
            empty.push(format!("{} çok etiketli", plan.many));
        }
        let tail = if empty.is_empty() {
            String::new()
        } else {
            format!(" Özniteliği boş kalan: {}.", empty.join(", "))
        };
        cx.say(
            Level::Success,
            format!(
                "{LABEL}: {} alan oluşturuldu; “{attribute}” {named} alana yazıldı.{tail}",
                plan.areas.len()
            ),
        );
        self.done = true;
        Flow::Exit
    }
}

/// The area's rings with the elevations their corners carry from the line
/// work (docs/adr/0142); none where none does (the web's `withElevations`).
fn with_elevations(g: &mut EntityGeometry, sources: &[Elevated]) {
    let zs = |pts: &[kentos_contracts::Vec2]| {
        let pts: Vec<Vec2> = pts.iter().map(|p| Vec2::new(p.x, p.y)).collect();
        let out = carry_elevations(&pts, true, None, sources, Carry::Along);
        out.iter().any(Option::is_some).then_some(out)
    };
    if let EntityGeometry::Polygon {
        pts,
        holes,
        zs: own,
        ..
    } = g
    {
        *own = zs(pts);
        for h in holes.iter_mut().flatten() {
            h.zs = zs(&h.pts);
        }
    }
}

/// `24 alan; 2 etiketsiz, 3 uç boşta`: the areas to write, then what to look at.
fn finding(plan: &Plan) -> String {
    let r = &plan.result;
    let mut extras = Vec::new();
    for (n, what) in [
        (plan.unlabelled, "etiketsiz"),
        (plan.many, "çok etiketli"),
        (r.on_boundary.len(), "etiket sınırda"),
        (plan.existing, "zaten alan"),
        (r.free_ends.len(), "uç boşta"),
    ] {
        if n > 0 {
            extras.push(format!("{n} {what}"));
        }
    }
    let head = if plan.areas.is_empty() {
        "yazılacak alan yok".to_owned()
    } else {
        format!("{} alan", plan.areas.len())
    };
    if extras.is_empty() {
        head
    } else {
        format!("{head}; {}", extras.join(", "))
    }
}

/// The finding beside the cursor, a count a line, and what writes it.
fn tag_lines(plan: &Plan) -> Vec<String> {
    if plan.areas.is_empty() {
        return vec!["Yazılacak alan yok".to_owned()];
    }
    let mut lines: Vec<String> = finding(plan)
        .split("; ")
        .flat_map(|part| part.split(", "))
        .map(str::to_owned)
        .collect();
    lines.push("Enter: uygula".to_owned());
    lines
}

/// The areas to be written, the free ends and the labels on a boundary (the web's `draw`).
fn drawn(plan: &Plan) -> Preview {
    let mut areas = Vec::new();
    let mut strokes = Vec::new();
    for a in plan.areas.iter().take(MAX_GHOSTS) {
        let (fill, dash, fill_tone) = match a.look {
            Look::One => (0.16, None, Tone::Accent),
            Look::None => (0.0, Some([6.0, 4.0]), Tone::Accent),
            Look::Many => (0.16, None, Tone::Danger),
        };
        areas.push(Area {
            rings: a.rings.clone(),
            fill,
            width: 1.5,
            dash,
            fill_tone,
        });
        // The danger colour's outline over the accent one the host draws.
        if a.look == Look::Many {
            strokes.extend(
                a.rings
                    .iter()
                    .map(|r| Stroke::solid(r.clone(), true).width(1.5).tone(Tone::Danger)),
            );
        }
    }
    let mut markers: Vec<Marker> = plan
        .result
        .free_ends
        .iter()
        .take(MAX_MARKS)
        .map(|p| Marker {
            at: *p,
            shape: MarkerShape::Cross(4.0),
            tone: Tone::Danger,
        })
        .collect();
    markers.extend(plan.result.on_boundary.iter().map(|&i| Marker {
        at: plan.labels[i].at,
        shape: MarkerShape::Ring(5.0),
        tone: Tone::Danger,
    }));
    Preview {
        areas,
        strokes,
        markers,
        ..Preview::default()
    }
}

impl Tool for Polygonize {
    fn id(&self) -> &'static str {
        ID
    }

    fn label(&self) -> &'static str {
        LABEL
    }

    fn cursor(&self) -> Cursor {
        Cursor::Pick
    }

    fn snaps(&self) -> bool {
        false
    }

    fn prompt(&self) -> Prompt {
        let Some(plan) = &self.plan else {
            return Prompt::new(LABEL, "yazılacak alan yok");
        };
        if self.typing {
            return Prompt::new(
                LABEL,
                format!("öznitelik adını yazın (şimdi: {})", plan.attribute),
            );
        }
        let islands = if plan.islands { "açık" } else { "kapalı" };
        Prompt::new(LABEL, finding(plan))
            .option_with("Adalar", "A", islands)
            .option_with("Öznitelik", "Ö", plan.attribute.clone())
            .option("Uygula", "Enter")
    }

    fn point_count(&self) -> usize {
        0
    }

    /// Takes the input and says what it found; with no line work, says so and leaves.
    fn activate(&mut self, cx: &mut Context<'_>) -> Flow {
        if !self.take_input(cx) {
            return Flow::Exit;
        }
        self.refresh(cx);
        let (lines, labels) = (self.lines.len(), self.labels.len());
        let scope = if self.whole {
            format!("bütün çizim: {lines} çizgi, {labels} etiket")
        } else {
            format!("seçili {lines} çizgi, {labels} etiket")
        };
        self.tell(cx, Some(scope));
        self.details(cx);
        Flow::Stay
    }

    /// The tag follows the cursor; a drawing changed under the tool (an undo) is worked out again.
    fn pointer_move(&mut self, p: &crate::Pointer, cx: &mut Context<'_>) {
        self.hover = Some(p.raw);
        if self.refresh(cx) {
            self.tell(cx, None);
        }
    }

    /// A click does nothing: Enter, the button or a quick right click write.
    fn pointer_down(&mut self, _p: &crate::Pointer, _cx: &mut Context<'_>) {}

    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        if self.typing {
            return false;
        }
        let key = crate::prompt::upper_tr(crate::js_trim(text));
        match key.as_str() {
            "A" => {
                cx.memory.polygonize_islands = !cx.memory.polygonize_islands;
                self.refresh(cx);
                self.tell(cx, None);
                true
            }
            "Ö" | "O" => {
                self.ask_name(cx);
                true
            }
            _ => false,
        }
    }

    /// The name's field answers: the name typed, or none (Esc). An empty or
    /// too long name is said and the old one kept (the web's `takeName`).
    fn text_typed(&mut self, text: Option<&str>, cx: &mut Context<'_>) {
        if !self.typing {
            return;
        }
        self.typing = false;
        let name = text.map(crate::js_trim).unwrap_or_default();
        if text.is_some() && name.is_empty() {
            cx.say(Level::Warn, "Öznitelik adı boş olamaz.");
        } else if !name.is_empty() {
            match crate::tool::Name::new(name) {
                Some(name) => cx.memory.polygonize_attribute = name,
                None => cx.say(Level::Warn, "Öznitelik adı en çok 60 harf olabilir."),
            }
        }
        self.refresh(cx);
        self.tell(cx, None);
    }

    /// Enter: the areas written in one step, and the tool leaves (the name's field answers its own Enter).
    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow {
        if self.typing {
            return Flow::Stay;
        }
        self.write(cx)
    }

    fn undo_step(&mut self, _cx: &mut Context<'_>) -> bool {
        false
    }

    fn finished(&self) -> bool {
        self.done
    }

    /// What will be written, and the finding beside the cursor.
    fn preview(&self, _format: &Format) -> Preview {
        let Some(plan) = &self.plan else {
            return Preview::default();
        };
        Preview {
            tag: self.hover.map(|at| Tag {
                at,
                lines: tag_lines(plan),
            }),
            ..plan.drawn.clone()
        }
    }
}
