//! Kütle (docs/adr/8178): a massing study's volume on Etkin kat, on its EK-4 layer MM-XXXX-KUTLE (docs/adr/8008).
//!
//! - Oturum (S): Köşeleri çiz takes corners; Enter, Kapat (K) or a click on the first corner writes it. Dikdörtgen takes
//!   two opposite corners along the drawing's axes. Kapalı çizgiden takes the closed line, polygon or circle clicked (a
//!   parcel's setback line, an imported footprint), its arcs and its courtyards kept.
//! - Yükseklik (Y) 9.00 m until changed: three storeys of the 3.00 m PAİY 23/1-d counts a building's storeys by (KentOS's
//!   first value, not a regulation's). Kot farkı (F): its bottom over the storey's floor. Ad (A): "Blok A"; empty: none.
//!
//! The mass `cad.masses.create` would write is drawn with its footprint's area and its volume; the click that ends its
//! footprint writes it in one undo step, and the tool stays for the next.

use kentos_commands::{
    Entity, MAX_MASS_HEIGHT, MAX_MASS_NAME, MAX_MASS_OFFSET, MAX_SPACE_POINTS, MassesCreate,
    NewMass, SpacePart, SpaceRing,
};
use kentos_geometry_core::tools::point_text::js_trim;
use kentos_native_application::{ExecutionContext, masses_create};

use crate::Vec2;
use crate::arch_layer::Kind;
use crate::edge::Outline;
use crate::format::Format;
use crate::log::Level;
use crate::points;
use crate::prompt::{Prompt, upper_tr};
use crate::tool::{Area, Context, Cursor, Flow, OptionChoice, Pointer, Preview, Tag, Tone, Tool};
use crate::tool_settings::Setting;

pub const ID: &str = "mass";

/// A mass's height until changed, metres: three storeys of 3.00 m.
pub const MASS_HEIGHT: f64 = 9.0;

/// A footprint smaller than this, m², has no area: its corners on one line.
const FLAT: f64 = 1e-4;

/// How its footprint is given.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Footprint {
    #[default]
    Draw,
    Rectangle,
    Line,
}

impl Footprint {
    const ALL: [Footprint; 3] = [Footprint::Draw, Footprint::Rectangle, Footprint::Line];

    fn label(self) -> &'static str {
        match self {
            Footprint::Draw => "Köşeleri çiz",
            Footprint::Rectangle => "Dikdörtgen",
            Footprint::Line => "Kapalı çizgiden",
        }
    }

    fn name(self) -> &'static str {
        match self {
            Footprint::Draw => "draw",
            Footprint::Rectangle => "rectangle",
            Footprint::Line => "line",
        }
    }
}

/// What the tool waits for in words.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Ask {
    Height,
    Offset,
    Name,
}

/// Kütle.
#[derive(Clone, Debug)]
pub struct MassTool {
    footprint: Footprint,
    height: f64,
    offset: f64,
    name: Option<String>,
    pts: Vec<Vec2>,
    hover: Option<Vec2>,
    /// The pointer unsnapped: what Kapalı çizgiden picks by.
    raw: Option<Vec2>,
    ask: Option<Ask>,
    drawn: Preview,
    /// The previewed mass's footprint area, m², and volume, m³.
    size: Option<(f64, f64)>,
    why: Option<String>,
}

impl Default for MassTool {
    fn default() -> Self {
        MassTool {
            footprint: Footprint::Draw,
            height: MASS_HEIGHT,
            offset: 0.0,
            name: None,
            pts: Vec::new(),
            hover: None,
            raw: None,
            ask: None,
            drawn: Preview::default(),
            size: None,
            why: None,
        }
    }
}

/// A ring of `pts` and their bulges, a straight one without them.
fn ring(pts: &[kentos_commands::Vec2], bulges: Option<&Vec<f64>>) -> SpaceRing {
    SpaceRing {
        pts: pts.to_vec(),
        bulges: bulges.filter(|b| b.iter().any(|x| *x != 0.0)).cloned(),
    }
}

