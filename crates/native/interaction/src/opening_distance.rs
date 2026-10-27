//! Açıklığın geçici ölçüleri (docs/adr/0701): the one selected opening's distances to the nearest references on each
//! side along its wall's face, drawn as dimensions over the drawing; the face is the one the pointer is by. A click on
//! a number opens a text field at it with the value; Enter moves the opening by `cad.openings.edit`'s `distance`, the
//! reference fixed, in one undo step; Esc leaves it as it was.

use kentos_commands::{DimensionSetFace, Entity, OpeningsEdit, OpeningsOperation, TextAlign, WallEnd};
use kentos_domain::{Slot, Uuid};
use kentos_native_application::opening_distances::{Measured, measured};
use kentos_native_application::{ExecutionContext, openings_edit};

use crate::prompt::Prompt;
use crate::tool::{Context, Flow, Pointer, Preview, Stroke, TextField, TextGhost, Tone, Tool, ViewChange};
use crate::{Format, Level, Vec2, dimension_layout};

pub const ID: &str = "openingDistance";
pub const LABEL: &str = "Açıklığın mesafesi";

/// The numbers' height and the dimensions' distance out from the face, logical pixels: readable at any zoom.
const TEXT_PX: f64 = 12.0;
const GAP_PX: f64 = 22.0;
/// How near a number a click takes it, logical pixels.
const REACH_PX: [f64; 2] = [30.0, 12.0];

/// One side's temporary dimension: its value and where its number stands.
#[derive(Clone, Debug, PartialEq)]
pub struct Side {
    pub start: bool,
    pub value: f64,
    pub text_at: Vec2,
}

/// The temporary dimensions of the selected opening, as last seen.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Overlay {
    slot: Option<Slot>,
    uid: Option<Uuid>,
    /// The opening measured on its left and right faces, at the drawing's revision `measured`.
    measured: Option<u64>,
    faces: [Option<Measured>; 2],
    face: Option<DimensionSetFace>,
    /// The view's scale the dimensions were drawn at.
    scale: Option<u64>,
    pub sides: Vec<Side>,
    pub preview: Preview,
}

fn v(p: kentos_commands::Vec2) -> Vec2 {
    Vec2::new(p.x, p.y)
}

/// Of the numbers whose text stands within reach of `screen` (a box `REACH_PX` wide and high round each), the nearest:
/// two numbers close together (a narrow opening's, a column's sides) go to the one the click is by, not the first.
pub(crate) fn nearest_number<'a, T>(
    items: &'a [T],
    text_at: impl Fn(&T) -> Vec2,
    screen: [f64; 2],
    cx: &Context<'_>,
) -> Option<&'a T> {
    items
        .iter()
        .filter_map(|item| {
            let p = cx.view.to_screen(text_at(item));
            let (dx, dy) = ((p[0] - screen[0]).abs(), (p[1] - screen[1]).abs());
            (dx <= REACH_PX[0] && dy <= REACH_PX[1]).then(|| ((dx / REACH_PX[0]).hypot(dy / REACH_PX[1]), item))
        })
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, item)| item)
}

/// An aligned dimension from `a` to `b`, `offset` to its left, numbers `height` high: drawn, never written.
pub(crate) fn dimension(a: Vec2, b: Vec2, offset: f64, height: f64) -> Entity {
    Entity::Dimension(kentos_commands::DimensionEntity {
        base: kentos_commands::EntityBase {
            look: Default::default(),
            id: 0,
            layer_id: String::new(),
            color: None,
            attrs: Default::default(),
            label: None,
            symbol: None,
            line_weight: None,
            label_pins: Vec::new(),
            attachments: Vec::new(),
        },
        a: kentos_commands::Vec2 { x: a.x, y: a.y },
        b: kentos_commands::Vec2 { x: b.x, y: b.y },
        offset,
        height,
        text: None,
        style: None,
        angle: None,
        c: None,
        mask: false,
        za: None,
        zb: None,
        refs: None,
        text_shift: None,
        breaks: Vec::new(),
        look: Default::default(),
        annotative: None,
        level: None,
    })
}

