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
use crate::document::{self, Document};

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
                if doc.dirty() { " •" } else { "" }
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
            // The layer tree's changes go through the document, as on the web: visibility
            // and lock are edits (unsaved) but not undo steps.
            Message::LayerVisible(id) => {
                if let Some(doc) = &mut self.document {
                    doc.model.toggle_layer_visible(&id);
                }
            }
            Message::LayerLocked(id) => {
                if let Some(doc) = &mut self.document {
                    doc.model.toggle_layer_locked(&id);
                }
            }
            Message::LayerExpanded(id) => {
                if let Some(doc) = &mut self.document
                    && let Some(expanded) = doc.find(&id).map(|n| !n.expanded)
                {
                    // Folding is kept in the file but is not an edit (web).
                    doc.model.set_layer_expanded(&id, expanded);
                }
            }
            Message::Opened(None) | Message::Saved(None) => {}
            Message::Opened(Some(Ok(doc))) => {
                self.output(format!(
                    "{} açıldı: {} nesne, {} katman.",
                    doc.name(),
                    doc.entity_count(),
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
                    let later = if doc.dirty() {
                        " Kayıt sürerken yapılan değişiklikler kaydedilmedi."
                    } else {
                        ""
                    };
                    self.output(format!("Kaydedildi: {}.{later}", path.display()));
                }
            }
            Message::CloseRequested(window) => {
                if self.document.as_ref().is_some_and(Document::dirty) {
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
                if self.document.as_ref().is_some_and(Document::dirty) {
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
            // An edit, not an undo step (web: LayerStore.showAll).
            "layer.showAll" => match &mut self.document {
                Some(doc) => doc.model.show_all_layers(),
                None => self.output("Açık çizim yok."),
            },
            "edit.undo" => self.step_history(true),
            "edit.redo" => self.step_history(false),
            _ => self.error(format!(
                "{id}: masaüstü işleyicisi eksik (catalog::PORTED ile karşılaştırın)"
            )),
        }
        Task::none()
    }

    /// Undoes or redoes the drawing's last step, saying which (the web's
    /// “Geri alındı: Ekle”); with nothing to undo or redo, says that.
    fn step_history(&mut self, undo: bool) {
        let Some(doc) = &mut self.document else {
            self.output("Açık çizim yok.");
            return;
        };
        let step = if undo {
            doc.model.undo()
        } else {
            doc.model.redo()
        };
        self.output(match (step, undo) {
            (Some(label), true) => format!("Geri alındı: {label}"),
            (Some(label), false) => format!("Yinelendi: {label}"),
            (None, true) => "Geri alınacak değişiklik yok.".to_owned(),
            (None, false) => "Yinelenecek değişiklik yok.".to_owned(),
        });
    }

    /// Whether a ported command can run now: undo and redo only with a step
    /// to take (web: `isEnabled`, `watch: [doc.canUndo]`). Buttons of commands
    /// that cannot run are drawn dimmed.
    pub fn available(&self, id: &str) -> bool {
        let doc = self.document.as_ref().map(|doc| &doc.model);
        match id {
            "edit.undo" => doc.is_some_and(kentos_domain::Document::can_undo),
            "edit.redo" => doc.is_some_and(kentos_domain::Document::can_redo),
            _ => true,
        }
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
    /// What is written is the drawing as it is now, with its revision: a change
    /// made while the file is written stays unsaved.
    fn save(&mut self, choose: bool) -> Task<Message> {
        let Some(doc) = &self.document else {
            self.output("Kaydedilecek çizim yok. Önce bir çizim açın (Ctrl+O).");
            return Task::none();
        };
        let snapshot = doc.model.to_snapshot();
        let revision = doc.model.revision();
        let known = doc.path.clone().filter(|_| !choose);
        Task::perform(
            async move {
                let path = match known {
                    Some(path) => path,
                    None => {
                        let suggested = snapshot.name.trim_end_matches(".kcad").to_owned();
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
                Some(document::write(&snapshot, &path).map(|()| (path, revision)))
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

    fn with_demo() -> App {
        let (mut app, _) = App::boot(None);
        let snapshot = kentos_contracts::DocumentSnapshotV1::from_json(include_str!(
            "../../../fixtures/document/v1/sample.json"
        ))
        .expect("the web's demo file reads");
        app.document = Some(Document::new(snapshot, None).expect("opens"));
        app
    }

    fn last_output(app: &App) -> &str {
        match app.history.last() {
            Some(Entry::Output(text)) => text,
            other => panic!("expected an output line, found {other:?}"),
        }
    }

    #[test]
    fn undo_and_redo_run_through_the_document_and_say_what_they_did() {
        let mut app = with_demo();
        assert!(!app.available("edit.undo") && !app.available("edit.redo"));
        let _ = app.run("edit.undo");
        assert_eq!(last_output(&app), "Geri alınacak değişiklik yok.");

        // The desktop has no drawing tools yet: an edit made on the document directly.
        let doc = app.document.as_mut().expect("open");
        let first = doc.model.entities().next().expect("an object").clone();
        let count = doc.entity_count();
        doc.model.add(first).expect("a slot");
        assert!(app.available("edit.undo"));

        let _ = app.run("edit.undo");
        assert_eq!(last_output(&app), "Geri alındı: Ekle");
        let doc = app.document.as_ref().expect("open");
        assert_eq!(doc.entity_count(), count);
        assert!(doc.dirty(), "an undo is a change to save");
        assert!(app.available("edit.redo") && !app.available("edit.undo"));

        let _ = app.run("edit.redo");
        assert_eq!(last_output(&app), "Yinelendi: Ekle");
        assert_eq!(
            app.document.as_ref().map(Document::entity_count),
            Some(count + 1)
        );
        let _ = app.run("edit.redo");
        assert_eq!(last_output(&app), "Yinelenecek değişiklik yok.");
    }

    #[test]
    fn layer_changes_are_edits_but_not_undo_steps_and_folding_is_neither() {
        let mut app = with_demo();
        let group = app
            .document
            .as_ref()
            .and_then(|doc| doc.layers().iter().find(|n| !n.children.is_empty()))
            .map(|n| n.id.clone())
            .expect("the demo has a group");
        let _ = app.update(Message::LayerExpanded(group.clone()));
        let doc = app.document.as_ref().expect("open");
        assert!(doc.find(&group).is_some_and(|n| !n.expanded));
        assert!(!doc.dirty(), "folding is not an edit (web)");

        let _ = app.update(Message::LayerVisible(group.clone()));
        let _ = app.update(Message::LayerLocked(group));
        let _ = app.run("layer.showAll");
        let doc = app.document.as_ref().expect("open");
        assert!(doc.dirty());
        assert!(
            !app.available("edit.undo"),
            "visibility and lock are not undo steps (web)"
        );
    }

    #[test]
    fn the_web_keys_undo_and_redo() {
        use keyboard::key::{Code, Physical};
        use keyboard::{Key, Location, Modifiers};
        let chord = |modifiers: Modifiers, letter: &str| {
            key_event(
                Event::Keyboard(keyboard::Event::KeyPressed {
                    key: Key::Character(letter.into()),
                    modified_key: Key::Character(letter.into()),
                    physical_key: Physical::Code(Code::KeyZ),
                    location: Location::Standard,
                    modifiers,
                    text: None,
                    repeat: false,
                }),
                event::Status::Ignored,
                window::Id::unique(),
            )
        };
        assert!(matches!(
            chord(Modifiers::CTRL, "z"),
            Some(Message::Run("edit.undo"))
        ));
        assert!(matches!(
            chord(Modifiers::CTRL, "y"),
            Some(Message::Run("edit.redo"))
        ));
        assert!(matches!(
            chord(Modifiers::CTRL | Modifiers::SHIFT, "Z"),
            Some(Message::Run("edit.redo"))
        ));
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
