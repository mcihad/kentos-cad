//! Kiriş (docs/adr/3041 §1, §3): a beam, a lintel or a bond beam, its Tür (T).
//!
//! - Kiriş: its axis clicked point by point, its band previewed as the plan will draw it; Enter or a quick right click
//!   writes it. 25/50 centred until changed: TBDY 2018 7.4.1.1's least web is 25 cm, a usual depth 50.
//! - Hatıl: drawn as a beam, sitting in its wall at its Kattan yükseklik over its storey's floor; 20/20 at 1.00 m until
//!   changed.
//! - Lento: a door or a window clicked, a lintel over it: as wide as its wall, its Oturma beyond each jamb (TBDY 2018
//!   11.5.3: at least 20 cm), its Yükseklik high (at least 15 cm; 20 until changed), at the opening's head.
//!
//! Each on Etkin kat, on its EK-4 layer (MM-TASI-KIRIS, MM-LENT-GENEL, MM-HATL-GENEL; docs/adr/3008), its number from
//! Numara and the next the next one (K101 → K102).

use kentos_commands::{BeamRole, BeamsCreate, Entity, NewBeam, WallJustify};
use kentos_domain::Slot;
use kentos_geometry_core::tools::point_text::js_trim;
use kentos_native_application::arch_elements::{LINTEL_BEARING, LINTEL_HEIGHT, lintel_over};
use kentos_native_application::{ExecutionContext, beams_create};

use crate::Vec2;
use crate::edge::Outline;
use crate::format::Format;
use crate::log::Level;
use crate::points;
use crate::prompt::{Prompt, upper_tr};
use crate::tool::{Context, Cursor, Flow, OptionChoice, Pointer, Preview, Tag, Tone, Tool};
use crate::tool_settings::Setting;

pub const BEAM_ID: &str = "beam";

/// What the tool asks for in words.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Ask {
    Width,
    Height,
    Offset,
    Base,
    Bearing,
    Tag,
}

/// Kiriş.
#[derive(Clone, Debug)]
pub struct BeamTool {
    role: BeamRole,
    width: f64,
    height: f64,
    justify: WallJustify,
    offset: f64,
    /// A bond beam's underside over its storey's floor.
    base: f64,
    /// A lintel's bearing on the wall beyond each jamb.
    bearing: f64,
    tag: Option<String>,
    pts: Vec<Vec2>,
    hover: Option<Vec2>,
    /// A lintel's opening under the pointer.
    over: Option<Slot>,
    ask: Option<Ask>,
    drawn: Preview,
    why: Option<String>,
}

impl Default for BeamTool {
    fn default() -> Self {
        BeamTool {
            role: BeamRole::Beam,
            width: 0.25,
            height: 0.5,
            justify: WallJustify::Center,
            offset: 0.0,
            base: 1.0,
            bearing: LINTEL_BEARING,
            tag: None,
            pts: Vec::new(),
            hover: None,
            over: None,
            ask: None,
            drawn: Preview::default(),
            why: None,
        }
    }
}

fn justify_key(j: WallJustify) -> &'static str {
    match j {
        WallJustify::Left => "left",
        WallJustify::Center => "center",
        WallJustify::Right => "right",
    }
}

fn role_key(r: BeamRole) -> &'static str {
    match r {
        BeamRole::Beam => "beam",
        BeamRole::Lintel => "lintel",
        BeamRole::BondBeam => "bondBeam",
    }
}

impl BeamTool {
    pub fn new() -> Self {
        Self::default()
    }

    /// Its role taken, with that role's sizes until changed.
    fn set_role(&mut self, role: BeamRole) {
        if role == self.role {
            return;
        }
        self.role = role;
        self.pts.clear();
        self.over = None;
        match role {
            BeamRole::Beam => (self.width, self.height) = (0.25, 0.5),
            BeamRole::Lintel => self.height = LINTEL_HEIGHT,
            BeamRole::BondBeam => (self.width, self.height) = (0.2, 0.2),
        }
    }

    /// The call that writes `beam` on `layer_id`.
    fn call(&self, beam: NewBeam, layer_id: String, cx: &Context<'_>) -> BeamsCreate {
        BeamsCreate {
            layer_id,
            beams: vec![beam],
            color: cx.draft.color_text(),
            line_weight: cx.draft.line_weight,
            expected_revision: None,
        }
    }

