//! Uzat-kısalt: the web's `LengthenTool` (AutoCAD LENGTHEN,
//! `apps/web/src/tools/lengthenTool.ts`) on its edge-picking base, step for
//! step (docs/adr/0047):
//!
//! - click near the end of a line, an arc or an open polyline to change;
//! - Dinamik (D, the default): the end then follows the mouse (past the end
//!   the end segment continues, inside the path it is cut back; object snaps
//!   apply) until a click, or a typed number is the new total length;
//! - Fark (F), Yüzde (Y), Toplam (T): the next number typed is the value,
//!   and every end clicked changes at once by it (the preview shows it);
//! - the mode and the values stay for as long as the app lives
//!   ([`crate::tool::Memory`]); Enter or Esc drops a picked end or a value
//!   asked for; with neither, Enter leaves.
//!
//! The object takes its new geometry through `cad.entities.edit`:
//! “Uzunluk 10.000 m → 12.000 m.” The lengths and the geometry are the
//! shared core's (`length_of`, `near_end`, `length_toward`, `lengthen_entity`).

use kentos_contracts::{EditOperation, Entity, EntityEdit};
use kentos_domain::{Document, Slot};
use kentos_geometry_core::ops::curve_cuts::Geometry;
use kentos_geometry_core::ops::lengthen::{length_of, length_toward, lengthen_entity, near_end};
use kentos_geometry_core::tools::point_text::parse_number;
use kentos_native_application::geometry::shape;

use crate::edge::{self, Hover, Outline};
use crate::format::Format;
use crate::log::Level;
use crate::prompt::{Prompt, upper_tr};
use crate::tool::{Context, Flow, LengthenMode, Memory, Pointer, Preview, Tag, Tone, Tool};
use crate::{Vec2, js_trim};

/// The lengthen tool's id: its command is `tool.lengthen`.
pub const ID: &str = "lengthen";
pub const LABEL: &str = "Uzat-kısalt";

/// What Uzat-kısalt takes, off locked layers: whatever has a length it can change.
fn editable(e: &Entity, doc: &Document) -> bool {
    length_of(&shape(e)).is_some() && edge::unlocked(e, doc)
}

/// A mode's name in the prompt.
fn mode_name(mode: LengthenMode) -> &'static str {
    match mode {
        LengthenMode::Dynamic => "dinamik",
        LengthenMode::Delta => "fark",
        LengthenMode::Percent => "yüzde",
        LengthenMode::Total => "toplam",
    }
}

/// A number as JavaScript writes it in a template: `100`, `12.5`.
fn js_number(v: f64) -> String {
    format!("{v}")
}

/// The lengthen tool.
#[derive(Clone, Debug, Default)]
pub struct Lengthen {
    /// The value asked for after Fark, Yüzde or Toplam.
    ask: Option<LengthenMode>,
    /// The object picked in the dynamic mode, and whether its end moves (not its start).
    target: Option<(Slot, bool)>,
    /// The pointer's point, snapped.
    mouse: Option<Vec2>,
    hover: Option<Hover>,
    drawn: Preview,
    seen: Option<(Memory, Format)>,
}

impl Lengthen {
    pub fn new() -> Self {
        Self::default()
    }

    fn see(&mut self, cx: &Context<'_>) {
        self.seen = Some((*cx.memory, cx.format()));
    }

    fn memory(&self) -> Memory {
        self.seen.map(|(m, _)| m).unwrap_or_default()
    }

    /// The new length an end clicked in Fark, Yüzde or Toplam gets.
    fn length_for(m: &Memory, e: &Entity) -> f64 {
        let l = length_of(&shape(e)).unwrap_or(0.0);
        match m.lengthen_mode {
            LengthenMode::Delta => l + m.lengthen_delta,
            LengthenMode::Percent => (l * m.lengthen_percent) / 100.0,
            _ => m.lengthen_total,
        }
    }

