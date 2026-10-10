//! Eğri boyunca yazı (docs/adr/0196 §4): the web's `TextAlongTool` and
//! `TextCurveTool` (`apps/web/src/tools/textAlongTool.ts`).
//!
//! - **Eğri boyunca yazı** (`textAlong`): a click picks the curve (a line, an
//!   arc, a circle, a polyline's part, an area's ring or hole, an ellipse, a
//!   spline); the next places the text on it, its letters following the
//!   cursor along the curve (the last text written, “Yazı” at first); a text
//!   field opens there, and Enter writes the text along the piece of the
//!   curve its letters take (the core's `text::along::piece`), through
//!   `cad.entities.create` (step “Eğri boyunca yazı”). Yükseklik (Y) is
//!   Yazı's; Hiza (H: Başı, Ortası, Sonu: what of the text the click is) and
//!   Konum (K: Üstünde, Ortasında, Altında: the letters over, on or under
//!   the curve) stay for as long as the app lives; Stil (S) in a CAD project.
//! - **Yazıyı eğriye oturt** (`textCurve`), on the modify tools' base, its
//!   methods' letters: Eğriye oturt (O) puts the selected texts on the curve
//!   clicked, each where its box's middle falls on it (`textPath`);
//!   Doğrultuya döndür (D) turns the selected straight texts to the
//!   clicked edge's direction, readable, each about its own point
//!   (`textTurn`); Düzleştir (Z) makes the selected curved texts straight at
//!   once (`textStraighten`). Through `cad.entities.edit`, one step each.

use kentos_contracts::{
    AnnotationKind, EditOperation, Entity, EntityEdit, TextAlign, TextEntity, TextPath,
};
use kentos_domain::Slot;
use kentos_geometry_core::entity::{Shape, TextPlace};
use kentos_geometry_core::text::Font;
use kentos_geometry_core::text::along::{Along, Curve, Letter, piece, readable_turn};
use kentos_geometry_core::tools::point_text::{js_trim, parse_number};
use kentos_native_application::geometry::{core_face, drawing_font, edit_geometry, shape};

use crate::Vec2;
use crate::edge;
use crate::format::{Format, js_number};
use crate::log::Level;
use crate::modify::{Modify, Stages};
use crate::points;
use crate::prompt::{Prompt, upper_tr};
use crate::styles;
use crate::tool::{
    Context, Cursor, Flow, Marker, MarkerShape, Memory, OptionChoice, Pointer, Preview, Stroke,
    TextField, TextGhost, Tone, Tool, ViewChange,
};

/// Eğri boyunca yazı's id: its command is `tool.textAlong`.
pub const ID: &str = "textAlong";
pub const LABEL: &str = "Eğri boyunca yazı";
/// Yazıyı eğriye oturt's id: its command is `tool.textCurve`.
pub const CURVE_ID: &str = "textCurve";
pub const CURVE_LABEL: &str = "Yazıyı eğriye oturt";

/// What the preview writes before anything was typed.
const SAMPLE: &str = "Yazı";
/// At most this many texts are previewed at once (Eğriye oturt, Doğrultuya döndür).
const MAX_PREVIEWED: usize = 50;

/// The kinds a text may follow (§4): their paths, by the core's `paths_of`.
const CURVE_KINDS: [&str; 7] = [
    "line", "arc", "circle", "polyline", "polygon", "ellipse", "spline",
];

/// Hiza's choices: what of the text the click is, its share along, its word and letter.
pub const SHARES: [(f64, &str, &str); 3] = [
    (0.0, "Başı", "başı"),
    (0.5, "Ortası", "ortası"),
    (1.0, "Sonu", "sonu"),
];
/// Konum's choices: the letters over, on or under the curve.
pub const SIDES: [(Side, &str, &str); 3] = [
    (Side::Over, "Üstünde", "üstünde"),
    (Side::On, "Ortasında", "ortasında"),
    (Side::Under, "Altında", "altında"),
];

/// Where the letters stand against the curve (§4): the alignment's share up.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    /// Over it: the letters' bottom on it.
    Over,
    /// On it: their middle.
    On,
    /// Under it: their top.
    Under,
}

