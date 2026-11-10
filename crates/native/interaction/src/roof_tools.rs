//! Çatı (docs/adr/8044 §1, §4). The tool says what was clicked and chosen; `cad.roofs.create` checks and writes, one undo
//! step, and the plan is the preview.
//!
//! - Sınır (S): Binadan finds the outer faces of the walls round the click (as Döşeme's Dış yüzler), on the roof's
//!   storey or the one under it, and draws the eaves Saçak (A) beyond them; Köşeleri çiz takes the eaves' corners and
//!   Enter closes them.
//! - Tür (T): Kırma, Beşik, Tek eğimli, Düz. The front edge is the one nearest the click (Binadan) or the first drawn:
//!   a shed's water runs off it, a gable roof's ridge runs along it.
//! - Eğim (M: per cent, as Rampa's), Saçak (A), Kalınlık (E), Kot farkı (F), Kaplama (P), Kat (L).
//!
//! The roof's eaves stand at its storey's top and its offset. Without an offset typed (Kot farkı empty) its underside sits
//! on its storey's top at the walls' outer faces (Revit's Truss: the plate on the wall's outside): the offset is its
//! thickness less the overhang's rise (25 cm − 50 cm × %33 = 8.5 cm); drawn by its corners, at its eaves line. Its tag
//! says its ridge's height over ±0.00 and, past PAİY 40's 5.00 m, that it is too high.

use kentos_commands::{CommandResult, Entity, NewRoof, RoofsCreate};
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::geom::roof;
use kentos_geometry_core::tools::point_text::{js_trim, parse_number};
use kentos_native_application::roofs::{RoofForm, eaves_from_walls, nearest_edge, slopes_for};
use kentos_native_application::{ExecutionContext, roofs_create};

use crate::Vec2;
use crate::edge::Outline;
use crate::format::Format;
use crate::log::Level;
use crate::points;
use crate::prompt::{Prompt, upper_tr};
use crate::tool::{Context, Cursor, Flow, OptionChoice, Pointer, Preview, Tag, Tone, Tool};

pub const ROOF_ID: &str = "roof";

/// A roof's slope when none is given, per cent: 33 % (18°), a tiled roof's usual pitch, under the Sivas İmar
/// Yönetmeliği's 40 %.
pub const ROOF_SLOPE: f64 = 33.0;
/// How far the eaves stand past the walls when none is given, metres; PAİY 42 allows 1.00 m.
pub const ROOF_OVERHANG: f64 = 0.5;
/// PAİY 40 (4): a ridge stands at most 5.00 m over the eaves.
pub const PAIY_RIDGE: f64 = 5.0;

/// How the eaves are given.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Bound {
    #[default]
    Building,
    Draw,
}

impl Bound {
    fn label(self) -> &'static str {
        match self {
            Bound::Building => "Binadan",
            Bound::Draw => "Köşeleri çiz",
        }
    }
    fn name(self) -> &'static str {
        match self {
            Bound::Building => "building",
            Bound::Draw => "draw",
        }
    }
}

/// What the tool waits for after its key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Ask {
    Slope,
    Overhang,
    Thickness,
    Offset,
    Material,
}

/// The icon of a kind in Tür's menu.
fn form_icon(form: RoofForm) -> &'static str {
    match form {
        RoofForm::Hip => "roof",
        RoofForm::Gable => "roofGable",
        RoofForm::Shed => "roofShed",
        RoofForm::Flat => "roofFlat",
    }
}

