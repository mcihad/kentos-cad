//! Çatı bacası and Çatı penceresi (docs/adr/8044 §6). The tool says which roof was clicked and where; `cad.arch.edit`
//! writes the roof's parts, the ones it had and the new one, as one undo step, and its plan is the preview.
//!
//! - Çatı bacası: Tür (T) Duman bacası, its top Yükseklik (Y, 80 cm) over the ridge (Binaların Yangından Korunması
//!   Hakkında Yönetmelik 58/2: "ait olduğu bina mahyasının en az 0.8 m üzerine"), or Hava bacası, its top 60 cm over
//!   the roof at its highest corner (PAİY 40/5: at most that); En (E) and Derinlik (D), 40 × 40 cm. Squared with the
//!   eaves nearest it, as the building stands.
//! - Çatı penceresi: En (E, 78 cm) along its face's eaves and Boy (Y, 118 cm, Kolon's letter) down its slope, in the roof's plane, the
//!   usual roof window; its middle on a sloping face and its outline wholly in it. Sivas İmar Yönetmeliği 40 allows at
//!   most 1.20 × 1.50 m: the preview says when it is larger.
//!
//! - İniş borusu (docs/adr/8044 §8): Çap (Ç, Ø100 mm); a click near a sloping eaves edge puts it under the gutter, its
//!   middle at the gutter's middle (Mimarlık Hizmetleri Şartnamesi 14.5.3 x: drawn at its real size, its size written).
//!
//! The roof is the one whose eaves are round the click among those the view shows (a downpipe's: whose sloping eaves
//! the click is nearest); one the view draws whole before one it draws by its eaves alone (its own storey's plan, where
//! its parts are not drawn), then the highest.

use kentos_commands::{
    ArchChange, ArchEdit, ArchOperation, CommandResult, Entity, RoofEntity, RoofPart, RoofPartKind,
};
use kentos_domain::Slot;
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::geom::roof;
use kentos_geometry_core::tools::point_text::js_trim;
use kentos_native_application::{ExecutionContext, arch_edit};

use crate::Vec2;
use crate::format::Format;
use crate::log::Level;
use crate::points;
use crate::prompt::{Prompt, upper_tr};
use crate::tool::{Context, Cursor, Flow, OptionChoice, Pointer, Preview, Stroke, Tag, Tone, Tool};

pub const CHIMNEY_ID: &str = "roofChimney";
pub const WINDOW_ID: &str = "roofWindow";
pub const DOWNPIPE_ID: &str = "roofDownpipe";
/// How far from a sloping eaves edge a click still means a downpipe under its gutter, metres.
const DOWNPIPE_REACH: f64 = 1.0;

/// A chimney's and a vent's width and depth when none is given, metres: a flue in brick, as drawn on permit sets.
pub const CHIMNEY_SIZE: f64 = 0.4;
/// A roof window's width and depth when none is given, metres: the usual 78 × 118 cm.
pub const WINDOW_SIZE: (f64, f64) = (0.78, 1.18);
/// Sivas İmar Yönetmeliği 40: a roof window at most 1.20 m wide and 1.50 m long.
pub const SIVAS_WINDOW: (f64, f64) = (1.2, 1.5);

/// What the tool waits for after its key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Ask {
    Width,
    Depth,
    Rise,
}

/// What the plan writes: the roof's ridge and the part's top over ±0.00 (a chimney's or a vent's), the face's slope (a
/// window's).
#[derive(Clone, Copy, Debug, PartialEq)]
struct Made {
    ridge: f64,
    top: Option<f64>,
    slope: Option<f64>,
}

/// Çatı bacası or Çatı penceresi.
#[derive(Clone, Debug)]
pub struct RoofPartTool {
    kind: RoofPartKind,
    width: f64,
    depth: f64,
    /// A chimney's or a vent's top over the ridge or the roof as typed; none: its kind's.
    rise: Option<f64>,
    hover: Option<Vec2>,
    ask: Option<Ask>,
    drawn: Preview,
    made: Option<Made>,
    why: Option<String>,
    format: Format,
}

