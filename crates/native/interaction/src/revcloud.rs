//! Revizyon bulutu: the web's `RevCloudTool` (AutoCAD REVCLOUD,
//! `apps/web/src/tools/markupTools.ts`) on its `PointInputTool` base, step
//! for step (docs/adr/0057):
//!
//! - a rectangle's two corners (the default), or with Çokgen (Ç, or C) a
//!   polygon's corners ended by Enter or a quick right click; Dikdörtgen (D)
//!   goes back; both before the first corner; Nesne (N, AutoCAD's Object,
//!   docs/adr/0287): every closed object clicked (an area, a closed polyline,
//!   a circle, a closed curve, a full ellipse) gets a cloud round its outer
//!   outline (`cloud_around`), the object stays;
//! - Yay boyu (U): the arcs' length in paper millimetres at the project's
//!   plot scale; the shape and the length stay for as long as the app lives.
//!
//! The cloud is a closed area of outward arcs written by `cad.revcloud.create`
//! (docs/adr/0622 §9), its own object and undo step (“Ekle”): “Revizyon
//! bulutu eklendi: 24 yay.” The tool gives the rectangle's corners, the
//! polygon's or the object's persistent id and the arc in metres (paper
//! millimetres at the plot scale); the preview is the command's ring
//! (`shapes::revcloud_ring`). With Çakışma or Topoloji changing the area it is
//! written as any area is (`points::write_area_by`).

use kentos_commands::{RevcloudCreate, RevcloudFrame};
use kentos_geometry_core::geom::arc::DEFAULT_STEP;
use kentos_geometry_core::geom::bulge::bulge_path_outline;
use kentos_geometry_core::geometry::dist;
use kentos_geometry_core::tools::point_text::{js_trim, parse_number};
use kentos_native_application::{ExecutionContext, shapes};

use crate::Vec2;
use crate::format::{Format, js_number};
use crate::log::Level;
use crate::points::{self, Taken};
use crate::prompt::{Prompt, upper_tr};
use crate::tool::{Context, Flow, Memory, Pointer, Preview, Stroke, Tool};

/// The revision cloud tool's id: its command is `tool.revcloud`.
pub const ID: &str = "revcloud";
pub const LABEL: &str = "Revizyon bulutu";

/// The revision cloud tool.
#[derive(Clone, Debug, Default)]
pub struct RevCloud {
    d: Taken,
    ask_arc: bool,
    /// Nesne: clicks pick closed objects.
    object: bool,
    /// What the session remembered and the project's plot scale, as of the last call.
    seen: Option<(Memory, f64)>,
}

impl RevCloud {
    pub fn new() -> Self {
        Self::default()
    }

    fn see(&mut self, cx: &Context<'_>) {
        self.seen = Some((*cx.memory, cx.doc.settings().plot_scale));
    }

    fn memory(&self) -> Memory {
        self.seen.map_or_else(Memory::default, |(m, _)| m)
    }

    /// The arcs' length in world metres: paper millimetres at the plot scale.
    fn arc_length(&self) -> f64 {
        let (m, scale) = self.seen.unwrap_or((Memory::default(), 1000.0));
        (m.cloud_arc_mm / 1000.0) * scale
    }

    /// Dikdörtgen (D), Çokgen (Ç) and Yay boyu (U), before the first corner.
    fn option(&mut self, key: &str, cx: &mut Context<'_>) -> bool {
        if !self.d.pts.is_empty() {
            return false;
        }
        match key {
            "D" => {
                cx.memory.cloud_rect = true;
                self.object = false;
            }
            "Ç" | "C" => {
                cx.memory.cloud_rect = false;
                self.object = false;
            }
            "N" => self.object = true,
            "U" => self.ask_arc = true,
            _ => return false,
        }
        true
    }

    fn accept(&mut self, p: Vec2, cx: &mut Context<'_>) {
        self.d.begin(p, cx);
        if self.ask_arc || self.d.last().is_some_and(|last| dist(last, p) <= 1e-9) {
            return;
        }
        self.d.pts.push(p);
        if cx.memory.cloud_rect && self.d.pts.len() == 2 {
            self.finish(cx);
        }
    }

    /// The command's input for `frame`, with the drawing's look and the template's attributes.
    fn request(&self, frame: RevcloudFrame, cx: &Context<'_>) -> RevcloudCreate {
        RevcloudCreate {
            layer_id: cx.doc.layers().active().to_owned(),
            frame,
            arc: self.arc_length(),
            color: cx.draft.color_text(),
            line_weight: cx.draft.line_weight,
            line_type: cx.draft.line_type,
            attrs: cx.template_attrs(None),
            label: cx.template_label(),
            symbol: cx.template_symbol(),
            expected_revision: None,
            rules: None,
        }
    }

    /// The frame of the corners so far: the rectangle on two corners, or the corners.
    fn frame_of(&self, pts: &[Vec2]) -> RevcloudFrame {
        match (self.memory().cloud_rect, pts) {
            (true, [a, b, ..]) => RevcloudFrame::Rectangle {
                a: points::wire(*a),
                b: points::wire(*b),
            },
            _ => RevcloudFrame::Polygon {
                pts: pts.iter().copied().map(points::wire).collect(),
            },
        }
    }

