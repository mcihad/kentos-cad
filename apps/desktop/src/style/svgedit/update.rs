//! What the SVG editor's events do on the application's side (the web's
//! `openSvgEditor`, `SvgEditor.save` and `confirmClose`): opening it on a
//! library drawing or a new one, over the window that opened it; keys;
//! saving to the library (a system drawing as the user's copy) and telling
//! the opener; closing, asking first about unsaved changes.

use iced::Task;
use iced::advanced::widget::operate;
use kentos_native_style::library::{ItemKind, Source};
use kentos_svg_core::import::{ImportOptions, SymbolColor};
use serde_json::{Map, Value};

use super::files::{self, Pending, read};
use super::keys::KeyOutcome;
use super::state::{After, Opening, Original, Question, SvgEditor};
use super::{Change, Event, ev};
use crate::app::{App, Dialog, Message};
use crate::keys::KeyPress;

/// How a library drawing is read (`readSvg` with its defaults: the symbol's colours kept).
fn library_options() -> ImportOptions {
    ImportOptions {
        symbol_color: SymbolColor::Auto,
        second_color: None,
        editor: false,
    }
}

impl App {
    /// The preview's colours of the theme: the symbol's colour and the paper.
    fn svgedit_theme(&self) -> (String, String) {
        let p = self.style_palette();
        (p.ink, p.paper)
    }

    /// The editor on a drawing of the library, or on a new one (`openSvgEditor`).
    pub(crate) fn open_svg_editor(&mut self, o: Opening) {
        let (ink, paper) = self.svgedit_theme();
        let mut note: Option<(String, bool)> = None;
        let mut reference = None;
        let (doc, original, name, path) = match &o.id {
            Some(id) => {
                let Some((item, source)) = self.styles.library.get(id) else {
                    return;
                };
                if item.kind() != ItemKind::Asset || item.format() != Some("svg") {
                    self.warn("Yalnızca SVG çizimleri düzenlenebilir; görüntüler (PNG, JPEG) değişmez.");
                    return;
                }
                let read = match read::read_svg(item.data().unwrap_or(""), &library_options()) {
                    Ok(r) => r,
                    Err(e) => {
                        self.warn(format!("“{}” açılamadı: {e}", item.name()));
                        return;
                    }
                };
                let editable = source.editable();
                let (_, lost) = read::summary(&read.imported.report);
                if !lost.is_empty() {
                    note = Some((format!("Açılırken: {}.", lost.join(", ")), true));
                } else if !editable {
                    note = Some((
                        "Sistem çizimi: kaydedince Kitaplığım'a kopyası yazılır.".to_owned(),
                        false,
                    ));
                }
                reference = read.imported.reference.clone();
                let name = if editable {
                    item.name().to_owned()
                } else {
                    format!("{} (kopya)", item.name())
                };
                let path: Vec<String> = if editable {
                    item.path().into_iter().map(str::to_owned).collect()
                } else {
                    o.path.clone().unwrap_or_else(|| vec!["Çizimlerim".to_owned()])
                };
                (
                    read.doc,
                    Some(Original {
                        id: id.clone(),
                        source,
                        editable,
                    }),
                    name,
                    path,
                )
            }
            None => (
                super::Drawing::new(100.0, 100.0),
                None,
                "Yeni çizim".to_owned(),
                o.path.clone().unwrap_or_else(|| vec!["Çizimlerim".to_owned()]),
            ),
        };
        let mut ed = SvgEditor::new(
            doc,
            original,
            name,
            path,
            o.after,
            self.dialog,
            &ink,
            &paper,
        );
        ed.images = self.styles.images.clone();
        *ed.theme.borrow_mut() = (ink, paper);
        files::reference::restore(&mut ed, reference, &self.styles.images);
        if let Some((text, warn)) = note {
            ed.status(text, warn);
        }
        self.styles.svg_editor = Some(ed);
        self.dialog = Some(Dialog::SvgEditor);
    }

    /// The window goes; the one under it comes back when it is still there.
    fn close_svg_editor(&mut self) {
        let under = self.styles.svg_editor.take().and_then(|e| e.under);
        self.dialog = under.filter(|d| match d {
            Dialog::StyleManager => self.styles.manager.is_some(),
            Dialog::SymbolDesigner => self.styles.designer.is_some(),
            Dialog::LayerStyle => self.styles.layer_style.is_some(),
            _ => true,
        });
    }

