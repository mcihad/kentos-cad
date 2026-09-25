//! The desktop shell's state and update (docs/adr/0017): the open drawing,
//! the ribbon, the docked panels, the command line and the dialogs.
//!
//! Every button, shortcut and typed name runs a web command id through
//! [`App::run`]; the ones the desktop does not run yet say so instead of
//! doing nothing (CLAUDE.md §4.5).

use std::path::PathBuf;

use iced::widget::operation;
use iced::{Event, Subscription, Task, Theme, event, keyboard, window};

use kentos_ui::icon::Icon;
use kentos_ui::theme::{self, Accent, Mode};
use kentos_ui::widget::command_line::Entry;
use kentos_ui::widget::docking::{self, Docks, Side};

use crate::catalog::{Standing, catalog};
use crate::document::Document;

pub const COMMAND_INPUT: &str = "komut-satiri";

/// Width of the right dock at start (12 px body text; the showcase's).
const DOCK_WIDTH: f32 = 320.0;

/// The docked panels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Panel {
    Layers,
    Properties,
}

impl Panel {
    pub fn title(self) -> &'static str {
        match self {
            Panel::Layers => "Katmanlar",
            Panel::Properties => "Özellikler",
        }
    }

    pub fn icon(self) -> Icon {
        match self {
            Panel::Layers => Icon::Layers,
            Panel::Properties => Icon::Properties,
        }
    }

    fn layout() -> Docks<Panel> {
        let mut docks = Docks::new();
        docks.dock(Panel::Layers, Side::Right);
        docks.split(Panel::Properties, Side::Right);
        docks.set_size(Side::Right, DOCK_WIDTH);
        docks
    }
}

/// What waits for an answer in the middle of the window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dialog {
    About,
    Shortcuts,
    /// The drawing has unsaved changes; asked before `then` throws them away.
    Unsaved(Then),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Then {
    Open,
    Close(window::Id),
}

