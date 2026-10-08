//! Yazı: the web's `TextTool` (`apps/web/src/tools/annotateTools.ts`) on its
//! `PointInputTool` base, step for step:
//!
//! - a click (or a typed point) where the text starts opens a text field
//!   right there; the host shows it (`ViewChange::Text`) and gives back what
//!   was typed (`Tool::text_typed`): Enter adds the text and the tool waits
//!   for the next one, Esc drops the field;
//! - Yükseklik (Y) asks for the height in paper millimetres (the project's
//!   plot scale makes it metres), Açı (A) for the angle in degrees or two
//!   clicks along an edge (a direction pointing left is turned around, kept
//!   readable); both stay for as long as the app lives, and so do (docs/adr/0145
//!   §6) Hiza (H: the point of the text the click is, from its menu or its
//!   name typed together, “sağüst”), Genişlik (G: the letters' width factor),
//!   Zemin (Z: the box filled with the drawing's colour) and Artır (R: the
//!   next field opens with the last text's number one more; the last text is
//!   this run's);
//! - a locked active layer is said at the click and no field opens (the
//!   web's newest rule); Esc leaves the tool, and inside the field it only
//!   drops the text (the host's).
//!
//! The text is written through `cad.entities.create` (docs/adr/0057): the
//! active layer, one undo step (“Ekle”), “Yazı eklendi: “…”” said.

use kentos_contracts::{
    AnnotationKind, EntityGeometry, MAX_WIDTH_FACTOR, TextAlign, width_factor_ok,
};
use kentos_geometry_core::geometry::dist;
use kentos_geometry_core::text::edit::increment;
use kentos_geometry_core::tools::drawing::text_angle;
use kentos_geometry_core::tools::point_text::{js_trim, parse_number};

use crate::Vec2;
use crate::format::{Format, fixed, js_number};
use crate::log::Level;
use crate::points::{self, Taken};
use crate::prompt::{Prompt, upper_tr};
use crate::styles;
use crate::tool::{
    Context, Flow, Marker, MarkerShape, Memory, OptionChoice, Pointer, Preview, Stroke, Tag,
    TextField, Tone, Tool, ViewChange,
};

/// The text tool's id: its command is `tool.text`.
pub const ID: &str = "text";
pub const LABEL: &str = "Yazı";

/// The hover box's least height on screen, logical pixels (the web's `Math.max(8, …)`).
const LEAST_BOX_PX: f64 = 8.0;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Stage {
    /// Where the text starts.
    #[default]
    Pos,
    Height,
    Angle,
    /// Hiza: its name typed, or chosen from its menu.
    Align,
    /// Genişlik: the width factor typed.
    Width,
    /// Stil: a text style's name typed, or chosen from its menu (docs/adr/0183 §4).
    Style,
    /// The field is open at `at`.
    Typing,
}

/// The alignments in the picker's order (docs/adr/0145 §6), row by row (top,
/// middle, bottom, the baseline; left, centre, right): the value (none: the
/// left of the baseline, a text without the field), its name as the prompt
/// writes it, at the head of a menu row, and the web's icon. The web's
/// `TEXT_ALIGN_ROWS` and `textAlignName` are the same.
pub const ALIGNS: [(Option<TextAlign>, &str, &str, &str); 12] = [
    (
        Some(TextAlign::TopLeft),
        "sol üst",
        "Sol üst",
        "textAlignTopLeft",
    ),
    (
        Some(TextAlign::TopCenter),
        "orta üst",
        "Orta üst",
        "textAlignTopCenter",
    ),
    (
        Some(TextAlign::TopRight),
        "sağ üst",
        "Sağ üst",
        "textAlignTopRight",
    ),
    (
        Some(TextAlign::MiddleLeft),
        "sol orta",
        "Sol orta",
        "textAlignMiddleLeft",
    ),
    (
        Some(TextAlign::MiddleCenter),
        "orta",
        "Orta",
        "textAlignMiddleCenter",
    ),
    (
        Some(TextAlign::MiddleRight),
        "sağ orta",
        "Sağ orta",
        "textAlignMiddleRight",
    ),
    (
        Some(TextAlign::BottomLeft),
        "sol alt",
        "Sol alt",
        "textAlignBottomLeft",
    ),
    (
        Some(TextAlign::BottomCenter),
        "orta alt",
        "Orta alt",
        "textAlignBottomCenter",
    ),
    (
        Some(TextAlign::BottomRight),
        "sağ alt",
        "Sağ alt",
        "textAlignBottomRight",
    ),
    (None, "sol taban", "Sol taban", "textAlignBaselineLeft"),
    (
        Some(TextAlign::BaselineCenter),
        "orta taban",
        "Orta taban",
        "textAlignBaselineCenter",
    ),
    (
        Some(TextAlign::BaselineRight),
        "sağ taban",
        "Sağ taban",
        "textAlignBaselineRight",
    ),
];

