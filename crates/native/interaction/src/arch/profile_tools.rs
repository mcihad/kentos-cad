//! Duvar profili (docs/adr/8173 §5): a facade moulding (Cephe silmesi) or a coping (Duvar harpuştası), its Tür (T); in
//! the model a beam's band that is a covering. Their full terms, not "Silme" alone, which reads as deleting.
//!
//! - Cephe silmesi: Sınır (S) Dış yüzler finds the walls' outer outline round the click and runs the band round it, out of it;
//!   Çiz takes its axis point by point, Enter writes it. Çıkıntı (E) 5 cm, Yükseklik (Y) 20 cm and Kattan yükseklik (A)
//!   2.80 until changed: a 3 m storey's floor band.
//! - Duvar harpuştası: a click on a wall puts it on the wall's top, along its middle, Taşma (E) past each face: 5 cm past, 5 cm
//!   thick until changed.
//!
//! Each on Etkin kat (a coping on its wall's), on its EK-2 layer: MM-CEPH-SILME, MM-HARP-<malzeme>.

use kentos_commands::{BeamsCreate, Covering, Entity, NewBeam, WallJustify};
use kentos_domain::Slot;
use kentos_geometry_core::tools::point_text::js_trim;
use kentos_native_application::{ExecutionContext, beams_create};

use crate::Vec2;
use crate::edge::Outline;
use crate::format::Format;
use crate::log::Level;
use crate::points;
use crate::prompt::{Prompt, upper_tr};
use crate::tool::{Context, Cursor, Flow, OptionChoice, Pointer, Preview, Tag, Tone, Tool};
use crate::tool_settings::Setting;

/// Cephe silmesi.
pub const MOULDING_ID: &str = "moulding";
/// Duvar harpuştası.
pub const COPING_ID: &str = "coping";

/// A moulding's projection, height and base until changed, metres (docs/adr/8173 §5.2).
const MOULDING: (f64, f64, f64) = (0.05, 0.2, 2.8);
/// A coping's overhang past each face and its thickness until changed.
const COPING: (f64, f64) = (0.05, 0.05);

/// What the tool asks for in words.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Ask {
    Width,
    Height,
    Base,
    Material,
}

/// How a moulding's axis is given.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Bound {
    /// The walls' outer outline round the click.
    Outline,
    /// Its points clicked.
    Draw,
}

/// Duvar profili.
#[derive(Clone, Debug)]
pub struct ProfileTool {
    /// A coping (Duvar harpuştası), else a facade moulding (Cephe silmesi).
    coping: bool,
    bound: Bound,
    /// A moulding's projection; a coping's overhang past each face.
    width: f64,
    height: f64,
    /// A moulding's base over its storey's floor.
    base: f64,
    /// The side of a drawn moulding's axis it lies on.
    justify: WallJustify,
    material: Option<String>,
    /// A drawn moulding's axis so far.
    pts: Vec<Vec2>,
    hover: Option<Vec2>,
    /// The wall under the pointer, a coping's.
    wall: Option<Slot>,
    ask: Option<Ask>,
    drawn: Preview,
    why: Option<String>,
}

/// Metres in centimetres as a plan writes them: 5, 22.5.
fn cm(m: f64) -> String {
    let c = (m * 1000.0).round() / 10.0;
    if c.fract() == 0.0 {
        format!("{c:.0}")
    } else {
        format!("{c:.1}")
    }
}

fn justify_key(j: WallJustify) -> &'static str {
    match j {
        WallJustify::Left => "left",
        WallJustify::Center => "center",
        WallJustify::Right => "right",
    }
}

/// A closed ring as a band's axis that runs round it once: from the middle of its first edge round to it again, so
/// that its two ends meet end to end on a straight edge and every corner is mitred; counter-clockwise, its outside on
/// its right.
fn round_axis(ring: &[Vec2]) -> Option<Vec<Vec2>> {
    let mut ring: Vec<Vec2> = ring.to_vec();
    if ring.len() > 1 && ring.first() == ring.last() {
        ring.pop();
    }
    if ring.len() < 3 {
        return None;
    }
    let area: f64 = (0..ring.len())
        .map(|i| {
            let (a, b) = (ring[i], ring[(i + 1) % ring.len()]);
            a.x * b.y - b.x * a.y
        })
        .sum();
    if area < 0.0 {
        ring.reverse();
    }
    let (a, b) = (ring[0], ring[1]);
    let middle = Vec2::new((a.x + b.x) / 2.0, (a.y + b.y) / 2.0);
    let mut axis = vec![middle];
    axis.extend(ring[1..].iter().copied());
    axis.push(ring[0]);
    axis.push(middle);
    Some(axis)
}

