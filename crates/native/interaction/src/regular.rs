//! Düzgün çokgen: the web's `RegularPolygonTool` (AutoCAD POLYGON,
//! `apps/web/src/tools/shapeTools.ts`) on its `PointInputTool` base, step for
//! step (docs/adr/0032):
//!
//! - the number of sides, typed before the centre or after Kenar sayısı (S),
//!   a whole number from 3 to 1024;
//! - then the centre and a corner (the circle through the corners), or with
//!   Çember (Ç) an edge's middle (the circle touching the edges); a typed
//!   radius keeps the bottom edge horizontal;
//! - or Kenardan (K): the two ends of one edge; Merkezden (M) goes back;
//! - the side count and the circle stay for the next polygons, for as long as
//!   the app lives ([`crate::tool::Memory`]).
//!
//! It is written through `cad.polygon.create` and says “6 kenarlı düzgün
//! çokgen eklendi: kenar …, alan …”. The polygon is the shared core's
//! (`regularPolygon`, `regularPolygonOnEdge`, `regularPolygonRadius`).

use kentos_geometry_core::entity::tessellate_circle;
use kentos_geometry_core::geom::shapes::{regular_polygon, regular_polygon_on_edge};
use kentos_geometry_core::geometry::{dist, signed_area};
use kentos_geometry_core::tools::drawing::regular_polygon_radius;
use kentos_geometry_core::tools::point_text::{js_trim, point_from_text};

use crate::Vec2;
use crate::format::Format;
use crate::log::Level;
use crate::points::{self, Taken, plain_number};
use crate::prompt::{Prompt, upper_tr};
use crate::tool::{Context, Flow, Memory, Pointer, Preview, Stroke, Tag, Tool};

/// The regular polygon tool's id: its command is `tool.regularPolygon`.
pub const ID: &str = "regularPolygon";
pub const LABEL: &str = "Düzgün çokgen";

/// The regular polygon tool.
#[derive(Clone, Debug, Default)]
pub struct RegularPolygon {
    d: Taken,
    /// Kenardan: the two ends of an edge instead of a centre.
    by_edge: bool,
    /// Kenar sayısı: the next number typed is the side count.
    ask_sides: bool,
    /// What the session remembered and the project's units, as of the last call.
    seen: Option<(Memory, Format)>,
}

impl RegularPolygon {
    pub fn new() -> Self {
        Self::default()
    }

    fn see(&mut self, cx: &Context<'_>) {
        self.seen = Some((*cx.memory, cx.format()));
    }

    fn memory(&self) -> Memory {
        self.seen.map(|(m, _)| m).unwrap_or_default()
    }

    /// The polygon to `p`: on the edge from the first point, or about it as the centre.
    fn shape(&self, p: Vec2) -> Option<Vec<Vec2>> {
        let m = self.memory();
        let first = *self.d.pts.first()?;
        let sides = f64::from(m.polygon_sides);
        if self.by_edge {
            regular_polygon_on_edge(first, p, sides)
        } else {
            let mode = if m.polygon_inscribed {
                "inscribed"
            } else {
                "circumscribed"
            };
            regular_polygon(first, sides, p, mode)
        }
    }

    fn option(&mut self, key: &str, cx: &mut Context<'_>) -> bool {
        if !self.d.pts.is_empty() {
            return false;
        }
        match key {
            "S" => self.ask_sides = true,
            "Ç" | "C" => cx.memory.polygon_inscribed = !cx.memory.polygon_inscribed,
            "K" => self.by_edge = true,
            "M" => self.by_edge = false,
            _ => return false,
        }
        true
    }

    fn accept(&mut self, p: Vec2, cx: &mut Context<'_>) {
        self.d.begin(p, cx);
        if self.ask_sides {
            return;
        }
        if self.d.pts.is_empty() {
            self.d.pts.push(p);
            return;
        }
        self.see(cx);
        let ring = self.shape(p);
        self.commit(ring, cx);
    }

    fn commit(&mut self, ring: Option<Vec<Vec2>>, cx: &mut Context<'_>) {
        let Some(ring) = ring else {
            cx.say(
                Level::Warn,
                "Çokgen için merkezden uzakta bir nokta gösterin.",
            );
            return;
        };
        if points::write_ring(&mut self.d, &ring, None, cx) {
            let format = cx.format();
            let line = format!(
                "{} kenarlı düzgün çokgen eklendi: kenar {}, alan {}",
                ring.len(),
                format.length(dist(ring[0], ring[1])),
                format.area(signed_area(&ring).abs())
            );
            cx.say(Level::Success, line);
        }
        self.d.pts.clear();
    }

