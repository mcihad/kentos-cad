//! Paralel kaydır (docs/adr/0191; Netcad's Paralel Kaydır and Alan
//! Düzeltme (Paralel); the web's `EdgeShiftTool`,
//! `apps/web/src/tools/edgeShiftTool.ts`): an area's or a polyline's
//! straight edge moved parallel to itself, its neighbours lengthened or
//! shortened to meet it (the core's `ops::edge_shift`). The edge is clicked;
//! it follows the cursor (the parallel line through the cursor's point,
//! snaps on), the distance and an area's new size beside it. A click there,
//! a typed distance, or Alan (A) and a target size writes it through
//! `cad.entities.edit` (step “Paralel kaydır”) and the tool waits for the
//! next edge. Esc and Ctrl+Z let the edge go; Esc with none leaves. While
//! Topolojik düzenleme is on (docs/adr/0160, 1960) the objects sharing the
//! edge's corners move with it: the preview draws them and a neighbour
//! area's new size, the command writes them in the same step.
//!
//! Referans (R, docs/adr/2580): a parallel straight edge of another object is clicked; the distance shown and typed is
//! then the edge's distance from it (“şimdi bu değere getir”). Koru (K) turns Bu uzaklığı koru on: the distance is
//! written as a map bond, so the edge follows when the reference moves (“bu ilişkiyi koru”); Enter keeps the distance
//! there is now. The reference stays for the next edge while it is parallel.

use kentos_commands::{ConstraintRule, Entity, ShapeWork, ShiftReference, StretchTopology};
use kentos_domain::{Document, Slot};
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::ops::arch_bonds::PARALLEL;
use kentos_geometry_core::ops::edge_shift::{self as rules, Picked};
use kentos_geometry_core::ops::subref::{EdgeKind, Place, edge_line, edge_place_near};
use kentos_geometry_core::tools::point_text::{js_trim, parse_number};
use kentos_native_application::geometry::shape;

use crate::Vec2;
use crate::edge::{self, Outline};
use crate::format::Format;
use crate::log::Level;
use crate::points;
use crate::prompt::{Prompt, upper_tr};
use crate::tool::{Context, Cursor, Flow, Marker, MarkerShape, Pointer, Preview, Stroke, Tag, Tone, Tool};

/// The tool's id: its command is `tool.edgeShift`.
pub const ID: &str = "edgeShift";
pub const LABEL: &str = "Paralel kaydır";
/// A click where no area's or polyline's edge is.
pub const NO_EDGE_HERE: &str = "Tıklanan yerde kaydırılacak kenar yok; kilitsiz bir alanın ya da çoklu çizginin kenarına tıklayın.";
/// A click for the reference where no straight edge is.
pub const NO_REFERENCE_HERE: &str =
    "Tıklanan yerde düz kenar yok; referans bir çizgi ya da bir yolun veya alanın düz kenarıdır.";

/// The edge picked: its object, the object's shape then and how it moves.
#[derive(Clone, Debug)]
struct Picking {
    slot: Slot,
    shape: Shape,
    picked: Picked,
    /// The area's size then (none: a polyline).
    area: Option<f64>,
}

/// The reference the edge keeps its distance from (docs/adr/2580): its object, its straight segment's ends and the
/// point that names the segment to the command.
#[derive(Clone, Copy, Debug)]
struct Reference {
    slot: Slot,
    a: Vec2,
    b: Vec2,
    at: Vec2,
}

impl Reference {
    fn unit(&self) -> Vec2 {
        let l = (self.b.x - self.a.x).hypot(self.b.y - self.a.y);
        Vec2::new((self.b.x - self.a.x) / l, (self.b.y - self.a.y) / l)
    }

    /// A point's signed distance from the reference's line, its left positive.
    fn across(&self, p: Vec2) -> f64 {
        let u = self.unit();
        -u.y * (p.x - self.a.x) + u.x * (p.y - self.a.y)
    }

    /// The foot of `p` on the reference's line.
    fn foot(&self, p: Vec2) -> Vec2 {
        let u = self.unit();
        let t = u.x * (p.x - self.a.x) + u.y * (p.y - self.a.y);
        Vec2::new(self.a.x + u.x * t, self.a.y + u.y * t)
    }
}

/// How the edge moves: by a shift along its normal, or to a distance from the reference (none: where it is, the bond
/// only).
#[derive(Clone, Copy, Debug)]
enum Amount {
    Shift(f64),
    FromReference(Option<f64>),
}

