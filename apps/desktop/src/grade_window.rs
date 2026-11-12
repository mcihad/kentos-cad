//! Eğimle kot ver (docs/adr/1290; `tool.grade`): the selected line's, polyline's or area's vertices from one to another
//! take elevations along a constant grade, by `cad.entities.grade`. The window is the preview: the command's plan says
//! the range's horizontal length, the grade and the end's elevation; Uygula writes one undo step. Vertices are numbered
//! from 1, as Koordinat listesi numbers them; the grade is typed in percent. Sahneden seç takes the range by two clicks
//! on the drawing, each the path's vertex nearest it within the pick aperture (docs/adr/2180). While the window is open
//! the range is marked on the drawing: its start and end vertices ringed, the path between them drawn over in the
//! accent, arcs as points along them; while the end is being picked, the start alone (docs/adr/2980).
//!
//! docs/adr/3380: Hedef chooses the grade, the end's elevation or the high or low point (Yüksek/alçak nokta): the end's
//! elevation, the grade from the start and the grade from the end (each from its vertex into the range, up positive);
//! the preview says where the point falls (a new vertex between two, or an existing one) and rings it on the drawing.
//!
//! docs/adr/3780: Hedef Referanstan raises or lowers the object whole from a reference point, by `cad.entities.raise`
//! (grade_reference.rs); Hedef is the form's first row. docs/adr/4180: Hedef Yüzeyden gives the vertices the project's
//! terrain surface's heights, by `cad.entities.drape` (grade_surface.rs). docs/adr/4980: the range (Eğim, Bitiş kotu,
//! Yüksek/alçak nokta and Yüzeyden's Aralık) may lie on any ring of a multi-part object or one with holes (Halka, or the
//! ring Sahneden seç's first click is nearest); the window numbers that ring's vertices and marks it.

use iced::widget::Column;
use iced::{Element, Task};
use kentos_commands::{CommandResult, EntitiesGrade, EntitiesGraded, EntitiesRaised, Entity, GradePointKind, GradedPoint};
use kentos_interaction::Level;
use kentos_interaction::pick::PickPoint;
use kentos_native_application::{ExecutionContext, entities_grade};
use kentos_ui::widget::{Dialog as Frame, Form, NumberInput, Segmented};

use crate::app::{App, Dialog, Message};
use crate::exchange::words::{self, Kind};
use crate::grade_reference::By;
use crate::traces::Control;

pub const TITLE: &str = "Eğimle kot ver";
const APPLY: &str = "Uygula";
const CANCEL: &str = "Vazgeç";
pub(crate) const PICK: &str = "Sahneden seç";

/// What the range's end is given by (docs/adr/3380): the grade, the end's elevation or the high or low point; or the
/// object raised whole from a reference (docs/adr/3780).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Target {
    #[default]
    Grade,
    End,
    Peak,
    Reference,
    Surface,
}

impl std::fmt::Display for Target {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Target::Grade => "Eğim",
            Target::End => "Bitiş kotu",
            Target::Peak => "Yüksek/alçak nokta",
            Target::Reference => "Referanstan",
            Target::Surface => "Yüzeyden",
        })
    }
}

/// The point's name as the preview and the message say it.
fn kind_words(k: GradePointKind) -> &'static str {
    match k {
        GradePointKind::High => "Yüksek nokta",
        GradePointKind::Low => "Alçak nokta",
        GradePointKind::Break => "Eğimin değiştiği nokta",
    }
}

/// Where the point falls, the vertices numbered from 1: “2. ile 3. köşe arasına yeni köşe” or “2. köşeye”.
fn place_words(p: &GradedPoint) -> String {
    if p.inserted {
        format!("{}. ile {}. köşe arasına yeni köşe", p.index, p.index + 1)
    } else {
        format!("{}. köşeye (köşe eklenmez)", p.index + 1)
    }
}

/// Which end of the range a click on the drawing gives (docs/adr/2180); for Referanstan the reference, then the vertex
/// (docs/adr/3780).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pick {
    Start,
    End,
    Reference,
    Vertex,
}

