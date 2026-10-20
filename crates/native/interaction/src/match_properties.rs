//! Özellik kopyala (docs/adr/0140, 0304; AutoCAD's MATCHPROP): one object's properties given to others.
//!
//! - Click the source: any object, hovered as the pointer moves; it is
//!   ringed in the snap colour and the tag beside the cursor says what will be
//!   copied.
//! - Then click targets, or drag a window over them (left to right holds only
//!   what the box wholly contains, right to left also what it touches). Each
//!   click or window is one undo step, “Özellik kopyala”; a quick right click
//!   or Enter ends the tool.
//! - The prompt's switches choose what is copied (AutoCAD's Settings), kept for the session: Katman (K), Renk (R),
//!   Çizgi tipi (T, its factor too), Kalınlık (L), Saydamlık (S), Sembol (M), and the kind's Yazı (Y), Ölçü (Ö),
//!   Tarama (A), Kılavuz (V). Every one starts on.
//! - Esc goes back to picking the source; with none picked, it leaves.
//!
//! Attributes and the label are not copied. The tool gives the command `cad.entities.match` the source, the targets
//! and the switches; the command reads the values (a line weight only between objects drawn with lines, the kind's
//! only between objects of one kind) and writes, leaving locked objects out with a warning.

use kentos_commands::{EntitiesMatch, Entity, MatchProperty};
use kentos_domain::Slot;
use kentos_geometry_core::geometry::dist;
use kentos_native_application::geometry::shape;
use kentos_native_application::{ExecutionContext, entities_match};

use crate::Vec2;
use crate::edge::Outline;
use crate::format::Format;
use crate::log::Level;
use crate::points;
use crate::prompt::{Prompt, upper_tr};
use crate::select::SelectBox;
use crate::tool::{Context, Cursor, Flow, Pointer, Preview, Tag, Tone, Tool};

/// Özellik kopyala's id: its command is `tool.matchProperties`.
pub const ID: &str = "matchProperties";
pub const LABEL: &str = "Özellik kopyala";

/// How far the pointer must move with the button down to draw a window, logical pixels.
const DRAG_THRESHOLD: f64 = 4.0;

/// The prompt's switches: their names, keys, the words of the message, and the properties each gives.
const SWITCHES: [(&str, &str, &str, &[MatchProperty]); 10] = [
    ("Katman", "K", "katman", &[MatchProperty::Layer]),
    ("Renk", "R", "renk", &[MatchProperty::Color]),
    (
        "Çizgi tipi",
        "T",
        "çizgi tipi",
        &[MatchProperty::LineType, MatchProperty::LineTypeScale],
    ),
    ("Kalınlık", "L", "kalınlık", &[MatchProperty::LineWeight]),
    (
        "Saydamlık",
        "S",
        "saydamlık",
        &[MatchProperty::Transparency],
    ),
    ("Sembol", "M", "sembol", &[MatchProperty::Symbol]),
    ("Yazı", "Y", "yazı", &[MatchProperty::Text]),
    ("Ölçü", "Ö", "ölçü", &[MatchProperty::Dimension]),
    ("Tarama", "A", "tarama", &[MatchProperty::Hatch]),
    ("Kılavuz", "V", "kılavuz", &[MatchProperty::Leader]),
];

/// Whether switch `i` is on in the session's `off` bits.
fn on(off: u16, i: usize) -> bool {
    off & (1 << i) == 0
}

/// The object whose properties are copied, as they were when it was picked.
#[derive(Clone, Debug, PartialEq)]
struct Source {
    slot: Slot,
    layer_name: String,
    entity: Entity,
}

/// A press on the drawing: where it began and where the pointer is.
#[derive(Clone, Copy, Debug)]
struct Press {
    from: [f64; 2],
    from_world: Vec2,
    to: [f64; 2],
    to_world: Vec2,
    dragging: bool,
}

/// Özellik kopyala.
#[derive(Clone, Debug, Default)]
pub struct MatchProperties {
    source: Option<Source>,
    press: Option<Press>,
    /// The pointer's world point, for the tag.
    hover: Option<Vec2>,
    /// The source ringed, and what the tag says.
    drawn: Preview,
    lines: Vec<String>,
    /// The session's switches turned off, as the prompt last saw them.
    off: u16,
}

impl MatchProperties {
    pub fn new() -> Self {
        Self::default()
    }

