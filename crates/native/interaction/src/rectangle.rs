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
//!   click shows the side it opens to; Alan (A, AutoCAD's Area), the area
//!   in the project's area unit, then one side's length: the other side is
//!   the area over it (`rect_side_for_area`), then a click shows the side;
//! - the ribbon's methods (docs/adr/0287): `BOYUT` and `ALAN` before the first
//!   corner go to Boyutlar or Alan after every first corner while the tool runs;
//! - the rotation and the corners stay for the next rectangles, for as long
//!   as the app lives (the web's static fields, [`crate::tool::Memory`]).
//!
//! Every rectangle is written through `cad.rectangle.create` (docs/adr/0290),
//! one closed area and one undo step, rounded or cut corners as arcs or extra
//! corners, and says “Dikdörtgen eklendi: 20.000 × 10.000 m”. The tool gives
//! the command what was given (the corners, the size, the rotation, the
//! corners' style); the ring, for the preview too, is the command's
//! (`shapes::rectangle_ring`). Corners that do not fit are said and the
//! rectangle is written sharp. With the overlap control or Topoloji changing
//! the area, it is written as any new area is (`points::write_area_by`).

use kentos_contracts::{RectangleCorners, RectangleCreate, RectangleFrame};
use kentos_geometry_core::geom::shapes::rect_side_for_area;
use kentos_geometry_core::geometry::{dist, signed_area};
use kentos_geometry_core::jsmath::PI;
use kentos_geometry_core::tools::drawing::direction_angle;
use kentos_geometry_core::tools::point_text::{is_js_space, js_trim, parse_number};
use kentos_native_application::{ExecutionContext, shapes};

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

#[derive(Clone, Copy, Debug, PartialEq)]
enum Stage {
    First,
    Second,
    CornerSize,
    Rotation,
    Size,
    Side,
    /// Alan: the area typed.
    Area,
    /// Alan: one side's length typed, the area kept.
    AreaSide(f64),
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
    /// The ribbon's method (docs/adr/0287): the stage after every first corner.
    method: Option<Stage>,
}

