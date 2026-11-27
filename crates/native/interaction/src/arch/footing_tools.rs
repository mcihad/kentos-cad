//! Temel (docs/adr/8172 §3): a pad footing or a strip footing, its Tür (T); in the model a beam's role.
//!
//! - Tekil temel: a click puts it, its middle on the point, turned by its Açı (D). A click on a column puts it under
//!   the column instead: its middle on the column's, turned with it. 150 × 150 cm and 50 cm high until changed.
//! - Sürekli temel: its axis clicked point by point, its band previewed as the plan will draw it; Enter or a quick
//!   right click writes it. 60 cm wide and 50 cm high until changed.
//!
//! Both stand on their base, Kattan yükseklik (A) over their storey's floor: −0.80 until changed, so that their top is
//! 30 cm under the floor (docs/adr/8172 §3.3; KentOS's own first values, not a regulation's). Each on Etkin kat, on
//! its EK-2 layer (MM-TEML-TEKIL, MM-TEML-SUREKLI), its number from Numara and the next the next one (T101 → T102).

use kentos_commands::{BeamRole, BeamsCreate, Entity, NewBeam, WallJustify};
use kentos_domain::Slot;
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::geom::arch::column_center;
use kentos_geometry_core::tools::point_text::{js_trim, parse_number};
use kentos_native_application::geometry::shape;
use kentos_native_application::{ExecutionContext, beams_create};

use crate::Vec2;
use crate::edge::Outline;
use crate::format::Format;
use crate::log::Level;
use crate::points;
use crate::prompt::{Prompt, upper_tr};
use crate::tool::{Context, Cursor, Flow, OptionChoice, Pointer, Preview, Tag, Tone, Tool};
use crate::tool_settings::Setting;

/// Tekil temel.
pub const PAD_ID: &str = "footingPad";
/// Sürekli temel.
pub const STRIP_ID: &str = "footingStrip";

/// A pad's two sides and its height until changed, metres (docs/adr/8172 §3.3).
const PAD: (f64, f64, f64) = (1.5, 1.5, 0.5);
/// A strip's width and height until changed.
const STRIP: (f64, f64) = (0.6, 0.5);
/// Their base over the storey's floor until changed: their top 30 cm under it.
const BASE: f64 = -0.8;

/// What the tool asks for in words.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Ask {
    Width,
    Length,
    Height,
    Base,
    Turn,
    Tag,
}

/// Temel.
#[derive(Clone, Debug)]
pub struct FootingTool {
    /// Tekil or Sürekli temel.
    role: BeamRole,
    /// Across its axis: a pad's one side, a strip's width.
    width: f64,
    /// A pad's other side, along its axis.
    length: f64,
    height: f64,
    /// Its base over its storey's floor: under it, negative.
    base: f64,
    /// The side of its axis a strip lies on.
    justify: WallJustify,
    /// A pad's turn when it stands under no column, radians counter-clockwise.
    turn: f64,
    tag: Option<String>,
    /// A strip's axis so far.
    pts: Vec<Vec2>,
    hover: Option<Vec2>,
    /// The column under the pointer, a pad's place.
    column: Option<Slot>,
    ask: Option<Ask>,
    drawn: Preview,
    why: Option<String>,
}

/// Metres in centimetres as a construction plan writes them: 150, 22.5.
fn cm(m: f64) -> String {
    let c = (m * 1000.0).round() / 10.0;
    if c.fract() == 0.0 { format!("{c:.0}") } else { format!("{c:.1}") }
}

fn justify_key(j: WallJustify) -> &'static str {
    match j {
        WallJustify::Left => "left",
        WallJustify::Center => "center",
        WallJustify::Right => "right",
    }
}

impl FootingTool {
    /// Tekil temel.
    pub fn pad() -> Self {
        FootingTool {
            role: BeamRole::PadFooting,
            width: PAD.0,
            height: PAD.2,
            ..Self::strip()
        }
    }

    /// Sürekli temel.
    pub fn strip() -> Self {
        FootingTool {
            role: BeamRole::StripFooting,
            width: STRIP.0,
            length: PAD.1,
            height: STRIP.1,
            base: BASE,
            justify: WallJustify::Center,
            turn: 0.0,
            tag: None,
            pts: Vec::new(),
            hover: None,
            column: None,
            ask: None,
            drawn: Preview::default(),
            why: None,
        }
    }

    fn is_pad(&self) -> bool {
        self.role == BeamRole::PadFooting
    }