fn align_row(
    a: Option<TextAlign>,
) -> &'static (Option<TextAlign>, &'static str, &'static str, &'static str) {
    ALIGNS.iter().find(|row| row.0 == a).unwrap_or(&ALIGNS[9])
}

/// An alignment's name, lower case: “sol taban”, “orta”, “sağ üst”.
pub fn align_name(a: Option<TextAlign>) -> &'static str {
    align_row(a).1
}

/// An alignment's name at the head of a menu row or a cell: “Sol taban”.
pub fn align_label(a: Option<TextAlign>) -> &'static str {
    align_row(a).2
}

/// An alignment's icon in the web's set (`ui/icons.ts`).
pub fn align_icon(a: Option<TextAlign>) -> &'static str {
    align_row(a).3
}

/// A name as typed, folded (the web's `foldName`): lower case, Turkish
/// letters without their marks, nothing but letters (“Sağ-üst” → “sagust”).
pub(crate) fn fold_name(s: &str) -> String {
    s.chars()
        .flat_map(|c| match c {
            'I' => vec!['ı'],
            'İ' => vec!['i'],
            _ => c.to_lowercase().collect(),
        })
        .map(|c| match c {
            'ç' => 'c',
            'ğ' => 'g',
            'ı' => 'i',
            'ö' => 'o',
            'ş' => 's',
            'ü' => 'u',
            c => c,
        })
        .filter(char::is_ascii_lowercase)
        .collect()
}

/// The alignment a typed name is (Hiza), written together or apart, with or
/// without the Turkish marks (“sağüst”, “sag-ust”; “orta” and “ortaorta”
/// are the middle); none when it names none. The web's `textAlignFromName`.
pub fn align_from_name(typed: &str) -> Option<Option<TextAlign>> {
    let folded = fold_name(typed);
    if folded == "ortaorta" {
        return Some(Some(TextAlign::MiddleCenter));
    }
    ALIGNS
        .iter()
        .find(|row| fold_name(row.1) == folded)
        .map(|row| row.0)
}

/// A width factor as Yazı's prompt and Öznitelikler write it: `+f.toFixed(4)`
/// (“0.8”, “1”).
pub fn width_factor_text(f: f64) -> String {
    trimmed(f, 4)
}

/// `açık` or `kapalı`, as the prompt says a switch.
fn on_off(on: bool) -> &'static str {
    if on { "açık" } else { "kapalı" }
}

/// The text tool.
#[derive(Clone, Debug, Default)]
pub struct Text {
    d: Taken,
    stage: Stage,
    /// Where the typed text will start.
    at: Option<Vec2>,
    /// The first of the two clicks that give the angle.
    angle_from: Option<Vec2>,
    /// The hover box's height in metres at the last move: the text's own, at
    /// least 8 px on screen.
    box_height: f64,
    /// What the session remembered and the project's units, as of the last call.
    seen: Option<(Memory, Format)>,
    /// The text styles as of the last call: a CAD project's Stil (docs/adr/0183 §4).
    styles: styles::Seen,
    /// The last text this run wrote: Artır's next field starts from it.
    last_text: Option<String>,
}

/// The active layer's lock sentence, when it is locked (the drawing tools' words).
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

/// A number as the web writes it in the prompt: `+n.toFixed(places)`.
fn trimmed(n: f64, places: usize) -> String {
    js_number(fixed(n, places).parse::<f64>().unwrap_or(n))
}

impl Text {
    pub fn new() -> Self {
        Self::default()
    }

    fn see(&mut self, cx: &Context<'_>) {
        self.seen = Some((cx.seen_memory(), cx.format()));
        self.styles = styles::Seen::text(cx);
    }

    /// The point Orto and tracking go from: the angle's first click.
    fn last(&self) -> Option<Vec2> {
        match self.stage {
            Stage::Angle => self.angle_from,
            _ => None,
        }
    }