/// A roof the plan writes: its kind ("Kırma çatı"), its ridge's height over ±0.00 and over its eaves, metres.
type Made = (&'static str, f64, f64);

/// Çatı.
#[derive(Clone, Debug)]
pub struct RoofTool {
    bound: Bound,
    form: RoofForm,
    slope: f64,
    overhang: f64,
    thickness: f64,
    /// Its offset as typed; none: its underside sits on its storey's top at the walls ([`RoofTool::offset_now`]).
    offset: Option<f64>,
    material: Option<String>,
    level: crate::architecture_level::Choice,
    pts: Vec<Vec2>,
    hover: Option<Vec2>,
    ask: Option<Ask>,
    drawn: Preview,
    /// What the plan would write; or why it would not.
    made: Option<Made>,
    why: Option<String>,
    format: Format,
}

impl Default for RoofTool {
    fn default() -> Self {
        Self {
            bound: Bound::default(),
            form: RoofForm::default(),
            slope: ROOF_SLOPE,
            overhang: ROOF_OVERHANG,
            thickness: kentos_commands::DEFAULT_ROOF_THICKNESS,
            offset: None,
            material: None,
            level: Default::default(),
            pts: Vec::new(),
            hover: None,
            ask: None,
            drawn: Preview::default(),
            made: None,
            why: None,
            format: Format::default(),
        }
    }
}

impl RoofTool {
    pub fn new() -> Self {
        Self::default()
    }

    fn draws(&self) -> bool {
        self.bound == Bound::Draw
    }

    /// Its offset: as typed, else what sits its underside on its storey's top at the walls' outer faces (its thickness
    /// less the overhang's rise at its slope; drawn by its corners, at its eaves line: its thickness).
    fn offset_now(&self) -> f64 {
        self.offset.unwrap_or_else(|| {
            let run = if self.draws() { 0.0 } else { self.overhang };
            let rise = if self.form == RoofForm::Flat {
                0.0
            } else {
                run * self.slope / 100.0
            };
            self.thickness - rise
        })
    }

    fn offset_text(&self, f: &Format) -> String {
        match self.offset {
            Some(v) => f.length(v),
            None => format!("Duvara oturur ({})", f.length(self.offset_now())),
        }
    }

    /// The roof at the click (Binadan) or over the corners drawn (the first edge the front one); why there is none.
    fn new_roof(
        &self,
        at: Option<Vec2>,
        corners: Option<&[Vec2]>,
        cx: &Context<'_>,
    ) -> Result<NewRoof, String> {
        let (pts, front) = match (at, corners) {
            (_, Some(c)) => (points::wire_all(c), 0),
            (Some(at), None) => {
                let level = self.level.id();
                let pts =
                    eaves_from_walls(cx.doc, points::wire(at), level.as_deref(), self.overhang)?;
                let front = nearest_edge(&pts, points::wire(at));
                (pts, front)
            }
            (None, None) => return Err(String::new()),
        };
        Ok(NewRoof {
            slopes: slopes_for(&pts, self.form, self.slope, front),
            pts,
            offset: Some(self.offset_now()).filter(|v| *v != 0.0),
            thickness: Some(self.thickness),
            // Drawn by its corners, the walls are not known: no overhang is said.
            overhang: (!self.draws()).then_some(self.overhang),
            material: self.material.clone(),
            parts: None,
            arch: self.level.arch(),
            attrs: None,
        })
    }

    /// The layer kind of the roof the tool writes: by its covering (docs/adr/8008), Kiremit when none is given.
    fn layer_kind(&self) -> crate::arch_layer::Kind {
        crate::arch_layer::Kind::Roof(Some(
            self.material.clone().unwrap_or_else(|| "Kiremit".into()),
        ))
    }

    fn create_input(&self, roof: NewRoof, layer_id: String, cx: &Context<'_>) -> RoofsCreate {
        // The view's tadilat durumu (docs/adr/8039 §3).
        let roof = NewRoof {
            arch: cx.phased(roof.arch.clone()),
            ..roof
        };
        RoofsCreate {
            layer_id,
            roofs: vec![roof],
            color: cx.draft.color_text(),
            line_weight: cx.draft.line_weight,
            expected_revision: None,
        }
    }

    /// The click or the corners drawn and the pointer: none until a drawn eaves line has three corners.
    fn hovered(&self) -> Option<(Option<Vec2>, Option<Vec<Vec2>>)> {
        let at = self.hover?;
        if self.draws() {
            let mut c = self.pts.clone();
            c.push(at);
            (c.len() >= 3).then_some((None, Some(c)))
        } else {
            Some((Some(at), None))
        }
    }

    /// What the plan of `roof` writes: its kind, its ridge's height over ±0.00 and over its eaves, and its shape as the
    /// store draws it; why it writes none.
    fn plan_of(&self, roof: NewRoof, cx: &Context<'_>) -> Result<(Made, Shape), String> {
        let input = self.create_input(roof, crate::arch_layer::planned(&self.layer_kind(), cx), cx);
        match roofs_create::plan_in(cx.doc, &input) {
            CommandResult::Completed { output, .. } => {
                let Some(Entity::Roof(r)) = output.roofs.first() else {
                    return Err(String::new());
                };
                let shape = kentos_native_application::geometry::roof_shape(r, cx.doc.settings());
                let (top, rise) = match &shape {
                    Shape::Roof { pts, roof } => {
                        let rise = roof::ridge_rise(pts, roof).unwrap_or(0.0);
                        (roof.base + rise, rise)
                    }
                    _ => (0.0, 0.0),
                };
                Ok(((r.roof.kind_name(), top, rise), shape))
            }
            CommandResult::Failed { error }
            | CommandResult::Conflict { error }
            | CommandResult::NeedsInput { error } => Err(error.message),
            CommandResult::Queued { .. } | CommandResult::Cancelled => Err(String::new()),
        }
    }

    /// The plan at the hover, drawn.
    fn see(&mut self, cx: &Context<'_>) {
        self.format = cx.format();
        self.level.see(cx.doc.settings());
        // Etkin kat: a roof stands on its storey's top, so a storey of no walls (the roof's own) is taken too.
        self.level.follow(cx.level);
        self.drawn = Preview::default();
        self.made = None;
        self.why = None;
        if self.draws() {
            self.drawn.path = self.pts.clone();
            if let Some(at) = self.hover {
                self.drawn.path.push(at);
            }
        }
        let Some((at, corners)) = self.hovered() else {
            return;
        };
        match self
            .new_roof(at, corners.as_deref(), cx)
            .and_then(|roof| self.plan_of(roof, cx))
        {
            Ok((made, shape)) => {
                self.drawn
                    .strokes
                    .extend(Outline::of(&shape, None, 2.0, Tone::Accent).strokes);
                self.made = Some(made);
            }
            Err(why) => self.why = (!why.is_empty()).then_some(why),
        }
    }

    /// Writes the roof at the click or over the corners drawn, on its layer, opened with it when missing (docs/adr/8008);
    /// says what it wrote, or why not.
    fn write(&mut self, at: Option<Vec2>, corners: Option<&[Vec2]>, cx: &mut Context<'_>) -> bool {
        let planned = self
            .new_roof(at, corners, cx)
            .and_then(|roof| self.plan_of(roof.clone(), cx).map(|(made, _)| (roof, made)));
        let (roof, (kind, top, _)) = match planned {
            Ok(found) => found,
            Err(why) => {
                if !why.is_empty() {
                    cx.say(Level::Warn, why);
                }
                return false;
            }
        };
        let layer_kind = self.layer_kind();
        let written = crate::arch_layer::writing(&layer_kind, cx, |layer, cx| {
            let input = self.create_input(roof, layer.to_owned(), cx);
            let result = roofs_create::execute(&mut ExecutionContext::new(cx.doc), input);
            points::written(result, cx).is_some()
        });
        if !written {
            return false;
        }
        let level = self.level.id();
        let seen = level
            .as_deref()
            .filter(|l| crate::level_view::roof_over(cx.doc.settings(), l, self.offset_now()))
            .map(|l| match crate::level_view::above(cx.doc.settings(), l) {
                Some(up) => format!(
                    " Kat planında saçağı kesikli; bütünü “{}” planında ve Bütün katlar'da.",
                    up.name
                ),
                None => " Kat planında saçağı kesikli; bütünü Bütün katlar'da.".to_owned(),
            })
            .unwrap_or_default();
        cx.say(
            Level::Success,
            format!("{kind} çizildi; mahya {}.{seen}", roof::level_words(top)),
        );
        self.pts.clear();
        true
    }

    fn bound_choices(&self) -> Vec<OptionChoice> {
        [Bound::Building, Bound::Draw]
            .iter()
            .map(|b| OptionChoice {
                typed: b.name().into(),
                label: b.label().into(),
                icon: Some(match b {
                    Bound::Building => "spaceGross",
                    Bound::Draw => "revcloudPolygon",
                }),
                preview: None,
                checked: *b == self.bound,
                command: None,
            })
            .collect()
    }

    fn form_choices(&self) -> Vec<OptionChoice> {
        RoofForm::ALL
            .iter()
            .map(|f| OptionChoice {
                typed: f.name().into(),
                label: f.label().into(),
                icon: Some(form_icon(*f)),
                preview: None,
                checked: *f == self.form,
                command: None,
            })
            .collect()
    }

    fn slope_text(&self) -> String {
        roof::slope_words(self.slope)
    }

    /// A typed value set; false (and said) when it is none.
    fn set(&mut self, ask: Ask, text: &str, cx: &mut Context<'_>) -> bool {
        let text = js_trim(text);
        if ask == Ask::Material {
            self.material = (!text.is_empty()).then(|| text.to_owned());
            return true;
        }
        // Empty: the underside sits on the storey's top at the walls again.
        if ask == Ask::Offset && text.is_empty() {
            self.offset = None;
            return true;
        }
        if ask == Ask::Slope {
            let typed = text.strip_prefix('%').unwrap_or(text).trim();
            return match parse_number(typed)
                .filter(|v| *v > 0.0 && *v <= kentos_commands::MAX_ROOF_SLOPE)
            {
                Some(v) => {
                    self.slope = v;
                    true
                }
                None => {
                    cx.say(
                        Level::Warn,
                        format!(
                            "“{text}” bir eğim değil; yüzde olarak yazın (33 ya da %33), sıfırdan büyük, en çok {}.",
                            kentos_commands::MAX_ROOF_SLOPE
                        ),
                    );
                    false
                }
            };
        }
        let Some(v) = cx.typed_length(text).filter(|v| v.is_finite()) else {
            cx.say(
                Level::Warn,
                format!("“{text}” bir uzunluk değil; 50 cm ya da 0.50 gibi yazın."),
            );
            return false;
        };
        match ask {
            Ask::Overhang if v < 0.0 => {
                cx.say(
                    Level::Warn,
                    "Saçak eksi olamaz; duvarın dışına uzaklığı yazın.".to_owned(),
                );
                return false;
            }
            Ask::Overhang => self.overhang = v,
            Ask::Thickness if v <= 0.0 => {
                cx.say(
                    Level::Warn,
                    "Çatının kalınlığı sıfırdan büyük olmalı.".to_owned(),
                );
                return false;
            }
            Ask::Thickness => self.thickness = v,
            Ask::Offset => self.offset = Some(v),
            Ask::Slope | Ask::Material => {}
        }
        true
    }

    fn ask_of(key: &str) -> Option<Ask> {
        Some(match key {
            "M" => Ask::Slope,
            "A" => Ask::Overhang,
            "E" => Ask::Thickness,
            "F" => Ask::Offset,
            "P" => Ask::Material,
            _ => return None,
        })
    }
}

impl Tool for RoofTool {
    fn layer_kind(&self, _cx: &Context<'_>) -> Option<crate::arch_layer::Kind> {
        Some(self.layer_kind())
    }

    fn activate(&mut self, cx: &mut Context<'_>) -> Flow {
        self.see(cx);
        Flow::Stay
    }

    fn level_changed(&mut self, cx: &mut Context<'_>) {
        self.see(cx);
    }

    fn id(&self) -> &'static str {
        ROOF_ID
    }

    fn label(&self) -> &'static str {
        "Çatı"
    }

    fn cursor(&self) -> Cursor {
        Cursor::Cross
    }

    fn snaps(&self) -> bool {
        self.draws()
    }

    fn snap_from(&self) -> Option<Vec2> {
        self.pts.last().copied()
    }

    fn point_count(&self) -> usize {
        self.pts.len()
    }

    fn undo_step(&mut self, cx: &mut Context<'_>) -> bool {
        if self.pts.pop().is_some() {
            self.see(cx);
            return true;
        }
        false
    }

    fn prompt(&self) -> Prompt {
        let words = |label: &'static str| Prompt::new("Çatı", label).option("Geri", "G");
        match self.ask {
            Some(Ask::Slope) => return words("eğimi yüzde olarak yazın (33 gibi)"),
            Some(Ask::Overhang) => {
                return words("saçağın duvarın dış yüzünden uzaklığını yazın (50 cm gibi)");
            }
            Some(Ask::Thickness) => return words("çatının kalınlığını yazın (25 cm gibi)"),
            Some(Ask::Offset) => {
                return words(
                    "saçağın katın üstüne göre farkını yazın (-10 cm gibi); boş: çatı duvara oturur",
                );
            }
            Some(Ask::Material) => return words("kaplamayı yazın (Kiremit gibi)"),
            None => {}
        }
        let step = match (self.draws(), self.pts.len()) {
            (false, _) => "binanın içine, saçağın akacağı kenara yakın tıklayın",
            (true, 0) => "saçağın ilk köşesine tıklayın (ilk kenar ön kenardır)",
            (true, n) if n < 3 => "saçağın sonraki köşesine tıklayın",
            _ => "köşelere tıklayın, Enter ile kapatın",
        };
        let f = &self.format;
        let mut p = Prompt::new("Çatı", step)
            .option_with("Sınır", "S", self.bound.label())
            .option_with("Tür", "T", self.form.label())
            .option_with("Eğim", "M", self.slope_text())
            .option_with("Saçak", "A", f.length(self.overhang))
            .option_with("Kalınlık", "E", f.length(self.thickness))
            .option_with("Kot farkı", "F", self.offset_text(f))
            .option_with(
                "Kaplama",
                "P",
                self.material.clone().unwrap_or_else(|| "Kiremit".into()),
            )
            .option_with("Kat", "L", self.level.label());
        if self.draws() && !self.pts.is_empty() {
            p = p.option("Geri al", "G");
        }
        p.option("Bitir", "Enter")
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.hover = Some(if self.draws() { p.world } else { p.raw });
        self.see(cx);
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        if self.draws() {
            self.accept_point(p.world, cx);
            return;
        }
        self.write(Some(p.raw), None, cx);
        self.see(cx);
    }

    fn accepts_points(&self) -> bool {
        self.draws()
    }

    fn accept_point(&mut self, p: Vec2, cx: &mut Context<'_>) -> bool {
        if !self.draws() {
            return false;
        }
        self.pts.push(p);
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
            if !self.set(ask, text, cx) {
                return true;
            }
            self.ask = None;
            self.see(cx);
            return true;
        }
        match key.as_str() {
            "S" => {
                self.bound = match self.bound {
                    Bound::Building => Bound::Draw,
                    Bound::Draw => Bound::Building,
                };
                self.pts.clear();
            }
            "T" => {
                let i = RoofForm::ALL
                    .iter()
                    .position(|f| *f == self.form)
                    .unwrap_or(0);
                self.form = RoofForm::ALL[(i + 1) % RoofForm::ALL.len()];
            }
            "L" => self.level.next(),
            "G" if self.draws() => return self.undo_step(cx),
            k => {
                if let Some(ask) = Self::ask_of(k) {
                    self.ask = Some(ask);
                    return true;
                }
                if self.draws()
                    && let Some(p) = cx.typed_point(text, self.pts.last().copied(), self.hover)
                {
                    return self.accept_point(p, cx);
                }
                return false;
            }
        }
        self.see(cx);
        true
    }

    fn option_choices(&self, key: &str) -> Vec<OptionChoice> {
        match key {
            "S" => self.bound_choices(),
            "T" => self.form_choices(),
            "L" => self.level.setting().choices,
            _ => Vec::new(),
        }
    }

    fn settings(&self, f: &Format) -> Vec<crate::tool_settings::Setting> {
        use crate::tool_settings::Setting;
        vec![
            Setting::choice("S", "Sınır", self.bound_choices()),
            Setting::choice("T", "Tür", self.form_choices()),
            Setting::text("M", "Eğim (%)", self.slope_text()),
            Setting::text("A", "Saçak", f.length(self.overhang)),
            Setting::text("E", "Kalınlık", f.length(self.thickness)),
            Setting::text("F", "Kot farkı (boş: duvara oturur)", self.offset_text(f)),
            Setting::text(
                "P",
                "Kaplama",
                self.material.clone().unwrap_or_else(|| "Kiremit".into()),
            ),
            self.level.setting(),
            Setting::hint(
                "TH",
                "Ön kenar",
                "Binadan: tıklanan yere en yakın kenar; Köşeleri çiz: ilk kenar. Tek eğimli çatının suyu ön kenardan \
                 akar, beşik çatının mahyası ön kenara paraleldir.",
            ),
        ]
    }

    fn configure(&mut self, key: &str, value: &str, cx: &mut Context<'_>) -> bool {
        if matches!(key, "S" | "T" | "L") {
            return self.choose_option(key, value, cx);
        }
        let Some(ask) = Self::ask_of(key) else {
            return false;
        };
        if !self.set(ask, value, cx) {
            return false;
        }
        self.see(cx);
        true
    }

    fn choose_option(&mut self, key: &str, typed: &str, cx: &mut Context<'_>) -> bool {
        match key {
            "S" => {
                let Some(b) = [Bound::Building, Bound::Draw]
                    .into_iter()
                    .find(|b| b.name() == typed || b.label() == typed)
                else {
                    return false;
                };
                if self.bound != b {
                    self.pts.clear();
                }
                self.bound = b;
            }
            "T" => {
                let Some(f) = RoofForm::ALL
                    .into_iter()
                    .find(|f| f.name() == typed || f.label() == typed)
                else {
                    return false;
                };
                self.form = f;
            }
            "L" => {
                self.level.see(cx.doc.settings());
                if !self.level.choose(typed) {
                    return false;
                }
            }
            _ => return false,
        }
        self.see(cx);
        true
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
        if self.draws() && self.pts.len() >= 3 {
            let corners = self.pts.clone();
            self.write(None, Some(&corners), cx);
            self.see(cx);
            return Flow::Stay;
        }
        Flow::Exit
    }

    fn preview(&self, format: &Format) -> Preview {
        let tag = self.hover.map(|at| Tag {
            at,
            lines: match (&self.why, self.made) {
                (Some(why), _) if !self.draws() || self.pts.len() >= 2 => {
                    vec!["Çatı".to_owned(), why.clone()]
                }
                (_, Some((kind, top, rise))) => {
                    let mut lines = vec![
                        format!("{kind} · {}", roof::slope_words(self.slope)),
                        format!(
                            "Mahya {} · {} yüksek",
                            roof::level_words(top),
                            format.length(rise)
                        ),
                    ];
                    if !self.draws() {
                        lines.push(format!("Saçak {}", format.length(self.overhang)));
                    }
                    if rise > PAIY_RIDGE + 1e-9 {
                        lines.push(format!(
                            "PAİY 40: mahya en çok {} yüksek olur",
                            format.length(PAIY_RIDGE)
                        ));
                    }
                    lines
                }
                _ => vec!["Çatı".to_owned()],
            },
        });
        let too_high = self
            .made
            .is_some_and(|(_, _, rise)| rise > PAIY_RIDGE + 1e-9);
        Preview {
            tag,
            tag_tone: if (self.why.is_some() && self.made.is_none() && !self.draws()) || too_high {
                Tone::Danger
            } else {
                Tone::Accent
            },
            ..self.drawn.clone()
        }
    }
}
