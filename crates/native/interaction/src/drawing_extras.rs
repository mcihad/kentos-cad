//! Çizim ekleri (docs/adr/0197 §4): the web's `TangentLineTool`,
//! `FourthCornerTool` and `RangeRingsTool` (`apps/web/src/tools/drawingExtrasTools.ts`).
//!
//! - **İki daireye teğet** (`tangentLine`): a click picks the first circle or
//!   arc near where the tangent is to touch it; over the second one the
//!   preview shows the tangent the clicks choose bright and the others
//!   dashed (the core's `common_tangents`, `chosen_tangent`); the click
//!   writes it as a line through `cad.entities.create` (step “İki daireye
//!   teğet”), and the tool waits for the next first circle.
//! - **Dördüncü köşe** (`fourthCorner`): three corners clicked (snapping) or
//!   typed, the middle one second; the preview draws the parallelogram
//!   dashed; the third writes the fourth corner (`fourth_corner`) as a
//!   point, or with Çıktı (Ç) Alan the four corners as an area (step
//!   “Dördüncü köşe”).
//! - **Menzil halkaları** (`rangeRings`): the centre clicked (snapping) or
//!   typed, the rings following the cursor; Aralık (A, in the project's
//!   unit), Sayı (S, 1 to 100) and Işın (I, 0 to 360) typed (`range_rings`);
//!   the circles and rays in one step (“Menzil halkaları”).
//!
//! Çıktı, Aralık, Sayı and Işın stay for as long as the app lives
//! ([`crate::tool::Memory`]).

use kentos_contracts::{CreateOperation, EntityGeometry};
use kentos_domain::Slot;
use kentos_geometry_core::entity::tessellate_circle;
use kentos_geometry_core::geometry::dist;
use kentos_geometry_core::tools::drawing_extras::{
    MAX_RAYS, MAX_RINGS, Round, Tangent, chosen_tangent, common_tangents, fourth_corner,
    range_rings,
};
use kentos_geometry_core::tools::point_text::js_trim;
use kentos_native_application::geometry::shape;

use crate::Vec2;
use crate::format::Format;
use crate::log::Level;
use crate::points::{self, Taken, plain_number, wire};
use crate::prompt::{Prompt, upper_tr};
use crate::tool::{
    Context, Cursor, Flow, Marker, MarkerShape, Memory, OptionChoice, Pointer, Preview, Stroke,
    Tag, Tone, Tool,
};

/// İki daireye teğet's id: its command is `tool.tangentLine`.
pub const TANGENT_ID: &str = "tangentLine";
pub const TANGENT_LABEL: &str = "İki daireye teğet";
/// Dördüncü köşe's id: its command is `tool.fourthCorner`.
pub const FOURTH_ID: &str = "fourthCorner";
pub const FOURTH_LABEL: &str = "Dördüncü köşe";
/// Menzil halkaları's id: its command is `tool.rangeRings`.
pub const RINGS_ID: &str = "rangeRings";
pub const RINGS_LABEL: &str = "Menzil halkaları";

/// How finely the preview draws a ring.
const RING_SEGMENTS: f64 = 96.0;

// ── İki daireye teğet ───────────────────────────────────────────────────

/// A circle or an arc under the cursor: its object and its circle.
fn round_under(p: &Pointer, cx: &Context<'_>, besides: Option<Slot>) -> Option<(Slot, Round)> {
    let slot = cx.spatial.pick_edge(p.raw, cx.pick_tolerance(), |s| {
        Some(s) != besides
            && cx
                .doc
                .get(s)
                .is_some_and(|e| matches!(e.kind(), "circle" | "arc"))
    })?;
    Some((slot, Round::of(&shape(cx.doc.get(slot)?))?))
}

/// What a tangent is called beside the cursor.
fn tangent_name(t: &Tangent) -> &'static str {
    if t.kind.starts_with("outer") {
        "Dış teğet"
    } else {
        "İç teğet"
    }
}

