//! The type of a project opened without one (docs/adr/0165 §1; the web's
//! `ui/settings/ProjectTypeDialog.ts`): written before types, or under the
//! former Hibrit mode. Asked once per open, from the open's own path (a file,
//! a recovery copy, a cloud project), with the CAD and CBS cards; the default
//! for new projects is chosen. Seç writes it to the project (an edit, not an
//! undo step, as any project setting); Sonra, Esc or × leave it unasked: the
//! project shows as CBS meanwhile and the next open asks again. Another
//! drawing on screen drops the question.

use iced::Element;
use iced::widget::column;
use kentos_contracts::Workspace;
use kentos_ui::label;
use kentos_ui::widget::{Dialog, overlay};

use super::{Event as ProjectEvent, Window, message, modes};
use crate::app::{App, Message};
use crate::catalog::mode_command;
use crate::exchange::words;

#[derive(Debug)]
pub struct State {
    /// The drawing asked about.
    session: u64,
    /// Its name, as the question says it.
    pub(super) name: String,
    choice: Workspace,
}

#[derive(Debug, Clone)]
pub enum Event {
    Choose(Workspace),
    Pick,
    Later,
}

fn event(e: Event) -> Message {
    message(ProjectEvent::Type(e))
}

impl App {
    /// After an open the user made: the project's type is asked when it has none.
    pub(crate) fn ask_project_type(&mut self) {
        let Some(doc) = &self.document else { return };
        if doc.settings().project_type().is_some() || self.dialog.is_some() {
            return;
        }
        let state = State {
            session: doc.session,
            name: doc.name().to_string(),
            choice: super::wizard::default_type(self),
        };
        self.open_project_window(Window::Type(Box::new(state)));
    }

    pub(super) fn project_type_event(&mut self, e: Event) {
        let Some(Window::Type(state)) = &mut self.project else {
            return;
        };
        match e {
            Event::Choose(w) => state.choice = w,
            Event::Pick => {
                let (session, choice) = (state.session, state.choice);
                self.close_project_window();
                // The question was about the drawing on screen when it was asked.
                if self.document.as_ref().is_some_and(|d| d.session == session) {
                    self.choose_mode(mode_command(choice));
                }
            }
            Event::Later => {
                let name = std::mem::take(&mut state.name);
                self.close_project_window();
                self.type_left_unasked(&name);
            }
        }
    }

    /// Sonra, Esc or ×: the project stays without a type, shown as CBS.
    pub(super) fn type_left_unasked(&mut self, name: &str) {
        self.output(format!(
            "“{name}” projesinin türü seçilmedi; CBS olarak gösteriliyor. Durum çubuğundaki türden ya da Proje ayarları’ndan seçilir."
        ));
    }

    pub(super) fn project_type_view<'a>(&'a self, s: &'a State) -> Element<'a, Message> {
        let body = column![
            label::body(format!(
                "“{}” projesinin türü henüz seçilmedi. Sahne, eksenler ve şerit türe göredir; çizimin verisi değişmez. Seçtiğiniz tür projeye yazılır; Proje ayarları’ndan değiştirilebilir.",
                s.name
            )),
            modes(s.choice, false, false, |w| event(Event::Choose(w))),
        ]
        .spacing(14);
        overlay::blocking(
            Dialog::new("Proje türü")
                .push(body)
                .action(words::secondary("Sonra", Some(event(Event::Later))))
                .action(words::primary("Seç", Some(event(Event::Pick))))
                .width(640.0),
        )
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use kentos_contracts::Workspace;

    use super::Event;
    use crate::app::{App, Dialog, Message};
    use crate::files_testing::{app_with_drawing, drive, last_said, saved, scratch};
    use crate::opening::Purpose;
    use crate::project::{Event as ProjectEvent, Window};

    fn send(app: &mut App, e: Event) {
        let _ = app.update(Message::Project(Box::new(ProjectEvent::Type(e))));
    }

    /// Opens a file through the open's own path, as Dosya → Aç does.
    fn open(app: &mut App, path: PathBuf) {
        let task = app.start_opening(path, Purpose::File);
        drive(app, task);
    }

    fn asked(app: &App) -> bool {
        app.dialog == Some(Dialog::Project) && matches!(app.project, Some(Window::Type(_)))
    }

    #[test]
    fn a_project_opened_without_a_type_is_asked_it_and_sec_writes_it() {
        let dir = scratch("tur-sorusu");
        // The web's sample drawing, written under the former Hibrit mode.
        let path = saved(&dir, "eski.kcad", 0);
        let mut app = app_with_drawing();
        open(&mut app, path);
        assert!(asked(&app));
        let doc = app.document.as_ref().expect("a drawing");
        assert_eq!(doc.settings().workspace, None);
        assert!(!doc.dirty());
        let undo = doc.model.can_undo();
        // Seç writes the chosen type: an edit, not an undo step.
        send(&mut app, Event::Choose(Workspace::Cad));
        send(&mut app, Event::Pick);
        assert_eq!(app.dialog, None);
        let doc = app.document.as_ref().expect("a drawing");
        assert_eq!(doc.settings().workspace, Some(Workspace::Cad));
        assert!(doc.dirty());
        assert_eq!(doc.model.can_undo(), undo);
        assert_eq!(app.work_mode(), Workspace::Cad);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn sonra_or_esc_leave_it_unasked_shown_as_cbs_and_the_next_open_asks_again() {
        let dir = scratch("tur-sonra");
        let path = saved(&dir, "eski.kcad", 0);
        let mut app = app_with_drawing();
        open(&mut app, path.clone());
        send(&mut app, Event::Later);
        assert_eq!(app.dialog, None);
        let doc = app.document.as_ref().expect("a drawing");
        assert_eq!(doc.settings().workspace, None);
        assert!(!doc.dirty());
        assert_eq!(app.work_mode(), Workspace::Gis);
        assert!(
            last_said(&app).contains("türü seçilmedi; CBS olarak gösteriliyor"),
            "{}",
            last_said(&app)
        );
        open(&mut app, path);
        assert!(asked(&app), "asked again");
        // Esc or × says the same.
        let _ = app.update(Message::DialogClosed);
        assert!(app.project.is_none() && app.dialog.is_none());
        assert!(last_said(&app).contains("türü seçilmedi"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_project_with_its_type_is_not_asked_and_the_question_is_the_drawing_s_it_was_asked_about() {
        let dir = scratch("tur-var");
        let path = saved(&dir, "eski.kcad", 0);
        let mut app = app_with_drawing();
        open(&mut app, path.clone());
        send(&mut app, Event::Pick);
        let typed = dir.join("türlü.kcad");
        let snapshot = app
            .document
            .as_ref()
            .expect("a drawing")
            .model
            .to_snapshot_v2();
        crate::document::write(&snapshot, &typed).expect("writes");
        open(&mut app, typed);
        assert!(!asked(&app), "a typed project is not asked");
        // Asked about one drawing, answered over another: the answer goes nowhere.
        open(&mut app, path);
        assert!(asked(&app));
        let other = crate::files_testing::drawing(1);
        let _ = app.update(Message::Opened(Some(Ok(Box::new(other)))));
        send(&mut app, Event::Choose(Workspace::Cad));
        send(&mut app, Event::Pick);
        assert_eq!(
            app.document
                .as_ref()
                .expect("a drawing")
                .settings()
                .workspace,
            None
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
