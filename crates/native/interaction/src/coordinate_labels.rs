//! Koordinat yaz and Köşelere koordinat yaz (docs/adr/0185; Netcad's
//! Koordinat Yaz; the web's `CoordinateLabelTool` and
//! `CoordinateVerticesTool`, `apps/web/src/tools/coordinateLabelTool.ts`).
//! A label is a leader from its place to an elbow and a bar, the template's
//! first line over the bar and the others under it; without the leader its
//! lines stand beside the place. Where and what is the core's
//! (`ops::coordinate_labels`); the tools give it the places and the options
//! and write what it gives back through `cad.entities.create` (step
//! “Koordinat yaz”) on the active layer, in the current colour.
//!
//! - Koordinat yaz: each click, typed point or `#ad` gets a label, shown at
//!   the cursor before; the elevation and the point's name of the visible
//!   objects at the place are its. Enter or Esc ends.
//! - Köşelere koordinat yaz: selection first (points, lines, polylines and
//!   areas), as the modify tools; every place of their vertices is shown with
//!   its label (the coordinate schedule's places and names); Enter, Uygula
//!   or a quick right click writes them in one step and the tool leaves. With
//!   Çizelge (Ç) the coordinate schedule of the same objects then hangs from
//!   the cursor (Tablo ekle's placement, [`ViewChange::PlaceTable`]).
//!
//! The options are the session's (`Memory::coordinate_*`): Stil (S, a CAD
//! project's), Kollu (K), Yön (O: the chip's menu; the key turns to the next
//! one), Şablon (Ş, in the text field; an empty Enter gives the type's back),
//! Basamak (B; an empty Enter gives the project's back), Yükseklik (Y, paper
//! mm).

use std::collections::HashSet;

use kentos_contracts::{CreateOperation, DrawingUnit, Entity, EntityGeometry, TextFace};
use kentos_domain::Slot;
use kentos_geometry_core::geom::affine::Affine;
use kentos_geometry_core::ops::coordinate_labels::{
    self as rules, Direction, LabelUnits, Labels, Options, Place,
};
use kentos_geometry_core::ops::table::ScheduleKind;
use kentos_geometry_core::tools::point_text::{js_trim, parse_number};
use kentos_native_application::geometry::drawing_font;

use crate::Vec2;
use crate::elevation::elevation_at;
use crate::format::{Axes, Format, js_number};
use crate::log::Level;
use crate::modify::{MAX_GHOSTS, Modify, Stages};
use crate::points::{self, Taken, wire, wire_all};
use crate::prompt::{Prompt, upper_tr};
use crate::styles;
use crate::table;
use crate::tool::{
    Context, Flow, Memory, Name, OptionChoice, Pointer, Preview, Stroke, Tag, TextField, TextGhost,
    Tool, ViewChange,
};

pub const ID: &str = "coordinateLabel";
pub const LABEL: &str = "Koordinat yaz";
pub const VERTICES_ID: &str = "coordinateVertices";
pub const VERTICES_LABEL: &str = "Köşelere koordinat yaz";
/// The coordinate schedule's placement (Çizelge, docs/adr/0185 §1).
pub const SCHEDULE_LABEL: &str = "Koordinat çizelgesi";
/// Yükseklik's first value, paper mm.
pub const FIRST_HEIGHT_MM: f64 = 2.0;
/// The most decimals Basamak takes.
pub const MOST_DECIMALS: u8 = 8;
/// What is said when the selection has no vertices to take.
const NOTHING: &str = "Köşelere koordinat yaz: seçimde nokta, çizgi, çoklu çizgi ya da alan yok.";

/// The template a project's type starts with: east over north, as Netcad writes them.
pub fn first_template(cad: bool) -> &'static str {
    if cad { "{X}|{Y}" } else { "{Y}|{X}" }
}

/// Yön's ways in the order its key turns them: each its name and icon.
pub const DIRECTIONS: [(Direction, &str, &str); 5] = [
    (Direction::Auto, "Otomatik", "labelAuto"),
    (Direction::NorthEast, "Sağ üst", "labelNorthEast"),
    (Direction::NorthWest, "Sol üst", "labelNorthWest"),
    (Direction::SouthWest, "Sol alt", "labelSouthWest"),
    (Direction::SouthEast, "Sağ alt", "labelSouthEast"),
];

