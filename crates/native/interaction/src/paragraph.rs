//! Çok satırlı yazı (docs/adr/0182 §4): the web's `ParagraphTextTool`
//! (`apps/web/src/tools/paragraphTool.ts`), step for step:
//!
//! - two corners give the box (the core's `corner_box`): its width along the
//!   text's angle (Yazı's Açı) and its top left corner, which the text hangs
//!   from, its alignment the top's left; two clicks on one point give no box
//!   (the lines end only at their breaks);
//! - the host opens the paragraph editor at the box (`ViewChange::Paragraph`)
//!   and gives back the text and its letter formats (`Tool::paragraph_typed`):
//!   Tamam writes it through `cad.entities.create` (one step, “Ekle”), its ends'
//!   white space left out; Vazgeç drops it; the tool waits for the next box;
//! - Yükseklik (Y) and Açı (A) are Yazı's, Zemin (Z) Yazı's mask; Satır aralığı
//!   (S) the lines' spacing, from 0.25 to 4, kept as long as the app lives;
//! - a locked active layer is said at the first corner and nothing opens.

use kentos_contracts::{
    EntityGeometry, MAX_LINE_SPACING, MIN_LINE_SPACING, TextAlign, TextRun, TextScript,
};
use kentos_geometry_core::text::paragraph::{Run, Script, corner_box, retext};
use kentos_geometry_core::tools::point_text::{js_trim, parse_number};

use crate::Vec2;
use crate::format::{Format, fixed, js_number};
use crate::log::Level;
use crate::points::{self, Taken};
use crate::prompt::{Prompt, upper_tr};
use crate::styles;
use crate::tool::{
    Context, Flow, Memory, OptionChoice, ParagraphField, Pointer, Preview, Stroke, Tag, Tone, Tool,
    ViewChange,
};

/// The tool's id: its command is `tool.mtext`.
pub const ID: &str = "mtext";
pub const LABEL: &str = "Çok satırlı yazı";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Stage {
    /// The box's first corner.
    #[default]
    First,
    /// Its opposite corner.
    Second,
    Height,
    Angle,
    Spacing,
    /// Stil: a text style's name typed, or chosen from its menu (docs/adr/0183 §4).
    Style,
    /// The editor is open at `at`.
    Typing,
}

/// The tool.
#[derive(Clone, Debug, Default)]
pub struct ParagraphText {
    d: Taken,
    stage: Stage,
    first: Option<Vec2>,
    /// The editor's box: the text's point and width.
    at: Option<(Vec2, Option<f64>)>,
    seen: Option<(Memory, Format)>,
    /// The text styles as of the last call: a CAD project's Stil (docs/adr/0183 §4).
    styles: styles::Seen,
}

/// Paper millimetres as metres at the project's plot scale (Yazı's).
fn paper(mm: f64, cx: &Context<'_>) -> f64 {
    mm / 1000.0 * cx.doc.settings().plot_scale
}

/// The active layer's lock sentence, when it is locked (Yazı's words).
fn active_locked(cx: &Context<'_>) -> Option<String> {
    let layers = cx.doc.layers();
    let id = layers.active();
    layers.is_locked(id).then(|| {
        let name = layers.get(id).map_or(id, |n| n.name.as_str());
        format!(
            "“{name}” katmanı kilitli. Kilidi Katmanlar panelinden açın ya da başka bir katmanı etkinleştirin."
        )
    })
}

/// `açık` or `kapalı`, as the prompt says a switch.
fn on_off(on: bool) -> &'static str {
    if on { "açık" } else { "kapalı" }
}

/// A number as the prompt writes it: `+n.toFixed(places)`.
fn trimmed(n: f64, places: usize) -> String {
    js_number(fixed(n, places).parse::<f64>().unwrap_or(n))
}