/// The alignment a share along and a side make.
pub fn align_of(share: f64, side: Side) -> TextAlign {
    let left = share < 0.25;
    let right = share > 0.75;
    match (side, left, right) {
        (Side::Over, true, _) => TextAlign::BottomLeft,
        (Side::Over, _, true) => TextAlign::BottomRight,
        (Side::Over, _, _) => TextAlign::BottomCenter,
        (Side::On, true, _) => TextAlign::MiddleLeft,
        (Side::On, _, true) => TextAlign::MiddleRight,
        (Side::On, _, _) => TextAlign::MiddleCenter,
        (Side::Under, true, _) => TextAlign::TopLeft,
        (Side::Under, _, true) => TextAlign::TopRight,
        (Side::Under, _, _) => TextAlign::TopCenter,
    }
}

/// An alignment's share along and side (none or a baseline: over the curve).
pub fn split(a: TextAlign) -> (f64, Side) {
    let side = match a {
        TextAlign::MiddleLeft | TextAlign::MiddleCenter | TextAlign::MiddleRight => Side::On,
        TextAlign::TopLeft | TextAlign::TopCenter | TextAlign::TopRight => Side::Under,
        _ => Side::Over,
    };
    (a.along(), side)
}

/// A core curve as the contract's.
fn wire_curve(c: &Curve) -> TextPath {
    TextPath {
        pts: points::wire_all(&c.pts),
        bulges: c.bulges.clone(),
    }
}

/// The letters' length of `text` at `height`, in `face` or the drawing's
/// typeface (§2.2): what the curve's piece is cut to.
fn letters_length(
    text: &str,
    height: f64,
    width_factor: f64,
    face: &kentos_contracts::TextFace,
    font: Font,
) -> f64 {
    let core = core_face(face);
    let straight = Curve {
        pts: vec![Vec2::new(1.0, 0.0)],
        bulges: None,
    };
    Along {
        p: Vec2::new(0.0, 0.0),
        rotation: 0.0,
        curve: &straight,
        text,
        runs: &[],
        height,
        width_factor,
        align: None,
        font: core.font_or(font),
        bold: core.is_bold(),
        lean: core.lean(),
    }
    .advances()
    .iter()
    .sum()
}

/// A text's letters, faint, one ghost each (the web's `drawTextGhost`).
fn ghosts(t: &TextEntity, font: Font, out: &mut Vec<TextGhost>) {
    let s = shape(&Entity::Text(t.clone()));
    let Some(along) = TextPlace::of(&s).and_then(|p| p.along(font)) else {
        return;
    };
    let letters = along.letters();
    for (l, ch) in letters.iter().zip(t.text.chars()) {
        if ch.is_whitespace() {
            continue;
        }
        out.push(letter_ghost(l, ch, t));
    }
}

fn letter_ghost(l: &Letter, ch: char, t: &TextEntity) -> TextGhost {
    TextGhost {
        p: l.at,
        text: ch.to_string(),
        height: t.height,
        rotation: l.turn,
        align: None,
        mask: false,
        face: t.face.clone(),
        width_factor: t.width_factor.unwrap_or(1.0),
    }
}

/// The curve's object under the cursor: a visible one of the kinds a text follows.
fn curve_under(p: &Pointer, cx: &Context<'_>) -> Option<Slot> {
    cx.spatial.pick_edge(p.raw, cx.pick_tolerance(), |s| {
        cx.doc
            .get(s)
            .is_some_and(|e| CURVE_KINDS.contains(&e.kind()))
    })
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

// ── Eğri boyunca yazı ───────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Stage {
    /// The curve to follow.
    #[default]
    Curve,
    /// Where on it the text stands.
    Place,
    /// Yükseklik typed.
    Height,
    /// Stil chosen or typed.
    Style,
    /// The field is open.
    Typing,
}

/// Eğri boyunca yazı.
#[derive(Clone, Debug, Default)]
pub struct TextAlong {
    stage: Stage,
    /// The curve picked: its object and its shape.
    curve: Option<(Slot, Shape)>,
    /// The object under the cursor while a curve is to be picked.
    under: Option<Slot>,
    /// The click on the curve the field opened at.
    at: Option<Vec2>,
    /// The letters the cursor shows: the text to come where it would stand.
    shown: Option<TextEntity>,
    /// The last text written: what the preview writes.
    last_text: Option<String>,
    seen: Option<(Memory, Format)>,
    styles: styles::Seen,
    font: Font,
}

