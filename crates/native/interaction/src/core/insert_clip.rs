//! Kırp (docs/adr/0351 §1; AutoCAD's XCLIP): a block's or an external reference's insert is clicked; its boundary is
//! drawn as a rectangle by two corners (Dikdörtgen, D) or as a polygon by its corners and Enter (Çokgen, Ç), Ters (T)
//! keeps the outside and Çerçeve (E) goes Gizli → Ekranda → Yazdır. Kaldır (K) takes a clipping away. The tool writes
//! with `cad.inserts.clip` (one step “Kırp”); the command keeps the boundary in the definition. Esc and Ctrl+Z take
//! back the last corner, then the insert; Esc with none leaves.

use kentos_commands::{ClipBoundary, ClipFrame, Entity, InsertsClip, InsertsClipOperation};
use kentos_domain::{Document, Slot};
use kentos_geometry_core::tools::point_text::js_trim;
use kentos_native_application::{ExecutionContext, inserts_clip};

use crate::Vec2;
use crate::edge;
use crate::format::Format;
use crate::log::Level;
use crate::points::{wire, written};
use crate::prompt::{Prompt, upper_tr};
use crate::tool::{Context, Cursor, Flow, Pointer, Preview, Stroke, Tone, Tool};

/// The tool's id: its command is `tool.xclip`.
pub const ID: &str = "xclip";
pub const LABEL: &str = "Kırp";
/// A click on no insert.
pub const NO_INSERT: &str =
    "Tıklanan yerde kilitsiz bir blok ya da dış başvuru yok; kırpılacak yerleştirmeye tıklayın.";

fn editable(e: &Entity, doc: &Document) -> bool {
    matches!(e, Entity::Insert(_)) && edge::unlocked(e, doc)
}

fn frame_word(f: ClipFrame) -> &'static str {
    match f {
        ClipFrame::Hide => "Gizli",
        ClipFrame::Show => "Ekranda",
        ClipFrame::Plot => "Yazdır",
    }
}

pub struct InsertClip {
    insert: Option<Slot>,
    pts: Vec<Vec2>,
    hover: Option<Vec2>,
    polygon: bool,
    inverted: bool,
    frame: ClipFrame,
}

impl Default for InsertClip {
    fn default() -> Self {
        Self::new()
    }
}

impl InsertClip {
    pub fn new() -> Self {
        Self { insert: None, pts: Vec::new(), hover: None, polygon: false, inverted: false, frame: ClipFrame::Hide }
    }

    fn clipped(&self, doc: &Document) -> bool {
        self.insert
            .and_then(|s| doc.get(s))
            .is_some_and(|e| matches!(e, Entity::Insert(i) if i.clipping.is_some()))
    }

    fn run(&mut self, operation: InsertsClipOperation, cx: &mut Context<'_>) {
        let Some(slot) = self.insert else { return };
        let input = InsertsClip { uids: vec![edge::uid(cx.doc, slot)], operation, expected_revision: None };
        let removed = matches!(input.operation, InsertsClipOperation::Remove);
        let result = inserts_clip::execute(&mut ExecutionContext::new(cx.doc), input);
        if written(result, cx).is_some() {
            cx.say(Level::Success, if removed { format!("{LABEL}: kırpma kaldırıldı.") } else { format!("{LABEL}: kırpıldı.") });
            self.insert = None;
        }
        self.pts.clear();
    }

    fn boundary(&self, ring: Vec<Vec2>) -> ClipBoundary {
        if !self.polygon && ring.len() == 2 {
            ClipBoundary::Rectangle { a: wire(ring[0]), b: wire(ring[1]) }
        } else {
            ClipBoundary::Polygon { points: ring.into_iter().map(wire).collect() }
        }
    }

    fn set(&mut self, ring: Vec<Vec2>, cx: &mut Context<'_>) {
        let boundary = self.boundary(ring);
        self.run(InsertsClipOperation::Set { boundary, inverted: self.inverted, frame: self.frame }, cx);
    }

    fn rectangle(a: Vec2, b: Vec2) -> Vec<Vec2> {
        vec![a, Vec2::new(b.x, a.y), b, Vec2::new(a.x, b.y)]
    }
}