    /// Its role taken, with that role's sizes until changed.
    fn set_role(&mut self, role: BeamRole) {
        if role == self.role || !role.is_footing() {
            return;
        }
        self.role = role;
        self.pts.clear();
        self.column = None;
        if self.is_pad() {
            (self.width, self.length, self.height) = PAD;
        } else {
            (self.width, self.height) = STRIP;
        }
    }

    /// Its size as the plan's words: a pad's sides over its height ("150×150/50"), a strip's width ("60/50").
    fn size(&self) -> String {
        if self.is_pad() {
            format!("{}×{}/{}", cm(self.width), cm(self.length), cm(self.height))
        } else {
            format!("{}/{}", cm(self.width), cm(self.height))
        }
    }

    /// The column under `at`.
    fn column_at(at: Vec2, cx: &Context<'_>) -> Option<Slot> {
        cx.spatial
            .hits(at, cx.pick_tolerance())
            .into_iter()
            .find(|s| matches!(cx.doc.get(*s), Some(Entity::Column(_))))
    }

    /// The middle of the column `slot` and its turn.
    fn column_middle(slot: Slot, cx: &Context<'_>) -> Option<(Vec2, f64)> {
        let e = cx.doc.get(slot).filter(|e| matches!(e, Entity::Column(_)))?;
        let Shape::Column { at, column } = shape(e) else {
            return None;
        };
        Some((column_center(at, &column), column.rotation))
    }

    /// The footing on the axis `pts`.
    fn along(&self, pts: &[Vec2], cx: &Context<'_>) -> NewBeam {
        NewBeam {
            pts: points::wire_all(pts),
            width: self.width,
            height: self.height,
            justify: (!self.is_pad() && self.justify != WallJustify::Center).then_some(self.justify),
            role: Some(self.role),
            base_offset: Some(self.base),
            tag: self.tag.clone(),
            // Etkin kat (docs/adr/0713).
            arch: cx.new_arch(),
            ..NewBeam::default()
        }
    }

    /// A pad at `at`: under the column there, else its middle on the point.
    fn pad_at(&self, at: Vec2, cx: &Context<'_>) -> NewBeam {
        let (middle, turn) = self
            .column
            .and_then(|s| Self::column_middle(s, cx))
            .unwrap_or((at, self.turn));
        let (dx, dy) = (turn.cos() * self.length / 2.0, turn.sin() * self.length / 2.0);
        let ends = [
            Vec2::new(middle.x - dx, middle.y - dy),
            Vec2::new(middle.x + dx, middle.y + dy),
        ];
        self.along(&ends, cx)
    }

    /// A strip's axis so far and the pointer.
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

    /// The call that writes `footing` on `layer_id`.
    fn call(&self, footing: NewBeam, layer_id: String, cx: &Context<'_>) -> BeamsCreate {
        BeamsCreate {
            layer_id,
            beams: vec![footing],
            color: cx.draft.color_text(),
            line_weight: cx.draft.line_weight,
            expected_revision: None,
        }
    }

    /// What a click would write now: a pad at the pointer, else the strip's axis so far.
    fn pending(&self, cx: &Context<'_>) -> Option<NewBeam> {
        if self.is_pad() {
            return self.hover.map(|at| self.pad_at(at, cx));
        }
        let pts = self.axis();
        (pts.len() >= 2).then(|| self.along(&pts, cx))
    }

    /// The footing a click would write, drawn; why there is none.
    fn see(&mut self, cx: &Context<'_>) {
        self.drawn = Preview::default();
        self.why = None;
        let Some(footing) = self.pending(cx) else {
            return;
        };
        let input = self.call(footing, crate::arch_layer::planned(&self.kind(), cx), cx);
        match beams_create::plan_in(cx.doc, &input) {
            kentos_commands::CommandResult::Completed { output, .. } => {
                for e in &output.beams {
                    self.drawn.strokes.extend(
                        Outline::of(&shape(e), None, 2.0, Tone::Accent).strokes,
                    );
                }
            }
            kentos_commands::CommandResult::Failed { error }
            | kentos_commands::CommandResult::Conflict { error }
            | kentos_commands::CommandResult::NeedsInput { error } => self.why = Some(error.message),
            _ => {}
        }
    }

    /// Writes `footing` on its layer, opened with it when missing (docs/adr/8008); says what it wrote.
    fn write(&mut self, footing: NewBeam, cx: &mut Context<'_>) {
        let size = self.size();
        let kind = self.kind();
        let written = crate::arch_layer::writing(&kind, cx, |layer, cx| {
            let input = self.call(footing, layer.to_owned(), cx);
            let result = beams_create::execute(&mut ExecutionContext::new(cx.doc), input);
            points::written(result, cx).is_some()
        });
        if !written {
            return;
        }
        let what = if self.is_pad() { "Tekil temel kondu" } else { "Sürekli temel çizildi" };
        match self.tag.take() {
            // The next one the next number (T101 → T102), as the beam tool's.
            Some(t) => {
                cx.say(Level::Success, format!("{what}: {t} {size}."));
                self.tag = Some(kentos_commands::next_label(&t));
            }
            None => cx.say(Level::Success, format!("{what}: {size}.")),
        }
    }