    /// The source, read from the drawing.
    fn pick_source(&mut self, slot: Slot, cx: &mut Context<'_>) {
        let Some(e) = cx.doc.get(slot) else { return };
        let base = e.base();
        let layer_name = cx
            .doc
            .layers()
            .get(&base.layer_id)
            .map_or_else(|| base.layer_id.clone(), |n| n.name.clone());
        let outline = Outline::of(&shape(e), None, 2.5, Tone::Snap);
        self.drawn = Preview {
            strokes: outline.strokes,
            marks: outline.marks,
            ..Preview::default()
        };
        self.source = Some(Source {
            slot,
            layer_name,
            entity: e.clone(),
        });
        self.retag(cx);
        cx.selection.set_hover(None);
        let name = self.source.as_ref().map_or("", |s| s.layer_name.as_str());
        let text = format!(
            "Kaynak: “{name}” katmanındaki nesne. Özellikleri verilecek nesnelere tıklayın ya da pencereyle seçin."
        );
        cx.say(Level::Info, text);
    }

    fn retag(&mut self, cx: &Context<'_>) {
        self.off = cx.memory.match_off;
        self.lines = self
            .source
            .as_ref()
            .map(|s| s.tag(&cx.format(), self.off))
            .unwrap_or_default();
    }

    /// The properties the switches give; none when every one is on (the command's default).
    fn properties(off: u16) -> Option<Vec<MatchProperty>> {
        (off != 0).then(|| {
            SWITCHES
                .iter()
                .enumerate()
                .filter(|(i, _)| on(off, *i))
                .flat_map(|(_, s)| s.3.iter().copied())
                .collect()
        })
    }

    /// Gives the source's properties to `slots` through `cad.entities.match`: one undo step.
    fn give(&mut self, slots: Vec<Slot>, cx: &mut Context<'_>) {
        let Some(source) = self.source.clone() else {
            return;
        };
        let off = cx.memory.match_off;
        if (0..SWITCHES.len()).all(|i| !on(off, i)) {
            cx.say(
                Level::Warn,
                "Bütün özellikler kapalı: istemden en az birini açın.",
            );
            return;
        }
        let uids: Vec<String> = slots
            .into_iter()
            .filter(|s| *s != source.slot)
            .filter_map(|s| cx.doc.uid(s))
            .map(|u| u.to_string())
            .collect();
        if uids.is_empty() {
            return;
        }
        let Some(from) = cx.doc.uid(source.slot) else {
            return;
        };
        let input = EntitiesMatch {
            source: from.to_string(),
            uids,
            properties: Self::properties(off),
            expected_revision: None,
        };
        let result = entities_match::execute(&mut ExecutionContext::new(cx.doc), input);
        let Some(out) = points::written(result, cx) else {
            return;
        };
        if out.changed.is_empty() {
            cx.say(
                Level::Info,
                "Nesneler zaten kaynağın seçilen özelliklerinde.",
            );
            return;
        }
        let words: Vec<String> = SWITCHES
            .iter()
            .enumerate()
            .filter(|(i, _)| on(off, *i))
            .map(|(i, s)| {
                if i == 0 {
                    format!("“{}” katmanı", source.layer_name)
                } else {
                    s.2.to_owned()
                }
            })
            .collect();
        cx.say(
            Level::Success,
            format!(
                "Özellikler {} nesneye kopyalandı: {}.",
                out.changed.len(),
                words.join(", ")
            ),
        );
    }
}

impl Source {
    /// What the tag beside the cursor says the targets will get: the switches on.
    fn tag(&self, f: &Format, off: u16) -> Vec<String> {
        let by_layer = "katmana göre";
        let base = self.entity.base();
        let lines = self.entity.draws_lines();
        let mut out = Vec::new();
        if on(off, 0) {
            out.push(format!("Katman: {}", self.layer_name));
        }
        if on(off, 1) {
            out.push(format!(
                "Renk: {}",
                base.color.as_deref().unwrap_or(by_layer)
            ));
        }
        if on(off, 2) {
            let kind = base.look.line_type.map_or(by_layer, |t| match t {
                kentos_commands::LineType::Continuous => "sürekli",
                kentos_commands::LineType::Dashed => "kesikli",
                kentos_commands::LineType::Dashdot => "kesik noktalı",
                kentos_commands::LineType::Dotted => "noktalı",
            });
            out.push(format!("Çizgi tipi: {kind}"));
        }
        if on(off, 3) {
            let weight = match (lines, base.line_weight) {
                (false, _) => "—".to_owned(),
                (true, None) => by_layer.to_owned(),
                (true, Some(w)) => format!("{} mm", f.length_bare(w)),
            };
            out.push(format!("Kalınlık: {weight}"));
        }
        if on(off, 4) {
            out.push(format!(
                "Saydamlık: {}",
                base.look.transparency.map_or_else(
                    || by_layer.to_owned(),
                    |t| format!("%{}", (t * 100.0).round())
                )
            ));
        }
        if on(off, 5) {
            out.push(format!(
                "Sembol: {}",
                base.symbol.as_deref().unwrap_or(by_layer)
            ));
        }
        let kind = match &self.entity {
            Entity::Text(_) if on(off, 6) => Some("Yazı: yüz, yükseklik, genişlik"),
            Entity::Dimension(_) if on(off, 7) => Some("Ölçü: görünüş, değer yüksekliği, zemin"),
            Entity::Hatch(_) if on(off, 8) => Some("Tarama: desen"),
            Entity::Leader(_) if on(off, 9) => Some("Kılavuz: ok, yükseklik, zemin"),
            _ => None,
        };
        out.extend(kind.map(str::to_owned));
        out
    }
}

