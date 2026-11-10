//! Kesit çıkar (docs/adr/8042 §5; M42, ADV-19): a click on a section mark's line picks it (a mark selected before the
//! tool starts is taken); the next click places the section's left end at ±0.00 (Enter: right of the drawing) and
//! `cad.sections.edit` draws the section from the model into the mark's block, one undo step. Its plan is the preview
//! drawn at the cursor. A mark whose section is drawn already is drawn again (Yeniden çiz) when the model changed, and
//! said to be current when it did not.

use kentos_commands::{
    BuildingSectionLeft, BuildingSectionsEdit, BuildingSectionsOperation, CommandResult, Entity,
    TextAlign, ViewMarkForm,
};
use kentos_domain::Slot;
use kentos_native_application::geometry::shape;
use kentos_native_application::{ExecutionContext, sections, sections_edit};

use crate::Vec2;
use crate::edge::Outline;
use crate::format::Format;
use crate::log::Level;
use crate::points;
use crate::prompt::Prompt;
use crate::tool::{Context, Cursor, Flow, Pointer, Preview, Tag, TextGhost, Tone, Tool};

pub const ID: &str = "sectionDraw";
/// Görünüş çıkar (docs/adr/8043): the same tool on an elevation mark, picked by its bubble.
pub const FACADE_ID: &str = "facadeDraw";

#[derive(Clone, Debug, Default)]
pub struct SectionTool {
    /// The marks it draws from: section marks (Kesit çıkar) or elevation marks (Görünüş çıkar).
    form: ViewMarkForm,
    mark: Option<Slot>,
    hover: Option<Vec2>,
    drawn: Preview,
    note: Option<String>,
    problem: Option<String>,
    /// The preview's objects in the block's coordinates, drawn moved to the cursor.
    pieces: Vec<Entity>,
}

/// The words for what the model does not hold.
fn left_words(why: BuildingSectionLeft) -> &'static str {
    match why {
        BuildingSectionLeft::Roof => "çatı",
        BuildingSectionLeft::NaturalGround => "tabii zemin",
        BuildingSectionLeft::Curved => "eğri öğe",
        BuildingSectionLeft::Ramp => "rampa",
        BuildingSectionLeft::NoHeight => "yüksekliği olmayan duvar",
        BuildingSectionLeft::HeadUnknown => "lentosuz açıklık",
        BuildingSectionLeft::StairWaist => "plaksız merdiven",
        BuildingSectionLeft::Recess => "niş ve baca",
    }
}

fn core_align(a: Option<TextAlign>) -> kentos_geometry_core::text::TextAlign {
    use kentos_geometry_core::text::TextAlign as C;
    match a {
        Some(TextAlign::TopCenter) => C::TopCenter,
        Some(TextAlign::BottomCenter) => C::BottomCenter,
        Some(TextAlign::BottomLeft) => C::BottomLeft,
        _ => C::MiddleCenter,
    }
}

impl SectionTool {
    pub fn new() -> Self {
        Self::default()
    }

    /// Görünüş çıkar: elevation marks.
    pub fn facades() -> Self {
        Self {
            form: ViewMarkForm::Elevation,
            ..Self::default()
        }
    }

    fn elevations(&self) -> bool {
        self.form == ViewMarkForm::Elevation
    }

