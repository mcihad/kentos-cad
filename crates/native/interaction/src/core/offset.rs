//! Ötele: the web's `OffsetTool` (`apps/web/src/tools/edgeTools.ts`) on its
//! edge-picking base, step for step (docs/adr/0047):
//!
//! - click the object to offset (hovered by its edge as the pointer moves);
//! - then the side the copy goes to, at the remembered distance, or with
//!   Noktadan geç (N) the point it passes through (the distance is then the
//!   point's to the object, and object snaps apply);
//! - a typed number sets the distance (and turns Noktadan geç off); the
//!   distance and the options stay for as long as the app lives
//!   ([`crate::tool::Memory`]);
//! - İki yana (I) makes two copies, one on each side of the object, at the
//!   same distance; Kaynağı sil (S) deletes the object in the same step
//!   (docs/adr/0140);
//! - Enter or Esc drops the picked object; with none, Enter leaves.
//!
//! The copy is a new object from the picked one (its layer and colour):
//! “1.000 m ötelenmiş kopya eklendi.” One undo step, copies and deletion
//! together. The tool asks for it by its meaning through the product
//! command `cad.entities.offset` (docs/adr/0246): the object's id, the
//! distance or the point to pass through, the side, İki yana and Kaynağı
//! sil. The preview is the command's plan and the click its write, one
//! computation, the shared core's, as Python and MCP get it.

use kentos_commands::{
    CommandResult, EntitiesOffset, EntitiesOffsetPlan, OffsetApproximation, OffsetBy,
};
use kentos_domain::{Document, Slot};
use kentos_geometry_core::ops::offset::through_distance;
use kentos_geometry_core::tools::point_text::js_trim;
use kentos_native_application::geometry::shape;
use kentos_native_application::{ExecutionContext, offset};

use crate::Vec2;
use crate::edge::{self, Outline};
use crate::format::Format;
use crate::log::Level;
use crate::points::{self, wire};
use crate::prompt::{Prompt, upper_tr};
use crate::tool::{Context, Cursor, Flow, Memory, Pointer, Preview, Tag, Tone, Tool};

/// The offset tool's id: its command is `tool.offset`.
pub const ID: &str = "offset";
pub const LABEL: &str = "Ötele";

fn millimetres(mm: f64) -> String {
    let text = crate::format::fixed(mm, 6);
    if mm > 0. && text.parse::<f64>().ok() == Some(0.) {
        return mm.to_string();
    }
    text.trim_end_matches('0').trim_end_matches('.').to_owned()
}

/// The offset tool.
#[derive(Clone, Debug, Default)]
pub struct Offset {
    /// The object picked to offset.
    target: Option<Slot>,
    /// Where the copy goes: the pointer, snapped with Noktadan geç.
    side: Option<Vec2>,
    /// The preview, made when the pointer moves: `preview` sees no drawing.
    drawn: Preview,
    /// What the session remembered and the project's units, as of the last call.
    seen: Option<(Memory, Format)>,
    ask_tolerance: bool,
    cache: Option<(u64, EntitiesOffset, CommandResult<EntitiesOffsetPlan>)>,
}

impl Offset {
    pub fn new() -> Self {
        Self::default()
    }

    fn see(&mut self, cx: &Context<'_>) {
        self.seen = Some((*cx.memory, cx.format()));
    }

    fn memory(&self) -> Memory {
        self.seen.map(|(m, _)| m).unwrap_or_default()
    }

    /// The distance for a side point: the remembered one, or with Noktadan
    /// geç the point's distance to the object (the core's).
    fn distance_for(&self, target: &kentos_geometry_core::entity::Shape, p: Vec2) -> f64 {
        let m = self.memory();
        if m.offset_through {
            through_distance(target, p)
        } else {
            m.offset_distance
        }
    }