impl ProfileTool {
    /// Cephe silmesi.
    pub fn moulding() -> Self {
        ProfileTool {
            coping: false,
            bound: Bound::Outline,
            width: MOULDING.0,
            height: MOULDING.1,
            base: MOULDING.2,
            justify: WallJustify::Center,
            material: None,
            pts: Vec::new(),
            hover: None,
            wall: None,
            ask: None,
            drawn: Preview::default(),
            why: None,
        }
    }

    /// Duvar harpuştası.
    pub fn coping() -> Self {
        let mut tool = Self::moulding();
        tool.set_coping(true);
        tool
    }

    fn name(&self) -> &'static str {
        if self.coping {
            "Duvar harpuştası"
        } else {
            "Cephe silmesi"
        }
    }

    /// Its Tür taken, with that kind's sizes until changed.
    fn set_coping(&mut self, coping: bool) {
        if coping == self.coping {
            return;
        }
        self.coping = coping;
        self.pts.clear();
        self.wall = None;
        if coping {
            (self.width, self.height) = COPING;
        } else {
            (self.width, self.height, self.base) = MOULDING;
        }
    }

    fn covering(&self) -> Covering {
        if self.coping {
            Covering::coping()
        } else {
            Covering::facade_moulding()
        }
    }

    fn kind(&self) -> crate::arch_layer::Kind {
        if self.coping {
            crate::arch_layer::Kind::Coping(self.material.clone())
        } else {
            crate::arch_layer::Kind::Moulding
        }
    }

    /// The floor of the level `level`, metres over ±0.00 (0 without one).
    fn floor_of(cx: &Context<'_>, level: Option<&str>) -> f64 {
        let levels = cx
            .doc
            .settings()
            .arch
            .as_ref()
            .map(|a| a.levels.as_slice())
            .unwrap_or_default();
        level
            .and_then(|id| levels.iter().find(|l| l.id == id))
            .map_or(0.0, |l| l.elevation)
    }

    /// The straight wall under `at`.
    fn wall_at(at: Vec2, cx: &Context<'_>) -> Option<Slot> {
        cx.spatial
            .hits(at, cx.pick_tolerance())
            .into_iter()
            .find(|s| matches!(cx.doc.get(*s), Some(Entity::Wall(_))))
    }

    /// A moulding on the axis `pts`, lying on `justify`'s side of it.
    fn moulding_on(&self, pts: &[Vec2], justify: WallJustify, cx: &Context<'_>) -> NewBeam {
        NewBeam {
            pts: points::wire_all(pts),
            width: self.width,
            height: self.height,
            justify: (justify != WallJustify::Center).then_some(justify),
            base_offset: Some(self.base),
            material: self.material.clone(),
            covering: Some(self.covering()),
            // Etkin kat (docs/adr/0713).
            arch: cx.new_arch(),
            ..NewBeam::default()
        }
    }

    /// The moulding round the walls' outer outline round `seed`, out of it; why there is none.
    fn moulding_round(&self, seed: Vec2, cx: &Context<'_>) -> Result<NewBeam, String> {
        let level = cx.new_arch().and_then(|a| a.level);
        let Some(part) = cx
            .doc
            .slab_from_walls(points::wire(seed), level.as_deref(), false)
        else {
            return Err("Nokta hiçbir duvar konturunun içinde değil; yapının içine tıklayın ya da Sınır'ı Çiz yapın.".into());
        };
        if part
            .outer
            .bulges
            .as_ref()
            .is_some_and(|b| b.iter().any(|v| *v != 0.0))
        {
            return Err("Eğrisel konturun cephe silmesi çizilemiyor: Sınır'ı Çiz yapıp düz kenarlarla verin.".into());
        }
        let ring: Vec<Vec2> = part.outer.pts.iter().map(|p| Vec2::new(p.x, p.y)).collect();
        let axis = round_axis(&ring).ok_or("Duvar konturu kapalı bir halka değil.")?;
        // Counter-clockwise, its outside on its right: the axis is the band's left face.
        Ok(self.moulding_on(&axis, WallJustify::Left, cx))
    }

    /// The coping on the wall `slot`: along its middle, on its top, its overhang past each face; why there is none.
    fn coping_on(&self, slot: Slot, cx: &Context<'_>) -> Result<NewBeam, String> {
        let Some(Entity::Wall(w)) = cx.doc.get(slot) else {
            return Err("Burada duvar yok.".into());
        };
        let w = &w.wall;
        if w.bulges
            .as_ref()
            .is_some_and(|b| b.iter().any(|v| *v != 0.0))
        {
            return Err("Eğrisel duvarın harpuştası çizilemiyor; düz bir duvara tıklayın.".into());
        }
        let Some(height) = cx.doc.wall_height(w) else {
            return Err("Duvarın yüksekliği yok: katı ya da kendi yüksekliği verilmeli.".into());
        };
        let floor = Self::floor_of(cx, w.arch.level.as_deref());
        let top = cx.doc.wall_foot(w) + height - floor;
        // Its middle: a wall drawn on its left face lies to the right of its line (docs/adr/8041 §1).
        let middle = match w.justify {
            WallJustify::Center => 0.0,
            WallJustify::Left => -w.width / 2.0,
            WallJustify::Right => w.width / 2.0,
        };
        let line: Vec<Vec2> = w.pts.iter().map(|p| Vec2::new(p.x, p.y)).collect();
        let axis = if w.closed {
            let centre = kentos_geometry_core::geom::beam::side(&line, middle);
            round_axis(&centre).ok_or("Kapalı duvar bir halka değil.")?
        } else {
            kentos_geometry_core::geom::beam::side(&line, middle)
        };
        Ok(NewBeam {
            pts: points::wire_all(&axis),
            width: w.width + 2.0 * self.width,
            height: self.height,
            base_offset: Some(top),
            material: self.material.clone(),
            covering: Some(self.covering()),
            arch: Some(kentos_commands::ArchInfo {
                level: w.arch.level.clone(),
                ..cx.new_arch().unwrap_or_default()
            }),
            ..NewBeam::default()
        })
    }

    /// A drawn moulding's axis so far and the pointer.
    fn axis(&self) -> Vec<Vec2> {
        let mut pts = self.pts.clone();
        if let Some(h) = self.hover
            && pts.last() != Some(&h)
        {
            pts.push(h);
        }
        pts
    }

    /// What a click would write now; why it would write none.
    fn pending(&self, cx: &Context<'_>) -> Option<Result<NewBeam, String>> {
        if self.coping {
            return self.wall.map(|s| self.coping_on(s, cx));
        }
        match self.bound {
            Bound::Outline => self.hover.map(|at| self.moulding_round(at, cx)),
            Bound::Draw => {
                let pts = self.axis();
                (pts.len() >= 2).then(|| Ok(self.moulding_on(&pts, self.justify, cx)))
            }
        }
    }

    fn call(&self, profile: NewBeam, layer_id: String, cx: &Context<'_>) -> BeamsCreate {
        BeamsCreate {
            layer_id,
            beams: vec![profile],
            color: cx.draft.color_text(),
            line_weight: cx.draft.line_weight,
            expected_revision: None,
        }
    }

    /// The profile a click would write, drawn; why there is none.
    fn see(&mut self, cx: &Context<'_>) {
        self.drawn = Preview::default();
        self.why = None;
        let profile = match self.pending(cx) {
            None => return,
            Some(Err(why)) => {
                self.why = Some(why);
                return;
            }
            Some(Ok(p)) => p,
        };
        let input = self.call(profile, crate::arch_layer::planned(&self.kind(), cx), cx);
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

    /// Writes `profile` on its layer, opened with it when missing (docs/adr/8008); says what it wrote.
    fn write(&mut self, profile: NewBeam, cx: &mut Context<'_>) {
        let kind = self.kind();
        let written = crate::arch_layer::writing(&kind, cx, |layer, cx| {
            let input = self.call(profile, layer.to_owned(), cx);
            let result = beams_create::execute(&mut ExecutionContext::new(cx.doc), input);
            points::written(result, cx).is_some()
        });
        if written {
            let what = if self.coping {
                "Duvar harpuştası kondu"
            } else {
                "Cephe silmesi çizildi"
            };
            cx.say(Level::Success, format!("{what}: {}.", self.size()));
        }
    }

    /// Its size in words: a moulding's projection over its height ("5/20"), a coping's overhang and thickness.
    fn size(&self) -> String {
        if self.coping {
            format!("{} cm taşma, {} cm", cm(self.width), cm(self.height))
        } else {
            format!("{}/{}", cm(self.width), cm(self.height))
        }
    }

    /// A setting's value taken; false when it cannot be read.
    fn apply(&mut self, key: &str, value: &str, cx: &mut Context<'_>) -> bool {
        let length =
            |cx: &mut Context<'_>| cx.typed_length(value).filter(|v| v.is_finite() && *v > 0.0);
        match key {
            "T" => match value {
                "moulding" => {
                    self.set_coping(false);
                    true
                }
                "coping" => {
                    self.set_coping(true);
                    true
                }
                _ => false,
            },
            "S" if !self.coping => match value {
                "outline" => {
                    self.bound = Bound::Outline;
                    self.pts.clear();
                    true
                }
                "draw" => {
                    self.bound = Bound::Draw;
                    true
                }
                _ => false,
            },
            "E" => length(cx).map(|v| self.width = v).is_some(),
            "Y" => length(cx).map(|v| self.height = v).is_some(),
            "A" if !self.coping => cx
                .typed_length(js_trim(value))
                .filter(|v| v.is_finite())
                .map(|v| self.base = v)
                .is_some(),
            "M" => {
                let t = js_trim(value);
                self.material = (!t.is_empty() && upper_tr(t) != "YOK").then(|| t.to_owned());
                true
            }
            "H" if !self.coping => match value {
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

    fn draws(&self) -> bool {
        !self.coping && self.bound == Bound::Draw
    }
}

impl Tool for ProfileTool {
    fn layer_kind(&self, _cx: &Context<'_>) -> Option<crate::arch_layer::Kind> {
        Some(self.kind())
    }

    fn id(&self) -> &'static str {
        if self.coping { COPING_ID } else { MOULDING_ID }
    }

    fn label(&self) -> &'static str {
        self.name()
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
        let name = self.name();
        if let Some(ask) = self.ask {
            let words = match ask {
                Ask::Width if self.coping => "her iki yüzden taşmasını yazın",
                Ask::Width => "duvardan çıkıntısını yazın",
                Ask::Height => "yüksekliğini (kalınlığını) yazın",
                Ask::Base => "alt yüzünün katın döşemesinden yüksekliğini yazın",
                Ask::Material => "malzemesini yazın (Taş, Prekast beton gibi; boş: yok)",
            };
            return Prompt::new(name, words).option("Geri", "G");
        }
        let n = |v: f64| v.to_string();
        let material = self.material.clone().unwrap_or_else(|| "Yok".into());
        if self.coping {
            return Prompt::new(name, "tepesine harpuşta konacak duvara tıklayın")
                .option_with("Tür", "T", name)
                .option_with("Taşma", "E", n(self.width))
                .option_with("Yükseklik", "Y", n(self.height))
                .option_with("Malzeme", "M", material);
        }
        let step = match (self.bound, self.pts.len()) {
            (Bound::Outline, _) => "duvarların içine tıklayın: dış yüzler boyunca cephe silmesi",
            (Bound::Draw, 0) => "ekseninin başına tıklayın",
            (Bound::Draw, 1) => "sonraki noktaya tıklayın",
            (Bound::Draw, _) => "sonraki noktaya tıklayın ya da Enter ile yazın",
        };
        let mut p = Prompt::new(name, step)
            .option_with("Tür", "T", name)
            .option_with(
                "Sınır",
                "S",
                if self.bound == Bound::Outline {
                    "Dış yüzler"
                } else {
                    "Çiz"
                },
            )
            .option_with("Çıkıntı", "E", n(self.width))
            .option_with("Yükseklik", "Y", n(self.height))
            .option_with("Kattan yükseklik", "A", n(self.base));
        if self.bound == Bound::Draw {
            p = p.option_with("Hiza", "H", self.justify.label());
        }
        p = p.option_with("Malzeme", "M", material);
        if self.bound == Bound::Draw {
            p = p.option("Bitir", "Enter");
        }
        p
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.hover = Some(if self.draws() { p.world } else { p.raw });
        if self.coping {
            self.wall = Self::wall_at(p.raw, cx);
            cx.selection.set_hover(self.wall);
        }
        self.see(cx);
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        if self.draws() {
            self.accept_point(p.world, cx);
            return;
        }
        if self.coping {
            self.wall = Self::wall_at(p.raw, cx);
        } else {
            self.hover = Some(p.raw);
        }
        match self.pending(cx) {
            Some(Ok(profile)) => self.write(profile, cx),
            Some(Err(why)) => cx.say(Level::Warn, why),
            None => cx.say(Level::Info, "Burada duvar yok."),
        }
        self.see(cx);
    }

    fn accepts_points(&self) -> bool {
        self.draws()
    }

    fn accept_point(&mut self, p: Vec2, cx: &mut Context<'_>) -> bool {
        if !self.draws() {
            return false;
        }
        if self.pts.last() == Some(&p) {
            return true;
        }
        self.pts.push(p);
        if self.pts.len() == kentos_commands::MAX_BEAM_POINTS {
            let pts = std::mem::take(&mut self.pts);
            let profile = self.moulding_on(&pts, self.justify, cx);
            self.write(profile, cx);
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
                Ask::Base => "A",
                Ask::Material => "M",
            };
            if !self.apply(field, text, cx) {
                cx.say(
                    Level::Warn,
                    "Değer okunamadı; çıkıntı, taşma ve yükseklik sıfırdan büyük bir uzunluktur.",
                );
                return true;
            }
            self.ask = None;
            self.see(cx);
            return true;
        }
        self.ask = match key.as_str() {
            "T" => {
                self.set_coping(!self.coping);
                self.see(cx);
                return true;
            }
            "S" if !self.coping => {
                self.bound = if self.bound == Bound::Outline {
                    Bound::Draw
                } else {
                    Bound::Outline
                };
                self.pts.clear();
                self.see(cx);
                return true;
            }
            "E" => Some(Ask::Width),
            "Y" => Some(Ask::Height),
            "A" if !self.coping => Some(Ask::Base),
            "M" => Some(Ask::Material),
            "H" if self.draws() => {
                self.justify = match self.justify {
                    WallJustify::Center => WallJustify::Left,
                    WallJustify::Left => WallJustify::Right,
                    WallJustify::Right => WallJustify::Center,
                };
                self.see(cx);
                return true;
            }
            _ => {
                if self.draws()
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
                &[
                    ("moulding", "Cephe silmesi"),
                    ("coping", "Duvar harpuştası"),
                ],
                if self.coping { "coping" } else { "moulding" },
            ),
        )];
        if !self.coping {
            out.push(Setting::choice(
                "S",
                "Sınır",
                crate::tool_settings::choices(
                    &[("outline", "Dış yüzler"), ("draw", "Çiz")],
                    if self.bound == Bound::Outline {
                        "outline"
                    } else {
                        "draw"
                    },
                ),
            ));
        }
        out.push(Setting::text(
            "E",
            if self.coping { "Taşma" } else { "Çıkıntı" },
            f.length(self.width),
        ));
        out.push(Setting::text("Y", "Yükseklik", f.length(self.height)));
        if !self.coping {
            out.push(Setting::text("A", "Kattan yükseklik", f.length(self.base)));
        }
        if self.draws() {
            out.push(Setting::choice(
                "H",
                "Hiza",
                crate::tool_settings::choices(
                    &[("center", "Orta"), ("left", "Sol"), ("right", "Sağ")],
                    justify_key(self.justify),
                ),
            ));
        }
        out.push(Setting::text(
            "M",
            "Malzeme",
            self.material.clone().unwrap_or_else(|| "Yok".into()),
        ));
        out.push(Setting::hint(
            "P",
            "Yeri",
            if self.coping {
                "Duvar harpuştası tıklanan duvarın tepesine, ortasına oturur ve her iki yüzden taşma kadar taşar: parapetin, bahçe duvarının başlığı. IFC'de kaplama (IfcCovering, COPING); duvarı kısaltmaz."
            } else {
                "Cephe silmesi duvarların dış yüzü boyunca, katın döşemesinden kattan yükseklik kadar yukarıda dolanır; çıkıntısı kadar dışarı taşar: kat silmesi, subasman silmesi. IFC'de kaplama (IfcCovering, MOLDING)."
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
            "Değer uygulanmadı: çıkıntı, taşma ve yükseklik sıfırdan büyük bir uzunluktur.",
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
        if self.draws() && self.pts.len() >= 2 {
            let pts = std::mem::take(&mut self.pts);
            let profile = self.moulding_on(&pts, self.justify, cx);
            self.write(profile, cx);
            self.see(cx);
            return Flow::Stay;
        }
        Flow::Exit
    }

    fn preview(&self, _format: &Format) -> Preview {
        let mut out = self.drawn.clone();
        if self.draws() {
            out.path = self.axis();
        }
        out.tag = self.hover.map(|at| {
            let mut lines = vec![format!("{} {}", self.name(), self.size())];
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_ring_runs_round_once_from_the_middle_of_its_first_edge_counter_clockwise() {
        // Clockwise in, counter-clockwise out: from the middle of an edge round to it again.
        let ring = [
            Vec2::new(0.0, 0.0),
            Vec2::new(0.0, 4.0),
            Vec2::new(6.0, 4.0),
            Vec2::new(6.0, 0.0),
        ];
        let axis = round_axis(&ring).expect("a ring");
        assert_eq!(axis.len(), 6);
        assert_eq!(axis.first(), axis.last());
        let area: f64 = axis
            .windows(2)
            .map(|w| w[0].x * w[1].y - w[1].x * w[0].y)
            .sum();
        assert!(area > 0.0, "counter-clockwise");
        assert!(round_axis(&ring[..2]).is_none());
    }
}