#[derive(Clone, Debug, Default)]
pub struct State {
    pub uid: String,
    /// From and to, numbered from 1.
    pub from: f64,
    pub to: f64,
    pub start: f64,
    /// What the end is given by.
    pub target: Target,
    pub end: f64,
    /// Percent.
    pub percent: f64,
    /// The high or low point's grades from the start and from the end, percent.
    pub ahead: f64,
    pub back: f64,
    pub plan: Option<Result<EntitiesGraded, String>>,
    /// The end a click on the drawing is awaited for (Sahneden seç).
    pub picking: Option<Pick>,
    /// Referanstan (docs/adr/3780): the vertex, numbered from 1; the reference's place (east, north) and elevation,
    /// whether one is given; by the grade (`percent`) or the difference; the command's plan.
    pub at: f64,
    pub east: f64,
    pub north: f64,
    pub height: f64,
    pub referenced: bool,
    pub by: By,
    pub difference: f64,
    pub raised: Option<Result<EntitiesRaised, String>>,
    /// Yüzeyden (docs/adr/4180): the surface's id, whether break points go in, the command's plan; the height over the
    /// surface and whether only the range from Başlangıç to Bitiş takes heights (docs/adr/4580).
    pub surface: String,
    pub breaks: bool,
    pub above: f64,
    pub partial: bool,
    /// Docs/adr/4980: the range's part and ring (0, 0: the first part's outer ring).
    pub ring: (u32, u32),
    pub draped: Option<Result<kentos_commands::EntitiesDraped, String>>,
}

#[derive(Clone, Debug)]
pub enum Event {
    From(f64),
    To(f64),
    Start(f64),
    Target(Target),
    End(f64),
    Percent(f64),
    Ahead(f64),
    Back(f64),
    /// Referanstan's vertex, the reference's Y (east), X (north) and elevation, by what, the difference.
    At(f64),
    East(f64),
    North(f64),
    Height(f64),
    By(By),
    Difference(f64),
    /// Yüzeyden's surface and break points; the height over it and the range (docs/adr/4580).
    Surface(String),
    Breaks(bool),
    Above(f64),
    Partial(bool),
    /// Docs/adr/4980: the range's ring, as `part:ring`.
    Ring(String),
    /// Sahneden seç: the range by two clicks, its start then its end; for Referanstan the reference, then the vertex.
    Pick,
    Apply,
    Close,
}

pub(crate) fn msg(e: Event) -> Message {
    Message::Grade(e)
}

impl App {
    fn grade_input(&self) -> EntitiesGrade {
        let s = &self.grade;
        EntitiesGrade {
            uid: s.uid.clone(),
            from: (s.from.max(1.0) as u32) - 1,
            to: (s.to.max(1.0) as u32) - 1,
            part: s.ring.0,
            ring: s.ring.1,
            start: Some(s.start),
            end: (s.target != Target::Grade).then_some(s.end),
            grade: (s.target == Target::Grade).then_some(s.percent / 100.0),
            ahead: (s.target == Target::Peak).then_some(s.ahead / 100.0),
            back: (s.target == Target::Peak).then_some(s.back / 100.0),
            expected_revision: None,
        }
    }

    fn grade_plan(&mut self) {
        if self.grade.target == Target::Reference {
            self.raise_plan();
            return;
        }
        if self.grade.target == Target::Surface {
            self.drape_plan();
            return;
        }
        let input = self.grade_input();
        let Some(doc) = self.document.as_mut() else { return };
        self.grade.plan = Some(match entities_grade::plan(&ExecutionContext::new(&mut doc.model), &input) {
            CommandResult::Completed { output, .. } => Ok(output),
            CommandResult::Failed { error } | CommandResult::Conflict { error } | CommandResult::NeedsInput { error } => {
                Err(error.message)
            }
            _ => Err("Önizleme alınamadı.".to_owned()),
        });
    }