    /// Esc, Vazgeç, × or the backdrop (`confirmClose`): a question or a window
    /// over the editor goes first; unsaved changes are asked about; else it
    /// closes. Sets the dialog itself (`close_dialog` has taken it).
    pub(crate) fn svgedit_close_request(&mut self) {
        let Some(ed) = &mut self.styles.svg_editor else {
            return;
        };
        if ed.question.take().is_some() || ed.files.dialog.take().is_some() {
            self.dialog = Some(Dialog::SvgEditor);
            return;
        }
        if ed.dirty() && ed.savable() {
            ed.question = Some(Question::Close);
            self.dialog = Some(Dialog::SvgEditor);
            return;
        }
        self.close_svg_editor();
    }

    /// Kaydet (`save`): false when it refused (an empty drawing) and said why.
    fn svgedit_save(&mut self) -> bool {
        let Some(ed) = &mut self.styles.svg_editor else {
            return false;
        };
        if !ed.savable() {
            ed.warn("Boş çizim kaydedilmez: önce bir şekil çizin.");
            return false;
        }
        let svg = kentos_native_style::file::sanitize_svg(&ed.svg_text(false));
        let name = ed.save_name();
        let path = ed.save_path();
        let (width, height) = (ed.doc.width, ed.doc.height);
        let original = ed.original.clone();
        let result = match original.filter(|o| o.editable) {
            Some(o) => {
                let mut patch = Map::new();
                patch.insert("name".into(), Value::from(name.clone()));
                patch.insert("path".into(), serde_json::json!(path));
                patch.insert("data".into(), Value::from(svg));
                patch.insert("width".into(), Value::from(width));
                patch.insert("height".into(), Value::from(height));
                self.styles
                    .library
                    .update(&o.id, &patch)
                    .map(|source| (o.id.clone(), source))
            }
            None => {
                let asset = kentos_native_style::file::svg_asset(
                    &name,
                    &path,
                    &svg,
                    &kentos_native_style::library::new_item_id("a"),
                );
                self.styles
                    .library
                    .add(Source::User, asset)
                    .map(|item| (item.id().to_owned(), Source::User))
            }
        };
        match result {
            Ok((id, source)) => {
                self.library_changed(source);
                if let Some(ed) = &mut self.styles.svg_editor {
                    ed.original = Some(Original {
                        id: id.clone(),
                        source,
                        editable: true,
                    });
                    ed.saved();
                    ed.say(format!("“{name}” kaydedildi."));
                }
                self.svgedit_saved(&id);
                true
            }
            Err(e) => {
                if let Some(ed) = &mut self.styles.svg_editor {
                    ed.warn(e);
                }
                false
            }
        }
    }

    /// Tells the window that opened the editor about a saved drawing (`onSaved`).
    pub(crate) fn svgedit_saved(&mut self, id: &str) {
        let after = self
            .styles
            .svg_editor
            .as_ref()
            .map_or(After::Nothing, |e| e.after.clone());
        match after {
            After::Manager => self.reveal_in_manager(id),
            After::Designer(key) => {
                if let Some(d) = &mut self.styles.designer {
                    let at = d.selected;
                    d.edit(crate::style::designer::Edit {
                        at,
                        key: key.clone(),
                        text: None,
                        patch: Some(kentos_native_style::designer::set(&key, Value::from(id))),
                        focus: None,
                    });
                }
            }
            After::Nothing => {}
        }
    }

    /// A key while the editor is on top: Esc at once, the rest once it is
    /// known whether a field holds the keyboard.
    pub(crate) fn svgedit_key(&mut self, press: &KeyPress) -> Option<Task<Message>> {
        use iced::keyboard::key::Named;
        let ed = self.styles.svg_editor.as_mut()?;
        if ed.question.is_some() {
            return None;
        }
        if ed.files.dialog.is_some() {
            // A window over the editor keeps the editor's keys; Esc closes it.
            if press.named() == Some(Named::Escape) {
                ed.files.dialog = None;
                ed.touch();
            }
            return Some(Task::none());
        }
        if press.named() == Some(Named::Escape) {
            let outcome = ed.key(press, &crate::style::designer::Focus::default());
            return Some(self.svgedit_outcome(outcome));
        }
        let press = press.clone();
        Some(
            operate(crate::style::designer::focused())
                .map(move |focus| ev(Event::Key(press.clone(), focus))),
        )
    }

