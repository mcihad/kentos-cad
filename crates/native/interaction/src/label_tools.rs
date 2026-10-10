//! The label tools (docs/adr/0212 §4; the web's `tools/labelTools.ts`):
//! Etiketi taşı, Etiketi döndür, Etiketi sabitle and Etiketi gizle. Each
//! takes a label the label engine placed in the view (`Spatial::label_at`,
//! the main view's last labels) and writes its pin with `cad.labels.pin` as
//! one undo step “Etiket”; the pin is the label's middle from its object's
//! anchor, so the label goes with the object.
//!
//! - Etiketi taşı: a click on a label, then its new place (it moves as the
//!   pointer does from where it was taken; a typed point is its new middle).
//! - Etiketi döndür: a click on a label, then its angle: toward the pointer
//!   from its middle, made to read (Shift: 15° steps), or typed in degrees
//!   counter-clockwise.
//! - Etiketi sabitle: a click pins a label where it is; a window pins every
//!   label whose middle is in it. Çöz (Ç) frees pinned labels the same way.
//! - Etiketi gizle: a click hides a label. Göster (G) draws the hidden ones
//!   faint, and a click on one shows it again.
//!
//! Unplaced labels (drawn while Yerleşmeyen etiketleri göster is on) are
//! taken too: moving one is how it gets a place.

use kentos_contracts::{LabelPin, LabelPinChange, LabelsMode, LabelsPin};
use kentos_domain::Slot;
use kentos_geometry_core::labels::engine::{HIDDEN, PINNED, UNPLACED};
use kentos_geometry_core::store::placing::LabelHit;
use kentos_native_application::{CommandResult, ExecutionContext, labels_pin};

use crate::Vec2;
use crate::format::Format;
use crate::log::Level;
use crate::prompt::{Prompt, upper_tr};
use crate::select::SelectBox;
use crate::tool::{Context, Cursor, Flow, Pointer, Preview, Stroke, Tag, Tone, Tool};

/// Etiketi taşı's id: its command is `tool.labelMove`.
pub const MOVE_ID: &str = "labelMove";
/// Etiketi döndür's id: its command is `tool.labelRotate`.
pub const ROTATE_ID: &str = "labelRotate";
/// Etiketi sabitle's id: its command is `tool.labelPin`.
pub const PIN_ID: &str = "labelPin";
/// Etiketi gizle's id: its command is `tool.labelHide`.
pub const HIDE_ID: &str = "labelHide";

/// How far a press moves before it is a window, logical pixels.
const DRAG_THRESHOLD: f64 = 4.0;
/// Shift's steps for an angle, degrees.
const STEP_DEGREES: f64 = 15.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Action {
    Move,
    Rotate,
    Pin,
    Hide,
}

/// A press on the drawing: where it began and where the pointer is.
#[derive(Clone, Copy, Debug)]
struct Press {
    from: [f64; 2],
    to: [f64; 2],
    from_world: Vec2,
    to_world: Vec2,
    dragging: bool,
}

/// One of the four label tools.
#[derive(Clone, Debug)]
pub struct LabelTool {
    action: Action,
    /// The label taken (Etiketi taşı, Etiketi döndür) and where it was taken.
    taken: Option<(LabelHit, Vec2)>,
    /// Çöz for Etiketi sabitle, Göster for Etiketi gizle.
    other: bool,
    /// The label under the pointer and the pointer's world point.
    hover: Option<(LabelHit, Vec2)>,
    /// The pointer's world point and Shift.
    pointer: Option<(Vec2, bool)>,
    press: Option<Press>,
    /// World metres a logical pixel spans, as the view had it last.
    metres: f64,
    /// The hover's tag lines.
    lines: Vec<String>,
}

impl LabelTool {
    fn with(action: Action) -> Self {
        Self {
            action,
            taken: None,
            other: false,
            hover: None,
            pointer: None,
            press: None,
            metres: 1.0,
            lines: Vec::new(),
        }
    }

    pub fn move_label() -> Self {
        Self::with(Action::Move)
    }

    pub fn rotate_label() -> Self {
        Self::with(Action::Rotate)
    }

    pub fn pin_label() -> Self {
        Self::with(Action::Pin)
    }

    pub fn hide_label() -> Self {
        Self::with(Action::Hide)
    }

    /// Whether a click takes hidden labels: Etiketi gizle's Göster.
    fn showing(&self) -> bool {
        self.action == Action::Hide && self.other
    }