impl Tool for MatchProperties {
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
        let mut prompt = match &self.source {
            None => Prompt::new(LABEL, "özellikleri alınacak nesneye tıklayın"),
            Some(s) => Prompt::new(
                LABEL,
                "özellik verilecek nesnelere tıklayın ya da pencereyle seçin",
            )
            .note(format!("kaynak: “{}” katmanı", s.layer_name))
            .then()
            .option("Bitir", "Enter"),
        }
        .then();
        for (i, (label, key, _, _)) in SWITCHES.iter().enumerate() {
            prompt =
                prompt.option_with(label, key, if on(self.off, i) { "açık" } else { "kapalı" });
        }
        prompt
    }

    fn activate(&mut self, cx: &mut Context<'_>) -> Flow {
        self.off = cx.memory.match_off;
        Flow::Stay
    }

    fn point_count(&self) -> usize {
        0
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.hover = Some(p.raw);
        match &mut self.press {
            Some(press) => {
                press.to = p.screen;
                // The first corner stays on the drawing through a pan or a zoom (docs/adr/0277).
                press.from = cx.view.to_screen(press.from_world);
                press.to_world = p.raw;
                let moved = dist(
                    Vec2::new(press.from[0], press.from[1]),
                    Vec2::new(p.screen[0], p.screen[1]),
                );
                if !press.dragging && moved > DRAG_THRESHOLD {
                    press.dragging = true;
                    cx.selection.set_hover(None);
                }
            }
            None => {
                let hit = crate::selectable::hover(cx, p.raw);
                cx.selection.set_hover(hit);
            }
        }
    }

    fn pointer_down(&mut self, p: &Pointer, _cx: &mut Context<'_>) {
        self.press = Some(Press {
            from: p.screen,
            from_world: p.raw,
            to: p.screen,
            to_world: p.raw,
            dragging: false,
        });
    }

    /// A click picks the source, then gives to the object under it; a drag
    /// past 4 px gives to what its window holds.
    fn pointer_up(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        let Some(press) = self.press.take() else {
            return;
        };
        // The selection filter's kinds, as every selecting step (docs/adr/0278).
        let hit = crate::selectable::pick(cx, p.raw);
        if self.source.is_none() {
            if press.dragging {
                return;
            }
            match hit {
                Some(slot) => self.pick_source(slot, cx),
                None => cx.say(Level::Warn, "Özellikleri alınacak bir nesneye tıklayın."),
            }
            return;
        }
        if press.dragging {
            let crossing = SelectBox {
                from: cx.view.to_screen(press.from_world),
                to: p.screen,
                worlds: None,
            }
            .crossing();
            let slots = crate::selectable::in_box(cx, press.from_world, p.raw, crossing);
            self.give(slots, cx);
        } else if let Some(slot) = hit {
            self.give(vec![slot], cx);
        } else {
            cx.say(
                Level::Warn,
                "Özellik verilecek bir nesneye tıklayın ya da pencereyle seçin.",
            );
        }
        cx.selection.set_hover(None);
    }

    /// A switch's key turns it on or off for the session.
    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        let key = upper_tr(text.trim());
        let Some(i) = SWITCHES.iter().position(|s| s.1 == key) else {
            return false;
        };
        cx.memory.match_off ^= 1 << i;
        self.retag(cx);
        true
    }

    /// Enter and a quick right click end the tool.
    fn confirm(&mut self, _cx: &mut Context<'_>) -> Flow {
        Flow::Exit
    }

    /// Esc goes back to picking the source; with none picked, it leaves.
    fn cancel(&mut self, cx: &mut Context<'_>) -> bool {
        if self.source.is_none() {
            return false;
        }
        self.source = None;
        self.press = None;
        self.drawn = Preview::default();
        self.lines.clear();
        cx.selection.set_hover(None);
        true
    }

    fn undo_step(&mut self, _cx: &mut Context<'_>) -> bool {
        false
    }

    fn select_box(&self) -> Option<SelectBox> {
        let press = self.press.filter(|p| p.dragging)?;
        Some(SelectBox {
            from: press.from,
            to: press.to,
            worlds: Some((press.from_world, press.to_world)),
        })
    }

    fn preview(&self, _format: &Format) -> Preview {
        let mut preview = self.drawn.clone();
        preview.tag = self.hover.filter(|_| !self.lines.is_empty()).map(|at| Tag {
            at,
            lines: self.lines.clone(),
        });
        preview
    }
}