impl TextAlong {
    pub fn new() -> Self {
        Self::default()
    }

    fn see(&mut self, cx: &Context<'_>) {
        self.seen = Some((cx.seen_memory(), cx.format()));
        self.styles = styles::Seen::text(cx);
        self.font = drawing_font(cx.doc.settings().drawing_font);
    }

    fn memory(&self) -> Memory {
        self.seen.map_or_else(Memory::default, |(m, _)| m)
    }

    /// The text `words` where the click `at` puts it on the curve: its own
    /// piece, alignment and face; none off any piece.
    fn placed(&self, words: &str, at: Vec2, cx: &Context<'_>) -> Option<TextEntity> {
        let (_, curve) = self.curve.as_ref()?;
        let m = *cx.memory;
        let height = cx.annotation_height(AnnotationKind::Text);
        let face = styles::text_face(cx);
        let length = letters_length(words, height, 1.0, &face, self.font);
        let (share, _) = split(m.along_align);
        let (p, rotation, path) = piece(curve, at, length, share)?;
        Some(TextEntity {
            base: kentos_contracts::EntityBase {
                id: 0,
                layer_id: String::new(),
                color: None,
                attrs: Default::default(),
                label: None,
                symbol: None,
                line_weight: None,
                label_pins: Vec::new(),
            },
            p: points::wire(p),
            text: words.to_owned(),
            height,
            rotation,
            align: Some(m.along_align),
            width_factor: None,
            mask: false,
            label_of: None,
            label_scale: None,
            paragraph: Default::default(),
            face,
            path: Some(wire_curve(&path)),
        })
    }

    fn option(&mut self, key: &str, cx: &mut Context<'_>) -> bool {
        if !matches!(self.stage, Stage::Curve | Stage::Place) {
            return false;
        }
        match key {
            "Y" => self.stage = Stage::Height,
            "H" => {
                // Başı → Ortası → Sonu → Başı.
                let (share, side) = split(cx.memory.along_align);
                let next = if share < 0.25 {
                    0.5
                } else if share < 0.75 {
                    1.0
                } else {
                    0.0
                };
                cx.memory.along_align = align_of(next, side);
            }
            "K" => {
                let (share, side) = split(cx.memory.along_align);
                let next = match side {
                    Side::Over => Side::On,
                    Side::On => Side::Under,
                    Side::Under => Side::Over,
                };
                cx.memory.along_align = align_of(share, next);
            }
            "S" if styles::shown(cx.doc.settings()) => self.stage = Stage::Style,
            _ => return false,
        }
        true
    }
}

fn share_word(share: f64) -> &'static str {
    SHARES
        .iter()
        .find(|(s, _, _)| (*s - share).abs() < 0.25)
        .map_or("Ortası", |(_, w, _)| w)
}

fn side_word(side: Side) -> &'static str {
    SIDES
        .iter()
        .find(|(s, _, _)| *s == side)
        .map_or("Üstünde", |(_, w, _)| w)
}

