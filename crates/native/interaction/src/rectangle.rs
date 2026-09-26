//! Dikdörtgen: the web's `RectangleTool` (AutoCAD RECTANG,
//! `apps/web/src/tools/shapeTools.ts`) on its `PointInputTool` base, step for
//! step (docs/adr/0032):
//!
//! - two opposite corners; the corners are free (object snaps apply, ortho
//!   and polar tracking would flatten the box and do not);
//! - before the first corner: Köşe yuvarla (Y) or Pah (P), then the radius
//!   or the distance (0: sharp corners);
//! - before the second: Döndür (D), a typed angle in degrees or a direction
//!   pointed from the first corner; Boyutlar (B), `uzunluk,genişlik`, then a
//!   click shows the side it opens to;
//! - the rotation and the corners stay for the next rectangles, for as long
//!   as the app lives (the web's static fields, [`crate::tool::Memory`]).
//!
//! Every rectangle is written through `cad.polygon.create`, one closed area
//! and one undo step, rounded or cut corners as arcs or extra corners, and
//! says “Dikdörtgen eklendi: 20.000 × 10.000 m”. The rings and their corners
//! are the shared core's (`rectFromCorners`, `rectFromSize`, `cornersOfRing`);
//! none is computed here.

use kentos_geometry_core::geom::shapes::{rect_from_corners, rect_from_size};
use kentos_geometry_core::geometry::{dist, signed_area};
use kentos_geometry_core::jsmath::PI;
use kentos_geometry_core::ops::fillet::{CornerOp, CornerResult, corners_of_ring};
use kentos_geometry_core::tools::drawing::direction_angle;
use kentos_geometry_core::tools::point_text::{
    is_js_space, js_trim, parse_number, point_from_text,
};

use crate::Vec2;
use crate::format::{Format, short_degrees};
use crate::log::Level;
use crate::points::{self, Taken};
use crate::prompt::{Prompt, upper_tr};
use crate::tool::{Context, Corners, Flow, Memory, Pointer, Preview, Stroke, Tag, Tool};

/// The rectangle tool's id: its command is `tool.rectangle`.
pub const ID: &str = "rectangle";
pub const LABEL: &str = "Dikdörtgen";
const DEG: f64 = PI / 180.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Stage {
    First,
    Second,
    CornerSize,
    Rotation,
    Size,
    Side,
}

/// The corner style being asked for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Pending {
    Fillet,
    Chamfer,
}

/// The rectangle tool.
#[derive(Clone, Debug)]
pub struct Rectangle {
    d: Taken,
    stage: Stage,
    pending: Pending,
    /// Boyutlar: length along the rotation, and width.
    size: Option<(f64, f64)>,
    /// What the session remembered and the project's units, as of the last call.
    seen: Option<(Memory, Format)>,
}

impl Default for Rectangle {
    fn default() -> Self {
        Self {
            d: Taken::default(),
            stage: Stage::First,
            pending: Pending::Fillet,
            size: None,
            seen: None,
        }
    }
}

impl Rectangle {
    pub fn new() -> Self {
        Self::default()
    }

    fn see(&mut self, cx: &Context<'_>) {
        self.seen = Some((*cx.memory, cx.format()));
    }

    fn rotation(&self) -> f64 {
        self.seen.map_or(0.0, |(m, _)| m.rect_rotation)
    }

    /// The ring from corner `a` to `p`: by its size in the Side stage, else by opposite corners.
    fn ring(&self, a: Vec2, p: Vec2) -> Option<Vec<Vec2>> {
        match (self.stage, self.size) {
            (Stage::Side, Some((length, width))) => {
                rect_from_size(a, length, width, self.rotation(), p)
            }
            _ => rect_from_corners(a, p, self.rotation()),
        }
    }

    fn option(&mut self, key: &str) -> bool {
        match (self.stage, key) {
            (Stage::First, "Y" | "P") => {
                self.pending = if key == "Y" {
                    Pending::Fillet
                } else {
                    Pending::Chamfer
                };
                self.stage = Stage::CornerSize;
            }
            (Stage::Second, "D") => self.stage = Stage::Rotation,
            (Stage::Second, "B") => self.stage = Stage::Size,
            _ => return false,
        }
        true
    }