    /// Typed text (the web's `input`): an option, the side count, a radius, or a point.
    fn typed(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        if self.option(&upper_tr(js_trim(text)), cx) {
            return true;
        }
        if let Some(n) = plain_number(text) {
            // Before any point, a bare number is the side count; after the centre, a radius.
            if self.ask_sides || self.d.pts.is_empty() {
                if n.fract() != 0.0 || !(3.0..=1024.0).contains(&n) {
                    cx.say(
                        Level::Warn,
                        "Kenar sayısı 3 ile 1024 arasında bir tam sayı olmalı.",
                    );
                    return true;
                }
                cx.memory.polygon_sides = n as u32;
                self.ask_sides = false;
                return true;
            }
            if !self.by_edge && self.d.pts.len() == 1 && n > 0.0 {
                // The bottom edge horizontal: the edge middle sits straight below the centre.
                let m = *cx.memory;
                let ring = regular_polygon_radius(
                    self.d.pts[0],
                    f64::from(m.polygon_sides),
                    n,
                    m.polygon_inscribed,
                );
                self.commit(ring, cx);
                return true;
            }
        }
        match point_from_text(text, self.d.last(), self.d.hover, |_| None) {
            Some(p) => {
                self.accept(p, cx);
                true
            }
            None => false,
        }
    }
}

impl Tool for RegularPolygon {
    fn id(&self) -> &'static str {
        ID
    }

    fn label(&self) -> &'static str {
        LABEL
    }

    fn prompt(&self) -> Prompt {
        let m = self.memory();
        let sides = m.polygon_sides.to_string();
        let n = self.d.pts.len();
        if self.ask_sides {
            return Prompt::new(LABEL, "kenar sayısını yazın (3 ile 1024 arası)");
        }
        if self.by_edge {
            return if n == 0 {
                Prompt::new(LABEL, "kenarın ilk ucunu belirtin")
                    .option_with("Kenar sayısı", "S", sides)
                    .option("Merkezden", "M")
            } else {
                Prompt::new(LABEL, "kenarın ikinci ucunu belirtin")
            };
        }
        if n == 0 {
            let circle = if m.polygon_inscribed {
                "köşeler üzerinde"
            } else {
                "kenarlara teğet"
            };
            return Prompt::new(LABEL, "merkezi belirtin ya da kenar sayısını yazın")
                .option_with("Kenar sayısı", "S", sides)
                .option_with("Çember", "Ç", circle)
                .option("Kenardan", "K");
        }
        Prompt::new(
            LABEL,
            if m.polygon_inscribed {
                "bir köşeyi gösterin ya da çember yarıçapını yazın"
            } else {
                "bir kenarın ortasını gösterin ya da iç teğet çember yarıçapını yazın"
            },
        )
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
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        let p = self.d.constrain(p, cx);
        self.accept(p, cx);
        self.see(cx);
    }

    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        let done = self.typed(text, cx);
        self.see(cx);
        done
    }

    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow {
        let flow = if self.d.pts.is_empty() {
            Flow::Exit
        } else {
            self.d.reset();
            Flow::Stay
        };
        self.see(cx);
        flow
    }

    fn undo_step(&mut self, cx: &mut Context<'_>) -> bool {
        let done = self.d.undo_step(false, cx);
        self.see(cx);
        done
    }

    fn preview(&self, format: &Format) -> Preview {
        let (Some(h), Some(&c)) = (self.d.hover, self.d.pts.first()) else {
            return points::chain_preview(&self.d, format);
        };
        let Some(ring) = self.shape(h) else {
            return Preview::default();
        };
        let mut strokes = Vec::new();
        if !self.by_edge {
            strokes.push(Stroke::dashed(
                tessellate_circle(c, dist(c, h), 96.0),
                true,
                [2.0, 4.0],
            ));
            strokes.push(Stroke::dashed(vec![c, h], false, [3.0, 3.0]));
        }
        let lines = vec![
            format!("{} kenar", ring.len()),
            format!("Kenar {}", format.length(dist(ring[0], ring[1]))),
            format!("Alan {}", format.area(signed_area(&ring).abs())),
        ];
        strokes.push(Stroke::solid(ring, true));
        Preview {
            strokes,
            tag: Some(Tag { at: h, lines }),
            ..Preview::default()
        }
    }
}
