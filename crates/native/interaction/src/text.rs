//! Yazı: the web's `TextTool` (`apps/web/src/tools/annotateTools.ts`) on its
//! `PointInputTool` base, step for step:
//!
//! - a click (or a typed point) where the text starts opens a text field
//!   right there; the host shows it (`ViewChange::Text`) and gives back what
//!   was typed (`Tool::text_typed`): Enter adds the text and the tool waits
//!   for the next one, Esc drops the field;
//! - Yükseklik (Y) asks for the height in paper millimetres (the project's
//!   plot scale makes it metres), Açı (A) for the angle in degrees or two
//!   clicks along an edge (a direction pointing left is turned around, kept
//!   readable); both stay for as long as the app lives;
//! - a locked active layer is said at the click and no field opens (the
//!   web's newest rule); Esc leaves the tool, and inside the field it only
//!   drops the text (the host's).
//!
//! The text is written through `cad.entities.create` (docs/adr/0057): the
//! active layer, one undo step (“Ekle”), “Yazı eklendi: “…”” said.

use kentos_contracts::EntityGeometry;
use kentos_geometry_core::geometry::dist;
use kentos_geometry_core::tools::drawing::text_angle;
use kentos_geometry_core::tools::point_text::{js_trim, parse_number, point_from_text};

use crate::Vec2;
use crate::format::{Format, fixed, js_number};
use crate::log::Level;
use crate::points::{self, Taken};
use crate::prompt::{Prompt, upper_tr};
use crate::tool::{
    Context, Flow, Memory, Pointer, Preview, Stroke, Tag, TextField, Tone, Tool, ViewChange,
};

/// The text tool's id: its command is `tool.text`.
pub const ID: &str = "text";
pub const LABEL: &str = "Yazı";

/// The hover box's least height on screen, logical pixels (the web's `Math.max(8, …)`).
const LEAST_BOX_PX: f64 = 8.0;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Stage {
    /// Where the text starts.
    #[default]
    Pos,
    Height,
    Angle,
    /// The field is open at `at`.
    Typing,
}

/// The text tool.
#[derive(Clone, Debug, Default)]
pub struct Text {
    d: Taken,
    stage: Stage,
    /// Where the typed text will start.
    at: Option<Vec2>,
    /// The first of the two clicks that give the angle.
    angle_from: Option<Vec2>,
    /// The hover box's height in metres at the last move: the text's own, at
    /// least 8 px on screen.
    box_height: f64,
    /// What the session remembered and the project's units, as of the last call.
    seen: Option<(Memory, Format)>,
}

/// The active layer's lock sentence, when it is locked (the drawing tools' words).
fn active_locked(cx: &Context<'_>) -> Option<String> {
    let layers = cx.doc.layers();
    let id = layers.active();
    layers.is_locked(id).then(|| {
        let name = layers.get(id).map_or(id, |n| n.name.as_str());
        format!(
            "“{name}” katmanı kilitli. Kilidi Katmanlar panelinden açın ya da başka bir katmanı etkinleştirin."
        )
    })
}

/// Paper millimetres as metres at the project's plot scale (the web's `paper`).
fn paper(mm: f64, cx: &Context<'_>) -> f64 {
    mm / 1000.0 * cx.doc.settings().plot_scale
}

/// A number as the web writes it in the prompt: `+n.toFixed(places)`.
fn trimmed(n: f64, places: usize) -> String {
    js_number(fixed(n, places).parse::<f64>().unwrap_or(n))
}

impl Text {
    pub fn new() -> Self {
        Self::default()
    }

    fn see(&mut self, cx: &Context<'_>) {
        self.seen = Some((*cx.memory, cx.format()));
    }

    /// The point Orto and tracking go from: the angle's first click.
    fn last(&self) -> Option<Vec2> {
        match self.stage {
            Stage::Angle => self.angle_from,
            _ => None,
        }
    }

    fn option(&mut self, key: &str) -> bool {
        if self.stage != Stage::Pos {
            return false;
        }
        self.stage = match key {
            "Y" => Stage::Height,
            "A" => Stage::Angle,
            _ => return false,
        };
        self.angle_from = None;
        true
    }

    /// A point given (the web's `accept`, then `onPoint`). Every point starts
    /// a new object: what was written before is the drawing's to undo.
    fn accept(&mut self, p: Vec2, cx: &mut Context<'_>) {
        points::echo(p, cx);
        self.d.reset();
        match self.stage {
            Stage::Angle => match self.angle_from {
                None => self.angle_from = Some(p),
                Some(from) if dist(from, p) < 1e-9 => {}
                Some(from) => {
                    // Kept readable: a direction pointing left is turned around.
                    cx.memory.text_angle = text_angle(from, p);
                    self.angle_from = None;
                    self.stage = Stage::Pos;
                }
            },
            // A locked active layer is said at the click, and no field opens.
            Stage::Pos if active_locked(cx).is_some() => {
                let line = active_locked(cx).unwrap_or_default();
                cx.say(Level::Warn, line);
            }
            Stage::Pos => {
                self.at = Some(p);
                self.stage = Stage::Typing;
                cx.view_changes.push(ViewChange::Text(TextField {
                    at: p,
                    height: paper(cx.memory.text_height_mm, cx),
                    rotation: cx.memory.text_angle,
                }));
            }
            Stage::Height | Stage::Typing => {}
        }
    }

    /// The field closed: back to where the next text starts (the web's `afterTyping`).
    fn after_typing(&mut self) {
        self.at = None;
        self.stage = Stage::Pos;
    }
}