/// The selected opening, alone and not on a locked layer.
fn selected(cx: &Context<'_>) -> Option<(Slot, Uuid)> {
    let [slot] = cx.selection.ids() else {
        return None;
    };
    let e = cx.doc.get(*slot)?;
    if !matches!(e, Entity::Opening(_)) || cx.doc.layers().is_locked(&e.base().layer_id) {
        return None;
    }
    Some((*slot, cx.doc.uid(*slot)?))
}

/// The face the pointer is by: of the two faces' lines, the nearer.
fn face_by(left: &Measured, right: &Measured, at: Vec2) -> DimensionSetFace {
    let off = |m: &Measured| {
        let [a, b] = m.line;
        let l = (b.x - a.x).hypot(b.y - a.y);
        ((at.x - a.x) * -(b.y - a.y) / l + (at.y - a.y) * (b.x - a.x) / l).abs()
    };
    if off(right) < off(left) {
        DimensionSetFace::Right
    } else {
        DimensionSetFace::Left
    }
}

impl Overlay {
    /// Brought up to date for the selection, the pointer at `pointer` (none: the face as it was, else the left).
    /// The opening is measured once per revision; a pointer's move only picks the face.
    pub fn see(&mut self, pointer: Option<Vec2>, cx: &Context<'_>) {
        let Some((slot, uid)) = selected(cx) else {
            *self = Self::default();
            return;
        };
        let revision = cx.doc.revision();
        if self.slot != Some(slot) || self.measured != Some(revision) {
            let Some(Entity::Opening(o)) = cx.doc.get(slot) else {
                *self = Self::default();
                return;
            };
            let on = |face| measured(cx.doc, slot, &o.opening, face);
            let face = self.face.filter(|_| self.slot == Some(slot));
            *self = Self {
                slot: Some(slot),
                uid: Some(uid),
                measured: Some(revision),
                faces: [on(DimensionSetFace::Left), on(DimensionSetFace::Right)],
                face,
                ..Self::default()
            };
        }
        let face = match (pointer, &self.faces) {
            (_, [None, None]) => {
                self.sides.clear();
                self.preview = Preview::default();
                return;
            }
            (_, [Some(_), None]) => DimensionSetFace::Left,
            (_, [None, Some(_)]) => DimensionSetFace::Right,
            (Some(at), [Some(l), Some(r)]) => face_by(l, r, at),
            (None, _) => self.face.unwrap_or_default(),
        };
        let scale = cx.view.world_length(1.0).to_bits();
        if self.face == Some(face) && self.scale == Some(scale) && !self.sides.is_empty() {
            return;
        }
        self.face = Some(face);
        self.scale = Some(scale);
        self.sides.clear();
        self.preview = Preview::default();
        let Some(m) = self.faces[usize::from(face == DimensionSetFace::Right)].clone() else {
            return;
        };
        self.draw(&m, face, cx);
    }

