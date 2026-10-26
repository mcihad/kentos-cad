//! Kapı, Pencere, Boşluk and the openings' edits by a click (docs/adr/0341 §5, plan/cad.md `CAD-ARCH-15`, `19`). The
//! tools say what was clicked and chosen; `cad.openings.create` and `cad.openings.edit` compute and write, one undo
//! step each, and the plan is the preview.
//!
//! - Kapı, Pencere, Boşluk: with the pointer by a wall, the opening the command would put there is drawn; a click
//!   writes it and the tool stays. Tür (T) the type, Genişlik (E) a width over the type's, Yan (Y) the side it opens
//!   to (Otomatik: the pointer's side), Menteşe (M) its hinge's end, Uzaklık (U) its near jamb that far from the
//!   wall's near end (a number; 0 or Esc takes it off), Orta (O) centred on the edge.
//! - Açıklığı çevir: a click turns the side it opens to, Shift+click its hinge.
//! - Açıklığı taşı: the opening, then a place by a wall (its own or another).
//! - Numarala: Önek (Ö) and Başlangıç (B); Enter numbers the openings selected, or every opening.

use kentos_commands::{
    CommandResult, Entity, NewOpening, OpeningFlip, OpeningForm, OpeningHand, OpeningSide,
    OpeningsCreate, OpeningsEdit, OpeningsOperation, builtin_opening_types,
};
use kentos_domain::Slot;
use kentos_geometry_core::tools::point_text::js_trim;
use kentos_native_application::geometry::shape;
use kentos_native_application::{ExecutionContext, openings_create, openings_edit};

use crate::Vec2;
use crate::edge::Outline;
use crate::format::Format;
use crate::log::Level;
use crate::points;
use crate::prompt::{Prompt, upper_tr};
use crate::tool::{Context, Cursor, Flow, OptionChoice, Pointer, Preview, Tag, Tone, Tool};

#[path = "opening_tool_placement.rs"]
mod placement;

pub const DOOR_ID: &str = "door";
pub const WINDOW_ID: &str = "window";
pub const OPENING_ID: &str = "opening";
pub const FLIP_ID: &str = "openingFlip";
pub const MOVE_ID: &str = "openingMove";
pub const NUMBER_ID: &str = "openingNumber";

/// How far from a wall's face the pointer may be to put an opening in it, metres.
const NEAR_WALL: f64 = 1.0;

/// The icon of an opening type in Tür's menu.
fn type_icon(
    form: OpeningForm,
    leaves: u8,
    operation: kentos_commands::OpeningOperation,
) -> &'static str {
    use kentos_commands::OpeningOperation as Op;
    match (form, leaves, operation) {
        (OpeningForm::Door, _, Op::Slide) => "doorSliding",
        (OpeningForm::Door, 2, _) => "doorDouble",
        (OpeningForm::Door, _, _) => "door",
        (OpeningForm::Window, _, Op::Fixed) => "windowFixed",
        (OpeningForm::Window, _, _) => "window",
        (OpeningForm::Opening, _, _) => "wallOpening",
    }
}

/// A number the tool waits for after its key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Ask {
    Width,
    Distance,
    /// A type's name, typed after T: words (Space types a space).
    Type,
}

/// Kapı, Pencere or Boşluk.
#[derive(Clone, Debug)]
pub struct OpeningTool {
    form: OpeningForm,
    /// The type's id; none: the form's first.
    kind: Option<String>,
    width: Option<f64>,
    side: Option<OpeningSide>,
    hand: OpeningHand,
    distance: Option<f64>,
    middle: bool,
    ask: Option<Ask>,
    hover: Option<(Slot, Vec2)>,
    /// The opening the plan would write, drawn; or why it would not.
    drawn: Preview,
    why: Option<String>,
    placement: placement::Placement,
}

impl OpeningTool {
    fn with(form: OpeningForm) -> Self {
        Self {
            form,
            kind: None,
            width: None,
            side: None,
            hand: OpeningHand::Start,
            distance: None,
            middle: false,
            ask: None,
            hover: None,
            drawn: Preview::default(),
            why: None,
            placement: Default::default(),
        }
    }