    fn accept(&mut self, p: Vec2, cx: &mut Context<'_>) {
        self.d.begin(p, cx);
        let a = match self.d.last() {
            Some(a) if self.stage != Stage::First => a,
            _ => {
                self.d.pts = vec![p];
                self.stage = Stage::Second;
                return;
            }
        };
        if self.stage == Stage::Rotation {
            if dist(a, p) > 1e-9 {
                cx.memory.rect_rotation = direction_angle(a, p);
            }
            self.stage = Stage::Second;
            return;
        }
        match self.ring(a, p) {
            Some(ring) => self.commit(&ring, cx),
            None => cx.say(
                Level::Warn,
                "Dikdörtgenin kenarları sıfır olamaz; başka bir köşe gösterin.",
            ),
        }
    }

    /// The ring with the corner style applied to every corner by the core,
    /// or plain with a warning when it cannot be (the web's `styled`).
    fn styled(ring: &[Vec2], cx: &mut Context<'_>) -> (Vec<Vec2>, Option<Vec<f64>>) {
        let op = match cx.memory.rect_corners {
            Corners::Sharp => return (ring.to_vec(), None),
            Corners::Fillet(radius) => CornerOp::Radius(radius),
            Corners::Chamfer(d) => CornerOp::Chamfer(d, d),
        };
        match corners_of_ring(ring, &op) {
            Ok(CornerResult::Path(path)) => (path.pts, path.bulges),
            Ok(CornerResult::Error(error)) | Err(error) => {
                cx.say(Level::Warn, format!("Köşeler işlenmedi: {error}"));
                (ring.to_vec(), None)
            }
        }
    }

    fn commit(&mut self, ring: &[Vec2], cx: &mut Context<'_>) {
        let (pts, bulges) = Self::styled(ring, cx);
        let w = dist(ring[0], ring[1]);
        let h = dist(ring[1], ring[2]);
        if points::write_ring(&mut self.d, &pts, bulges, cx) {
            let format = cx.format();
            let line = format!(
                "Dikdörtgen eklendi: {} × {}",
                format.length_bare(w),
                format.length(h)
            );
            cx.say(Level::Success, line);
        }
        self.size = None;
        self.stage = Stage::First;
        self.d.pts.clear();
    }

    fn reset(&mut self) {
        self.stage = Stage::First;
        self.size = None;
        self.d.reset();
    }
}

/// `uzunluk,genişlik`: two unsigned decimals with a comma, a semicolon or a
/// space between (the web's `/^(\d+(?:\.\d+)?)\s*[,; ]\s*(\d+(?:\.\d+)?)$/`).
fn size_text(text: &str) -> Option<(f64, f64)> {
    let t = js_trim(text);
    let (length, rest) = unsigned(t)?;
    let space = rest.len() - rest.trim_start_matches(is_js_space).len();
    let (gap, after) = rest.split_at(space);
    let after = match after.strip_prefix([',', ';']) {
        Some(after) => after.trim_start_matches(is_js_space),
        // A space is a separator of its own; other white space is not.
        None if gap.contains(' ') => after,
        None => return None,
    };
    let (width, end) = unsigned(after)?;
    end.is_empty().then_some((length, width))
}

/// `\d+(?:\.\d+)?` at the start of `s`: the number and what follows.
fn unsigned(s: &str) -> Option<(f64, &str)> {
    let digits = |from: &str| from.bytes().take_while(u8::is_ascii_digit).count();
    let whole = digits(s);
    if whole == 0 {
        return None;
    }
    let mut end = whole;
    if s[end..].starts_with('.') {
        let fraction = digits(&s[end + 1..]);
        if fraction > 0 {
            end += 1 + fraction;
        }
    }
    Some((s[..end].parse().ok()?, &s[end..]))
}