    /// The cloud as the command makes it, written by the command unless Çakışma or Topoloji changes it; a refusal
    /// says the command's reason.
    fn write(&mut self, input: RevcloudCreate, cx: &mut Context<'_>) {
        match shapes::revcloud_ring(Some(&*cx.doc), &input) {
            Ok((pts, _)) => {
                let arcs = pts.len();
                let mut input = input;
                let rules = points::area_rules(cx, &input.layer_id);
                input.rules = rules;
                let mut write = |cx: &mut Context<'_>| {
                    let result = shapes::execute_revcloud(
                        &mut ExecutionContext::with_store(cx.doc, cx.spatial.store()),
                        input.clone(),
                    );
                    points::written(result, cx).map(|out| out.id)
                };
                if points::write_area_by(&mut self.d, cx, &mut write).is_some() {
                    cx.say(
                        Level::Success,
                        format!("Revizyon bulutu eklendi: {arcs} yay."),
                    );
                }
            }
            Err(error) => cx.say(Level::Warn, error.message),
        }
    }

    /// Nesne: the cloud round the closed object under the press, by its persistent id.
    fn around(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.see(cx);
        let uid = cx
            .spatial
            .pick_edge(p.raw, cx.pick_tolerance(), |s| cx.doc.get(s).is_some())
            .and_then(|slot| cx.doc.uid(slot))
            .map(|uid| uid.to_string());
        match uid {
            Some(uid) => {
                let input = self.request(RevcloudFrame::Object { uid }, cx);
                self.write(input, cx);
                self.d.reset();
            }
            None => cx.say(
                Level::Warn,
                "Kapalı bir nesneye tıklayın: alan, kapalı çoklu çizgi, daire, kapalı eğri ya da elips.",
            ),
        }
    }

    /// Writes the cloud through `cad.revcloud.create` (docs/adr/0622 §9); a
    /// refused one says only the command's reason. The draft starts over.
    fn finish(&mut self, cx: &mut Context<'_>) {
        self.see(cx);
        if !self.d.pts.is_empty() {
            let input = self.request(self.frame_of(&self.d.pts), cx);
            self.write(input, cx);
        }
        self.d.reset();
    }
}

impl Tool for RevCloud {
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
        if self.ask_arc {
            return Prompt::new(LABEL, "yay boyunu kâğıt milimetresi olarak yazın");
        }
        let m = self.memory();
        let n = self.d.pts.len();
        let object = self.object;
        let first = |step: &'static str| {
            let prompt = Prompt::new(LABEL, step);
            let prompt = if object {
                prompt.option("Dikdörtgen", "D").option("Çokgen", "Ç")
            } else if m.cloud_rect {
                prompt.option("Çokgen", "Ç").option("Nesne", "N")
            } else {
                prompt.option("Dikdörtgen", "D").option("Nesne", "N")
            };
            prompt.option_with("Yay boyu", "U", format!("{} mm", js_number(m.cloud_arc_mm)))
        };
        if object {
            return first("buluta çevrilecek kapalı nesneye tıklayın");
        }
        match (m.cloud_rect, n) {
            (true, 0) => first("dikdörtgenin bir köşesine tıklayın"),
            (true, _) => Prompt::new(LABEL, "karşı köşeye tıklayın"),
            (false, 0) => first("bulutun ilk köşesine tıklayın"),
            (false, 1 | 2) => Prompt::new(LABEL, "sonraki köşeye tıklayın"),
            (false, _) => Prompt::new(
                LABEL,
                "sonraki köşeye tıklayın ya da bitirmek için sağ tıklayın",
            ),
        }
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
        if self.object && !self.ask_arc {
            self.around(p, cx);
            return;
        }
        let p = self.d.constrain(p, cx);
        self.accept(p, cx);
        self.see(cx);
    }

    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        let done = if self.option(&upper_tr(js_trim(text)), cx) {
            true
        } else if let (true, Some(n)) = (self.ask_arc, parse_number(text)) {
            if n > 0.0 {
                cx.memory.cloud_arc_mm = n;
            } else {
                cx.say(Level::Warn, "Yay boyu sıfırdan büyük olmalı.");
            }
            self.ask_arc = false;
            true
        } else {
            match cx.typed_point(text, self.d.last(), self.d.hover) {
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

    /// Enter: the corners so far make the cloud; with none, the tool leaves.
    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow {
        if self.d.pts.is_empty() {
            return Flow::Exit;
        }
        self.finish(cx);
        Flow::Stay
    }

    fn undo_step(&mut self, cx: &mut Context<'_>) -> bool {
        self.d.undo_step(false, cx)
    }

    fn preview(&self, _format: &Format) -> Preview {
        let pts: Vec<Vec2> = self.d.pts.iter().copied().chain(self.d.hover).collect();
        let tracking = self.d.hover.and(self.d.tracking);
        if pts.len() < 2 {
            return Preview {
                tracking,
                ..Preview::default()
            };
        }
        let input = RevcloudCreate {
            layer_id: String::new(),
            frame: self.frame_of(&pts),
            arc: self.arc_length(),
            color: None,
            line_weight: None,
            line_type: None,
            attrs: None,
            label: None,
            symbol: None,
            expected_revision: None,
            rules: None,
        };
        let stroke = match shapes::revcloud_ring(None, &input) {
            Ok((cloud, bulges)) => Stroke::solid(
                bulge_path_outline(&cloud, bulges.as_deref(), true, DEFAULT_STEP),
                true,
            )
            .width(1.5),
            Err(_) => Stroke::dashed(pts, false, [4.0, 3.0]),
        };
        Preview {
            strokes: vec![stroke],
            tracking,
            ..Preview::default()
        }
    }
}
