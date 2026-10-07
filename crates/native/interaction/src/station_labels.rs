//! Km yaz (docs/adr/0189; Netcad's Obje Üzerinde Dizi with ??KM labels; the
//! web's `StationLabelTool`, `apps/web/src/tools/stationLabelTool.ts`): a
//! route's stations at the multiples of an interval of its km and its ends,
//! and at each a tick, the km as a text square to the route, a cross-section
//! and a point. Where and what is the core's (`ops::stationing`); the tool
//! picks the route, shows everything faint and writes it through
//! `cad.entities.create` (step “Km yaz”) on the active layer, in the current
//! colour.
//!
//! - A click on a route (a line, a polyline, an arc, a circle, an ellipse, a
//!   fit-point curve, an area's outer ring) picks it; a single selected route
//!   is taken at the start. Enter, Uygula or a quick right click writes and
//!   the tool waits for the next route. Esc and Ctrl+Z let the route go; Esc
//!   with none leaves.
//! - The options are the session's (`Memory::station_*`): Aralık (A, m of
//!   km), Başlangıç (B, a km), Ters (T, each route's own), Yazı (Y: sol,
//!   sağ, yok; the key turns to the next, the chip offers them), Yükseklik
//!   (H, paper mm), Stil (S, a CAD project's), İşaret (İ), Enkesit (E, the
//!   half width; 0 none), Nokta (N, the offset, the right positive; an empty
//!   Enter none), Uçlar (U).

use std::collections::BTreeMap;

use kentos_contracts::{CreateOperation, EntityGeometry};
use kentos_domain::Slot;
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::ops::stationing::{self as rules, Look, Rules, Side, Stationing};
use kentos_geometry_core::tools::point_calc::{km_text, km_value, route_of};
use kentos_geometry_core::tools::point_text::{js_trim, parse_number};

use crate::Vec2;
use crate::format::{Format, js_number};
use crate::log::Level;
use crate::modify::MAX_GHOSTS;
use crate::points::{self, wire};
use crate::prompt::{Prompt, upper_tr};
use crate::styles;
use crate::tool::{
    Context, Cursor, Flow, Marker, MarkerShape, Memory, OptionChoice, Pointer, Preview, Stroke,
    TextGhost, Tone, Tool,
};

/// The tool's id: its command is `tool.stationLabels`.
pub const ID: &str = "stationLabels";
pub const LABEL: &str = "Km yaz";
/// Aralık at the start, metres of km.
pub const FIRST_INTERVAL: f64 = 20.0;
/// Yükseklik at the start, paper mm.
pub const FIRST_HEIGHT_MM: f64 = 2.0;
/// The tick's half length, paper mm.
pub const TICK_MM: f64 = 2.0;
/// Why a click gave no route.
pub const NO_OBJECT_HERE: &str = "Tıklanan yerde nesne yok; çizginin, yayın, dairenin, elipsin, eğrinin ya da alanın üzerine tıklayın.";

/// The values Yazı offers, in its key's order: the side, its name, its icon.
pub const SIDES: [(Option<Side>, &str); 3] = [
    (Some(Side::Left), "sol"),
    (Some(Side::Right), "sağ"),
    (None, "yok"),
];

/// What a key asked for, waiting for its value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Asking {
    Interval,
    Start,
    Height,
    Section,
    Point,
    Style,
}

/// The route walked: the object's slot and its shape as it was picked.
#[derive(Clone, Debug)]
struct Route {
    slot: Slot,
    shape: Shape,
}

pub struct StationLabels {
    route: Option<Route>,
    reverse: bool,
    asking: Option<Asking>,
    /// What the route takes now, or why nothing; worked out again when the
    /// options, the project or the route changed.
    plan: Option<Result<Stationing, String>>,
    seen: Option<(Memory, Format, styles::Seen, bool, Option<Slot>)>,
    /// What the plan was written with: the texts' height (m), face and width.
    height: f64,
    face: kentos_contracts::TextFace,
    width_factor: f64,
}

