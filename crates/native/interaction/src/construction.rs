//! Yardımcı çizgi and Işın: the web's `XlineTool` and `RayTool` (AutoCAD
//! XLINE and RAY, `apps/web/src/tools/constructionTools.ts`) on their
//! `PointInputTool` base, step for step (docs/adr/0057):
//!
//! - Yardımcı çizgi: a point, then every point a line through it passes
//!   through, one line a click; before the first point Yatay (Y), Düşey (D)
//!   and Açı (A, a typed angle that stays for as long as the app lives) fix
//!   the direction and every click is a line; Açıortay (B) takes an angle's
//!   vertex and a point on each arm; Paralel (P, AutoCAD's Offset, docs/adr/0287)
//!   takes a distance typed (or Noktadan, N: through a point), then a straight
//!   edge picked and the side clicked, one line a pick, the distance kept
//!   (`xline_offset`); Enter starts over;
//! - Işın: a start point, then one ray towards every point clicked.
//!
//! Every line is written through `cad.entities.create`, its own object and
//! undo step (“Ekle”): “Yardımcı çizgi eklendi.”, “Işın eklendi.” The
//! preview runs across the whole view, dashed, with the line's angle (0 to
//! 180°); the directions are the shared core's (`xline_direction`,
//! `unit_toward`).

use kentos_commands::EntityGeometry;
use kentos_geometry_core::geom::intersect::Edge;
use kentos_geometry_core::geometry::angle_deg;
use kentos_geometry_core::tools::drawing::{unit_toward, xline_direction, xline_offset};
use kentos_geometry_core::tools::point_text::{js_trim, parse_number};

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
    /// Paralel (docs/adr/0287): parallel to a straight edge.
    Offset,
}

/// Paralel's step.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Offset {
    /// The distance (or Noktadan) is asked.
    Distance,
    /// A straight edge is picked: `None` through a point, else the distance.
    Edge(Option<f64>),
    /// The side (or the point) is shown for the edge picked.
    Side(Option<f64>, Vec2, Vec2),
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
            Mode::Offset => "offset",
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
    offset: Offset,
}

impl Default for Xline {
    fn default() -> Self {
        Self {
            d: Taken::default(),
            mode: Mode::Point,
            ask_angle: false,
            angle: 0.0,
            reach: 0.0,
            offset: Offset::Distance,
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
            "P" => Mode::Offset,
            // Noktadan: through a point, while Paralel asks its distance.
            "N" if self.mode == Mode::Offset && self.offset == Offset::Distance => {
                self.offset = Offset::Edge(None);
                return true;
            }
            _ => return false,
        };
        self.ask_angle = key == "A";
        self.offset = Offset::Distance;
        true
    }

    /// Paralel: a press picks the straight edge, then shows the side (or the point).
    fn offset_press(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        match self.offset {
            Offset::Distance => cx.say(
                Level::Warn,
                "Önce paralel uzaklığı yazın ya da Noktadan (N) seçin.",
            ),
            Offset::Edge(distance) => match crate::locks::picked_edge(p.raw, cx) {
                Some((Edge::Seg { a, b }, _)) => self.offset = Offset::Side(distance, a, b),
                _ => cx.say(
                    Level::Warn,
                    "Düz bir kenara tıklayın: çizgi, çoklu çizgi ya da alan kenarı.",
                ),
            },
            Offset::Side(distance, a, b) => {
                let side = self.d.constrain(p, cx);
                self.write_offset(distance, a, b, side, cx);
            }
        }
    }