    fn option(&mut self, key: &str, cx: &mut Context<'_>) -> bool {
        if self.stage != Stage::Pos {
            return false;
        }
        match key {
            "Y" => self.stage = Stage::Height,
            "A" => {
                self.stage = Stage::Angle;
                self.angle_from = None;
            }
            "H" => self.stage = Stage::Align,
            "G" => self.stage = Stage::Width,
            "Z" => cx.memory.text_mask = !cx.memory.text_mask,
            "R" => cx.memory.text_increment = !cx.memory.text_increment,
            "S" if styles::shown(cx.doc.settings()) => self.stage = Stage::Style,
            _ => return false,
        }
        true
    }

    /// A typed or chosen alignment: kept, and the tool waits for the click
    /// again; a word that names none is said.
    fn take_align(&mut self, typed: &str, cx: &mut Context<'_>) -> bool {
        match align_from_name(typed) {
            Some(a) => {
                cx.memory.text_align = a;
                self.stage = Stage::Pos;
            }
            None => cx.say(
                Level::Warn,
                format!(
                    "“{typed}” bir hiza adı değil. Hizayı menüden seçin ya da adını bitişik yazın: solüst, ortaüst, sağüst, solorta, orta, sağorta, solalt, ortaalt, sağalt, soltaban, ortataban, sağtaban."
                ),
            ),
        }
        true
    }

    /// A point given (the web's `accept`, then `onPoint`). Every point starts
    /// a new object: what was written before is the drawing's to undo.
    fn accept(&mut self, p: Vec2, cx: &mut Context<'_>) {
        points::echo(p, cx);
        self.d.reset();
        match self.stage {
            Stage::Angle => match self.angle_from {
                None => self.angle_from = Some(p),
                Some(from) if dist(from, p) < 1e-9 => {}
                Some(from) => {
                    // Kept readable: a direction pointing left is turned around.
                    cx.memory.text_angle = text_angle(from, p);
                    self.angle_from = None;
                    self.stage = Stage::Pos;
                }
            },
            // A locked active layer is said at the click, and no field opens.
            Stage::Pos if active_locked(cx).is_some() => {
                let line = active_locked(cx).unwrap_or_default();
                cx.say(Level::Warn, line);
            }
            Stage::Pos => {
                self.at = Some(p);
                self.stage = Stage::Typing;
                let m = *cx.memory;
                // Artır: the last text's number one more; a text that ends with
                // no number comes back as it is (docs/adr/0145 §3).
                let initial = self
                    .last_text
                    .as_deref()
                    .filter(|_| m.text_increment)
                    .map(|t| increment(t).unwrap_or_else(|| t.to_owned()));
                cx.view_changes.push(ViewChange::Text(TextField {
                    at: p,
                    height: cx.annotation_height(AnnotationKind::Text),
                    rotation: m.text_angle,
                    align: m.text_align,
                    width_factor: m.text_width_factor,
                    initial,
                    placeholder: None,
                    hint: None,
                    empty: false,
                    face: styles::text_face(cx),
                }));
            }
            Stage::Height | Stage::Align | Stage::Width | Stage::Style | Stage::Typing => {}
        }
    }

    /// The field closed: back to where the next text starts (the web's `afterTyping`).
    fn after_typing(&mut self) {
        self.at = None;
        self.stage = Stage::Pos;
    }
}

