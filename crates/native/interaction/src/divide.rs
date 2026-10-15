//! Böl: the web's `DivideTool` (AutoCAD DIVIDE and MEASURE,
//! `apps/web/src/tools/pathEditTools.ts`) on its edge-picking base, step
//! for step (docs/adr/0057), with blocks, the step's first distance and the
//! object's ends (docs/adr/0299):
//!
//! - the object is picked by its edge (a line, polyline, closed area, arc,
//!   circle, ellipse or spline, on any layer); the points to come are shown
//!   on it, measured from the end nearer the click;
//! - a typed number of parts (2 to 10 000), or with Aralık (A) a step in
//!   metres, places the points; Enter (or a quick right click) takes the
//!   last value; Parça sayısı (P) goes back to parts;
//! - Blok (B) puts the drawing's next block at the places instead of points
//!   (round to Nokta), Nokta (N) goes back to points, Hizala (H) turns the
//!   inserts with the object, Uçlar (U) also marks the object's own start,
//!   end or both, İlk (İ) asks a step's first distance (Enter alone: at the
//!   step again);
//! - the choices stay for as long as the app lives ([`Memory`]);
//! - Esc drops the picked object; with none, the tool leaves.
//!
//! The product command `cad.entities.divide` does the work (docs/adr/0254
//! §5, 0299): its plan is the preview (points or the inserts' outlines), its
//! execute the write on the active layer, as one undo step “Böl”:
//! “4 nokta kondu.”, “4 blok kondu.”

use kentos_contracts::{CommandResult, DivideBlock, DivideEnds, EntitiesDivide, Entity, NewLook};
use kentos_domain::{Document, Slot};
use kentos_geometry_core::ops::path::{nearest_s, path_of};
use kentos_geometry_core::tools::point_text::js_trim;
use kentos_native_application::geometry::shape;
use kentos_native_application::{ExecutionContext, divide};

use crate::Vec2;
use crate::edge::{self, Outline};
use crate::format::Format;
use crate::log::Level;
use crate::modify::ghosts;
use crate::points;
use crate::prompt::{Prompt, upper_tr};
use crate::tool::{
    Context, Cursor, Flow, Marker, MarkerShape, Memory, Pointer, Preview, Tone, Tool,
};

/// The divide tool's id: its command is `tool.divide`.
pub const ID: &str = "divide";
pub const LABEL: &str = "Böl";

/// Most points a division places (and the most a typed part count may ask for).
const MAX_POINTS: usize = 10_000;
/// The preview marks the points only up to this many (the web's draw).
const MAX_SHOWN: usize = 2000;
/// The preview draws the inserts' outlines only up to this many.
const MAX_INSERTS_SHOWN: usize = 400;

/// The kinds that can be divided, on any layer: the points go on the active one.
fn divisible(e: &Entity, _: &Document) -> bool {
    matches!(
        e,
        Entity::Line(_)
            | Entity::Polyline(_)
            | Entity::Polygon(_)
            | Entity::Arc(_)
            | Entity::Circle(_)
            | Entity::Ellipse(_)
            | Entity::Spline(_)
            | Entity::Nurbs(_)
    )
}

/// The object picked, and whether measuring starts from its far end.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Target {
    slot: Slot,
    from_end: bool,
}

/// An Uçlar choice's word in the prompt.
fn ends_word(ends: DivideEnds) -> &'static str {
    match ends {
        DivideEnds::None => "yok",
        DivideEnds::Start => "baş",
        DivideEnds::End => "son",
        DivideEnds::Both => "iki uç",
    }
}

/// The divide tool.
#[derive(Clone, Debug, Default)]
pub struct Divide {
    target: Option<Target>,
    /// İlk asked for: the next number is a step's first distance.
    asking_start: bool,
    /// What the session remembered, the project's units and the block's name, as of the last call.
    seen: Option<(Memory, Format, String)>,
    /// The picked object and its points to come, as of the last call.
    drawn: Preview,
}

impl Divide {
    pub fn new() -> Self {
        Self::default()
    }

    /// Takes in what the session remembers and redraws the places to come (the command's plan).
    fn see(&mut self, cx: &Context<'_>) {
        let m = *cx.memory;
        let name = m
            .divide_block
            .and_then(|id| cx.doc.block(id))
            .map_or_else(String::new, |b| b.name.clone());
        self.seen = Some((m, cx.format(), name));
        self.drawn = Preview::default();
        let Some(target) = self.target else { return };
        let Some(e) = cx.doc.get(target.slot) else {
            return;
        };
        let mut out = Outline::of(&shape(e), None, 1.5, Tone::Accent);
        let mut markers = Vec::new();
        if let Some(input) = Self::request(target, cx)
            && let CommandResult::Completed { output, .. } = divide::plan_in(cx.doc, &input)
        {
            let made = output.entities;
            for e in &made {
                match e {
                    Entity::Insert(i) if made.len() <= MAX_INSERTS_SHOWN => {
                        let paths = cx.spatial.store().insert_outlines(
                            &i.block.to_text(),
                            Vec2::new(i.p.x, i.p.y),
                            i.scale,
                            i.rotation,
                            i.mirror,
                        );
                        let (strokes, marks) = ghosts(&paths);
                        out.strokes.extend(strokes);
                        out.marks.extend(marks);
                    }
                    Entity::Point(p) if made.len() <= MAX_SHOWN => markers.push(Marker {
                        at: Vec2::new(p.p.x, p.p.y),
                        shape: MarkerShape::Circle(3.5),
                        tone: Tone::Accent,
                    }),
                    _ => {}
                }
            }
        }
        self.drawn = Preview {
            strokes: out.strokes,
            marks: out.marks,
            markers,
            ..Preview::default()
        };
    }