impl Tool for Rectangle {
    fn id(&self) -> &'static str {
        ID
    }

    fn label(&self) -> &'static str {
        LABEL
    }

    fn prompt(&self) -> Prompt {
        let (memory, format) = self.seen.unwrap_or_default();
        match self.stage {
            Stage::CornerSize if self.pending == Pending::Fillet => {
                Prompt::new(LABEL, "köşe yarıçapını yazın (0: keskin köşe)")
            }
            Stage::CornerSize => Prompt::new(LABEL, "pah mesafesini yazın (0: keskin köşe)"),
            Stage::Rotation => Prompt::new(
                LABEL,
                "dönme açısını yazın (derece) ya da ilk köşeden bir yön gösterin",
            ),
            Stage::Size => Prompt::new(LABEL, "uzunluk ve genişliği yazın, ör. 20,10"),
            Stage::Side => Prompt::new(LABEL, "dikdörtgenin hangi yana açılacağını gösterin"),
            Stage::Second => Prompt::new(LABEL, "karşı köşeyi belirtin")
                .option_with("Döndür", "D", short_degrees(memory.rect_rotation))
                .option("Boyutlar", "B"),
            Stage::First => {
                let (fillet, chamfer) = match memory.rect_corners {
                    Corners::Fillet(size) => (format.length(size), "kapalı".to_owned()),
                    Corners::Chamfer(size) => ("kapalı".to_owned(), format.length(size)),
                    Corners::Sharp => ("kapalı".to_owned(), "kapalı".to_owned()),
                };
                Prompt::new(LABEL, "ilk köşeyi belirtin")
                    .option_with("Köşe yuvarla", "Y", fillet)
                    .option_with("Pah", "P", chamfer)
            }
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

    /// The corners are free: ortho and polar tracking would flatten the box.
    fn pointer_move(&mut self, p: &Pointer, _cx: &mut Context<'_>) {
        self.d.hover = Some(p.world);
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.accept(p.world, cx);
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
            self.reset();
            Flow::Stay
        };
        self.see(cx);
        flow
    }

    /// Ctrl+Z (ADR 0018): the rectangle just written (as an undo), else the
    /// rectangle being given starts over.
    fn undo_step(&mut self, cx: &mut Context<'_>) -> bool {
        let done = if self.d.undo_last_made(cx) {
            true
        } else if self.d.pts.is_empty() {
            false
        } else {
            self.reset();
            true
        };
        self.see(cx);
        done
    }

    fn preview(&self, format: &Format) -> Preview {
        let (Some(a), Some(h)) = (self.d.last(), self.d.hover) else {
            return Preview::default();
        };
        if self.stage == Stage::Rotation {
            return Preview {
                strokes: vec![Stroke::dashed(vec![a, h], false, [3.0, 3.0])],
                tag: Some(Tag {
                    at: h,
                    lines: vec![format!("Açı {}", short_degrees(direction_angle(a, h)))],
                }),
                ..Preview::default()
            };
        }
        if !matches!(self.stage, Stage::Second | Stage::Side) {
            return Preview::default();
        }
        let Some(ring) = self.ring(a, h) else {
            return Preview::default();
        };
        let lines = vec![
            format!(
                "{} × {}",
                format.length_bare(dist(ring[0], ring[1])),
                format.length(dist(ring[1], ring[2]))
            ),
            format!("Alan {}", format.area(signed_area(&ring).abs())),
        ];
        Preview {
            strokes: vec![Stroke::solid(ring, true)],
            tag: Some(Tag { at: h, lines }),
            ..Preview::default()
        }
    }
}

impl Rectangle {
    /// Typed text by stage (the web's `input`): an option, the corner size,
    /// the rotation, the size, or a point.
    fn typed(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        if self.option(&upper_tr(js_trim(text))) {
            return true;
        }
        let n = parse_number(text);
        match self.stage {
            Stage::CornerSize => {
                let Some(n) = n.filter(|n| *n >= 0.0) else {
                    return false;
                };
                cx.memory.rect_corners = match (n > 0.0, self.pending) {
                    (false, _) => Corners::Sharp,
                    (true, Pending::Fillet) => Corners::Fillet(n),
                    (true, Pending::Chamfer) => Corners::Chamfer(n),
                };
                self.stage = Stage::First;
                true
            }
            Stage::Rotation => {
                let Some(n) = n else {
                    return false;
                };
                cx.memory.rect_rotation = n * DEG;
                self.stage = Stage::Second;
                true
            }
            Stage::Size => match size_text(text) {
                Some((length, width)) if length > 0.0 && width > 0.0 => {
                    self.size = Some((length, width));
                    self.stage = Stage::Side;
                    true
                }
                _ => false,
            },
            _ => match point_from_text(text, self.d.last(), self.d.hover, |_| None) {
                Some(p) => {
                    self.accept(p, cx);
                    true
                }
                None => false,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes_are_read_as_the_web_reads_them() {
        assert_eq!(size_text("20,10"), Some((20.0, 10.0)));
        assert_eq!(size_text(" 20.5 ; 7 "), Some((20.5, 7.0)));
        assert_eq!(size_text("20 10"), Some((20.0, 10.0)));
        assert_eq!(size_text("20  ,  10"), Some((20.0, 10.0)));
        assert_eq!(
            size_text("20\t10"),
            None,
            "a tab is white space, not a separator"
        );
        assert_eq!(size_text("-20,10"), None);
        assert_eq!(size_text("20,"), None);
        assert_eq!(size_text("20.,10"), None);
        assert_eq!(size_text("20,10x"), None);
    }
}
