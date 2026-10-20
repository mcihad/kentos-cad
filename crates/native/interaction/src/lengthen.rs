//! Uzat-kısalt: the web's `LengthenTool` (AutoCAD LENGTHEN,
//! `apps/web/src/tools/lengthenTool.ts`) on its edge-picking base, step for
//! step (docs/adr/0047):
//!
//! - click near the end of a line, an arc or an open polyline to change;
//! - Dinamik (D, the default): the end then follows the mouse (past the end
//!   the end segment continues, inside the path it is cut back; object snaps
//!   apply) until a click, or a typed number is the new total length;
//! - Fark (F), Yüzde (Y), Toplam (T): the next number typed is the value,
//!   and every end clicked changes at once by it (the preview shows it); in
//!   Fark and Toplam `<45` is an arc's included angle instead (docs/adr/0298);
//! - the mode and the values stay for as long as the app lives
//!   ([`crate::tool::Memory`]); Enter or Esc drops a picked end or a value
//!   asked for; with neither, Enter leaves.
//!
//! The product command `cad.entities.lengthen` does the work (docs/adr/0298):
//! its plan is the preview, its execute the write, as one undo step
//! “Uzat-kısalt”: “Uzunluk 10.000 m → 12.000 m.” The tool asks only which end
//! (the core's `near_end`) and how the length changes.

use kentos_commands::{CommandResult, EntitiesLengthen, Entity, LengthenChange, LengthenEnd};
use kentos_domain::{Document, Slot};
use kentos_geometry_core::ops::lengthen::{length_of, near_end};
use kentos_geometry_core::tools::point_text::parse_number;
use kentos_native_application::geometry::shape;
use kentos_native_application::lengthen::lengths;
use kentos_native_application::{ExecutionContext, entities_lengthen};

use crate::edge::{self, Hover, Outline};
use crate::format::Format;
use crate::log::Level;
use crate::points::{self, wire};
use crate::prompt::{Prompt, upper_tr};
use crate::tool::{Context, Cursor, Flow, LengthenMode, Memory, Pointer, Preview, Tag, Tone, Tool};
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

/// A Fark or Toplam value in the prompt: its length, or `<` and an arc's angle.
fn value_text(length: f64, angle: Option<f64>, f: &Format) -> String {
    match angle {
        Some(a) => format!("<{}", f.angle(a)),
        None => f.length(length),
    }
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

    /// The change an end clicked in Fark, Yüzde or Toplam gets.
    fn change_for(m: &Memory) -> LengthenChange {
        match m.lengthen_mode {
            LengthenMode::Delta => match m.lengthen_delta_angle {
                Some(a) => LengthenChange::Delta {
                    length: None,
                    angle: Some(a),
                },
                None => LengthenChange::Delta {
                    length: Some(m.lengthen_delta),
                    angle: None,
                },
            },
            LengthenMode::Percent => LengthenChange::Percent {
                percent: m.lengthen_percent,
            },
            _ => match m.lengthen_total_angle {
                Some(a) => LengthenChange::Total {
                    length: None,
                    angle: Some(a),
                },
                None => LengthenChange::Total {
                    length: Some(m.lengthen_total),
                    angle: None,
                },
            },
        }
    }

    /// The command's input: `slot`'s end (or start) changed by `change`.
    fn request(
        slot: Slot,
        at_end: bool,
        change: LengthenChange,
        doc: &Document,
    ) -> Option<EntitiesLengthen> {
        Some(EntitiesLengthen {
            uid: doc.uid(slot)?.to_string(),
            end: if at_end {
                LengthenEnd::End
            } else {
                LengthenEnd::Start
            },
            change,
            expected_revision: None,
        })
    }

    /// Writes the new length through the command.
    fn apply(&mut self, slot: Slot, at_end: bool, change: LengthenChange, cx: &mut Context<'_>) {
        let Some(input) = Self::request(slot, at_end, change, cx.doc) else {
            return;
        };
        let said = lengths(cx.doc, &input);
        let result = entities_lengthen::execute(&mut ExecutionContext::new(cx.doc), input);
        if points::written(result, cx).is_some() {
            if let Some((before, after)) = said {
                let f = cx.format();
                cx.say(
                    Level::Success,
                    format!("Uzunluk {} → {}.", f.length(before), f.length(after)),
                );
            }
            self.hover = None;
        }
    }

    /// The command's plan of `change` at the moving end, and the change beside `at`.
    fn show(
        &mut self,
        slot: Slot,
        at_end: bool,
        change: LengthenChange,
        at: Vec2,
        cx: &Context<'_>,
    ) {
        let Some(input) = Self::request(slot, at_end, change, cx.doc) else {
            return;
        };
        let CommandResult::Completed { output, .. } = entities_lengthen::plan_in(cx.doc, &input)
        else {
            return;
        };
        let (Some(e), Some((before, after))) = (output.changed.first(), lengths(cx.doc, &input))
        else {
            return;
        };
        let f = cx.format();
        let out = Outline::of(&shape(e), None, 2.0, Tone::Accent);
        self.drawn = Preview {
            strokes: out.strokes,
            marks: out.marks,
            tag: Some(Tag {
                at,
                lines: vec![format!("{} → {}", f.length(before), f.length(after))],
            }),
            ..Preview::default()
        };
    }

    /// The preview (the web's `draw`): the end following the mouse, or in
    /// the other modes what a click on the hovered end would do.
    fn redraw(&mut self, cx: &Context<'_>) {
        self.drawn = Preview::default();
        let m = *cx.memory;
        if let (Some((slot, at_end)), Some(mouse)) = (self.target, self.mouse) {
            let to = wire(mouse);
            return self.show(slot, at_end, LengthenChange::Dynamic { to }, mouse, cx);
        }
        if self.target.is_none()
            && m.lengthen_mode != LengthenMode::Dynamic
            && let Some(h) = self.hover
            && let Some(e) = cx.doc.get(h.slot)
        {
            let at_end = near_end(&shape(e), h.at);
            self.show(h.slot, at_end, Self::change_for(&m), h.at, cx);
        }
    }
}