impl RoofPartTool {
    /// Çatı bacası: a chimney (Duman bacası) until Tür says a vent.
    pub fn chimney() -> Self {
        Self::of(RoofPartKind::Chimney, CHIMNEY_SIZE, CHIMNEY_SIZE)
    }

    /// Çatı penceresi.
    pub fn window() -> Self {
        Self::of(RoofPartKind::Window, WINDOW_SIZE.0, WINDOW_SIZE.1)
    }

    /// İniş borusu.
    pub fn downpipe() -> Self {
        let d = kentos_commands::DEFAULT_DOWNPIPE;
        Self::of(RoofPartKind::Downpipe, d, d)
    }

    fn of(kind: RoofPartKind, width: f64, depth: f64) -> Self {
        Self {
            kind,
            width,
            depth,
            rise: None,
            hover: None,
            ask: None,
            drawn: Preview::default(),
            made: None,
            why: None,
            format: Format::default(),
        }
    }

    fn is_window(&self) -> bool {
        self.kind == RoofPartKind::Window
    }

    fn is_downpipe(&self) -> bool {
        self.kind == RoofPartKind::Downpipe
    }

    /// A chimney made a vent or back: its rise its new kind's rule (80 cm over the ridge, 60 cm over the roof) again.
    fn set_kind(&mut self, kind: RoofPartKind) {
        if kind != self.kind {
            self.rise = None;
        }
        self.kind = kind;
    }

    /// A chimney's or a vent's top over the ridge or the roof: as typed, else its kind's.
    fn rise_now(&self) -> f64 {
        self.rise.unwrap_or(match self.kind {
            RoofPartKind::Vent => kentos_commands::VENT_OVER_ROOF,
            _ => kentos_commands::CHIMNEY_OVER_RIDGE,
        })
    }