/// Where the cursor moves the edge: the place, the distance, the shape moved there (or why not) and, while
/// Topolojik düzenleme is on, the neighbours moved with it (their shapes, each area's size before and after) and their
/// vertices that move, where they go (docs/adr/3000).
type At = (Vec2, f64, Result<Shape, String>, Vec<(Shape, Option<(f64, f64)>)>, Vec<Vec2>);

/// A shape's vertices in order: a point's, a line's ends, a path's or an area's outer ring and holes.
fn vertices(s: &Shape) -> Vec<Vec2> {
    match s {
        Shape::Point { p, .. } => vec![*p],
        Shape::Line { a, b } => vec![*a, *b],
        Shape::Polyline { pts, holes, .. } | Shape::Polygon { pts, holes, .. } => {
            let mut out = pts.clone();
            for h in holes.iter().flatten() {
                out.extend(h.pts.iter().copied());
            }
            out
        }
        _ => Vec::new(),
    }
}

/// The places a neighbour's vertices move to (docs/adr/3000): each vertex of `now` that is not where `then` had it,
/// the two shapes having the same vertices in the same order (as the topology's follow writes them).
fn moved_vertices(then: &Shape, now: &Shape, out: &mut Vec<Vec2>) {
    let (a, b) = (vertices(then), vertices(now));
    if a.len() != b.len() {
        return;
    }
    for (p, q) in a.iter().zip(&b) {
        let gone = (q.x - p.x).abs() > 1e-9 || (q.y - p.y).abs() > 1e-9;
        if gone && !out.iter().any(|o| (o.x - q.x).abs() <= 1e-9 && (o.y - q.y).abs() <= 1e-9) {
            out.push(*q);
        }
    }
}

pub struct EdgeShift {
    edge: Option<Picking>,
    asking: bool,
    /// Referans (R): the next click picks the reference.
    referencing: bool,
    reference: Option<Reference>,
    /// Koru (K): Bu uzaklığı koru.
    keep: bool,
    at: Option<At>,
    format: Format,
}

impl Default for EdgeShift {
    fn default() -> Self {
        Self::new()
    }
}

/// An area or a polyline, not on a locked layer: what the tool shifts.
fn editable(e: &Entity, doc: &Document) -> bool {
    matches!(e, Entity::Polygon(_) | Entity::Polyline(_)) && edge::unlocked(e, doc)
}

/// A signed amount in the project's units, its sign always shown.
fn signed(text: String, v: f64) -> String {
    if v > 0.0 { format!("+{text}") } else { text }
}

/// The sine of the angle between a picked edge and a reference.
fn sine(k: &Picked, r: &Reference) -> f64 {
    let l = (k.b.x - k.a.x).hypot(k.b.y - k.a.y);
    let w = Vec2::new((k.b.x - k.a.x) / l, (k.b.y - k.a.y) / l);
    let u = r.unit();
    (u.x * w.y - u.y * w.x).abs()
}

impl EdgeShift {
    pub fn new() -> Self {
        Self {
            edge: None,
            asking: false,
            referencing: false,
            reference: None,
            keep: false,
            at: None,
            format: Format::default(),
        }
    }

    /// The edge under the click, or why none is taken. A reference not parallel to it is let go.
    fn pick(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        let Some(slot) = edge::pick_said(p, cx, editable) else {
            cx.say(Level::Warn, NO_EDGE_HERE);
            return;
        };
        let Some(e) = cx.doc.get(slot) else {
            return;
        };
        let geom = shape(e);
        match rules::pick(&geom, p.raw) {
            Ok(picked) => {
                if let Some(r) = self.reference
                    && (sine(&picked, &r) > PARALLEL || picked.bulge != 0.0)
                {
                    self.reference = None;
                    self.keep = false;
                    cx.say(Level::Info, "Referans bu kenara paralel değil; referans bırakıldı.");
                }
                self.edge = Some(Picking {
                    slot,
                    area: rules::area_of(&geom),
                    shape: geom,
                    picked,
                });
                cx.selection.set_hover(None);
            }
            Err(why) => cx.say(Level::Warn, why),
        }
    }