fn direction_name(d: Direction) -> &'static str {
    DIRECTIONS
        .iter()
        .find(|w| w.0 == d)
        .map_or("Otomatik", |w| w.1)
}

/// The kinds whose places Köşelere koordinat yaz takes (docs/adr/0185 §1).
fn is_source(e: &Entity) -> bool {
    matches!(
        e,
        Entity::Point(_) | Entity::Line(_) | Entity::Polyline(_) | Entity::Polygon(_)
    )
}

/// What an option asks for while it is asked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Asking {
    Decimals,
    Height,
    Style,
    /// Şablon, in the text field.
    Template,
}

/// What the tools write with: the core's options and units, the texts'
/// height (m), face and width factor.
#[derive(Clone, Debug, PartialEq)]
struct Look {
    options: Options,
    units: LabelUnits,
    face: TextFace,
    width_factor: Option<f64>,
}

/// The template the session writes with: its own, else the type's.
fn template(m: &Memory, f: &Format) -> String {
    let own = m.coordinate_template.as_str();
    if own.is_empty() {
        first_template(f.axes == Axes::Cad).to_owned()
    } else {
        own.to_owned()
    }
}

/// The decimals the session writes with: Basamak's, else the project's.
fn decimals(m: &Memory, f: &Format) -> usize {
    m.coordinate_decimals.map_or(f.length_decimals, usize::from)
}

/// The paper height, mm: a CAD project's chosen style's when it fixes one,
/// else Yükseklik's.
fn height_mm(cx: &Context<'_>) -> f64 {
    let settings = cx.doc.settings();
    styles::shown(settings)
        .then(|| styles::text_style(cx.memory, settings).and_then(|s| s.height))
        .flatten()
        .unwrap_or(cx.memory.coordinate_height_mm)
}

fn look(cx: &Context<'_>) -> Look {
    let m: &Memory = cx.memory;
    let f = cx.format();
    let settings = cx.doc.settings();
    let face = styles::text_face(cx);
    let width_factor = styles::shown(settings)
        .then(|| styles::text_style(m, settings).and_then(|s| s.width_factor))
        .flatten()
        .filter(|w| *w != 1.0);
    let options = Options {
        template: template(m, &f),
        decimals: decimals(m, &f),
        height: height_mm(cx) / 1000.0 * settings.plot_scale,
        leader: m.coordinate_leader,
        direction: m.coordinate_direction,
        font: drawing_font(face.font.or(settings.drawing_font)),
        bold: face.bold,
        width_factor: width_factor.unwrap_or(1.0),
    };
    let units = LabelUnits {
        cad: f.axes == Axes::Cad,
        per_metre: match f.unit {
            DrawingUnit::M => 1.0,
            DrawingUnit::Cm => 100.0,
            DrawingUnit::Mm => 1000.0,
        },
    };
    Look {
        options,
        units,
        face,
        width_factor,
    }
}

/// The objects a label is written as: its leader (an open polyline of three
/// vertices) when it has one, then its lines, each a text.
fn geometries(labels: &Labels, look: &Look) -> Vec<EntityGeometry> {
    let mut out = Vec::new();
    for l in &labels.labels {
        if let Some(leader) = &l.leader {
            out.push(EntityGeometry::Polyline {
                pts: wire_all(leader),
                bulges: None,
                zs: None,
                parts: None,
            });
        }
        for t in &l.texts {
            out.push(EntityGeometry::Text {
                p: wire(t.p),
                text: t.text.clone(),
                height: look.options.height,
                rotation: 0.0,
                align: t
                    .align
                    .and_then(|a| kentos_contracts::TextAlign::from_name(a.name())),
                width_factor: look.width_factor,
                mask: false,
                box_width: None,
                line_spacing: None,
                runs: Vec::new(),
                face: look.face.clone(),
                path: None,
            });
        }
    }
    out
}

