//! Eğimle kot ver's Hedef Yüzeyden (docs/adr/4180; grade_window.rs): the selected line, polyline or area takes the
//! project's terrain surface's heights by `cad.entities.drape`. Every vertex on the surface takes its height; one off
//! it keeps its own. Ara kırık noktaları puts a vertex wherever a straight edge crosses an edge of the surface's
//! triangles. Docs/adr/4580: Kot farkı puts the vertices that far above (or below) the surface; Aralık gives heights
//! only to the vertices from Başlangıç to Bitiş, picked on the drawing as Eğimle kot ver's range. Docs/adr/4980: on a
//! multi-part object or one with holes Halka chooses the ring the range is on. The preview is the command's plan;
//! Uygula writes one undo step.

use iced::Element;
use kentos_commands::{CommandResult, DrapeRange, EntitiesDrape, EntitiesDraped, Entity, Vec2};
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, entities_drape};
use kentos_ui::widget::switch::Switch;
use kentos_ui::widget::{Form, NumberInput};

use crate::app::{App, Message};
use crate::exchange::words::{self, Kind};
use crate::grade_window::{Event, State, TITLE, msg};
use crate::style::fields;

/// One ring of an object: its part and ring (docs/adr/4980), its vertices, its bulges.
pub(crate) type Ring = ((u32, u32), Vec<Vec2>, Option<Vec<f64>>);

/// An object's rings in the command's order: each part (the first is the object's own vertices), its outer ring, then
/// its holes. A line is one ring of its two ends.
pub(crate) fn rings(e: &Entity) -> Vec<Ring> {
    match e {
        Entity::Line(l) => vec![((0, 0), vec![l.a, l.b], None)],
        Entity::Polyline(p) | Entity::Polygon(p) => {
            let mut out = vec![((0, 0), p.pts.clone(), p.bulges.clone())];
            for (h, r) in p.holes.iter().flatten().enumerate() {
                out.push(((0, h as u32 + 1), r.pts.clone(), r.bulges.clone()));
            }
            for (k, q) in p.parts.iter().flatten().enumerate() {
                out.push(((k as u32 + 1, 0), q.pts.clone(), q.bulges.clone()));
                for (h, r) in q.holes.iter().flatten().enumerate() {
                    out.push(((k as u32 + 1, h as u32 + 1), r.pts.clone(), r.bulges.clone()));
                }
            }
            out
        }
        _ => Vec::new(),
    }
}

/// A ring's name in the window, numbered from 1: “2. parça, 1. delik”.
fn ring_name((part, ring): (u32, u32)) -> String {
    match ring {
        0 => format!("{}. parça, dış halka", part + 1),
        h => format!("{}. parça, {h}. delik", part + 1),
    }
}

impl App {
    /// The project's terrain surfaces: id and name.
    pub(crate) fn grade_surfaces(&self) -> Vec<(String, String)> {
        self.document
            .as_ref()
            .and_then(|d| d.model.settings().road.as_ref())
            .map(|r| r.surfaces.iter().map(|s| (s.id.clone(), s.name.clone())).collect())
            .unwrap_or_default()
    }

    fn drape_input(&self) -> EntitiesDrape {
        let s = &self.grade;
        EntitiesDrape {
            uids: vec![s.uid.clone()],
            surface: s.surface.clone(),
            breaks: s.breaks,
            offset: (s.above != 0.0).then_some(s.above),
            range: s.partial.then(|| DrapeRange {
                from: (s.from.max(1.0) as u32) - 1,
                to: (s.to.max(1.0) as u32) - 1,
                part: s.ring.0,
                ring: s.ring.1,
            }),
            expected_revision: None,
        }
    }

    /// The command's plan for what the window holds; none without a surface.
    pub(crate) fn drape_plan(&mut self) {
        if self.grade.surface.is_empty() {
            self.grade.draped = None;
            return;
        }
        let input = self.drape_input();
        let Some(doc) = self.document.as_mut() else { return };
        self.grade.draped = Some(match entities_drape::plan(&ExecutionContext::new(&mut doc.model), &input) {
            CommandResult::Completed { output, .. } => Ok(output),
            CommandResult::Failed { error } | CommandResult::Conflict { error } | CommandResult::NeedsInput { error } => {
                Err(error.message)
            }
            _ => Err("Önizleme alınamadı.".to_owned()),
        });
    }

    /// Uygula: one undo step; the message says what was done.
    pub(crate) fn drape_apply(&mut self) {
        let input = self.drape_input();
        let Some(doc) = self.document.as_mut() else { return };
        match entities_drape::execute(&mut ExecutionContext::new(&mut doc.model), input) {
            CommandResult::Completed { output, .. } => {
                self.say(Level::Success, format!("Yüzeyden kot verildi: {} (tek adımda geri alınır).", drape_words(&output)));
                self.grade = State::default();
                self.dialog = None;
            }
            CommandResult::Failed { error } | CommandResult::Conflict { error } | CommandResult::NeedsInput { error } => {
                self.warn(format!("{TITLE}: {}", error.message));
            }
            _ => {}
        }
    }

