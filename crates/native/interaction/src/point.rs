//! Nokta and Kot noktası: the web's `SurveyPointTool` (docs/adr/0152 §2–§3,
//! `apps/web/src/tools/surveyPointTool.ts`) and its `PointTool` with an
//! elevation (`askZ`, `apps/web/src/tools/drawTools.ts`) on their
//! `PointInputTool` base, step for step (docs/adr/0032, 0057):
//!
//! - Nokta: every point, clicked (snaps, ortho and polar tracking applied)
//!   or typed (the shared grammar, `#ad` included), writes one point object
//!   through `cad.point.create`: its own object and its own undo step; the
//!   point's echo is its message. Ad (A) and Kod (K) are asked in the text
//!   field over the drawing (letters typed over it would start a command),
//!   Kot (Z) is typed; an empty answer clears them. All three are kept for
//!   the session ([`Memory`]), and the name moves on by Yazı's Artır after
//!   each point written (101 → 102; a name not ending in a number stays).
//!   Where a point already is (1 µm) it asks first: Düzelt gives that point
//!   the name, code and elevation given (one step “Nokta düzelt”), Ekle
//!   writes the new one too, Atla (Esc) passes;
//! - Kot noktası: after each point the elevation is typed (clicks wait
//!   meanwhile; “Kot?” beside the point); the point goes on the spot
//!   elevations' layer (`kot`) with the elevation, its text (two decimals)
//!   and its attributes (Tür, Z (m));
//! - neither holds a point, so a confirm (Enter, Space, a quick right click)
//!   leaves;
//! - Ctrl+Z takes the newest point (or Düzelt) back as an undo while the
//!   drawing has not changed since, and Nokta gives its name back; with
//!   nothing of its own it undoes the drawing.
//!
//! Nothing else is drawn while they run but the cursor, the snap marker and
//! the name Nokta's next point takes, as on the web.

use std::collections::BTreeMap;

use kentos_contracts::{
    CommandResult, EditOperation, EntitiesEdit, EntitiesSetProperties, Entity, EntityEdit,
    EntityGeometry, PointCreate, PropertiesOperation,
};
use kentos_domain::{Document, Slot};
use kentos_geometry_core::jsmath::js_hypot;
use kentos_geometry_core::text::edit::increment;
use kentos_native_application::{ExecutionContext, edit, point, set};

use crate::Vec2;
use crate::format::{Format, fixed};
use crate::log::Level;
use crate::points::{self, Taken, wire};
use crate::prompt::{Prompt, upper_tr};
use crate::tool::{
    Context, Flow, Memory, Name, Pointer, Preview, Tag, TextField, Tool, ViewChange,
};

/// The point tool's id: its command is `tool.point`.
pub const ID: &str = "point";
pub const LABEL: &str = "Nokta";
/// The spot elevation tool's id: its command is `tool.spot` (docs/adr/0057).
pub const SPOT_ID: &str = "spot";
pub const SPOT_LABEL: &str = "Kot noktası";
/// The layer spot elevations go on (the web's `LAYERS.spot`, the project template's).
pub const SPOT_LAYER: &str = "kot";
/// Düzelt's undo step (docs/adr/0152 §3).
pub const CORRECT_LABEL: &str = "Nokta düzelt";
/// “The same place”, metres (docs/adr/0142).
const SAME: f64 = 1e-6;

/// Ad (A) or Kod (K): Nokta's values asked in the text field (docs/adr/0152 §2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Field {
    Name,
    Code,
}

impl Field {
    /// Its key: `A`, `K`.
    pub fn of_key(key: &str) -> Option<Self> {
        match key {
            "A" => Some(Self::Name),
            "K" => Some(Self::Code),
            _ => None,
        }
    }

    /// What the empty field shows, and what is said of a value too long.
    fn what(self) -> &'static str {
        match self {
            Self::Name => "Noktanın adı",
            Self::Code => "Noktaların kodu",
        }
    }

    fn hint(self) -> &'static str {
        match self {
            Self::Name => "Enter: kaydet · boş Enter: adsız · Esc: vazgeç",
            Self::Code => "Enter: kaydet · boş Enter: kodsuz · Esc: vazgeç",
        }
    }
}