    /// The dimensions of `m` on `face`, out of the wall by a few pixels, and the reference marks.
    fn draw(&mut self, m: &Measured, face: DimensionSetFace, cx: &Context<'_>) {
        let [a, b] = m.line;
        let l = (b.x - a.x).hypot(b.y - a.y);
        let u = ((b.x - a.x) / l, (b.y - a.y) / l);
        // Out of the wall: the left face's left, the right face's right.
        let out = match face {
            DimensionSetFace::Left => (-u.1, u.0),
            DimensionSetFace::Right => (u.1, -u.0),
        };
        let gap = cx.view.world_length(GAP_PX);
        let height = cx.view.world_length(TEXT_PX);
        for start in [true, false] {
            let (Some(value), Some(r)) = (m.distance(start), if start { m.start } else { m.end }) else {
                continue;
            };
            let jamb = v(m.jambs[usize::from(!start)].0);
            let to = v(r.at);
            let along = (to.x - jamb.x, to.y - jamb.y);
            let length = along.0.hypot(along.1);
            // A dimension's offset is measured to the left of its first point → its second.
            let left = if length > 0.0 {
                (-along.1 / length, along.0 / length)
            } else {
                out
            };
            let offset = gap * (out.0 * left.0 + out.1 * left.1);
            let entity = (length > 1e-9).then(|| dimension(jamb, to, offset, height));
            let text_at = match entity.as_ref().and_then(dimension_layout) {
                Some(layout) => layout.text_at,
                None => Vec2::new(jamb.x + out.0 * gap, jamb.y + out.1 * gap),
            };
            match &entity {
                Some(entity) => {
                    let mut drawn = crate::wall_dimensions::dimension_preview(
                        entity,
                        &cx.format(),
                        cx.doc.settings().drawing_font,
                    );
                    for s in &mut drawn.strokes {
                        s.tone = Tone::Snap;
                    }
                    self.preview.strokes.extend(drawn.strokes);
                    // The number over a mask of the view's ground: the line it stands on does not cross it.
                    self.preview
                        .texts
                        .extend(drawn.texts.into_iter().map(|t| TextGhost { mask: true, ..t }));
                }
                // A jamb at its reference: the number alone.
                None => self.preview.texts.push(TextGhost {
                    p: text_at,
                    text: cx.format().length(0.0),
                    height,
                    rotation: 0.0,
                    align: Some(kentos_geometry_core::text::TextAlign::MiddleCenter),
                    mask: true,
                    face: Default::default(),
                    width_factor: 1.0,
                }),
            }
            // The reference, marked where it stands on the face.
            let mut mark = Stroke::solid(
                vec![
                    Vec2::new(to.x - out.0 * height * 0.6, to.y - out.1 * height * 0.6),
                    Vec2::new(to.x + out.0 * (gap + height * 0.6), to.y + out.1 * (gap + height * 0.6)),
                ],
                false,
            );
            mark.tone = Tone::Snap;
            mark.width = 2.0;
            self.preview.strokes.push(mark);
            self.sides.push(Side {
                start,
                value,
                text_at,
            });
        }
    }

    /// The text field of the side whose number is under the pointer.
    pub fn at(&self, screen: [f64; 2], cx: &Context<'_>) -> Option<OpeningDistance> {
        let side = nearest_number(&self.sides, |s| s.text_at, screen, cx)?;
        Some(OpeningDistance {
            uid: self.uid?,
            side: if side.start { WallEnd::Start } else { WallEnd::End },
            face: self.face?,
            at: side.text_at,
            value: side.value,
            done: false,
        })
    }
}

/// The text field over a number, and the move it writes.
#[derive(Clone, Debug, PartialEq)]
pub struct OpeningDistance {
    uid: Uuid,
    side: WallEnd,
    face: DimensionSetFace,
    at: Vec2,
    value: f64,
    done: bool,
}

impl OpeningDistance {
    fn field(&self, initial: String, cx: &mut Context<'_>) {
        cx.view_changes.push(ViewChange::Text(TextField {
            at: self.at,
            height: cx.view.world_length(14.0),
            rotation: 0.0,
            align: Some(TextAlign::MiddleCenter),
            width_factor: 1.0,
            initial: Some(initial),
            placeholder: Some("Mesafe"),
            hint: Some("Yeni mesafeyi yazın (90 cm, 0.9); Enter açıklığı taşır, Esc vazgeçer."),
            empty: false,
            face: Default::default(),
        }));
    }
}

impl Tool for OpeningDistance {
    fn id(&self) -> &'static str {
        ID
    }
    fn label(&self) -> &'static str {
        LABEL
    }
    fn prompt(&self) -> Prompt {
        Prompt::new(LABEL, "yeni mesafeyi yazın; Enter taşır, Esc vazgeçer")
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
                "Geçerli bir uzunluk yazın; örneğin 90 cm, 0.9 ya da 0.9 m.",
            );
            self.field(text.to_owned(), cx);
            return;
        };
        let input = OpeningsEdit {
            uids: vec![self.uid.to_string()],
            expected_revision: None,
            operation: OpeningsOperation::Distance {
                side: self.side,
                face: self.face,
                distance,
            },
        };
        let result = openings_edit::execute(&mut ExecutionContext::new(cx.doc), input);
        if crate::points::written(result, cx).is_some() {
            let side = if matches!(self.side, WallEnd::Start) {
                "baştaki"
            } else {
                "sondaki"
            };
            let shown = cx.format().length(distance);
            cx.say(Level::Success, format!("Açıklık taşındı: {side} mesafesi {shown}."));
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