/// One part of the corners `pts`, no holes.
fn part(pts: &[Vec2]) -> SpacePart {
    SpacePart {
        outer: ring(&points::wire_all(pts), None),
        holes: Vec::new(),
    }
}

/// The rectangle of opposite corners `a` and `b` along the drawing's axes, counter-clockwise.
fn rectangle(a: Vec2, b: Vec2) -> SpacePart {
    let (x0, x1) = (a.x.min(b.x), a.x.max(b.x));
    let (y0, y1) = (a.y.min(b.y), a.y.max(b.y));
    part(&[
        Vec2::new(x0, y0),
        Vec2::new(x1, y0),
        Vec2::new(x1, y1),
        Vec2::new(x0, y1),
    ])
}

/// The footprint of a closed line, a polygon (its parts and holes) or a circle; none for any other object.
fn footprint_of(e: &Entity) -> Option<Vec<SpacePart>> {
    let holes = |h: &Option<Vec<kentos_commands::RingGeometry>>| {
        h.iter()
            .flatten()
            .map(|r| ring(&r.pts, r.bulges.as_ref()))
            .collect::<Vec<_>>()
    };
    match e {
        Entity::Polygon(p) if p.pts.len() >= 3 => {
            let mut out = vec![SpacePart {
                outer: ring(&p.pts, p.bulges.as_ref()),
                holes: holes(&p.holes),
            }];
            out.extend(
                p.parts
                    .iter()
                    .flatten()
                    .filter(|a| a.pts.len() >= 3)
                    .map(|a| SpacePart {
                        outer: ring(&a.pts, a.bulges.as_ref()),
                        holes: holes(&a.holes),
                    }),
            );
            Some(out)
        }
        Entity::Polyline(p) if p.closed && p.pts.len() >= 3 => Some(vec![SpacePart {
            outer: ring(&p.pts, p.bulges.as_ref()),
            holes: Vec::new(),
        }]),
        // An open line whose last vertex is its first: closed but not said so (a drawing from elsewhere).
        Entity::Polyline(p)
            if p.pts.len() >= 4 && p.pts.first() == p.pts.last() && p.parts.is_none() =>
        {
            let n = p.pts.len() - 1;
            let bulges = p
                .bulges
                .as_ref()
                .map(|b| b.iter().take(n).copied().collect::<Vec<_>>());
            Some(vec![SpacePart {
                outer: ring(&p.pts[..n], bulges.as_ref()),
                holes: Vec::new(),
            }])
        }
        // Two half circles counter-clockwise.
        Entity::Circle(c) if c.r > 0.0 => {
            let at = |dx: f64| kentos_commands::Vec2 {
                x: c.c.x + dx,
                y: c.c.y,
            };
            Some(vec![SpacePart {
                outer: SpaceRing {
                    pts: vec![at(c.r), at(-c.r)],
                    bulges: Some(vec![1.0, 1.0]),
                },
                holes: Vec::new(),
            }])
        }
        _ => None,
    }
}

/// The footprint's area, holes off, m².
fn area_of(parts: &[SpacePart]) -> f64 {
    kentos_geometry_core::geom::space::area(&kentos_native_application::spaces::areas(parts))
}

impl MassTool {
    pub fn new() -> Self {
        Self::default()
    }

    /// The call that writes one mass of `parts` on `layer_id`.
    fn call(&self, parts: Vec<SpacePart>, layer_id: String, cx: &Context<'_>) -> MassesCreate {
        MassesCreate {
            layer_id,
            masses: vec![NewMass {
                parts,
                height: self.height,
                offset: (self.offset != 0.0).then_some(self.offset),
                name: self.name.clone(),
                // Etkin kat (docs/adr/0713).
                arch: cx.new_arch(),
                attrs: None,
            }],
            color: cx.draft.color_text(),
            line_weight: cx.draft.line_weight,
            expected_revision: None,
        }
    }

