//! NURBS eğrisi (docs/adr/0259 §6): a NURBS curve by the points it passes
//! through (Geçiş noktaları, AutoCAD's SPLINE by fit points) or by its
//! control points (Denetim noktaları, SPLINE by control vertices), open or
//! closed, of the degree asked.
//!
//! - the points, clicked or typed; Geri (G) takes the last one back, as
//!   Ctrl+Z does;
//! - Yöntem (Y) turns between fit and control points; Derece (D) takes the
//!   degree typed next (1 to 11); both are kept for the session;
//! - Enter (or a quick right click) ends an open curve; Kapat (K) a closed
//!   one; with too few points the command says why and the points stay;
//! - the preview is `cad.nurbs.create`'s plan for the points and the
//!   cursor: the curve solid; dashed, its control polygon (by control
//!   points) or the chain of the points given (through points).
//!
//! The curve is computed and written by the command (`cad.nurbs.create`,
//! its own object and undo step “Ekle”): the tool, Python and an agent give
//! the same points and get the same curve.

use kentos_contracts::{CommandResult, Entity, NurbsBy, NurbsCreate};
use kentos_geometry_core::geometry::dist;
use kentos_geometry_core::tools::point_text::js_trim;
use kentos_native_application::geometry::shape;
use kentos_native_application::{ExecutionContext, nurbs};

use crate::Vec2;
use crate::format::Format;
use crate::log::Level;
use crate::points::{self, Taken, plain_number, wire_all};
use crate::prompt::{Prompt, upper_tr};
use crate::tool::{Context, Flow, Pointer, Preview, Stroke, Tool};

/// The tool's id: its command is `tool.nurbs`.
pub const ID: &str = "nurbs";
pub const LABEL: &str = "NURBS eğrisi";

/// The NURBS curve tool.
#[derive(Clone, Debug, Default)]
pub struct NurbsTool {
    d: Taken,
    /// The next number typed is the degree (Derece pressed).
    ask_degree: bool,
    /// The plan's curve and control polygon for the points and the cursor.
    curve: Vec<Vec2>,
    polygon: Vec<Vec2>,
    /// The session's method and degree as of the last call (`prompt` sees no session).
    seen: Option<(bool, u32)>,
}

impl NurbsTool {
    pub fn new() -> Self {
        Self::default()
    }

    /// The command's input for `pts`: what the tool was given, the active
    /// layer, the current colour and weight, an object template's data.
    fn request(pts: &[Vec2], closed: bool, cx: &Context<'_>) -> NurbsCreate {
        let m = &*cx.memory;
        let degree = m.nurbs_degree;
        let by = if m.nurbs_control {
            NurbsBy::Control {
                // An open curve of fewer control points takes the highest degree they allow.
                degree: if closed {
                    degree
                } else {
                    degree.min(pts.len().saturating_sub(1).max(1) as u32)
                },
                ctrl: wire_all(pts),
                weights: None,
                knots: None,
            }
        } else {
            NurbsBy::Fit {
                pts: wire_all(pts),
                degree: Some(degree),
            }
        };
        NurbsCreate {
            line_type: cx.draft.line_type,
            layer_id: cx.doc.layers().active().to_owned(),
            by,
            closed,
            color: cx.draft.color_text(),
            line_weight: cx.draft.line_weight,
            attrs: cx.template_attrs(None),
            label: cx.template_label(),
            symbol: cx.template_symbol(),
            expected_revision: None,
        }
    }

    /// The plan for the points and the cursor, drawn: the curve and its control polygon.
    fn redraw(&mut self, cx: &Context<'_>) {
        self.seen = Some((cx.memory.nurbs_control, cx.memory.nurbs_degree));
        self.curve.clear();
        self.polygon.clear();
        let pts: Vec<Vec2> = self.d.pts.iter().copied().chain(self.d.hover).collect();
        if pts.len() < 2 {
            return;
        }
        if let CommandResult::Completed { output, .. } =
            nurbs::plan_in(cx.doc, &Self::request(&pts, false, cx))
            && let Entity::Nurbs(n) = &output.entity
        {
            self.curve = kentos_geometry_core::geom::nurbs::outline(&shape(&output.entity));
            // By control points their polygon is what is drawn; through points the points themselves.
            if cx.memory.nurbs_control {
                self.polygon = n.ctrl.iter().map(|p| Vec2::new(p.x, p.y)).collect();
            }
        }
    }

    fn accept(&mut self, p: Vec2, cx: &mut Context<'_>) {
        self.d.begin(p, cx);
        if self.d.last().is_none_or(|last| dist(last, p) > 1e-9) {
            self.d.pts.push(p);
        }
        self.redraw(cx);
    }