/// Labels drawn as they will be: their leaders dashed, their lines faint in
/// their face; the first `most` of them.
fn ghost(labels: &Labels, look: &Look, most: usize) -> Preview {
    let shown = labels.labels.iter().take(most);
    Preview {
        strokes: shown
            .clone()
            .filter_map(|l| l.leader.as_ref())
            .map(|leader| Stroke::dashed(leader.clone(), false, [4.0, 3.0]))
            .collect(),
        texts: shown
            .flat_map(|l| l.texts.iter())
            .map(|t| TextGhost {
                p: t.p,
                text: t.text.clone(),
                height: look.options.height,
                rotation: 0.0,
                align: t.align,
                mask: false,
                face: look.face.clone(),
                width_factor: look.options.width_factor,
            })
            .collect(),
        ..Preview::default()
    }
}

/// The options as a prompt shows them, after what the stage asks.
fn with_options(
    prompt: Prompt,
    m: &Memory,
    f: &Format,
    style: &styles::Seen,
    schedule: bool,
) -> Prompt {
    let on = |b: bool| if b { "açık" } else { "kapalı" };
    let decimals = match m.coordinate_decimals {
        Some(d) => d.to_string(),
        None => format!("{} (proje)", f.length_decimals),
    };
    let prompt = prompt
        .option_if(style.shown, "Stil", "S", style.chosen.clone())
        .option_with("Kollu", "K", on(m.coordinate_leader))
        .option_with("Yön", "O", direction_name(m.coordinate_direction))
        .option_with("Şablon", "Ş", template(m, f))
        .option_with("Basamak", "B", decimals)
        .option_with(
            "Yükseklik",
            "Y",
            format!("{} mm", js_number(m.coordinate_height_mm)),
        );
    if schedule {
        prompt.option_with("Çizelge", "Ç", on(m.coordinate_schedule))
    } else {
        prompt
    }
}

/// What a stage that asks says.
fn asking_prompt(label: &'static str, asking: Asking, style: &styles::Seen) -> Prompt {
    match asking {
        Asking::Decimals => Prompt::new(
            label,
            format!("ondalık basamak sayısını yazın (0–{MOST_DECIMALS}; boş Enter: projeninki)"),
        ),
        Asking::Height => Prompt::new(label, "kâğıt üzerindeki yazı yüksekliğini mm olarak yazın"),
        Asking::Style => Prompt::new(label, "yazı stilini menüden seçin ya da adını yazın")
            .option_with("Stil", "S", style.chosen.clone()),
        Asking::Template => Prompt::new(label, "şablonu yazın"),
    }
}

/// Şablon's field by the cursor (or the view's middle), the template in it.
fn ask_template(near: Option<Vec2>, cx: &Context<'_>) -> ViewChange {
    let b = cx.view.visible();
    let at = near.unwrap_or(Vec2::new(
        (b.min_x + b.max_x) / 2.0,
        (b.min_y + b.max_y) / 2.0,
    ));
    ViewChange::Text(TextField {
        at,
        // Readable at any zoom: 14 px on the screen.
        height: cx.view.world_length(14.0),
        rotation: 0.0,
        align: None,
        width_factor: 1.0,
        initial: Some(template(cx.memory, &cx.format())),
        placeholder: Some("Koordinat yazısının şablonu"),
        hint: Some(
            "Satırlar | ile ayrılır; {Y}, {X}, {Z} ve {ad} yerine değerler · boş Enter: türün şablonu · Esc: vazgeç",
        ),
        empty: true,
        face: TextFace::default(),
    })
}