    /// The reference under the click (docs/adr/2580): the nearest straight edge of any object there, parallel to the
    /// picked edge within 0.001° and not the edge itself; or why none is taken (the tool keeps asking).
    fn pick_reference(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        let Some(picking) = &self.edge else {
            return;
        };
        let found = edge::pick(p, cx, |_, _| true).and_then(|slot| {
            let core = edge::core(cx.doc.get(slot)?);
            let place = edge_place_near(&core, EdgeKind::Straight, p.raw)?;
            let (a, b) = edge_line(&core, place)?;
            let index = match place {
                Place::Along {
                    part, ring, index, ..
                }
                | Place::Mid { part, ring, index } => (part, ring, index),
                _ => (0, 0, 0),
            };
            (a != b).then_some((slot, a, b, index))
        });
        let Some((slot, a, b, (part, ring, index))) = found else {
            cx.say(Level::Warn, NO_REFERENCE_HERE);
            return;
        };
        let k = &picking.picked;
        if slot == picking.slot && (part, ring, index) == (0, k.ring, k.edge) {
            cx.say(Level::Warn, "Kenar kendisinin referansı olamaz; başka bir kenar seçin.");
            return;
        }
        let r = Reference {
            slot,
            a,
            b,
            at: Vec2::new((a.x + b.x) / 2.0, (a.y + b.y) / 2.0),
        };
        if k.bulge != 0.0 {
            cx.say(
                Level::Warn,
                "Yaylı kenarın bir doğruya uzaklığı tek bir sayı değil; referansla yalnız düz kenar kaydırılır.",
            );
            self.referencing = false;
            return;
        }
        let s = sine(k, &r);
        if s > PARALLEL {
            cx.say(
                Level::Warn,
                format!(
                    "Kenar referansa paralel değil (aralarında {:.4}° var); uzaklık yalnız 0,001° içinde paralel kenarlar arasında tutulur.",
                    s.min(1.0).asin().to_degrees()
                ),
            );
            return;
        }
        self.referencing = false;
        self.reference = Some(r);
        let now = r.across(k.a).abs();
        cx.say(
            Level::Info,
            format!(
                "Referans seçildi: kenar şimdi {} uzakta. Uzaklığı yazın ya da yerine tıklayın; Koru (K) uzaklığı kalıcı bağ yapar.",
                cx.format().length(now)
            ),
        );
    }

    /// The point's distance from the picked edge along its normal; none
    /// with no edge picked.
    fn distance_at(&self, p: Vec2) -> Option<f64> {
        // From the edge's middle (an arc's offset is radial there, docs/adr/1024).
        let k = &self.edge.as_ref()?.picked;
        Some((p.x - k.mid.x) * k.normal.x + (p.y - k.mid.y) * k.normal.y)
    }

    /// The point's distance from the reference on the edge's side (negative: past the reference); none without one.
    fn from_reference(&self, p: Vec2) -> Option<f64> {
        let (k, r) = (&self.edge.as_ref()?.picked, self.reference?);
        let side = if r.across(k.a) >= 0.0 { 1.0 } else { -1.0 };
        Some(side * r.across(p))
    }