impl Tool for Text {
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
        let memory = self.seen.map_or_else(Memory::default, |(m, _)| m);
        match self.stage {
            Stage::Height => {
                Prompt::new(LABEL, "kâğıt üzerindeki yazı yüksekliğini mm olarak yazın")
            }
            Stage::Angle if self.angle_from.is_some() => {
                Prompt::new(LABEL, "doğrultunun ikinci noktasına tıklayın")
            }
            Stage::Angle => Prompt::new(
                LABEL,
                "açıyı yazın (derece) ya da doğrultu için iki noktaya tıklayın",
            ),
            Stage::Align => Prompt::new(
                LABEL,
                "hizayı seçin ya da adını bitişik yazın: sağüst, orta, soltaban …",
            )
            .option_with("Hiza", "H", align_name(memory.text_align)),
            Stage::Width => Prompt::new(
                LABEL,
                format!(
                    "genişlik çarpanını yazın (1: harflerin kendi eni; 0'dan büyük, en çok {})",
                    js_number(MAX_WIDTH_FACTOR)
                ),
            ),
            Stage::Typing => Prompt::new(
                LABEL,
                "yazıyı tıkladığınız yere yazın; Enter ekler, Esc vazgeçer",
            ),
            Stage::Style => Prompt::new(LABEL, "yazı stilini menüden seçin ya da adını yazın")
                .option_with("Stil", "S", self.styles.chosen.clone()),
            Stage::Pos => Prompt::new(LABEL, "yazının başlangıcına tıklayın")
                .option_if(self.styles.shown, "Stil", "S", self.styles.chosen.clone())
                .option_with(
                    "Yükseklik",
                    "Y",
                    format!("{} mm", js_number(memory.heights.mm(AnnotationKind::Text))),
                )
                .option_with("Açı", "A", format!("{}°", trimmed(memory.text_angle, 4)))
                .option_with("Hiza", "H", align_name(memory.text_align))
                .option_with("Genişlik", "G", width_factor_text(memory.text_width_factor))
                .option_with("Zemin", "Z", on_off(memory.text_mask))
                .option_with("Artır", "R", on_off(memory.text_increment)),
        }
    }

    fn point_count(&self) -> usize {
        0
    }

    fn activate(&mut self, cx: &mut Context<'_>) -> Flow {
        self.see(cx);
        Flow::Stay
    }

    fn snap_from(&self) -> Option<Vec2> {
        self.angle_from.or(self.at)
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        let (point, tracking) = points::constrain(self.last(), p, cx);
        self.d.tracking = tracking;
        self.d.hover = Some(point);
        self.box_height = cx
            .annotation_height(AnnotationKind::Text)
            .max(cx.view.world_length(LEAST_BOX_PX));
        self.see(cx);
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        let (point, _) = points::constrain(self.last(), p, cx);
        self.accept(point, cx);
        self.see(cx);
    }

    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        let t = js_trim(text);
        let done = if self.option(&upper_tr(t), cx) {
            true
        } else if self.stage == Stage::Align {
            self.take_align(t, cx)
        } else if self.stage == Stage::Style {
            // A name the project has none of is said; the tool waits for another.
            if styles::take_text(t, cx) {
                self.stage = Stage::Pos;
            }
            true
        } else {
            match (self.stage, parse_number(t)) {
                (Stage::Height, Some(n)) if n > 0.0 => {
                    cx.memory.heights.set(AnnotationKind::Text, Some(n));
                    self.stage = Stage::Pos;
                    true
                }
                (Stage::Height, _) => false,
                (Stage::Angle, Some(n)) => {
                    cx.memory.text_angle = n;
                    self.angle_from = None;
                    self.stage = Stage::Pos;
                    true
                }
                (Stage::Angle, None) => false,
                (Stage::Width, Some(n)) => {
                    if width_factor_ok(n) {
                        cx.memory.text_width_factor = n;
                        self.stage = Stage::Pos;
                    } else {
                        cx.say(
                            Level::Warn,
                            format!(
                                "Genişlik çarpanı 0'dan büyük, en çok {} olmalı; {t} verildi. Harflerin kendi eni için 1 yazın.",
                                js_number(MAX_WIDTH_FACTOR)
                            ),
                        );
                    }
                    true
                }
                (Stage::Width, None) => false,
                _ => match cx.typed_point(text, self.last(), self.d.hover) {
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

    /// Hiza's menu: the twelve points, row by row (docs/adr/0145 §6), while
    /// the tool waits for a click or for one.
    fn option_choices(&self, key: &str) -> Vec<OptionChoice> {
        // Stil's menu: Standart, the project's text styles and their window (docs/adr/0183 §4).
        if key == "S" && self.styles.shown && matches!(self.stage, Stage::Pos | Stage::Style) {
            return self
                .styles
                .choices(styles::TEXT_STYLES_ENTRY, styles::TEXT_STYLES);
        }
        if key != "H" || !matches!(self.stage, Stage::Pos | Stage::Align) {
            return Vec::new();
        }
        let chosen = self
            .seen
            .map_or_else(Memory::default, |(m, _)| m)
            .text_align;
        ALIGNS
            .iter()
            .map(|&(a, typed, label, icon)| OptionChoice {
                label: label.to_owned(),
                typed: typed.to_owned(),
                icon: Some(icon),
                preview: None,
                checked: a == chosen,
                command: None,
            })
            .collect()
    }

    fn choose_option(&mut self, key: &str, typed: &str, cx: &mut Context<'_>) -> bool {
        if key == "S"
            && styles::shown(cx.doc.settings())
            && matches!(self.stage, Stage::Pos | Stage::Style)
        {
            if styles::take_text(typed, cx) {
                self.stage = Stage::Pos;
            }
            self.see(cx);
            return true;
        }
        if key != "H" || !matches!(self.stage, Stage::Pos | Stage::Align) {
            return false;
        }
        let taken = self.take_align(typed, cx);
        self.see(cx);
        taken
    }

    /// The field's answer: the text to add, or none (Esc, nothing typed).
    fn text_typed(&mut self, text: Option<&str>, cx: &mut Context<'_>) {
        if self.stage != Stage::Typing {
            return;
        }
        if let (Some(p), Some(text)) = (self.at, text.map(js_trim).filter(|t| !t.is_empty())) {
            let m = *cx.memory;
            // The defaults are no fields: the left of the baseline, a factor of 1, no mask.
            let geometry = EntityGeometry::Text {
                p: points::wire(p),
                text: text.to_owned(),
                height: cx.annotation_height(AnnotationKind::Text),
                rotation: m.text_angle,
                align: m.text_align,
                width_factor: (m.text_width_factor != 1.0).then_some(m.text_width_factor),
                mask: m.text_mask,
                box_width: None,
                line_spacing: None,
                runs: Vec::new(),
                face: styles::text_face(cx),
                path: None,
            };
            if let Some(out) = points::write_objects(vec![geometry], None, cx)
                && let Some(&id) = out.ids.first()
            {
                self.d.note(id, cx);
                self.last_text = Some(text.to_owned());
                cx.say(Level::Success, format!("Yazı eklendi: “{text}”"));
            }
        }
        self.after_typing();
        self.see(cx);
    }

    /// Where the text starts: a confirm leaves; typing, it drops the field
    /// (the web's `confirm`).
    /// A style's name is words: Space types a space (docs/adr/0183 §4).
    fn takes_words(&self) -> bool {
        self.stage == Stage::Style
    }

    fn confirm(&mut self, _cx: &mut Context<'_>) -> Flow {
        match self.stage {
            Stage::Pos => Flow::Exit,
            _ => {
                self.after_typing();
                Flow::Stay
            }
        }
    }

    fn undo_step(&mut self, cx: &mut Context<'_>) -> bool {
        self.d.undo_last_made(cx)
    }

    /// The angle's line from its first click, or where the text will sit: a
    /// dashed box of its height along its angle.
    fn preview(&self, _format: &Format) -> Preview {
        let Some(h) = self.d.hover else {
            return Preview::default();
        };
        let memory = self.seen.map_or_else(Memory::default, |(m, _)| m);
        match (self.stage, self.angle_from) {
            (Stage::Angle, Some(from)) => {
                let degrees = kentos_geometry_core::geometry::angle_deg(from, h);
                Preview {
                    strokes: vec![Stroke {
                        pts: vec![from, h],
                        closed: false,
                        dash: Some([3.0, 3.0]),
                        width: 1.0,
                        tone: Tone::Accent,
                    }],
                    tag: Some(Tag {
                        at: h,
                        lines: vec![format!("Açı {}°", fixed(degrees, 2))],
                    }),
                    ..Preview::default()
                }
            }
            // Four heights wide times the width factor, placed about the
            // pointer as the alignment says (docs/adr/0145), the pointer's point marked.
            (Stage::Pos, _) => {
                let a = memory.text_angle.to_radians();
                let (dx, dy) = (a.cos(), a.sin());
                let height = self.box_height;
                let width = height * 4.0 * memory.text_width_factor;
                let (along, up) = memory
                    .text_align
                    .map_or((0.0, 0.0), |a| (a.along(), a.up()));
                // A point of the box, `x` along its baseline and `y` up from it, counted from its start.
                let (x0, y0) = (-along * width, -up * height);
                let at = |x: f64, y: f64| {
                    let (x, y) = (x0 + x, y0 + y);
                    Vec2::new(h.x + dx * x - dy * y, h.y + dy * x + dx * y)
                };
                Preview {
                    strokes: vec![Stroke {
                        pts: vec![
                            at(0.0, 0.0),
                            at(width, 0.0),
                            at(width, height),
                            at(0.0, height),
                        ],
                        closed: true,
                        dash: Some([3.0, 3.0]),
                        width: 1.0,
                        tone: Tone::Accent,
                    }],
                    markers: vec![Marker {
                        at: h,
                        shape: MarkerShape::Ring(2.5),
                        tone: Tone::Accent,
                    }],
                    ..Preview::default()
                }
            }
            _ => Preview::default(),
        }
    }
}