    /// What the tool asks of `cad.entities.offset` for the object at
    /// `slot` and the side point `side`: the remembered distance or, with
    /// Noktadan geç, the point to pass through; İki yana and Kaynağı sil.
    fn request(&self, doc: &Document, slot: Slot, side: Vec2) -> EntitiesOffset {
        let m = self.memory();
        let by = if m.offset_through {
            OffsetBy::Through { point: wire(side) }
        } else {
            OffsetBy::Distance {
                distance: m.offset_distance,
                side: wire(side),
            }
        };
        EntitiesOffset {
            uids: vec![edge::uid(doc, slot)],
            by,
            approximation: m.offset_approximate.then_some(OffsetApproximation {
                tolerance: m.offset_tolerance,
            }),
            both_sides: m.offset_both,
            delete_source: m.offset_erase,
            layer_id: None,
            expected_revision: None,
        }
    }

    /// The copies at `side` dashed (the command's plan), the object solid,
    /// the distance beside the cursor.
    fn redraw(&mut self, cx: &Context<'_>) {
        self.drawn = Preview::default();
        let (Some(slot), Some(side)) = (self.target, self.side) else {
            return;
        };
        let Some(target) = cx.doc.get(slot).map(shape) else {
            return;
        };
        let mut d = self.distance_for(&target, side);
        let mut out = Outline::default();
        let request = self.request(cx.doc, slot, side);
        let mut key = request.clone();
        // A fixed-distance regular curve parallel depends on its oriented
        // side, not every cursor coordinate on that side. Reuse its plan
        // until the document, settings or side changes. Through-point and
        // two-sided requests retain their full point in the cache key.
        if !self.memory().offset_both
            && let OffsetBy::Distance {
                side: ref mut point,
                ..
            } = key.by
            && let Some((p, t)) = kentos_geometry_core::ops::offset_bounded::nearest(&target, side)
        {
            point.x = if t.x * (side.y - p.y) - t.y * (side.x - p.x) > 0. {
                1.
            } else {
                -1.
            };
            point.y = 0.;
        }
        if !self
            .cache
            .as_ref()
            .is_some_and(|(revision, old, _)| *revision == cx.doc.revision() && *old == key)
        {
            let result = offset::plan_in(cx.doc, &request);
            self.cache = Some((cx.doc.revision(), key, result));
        }
        let Some((_, _, planned)) = &self.cache else {
            return;
        };
        let mut diagnostics = Vec::new();
        if let CommandResult::Completed { output, .. } = &planned {
            for report in &output.approximations {
                diagnostics.push(format!(
                    "Yaklaşık · hata ≤ {} mm",
                    millimetres((report.error_bound * 1e9).ceil() / 1e6)
                ));
                if report.crossings > 0 {
                    diagnostics.push(format!("{} kendi kesişimi", report.crossings));
                }
            }
            d = output.distances.first().copied().unwrap_or(d);
            for copy in &output.entities {
                out = out.and(Outline::of(
                    &shape(copy),
                    Some([4.0, 3.0]),
                    1.0,
                    Tone::Accent,
                ));
            }
        }
        if let CommandResult::Failed { error } = &planned {
            // Wrapped near the cursor; failed calculations never look like an
            // approved dashed result. The settings remain available to fix it.
            let mut line = String::new();
            for word in error.message.split_whitespace() {
                if !line.is_empty() && line.chars().count() + word.chars().count() > 42 {
                    diagnostics.push(std::mem::take(&mut line));
                }
                if !line.is_empty() {
                    line.push(' ');
                }
                line.push_str(word);
            }
            if !line.is_empty() {
                diagnostics.push(line);
            }
        }
        let out = out.and(Outline::of(&target, None, 1.0, Tone::Accent));
        let mut lines = vec![format!("Mesafe {}", cx.format().length(d))];
        if self.memory().offset_both {
            lines.push("İki yana".to_owned());
        }
        if self.memory().offset_erase {
            lines.push("Kaynak silinir".to_owned());
        }
        lines.extend(diagnostics);
        self.drawn = Preview {
            strokes: out.strokes,
            marks: out.marks,
            tag: Some(Tag { at: side, lines }),
            ..Preview::default()
        };
    }