impl Default for StationLabels {
    fn default() -> Self {
        Self::new()
    }
}

/// The side's name as the prompt says it.
fn side_name(side: Option<Side>) -> &'static str {
    SIDES
        .iter()
        .find(|(s, _)| *s == side)
        .map_or("sol", |(_, name)| name)
}

/// The paper height, mm: a CAD project's chosen style's when it fixes one,
/// else Yükseklik's.
fn height_mm(cx: &Context<'_>) -> f64 {
    let settings = cx.doc.settings();
    styles::shown(settings)
        .then(|| styles::text_style(cx.memory, settings).and_then(|s| s.height))
        .flatten()
        .unwrap_or(cx.memory.station_height_mm)
}

impl StationLabels {
    pub fn new() -> Self {
        Self {
            route: None,
            reverse: false,
            asking: None,
            plan: None,
            seen: None,
            height: 1.0,
            face: kentos_contracts::TextFace::default(),
            width_factor: 1.0,
        }
    }

    /// The plan for the route as the options are now; worked out again only
    /// when something it depends on changed.
    fn see(&mut self, cx: &mut Context<'_>) {
        let key = (
            *cx.memory,
            cx.format(),
            styles::Seen::text(cx),
            self.reverse,
            self.route.as_ref().map(|r| r.slot),
        );
        if self.seen.as_ref() == Some(&key) {
            return;
        }
        self.seen = Some(key);
        let Some(route) = &self.route else {
            self.plan = None;
            return;
        };
        let m = *cx.memory;
        let f = cx.format();
        let scale = cx.doc.settings().plot_scale / 1000.0;
        self.height = height_mm(cx) * scale;
        self.face = styles::text_face(cx);
        self.width_factor = styles::text_width_factor(cx).unwrap_or(1.0);
        let r = Rules {
            interval: m.station_interval,
            start: m.station_start,
            reverse: self.reverse,
            ends: m.station_ends,
            decimals: f.length_decimals,
        };
        let look = Look {
            text: m.station_text,
            height: self.height,
            tick: if m.station_tick { TICK_MM * scale } else { 0.0 },
            section: m.station_section,
            point: m.station_point,
        };
        let plan = rules::stationing(&route.shape, &r, &look);
        if let Err(why) = &plan {
            cx.say(Level::Warn, why.clone());
        }
        self.plan = Some(plan);
    }

    /// The route under the click: the most specific object that has one.
    fn pick(&mut self, at: Vec2, cx: &mut Context<'_>) {
        let hits = cx.spatial.hits(at, cx.pick_tolerance());
        if hits.is_empty() {
            cx.say(Level::Warn, NO_OBJECT_HERE);
            return;
        }
        let found = hits.into_iter().find_map(|slot| {
            let shape = kentos_native_application::geometry::shape(cx.doc.get(slot)?);
            route_of(&shape).map(|_| Route { slot, shape })
        });
        match found {
            Some(route) => {
                self.route = Some(route);
                self.reverse = false;
                self.seen = None;
                self.see(cx);
            }
            None => cx.say(Level::Warn, rules::NO_ROUTE),
        }
    }

