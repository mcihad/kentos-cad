//! What a CAD drawing is drawn for, asked once (docs/adr/0801; as the type of
//! a project without one is, ask_type.rs): Mimari proje or Harita / imar
//! planı (docs/adr/0802: two CAD works), which decides the KentOS program the
//! project belongs to and the ribbon's tabs. A CAD project written before works has none.
//!
//! kentos-cad asks it over the drawing on screen once it is open; Seç writes
//! the work to the project (an edit, not an undo step, as any project
//! setting). A module's program asks before the drawing goes on screen: a
//! work of this module places the drawing with it, a work of another module
//! hands the file to that module's program (program.rs), and nothing is
//! opened here. A project without a type is asked its type and work at once
//! there, with the CBS project beside the works. Sonra, Esc and × place the
//! drawing as it is: kentos-cad shows the whole CAD ribbon, a module its own
//! work's; the next open asks again, as does the status bar's type cell.

use iced::Element;
use iced::widget::{Column, column};
use kentos_commands::{CadWork, ProjectSettings, SettingsSet, Workspace};
use kentos_interaction::Level;
use kentos_launch::Module;
use kentos_project::wizard::WORKS;
use kentos_ui::icon::icon;
use kentos_ui::label;
use kentos_ui::widget::{Dialog, overlay};

use super::{Event as ProjectEvent, Window, message};
use crate::app::{App, Message};
use crate::exchange::words;
use crate::opening::Opened;
use crate::program::{module_of_work, open_in, type_of, works_of};

/// The question, as the window's title says it.
pub(crate) const WORK_TITLE: &str = "Bu CAD çizimi hangi iş için?";
/// The question for a project without a type, asked its type and work at once.
pub(crate) const KIND_TITLE: &str = "Bu proje hangi iş için?";
const LATER: &str = "Sonra";
const PICK: &str = "Seç";

/// A card of the question.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Choice {
    Work(CadWork),
    /// A CBS project: only for a project without a type, in a module's program.
    Gis,
}

impl Choice {
    /// The module the answer's project belongs to.
    pub(crate) fn module(self) -> Module {
        match self {
            Choice::Work(w) => module_of_work(w),
            Choice::Gis => Module::Gis,
        }
    }
}

/// What is asked about.
#[derive(Debug)]
enum About {
    /// The drawing on screen, by its session.
    Shown(u64),
    /// A module's program's open, held until answered.
    Held(Box<Opened>),
}

#[derive(Debug)]
pub struct State {
    about: About,
    /// The drawing's name, as the question says it.
    pub(super) name: String,
    choices: Vec<Choice>,
    choice: Choice,
    /// The project has no type yet: the answer gives it its type too.
    typeless: bool,
}

#[derive(Debug, Clone)]
pub enum Event {
    Choose(Choice),
    Pick,
    Later,
}

fn event(e: Event) -> Message {
    message(ProjectEvent::Work(e))
}

/// The work a drawing looks drawn for, as the wizard's works make them (docs/adr/0802): a drawing on a
/// coordinate system a map, any other a building.
pub(crate) fn guessed_work(s: &ProjectSettings) -> CadWork {
    if s.has_system() {
        CadWork::Map
    } else {
        CadWork::Architecture
    }
}

/// The guess, when it is the module's; else the module's first work.
pub(crate) fn guessed_work_in(m: Module, s: &ProjectSettings) -> CadWork {
    let works = works_of(m);
    let guess = guessed_work(s);
    if works.is_empty() || works.contains(&guess) {
        guess
    } else {
        works[0]
    }
}

/// Writes the answer to the project: its work, and its type for a project without one; an edit, not an undo step.
fn answered(
    doc: &mut kentos_domain::Document,
    typeless: bool,
    choice: Choice,
) -> Result<(), String> {
    let module = choice.module();
    let input = SettingsSet {
        project_type: typeless.then_some(type_of(module)),
        cad_work: match choice {
            Choice::Work(w) => Some(w),
            Choice::Gis => None,
        },
        ..Default::default()
    };
    use kentos_commands::CommandResult;
    match kentos_native_application::settings_set::execute(
        &mut kentos_native_application::ExecutionContext::new(doc),
        input,
    ) {
        CommandResult::Completed { .. } => Ok(()),
        CommandResult::Failed { error }
        | CommandResult::Conflict { error }
        | CommandResult::NeedsInput { error } => Err(error.message),
        CommandResult::Queued { .. } | CommandResult::Cancelled => {
            Err("Çizimin işi yazılamadı.".to_owned())
        }
    }
}