    /// A beam or a bond beam on the axis `pts`.
    fn along(&self, pts: &[Vec2], cx: &Context<'_>) -> NewBeam {
        let bond = self.role == BeamRole::BondBeam;
        NewBeam {
            pts: points::wire_all(pts),
            width: self.width,
            height: self.height,
            justify: (self.justify != WallJustify::Center).then_some(self.justify),
            offset: (!bond && self.offset != 0.0).then_some(self.offset),
            role: bond.then_some(BeamRole::BondBeam),
            base_offset: bond.then_some(self.base),
            tag: self.tag.clone(),
            // Etkin kat (docs/adr/0713).
            arch: cx.new_arch(),
            ..NewBeam::default()
        }
    }

    /// A lintel over the opening `slot` (docs/adr/3041 §3); none for another object or an opening it cannot sit on.
    fn lintel(&self, slot: Slot, cx: &Context<'_>) -> Option<NewBeam> {
        let Some(Entity::Opening(o)) = cx.doc.get(slot) else {
            return None;
        };
        let mut n = lintel_over(cx.doc, &o.opening, self.height, self.bearing)?;
        n.tag = self.tag.clone();
        Some(n)
    }

    /// The door or window under `at`.
    fn opening_at(at: Vec2, cx: &Context<'_>) -> Option<Slot> {
        cx.spatial
            .hits(at, cx.pick_tolerance())
            .into_iter()
            .find(|s| matches!(cx.doc.get(*s), Some(Entity::Opening(_))))
    }

    /// The axis so far and the pointer.
    fn axis(&self) -> Vec<Vec2> {
        let mut pts = self.pts.clone();
        if let Some(h) = self.hover
            && pts.last() != Some(&h)
        {
            pts.push(h);
        }
        pts
    }

    fn kind(&self) -> crate::arch_layer::Kind {
        crate::arch_layer::Kind::Beam(self.role)
    }

    /// What the plan would write now: a lintel over the opening under the pointer, else the axis so far.
    fn pending(&self, cx: &Context<'_>) -> Option<NewBeam> {
        match self.role {
            BeamRole::Lintel => self.over.and_then(|s| self.lintel(s, cx)),
            _ => {
                let pts = self.axis();
                (pts.len() >= 2).then(|| self.along(&pts, cx))
            }
        }
    }

    /// The beam the plan would write, drawn; why there is none.
    fn see(&mut self, cx: &Context<'_>) {
        self.drawn = Preview::default();
        self.why = None;
        if self.role == BeamRole::Lintel && self.over.is_some_and(|s| self.lintel(s, cx).is_none()) {
            self.why = Some("Bu açıklığın üstüne lento konamıyor: duvarı ya da söveleri yok.".into());
            return;
        }
        let Some(beam) = self.pending(cx) else {
            return;
        };
        let input = self.call(beam, crate::arch_layer::planned(&self.kind(), cx), cx);
        match beams_create::plan_in(cx.doc, &input) {
            kentos_commands::CommandResult::Completed { output, .. } => {
                for e in &output.beams {
                    self.drawn.strokes.extend(
                        Outline::of(
                            &kentos_native_application::geometry::shape(e),
                            None,
                            2.0,
                            Tone::Accent,
                        )
                        .strokes,
                    );
                }
            }
            kentos_commands::CommandResult::Failed { error }
            | kentos_commands::CommandResult::Conflict { error }
            | kentos_commands::CommandResult::NeedsInput { error } => {
                self.why = Some(error.message)
            }
            _ => {}
        }
    }

    /// Writes `beam` on its layer, opened with it when missing (docs/adr/3008); says what it wrote.
    fn write(&mut self, beam: NewBeam, cx: &mut Context<'_>) {
        let size = kentos_commands::BeamFields {
            width: beam.width,
            height: beam.height,
            ..Default::default()
        }
        .size_text();
        let kind = self.kind();
        let written = crate::arch_layer::writing(&kind, cx, |layer, cx| {
            let input = self.call(beam, layer.to_owned(), cx);
            let result = beams_create::execute(&mut ExecutionContext::new(cx.doc), input);
            points::written(result, cx).is_some()
        });
        if !written {
            return;
        }
        let what = match self.role {
            BeamRole::Beam => "Kiriş çizildi",
            BeamRole::Lintel => "Lento kondu",
            BeamRole::BondBeam => "Hatıl çizildi",
        };
        match self.tag.take() {
            // The next one the next number (K101 → K102), as the column tool's.
            Some(t) => {
                cx.say(Level::Success, format!("{what}: {t} {size}."));
                self.tag = Some(kentos_commands::next_label(&t));
            }
            None => cx.say(Level::Success, format!("{what}: {size}.")),
        }
    }

