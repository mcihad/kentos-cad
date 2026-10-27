//! Duvarın geçici ölçüleri (docs/adr/0710): the one selected wall's clear distances on each side of the straight edge
//! the pointer is nearest, each to the nearest parallel wall, drawn as dimensions across the gaps. A click on a number
//! opens a text field at it with the value; Enter moves the edge by `cad.walls.edit`'s `distance`, the wall across
//! fixed and what is joined following, in one undo step; Esc leaves it as it was.

use kentos_commands::{DimensionSetFace, Entity, TextAlign, WallGaps, WallsEdit, WallsOperation};
use kentos_domain::{Slot, Uuid};
use kentos_native_application::{ExecutionContext, wall_distances, walls_edit};

use crate::prompt::Prompt;
use crate::tool::{Context, Flow, Pointer, Preview, Stroke, TextField, TextGhost, Tone, Tool, ViewChange};
use crate::{Format, Level, Vec2, dimension_layout};

pub const ID: &str = "wallDistance";
pub const LABEL: &str = "Duvarın mesafesi";

/// The numbers' height, logical pixels: readable at any zoom.
const TEXT_PX: f64 = 12.0;

/// One side's temporary dimension: its side of the edge, its value and where its number stands.
#[derive(Clone, Debug, PartialEq)]
pub struct Side {
    pub left: bool,
    pub value: f64,
    pub text_at: Vec2,
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
    /// The view's scale the dimensions were drawn at.
    scale: Option<u64>,
    pub sides: Vec<Side>,
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

impl Overlay {
    /// Brought up to date for the selection, the pointer at `pointer` (none: the edge as it was, else the first).
    /// The wall is measured once per revision; a pointer's move only picks the edge.
    pub fn see(&mut self, pointer: Option<Vec2>, cx: &Context<'_>) {
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
            self.sides.clear();
            self.preview = Preview::default();
            return;
        };
        let scale = cx.view.world_length(1.0).to_bits();
        if self.edge == Some(edge) && self.scale == Some(scale) && !self.preview.strokes.is_empty()
        {
            return;
        }
        self.edge = Some(edge);
        self.scale = Some(scale);
        self.sides.clear();
        self.preview = Preview::default();
        if let Some(g) = self.edges[edge].0.clone() {
            self.draw(&g, cx);
        }
    }

    /// The gaps of `g` drawn across them, and the walls measured to marked where they are met.
    fn draw(&mut self, g: &WallGaps, cx: &Context<'_>) {
        let height = cx.view.world_length(TEXT_PX);
        for (left, gap) in [(true, &g.left), (false, &g.right)] {
            let Some(gap) = gap else { continue };
            let (from, to) = (v(gap.from), v(gap.to));
            let length = (to.x - from.x).hypot(to.y - from.y);
            let text_at = Vec2::new((from.x + to.x) / 2.0, (from.y + to.y) / 2.0);
            if length > 1e-9 {
                let entity = crate::opening_distance::dimension(from, to, 0.0, height);
                let mut drawn = crate::wall_dimensions::dimension_preview(
                    &entity,
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
                // The face met, marked across the measurement.
                let (dx, dy) = ((to.x - from.x) / length, (to.y - from.y) / length);
                let n = (-dy * height * 0.8, dx * height * 0.8);
                let mut mark = Stroke::solid(
                    vec![
                        Vec2::new(to.x - n.0, to.y - n.1),
                        Vec2::new(to.x + n.0, to.y + n.1),
                    ],
                    false,
                );
                mark.tone = Tone::Snap;
                mark.width = 2.0;
                self.preview.strokes.push(mark);
            }
            let text_at = match (length > 1e-9)
                .then(|| crate::opening_distance::dimension(from, to, 0.0, height))
                .as_ref()
                .and_then(dimension_layout)
            {
                Some(layout) => layout.text_at,
                None => text_at,
            };
            self.sides.push(Side {
                left,
                value: gap.distance,
                text_at,
            });
        }
    }

    /// The text field of the side whose number is under the pointer.
    pub fn at(&self, screen: [f64; 2], cx: &Context<'_>) -> Option<WallDistance> {
        let side = crate::opening_distance::nearest_number(&self.sides, |s| s.text_at, screen, cx)?;
        Some(WallDistance {
            uid: self.uid?,
            edge: self.edge? as u32,
            side: if side.left {
                DimensionSetFace::Left
            } else {
                DimensionSetFace::Right
            },
            at: side.text_at,
            value: side.value,
            done: false,
        })
    }
}

/// The text field over a number, and the move it writes.
#[derive(Clone, Debug, PartialEq)]
pub struct WallDistance {
    uid: Uuid,
    edge: u32,
    side: DimensionSetFace,
    at: Vec2,
    value: f64,
    done: bool,
}

impl WallDistance {
    fn field(&self, initial: String, cx: &mut Context<'_>) {
        cx.view_changes.push(ViewChange::Text(TextField {
            at: self.at,
            height: cx.view.world_length(14.0),
            rotation: 0.0,
            align: Some(TextAlign::MiddleCenter),
            width_factor: 1.0,
            initial: Some(initial),
            placeholder: Some("Mesafe"),
            hint: Some("Yeni net mesafeyi yazın (2.40, 240 cm); Enter duvarı kaydırır, karşıdaki duvar yerinde kalır; Esc vazgeçer."),
            empty: false,
            face: Default::default(),
        }));
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
            },
        };
        let result = walls_edit::execute(&mut ExecutionContext::new(cx.doc), input);
        if let Some(done) = crate::points::written(result, cx) {
            let shown = cx.format().length(distance);
            let joined = done.joined.len();
            cx.say(
                Level::Success,
                if joined > 0 {
                    format!("Duvar kaydırıldı: net mesafe {shown}; bağlı {joined} duvar izledi.")
                } else {
                    format!("Duvar kaydırıldı: net mesafe {shown}.")
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
