//! Yardımcı çizgi and Işın: the web's `XlineTool` and `RayTool` (AutoCAD
//! XLINE and RAY, `apps/web/src/tools/constructionTools.ts`) on their
//! `PointInputTool` base, step for step (docs/adr/0057):
//!
//! - Yardımcı çizgi: a point, then every point a line through it passes
//!   through, one line a click; before the first point Yatay (Y), Düşey (D)
//!   and Açı (A, a typed angle that stays for as long as the app lives) fix
//!   the direction and every click is a line; Açıortay (B) takes an angle's
//!   vertex and a point on each arm; Enter starts over;
//! - Işın: a start point, then one ray towards every point clicked.
//!
//! Every line is written through `cad.entities.create`, its own object and
//! undo step (“Ekle”): “Yardımcı çizgi eklendi.”, “Işın eklendi.” The
//! preview runs across the whole view, dashed, with the line's angle (0 to
//! 180°); the directions are the shared core's (`xline_direction`,
//! `unit_toward`).

use kentos_contracts::EntityGeometry;
use kentos_geometry_core::geometry::angle_deg;
use kentos_geometry_core::tools::drawing::{unit_toward, xline_direction};
use kentos_geometry_core::tools::point_text::{js_trim, parse_number, point_from_text};

use crate::Vec2;
use crate::format::{Format, fixed, js_number};
use crate::log::Level;
use crate::points::{self, Taken, wire};
use crate::prompt::{Prompt, upper_tr};
use crate::tool::{Context, Flow, Pointer, Preview, Stroke, Tag, Tool};

/// The construction line tool's id: its command is `tool.xline`.
pub const XLINE_ID: &str = "xline";
pub const XLINE_LABEL: &str = "Yardımcı çizgi";
/// The ray tool's id: its command is `tool.ray`.
pub const RAY_ID: &str = "ray";
pub const RAY_LABEL: &str = "Işın";

/// How far the preview runs either way, in view diagonals (the web's `strokeInfinite`).
const REACH: f64 = 4.0;

/// The line through `p` along `dir`, as far as the preview draws it; from `p` only for a ray.
fn infinite(p: Vec2, dir: Vec2, reach: f64, ray: bool) -> Stroke {
    let far = |k: f64| Vec2::new(p.x + dir.x * reach * k, p.y + dir.y * reach * k);
    let from = if ray { p } else { far(-1.0) };
    Stroke::dashed(vec![from, far(1.0)], false, [6.0, 4.0])
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    Point,
    Horizontal,
    Vertical,
    Angle,
    Bisect,
}

impl Mode {
    /// The core's name for it (`xline_direction`).
    fn name(self) -> &'static str {
        match self {
            Mode::Point => "point",
            Mode::Horizontal => "horizontal",
            Mode::Vertical => "vertical",
            Mode::Angle => "angle",
            Mode::Bisect => "bisect",
        }
    }

    /// Whether its lines pass through a first point (a vertex).
    fn needs_base(self) -> bool {
        matches!(self, Mode::Point | Mode::Bisect)
    }
}

/// The construction line tool.
#[derive(Clone, Debug)]
pub struct Xline {
    d: Taken,
    mode: Mode,
    ask_angle: bool,
    /// The typed angle, degrees, as the session remembered it at the last call.
    angle: f64,
    reach: f64,
}

impl Default for Xline {
    fn default() -> Self {
        Self {
            d: Taken::default(),
            mode: Mode::Point,
            ask_angle: false,
            angle: 0.0,
            reach: 0.0,
        }
    }
}

impl Xline {
    pub fn new() -> Self {
        Self::default()
    }

    fn see(&mut self, cx: &Context<'_>) {
        self.angle = cx.memory.xline_angle;
        self.reach = points::reach(cx.view, REACH);
    }