/// The contract's runs as the core takes them.
fn core_runs(runs: &[TextRun]) -> Vec<Run> {
    runs.iter()
        .map(|r| Run {
            start: r.start,
            end: r.end,
            bold: r.bold,
            italic: r.italic,
            underline: r.underline,
            script: r.script.map(|s| match s {
                TextScript::Super => Script::Super,
                TextScript::Sub => Script::Sub,
            }),
            color: r.color.clone(),
        })
        .collect()
}

/// The core's runs as the contract writes them.
fn contract_runs(runs: Vec<Run>) -> Vec<TextRun> {
    runs.into_iter()
        .map(|r| TextRun {
            start: r.start,
            end: r.end,
            bold: r.bold,
            italic: r.italic,
            underline: r.underline,
            script: r.script.map(|s| match s {
                Script::Super => TextScript::Super,
                Script::Sub => TextScript::Sub,
            }),
            color: r.color,
        })
        .collect()
}

/// A typed text without the white space at its ends, its runs following its letters.
pub fn trimmed_paragraph(text: &str, runs: &[TextRun]) -> (String, Vec<TextRun>) {
    let kept = text.trim_matches(|c: char| c.is_whitespace()).to_owned();
    if kept == text {
        return (kept, runs.to_vec());
    }
    let after = contract_runs(retext(&core_runs(runs), text, &kept));
    (kept, after)
}

impl ParagraphText {
    pub fn new() -> Self {
        Self::default()
    }

    fn see(&mut self, cx: &Context<'_>) {
        self.seen = Some((*cx.memory, cx.format()));
        self.styles = styles::Seen::text(cx);
    }

    fn option(&mut self, key: &str, cx: &mut Context<'_>) -> bool {
        if self.stage != Stage::First {
            return false;
        }
        match key {
            "Y" => self.stage = Stage::Height,
            "A" => self.stage = Stage::Angle,
            // Satır aralığı's R: S is Stil, as in Yazı (docs/adr/0183 §4).
            "R" => self.stage = Stage::Spacing,
            "S" if styles::shown(cx.doc.settings()) => self.stage = Stage::Style,
            "Z" => cx.memory.text_mask = !cx.memory.text_mask,
            _ => return false,
        }
        true
    }

    /// A point given: the first corner, then the opposite one (the editor opens).
    fn accept(&mut self, p: Vec2, cx: &mut Context<'_>) {
        points::echo(p, cx);
        self.d.reset();
        match (self.stage, self.first) {
            (Stage::First, _) if active_locked(cx).is_some() => {
                let line = active_locked(cx).unwrap_or_default();
                cx.say(Level::Warn, line);
            }
            (Stage::First, _) => {
                self.first = Some(p);
                self.stage = Stage::Second;
            }
            (Stage::Second, Some(first)) => {
                let m = *cx.memory;
                let (at, width) = corner_box(first, p, m.text_angle);
                self.at = Some((at, width));
                self.stage = Stage::Typing;
                cx.view_changes.push(ViewChange::Paragraph(ParagraphField {
                    at,
                    height: paper(m.text_height_mm, cx),
                    rotation: m.text_angle,
                    box_width: width,
                    line_spacing: (m.paragraph_spacing != 1.0).then_some(m.paragraph_spacing),
                    mask: m.text_mask,
                    face: styles::text_face(cx),
                    width_factor: styles::text_width_factor(cx),
                }));
            }
            _ => {}
        }
    }

    /// The editor closed, or the box was dropped: back to the first corner.
    fn restart(&mut self) {
        self.first = None;
        self.at = None;
        self.stage = Stage::First;
    }
}