    /// Writes the edge moved by `amount` and says so; the tool waits for the
    /// next edge (the reference stays). Whether it was written.
    fn write(&mut self, amount: Amount, cx: &mut Context<'_>) -> bool {
        let Some(picking) = &self.edge else {
            return false;
        };
        let k = &picking.picked;
        // The distance a bond keeps: the one given, or the one there is now.
        let kept = match (amount, self.reference) {
            (Amount::FromReference(Some(d)), _) => d,
            (Amount::FromReference(None), Some(r)) => r.across(k.a).abs(),
            _ => 0.0,
        };
        let (distance, reference) = match (amount, self.reference) {
            (Amount::FromReference(d), Some(r)) => (
                d,
                Some(ShiftReference {
                    id: edge::uid(cx.doc, r.slot),
                    at: points::wire(r.at),
                    keep: self.keep,
                }),
            ),
            (Amount::Shift(d), _) => (Some(d), None),
            (Amount::FromReference(_), None) => return false,
        };
        // The picked edge's middle names it (an arc's on the arc); the command picks the same nearest edge
        // (docs/adr/0622 §9, 1024).
        let work = ShapeWork::EdgeShift {
            id: edge::uid(cx.doc, picking.slot),
            at: points::wire(k.mid),
            distance,
            area: None,
            topology: cx.draft.topology.then_some(StretchTopology {
                points: cx.draft.topology_points,
                stop_on_locked: cx.draft.topology_stop_on_locked,
            }),
            reference,
        };
        let Some(out) = points::write_shape(work, cx) else {
            return false;
        };
        if !out.neighbours.is_empty() {
            let inside = match out.carried {
                0 => String::new(),
                n => format!("; kenarın ortasındaki {n} ortak köşe de kenarla kaydı"),
            };
            cx.say(
                Level::Info,
                format!("Topolojik düzenleme: {} komşu nesne de değişti{inside}.", out.neighbours.len()),
            );
        }
        let moved_area = picking.area.map(|_| out.after);
        let format = cx.format();
        let area = moved_area.map(|a| format!(", alan {}", format.area(a))).unwrap_or_default();
        // The bond's parameter, as the Kısıtlar panel names it.
        let parameter = out.bonds.first().and_then(|id| {
            cx.doc
                .constraints()
                .constraints
                .iter()
                .find(|c| c.id.to_string() == *id)
                .and_then(|c| match &c.rule {
                    ConstraintRule::Distance { parameter, .. } => Some(parameter.clone()),
                    _ => None,
                })
        });
        let said = match (amount, parameter) {
            (Amount::FromReference(_), Some(p)) => format!(
                "{LABEL}: kenar referanstan {} uzakta{area}; uzaklık {p} olarak korunuyor, referans kayınca kenar izler.",
                format.length(kept)
            ),
            (Amount::FromReference(Some(d)), None) => {
                format!("{LABEL}: kenar referanstan {} uzağa kaydırıldı{area}.", format.length(d))
            }
            (Amount::FromReference(None), None) => format!("{LABEL}: kenar yerinde kaldı."),
            (Amount::Shift(d), _) => format!("{LABEL}: kenar {} kaydırıldı{area}.", format.length(d)),
        };
        cx.say(Level::Success, said);
        self.edge = None;
        self.at = None;
        true
    }
}