    /// Yatay (Y), Düşey (D), Açı (A) and Açıortay (B), before the first point.
    fn option(&mut self, key: &str) -> bool {
        if !self.d.pts.is_empty() {
            return false;
        }
        self.mode = match key {
            "Y" => Mode::Horizontal,
            "D" => Mode::Vertical,
            "A" => Mode::Angle,
            "B" => Mode::Bisect,
            _ => return false,
        };
        self.ask_angle = key == "A";
        true
    }

    /// The direction of the line through `p` in the current mode; `None`
    /// while it needs more points (a bisector is square to opposite arms).
    fn dir_for(&self, p: Vec2) -> Option<Vec2> {
        xline_direction(self.mode.name(), &self.d.pts, p, self.angle)
    }

    /// The point the line for a click at `p` passes through.
    fn base_for(&self, p: Vec2) -> Vec2 {
        match (self.mode.needs_base(), self.d.pts.first()) {
            (true, Some(&base)) => base,
            _ => p,
        }
    }

    fn accept(&mut self, p: Vec2, cx: &mut Context<'_>) {
        self.d.begin(p, cx);
        let n = self.d.pts.len();
        if (self.mode.needs_base() && n == 0) || (self.mode == Mode::Bisect && n == 1) {
            self.d.pts.push(p);
            return;
        }
        let Some(dir) = self.dir_for(p) else { return };
        let geometry = EntityGeometry::Xline {
            p: wire(self.base_for(p)),
            dir: wire(dir),
        };
        if let Some(out) = points::write_objects(vec![geometry], None, cx) {
            if let Some(&id) = out.ids.first() {
                self.d.note(id, cx);
            }
            cx.say(Level::Success, "Yardımcı çizgi eklendi.");
        }
    }

    fn reset(&mut self) {
        self.mode = Mode::Point;
        self.ask_angle = false;
        self.d.reset();
    }
}