    /// Writes the new length through the edit command (the web's `apply`).
    fn apply(&mut self, slot: Slot, at_end: bool, length: f64, cx: &mut Context<'_>) {
        let Some(e) = cx.doc.get(slot) else { return };
        let before = length_of(&shape(e)).unwrap_or(0.0);
        match lengthen_entity(&edge::core(e), at_end, length) {
            Geometry::Error(error) => cx.say(Level::Warn, error),
            Geometry::Ok(g) => {
                let uid = edge::uid(cx.doc, slot);
                let written = edge::geometry(&g.shape).and_then(|geometry| {
                    edge::write(
                        EditOperation::Lengthen,
                        vec![EntityEdit::Update { uid, geometry }],
                        cx,
                    )
                });
                if written.is_some() {
                    let f = cx.format();
                    let text = format!("Uzunluk {} → {}.", f.length(before), f.length(length));
                    cx.say(Level::Success, text);
                }
                self.hover = None;
            }
        }
    }

    /// The result of `length` at the moving end, and the change beside `at`.
    fn show(&mut self, e: &Entity, at_end: bool, length: Option<f64>, at: Vec2, f: &Format) {
        let Some(length) = length else { return };
        let Geometry::Ok(g) = lengthen_entity(&edge::core(e), at_end, length) else {
            return;
        };
        let out = Outline::of(&g.shape, None, 2.0, Tone::Accent);
        let before = length_of(&shape(e)).unwrap_or(0.0);
        self.drawn = Preview {
            strokes: out.strokes,
            marks: out.marks,
            tag: Some(Tag {
                at,
                lines: vec![format!("{} → {}", f.length(before), f.length(length))],
            }),
            ..Preview::default()
        };
    }

    /// The preview (the web's `draw`): the end following the mouse, or in
    /// the other modes what a click on the hovered end would do.
    fn redraw(&mut self, cx: &Context<'_>) {
        self.drawn = Preview::default();
        let m = *cx.memory;
        let f = cx.format();
        if let (Some((slot, at_end)), Some(mouse)) = (self.target, self.mouse) {
            if let Some(e) = cx.doc.get(slot) {
                let length = length_toward(&shape(e), at_end, mouse);
                self.show(e, at_end, length, mouse, &f);
            }
            return;
        }
        if self.target.is_none()
            && m.lengthen_mode != LengthenMode::Dynamic
            && let Some(h) = self.hover
            && let Some(e) = cx.doc.get(h.slot)
        {
            let at_end = near_end(&shape(e), h.at);
            let length = Self::length_for(&m, e);
            self.show(e, at_end, Some(length), h.at, &f);
        }
    }
}

