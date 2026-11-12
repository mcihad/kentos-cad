//! A file another KentOS program opens (docs/adr/0801): a module's program
//! opens only its own module's files. Asked before anything goes on screen:
//! “Bu dosya bir KentOS GIS projesi. KentOS GIS'te açılsın mı?”, and the
//! button starts that program with the file (program.rs); Vazgeç, Esc and ×
//! leave everything as it was. A CAD drawing whose work is not known yet (an
//! older KentOS CAD file, a DWG, DXF or NCZ) is offered to both CAD programs
//! from KentOS GIS; the program it goes to asks its work.

use std::path::{Path, PathBuf};

use iced::Element;
use iced::widget::column;
use kentos_interaction::Level;
use kentos_launch::Module;
use kentos_ui::label;
use kentos_ui::widget::{Dialog, overlay};

use super::{Event as ProjectEvent, Window, message};
use crate::app::{App, Message};
use crate::exchange::words;
use crate::opening::Opened;
use crate::program::{a_project_of, in_program, module_of_work, open_in};
use crate::project::ask_work::guessed_work;

/// The question's title.
pub(crate) const HAND_TITLE: &str = "Başka programın dosyası";
const CANCEL: &str = "Vazgeç";

#[derive(Debug)]
pub struct State {
    path: PathBuf,
    /// The file's name, as the question says it.
    pub(super) name: String,
    /// The programs it may go to: its module's, or both CAD programs for a CAD drawing whose work is not known.
    targets: Vec<Module>,
    /// The one the primary button starts: the drawing's look's (a coordinate system: KentOS Harita) among two.
    preferred: Module,
}

#[derive(Debug, Clone)]
pub enum Event {
    Open(Module),
    Cancel,
}

fn event(e: Event) -> Message {
    message(ProjectEvent::Hand(e))
}

impl App {
    /// Asks to hand a read file over to `targets`' programs; what was read is dropped.
    pub(crate) fn ask_hand_over(&mut self, opened: &Opened, targets: Vec<Module>) {
        let name = opened.source.file_name().map_or_else(
            || opened.source.display().to_string(),
            |n| n.to_string_lossy().into_owned(),
        );
        let guess = module_of_work(guessed_work(opened.doc.settings()));
        let preferred = if targets.contains(&guess) {
            guess
        } else {
            targets.first().copied().unwrap_or(guess)
        };
        let state = State {
            path: opened.source.clone(),
            name,
            targets,
            preferred,
        };
        self.open_project_window(Window::Hand(Box::new(state)));
    }

    pub(super) fn project_hand_event(&mut self, e: Event) {
        let Some(Window::Hand(state)) = &self.project else {
            return;
        };
        let (path, name) = (state.path.clone(), state.name.clone());
        let targets = state.targets.clone();
        self.close_project_window();
        match e {
            Event::Open(module) if targets.contains(&module) => {
                self.hand_file_over(module, &path, &name)
            }
            Event::Open(_) => {}
            Event::Cancel => self.output(format!(
                "“{name}” açılmadı; ekrandaki çizim olduğu gibi duruyor."
            )),
        }
    }

    /// Starts `module`'s program with the file: said either way, nothing opens here.
    pub(crate) fn hand_file_over(&mut self, module: Module, path: &Path, name: &str) {
        match (self.hand_over)(module, path) {
            Ok(_) => self.say(
                Level::Success,
                format!("“{name}” {} açılıyor.", in_program(module)),
            ),
            Err(why) => self.say(Level::Error, why),
        }
    }

    /// Whether the hand-over question is the window on top.
    #[cfg(test)]
    pub(crate) fn asking_hand_over(&self) -> Option<&[Module]> {
        match &self.project {
            Some(Window::Hand(s)) if self.dialog == Some(crate::app::Dialog::Project) => {
                Some(&s.targets)
            }
            _ => None,
        }
    }

    pub(super) fn project_hand_view<'a>(&'a self, s: &'a State) -> Element<'a, Message> {
        let here = self.program_title();
        let (lead, note) = match s.targets.as_slice() {
            [one] => (
                format!(
                    "“{}” {}. {} açılsın mı?",
                    s.name,
                    a_project_of(*one),
                    in_program(*one)
                ),
                format!("{here} yalnız kendi projelerini açar; dosya değişmez."),
            ),
            _ => (
                format!(
                    "“{}” bir CAD çizimi; {here} yalnız CBS projelerini açar. Hangi programda açılsın?",
                    s.name
                ),
                "Mimari proje KentOS Mimari'de, Harita / imar planı KentOS Harita'da açılır; çizimin işi orada sorulur. Dosya değişmez."
                    .to_owned(),
            ),
        };
        // One primary answer (DESIGN.md §7.9): the preferred program's; the other stands apart on the left.
        let mut dialog = Dialog::new(HAND_TITLE)
            .push(column![label::body(lead), label::muted(note)].spacing(10));
        for m in s.targets.iter().filter(|m| **m != s.preferred) {
            dialog = dialog.aside(words::secondary(open_in(*m), Some(event(Event::Open(*m)))));
        }
        let dialog = dialog
            .action(words::secondary(CANCEL, Some(event(Event::Cancel))))
            .action(words::primary(
                open_in(s.preferred),
                Some(event(Event::Open(s.preferred))),
            ));
        overlay::blocking(dialog.width(560.0))
    }
}
