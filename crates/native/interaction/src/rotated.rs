//! Döndürülmüş dikdörtgen: the web's `RotatedRectangleTool`
//! (`apps/web/src/tools/shapeTools.ts`) on its `PointInputTool` base, step
//! for step (docs/adr/0032): one edge (its length and direction with snaps,
//! ortho, polar tracking or a typed distance), then the width, pulled sideways
//! with the mouse or typed (the side the mouse is on decides where it goes).
//! It is written through `cad.polygon.create` and says “Dikdörtgen eklendi:
//! 20.000 × 10.000 m”; Ctrl+Z takes back one point at a time. The rectangle is
//! the shared core's (`rectFromEdge`, `sideDistance`).

use kentos_geometry_core::geom::shapes::{rect_from_edge, side_distance};
use kentos_geometry_core::geometry::{dist, signed_area};
use kentos_geometry_core::tools::point_text::point_from_text;

use crate::Vec2;
use crate::format::Format;
use crate::log::Level;
use crate::points::{self, Taken, plain_number};
use crate::prompt::Prompt;
use crate::tool::{Context, Flow, Pointer, Preview, Stroke, Tag, Tool};

/// The rotated rectangle tool's id: its command is `tool.rectangle3`.
pub const ID: &str = "rectangle3";
pub const LABEL: &str = "Döndürülmüş dikdörtgen";

/// The rotated rectangle tool.
#[derive(Clone, Debug, Default)]
pub struct RotatedRectangle {
    d: Taken,
}

impl RotatedRectangle {
    pub fn new() -> Self {
        Self::default()
    }

    fn accept(&mut self, p: Vec2, cx: &mut Context<'_>) {
        self.d.begin(p, cx);
        if self.d.pts.len() < 2 {
            if self.d.last().is_none_or(|last| dist(last, p) > 1e-9) {
                self.d.pts.push(p);
            }
            return;
        }
        let width = side_distance(self.d.pts[0], self.d.pts[1], p);
        self.commit(width, cx);
    }

    fn commit(&mut self, width: f64, cx: &mut Context<'_>) {
        let Some(ring) = rect_from_edge(self.d.pts[0], self.d.pts[1], width) else {
            cx.say(
                Level::Warn,
                "Genişlik sıfır olamaz; kenardan uzaklaşarak tıklayın.",
            );
            return;
        };
        if points::write_ring(&mut self.d, &ring, None, cx) {
            let format = cx.format();
            let line = format!(
                "Dikdörtgen eklendi: {} × {}",
                format.length_bare(dist(ring[0], ring[1])),
                format.length(width.abs())
            );
            cx.say(Level::Success, line);
        }
        self.d.pts.clear();
    }
}

impl Tool for RotatedRectangle {
    fn id(&self) -> &'static str {
        ID
    }

    fn label(&self) -> &'static str {
        LABEL
    }

    fn prompt(&self) -> Prompt {
        Prompt::new(
            LABEL,
            match self.d.pts.len() {
                0 => "kenarın ilk noktasını belirtin",
                1 => "kenarın ikinci noktasını belirtin ya da uzunluk yazın",
                _ => "genişliği fareyle yana çekerek gösterin ya da yazın",
            },
        )
    }

    fn point_count(&self) -> usize {
        self.d.pts.len()
    }

    fn snap_from(&self) -> Option<Vec2> {
        self.d.last()
    }

    /// The width is measured square to the edge whatever the mouse does.
    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.d.hover = Some(if self.d.pts.len() == 2 {
            p.world
        } else {
            self.d.constrain(p, cx)
        });
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        let p = if self.d.pts.len() == 2 {
            p.world
        } else {
            self.d.constrain(p, cx)
        };
        self.accept(p, cx);
    }

    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        if self.d.pts.len() == 2
            && let Some(n) = plain_number(text).filter(|n| *n > 0.0)
        {
            // A typed width goes to the side the mouse is on.
            let (a, b) = (self.d.pts[0], self.d.pts[1]);
            let side = match self.d.hover {
                Some(h) if side_distance(a, b, h) < 0.0 => -1.0,
                _ => 1.0,
            };
            self.commit(n * side, cx);
            return true;
        }
        match point_from_text(text, self.d.last(), self.d.hover, |_| None) {
            Some(p) => {
                self.accept(p, cx);
                true
            }
            None => false,
        }
    }

    fn confirm(&mut self, _cx: &mut Context<'_>) -> Flow {
        if self.d.pts.is_empty() {
            return Flow::Exit;
        }
        self.d.reset();
        Flow::Stay
    }

    /// Ctrl+Z (ADR 0018): the rectangle just written (as an undo), else one point back.
    fn undo_step(&mut self, cx: &mut Context<'_>) -> bool {
        self.d.undo_step(true, cx)
    }

    fn preview(&self, format: &Format) -> Preview {
        let (Some(h), [a, b]) = (self.d.hover, self.d.pts.as_slice()) else {
            return points::chain_preview(&self.d, format);
        };
        let (a, b) = (*a, *b);
        let w = side_distance(a, b, h);
        let mut preview = Preview {
            strokes: vec![Stroke::solid(vec![a, b], false).width(2.0)],
            ..Preview::default()
        };
        let Some(ring) = rect_from_edge(a, b, w) else {
            return preview;
        };
        preview.tag = Some(Tag {
            at: h,
            lines: vec![
                format!(
                    "{} × {}",
                    format.length_bare(dist(a, b)),
                    format.length(w.abs())
                ),
                format!("Alan {}", format.area(signed_area(&ring).abs())),
            ],
        });
        preview.strokes.push(Stroke::solid(ring, true));
        preview
    }
}