    /// The mark of the tool's form nearest the point on a shown layer: a section mark whose line passes within the
    /// pick distance; an elevation mark whose bubble holds the point (twice its words' height, at least the pick
    /// distance).
    fn mark_at(&self, at: Vec2, cx: &Context<'_>) -> Option<Slot> {
        let tol = cx.pick_tolerance();
        let mut best: Option<(f64, Slot)> = None;
        for e in cx.doc.entities() {
            let Entity::ViewMark(v) = e else { continue };
            if v.mark.form != self.form || !cx.doc.layers().is_visible(&e.base().layer_id) {
                continue;
            }
            if self.elevations() {
                let Some(c) = v.mark.points.first() else {
                    continue;
                };
                let h = v.mark.size.unwrap_or(2.5) * cx.doc.settings().plot_scale / 1000.0;
                let d = (at.x - c.x).hypot(at.y - c.y);
                if d <= (2.0 * h).max(tol) && best.is_none_or(|(b, _)| d < b) {
                    best = Some((d, Slot(e.base().id)));
                }
                continue;
            }
            for w in v.mark.points.windows(2) {
                let (a, b) = (w[0], w[1]);
                let (dx, dy) = (b.x - a.x, b.y - a.y);
                let len2 = dx * dx + dy * dy;
                let t = if len2 > 0.0 {
                    (((at.x - a.x) * dx + (at.y - a.y) * dy) / len2).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                let d = (at.x - (a.x + dx * t)).hypot(at.y - (a.y + dy * t));
                if d <= tol && best.is_none_or(|(b, _)| d < b) {
                    best = Some((d, Slot(e.base().id)));
                }
            }
        }
        best.map(|(_, s)| s)
    }

    /// The operation for the picked mark: a refresh when its section is drawn, else a derive at `at`.
    fn input_at(&self, at: Option<Vec2>, cx: &Context<'_>) -> Option<BuildingSectionsEdit> {
        let uid = cx.doc.uid(self.mark?)?;
        let mark = uid.to_string();
        let operation = if cx.doc.block(sections::block_id(uid)).is_some() {
            BuildingSectionsOperation::Refresh { mark }
        } else {
            BuildingSectionsOperation::Derive {
                mark,
                at: at.map(|p| kentos_commands::Vec2 { x: p.x, y: p.y }),
            }
        };
        Some(BuildingSectionsEdit {
            operation,
            expected_revision: None,
        })
    }

    fn see(&mut self, cx: &Context<'_>) {
        self.problem = None;
        self.note = None;
        self.pieces.clear();
        let Some(input) = self.input_at(self.hover, cx) else {
            self.drawn = Preview::default();
            return;
        };
        match sections_edit::plan_in(cx.doc, &input) {
            CommandResult::Completed { output, .. } => {
                if matches!(input.operation, BuildingSectionsOperation::Derive { .. }) {
                    self.pieces = output.drawn;
                }
                let mut words: Vec<&str> = Vec::new();
                for l in &output.left_out {
                    let w = left_words(l.why);
                    if !words.contains(&w) {
                        words.push(w);
                    }
                }
                let mut note = output.name.clone();
                if !words.is_empty() {
                    note.push_str(&format!(" · çizilmeyen: {}", words.join(", ")));
                }
                self.note = Some(note);
            }
            CommandResult::Failed { error }
            | CommandResult::Conflict { error }
            | CommandResult::NeedsInput { error } => self.problem = Some(error.message),
            CommandResult::Queued { .. } | CommandResult::Cancelled => {}
        }
        self.place();
    }

    /// The preview's objects moved so that the section's base point is at the cursor.
    fn place(&mut self) {
        self.drawn = Preview::default();
        let Some(at) = self.hover else { return };
        for e in &self.pieces {
            match e {
                Entity::Text(t) => self.drawn.texts.push(TextGhost {
                    p: Vec2::new(t.p.x + at.x, t.p.y + at.y),
                    text: t.text.clone(),
                    height: t.height,
                    rotation: t.rotation,
                    align: Some(core_align(t.align)),
                    mask: false,
                    face: Default::default(),
                    width_factor: 1.0,
                }),
                Entity::Hatch(_) => {}
                other => {
                    let mut o = Outline::of(&shape(other), None, 1.0, Tone::Accent);
                    for s in &mut o.strokes {
                        for q in &mut s.pts {
                            q.x += at.x;
                            q.y += at.y;
                        }
                    }
                    self.drawn.strokes.extend(o.strokes);
                }
            }
        }
    }

    fn write(&mut self, at: Option<Vec2>, cx: &mut Context<'_>) {
        let Some(input) = self.input_at(at, cx) else {
            return;
        };
        let refresh = matches!(input.operation, BuildingSectionsOperation::Refresh { .. });
        let result = sections_edit::execute(&mut ExecutionContext::new(cx.doc), input);
        if let Some(out) = points::written(result, cx) {
            cx.say(
                Level::Success,
                if refresh {
                    format!("{} yeniden çizildi; paftadaki yerleşimleri de güncellendi.", out.name)
                } else {
                    format!(
                        "{} çıkarıldı; plan değişince Öznitelikler “eski” der, Yeniden çiz günceller.",
                        out.name
                    )
                },
            );
            self.mark = None;
            self.pieces.clear();
            self.drawn = Preview::default();
            self.note = None;
        }
    }
}

impl Tool for SectionTool {
    fn id(&self) -> &'static str {
        if self.elevations() { FACADE_ID } else { ID }
    }