    /// Eğimle kot ver (`tool.grade`): the one selected path, from its first vertex to its last.
    pub(crate) fn open_grade(&mut self) -> Task<Message> {
        let Some(doc) = self.document.as_ref() else {
            self.output("Açık çizim yok.");
            return Task::none();
        };
        let picked = match self.selection.ids() {
            [one] => doc.model.get(*one).zip(doc.model.uid(*one)),
            _ => None,
        };
        let Some((e, uid)) = picked else {
            self.warn("Eğimle kot ver: önce bir çizgi, çoklu çizgi ya da alan seçin.");
            return Task::none();
        };
        let (n, z0) = match e {
            Entity::Line(l) => (2, l.za),
            Entity::Polyline(p) | Entity::Polygon(p) => (p.pts.len(), p.zs.as_ref().and_then(|z| z.first().copied().flatten())),
            _ => {
                self.warn("Eğimle kot ver: seçili nesne çizgi, çoklu çizgi ya da alan değil.");
                return Task::none();
            }
        };
        // A reference given before stays for the next object, with Referanstan.
        let kept = std::mem::take(&mut self.grade);
        let kept = kept.referenced.then_some(kept);
        self.grade = State {
            uid: uid.to_string(),
            from: 1.0,
            to: n as f64,
            start: z0.unwrap_or(0.0),
            target: if kept.is_some() { Target::Reference } else { Target::Grade },
            end: z0.unwrap_or(0.0),
            percent: kept.as_ref().map_or(2.0, |k| k.percent),
            ahead: 2.0,
            back: 2.0,
            plan: None,
            picking: None,
            at: 1.0,
            east: kept.as_ref().map_or(0.0, |k| k.east),
            north: kept.as_ref().map_or(0.0, |k| k.north),
            height: kept.as_ref().map_or(0.0, |k| k.height),
            referenced: kept.is_some(),
            by: kept.as_ref().map_or(By::Grade, |k| k.by),
            difference: kept.as_ref().map_or(0.0, |k| k.difference),
            raised: None,
            surface: self.grade_surfaces().first().map(|(id, _)| id.clone()).unwrap_or_default(),
            breaks: false,
            above: 0.0,
            partial: false,
            ring: (0, 0),
            draped: None,
        };
        self.grade_plan();
        self.dialog = Some(Dialog::Grade);
        Task::none()
    }

