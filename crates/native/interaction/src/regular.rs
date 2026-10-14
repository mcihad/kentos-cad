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
//! It is written through `cad.polygon.regular` (docs/adr/0290) and says “6
//! kenarlı düzgün çokgen eklendi: kenar …, alan …”. The tool gives the command
//! what was given (the side count, the centre and a corner or an edge's
//! middle, or an edge; a typed radius as the point it gives,
//! `regular_radius_point`); the ring, for the preview too, is the command's
//! (`shapes::regular_ring`). With the overlap control or Topoloji changing the
//! area, it is written as any new area is (`points::write_area_by`).

use kentos_contracts::{RegularFrame, RegularPolygonCreate};
use kentos_geometry_core::entity::tessellate_circle;
use kentos_geometry_core::geometry::{dist, signed_area};
use kentos_geometry_core::tools::drawing::regular_radius_point;
use kentos_geometry_core::tools::point_text::js_trim;
use kentos_native_application::{ExecutionContext, shapes};

use crate::Vec2;
use crate::format::Format;
use crate::log::Level;
use crate::points::{self, Taken};
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

    /// What the command is given for `p`: the edge from the first point, or the centre and `p` as a corner or
    /// an edge's middle.
    fn frame(&self, p: Vec2) -> Option<RegularFrame> {
        let m = self.memory();
        let first = points::wire(*self.d.pts.first()?);
        let p = points::wire(p);
        Some(if self.by_edge {
            RegularFrame::Edge { a: first, b: p }
        } else if m.polygon_inscribed {
            RegularFrame::Inscribed {
                center: first,
                corner: p,
            }
        } else {
            RegularFrame::Circumscribed {
                center: first,
                middle: p,
            }
        })
    }

    /// The command's input for `frame`.
    fn request(&self, frame: RegularFrame, cx: &Context<'_>) -> RegularPolygonCreate {
        RegularPolygonCreate {
            layer_id: cx.doc.layers().active().to_owned(),
            sides: self.memory().polygon_sides,
            frame,
            color: cx.draft.color_text(),
            line_weight: cx.draft.line_weight,
            line_type: cx.draft.line_type,
            attrs: cx.template_attrs(None),
            label: cx.template_label(),
            symbol: cx.template_symbol(),
            expected_revision: None,
        }
    }

    /// The polygon to `p` as the command makes it: the preview.
    fn shape(&self, p: Vec2) -> Option<Vec<Vec2>> {
        let input = RegularPolygonCreate {
            layer_id: String::new(),
            sides: self.memory().polygon_sides,
            frame: self.frame(p)?,
            color: None,
            line_weight: None,
            line_type: None,
            attrs: None,
            label: None,
            symbol: None,
            expected_revision: None,
        };
        shapes::regular_ring(&input).ok()
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
            // The ribbon's methods (docs/adr/0287): the circle through the corners or touching the edges,
            // set, not turned (Çember turns it).
            "İÇ" | "IC" => {
                self.by_edge = false;
                cx.memory.polygon_inscribed = true;
            }
            "DIŞ" | "DIS" => {
                self.by_edge = false;
                cx.memory.polygon_inscribed = false;
            }
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
        let frame = self.frame(p);
        self.commit(frame, cx);
    }

    fn commit(&mut self, frame: Option<RegularFrame>, cx: &mut Context<'_>) {
        let input = frame.map(|frame| self.request(frame, cx));
        let ring = input
            .as_ref()
            .and_then(|input| shapes::regular_ring(input).ok());
        let (Some(ring), Some(input)) = (ring, input) else {
            cx.say(
                Level::Warn,
                "Çokgen için merkezden uzakta bir nokta gösterin.",
            );
            return;
        };
        let mut write = |cx: &mut Context<'_>| {
            let result = shapes::execute_regular(&mut ExecutionContext::new(cx.doc), input.clone());
            points::written(result, cx).map(|out| out.id)
        };
        if let Some(area) = points::write_area_by(&mut self.d, &ring, None, cx, Some(&mut write)) {
            let format = cx.format();
            let line = format!(
                "{} kenarlı düzgün çokgen eklendi: kenar {}, alan {}",
                ring.len(),
                format.length(dist(ring[0], ring[1])),
                format.area(area)
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
        if let Some((n, metres)) = crate::points::plain_value(text, cx) {
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
                // A radius typed in the project's unit (docs/adr/0165 §2).
                let c = self.d.pts[0];
                let at = points::wire(regular_radius_point(
                    c,
                    f64::from(m.polygon_sides),
                    metres,
                    m.polygon_inscribed,
                ));
                let frame = if m.polygon_inscribed {
                    RegularFrame::Inscribed {
                        center: points::wire(c),
                        corner: at,
                    }
                } else {
                    RegularFrame::Circumscribed {
                        center: points::wire(c),
                        middle: at,
                    }
                };
                self.see(cx);
                self.commit(Some(frame), cx);
                return true;
            }
        }
        match cx.typed_point(text, self.d.last(), self.d.hover) {
            Some(p) => {
                self.accept(p, cx);
                true
            }
            None => false,
        }
    }
}

impl Tool for RegularPolygon {
    /// A point computed by the point calculator, as if clicked (the web's `acceptPoint`).
    fn accepts_points(&self) -> bool {
        true
    }

    fn accept_point(&mut self, p: Vec2, cx: &mut Context<'_>) -> bool {
        self.accept(p, cx);
        true
    }
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