/// The shared options' keys: what became of `key`, or `None` when it is
/// none of them. A key that asks for a value starts asking it.
fn option(
    key: &str,
    near: Option<Vec2>,
    schedule: bool,
    asking: &mut Option<Asking>,
    cx: &mut Context<'_>,
) -> Option<bool> {
    let m = &mut cx.memory;
    match key {
        "K" => m.coordinate_leader = !m.coordinate_leader,
        "O" => {
            let at = DIRECTIONS
                .iter()
                .position(|w| w.0 == m.coordinate_direction)
                .unwrap_or(0);
            m.coordinate_direction = DIRECTIONS[(at + 1) % DIRECTIONS.len()].0;
        }
        "Ç" if schedule => m.coordinate_schedule = !m.coordinate_schedule,
        "B" => *asking = Some(Asking::Decimals),
        "Y" => *asking = Some(Asking::Height),
        "S" if styles::shown(cx.doc.settings()) => *asking = Some(Asking::Style),
        "Ş" => {
            *asking = Some(Asking::Template);
            let field = ask_template(near, cx);
            cx.view_changes.push(field);
        }
        _ => return None,
    }
    Some(true)
}

/// A value typed for what is asked: whether it was taken (a wrong one is
/// said and asked again).
fn answer(asking: &mut Option<Asking>, text: &str, cx: &mut Context<'_>) -> bool {
    let Some(what) = *asking else {
        return false;
    };
    let t = js_trim(text);
    match what {
        Asking::Decimals => match t.parse::<u8>() {
            Ok(d) if d <= MOST_DECIMALS => {
                cx.memory.coordinate_decimals = Some(d);
                *asking = None;
            }
            _ => cx.say(
                Level::Warn,
                format!(
                    "Basamak 0 ile {MOST_DECIMALS} arasında bir tam sayı olmalı; “{t}” yazıldı."
                ),
            ),
        },
        Asking::Height => match parse_number(t) {
            Some(n) if n > 0.0 && n.is_finite() => {
                cx.memory.coordinate_height_mm = n;
                *asking = None;
            }
            _ => cx.say(
                Level::Warn,
                format!("Yükseklik sıfırdan büyük bir sayı olmalı (kâğıtta mm); “{t}” yazıldı."),
            ),
        },
        Asking::Style => {
            if styles::take_text(t, cx) {
                *asking = None;
            }
        }
        Asking::Template => return false,
    }
    true
}

/// Enter while a value is asked: Basamak's empty Enter gives the project's
/// back; every other one keeps the value.
fn confirm_asking(asking: &mut Option<Asking>, cx: &mut Context<'_>) {
    if *asking == Some(Asking::Decimals) {
        cx.memory.coordinate_decimals = None;
    }
    if *asking != Some(Asking::Template) {
        *asking = None;
    }
}

/// Şablon's field answered: the typed template (an empty one gives the
/// type's back), or none (Esc).
fn template_typed(asking: &mut Option<Asking>, text: Option<&str>, cx: &mut Context<'_>) {
    if *asking != Some(Asking::Template) {
        return;
    }
    *asking = None;
    let Some(text) = text else {
        return;
    };
    let t = js_trim(text);
    match Name::new(t) {
        Some(name) => cx.memory.coordinate_template = name,
        None => cx.say(
            Level::Warn,
            format!("Şablon en çok {} harf olabilir.", Name::MAX_CHARS),
        ),
    }
}

/// Yön's and Stil's menus.
fn choices(key: &str, m: &Memory, style: &styles::Seen) -> Vec<OptionChoice> {
    match key {
        "O" => DIRECTIONS
            .iter()
            .map(|&(d, name, icon)| OptionChoice {
                label: name.to_owned(),
                typed: name.to_lowercase(),
                icon: Some(icon),
                preview: None,
                checked: d == m.coordinate_direction,
                command: None,
            })
            .collect(),
        "S" if style.shown => style.choices(styles::TEXT_STYLES_ENTRY, styles::TEXT_STYLES),
        _ => Vec::new(),
    }
}

/// One of Yön's or Stil's values chosen from its menu.
fn choose(key: &str, typed: &str, asking: &mut Option<Asking>, cx: &mut Context<'_>) -> bool {
    match key {
        "O" => {
            let typed = typed.trim().to_lowercase();
            match DIRECTIONS.iter().find(|w| w.1.to_lowercase() == typed) {
                Some(w) => {
                    cx.memory.coordinate_direction = w.0;
                    true
                }
                None => false,
            }
        }
        "S" if styles::shown(cx.doc.settings()) => {
            if styles::take_text(typed, cx) {
                *asking = None;
            }
            true
        }
        _ => false,
    }
}