    /// Writes the strip's axis drawn so far.
    fn write_axis(&mut self, cx: &mut Context<'_>) {
        let pts = std::mem::take(&mut self.pts);
        let footing = self.along(&pts, cx);
        self.write(footing, cx);
    }

    /// A setting's value taken; false when it cannot be read.
    fn apply(&mut self, key: &str, value: &str, cx: &mut Context<'_>) -> bool {
        let length =
            |cx: &mut Context<'_>| cx.typed_length(value).filter(|v| v.is_finite() && *v > 0.0);
        match key {
            "T" => match value {
                "padFooting" => {
                    self.set_role(BeamRole::PadFooting);
                    true
                }
                "stripFooting" => {
                    self.set_role(BeamRole::StripFooting);
                    true
                }
                _ => false,
            },
            "E" => length(cx).map(|v| self.width = v).is_some(),
            "B" if self.is_pad() => length(cx).map(|v| self.length = v).is_some(),
            "Y" => length(cx).map(|v| self.height = v).is_some(),
            // Any height: under the floor negative.
            "A" => cx
                .typed_length(js_trim(value))
                .filter(|v| v.is_finite())
                .map(|v| self.base = v)
                .is_some(),
            "D" if self.is_pad() => match parse_number(js_trim(value).trim_end_matches('°')).filter(|v| v.is_finite()) {
                Some(d) => {
                    self.turn = d.to_radians();
                    true
                }
                None => false,
            },
            "N" => {
                let t = js_trim(value);
                self.tag = (!t.is_empty() && upper_tr(t) != "YOK").then(|| t.to_owned());
                true
            }
            "H" if !self.is_pad() => match value {
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

impl Tool for FootingTool {
    fn layer_kind(&self, _cx: &Context<'_>) -> Option<crate::arch_layer::Kind> {
        Some(self.kind())
    }

    fn id(&self) -> &'static str {
        if self.is_pad() { PAD_ID } else { STRIP_ID }
    }

    fn label(&self) -> &'static str {
        self.role.label()
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
                Ask::Width if self.is_pad() => "eksenine dik kenarını yazın",
                Ask::Width => "genişliğini yazın",
                Ask::Length => "eksenince kenarını yazın",
                Ask::Height => "yüksekliğini (kalınlığını) yazın",
                Ask::Base => "tabanının katın döşemesinden yüksekliğini yazın (altında: eksi; −0.80 gibi)",
                Ask::Turn => "dönüşünü derece olarak yazın (kolonun altındaki temel kolonla döner)",
                Ask::Tag => "numarasını yazın (T101 gibi; boş: yok)",
            };
            return Prompt::new(name, words).option("Geri", "G");
        }
        let n = |v: f64| v.to_string();
        let tag = self.tag.clone().unwrap_or_else(|| "Yok".into());
        if self.is_pad() {
            return Prompt::new(name, "konacağı yere ya da altına konacağı kolona tıklayın")
                .option_with("Tür", "T", name)
                .option_with("Genişlik", "E", n(self.width))
                .option_with("Boy", "B", n(self.length))
                .option_with("Yükseklik", "Y", n(self.height))
                .option_with("Kattan yükseklik", "A", n(self.base))
                .option_with("Açı", "D", n(self.turn.to_degrees()))
                .option_with("Numara", "N", tag);
        }
        let step = match self.pts.len() {
            0 => "ekseninin başına tıklayın",
            1 => "sonraki noktaya tıklayın",
            _ => "sonraki noktaya tıklayın ya da Enter ile yazın",
        };
        Prompt::new(name, step)
            .option_with("Tür", "T", name)
            .option_with("Genişlik", "E", n(self.width))
            .option_with("Yükseklik", "Y", n(self.height))
            .option_with("Hiza", "H", self.justify.label())
            .option_with("Kattan yükseklik", "A", n(self.base))
            .option_with("Numara", "N", tag)
            .option("Bitir", "Enter")
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.hover = Some(p.world);
        if self.is_pad() {
            self.column = Self::column_at(p.raw, cx);
            cx.selection.set_hover(self.column);
        }
        self.see(cx);
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        if self.is_pad() {
            self.column = Self::column_at(p.raw, cx);
            let pad = self.pad_at(p.world, cx);
            self.write(pad, cx);
            self.see(cx);
            return;
        }
        self.accept_point(p.world, cx);
    }

