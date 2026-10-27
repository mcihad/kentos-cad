//! Kolonun geçici ölçüleri (docs/adr/0712): the one selected column's clear distances on its four sides, each to the
//! nearest parallel face (a wall's or another column's), drawn as dimensions across the gaps. A click on a number opens
//! a text field at it with the value; Enter moves the column square to that side by `cad.arch.edit`'s `distance`, the
//! face across fixed, in one undo step; Esc leaves it as it was. A column tied to grids says why it does not move at
//! the click, before anything is typed.

use kentos_commands::{ArchEdit, ArchOperation, ColumnGaps, CommandResult, Entity, TextAlign};
use kentos_domain::{Slot, Uuid};
use kentos_native_application::{ExecutionContext, arch_edit, column_distances};

use crate::prompt::Prompt;
use crate::tool::{Context, Flow, Pointer, Preview, Stroke, TextField, TextGhost, Tone, Tool, ViewChange};
use crate::{Format, Level, Vec2, dimension_layout};

pub const ID: &str = "columnDistance";
pub const LABEL: &str = "Kolonun mesafesi";

/// The numbers' height, logical pixels: readable at any zoom.
const TEXT_PX: f64 = 12.0;

/// One side's temporary dimension: its side, its value and where its number stands.
#[derive(Clone, Debug, PartialEq)]
pub struct Side {
    pub side: u32,
    pub value: f64,
    pub text_at: Vec2,
}

/// The temporary dimensions of the selected column, as last seen.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Overlay {
    slot: Option<Slot>,
    uid: Option<Uuid>,
    measured: Option<u64>,
    gaps: Option<ColumnGaps>,
    scale: Option<u64>,
    pub sides: Vec<Side>,
    pub preview: Preview,
}

fn v(p: kentos_commands::Vec2) -> Vec2 {
    Vec2::new(p.x, p.y)
}

/// The selected column, alone and not on a locked layer.
fn selected(cx: &Context<'_>) -> Option<(Slot, Uuid)> {
    let [slot] = cx.selection.ids() else {
        return None;
    };
    let e = cx.doc.get(*slot)?;
    if !matches!(e, Entity::Column(_)) || cx.doc.layers().is_locked(&e.base().layer_id) {
        return None;
    }
    Some((*slot, cx.doc.uid(*slot)?))
}

impl Overlay {
    /// Brought up to date for the selection: measured once per revision, drawn again when the view's scale changes.
    pub fn see(&mut self, cx: &Context<'_>) {
        let Some((slot, uid)) = selected(cx) else {
            *self = Self::default();
            return;
        };
        let revision = cx.doc.revision();
        let scale = cx.view.world_length(1.0).to_bits();
        if self.slot == Some(slot) && self.measured == Some(revision) && self.scale == Some(scale) {
            return;
        }
        *self = Self {
            slot: Some(slot),
            uid: Some(uid),
            measured: Some(revision),
            gaps: column_distances::gaps_of(cx.doc, slot).ok(),
            scale: Some(scale),
            ..Self::default()
        };
        self.draw(cx);
    }

    /// The gaps drawn across them, the faces met marked.
    fn draw(&mut self, cx: &Context<'_>) {
        let Some(gaps) = self.gaps.clone() else {
            return;
        };
        let height = cx.view.world_length(TEXT_PX);
        for gap in &gaps.sides {
            let (from, to) = (v(gap.from), v(gap.to));
            let length = (to.x - from.x).hypot(to.y - from.y);
            let mut text_at = Vec2::new((from.x + to.x) / 2.0, (from.y + to.y) / 2.0);
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
                if let Some(layout) = dimension_layout(&entity) {
                    text_at = layout.text_at;
                }
            }
            self.sides.push(Side {
                side: gap.side,
                value: gap.distance,
                text_at,
            });
        }
    }

    /// The text field of the side whose number is under the pointer.
    pub fn at(&self, screen: [f64; 2], cx: &Context<'_>) -> Option<ColumnDistance> {
        let side = crate::opening_distance::nearest_number(&self.sides, |s| s.text_at, screen, cx)?;
        Some(ColumnDistance {
            uid: self.uid?,
            side: side.side,
            at: side.text_at,
            value: side.value,
            tied: self.gaps.as_ref().is_some_and(|g| g.tied),
            done: false,
        })
    }
}

/// The text field over a number, and the move it writes.
#[derive(Clone, Debug, PartialEq)]
pub struct ColumnDistance {
    uid: Uuid,
    side: u32,
    at: Vec2,
    value: f64,
    /// Tied to grids: the command's refusal is said at the click, no field opens.
    tied: bool,
    done: bool,
}

impl ColumnDistance {
    fn edit(&self, distance: f64) -> ArchEdit {
        ArchEdit {
            uids: vec![self.uid.to_string()],
            operation: ArchOperation::Distance {
                side: self.side,
                distance,
            },
            expected_revision: None,
        }
    }

    fn field(&self, initial: String, cx: &mut Context<'_>) {
        cx.view_changes.push(ViewChange::Text(TextField {
            at: self.at,
            height: cx.view.world_length(14.0),
            rotation: 0.0,
            align: Some(TextAlign::MiddleCenter),
            width_factor: 1.0,
            initial: Some(initial),
            placeholder: Some("Mesafe"),
            hint: Some("Yeni net mesafeyi yazın (1.20, 120 cm); Enter kolonu kaydırır, karşıdaki yüz yerinde kalır; Esc vazgeçer."),
            empty: false,
            face: Default::default(),
        }));
    }
}

impl Tool for ColumnDistance {
    fn id(&self) -> &'static str {
        ID
    }
    fn label(&self) -> &'static str {
        LABEL
    }
    fn prompt(&self) -> Prompt {
        Prompt::new(LABEL, "yeni net mesafeyi yazın; Enter kolonu kaydırır, Esc vazgeçer")
    }
    fn point_count(&self) -> usize {
        0
    }
    fn activate(&mut self, cx: &mut Context<'_>) -> Flow {
        if self.tied {
            if let CommandResult::Failed { error } = arch_edit::plan_in(cx.doc, &self.edit(self.value)) {
                cx.say(Level::Warn, error.message);
            }
            self.done = true;
            return Flow::Exit;
        }
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
                "Geçerli bir uzunluk yazın; örneğin 1.20, 120 cm ya da 1.2 m.",
            );
            self.field(text.to_owned(), cx);
            return;
        };
        let result = arch_edit::execute(&mut ExecutionContext::new(cx.doc), self.edit(distance));
        if crate::points::written(result, cx).is_some() {
            let shown = cx.format().length(distance);
            cx.say(Level::Success, format!("Kolon kaydırıldı: net mesafe {shown}."));
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