// ── Koordinat yaz ───────────────────────────────────────────────────────

/// The name a point object gives the place `at` (docs/adr/0185 §2): its
/// label, the k-th of its other points “label (k+1)”; none elsewhere.
fn name_at(e: &Entity, at: Vec2) -> Option<String> {
    let Entity::Point(p) = e else {
        return None;
    };
    let label = p.base.label.as_deref().map(|l| l.trim_matches(' '))?;
    if label.is_empty() {
        return None;
    }
    let near =
        |q: &kentos_contracts::Vec2| (q.x - at.x).abs() <= 1e-6 && (q.y - at.y).abs() <= 1e-6;
    if near(&p.p) {
        return Some(label.to_owned());
    }
    let k = p.parts.iter().flatten().position(|q| near(&q.p))?;
    Some(format!("{label} ({})", k + 2))
}

/// The place `p` with the elevation and the name the visible objects there
/// have (1 µm, in the document's order): the first elevation, the first
/// point's name; a click, a typed point and `#ad` alike, whatever the snap
/// stood on (docs/adr/0185 §2).
fn place_at(p: Vec2, cx: &Context<'_>) -> Place {
    let near = 1e-6;
    let (mut z, mut name) = (None, None);
    for s in cx.spatial.in_rect(
        Vec2::new(p.x - near, p.y - near),
        Vec2::new(p.x + near, p.y + near),
        true,
    ) {
        let Some(e) = cx.doc.get(s) else {
            continue;
        };
        if z.is_none() {
            z = elevation_at(e, p);
        }
        if name.is_none() {
            name = name_at(e, p);
        }
        if z.is_some() && name.is_some() {
            break;
        }
    }
    Place {
        p,
        z,
        name,
        centre: None,
    }
}

/// Koordinat yaz: a label at each point given.
#[derive(Default)]
pub struct CoordinateLabel {
    d: Taken,
    /// The place under the cursor, as of the last pointer move.
    place: Option<Place>,
    asking: Option<Asking>,
    /// What the prompt and the preview show, as of the last event.
    seen: Option<(Memory, Format, styles::Seen, Look)>,
}

impl CoordinateLabel {
    pub fn new() -> Self {
        Self::default()
    }

    fn see(&mut self, cx: &Context<'_>) {
        self.seen = Some((*cx.memory, cx.format(), styles::Seen::text(cx), look(cx)));
    }

    /// Writes the label of `place` in its own step; a place the template
    /// leaves no line is said.
    fn write(&mut self, place: Place, cx: &mut Context<'_>) {
        self.d.begin(place.p, cx);
        let look = look(cx);
        let labels = rules::labels(std::slice::from_ref(&place), &look.options, &look.units);
        if labels.labels.is_empty() {
            cx.say(
                Level::Warn,
                format!("{LABEL}: şablon bu noktaya satır bırakmadı (adı ya da kotu yok)."),
            );
            return;
        }
        if let Some(out) = points::write_objects(
            geometries(&labels, &look),
            Some(CreateOperation::Coordinates),
            cx,
        ) && let Some(&id) = out.ids.first()
        {
            self.d.note(id, cx);
        }
    }
}

