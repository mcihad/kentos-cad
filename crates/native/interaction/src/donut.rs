//! Halka: the web's `DonutTool` (AutoCAD DONUT,
//! `apps/web/src/tools/markupTools.ts`) on its `PointInputTool` base, step
//! for step (docs/adr/0057):
//!
//! - every click (or typed point) places a filled ring at it; the tool waits
//!   for the next;
//! - İç çap (İ, or I) and Dış çap (D) ask for the diameters; an inner one of
//!   0 gives a filled disc, a bigger hole keeps the ring's width; both stay
//!   for as long as the app lives.
//!
//! Every ring is a solid hatch written through `cad.entities.create`, its
//! own object and undo step (“Ekle”); the ring says nothing but its point,
//! as on the web. The rings' points are the shared core's (`donut_rings`).

use kentos_contracts::{EntityGeometry, HatchPattern, HatchPatternType};
use kentos_geometry_core::tools::drawing::donut_rings;
use kentos_geometry_core::tools::point_text::{js_trim, point_from_text};

use crate::Vec2;
use crate::format::Format;
use crate::log::Level;
use crate::points::{self, Taken, wire_all};
use crate::prompt::{Prompt, upper_tr};
use crate::tool::{Area, Context, Flow, Memory, Pointer, Preview, Tool};

/// The donut tool's id: its command is `tool.donut`.
pub const ID: &str = "donut";
pub const LABEL: &str = "Halka";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Ask {
    Inner,
    Outer,
}

/// The donut tool.
#[derive(Clone, Debug, Default)]
pub struct Donut {
    d: Taken,
    ask: Option<Ask>,
    /// What the session remembered and the project's units, as of the last call.
    seen: Option<(Memory, Format)>,
}

impl Donut {
    pub fn new() -> Self {
        Self::default()
    }

    fn see(&mut self, cx: &Context<'_>) {
        self.seen = Some((*cx.memory, cx.format()));
    }

    fn diameters(&self) -> (f64, f64) {
        let m = self.seen.map_or_else(Memory::default, |(m, _)| m);
        (m.donut_inner, m.donut_outer)
    }

    fn option(&mut self, key: &str) -> bool {
        self.ask = match key {
            "İ" | "I" => Some(Ask::Inner),
            "D" => Some(Ask::Outer),
            _ => return false,
        };
        true
    }

    fn accept(&mut self, p: Vec2, cx: &mut Context<'_>) {
        self.d.begin(p, cx);
        if self.ask.is_some() {
            return;
        }
        let rings = donut_rings(p, cx.memory.donut_inner, cx.memory.donut_outer);
        let geometry = EntityGeometry::Hatch {
            ring: wire_all(&rings.ring),
            holes: rings
                .holes
                .map(|holes| holes.iter().map(|h| wire_all(h)).collect()),
            // A solid fill with the inner circle left out: AutoCAD's wide polyline.
            pattern: HatchPattern {
                kind: HatchPatternType::Solid,
                angle: 0.0,
                spacing: 1.0,
            },
        };
        if let Some(out) = points::write_objects(vec![geometry], None, cx)
            && let Some(&id) = out.ids.first()
        {
            self.d.note(id, cx);
        }
    }

    /// A typed diameter while one is asked for. A bigger hole keeps the
    /// ring's width: the outer diameter follows.
    fn diameter(&mut self, ask: Ask, n: f64, cx: &mut Context<'_>) {
        let (inner, outer) = (cx.memory.donut_inner, cx.memory.donut_outer);
        match ask {
            Ask::Inner if n < 0.0 => {
                cx.say(Level::Warn, "İç çap sıfır ya da pozitif olmalı.");
            }
            Ask::Inner => {
                if n >= outer {
                    cx.memory.donut_outer = n + (outer - inner);
                }
                cx.memory.donut_inner = n;
                self.ask = None;
            }
            Ask::Outer if n <= inner => {
                let inner = cx.format().length(inner);
                cx.say(
                    Level::Warn,
                    format!("Dış çap iç çaptan ({inner}) büyük olmalı."),
                );
            }
            Ask::Outer => {
                cx.memory.donut_outer = n;
                self.ask = None;
            }
        }
    }
}

impl Tool for Donut {
    fn id(&self) -> &'static str {
        ID
    }

    fn label(&self) -> &'static str {
        LABEL
    }

    fn prompt(&self) -> Prompt {
        let step = match self.ask {
            Some(Ask::Inner) => "iç çapı yazın",
            Some(Ask::Outer) => "dış çapı yazın",
            None => "halkanın merkezine tıklayın",
        };
        let (inner, outer) = self.diameters();
        let format = self.seen.map_or_else(Format::default, |(_, f)| f);
        Prompt::new(LABEL, step)
            .option_with("İç çap", "İ", format.length(inner))
            .option_with("Dış çap", "D", format.length(outer))
    }

    fn point_count(&self) -> usize {
        self.d.pts.len()
    }

    fn activate(&mut self, cx: &mut Context<'_>) -> Flow {
        self.see(cx);
        Flow::Stay
    }

    fn snap_from(&self) -> Option<Vec2> {
        self.d.last()
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.d.hover = Some(self.d.constrain(p, cx));
        self.see(cx);
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        let p = self.d.constrain(p, cx);
        self.accept(p, cx);
        self.see(cx);
    }

    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        let done = if self.option(&upper_tr(js_trim(text))) {
            true
        } else if let (Some(ask), Some(n)) = (self.ask, points::plain_number(text)) {
            self.diameter(ask, n, cx);
            true
        } else {
            match point_from_text(text, self.d.last(), self.d.hover, |_| None) {
                Some(p) => {
                    self.accept(p, cx);
                    true
                }
                None => false,
            }
        };
        self.see(cx);
        done
    }

    /// It never holds a point: a confirm leaves (the web's `PointInputTool.confirm`).
    fn confirm(&mut self, _cx: &mut Context<'_>) -> Flow {
        Flow::Exit
    }

    fn undo_step(&mut self, cx: &mut Context<'_>) -> bool {
        self.d.undo_step(false, cx)
    }

    /// The ring under the cursor, filled; nothing while a diameter is asked for.
    fn preview(&self, _format: &Format) -> Preview {
        let (Some(h), None) = (self.d.hover, self.ask) else {
            return Preview::default();
        };
        let (inner, outer) = self.diameters();
        let rings = donut_rings(h, inner, outer);
        Preview {
            areas: vec![Area {
                rings: std::iter::once(rings.ring)
                    .chain(rings.holes.into_iter().flatten())
                    .collect(),
                fill: 0.35,
                width: 1.0,
            }],
            ..Preview::default()
        }
    }
}