    /// The label under the pointer: unplaced ones too while they are drawn, hidden ones under Göster.
    fn hit(&self, at: Vec2, cx: &Context<'_>) -> Option<LabelHit> {
        let all = self.showing() || cx.draft.unplaced_labels;
        let hit = cx.spatial.label_at(at, cx.draft.pick_aperture, all)?;
        if self.showing() {
            return (hit.state & HIDDEN != 0).then_some(hit);
        }
        (hit.state & HIDDEN == 0).then_some(hit)
    }

    /// The angle Etiketi döndür gives toward `at`, degrees: made to read, Shift's 15° steps.
    fn angle_toward(&self, at: Vec2, shift: bool) -> Option<f64> {
        let (hit, _) = self.taken?;
        let (dx, dy) = (at.x - hit.at.x, at.y - hit.at.y);
        if dx == 0.0 && dy == 0.0 {
            return None;
        }
        let mut a = dy.atan2(dx).to_degrees();
        while a > 90.0 {
            a -= 180.0;
        }
        while a <= -90.0 {
            a += 180.0;
        }
        if shift {
            a = (a / STEP_DEGREES).round() * STEP_DEGREES;
        }
        Some(if a == 0.0 { 0.0 } else { a })
    }

    /// Where Etiketi taşı puts the taken label's middle with the pointer at `at`.
    fn moved(&self, at: Vec2) -> Option<Vec2> {
        let (hit, from) = self.taken?;
        Some(Vec2::new(
            hit.at.x + at.x - from.x,
            hit.at.y + at.y - from.y,
        ))
    }
}

/// A rule-based layer's class name of a label; none for a single label's.
fn class_name(cx: &Context<'_>, slot: Slot, class: u16) -> Option<String> {
    let e = cx.doc.get(slot)?;
    let labels = cx
        .doc
        .layers()
        .get(&e.base().layer_id)?
        .style
        .labels
        .as_ref()?;
    if labels.mode != LabelsMode::Rules {
        return None;
    }
    labels
        .classes
        .get(usize::from(class))
        .map(|c| c.name.clone())
}

/// The pin a label has now (its object's, by class), if any.
fn pin_of(cx: &Context<'_>, slot: Slot, class: &Option<String>) -> Option<LabelPin> {
    let e = cx.doc.get(slot)?;
    e.base()
        .label_pins
        .iter()
        .find(|p| p.class == *class)
        .cloned()
}

/// A label's place as a pin: its middle from its object's anchor, its angle; none without an anchor.
fn pinned_at(cx: &Context<'_>, slot: Slot, middle: Vec2, angle: f64) -> Option<LabelPin> {
    let anchor = cx.spatial.label_anchor(slot)?;
    Some(LabelPin {
        class: None,
        at: Some(kentos_contracts::Vec2 {
            x: middle.x - anchor.x,
            y: middle.y - anchor.y,
        }),
        rotation: (angle != 0.0).then_some(angle),
        hidden: None,
    })
}

/// Writes the changes with `cad.labels.pin`, says `done` with how many changed (or why not); whether it wrote.
fn write(
    cx: &mut Context<'_>,
    changes: Vec<(Slot, Option<String>, Option<LabelPin>)>,
    done: impl Fn(u32) -> String,
) -> bool {
    let mut pins = Vec::with_capacity(changes.len());
    for (slot, class, pin) in changes {
        let Some(uid) = cx.doc.uid(slot) else {
            continue;
        };
        pins.push(LabelPinChange {
            uid: uid.to_string(),
            class,
            pin,
        });
    }
    if pins.is_empty() {
        return false;
    }
    let input = LabelsPin {
        pins,
        expected_revision: None,
    };
    match labels_pin::execute(&mut ExecutionContext::new(cx.doc), input) {
        CommandResult::Completed { output, .. } => {
            cx.say(Level::Success, done(output.changed));
            output.changed > 0
        }
        CommandResult::Failed { error }
        | CommandResult::Conflict { error }
        | CommandResult::NeedsInput { error } => {
            cx.say(Level::Warn, error.message);
            false
        }
        CommandResult::Queued { .. } | CommandResult::Cancelled => false,
    }
}

