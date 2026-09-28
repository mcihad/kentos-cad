//! The script editor (docs/adr/0136): the Python tab's Betik side. A
//! script in coloured code, its file (Yeni, Aç, Kaydet, Farklı kaydet), run
//! whole (F5) or its selection (Ctrl+Enter) in the console's Python, whose
//! output is beside it; a failed run puts the cursor on its line.
//!
//! What is typed is kept as a draft beside the program's other history
//! (`$XDG_STATE_HOME/kentos-cad/python-betik.json`), so an unsaved script is
//! there after a restart; Yeni and Aç over unsaved work ask first.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use iced::keyboard::Key;
use iced::keyboard::key::Named;
use iced::widget::text_editor::{self, Action, Binding, Cursor, Edit, KeyPress, Position, Status};
use iced::widget::{Column, button, container, row, text, text_editor as editor};
use iced::{Element, Fill, Length, Padding, Task, Theme};
use kentos_ui::icon::{Icon, icon};
use kentos_ui::style;
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::{Tip, tip};
use serde::{Deserialize, Serialize};

use super::{Kind, assist, code};
use crate::app::{App, Message};

const DRAFT: &str = "python-betik.json";
const FORMAT: &str = "kentos.python-draft";

/// A question over unsaved work: what waits for the answer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pending {
    New,
    Open,
}

/// The script being written.
#[derive(Default)]
pub struct Script {
    pub content: text_editor::Content,
    pub path: Option<PathBuf>,
    pub dirty: bool,
    pub asking: Option<Pending>,
    /// Where the draft is kept; none in tests and without a state folder.
    folder: Option<PathBuf>,
}

#[derive(Serialize, Deserialize)]
struct Draft {
    format: String,
    version: u32,
    path: Option<PathBuf>,
    text: String,
    dirty: bool,
}

impl Script {
    /// The draft kept in `folder`; an unreadable one is an empty script.
    pub fn load(folder: &Path) -> Self {
        let draft = std::fs::read_to_string(folder.join(DRAFT))
            .ok()
            .and_then(|t| serde_json::from_str::<Draft>(&t).ok())
            .filter(|d| d.format == FORMAT && d.version == 1);
        let mut script = Self {
            folder: Some(folder.to_path_buf()),
            ..Self::default()
        };
        if let Some(d) = draft {
            script.content = text_editor::Content::with_text(&d.text);
            script.path = d.path;
            script.dirty = d.dirty;
        }
        script
    }

    /// Writes the draft (never a reason to stop: the file is a convenience).
    fn keep(&self) {
        let Some(folder) = &self.folder else {
            return;
        };
        let draft = Draft {
            format: FORMAT.to_owned(),
            version: 1,
            path: self.path.clone(),
            text: self.content.text(),
            dirty: self.dirty,
        };
        if let Ok(json) = serde_json::to_string(&draft) {
            let _ = std::fs::create_dir_all(folder);
            let _ = std::fs::write(folder.join(DRAFT), json);
        }
    }

    /// The name tracebacks and the tab show: the file, or “<betik>”.
    pub fn run_name(&self) -> String {
        self.path
            .as_ref()
            .map_or_else(|| "<betik>".to_owned(), |p| p.display().to_string())
    }

    pub fn title(&self) -> String {
        self.path.as_ref().and_then(|p| p.file_name()).map_or_else(
            || "Adsız betik".to_owned(),
            |n| n.to_string_lossy().into_owned(),
        )
    }

    /// Puts the cursor on `line` (from 1), the line selected.
    pub fn show_line(&mut self, line: usize) {
        let code = self.content.text();
        let Some(text) = code.split('\n').nth(line.saturating_sub(1)) else {
            return;
        };
        self.content.move_to(Cursor {
            position: Position {
                line: line - 1,
                column: text.len(),
            },
            selection: Some(Position {
                line: line - 1,
                column: text.len() - text.trim_start().len(),
            }),
        });
    }
}

/// What the Betik side asked for.
#[derive(Debug, Clone)]
pub enum Event {
    Edit(text_editor::Action),
    /// Enter: a new line indented as this one, and more after a `:`.
    Newline,
    Indent,
    New,
    Open,
    Opened(Option<PathBuf>),
    Save,
    SaveAs,
    SavedAs(Option<PathBuf>),
    RunAll,
    RunSelection,
    /// The answer to the question over unsaved work.
    KeepAndGo,
    DropAndGo,
    Cancel,
}