/// Why two circles or arcs have no tangent to write.
fn no_tangent(first: &Round, second: &Round) -> &'static str {
    if first.span.is_none() && second.span.is_none() {
        "Bu iki dairenin ortak teğeti yok: biri ötekinin içinde. Birbirinin dışındaki ya da kesişen iki daire seçin."
    } else {
        "Bu iki nesnenin ortak teğeti yok: teğetlerin değdiği yerler yayın dışında kalıyor. Başka bir yay ya da daire seçin."
    }
}

/// İki daireye teğet.
#[derive(Clone, Debug, Default)]
pub struct TangentLine {
    /// The first circle or arc: its object, its circle and where it was clicked.
    first: Option<(Slot, Round, Vec2)>,
    /// The circle or arc under the cursor, the first's partner to be.
    under: Option<(Slot, Round)>,
    hover: Option<Vec2>,
}

impl TangentLine {
    pub fn new() -> Self {
        Self::default()
    }

    /// The tangents of the first and the one under the cursor, and the one the clicks choose.
    fn tangents(&self) -> Option<(Vec<Tangent>, Option<Tangent>)> {
        let (_, first, p1) = self.first?;
        let (_, second) = self.under?;
        let hover = self.hover?;
        let all = common_tangents(&first, &second);
        let chosen = chosen_tangent(&all, p1, hover);
        Some((all, chosen))
    }
}

impl Tool for TangentLine {
    fn id(&self) -> &'static str {
        TANGENT_ID
    }

    fn label(&self) -> &'static str {
        TANGENT_LABEL
    }

    fn prompt(&self) -> Prompt {
        if self.first.is_none() {
            Prompt::new(
                TANGENT_LABEL,
                "ilk daireye ya da yaya, teğetin değeceği yerin yakınından tıklayın",
            )
        } else {
            Prompt::new(
                TANGENT_LABEL,
                "ikinci daireye ya da yaya, teğetin değeceği yerin yakınından tıklayın; Esc ilkini bıraktırır",
            )
        }
    }

    fn point_count(&self) -> usize {
        usize::from(self.first.is_some())
    }

    fn snaps(&self) -> bool {
        false
    }

    fn cursor(&self) -> Cursor {
        Cursor::Pick
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.hover = Some(p.raw);
        self.under = round_under(p, cx, self.first.map(|(slot, _, _)| slot));
        cx.selection.set_hover(self.under.map(|(slot, _)| slot));
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.hover = Some(p.raw);
        let Some((first_slot, first, p1)) = self.first else {
            match round_under(p, cx, None) {
                Some((slot, round)) => {
                    self.first = Some((slot, round, p.raw));
                    self.under = None;
                    cx.selection.set_hover(None);
                }
                None => cx.say(
                    Level::Warn,
                    "Tıklanan yerde daire ya da yay yok. Teğetin değeceği daireye ya da yaya tıklayın.",
                ),
            }
            return;
        };
        let Some((_, second)) = round_under(p, cx, Some(first_slot)) else {
            cx.say(
                Level::Warn,
                "Tıklanan yerde ikinci bir daire ya da yay yok. Teğetin değeceği ikinci daireye ya da yaya tıklayın.",
            );
            return;
        };
        let Some(t) = chosen_tangent(&common_tangents(&first, &second), p1, p.raw) else {
            cx.say(Level::Warn, no_tangent(&first, &second));
            return;
        };
        let line = EntityGeometry::Line {
            a: wire(t.a),
            b: wire(t.b),
            zs: None,
        };
        if points::write_objects(vec![line], Some(CreateOperation::TangentLine), cx).is_some() {
            cx.say(
                Level::Success,
                format!(
                    "{} eklendi: {}.",
                    tangent_name(&t),
                    cx.format().length(dist(t.a, t.b))
                ),
            );
        }
        self.first = None;
        self.under = None;
        cx.selection.set_hover(None);
    }

    fn input(&mut self, _text: &str, _cx: &mut Context<'_>) -> bool {
        false
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

    /// Where the first was clicked; over a second, the tangent the clicks
    /// choose bright, the others dashed, its kind and length beside the cursor.
    fn preview(&self, format: &Format) -> Preview {
        let mut preview = Preview::default();
        let Some((_, _, p1)) = self.first else {
            return preview;
        };
        preview.squares.push(p1);
        let Some((all, chosen)) = self.tangents() else {
            return preview;
        };
        for t in &all {
            if Some(*t) != chosen {
                preview
                    .strokes
                    .push(Stroke::dashed(vec![t.a, t.b], false, [4.0, 3.0]));
            }
        }
        let at = self.hover.unwrap_or(p1);
        match chosen {
            Some(t) => {
                preview
                    .strokes
                    .push(Stroke::solid(vec![t.a, t.b], false).width(1.5));
                preview.markers.extend([t.a, t.b].map(|at| Marker {
                    at,
                    shape: MarkerShape::Ring(4.5),
                    tone: Tone::Snap,
                }));
                preview.tag = Some(Tag {
                    at,
                    lines: vec![tangent_name(&t).to_owned(), format.length(dist(t.a, t.b))],
                });
            }
            None => {
                preview.tag_tone = Tone::Danger;
                preview.tag = Some(Tag {
                    at,
                    lines: vec!["Ortak teğet yok".to_owned()],
                });
            }
        }
        preview
    }
}