impl Default for Rectangle {
    fn default() -> Self {
        Self {
            d: Taken::default(),
            stage: Stage::First,
            pending: Pending::Fillet,
            size: None,
            seen: None,
            method: None,
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

    /// What the command is given from corner `a` to `p`: by its size in the Side stage, else by opposite
    /// corners; sharp (the corners are the caller's to add).
    fn frame(&self, a: Vec2, p: Vec2) -> RectangleFrame {
        match (self.stage, self.size) {
            (Stage::Side, Some((length, width))) => RectangleFrame::Size {
                corner: points::wire(a),
                length,
                width,
                toward: points::wire(p),
            },
            _ => RectangleFrame::Corners {
                a: points::wire(a),
                b: points::wire(p),
            },
        }
    }

    /// The command's input from corner `a` to `p`, its corners as given.
    fn request(
        &self,
        a: Vec2,
        p: Vec2,
        corners: Option<RectangleCorners>,
        cx: &Context<'_>,
    ) -> RectangleCreate {
        let rotation = self.rotation();
        RectangleCreate {
            layer_id: cx.doc.layers().active().to_owned(),
            frame: self.frame(a, p),
            rotation: (rotation != 0.0).then_some(rotation),
            corners,
            color: cx.draft.color_text(),
            line_weight: cx.draft.line_weight,
            line_type: cx.draft.line_type,
            attrs: cx.template_attrs(None),
            label: cx.template_label(),
            symbol: cx.template_symbol(),
            expected_revision: None,
        }
    }

    /// The ring from corner `a` to `p` as the command makes it, sharp: the preview.
    fn ring(&self, a: Vec2, p: Vec2) -> Option<Vec<Vec2>> {
        let input = RectangleCreate {
            layer_id: String::new(),
            frame: self.frame(a, p),
            rotation: Some(self.rotation()),
            corners: None,
            color: None,
            line_weight: None,
            line_type: None,
            attrs: None,
            label: None,
            symbol: None,
            expected_revision: None,
        };
        shapes::rectangle_ring(&input).ok().map(|(pts, _)| pts)
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
            (Stage::Second, "A") => self.stage = Stage::Area,
            (Stage::First, "BOYUT") => self.method = Some(Stage::Size),
            (Stage::First, "ALAN") => self.method = Some(Stage::Area),
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
                self.stage = self.method.unwrap_or(Stage::Second);
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
            Some(ring) => self.commit(a, p, &ring, cx),
            None => cx.say(
                Level::Warn,
                "Dikdörtgenin kenarları sıfır olamaz; başka bir köşe gösterin.",
            ),
        }
    }

    /// Writes the rectangle from `a` to `p` through `cad.rectangle.create`, its corners as remembered;
    /// corners that do not fit are said and it is written sharp (the web's `styled`).
    fn commit(&mut self, a: Vec2, p: Vec2, ring: &[Vec2], cx: &mut Context<'_>) {
        let corners = match cx.memory.rect_corners {
            Corners::Sharp => None,
            Corners::Fillet(radius) => Some(RectangleCorners::Fillet { radius }),
            Corners::Chamfer(distance) => Some(RectangleCorners::Chamfer { distance }),
        };
        let mut input = self.request(a, p, corners, cx);
        let (pts, bulges) = match shapes::rectangle_ring(&input) {
            Ok(ring) => ring,
            Err(refusal) => {
                cx.say(
                    Level::Warn,
                    format!("Köşeler işlenmedi: {}", refusal.message),
                );
                input.corners = None;
                (ring.to_vec(), None)
            }
        };
        let w = dist(ring[0], ring[1]);
        let h = dist(ring[1], ring[2]);
        let mut write = |cx: &mut Context<'_>| {
            let result =
                shapes::execute_rectangle(&mut ExecutionContext::new(cx.doc), input.clone());
            points::written(result, cx).map(|out| out.id)
        };
        if points::write_area_by(&mut self.d, &pts, bulges, cx, Some(&mut write)).is_some() {
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
            Stage::Area => {
                Prompt::new(LABEL, format!("alanı yazın ({})", format.area_unit_label()))
            }
            Stage::AreaSide(_) => Prompt::new(
                LABEL,
                "bir kenarın uzunluğunu yazın; öbür kenar alandan bulunur",
            ),
            Stage::Side => Prompt::new(LABEL, "dikdörtgenin hangi yana açılacağını gösterin"),
            Stage::Second => Prompt::new(LABEL, "karşı köşeyi belirtin")
                .option_with("Döndür", "D", short_degrees(memory.rect_rotation))
                .option("Boyutlar", "B")
                .option("Alan", "A"),
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
                let Some((n, size)) = crate::points::plain_value(text, cx)
                    .or_else(|| n.map(|n| (n, cx.format().to_metres(n))))
                    .filter(|(n, _)| *n >= 0.0)
                else {
                    return false;
                };
                // Typed in the project's unit (docs/adr/0165 §2) or with its own (docs/adr/0266).
                cx.memory.rect_corners = match (n > 0.0, self.pending) {
                    (false, _) => Corners::Sharp,
                    (true, Pending::Fillet) => Corners::Fillet(size),
                    (true, Pending::Chamfer) => Corners::Chamfer(size),
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
            Stage::Area => {
                // In the project's area unit (m², dönüm, hektar; docs/adr/0191's rule).
                let f = cx.format();
                match n.filter(|n| *n > 0.0) {
                    Some(n) => {
                        self.stage = Stage::AreaSide(f.area_to_square_metres(n));
                        true
                    }
                    None => false,
                }
            }
            Stage::AreaSide(area) => {
                let Some((_, side)) = crate::points::plain_value(text, cx)
                    .or_else(|| n.map(|n| (n, cx.format().to_metres(n))))
                else {
                    return false;
                };
                match rect_side_for_area(area, side) {
                    Some(width) => {
                        self.size = Some((side, width));
                        self.stage = Stage::Side;
                        true
                    }
                    None => {
                        cx.say(Level::Warn, "Kenar uzunluğu sıfırdan büyük olmalı.");
                        true
                    }
                }
            }
            Stage::Size => {
                // Typed in the project's unit (docs/adr/0165 §2), or with their own (docs/adr/0266).
                let f = cx.format();
                let size = size_text(text)
                    .map(|(l, w)| (f.to_metres(l), f.to_metres(w)))
                    .or_else(|| {
                        let (l, w) =
                            kentos_geometry_core::tools::point_text::parse_length_pair(text)?;
                        Some((l.metres(|n| f.to_metres(n)), w.metres(|n| f.to_metres(n))))
                    });
                match size {
                    Some((length, width)) if length > 0.0 && width > 0.0 => {
                        self.size = Some((length, width));
                        self.stage = Stage::Side;
                        true
                    }
                    _ => false,
                }
            }
            _ => match cx.typed_point(text, self.d.last(), self.d.hover) {
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