    /// The division the session remembers, as the command takes it.
    fn division(m: &Memory) -> kentos_contracts::Division {
        if m.divide_by_step {
            kentos_contracts::Division::Step {
                length: m.divide_step,
            }
        } else {
            kentos_contracts::Division::Parts {
                count: m.divide_parts,
            }
        }
    }

    /// The command's input for the picked object, with what the session remembers.
    fn request(target: Target, cx: &Context<'_>) -> Option<EntitiesDivide> {
        let m = *cx.memory;
        let uid = cx.doc.uid(target.slot)?;
        let block = match m.divide_block {
            Some(id) => Some(DivideBlock {
                name: cx.doc.block(id)?.name.clone(),
                align: m.divide_align,
                scale: 1.0,
                rotation: 0.0,
            }),
            None => None,
        };
        Some(EntitiesDivide {
            uid: uid.to_string(),
            division: Self::division(&m),
            from_end: target.from_end,
            // A first distance is a step's (docs/adr/0299 §1).
            start: m.divide_start.filter(|_| m.divide_by_step),
            ends: m.divide_ends,
            block,
            layer_id: cx.doc.layers().active().to_owned(),
            // The current colour; an object template's attributes, label and symbol (docs/adr/0176 §3).
            look: NewLook {
                color: cx.draft.color_text(),
                line_weight: None,
                attrs: cx.template_attrs(None),
                label: cx.template_label(),
                symbol: cx.template_symbol(),
            },
            expected_revision: None,
        })
    }

    /// Places the points or inserts on the active layer, one undo step “Böl”; the object is let go.
    fn commit(&mut self, cx: &mut Context<'_>) {
        let Some(target) = self.target.take() else {
            return;
        };
        let Some(input) = Self::request(target, cx) else {
            return;
        };
        let what = if input.block.is_some() {
            "blok"
        } else {
            "nokta"
        };
        let result = divide::execute(&mut ExecutionContext::new(cx.doc), input);
        if let Some(out) = points::written(result, cx) {
            cx.say(
                Level::Success,
                format!("{} {what} kondu.", out.created.len()),
            );
        }
    }

    /// Blok (B): the drawing's next block, after the last one points again.
    fn next_block(cx: &mut Context<'_>) {
        let blocks = cx.doc.blocks();
        let next = match cx.memory.divide_block {
            None => blocks.first().map(|b| b.id),
            Some(id) => {
                let at = blocks.iter().position(|b| b.id == id);
                at.and_then(|i| blocks.get(i + 1)).map(|b| b.id)
            }
        };
        if next.is_none() && cx.memory.divide_block.is_none() {
            cx.say(
                Level::Warn,
                "Çizimde blok yok; önce Blok oluştur ile bir blok tanımlayın.",
            );
        }
        cx.memory.divide_block = next;
    }
}

impl Tool for Divide {
    /// An object is picked (the web's `cursor = 'pick'`).
    fn cursor(&self) -> Cursor {
        Cursor::Pick
    }