/// The dashed box round a label, as it would stand at `middle` turned `angle` degrees.
fn frame(hit: &LabelHit, middle: Vec2, angle: f64, metres: f64) -> Stroke {
    let (hw, hh) = ((hit.w / 2.0 + 3.0) * metres, (hit.h / 2.0 + 3.0) * metres);
    let (c, s) = (angle.to_radians().cos(), angle.to_radians().sin());
    let at = |u: f64, v: f64| Vec2::new(middle.x + u * c - v * s, middle.y + u * s + v * c);
    Stroke {
        pts: vec![at(-hw, -hh), at(hw, -hh), at(hw, hh), at(-hw, hh)],
        closed: true,
        dash: Some([4.0, 3.0]),
        width: 1.5,
        tone: Tone::Accent,
    }
}

impl Tool for LabelTool {
    fn id(&self) -> &'static str {
        match self.action {
            Action::Move => MOVE_ID,
            Action::Rotate => ROTATE_ID,
            Action::Pin => PIN_ID,
            Action::Hide => HIDE_ID,
        }
    }

    fn label(&self) -> &'static str {
        match self.action {
            Action::Move => "Etiketi taşı",
            Action::Rotate => "Etiketi döndür",
            Action::Pin => "Etiketi sabitle",
            Action::Hide => "Etiketi gizle",
        }
    }

    fn cursor(&self) -> Cursor {
        if self.taken.is_some() {
            Cursor::Cross
        } else {
            Cursor::Pick
        }
    }

    fn snaps(&self) -> bool {
        false
    }

    fn prompt(&self) -> Prompt {
        let label = self.label();
        match (self.action, self.taken.is_some()) {
            (Action::Move, false) => Prompt::new(label, "taşınacak etikete tıklayın"),
            (Action::Move, true) => {
                Prompt::new(label, "etiketin yeni yerine tıklayın ya da koordinat yazın")
            }
            (Action::Rotate, false) => Prompt::new(label, "döndürülecek etikete tıklayın"),
            (Action::Rotate, true) => {
                Prompt::new(label, "açıyı imleçle gösterin ya da derece olarak yazın")
                    .note("Shift: 15° adım")
            }
            (Action::Pin, _) if self.other => Prompt::new(
                label,
                "serbest bırakılacak etikete tıklayın ya da pencereyle seçin",
            )
            .toggle("Çöz", "Ç", true),
            (Action::Pin, _) => Prompt::new(
                label,
                "sabitlenecek etikete tıklayın ya da pencereyle seçin",
            )
            .toggle("Çöz", "Ç", false),
            (Action::Hide, _) if self.other => {
                Prompt::new(label, "yeniden gösterilecek gizli etikete tıklayın")
                    .toggle("Göster", "G", true)
            }
            (Action::Hide, _) => {
                Prompt::new(label, "gizlenecek etikete tıklayın").toggle("Göster", "G", false)
            }
        }
    }

    fn point_count(&self) -> usize {
        usize::from(self.taken.is_some())
    }

    fn shows_hidden_labels(&self) -> bool {
        self.showing()
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.metres = cx.view.world_length(1.0);
        self.pointer = Some((p.raw, p.shift));
        if let Some(press) = &mut self.press {
            press.to = p.screen;
            press.to_world = p.raw;
            let moved = (press.to[0] - press.from[0]).hypot(press.to[1] - press.from[1]);
            if !press.dragging && moved > DRAG_THRESHOLD {
                press.dragging = true;
            }
        }
        self.hover = if self.taken.is_none() {
            self.hit(p.raw, cx).map(|h| (h, p.raw))
        } else {
            None
        };
        self.lines = match self.hover {
            Some((hit, _)) => {
                let slot = Slot(hit.id as u32);
                let layer = cx
                    .doc
                    .get(slot)
                    .and_then(|e| cx.doc.layers().get(&e.base().layer_id))
                    .map_or_else(String::new, |n| n.name.clone());
                let class = class_name(cx, slot, hit.class);
                let mut first = match class {
                    Some(c) => format!("{layer}, {c}"),
                    None => layer,
                };
                if hit.state & PINNED != 0 {
                    first.push_str(" (sabit)");
                } else if hit.state & UNPLACED != 0 {
                    first.push_str(" (yerleşmedi)");
                }
                let click = match (self.action, self.other) {
                    (Action::Move, _) => "Tıklayın: taşı",
                    (Action::Rotate, _) => "Tıklayın: döndür",
                    (Action::Pin, false) => "Tıklayın: sabitle",
                    (Action::Pin, true) => "Tıklayın: serbest bırak",
                    (Action::Hide, false) => "Tıklayın: gizle",
                    (Action::Hide, true) => "Tıklayın: göster",
                };
                vec![first, click.to_owned()]
            }
            None => Vec::new(),
        };
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.metres = cx.view.world_length(1.0);
        match (self.action, self.taken) {
            (Action::Move, Some(_)) => {
                if let Some(middle) = self.moved(p.raw) {
                    self.put(middle, cx);
                }
            }
            (Action::Rotate, Some(_)) => {
                if let Some(angle) = self.angle_toward(p.raw, p.shift) {
                    self.turn(angle, cx);
                }
            }
            (Action::Move | Action::Rotate, None) => match self.hit(p.raw, cx) {
                Some(hit) => {
                    self.taken = Some((hit, p.raw));
                    self.hover = None;
                    self.lines.clear();
                }
                None => {
                    let what = if self.action == Action::Move {
                        "taşınacak"
                    } else {
                        "döndürülecek"
                    };
                    cx.say(Level::Warn, format!("Çizimde {what} bir etikete tıklayın."));
                }
            },
            (Action::Pin | Action::Hide, _) => {
                self.press = Some(Press {
                    from: p.screen,
                    to: p.screen,
                    from_world: p.raw,
                    to_world: p.raw,
                    dragging: false,
                });
            }
        }
    }

    fn pointer_up(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        let Some(press) = self.press.take() else {
            return;
        };
        let hits: Vec<LabelHit> = if press.dragging && self.action == Action::Pin {
            cx.spatial
                .labels_in(press.from_world, press.to_world, cx.draft.unplaced_labels)
                .into_iter()
                .filter(|h| h.state & HIDDEN == 0)
                .collect()
        } else {
            self.hit(p.raw, cx).into_iter().collect()
        };
        if hits.is_empty() {
            let text = match (self.action, self.other) {
                (Action::Pin, false) => "Sabitlenecek bir etikete tıklayın ya da pencereyle seçin.",
                (Action::Pin, true) => {
                    "Serbest bırakılacak sabit bir etikete tıklayın ya da pencereyle seçin."
                }
                (_, false) => "Gizlenecek bir etikete tıklayın.",
                (_, true) => "Yeniden gösterilecek gizli bir etikete tıklayın (soluk çizilenler).",
            };
            cx.say(Level::Warn, text);
            return;
        }
        match self.action {
            Action::Pin => self.pin(&hits, cx),
            _ => self.hide(&hits[0], cx),
        }
    }

    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        let key = upper_tr(crate::js_trim(text));
        match (self.action, key.as_str()) {
            (Action::Pin, "Ç" | "C") | (Action::Hide, "G") => {
                self.other = !self.other;
                return true;
            }
            _ => {}
        }
        match (self.action, self.taken) {
            (Action::Move, Some(_)) => {
                let Some(middle) = cx.typed_point(text, None, None) else {
                    return false;
                };
                self.put(middle, cx);
                true
            }
            (Action::Rotate, Some(_)) => {
                let Some(angle) = kentos_geometry_core::tools::point_text::parse_number(text)
                else {
                    return false;
                };
                self.turn(angle, cx);
                true
            }
            _ => false,
        }
    }

    /// Enter and a quick right click end the tool.
    fn confirm(&mut self, _cx: &mut Context<'_>) -> Flow {
        Flow::Exit
    }

    /// Esc lets a taken label go; with none, the tool leaves.
    fn cancel(&mut self, _cx: &mut Context<'_>) -> bool {
        if self.taken.take().is_some() {
            return true;
        }
        self.press = None;
        false
    }

    /// Ctrl+Z with a label taken lets it go; otherwise the drawing is undone.
    fn undo_step(&mut self, _cx: &mut Context<'_>) -> bool {
        self.taken.take().is_some()
    }

    fn select_box(&self) -> Option<SelectBox> {
        let press = self
            .press
            .filter(|p| p.dragging && self.action == Action::Pin)?;
        Some(SelectBox {
            from: press.from,
            to: press.to,
        })
    }

    fn preview(&self, _format: &Format) -> Preview {
        let mut preview = Preview::default();
        let m = self.metres;
        if let Some((hit, _)) = self.taken {
            preview.strokes.push(frame(&hit, hit.at, hit.angle, m));
            if let Some((at, shift)) = self.pointer {
                match self.action {
                    Action::Move => {
                        if let Some(middle) = self.moved(at) {
                            let mut box_ = frame(&hit, middle, hit.angle, m);
                            box_.dash = None;
                            preview.strokes.push(box_);
                            preview.guides.push([hit.at, middle]);
                        }
                    }
                    Action::Rotate => {
                        if let Some(angle) = self.angle_toward(at, shift) {
                            let mut box_ = frame(&hit, hit.at, angle, m);
                            box_.dash = None;
                            preview.strokes.push(box_);
                            preview.guides.push([hit.at, at]);
                            preview.tag = Some(Tag {
                                at,
                                lines: vec![format!("{angle:.1}°")],
                            });
                        }
                    }
                    _ => {}
                }
            }
            return preview;
        }
        if let Some((hit, at)) = self.hover {
            preview.strokes.push(frame(&hit, hit.at, hit.angle, m));
            preview.tag = (!self.lines.is_empty()).then(|| Tag {
                at,
                lines: self.lines.clone(),
            });
        }
        preview
    }
}