impl Tool for CoordinateLabel {
    fn id(&self) -> &'static str {
        ID
    }

    fn label(&self) -> &'static str {
        LABEL
    }

    fn prompt(&self) -> Prompt {
        let Some((m, f, style, _)) = &self.seen else {
            return Prompt::new(LABEL, "yazılacak noktaya tıklayın ya da Y,X yazın");
        };
        match self.asking {
            Some(asking) => asking_prompt(LABEL, asking, style),
            None => with_options(
                Prompt::new(LABEL, "yazılacak noktaya tıklayın ya da Y,X yazın"),
                m,
                f,
                style,
                false,
            ),
        }
    }

    fn point_count(&self) -> usize {
        0
    }

    fn activate(&mut self, cx: &mut Context<'_>) -> Flow {
        self.see(cx);
        Flow::Stay
    }

    /// A point computed by the point calculator, as if clicked.
    fn accepts_points(&self) -> bool {
        true
    }

    fn accept_point(&mut self, p: Vec2, cx: &mut Context<'_>) -> bool {
        self.write(place_at(p, cx), cx);
        self.see(cx);
        true
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        let at = self.d.constrain(p, cx);
        self.d.hover = Some(at);
        self.place = Some(place_at(at, cx));
        self.see(cx);
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        if self.asking.is_some() {
            return;
        }
        let at = self.d.constrain(p, cx);
        self.write(place_at(at, cx), cx);
        self.see(cx);
    }

    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        let done = if self.asking == Some(Asking::Template) {
            false
        } else if let Some(done) = option(
            &upper_tr(js_trim(text)),
            self.d.hover,
            false,
            &mut self.asking,
            cx,
        ) {
            done
        } else if self.asking.is_some() {
            answer(&mut self.asking, text, cx)
        } else {
            match cx.typed_point(text, self.d.last(), self.d.hover) {
                Some(p) => {
                    self.write(place_at(p, cx), cx);
                    true
                }
                None => false,
            }
        };
        self.see(cx);
        done
    }

    fn option_choices(&self, key: &str) -> Vec<OptionChoice> {
        match &self.seen {
            Some((m, _, style, _)) => choices(key, m, style),
            None => Vec::new(),
        }
    }

    fn choose_option(&mut self, key: &str, typed: &str, cx: &mut Context<'_>) -> bool {
        let taken = choose(key, typed, &mut self.asking, cx);
        self.see(cx);
        taken
    }

    /// A style's name is words: Space types a space (docs/adr/0183 §4).
    fn takes_words(&self) -> bool {
        self.asking == Some(Asking::Style)
    }

    fn text_typed(&mut self, text: Option<&str>, cx: &mut Context<'_>) {
        template_typed(&mut self.asking, text, cx);
        self.see(cx);
    }

    /// Enter ends, or takes back what is asked.
    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow {
        if self.asking.is_some() {
            confirm_asking(&mut self.asking, cx);
            self.see(cx);
            return Flow::Stay;
        }
        Flow::Exit
    }

    /// Esc while a value is asked keeps the old one.
    fn cancel(&mut self, cx: &mut Context<'_>) -> bool {
        if self.asking.take().is_some() {
            self.see(cx);
            return true;
        }
        false
    }

    fn undo_step(&mut self, cx: &mut Context<'_>) -> bool {
        self.d.undo_step(false, cx)
    }

    fn snap_from(&self) -> Option<Vec2> {
        None
    }

    /// The label at the cursor, as it will be written.
    fn preview(&self, _format: &Format) -> Preview {
        let (Some(place), Some((.., look)), None) = (&self.place, &self.seen, self.asking) else {
            return Preview {
                tracking: self.d.tracking,
                ..Preview::default()
            };
        };
        let labels = rules::labels(std::slice::from_ref(place), &look.options, &look.units);
        Preview {
            tracking: self.d.tracking,
            ..ghost(&labels, look, 1)
        }
    }
}

// ── Köşelere koordinat yaz ──────────────────────────────────────────────

/// What would be written for the drawing and the options as they were.
struct Plan {
    generation: u64,
    look: Look,
    labels: Labels,
}

#[derive(Default)]
pub struct CoordinateVertices {
    /// The objects it reads, taken when the selection is confirmed, in the drawing's order.
    slots: Vec<Slot>,
    asking: Option<Asking>,
    /// The cursor as it is: where the tag goes and Şablon's field opens.
    hover: Option<Vec2>,
    plan: Option<Plan>,
    seen: Option<(Memory, Format, styles::Seen)>,
}

impl CoordinateVertices {
    pub fn tool() -> Modify<Self> {
        Modify::with(Self::default())
    }