#[derive(Debug, Clone)]
pub enum Message {
    /// Run a web command id (ribbon, shortcut, command line, dialog).
    Run(&'static str),
    RibbonTab(&'static str),
    CommandInput(String),
    CommandSubmitted,
    CommandRun(String),
    CommandHistoryToggled,
    /// Typed while nothing had the focus: starts the command line (CAD habit).
    Typed(String),
    Dock(docking::Event<Panel>),
    LayerSelected(String),
    LayerVisible(String),
    LayerLocked(String),
    LayerExpanded(String),
    /// A drawing read from disk, or `None` when the file dialog was cancelled.
    /// Boxed: a drawing is large and messages are moved often.
    Opened(Option<Result<Box<Document>, String>>),
    /// A save of `revision` finished, or `None` when the dialog was cancelled.
    Saved(Option<Result<(PathBuf, u64), String>>),
    CloseRequested(window::Id),
    DialogConfirmed,
    DialogClosed,
}

pub struct App {
    pub document: Option<Document>,
    pub tab: &'static str,
    pub ribbon_collapsed: bool,
    pub mode: Mode,
    pub accent: Accent,
    pub docks: Docks<Panel>,
    pub selected_layer: Option<String>,
    pub history: Vec<Entry>,
    pub command_input: String,
    pub command_expanded: bool,
    pub dialog: Option<Dialog>,
}

impl App {
    /// The shell, opening `path` at once when given (`kentos-cad cizim.kcad`).
    pub fn boot(path: Option<PathBuf>) -> (Self, Task<Message>) {
        let app = Self {
            document: None,
            tab: catalog().tabs().nth(1).or(catalog().tabs().next()).map_or("home", |tab| tab.id),
            ribbon_collapsed: false,
            mode: Mode::Dark,
            accent: Accent::default(),
            docks: Panel::layout(),
            selected_layer: None,
            history: vec![Entry::Output(
                "KentOS CAD masaüstü hazır. Web'deki bütün komutlar şeritte; masaüstüne taşınmayanlar bunu söyler."
                    .to_owned(),
            )],
            command_input: String::new(),
            command_expanded: false,
            dialog: None,
        };
        let task = match path {
            Some(path) => Task::perform(
                async move { Some(Document::read(&path).map(Box::new)) },
                Message::Opened,
            ),
            None => Task::none(),
        };
        (app, task)
    }

    pub fn title(&self) -> String {
        match &self.document {
            Some(doc) => format!(
                "{}{} — KentOS CAD",
                doc.name(),
                if doc.dirty { " •" } else { "" }
            ),
            None => "KentOS CAD".to_owned(),
        }
    }

    pub fn theme(&self) -> Theme {
        theme::theme(self.mode, self.accent)
    }

    pub fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            event::listen_with(key_event),
            window::close_requests().map(Message::CloseRequested),
        ])
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Run(id) => return self.run(id),
            Message::RibbonTab(id) => self.tab = id,
            Message::CommandInput(text) => self.command_input = text,
            Message::CommandSubmitted => {
                let text = std::mem::take(&mut self.command_input);
                return self.run_typed(text.trim());
            }
            Message::CommandRun(name) => {
                self.command_input.clear();
                return self.run_typed(&name);
            }
            Message::CommandHistoryToggled => self.command_expanded = !self.command_expanded,
            Message::Typed(text) => {
                self.command_input.push_str(&text);
                return operation::focus(COMMAND_INPUT);
            }
            Message::Dock(event) => self.docks.update(event),
            Message::LayerSelected(id) => {
                self.selected_layer = (self.selected_layer.as_deref() != Some(&id)).then_some(id);
            }
            Message::LayerVisible(id) => {
                if let Some(doc) = &mut self.document {
                    doc.change_layer(&id, |n| n.visible = !n.visible);
                }
            }
            Message::LayerLocked(id) => {
                if let Some(doc) = &mut self.document {
                    doc.change_layer(&id, |n| n.locked = !n.locked);
                }
            }
            Message::LayerExpanded(id) => {
                if let Some(doc) = &mut self.document {
                    // Folding is part of the file (the web keeps it too).
                    doc.change_layer(&id, |n| n.expanded = !n.expanded);
                }
            }
            Message::Opened(None) | Message::Saved(None) => {}
            Message::Opened(Some(Ok(doc))) => {
                self.output(format!(
                    "{} açıldı: {} nesne, {} katman.",
                    doc.name(),
                    doc.snapshot.entities.len(),
                    doc.layer_count()
                ));
                self.selected_layer = None;
                self.document = Some(*doc);
            }
            Message::Opened(Some(Err(error))) | Message::Saved(Some(Err(error))) => {
                self.error(error)
            }
            Message::Saved(Some(Ok((path, revision)))) => {
                if let Some(doc) = &mut self.document {
                    doc.saved(path.clone(), revision);
                    let later = if doc.dirty {
                        " Kayıt sürerken yapılan değişiklikler kaydedilmedi."
                    } else {
                        ""
                    };
                    self.output(format!("Kaydedildi: {}.{later}", path.display()));
                }
            }
            Message::CloseRequested(window) => {
                if self.document.as_ref().is_some_and(|doc| doc.dirty) {
                    self.dialog = Some(Dialog::Unsaved(Then::Close(window)));
                } else {
                    return window::close(window);
                }
            }
            Message::DialogConfirmed => {
                if let Some(Dialog::Unsaved(then)) = self.dialog.take() {
                    return match then {
                        Then::Open => self.open(),
                        Then::Close(window) => window::close(window),
                    };
                }
            }
            Message::DialogClosed => self.dialog = None,
        }
        Task::none()
    }

    /// Runs a web command id: the desktop's handler, or a note that it is not here yet.
    pub fn run(&mut self, id: &'static str) -> Task<Message> {
        let Some(command) = catalog().get(id) else {
            self.error(format!("Komut bulunamadı: {id}"));
            return Task::none();
        };
        match command.standing {
            Standing::Ported => {}
            Standing::OnTheWeb => {
                self.output(format!(
                    "“{}” web'de var; masaüstüne henüz taşınmadı.",
                    command.title
                ));
                return Task::none();
            }
            Standing::Pending => {
                let why = command.pending_note.unwrap_or("Geliştirme aşamasında");
                self.output(format!("“{}”: {why}.", command.title));
                return Task::none();
            }
        }

        match id {
            "file.open" => {
                if self.document.as_ref().is_some_and(|doc| doc.dirty) {
                    self.dialog = Some(Dialog::Unsaved(Then::Open));
                    return Task::none();
                }
                return self.open();
            }
            "file.save" => return self.save(false),
            "file.saveAs" => return self.save(true),
            "view.theme.dark" => self.mode = Mode::Dark,
            "view.theme.light" => self.mode = Mode::Light,
            "view.theme.toggle" => {
                self.mode = if self.mode == Mode::Light {
                    Mode::Dark
                } else {
                    Mode::Light
                };
            }
            "view.ribbonCollapse" => self.ribbon_collapsed = !self.ribbon_collapsed,
            "commandline.focus" => return operation::focus(COMMAND_INPUT),
            "help.about" => self.dialog = Some(Dialog::About),
            "help.shortcuts" => self.dialog = Some(Dialog::Shortcuts),
            "layer.showAll" => match &mut self.document {
                Some(doc) => doc.show_all(),
                None => self.output("Açık çizim yok."),
            },
            _ => self.error(format!(
                "{id}: masaüstü işleyicisi eksik (catalog::PORTED ile karşılaştırın)"
            )),
        }
        Task::none()
    }

    /// A name typed in the command line: an alias, the command's name or its title.
    fn run_typed(&mut self, text: &str) -> Task<Message> {
        if text.is_empty() {
            return Task::none();
        }
        self.history.push(Entry::Input(text.to_owned()));
        let folded = fold(text);
        let found = catalog().commands().iter().find(|c| {
            c.aliases.iter().any(|a| fold(a) == folded) || fold(c.title) == folded || c.id == text
        });
        match found {
            Some(command) => self.run(command.id),
            None => {
                self.error(format!("Bilinmeyen komut: {text}. Komut adları için F1 ya da Yardım → Klavye kısayolları."));
                Task::none()
            }
        }
    }

    fn open(&mut self) -> Task<Message> {
        Task::perform(
            async {
                let file = rfd::AsyncFileDialog::new()
                    .set_title("Çizim aç")
                    .add_filter("KentOS çizimi (.kcad)", &["kcad"])
                    .pick_file()
                    .await?;
                Some(Document::read(file.path()).map(Box::new))
            },
            Message::Opened,
        )
    }

    /// Saves to the drawing's file, or asks where (always, with `choose`).
    fn save(&mut self, choose: bool) -> Task<Message> {
        let Some(doc) = &self.document else {
            self.output("Kaydedilecek çizim yok. Önce bir çizim açın (Ctrl+O).");
            return Task::none();
        };
        let doc = doc.clone();
        let known = doc.path.clone().filter(|_| !choose);
        Task::perform(
            async move {
                let path = match known {
                    Some(path) => path,
                    None => {
                        let suggested = doc.name().trim_end_matches(".kcad").to_owned();
                        let file = rfd::AsyncFileDialog::new()
                            .set_title("Farklı kaydet")
                            .add_filter("KentOS çizimi (.kcad)", &["kcad"])
                            .set_file_name(format!("{suggested}.kcad"))
                            .save_file()
                            .await?;
                        let path = file.path().to_path_buf();
                        if path.extension().is_some_and(|e| e == "kcad") {
                            path
                        } else {
                            path.with_extension("kcad")
                        }
                    }
                };
                Some(doc.write(&path).map(|()| (path, doc.revision)))
            },
            Message::Saved,
        )
    }

    fn output(&mut self, text: impl Into<String>) {
        self.history.push(Entry::Output(text.into()));
    }

    fn error(&mut self, text: impl Into<String>) {
        self.history.push(Entry::Error(text.into()));
    }
}