    /// What a chimney's or a vent's rise is over.
    fn over_words(&self) -> &'static str {
        match self.kind {
            RoofPartKind::Vent => "çatının",
            _ => "mahyanın",
        }
    }

    /// The part's name in a sentence's object: "bacayı", "pencereyi", "boruyu".
    fn object_word(&self) -> &'static str {
        match self.kind {
            RoofPartKind::Window => "pencereyi",
            RoofPartKind::Downpipe => "boruyu",
            _ => "bacayı",
        }
    }

    /// The roof whose eaves are round `at` among those the view shows: one drawn whole before one drawn by its eaves
    /// alone, then the highest eaves; why there is none to put the part on.
    fn roof_at(&self, at: Vec2, cx: &Context<'_>) -> Result<(Slot, RoofEntity), String> {
        let (_, looks) = cx.spatial.level_looks();
        let settings = cx.doc.settings();
        let eaves = |r: &RoofEntity| {
            kentos_native_application::geometry::storey_top(settings, r.roof.arch.level.as_deref())
                .unwrap_or(0.0)
                + r.roof.offset
        };
        let mut found: Vec<(Slot, &RoofEntity)> = cx
            .doc
            .entities_with_uids()
            .filter_map(|(uid, e)| match e {
                Entity::Roof(r) => Some((cx.doc.slot_of(uid)?, r)),
                _ => None,
            })
            .filter(|(slot, r)| {
                cx.doc.layers().is_visible(&r.base.layer_id)
                    && !looks.hidden.contains(&slot.0)
                    && cx.spatial.view_shown(*slot)
            })
            .filter(|(_, r)| {
                let ring: Vec<Vec2> = r.roof.pts.iter().map(|p| Vec2::new(p.x, p.y)).collect();
                kentos_geometry_core::geometry::point_in_polygon(at, &ring)
            })
            .collect();
        found.sort_by(|(a, ra), (b, rb)| {
            looks
                .roofs
                .contains(&a.0)
                .cmp(&looks.roofs.contains(&b.0))
                .then(eaves(rb).total_cmp(&eaves(ra)))
        });
        let Some((slot, r)) = found.first() else {
            return Err(format!(
                "Burada çatı yok; {} çatının saçak çizgisinin içine koyun.",
                self.object_word()
            ));
        };
        if looks.roofs.contains(&slot.0) {
            let up = r
                .roof
                .arch
                .level
                .as_deref()
                .and_then(|l| crate::level_view::above(settings, l))
                .map(|l| format!("“{}” planında ya da ", l.name))
                .unwrap_or_default();
            return Err(format!(
                "Bu katın planında çatının yalnız saçağı görünüyor; bacaları ve pencereleri {up}Bütün katlar'da konur."
            ));
        }
        Ok((*slot, (*r).clone()))
    }

    /// The roof whose sloping eaves are nearest `at`, within [`DOWNPIPE_REACH`], among those the view draws whole; the
    /// downpipe's middle under its gutter there: the click moved square onto the nearest sloping eaves edge, then out by
    /// half the gutter's width (as it is, when the roof has no gutter: the command says so).
    fn downpipe_at(&self, at: Vec2, cx: &Context<'_>) -> Result<(Slot, RoofEntity, Vec2), String> {
        let (_, looks) = cx.spatial.level_looks();
        let mut best: Option<(f64, Slot, &RoofEntity, Vec2)> = None;
        for (uid, e) in cx.doc.entities_with_uids() {
            let Entity::Roof(r) = e else { continue };
            let Some(slot) = cx.doc.slot_of(uid) else {
                continue;
            };
            if !cx.doc.layers().is_visible(&r.base.layer_id)
                || looks.hidden.contains(&slot.0)
                || looks.roofs.contains(&slot.0)
                || !cx.spatial.view_shown(slot)
            {
                continue;
            }
            let pts = &r.roof.pts;
            let n = pts.len();
            let ccw = pts
                .iter()
                .enumerate()
                .map(|(i, a)| {
                    let b = pts[(i + 1) % n];
                    a.x * b.y - b.x * a.y
                })
                .sum::<f64>()
                >= 0.0;
            for i in (0..n).filter(|i| r.roof.slopes.get(*i).is_some_and(Option::is_some)) {
                let (a, b) = (pts[i], pts[(i + 1) % n]);
                let (dx, dy) = (b.x - a.x, b.y - a.y);
                let len2 = dx * dx + dy * dy;
                if len2 <= 0.0 {
                    continue;
                }
                let t = (((at.x - a.x) * dx + (at.y - a.y) * dy) / len2).clamp(0.0, 1.0);
                let foot = Vec2::new(a.x + dx * t, a.y + dy * t);
                let d = (foot.x - at.x).hypot(foot.y - at.y);
                if d > DOWNPIPE_REACH || best.as_ref().is_some_and(|(bd, ..)| *bd <= d) {
                    continue;
                }
                let l = len2.sqrt();
                let out = if ccw {
                    Vec2::new(dy / l, -dx / l)
                } else {
                    Vec2::new(-dy / l, dx / l)
                };
                // Set at a corner it comes down the wall, not the air past the building's corner (docs/adr/8044 §8):
                // on a roof drawn from its walls its middle stays the overhang, the clips' standoff and its radius in
                // from the eaves' ends.
                let margin = r
                    .roof
                    .overhang
                    .map_or(0.0, |o| o + roof::PIPE_STANDOFF + self.width / 2.0);
                let t = if l > 2.0 * margin {
                    t.clamp(margin / l, 1.0 - margin / l)
                } else {
                    t
                };
                let foot = Vec2::new(a.x + dx * t, a.y + dy * t);
                let half = r.roof.gutter.map_or(0.0, |g| g / 2.0);
                let place = r.roof.gutter.map_or(at, |_| {
                    Vec2::new(foot.x + out.x * half, foot.y + out.y * half)
                });
                best = Some((d, slot, r, place));
            }
        }
        best.map(|(_, slot, r, place)| (slot, r.clone(), place))
            .ok_or_else(|| {
                "Yakında eğimli bir saçak yok; boruyu çatının oluğunun yanına koyun.".to_owned()
            })
    }

    /// The part at `at` on `r`: a chimney or a vent squared with the eaves nearest it (its turn the eaves' direction
    /// within ±45°: an upright building's chimney is not turned); a window takes its face's.
    fn part_at(&self, at: Vec2, r: &RoofEntity) -> RoofPart {
        let rotation = (!self.is_window() && !self.is_downpipe())
            .then(|| {
                let pts = &r.roof.pts;
                let k = kentos_native_application::roofs::nearest_edge(pts, points::wire(at));
                let (a, b) = (pts[k], pts[(k + 1) % pts.len()]);
                let quarter = std::f64::consts::FRAC_PI_2;
                let mut turn = (b.y - a.y).atan2(b.x - a.x).rem_euclid(quarter);
                if turn > quarter / 2.0 {
                    turn -= quarter;
                }
                turn
            })
            .filter(|t| t.abs() > 1e-9);
        RoofPart {
            kind: self.kind,
            at: points::wire(at),
            width: self.width,
            depth: self.depth,
            rotation,
            rise: (!self.is_window() && !self.is_downpipe()).then(|| self.rise_now()),
        }
    }

    /// `cad.arch.edit`'s input: the roof's parts and the new one.
    fn input(slot: Slot, r: &RoofEntity, part: RoofPart, cx: &Context<'_>) -> Option<ArchEdit> {
        let mut parts = r.roof.parts.clone();
        parts.push(part);
        Some(ArchEdit {
            uids: vec![cx.doc.uid(slot)?.to_string()],
            operation: ArchOperation::Set(Box::new(ArchChange {
                parts: Some(parts),
                ..Default::default()
            })),
            expected_revision: None,
        })
    }

    /// What the plan writes for a part at `at`: the input, what it makes and the part's outline (a chimney's with its
    /// diagonals); why it writes none.
    fn planned(&self, at: Vec2, cx: &Context<'_>) -> Result<(ArchEdit, Made, Vec<Stroke>), String> {
        let (slot, r, at) = if self.is_downpipe() {
            self.downpipe_at(at, cx)?
        } else {
            let (slot, r) = self.roof_at(at, cx)?;
            (slot, r, at)
        };
        let input = Self::input(slot, &r, self.part_at(at, &r), cx).ok_or_else(String::new)?;
        let written = match arch_edit::plan_in(cx.doc, &input) {
            CommandResult::Completed { output, .. } => output,
            CommandResult::Failed { error }
            | CommandResult::Conflict { error }
            | CommandResult::NeedsInput { error } => return Err(error.message),
            CommandResult::Queued { .. } | CommandResult::Cancelled => return Err(String::new()),
        };
        let Some(Entity::Roof(after)) = written.elements.first() else {
            return Err(String::new());
        };
        let Shape::Roof { pts, roof: spec } =
            kentos_native_application::geometry::roof_shape(after, cx.doc.settings())
        else {
            return Err(String::new());
        };
        let Some(part) = spec.parts.last() else {
            return Err(String::new());
        };
        // A downpipe round, at its size.
        let ring = if self.is_downpipe() {
            let r = part.width / 2.0;
            (0..24)
                .map(|k| {
                    let a = std::f64::consts::TAU * f64::from(k) / 24.0;
                    Vec2::new(part.at.x + r * a.cos(), part.at.y + r * a.sin())
                })
                .collect()
        } else {
            roof::part_ring(&pts, &spec, part).unwrap_or_default()
        };
        let made = Made {
            ridge: spec.base + roof::ridge_rise(&pts, &spec).unwrap_or(0.0),
            top: roof::part_top(&pts, &spec, part),
            slope: roof::part_face(&pts, &spec, part)
                .and_then(|edge| spec.slopes.get(edge).copied().flatten()),
        };
        let mut strokes = vec![Stroke {
            pts: ring.clone(),
            closed: true,
            dash: None,
            width: 2.0,
            tone: Tone::Accent,
        }];
        // As the plan draws them: a chimney's two diagonals, a vent's one.
        if ring.len() == 4 {
            let diagonals: &[[usize; 2]] = match self.kind {
                RoofPartKind::Chimney => &[[0, 2], [1, 3]],
                RoofPartKind::Vent => &[[0, 2]],
                RoofPartKind::Window | RoofPartKind::Downpipe => &[],
            };
            strokes.extend(diagonals.iter().map(|[i, j]| Stroke {
                pts: vec![ring[*i], ring[*j]],
                closed: false,
                dash: None,
                width: 1.0,
                tone: Tone::Accent,
            }));
        }
        Ok((input, made, strokes))
    }

    /// The plan at the hover, drawn.
    fn see(&mut self, cx: &Context<'_>) {
        self.format = cx.format();
        self.drawn = Preview::default();
        self.made = None;
        self.why = None;
        let Some(at) = self.hover else {
            return;
        };
        match self.planned(at, cx) {
            Ok((_, made, strokes)) => {
                self.drawn.strokes = strokes;
                self.made = Some(made);
            }
            Err(why) => self.why = (!why.is_empty()).then_some(why),
        }
    }

    /// Writes the part at `at` into the roof under it; says what it wrote, or why not.
    fn write(&mut self, at: Vec2, cx: &mut Context<'_>) -> bool {
        let (input, made, _) = match self.planned(at, cx) {
            Ok(planned) => planned,
            Err(why) => {
                if !why.is_empty() {
                    cx.say(Level::Warn, why);
                }
                return false;
            }
        };
        let result = arch_edit::execute(&mut ExecutionContext::new(cx.doc), input);
        if points::written(result, cx).is_none() {
            return false;
        }
        let f = cx.format();
        let said = match (self.kind, made.top) {
            (RoofPartKind::Downpipe, _) => format!(
                "İniş borusu kondu: Ø{}, oluğun altında.",
                (self.width * 1000.0).round()
            ),
            (RoofPartKind::Window, _) => format!(
                "Çatı penceresi kondu: {}{}.",
                size(&f, self.width, self.depth),
                made.slope
                    .map(|s| format!(", {} eğimli yüzde", roof::slope_words(s)))
                    .unwrap_or_default()
            ),
            (kind, Some(top)) => format!(
                "{} kondu; tepesi {} ({} {} üstünde).",
                kind.label(),
                roof::level_words(top),
                self.over_words(),
                f.length(self.rise_now())
            ),
            (kind, None) => format!("{} kondu.", kind.label()),
        };
        cx.say(Level::Success, said);
        true
    }

    /// What the preview warns of, in the rule's words: a chimney under 80 cm over the ridge, a vent over 60 cm over
    /// the roof, a window larger than Sivas's.
    fn warning(&self, f: &Format) -> Option<String> {
        match self.kind {
            RoofPartKind::Chimney
                if self.rise_now() < kentos_commands::CHIMNEY_OVER_RIDGE - 1e-9 =>
            {
                Some(format!(
                    "Yangın Yön. 58/2: mahyanın en az {} üstüne çıkar",
                    f.length(kentos_commands::CHIMNEY_OVER_RIDGE)
                ))
            }
            RoofPartKind::Vent if self.rise_now() > kentos_commands::VENT_OVER_ROOF + 1e-9 => {
                Some(format!(
                    "PAİY 40/5: çatıyı en çok {} aşar",
                    f.length(kentos_commands::VENT_OVER_ROOF)
                ))
            }
            RoofPartKind::Window
                if self.width > SIVAS_WINDOW.0 + 1e-9 || self.depth > SIVAS_WINDOW.1 + 1e-9 =>
            {
                Some(format!(
                    "Sivas İmar Yön. 40: en çok {}",
                    size(f, SIVAS_WINDOW.0, SIVAS_WINDOW.1)
                ))
            }
            _ => None,
        }
    }

    fn kind_choices(&self) -> Vec<OptionChoice> {
        [RoofPartKind::Chimney, RoofPartKind::Vent]
            .iter()
            .map(|k| OptionChoice {
                typed: kind_name(*k).into(),
                label: k.label().into(),
                icon: Some(kind_icon(*k)),
                preview: None,
                checked: *k == self.kind,
                command: None,
            })
            .collect()
    }

    fn depth_label(&self) -> &'static str {
        if self.is_window() { "Boy" } else { "Derinlik" }
    }

    fn depth_key(&self) -> &'static str {
        if self.is_window() { "Y" } else { "D" }
    }

    fn ask_of(&self, key: &str) -> Option<Ask> {
        if self.is_downpipe() {
            return (key == "Ç").then_some(Ask::Width);
        }
        Some(match key {
            "E" => Ask::Width,
            "Y" if self.is_window() => Ask::Depth,
            "D" if !self.is_window() => Ask::Depth,
            "Y" if !self.is_window() => Ask::Rise,
            _ => return None,
        })
    }

    /// A typed value set; false (and said) when it is none.
    fn set(&mut self, ask: Ask, text: &str, cx: &mut Context<'_>) -> bool {
        let text = js_trim(text);
        if ask == Ask::Rise && text.is_empty() {
            self.rise = None;
            return true;
        }
        let Some(v) = cx.typed_length(text).filter(|v| v.is_finite()) else {
            cx.say(
                Level::Warn,
                format!("“{text}” bir uzunluk değil; 40 cm ya da 0.40 gibi yazın."),
            );
            return false;
        };
        let top = kentos_commands::MAX_ROOF_PART;
        match ask {
            Ask::Width | Ask::Depth if !(v > 0.0 && v <= top) => {
                cx.say(
                    Level::Warn,
                    format!(
                        "Ölçü sıfırdan büyük, en çok {} olmalı.",
                        cx.format().length(top)
                    ),
                );
                return false;
            }
            // A downpipe is round: its depth its width.
            Ask::Width if self.is_downpipe() => {
                self.width = v;
                self.depth = v;
            }
            Ask::Width => self.width = v,
            Ask::Depth => self.depth = v,
            Ask::Rise if v.abs() > top => {
                cx.say(
                    Level::Warn,
                    format!("Yükseklik en çok ±{} olabilir.", cx.format().length(top)),
                );
                return false;
            }
            Ask::Rise => self.rise = Some(v),
        }
        true
    }
}

