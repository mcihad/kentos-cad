//! Oda iç görünüşü (docs/adr/0799, ADV-06f): a derived drawing, not a mark. A click inside a space picks the room; the
//! next click places the first side's lower left corner (Enter: beside the room) and `cad.elevations.edit` derives the
//! elevations from the model on a layer of their own, one undo step; its plan is the preview drawn at the cursor. The
//! tool says plainly what the model does not hold (the plan's leftOut). Yanlar (Y): every straight side, or the letters
//! typed (A,C); Aralık (R): the space between sides.
//!
//! A section or elevation mark (Kesit, Görünüş) only points at a view: it never claims a drawing was made. This tool
//! makes the drawing.

use kentos_commands::{
    CommandResult, ElevationLeft, ElevationsEdit, ElevationsOperation, Entity, TextAlign,
};
use kentos_domain::Slot;
use kentos_geometry_core::tools::point_text::js_trim;
use kentos_native_application::geometry::shape;
use kentos_native_application::{ExecutionContext, elevations_edit};

use crate::Vec2;
use crate::edge::Outline;
use crate::format::Format;
use crate::log::Level;
use crate::points;
use crate::prompt::{Prompt, upper_tr};
use crate::tool::{Context, Cursor, Flow, Pointer, Preview, Tag, TextGhost, Tone, Tool};

pub const ID: &str = "roomElevation";

/// What the tool waits for after its key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Ask {
    Sides,
    Gap,
}

#[derive(Clone, Debug)]
pub struct RoomElevationTool {
    space: Option<Slot>,
    hover: Option<Vec2>,
    /// The sides asked for, by number; empty: all.
    sides: Vec<u32>,
    gap: f64,
    ask: Option<Ask>,
    drawn: Preview,
    note: Option<String>,
    problem: Option<String>,
}

impl Default for RoomElevationTool {
    fn default() -> Self {
        Self::new()
    }
}

/// The side's number of a letter: A 0, B 1 … Z 25, AA 26.
fn side_of(word: &str) -> Option<u32> {
    let mut n: u64 = 0;
    if word.is_empty() {
        return None;
    }
    for c in word.chars() {
        if !c.is_ascii_uppercase() {
            return None;
        }
        n = n * 26 + (c as u64 - 'A' as u64 + 1);
    }
    u32::try_from(n - 1).ok()
}

/// The words for what the model does not hold.
fn left_words(why: ElevationLeft) -> &'static str {
    match why {
        ElevationLeft::CurvedSide => "yaylı yan",
        ElevationLeft::Hole => "mahal boşluğu",
        ElevationLeft::Column => "kolon",
        ElevationLeft::Stair => "merdiven",
        ElevationLeft::Ramp => "rampa",
        ElevationLeft::Fixture => "donatı",
        ElevationLeft::Flue => "baca",
        ElevationLeft::BrokenOpening => "kopuk açıklık",
        ElevationLeft::HeadUnknown => "lentosuz açıklık (kesikli)",
    }
}

fn core_align(a: Option<TextAlign>) -> kentos_geometry_core::text::TextAlign {
    use kentos_geometry_core::text::TextAlign as C;
    match a {
        Some(TextAlign::TopCenter) => C::TopCenter,
        _ => C::MiddleCenter,
    }
}

impl RoomElevationTool {
    pub fn new() -> Self {
        Self {
            space: None,
            hover: None,
            sides: Vec::new(),
            gap: kentos_native_application::elevations::GAP,
            ask: None,
            drawn: Preview::default(),
            note: None,
            problem: None,
        }
    }

    /// The space under the point: the smallest whose outer rings hold it, on a shown layer.
    fn space_at(at: Vec2, cx: &Context<'_>) -> Option<Slot> {
        let mut best: Option<(f64, Slot)> = None;
        for e in cx.doc.entities() {
            let Entity::Space(s) = e else { continue };
            if !cx.doc.layers().is_visible(&e.base().layer_id) {
                continue;
            }
            let mut odd = false;
            for part in &s.space.parts {
                let r = &part.outer.pts;
                let n = r.len();
                for i in 0..n {
                    let (a, b) = (r[i], r[(i + 1) % n]);
                    if (a.y > at.y) != (b.y > at.y)
                        && at.x < a.x + (at.y - a.y) * (b.x - a.x) / (b.y - a.y)
                    {
                        odd = !odd;
                    }
                }
            }
            if odd {
                let area = kentos_native_application::spaces::area_of(&s.space.parts);
                if best.is_none_or(|(b, _)| area < b) {
                    best = Some((area, Slot(e.base().id)));
                }
            }
        }
        best.map(|(_, s)| s)
    }

    fn input_at(&self, at: Option<Vec2>, cx: &Context<'_>) -> Option<ElevationsEdit> {
        let space = cx.doc.uid(self.space?)?.to_string();
        Some(ElevationsEdit {
            operation: ElevationsOperation::Derive {
                space,
                sides: self.sides.clone(),
                at: at.map(|p| kentos_commands::Vec2 { x: p.x, y: p.y }),
                gap: Some(self.gap),
                name: None,
            },
            expected_revision: None,
        })
    }