    pub(crate) fn grade_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::From(v) => self.grade.from = v.round(),
            Event::To(v) => self.grade.to = v.round(),
            Event::Start(v) => self.grade.start = v,
            Event::Target(t) => self.grade.target = t,
            Event::End(v) => self.grade.end = v,
            Event::Percent(v) => self.grade.percent = v,
            Event::Ahead(v) => self.grade.ahead = v,
            Event::Back(v) => self.grade.back = v,
            Event::At(v) => self.grade.at = v.round(),
            Event::East(v) => (self.grade.east, self.grade.referenced) = (v, true),
            Event::North(v) => (self.grade.north, self.grade.referenced) = (v, true),
            Event::Height(v) => (self.grade.height, self.grade.referenced) = (v, true),
            Event::By(b) => self.grade.by = b,
            Event::Difference(v) => self.grade.difference = v,
            Event::Surface(id) => self.grade.surface = id,
            Event::Breaks(on) => self.grade.breaks = on,
            Event::Above(v) => self.grade.above = v,
            Event::Partial(on) => self.grade.partial = on,
            Event::Ring(id) => {
                if let Some((part, ring)) = id.split_once(':').and_then(|(a, b)| Some((a.parse().ok()?, b.parse().ok()?))) {
                    self.grade.ring = (part, ring);
                    // The new ring's vertices from its first to its last; Referanstan's its first.
                    self.grade.from = 1.0;
                    self.grade.at = 1.0;
                    self.grade.to = self.grade_vertices().len().max(1) as f64;
                }
            }
            Event::Pick => {
                let first = if self.grade.target == Target::Reference { Pick::Reference } else { Pick::Start };
                self.start_grade_pick(first);
                return Task::none();
            }
            Event::Apply if self.grade.target == Target::Reference => {
                self.raise_apply();
                return Task::none();
            }
            Event::Apply if self.grade.target == Target::Surface => {
                self.drape_apply();
                return Task::none();
            }
            Event::Apply => {
                let input = self.grade_input();
                let Some(doc) = self.document.as_mut() else { return Task::none() };
                match entities_grade::execute(&mut ExecutionContext::new(&mut doc.model), input) {
                    CommandResult::Completed { output, .. } => {
                        let f = self.format();
                        let said = match &output.point {
                            Some(p) => format!(
                                "Eğimle kot verildi: {} {}, kot {}; {} köşe, {} → {} (tek adımda geri alınır).",
                                kind_words(p.kind),
                                place_words(p),
                                f.length(p.z),
                                output.zs.len(),
                                f.length(output.start),
                                f.length(output.end)
                            ),
                            None => format!(
                                "Eğimle kot verildi: {} köşe, %{:.3} eğim, {:.3} → {:.3} m (tek adımda geri alınır).",
                                output.zs.len(),
                                output.grade * 100.0,
                                output.start,
                                output.end
                            ),
                        };
                        self.say(Level::Success, said);
                        self.grade = State::default();
                        self.dialog = None;
                    }
                    CommandResult::Failed { error } | CommandResult::Conflict { error } | CommandResult::NeedsInput { error } => {
                        self.warn(format!("{TITLE}: {}", error.message));
                    }
                    _ => {}
                }
                return Task::none();
            }
            Event::Close => {
                self.grade = State::default();
                self.dialog = None;
                return Task::none();
            }
        }
        self.grade_plan();
        Task::none()
    }

    /// The path's vertices as the window numbers them (from 0 here): a line's two ends, a polyline's or an area's outer
    /// ring's points; Yüzeyden's those of the ring its range is on (docs/adr/4980).
    pub(crate) fn grade_vertices(&self) -> Vec<kentos_commands::Vec2> {
        self.grade_ring().0
    }

    /// The ring the window works on: its vertices and bulges (see `grade_vertices`).
    fn grade_ring(&self) -> (Vec<kentos_commands::Vec2>, Option<Vec<f64>>) {
        let Some(e) = self.grade_object() else { return (Vec::new(), None) };
        let at = self.grade.ring;
        crate::grade_surface::rings(e).into_iter().find(|r| r.0 == at).map_or((Vec::new(), None), |r| (r.1, r.2))
    }

    /// The object the window was opened on.
    pub(crate) fn grade_object(&self) -> Option<&Entity> {
        let doc = self.document.as_ref()?;
        kentos_domain::Uuid::parse_str(&self.grade.uid).ok().and_then(|u| doc.model.slot_of(u)).and_then(|s| doc.model.get(s))
    }

    /// The range on the drawing (docs/adr/2980): the path from its start vertex to its end, arcs as points along them,
    /// and the two vertices; while the end is being picked the start alone, while the start is, nothing. Nothing when
    /// Eğimle kot ver is not open or the range does not hold.
    pub(crate) fn grade_marks(&self) -> (Vec<kentos_commands::Vec2>, Vec<kentos_commands::Vec2>) {
        let open = self.dialog == Some(Dialog::Grade) || self.grade.picking.is_some();
        if open && self.grade.target == Target::Reference {
            return self.reference_marks();
        }
        // Yüzeyden takes the whole object unless its range is on (docs/adr/4580): then the range is marked too.
        if self.grade.target == Target::Surface && !self.grade.partial {
            return (Vec::new(), Vec::new());
        }
        if !open {
            return (Vec::new(), Vec::new());
        }
        let (pts, bulges) = self.grade_ring();
        let (from, to) = (self.grade.from.max(1.0) as usize - 1, self.grade.to.max(1.0) as usize - 1);
        match self.grade.picking {
            Some(Pick::Start | Pick::Reference | Pick::Vertex) => (Vec::new(), Vec::new()),
            Some(Pick::End) => (Vec::new(), pts.get(from).copied().into_iter().collect()),
            None if from < to && to < pts.len() => {
                let span = &pts[from..=to];
                let bent = bulges.as_deref().and_then(|b| b.get(from..to));
                let size = span.windows(2).map(|w| (w[1].x - w[0].x).hypot(w[1].y - w[0].y)).sum::<f64>();
                let to_core = |p: &kentos_commands::Vec2| kentos_geometry_core::Vec2::new(p.x, p.y);
                let core: Vec<_> = span.iter().map(to_core).collect();
                let path = kentos_geometry_core::tessellate::bulge_path(&core, bent, false, (size / 2000.0).max(1e-6))
                    .into_iter()
                    .map(|p| kentos_commands::Vec2 { x: p.x, y: p.y })
                    .collect();
                // The high or low point the preview found, ringed with the range's ends (docs/adr/3380).
                let point = match &self.grade.plan {
                    Some(Ok(EntitiesGraded { point: Some(p), .. })) if self.grade.target == Target::Peak => {
                        Some(kentos_commands::Vec2 { x: p.x, y: p.y })
                    }
                    _ => None,
                };
                (path, [pts[from], pts[to]].into_iter().chain(point).collect())
            }
            None => (Vec::new(), Vec::new()),
        }
    }

    /// Sahneden seç (docs/adr/2180): the window steps aside for a click on the drawing.
    pub(crate) fn start_grade_pick(&mut self, end: Pick) {
        self.grade.picking = Some(end);
        self.dialog = None;
        let asks = match end {
            Pick::Start => "Başlangıç köşesine tıklayın",
            Pick::End => "Bitiş köşesine tıklayın",
            Pick::Reference => "Referans noktasına ya da kotlu bir köşeye tıklayın",
            Pick::Vertex => "Kotu verilecek köşeye tıklayın",
        };
        self.say(Level::Command, format!("{TITLE} › {asks}"));
        self.field = None;
        self.snap = None;
        self.session.run(Box::new(PickPoint::new(TITLE, asks)));
        self.with_tool(|s, cx| s.activate(cx));
    }

    /// The point Sahneden seç asked for (input.rs); false when it asked none. The path's vertex nearest it within the
    /// pick aperture is the range's start, then its end; the window opens again with the range previewed. Esc goes back
    /// to the window; a click by no vertex is said.
    pub(crate) fn grade_picked(&mut self, p: Option<kentos_interaction::Vec2>) -> bool {
        let Some(end) = self.grade.picking.take() else {
            return false;
        };
        self.dialog = Some(Dialog::Grade);
        let Some(p) = p else {
            return true;
        };
        let reach = self.draft.pick_aperture / self.viewport.camera.scale;
        if end == Pick::Reference {
            self.reference_picked(p, reach);
            return true;
        }
        // The range starts, and Referanstan's vertex is, on whichever ring the click is nearest (docs/adr/4980); the
        // range's end on the same ring.
        if matches!(end, Pick::Start | Pick::Vertex) {
            let rings = self.grade_object().map(crate::grade_surface::rings).unwrap_or_default();
            let nearest = rings
                .iter()
                .flat_map(|r| r.1.iter().enumerate().map(move |(i, v)| (r.0, i, (v.x - p.x).hypot(v.y - p.y))))
                .filter(|(_, _, d)| *d <= reach)
                .min_by(|a, b| a.2.total_cmp(&b.2));
            if let Some((at, i, _)) = nearest {
                self.grade.ring = at;
                if end == Pick::Vertex {
                    self.grade.at = (i + 1) as f64;
                    self.grade_plan();
                } else {
                    self.grade.from = (i + 1) as f64;
                    self.start_grade_pick(Pick::End);
                }
                return true;
            }
        }
        let nearest = self
            .grade_vertices()
            .iter()
            .enumerate()
            .map(|(i, v)| (i, (v.x - p.x).hypot(v.y - p.y)))
            .filter(|(_, d)| *d <= reach)
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(i, _)| i);
        let Some(i) = nearest else {
            self.warn(format!("{TITLE}: tıklanan yer seçili nesnenin bir köşesine yakın değil; köşenin üstüne tıklayın."));
            return true;
        };
        match end {
            Pick::Start => {
                self.grade.from = (i + 1) as f64;
                self.start_grade_pick(Pick::End);
            }
            Pick::End => {
                self.grade.to = (i + 1) as f64;
                self.grade_plan();
            }
            Pick::Vertex | Pick::Reference => {
                self.grade.at = (i + 1) as f64;
                self.grade_plan();
            }
        }
        true
    }

    pub(crate) fn grade_view(&self) -> Element<'_, Message> {
        let s = &self.grade;
        // One label column, every box the same width (the owner: a form's fields line up).
        const BOX: f32 = 140.0;
        let end = || NumberInput::new(s.end, |v| msg(Event::End(v))).step(0.1).decimals(3).point().width(BOX);
        let percent = |v: f64, on: fn(f64) -> Event| NumberInput::new(v, move |x| msg(on(x))).step(0.1).decimals(3).point().width(BOX);
        // The chosen vertices where they are, as the drawing writes coordinates (Y, X).
        let pts = self.grade_vertices();
        let f = self.format();
        let at = |n: f64| {
            let i = (n.max(1.0) as usize) - 1;
            pts.get(i).map_or(String::new(), |v| format!(" (Y {}, X {})", f.coord(v.x), f.coord(v.y)))
        };
        // Hedef first: what is done decides the rows under it (docs/adr/3780).
        let targets = [Target::Grade, Target::End, Target::Peak, Target::Reference, Target::Surface];
        let form = Form::new().label_width(130.0).field("Hedef", Segmented::new(targets, s.target, |t| msg(Event::Target(t))));
        if s.target == Target::Reference {
            let (form, lines, ready) = self.reference_form(form, BOX);
            return frame(Column::new().spacing(12).push(form).push(words::summary(lines)), ready);
        }
        if s.target == Target::Surface {
            let (form, lines, ready) = self.surface_form(form);
            return frame(Column::new().spacing(12).push(form).push(words::summary(lines)), ready);
        }
        // Docs/adr/4980: the ring the range is on, when the object has more than one.
        let form = self.ring_field(form);
        let form = form
            .field(
                "Başlangıç köşesi",
                iced::widget::row![
                    NumberInput::new(s.from, |v| msg(Event::From(v))).step(1.0).decimals(0).width(BOX),
                    words::secondary(PICK, Some(msg(Event::Pick)))
                ]
                .spacing(8)
                .align_y(iced::Center),
            )
            .field("Bitiş köşesi", NumberInput::new(s.to, |v| msg(Event::To(v))).step(1.0).decimals(0).width(BOX))
            .help(format!(
                "Köşeler 1'den {}'e{}, Koordinat listesindeki gibi; Sahneden seç iki tıkla alır: başlangıç{}, bitiş{}.",
                pts.len(),
                self.ring_place(),
                at(s.from),
                at(s.to)
            ))
            .field("Başlangıç kotu (m)", NumberInput::new(s.start, |v| msg(Event::Start(v))).step(0.1).decimals(3).point().width(BOX));
        let form = match s.target {
            // Referanstan and Yüzeyden return their own forms above.
            Target::Grade | Target::Reference | Target::Surface => form.field("Eğim (%)", percent(s.percent, Event::Percent)),
            Target::End => form.field("Bitiş kotu (m)", end()),
            Target::Peak => form
                .field("Bitiş kotu (m)", end())
                .field("Başlangıçtan eğim (%)", percent(s.ahead, Event::Ahead))
                .field("Bitişten eğim (%)", percent(s.back, Event::Back))
                .help("Her eğim kendi köşesinden aralığın içine doğrudur, artı yukarı; ikisinin kesiştiği yere köşe eklenir."),
        };
        let line = match &s.plan {
            Some(Ok(EntitiesGraded { point: Some(at), zs, .. })) => words::text_line(
                Kind::Ok,
                format!(
                    "{}: {} (Y {}, X {}), başlangıçtan {}, kot {}; {} köşenin kotu değişecek.",
                    kind_words(at.kind),
                    place_words(at),
                    f.coord(at.x),
                    f.coord(at.y),
                    f.length(at.along),
                    f.length(at.z),
                    zs.len()
                ),
            ),
            Some(Ok(p)) => words::text_line(
                Kind::Ok,
                format!(
                    "Yatay uzunluk {:.3} m, eğim %{:.3}, kot {:.3} → {:.3} m; {} köşenin kotu değişecek.",
                    p.length,
                    p.grade * 100.0,
                    p.start,
                    p.end,
                    p.zs.len()
                ),
            ),
            Some(Err(why)) => words::text_line(Kind::Error, why.clone()),
            None => words::text_line(Kind::Info, "Önizleme yok."),
        };
        let about = words::text_line(
            Kind::Info,
            if s.target == Target::Peak {
                "Noktaya kadar köşeler başlangıçtan, sonrakiler bitişten eğimle kot alır (yaylı kenar yayı boyunca); aralığın dışındakiler kotlarını korur."
            } else {
                "Aradaki köşeler yol boyunca yatay uzaklıklarına göre (yaylı kenar yayı boyunca) kot alır; aralığın dışındakiler kotlarını korur."
            },
        );
        let body = Column::new().spacing(12).push(form).push(words::summary(vec![about, line]));
        frame(body, matches!(s.plan, Some(Ok(_))))
    }

    pub(crate) fn grade_control(&self, control: Control<'_>) -> Result<Option<Message>, String> {
        Ok(match control {
            Control::Press(APPLY) => Some(msg(Event::Apply)),
            Control::Press(CANCEL) => Some(msg(Event::Close)),
            Control::Press(PICK) => Some(msg(Event::Pick)),
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}

/// The window around its body: Vazgeç, and Uygula when the preview holds. Five targets side by side need the width.
fn frame(body: Column<'_, Message>, ready: bool) -> Element<'_, Message> {
    Frame::new(TITLE)
        .push(body)
        .aside(words::ghost(CANCEL, Some(msg(Event::Close))))
        .action(words::primary(APPLY, ready.then(|| msg(Event::Apply))))
        .width(680.0)
        .modal(msg(Event::Close))
}