impl Tool for TextAlong {
    fn id(&self) -> &'static str {
        ID
    }

    fn label(&self) -> &'static str {
        LABEL
    }

    fn prompt(&self) -> Prompt {
        let m = self.memory();
        let (share, side) = split(m.along_align);
        let options = |p: Prompt| {
            p.option_if(self.styles.shown, "Stil", "S", self.styles.chosen.clone())
                .option_with(
                    "Yükseklik",
                    "Y",
                    format!("{} mm", js_number(m.heights.mm(AnnotationKind::Text))),
                )
                .option_with("Hiza", "H", share_word(share))
                .option_with("Konum", "K", side_word(side))
        };
        match self.stage {
            Stage::Curve => options(Prompt::new(
                LABEL,
                "yazının izleyeceği çizgiye, yaya, daireye, alana ya da eğriye tıklayın",
            )),
            Stage::Place => options(Prompt::new(
                LABEL,
                "yazının yerine tıklayın; Esc başka eğri seçtirir",
            )),
            Stage::Height => {
                Prompt::new(LABEL, "kâğıt üzerindeki yazı yüksekliğini mm olarak yazın")
            }
            Stage::Style => Prompt::new(LABEL, "yazı stilini menüden seçin ya da adını yazın")
                .option_with("Stil", "S", self.styles.chosen.clone()),
            Stage::Typing => Prompt::new(
                LABEL,
                "yazıyı yazın; Enter eğri boyunca ekler, Esc vazgeçer",
            ),
        }
    }

    fn point_count(&self) -> usize {
        0
    }

    fn activate(&mut self, cx: &mut Context<'_>) -> Flow {
        self.see(cx);
        Flow::Stay
    }

    fn snaps(&self) -> bool {
        false
    }

    /// An object is wanted, then a place on it (the web's `cursor = 'pick'`).
    fn cursor(&self) -> Cursor {
        Cursor::Pick
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.see(cx);
        match self.stage {
            Stage::Curve => {
                self.under = curve_under(p, cx);
                cx.selection.set_hover(self.under);
            }
            Stage::Place => {
                // The curve picked stays lit while the text is placed.
                cx.selection
                    .set_hover(self.curve.as_ref().map(|(slot, _)| *slot));
                let words = self.last_text.clone().unwrap_or_else(|| SAMPLE.to_owned());
                self.shown = self.placed(&words, p.raw, cx);
            }
            _ => {}
        }
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.see(cx);
        match self.stage {
            Stage::Curve => {
                let Some(slot) = curve_under(p, cx) else {
                    cx.say(
                        Level::Warn,
                        "Tıklanan yerde çizgi, yay, daire, alan ya da eğri yok. Yazının izleyeceği nesneye tıklayın.",
                    );
                    return;
                };
                let Some(e) = cx.doc.get(slot) else {
                    return;
                };
                self.curve = Some((slot, shape(e)));
                cx.selection.set_hover(None);
                self.stage = Stage::Place;
            }
            Stage::Place => {
                if let Some(line) = active_locked(cx) {
                    cx.say(Level::Warn, line);
                    return;
                }
                let words = self.last_text.clone().unwrap_or_else(|| SAMPLE.to_owned());
                let Some(t) = self.placed(&words, p.raw, cx) else {
                    return;
                };
                self.at = Some(p.raw);
                self.stage = Stage::Typing;
                // The field straight at the click, along the piece's direction.
                cx.view_changes.push(ViewChange::Text(TextField {
                    at: p.raw,
                    height: t.height,
                    rotation: t.rotation,
                    align: Some(TextAlign::BaselineCenter),
                    width_factor: 1.0,
                    initial: self.last_text.clone(),
                    placeholder: None,
                    hint: Some("Eğri boyunca yazılacak yazı"),
                    empty: false,
                    face: styles::text_face(cx),
                }));
            }
            _ => {}
        }
    }

    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        let t = js_trim(text);
        let done = if self.option(&upper_tr(t), cx) {
            true
        } else if self.stage == Stage::Style {
            if styles::take_text(t, cx) {
                self.stage = if self.curve.is_some() {
                    Stage::Place
                } else {
                    Stage::Curve
                };
            }
            true
        } else if self.stage == Stage::Height {
            match parse_number(t) {
                Some(n) if n > 0.0 => {
                    cx.memory.heights.set(AnnotationKind::Text, Some(n));
                    self.stage = if self.curve.is_some() {
                        Stage::Place
                    } else {
                        Stage::Curve
                    };
                    true
                }
                _ => false,
            }
        } else {
            false
        };
        self.see(cx);
        done
    }

    /// Hiza's and Konum's menus; Stil's in a CAD project.
    fn option_choices(&self, key: &str) -> Vec<OptionChoice> {
        let m = self.memory();
        let (share, side) = split(m.along_align);
        match key {
            "S" if self.styles.shown => self
                .styles
                .choices(styles::TEXT_STYLES_ENTRY, styles::TEXT_STYLES),
            "H" => SHARES
                .iter()
                .map(|&(s, label, typed)| OptionChoice {
                    label: label.to_owned(),
                    typed: typed.to_owned(),
                    icon: Some(match typed {
                        "başı" => "textAlignBaselineLeft",
                        "sonu" => "textAlignBaselineRight",
                        _ => "textAlignBaselineCenter",
                    }),
                    preview: None,
                    checked: (s - share).abs() < 0.25,
                    command: None,
                })
                .collect(),
            "K" => SIDES
                .iter()
                .map(|&(s, label, typed)| OptionChoice {
                    label: label.to_owned(),
                    typed: typed.to_owned(),
                    icon: Some(match s {
                        Side::Over => "textAlignBottomCenter",
                        Side::On => "textAlignMiddleCenter",
                        Side::Under => "textAlignTopCenter",
                    }),
                    preview: None,
                    checked: s == side,
                    command: None,
                })
                .collect(),
            _ => Vec::new(),
        }
    }

    fn choose_option(&mut self, key: &str, typed: &str, cx: &mut Context<'_>) -> bool {
        let (share, side) = split(cx.memory.along_align);
        let taken = match key {
            "S" if styles::shown(cx.doc.settings()) => {
                if styles::take_text(typed, cx) && self.stage == Stage::Style {
                    self.stage = if self.curve.is_some() {
                        Stage::Place
                    } else {
                        Stage::Curve
                    };
                }
                true
            }
            "H" => match SHARES.iter().find(|(_, _, w)| *w == typed) {
                Some(&(s, _, _)) => {
                    cx.memory.along_align = align_of(s, side);
                    true
                }
                None => false,
            },
            "K" => match SIDES.iter().find(|(_, _, w)| *w == typed) {
                Some(&(s, _, _)) => {
                    cx.memory.along_align = align_of(share, s);
                    true
                }
                None => false,
            },
            _ => false,
        };
        self.see(cx);
        taken
    }

    /// The field's answer: the text written along the curve, or nothing.
    fn text_typed(&mut self, text: Option<&str>, cx: &mut Context<'_>) {
        if self.stage != Stage::Typing {
            return;
        }
        if let (Some(at), Some(words)) = (self.at, text.map(js_trim).filter(|t| !t.is_empty()))
            && let Some(t) = self.placed(words, at, cx)
        {
            let geometry = kentos_contracts::EntityGeometry::Text {
                p: t.p,
                text: t.text.clone(),
                height: t.height,
                rotation: t.rotation,
                align: t.align,
                width_factor: None,
                mask: false,
                box_width: None,
                line_spacing: None,
                runs: Vec::new(),
                face: t.face.clone(),
                path: t.path.clone(),
            };
            if points::write_objects(
                vec![geometry],
                Some(kentos_contracts::CreateOperation::TextAlong),
                cx,
            )
            .is_some()
            {
                self.last_text = Some(words.to_owned());
                cx.say(
                    Level::Success,
                    format!("Eğri boyunca yazı eklendi: “{words}”"),
                );
            }
        }
        // The next text follows a curve picked anew.
        self.at = None;
        self.curve = None;
        self.shown = None;
        cx.selection.set_hover(None);
        self.stage = Stage::Curve;
        self.see(cx);
    }

    fn takes_words(&self) -> bool {
        self.stage == Stage::Style
    }

    /// Esc steps back: from the place to the curve, then out.
    fn cancel(&mut self, cx: &mut Context<'_>) -> bool {
        match self.stage {
            Stage::Curve => false,
            Stage::Place => {
                self.curve = None;
                self.shown = None;
                cx.selection.set_hover(None);
                self.stage = Stage::Curve;
                self.see(cx);
                true
            }
            _ => {
                self.stage = if self.curve.is_some() {
                    Stage::Place
                } else {
                    Stage::Curve
                };
                true
            }
        }
    }

    /// Enter: as Esc, and out from the curve's stage.
    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow {
        if self.cancel(cx) {
            Flow::Stay
        } else {
            Flow::Exit
        }
    }

    fn undo_step(&mut self, _cx: &mut Context<'_>) -> bool {
        false
    }

    /// The curve picked, bright; the text to come along it, faint.
    fn preview(&self, _format: &Format) -> Preview {
        let mut preview = Preview::default();
        if self.stage == Stage::Place
            && let Some(t) = &self.shown
        {
            ghosts(t, self.font, &mut preview.texts);
            preview.markers.push(Marker {
                at: Vec2::new(t.p.x, t.p.y),
                shape: MarkerShape::Ring(2.5),
                tone: Tone::Accent,
            });
        }
        preview
    }
}