impl Tool for ParagraphText {
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
        let memory = self.seen.map_or_else(Memory::default, |(m, _)| m);
        match self.stage {
            Stage::Height => {
                Prompt::new(LABEL, "kâğıt üzerindeki yazı yüksekliğini mm olarak yazın")
            }
            Stage::Angle => Prompt::new(LABEL, "yazının açısını derece olarak yazın"),
            Stage::Spacing => Prompt::new(
                LABEL,
                format!(
                    "satır aralığını yazın (1: yüksekliğin 5/3'ü; {} ile {} arası)",
                    js_number(MIN_LINE_SPACING),
                    js_number(MAX_LINE_SPACING)
                ),
            ),
            Stage::Second => Prompt::new(LABEL, "kutunun karşı köşesine tıklayın"),
            Stage::Style => Prompt::new(LABEL, "yazı stilini menüden seçin ya da adını yazın")
                .option_with("Stil", "S", self.styles.chosen.clone()),
            Stage::Typing => Prompt::new(
                LABEL,
                "yazıyı kutuya yazın; Enter yeni satır, Ctrl+Enter ya da Tamam ekler, Esc vazgeçer",
            ),
            Stage::First => Prompt::new(LABEL, "yazı kutusunun ilk köşesine tıklayın")
                .option_if(self.styles.shown, "Stil", "S", self.styles.chosen.clone())
                .option_with(
                    "Yükseklik",
                    "Y",
                    format!("{} mm", js_number(memory.text_height_mm)),
                )
                .option_with("Açı", "A", format!("{}°", trimmed(memory.text_angle, 4)))
                .option_with("Satır aralığı", "R", trimmed(memory.paragraph_spacing, 4))
                .option_with("Zemin", "Z", on_off(memory.text_mask)),
        }
    }

    /// The box's first corner while the opposite one is awaited.
    fn point_count(&self) -> usize {
        usize::from(self.stage == Stage::Second)
    }

    fn activate(&mut self, cx: &mut Context<'_>) -> Flow {
        self.see(cx);
        Flow::Stay
    }

    fn snap_from(&self) -> Option<Vec2> {
        self.first
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        let (point, tracking) = points::constrain(self.first, p, cx);
        self.d.tracking = tracking;
        self.d.hover = Some(point);
        self.see(cx);
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        let (point, _) = points::constrain(self.first, p, cx);
        self.accept(point, cx);
        self.see(cx);
    }

    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        let t = js_trim(text);
        let done = if self.option(&upper_tr(t), cx) {
            true
        } else if self.stage == Stage::Style {
            // A name the project has none of is said; the tool waits for another.
            if styles::take_text(t, cx) {
                self.stage = Stage::First;
            }
            true
        } else {
            match (self.stage, parse_number(t)) {
                (Stage::Height, Some(n)) if n > 0.0 => {
                    cx.memory.text_height_mm = n;
                    self.stage = Stage::First;
                    true
                }
                (Stage::Angle, Some(n)) => {
                    cx.memory.text_angle = n;
                    self.stage = Stage::First;
                    true
                }
                (Stage::Spacing, Some(n)) => {
                    if (MIN_LINE_SPACING..=MAX_LINE_SPACING).contains(&n) {
                        cx.memory.paragraph_spacing = n;
                        self.stage = Stage::First;
                    } else {
                        cx.say(
                            Level::Warn,
                            format!(
                                "Satır aralığı {} ile {} arasında olmalı; {t} verildi. Yüksekliğin 5/3'ü için 1 yazın.",
                                js_number(MIN_LINE_SPACING),
                                js_number(MAX_LINE_SPACING)
                            ),
                        );
                    }
                    true
                }
                (Stage::Height | Stage::Angle | Stage::Spacing, _) => false,
                _ => match cx.typed_point(text, self.first, self.d.hover) {
                    Some(p) => {
                        self.accept(p, cx);
                        true
                    }
                    None => false,
                },
            }
        };
        self.see(cx);
        done
    }

    /// Stil's menu: Standart, the project's text styles and their window (docs/adr/0183 §4).
    fn option_choices(&self, key: &str) -> Vec<OptionChoice> {
        if key == "S" && self.styles.shown && matches!(self.stage, Stage::First | Stage::Style) {
            return self
                .styles
                .choices(styles::TEXT_STYLES_ENTRY, styles::TEXT_STYLES);
        }
        Vec::new()
    }

    fn choose_option(&mut self, key: &str, typed: &str, cx: &mut Context<'_>) -> bool {
        if key != "S"
            || !styles::shown(cx.doc.settings())
            || !matches!(self.stage, Stage::First | Stage::Style)
        {
            return false;
        }
        if styles::take_text(typed, cx) {
            self.stage = Stage::First;
        }
        self.see(cx);
        true
    }

    /// The editor's answer: the text and its formats to add, or none (Vazgeç).
    fn paragraph_typed(&mut self, typed: Option<(&str, &[TextRun])>, cx: &mut Context<'_>) {
        if self.stage != Stage::Typing {
            return;
        }
        if let (Some((at, width)), Some((text, runs))) = (self.at, typed) {
            let (text, runs) = trimmed_paragraph(text, runs);
            if !text.is_empty() {
                let m = *cx.memory;
                let geometry = EntityGeometry::Text {
                    p: points::wire(at),
                    text: text.clone(),
                    height: paper(m.text_height_mm, cx),
                    rotation: m.text_angle,
                    align: Some(TextAlign::TopLeft),
                    width_factor: styles::text_width_factor(cx),
                    mask: m.text_mask,
                    box_width: width,
                    line_spacing: (m.paragraph_spacing != 1.0).then_some(m.paragraph_spacing),
                    runs,
                    face: styles::text_face(cx),
                };
                if let Some(out) = points::write_objects(vec![geometry], None, cx)
                    && let Some(&id) = out.ids.first()
                {
                    self.d.note(id, cx);
                    let first = text.lines().next().unwrap_or_default();
                    let more = if text.contains('\n') { " …" } else { "" };
                    cx.say(
                        Level::Success,
                        format!("Çok satırlı yazı eklendi: “{first}{more}”"),
                    );
                }
            }
        }
        self.restart();
        self.see(cx);
    }

    /// At the first corner a confirm leaves; past it, it drops the box.
    /// A style's name is words: Space types a space (docs/adr/0183 §4).
    fn takes_words(&self) -> bool {
        self.stage == Stage::Style
    }

    fn confirm(&mut self, _cx: &mut Context<'_>) -> Flow {
        match self.stage {
            Stage::First => Flow::Exit,
            _ => {
                self.restart();
                Flow::Stay
            }
        }
    }

    /// Esc past the first corner drops the box; at it the session leaves the tool.
    fn cancel(&mut self, _cx: &mut Context<'_>) -> bool {
        if self.stage == Stage::First {
            return false;
        }
        self.restart();
        true
    }

    fn undo_step(&mut self, cx: &mut Context<'_>) -> bool {
        self.d.undo_last_made(cx)
    }

    /// The box from its first corner to the pointer, dashed, its width beside it.
    fn preview(&self, format: &Format) -> Preview {
        let (Some(first), Some(h), Stage::Second) = (self.first, self.d.hover, self.stage) else {
            return Preview::default();
        };
        let memory = self.seen.map_or_else(Memory::default, |(m, _)| m);
        let r = memory.text_angle.to_radians();
        let (c, s) = (r.cos(), r.sin());
        let (corner, width) = corner_box(first, h, memory.text_angle);
        // The box's depth: from its top down to the lower of the two corners.
        let up = |p: Vec2| -p.x * s + p.y * c;
        let depth = (up(first) - up(h)).abs();
        let w = width.unwrap_or(0.0);
        let at = |x: f64, y: f64| Vec2::new(corner.x + c * x + s * y, corner.y + s * x - c * y);
        Preview {
            strokes: vec![Stroke {
                pts: vec![at(0.0, 0.0), at(w, 0.0), at(w, depth), at(0.0, depth)],
                closed: true,
                dash: Some([4.0, 3.0]),
                width: 1.0,
                tone: Tone::Accent,
            }],
            tag: Some(Tag {
                at: h,
                lines: vec![match width {
                    Some(w) => format!("Kutu genişliği {}", format.length(w)),
                    None => "Kutusuz: satırlar yalnız satır sonlarında biter".to_owned(),
                }],
            }),
            ..Preview::default()
        }
    }
}