/// Turkish-aware case folding for command names: “çizgi”, “Cizgi” and “CIZGI” match.
fn fold(text: &str) -> String {
    text.trim()
        .chars()
        .map(|c| match c {
            'ç' | 'Ç' => 'c',
            'ğ' | 'Ğ' => 'g',
            'ı' | 'I' | 'İ' | 'i' => 'i',
            'ö' | 'Ö' => 'o',
            'ş' | 'Ş' => 's',
            'ü' | 'Ü' => 'u',
            c => c.to_ascii_lowercase(),
        })
        .collect()
}

/// Keys: a chord with Ctrl or Alt, or a function key, runs the web command
/// bound to it; other typed text starts the command line. Single-letter tool
/// keys come with the tools they start.
fn key_event(event: Event, status: event::Status, _window: window::Id) -> Option<Message> {
    let Event::Keyboard(keyboard::Event::KeyPressed {
        key,
        modifiers,
        text,
        ..
    }) = event
    else {
        return None;
    };
    if status == event::Status::Captured {
        return None;
    }
    let name = match key.as_ref() {
        keyboard::Key::Character(c) => c.to_uppercase(),
        keyboard::Key::Named(named) => match named {
            keyboard::key::Named::F1 => "F1".into(),
            keyboard::key::Named::F2 => "F2".into(),
            keyboard::key::Named::F3 => "F3".into(),
            keyboard::key::Named::F4 => "F4".into(),
            keyboard::key::Named::F6 => "F6".into(),
            keyboard::key::Named::F7 => "F7".into(),
            keyboard::key::Named::F8 => "F8".into(),
            keyboard::key::Named::F9 => "F9".into(),
            keyboard::key::Named::F10 => "F10".into(),
            _ => return None,
        },
        _ => return None,
    };
    let function = name.starts_with('F') && name.len() > 1;
    if modifiers.control() || modifiers.alt() || function {
        let mut chord = String::new();
        for (on, part) in [
            (modifiers.control(), "Ctrl+"),
            (modifiers.alt(), "Alt+"),
            (modifiers.shift(), "Shift+"),
        ] {
            if on {
                chord.push_str(part);
            }
        }
        chord.push_str(&name);
        return catalog()
            .commands()
            .iter()
            .find(|c| c.shortcuts.contains(&chord.as_str()))
            .map(|c| Message::Run(c.id));
    }
    let text = text?;
    (!text.chars().any(char::is_control)).then(|| Message::Typed(text.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::PORTED;

    #[test]
    fn every_ported_command_has_a_handler() {
        for id in PORTED {
            let (mut app, _) = App::boot(None);
            let _ = app.run(id);
            assert!(
                !app.history
                    .iter()
                    .any(|e| matches!(e, Entry::Error(text) if text.contains("işleyicisi eksik"))),
                "{id} is listed as ported but has no handler"
            );
        }
    }

    #[test]
    fn a_command_not_ported_says_so_and_changes_nothing() {
        let (mut app, _) = App::boot(None);
        let before = app.history.len();
        let _ = app.run("tool.line");
        assert_eq!(app.history.len(), before + 1);
        assert!(
            matches!(app.history.last(), Some(Entry::Output(text)) if text.contains("masaüstüne henüz taşınmadı"))
        );
    }

    #[test]
    fn typed_names_find_commands_whatever_the_case_and_turkish_letters() {
        let (mut app, _) = App::boot(None);
        app.mode = Mode::Light;
        let _ = app.update(Message::CommandRun("temayı değiştir".into()));
        assert_eq!(app.mode, Mode::Dark, "the title works as a name");
        let _ = app.update(Message::CommandRun("OLMAYAN".into()));
        assert!(matches!(app.history.last(), Some(Entry::Error(_))));
    }
}