// ── Yazıyı eğriye oturt ─────────────────────────────────────────────────

/// Yazıyı eğriye oturt's methods.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Method {
    /// Eğriye oturt (O).
    #[default]
    Fit,
    /// Doğrultuya döndür (D).
    Turn,
    /// Düzleştir (Z).
    Straighten,
}

const METHODS: [(Method, &str, &str); 3] = [
    (Method::Fit, "Eğriye oturt", "O"),
    (Method::Turn, "Doğrultuya döndür", "D"),
    (Method::Straighten, "Düzleştir", "Z"),
];

/// The stages after the texts are picked.
#[derive(Clone, Debug, Default)]
pub struct TextCurve {
    method: Method,
    /// The texts the method takes, by slot, as they are.
    texts: Vec<(Slot, TextEntity)>,
    /// The selection's texts the method leaves: of more than one line or
    /// linked (Eğriye oturt), curved (Doğrultuya döndür), straight
    /// (Düzleştir), on locked layers; and whether it has any text at all.
    left: (usize, usize),
    any: bool,
    /// What each would become with the cursor where it is.
    shown: Vec<TextEntity>,
    font: Font,
}

impl TextCurve {
    pub fn tool() -> Modify<Self> {
        Modify::with(Self::default())
    }

    fn take_letter(&mut self, text: &str) -> bool {
        let key = upper_tr(js_trim(text));
        match METHODS.iter().find(|(_, _, k)| *k == key) {
            Some((m, _, _)) => {
                self.method = *m;
                self.shown.clear();
                true
            }
            None => false,
        }
    }