    fn accepts_points(&self) -> bool {
        true
    }

    fn accept_point(&mut self, p: Vec2, cx: &mut Context<'_>) -> bool {
        if self.is_pad() {
            // A typed point: a pad there, its middle on it.
            self.column = None;
            let pad = self.pad_at(p, cx);
            self.write(pad, cx);
            self.see(cx);
            return true;
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
                Ask::Length => "B",
                Ask::Height => "Y",
                Ask::Base => "A",
                Ask::Turn => "D",
                Ask::Tag => "N",
            };
            if !self.apply(field, text, cx) {
                cx.say(
                    Level::Warn,
                    "Değer okunamadı; genişlik, boy ve yükseklik sıfırdan büyük bir uzunluktur.",
                );
                return true;
            }
            self.ask = None;
            self.see(cx);
            return true;
        }
        let pad = self.is_pad();
        self.ask = match key.as_str() {
            "T" => {
                self.set_role(if pad { BeamRole::StripFooting } else { BeamRole::PadFooting });
                self.see(cx);
                return true;
            }
            "E" => Some(Ask::Width),
            "B" if pad => Some(Ask::Length),
            "Y" => Some(Ask::Height),
            "A" => Some(Ask::Base),
            "D" if pad => Some(Ask::Turn),
            "N" => Some(Ask::Tag),
            "H" if !pad => {
                self.justify = match self.justify {
                    WallJustify::Center => WallJustify::Left,
                    WallJustify::Left => WallJustify::Right,
                    WallJustify::Right => WallJustify::Center,
                };
                self.see(cx);
                return true;
            }
            _ => {
                if let Some(p) = cx.typed_point(text, self.pts.last().copied(), self.hover) {
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
                &[("padFooting", "Tekil temel"), ("stripFooting", "Sürekli temel")],
                if self.is_pad() { "padFooting" } else { "stripFooting" },
            ),
        )];
        out.push(Setting::text("E", "Genişlik", f.length(self.width)));
        if self.is_pad() {
            out.push(Setting::text("B", "Boy", f.length(self.length)));
        }
        out.push(Setting::text("Y", "Yükseklik", f.length(self.height)));
        if !self.is_pad() {
            out.push(Setting::choice(
                "H",
                "Hiza",
                crate::tool_settings::choices(
                    &[("center", "Orta"), ("left", "Sol"), ("right", "Sağ")],
                    justify_key(self.justify),
                ),
            ));
        }
        out.push(Setting::text("A", "Kattan yükseklik", f.length(self.base)));
        if self.is_pad() {
            out.push(Setting::text(
                "D",
                "Açı",
                format!("{}°", crate::fixed(self.turn.to_degrees(), 2)),
            ));
        }
        out.push(Setting::text(
            "N",
            "Numara",
            self.tag.clone().unwrap_or_else(|| "Yok".into()),
        ));
        out.push(Setting::hint(
            "P",
            "Yeri",
            if self.is_pad() {
                "Tekil temel tıklanan yere ortalanır; bir kolona tıklanınca kolonun altına ortalanır ve kolonla döner. Tabanı katın döşemesinden kattan yükseklik kadar aşağıdadır; tepesi tabanından yüksekliği kadar yukarıda. Üstündeki kolon temelin tepesinde başlar; kesitte, 3B'de ve IFC'de (IfcFooting) görünür."
            } else {
                "Sürekli temel ekseni boyunca bir banttır; duvarların ya da kolon sıralarının altına çizilir. Tabanı katın döşemesinden kattan yükseklik kadar aşağıdadır; tepesi tabanından yüksekliği kadar yukarıda. Kesitte, 3B'de ve IFC'de (IfcFooting) görünür."
            },
        ));
        out
    }

    fn cursor_settings(&self) -> &'static [&'static str] {
        &["E", "Y"]
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
            "Değer uygulanmadı: genişlik, boy ve yükseklik sıfırdan büyük bir uzunluktur.",
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
        if !self.is_pad() && self.pts.len() >= 2 {
            self.write_axis(cx);
            self.see(cx);
            return Flow::Stay;
        }
        Flow::Exit
    }

    fn preview(&self, _format: &Format) -> Preview {
        let mut out = self.drawn.clone();
        if !self.is_pad() {
            out.path = self.axis();
        }
        out.tag = self.hover.map(|at| {
            let mut lines = vec![format!("{} {}", self.role.label(), self.size())];
            if self.is_pad() && self.column.is_some() {
                lines.push("Kolonun altına ortalanır".into());
            }
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