impl Tool for Lengthen {
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
        let m = self.memory();
        let f = self.seen.map(|(_, f)| f).unwrap_or_default();
        match (self.ask, self.target) {
            (Some(LengthenMode::Delta), _) => {
                Prompt::new(LABEL, "uzunluk farkını yazın (eksi kısaltır; yayda <açı)")
            }
            (Some(LengthenMode::Percent), _) => Prompt::new(
                LABEL,
                "yeni uzunluğu eski uzunluğun yüzdesi olarak yazın (100 = aynı)",
            ),
            (Some(_), _) => Prompt::new(LABEL, "yeni toplam uzunluğu yazın (yayda <açı)"),
            (None, Some(_)) => Prompt::new(
                LABEL,
                "yeni ucu fareyle gösterin ya da toplam uzunluğu yazın",
            ),
            (None, None) => Prompt::new(LABEL, "değiştirilecek ucun yakınına tıklayın")
                .note(format!("kip: {}", mode_name(m.lengthen_mode)))
                .then()
                .option("Dinamik", "D")
                .option_with(
                    "Fark",
                    "F",
                    value_text(m.lengthen_delta, m.lengthen_delta_angle, &f),
                )
                .option_with("Yüzde", "Y", js_number(m.lengthen_percent))
                .option_with(
                    "Toplam",
                    "T",
                    value_text(m.lengthen_total, m.lengthen_total_angle, &f),
                ),
        }
    }

    fn point_count(&self) -> usize {
        0
    }

    /// Fark and Toplam take an arc's angle as `<45`.
    fn takes_angle(&self) -> bool {
        matches!(self.ask, Some(LengthenMode::Delta | LengthenMode::Total))
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
            let to = wire(p.world);
            self.apply(slot, at_end, LengthenChange::Dynamic { to }, cx);
            self.drawn = Preview::default();
            return self.see(cx);
        }
        let Some(slot) = edge::pick_said(p, cx, editable) else {
            cx.say(
                Level::Warn,
                "Düzenlenebilir bir çizgiye, yaya, elips yayına ya da açık çoklu çizgiye tıklayın.",
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
        self.apply(slot, at_end, Self::change_for(&m), cx);
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
        // An arc's included angle after Fark or Toplam: `<45` in the project's angle unit.
        if let Some(ask @ (LengthenMode::Delta | LengthenMode::Total)) = self.ask
            && let Some(rest) = js_trim(text).strip_prefix('<')
            && let Some(n) = parse_number(rest)
        {
            let a = cx.format().angle_from_typed(n);
            if ask == LengthenMode::Delta {
                cx.memory.lengthen_delta_angle = Some(a);
            } else if a > 0.0 {
                cx.memory.lengthen_total_angle = Some(a);
            } else {
                cx.say(Level::Warn, "Değer sıfırdan büyük olmalı.");
                return true;
            }
            self.ask = None;
            self.see(cx);
            self.redraw(cx);
            return true;
        }
        let Some((n, length)) = crate::points::plain_value(text, cx)
            .or_else(|| parse_number(text).map(|n| (n, cx.format().to_metres(n))))
        else {
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
            // Lengths are typed in the project's unit (docs/adr/0165 §2) or with their own (docs/adr/0266); a
            // percentage has none.
            match ask {
                LengthenMode::Delta => {
                    cx.memory.lengthen_delta = length;
                    cx.memory.lengthen_delta_angle = None;
                }
                LengthenMode::Percent => cx.memory.lengthen_percent = n,
                _ => {
                    cx.memory.lengthen_total = length;
                    cx.memory.lengthen_total_angle = None;
                }
            }
            self.ask = None;
            self.see(cx);
            self.redraw(cx);
            return true;
        }
        if let Some((slot, at_end)) = self.target.take() {
            let change = LengthenChange::Total {
                length: Some(length),
                angle: None,
            };
            self.apply(slot, at_end, change, cx);
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