    fn svgedit_outcome(&mut self, outcome: KeyOutcome) -> Task<Message> {
        match outcome {
            KeyOutcome::Nothing => Task::none(),
            KeyOutcome::Close => {
                self.dialog = None;
                self.svgedit_close_request();
                Task::none()
            }
            KeyOutcome::Undo => {
                if let Some(ed) = &mut self.styles.svg_editor {
                    ed.undo();
                }
                Task::none()
            }
            KeyOutcome::Redo => {
                if let Some(ed) = &mut self.styles.svg_editor {
                    ed.redo();
                }
                Task::none()
            }
            KeyOutcome::File(cmd) => self.svgedit_file(files::Event::Cmd(cmd)),
        }
    }

    /// The editor's events.
    pub(crate) fn svgedit_event(&mut self, e: Event) -> Task<Message> {
        let shift = self.modifiers.shift();
        let (ink, paper) = self.svgedit_theme();
        let Some(ed) = self.styles.svg_editor.as_mut() else {
            return Task::none();
        };
        ed.shift_held = shift;
        *ed.theme.borrow_mut() = (ink, paper);
        let mut task = Task::none();
        match e {
            Event::Close => {
                self.dialog = None;
                self.svgedit_close_request();
                return Task::none();
            }
            Event::Stay => ed.question = None,
            Event::Discard => match ed.question.take() {
                Some(Question::Replace(pending)) => files::go_on(self, *pending),
                _ => self.close_svg_editor(),
            },
            Event::SaveAndClose => {
                let question = ed.question.take();
                if self.svgedit_save() {
                    match question {
                        Some(Question::Replace(pending)) => files::go_on(self, *pending),
                        _ => self.close_svg_editor(),
                    }
                }
            }
            Event::Save => {
                self.svgedit_save();
            }
            Event::Name(t) => {
                ed.name = t;
                ed.touch();
            }
            Event::Path(t) => {
                ed.path_text = t;
                ed.touch();
            }
            Event::Undo => ed.undo(),
            Event::Redo => ed.redo(),
            Event::Stage(input) => ed.pointer(input),
            Event::Tool(t) => ed.set_tool(t),
            Event::ZoomBy(f) => {
                ed.camera.zoom_by(f, None);
                ed.touch();
            }
            Event::Fit => {
                ed.camera.fit(&ed.doc, &ed.options);
                ed.touch();
            }
            Event::Do(Change(f)) => {
                f(ed);
                ed.touch();
            }
            Event::Settle(key) => {
                ed.typed.remove(&key);
                ed.settle();
            }
            Event::Key(press, focus) => {
                let outcome = ed.key(&press, &focus);
                task = self.svgedit_outcome(outcome);
            }
            Event::File(f) => task = self.svgedit_file(f),
        }
        let Some(ed) = self.styles.svg_editor.as_mut() else {
            return task;
        };
        ed.follow_source();
        if std::mem::take(&mut ed.pending_trace) {
            return Task::batch([task, files::trace::run(ed)]);
        }
        task
    }
}

/// A file dropped on the window, for the editor (the web's drop on its canvas).
pub fn dropped(
    event: iced::Event,
    _status: iced::event::Status,
    _window: iced::window::Id,
) -> Option<Message> {
    match event {
        iced::Event::Window(iced::window::Event::FileDropped(path)) => {
            Some(ev(Event::File(files::Event::Dropped(path))))
        }
        _ => None,
    }
}

/// The unsaved question's answers, for a window that asks to go on.
pub fn pending_words(q: &Question) -> (&'static str, &'static str) {
    match q {
        Question::Close => ("Pencere kapanırsa bu değişiklikler kaybolur.", "kapat"),
        Question::Replace(p) => (
            match **p {
                Pending::New | Pending::Asset(_) | Pending::Doc { .. } => {
                    "Başka bir çizim açılırsa bu değişiklikler kaybolur."
                }
            },
            "devam et",
        ),
    }
}