    /// Writes the copies, and with Kaynağı sil the deletion of the object,
    /// through `cad.entities.offset`: a copy that does not come out is said
    /// (the command's warning); with none, nothing is written (its refusal).
    fn write(&mut self, slot: Slot, side: Vec2, cx: &mut Context<'_>) -> bool {
        let input = self.request(cx.doc, slot, side);
        let result = offset::execute(&mut ExecutionContext::new(cx.doc), input);
        let Some(made) = points::written(result, cx) else {
            return false;
        };
        let Some(&d) = made.distances.first() else {
            return false;
        };
        cx.memory.offset_distance = d;
        let copies = made.created.len();
        let erase = !made.removed.is_empty();
        let length = cx.format().length(d);
        let made = if copies == 2 {
            format!("{length} iki yana ötelenmiş 2 kopya eklendi")
        } else {
            format!("{length} ötelenmiş kopya eklendi")
        };
        let text = if erase {
            format!("{made}; kaynak silindi.")
        } else {
            format!("{made}.")
        };
        cx.say(Level::Success, text);
        if erase {
            let doc = &*cx.doc;
            cx.selection.retain(|s| doc.get(s).is_some());
            cx.selection.set_hover(None);
        }
        true
    }

    fn drop_target(&mut self) {
        self.target = None;
        self.side = None;
        self.drawn = Preview::default();
        self.cache = None;
    }
}