/// The text field asking Ad (A) or Kod (K) by the cursor (or the view's
/// middle), the old value in it; an empty Enter answers with an empty text
/// (the web's `askPointField`). Köşelere nokta asks them too: they are Nokta's (§5).
pub(crate) fn ask_field(field: Field, near: Option<Vec2>, cx: &Context<'_>) -> ViewChange {
    let b = cx.view.visible();
    let at = near.unwrap_or(Vec2::new(
        (b.min_x + b.max_x) / 2.0,
        (b.min_y + b.max_y) / 2.0,
    ));
    let value = match field {
        Field::Name => cx.memory.point_name,
        Field::Code => cx.memory.point_code,
    };
    ViewChange::Text(TextField {
        at,
        // Readable at any zoom: 14 px on the screen.
        height: cx.view.world_length(14.0),
        rotation: 0.0,
        align: None,
        width_factor: 1.0,
        initial: Some(value.as_str().to_owned()),
        placeholder: Some(field.what()),
        hint: Some(field.hint()),
        empty: true,
        face: Default::default(),
    })
}

/// The field's answer: the value typed, trimmed (an empty one clears it), or
/// none (Esc: the old one stays). A value longer than [`Name::MAX_CHARS`] is
/// said and the old one kept. Whether the value changed.
pub(crate) fn take_field(field: Field, text: Option<&str>, cx: &mut Context<'_>) -> bool {
    let Some(text) = text else {
        return false;
    };
    let Some(value) = Name::new(crate::js_trim(text)) else {
        let line = format!("{} en çok {} harf olabilir.", field.what(), Name::MAX_CHARS);
        cx.say(Level::Warn, line);
        return false;
    };
    let kept = match field {
        Field::Name => &mut cx.memory.point_name,
        Field::Code => &mut cx.memory.point_code,
    };
    let changed = *kept != value;
    *kept = value;
    changed
}

/// The options' values as a prompt shows them: `Ad (A): 101 / Kod (K): SN`.
pub(crate) fn with_options(prompt: Prompt, m: &Memory) -> Prompt {
    let shown = |v: &Name| match v.as_str() {
        "" => "—".to_owned(),
        v => v.to_owned(),
    };
    prompt
        .option_with("Ad", "A", shown(&m.point_name))
        .option_with("Kod", "K", shown(&m.point_code))
}

/// The point already where a new one goes, the first in the drawing's
/// order, with its name (trimmed; none when it has none): the web's `pointAt`.
pub fn point_at(doc: &Document, p: Vec2) -> Option<(Slot, Option<String>)> {
    doc.entities().find_map(|e| match e {
        Entity::Point(q) if js_hypot(q.p.x - p.x, q.p.y - p.y) <= SAME => {
            let name = q.base.label.as_deref().map(crate::js_trim);
            Some((
                Slot(q.base.id),
                name.filter(|n| !n.is_empty()).map(str::to_owned),
            ))
        }
        _ => None,
    })
}

/// What Nokta is asking (docs/adr/0152 §2–§3).
#[derive(Clone, Debug, Default, PartialEq)]
enum Stage {
    /// A point's place.
    #[default]
    Points,
    /// Kot (Z): the elevation typed.
    Z,
    /// Ad (A) or Kod (K): the text field answers.
    Field(Field),
    /// A point is where the new one goes: Düzelt, Ekle or Atla.
    Question {
        at: Vec2,
        there: Slot,
        name: Option<String>,
    },
}

/// The point tool, or with `spot` the spot elevation tool.
#[derive(Clone, Debug, Default)]
pub struct Point {
    d: Taken,
    spot: bool,
    /// Kot noktası: the point waiting for its elevation.
    pending_z: Option<Vec2>,
    stage: Stage,
    /// The name the newest step took, given back when Ctrl+Z takes the step
    /// back; none when there is none to give.
    given_back: Option<Name>,
    /// The session's values and the project's units when last seen: what the prompt shows.
    seen: Option<(Memory, Format)>,
}

impl Point {
    pub fn new() -> Self {
        Self::default()
    }

    /// Kot noktası: a point, then its elevation.
    pub fn spot() -> Self {
        Self {
            spot: true,
            ..Self::default()
        }
    }

    fn see(&mut self, cx: &Context<'_>) {
        self.seen = Some((*cx.memory, cx.format()));
    }

    /// The web's `accept`: the point is echoed, and a new point makes what
    /// was written before the drawing's to undo (its name no longer given back).
    fn accept(&mut self, p: Vec2, cx: &mut Context<'_>) {
        self.given_back = None;
        self.d.begin(p, cx);
        if self.spot {
            self.pending_z = Some(p);
        } else if let Some((there, name)) = point_at(cx.doc, p) {
            self.stage = Stage::Question { at: p, there, name };
        } else {
            self.write_point(p, cx);
        }
        self.see(cx);
    }

    fn back(&mut self) {
        self.stage = Stage::Points;
    }