    /// Writes the axis drawn so far.
    fn write_axis(&mut self, cx: &mut Context<'_>) {
        let pts = std::mem::take(&mut self.pts);
        let beam = self.along(&pts, cx);
        self.write(beam, cx);
    }

    /// Its size as a plan writes it ("25/50").
    fn size(&self) -> String {
        kentos_commands::BeamFields {
            width: self.width,
            height: self.height,
            ..Default::default()
        }
        .size_text()
    }

    /// A setting's value taken; false when it cannot be read.
    fn apply(&mut self, key: &str, value: &str, cx: &mut Context<'_>) -> bool {
        let length =
            |cx: &mut Context<'_>| cx.typed_length(value).filter(|v| v.is_finite() && *v > 0.0);
        match key {
            "T" => match value {
                "beam" => {
                    self.set_role(BeamRole::Beam);
                    true
                }
                "lintel" => {
                    self.set_role(BeamRole::Lintel);
                    true
                }
                "bondBeam" => {
                    self.set_role(BeamRole::BondBeam);
                    true
                }
                _ => false,
            },
            "E" => length(cx).map(|v| self.width = v).is_some(),
            "Y" => length(cx).map(|v| self.height = v).is_some(),
            "O" => length(cx).map(|v| self.bearing = v).is_some(),
            "A" => cx
                .typed_length(js_trim(value))
                .filter(|v| v.is_finite())
                .map(|v| self.base = v)
                .is_some(),
            "F" => {
                let t = js_trim(value);
                let v = if t.is_empty() {
                    Some(0.0)
                } else {
                    cx.typed_length(t).filter(|v| v.is_finite())
                };
                v.map(|v| self.offset = v).is_some()
            }
            "N" => {
                let t = js_trim(value);
                self.tag = (!t.is_empty()).then(|| t.to_owned());
                true
            }
            "H" => match value {
                "left" => {
                    self.justify = WallJustify::Left;
                    true
                }
                "center" => {
                    self.justify = WallJustify::Center;
                    true
                }
                "right" => {
                    self.justify = WallJustify::Right;
                    true
                }
                _ => false,
            },
            _ => false,
        }
    }
}

impl Tool for BeamTool {
    fn layer_kind(&self, _cx: &Context<'_>) -> Option<crate::arch_layer::Kind> {
        Some(self.kind())
    }