    /// Writes the plan in one step and says what came of it; the tool waits for the next route.
    fn write(&mut self, cx: &mut Context<'_>) {
        self.see(cx);
        let Some(Ok(plan)) = &self.plan else {
            return;
        };
        let km = |text: &str| Some(BTreeMap::from([("Km".to_owned(), text.to_owned())]));
        let mut objects: Vec<(EntityGeometry, Option<BTreeMap<String, String>>)> = Vec::new();
        for t in &plan.ticks {
            objects.push((line(t.a, t.b), None));
        }
        for t in &plan.texts {
            objects.push((
                EntityGeometry::Text {
                    p: wire(t.p),
                    text: t.text.clone(),
                    height: self.height,
                    rotation: t.rotation,
                    align: t
                        .align
                        .and_then(|a| kentos_contracts::TextAlign::from_name(a.name())),
                    width_factor: (self.width_factor != 1.0).then_some(self.width_factor),
                    mask: false,
                    box_width: None,
                    line_spacing: None,
                    runs: Vec::new(),
                    face: self.face.clone(),
                    path: None,
                },
                None,
            ));
        }
        for s in &plan.sections {
            objects.push((line(s.a, s.b), km(&s.km)));
        }
        for p in &plan.points {
            objects.push((
                EntityGeometry::Point {
                    p: wire(p.p),
                    z: None,
                    parts: None,
                },
                km(&p.km),
            ));
        }
        let count = plan.stations.len();
        if objects.is_empty() {
            cx.say(
                Level::Warn,
                format!(
                    "{LABEL}: yazılacak bir şey yok; Yazı, İşaret, Enkesit ya da Nokta’yı açın."
                ),
            );
            return;
        }
        if points::write_objects_each(objects, Some(CreateOperation::Stations), cx).is_none() {
            return;
        }
        cx.say(
            Level::Success,
            format!("{LABEL}: {count} istasyon yazıldı."),
        );
        self.route = None;
        self.reverse = false;
        self.plan = None;
        self.seen = None;
    }

    /// A key's option: what became of it, `None` when it is none.
    fn option(&mut self, key: &str, cx: &mut Context<'_>) -> Option<()> {
        let m = &mut cx.memory;
        match key {
            "A" => self.asking = Some(Asking::Interval),
            "B" => self.asking = Some(Asking::Start),
            "T" => self.reverse = !self.reverse,
            "Y" => {
                let at = SIDES
                    .iter()
                    .position(|(s, _)| *s == m.station_text)
                    .unwrap_or(0);
                m.station_text = SIDES[(at + 1) % SIDES.len()].0;
            }
            "H" => self.asking = Some(Asking::Height),
            "S" if styles::shown(cx.doc.settings()) => self.asking = Some(Asking::Style),
            "İ" => m.station_tick = !m.station_tick,
            "E" => self.asking = Some(Asking::Section),
            "N" => self.asking = Some(Asking::Point),
            "U" => m.station_ends = !m.station_ends,
            _ => return None,
        }
        Some(())
    }

    /// A value typed for what is asked: a wrong one is said and asked again.
    fn answer(&mut self, text: &str, cx: &mut Context<'_>) {
        let Some(what) = self.asking else {
            return;
        };
        let t = js_trim(text);
        let f = cx.format();
        let refused = |cx: &mut Context<'_>, why: &str| {
            cx.say(Level::Warn, format!("{why}; “{t}” yazıldı."));
        };
        match what {
            Asking::Interval => match parse_number(t) {
                Some(n) if n > 0.0 && n.is_finite() => {
                    cx.memory.station_interval = n;
                    self.asking = None;
                }
                _ => refused(
                    cx,
                    "Aralık sıfırdan büyük bir uzunluk olmalı (km’nin metresi)",
                ),
            },
            Asking::Start => match km_value(t) {
                Some(v) => {
                    cx.memory.station_start = v;
                    self.asking = None;
                }
                None => refused(
                    cx,
                    "Başlangıç bir km olmalı: k+mmm.mmm ya da metre (ör. 0+000)",
                ),
            },
            Asking::Height => match parse_number(t) {
                Some(n) if n > 0.0 && n.is_finite() => {
                    cx.memory.station_height_mm = n;
                    self.asking = None;
                }
                _ => refused(cx, "Yükseklik sıfırdan büyük bir sayı olmalı (kâğıtta mm)"),
            },
            Asking::Section => match parse_number(t) {
                Some(n) if n >= 0.0 && n.is_finite() => {
                    cx.memory.station_section = f.to_metres(n);
                    self.asking = None;
                }
                _ => refused(
                    cx,
                    "Enkesitin yarı genişliği sıfır ya da daha büyük bir uzunluk olmalı (0: yok)",
                ),
            },
            Asking::Point => match parse_number(t) {
                Some(n) if n.is_finite() => {
                    cx.memory.station_point = Some(f.to_metres(n));
                    self.asking = None;
                }
                _ => refused(
                    cx,
                    "Noktanın sapması bir uzunluk olmalı (sağa artı; boş Enter: nokta yok)",
                ),
            },
            Asking::Style => {
                if styles::take_text(t, cx) {
                    self.asking = None;
                }
            }
        }
    }
}