impl Tool for Text {
    fn id(&self) -> &'static str {
        ID
    }

    fn label(&self) -> &'static str {
        LABEL
    }

    fn prompt(&self) -> Prompt {
        let memory = self.seen.map_or_else(Memory::default, |(m, _)| m);
        match self.stage {
            Stage::Height => Prompt::new(LABEL, "kâğıt üzerindeki yazı yüksekliğini mm olarak yazın"),
            Stage::Angle if self.angle_from.is_some() => {
                Prompt::new(LABEL, "doğrultunun ikinci noktasına tıklayın")
            }
            Stage::Angle => Prompt::new(
                LABEL,
                "açıyı yazın (derece) ya da doğrultu için iki noktaya tıklayın",
            ),
            Stage::Typing => Prompt::new(
                LABEL,
                "yazıyı tıkladığınız yere yazın; Enter ekler, Esc vazgeçer",
            ),
            Stage::Pos => Prompt::new(LABEL, "yazının başlangıcına tıklayın")
                .option_with(
                    "Yükseklik",
                    "Y",
                    format!("{} mm", js_number(memory.text_height_mm)),
                )
                .option_with("Açı", "A", format!("{}°", trimmed(memory.text_angle, 4))),
        }
    }

    fn point_count(&self) -> usize {
        0
    }

    fn activate(&mut self, cx: &mut Context<'_>) -> Flow {
        self.see(cx);
        Flow::Stay
    }

    fn snap_from(&self) -> Option<Vec2> {
        self.angle_from.or(self.at)
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        let (point, tracking) = points::constrain(self.last(), p, cx);
        self.d.tracking = tracking;
        self.d.hover = Some(point);
        self.box_height = paper(cx.memory.text_height_mm, cx).max(cx.view.world_length(LEAST_BOX_PX));
        self.see(cx);
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        let (point, _) = points::constrain(self.last(), p, cx);
        self.accept(point, cx);
        self.see(cx);
    }

    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        let t = js_trim(text);
        let done = if self.option(&upper_tr(t)) {
            true
        } else {
            match (self.stage, parse_number(t)) {
                (Stage::Height, Some(n)) if n > 0.0 => {
                    cx.memory.text_height_mm = n;
                    self.stage = Stage::Pos;
                    true
                }
                (Stage::Height, _) => false,
                (Stage::Angle, Some(n)) => {
                    cx.memory.text_angle = n;
                    self.angle_from = None;
                    self.stage = Stage::Pos;
                    true
                }
                (Stage::Angle, None) => false,
                _ => match point_from_text(text, self.last(), self.d.hover, |_| None) {
                    Some(p) => {
                        self.accept(p, cx);
                        true
                    }
                    None => false,
                },
            }
        };
        self.see(cx);
        done
    }

    /// The field's answer: the text to add, or none (Esc, nothing typed).
    fn text_typed(&mut self, text: Option<&str>, cx: &mut Context<'_>) {
        if self.stage != Stage::Typing {
            return;
        }
        if let (Some(p), Some(text)) = (self.at, text.map(js_trim).filter(|t| !t.is_empty())) {
            let geometry = EntityGeometry::Text {
                p: points::wire(p),
                text: text.to_owned(),
                height: paper(cx.memory.text_height_mm, cx),
                rotation: cx.memory.text_angle,
            };
            if let Some(out) = points::write_objects(vec![geometry], None, cx)
                && let Some(&id) = out.ids.first()
            {
                self.d.note(id, cx);
                cx.say(Level::Success, format!("Yazı eklendi: “{text}”"));
            }
        }
        self.after_typing();
        self.see(cx);
    }

    /// Where the text starts: a confirm leaves; typing, it drops the field
    /// (the web's `confirm`).
    fn confirm(&mut self, _cx: &mut Context<'_>) -> Flow {
        match self.stage {
            Stage::Pos => Flow::Exit,
            _ => {
                self.after_typing();
                Flow::Stay
            }
        }
    }

    fn undo_step(&mut self, cx: &mut Context<'_>) -> bool {
        self.d.undo_last_made(cx)
    }

    /// The angle's line from its first click, or where the text will sit: a
    /// dashed box of its height along its angle.
    fn preview(&self, _format: &Format) -> Preview {
        let Some(h) = self.d.hover else {
            return Preview::default();
        };
        let memory = self.seen.map_or_else(Memory::default, |(m, _)| m);
        match (self.stage, self.angle_from) {
            (Stage::Angle, Some(from)) => {
                let degrees = kentos_geometry_core::geometry::angle_deg(from, h);
                Preview {
                    strokes: vec![Stroke {
                        pts: vec![from, h],
                        closed: false,
                        dash: Some([3.0, 3.0]),
                        width: 1.0,
                        tone: Tone::Accent,
                    }],
                    tag: Some(Tag {
                        at: h,
                        lines: vec![format!("Açı {}°", fixed(degrees, 2))],
                    }),
                    ..Preview::default()
                }
            }
            (Stage::Pos, _) => {
                let a = memory.text_angle.to_radians();
                let (dx, dy) = (a.cos(), a.sin());
                let height = self.box_height;
                let width = height * 4.0;
                let along = Vec2::new(h.x + dx * width, h.y + dy * width);
                let up = |p: Vec2| Vec2::new(p.x - dy * height, p.y + dx * height);
                Preview {
                    strokes: vec![Stroke {
                        pts: vec![h, along, up(along), up(h)],
                        closed: true,
                        dash: Some([3.0, 3.0]),
                        width: 1.0,
                        tone: Tone::Accent,
                    }],
                    ..Preview::default()
                }
            }
            _ => Preview::default(),
        }
    }
}