    /// The parallel line for the side (or the point) shown, through `cad.entities.create`; the next edge is asked.
    fn write_offset(
        &mut self,
        distance: Option<f64>,
        a: Vec2,
        b: Vec2,
        side: Vec2,
        cx: &mut Context<'_>,
    ) {
        let Some((p, dir)) = xline_offset(a, b, distance, side) else {
            cx.say(
                Level::Warn,
                "Kenarın bir yanını gösterin: nokta kenarın doğrusunun üstünde.",
            );
            return;
        };
        let geometry = EntityGeometry::Xline {
            p: wire(p),
            dir: wire(dir),
        };
        if let Some(out) = points::write_objects(vec![geometry], None, cx) {
            if let Some(&id) = out.ids.first() {
                self.d.note(id, cx);
            }
            cx.say(Level::Success, "Yardımcı çizgi eklendi.");
        }
        self.offset = Offset::Edge(distance);
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
        self.offset = Offset::Distance;
        self.d.reset();
    }
}

impl Tool for Xline {
    /// A point computed by the point calculator, as if clicked (the web's `acceptPoint`).
    fn accepts_points(&self) -> bool {
        true
    }

    fn accept_point(&mut self, p: Vec2, cx: &mut Context<'_>) -> bool {
        self.accept(p, cx);
        true
    }
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
        let step = match (self.mode, self.offset) {
            (Mode::Offset, Offset::Distance) => {
                return Prompt::new(XLINE_LABEL, "paralel uzaklığı yazın").option("Noktadan", "N");
            }
            (Mode::Offset, Offset::Edge(_)) => {
                return Prompt::new(XLINE_LABEL, "paralel olacağı düz kenara tıklayın")
                    .option("Bitir", "Enter");
            }
            (Mode::Offset, Offset::Side(None, _, _)) => {
                return Prompt::new(XLINE_LABEL, "geçeceği noktayı belirtin");
            }
            (Mode::Offset, Offset::Side(Some(_), _, _)) => {
                return Prompt::new(XLINE_LABEL, "hangi yana konacağını gösterin");
            }
            _ => self.mode,
        };
        let step = match step {
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
                    .option("Açıortay", "B")
                    .option("Paralel", "P");
            }
            Mode::Point => "geçeceği noktayı belirtin".to_owned(),
            Mode::Offset => String::new(),
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
        if self.mode == Mode::Offset {
            self.offset_press(p, cx);
            self.see(cx);
            return;
        }
        let p = self.d.constrain(p, cx);
        self.accept(p, cx);
        self.see(cx);
    }

    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        // Paralel's distance, in the project's unit or with its own (docs/adr/0266).
        if self.mode == Mode::Offset
            && self.offset == Offset::Distance
            && let Some((_, d)) = crate::points::plain_value(text, cx).filter(|(_, d)| *d > 0.0)
        {
            self.offset = Offset::Edge(Some(d));
            return true;
        }
        // The point Paralel's line passes through, or its side, typed.
        if let (Mode::Offset, Offset::Side(distance, a, b)) = (self.mode, self.offset)
            && let Some(p) = cx.typed_point(text, None, self.d.hover)
        {
            self.write_offset(distance, a, b, p, cx);
            return true;
        }
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

    /// Enter: the lines through this point end; with none, the tool leaves.
    fn confirm(&mut self, _cx: &mut Context<'_>) -> Flow {
        if self.mode == Mode::Offset {
            self.reset();
            return Flow::Stay;
        }
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
        if let (Mode::Offset, Offset::Side(distance, a, b)) = (self.mode, self.offset) {
            preview
                .strokes
                .push(Stroke::dashed(vec![a, b], false, [2.0, 2.0]));
            if let Some((p, dir)) = xline_offset(a, b, distance, h) {
                preview.strokes.push(infinite(p, dir, self.reach, false));
            }
            return preview;
        }
        if self.mode == Mode::Offset {
            return preview;
        }
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
    /// A point computed by the point calculator, as if clicked (the web's `acceptPoint`).
    fn accepts_points(&self) -> bool {
        true
    }

    fn accept_point(&mut self, p: Vec2, cx: &mut Context<'_>) -> bool {
        self.accept(p, cx);
        true
    }
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
        match cx.typed_point(text, self.d.last(), self.d.hover) {
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
