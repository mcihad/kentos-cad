//! Eğimle kot ver's Hedef Referanstan (docs/adr/3780; grade_window.rs): the selected line, polyline or area raised or
//! lowered whole from a reference point by `cad.entities.raise`. The chosen vertex (on the ring Halka names, docs/adr/4980)
//! takes the reference's elevation plus the grade times its horizontal distance from it (or plus the
//! difference); every other elevation moves by as much, so the object's own grades stay.
//!
//! - The reference is a place and its elevation: typed (Y, X, kot), or by Sahneden seç's first click, the nearest point
//!   or vertex with an elevation within the pick aperture; a click by none takes the place and leaves the elevation to
//!   type. The second click takes the vertex, as the range's clicks do.
//! - After Uygula the reference stays for the next object; Vazgeç lets it go.
//! - On the drawing the reference and the vertex are ringed, the line between them drawn over.

use std::fmt;

use iced::Element;
use iced::widget::row;
use kentos_commands::{CommandResult, EntitiesRaise, Entity, RaiseReference};
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, entities_raise};
use kentos_ui::widget::{Form, NumberInput, Segmented};

use crate::app::{App, Message};
use crate::exchange::words::{self, Kind};
use crate::grade_window::{Event, PICK, Pick, State, TITLE, Target, msg};

/// What the vertex's elevation is given by: the grade from the reference or the difference to it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum By {
    #[default]
    Grade,
    Difference,
}

impl fmt::Display for By {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            By::Grade => "Eğim",
            By::Difference => "Kot farkı",
        })
    }
}

/// A difference with its sign, the minus a real one (U+2212), as Kot ver says it; the length writes its unit.
fn signed(by: f64, f: &kentos_interaction::Format) -> String {
    if by < 0.0 { format!("−{}", f.length(-by)) } else { format!("+{}", f.length(by)) }
}

impl App {
    fn raise_input(&self) -> EntitiesRaise {
        let s = &self.road.grade;
        EntitiesRaise {
            uid: s.uid.clone(),
            at: (s.at.max(1.0) as u32) - 1,
            part: s.ring.0,
            ring: s.ring.1,
            reference: RaiseReference { x: s.east, y: s.north, z: s.height },
            grade: (s.by == By::Grade).then_some(s.percent / 100.0),
            difference: (s.by == By::Difference).then_some(s.difference),
            expected_revision: None,
        }
    }

    /// The command's plan for what the window holds; none until a reference is given.
    pub(crate) fn raise_plan(&mut self) {
        if !self.road.grade.referenced {
            self.road.grade.raised = None;
            return;
        }
        let input = self.raise_input();
        let Some(doc) = self.document.as_mut() else {
            return;
        };
        self.road.grade.raised = Some(
            match entities_raise::plan(&ExecutionContext::new(&mut doc.model), &input) {
                CommandResult::Completed { output, .. } => Ok(output),
                CommandResult::Failed { error }
                | CommandResult::Conflict { error }
                | CommandResult::NeedsInput { error } => Err(error.message),
                _ => Err("Önizleme alınamadı.".to_owned()),
            },
        );
    }

    /// Uygula: one undo step; the reference kept for the next object.
    pub(crate) fn raise_apply(&mut self) {
        let input = self.raise_input();
        let at = input.at + 1;
        let Some(doc) = self.document.as_mut() else { return };
        match entities_raise::execute(&mut ExecutionContext::new(&mut doc.model), input) {
            CommandResult::Completed { output, .. } => {
                let f = self.format();
                let said = format!(
                    "Referanstan kot verildi: {at}. köşe {}; nesnenin {} kotu {} kaydı (tek adımda geri alınır).",
                    f.length(output.after),
                    output.moved,
                    signed(output.by, &f)
                );
                self.say(Level::Success, said);
                let was = std::mem::take(&mut self.road.grade);
                self.road.grade = State {
                    target: Target::Reference,
                    referenced: true,
                    east: was.east,
                    north: was.north,
                    height: was.height,
                    by: was.by,
                    percent: was.percent,
                    difference: was.difference,
                    ..State::default()
                };
                self.dialog = None;
            }
            CommandResult::Failed { error } | CommandResult::Conflict { error } | CommandResult::NeedsInput { error } => {
                self.warn(format!("{TITLE}: {}", error.message));
            }
            _ => {}
        }
    }

    /// The point or vertex with an elevation nearest `p` within `reach`, among the visible objects there.
    fn elevated_near(&self, p: kentos_interaction::Vec2, reach: f64) -> Option<(kentos_commands::Vec2, f64)> {
        let doc = self.document.as_ref()?;
        let mut best: Option<(f64, kentos_commands::Vec2, f64)> = None;
        for slot in self.spatial.hits(p, reach) {
            let Some(e) = doc.model.get(slot) else { continue };
            let mut seen = |x: f64, y: f64, z: Option<f64>| {
                let Some(z) = z else { return };
                let d = (x - p.x).hypot(y - p.y);
                if d <= reach && best.is_none_or(|b| d < b.0) {
                    best = Some((d, kentos_commands::Vec2 { x, y }, z));
                }
            };
            match e {
                Entity::Point(q) => {
                    seen(q.p.x, q.p.y, q.z);
                    for part in q.parts.iter().flatten() {
                        seen(part.p.x, part.p.y, part.z);
                    }
                }
                _ => {
                    for path in kentos_native_application::elevation::paths(e) {
                        for (v, z) in path.pts.iter().zip(&path.zs) {
                            seen(v.x, v.y, *z);
                        }
                    }
                }
            }
        }
        best.map(|(_, q, z)| (q, z))
    }