    fn id(&self) -> &'static str {
        BEAM_ID
    }

    fn label(&self) -> &'static str {
        "Kiriş"
    }

    fn cursor(&self) -> Cursor {
        Cursor::Cross
    }

    fn snap_from(&self) -> Option<Vec2> {
        self.pts.last().copied()
    }

    fn point_count(&self) -> usize {
        self.pts.len()
    }

    fn level_changed(&mut self, cx: &mut Context<'_>) {
        self.see(cx);
    }

    fn undo_step(&mut self, cx: &mut Context<'_>) -> bool {
        if self.pts.pop().is_some() {
            self.see(cx);
            return true;
        }
        false
    }

    fn prompt(&self) -> Prompt {
        let name = self.role.label();
        if let Some(ask) = self.ask {
            let words = match ask {
                Ask::Width => "gövde genişliğini yazın",
                Ask::Height => "yüksekliğini yazın",
                Ask::Offset => "kirişin tepesinin kot farkını yazın (0: üst döşemenin betonarmesinde)",
                Ask::Base => "alt yüzünün katın döşemesinden yüksekliğini yazın",
                Ask::Bearing => "lentonun her iki yanda duvara oturma boyunu yazın (en az 20 cm)",
                Ask::Tag => "numarasını yazın (K101, L101, H101 gibi; boş: yok)",
            };
            return Prompt::new(name, words).option("Geri", "G");
        }
        let n = |v: f64| v.to_string();
        let tag = self.tag.clone().unwrap_or_else(|| "Yok".into());
        if self.role == BeamRole::Lintel {
            return Prompt::new(name, "lentonun konacağı kapıya ya da pencereye tıklayın")
                .option_with("Tür", "T", name)
                .option_with("Yükseklik", "Y", n(self.height))
                .option_with("Oturma", "O", n(self.bearing))
                .option_with("Numara", "N", tag);
        }
        let step = match self.pts.len() {
            0 => "ekseninin başına tıklayın",
            1 => "sonraki noktaya tıklayın",
            _ => "sonraki noktaya tıklayın ya da Enter ile yazın",
        };
        let p = Prompt::new(name, step)
            .option_with("Tür", "T", name)
            .option_with("Genişlik", "E", n(self.width))
            .option_with("Yükseklik", "Y", n(self.height))
            .option_with("Hiza", "H", self.justify.label());
        let p = if self.role == BeamRole::BondBeam {
            p.option_with("Kattan yükseklik", "A", n(self.base))
        } else {
            p.option_with("Kot farkı", "F", n(self.offset))
        };
        p.option_with("Numara", "N", tag).option("Bitir", "Enter")
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.hover = Some(p.world);
        if self.role == BeamRole::Lintel {
            self.over = Self::opening_at(p.raw, cx);
            cx.selection.set_hover(self.over);
        }
        self.see(cx);
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        if self.role == BeamRole::Lintel {
            self.over = Self::opening_at(p.raw, cx);
            match self.over.and_then(|s| self.lintel(s, cx)) {
                Some(beam) => self.write(beam, cx),
                None if self.over.is_some() => cx.say(
                    Level::Warn,
                    "Bu açıklığın üstüne lento konamıyor: duvarı ya da söveleri yok.",
                ),
                None => cx.say(Level::Info, "Burada kapı ya da pencere yok."),
            }
            self.see(cx);
            return;
        }
        self.accept_point(p.world, cx);
    }

    fn accepts_points(&self) -> bool {
        self.role != BeamRole::Lintel
    }

    fn accept_point(&mut self, p: Vec2, cx: &mut Context<'_>) -> bool {
        if self.role == BeamRole::Lintel {
            return false;
        }
        if self.pts.last() == Some(&p) {
            return true;
        }
        self.pts.push(p);
        if self.pts.len() == kentos_commands::MAX_BEAM_POINTS {
            self.write_axis(cx);
        }
        self.see(cx);
        true
    }

    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        let key = upper_tr(js_trim(text));
        if let Some(ask) = self.ask {
            if key == "G" {
                self.ask = None;
                return true;
            }
            let field = match ask {
                Ask::Width => "E",
                Ask::Height => "Y",
                Ask::Offset => "F",
                Ask::Base => "A",
                Ask::Bearing => "O",
                Ask::Tag => "N",
            };
            if !self.apply(field, text, cx) {
                cx.say(
                    Level::Warn,
                    "Değer okunamadı; genişlik, yükseklik ve oturma sıfırdan büyük bir uzunluktur.",
                );
                return true;
            }
            self.ask = None;
            self.see(cx);
            return true;
        }
        let lintel = self.role == BeamRole::Lintel;
        self.ask = match key.as_str() {
            "T" => {
                let next = match self.role {
                    BeamRole::Beam => BeamRole::Lintel,
                    BeamRole::Lintel => BeamRole::BondBeam,
                    BeamRole::BondBeam => BeamRole::Beam,
                };
                self.set_role(next);
                self.see(cx);
                return true;
            }
            "E" if !lintel => Some(Ask::Width),
            "Y" => Some(Ask::Height),
            "F" if self.role == BeamRole::Beam => Some(Ask::Offset),
            "A" if self.role == BeamRole::BondBeam => Some(Ask::Base),
            "O" if lintel => Some(Ask::Bearing),
            "N" => Some(Ask::Tag),
            "H" if !lintel => {
                self.justify = match self.justify {
                    WallJustify::Center => WallJustify::Left,
                    WallJustify::Left => WallJustify::Right,
                    WallJustify::Right => WallJustify::Center,
                };
                self.see(cx);
                return true;
            }
            _ => {
                if !lintel
                    && let Some(p) = cx.typed_point(text, self.pts.last().copied(), self.hover)
                {
                    return self.accept_point(p, cx);
                }
                return false;
            }
        };
        true
    }

    fn settings(&self, f: &Format) -> Vec<Setting> {
        let mut out = vec![Setting::choice(
            "T",
            "Tür",
            crate::tool_settings::choices(
                &[("beam", "Kiriş"), ("lintel", "Lento"), ("bondBeam", "Hatıl")],
                role_key(self.role),
            ),
        )];
        let tag = Setting::text(
            "N",
            "Numara",
            self.tag.clone().unwrap_or_else(|| "Yok".into()),
        );
        if self.role == BeamRole::Lintel {
            out.extend([
                Setting::text("Y", "Yükseklik", f.length(self.height)),
                Setting::text("O", "Oturma", f.length(self.bearing)),
                tag,
                Setting::hint(
                    "P",
                    "Yeri",
                    "Lento kapının ya da pencerenin başında oturur, iki yanda duvara oturma boyu kadar uzanır; genişliği duvarın kalınlığıdır. TBDY 2018 11.5.3: yüksekliği en az 15 cm, oturması en az 20 cm.",
                ),
            ]);
            return out;
        }
        out.extend([
            Setting::text("E", "Genişlik", f.length(self.width)),
            Setting::text("Y", "Yükseklik", f.length(self.height)),
            Setting::choice(
                "H",
                "Hiza",
                crate::tool_settings::choices(
                    &[("center", "Orta"), ("left", "Sol"), ("right", "Sağ")],
                    justify_key(self.justify),
                ),
            ),
        ]);
        if self.role == BeamRole::BondBeam {
            out.extend([
                Setting::text("A", "Kattan yükseklik", f.length(self.base)),
                tag,
                Setting::hint(
                    "P",
                    "Yeri",
                    "Hatıl duvarın içinde, katın döşemesinden kattan yükseklik kadar yukarıda durur; duvar onun üstünden sürer.",
                ),
            ]);
        } else {
            out.extend([
                Setting::text("F", "Kot farkı", f.length(self.offset)),
                tag,
                Setting::hint(
                    "P",
                    "Tepesi",
                    "Kirişin tepesi katın üstündeki döşemenin betonarmesindedir; döşeme yoksa katın tepesinde. Altındaki duvar kirişin altında biter.",
                ),
            ]);
        }
        out
    }

    fn cursor_settings(&self) -> &'static [&'static str] {
        if self.role == BeamRole::Lintel {
            &["Y", "O"]
        } else {
            &["E", "Y"]
        }
    }

    fn option_choices(&self, key: &str) -> Vec<OptionChoice> {
        self.settings(&Format::default())
            .into_iter()
            .find(|s| s.key == key)
            .map_or_else(Vec::new, |s| s.choices)
    }

    fn choose_option(&mut self, key: &str, value: &str, cx: &mut Context<'_>) -> bool {
        self.configure(key, value, cx)
    }

    fn configure(&mut self, key: &str, value: &str, cx: &mut Context<'_>) -> bool {
        if self.apply(key, value, cx) {
            self.ask = None;
            self.see(cx);
            return true;
        }
        cx.say(
            Level::Warn,
            "Değer uygulanmadı: genişlik, yükseklik ve oturma sıfırdan büyük bir uzunluktur.",
        );
        false
    }

    fn takes_words(&self) -> bool {
        self.ask.is_some()
    }

    fn cancel(&mut self, cx: &mut Context<'_>) -> bool {
        if self.ask.take().is_some() {
            return true;
        }
        if !self.pts.is_empty() {
            self.pts.clear();
            self.see(cx);
            return true;
        }
        false
    }

    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow {
        if self.role != BeamRole::Lintel && self.pts.len() >= 2 {
            self.write_axis(cx);
            self.see(cx);
            return Flow::Stay;
        }
        Flow::Exit
    }

    fn preview(&self, _format: &Format) -> Preview {
        let mut out = self.drawn.clone();
        if self.role != BeamRole::Lintel {
            out.path = self.axis();
        }
        out.tag = self.hover.map(|at| {
            let mut lines = vec![format!("{} {}", self.role.label(), self.size())];
            if let Some(why) = &self.why {
                lines.push(why.clone());
            }
            Tag { at, lines }
        });
        out.tag_tone = if self.why.is_some() {
            Tone::Danger
        } else {
            Tone::Accent
        };
        out
    }
}