    fn chips(&self, prompt: Prompt) -> Prompt {
        METHODS.iter().fold(prompt, |p, (m, label, key)| {
            p.toggle(label, key, *m == self.method)
        })
    }

    /// The selection's texts the method takes, off locked layers; what it
    /// leaves counted, said when it writes.
    fn gather(&mut self, cx: &Context<'_>) {
        self.font = drawing_font(cx.doc.settings().drawing_font);
        self.texts.clear();
        self.left = (0, 0);
        self.any = false;
        for &slot in cx.selection.ids() {
            let Some(Entity::Text(t)) = cx.doc.get(slot) else {
                continue;
            };
            self.any = true;
            if cx.doc.layers().is_locked(&t.base.layer_id) {
                self.left.1 += 1;
                continue;
            }
            let taken = match self.method {
                // A text of one line, not linked (§1).
                Method::Fit => {
                    !t.text.contains('\n')
                        && t.paragraph.box_width.is_none()
                        && t.paragraph.line_spacing.is_none()
                        && t.label_of.is_none()
                }
                Method::Turn => t.path.is_none(),
                Method::Straighten => t.path.is_some(),
            };
            if taken {
                self.texts.push((slot, t.clone()));
            } else {
                self.left.0 += 1;
            }
        }
    }

    /// What the method left out, said.
    fn say_left(&self, cx: &mut Context<'_>) {
        let (other, locked) = self.left;
        if other > 0 {
            cx.say(
                Level::Info,
                match self.method {
                    Method::Fit => format!(
                        "{other} çok satırlı ya da nesneye bağlı yazı eğriye oturmaz; atlandı."
                    ),
                    Method::Turn => {
                        format!("{other} eğri boyunca yazı döndürülmez; önce Düzleştir. Atlandı.")
                    }
                    Method::Straighten => format!("{other} yazı zaten düz; atlandı."),
                },
            );
        }
        if locked > 0 {
            cx.say(
                Level::Warn,
                format!("{locked} yazı kilitli katmanda; atlandı."),
            );
        }
    }

    /// Each text put on the curve `shape` where its box's middle falls.
    fn fitted(&self, curve: &Shape) -> Vec<(Slot, TextEntity)> {
        self.texts
            .iter()
            .filter_map(|(slot, t)| {
                let straight = TextEntity {
                    path: None,
                    ..t.clone()
                };
                let s = shape(&Entity::Text(straight));
                let place = TextPlace::of(&s)?;
                let ring = place.outline(self.font);
                let (mut lo, mut hi) = (ring[0], ring[0]);
                for q in &ring {
                    lo = Vec2::new(lo.x.min(q.x), lo.y.min(q.y));
                    hi = Vec2::new(hi.x.max(q.x), hi.y.max(q.y));
                }
                let middle = Vec2::new((lo.x + hi.x) / 2.0, (lo.y + hi.y) / 2.0);
                let length = letters_length(
                    &t.text,
                    t.height,
                    t.width_factor.unwrap_or(1.0),
                    &t.face,
                    self.font,
                );
                let (p, rotation, path) = piece(curve, middle, length, 0.5)?;
                Some((
                    *slot,
                    TextEntity {
                        p: points::wire(p),
                        rotation,
                        path: Some(wire_curve(&path)),
                        ..t.clone()
                    },
                ))
            })
            .collect()
    }

