//! Eğimle kot ver's Hedef Yüzeyden (docs/adr/4180; grade_window.rs): the selected line, polyline or area takes the
//! project's terrain surface's heights by `cad.entities.drape`, as Civil 3D's Elevations From Surface. Every vertex on
//! the surface takes its height; one off it keeps its own. Ara kırık noktaları puts a vertex wherever a straight edge
//! crosses an edge of the surface's triangles (Insert Intermediate Grade Break Points). The preview is the command's
//! plan; Uygula writes one undo step.

use iced::Element;
use kentos_commands::{CommandResult, EntitiesDrape, EntitiesDraped};
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, entities_drape};
use kentos_ui::widget::Form;
use kentos_ui::widget::switch::Switch;

use crate::app::{App, Message};
use crate::exchange::words::{self, Kind};
use crate::grade_window::{Event, State, TITLE, msg};
use crate::style::fields;

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
        EntitiesDrape { uids: vec![s.uid.clone()], surface: s.surface.clone(), breaks: s.breaks, expected_revision: None }
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
        let form = form
            .field("Yüzey", fields::select_owned(surfaces, &s.surface, |id| msg(Event::Surface(id))))
            .field(
                "Ara kırık noktaları",
                Switch::new(s.breaks, |on| msg(Event::Breaks(on))).label("Üçgen kenarlarını kestiği yerlere köşe ekle"),
            )
            .help("Yay bölünmez, çizgi iki ucunu tutar; ölçülü (M) ya da genişlikli çoklu çizgi bölünmez.");
        let about = words::text_line(
            Kind::Info,
            "Yüzeyin üstündeki her köşe yüzeyin kotunu alır (eski kotu değişir); dışındaki kendi kotunu korur. Kotlar yüzeye bağlı kalmaz.",
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