// ── Dördüncü köşe ───────────────────────────────────────────────────────

/// Çıktı's choices: its word typed, its label, its icon.
const OUTPUTS: [(bool, &str, &str, &str); 2] = [
    (false, "nokta", "Nokta", "point"),
    (true, "alan", "Alan", "polygon"),
];

/// Whether three corners stand on one line: no parallelogram.
fn in_line(a: Vec2, b: Vec2, c: Vec2) -> bool {
    let cross = (b.x - a.x) * (c.y - b.y) - (b.y - a.y) * (c.x - b.x);
    let scale = dist(a, b) * dist(b, c);
    cross.abs() <= 1e-9 * scale
}

/// Dördüncü köşe.
#[derive(Clone, Debug, Default)]
pub struct FourthCorner {
    d: Taken,
    seen: Option<Memory>,
}

impl FourthCorner {
    pub fn new() -> Self {
        Self::default()
    }

    fn area(&self) -> bool {
        self.seen.is_some_and(|m| m.fourth_area)
    }

    fn accept(&mut self, p: Vec2, cx: &mut Context<'_>) {
        self.seen = Some(*cx.memory);
        if self.d.last().is_some_and(|q| dist(q, p) < points::SAME) {
            cx.say(
                Level::Warn,
                "Köşe bir öncekiyle çakışıyor; başka bir yer gösterin.",
            );
            return;
        }
        self.d.begin(p, cx);
        self.d.pts.push(p);
        if self.d.pts.len() == 3 {
            self.write(cx);
        }
    }

    /// The fourth corner of the three in, as a point or the four as an area; the corners start over.
    fn write(&mut self, cx: &mut Context<'_>) {
        let [a, b, c] = [self.d.pts[0], self.d.pts[1], self.d.pts[2]];
        self.d.reset();
        if in_line(a, b, c) {
            cx.say(
                Level::Warn,
                "Üç köşe bir doğru üzerinde: paralelkenar olmaz. Köşeleri yeniden gösterin.",
            );
            return;
        }
        let d = fourth_corner(a, b, c);
        let geometry = if self.area() {
            EntityGeometry::Polygon {
                pts: points::wire_all(&[a, b, c, d]),
                bulges: None,
                holes: None,
                zs: None,
                parts: None,
            }
        } else {
            EntityGeometry::Point {
                p: wire(d),
                z: None,
                parts: None,
            }
        };
        if points::write_objects(vec![geometry], Some(CreateOperation::FourthCorner), cx).is_some()
        {
            let f = cx.format();
            let what = if self.area() {
                "Paralelkenar alanı eklendi"
            } else {
                "Dördüncü köşe kondu"
            };
            cx.say(Level::Success, format!("{what}: {}.", f.point(d)));
        }
    }
}