    fn see(&mut self, cx: &Context<'_>) {
        self.drawn = Preview::default();
        self.problem = None;
        self.note = None;
        let Some(input) = self.input_at(self.hover, cx) else {
            return;
        };
        match elevations_edit::plan_in(cx.doc, &input) {
            CommandResult::Completed { output, .. } => {
                for e in &output.drawn {
                    match e {
                        Entity::Text(t) => self.drawn.texts.push(TextGhost {
                            p: Vec2::new(t.p.x, t.p.y),
                            text: t.text.clone(),
                            height: t.height,
                            rotation: 0.0,
                            align: Some(core_align(t.align)),
                            mask: false,
                            face: Default::default(),
                            width_factor: 1.0,
                        }),
                        Entity::Dimension(_) => {}
                        other => self
                            .drawn
                            .strokes
                            .extend(Outline::of(&shape(other), None, 1.5, Tone::Accent).strokes),
                    }
                }
                let mut words: Vec<&str> = Vec::new();
                for l in &output.left_out {
                    let w = left_words(l.why);
                    if !words.contains(&w) {
                        words.push(w);
                    }
                }
                let letters: Vec<&str> = output.sides.iter().map(|s| s.letter.as_str()).collect();
                let mut note = format!("{} yan: {}", output.sides.len(), letters.join(", "));
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
    }

    fn write(&mut self, at: Option<Vec2>, cx: &mut Context<'_>) {
        let Some(input) = self.input_at(at, cx) else {
            return;
        };
        let result = elevations_edit::execute(&mut ExecutionContext::new(cx.doc), input);
        if let Some(out) = points::written(result, cx) {
            cx.say(
                Level::Success,
                format!(
                    "“{}” katmanına {} yan çizildi; model değişince Öznitelikler “eski” der, Yeniden üret günceller.",
                    out.name,
                    out.sides.len()
                ),
            );
            self.space = None;
            self.drawn = Preview::default();
            self.note = None;
        }
    }

    fn sides_text(&self) -> String {
        if self.sides.is_empty() {
            "Hepsi".into()
        } else {
            self.sides
                .iter()
                .map(|k| kentos_native_application::elevations::letter(*k))
                .collect::<Vec<_>>()
                .join(",")
        }
    }
}

impl Tool for RoomElevationTool {
    fn id(&self) -> &'static str {
        ID
    }

    fn label(&self) -> &'static str {
        "Oda iç görünüşü"
    }

    fn cursor(&self) -> Cursor {
        if self.space.is_none() {
            Cursor::Pick
        } else {
            Cursor::Cross
        }
    }

    fn snaps(&self) -> bool {
        self.space.is_some()
    }

    fn point_count(&self) -> usize {
        usize::from(self.space.is_some())
    }

    fn prompt(&self) -> Prompt {
        match self.ask {
            Some(Ask::Sides) => {
                return Prompt::new("Oda iç görünüşü", "yanların harflerini yazın (A,C; boş: hepsi)")
                    .option("Geri", "G");
            }
            Some(Ask::Gap) => {
                return Prompt::new("Oda iç görünüşü", "yanların arasını yazın (1.0)").option("Geri", "G");
            }
            None => {}
        }
        let step = if self.space.is_none() {
            "iç görünüşü üretilecek mahalin içine tıklayın"
        } else {
            "ilk yanın sol alt köşesini tıklayın (Enter: mahalin yanına)"
        };
        Prompt::new("Oda iç görünüşü", step)
            .option_with("Yanlar", "Y", self.sides_text())
            .option_with("Aralık", "R", format!("{} m", self.gap))
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.hover = Some(p.world);
        if self.space.is_none() {
            let space = Self::space_at(p.raw, cx);
            cx.selection.set_hover(space);
            return;
        }
        self.see(cx);
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        if self.space.is_none() {
            self.space = Self::space_at(p.raw, cx);
            if self.space.is_none() {
                cx.say(Level::Info, "Burada mahal yok; bir mahalin içine tıklayın.");
            }
            self.see(cx);
            return;
        }
        self.write(Some(p.world), cx);
    }

    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        let key = upper_tr(js_trim(text));
        match self.ask.take() {
            Some(Ask::Sides) => {
                if key != "G" {
                    let words: Vec<&str> = key
                        .split([',', ' ', ';'])
                        .filter(|w| !w.is_empty())
                        .collect();
                    let read: Option<Vec<u32>> = words.iter().map(|w| side_of(w)).collect();
                    match read {
                        Some(mut s) => {
                            s.sort_unstable();
                            s.dedup();
                            self.sides = s;
                        }
                        None => cx.say(Level::Warn, "Yanlar okunamadı: A,C gibi harfler yazın."),
                    }
                }
                self.see(cx);
                return true;
            }
            Some(Ask::Gap) => {
                if key != "G" {
                    match cx.typed_length(js_trim(text)) {
                        Some(g) if g > 0.0 => self.gap = g,
                        _ => cx.say(Level::Warn, "Aralık okunamadı: sıfırdan büyük bir uzunluk yazın."),
                    }
                }
                self.see(cx);
                return true;
            }
            None => {}
        }
        match key.as_str() {
            "Y" => self.ask = Some(Ask::Sides),
            "R" => self.ask = Some(Ask::Gap),
            _ => {
                if self.space.is_some()
                    && let Some(p) = cx.typed_point(text, None, self.hover)
                {
                    self.write(Some(p), cx);
                    return true;
                }
                return false;
            }
        }
        true
    }

    fn takes_words(&self) -> bool {
        self.ask.is_some()
    }

    fn cancel(&mut self, cx: &mut Context<'_>) -> bool {
        if self.ask.take().is_some() {
            return true;
        }
        if self.space.take().is_some() {
            self.see(cx);
            return true;
        }
        false
    }

    fn undo_step(&mut self, cx: &mut Context<'_>) -> bool {
        self.cancel(cx)
    }

    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow {
        if self.space.is_none() {
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
                lines: vec!["Türetilmiş çizim".into(), note.clone()],
            });
        }
        out
    }
}