/// Puts a held drawing's work (and type) as if the question were answered with it: a DWG, DXF or NCZ a CAD
/// module opens is drawn for its work.
pub(crate) fn give_work(opened: &mut Opened, work: CadWork) {
    let _ = answered(&mut opened.doc.model, false, Choice::Work(work));
}

impl App {
    /// After an open the user made (kentos-cad): a CAD drawing without a work is asked its work.
    /// The status bar's type cell asks it in any program; a module's program
    /// offers its own works only, nothing being handed over from the screen.
    pub(crate) fn ask_cad_work(&mut self) {
        let Some(doc) = &self.document else { return };
        let s = doc.settings();
        if s.project_type() != Some(Workspace::Cad)
            || s.cad_work().is_some()
            || self.dialog.is_some()
        {
            return;
        }
        let choices: Vec<Choice> = match self.program.filter(|m| !works_of(*m).is_empty()) {
            Some(m) => works_of(m).into_iter().map(Choice::Work).collect(),
            None => CadWork::ALL.into_iter().map(Choice::Work).collect(),
        };
        let guess = match self.program {
            Some(m) => guessed_work_in(m, s),
            None => guessed_work(s),
        };
        let choice = choices
            .iter()
            .copied()
            .find(|c| *c == Choice::Work(guess))
            .unwrap_or(choices[0]);
        let state = State {
            about: About::Shown(doc.session),
            name: doc.name().to_string(),
            choices,
            choice,
            typeless: false,
        };
        self.open_project_window(Window::Work(Box::new(state)));
    }

    /// A module's program's open (docs/adr/0801): a CAD drawing without a work,
    /// or a project without a type, held until it is answered; the module's work
    /// first.
    pub(crate) fn ask_cad_work_held(&mut self, opened: Opened, typeless: bool) {
        let Some(program) = self.program else {
            let _ = self.place_opened(opened);
            return;
        };
        let mut choices: Vec<Choice> = CadWork::ALL.into_iter().map(Choice::Work).collect();
        if typeless {
            choices.push(Choice::Gis);
        }
        let choice = if program == Module::Gis {
            Choice::Gis
        } else {
            Choice::Work(guessed_work_in(program, opened.doc.settings()))
        };
        let state = State {
            name: opened.doc.name().to_string(),
            about: About::Held(Box::new(opened)),
            choices,
            choice,
            typeless,
        };
        self.open_project_window(Window::Work(Box::new(state)));
    }

    pub(super) fn project_work_event(&mut self, e: Event) -> iced::Task<Message> {
        let Some(Window::Work(state)) = &mut self.project else {
            return iced::Task::none();
        };
        match e {
            Event::Choose(c) => {
                if state.choices.contains(&c) {
                    state.choice = c;
                }
                iced::Task::none()
            }
            Event::Pick => {
                let Some(Window::Work(state)) = self.project.take() else {
                    return iced::Task::none();
                };
                self.close_project_window();
                self.work_answered(*state)
            }
            Event::Later => self.work_left_unasked(),
        }
    }

    /// Seç, or the button of another module's program.
    fn work_answered(&mut self, state: State) -> iced::Task<Message> {
        let State {
            about,
            name,
            choice,
            typeless,
            ..
        } = state;
        let module = choice.module();
        match about {
            About::Held(mut opened) => {
                // Another module's work: its program opens the file; nothing opens here.
                if self.program.is_some_and(|p| p != module) {
                    self.hand_file_over(module, &opened.source.clone(), &name);
                    return iced::Task::none();
                }
                if let Err(why) = answered(&mut opened.doc.model, typeless, choice) {
                    self.warn(why);
                }
                opened.asked = true;
                let task = self.place_opened(*opened);
                self.work_said(&name, choice);
                task
            }
            About::Shown(session) => {
                // The question was about the drawing on screen when it was asked.
                let Some(doc) = self.document.as_mut().filter(|d| d.session == session) else {
                    return iced::Task::none();
                };
                match answered(&mut doc.model, typeless, choice) {
                    Ok(()) => {
                        // The tab stays open while the work keeps it.
                        self.tab = self.regular_tab();
                        self.work_said(&name, choice);
                    }
                    Err(why) => self.warn(why),
                }
                iced::Task::none()
            }
        }
    }

