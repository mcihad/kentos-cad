//! Özellik kopyala (docs/adr/0140): one object's layer, colour, line weight
//! and symbol given to others.
//!
//! - Click the source: any object, hovered as the pointer moves; it is
//!   ringed in the snap colour and the tag beside the cursor says what will be
//!   copied.
//! - Then click targets, or drag a window over them (left to right holds only
//!   what the box wholly contains, right to left also what it touches). Each
//!   click or window is one undo step, “Özellik kopyala”; a quick right click
//!   or Enter ends the tool.
//! - Esc goes back to picking the source; with none picked, it leaves.
//!
//! Attributes and the label are not copied, and a line weight only goes to
//! what is drawn with lines, and only when the source is (a point has none to
//! give). Objects on a locked layer are left out of a window and refused when
//! clicked; the command says so. Written through `cad.entities.set` (layer,
//! colour, line weight and symbol in one input); the tool names the step.

use kentos_contracts::{EntitiesSetProperties, PropertiesOperation};
use kentos_domain::Slot;
use kentos_geometry_core::geometry::dist;
use kentos_native_application::geometry::shape;
use kentos_native_application::{ExecutionContext, set};

use crate::Vec2;
use crate::edge::{self, Outline};
use crate::format::Format;
use crate::log::Level;
use crate::points;
use crate::prompt::Prompt;
use crate::select::SelectBox;
use crate::tool::{Context, Cursor, Flow, Pointer, Preview, Tag, Tone, Tool};

/// Özellik kopyala's id: its command is `tool.matchProperties`.
pub const ID: &str = "matchProperties";
pub const LABEL: &str = "Özellik kopyala";

/// How far the pointer must move with the button down to draw a window, logical pixels.
const DRAG_THRESHOLD: f64 = 4.0;

/// The object whose properties are copied, as they were when it was picked.
#[derive(Clone, Debug, PartialEq)]
struct Source {
    slot: Slot,
    layer: String,
    layer_name: String,
    color: Option<String>,
    line_weight: Option<f64>,
    symbol: Option<String>,
    /// Whether it is drawn with lines, so its weight means something.
    lines: bool,
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
        let source = Source {
            slot,
            layer: base.layer_id.clone(),
            layer_name,
            color: base.color.clone(),
            line_weight: base.line_weight,
            symbol: base.symbol.clone(),
            lines: e.draws_lines(),
        };
        let outline = Outline::of(&shape(e), None, 2.5, Tone::Snap);
        self.drawn = Preview {
            strokes: outline.strokes,
            marks: outline.marks,
            ..Preview::default()
        };
        self.lines = source.tag(&cx.format());
        self.source = Some(source);
        cx.selection.set_hover(None);
        let name = &self.source.as_ref().map_or("", |s| s.layer_name.as_str());
        let text = format!(
            "Kaynak: “{name}” katmanındaki nesne. Özellikleri verilecek nesnelere tıklayın ya da pencereyle seçin."
        );
        cx.say(Level::Info, text);
    }

    /// Gives the source's properties to `slots` as one undo step.
    fn give(&mut self, slots: Vec<Slot>, cx: &mut Context<'_>) {
        let Some(source) = self.source.clone() else {
            return;
        };
        let doc = &*cx.doc;
        let all: Vec<Slot> = slots
            .into_iter()
            .filter(|s| *s != source.slot && doc.get(*s).is_some())
            .collect();
        let unlocked: Vec<Slot> = all
            .iter()
            .copied()
            .filter(|s| doc.get(*s).is_some_and(|e| edge::unlocked(e, doc)))
            .collect();
        if unlocked.len() < all.len() && all.len() > 1 {
            cx.say(
                Level::Warn,
                format!(
                    "{} nesne kilitli katmanda olduğu için atlandı.",
                    all.len() - unlocked.len()
                ),
            );
        }
        // A single click on a locked object goes to the command, which says why.
        let targets = if all.len() == 1 { all } else { unlocked };
        if targets.is_empty() {
            return;
        }
        // A weight goes only to what is drawn with lines; the rest get the other three.
        let (lined, plain): (Vec<Slot>, Vec<Slot>) = targets
            .iter()
            .partition(|s| cx.doc.get(**s).is_some_and(|e| e.draws_lines()));
        let group = cx.doc.begin_group(LABEL);
        let mut changed = 0;
        let mut failed = false;
        for (slots, weight) in [(lined, source.lines), (plain, false)] {
            if slots.is_empty() {
                continue;
            }
            let input = EntitiesSetProperties {
                uids: slots
                    .iter()
                    .filter_map(|s| cx.doc.uid(*s))
                    .map(|u| u.to_string())
                    .collect(),
                layer_id: Some(source.layer.clone()),
                color: Some(source.color.clone()),
                line_weight: weight.then_some(source.line_weight),
                symbol: Some(source.symbol.clone()),
                attrs: None,
                label: None,
                operation: PropertiesOperation::Layer,
                expected_revision: None,
                unlink: false,
            };
            let result = set::execute(&mut ExecutionContext::new(cx.doc), input);
            match points::written(result, cx) {
                Some(out) => changed += out.changed.len(),
                None => {
                    failed = true;
                    break;
                }
            }
        }
        if failed {
            cx.doc.cancel_group(group);
            return;
        }
        cx.doc.end_group(group);
        if changed == 0 {
            cx.say(
                Level::Info,
                "Nesneler zaten kaynağın katmanında, renginde, kalınlığında ve sembolünde.",
            );
            return;
        }
        cx.say(
            Level::Success,
            format!(
                "Özellikler {changed} nesneye kopyalandı: “{}” katmanı, renk, kalınlık, sembol.",
                source.layer_name
            ),
        );
    }
}

impl Source {
    /// What the tag beside the cursor says the targets will get.
    fn tag(&self, f: &Format) -> Vec<String> {
        let by_layer = "katmana göre";
        let weight = match (self.lines, self.line_weight) {
            (false, _) => "—".to_owned(),
            (true, None) => by_layer.to_owned(),
            (true, Some(w)) => format!("{} mm", f.length_bare(w)),
        };
        vec![
            format!("Katman: {}", self.layer_name),
            format!("Renk: {}", self.color.as_deref().unwrap_or(by_layer)),
            format!("Kalınlık: {weight}"),
            format!("Sembol: {}", self.symbol.as_deref().unwrap_or(by_layer)),
        ]
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
        match &self.source {
            None => Prompt::new(LABEL, "özellikleri alınacak nesneye tıklayın"),
            Some(s) => Prompt::new(
                LABEL,
                "özellik verilecek nesnelere tıklayın ya da pencereyle seçin",
            )
            .note(format!("kaynak: “{}” katmanı", s.layer_name))
            .then()
            .option("Bitir", "Enter"),
        }
    }

    fn point_count(&self) -> usize {
        0
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.hover = Some(p.raw);
        match &mut self.press {
            Some(press) => {
                press.to = p.screen;
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
                let hit = cx.spatial.pick(p.raw, cx.pick_tolerance());
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
        let hit = cx.spatial.pick(p.raw, cx.pick_tolerance());
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
                from: press.from,
                to: press.to,
            }
            .crossing();
            let slots = cx
                .spatial
                .in_rect(press.from_world, press.to_world, crossing);
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

    fn input(&mut self, _text: &str, _cx: &mut Context<'_>) -> bool {
        false
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