    /// Sahneden seç's first click: the reference from the point or vertex there with an elevation, else the place
    /// alone (its elevation to type); then the vertex is asked for.
    pub(crate) fn reference_picked(&mut self, p: kentos_interaction::Vec2, reach: f64) {
        match self.elevated_near(p, reach) {
            Some((q, z)) => {
                (
                    self.road.grade.east,
                    self.road.grade.north,
                    self.road.grade.height,
                ) = (q.x, q.y, z)
            }
            None => {
                (self.road.grade.east, self.road.grade.north) = (p.x, p.y);
                self.warn(format!("{TITLE}: tıklanan yerde kotlu nokta ya da köşe yok; yeri alındı, referansın kotunu yazın."));
            }
        }
        self.road.grade.referenced = true;
        self.start_grade_pick(Pick::Vertex);
    }

    /// The reference and the vertex on the drawing: both ringed, the line between them; while the reference is being
    /// picked, nothing; while the vertex is, the reference alone.
    pub(crate) fn reference_marks(&self) -> (Vec<kentos_commands::Vec2>, Vec<kentos_commands::Vec2>) {
        let s = &self.road.grade;
        if !s.referenced || s.picking == Some(Pick::Reference) {
            return (Vec::new(), Vec::new());
        }
        let r = kentos_commands::Vec2 { x: s.east, y: s.north };
        if s.picking == Some(Pick::Vertex) {
            return (Vec::new(), vec![r]);
        }
        match self.grade_vertices().get((s.at.max(1.0) as usize) - 1) {
            Some(v) => (vec![r, *v], vec![r, *v]),
            None => (Vec::new(), vec![r]),
        }
    }

    /// Referanstan's rows under Hedef, the preview's lines and whether Uygula can write.
    pub(crate) fn reference_form<'a>(
        &'a self,
        form: Form<'a, Message>,
        width: f32,
    ) -> (Form<'a, Message>, Vec<Element<'a, Message>>, bool) {
        let s = &self.road.grade;
        let f = self.format();
        let number = |v: f64, on: fn(f64) -> Event, step: f64| {
            NumberInput::new(v, move |x| msg(on(x))).step(step).decimals(3).point().width(width)
        };
        let vertex = self
            .grade_vertices()
            .get((s.at.max(1.0) as usize) - 1)
            .map_or(String::new(), |v| format!(" (Y {}, X {})", f.coord(v.x), f.coord(v.y)));
        let form = form
            .field(
                "Referans Y, X",
                row![number(s.east, Event::East, 1.0), number(s.north, Event::North, 1.0), words::secondary(PICK, Some(msg(Event::Pick)))]
                    .spacing(8)
                    .align_y(iced::Center),
            )
            .field("Referans kotu (m)", number(s.height, Event::Height, 0.1));
        // Docs/adr/4980: the vertex's ring, when the object has more than one.
        let place = self.ring_place();
        let form = self
            .ring_field(form)
            .field("Köşe", NumberInput::new(s.at, |v| msg(Event::At(v))).step(1.0).decimals(0).width(width))
            .help(format!(
                "{} {} köşesi var{place}, 1'den sayılır; seçili köşe{vertex}. Sahneden seç iki tıkla alır: önce referans (kotlu bir nokta ya da köşe), sonra köşe.",
                if place.is_empty() { "Nesnenin" } else { "Halkanın" },
                self.grade_vertices().len(),
            ))
            .field("Fark", Segmented::new([By::Grade, By::Difference], s.by, |b| msg(Event::By(b))));
        let form = match s.by {
            By::Grade => form.field("Eğim (%)", number(s.percent, Event::Percent, 0.1)),
            By::Difference => form.field("Kot farkı (m)", number(s.difference, Event::Difference, 0.05)),
        }
        .help("Eğim referanstan köşeye doğrudur, artı yukarı; kot farkı köşenin kotu eksi referansınkidir.");
        let about = words::text_line(
            Kind::Info,
            "Nesnenin bütün kotları aynı farkla kayar, kendi eğimleri değişmez; kotsuz köşeler kotsuz kalır.",
        );
        let at = s.at.max(1.0) as u32;
        let line = match &s.raised {
            _ if !s.referenced => words::text_line(Kind::Info, "Referansı Sahneden seç ile alın ya da Y, X ve kotunu yazın."),
            Some(Ok(r)) => words::text_line(
                Kind::Ok,
                format!(
                    "Referanstan {}; {at}. köşe {} → {}{}; {} kot {} kayacak{}.",
                    f.length(r.distance),
                    f.length(r.before),
                    f.length(r.after),
                    r.grade.map_or(String::new(), |g| format!(" (%{:.3})", g * 100.0)),
                    r.moved,
                    signed(r.by, &f),
                    if r.unset > 0 { format!(", {} kotsuz köşe kotsuz kalır", r.unset) } else { String::new() }
                ),
            ),
            Some(Err(why)) => words::text_line(Kind::Error, why.clone()),
            None => words::text_line(Kind::Info, "Önizleme yok."),
        };
        let ready = s.referenced && matches!(s.raised, Some(Ok(_)));
        (form, vec![about, line], ready)
    }
}