    /// Letter options (the web's `option`): A, K and Z where a place is
    /// asked; D and E in the question.
    fn option(&mut self, key: &str, cx: &mut Context<'_>) -> bool {
        match (&self.stage, key) {
            (Stage::Question { .. }, "D") => self.correct(cx),
            (Stage::Question { at, .. }, "E") => {
                let at = *at;
                self.back();
                self.write_point(at, cx);
            }
            (Stage::Points, "A" | "K") => {
                let Some(field) = Field::of_key(key) else {
                    return false;
                };
                self.stage = Stage::Field(field);
                let ask = ask_field(field, self.d.hover, cx);
                cx.view_changes.push(ask);
            }
            (Stage::Points, "Z") => self.stage = Stage::Z,
            _ => return false,
        }
        true
    }

    /// Nokta's point through `cad.point.create` (docs/adr/0152 §2): the
    /// active layer, its name, code and elevation, the current colour
    /// explicit in its input (CMD-07), and an object template's symbol,
    /// attributes and label (its name before the label; docs/adr/0176 §3b);
    /// then the name moves on.
    fn write_point(&mut self, p: Vec2, cx: &mut Context<'_>) {
        let m = *cx.memory;
        let (name, code) = (m.point_name.as_str(), m.point_code.as_str());
        let input = PointCreate {
            layer_id: cx.doc.layers().active().to_owned(),
            p: wire(p),
            z: m.point_z,
            label: (!name.is_empty())
                .then(|| name.to_owned())
                .or_else(|| cx.template_label()),
            color: cx.draft.color_text(),
            attrs: cx.template_attrs(
                (!code.is_empty()).then(|| BTreeMap::from([("Kod".to_owned(), code.to_owned())])),
            ),
            expected_revision: None,
            symbol: cx.template_symbol(),
        };
        let result = point::execute(&mut ExecutionContext::new(cx.doc), input);
        if let Some(written) = points::written(result, cx) {
            self.d.note(written.id, cx);
            self.advance(cx);
        }
    }

    /// Düzelt: the point already there takes the name, code and elevation
    /// given (those given; the others stay), in one step “Nokta düzelt”: a
    /// refusal of either takes both back. Without any value it says so and
    /// the question stays.
    fn correct(&mut self, cx: &mut Context<'_>) {
        let Stage::Question { there, name, .. } = self.stage.clone() else {
            return;
        };
        let m = *cx.memory;
        let (given, code) = (m.point_name.as_str(), m.point_code.as_str());
        if given.is_empty() && code.is_empty() && m.point_z.is_none() {
            let line =
                format!("{LABEL}: düzeltilecek değer yok; Ad (A), Kod (K) ya da Kot (Z) verin.");
            cx.say(Level::Warn, line);
            return;
        }
        let (Some(uid), Some(Entity::Point(old))) = (cx.doc.uid(there), cx.doc.get(there)) else {
            self.back();
            return;
        };
        let (uid, at, old_z, others) = (uid.to_string(), old.p, old.z, old.parts.clone());
        let done = cx.doc.transact(CORRECT_LABEL, |doc| {
            if !given.is_empty() || !code.is_empty() {
                let input = EntitiesSetProperties {
                    uids: vec![uid.clone()],
                    layer_id: None,
                    color: None,
                    line_weight: None,
                    symbol: None,
                    attrs: (!code.is_empty())
                        .then(|| BTreeMap::from([("Kod".to_owned(), Some(code.to_owned()))])),
                    label: (!given.is_empty()).then(|| Some(given.to_owned())),
                    operation: PropertiesOperation::Attributes,
                    expected_revision: None,
                    unlink: false,
                };
                refusal(set::execute(&mut ExecutionContext::new(doc), input))?;
            }
            if let Some(z) = m.point_z
                && old_z != Some(z)
            {
                let input = EntitiesEdit {
                    operation: EditOperation::Elevation,
                    changes: vec![EntityEdit::Update {
                        uid: uid.clone(),
                        // A multi-point object keeps its other points (docs/adr/0174).
                        geometry: EntityGeometry::Point {
                            p: at,
                            z: Some(z),
                            parts: others.clone(),
                        },
                    }],
                    expected_revision: None,
                };
                refusal(edit::execute(&mut ExecutionContext::new(doc), input))?;
            }
            Ok::<(), String>(())
        });
        match done {
            Err(message) => cx.say(Level::Warn, message),
            Ok(()) => {
                let shown = if given.is_empty() {
                    name
                } else {
                    Some(given.to_owned())
                };
                let line = match shown {
                    Some(n) => format!("{LABEL}: “{n}” noktası düzeltildi."),
                    None => format!("{LABEL}: nokta düzeltildi."),
                };
                cx.say(Level::Success, line);
                // Ctrl+Z takes Düzelt back as the tool's own step, as it does a point.
                self.d.note(there.0, cx);
                self.advance(cx);
            }
        }
        self.back();
    }