    fn id(&self) -> &'static str {
        ID
    }

    fn label(&self) -> &'static str {
        LABEL
    }

    fn prompt(&self) -> Prompt {
        let (m, format, name) =
            self.seen
                .clone()
                .unwrap_or((Memory::default(), Format::default(), String::new()));
        if self.asking_start {
            return Prompt::new(
                LABEL,
                "ilk noktanın uzaklığını yazın (0: baştan; Enter: aralıkta)",
            );
        }
        let mode = if m.divide_by_step {
            format!("aralık {}", format.length(m.divide_step))
        } else {
            format!("{} parça", m.divide_parts)
        };
        let (other, key) = if m.divide_by_step {
            ("Parça sayısı", "P")
        } else {
            ("Aralık", "A")
        };
        let block = if name.is_empty() {
            "nokta".to_owned()
        } else {
            name
        };
        let start = m
            .divide_start
            .map_or_else(|| "aralıkta".to_owned(), |s| format.length(s));
        let base = match self.target {
            Some(_) => {
                let what = if m.divide_by_step {
                    "aralığı"
                } else {
                    "parça sayısını"
                };
                Prompt::new(LABEL, format!("{what} yazın ya da Enter ({mode})"))
            }
            None => Prompt::new(LABEL, "noktaların konacağı nesneyi seçin")
                .note(mode)
                .then(),
        };
        let base = base.option(other, key).option_with("Blok", "B", block);
        let base = if m.divide_block.is_some() {
            base.option("Nokta", "N")
        } else {
            base
        };
        base.option_if(
            m.divide_block.is_some(),
            "Hizala",
            "H",
            if m.divide_align { "açık" } else { "kapalı" },
        )
        .option_with("Uçlar", "U", ends_word(m.divide_ends))
        .option_if(m.divide_by_step, "İlk", "İ", start)
    }

    fn point_count(&self) -> usize {
        0
    }

    /// The object is picked by its edge: no snaps (the web's `EdgePickTool`).
    fn snaps(&self) -> bool {
        false
    }

    fn activate(&mut self, cx: &mut Context<'_>) -> Flow {
        self.see(cx);
        Flow::Stay
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        edge::hover(p, cx, divisible);
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        let Some(slot) = edge::pick_said(p, cx, divisible) else {
            cx.say(
                Level::Warn,
                "Çizgi, çoklu çizgi, yay, daire ya da eğriye tıklayın.",
            );
            return;
        };
        let path = cx.doc.get(slot).and_then(|e| path_of(&shape(e)));
        let Some(path) = path.filter(|path| path.length >= 1e-9) else {
            cx.say(Level::Warn, "Bu nesne bölünemez.");
            return;
        };
        // Measuring starts from the end nearer to the click.
        let from_end = !path.closed && nearest_s(&path, p.raw) > path.length / 2.0;
        self.target = Some(Target { slot, from_end });
        cx.selection.set_hover(None);
        self.see(cx);
    }

    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        let key = upper_tr(js_trim(text));
        let chosen = match key.as_str() {
            "A" | "P" => {
                cx.memory.divide_by_step = key == "A";
                true
            }
            "B" => {
                Self::next_block(cx);
                true
            }
            "N" => {
                cx.memory.divide_block = None;
                true
            }
            "H" => {
                cx.memory.divide_align = !cx.memory.divide_align;
                true
            }
            "U" => {
                cx.memory.divide_ends = match cx.memory.divide_ends {
                    DivideEnds::None => DivideEnds::Start,
                    DivideEnds::Start => DivideEnds::End,
                    DivideEnds::End => DivideEnds::Both,
                    DivideEnds::Both => DivideEnds::None,
                };
                true
            }
            "İ" if cx.memory.divide_by_step => {
                self.asking_start = true;
                true
            }
            _ => false,
        };
        if chosen {
            self.see(cx);
            return true;
        }
        if self.asking_start {
            let Some((_, metres)) = crate::points::plain_value(text, cx) else {
                return false;
            };
            if metres.is_nan() || metres < 0.0 {
                cx.say(
                    Level::Warn,
                    "İlk noktanın uzaklığı sıfır ya da sıfırdan büyük olmalı.",
                );
                return true;
            }
            cx.memory.divide_start = Some(metres);
            self.asking_start = false;
            self.see(cx);
            return true;
        }
        let Some((n, metres)) = crate::points::plain_value(text, cx).filter(|(n, _)| *n > 0.0)
        else {
            return false;
        };
        if cx.memory.divide_by_step {
            // Typed in the project's unit (docs/adr/0165 §2), or with its own (docs/adr/0266).
            cx.memory.divide_step = metres;
        } else {
            if n.fract() != 0.0 || !(2.0..=MAX_POINTS as f64).contains(&n) {
                cx.say(
                    Level::Warn,
                    "Parça sayısı 2 ile 10 000 arasında bir tam sayı olmalı.",
                );
                return true;
            }
            cx.memory.divide_parts = n as u32;
        }
        if self.target.is_some() {
            self.commit(cx);
        }
        self.see(cx);
        true
    }

    /// Enter: İlk asked for goes back to the step; else the picked object is divided by the last value; with
    /// none, the tool leaves.
    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow {
        if self.asking_start {
            self.asking_start = false;
            cx.memory.divide_start = None;
            self.see(cx);
            return Flow::Stay;
        }
        if self.target.is_none() {
            return Flow::Exit;
        }
        self.commit(cx);
        self.see(cx);
        Flow::Stay
    }

    /// Esc drops İlk asked for, then the picked object; with neither, the tool leaves.
    fn cancel(&mut self, cx: &mut Context<'_>) -> bool {
        if self.asking_start {
            self.asking_start = false;
            self.see(cx);
            return true;
        }
        if self.target.take().is_none() {
            return false;
        }
        self.see(cx);
        true
    }

    fn undo_step(&mut self, _cx: &mut Context<'_>) -> bool {
        false
    }

    fn preview(&self, _format: &Format) -> Preview {
        self.drawn.clone()
    }
}