impl Tool for FourthCorner {
    fn id(&self) -> &'static str {
        FOURTH_ID
    }

    fn label(&self) -> &'static str {
        FOURTH_LABEL
    }

    fn prompt(&self) -> Prompt {
        let step = match self.d.pts.len() {
            0 => "birinci köşeyi belirtin",
            1 => "ortadaki köşeyi belirtin (dördüncünün karşısı)",
            _ => "üçüncü köşeyi belirtin",
        };
        let output = if self.area() { "Alan" } else { "Nokta" };
        Prompt::new(FOURTH_LABEL, step)
            .then()
            .option_with("Çıktı", "Ç", output)
    }

    fn point_count(&self) -> usize {
        self.d.pts.len()
    }

    fn activate(&mut self, cx: &mut Context<'_>) -> Flow {
        self.seen = Some(*cx.memory);
        Flow::Stay
    }

    fn snap_from(&self) -> Option<Vec2> {
        self.d.last()
    }

    fn accepts_points(&self) -> bool {
        true
    }

    fn accept_point(&mut self, p: Vec2, cx: &mut Context<'_>) -> bool {
        self.accept(p, cx);
        true
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.seen = Some(*cx.memory);
        self.d.hover = Some(self.d.constrain(p, cx));
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        let point = self.d.constrain(p, cx);
        self.accept(point, cx);
    }

    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        let typed = js_trim(text);
        if upper_tr(typed) == "Ç" {
            cx.memory.fourth_area = !cx.memory.fourth_area;
            self.seen = Some(*cx.memory);
            return true;
        }
        let Some(p) = cx.typed_point(text, self.d.last(), self.d.hover) else {
            return false;
        };
        self.accept(p, cx);
        true
    }

    fn option_choices(&self, key: &str) -> Vec<OptionChoice> {
        if key != "Ç" {
            return Vec::new();
        }
        OUTPUTS
            .iter()
            .map(|&(area, typed, label, icon)| OptionChoice {
                label: label.to_owned(),
                typed: typed.to_owned(),
                icon: Some(icon),
                preview: None,
                checked: area == self.area(),
                command: None,
            })
            .collect()
    }

    fn choose_option(&mut self, key: &str, typed: &str, cx: &mut Context<'_>) -> bool {
        let word = upper_tr(js_trim(typed));
        let Some(&(area, ..)) = OUTPUTS
            .iter()
            .find(|(_, w, ..)| key == "Ç" && upper_tr(w) == word)
        else {
            return false;
        };
        cx.memory.fourth_area = area;
        self.seen = Some(*cx.memory);
        true
    }

    /// Corners in: they are let go; none: the tool leaves.
    fn confirm(&mut self, _cx: &mut Context<'_>) -> Flow {
        if self.d.pts.is_empty() {
            return Flow::Exit;
        }
        self.d.reset();
        Flow::Stay
    }

    fn cancel(&mut self, _cx: &mut Context<'_>) -> bool {
        self.d.pts.pop().is_some()
    }

    fn undo_step(&mut self, cx: &mut Context<'_>) -> bool {
        self.d.undo_step(true, cx)
    }

    /// The corners so far to the cursor; with two in, the parallelogram the
    /// cursor makes, dashed, its fourth corner marked.
    fn preview(&self, format: &Format) -> Preview {
        let mut preview = Preview {
            tracking: self.d.tracking,
            ..Preview::default()
        };
        let Some(hover) = self.d.hover else {
            return preview;
        };
        let chain: Vec<Vec2> = self.d.pts.iter().copied().chain([hover]).collect();
        preview.markers.extend(self.d.pts.iter().map(|&at| Marker {
            at,
            shape: MarkerShape::Ring(4.5),
            tone: Tone::Snap,
        }));
        if let [a, b] = self.d.pts[..] {
            if in_line(a, b, hover) {
                preview
                    .strokes
                    .push(Stroke::dashed(chain, false, [5.0, 3.0]));
                return preview;
            }
            let d = fourth_corner(a, b, hover);
            preview
                .strokes
                .push(Stroke::dashed(vec![a, b, hover, d], true, [5.0, 3.0]));
            preview.markers.push(Marker {
                at: d,
                shape: MarkerShape::Ring(7.5),
                tone: Tone::Accent,
            });
            preview.tag = Some(Tag {
                at: d,
                lines: vec!["Dördüncü köşe".to_owned(), format.point(d)],
            });
        } else if chain.len() > 1 {
            preview
                .strokes
                .push(Stroke::dashed(chain, false, [5.0, 3.0]));
        }
        preview
    }
}