fn ev(e: Event) -> Message {
    Message::Python(super::Event::Script(e))
}

/// The indentation a new line after the cursor's line takes.
pub(super) fn indentation(code: &str, cursor: usize) -> String {
    let before: String = code.chars().take(cursor).collect();
    let line = before.rsplit('\n').next().unwrap_or_default();
    let mut indent: String = line
        .chars()
        .take_while(|c| *c == ' ' || *c == '\t')
        .collect();
    if line.trim_end().ends_with(':') {
        indent.push_str("    ");
    }
    indent
}

/// The last line of `error` that names `name`: where a failed run stopped in the script.
pub(super) fn failed_line(error: &str, name: &str) -> Option<usize> {
    let mark = format!("File \"{name}\", line ");
    error.match_indices(&mark).last().and_then(|(at, _)| {
        error[at + mark.len()..]
            .chars()
            .take_while(char::is_ascii_digit)
            .collect::<String>()
            .parse()
            .ok()
    })
}

impl App {
    pub(crate) fn script_event(&mut self, event: Event) -> Task<Message> {
        let s = &mut self.python.script;
        match event {
            Event::Edit(action) => {
                let writes = matches!(action, Action::Edit(_));
                s.content.perform(action);
                if writes {
                    s.dirty = true;
                    s.keep();
                }
                Task::none()
            }
            Event::Newline => {
                let (code, cursor) = assist::caret(&s.content);
                let text = format!("\n{}", indentation(&code, cursor));
                s.content.perform(Action::Edit(Edit::Paste(Arc::new(text))));
                s.dirty = true;
                s.keep();
                Task::none()
            }
            Event::Indent => {
                s.content
                    .perform(Action::Edit(Edit::Paste(Arc::new("    ".to_owned()))));
                s.dirty = true;
                s.keep();
                Task::none()
            }
            Event::New if s.dirty => {
                s.asking = Some(Pending::New);
                Task::none()
            }
            Event::New => {
                *s = Script {
                    folder: s.folder.take(),
                    ..Script::default()
                };
                s.keep();
                Task::none()
            }
            Event::Open if s.dirty => {
                s.asking = Some(Pending::Open);
                Task::none()
            }
            Event::Open => Task::perform(
                async {
                    let file = rfd::AsyncFileDialog::new()
                        .set_title("Python betiği aç")
                        .add_filter("Python betiği (.py)", &["py"])
                        .pick_file()
                        .await?;
                    Some(file.path().to_path_buf())
                },
                |path| ev(Event::Opened(path)),
            ),
            Event::Opened(None) => Task::none(),
            Event::Opened(Some(path)) => {
                match std::fs::read_to_string(&path) {
                    Ok(text) => {
                        s.content = text_editor::Content::with_text(&text);
                        s.path = Some(path);
                        s.dirty = false;
                        s.keep();
                    }
                    Err(e) => self
                        .python
                        .push(Kind::Error, format!("{} okunamadı: {e}", path.display())),
                }
                Task::none()
            }
            Event::Save => match s.path.clone() {
                Some(path) => {
                    self.script_write(&path);
                    Task::none()
                }
                None => self.script_event(Event::SaveAs),
            },
            Event::SaveAs => {
                let name = s.title();
                let name = if name.ends_with(".py") {
                    name
                } else {
                    "betik.py".to_owned()
                };
                Task::perform(
                    async move {
                        let file = rfd::AsyncFileDialog::new()
                            .set_title("Python betiğini kaydet")
                            .add_filter("Python betiği (.py)", &["py"])
                            .set_file_name(name)
                            .save_file()
                            .await?;
                        Some(file.path().to_path_buf())
                    },
                    |path| ev(Event::SavedAs(path)),
                )
            }
            Event::SavedAs(None) => {
                // A question's “Kaydet” that was not saved after all: nothing goes on.
                self.python.script.asking = None;
                Task::none()
            }
            Event::SavedAs(Some(path)) => {
                if self.script_write(&path) {
                    return self.script_go_on();
                }
                Task::none()
            }
            Event::RunAll => {
                let (code, name) = (s.content.text(), s.run_name());
                self.python_run(code, Some(name))
            }
            Event::RunSelection => {
                let chosen = s.content.selection().filter(|t| !t.trim().is_empty());
                let code = chosen.unwrap_or_else(|| {
                    let (code, cursor) = assist::caret(&s.content);
                    let before: String = code.chars().take(cursor).collect();
                    let line = before.split('\n').count() - 1;
                    code.split('\n').nth(line).unwrap_or_default().to_owned()
                });
                // A selection runs as its own code: its lines count from its first.
                self.python_run(dedent(&code), Some("<betik seçimi>".to_owned()))
            }
            Event::KeepAndGo => match s.path.clone() {
                Some(path) => {
                    if self.script_write(&path) {
                        return self.script_go_on();
                    }
                    Task::none()
                }
                None => self.script_event(Event::SaveAs),
            },
            Event::DropAndGo => {
                s.dirty = false;
                self.script_go_on()
            }
            Event::Cancel => {
                s.asking = None;
                Task::none()
            }
        }
    }