    /// Each straight text turned to `degrees`, about its point.
    fn turned(&self, degrees: f64) -> Vec<(Slot, TextEntity)> {
        self.texts
            .iter()
            .map(|(slot, t)| {
                (
                    *slot,
                    TextEntity {
                        rotation: degrees,
                        ..t.clone()
                    },
                )
            })
            .collect()
    }

    /// Each curved text made straight (§4).
    fn straightened(&self) -> Vec<(Slot, TextEntity)> {
        self.texts
            .iter()
            .filter_map(|(slot, t)| {
                let s = shape(&Entity::Text(t.clone()));
                let (p, rotation) = TextPlace::of(&s)?.along(self.font)?.straight();
                Some((
                    *slot,
                    TextEntity {
                        p: points::wire(p),
                        rotation,
                        align: None,
                        path: None,
                        ..t.clone()
                    },
                ))
            })
            .collect()
    }

    /// What is said when the method has no text to take.
    fn none_left(&self, cx: &mut Context<'_>) -> Flow {
        cx.say(
            Level::Warn,
            match self.method {
                Method::Fit => {
                    "Seçimde eğriye oturacak yazı yok. Tek satırlık yazıları seçip yeniden deneyin."
                }
                Method::Turn => {
                    "Seçimde döndürülecek düz yazı yok. Yazıları seçip yeniden deneyin."
                }
                Method::Straighten => {
                    "Seçimde eğri boyunca yazı yok. Eğri boyunca yazıları seçip yeniden deneyin."
                }
            },
        );
        Flow::Exit
    }

    /// Writes the new texts as one edit; the flow after.
    fn write(&mut self, made: Vec<(Slot, TextEntity)>, cx: &mut Context<'_>) -> Flow {
        self.say_left(cx);
        let operation = match self.method {
            Method::Fit => EditOperation::TextPath,
            Method::Turn => EditOperation::TextTurn,
            Method::Straighten => EditOperation::TextStraighten,
        };
        let changes: Vec<EntityEdit> = made
            .iter()
            .filter_map(|(slot, t)| {
                Some(EntityEdit::Update {
                    uid: edge::uid(cx.doc, *slot),
                    geometry: edit_geometry(shape(&Entity::Text(t.clone())))?,
                })
            })
            .collect();
        if changes.is_empty() {
            cx.say(Level::Info, "Değişecek yazı yok.");
            return Flow::Exit;
        }
        let n = changes.len();
        if edge::write(operation, changes, cx).is_none() {
            return Flow::Stay;
        }
        cx.say(
            Level::Success,
            match self.method {
                Method::Fit => format!("{n} yazı eğriye oturtuldu."),
                Method::Turn => format!("{n} yazı doğrultuya döndürüldü."),
                Method::Straighten => format!("{n} yazı düzleştirildi."),
            },
        );
        Flow::Exit
    }
}