    /// The footprint of the closed line under `at`; why there is none.
    fn line_at(at: Vec2, cx: &Context<'_>) -> Result<Vec<SpacePart>, String> {
        cx.spatial
            .hits(at, cx.pick_tolerance())
            .into_iter()
            .find_map(|s| footprint_of(cx.doc.get(s)?))
            .ok_or_else(|| {
                "Burada kapalı bir çizgi, alan ya da daire yok: kenarına tıklayın.".into()
            })
    }

    /// The corners so far and the pointer.
    fn line_now(&self) -> Vec<Vec2> {
        let mut pts = self.pts.clone();
        if let Some(h) = self.hover
            && pts.last() != Some(&h)
        {
            pts.push(h);
        }
        pts
    }

    /// The footprint as it stands with the pointer; none yet; why there is none.
    fn footprint_now(&self, cx: &Context<'_>) -> Result<Option<Vec<SpacePart>>, String> {
        Ok(match self.footprint {
            Footprint::Draw => {
                let pts = self.line_now();
                (pts.len() >= 3).then(|| vec![part(&pts)])
            }
            Footprint::Rectangle => match (self.pts.first(), self.hover) {
                (Some(&a), Some(b)) if a != b => Some(vec![rectangle(a, b)]),
                _ => None,
            },
            Footprint::Line => match self.raw {
                Some(at) => Some(Self::line_at(at, cx)?),
                None => None,
            },
        })
    }

