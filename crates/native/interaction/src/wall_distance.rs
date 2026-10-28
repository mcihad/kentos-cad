//! Duvarın geçici ölçüleri (docs/adr/0710, 0728): the one selected wall's clear distances on each side of the straight
//! edge the pointer is nearest, each to the nearest parallel wall, drawn as dimensions across the gaps. A side with a
//! parallel grid too has a small square at its reference: a click switches it to the grid (from the edge's axis) and
//! back, the choice kept for the session. Along the edge, when an end of it is the path's, its length, the end that
//! stays marked solid (a square at the other end, when it can stay instead, switches it). A click on a number opens a
//! text field at it with the value; Enter moves the edge by `cad.walls.edit`'s `distance` (the wall or grid across
//! fixed) or makes it the length by its `length` (the marked end fixed), what is joined following, in one undo step;
//! Esc leaves it as it was.

use kentos_commands::{
    DimensionSetFace, DistanceTo, Entity, TextAlign, WallAnchor, WallFields, WallGaps, WallJustify, WallsEdit,
    WallsOperation,
};
use kentos_domain::{Slot, Uuid};
use kentos_native_application::{ExecutionContext, wall_distances, walls_edit};

use crate::prompt::Prompt;
use crate::temporary_refs::{Choices, Switch, other, shown};
use crate::tool::{
    Context, Flow, Marker, MarkerShape, Pointer, Preview, Stroke, TextField, TextGhost, Tone, Tool, ViewChange,
};
use crate::{Format, Level, Vec2, dimension_layout};

pub const ID: &str = "wallDistance";
pub const LABEL: &str = "Duvarın mesafesi";
pub const LENGTH_ID: &str = "wallLength";
pub const LENGTH_LABEL: &str = "Duvarın boyu";

/// The numbers' height, logical pixels: readable at any zoom.
const TEXT_PX: f64 = 12.0;
/// The length's dimension out of the wall's face, logical pixels.
const GAP_PX: f64 = 22.0;

/// One side's temporary dimension: its side of the edge, its value, where its number stands, what it measures to.
#[derive(Clone, Debug, PartialEq)]
pub struct Side {
    pub left: bool,
    pub value: f64,
    pub text_at: Vec2,
    pub to: DistanceTo,
}

/// The edge's length dimension: its value, where its number stands and the end that stays.
#[derive(Clone, Debug, PartialEq)]
pub struct Length {
    pub value: f64,
    pub text_at: Vec2,
    pub anchor: WallAnchor,
}

/// The temporary dimensions of the selected wall, as last seen.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Overlay {
    slot: Option<Slot>,
    uid: Option<Uuid>,
    /// Each edge's gaps at the drawing's revision `measured` (none: an arc edge), and its two points.
    measured: Option<u64>,
    edges: Vec<(Option<WallGaps>, [Vec2; 2])>,
    edge: Option<usize>,
    /// The view's scale the dimensions were drawn at; none: to be drawn again (a choice changed).
    scale: Option<u64>,
    pub sides: Vec<Side>,
    pub length: Option<Length>,
    /// The squares that switch a reference or the fixed end, where they stand.
    pub squares: Vec<(Vec2, Switch)>,
    pub preview: Preview,
}

fn v(p: kentos_commands::Vec2) -> Vec2 {
    Vec2::new(p.x, p.y)
}

/// The selected wall, alone and not on a locked layer.
fn selected(cx: &Context<'_>) -> Option<(Slot, Uuid)> {
    let [slot] = cx.selection.ids() else {
        return None;
    };
    let e = cx.doc.get(*slot)?;
    if !matches!(e, Entity::Wall(_)) || cx.doc.layers().is_locked(&e.base().layer_id) {
        return None;
    }
    Some((*slot, cx.doc.uid(*slot)?))
}