    /// Geri (G), Yöntem (Y), Derece (D), Kapat (K).
    fn option(&mut self, key: &str, cx: &mut Context<'_>) -> bool {
        match key {
            "G" if !self.d.pts.is_empty() => {
                self.d.pts.pop();
                self.redraw(cx);
                true
            }
            "Y" => {
                cx.memory.nurbs_control = !cx.memory.nurbs_control;
                self.redraw(cx);
                true
            }
            "D" => {
                self.ask_degree = true;
                self.redraw(cx);
                true
            }
            "K" if self.d.pts.len() >= 3 => {
                self.commit(true, cx);
                true
            }
            _ => false,
        }
    }

    /// Writes through `cad.nurbs.create`: its own object and undo step
    /// (“Ekle”), then its length from the core. A refusal (too few points
    /// for the degree) is said and the points stay.
    fn commit(&mut self, closed: bool, cx: &mut Context<'_>) {
        let input = Self::request(&self.d.pts, closed, cx);
        let n = self.d.pts.len();
        let result = nurbs::execute(&mut ExecutionContext::new(cx.doc), input);
        let Some(made) = points::written(result, cx) else {
            return;
        };
        self.d.note(made.id, cx);
        let length = cx
            .doc
            .get(kentos_domain::Slot(made.id))
            .and_then(|e| kentos_geometry_core::entity::entity_length(&shape(e)));
        let what = if closed {
            "Kapalı NURBS eğrisi"
        } else {
            "NURBS eğrisi"
        };
        let how = if cx.memory.nurbs_control {
            "denetim noktası"
        } else {
            "geçiş noktası"
        };
        if let Some(length) = length {
            cx.say(
                Level::Success,
                format!("{what} eklendi: {n} {how}, {}", cx.format().length(length)),
            );
        }
        self.d.reset();
        self.curve.clear();
        self.polygon.clear();
    }
}

impl Tool for NurbsTool {
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
        let (control, degree) = self.seen.unwrap_or((false, 3));
        let method = if control {
            "Denetim noktaları"
        } else {
            "Geçiş noktaları"
        };
        if self.ask_degree {
            return Prompt::new(LABEL, "dereceyi yazın (1 ile 11 arası)");
        }
        let what = if control {
            "denetim noktası"
        } else {
            "geçiş noktası"
        };
        let first = Prompt::new(
            LABEL,
            if self.d.pts.is_empty() {
                format!("ilk {what}nı belirtin")
            } else {
                format!("sonraki {what}nı belirtin")
            },
        )
        .option_with("Yöntem", "Y", method)
        .option_with("Derece", "D", degree.to_string());
        if self.d.pts.len() >= 2 {
            first
                .option("Kapat", "K")
                .option("Geri", "G")
                .option("Bitir", "Enter")
        } else {
            first
        }
    }

    fn point_count(&self) -> usize {
        self.d.pts.len()
    }

    fn snap_from(&self) -> Option<Vec2> {
        self.d.last()
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.d.hover = Some(self.d.constrain(p, cx));
        self.redraw(cx);
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        let p = self.d.constrain(p, cx);
        self.accept(p, cx);
    }

    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        if self.ask_degree
            && let Some(n) = plain_number(text)
        {
            if n.fract() != 0.0 || !(1.0..=11.0).contains(&n) {
                cx.say(Level::Warn, "Derece 1 ile 11 arasında bir tam sayı olmalı.");
                return true;
            }
            cx.memory.nurbs_degree = n as u32;
            self.ask_degree = false;
            self.redraw(cx);
            return true;
        }
        if self.option(&upper_tr(js_trim(text)), cx) {
            return true;
        }
        match cx.typed_point(text, self.d.last(), self.d.hover) {
            Some(p) => {
                self.accept(p, cx);
                true
            }
            None => false,
        }
    }

    /// Enter: an open curve of two points or more is written; fewer start
    /// over; with none, the tool leaves.
    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow {
        if self.ask_degree {
            self.ask_degree = false;
            return Flow::Stay;
        }
        if self.d.pts.is_empty() {
            return Flow::Exit;
        }
        if self.d.pts.len() >= 2 {
            self.commit(false, cx);
        } else {
            self.d.reset();
            self.curve.clear();
            self.polygon.clear();
        }
        Flow::Stay
    }

    /// Ctrl+Z: Geri first, the last point; then the curve just written, as an undo.
    fn undo_step(&mut self, cx: &mut Context<'_>) -> bool {
        if self.option("G", cx) {
            return true;
        }
        self.d.undo_last_made(cx)
    }

    fn preview(&self, _format: &Format) -> Preview {
        let mut strokes = Vec::new();
        if !self.curve.is_empty() {
            strokes.push(Stroke::solid(self.curve.clone(), false));
        }
        let chain: Vec<Vec2> = if self.polygon.is_empty() {
            self.d.pts.iter().copied().chain(self.d.hover).collect()
        } else {
            self.polygon.clone()
        };
        strokes.push(Stroke::dashed(chain, false, [2.0, 4.0]));
        Preview {
            strokes,
            marks: self.d.pts.clone(),
            tracking: self.d.hover.and(self.d.tracking),
            ..Preview::default()
        }
    }
}