impl Tool for Offset {
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
        if self.ask_tolerance {
            return Prompt::new(
                LABEL,
                "yaklaşım toleransını yazın (ör. 0.1mm); Enter ile onaylayın",
            );
        }
        // The toggles say their state only when it is on: the prompt has to fit a narrow
        // window with all three of them, and the distance is by the cursor (docs/adr/0140).
        let toggle = |p: Prompt, label: &'static str, key: &'static str, yes: bool| {
            if yes {
                p.option_with(label, key, "açık")
            } else {
                p.option(label, key)
            }
        };
        let toggles = |p: Prompt| {
            let p = toggle(p, "Noktadan geç", "N", m.offset_through);
            let p = toggle(p, "İki yana", "I", m.offset_both);
            let p = toggle(p, "Kaynağı sil", "S", m.offset_erase);
            let p = toggle(p, "Yaklaşım", "Y", m.offset_approximate);
            if m.offset_approximate {
                p.option_with(
                    "Tolerans",
                    "T",
                    format!("{} mm", millimetres(m.offset_tolerance * 1000.)),
                )
            } else {
                p
            }
        };
        match self.target {
            None => toggles(Prompt::new(LABEL, "ötelenecek nesneye tıklayın")),
            Some(_) => toggles(Prompt::new(
                LABEL,
                if m.offset_through {
                    "kopyanın geçeceği noktaya tıklayın"
                } else {
                    "kopyanın gideceği tarafa tıklayın"
                },
            )),
        }
    }

    fn point_count(&self) -> usize {
        0
    }

    fn activate(&mut self, cx: &mut Context<'_>) -> Flow {
        self.see(cx);
        Flow::Stay
    }

    /// Object snaps apply to the point the copy passes through.
    fn snaps(&self) -> bool {
        self.memory().offset_through && self.target.is_some()
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.see(cx);
        if self.target.is_some() {
            self.side = Some(p.world);
            return self.redraw(cx);
        }
        // Picking: the distance a copy would go, by the cursor over an object.
        let hit = edge::hover(p, cx, edge::unlocked);
        self.drawn = match hit {
            Some(_) if !self.memory().offset_through => Preview {
                tag: Some(Tag {
                    at: p.raw,
                    lines: vec![format!(
                        "Mesafe {}",
                        cx.format().length(self.memory().offset_distance)
                    )],
                }),
                ..Preview::default()
            },
            _ => Preview::default(),
        };
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.see(cx);
        let Some(slot) = self.target else {
            let Some(slot) = edge::pick_said(p, cx, edge::unlocked) else {
                cx.say(
                    Level::Warn,
                    "Düzenlenebilir bir çizgi, çoklu çizgi, daire ya da yaya tıklayın.",
                );
                return;
            };
            self.target = Some(slot);
            self.side = Some(p.raw);
            cx.selection.set_hover(None);
            return self.redraw(cx);
        };
        if cx.doc.get(slot).is_none() {
            return self.drop_target();
        }
        if self.write(slot, p.world, cx) {
            self.drop_target();
        }
        self.see(cx);
        self.redraw(cx);
    }

    /// N turns Noktadan geç over, I İki yana and S Kaynağı sil; a number
    /// above zero is the distance (and turns Noktadan geç off).
    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        if self.ask_tolerance {
            if let Some(n) = cx.typed_length(text).filter(|n| n.is_finite() && *n > 0.) {
                cx.memory.offset_tolerance = n;
                self.ask_tolerance = false;
                self.see(cx);
                self.redraw(cx);
            } else {
                cx.say(
                    Level::Warn,
                    "Sonlu, pozitif bir tolerans yazın (ör. 0.1mm).",
                );
            }
            return true;
        }
        match upper_tr(js_trim(text)).as_str() {
            "Y" => cx.memory.offset_approximate = !cx.memory.offset_approximate,
            "T" => self.ask_tolerance = true,
            "N" => cx.memory.offset_through = !cx.memory.offset_through,
            "I" => cx.memory.offset_both = !cx.memory.offset_both,
            "S" => cx.memory.offset_erase = !cx.memory.offset_erase,
            // Typed in the project's unit (docs/adr/0165 §2).
            _ => match cx.typed_length(text) {
                Some(n) if n > 0.0 => {
                    cx.memory.offset_distance = n;
                    cx.memory.offset_through = false;
                }
                _ => return false,
            },
        }
        self.see(cx);
        self.redraw(cx);
        true
    }

    fn settings(&self, f: &Format) -> Vec<crate::tool_settings::Setting> {
        use crate::tool_settings::Setting;
        let m = self.memory();
        let mut rows = vec![
            Setting::text("distance", "Mesafe", f.length(m.offset_distance)),
            Setting::toggle("N", "Noktadan geç", m.offset_through),
            Setting::toggle("I", "İki yana", m.offset_both),
            Setting::toggle("S", "Kaynağı sil", m.offset_erase),
            Setting::toggle("Y", "Eğri yaklaşımı", m.offset_approximate),
        ];
        if m.offset_approximate {
            rows.push(Setting::text(
                "T",
                "Tolerans",
                format!("{} mm", millimetres(m.offset_tolerance * 1000.)),
            ));
            rows.push(Setting::hint("approximation-info","Yaklaşık sonuç","Elips ve eğri paraleli çoklu çizgi olur. Kaynak, Kaynağı sil kapalıysa korunur. Tolerans yakınlaşmadan bağımsızdır."));
        }
        rows
    }

    fn configure(&mut self, key: &str, value: &str, cx: &mut Context<'_>) -> bool {
        match key {
            "N" | "I" | "S" | "Y" => {
                let yes = match value {
                    "yes" => true,
                    "no" => false,
                    _ => return false,
                };
                match key {
                    "N" => cx.memory.offset_through = yes,
                    "I" => cx.memory.offset_both = yes,
                    "S" => cx.memory.offset_erase = yes,
                    _ => cx.memory.offset_approximate = yes,
                }
            }
            "T" => {
                let Some(n) = cx.typed_length(value).filter(|n| n.is_finite() && *n > 0.) else {
                    cx.say(
                        Level::Warn,
                        "Sonlu, pozitif bir tolerans yazın (ör. 0.1mm).",
                    );
                    return false;
                };
                cx.memory.offset_tolerance = n;
            }
            "distance" => return self.input(value, cx),
            _ => return false,
        }
        self.see(cx);
        self.redraw(cx);
        true
    }

    /// Enter drops the picked object; with none, the tool leaves.
    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow {
        self.see(cx);
        if self.ask_tolerance {
            self.ask_tolerance = false;
            return Flow::Stay;
        }
        if self.target.is_some() {
            self.drop_target();
            return Flow::Stay;
        }
        Flow::Exit
    }

    fn cancel(&mut self, cx: &mut Context<'_>) -> bool {
        self.see(cx);
        if self.ask_tolerance {
            self.ask_tolerance = false;
            return true;
        }
        if self.target.is_none() {
            return false;
        }
        self.drop_target();
        true
    }

    /// Ctrl+Z undoes the drawing: the web's edge tools take no step back.
    fn undo_step(&mut self, _cx: &mut Context<'_>) -> bool {
        false
    }

    fn preview(&self, _format: &Format) -> Preview {
        self.drawn.clone()
    }
}