    /// Writes the labels in one step and says what came of it; with
    /// Çizelge, the coordinate schedule then hangs from the cursor.
    fn write(&mut self, cx: &mut Context<'_>) -> Flow {
        self.see(cx);
        let Some(plan) = &self.plan else {
            return Flow::Exit;
        };
        let (count, skipped) = (plan.labels.labels.len(), plan.labels.skipped);
        if count == 0 {
            cx.say(
                Level::Warn,
                format!(
                    "{VERTICES_LABEL}: yazılacak yazı yok; şablon {skipped} yere satır bırakmadı."
                ),
            );
            return Flow::Exit;
        }
        let objects = geometries(&plan.labels, &plan.look);
        if points::write_objects(objects, Some(CreateOperation::Coordinates), cx).is_none() {
            return Flow::Stay;
        }
        let past = if skipped > 0 {
            format!("; şablon {skipped} yere satır bırakmadı")
        } else {
            String::new()
        };
        cx.say(
            Level::Success,
            format!("{VERTICES_LABEL}: {count} yere yazıldı{past}."),
        );
        if cx.memory.coordinate_schedule {
            self.schedule(cx);
        }
        Flow::Exit
    }

    /// The coordinate schedule of the same objects, its names the labels',
    /// placed by Tablo ekle's tool: a heading row, every line, the labels'
    /// height and face.
    fn schedule(&self, cx: &mut Context<'_>) {
        let f = cx.format();
        let cells = table::schedule(cx.doc, ScheduleKind::Coordinates, &self.slots, &f);
        if let Some(problem) = &cells.problem {
            cx.say(Level::Warn, problem.clone());
            return;
        }
        let Some(plan) = &self.plan else {
            return;
        };
        let look = table::Look {
            header: true,
            height: plan.look.options.height,
            face: plan.look.face.clone(),
            grid: None,
            frame: None,
        };
        let source = table::source_of(cx.doc, ScheduleKind::Coordinates, &self.slots);
        let geometry = table::new_table(cx.doc, &cells, &look, Vec2::new(0.0, 0.0), Some(source));
        cx.view_changes
            .push(ViewChange::PlaceTable(geometry, SCHEDULE_LABEL));
    }
}