    pub fn door() -> Self {
        Self::with(OpeningForm::Door)
    }
    pub fn window() -> Self {
        Self::with(OpeningForm::Window)
    }
    pub fn opening() -> Self {
        Self::with(OpeningForm::Opening)
    }

    fn label(&self) -> &'static str {
        match self.form {
            OpeningForm::Door => "Kapı",
            OpeningForm::Window => "Pencere",
            OpeningForm::Opening => "Boşluk",
        }
    }

    /// The type chosen, or the form's first.
    fn type_name(&self) -> String {
        let all = builtin_opening_types();
        self.kind
            .as_deref()
            .and_then(|k| all.iter().find(|t| t.id == k))
            .or_else(|| all.iter().find(|t| t.form == self.form))
            .map_or_else(String::new, |t| t.name.clone())
    }

    fn create_input(&self, slot: Slot, at: Vec2, cx: &Context<'_>) -> Option<OpeningsCreate> {
        let host = cx.doc.uid(slot)?.to_string();
        let span = self
            .placement
            .first
            .map(|(_, a)| [points::wire(a), points::wire(at)]);
        let measured = span.is_none() && !self.middle;
        Some(OpeningsCreate {
            layer_id: cx.doc.layers().active().to_owned(),
            openings: vec![NewOpening {
                host,
                point: Some(points::wire(if span.is_some() {
                    self.placement.side_point.unwrap_or(at)
                } else {
                    at
                })),
                distance: if measured { self.distance } else { None },
                from: (measured && self.distance.is_some()).then_some(self.placement.from),
                distance_reference: (measured && self.distance.is_some())
                    .then_some(self.placement.reference),
                middle: span.is_none() && self.middle,
                span,
                kind: self.kind.clone(),
                form: self.kind.is_none().then_some(self.form),
                width: span.is_none().then_some(self.width).flatten(),
                hand: Some(self.hand),
                side: self.side,
                ..NewOpening::default()
            }],
            color: cx.draft.color_text(),
            line_weight: cx.draft.line_weight,
            expected_revision: None,
        })
    }

    /// The wall nearest the pointer within [`NEAR_WALL`] (or the pick tolerance, when it is wider): a door goes by a
    /// wall, its side the pointer's.
    fn wall_at(at: Vec2, cx: &Context<'_>) -> Option<Slot> {
        cx.spatial
            .hits(at, cx.pick_tolerance().max(NEAR_WALL))
            .into_iter()
            .find(|s| matches!(cx.doc.get(*s), Some(Entity::Wall(_))))
    }

    /// The plan at the hover, drawn.
    fn see(&mut self, cx: &Context<'_>) {
        self.drawn = Preview::default();
        self.why = None;
        self.placement.info.clear();
        self.placement.format = cx.format();
        if self.placement.copy {
            return;
        }
        let Some((slot, at)) = self.hover else { return };
        if let Some(wall) = cx.doc.get(slot) {
            let mut host = Outline::of(&shape(wall), None, 1.0, Tone::Snap);
            for stroke in &mut host.strokes {
                stroke.dash = Some([5.0, 4.0]);
            }
            self.drawn.strokes.extend(host.strokes);
        }
        if self.placement.two_points && self.placement.first.is_none() {
            return;
        }
        if self.placement.first.is_some_and(|(_, p)| p == at) {
            return;
        }
        let Some(input) = self.create_input(slot, at, cx) else {
            return;
        };
        match openings_create::plan_in(cx.doc, &input) {
            CommandResult::Completed { output, .. } => {
                for e in &output.openings {
                    let o = Outline::of(&shape(e), None, 2.0, Tone::Accent);
                    self.drawn.strokes.extend(o.strokes);
                    if let Entity::Opening(o) = e {
                        self.describe(&o.opening, cx);
                    }
                }
            }
            CommandResult::Failed { error }
            | CommandResult::Conflict { error }
            | CommandResult::NeedsInput { error } => self.why = Some(error.message),
            CommandResult::Queued { .. } | CommandResult::Cancelled => {}
        }
    }

    fn side_label(&self) -> &'static str {
        match self.side {
            None => "Otomatik",
            Some(OpeningSide::Left) => "Sol",
            Some(OpeningSide::Right) => "Sağ",
        }
    }
}