fn line(a: Vec2, b: Vec2) -> EntityGeometry {
    EntityGeometry::Line {
        a: wire(a),
        b: wire(b),
        zs: None,
    }
}

impl Tool for StationLabels {
    fn id(&self) -> &'static str {
        ID
    }

    fn label(&self) -> &'static str {
        LABEL
    }

    fn prompt(&self) -> Prompt {
        if let Some(what) = self.asking {
            let step = match what {
                Asking::Interval => "km aralığını metre olarak yazın (ör. 20)",
                Asking::Start => "güzergâhın başındaki km’yi yazın (ör. 0+000)",
                Asking::Height => "kâğıt üzerindeki yazı yüksekliğini mm olarak yazın",
                Asking::Section => "enkesitin yarı genişliğini yazın (0: yok)",
                Asking::Point => "noktaların sapmasını yazın (sağa artı; boş Enter: nokta yok)",
                Asking::Style => "yazı stilini menüden seçin ya da adını yazın",
            };
            return Prompt::new(LABEL, step);
        }
        let Some((m, f, style, _, _)) = &self.seen else {
            return Prompt::new(LABEL, "güzergâha tıklayın (km’si ilk köşesinde başlar)");
        };
        let on = |b: bool| if b { "açık" } else { "kapalı" };
        let step = match &self.plan {
            Some(Ok(plan)) => format!(
                "{} istasyon; Enter ile yazın ya da başka bir güzergâha tıklayın",
                plan.stations.len()
            ),
            Some(Err(_)) => "bu seçeneklerle yazılamıyor; seçenekleri değiştirin ya da başka bir güzergâha tıklayın".to_owned(),
            None => "güzergâha tıklayın (km’si ilk köşesinde başlar)".to_owned(),
        };
        let section = if m.station_section > 0.0 {
            f.length(m.station_section)
        } else {
            "yok".to_owned()
        };
        let point = m
            .station_point
            .map_or_else(|| "yok".to_owned(), |o| f.length(o));
        let prompt = Prompt::new(LABEL, step)
            .option_with(
                "Aralık",
                "A",
                format!("{} m", js_number(m.station_interval)),
            )
            .option_with(
                "Başlangıç",
                "B",
                km_text(m.station_start, f.length_decimals),
            )
            .toggle("Ters", "T", self.reverse)
            .option_with("Yazı", "Y", side_name(m.station_text))
            .option_with(
                "Yükseklik",
                "H",
                format!("{} mm", js_number(m.station_height_mm)),
            )
            .option_if(style.shown, "Stil", "S", style.chosen.clone())
            .option_with("İşaret", "İ", on(m.station_tick))
            .option_with("Enkesit", "E", section)
            .option_with("Nokta", "N", point)
            .option_with("Uçlar", "U", on(m.station_ends));
        if self.route.is_some() {
            prompt.option("Uygula", "Enter")
        } else {
            prompt
        }
    }

    fn point_count(&self) -> usize {
        usize::from(self.route.is_some())
    }

    fn activate(&mut self, cx: &mut Context<'_>) -> Flow {
        // A single selected route is taken at once.
        let selected: Vec<Slot> = cx.selection.ids().to_vec();
        if let [slot] = selected[..]
            && let Some(e) = cx.doc.get(slot)
        {
            let shape = kentos_native_application::geometry::shape(e);
            if route_of(&shape).is_some() {
                self.route = Some(Route { slot, shape });
            }
        }
        self.see(cx);
        Flow::Stay
    }

    fn snaps(&self) -> bool {
        false
    }

    /// A style's name is words: Space types a space (docs/adr/0183 §4).
    fn takes_words(&self) -> bool {
        self.asking == Some(Asking::Style)
    }

    fn cursor(&self) -> Cursor {
        Cursor::Pick
    }

    fn pointer_move(&mut self, _p: &Pointer, cx: &mut Context<'_>) {
        self.see(cx);
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        if self.asking.is_some() {
            return;
        }
        self.pick(p.raw, cx);
    }

    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        if self.asking.is_some() {
            self.answer(text, cx);
        } else if self.option(&upper_tr(js_trim(text)), cx).is_none() {
            return false;
        }
        self.see(cx);
        true
    }

    fn option_choices(&self, key: &str) -> Vec<OptionChoice> {
        match (key, &self.seen) {
            ("Y", Some((m, ..))) => SIDES
                .iter()
                .map(|&(side, name)| OptionChoice {
                    label: name.to_owned(),
                    typed: name.to_owned(),
                    icon: None,
                    preview: None,
                    checked: side == m.station_text,
                    command: None,
                })
                .collect(),
            ("S", Some((_, _, style, ..))) if style.shown => {
                style.choices(styles::TEXT_STYLES_ENTRY, styles::TEXT_STYLES)
            }
            _ => Vec::new(),
        }
    }

    fn choose_option(&mut self, key: &str, typed: &str, cx: &mut Context<'_>) -> bool {
        let taken = match key {
            "Y" => match SIDES.iter().find(|(_, name)| *name == typed.trim()) {
                Some(&(side, _)) => {
                    cx.memory.station_text = side;
                    true
                }
                None => false,
            },
            "S" if styles::shown(cx.doc.settings()) => {
                if styles::take_text(typed, cx) {
                    self.asking = None;
                }
                true
            }
            _ => false,
        };
        self.see(cx);
        taken
    }

    /// Enter: a value being asked ends (Nokta's empty one is no point);
    /// with a route, the plan is written; with none, the tool leaves.
    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow {
        if let Some(what) = self.asking.take() {
            if what == Asking::Point {
                cx.memory.station_point = None;
            }
            self.see(cx);
            return Flow::Stay;
        }
        if self.route.is_none() {
            return Flow::Exit;
        }
        self.write(cx);
        self.see(cx);
        Flow::Stay
    }

    /// Esc: a value being asked, then the route, go first.
    fn cancel(&mut self, cx: &mut Context<'_>) -> bool {
        if self.asking.take().is_some() {
            return true;
        }
        if self.route.take().is_some() {
            self.reverse = false;
            self.seen = None;
            self.see(cx);
            return true;
        }
        false
    }

    /// Ctrl+Z: the route goes; with none the drawing is undone.
    fn undo_step(&mut self, cx: &mut Context<'_>) -> bool {
        if self.asking.take().is_some() {
            return true;
        }
        if self.route.take().is_some() {
            self.reverse = false;
            self.seen = None;
            self.see(cx);
            return true;
        }
        false
    }

    fn preview(&self, _format: &Format) -> Preview {
        let Some(Ok(plan)) = &self.plan else {
            return Preview::default();
        };
        let mut out = Preview::default();
        let faint = |a: Vec2, b: Vec2| Stroke::solid(vec![a, b], false).tone(Tone::Snap);
        for t in plan.ticks.iter().take(MAX_GHOSTS) {
            out.strokes.push(faint(t.a, t.b));
        }
        for s in plan.sections.iter().take(MAX_GHOSTS) {
            out.strokes
                .push(Stroke::dashed(vec![s.a, s.b], false, [4.0, 3.0]).tone(Tone::Snap));
        }
        for p in plan.points.iter().take(MAX_GHOSTS) {
            out.markers.push(Marker {
                at: p.p,
                shape: MarkerShape::Ring(3.0),
                tone: Tone::Snap,
            });
        }
        out.texts = plan
            .texts
            .iter()
            .take(MAX_GHOSTS)
            .map(|t| TextGhost {
                p: t.p,
                text: t.text.clone(),
                height: self.height,
                rotation: t.rotation,
                align: t.align,
                mask: false,
                face: self.face.clone(),
                width_factor: self.width_factor,
            })
            .collect();
        out
    }
}