// ── Menzil halkaları ────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Asking {
    /// The centre.
    #[default]
    Centre,
    /// Aralık typed.
    Spacing,
    /// Sayı typed.
    Count,
    /// Işın typed.
    Rays,
}

/// Menzil halkaları.
#[derive(Clone, Debug, Default)]
pub struct RangeRings {
    d: Taken,
    asking: Asking,
    seen: Option<(Memory, Format)>,
}

impl RangeRings {
    pub fn new() -> Self {
        Self::default()
    }

    fn see(&mut self, cx: &Context<'_>) {
        self.seen = Some((*cx.memory, cx.format()));
    }

    fn memory(&self) -> Memory {
        self.seen.map(|(m, _)| m).unwrap_or_default()
    }

    /// The rings and rays round `c` at the kept values.
    fn objects(&self, c: Vec2) -> Option<(Vec<EntityGeometry>, usize, usize)> {
        let m = self.memory();
        let rings = range_rings(c, m.ring_spacing, m.ring_count, m.ring_rays)?;
        let (n, k) = (rings.radii.len(), rings.rays.len());
        let circles = rings
            .radii
            .iter()
            .map(|&r| EntityGeometry::Circle { c: wire(c), r });
        let rays = rings.rays.iter().map(|&end| EntityGeometry::Line {
            a: wire(c),
            b: wire(end),
            zs: None,
        });
        Some((circles.chain(rays).collect(), n, k))
    }

    fn write(&mut self, c: Vec2, cx: &mut Context<'_>) {
        self.see(cx);
        self.d.begin(c, cx);
        let Some((objects, n, k)) = self.objects(c) else {
            return;
        };
        if points::write_objects(objects, Some(CreateOperation::RangeRings), cx).is_some() {
            let what = if k == 0 {
                format!("{n} halka eklendi.")
            } else {
                format!("{n} halka ve {k} ışın eklendi.")
            };
            cx.say(Level::Success, what);
        }
    }

    /// A value typed for the value asked; false when it is not a number.
    fn value(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        let Some(n) = plain_number(text) else {
            return false;
        };
        match self.asking {
            Asking::Spacing => {
                let metres = cx.format().to_metres(n);
                if !(metres > 0.0 && metres.is_finite()) {
                    cx.say(Level::Warn, "Aralık sıfırdan büyük olmalı.");
                    return true;
                }
                cx.memory.ring_spacing = metres;
            }
            Asking::Count => {
                if n.fract() != 0.0 || !(1.0..=f64::from(MAX_RINGS)).contains(&n) {
                    cx.say(
                        Level::Warn,
                        "Halka sayısı 1 ile 100 arasında bir tam sayı olmalı.",
                    );
                    return true;
                }
                cx.memory.ring_count = n as u32;
            }
            Asking::Rays => {
                if n.fract() != 0.0 || !(0.0..=f64::from(MAX_RAYS)).contains(&n) {
                    cx.say(
                        Level::Warn,
                        "Işın sayısı 0 ile 360 arasında bir tam sayı olmalı (0: ışın yok).",
                    );
                    return true;
                }
                cx.memory.ring_rays = n as u32;
            }
            Asking::Centre => return false,
        }
        self.asking = Asking::Centre;
        self.see(cx);
        true
    }
}