impl Tool for OpeningTool {
    fn activate(&mut self, cx: &mut Context<'_>) -> Flow {
        self.see(cx);
        Flow::Stay
    }

    fn id(&self) -> &'static str {
        match self.form {
            OpeningForm::Door => DOOR_ID,
            OpeningForm::Window => WINDOW_ID,
            OpeningForm::Opening => OPENING_ID,
        }
    }

    fn label(&self) -> &'static str {
        OpeningTool::label(self)
    }

    fn cursor(&self) -> Cursor {
        Cursor::Pick
    }

    fn point_count(&self) -> usize {
        usize::from(self.placement.first.is_some())
    }

    fn undo_step(&mut self, cx: &mut Context<'_>) -> bool {
        let taken = self.placement.first.take().is_some();
        if taken {
            self.see(cx);
        }
        taken
    }

    fn prompt(&self) -> Prompt {
        if self.placement.copy {
            return Prompt::new(self.label(), "tipi ve eni alınacak aynı tür açıklığı seçin")
                .option("Geri", "G");
        }
        match self.ask {
            Some(Ask::Width) => {
                return Prompt::new(self.label(), "açıklığın genişliğini yazın")
                    .option("Geri", "G");
            }
            Some(Ask::Type) => {
                return Prompt::new(self.label(), "açıklık türünün adını yazın")
                    .option("Geri", "G");
            }
            Some(Ask::Distance) => {
                return Prompt::new(
                    self.label(),
                    "yakın pervazın duvarın ucuna uzaklığını yazın (0: yok)",
                )
                .option("Geri", "G");
            }
            None => {}
        }
        let note = if self.placement.two_points {
            if self.placement.first.is_some() {
                "aynı duvarda ikinci pervaz noktasını seçin"
            } else {
                "ilk pervaz noktasını duvarda seçin"
            }
        } else {
            "açıklığın konacağı yere duvarın yanından tıklayın"
        };
        let mut p = Prompt::new(self.label(), note)
            .option_with("Yöntem", "B", self.placement.method_label())
            .option_with("Tür", "T", self.type_name());
        if !self.placement.two_points && (self.form != OpeningForm::Opening || self.width.is_some())
        {
            p = p.option_with(
                "Genişlik",
                "E",
                self.width
                    .map_or_else(|| "Türün".to_owned(), |w| self.placement.format.length(w)),
            );
        }
        if self.form == OpeningForm::Door {
            p = p.option_with("Yan", "Y", self.side_label()).option_with(
                "Menteşe",
                "M",
                if self.hand == OpeningHand::End {
                    "Son"
                } else {
                    "Baş"
                },
            );
        }
        if self.placement.two_points {
            return p.option("Bitir", "Enter");
        }
        p.option_with(
            "Uzaklık",
            "U",
            self.distance
                .map_or_else(|| "Yok".to_owned(), |d| self.placement.format.length(d)),
        )
        .option_with("Duvar ucu", "F", self.placement.end_label())
        .option_with("Mesafe referansı", "R", self.placement.reference_label())
        .toggle("Orta", "O", self.middle)
        .option("Bitir", "Enter")
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        if self.placement.copy {
            cx.selection.set_hover(self.sample_at(p.raw, cx));
            return;
        }
        self.placement.side_point = Some(p.raw);
        let hit = self
            .placement
            .first
            .map(|(s, _)| s)
            .or_else(|| Self::wall_at(p.raw, cx));
        cx.selection.set_hover(hit);
        self.hover = hit.map(|s| {
            (
                s,
                if self.placement.two_points {
                    p.world
                } else {
                    p.raw
                },
            )
        });
        self.see(cx);
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        if self.placement.copy {
            if let Some(slot) = self.sample_at(p.raw, cx) {
                self.take_sample(slot, cx);
            } else {
                cx.say(Level::Warn, "Aynı türden bir açıklık seçin.");
            }
            return;
        }
        self.placement.side_point = Some(p.raw);
        let Some(slot) = Self::wall_at(p.raw, cx) else {
            cx.say(
                Level::Warn,
                format!("{}: bir duvarın yanına tıklayın.", self.label()),
            );
            return;
        };
        if self.placement.two_points {
            if let Some((first, _)) = self.placement.first {
                if first != slot {
                    cx.say(
                        Level::Warn,
                        "İkinci pervaz aynı duvarda olmalı; Escape ilk noktayı geri alır.",
                    );
                    return;
                }
            } else {
                self.placement.first = Some((slot, p.world));
                self.hover = Some((slot, p.world));
                self.see(cx);
                return;
            }
        }
        let at = if self.placement.two_points {
            p.world
        } else {
            p.raw
        };
        self.hover = Some((slot, at));
        let Some(input) = self.create_input(slot, at, cx) else {
            return;
        };
        let result = openings_create::execute(&mut ExecutionContext::new(cx.doc), input);
        if points::written(result, cx).is_some() {
            self.placement.first = None;
            cx.say(Level::Success, format!("{} eklendi.", self.label()));
        }
        self.see(cx);
    }

    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        let key = upper_tr(js_trim(text));
        if self.ask.is_none() && self.placement_input(&key, cx) {
            return true;
        }
        if let Some(ask) = self.ask {
            if key == "G" {
                self.ask = None;
                return true;
            }
            if ask == Ask::Type {
                let folded = js_trim(text).to_lowercase();
                match builtin_opening_types()
                    .into_iter()
                    .filter(|t| t.form == self.form)
                    .find(|t| t.id == js_trim(text) || t.name.to_lowercase() == folded)
                {
                    Some(t) => {
                        self.kind = Some(t.id);
                        self.width = None;
                    }
                    None => cx.say(
                        Level::Warn,
                        format!(
                            "“{}” adında {} türü yok.",
                            js_trim(text),
                            self.label().to_lowercase()
                        ),
                    ),
                }
                self.ask = None;
                self.see(cx);
                return true;
            }
            let Some(v) = cx.typed_length(text) else {
                cx.say(Level::Warn, "Bir uzunluk yazın.");
                return true;
            };
            match ask {
                Ask::Width if v > 0.0 && v <= 100.0 => self.width = Some(v),
                Ask::Width => cx.say(
                    Level::Warn,
                    "Açıklığın genişliği sıfırdan büyük ve en çok 100 m olmalı.",
                ),
                Ask::Distance if v > 0.0 => self.distance = Some(v),
                Ask::Distance => self.distance = None,
                Ask::Type => {}
            }
            self.ask = None;
            self.see(cx);
            return true;
        }
        match key.as_str() {
            "T" => self.ask = Some(Ask::Type),
            "E" => self.ask = Some(Ask::Width),
            "U" => self.ask = Some(Ask::Distance),
            "O" => self.middle = !self.middle,
            "Y" if self.form == OpeningForm::Door => {
                self.side = match self.side {
                    None => Some(OpeningSide::Left),
                    Some(OpeningSide::Left) => Some(OpeningSide::Right),
                    Some(OpeningSide::Right) => None,
                }
            }
            "M" if self.form == OpeningForm::Door => self.hand = self.hand.other(),
            _ => {
                if self.placement.two_points
                    && !self.placement.copy
                    && let Some(p) = cx.typed_point(
                        text,
                        self.placement.first.map(|(_, p)| p),
                        self.hover.map(|(_, p)| p),
                    )
                {
                    self.pointer_down(&Pointer::new(p, cx.view.to_screen(p), false, None), cx);
                    return true;
                }
                return false;
            }
        }
        self.see(cx);
        true
    }

    fn option_choices(&self, key: &str) -> Vec<OptionChoice> {
        if let Some(choices) = self.placement_choices(key) {
            return choices;
        }
        let choice =
            |typed: String, label: String, icon: &'static str, checked: bool| OptionChoice {
                typed,
                label,
                icon: Some(icon),
                preview: None,
                checked,
                command: None,
            };
        match key {
            "T" => {
                let current = self.type_name();
                builtin_opening_types()
                    .into_iter()
                    .filter(|t| t.form == self.form)
                    .map(|t| {
                        let icon = type_icon(t.form, t.leaves, t.operation);
                        choice(t.id.clone(), t.name.clone(), icon, t.name == current)
                    })
                    .collect()
            }
            "Y" if self.form == OpeningForm::Door => [
                (None, "Otomatik", "openingSideAuto"),
                (Some(OpeningSide::Left), "Sol", "openingSideLeft"),
                (Some(OpeningSide::Right), "Sağ", "openingSideRight"),
            ]
            .into_iter()
            .map(|(s, l, i)| choice(l.to_owned(), l.to_owned(), i, s == self.side))
            .collect(),
            _ => Vec::new(),
        }
    }

    fn settings(&self, f: &Format) -> Vec<crate::tool_settings::Setting> {
        self.placement_settings(f)
    }

    fn configure(&mut self, key: &str, value: &str, cx: &mut Context<'_>) -> bool {
        self.apply_settings(key, value, cx)
    }

    fn choose_option(&mut self, key: &str, typed: &str, cx: &mut Context<'_>) -> bool {
        if matches!(key, "B" | "F" | "R") {
            return self.placement_configure(key, typed, cx);
        }
        match key {
            "T" => {
                if builtin_opening_types()
                    .iter()
                    .any(|t| t.id == typed && t.form == self.form)
                {
                    self.kind = Some(typed.to_owned());
                    self.width = None;
                } else {
                    return false;
                }
            }
            "Y" => {
                self.side = match typed {
                    "Sol" => Some(OpeningSide::Left),
                    "Sağ" => Some(OpeningSide::Right),
                    _ => None,
                }
            }
            _ => return false,
        }
        self.see(cx);
        true
    }

    fn takes_words(&self) -> bool {
        self.ask == Some(Ask::Type)
    }

    fn cancel(&mut self, cx: &mut Context<'_>) -> bool {
        if self.placement.first.take().is_some() || std::mem::take(&mut self.placement.copy) {
            self.see(cx);
            return true;
        }
        if self.ask.take().is_some() {
            return true;
        }
        if self.distance.take().is_some() {
            self.see(cx);
            return true;
        }
        false
    }

    fn confirm(&mut self, _cx: &mut Context<'_>) -> Flow {
        Flow::Exit
    }

    fn preview(&self, _format: &Format) -> Preview {
        let tag = self.hover.map(|(_, at)| Tag {
            at,
            lines: match &self.why {
                Some(why) => placement::error_lines(self.label(), why),
                None => {
                    let mut lines = vec![format!("{} · {}", self.label(), self.type_name())];
                    lines.extend(self.placement.info.clone());
                    lines
                }
            },
        });
        Preview {
            squares: self
                .placement
                .first
                .map(|(_, p)| vec![p])
                .unwrap_or_default(),
            tag,
            tag_tone: if self.why.is_some() {
                Tone::Danger
            } else {
                Tone::Accent
            },
            ..self.drawn.clone()
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Action {
    Flip,
    Move,
    Number,
}

/// Açıklığı çevir, Açıklığı taşı, Numarala.
#[derive(Clone, Debug)]
pub struct OpeningEditTool {
    action: Action,
    /// Taşı's opening.
    picked: Option<Slot>,
    prefix: String,
    start: u32,
    /// What Numarala waits for: the prefix (true) or the first number.
    asking: Option<bool>,
    hover: Option<(Slot, Vec2)>,
    drawn: Preview,
}

impl OpeningEditTool {
    fn with(action: Action) -> Self {
        Self {
            action,
            picked: None,
            prefix: "K".into(),
            start: 1,
            asking: None,
            hover: None,
            drawn: Preview::default(),
        }
    }

    pub fn flip() -> Self {
        Self::with(Action::Flip)
    }
    pub fn mover() -> Self {
        Self::with(Action::Move)
    }
    pub fn number() -> Self {
        Self::with(Action::Number)
    }

    fn name(&self) -> &'static str {
        match self.action {
            Action::Flip => "Açıklığı çevir",
            Action::Move => "Açıklığı taşı",
            Action::Number => "Numarala",
        }
    }

    fn step(&self) -> &'static str {
        match (self.action, self.picked.is_some()) {
            (Action::Flip, _) => "açılışı çevrilecek açıklığa tıklayın (Shift: menteşe)",
            (Action::Move, false) => "taşınacak açıklığa tıklayın",
            (Action::Move, true) => "yeni yerine duvarın yanından tıklayın",
            (Action::Number, _) => "Enter: seçili açıklıklar, seçim yoksa hepsi numaralanır",
        }
    }

    fn of_kind(at: Vec2, cx: &Context<'_>, opening: bool) -> Option<Slot> {
        let tol = if opening {
            cx.pick_tolerance()
        } else {
            cx.pick_tolerance().max(NEAR_WALL)
        };
        cx.spatial
            .hits(at, tol)
            .into_iter()
            .find(|s| match cx.doc.get(*s) {
                Some(Entity::Opening(_)) => opening,
                Some(Entity::Wall(_)) => !opening,
                _ => false,
            })
    }

    fn run(&self, uids: Vec<String>, operation: OpeningsOperation, cx: &mut Context<'_>) -> bool {
        let input = OpeningsEdit {
            uids,
            operation,
            expected_revision: None,
        };
        let result = openings_edit::execute(&mut ExecutionContext::new(cx.doc), input);
        points::written(result, cx).is_some()
    }

    fn see(&mut self, cx: &Context<'_>) {
        let mut out = Preview::default();
        if let Some(e) = self.picked.and_then(|s| cx.doc.get(s)) {
            let o = Outline::of(&shape(e), Some([6.0, 4.0]), 2.5, Tone::Accent);
            out.strokes.extend(o.strokes);
        }
        self.drawn = out;
    }
}