    /// The mass the command would write now, drawn with its area and volume; why there is none.
    fn see(&mut self, cx: &Context<'_>) {
        self.drawn = Preview::default();
        self.size = None;
        self.why = None;
        let parts = match self.footprint_now(cx) {
            Ok(Some(parts)) => parts,
            Ok(None) => return,
            Err(why) => {
                self.why = Some(why);
                return;
            }
        };
        let input = self.call(parts, crate::arch_layer::planned(&Kind::Mass, cx), cx);
        match masses_create::plan_in(cx.doc, &input) {
            kentos_commands::CommandResult::Completed { output, .. } => {
                for e in &output.masses {
                    let shape = kentos_native_application::geometry::shape(e);
                    self.drawn
                        .strokes
                        .extend(Outline::of(&shape, None, 2.0, Tone::Accent).strokes);
                    let Entity::Mass(m) = e else { continue };
                    self.drawn.areas.push(Area {
                        rings: crate::slab_tools::rings_of(&m.mass.parts),
                        fill: 0.14,
                        width: 0.0,
                        dash: None,
                        fill_tone: Tone::Accent,
                    });
                    let area = area_of(&m.mass.parts);
                    self.size = Some((area, area * m.mass.height));
                    if area < FLAT {
                        self.why =
                            Some("Oturumun alanı yok: köşeleri bir doğrunun üstünde.".into());
                    }
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

    /// Writes a mass of `parts` on its layer, opened with it when missing (docs/adr/8008), as one undo step; says what
    /// it wrote.
    fn write(&self, parts: &[SpacePart], cx: &mut Context<'_>) -> bool {
        let area = area_of(parts);
        if area < FLAT {
            cx.say(
                Level::Warn,
                "Oturumun alanı yok: köşeleri bir doğrunun üstünde. Kütle çizilmedi.",
            );
            return false;
        }
        let written = crate::arch_layer::writing(&Kind::Mass, cx, |layer, cx| {
            let input = self.call(parts.to_vec(), layer.to_owned(), cx);
            let result = masses_create::execute(&mut ExecutionContext::new(cx.doc), input);
            points::written(result, cx).is_some()
        });
        if written {
            let f = cx.format();
            let name = self
                .name
                .as_ref()
                .map_or_else(String::new, |n| format!(" ({n})"));
            cx.say(
                Level::Success,
                format!(
                    "Kütle çizildi{name}: oturumu {}, {} yüksek, hacmi {} m³.",
                    f.area(area),
                    f.length(self.height),
                    crate::format::fixed(area * self.height, f.area_decimals)
                ),
            );
        }
        written
    }

    /// Writes the corners drawn so far; they stay when it is refused.
    fn write_drawn(&mut self, cx: &mut Context<'_>) {
        if self.write(&[part(&self.pts)], cx) {
            self.pts.clear();
        }
        self.see(cx);
    }

    /// A setting's value taken; false when it cannot be read.
    fn apply(&mut self, key: &str, value: &str, cx: &mut Context<'_>) -> bool {
        match key {
            "S" => match Footprint::ALL.into_iter().find(|f| f.name() == value) {
                Some(f) => {
                    if f != self.footprint {
                        self.footprint = f;
                        self.pts.clear();
                    }
                    true
                }
                None => false,
            },
            "Y" => cx
                .typed_length(value)
                .filter(|v| *v > 0.0 && *v <= MAX_MASS_HEIGHT)
                .map(|v| self.height = v)
                .is_some(),
            "F" => {
                let t = js_trim(value);
                let v = if t.is_empty() {
                    Some(0.0)
                } else {
                    cx.typed_length(t).filter(|v| v.abs() <= MAX_MASS_OFFSET)
                };
                v.map(|v| self.offset = v).is_some()
            }
            "A" => {
                let t = value.trim();
                if t.chars().count() > MAX_MASS_NAME || t.chars().any(char::is_control) {
                    return false;
                }
                self.name = (!t.is_empty()).then(|| t.to_owned());
                true
            }
            _ => false,
        }
    }

    fn refusal(key: &str, f: &Format) -> String {
        match key {
            "Y" => format!(
                "Kütlenin yüksekliği sıfırdan büyük ve en çok {} olmalı.",
                f.length(MAX_MASS_HEIGHT)
            ),
            "F" => format!(
                "Kot farkı {} ile {} arasında olmalı.",
                f.length(-MAX_MASS_OFFSET),
                f.length(MAX_MASS_OFFSET)
            ),
            "A" => format!(
                "Kütlenin adı en çok {MAX_MASS_NAME} karakter olur ve denetim karakteri taşımaz."
            ),
            _ => "Geçerli bir kütle seçeneği belirleyin.".into(),
        }
    }
}

impl Tool for MassTool {
    fn layer_kind(&self, _cx: &Context<'_>) -> Option<Kind> {
        Some(Kind::Mass)
    }

    fn id(&self) -> &'static str {
        ID
    }

    fn label(&self) -> &'static str {
        "Kütle"
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
        if let Some(ask) = self.ask {
            let words = match ask {
                Ask::Height => "tabanından tepesine yüksekliğini yazın",
                Ask::Offset => {
                    "tabanının katın döşemesinden kot farkını yazın (0: katın döşemesinde)"
                }
                Ask::Name => "adını yazın (Blok A); boş: adsız",
            };
            return Prompt::new("Kütle", words).option("Geri", "G");
        }
        let n = |v: f64| v.to_string();
        let step = match (self.footprint, self.pts.len()) {
            (Footprint::Line, _) => "kapalı çizginin, alanın ya da dairenin kenarına tıklayın",
            (Footprint::Rectangle, 0) => "dikdörtgenin bir köşesine tıklayın",
            (Footprint::Rectangle, _) => "karşı köşesine tıklayın",
            (Footprint::Draw, 0) => "oturumunun ilk köşesine tıklayın",
            (Footprint::Draw, 1 | 2) => "sonraki köşeye tıklayın",
            (Footprint::Draw, _) => "sonraki köşeye tıklayın ya da Enter ile yazın",
        };
        let mut p = Prompt::new("Kütle", step)
            .option_with("Oturum", "S", self.footprint.label())
            .option_with("Yükseklik", "Y", n(self.height))
            .option_with("Kot farkı", "F", n(self.offset))
            .option_with("Ad", "A", self.name.as_deref().unwrap_or("yok"));
        if self.footprint == Footprint::Draw && self.pts.len() >= 3 {
            p = p.option("Kapat", "K").option("Bitir", "Enter");
        }
        p
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.hover = Some(p.world);
        self.raw = Some(p.raw);
        self.see(cx);
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        if self.footprint == Footprint::Line {
            match Self::line_at(p.raw, cx) {
                Ok(parts) => {
                    self.write(&parts, cx);
                }
                Err(why) => cx.say(Level::Info, why),
            }
            self.see(cx);
            return;
        }
        self.accept_point(p.world, cx);
    }

    fn accepts_points(&self) -> bool {
        self.footprint != Footprint::Line
    }

    fn accept_point(&mut self, p: Vec2, cx: &mut Context<'_>) -> bool {
        self.hover = Some(p);
        match self.footprint {
            Footprint::Line => return false,
            Footprint::Rectangle => match self.pts.first() {
                None => self.pts.push(p),
                Some(&a) => {
                    if self.write(&[rectangle(a, p)], cx) {
                        self.pts.clear();
                    }
                }
            },
            Footprint::Draw => {
                if self.pts.last() == Some(&p) {
                    return true;
                }
                // Its first corner again closes it.
                if self.pts.len() >= 3 && self.pts.first() == Some(&p) {
                    self.write_drawn(cx);
                    return true;
                }
                if self.pts.len() == MAX_SPACE_POINTS {
                    cx.say(
                        Level::Warn,
                        "Köşe sınırına ulaşıldı: Enter ile yazın ya da son köşeyi geri alın.",
                    );
                    return true;
                }
                self.pts.push(p);
            }
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
                Ask::Height => "Y",
                Ask::Offset => "F",
                Ask::Name => "A",
            };
            if !self.apply(field, text, cx) {
                cx.say(Level::Warn, Self::refusal(field, &cx.format()));
                return true;
            }
            self.ask = None;
            self.see(cx);
            return true;
        }
        self.ask = match key.as_str() {
            "S" => {
                let i = Footprint::ALL
                    .iter()
                    .position(|f| *f == self.footprint)
                    .unwrap_or(0);
                self.apply(
                    "S",
                    Footprint::ALL[(i + 1) % Footprint::ALL.len()].name(),
                    cx,
                );
                self.see(cx);
                return true;
            }
            "K" if self.footprint == Footprint::Draw && self.pts.len() >= 3 => {
                self.write_drawn(cx);
                return true;
            }
            "Y" => Some(Ask::Height),
            "F" => Some(Ask::Offset),
            "A" => Some(Ask::Name),
            _ => {
                if self.accepts_points()
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
        let footprints: Vec<(&str, &str)> = Footprint::ALL
            .iter()
            .map(|x| (x.name(), x.label()))
            .collect();
        vec![
            Setting::choice(
                "S",
                "Oturum",
                crate::tool_settings::choices(&footprints, self.footprint.name()),
            ),
            Setting::text("Y", "Yükseklik", f.length(self.height)),
            Setting::text("F", "Kot farkı", f.length(self.offset)),
            Setting::text("A", "Ad", self.name.clone().unwrap_or_default()),
            Setting::hint(
                "P",
                "Kütle",
                "Tasarımın duvarları ve döşemeleri çizilmeden önceki hacmi: oturumu ve yüksekliğiyle bir prizma. Planda, \
                 kesitte ve 3B'de görünür, gölge düşürür; binanın elemanı değildir: IFC'ye, metraja ve denetime girmez.",
            ),
        ]
    }

    fn cursor_settings(&self) -> &'static [&'static str] {
        &["Y"]
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
        cx.say(Level::Warn, Self::refusal(key, &cx.format()));
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
        if self.footprint == Footprint::Draw && self.pts.len() >= 3 {
            self.write_drawn(cx);
            return Flow::Stay;
        }
        Flow::Exit
    }

    fn preview(&self, format: &Format) -> Preview {
        let mut out = self.drawn.clone();
        if self.footprint == Footprint::Draw {
            out.path = self.line_now();
        }
        out.tag = self.hover.map(|at| {
            let mut first = format!("Kütle {}", format.length(self.height));
            if let Some(name) = &self.name {
                first = format!("{name} · {first}");
            }
            let mut lines = vec![first];
            if let Some((area, volume)) = self.size {
                lines.push(format!(
                    "Oturum {} · hacim {} m³",
                    format.area(area),
                    crate::format::fixed(volume, format.area_decimals)
                ));
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