    fn work_said(&mut self, name: &str, choice: Choice) {
        let what = match choice {
            Choice::Work(w) => w.name(),
            Choice::Gis => "CBS projesi",
        };
        self.say(
            Level::Success,
            format!(
                "“{name}” çiziminin işi: {what}. Projeye yazıldı; kaydedince dosyada kalır. Şeritte olmayan komutlar komut satırından ve kısayoluyla yine çalışır."
            ),
        );
    }

    /// Sonra, Esc or ×: a held drawing goes on screen as it is; the work stays unasked.
    pub(super) fn work_left_unasked(&mut self) -> iced::Task<Message> {
        let Some(Window::Work(state)) = self.project.take() else {
            return iced::Task::none();
        };
        self.close_project_window();
        let State { about, name, .. } = *state;
        let task = match about {
            About::Held(mut opened) => {
                opened.asked = true;
                self.place_opened(*opened)
            }
            About::Shown(_) => iced::Task::none(),
        };
        let shown = match self.ribbon_work() {
            Some(w) if self.program.is_some() => format!("{} şeridi gösteriliyor", w.name()),
            _ => "bütün CAD şeridi gösteriliyor".to_owned(),
        };
        self.output(format!(
            "“{name}” çiziminin işi seçilmedi; {shown}. Durum çubuğundaki türe basılınca seçilir."
        ));
        task
    }

    /// Whether the work question is the window on top.
    #[cfg(test)]
    pub(crate) fn asking_work(&self) -> bool {
        self.dialog == Some(crate::app::Dialog::Project)
            && matches!(self.project, Some(Window::Work(_)))
    }

    pub(super) fn project_work_view<'a>(&'a self, s: &'a State) -> Element<'a, Message> {
        let lead = if s.typeless {
            format!(
                "“{}” projesinin türü ve işi henüz seçilmedi. İş; projenin hangi KentOS programında açılacağını ve şeritte hangi sekmelerin görüneceğini belirler, çizimin verisi değişmez. Seçtiğiniz iş projeye yazılır ve sonra değişmez.",
                s.name
            )
        } else {
            format!(
                "“{}” çiziminin işi henüz seçilmedi. İş; çizimin hangi KentOS programında açılacağını ve şeritte hangi sekmelerin görüneceğini belirler, çizimin verisi değişmez. Seçtiğiniz iş projeye yazılır ve sonra değişmez.",
                s.name
            )
        };
        let cards = Column::with_children(s.choices.iter().map(|c| {
            let (glyph, name, note) = match c {
                Choice::Work(w) => {
                    let card = WORKS.iter().find(|x| x.work == *w).unwrap_or(&WORKS[0]);
                    (
                        crate::icons::from_web(Some(card.icon)),
                        card.name,
                        card.note,
                    )
                }
                Choice::Gis => (
                    crate::modes::mode_icon(Workspace::Gis),
                    "CBS projesi",
                    "Harita katmanları, öznitelikler ve mekânsal analiz; KentOS GIS'te açılır.",
                ),
            };
            super::wizard::choice_card(
                icon(glyph).size(22.0).into(),
                name,
                note,
                s.choice == *c,
                event(Event::Choose(*c)),
            )
        }))
        .spacing(8);
        let body = column![label::body(lead), cards].spacing(14);
        let hands_over = matches!(s.about, About::Held(_))
            && self.program.is_some_and(|p| p != s.choice.module());
        let pick = if hands_over {
            open_in(s.choice.module())
        } else {
            PICK
        };
        overlay::blocking(
            Dialog::new(if s.typeless { KIND_TITLE } else { WORK_TITLE })
                .push(body)
                .action(words::secondary(LATER, Some(event(Event::Later))))
                .action(words::primary(pick, Some(event(Event::Pick))))
                .width(640.0),
        )
    }
}
