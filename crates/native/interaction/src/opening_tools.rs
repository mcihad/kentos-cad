//! Kapı, Pencere, Boşluk and the openings' edits by a click (docs/adr/0341 §5, plan/cad.md `CAD-ARCH-15`, `19`). The
//! tools say what was clicked and chosen; `cad.openings.create` and `cad.openings.edit` compute and write, one undo
//! step each, and the plan is the preview.
//!
//! - Kapı, Pencere, Boşluk, Niş, Baca (docs/adr/0719): with the pointer by a wall, the opening the command would put
//!   there is drawn; a click writes it and the tool stays. A niche or flue takes its face from the side of the wall
//!   the pointer is on (Yüz, Y), its depth (Derinlik, D), a flue its distance from that face (Yüzden, Z; Ortada:
//!   centred); a bare opening, a niche or a flue its sill and head (Alt, L; Üst, Ü), which say whether plan cuts it. Numara (N, docs/adr/0714): Otomatik numbers it after the last numbered opening of
//!   its form (K3 → K4; none yet: K1, P1, B1), a number typed starts there, Yok leaves it unnumbered. Tür (T) the type, Genişlik (E) a width over the type's, Yan (Y) the side it opens
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

#[path = "opening_tool_doors.rs"]
mod doors;

pub const DOOR_ID: &str = "door";
pub const WINDOW_ID: &str = "window";
pub const OPENING_ID: &str = "opening";
pub const NICHE_ID: &str = "niche";
pub const FLUE_ID: &str = "flue";
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
        (OpeningForm::Door, _, Op::Revolving) => "doorRevolving",
        (OpeningForm::Door, _, Op::Pocket) => "doorPocket",
        (OpeningForm::Door, _, Op::Folding) => "doorFolding",
        (OpeningForm::Door, 2, _) => "doorDouble",
        (OpeningForm::Door, _, _) => "door",
        (OpeningForm::Window, _, Op::Fixed) => "windowFixed",
        (OpeningForm::Window, _, _) => "window",
        (OpeningForm::Opening, _, _) => "wallOpening",
        (OpeningForm::Niche, _, _) => "wallNiche",
        (OpeningForm::Flue, _, _) => "wallFlue",
    }
}

/// A number the tool waits for after its key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Ask {
    Width,
    Distance,
    /// A type's name, typed after T: words (Space types a space).
    Type,
    /// A niche's or flue's depth (D), a flue's distance from its face (Z), the sill (L) and the head (Ü).
    Depth,
    Inset,
    Sill,
    Head,
    /// A door's leaves (K), a revolving door's drum (J), a pocket door's lip (P), two sliding leaves' overlap (S)
    /// (docs/adr/0777 §5).
    Leaves,
    Drum,
    Lip,
    Overlap,
}

/// Kapı, Pencere or Boşluk.
#[derive(Clone, Debug)]
pub struct OpeningTool {
    form: OpeningForm,
    /// What number the next opening takes (docs/adr/0714).
    numbering: Numbering,
    /// That number, as last worked out (the settings show it).
    next: Option<String>,
    /// The type's id; none: the form's first.
    kind: Option<String>,
    /// The project's door and window types (docs/adr/0772), as the drawing had them when last seen: listed first,
    /// before KentOS's own.
    project: Vec<kentos_commands::OpeningTypeDef>,
    width: Option<f64>,
    side: Option<OpeningSide>,
    hand: OpeningHand,
    distance: Option<f64>,
    middle: bool,
    /// Over the type's: a niche's or flue's depth, a flue's distance from its face (`Some(None)`: centred), the sill
    /// and the head (docs/adr/0719).
    depth: Option<f64>,
    inset: Option<Option<f64>>,
    sill: Option<f64>,
    head: Option<f64>,
    /// A door's operation, leaves and special door's construction over its type's (docs/adr/0777 §5).
    moves: doors::Moves,
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
            numbering: Numbering::Auto,
            next: None,
            kind: None,
            project: Vec::new(),
            width: None,
            side: None,
            hand: OpeningHand::Start,
            distance: None,
            middle: false,
            depth: None,
            inset: None,
            sill: None,
            head: None,
            moves: doors::Moves::default(),
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
    /// Niş and Baca: unnumbered at first (a niche or flue rarely has a mark; Numara turns it on).
    pub fn niche() -> Self {
        Self {
            numbering: Numbering::Off,
            ..Self::with(OpeningForm::Niche)
        }
    }
    pub fn flue() -> Self {
        Self {
            numbering: Numbering::Off,
            ..Self::with(OpeningForm::Flue)
        }
    }