impl Stages for TextCurve {
    fn id(&self) -> &'static str {
        CURVE_ID
    }

    fn label(&self) -> &'static str {
        CURVE_LABEL
    }

    /// The selection's texts taken; Düzleştir writes at once. A selection
    /// with no text at all leaves (said); the method's letter may still come
    /// (the ribbon's methods type it).
    fn begin(&mut self, cx: &mut Context<'_>) -> Flow {
        self.shown.clear();
        self.gather(cx);
        if !self.any {
            cx.say(
                Level::Warn,
                "Seçimde yazı yok. Yazıları seçip yeniden deneyin.",
            );
            return Flow::Exit;
        }
        if self.method == Method::Straighten {
            if self.texts.is_empty() {
                return self.none_left(cx);
            }
            let made = self.straightened();
            return self.write(made, cx);
        }
        Flow::Stay
    }

    fn picking_hint(&self, prompt: Prompt) -> Prompt {
        self.chips(prompt)
    }

    fn picking_input(&mut self, text: &str, _cx: &mut Context<'_>) -> bool {
        self.take_letter(text)
    }

    fn anchor(&self) -> Option<Vec2> {
        None
    }

    fn snaps(&self) -> bool {
        false
    }

    fn prompt(&self, _n: usize) -> Prompt {
        let step = match self.method {
            Method::Fit => "yazıların oturacağı eğriye tıklayın",
            Method::Turn => "doğrultusu alınacak kenara tıklayın",
            Method::Straighten => "Enter ile düzleştirin",
        };
        self.chips(Prompt::new(CURVE_LABEL, step))
    }

    fn pointer(&mut self, p: &Pointer, down: bool, cx: &mut Context<'_>) -> Option<Flow> {
        match self.method {
            _ if down && self.texts.is_empty() => return Some(self.none_left(cx)),
            Method::Fit => {
                let slot = curve_under(p, cx);
                cx.selection.set_hover(slot);
                let made = slot
                    .and_then(|s| cx.doc.get(s))
                    .map(|e| self.fitted(&shape(e)))
                    .unwrap_or_default();
                if down {
                    if made.is_empty() {
                        cx.say(
                            Level::Warn,
                            "Tıklanan yerde çizgi, yay, daire, alan ya da eğri yok. Yazıların oturacağı nesneye tıklayın.",
                        );
                        return Some(Flow::Stay);
                    }
                    return Some(self.write(made, cx));
                }
                self.shown = made.into_iter().map(|(_, t)| t).collect();
            }
            Method::Turn => {
                let Some((_, dir)) = crate::locks::picked_edge(p.raw, cx) else {
                    self.shown.clear();
                    if down {
                        cx.say(
                            Level::Warn,
                            "Tıklanan yerde kenar ya da yay yok. Doğrultusu alınacak kenara tıklayın.",
                        );
                        return Some(Flow::Stay);
                    }
                    return None;
                };
                let made = self.turned(readable_turn(dir.x, dir.y));
                if down {
                    return Some(self.write(made, cx));
                }
                self.shown = made.into_iter().map(|(_, t)| t).collect();
            }
            Method::Straighten => {
                if down {
                    let made = self.straightened();
                    return Some(self.write(made, cx));
                }
            }
        }
        None
    }

    fn point(&mut self, _p: Vec2, _cx: &mut Context<'_>) -> Flow {
        Flow::Stay
    }

    fn typed(&mut self, text: &str, cx: &mut Context<'_>) -> Option<Flow> {
        if !self.take_letter(text) {
            return None;
        }
        Some(self.begin(cx))
    }

    fn typed_points(&self) -> bool {
        false
    }

    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow {
        if self.method == Method::Straighten {
            let made = self.straightened();
            return self.write(made, cx);
        }
        Flow::Exit
    }

    fn preview(&self, _hover: Vec2) -> Option<kentos_geometry_core::geom::affine::Affine> {
        None
    }

    /// The texts where they would go: curved ones letter by letter, turned
    /// ones as their boxes.
    fn stage_preview(&self, _hover: Option<Vec2>, _format: &Format) -> Option<Preview> {
        let mut preview = Preview::default();
        for t in self.shown.iter().take(MAX_PREVIEWED) {
            if t.path.is_some() {
                ghosts(t, self.font, &mut preview.texts);
            } else {
                let s = shape(&Entity::Text(t.clone()));
                if let Some(place) = TextPlace::of(&s) {
                    preview.strokes.push(Stroke {
                        pts: place.outline(self.font),
                        closed: true,
                        dash: Some([3.0, 3.0]),
                        width: 1.0,
                        tone: Tone::Accent,
                    });
                    preview.texts.push(TextGhost {
                        p: Vec2::new(t.p.x, t.p.y),
                        text: t.text.clone(),
                        height: t.height,
                        rotation: t.rotation,
                        align: t.align.and_then(|a| {
                            kentos_geometry_core::text::TextAlign::from_name(a.name())
                        }),
                        mask: false,
                        face: t.face.clone(),
                        width_factor: t.width_factor.unwrap_or(1.0),
                    });
                }
            }
        }
        Some(preview)
    }
}