    /// Writes the script to `path`; whether it was written.
    fn script_write(&mut self, path: &Path) -> bool {
        let text = self.python.script.content.text();
        match std::fs::write(path, text.as_bytes()) {
            Ok(()) => {
                let s = &mut self.python.script;
                s.path = Some(path.to_path_buf());
                s.dirty = false;
                s.keep();
                true
            }
            Err(e) => {
                self.python.push(
                    Kind::Error,
                    format!(
                        "{} yazılamadı: {e}. Klasörün yazılabilir olduğunu denetleyin.",
                        path.display()
                    ),
                );
                false
            }
        }
    }

    /// The question answered: Yeni or Aç goes on.
    fn script_go_on(&mut self) -> Task<Message> {
        match self.python.script.asking.take() {
            Some(Pending::New) => self.script_event(Event::New),
            Some(Pending::Open) => self.script_event(Event::Open),
            None => Task::none(),
        }
    }

    /// The Betik side: the script and its bar on the left, the output on the right.
    pub(crate) fn python_script_view<'a>(
        &'a self,
        mode_switch: Element<'a, Message>,
        output: Element<'a, Message>,
    ) -> Element<'a, Message> {
        let s = &self.python.script;
        let running = self.python.running.is_some();
        let title = format!("{}{}", s.title(), if s.dirty { " •" } else { "" });
        let small = |glyph: Icon, about: Tip, message: Option<Message>| -> Element<'a, Message> {
            tip(
                button(icon(glyph).size(15.0))
                    .on_press_maybe(message)
                    .padding([5, 6])
                    .style(style::button::flat),
                about,
                iced::widget::tooltip::Position::Top,
            )
        };
        let run = if running {
            button(
                row![
                    icon(Icon::Stop).size(14.0),
                    text("Durdur").size(typography::body())
                ]
                .spacing(6)
                .align_y(iced::Center),
            )
            .on_press(Message::Python(super::Event::Stop))
            .padding([5, 12])
            .style(style::button::danger)
        } else {
            button(
                row![
                    icon(Icon::Play).size(14.0),
                    text("Çalıştır").size(typography::body())
                ]
                .spacing(6)
                .align_y(iced::Center),
            )
            .on_press(ev(Event::RunAll))
            .padding([5, 12])
            .style(style::button::primary)
        };
        let bar = row![
            mode_switch,
            text(title)
                .size(typography::body())
                .style(|theme: &Theme| text::Style {
                    color: Some(Tokens::of(theme).muted),
                }),
            iced::widget::space::horizontal(),
            small(
                Icon::DocumentNew,
                Tip::new("Yeni betik"),
                (!running).then_some(ev(Event::New))
            ),
            small(
                Icon::Open,
                Tip::new("Aç…").detail(".py"),
                (!running).then_some(ev(Event::Open))
            ),
            small(
                Icon::Save,
                Tip::new("Kaydet").detail("Ctrl+S"),
                Some(ev(Event::Save))
            ),
            small(
                Icon::SaveAs,
                Tip::new("Farklı kaydet…").detail("Ctrl+Shift+S"),
                Some(ev(Event::SaveAs))
            ),
            small(
                Icon::Play,
                Tip::new("Seçimi çalıştır").detail("Ctrl+Enter · seçim yoksa imlecin satırı"),
                (!running).then_some(ev(Event::RunSelection)),
            ),
            tip(
                run,
                if running {
                    Tip::new("Durdur").detail("Python'u kapatır; yazdıkları geri alınır")
                } else {
                    Tip::new("Çalıştır").detail("F5 · betiğin tamamı")
                },
                iced::widget::tooltip::Position::Top,
            ),
        ]
        .spacing(4)
        .padding([4, 12])
        .align_y(iced::Center);
        let mut left = Column::new().push(bar);
        if let Some(pending) = s.asking {
            let what = match pending {
                Pending::New => "Yeni betik",
                Pending::Open => "Başka bir betik açmak",
            };
            let question = row![
                text(format!(
                    "{} kaydedilmemiş. {what} için önce kaydedilsin mi?",
                    s.title()
                ))
                .size(typography::body())
                .width(Fill),
                button(text("Kaydet").size(typography::body()))
                    .on_press(ev(Event::KeepAndGo))
                    .padding([4, 10])
                    .style(style::button::primary),
                button(text("Kaydetme").size(typography::body()))
                    .on_press(ev(Event::DropAndGo))
                    .padding([4, 10])
                    .style(style::button::secondary),
                button(text("Vazgeç").size(typography::body()))
                    .on_press(ev(Event::Cancel))
                    .padding([4, 10])
                    .style(style::button::flat),
            ]
            .spacing(6)
            .align_y(iced::Center);
            left = left.push(
                container(question)
                    .padding([6, 12])
                    .width(Fill)
                    .style(style::container::popover),
            );
        }
        let area = editor(&s.content)
            .placeholder("# Python betiği: doc açık çizim, cad kentos.cad")
            .on_action(|a| ev(Event::Edit(a)))
            .font(typography::mono())
            .size(typography::body())
            .height(Fill)
            .padding(Padding::from([8, 12]))
            .highlight_with::<code::Python>((), code::format)
            .key_binding(keys)
            .style(style::field::text_area);
        left = left.push(
            container(area)
                .padding(Padding::new(0.0).left(12.0).right(6.0).bottom(8.0))
                .height(Fill),
        );
        row![
            left.width(Length::FillPortion(3)),
            kentos_ui::widget::vertical_divider(),
            container(output).width(Length::FillPortion(2)).height(Fill),
        ]
        .height(Fill)
        .into()
    }
}