    /// Halka (docs/adr/4980): the ring the range is on, a row only when the object has more than one.
    pub(crate) fn ring_field<'a>(&'a self, form: Form<'a, Message>) -> Form<'a, Message> {
        let all = self.grade_object().map(rings).unwrap_or_default();
        if all.len() < 2 {
            return form;
        }
        let at = self.grade.ring;
        let choices = all.iter().map(|r| (format!("{}:{}", r.0.0, r.0.1), ring_name(r.0))).collect();
        form.field("Halka", fields::select_owned(choices, &format!("{}:{}", at.0, at.1), |id| msg(Event::Ring(id))))
    }

    /// The ring's name for the vertices' help line, “ (2. parça, 1. delik)”; nothing for an object of one ring.
    pub(crate) fn ring_place(&self) -> String {
        let many = self.grade_object().map(rings).is_some_and(|r| r.len() > 1);
        if many { format!(" ({})", ring_name(self.grade.ring)) } else { String::new() }
    }

    /// Yüzeyden's rows: the surface and the break points; the summary's lines; whether Uygula may go.
    pub(crate) fn surface_form<'a>(&'a self, form: Form<'a, Message>) -> (Form<'a, Message>, Vec<Element<'a, Message>>, bool) {
        let s = &self.grade;
        let surfaces = self.grade_surfaces();
        if surfaces.is_empty() {
            let line = words::text_line(
                Kind::Error,
                "Projede arazi yüzeyi yok: önce Arazi yüzeyleri penceresinden bir yüzey kurun.",
            );
            return (form, vec![line], false);
        }
        const BOX: f32 = 140.0;
        let mut form = form
            .field("Yüzey", fields::select_owned(surfaces, &s.surface, |id| msg(Event::Surface(id))))
            .field(
                "Kot farkı (m)",
                NumberInput::new(s.above, |v| msg(Event::Above(v))).step(0.05).decimals(3).point().width(BOX),
            )
            .help("Yüzeyin kotuna eklenir: artı yüzeyin üstü, eksi altı.")
            .field(
                "Ara kırık noktaları",
                Switch::new(s.breaks, |on| msg(Event::Breaks(on))).label("Üçgen kenarlarını kestiği yerlere köşe ekle"),
            )
            .help("Yay bölünmez, çizgi iki ucunu tutar; ölçülü (M) ya da genişlikli çoklu çizgi bölünmez.")
            .field(
                "Aralık",
                Switch::new(s.partial, |on| msg(Event::Partial(on))).label("Yalnız başlangıç ile bitiş köşesi arası"),
            );
        // Docs/adr/4580: the range's two vertices, as Eğimle kot ver's own (Sahneden seç takes them by two clicks).
        if s.partial {
            // Docs/adr/4980: the ring, when the object has more than one.
            let place = self.ring_place();
            form = self.ring_field(form);
            form = form
                .field(
                    "Başlangıç köşesi",
                    iced::widget::row![
                        NumberInput::new(s.from, |v| msg(Event::From(v))).step(1.0).decimals(0).width(BOX),
                        words::secondary(crate::grade_window::PICK, Some(msg(Event::Pick)))
                    ]
                    .spacing(8)
                    .align_y(iced::Center),
                )
                .field("Bitiş köşesi", NumberInput::new(s.to, |v| msg(Event::To(v))).step(1.0).decimals(0).width(BOX))
                .help(format!(
                    "Köşeler 1'den {}'e{place}; aradaki kenarlar ara kırık noktası alır, öbür köşeler kotunu korur.",
                    self.grade_vertices().len()
                ));
        }
        let f = self.format();
        let over = if s.above > 0.0 {
            format!(" artı {}", f.length(s.above))
        } else if s.above < 0.0 {
            format!(" eksi {}", f.length(-s.above))
        } else {
            String::new()
        };
        let about = words::text_line(
            Kind::Info,
            if s.partial {
                format!(
                    "Aralıktaki köşelerden yüzeyin üstündekiler yüzeyin kotunu{over} alır (eski kotu değişir); yüzeyin ve aralığın dışındakiler kendi kotunu korur. Kotlar yüzeye bağlı kalmaz."
                )
            } else {
                format!(
                    "Yüzeyin üstündeki her köşe yüzeyin kotunu{over} alır (eski kotu değişir); dışındaki kendi kotunu korur. Kotlar yüzeye bağlı kalmaz."
                )
            },
        );
        let line = match &s.draped {
            Some(Ok(d)) => words::text_line(Kind::Ok, format!("{}.", drape_words(d))),
            Some(Err(why)) => words::text_line(Kind::Error, why.clone()),
            None => words::text_line(Kind::Info, "Önizleme yok."),
        };
        (form, vec![about, line], matches!(s.draped, Some(Ok(_))))
    }
}

/// What a drape does, in words: the vertices that take the surface's height, those off it, the break points.
fn drape_words(d: &EntitiesDraped) -> String {
    let mut out = format!("{} köşe yüzeyden kot alır", d.set);
    if d.outside > 0 {
        out.push_str(&format!(", {} köşe yüzeyin dışında, kendi kotunu korur", d.outside));
    }
    if d.inserted > 0 {
        out.push_str(&format!("; {} ara kırık noktası eklenir", d.inserted));
    }
    if d.objects.iter().any(|o| o.unsplit) {
        out.push_str("; bu nesne ara kırık noktası almaz");
    }
    out
}