/// A part's size: its width and depth, the unit once ("0.780 × 1.180 m").
fn size(f: &Format, width: f64, depth: f64) -> String {
    format!("{} × {}", f.length_bare(width), f.length(depth))
}

/// A kind's name on the command line.
fn kind_name(kind: RoofPartKind) -> &'static str {
    match kind {
        RoofPartKind::Chimney => "chimney",
        RoofPartKind::Vent => "vent",
        RoofPartKind::Window => "window",
        RoofPartKind::Downpipe => "downpipe",
    }
}

/// A kind's icon in Tür's menu.
fn kind_icon(kind: RoofPartKind) -> &'static str {
    match kind {
        RoofPartKind::Chimney => "roofChimney",
        RoofPartKind::Vent => "roofVent",
        RoofPartKind::Window => "roofWindow",
        RoofPartKind::Downpipe => "roofDownpipe",
    }
}

impl Tool for RoofPartTool {
    fn activate(&mut self, cx: &mut Context<'_>) -> Flow {
        self.see(cx);
        Flow::Stay
    }

    fn level_changed(&mut self, cx: &mut Context<'_>) {
        self.see(cx);
    }

    fn id(&self) -> &'static str {
        match self.kind {
            RoofPartKind::Window => WINDOW_ID,
            RoofPartKind::Downpipe => DOWNPIPE_ID,
            _ => CHIMNEY_ID,
        }
    }

    fn label(&self) -> &'static str {
        match self.kind {
            RoofPartKind::Window => "Çatı penceresi",
            RoofPartKind::Downpipe => "İniş borusu",
            _ => "Çatı bacası",
        }
    }

    fn cursor(&self) -> Cursor {
        Cursor::Cross
    }

    fn snaps(&self) -> bool {
        true
    }

    /// One click a part: no points kept.
    fn point_count(&self) -> usize {
        0
    }

    fn undo_step(&mut self, _cx: &mut Context<'_>) -> bool {
        false
    }

    fn prompt(&self) -> Prompt {
        let title = self.label();
        let words = |label: &'static str| Prompt::new(title, label).option("Geri", "G");
        match self.ask {
            Some(Ask::Width) if self.is_downpipe() => {
                return words("borunun çapını yazın (10 cm gibi)");
            }
            Some(Ask::Width) if self.is_window() => {
                return words("pencerenin saçak boyunca enini yazın (78 cm gibi)");
            }
            Some(Ask::Width) => return words("bacanın enini yazın (40 cm gibi)"),
            Some(Ask::Depth) if self.is_window() => {
                return words("pencerenin eğim boyunca boyunu yazın (118 cm gibi)");
            }
            Some(Ask::Depth) => return words("bacanın derinliğini yazın (40 cm gibi)"),
            Some(Ask::Rise) => {
                return words(match self.kind {
                    RoofPartKind::Vent => {
                        "çatının üstündeki yüksekliğini yazın (60 cm gibi); boş: 60 cm"
                    }
                    _ => "mahyanın üstündeki yüksekliğini yazın (80 cm gibi); boş: 80 cm",
                });
            }
            None => {}
        }
        let f = &self.format;
        if self.is_downpipe() {
            return Prompt::new(
                title,
                "borunun yerine, eğimli saçağın oluğunun yanına tıklayın",
            )
            .option_with("Çap", "Ç", f.length(self.width))
            .option("Bitir", "Enter");
        }
        if self.is_window() {
            return Prompt::new(
                title,
                "pencerenin ortasına, eğimli bir çatı yüzünün üstüne tıklayın",
            )
            .option_with("En", "E", f.length(self.width))
            .option_with("Boy", "Y", f.length(self.depth))
            .option("Bitir", "Enter");
        }
        Prompt::new(title, "bacanın ortasına, çatının üstüne tıklayın")
            .option_with("Tür", "T", self.kind.label())
            .option_with("En", "E", f.length(self.width))
            .option_with("Derinlik", "D", f.length(self.depth))
            .option_with("Yükseklik", "Y", f.length(self.rise_now()))
            .option("Bitir", "Enter")
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.hover = Some(p.world);
        self.see(cx);
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.write(p.world, cx);
        self.see(cx);
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
        if key == "T" && !self.is_window() {
            let other = match self.kind {
                RoofPartKind::Chimney => RoofPartKind::Vent,
                _ => RoofPartKind::Chimney,
            };
            self.set_kind(other);
            self.see(cx);
            return true;
        }
        if let Some(ask) = self.ask_of(&key) {
            self.ask = Some(ask);
            return true;
        }
        // A typed point: the part's middle there.
        if let Some(p) = cx.typed_point(text, None, self.hover) {
            return self.write(p, cx);
        }
        false
    }

    fn option_choices(&self, key: &str) -> Vec<OptionChoice> {
        match key {
            "T" if !self.is_window() => self.kind_choices(),
            _ => Vec::new(),
        }
    }

    fn settings(&self, f: &Format) -> Vec<crate::tool_settings::Setting> {
        use crate::tool_settings::Setting;
        let mut out = Vec::new();
        if self.is_downpipe() {
            out.push(Setting::text("Ç", "Çap", f.length(self.width)));
            out.push(Setting::hint(
                "TH",
                "Yerleşim",
                "Boru oluğun altına, ortası oluğun ortasına gelecek biçimde konur; planda gerçek boyunda çizilir ve çapı \
                 yazılır (Mimarlık Hizmetleri Şartnamesi 14.5.3 x). Çap yağış ve çatı alanına göre TS EN 12056-3 ile \
                 seçilir.",
            ));
            return out;
        }
        if !self.is_window() {
            out.push(Setting::choice("T", "Tür", self.kind_choices()));
        }
        out.push(Setting::text("E", "En", f.length(self.width)));
        out.push(Setting::text(
            self.depth_key(),
            self.depth_label(),
            f.length(self.depth),
        ));
        if self.is_window() {
            out.push(Setting::hint(
                "TH",
                "Yönetmelik",
                "Pencere çatının düzlemindedir: eni saçak boyunca, boyu eğim boyunca. Sivas İmar Yönetmeliği 40: en \
                 çok 1,20 × 1,50 m, cepheye en az 1 m, birbirine en az 2 m.",
            ));
        } else {
            out.push(Setting::text(
                "Y",
                match self.kind {
                    RoofPartKind::Vent => "Yükseklik (çatıdan)",
                    _ => "Yükseklik (mahyadan)",
                },
                f.length(self.rise_now()),
            ));
            out.push(Setting::hint(
                "TH",
                "Yönetmelik",
                "Duman bacası mahyanın en az 80 cm üstüne çıkar (Binaların Yangından Korunması Hakkında Yönetmelik \
                 58/2); hava bacası çatıyı en çok 60 cm aşar (PAİY 40/5).",
            ));
        }
        out
    }

    fn configure(&mut self, key: &str, value: &str, cx: &mut Context<'_>) -> bool {
        if key == "T" {
            return self.choose_option(key, value, cx);
        }
        let Some(ask) = self.ask_of(key) else {
            return false;
        };
        if !self.set(ask, value, cx) {
            return false;
        }
        self.see(cx);
        true
    }

    fn choose_option(&mut self, key: &str, typed: &str, cx: &mut Context<'_>) -> bool {
        if key != "T" || self.is_window() {
            return false;
        }
        let Some(k) = [RoofPartKind::Chimney, RoofPartKind::Vent]
            .into_iter()
            .find(|k| kind_name(*k) == typed || k.label() == typed)
        else {
            return false;
        };
        self.set_kind(k);
        self.see(cx);
        true
    }

    fn takes_words(&self) -> bool {
        self.ask.is_some()
    }

    fn cancel(&mut self, _cx: &mut Context<'_>) -> bool {
        self.ask.take().is_some()
    }

    fn confirm(&mut self, _cx: &mut Context<'_>) -> Flow {
        // Enter alone leaves a step asking a value, as the other tools do (docs/adr/8044 §9); from the main step it
        // ends the tool.
        if self.ask.take().is_some() {
            return Flow::Stay;
        }
        Flow::Exit
    }

    fn preview(&self, format: &Format) -> Preview {
        let warning = self.warning(format);
        let tag = self.hover.map(|at| Tag {
            at,
            lines: match (&self.why, self.made) {
                (Some(why), _) => vec![self.label().to_owned(), why.clone()],
                (_, Some(made)) => {
                    let mut lines = vec![if self.is_downpipe() {
                        format!("İniş borusu · Ø{}", (self.width * 1000.0).round())
                    } else {
                        format!(
                            "{} · {}",
                            self.kind.label(),
                            size(format, self.width, self.depth)
                        )
                    }];
                    match (made.top, made.slope) {
                        (Some(top), _) => lines.push(format!(
                            "Tepesi {} · {} {} üstünde",
                            roof::level_words(top),
                            self.over_words(),
                            format.length(self.rise_now())
                        )),
                        (None, Some(slope)) => {
                            lines.push(format!("Yüzün eğimi {}", roof::slope_words(slope)))
                        }
                        (None, None) => {}
                    }
                    if made.top.is_some() {
                        lines.push(format!("Mahya {}", roof::level_words(made.ridge)));
                    }
                    lines.extend(warning.clone());
                    lines
                }
                _ => vec![self.label().to_owned()],
            },
        });
        Preview {
            tag,
            tag_tone: if self.why.is_some() || warning.is_some() {
                Tone::Danger
            } else {
                Tone::Accent
            },
            ..self.drawn.clone()
        }
    }
}