impl Tool for InsertClip {
    fn id(&self) -> &'static str {
        ID
    }

    fn label(&self) -> &'static str {
        LABEL
    }

    fn prompt(&self) -> Prompt {
        if self.insert.is_none() {
            return Prompt::new(LABEL, "kırpılacak blok ya da dış başvuruya tıklayın");
        }
        let step = match (self.polygon, self.pts.len()) {
            (false, 0) => "sınırın ilk köşesine tıklayın",
            (false, _) => "sınırın karşı köşesine tıklayın",
            (true, n) if n < 3 => "sınırın köşelerine tıklayın",
            (true, _) => "sonraki köşeye tıklayın ya da Enter ile bitirin",
        };
        Prompt::new(LABEL, step)
            .toggle("Dikdörtgen", "D", !self.polygon)
            .toggle("Çokgen", "Ç", self.polygon)
            .toggle("Ters", "T", self.inverted)
            .option_with("Çerçeve", "E", frame_word(self.frame))
            .option("Kaldır", "K")
    }

    fn point_count(&self) -> usize {
        usize::from(self.insert.is_some()) + self.pts.len()
    }

    fn activate(&mut self, cx: &mut Context<'_>) -> Flow {
        self.polygon = cx.memory.image_clip_polygon;
        // A single selected insert is the one clipped.
        let selected: Vec<Slot> = cx.selection.ids().to_vec();
        if let [s] = selected.as_slice()
            && cx.doc.get(*s).is_some_and(|e| editable(e, cx.doc))
        {
            self.insert = Some(*s);
        }
        Flow::Stay
    }

    fn snaps(&self) -> bool {
        self.insert.is_some()
    }

    fn cursor(&self) -> Cursor {
        if self.insert.is_some() { Cursor::Cross } else { Cursor::Pick }
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.hover = Some(p.world);
        if self.insert.is_none() {
            edge::hover(p, cx, editable);
        }
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        if self.insert.is_none() {
            match edge::pick_said(p, cx, editable) {
                Some(slot) => {
                    self.insert = Some(slot);
                    cx.selection.set_hover(None);
                }
                None => cx.say(Level::Warn, NO_INSERT),
            }
            return;
        }
        self.pts.push(p.world);
        if !self.polygon && self.pts.len() == 2 {
            let ring = std::mem::take(&mut self.pts);
            self.set(ring, cx);
        }
    }

    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        if self.insert.is_none() {
            return false;
        }
        match upper_tr(js_trim(text)).as_str() {
            "D" => {
                self.polygon = false;
                self.pts.clear();
            }
            "Ç" | "C" => {
                self.polygon = true;
                self.pts.clear();
            }
            "T" => self.inverted = !self.inverted,
            "E" => {
                self.frame = match self.frame {
                    ClipFrame::Hide => ClipFrame::Show,
                    ClipFrame::Show => ClipFrame::Plot,
                    ClipFrame::Plot => ClipFrame::Hide,
                };
            }
            "K" if self.clipped(cx.doc) => self.run(InsertsClipOperation::Remove, cx),
            "K" => {
                cx.say(Level::Warn, "Bu yerleştirme kırpılmış değil.");
                return true;
            }
            _ => return false,
        }
        cx.memory.image_clip_polygon = self.polygon;
        true
    }

    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow {
        if self.polygon && self.pts.len() >= 3 {
            let ring = std::mem::take(&mut self.pts);
            self.set(ring, cx);
            return Flow::Stay;
        }
        Flow::Exit
    }

    fn cancel(&mut self, cx: &mut Context<'_>) -> bool {
        self.undo_step(cx)
    }

    fn undo_step(&mut self, _cx: &mut Context<'_>) -> bool {
        if self.pts.pop().is_some() {
            return true;
        }
        self.insert.take().is_some()
    }

    fn preview(&self, _format: &Format) -> Preview {
        let mut out = Preview::default();
        if self.insert.is_none() {
            return out;
        }
        let mut ring = self.pts.clone();
        if let Some(h) = self.hover {
            if !self.polygon && ring.len() == 1 {
                ring = Self::rectangle(ring[0], h);
            } else if !ring.is_empty() {
                ring.push(h);
            }
        }
        if ring.len() >= 2 {
            out.strokes.push(Stroke::dashed(ring, true, [6.0, 4.0]).tone(if self.inverted { Tone::Snap } else { Tone::Accent }));
        }
        out
    }
}