    fn label(&self) -> &'static str {
        if self.elevations() {
            "Görünüş çıkar"
        } else {
            "Kesit çıkar"
        }
    }

    fn cursor(&self) -> Cursor {
        if self.mark.is_none() {
            Cursor::Pick
        } else {
            Cursor::Cross
        }
    }

    fn snaps(&self) -> bool {
        self.mark.is_some()
    }

    fn point_count(&self) -> usize {
        usize::from(self.mark.is_some())
    }

    fn activate(&mut self, cx: &mut Context<'_>) -> Flow {
        // A section mark selected before the tool started is the one.
        if let [one] = cx.selection.ids()
            && matches!(cx.doc.get(*one), Some(Entity::ViewMark(v)) if v.mark.form == self.form)
        {
            self.mark = Some(*one);
            self.see(cx);
        }
        Flow::Stay
    }

    fn prompt(&self) -> Prompt {
        let step = match (self.elevations(), self.mark.is_none(), self.pieces.is_empty()) {
            (false, true, _) => "kesiti çıkarılacak kesit işaretinin çizgisine tıklayın",
            (true, true, _) => "görünüşü çıkarılacak görünüş işaretinin balonuna tıklayın",
            (false, false, true) => "kesit çıkarılmış: Enter ile yeniden çizin",
            (true, false, true) => "görünüş çıkarılmış: Enter ile yeniden çizin",
            (false, false, false) => "kesitin sol ucunun ±0,00'ını tıklayın (Enter: çizimin sağına)",
            (true, false, false) => "görünüşün sol ucunun ±0,00'ını tıklayın (Enter: çizimin sağına)",
        };
        Prompt::new(self.label(), step)
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.hover = Some(p.world);
        if self.mark.is_none() {
            let mark = self.mark_at(p.raw, cx);
            cx.selection.set_hover(mark);
            return;
        }
        if self.pieces.is_empty() && self.note.is_none() && self.problem.is_none() {
            self.see(cx);
        } else {
            self.place();
        }
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        if self.mark.is_none() {
            self.mark = self.mark_at(p.raw, cx);
            if self.mark.is_none() {
                cx.say(
                    Level::Info,
                    if self.elevations() {
                        "Burada görünüş işareti yok; planda görünüş işaretinin balonuna tıklayın."
                    } else {
                        "Burada kesit işareti yok; planda kesit çizgisine tıklayın."
                    },
                );
            }
            self.see(cx);
            return;
        }
        if self.pieces.is_empty() {
            // Drawn already: a click draws it again where it is.
            self.write(None, cx);
            return;
        }
        self.write(Some(p.world), cx);
    }

    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        if self.mark.is_some()
            && !self.pieces.is_empty()
            && let Some(p) = cx.typed_point(text, None, self.hover)
        {
            self.write(Some(p), cx);
            return true;
        }
        false
    }

    fn cancel(&mut self, cx: &mut Context<'_>) -> bool {
        if self.mark.take().is_some() {
            self.pieces.clear();
            self.drawn = Preview::default();
            self.note = None;
            self.problem = None;
            cx.selection.set_hover(None);
            return true;
        }
        false
    }

    fn undo_step(&mut self, cx: &mut Context<'_>) -> bool {
        self.cancel(cx)
    }

    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow {
        if self.mark.is_none() {
            return Flow::Exit;
        }
        self.write(None, cx);
        Flow::Stay
    }

    fn preview(&self, _format: &Format) -> Preview {
        let mut out = self.drawn.clone();
        let Some(at) = self.hover else {
            return out;
        };
        if let Some(why) = self.problem.as_ref().filter(|w| !w.is_empty()) {
            out.tag = Some(Tag {
                at,
                lines: vec![why.clone()],
            });
            out.tag_tone = Tone::Danger;
        } else if let Some(note) = &self.note {
            out.tag = Some(Tag {
                at,
                lines: vec![
                    if self.elevations() { "Görünüş" } else { "Kesit" }.into(),
                    note.clone(),
                ],
            });
        }
        out
    }
}