impl Tool for Lengthen {
    fn id(&self) -> &'static str {
        ID
    }

    fn label(&self) -> &'static str {
        LABEL
    }

    fn prompt(&self) -> Prompt {
        let m = self.memory();
        let f = self.seen.map(|(_, f)| f).unwrap_or_default();
        match (self.ask, self.target) {
            (Some(LengthenMode::Delta), _) => {
                Prompt::new(LABEL, "uzunluk farkını yazın (eksi kısaltır)")
            }
            (Some(LengthenMode::Percent), _) => Prompt::new(
                LABEL,
                "yeni uzunluğu eski uzunluğun yüzdesi olarak yazın (100 = aynı)",
            ),
            (Some(_), _) => Prompt::new(LABEL, "yeni toplam uzunluğu yazın"),
            (None, Some(_)) => Prompt::new(
                LABEL,
                "yeni ucu fareyle gösterin ya da toplam uzunluğu yazın",
            ),
            (None, None) => Prompt::new(LABEL, "değiştirilecek ucun yakınına tıklayın")
                .note(format!("kip: {}", mode_name(m.lengthen_mode)))
                .then()
                .option("Dinamik", "D")
                .option_with("Fark", "F", f.length(m.lengthen_delta))
                .option_with("Yüzde", "Y", js_number(m.lengthen_percent))
                .option_with("Toplam", "T", f.length(m.lengthen_total)),
        }
    }

    fn point_count(&self) -> usize {
        0
    }

    fn activate(&mut self, cx: &mut Context<'_>) -> Flow {
        self.see(cx);
        Flow::Stay
    }

    /// The moving end snaps.
    fn snaps(&self) -> bool {
        self.target.is_some()
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.see(cx);
        self.mouse = Some(p.world);
        if self.target.is_none() {
            self.hover = edge::hover(p, cx, editable);
        }
        self.redraw(cx);
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.see(cx);
        if let Some((slot, at_end)) = self.target.take() {
            let length = cx
                .doc
                .get(slot)
                .and_then(|e| length_toward(&shape(e), at_end, p.world));
            if let Some(length) = length {
                self.apply(slot, at_end, length, cx);
            }
            self.drawn = Preview::default();
            return self.see(cx);
        }
        let Some(slot) = edge::pick(p, cx, editable) else {
            cx.say(
                Level::Warn,
                "Düzenlenebilir bir çizgiye, yaya ya da açık çoklu çizgiye tıklayın.",
            );
            return;
        };
        let Some(e) = cx.doc.get(slot) else { return };
        let at_end = near_end(&shape(e), p.raw);
        let m = *cx.memory;
        if m.lengthen_mode == LengthenMode::Dynamic {
            self.target = Some((slot, at_end));
            cx.selection.set_hover(None);
            return self.redraw(cx);
        }
        let length = Self::length_for(&m, e);
        self.apply(slot, at_end, length, cx);
        self.drawn = Preview::default();
        self.see(cx);
    }

    /// D, F, Y, T choose the mode (F, Y and T then ask for their value); a
    /// number is the value asked for, or the dynamic end's total length.
    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        self.see(cx);
        let mode = match upper_tr(js_trim(text)).as_str() {
            "D" => Some(LengthenMode::Dynamic),
            "F" => Some(LengthenMode::Delta),
            "Y" => Some(LengthenMode::Percent),
            "T" => Some(LengthenMode::Total),
            _ => None,
        };
        if let Some(mode) = mode
            && self.target.is_none()
        {
            cx.memory.lengthen_mode = mode;
            self.ask = (mode != LengthenMode::Dynamic).then_some(mode);
            self.see(cx);
            self.redraw(cx);
            return true;
        }
        let Some(n) = parse_number(text) else {
            return false;
        };
        if let Some(ask) = self.ask {
            // NaN is not above zero either (the web's `!(n > 0)`).
            if ask != LengthenMode::Delta
                && n.partial_cmp(&0.0) != Some(std::cmp::Ordering::Greater)
            {
                cx.say(Level::Warn, "Değer sıfırdan büyük olmalı.");
                return true;
            }
            match ask {
                LengthenMode::Delta => cx.memory.lengthen_delta = n,
                LengthenMode::Percent => cx.memory.lengthen_percent = n,
                _ => cx.memory.lengthen_total = n,
            }
            self.ask = None;
            self.see(cx);
            self.redraw(cx);
            return true;
        }
        if let Some((slot, at_end)) = self.target.take() {
            self.apply(slot, at_end, n, cx);
            self.drawn = Preview::default();
            self.see(cx);
            return true;
        }
        false
    }

    /// Drops a picked end or a value asked for; with neither, the tool leaves.
    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow {
        self.see(cx);
        if self.target.is_some() || self.ask.is_some() {
            self.target = None;
            self.ask = None;
            self.drawn = Preview::default();
            return Flow::Stay;
        }
        Flow::Exit
    }

    fn cancel(&mut self, cx: &mut Context<'_>) -> bool {
        self.see(cx);
        if self.target.is_none() && self.ask.is_none() {
            return false;
        }
        self.target = None;
        self.ask = None;
        self.drawn = Preview::default();
        true
    }

    fn undo_step(&mut self, _cx: &mut Context<'_>) -> bool {
        false
    }

    fn preview(&self, _format: &Format) -> Preview {
        self.drawn.clone()
    }
}