impl LabelTool {
    /// Etiketi taşı: the taken label's middle at `middle`, its angle kept.
    fn put(&mut self, middle: Vec2, cx: &mut Context<'_>) {
        let Some((hit, _)) = self.taken.take() else {
            return;
        };
        let slot = Slot(hit.id as u32);
        let class = class_name(cx, slot, hit.class);
        let angle = if hit.state & kentos_geometry_core::labels::engine::CURVED != 0 {
            0.0
        } else {
            hit.angle
        };
        let Some(pin) = pinned_at(cx, slot, middle, angle) else {
            return;
        };
        write(cx, vec![(slot, class, Some(pin))], |_| {
            "Etiket taşındı ve sabitlendi.".to_owned()
        });
    }

    /// Etiketi döndür: the taken label turned `angle` degrees where it is.
    fn turn(&mut self, angle: f64, cx: &mut Context<'_>) {
        let Some((hit, _)) = self.taken.take() else {
            return;
        };
        let slot = Slot(hit.id as u32);
        let class = class_name(cx, slot, hit.class);
        let Some(pin) = pinned_at(cx, slot, hit.at, angle) else {
            return;
        };
        write(cx, vec![(slot, class, Some(pin))], |_| {
            "Etiket döndürüldü ve sabitlendi.".to_owned()
        });
    }