/// A selection's lines without the indentation they share.
fn dedent(code: &str) -> String {
    let shared = code
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.len() - l.trim_start().len())
        .min()
        .unwrap_or(0);
    code.lines()
        .map(|l| l.get(shared..).unwrap_or(l.trim_start()))
        .collect::<Vec<_>>()
        .join("\n")
}

fn keys(kp: KeyPress) -> Option<Binding<Message>> {
    if !matches!(kp.status, Status::Focused { .. }) {
        return None;
    }
    let custom = |e: Event| Some(Binding::Custom(ev(e)));
    let control = kp.modifiers.control();
    let shift = kp.modifiers.shift();
    match kp.key.as_ref() {
        Key::Named(Named::F5) => custom(Event::RunAll),
        Key::Named(Named::Enter) if control => custom(Event::RunSelection),
        Key::Named(Named::Enter) if !shift && !kp.modifiers.alt() => custom(Event::Newline),
        Key::Named(Named::Tab) if !shift => custom(Event::Indent),
        Key::Character(c) if control && c.eq_ignore_ascii_case("s") => {
            custom(if shift { Event::SaveAs } else { Event::Save })
        }
        _ => Binding::from_key_press(kp),
    }
}

#[cfg(test)]
mod tests {
    use super::{dedent, failed_line, indentation};

    #[test]
    fn a_new_line_keeps_the_indentation_and_adds_after_a_colon() {
        assert_eq!(indentation("x = 1", 5), "");
        assert_eq!(indentation("for p in pts:", 13), "    ");
        assert_eq!(indentation("def f():\n    if a:", 18), "        ");
        assert_eq!(indentation("def f():\n    y = 2", 18), "    ");
    }

    #[test]
    fn a_failed_run_names_its_line_in_the_script() {
        let error = "Traceback (most recent call last):\n  File \"<betik>\", line 3, in <module>\n    f()\n  File \"<betik>\", line 7, in f\nValueError: x\n";
        assert_eq!(failed_line(error, "<betik>"), Some(7));
        assert_eq!(failed_line(error, "/tmp/a.py"), None);
    }

    #[test]
    fn a_selection_loses_its_shared_indentation() {
        assert_eq!(
            dedent("    a = 1\n    if a:\n        b = 2"),
            "a = 1\nif a:\n    b = 2"
        );
        assert_eq!(dedent("x"), "x");
    }
}