impl Tool for Xline {
    fn id(&self) -> &'static str {
        XLINE_ID
    }

    fn label(&self) -> &'static str {
        XLINE_LABEL
    }

    fn prompt(&self) -> Prompt {
        if self.ask_angle {
            return Prompt::new(
                XLINE_LABEL,
                "açıyı yazın (derece, doğudan saat yönünün tersine)",
            );
        }
        let n = self.d.pts.len();
        let step = match self.mode {
            Mode::Horizontal => "geçeceği noktayı belirtin (yatay)".to_owned(),
            Mode::Vertical => "geçeceği noktayı belirtin (düşey)".to_owned(),
            Mode::Angle => {
                // `+XlineTool.angle.toFixed(4)`: four decimals at most, no trailing zeros.
                let shown = fixed(self.angle, 4).parse::<f64>().unwrap_or(self.angle);
                format!("geçeceği noktayı belirtin ({}°)", js_number(shown))
            }
            Mode::Bisect if n == 0 => return Prompt::new(XLINE_LABEL, "açının köşesini belirtin"),
            Mode::Bisect if n == 1 => {
                return Prompt::new(XLINE_LABEL, "açının başlangıç kolunda bir nokta belirtin");
            }
            Mode::Bisect => "açının bitiş kolunda bir nokta belirtin".to_owned(),
            Mode::Point if n == 0 => {
                return Prompt::new(XLINE_LABEL, "bir nokta belirtin")
                    .option("Yatay", "Y")
                    .option("Düşey", "D")
                    .option("Açı", "A")
                    .option("Açıortay", "B");
            }
            Mode::Point => "geçeceği noktayı belirtin".to_owned(),
        };
        Prompt::new(XLINE_LABEL, step).option("Bitir", "Enter")
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
        if self.ask_angle {
            let Some(n) = parse_number(text) else {
                return false;
            };
            cx.memory.xline_angle = n;
            self.ask_angle = false;
            self.see(cx);
            return true;
        }
        let done = if self.option(&upper_tr(js_trim(text))) {
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

    /// Enter: the lines through this point end; with none, the tool leaves.
    fn confirm(&mut self, _cx: &mut Context<'_>) -> Flow {
        if self.d.pts.is_empty() {
            return Flow::Exit;
        }
        self.reset();
        Flow::Stay
    }

    /// Ctrl+Z (ADR 0018): the newest line from this point (as an undo), else
    /// the point, and the mode starts over.
    fn undo_step(&mut self, cx: &mut Context<'_>) -> bool {
        if self.d.undo_last_made(cx) {
            return true;
        }
        if self.d.pts.is_empty() {
            return false;
        }
        self.reset();
        true
    }

    fn preview(&self, _format: &Format) -> Preview {
        let Some(h) = self.d.hover else {
            return Preview::default();
        };
        let mut preview = Preview {
            tracking: self.d.tracking,
            ..Preview::default()
        };
        if self.mode == Mode::Bisect && self.d.pts.len() == 1 {
            preview
                .strokes
                .push(Stroke::dashed(vec![self.d.pts[0], h], false, [3.0, 3.0]));
        }
        let placed = !self.mode.needs_base() || !self.d.pts.is_empty();
        if let (Some(dir), true) = (self.dir_for(h), placed) {
            preview
                .strokes
                .push(infinite(self.base_for(h), dir, self.reach, false));
            // A line has no sense of direction: its angle in 0–180°.
            let a = ((angle_deg(Vec2::new(0.0, 0.0), dir) % 180.0) + 180.0) % 180.0;
            preview.tag = Some(Tag {
                at: h,
                lines: vec![format!("Açı {}°", fixed(a, 2))],
            });
        }
        preview
    }
}

/// The ray tool.
#[derive(Clone, Debug, Default)]
pub struct Ray {
    d: Taken,
    reach: f64,
}

impl Ray {
    pub fn new() -> Self {
        Self::default()
    }

    fn accept(&mut self, p: Vec2, cx: &mut Context<'_>) {
        self.d.begin(p, cx);
        let Some(&start) = self.d.pts.first() else {
            self.d.pts.push(p);
            return;
        };
        let Some(dir) = unit_toward(start, p) else {
            return;
        };
        let geometry = EntityGeometry::Ray {
            p: wire(start),
            dir: wire(dir),
        };
        if let Some(out) = points::write_objects(vec![geometry], None, cx) {
            if let Some(&id) = out.ids.first() {
                self.d.note(id, cx);
            }
            cx.say(Level::Success, "Işın eklendi.");
        }
    }
}

impl Tool for Ray {
    fn id(&self) -> &'static str {
        RAY_ID
    }

    fn label(&self) -> &'static str {
        RAY_LABEL
    }

    fn prompt(&self) -> Prompt {
        if self.d.pts.is_empty() {
            Prompt::new(RAY_LABEL, "başlangıç noktasını belirtin")
        } else {
            Prompt::new(RAY_LABEL, "geçeceği noktayı belirtin").option("Bitir", "Enter")
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
        self.reach = points::reach(cx.view, REACH);
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        let p = self.d.constrain(p, cx);
        self.accept(p, cx);
        self.reach = points::reach(cx.view, REACH);
    }

    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        match point_from_text(text, self.d.last(), self.d.hover, |_| None) {
            Some(p) => {
                self.accept(p, cx);
                true
            }
            None => false,
        }
    }

    /// Enter: the rays from this start end; with none, the tool leaves.
    fn confirm(&mut self, _cx: &mut Context<'_>) -> Flow {
        if self.d.pts.is_empty() {
            return Flow::Exit;
        }
        self.d.reset();
        Flow::Stay
    }

    /// Ctrl+Z (ADR 0018): the newest ray (as an undo), else the start point.
    fn undo_step(&mut self, cx: &mut Context<'_>) -> bool {
        self.d.undo_step(false, cx)
    }

    fn preview(&self, format: &Format) -> Preview {
        let (Some(h), Some(&start)) = (self.d.hover, self.d.pts.first()) else {
            return points::chain_preview(&self.d, format);
        };
        Preview {
            strokes: unit_toward(start, h)
                .map(|dir| infinite(start, dir, self.reach, true))
                .into_iter()
                .collect(),
            tracking: self.d.tracking,
            ..Preview::default()
        }
    }
}