    /// Etiketi sabitle: the labels pinned where they are, or (Çöz) freed.
    fn pin(&mut self, hits: &[LabelHit], cx: &mut Context<'_>) {
        let mut changes = Vec::new();
        for hit in hits {
            let slot = Slot(hit.id as u32);
            let class = class_name(cx, slot, hit.class);
            let pinned = hit.state & PINNED != 0;
            if self.other {
                if pinned {
                    changes.push((slot, class, None));
                }
            } else if !pinned {
                let curved = hit.state & kentos_geometry_core::labels::engine::CURVED != 0;
                if let Some(pin) = pinned_at(cx, slot, hit.at, if curved { 0.0 } else { hit.angle })
                {
                    changes.push((slot, class, Some(pin)));
                }
            }
        }
        if changes.is_empty() {
            let text = if self.other {
                "Seçilen etiketlerin hiçbiri sabit değil."
            } else {
                "Seçilen etiketler zaten sabit."
            };
            cx.say(Level::Info, text);
            return;
        }
        let free = self.other;
        let n = changes.len();
        write(cx, changes, move |_| match (free, n) {
            (false, 1) => "Etiket sabitlendi.".to_owned(),
            (false, n) => format!("{n} etiket sabitlendi."),
            (true, 1) => "Etiket serbest bırakıldı.".to_owned(),
            (true, n) => format!("{n} etiket serbest bırakıldı."),
        });
    }

    /// Etiketi gizle: the label hidden (its place kept), or (Göster) shown again.
    fn hide(&mut self, hit: &LabelHit, cx: &mut Context<'_>) {
        let slot = Slot(hit.id as u32);
        let class = class_name(cx, slot, hit.class);
        let now = pin_of(cx, slot, &class);
        let pin = if self.other {
            // Shown again: a pin with a place keeps it, one only hiding goes.
            now.filter(|p| p.at.is_some())
                .map(|p| LabelPin { hidden: None, ..p })
        } else {
            Some(match now {
                Some(p) if p.at.is_some() => LabelPin {
                    hidden: Some(true),
                    ..p
                },
                _ => LabelPin {
                    hidden: Some(true),
                    ..LabelPin::default()
                },
            })
        };
        let pin = pin.map(|p| LabelPin { class: None, ..p });
        let text = if self.other {
            "Etiket yeniden gösteriliyor."
        } else {
            "Etiket gizlendi."
        };
        write(cx, vec![(slot, class, pin)], move |_| text.to_owned());
    }
}