    fn label(&self) -> &'static str {
        self.form.label()
    }

    /// The form's first number's prefix: Kapı K, Pencere P, Boşluk B, Niş N, Baca BC.
    fn prefix(&self) -> &'static str {
        match self.form {
            OpeningForm::Door => "K",
            OpeningForm::Window => "P",
            OpeningForm::Opening => "B",
            OpeningForm::Niche => "N",
            OpeningForm::Flue => "BC",
        }
    }

    /// The types offered: the project's (docs/adr/0772), then KentOS's own.
    fn all_types(&self) -> Vec<kentos_commands::OpeningTypeDef> {
        let mut all = self.project.clone();
        all.extend(builtin_opening_types());
        all
    }

    /// The type chosen, or the form's first of KentOS's own: what `cad.openings.create` takes when it names none
    /// (a project's type is only ever chosen, docs/adr/0777 §5).
    fn kind_def(&self) -> Option<kentos_commands::OpeningTypeDef> {
        let all = self.all_types();
        self.kind
            .as_deref()
            .and_then(|k| all.iter().find(|t| t.id == k))
            .cloned()
            .or_else(|| builtin_opening_types().into_iter().find(|t| t.form == self.form))
    }

    /// Whether Yüz, Alt and Üst apply: a form plan cuts by its heights (docs/adr/0719 §2).
    fn by_height(&self) -> bool {
        self.form.cut_by_height()
    }

    /// The number the next opening takes (docs/adr/0714): none with Yok; the one typed, else after the last numbered
    /// opening of its form (the highest slot), else the form's first; on until no opening of its form has it.
    fn next_number(&self, doc: &kentos_domain::Document) -> Option<String> {
        let mut used = std::collections::HashSet::new();
        let mut last: Option<(u32, String)> = None;
        for e in doc.entities() {
            if let Entity::Opening(o) = e
                && o.opening.form == self.form
                && let Some(n) = o.opening.number.as_ref().filter(|n| !n.is_empty())
            {
                used.insert(n.clone());
                if last.as_ref().is_none_or(|(id, _)| e.base().id > *id) {
                    last = Some((e.base().id, n.clone()));
                }
            }
        }
        let mut number = match &self.numbering {
            Numbering::Off => return None,
            Numbering::Next(n) => n.clone(),
            Numbering::Auto => last.map_or_else(|| format!("{}1", self.prefix()), |(_, n)| next_after(&n)),
        };
        // Bounded: as many steps as there are numbers in use and one more.
        for _ in 0..=used.len() {
            if !used.contains(&number) {
                break;
            }
            number = next_after(&number);
        }
        Some(number)
    }

    /// Numara's words in the settings.
    fn number_shown(&self) -> String {
        match (&self.numbering, &self.next) {
            (Numbering::Off, _) => "Yok".into(),
            (Numbering::Next(n), _) => n.clone(),
            (Numbering::Auto, Some(n)) => format!("{n} · otomatik"),
            (Numbering::Auto, None) => "Otomatik".into(),
        }
    }

    /// Numara (N) given: empty or “otomatik” Otomatik, “yok” or “-” none, else the number to start at.
    fn set_numbering(&mut self, value: &str, cx: &mut Context<'_>) -> bool {
        let value = js_trim(value);
        let folded = value.to_lowercase();
        self.numbering = if value.is_empty() || folded == "otomatik" || folded.ends_with("· otomatik") {
            Numbering::Auto
        } else if folded == "yok" || value == "-" {
            Numbering::Off
        } else if value.chars().count() > 128 {
            cx.say(Level::Warn, "Numara en çok 128 karakter olabilir.");
            return false;
        } else {
            Numbering::Next(value.to_owned())
        };
        self.see(cx);
        true
    }

    /// The type chosen, or the form's first.
    fn type_name(&self) -> String {
        self.kind_def().map_or_else(String::new, |t| t.name)
    }

    fn create_input(&self, slot: Slot, at: Vec2, cx: &Context<'_>) -> Option<OpeningsCreate> {
        let host = cx.doc.uid(slot)?.to_string();
        let span = self
            .placement
            .first
            .map(|(_, a)| [points::wire(a), points::wire(at)]);
        let measured = span.is_none() && !self.middle;
        let (operation, leaves, build) = self.door_input();
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
                sill: self.sill,
                head: self.head,
                depth: self.depth.filter(|_| self.form.is_recess()),
                inset: self.inset.flatten().filter(|_| self.form == OpeningForm::Flue),
                // Numara (docs/adr/0714), explicit in the input (CMD-07).
                number: self.next_number(cx.doc),
                operation,
                leaves,
                build,
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
        self.project = cx
            .doc
            .opening_types()
            .iter()
            .map(|t| kentos_commands::OpeningTypeDef {
                id: t.id.clone(),
                name: t.name.clone(),
                form: t.form,
                width: t.width,
                leaves: t.leaves,
                operation: t.operation,
                swing: t.swing,
                frame: t.frame,
                sill: t.sill,
                head: t.head,
                depth: t.depth,
            })
            .collect();
        self.next = self.next_number(cx.doc);
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
                crate::wall_dimensions::append_followers(&mut self.drawn, &output.followers, cx);
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

    /// Derinlik's words: the one typed, else the type's.
    fn depth_shown(&self) -> String {
        let f = &self.placement.format;
        match self.depth.or_else(|| self.kind_def().and_then(|t| t.depth)) {
            Some(d) if self.depth.is_some() => f.length(d),
            Some(d) => format!("{} · türün", f.length(d)),
            None => "Yok".into(),
        }
    }

    /// Yüzden's words: Ortada, or the distance typed.
    fn inset_shown(&self) -> String {
        match self.inset.flatten() {
            Some(v) => self.placement.format.length(v),
            None => "Ortada".into(),
        }
    }

    /// Alt's (`sill`) or Üst's words: the one typed, else the type's (Döşeme, Tam boy when it has none).
    fn height_shown(&self, sill: bool) -> String {
        let f = &self.placement.format;
        let own = if sill { self.sill } else { self.head };
        let typed = self.kind_def().and_then(|t| if sill { t.sill } else { t.head });
        match (own, typed) {
            (Some(v), _) => f.length(v),
            (None, Some(v)) => format!("{} · türün", f.length(v)),
            (None, None) if sill => "Döşeme".into(),
            (None, None) => "Tam boy".into(),
        }
    }

    /// A depth, inset, sill or head typed: `false` (and said why) when it cannot be.
    fn take_height(&mut self, ask: Ask, v: f64, cx: &mut Context<'_>) -> bool {
        match ask {
            Ask::Depth if v > 0.0 && v <= kentos_commands::MAX_RECESS_DEPTH => self.depth = Some(v),
            Ask::Depth => {
                cx.say(Level::Warn, "Derinlik sıfırdan büyük, en çok 10 m olmalı.");
                return false;
            }
            Ask::Inset if v > 0.0 && v <= kentos_commands::MAX_RECESS_DEPTH => self.inset = Some(Some(v)),
            Ask::Inset => {
                cx.say(Level::Warn, "Yüzden uzaklık sıfırdan büyük olmalı; ortada için boş bırakın.");
                return false;
            }
            Ask::Sill | Ask::Head if v.abs() > 1000.0 => {
                cx.say(Level::Warn, "Kot en çok 1000 m olabilir.");
                return false;
            }
            Ask::Sill => self.sill = Some(v),
            Ask::Head => self.head = Some(v),
            _ => return false,
        }
        true
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
            OpeningForm::Niche => NICHE_ID,
            OpeningForm::Flue => FLUE_ID,
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
            Some(Ask::Depth) => {
                return Prompt::new(self.label(), "duvarın içine ne kadar girdiğini yazın")
                    .option("Geri", "G");
            }
            Some(Ask::Inset) => {
                return Prompt::new(self.label(), "bacanın yüzden uzaklığını yazın (boş: ortada)")
                    .option("Geri", "G");
            }
            Some(Ask::Sill) => {
                return Prompt::new(self.label(), "alt kotunu yazın (döşemeden)").option("Geri", "G");
            }
            Some(Ask::Head) => {
                return Prompt::new(self.label(), "üst kotunu yazın (döşemeden)").option("Geri", "G");
            }
            Some(Ask::Leaves) => {
                return Prompt::new(self.label(), "kanat ya da panel sayısını yazın").option("Geri", "G");
            }
            Some(Ask::Drum) => {
                return Prompt::new(self.label(), "tamburun iç çapını yazın (boş: ışık kadar)").option("Geri", "G");
            }
            Some(Ask::Lip) => {
                return Prompt::new(self.label(), "açıkken cepten kalan payı yazın").option("Geri", "G");
            }
            Some(Ask::Overlap) => {
                return Prompt::new(self.label(), "iki kanadın bindirmesini yazın (boş: %5)").option("Geri", "G");
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
        if self.form.is_recess() {
            p = p
                .option_with("Derinlik", "D", self.depth_shown())
                .option_with("Yüz", "Y", self.side_label());
            if self.form == OpeningForm::Flue {
                p = p.option_with("Yüzden", "Z", self.inset_shown());
            }
        }
        if self.by_height() {
            p = p
                .option_with("Alt", "L", self.height_shown(true))
                .option_with("Üst", "Ü", self.height_shown(false));
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
            p = self.door_prompt(p, &self.placement.format);
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
        let number = input.openings.first().and_then(|o| o.number.clone());
        let result = openings_create::execute(&mut ExecutionContext::new(cx.doc), input);
        if points::written(result, cx).is_some() {
            self.placement.first = None;
            // A number typed was the first: the next ones go on from it.
            if matches!(self.numbering, Numbering::Next(_)) {
                self.numbering = Numbering::Auto;
            }
            cx.say(
                Level::Success,
                match number {
                    Some(n) => format!("{} {n} eklendi.", self.label()),
                    None => format!("{} eklendi.", self.label()),
                },
            );
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
                match self
                    .all_types()
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
            if let Some(done) = self.door_answer(ask, text, cx) {
                let _ = done;
                return true;
            }
            if ask == Ask::Inset && js_trim(text).is_empty() {
                self.inset = Some(None);
                self.ask = None;
                self.see(cx);
                return true;
            }
            let Some(v) = cx.typed_length(text) else {
                cx.say(Level::Warn, "Bir uzunluk yazın.");
                return true;
            };
            match ask {
                Ask::Depth | Ask::Inset | Ask::Sill | Ask::Head => {
                    if !self.take_height(ask, v, cx) {
                        return true;
                    }
                }
                Ask::Width if v > 0.0 && v <= 100.0 => self.width = Some(v),
                Ask::Width => cx.say(
                    Level::Warn,
                    "Açıklığın genişliği sıfırdan büyük ve en çok 100 m olmalı.",
                ),
                Ask::Distance if v > 0.0 => self.distance = Some(v),
                Ask::Distance => self.distance = None,
                Ask::Type | Ask::Leaves | Ask::Drum | Ask::Lip | Ask::Overlap => {}
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
            "D" if self.form.is_recess() => self.ask = Some(Ask::Depth),
            "Z" if self.form == OpeningForm::Flue => self.ask = Some(Ask::Inset),
            "L" if self.by_height() => self.ask = Some(Ask::Sill),
            "Ü" if self.by_height() => self.ask = Some(Ask::Head),
            "Y" if self.form == OpeningForm::Door || self.form.is_recess() => {
                self.side = match self.side {
                    None => Some(OpeningSide::Left),
                    Some(OpeningSide::Left) => Some(OpeningSide::Right),
                    Some(OpeningSide::Right) => None,
                }
            }
            "M" if self.form == OpeningForm::Door => self.hand = self.hand.other(),
            k if self.door_key(k, cx) => return true,
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
                self.all_types()
                    .into_iter()
                    .filter(|t| t.form == self.form)
                    .map(|t| {
                        let icon = type_icon(t.form, t.leaves, t.operation);
                        choice(t.id.clone(), t.name.clone(), icon, t.name == current)
                    })
                    .collect()
            }
            "Y" if self.form == OpeningForm::Door || self.form.is_recess() => [
                (None, "Otomatik", "openingSideAuto"),
                (Some(OpeningSide::Left), "Sol", "openingSideLeft"),
                (Some(OpeningSide::Right), "Sağ", "openingSideRight"),
            ]
            .into_iter()
            .map(|(s, l, i)| choice(l.to_owned(), l.to_owned(), i, s == self.side))
            .collect(),
            "Ç" if self.form == OpeningForm::Door => self.operation_choices(),
            _ => Vec::new(),
        }
    }

    fn settings(&self, f: &Format) -> Vec<crate::tool_settings::Setting> {
        let mut rows = self.placement_settings(f);
        self.door_settings(f, &mut rows);
        rows
    }

    fn configure(&mut self, key: &str, value: &str, cx: &mut Context<'_>) -> bool {
        if let Some(done) = self.door_configure(key, value, cx) {
            return done;
        }
        self.apply_settings(key, value, cx)
    }

    fn choose_option(&mut self, key: &str, typed: &str, cx: &mut Context<'_>) -> bool {
        if matches!(key, "B" | "F" | "R") {
            return self.placement_configure(key, typed, cx);
        }
        if let Some(done) = self.door_configure(key, typed, cx) {
            return done;
        }
        match key {
            "T" => {
                if self
                    .all_types()
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

/// What number a placed opening takes (docs/adr/0714).
#[derive(Clone, Debug, Default, PartialEq)]
enum Numbering {
    /// After the last numbered opening of its form (K3 → K4), else the form's first (K1, P1, B1).
    #[default]
    Auto,
    /// None.
    Off,
    /// This one; the next ones on from it.
    Next(String),
}

/// `n` with its trailing number one up, as wide as it was (K-07 → K-08, K9 → K10, K-99 → K-100); one without a
/// number (or one too long to count) with 1 after it.
pub(crate) fn next_after(n: &str) -> String {
    let digits = n.chars().rev().take_while(char::is_ascii_digit).count();
    // ASCII digits: as many bytes as letters.
    let (head, tail) = n.split_at(n.len() - digits);
    match tail.parse::<u128>() {
        Ok(v) if !tail.is_empty() => format!("{head}{:0width$}", v + 1, width = tail.len()),
        _ => format!("{n}1"),
    }
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
        if self.action == Action::Move
            && let (Some(source), Some((wall, at))) = (self.picked, self.hover)
            && let (Some(uid), Some(to)) = (cx.doc.uid(source), cx.doc.uid(wall))
        {
            let input = OpeningsEdit {
                uids: vec![uid.to_string()],
                operation: OpeningsOperation::Rehost {
                    to: to.to_string(),
                    point: Some(points::wire(at)),
                },
                expected_revision: None,
            };
            match openings_edit::plan_in(cx.doc, &input) {
                CommandResult::Completed { output, .. } => {
                    for entity in &output.openings {
                        out.strokes
                            .extend(Outline::of(&shape(entity), None, 2.0, Tone::Accent).strokes);
                    }
                    crate::wall_dimensions::append_followers(&mut out, &output.followers, cx);
                }
                CommandResult::Failed { error }
                | CommandResult::Conflict { error }
                | CommandResult::NeedsInput { error } => {
                    // The bonds in the way marked where they stand (docs/adr/0790 §1).
                    if let Some(refused) = crate::bond_refusal::Refused::of(&error, cx) {
                        out = refused.over(out, at);
                    } else {
                        out.tag = Some(Tag {
                            at,
                            lines: crate::wall_dimensions::wrapped(&error.message),
                        });
                        out.tag_tone = Tone::Danger;
                    }
                }
                _ => {}
            }
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
        self.see(cx);
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
            tag: self.drawn.tag.clone().or_else(|| {
                self.hover.map(|(_, at)| Tag {
                    at,
                    lines: vec![self.name().to_owned(), self.step().to_owned()],
                })
            }),
            ..self.drawn.clone()
        }
    }
}