    /// The name moves on by Yazı's Artır; a name not ending in a number
    /// stays. Ctrl+Z gives the old one back.
    fn advance(&mut self, cx: &mut Context<'_>) {
        let name = cx.memory.point_name;
        self.given_back = (!name.as_str().is_empty()).then_some(name);
        if let Some(next) = increment(name.as_str()).as_deref().and_then(Name::new) {
            cx.memory.point_name = next;
        }
    }

    /// Kot noktası's point through `cad.point.create` (docs/adr/0032,
    /// 0057): the spot elevations' layer with its elevation, text and
    /// attributes, and the current colour explicit in its input (CMD-07; the
    /// web's `writePoint`).
    fn write_spot(&mut self, p: Vec2, z: f64, cx: &mut Context<'_>) {
        // Another active layer would not help: the spot layer's lock is said in the tool's own words (docs/adr/0067).
        let layers = cx.doc.layers();
        if let Some(node) = layers.get(SPOT_LAYER)
            && layers.is_locked(SPOT_LAYER)
        {
            let text = format!(
                "“{}” katmanı kilitli; {SPOT_LABEL} bu katmana yazar. Kilidi Katmanlar panelinden açın.",
                node.name
            );
            cx.say(Level::Warn, text);
            return;
        }
        // A drawing without the spot layer gets it, in the point's own undo step (docs/adr/0067).
        let Ok(opened) = crate::standard_layer::open_if_missing(SPOT_LAYER, "kot noktası", cx)
        else {
            return;
        };
        let input = PointCreate {
            layer_id: SPOT_LAYER.to_owned(),
            p: wire(p),
            z: Some(z),
            // Labelled in the unit it was typed in; kept, and its attribute written, in metres (docs/adr/0165 §2).
            label: Some(fixed(cx.format().from_metres(z), 2)),
            color: cx.draft.color_text(),
            attrs: Some(BTreeMap::from([
                ("Tür".to_owned(), "Kot noktası".to_owned()),
                ("Z (m)".to_owned(), fixed(z, 3)),
            ])),
            expected_revision: None,
            symbol: None,
        };
        let result = point::execute(&mut ExecutionContext::new(cx.doc), input);
        match points::written(result, cx) {
            Some(written) => {
                if let Some(opened) = opened {
                    opened.keep(cx);
                }
                self.d.note(written.id, cx);
            }
            None => {
                if let Some(opened) = opened {
                    opened.drop(cx);
                }
            }
        }
    }
}

/// A command's refusal as its message; nothing for an answer that wrote.
fn refusal<T>(result: CommandResult<T>) -> Result<(), String> {
    match result {
        CommandResult::Failed { error }
        | CommandResult::Conflict { error }
        | CommandResult::NeedsInput { error } => Err(error.message),
        _ => Ok(()),
    }
}

impl Tool for Point {
    /// A point computed by the point calculator or named by `#ad`, as if
    /// clicked (the web's `acceptPoint`): only where a place is asked.
    fn accepts_points(&self) -> bool {
        true
    }

    fn accept_point(&mut self, p: Vec2, cx: &mut Context<'_>) -> bool {
        if self.stage != Stage::Points || self.pending_z.is_some() {
            return false;
        }
        self.accept(p, cx);
        true
    }