/// How far `p` is from segment `a`–`b`.
fn off_segment(p: Vec2, [a, b]: [Vec2; 2]) -> f64 {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let l2 = dx * dx + dy * dy;
    let t = if l2 > 0.0 {
        (((p.x - a.x) * dx + (p.y - a.y) * dy) / l2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    (p.x - (a.x + dx * t)).hypot(p.y - (a.y + dy * t))
}

/// A wall's faces as offsets to its path's left: its left face, its right face.
fn faces(w: &WallFields) -> (f64, f64) {
    let lf = w.offset
        + match w.justify {
            WallJustify::Left => 0.0,
            WallJustify::Center => w.width / 2.0,
            WallJustify::Right => w.width,
        };
    (lf, lf - w.width)
}

/// The end an edge keeps when its length changes: the one that cannot move, else the session's choice, else its start.
pub fn anchor_of(start_free: bool, end_free: bool, chosen: Option<WallAnchor>) -> WallAnchor {
    match (start_free, end_free) {
        (true, true) => match chosen {
            Some(WallAnchor::End) => WallAnchor::End,
            _ => WallAnchor::Start,
        },
        (true, false) => WallAnchor::End,
        _ => WallAnchor::Start,
    }
}

impl Overlay {
    /// Brought up to date for the selection, the pointer at `pointer` (none: the edge as it was, else the first).
    /// The wall is measured once per revision; a pointer's move only picks the edge.
    pub fn see(&mut self, pointer: Option<Vec2>, choices: &Choices, cx: &Context<'_>) {
        let Some((slot, uid)) = selected(cx) else {
            *self = Self::default();
            return;
        };
        let revision = cx.doc.revision();
        if self.slot != Some(slot) || self.measured != Some(revision) {
            let Some(Entity::Wall(w)) = cx.doc.get(slot) else {
                *self = Self::default();
                return;
            };
            let pts: Vec<Vec2> = w.wall.pts.iter().map(|p| v(*p)).collect();
            let count = if w.wall.closed {
                pts.len()
            } else {
                pts.len().saturating_sub(1)
            };
            let edges = (0..count)
                .map(|k| {
                    let ends = [pts[k], pts[(k + 1) % pts.len()]];
                    let gaps = wall_distances::gaps_of(cx.doc, slot, Some(k as u32)).ok();
                    (gaps, ends)
                })
                .collect();
            let edge = self.edge.filter(|_| self.slot == Some(slot));
            *self = Self {
                slot: Some(slot),
                uid: Some(uid),
                measured: Some(revision),
                edges,
                edge,
                ..Self::default()
            };
        }
        let measured: Vec<usize> = (0..self.edges.len())
            .filter(|k| self.edges[*k].0.is_some())
            .collect();
        let edge = match pointer {
            Some(at) => measured.iter().copied().min_by(|a, b| {
                off_segment(at, self.edges[*a].1).total_cmp(&off_segment(at, self.edges[*b].1))
            }),
            None => self.edge.or_else(|| measured.first().copied()),
        };
        let Some(edge) = edge else {
            self.clear();
            return;
        };
        let scale = cx.view.world_length(1.0).to_bits();
        if self.edge == Some(edge) && self.scale == Some(scale) && !self.preview.strokes.is_empty() {
            return;
        }
        self.edge = Some(edge);
        self.scale = Some(scale);
        self.clear();
        if let Some(g) = self.edges[edge].0.clone() {
            self.draw(&g, choices, cx);
        }
    }

    /// Drawn again at the next look: a choice changed.
    pub fn redraw(&mut self) {
        self.scale = None;
    }

    /// Whether anything is drawn.
    pub fn shows(&self) -> bool {
        !self.sides.is_empty() || self.length.is_some()
    }

    fn clear(&mut self) {
        self.sides.clear();
        self.length = None;
        self.squares.clear();
        self.preview = Preview::default();
    }

    /// A dimension from `from` to `to`, `offset` to its left, drawn in the snap tone: where its number stands.
    fn dimension(&mut self, from: Vec2, to: Vec2, offset: f64, cx: &Context<'_>) -> Vec2 {
        let height = cx.view.world_length(TEXT_PX);
        let entity = crate::opening_distance::dimension(from, to, offset, height);
        let mut drawn =
            crate::wall_dimensions::dimension_preview(&entity, &cx.format(), cx.doc.settings().drawing_font);
        for s in &mut drawn.strokes {
            s.tone = Tone::Snap;
        }
        self.preview.strokes.extend(drawn.strokes);
        // The number over a mask of the view's ground: the line it stands on does not cross it.
        self.preview
            .texts
            .extend(drawn.texts.into_iter().map(|t| TextGhost { mask: true, ..t }));
        dimension_layout(&entity).map_or(Vec2::new((from.x + to.x) / 2.0, (from.y + to.y) / 2.0), |l| l.text_at)
    }

    /// The gaps of `g` drawn across them (each side's chosen reference), the faces or grids met marked where they are
    /// met; the edge's length along it.
    fn draw(&mut self, g: &WallGaps, choices: &Choices, cx: &Context<'_>) {
        let Some(uid) = self.uid else { return };
        let height = cx.view.world_length(TEXT_PX);
        for (left, face, grid) in [(true, &g.left, &g.left_grid), (false, &g.right, &g.right_grid)] {
            let side = u32::from(!left);
            let Some(to) = shown(face.is_some(), grid.is_some(), choices.to(uid, g.edge, side)) else {
                continue;
            };
            let (from, at, value) = match (to, face, grid) {
                (DistanceTo::Face, Some(f), _) => (v(f.from), v(f.to), f.distance),
                (DistanceTo::Grid, _, Some(x)) => (v(x.from), v(x.to), x.distance),
                _ => continue,
            };
            let length = (at.x - from.x).hypot(at.y - from.y);
            let mut text_at = Vec2::new((from.x + at.x) / 2.0, (from.y + at.y) / 2.0);
            if length > 1e-9 {
                text_at = self.dimension(from, at, 0.0, cx);
                // The face or grid met, marked across the measurement.
                let (dx, dy) = ((at.x - from.x) / length, (at.y - from.y) / length);
                let n = (-dy * height * 0.8, dx * height * 0.8);
                let mut mark = Stroke::solid(
                    vec![Vec2::new(at.x - n.0, at.y - n.1), Vec2::new(at.x + n.0, at.y + n.1)],
                    false,
                );
                mark.tone = Tone::Snap;
                mark.width = 2.0;
                self.preview.strokes.push(mark);
            } else {
                // On the grid: the number alone.
                self.preview.texts.push(TextGhost {
                    p: text_at,
                    text: cx.format().length(0.0),
                    height,
                    rotation: 0.0,
                    align: Some(kentos_geometry_core::text::TextAlign::MiddleCenter),
                    mask: true,
                    face: Default::default(),
                    width_factor: 1.0,
                });
            }
            // Two references: a square at this one switches to the other.
            if face.is_some() && grid.is_some() {
                self.preview.squares.push(at);
                self.squares.push((
                    at,
                    Switch::Reference {
                        uid,
                        edge: g.edge,
                        side,
                        to: other(to),
                    },
                ));
            }
            self.sides.push(Side {
                left,
                value,
                text_at,
                to,
            });
        }
        if let Some(l) = &g.length {
            self.draw_length(g, l, choices, cx);
        }
    }

    /// The edge's length along it, out of the face where no gap is drawn (with both, beyond the longer one, whose number
    /// stands farther out of the wall; neither, its right), the end that stays solid.
    fn draw_length(&mut self, g: &WallGaps, l: &kentos_commands::WallLength, choices: &Choices, cx: &Context<'_>) {
        let (Some(uid), Some(slot)) = (self.uid, self.slot) else { return };
        let Some(Entity::Wall(w)) = cx.doc.get(slot) else { return };
        let (lf, rf) = faces(&w.wall);
        let gap = cx.view.world_length(GAP_PX);
        let value = |left: bool| self.sides.iter().find(|s| s.left == left).map(|s| s.value);
        let on_left = match (value(true), value(false)) {
            (None, Some(_)) => true,
            (Some(l), Some(r)) => l > r,
            _ => false,
        };
        let offset = if on_left { lf + gap } else { rf - gap };
        let (a, b) = (v(l.from), v(l.to));
        let text_at = self.dimension(a, b, offset, cx);
        let anchor = anchor_of(l.start_free, l.end_free, choices.anchor(uid, g.edge));
        // The marks on the dimension line's ends, `offset` to the left of the edge, clear of the wall's own grips.
        let length = (b.x - a.x).hypot(b.y - a.y);
        let on_line = |p: Vec2| {
            if length > 1e-12 {
                Vec2::new(p.x - (b.y - a.y) / length * offset, p.y + (b.x - a.x) / length * offset)
            } else {
                p
            }
        };
        let (fixed, free) = match anchor {
            WallAnchor::End => (on_line(b), on_line(a)),
            _ => (on_line(a), on_line(b)),
        };
        // The end that stays: a filled square; the one that can stay instead, a hollow 9 px one to click.
        self.preview.markers.push(Marker {
            at: fixed,
            shape: MarkerShape::Square(3.5),
            tone: Tone::Accent,
        });
        if l.start_free && l.end_free {
            self.preview.squares.push(free);
            self.squares.push((
                free,
                Switch::Anchor {
                    uid,
                    edge: g.edge,
                    anchor: match anchor {
                        WallAnchor::End => WallAnchor::Start,
                        _ => WallAnchor::End,
                    },
                },
            ));
        }
        self.length = Some(Length {
            value: l.length,
            text_at,
            anchor,
        });
    }

    /// The text field of the number under the pointer: a side's distance or the edge's length.
    pub fn at(&self, screen: [f64; 2], cx: &Context<'_>) -> Option<Box<dyn Tool>> {
        enum Number<'a> {
            Side(&'a Side),
            Length(&'a Length),
        }
        let mut numbers: Vec<(Vec2, Number<'_>)> = self.sides.iter().map(|s| (s.text_at, Number::Side(s))).collect();
        if let Some(l) = &self.length {
            numbers.push((l.text_at, Number::Length(l)));
        }
        let (_, number) = crate::opening_distance::nearest_number(&numbers, |n| n.0, screen, cx)?;
        let (uid, edge) = (self.uid?, self.edge? as u32);
        Some(match number {
            Number::Side(side) => Box::new(WallDistance {
                uid,
                edge,
                side: if side.left {
                    DimensionSetFace::Left
                } else {
                    DimensionSetFace::Right
                },
                to: side.to,
                at: side.text_at,
                value: side.value,
                done: false,
            }),
            Number::Length(l) => Box::new(WallLength {
                uid,
                edge,
                anchor: l.anchor,
                at: l.text_at,
                value: l.value,
                done: false,
            }),
        })
    }
}

/// A text field's `ViewChange` at `at`, its value `initial`, saying `hint`.
fn field(at: Vec2, initial: String, placeholder: &'static str, hint: &'static str, cx: &mut Context<'_>) {
    cx.view_changes.push(ViewChange::Text(TextField {
        at,
        height: cx.view.world_length(14.0),
        rotation: 0.0,
        align: Some(TextAlign::MiddleCenter),
        width_factor: 1.0,
        initial: Some(initial),
        placeholder: Some(placeholder),
        hint: Some(hint),
        empty: false,
        face: Default::default(),
    }));
}

/// The text field over a side's number, and the move it writes.
#[derive(Clone, Debug, PartialEq)]
pub struct WallDistance {
    uid: Uuid,
    edge: u32,
    side: DimensionSetFace,
    to: DistanceTo,
    at: Vec2,
    value: f64,
    done: bool,
}

impl WallDistance {
    fn field(&self, initial: String, cx: &mut Context<'_>) {
        let hint = match self.to {
            DistanceTo::Face => {
                "Yeni net mesafeyi yazın (2.40, 240 cm); Enter duvarı kaydırır, karşıdaki duvar yerinde kalır; Esc vazgeçer."
            }
            DistanceTo::Grid => {
                "Duvarın ekseninden aksa yeni mesafeyi yazın (0 ekseni aksa koyar); Enter duvarı kaydırır, aks yerinde kalır; Esc vazgeçer."
            }
        };
        field(self.at, initial, "Mesafe", hint, cx);
    }
}

impl Tool for WallDistance {
    fn id(&self) -> &'static str {
        ID
    }
    fn label(&self) -> &'static str {
        LABEL
    }
    fn prompt(&self) -> Prompt {
        Prompt::new(
            LABEL,
            "yeni net mesafeyi yazın; Enter duvarı kaydırır, Esc vazgeçer",
        )
    }
    fn point_count(&self) -> usize {
        0
    }
    fn activate(&mut self, cx: &mut Context<'_>) -> Flow {
        let shown = cx.format().length(self.value);
        self.field(shown, cx);
        Flow::Stay
    }
    fn pointer_move(&mut self, _p: &Pointer, _cx: &mut Context<'_>) {}
    /// A click elsewhere leaves the field as Esc does.
    fn pointer_down(&mut self, _p: &Pointer, _cx: &mut Context<'_>) {
        self.done = true;
    }
    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        self.text_typed(Some(text), cx);
        true
    }
    fn confirm(&mut self, _cx: &mut Context<'_>) -> Flow {
        Flow::Exit
    }
    fn undo_step(&mut self, _cx: &mut Context<'_>) -> bool {
        self.done = true;
        true
    }
    fn text_typed(&mut self, text: Option<&str>, cx: &mut Context<'_>) {
        let Some(text) = text else {
            self.done = true;
            return;
        };
        let Some(distance) = cx.typed_length(text).filter(|d| d.is_finite()) else {
            cx.say(
                Level::Warn,
                "Geçerli bir uzunluk yazın; örneğin 2.40, 240 cm ya da 2.4 m.",
            );
            self.field(text.to_owned(), cx);
            return;
        };
        let input = WallsEdit {
            uids: vec![self.uid.to_string()],
            expected_revision: None,
            operation: WallsOperation::Distance {
                edge: Some(self.edge),
                side: self.side,
                distance,
                to: self.to,
            },
        };
        let result = walls_edit::execute(&mut ExecutionContext::new(cx.doc), input);
        if let Some(done) = crate::points::written(result, cx) {
            let shown = cx.format().length(distance);
            let joined = done.joined.len();
            let what = match self.to {
                DistanceTo::Face => "net mesafe",
                DistanceTo::Grid => "aksa mesafe",
            };
            cx.say(
                Level::Success,
                if joined > 0 {
                    format!("Duvar kaydırıldı: {what} {shown}; bağlı {joined} duvar izledi.")
                } else {
                    format!("Duvar kaydırıldı: {what} {shown}.")
                },
            );
        }
        self.done = true;
    }
    fn cancel(&mut self, _cx: &mut Context<'_>) -> bool {
        self.done = true;
        true
    }
    fn finished(&self) -> bool {
        self.done
    }
    fn preview(&self, _: &Format) -> Preview {
        Preview::default()
    }
}

/// The text field over the edge's length, and the length it writes.
#[derive(Clone, Debug, PartialEq)]
pub struct WallLength {
    uid: Uuid,
    edge: u32,
    anchor: WallAnchor,
    at: Vec2,
    value: f64,
    done: bool,
}

impl WallLength {
    fn field(&self, initial: String, cx: &mut Context<'_>) {
        let hint = match self.anchor {
            WallAnchor::End => {
                "Kenarın yeni boyunu yazın (4.20, 420 cm); Enter başını kaydırır, işaretli sonu yerinde kalır; Esc vazgeçer."
            }
            _ => "Kenarın yeni boyunu yazın (4.20, 420 cm); Enter sonunu kaydırır, işaretli başı yerinde kalır; Esc vazgeçer.",
        };
        field(self.at, initial, "Boy", hint, cx);
    }
}

impl Tool for WallLength {
    fn id(&self) -> &'static str {
        LENGTH_ID
    }
    fn label(&self) -> &'static str {
        LENGTH_LABEL
    }
    fn prompt(&self) -> Prompt {
        Prompt::new(
            LENGTH_LABEL,
            "kenarın yeni boyunu yazın; Enter boyu değiştirir, Esc vazgeçer",
        )
    }
    fn point_count(&self) -> usize {
        0
    }
    fn activate(&mut self, cx: &mut Context<'_>) -> Flow {
        let shown = cx.format().length(self.value);
        self.field(shown, cx);
        Flow::Stay
    }
    fn pointer_move(&mut self, _p: &Pointer, _cx: &mut Context<'_>) {}
    /// A click elsewhere leaves the field as Esc does.
    fn pointer_down(&mut self, _p: &Pointer, _cx: &mut Context<'_>) {
        self.done = true;
    }
    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        self.text_typed(Some(text), cx);
        true
    }
    fn confirm(&mut self, _cx: &mut Context<'_>) -> Flow {
        Flow::Exit
    }
    fn undo_step(&mut self, _cx: &mut Context<'_>) -> bool {
        self.done = true;
        true
    }
    fn text_typed(&mut self, text: Option<&str>, cx: &mut Context<'_>) {
        let Some(text) = text else {
            self.done = true;
            return;
        };
        let Some(length) = cx.typed_length(text).filter(|d| d.is_finite()) else {
            cx.say(
                Level::Warn,
                "Geçerli bir uzunluk yazın; örneğin 4.20, 420 cm ya da 4.2 m.",
            );
            self.field(text.to_owned(), cx);
            return;
        };
        let input = WallsEdit {
            uids: vec![self.uid.to_string()],
            expected_revision: None,
            operation: WallsOperation::Length {
                edge: Some(self.edge),
                length,
                anchor: self.anchor,
            },
        };
        let result = walls_edit::execute(&mut ExecutionContext::new(cx.doc), input);
        if let Some(done) = crate::points::written(result, cx) {
            let shown = cx.format().length(length);
            let joined = done.joined.len();
            cx.say(
                Level::Success,
                if joined > 0 {
                    format!("Duvarın boyu {shown} oldu; bağlı {joined} duvar izledi.")
                } else {
                    format!("Duvarın boyu {shown} oldu.")
                },
            );
        }
        self.done = true;
    }
    fn cancel(&mut self, _cx: &mut Context<'_>) -> bool {
        self.done = true;
        true
    }
    fn finished(&self) -> bool {
        self.done
    }
    fn preview(&self, _: &Format) -> Preview {
        Preview::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_end_that_stays_is_the_one_that_cannot_move_else_the_choice_else_the_start() {
        assert_eq!(anchor_of(true, true, None), WallAnchor::Start);
        assert_eq!(anchor_of(true, true, Some(WallAnchor::End)), WallAnchor::End);
        assert_eq!(anchor_of(true, true, Some(WallAnchor::Middle)), WallAnchor::Start, "not offered here");
        assert_eq!(anchor_of(true, false, Some(WallAnchor::Start)), WallAnchor::End, "only its start moves");
        assert_eq!(anchor_of(false, true, Some(WallAnchor::End)), WallAnchor::Start);
    }
}