impl Tool for EdgeShift {
    fn id(&self) -> &'static str {
        ID
    }

    fn label(&self) -> &'static str {
        LABEL
    }

    fn prompt(&self) -> Prompt {
        if self.asking {
            return Prompt::new(
                LABEL,
                format!("hedef alanı yazın ({})", self.format.area_unit_label()),
            );
        }
        if self.referencing {
            return Prompt::new(
                LABEL,
                "referans kenara tıklayın (paralel bir çizgi ya da yolun veya alanın düz kenarı)",
            );
        }
        match &self.edge {
            None => Prompt::new(
                LABEL,
                "kaydırılacak kenara tıklayın (alan ya da çoklu çizgi)",
            ),
            Some(_) if self.reference.is_some() => {
                let prompt = Prompt::new(LABEL, "referansa uzaklığı yazın ya da yerine tıklayın")
                    .option_with("Referans", "R", "seçili")
                    .toggle("Koru", "K", self.keep);
                if self.keep {
                    prompt.option("Şimdikini koru", "Enter")
                } else {
                    prompt
                }
            }
            Some(picking) => {
                let step = if picking.area.is_some() {
                    "uzaklığı yazın ya da yerine tıklayın (dışarı artı)"
                } else {
                    "uzaklığı yazın ya da yerine tıklayın (sağa artı)"
                };
                let prompt = Prompt::new(LABEL, step);
                let prompt = if picking.area.is_some() {
                    prompt.option("Alan", "A")
                } else {
                    prompt
                };
                prompt.option("Referans", "R")
            }
        }
    }

    fn point_count(&self) -> usize {
        usize::from(self.edge.is_some())
    }

    fn activate(&mut self, cx: &mut Context<'_>) -> Flow {
        self.format = cx.format();
        Flow::Stay
    }

    /// The cursor snaps once the edge is picked: the edge goes through the point.
    fn snaps(&self) -> bool {
        self.edge.is_some() && !self.referencing
    }

    fn cursor(&self) -> Cursor {
        if self.edge.is_some() && !self.referencing {
            Cursor::Cross
        } else {
            Cursor::Pick
        }
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.format = cx.format();
        if self.referencing {
            self.at = None;
            return;
        }
        let Some(d) = self.distance_at(p.world) else {
            edge::hover(p, cx, editable);
            return;
        };
        let Some(picking) = self.edge.as_ref() else {
            return;
        };
        let k = &picking.picked;
        let mut moved = rules::shifted(&picking.shape, k.ring, k.edge, d);
        // The neighbours that go with it while Topolojik düzenleme is on (the command's own rule), the vertices inside
        // the shared edge too (docs/adr/2600); the command's refusal when one cannot go.
        let found = match &moved {
            Ok(after) => crate::neighbours::shifted_neighbours(cx, picking.slot, &picking.shape, after, (k.ring, k.edge)),
            Err(_) => Ok(None),
        };
        let found = match found {
            Ok(n) => n,
            Err(why) => {
                moved = Err(why.to_owned());
                None
            }
        };
        let mut carried = Vec::new();
        let around = match &moved {
            Ok(_) => found
                .map(|n| {
                    n.changes
                        .iter()
                        .zip(n.shapes)
                        .map(|(change, now)| {
                            let then = match change {
                                kentos_commands::EntityEdit::Update { uid, .. } => uid
                                    .parse::<kentos_domain::Uuid>()
                                    .ok()
                                    .and_then(|u| cx.doc.slot_of(u))
                                    .and_then(|s| cx.doc.get(s))
                                    .map(shape),
                                _ => None,
                            };
                            if let Some(t) = &then {
                                moved_vertices(t, &now, &mut carried);
                            }
                            let sizes = then.and_then(|t| Some((rules::area_of(&t)?, rules::area_of(&now)?)));
                            (now, sizes)
                        })
                        .collect()
                })
                .unwrap_or_default(),
            Err(_) => Vec::new(),
        };
        self.at = Some((p.world, d, moved, around, carried));
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        if self.asking {
            return;
        }
        if self.referencing {
            self.pick_reference(p, cx);
            return;
        }
        match (self.distance_at(p.world), self.from_reference(p.world)) {
            (None, _) => self.pick(p, cx),
            // Past the reference the edge would leave its side: said, nothing written.
            (Some(_), Some(from)) if from < 0.0 => cx.say(
                Level::Warn,
                "İmleç referansın öbür yanında; kenar bulunduğu yanda kalır. Referansın kenar yanına tıklayın.",
            ),
            (Some(_), Some(from)) => {
                self.write(Amount::FromReference(Some(from)), cx);
            }
            (Some(d), None) => {
                self.write(Amount::Shift(d), cx);
            }
        }
    }

    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        let t = js_trim(text);
        let Some((shape, ring, index, has_area)) = self.edge.as_ref().map(|picking| {
            let k = &picking.picked;
            (
                picking.shape.clone(),
                k.ring,
                k.edge,
                picking.area.is_some(),
            )
        }) else {
            return false;
        };
        if self.asking {
            let format = cx.format();
            match parse_number(t) {
                Some(n) if n > 0.0 && n.is_finite() => {
                    let target = format.area_to_square_metres(n);
                    match rules::for_area(&shape, ring, index, target) {
                        Ok(d) => {
                            self.asking = false;
                            self.write(Amount::Shift(d), cx);
                        }
                        Err(why) => cx.say(Level::Warn, why),
                    }
                }
                _ => cx.say(
                    Level::Warn,
                    format!("Hedef alan sıfırdan büyük bir sayı olmalı; “{t}” yazıldı."),
                ),
            }
            return true;
        }
        match upper_tr(t).as_str() {
            "A" => {
                if !has_area {
                    cx.say(Level::Warn, rules::NOT_AREA);
                } else if self.reference.is_some() {
                    cx.say(Level::Warn, "Referansla hedef alan verilmez; referansa uzaklığı yazın.");
                } else {
                    self.asking = true;
                }
                return true;
            }
            // Referans (docs/adr/2580): the next click picks it, again when one is chosen.
            "R" => {
                self.referencing = true;
                self.at = None;
                return true;
            }
            "K" => {
                if self.reference.is_some() {
                    self.keep = !self.keep;
                } else {
                    cx.say(
                        Level::Warn,
                        "Bu uzaklığı koru bir referansla olur: önce Referans (R) ile paralel bir kenar seçin.",
                    );
                }
                return true;
            }
            _ => {}
        }
        match cx.typed_length(t) {
            Some(d) if d.is_finite() && self.reference.is_some() => {
                if d < 0.0 {
                    cx.say(
                        Level::Warn,
                        "Referansa uzaklık eksi olamaz: kenar bulunduğu yanda kalır.",
                    );
                } else {
                    self.write(Amount::FromReference(Some(d)), cx);
                }
                true
            }
            Some(d) if d.is_finite() => {
                self.write(Amount::Shift(d), cx);
                true
            }
            _ => false,
        }
    }

    /// Enter: Alan's question or the reference's ends; with Koru on, the distance there is now is kept as a bond;
    /// else the tool leaves (a click or a typed distance writes).
    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow {
        if std::mem::take(&mut self.asking) || std::mem::take(&mut self.referencing) {
            return Flow::Stay;
        }
        if self.keep && self.reference.is_some() && self.edge.is_some() {
            self.write(Amount::FromReference(None), cx);
            return Flow::Stay;
        }
        Flow::Exit
    }

    /// Esc: Alan's question or the reference's, then the edge, then the reference go first.
    fn cancel(&mut self, cx: &mut Context<'_>) -> bool {
        if std::mem::take(&mut self.asking) || std::mem::take(&mut self.referencing) {
            return true;
        }
        if self.edge.is_none() && self.reference.take().is_some() {
            self.keep = false;
            return true;
        }
        self.undo_step(cx)
    }

    /// Ctrl+Z: the edge goes; with none the drawing is undone.
    fn undo_step(&mut self, _cx: &mut Context<'_>) -> bool {
        self.asking = false;
        self.referencing = false;
        self.at = None;
        self.edge.take().is_some()
    }

    fn preview(&self, format: &Format) -> Preview {
        let mut out = Preview::default();
        let Some(picking) = &self.edge else {
            return out;
        };
        let k = &picking.picked;
        // The edge as it is, then the shape moved to the cursor.
        // An arc edge drawn as its arc (docs/adr/1024).
        let edge_pts = kentos_geometry_core::geom::bulge::bulge_path_outline(
            &[k.a, k.b],
            Some(&[k.bulge]),
            false,
            std::f64::consts::PI / 90.0,
        );
        out.strokes.push(Stroke::solid(edge_pts, false).width(2.0).tone(Tone::Snap));
        // The reference, the line the edge keeps its distance from (docs/adr/2580): long dashes, stays where it is.
        if let Some(r) = self.reference {
            out.strokes.push(Stroke {
                pts: vec![r.a, r.b],
                closed: false,
                dash: Some([10.0, 4.0]),
                width: 2.0,
                tone: Tone::Snap,
            });
        }
        let Some((at, d, moved, around, carried)) = &self.at else {
            return out;
        };
        let mut lines = match self.from_reference(*at) {
            Some(from) => {
                let mut l = vec![format!("referanstan {}", format.length(from))];
                if self.keep {
                    l.push("uzaklık korunacak".into());
                }
                l
            }
            None => vec![format.length(*d)],
        };
        // From the reference to the cursor, square to it: the distance asked.
        if let Some(r) = self.reference {
            out.strokes.push(Stroke {
                pts: vec![r.foot(*at), *at],
                closed: false,
                dash: Some([3.0, 3.0]),
                width: 1.0,
                tone: Tone::Accent,
            });
        }
        match moved {
            Ok(moved) if self.from_reference(*at).is_none_or(|f| f >= 0.0) => {
                let outline = Outline::of(moved, Some([6.0, 4.0]), 1.0, Tone::Accent);
                out.strokes.extend(outline.strokes);
                if let (Some(before), Some(after)) = (picking.area, rules::area_of(moved)) {
                    lines.push(format!(
                        "{} ({})",
                        format.area(after),
                        signed(format.area_bare(after - before), after - before)
                    ));
                }
                // The neighbours moved with it, and a neighbour area's new size (docs/adr/1960).
                for (now, sizes) in around {
                    out.strokes.extend(Outline::of(now, Some([3.0, 3.0]), 1.0, Tone::Snap).strokes);
                    if let Some((before, after)) = sizes {
                        lines.push(format!(
                            "komşu {} ({})",
                            format.area(*after),
                            signed(format.area_bare(after - before), after - before)
                        ));
                    }
                }
                // Where the neighbours' vertices go, each ringed once (docs/adr/3000).
                for &p in carried {
                    out.markers.push(Marker { at: p, shape: MarkerShape::Ring(5.0), tone: Tone::Snap });
                }
                if !carried.is_empty() {
                    lines.push(format!("komşunun {} köşesi birlikte kayar", carried.len()));
                }
            }
            Ok(_) => {
                lines.push("referansın öbür yanı".into());
                out.tag_tone = Tone::Danger;
            }
            Err(why) => {
                lines.push(why.clone());
                out.tag_tone = Tone::Danger;
            }
        }
        out.tag = Some(Tag { at: *at, lines });
        out
    }
}