    fn id(&self) -> &'static str {
        if self.spot { SPOT_ID } else { ID }
    }

    fn label(&self) -> &'static str {
        if self.spot { SPOT_LABEL } else { LABEL }
    }

    fn activate(&mut self, cx: &mut Context<'_>) -> Flow {
        self.see(cx);
        Flow::Stay
    }

    fn prompt(&self) -> Prompt {
        if self.spot {
            let step = if self.pending_z.is_some() {
                // Typed in the project's unit (docs/adr/0165 §2).
                let unit = self.seen.map(|(_, f)| f).unwrap_or_default();
                format!("kot değerini yazın ({})", unit.length_unit_label())
            } else {
                "nokta konumunu belirtin".to_owned()
            };
            return Prompt::new(SPOT_LABEL, step);
        }
        let (m, format) = self.seen.unwrap_or_default();
        match &self.stage {
            Stage::Z => Prompt::new(LABEL, "noktaların kotunu yazın, metre (boş Enter: kotsuz)"),
            Stage::Field(_) => Prompt::new(LABEL, "değeri yazın"),
            Stage::Question { name, .. } => {
                let there = match name {
                    Some(n) => format!("bu yerde “{n}” noktası var"),
                    None => "bu yerde adsız bir nokta var".to_owned(),
                };
                Prompt::new(LABEL, there)
                    .option("Düzelt", "D")
                    .option("Ekle", "E")
                    .option("Atla", "Esc")
            }
            Stage::Points => {
                let z = m
                    .point_z
                    .map_or_else(|| "yok".to_owned(), |z| format.length(z));
                with_options(Prompt::new(LABEL, "nokta konumunu belirtin"), &m)
                    .option_with("Kot", "Z", z)
            }
        }
    }

    fn point_count(&self) -> usize {
        self.d.pts.len()
    }

    fn snap_from(&self) -> Option<Vec2> {
        self.d.last()
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.d.hover = Some(self.d.constrain(p, cx));
    }

    /// While an elevation, a value or an answer is asked for, a click waits.
    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        if self.pending_z.is_some() || self.stage != Stage::Points {
            return;
        }
        let p = self.d.constrain(p, cx);
        self.accept(p, cx);
    }

    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        if let Some(p) = self.pending_z {
            let Some(z) = cx.typed_length(text) else {
                return false;
            };
            self.write_spot(p, z, cx);
            self.pending_z = None;
            return true;
        }
        let key = upper_tr(crate::js_trim(text));
        let taken = match self.stage {
            Stage::Z => match cx.typed_length(text) {
                Some(z) => {
                    cx.memory.point_z = Some(z);
                    self.back();
                    true
                }
                None => false,
            },
            Stage::Field(_) => false,
            Stage::Question { .. } => self.option(&key, cx),
            Stage::Points => {
                (!self.spot && self.option(&key, cx))
                    || match cx.typed_point(text, self.d.last(), self.d.hover) {
                        Some(p) => {
                            self.accept(p, cx);
                            true
                        }
                        None => false,
                    }
            }
        };
        self.see(cx);
        taken
    }

    /// Ad's or Kod's field answers (the web's `askPointField`): a name given
    /// anew is not taken back by Ctrl+Z.
    fn text_typed(&mut self, text: Option<&str>, cx: &mut Context<'_>) {
        let Stage::Field(field) = self.stage else {
            return;
        };
        if take_field(field, text, cx) && field == Field::Name {
            self.given_back = None;
        }
        self.back();
        self.see(cx);
    }

    /// Enter: an empty one in Kot clears it, in the question it does
    /// nothing; otherwise it leaves (it never holds a point; the web's
    /// `PointInputTool.confirm`), an elevation not yet typed with it.
    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow {
        match self.stage {
            Stage::Z => {
                cx.memory.point_z = None;
                self.back();
                self.see(cx);
                Flow::Stay
            }
            Stage::Points => Flow::Exit,
            _ => Flow::Stay,
        }
    }

    /// Esc: out of Kot, or past the point asked about (Atla); otherwise the tool leaves.
    fn cancel(&mut self, cx: &mut Context<'_>) -> bool {
        match self.stage {
            Stage::Z => self.back(),
            Stage::Question { .. } => {
                cx.say(Level::Info, format!("{LABEL}: atlandı."));
                self.back();
            }
            _ => return false,
        }
        true
    }

    /// Ctrl+Z: out of Kot or the question first (as Esc); then the newest
    /// point or Düzelt as an undo while the drawing has not changed since,
    /// its name given back (105 taken back: the next is 105 again); else the
    /// drawing's undo.
    fn undo_step(&mut self, cx: &mut Context<'_>) -> bool {
        match self.stage {
            Stage::Field(_) => return true,
            Stage::Z | Stage::Question { .. } => {
                self.back();
                return true;
            }
            Stage::Points => {}
        }
        let name = self.given_back.take();
        if !self.d.undo_step(false, cx) {
            return false;
        }
        if let Some(name) = name {
            cx.memory.point_name = name;
        }
        self.see(cx);
        true
    }

    /// “Kot?” beside the point waiting for its elevation; the name Nokta's
    /// next point takes beside the cursor.
    fn preview(&self, _format: &Format) -> Preview {
        let tag = match (self.pending_z, &self.stage, self.d.hover, self.seen) {
            (Some(at), ..) => Some(Tag {
                at,
                lines: vec!["Kot?".to_owned()],
            }),
            (None, Stage::Points, Some(at), Some((m, _))) if !self.spot => {
                let name = m.point_name.as_str();
                (!name.is_empty()).then(|| Tag {
                    at,
                    lines: vec![name.to_owned()],
                })
            }
            _ => None,
        };
        Preview {
            tag,
            ..Preview::default()
        }
    }
}