impl Tool for OpeningEditTool {
    fn id(&self) -> &'static str {
        match self.action {
            Action::Flip => FLIP_ID,
            Action::Move => MOVE_ID,
            Action::Number => NUMBER_ID,
        }
    }

    fn label(&self) -> &'static str {
        self.name()
    }

    fn cursor(&self) -> Cursor {
        Cursor::Pick
    }

    fn point_count(&self) -> usize {
        0
    }

    fn undo_step(&mut self, _cx: &mut Context<'_>) -> bool {
        false
    }

    fn prompt(&self) -> Prompt {
        match (self.action, self.asking) {
            (Action::Number, Some(true)) => {
                Prompt::new(self.name(), "numaraların önekini yazın").option("Geri", "G")
            }
            (Action::Number, Some(false)) => {
                Prompt::new(self.name(), "ilk numarayı yazın").option("Geri", "G")
            }
            (Action::Number, None) => Prompt::new(self.name(), self.step())
                .option_with("Önek", "Ö", self.prefix.clone())
                .option_with("Başlangıç", "B", self.start.to_string())
                .option("Numarala", "Enter"),
            _ => Prompt::new(self.name(), self.step()).option("Bitir", "Enter"),
        }
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        let want_opening = !(self.action == Action::Move && self.picked.is_some());
        let hit = Self::of_kind(p.raw, cx, want_opening);
        cx.selection.set_hover(hit);
        self.hover = hit.map(|s| (s, p.raw));
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        match self.action {
            Action::Number => {}
            Action::Flip => {
                let Some(slot) = Self::of_kind(p.raw, cx, true) else {
                    cx.say(
                        Level::Warn,
                        "Açıklığı çevir: bir kapıya, pencereye ya da boşluğa tıklayın.",
                    );
                    return;
                };
                let Some(uid) = cx.doc.uid(slot).map(|u| u.to_string()) else {
                    return;
                };
                let what = if p.shift {
                    OpeningFlip::Hand
                } else {
                    OpeningFlip::Side
                };
                if self.run(vec![uid], OpeningsOperation::Flip { what }, cx) {
                    let said = if p.shift {
                        "Menteşe çevrildi."
                    } else {
                        "Açılış yanı çevrildi."
                    };
                    cx.say(Level::Success, said);
                }
            }
            Action::Move => match self.picked {
                None => {
                    let Some(slot) = Self::of_kind(p.raw, cx, true) else {
                        cx.say(
                            Level::Warn,
                            "Açıklığı taşı: bir kapıya, pencereye ya da boşluğa tıklayın.",
                        );
                        return;
                    };
                    self.picked = Some(slot);
                }
                Some(slot) => {
                    let Some(wall) = Self::of_kind(p.raw, cx, false) else {
                        cx.say(Level::Warn, "Açıklığı taşı: bir duvarın yanına tıklayın.");
                        return;
                    };
                    let (Some(uid), Some(to)) = (cx.doc.uid(slot), cx.doc.uid(wall)) else {
                        return;
                    };
                    let op = OpeningsOperation::Rehost {
                        to: to.to_string(),
                        point: Some(points::wire(p.raw)),
                    };
                    if self.run(vec![uid.to_string()], op, cx) {
                        cx.say(Level::Success, "Açıklık taşındı.");
                        self.picked = None;
                    }
                }
            },
        }
        self.see(cx);
    }

    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        if self.action != Action::Number {
            return false;
        }
        let key = upper_tr(js_trim(text));
        match self.asking {
            Some(_) if key == "G" => self.asking = None,
            Some(true) => {
                self.prefix = js_trim(text).to_owned();
                self.asking = None;
            }
            Some(false) => match js_trim(text).parse::<u32>() {
                Ok(n) => {
                    self.start = n;
                    self.asking = None;
                }
                Err(_) => cx.say(Level::Warn, "İlk numara bir tam sayı olmalı."),
            },
            None => match key.as_str() {
                "Ö" | "O" => self.asking = Some(true),
                "B" => self.asking = Some(false),
                _ => return false,
            },
        }
        true
    }

    fn takes_words(&self) -> bool {
        self.asking == Some(true)
    }

    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow {
        if self.action != Action::Number {
            return Flow::Exit;
        }
        let uids: Vec<String> = cx
            .selection
            .ids()
            .iter()
            .filter(|s| matches!(cx.doc.get(**s), Some(Entity::Opening(_))))
            .filter_map(|s| cx.doc.uid(*s).map(|u| u.to_string()))
            .collect();
        let op = OpeningsOperation::Renumber {
            prefix: self.prefix.clone(),
            start: self.start,
            form: None,
        };
        let n = if uids.is_empty() {
            cx.doc
                .entities()
                .filter(|e| matches!(e, Entity::Opening(_)))
                .count()
        } else {
            uids.len()
        };
        if self.run(uids, op, cx) {
            cx.say(Level::Success, format!("{n} açıklık numaralandı."));
        }
        Flow::Exit
    }

    fn cancel(&mut self, cx: &mut Context<'_>) -> bool {
        if self.asking.take().is_some() || self.picked.take().is_some() {
            self.see(cx);
            return true;
        }
        false
    }

    fn preview(&self, _format: &Format) -> Preview {
        Preview {
            tag: self.hover.map(|(_, at)| Tag {
                at,
                lines: vec![self.name().to_owned(), self.step().to_owned()],
            }),
            ..self.drawn.clone()
        }
    }
}