impl Tool for RangeRings {
    fn id(&self) -> &'static str {
        RINGS_ID
    }

    fn label(&self) -> &'static str {
        RINGS_LABEL
    }

    fn prompt(&self) -> Prompt {
        let (m, f) = self.seen.unwrap_or_default();
        match self.asking {
            Asking::Centre => Prompt::new(RINGS_LABEL, "merkezi belirtin")
                .then()
                .option_with("Aralık", "A", f.length(m.ring_spacing))
                .option_with("Sayı", "S", m.ring_count.to_string())
                .option_with("Işın", "I", m.ring_rays.to_string()),
            Asking::Spacing => Prompt::new(
                RINGS_LABEL,
                format!(
                    "halkaların aralığını {} olarak yazın (Enter: {})",
                    f.length_unit_label(),
                    f.plain(m.ring_spacing)
                ),
            ),
            Asking::Count => Prompt::new(
                RINGS_LABEL,
                format!(
                    "halka sayısını yazın, 1 ile 100 arası (Enter: {})",
                    m.ring_count
                ),
            ),
            Asking::Rays => Prompt::new(
                RINGS_LABEL,
                format!(
                    "ışın sayısını yazın, 0 ile 360 arası; 0 ışınsız (Enter: {})",
                    m.ring_rays
                ),
            ),
        }
    }

    fn point_count(&self) -> usize {
        0
    }

    fn activate(&mut self, cx: &mut Context<'_>) -> Flow {
        self.see(cx);
        Flow::Stay
    }

    fn accepts_points(&self) -> bool {
        self.asking == Asking::Centre
    }

    fn accept_point(&mut self, p: Vec2, cx: &mut Context<'_>) -> bool {
        if self.asking != Asking::Centre {
            return false;
        }
        self.write(p, cx);
        true
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.see(cx);
        self.d.hover = Some(self.d.constrain(p, cx));
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        if self.asking != Asking::Centre {
            self.asking = Asking::Centre;
        }
        let point = self.d.constrain(p, cx);
        self.write(point, cx);
    }

    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        self.see(cx);
        let typed = js_trim(text);
        let asked = match upper_tr(typed).as_str() {
            "A" => Some(Asking::Spacing),
            "S" => Some(Asking::Count),
            "I" => Some(Asking::Rays),
            _ => None,
        };
        if let Some(asked) = asked {
            self.asking = asked;
            return true;
        }
        if self.asking != Asking::Centre {
            return self.value(typed, cx);
        }
        let Some(p) = cx.typed_point(text, None, self.d.hover) else {
            return false;
        };
        self.write(p, cx);
        true
    }

    /// A value asked: it stays as it was; else the tool leaves.
    fn confirm(&mut self, _cx: &mut Context<'_>) -> Flow {
        if self.asking != Asking::Centre {
            self.asking = Asking::Centre;
            return Flow::Stay;
        }
        Flow::Exit
    }

    fn cancel(&mut self, _cx: &mut Context<'_>) -> bool {
        if self.asking == Asking::Centre {
            return false;
        }
        self.asking = Asking::Centre;
        true
    }

    fn undo_step(&mut self, _cx: &mut Context<'_>) -> bool {
        false
    }

    /// The rings and rays round the cursor, the outer radius beside it.
    fn preview(&self, format: &Format) -> Preview {
        let mut preview = Preview {
            tracking: self.d.tracking,
            ..Preview::default()
        };
        let Some(c) = self.d.hover else {
            return preview;
        };
        let m = self.memory();
        let Some(rings) = range_rings(c, m.ring_spacing, m.ring_count, m.ring_rays) else {
            return preview;
        };
        preview.strokes.extend(
            rings
                .radii
                .iter()
                .map(|&r| Stroke::solid(tessellate_circle(c, r, RING_SEGMENTS), true)),
        );
        preview.strokes.extend(
            rings
                .rays
                .iter()
                .map(|&end| Stroke::dashed(vec![c, end], false, [5.0, 3.0])),
        );
        preview.markers.push(Marker {
            at: c,
            shape: MarkerShape::Ring(4.5),
            tone: Tone::Snap,
        });
        let outer = rings.radii.last().copied().unwrap_or(0.0);
        preview.tag = Some(Tag {
            at: c,
            lines: vec![
                format!("{} × {}", rings.radii.len(), format.length(m.ring_spacing)),
                format!("dış yarıçap {}", format.length(outer)),
            ],
        });
        preview
    }
}