impl Stages for CoordinateVertices {
    fn id(&self) -> &'static str {
        VERTICES_ID
    }

    fn label(&self) -> &'static str {
        VERTICES_LABEL
    }

    /// The options and the labels for the drawing as it is: worked out again
    /// when the drawing or the options changed.
    fn see(&mut self, cx: &Context<'_>) {
        self.seen = Some((*cx.memory, cx.format(), styles::Seen::text(cx)));
        if self.slots.is_empty() {
            return;
        }
        let generation = cx.doc.generation();
        let look = look(cx);
        if self
            .plan
            .as_ref()
            .is_some_and(|p| p.generation == generation && p.look == look)
        {
            return;
        }
        let objects: Vec<_> = self
            .slots
            .iter()
            .filter_map(|s| cx.doc.get(*s))
            .map(table::listed)
            .collect();
        let places = rules::places(&objects);
        let labels = rules::labels(&places, &look.options, &look.units);
        self.plan = Some(Plan {
            generation,
            look,
            labels,
        });
    }

    fn begin(&mut self, cx: &mut Context<'_>) -> Flow {
        self.asking = None;
        self.plan = None;
        let chosen: HashSet<Slot> = cx
            .selection
            .ids()
            .iter()
            .copied()
            .filter(|&s| cx.doc.get(s).is_some_and(is_source))
            .collect();
        let chosen: Vec<Slot> = chosen.into_iter().collect();
        self.slots = table::in_order(cx.doc, &chosen);
        if self.slots.is_empty() {
            cx.say(Level::Warn, NOTHING);
            return Flow::Exit;
        }
        Flow::Stay
    }

    /// Nothing is placed by the cursor.
    fn snaps(&self) -> bool {
        false
    }

    fn anchor(&self) -> Option<Vec2> {
        None
    }

    fn prompt(&self, _n: usize) -> Prompt {
        let Some((m, f, style)) = &self.seen else {
            return Prompt::new(VERTICES_LABEL, "");
        };
        if let Some(asking) = self.asking {
            return asking_prompt(VERTICES_LABEL, asking, style);
        }
        let step = self
            .plan
            .as_ref()
            .map_or_else(|| "yazılacak yer yok".to_owned(), |p| finding(&p.labels));
        with_options(Prompt::new(VERTICES_LABEL, step), m, f, style, true).option("Uygula", "Enter")
    }

    /// The cursor as it is; a click places nothing.
    fn pointer(&mut self, p: &Pointer, down: bool, _cx: &mut Context<'_>) -> Option<Flow> {
        if down {
            return Some(Flow::Stay);
        }
        self.hover = Some(p.raw);
        None
    }

    fn point(&mut self, _p: Vec2, _cx: &mut Context<'_>) -> Flow {
        Flow::Stay
    }

    fn typed(&mut self, text: &str, cx: &mut Context<'_>) -> Option<Flow> {
        if self.asking == Some(Asking::Template) {
            return None;
        }
        let taken = match option(
            &upper_tr(js_trim(text)),
            self.hover,
            true,
            &mut self.asking,
            cx,
        ) {
            Some(done) => done,
            None => answer(&mut self.asking, text, cx),
        };
        self.see(cx);
        taken.then_some(Flow::Stay)
    }

    /// Nothing typed is a point.
    fn typed_points(&self) -> bool {
        false
    }

    fn takes_points(&self) -> bool {
        false
    }

    fn option_choices(&self, key: &str) -> Vec<OptionChoice> {
        match &self.seen {
            Some((m, _, style)) => choices(key, m, style),
            None => Vec::new(),
        }
    }

    fn choose_option(&mut self, key: &str, typed: &str, cx: &mut Context<'_>) -> bool {
        let taken = choose(key, typed, &mut self.asking, cx);
        self.see(cx);
        taken
    }

    fn takes_words(&self) -> bool {
        self.asking == Some(Asking::Style)
    }

    fn text_typed(&mut self, text: Option<&str>, cx: &mut Context<'_>) {
        template_typed(&mut self.asking, text, cx);
        self.see(cx);
    }

    /// Enter: the labels written in one step and the tool leaves; while a
    /// value is asked, it takes it back.
    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow {
        if self.asking.is_some() {
            confirm_asking(&mut self.asking, cx);
            self.see(cx);
            return Flow::Stay;
        }
        self.write(cx)
    }

    /// Esc while a value is asked keeps the old one; otherwise the tool leaves.
    fn back(&mut self, cx: &mut Context<'_>) -> bool {
        if self.asking.take().is_some() {
            self.see(cx);
            return true;
        }
        false
    }

    fn preview(&self, _hover: Vec2) -> Option<Affine> {
        None
    }

    /// The labels to come (as the modify tools' ghosts, a very large
    /// selection shows its first ones), the finding beside the cursor.
    fn stage_preview(&self, hover: Option<Vec2>, _format: &Format) -> Option<Preview> {
        let plan = self.plan.as_ref()?;
        let tag = hover.map(|at| Tag {
            at,
            lines: tag_lines(&plan.labels),
        });
        Some(Preview {
            tag,
            ..ghost(&plan.labels, &plan.look, MAX_GHOSTS)
        })
    }
}

/// `12 yer; şablon 2 yere satır bırakmadı`: what would be written, then what is passed over.
fn finding(labels: &Labels) -> String {
    match (labels.labels.len(), labels.skipped) {
        (0, 0) => "yazılacak yer yok".to_owned(),
        (0, s) => format!("yazılacak yazı yok; şablon {s} yere satır bırakmadı"),
        (n, 0) => format!("{n} yere yazılacak"),
        (n, s) => format!("{n} yere yazılacak; şablon {s} yere satır bırakmadı"),
    }
}

/// The finding beside the cursor, a part a line, and what writes it.
fn tag_lines(labels: &Labels) -> Vec<String> {
    let mut lines: Vec<String> = finding(labels).split("; ").map(str::to_owned).collect();
    if !labels.labels.is_empty() {
        lines.push("Enter: uygula".to_owned());
    }
    lines
}
