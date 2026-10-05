//! Şablonlar (the web's `ui/templates/TemplatesPanel.ts`, docs/adr/0176 §4):
//! the style library's object templates in the dock's upper slot, a tab
//! beside Katmanlar, İşlemler and Bloklar. The templates are grouped by their
//! categories (`object_template::listed`, the web's `listTemplates`), each row
//! its picture, its name and what it draws where (“Kapalı alan · Kadastro /
//! Parsel”); the last used ones come first while nothing is searched, and the
//! one being drawn with is marked. A press draws with the template
//! (templates.rs); ↓ in the search gives the list the keys, ↑ ↓ choose and
//! Enter draws. The drawing then has the keyboard. A group's row opens and
//! closes it. The row's menu draws, shows the template in the Stil
//! yöneticisi, copies it to Kitaplığım or the project and deletes it from an
//! editable source, asked first: the library keeps no undo.

use std::collections::BTreeSet;

use iced::keyboard::key::Named;
use iced::widget::{Column, button, column, container, row, text, tooltip};
use iced::{Center, Element, Fill, Task, Theme};
use kentos_native_style::library::{ItemKind, Source};
use kentos_native_style::object_template::{listed, tool_label};
use kentos_ui::icon::{Icon, icon};
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::{ContextMenu, Menu, SearchBox, Tip, VirtualList, tip};
use kentos_ui::{label, style};
use serde_json::Value;

use crate::app::{App, Dialog, Message, Panel};
use crate::keys::KeyPress;
use crate::style::manager::details::symbol_of_item;
use crate::style::thumbs::Look;

/// The search box's id.
const SEARCH: &str = "sablon-ara";
/// A row's height and a picture's side, at the body text's size (the web's 40 and 30 px).
const ROW: f32 = 40.0;
const THUMB: f32 = 30.0;
/// How many last used templates the panel shows first.
pub(crate) const RECENT: usize = 5;
/// The recent group's key: no category path joins to it.
const RECENT_KEY: &str = "\u{1}son";

/// What the panel keeps while the app runs.
#[derive(Debug, Default)]
pub struct PanelState {
    query: String,
    /// The row the keys chose, by its key (`group\0id`).
    focused: Option<String>,
    /// Groups closed by hand, by key.
    closed: BTreeSet<String>,
    /// The template Sil asks about: its id and name (`Dialog::RemoveTemplate`).
    pub(crate) deleting: Option<(String, String)>,
    /// Whether the list has the keyboard (↓ in the search).
    pub(crate) keyboard: bool,
}

#[derive(Debug, Clone)]
pub enum Event {
    Search(String),
    /// ↓ in the search: the list takes the keys, its first template chosen.
    Down,
    /// A group's row: opens or closes it.
    Toggle(String),
    /// A template's row pressed: draws with it.
    Draw(String),
    /// Stil yöneticisinde göster.
    Show(String),
    /// Düzenle (a system template's copy): Şablon düzenleyici (template_editor.rs).
    Edit(String),
    Copy(String, Source),
    Remove(String),
}

fn msg(event: Event) -> Message {
    Message::TemplatesPanel(event)
}

/// A listed row: a group, or a template under one.
enum Line {
    Group {
        key: String,
        label: String,
        count: usize,
        open: bool,
    },
    Template {
        key: String,
        id: String,
    },
}

/// What a template draws where: “Kapalı alan · Kadastro / Parsel”.
pub(crate) fn template_where(template: &Value) -> String {
    let tool = template.get("tool").and_then(Value::as_str).unwrap_or("");
    let layer = template.get("layer");
    let mut parts: Vec<&str> = layer
        .and_then(|l| l.get("path"))
        .and_then(Value::as_array)
        .map(|p| p.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();
    if let Some(name) = layer.and_then(|l| l.get("name")).and_then(Value::as_str) {
        parts.push(name);
    }
    format!("{} · {}", tool_label(tool), parts.join(" / "))
}

/// Where an item lives, as the Stil yöneticisi says it.
fn source_text(source: Source) -> &'static str {
    match source {
        Source::System => "Sistem",
        Source::User => "Kitaplığım",
        Source::Project => "Proje",
    }
}

impl App {
    /// The rows as listed now: the last used templates while nothing is
    /// searched, then the groups the library's list gives, each followed by
    /// its templates when it is open.
    fn template_lines(&self) -> Vec<Line> {
        let lib = &self.styles.library;
        let state = &self.templates_panel;
        let mut lines = Vec::new();
        let mut push = |key: String, label: String, ids: Vec<String>| {
            let open = !state.closed.contains(&key);
            lines.push(Line::Group {
                key: key.clone(),
                label,
                count: ids.len(),
                open,
            });
            if open {
                lines.extend(ids.into_iter().map(|id| Line::Template {
                    key: format!("{key}\0{id}"),
                    id,
                }));
            }
        };
        if state.query.trim().is_empty() {
            let recent: Vec<String> = self
                .recent_templates
                .iter()
                .filter(|id| {
                    lib.get(id)
                        .is_some_and(|(item, _)| item.kind() == ItemKind::Template)
                })
                .cloned()
                .collect();
            if !recent.is_empty() {
                push(
                    RECENT_KEY.to_owned(),
                    "Son kullanılanlar".to_owned(),
                    recent,
                );
            }
        }
        for group in listed(lib, &state.query) {
            let key = group.path.join("\0");
            let label = if group.path.is_empty() {
                "Kategorisiz".to_owned()
            } else {
                group.path.join(" / ")
            };
            push(
                key,
                label,
                group.items.into_iter().map(|(id, _)| id).collect(),
            );
        }
        lines
    }

    /// `template.panel`: the dock shows the Şablonlar tab.
    pub(crate) fn show_templates_panel(&mut self) -> Task<Message> {
        if !self.right_panel_shown() {
            self.toggle_right_panel();
        }
        self.docks
            .update(kentos_ui::widget::docking::Event::Selected(
                Panel::Templates,
            ));
        iced::widget::operation::focus(SEARCH)
    }

    pub(crate) fn templates_panel_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Search(text) => {
                self.templates_panel.query = text;
                self.templates_panel.keyboard = false;
            }
            Event::Down => {
                let first = self.template_lines().into_iter().find_map(|l| match l {
                    Line::Template { key, .. } => Some(key),
                    Line::Group { .. } => None,
                });
                if let Some(key) = first {
                    self.templates_panel.focused = Some(key);
                    self.templates_panel.keyboard = true;
                    self.layers_keyboard = false;
                    self.blocks_panel.keyboard = false;
                    self.vertices.keyboard = false;
                    return crate::input::release_keyboard();
                }
            }
            Event::Toggle(key) => {
                if !self.templates_panel.closed.remove(&key) {
                    self.templates_panel.closed.insert(key);
                }
            }
            Event::Draw(id) => return self.template_pressed(&id),
            Event::Show(id) => return self.open_style_manager(None, Some(id)),
            Event::Edit(id) => return self.open_template_editor(Some(&id), None),
            Event::Copy(id, to) => self.copy_template(&id, to),
            Event::Remove(id) => self.ask_remove_template(&id),
        }
        Task::none()
    }

    /// A template's row pressed, or Enter on it: the drawing takes the keys.
    fn template_pressed(&mut self, id: &str) -> Task<Message> {
        self.templates_panel.keyboard = false;
        let task = self.draw_template(id);
        if self.session.is_running() {
            return Task::batch([task, crate::input::release_keyboard()]);
        }
        task
    }

    fn copy_template(&mut self, id: &str, to: Source) {
        if to == Source::Project && self.document.is_none() {
            return self
                .warn("Açık çizim yok: proje kitaplığına ancak bir çizim açıkken kopyalanır.");
        }
        let name = self
            .styles
            .library
            .get(id)
            .map_or_else(String::new, |(i, _)| i.name().to_owned());
        match self.styles.library.copy(id, to, None, None) {
            Ok(_) => {
                self.library_changed(to);
                let whose = if to == Source::User {
                    "Kitaplığım"
                } else {
                    "Proje"
                };
                self.output(format!("“{name}” {whose} kitaplığına kopyalandı."));
            }
            Err(e) => self.warn(e),
        }
    }

    /// Sil (and Delete on a row): asked first, the library keeping no undo.
    fn ask_remove_template(&mut self, id: &str) {
        let Some((item, source)) = self.styles.library.get(id) else {
            return;
        };
        if !source.editable() {
            return self
                .warn("Sistem şablonu silinmez; kopyasını Kitaplığım’a alıp onu düzenleyin.");
        }
        self.templates_panel.deleting = Some((id.to_owned(), item.name().to_owned()));
        self.dialog = Some(Dialog::RemoveTemplate);
    }

    /// The question's Sil: the template goes from its library.
    pub(crate) fn remove_template(&mut self) {
        let Some((id, name)) = self.templates_panel.deleting.take() else {
            return;
        };
        match self.styles.library.remove(&id) {
            Ok(source) => {
                self.library_changed(source);
                self.output(format!("“{name}” silindi."));
            }
            Err(e) => self.warn(e),
        }
    }

    /// The question Sil asks.
    pub(crate) fn remove_template_question(&self) -> Element<'_, Message> {
        let (id, name) = self.templates_panel.deleting.clone().unwrap_or_default();
        let whose = match self.styles.library.get(&id).map(|(_, s)| s) {
            Some(Source::Project) => "projenin kitaplığından",
            _ => "Kitaplığım’dan",
        };
        kentos_ui::widget::overlay::modal(
            kentos_ui::widget::Confirm::new(
                "Kitaplıktan sil",
                Message::DialogConfirmed,
                Message::DialogClosed,
            )
            .message(format!("“{name}” {whose} silinsin mi? Bu geri alınamaz."))
            .confirm("Sil")
            .destructive(),
            Message::DialogClosed,
        )
    }

    /// The list's keys while it has the keyboard: ↑ ↓ choose, Enter draws,
    /// Delete deletes, Esc gives the keyboard back to the drawing.
    pub(crate) fn templates_key(&mut self, press: &KeyPress) -> Option<Task<Message>> {
        let rows: Vec<(String, String)> = self
            .template_lines()
            .into_iter()
            .filter_map(|l| match l {
                Line::Template { key, id } => Some((key, id)),
                Line::Group { .. } => None,
            })
            .collect();
        if rows.is_empty() {
            return None;
        }
        let at = rows
            .iter()
            .position(|(key, _)| Some(key) == self.templates_panel.focused.as_ref());
        let mut step = |d: isize| {
            let n = rows.len() as isize;
            let i = at.map_or(0, |i| i as isize + d).clamp(0, n - 1);
            self.templates_panel.focused = rows.get(i as usize).map(|(key, _)| key.clone());
        };
        let id = at.map(|i| rows[i].1.clone());
        match press.named()? {
            Named::ArrowDown => step(1),
            Named::ArrowUp => step(-1),
            Named::Enter => return id.map(|id| self.template_pressed(&id)),
            Named::Delete => {
                if let Some(id) = id {
                    self.ask_remove_template(&id);
                }
            }
            Named::Escape => self.templates_panel.keyboard = false,
            _ => return None,
        }
        Some(Task::none())
    }

    /// The header's meta: how many templates the library has (the head holds the dock's tabs).
    pub(crate) fn templates_panel_actions(&self) -> Element<'_, Message> {
        let count = self
            .styles
            .library
            .items(None)
            .iter()
            .filter(|(item, _)| item.kind() == ItemKind::Template)
            .count();
        let meta = if count > 0 {
            format!("{count} şablon")
        } else {
            String::new()
        };
        label::caption(meta).into()
    }

    /// The panel's body: the search with Stil yöneticisi beside it, and the list.
    pub(crate) fn templates_panel_view(&self) -> Element<'_, Message> {
        let search = SearchBox::new(self.templates_panel.query.clone(), "Şablon ara", |t| {
            msg(Event::Search(t))
        })
        .id(SEARCH)
        .on_down(msg(Event::Down))
        .fill()
        .height(28.0);
        // Yeni şablon, Seçili nesneden şablon and Stil yöneticisi beside the search (the web's toolbar).
        let search = container(
            row![
                search,
                self.templates_button("template.new"),
                self.templates_button("template.fromSelection"),
                self.templates_button("style.manager"),
            ]
            .spacing(2)
            .align_y(Center),
        )
        .padding([6, 8])
        .width(Fill);
        let lines = self.template_lines();
        if lines.is_empty() {
            let words = if self.templates_panel.query.trim().is_empty() {
                "Kitaplıkta nesne şablonu yok. Şablonlar Stil yöneticisinde ya da bir .kstil dosyasıyla gelir."
            } else {
                "Aramayla eşleşen şablon yok. Başka bir ad deneyin."
            };
            return column![
                search,
                container(label::caption(words))
                    .padding([8, 12])
                    .width(Fill)
            ]
            .into();
        }
        let focused = self.templates_panel.focused.clone();
        let reveal = lines.iter().position(|l| match l {
            Line::Template { key, .. } => Some(key) == focused.as_ref(),
            Line::Group { .. } => false,
        });
        let drawing = self.template.as_ref().map(|run| run.id.clone());
        let palette = self.style_palette();
        let list = VirtualList::new(
            lines.len(),
            typography::scaled(ROW),
            move |i| match &lines[i] {
                Line::Group {
                    key,
                    label,
                    count,
                    open,
                } => group_row(key, label, *count, *open),
                Line::Template { key, id } => {
                    let look = Look {
                        palette: &palette,
                        library: &self.styles.library,
                        images: &self.styles.images,
                    };
                    self.template_row(
                        id,
                        &look,
                        drawing.as_deref() == Some(id.as_str()),
                        focused.as_deref() == Some(key.as_str()),
                    )
                }
            },
        )
        .reveal(reveal)
        .height(Fill);
        column![search, list].into()
    }

    /// A button beside the search: the command's icon, its title in the tip.
    fn templates_button(&self, id: &'static str) -> Element<'_, Message> {
        let command = crate::catalog::catalog().get(id);
        let title = command.map_or(id, |c| c.title);
        let glyph = command.map_or(Icon::Button, |c| c.icon);
        tip(
            button(icon(glyph).size(16.0))
                .on_press_maybe(self.available(id).then_some(Message::Run(id)))
                .padding([4, 5])
                .style(style::button::ghost),
            Tip::new(title.to_owned()),
            tooltip::Position::Bottom,
        )
    }

    /// One template: the picture, the name over what it draws where.
    fn template_row<'a>(
        &'a self,
        id: &str,
        look: &Look<'_>,
        drawing: bool,
        focused: bool,
    ) -> Element<'a, Message> {
        let lib = &self.styles.library;
        let Some((item, source)) = lib.get(id) else {
            return iced::widget::space().into();
        };
        let side = typography::scaled(THUMB);
        let symbol = symbol_of_item(item, lib);
        let picture =
            container(
                self.styles
                    .thumbs
                    .picture(&symbol, None, (side, side), None, look),
            )
            .padding(1)
            .style(move |theme: &Theme| thumb_tile(theme, drawing));
        let font = if drawing {
            typography::ui_strong()
        } else {
            typography::ui()
        };
        let template = item.template().cloned().unwrap_or(Value::Null);
        let where_ = template_where(&template);
        let words = Column::new()
            .spacing(1)
            .width(Fill)
            .push(
                text(item.name().to_owned())
                    .font(font)
                    .size(typography::body())
                    .wrapping(text::Wrapping::None),
            )
            .push(
                label::caption(where_.clone())
                    .wrapping(text::Wrapping::None)
                    .style(|t: &Theme| text::Style {
                        color: Some(Tokens::of(t).faint),
                    }),
            );
        let face = row![picture, words]
            .spacing(8)
            .padding([0, 8])
            .align_y(Center)
            .height(Fill);
        let owned = id.to_owned();
        let face = button(face)
            .on_press(msg(Event::Draw(owned.clone())))
            .padding(0)
            .width(Fill)
            .height(Fill)
            .style(style::button::row(focused));
        let body = [
            item.text("description").map(str::to_owned),
            Some(where_),
            Some(source_text(source).to_owned()),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join("\n");
        let face = tip(
            face,
            Tip::new(item.name().to_owned()).body(body),
            tooltip::Position::Left,
        );
        ContextMenu::new(face, move |_| self.template_menu(&owned)).into()
    }

    /// The row's menu (the web's `menuFor`).
    fn template_menu(&self, id: &str) -> Menu<Message> {
        let editable = self.styles.library.can_edit(id);
        let project_open = self.document.is_some();
        let owned = id.to_owned();
        let mut menu = Menu::new()
            .item("Şablonla çiz", msg(Event::Draw(owned.clone())))
            .icon(crate::icons::from_web(Some("templateDraw")))
            .shortcut("Enter")
            .item(
                if editable {
                    "Düzenle…"
                } else {
                    "Kopyasını düzenle…"
                },
                msg(Event::Edit(owned.clone())),
            )
            .icon(crate::icons::from_web(Some("edit")));
        if !editable {
            menu = menu.detail("Sistem şablonu değişmez; kopyası Kitaplığım’a kaydedilir.");
        }
        menu = menu
            .item("Stil yöneticisinde göster", msg(Event::Show(owned.clone())))
            .icon(crate::icons::from_web(Some("styles")))
            .separator()
            .item(
                "Kitaplığıma kopyala",
                msg(Event::Copy(owned.clone(), Source::User)),
            )
            .icon(crate::icons::from_web(Some("copy")))
            .detail("Bu bilgisayarda, bütün çizimlerde")
            .item(
                "Projeye kopyala",
                project_open.then(|| msg(Event::Copy(owned.clone(), Source::Project))),
            )
            .icon(crate::icons::from_web(Some("save")))
            .detail(if project_open {
                "Proje dosyasında; projeyi açan herkes görür"
            } else {
                "Açık çizim yok."
            })
            .separator()
            .item("Sil", editable.then(|| msg(Event::Remove(owned))))
            .icon(crate::icons::from_web(Some("trash")))
            .shortcut("Delete");
        if !editable {
            menu = menu.detail("Sistem şablonu silinmez; kopyası düzenlenir.");
        }
        menu
    }
}

/// A group's row: its caret, its category and how many templates it has; a press opens or closes it.
fn group_row<'a>(key: &str, label: &str, count: usize, open: bool) -> Element<'a, Message> {
    let caret = icon(if open {
        Icon::ChevronDown
    } else {
        Icon::ChevronRight
    })
    .size(12.0);
    let face = row![
        caret,
        text(label.to_owned())
            .font(typography::ui_strong())
            .size(typography::body())
            .width(Fill)
            .wrapping(text::Wrapping::None)
            .style(|t: &Theme| text::Style {
                color: Some(Tokens::of(t).muted),
            }),
        label::caption(count.to_string()),
    ]
    .spacing(6)
    .padding([0, 8])
    .align_y(Center)
    .height(Fill);
    button(face)
        .on_press(msg(Event::Toggle(key.to_owned())))
        .padding(0)
        .width(Fill)
        .height(Fill)
        .style(style::button::row(false))
        .into()
}

/// The picture's tile: its edge the accent on the template being drawn with (Bloklar's).
fn thumb_tile(theme: &Theme, drawing: bool) -> container::Style {
    let t = Tokens::of(theme);
    container::Style {
        border: iced::Border {
            color: if drawing { t.accent } else { t.border },
            width: 1.0,
            radius: style::button::radius().into(),
        },
        ..container::Style::default()
    }
}

#[cfg(test)]
mod tests {
    //! The Şablonlar panel as the user works it: its rows, the search, a
    //! group closed, a press that draws, the recent ones, the keys, a copy
    //! and Sil asked first.

    use iced::keyboard::key::{Named, NativeCode, Physical};
    use iced::keyboard::{Key, Modifiers};
    use kentos_contracts::ProjectStyles;
    use kentos_native_style::library::Source;
    use serde_json::json;

    use super::{Event, Line};
    use crate::app::{App, Dialog, Message};
    use crate::files_testing::app_with_drawing;
    use crate::keys::KeyPress;

    /// The sample drawing with three project templates: two in Kadastro, one without a category.
    fn app() -> App {
        let mut app = app_with_drawing();
        let template = |id: &str, name: &str, path: &[&str], template: serde_json::Value| json!({ "kind": "template", "id": id, "name": name, "path": path, "template": template });
        let styles = ProjectStyles {
            items: vec![
                template(
                    "p-parsel",
                    "Parsel sınırı",
                    &["Kadastro"],
                    json!({ "tool": "polygon", "layer": { "path": ["Kadastro"], "name": "Parsel" } }),
                ),
                template(
                    "p-ada",
                    "Ada sınırı",
                    &["Kadastro"],
                    json!({ "tool": "polyline", "layer": { "path": ["Kadastro"], "name": "Ada" } }),
                ),
                template(
                    "p-nokta",
                    "Poligon noktası",
                    &[],
                    json!({ "tool": "point", "layer": { "path": [], "name": "Nokta" } }),
                ),
            ],
            categories: vec![json!({ "path": ["Kadastro"] })],
        };
        app.document
            .as_mut()
            .expect("a drawing")
            .model
            .set_styles(styles);
        let _ = app.update(Message::Swallowed);
        app
    }

    fn send(app: &mut App, event: Event) {
        let _ = app.update(Message::TemplatesPanel(event));
    }

    /// The rows as words: “» group (n)” and the templates' ids.
    fn rows(app: &App) -> Vec<String> {
        app.template_lines()
            .into_iter()
            .map(|l| match l {
                Line::Group { label, count, .. } => format!("» {label} ({count})"),
                Line::Template { id, .. } => id,
            })
            .collect()
    }

    #[test]
    fn the_rows_are_the_groups_with_their_templates_and_the_search_finds_names() {
        let mut app = app();
        assert_eq!(
            rows(&app),
            [
                "» Kadastro (2)",
                "p-ada",
                "p-parsel",
                "» Kategorisiz (1)",
                "p-nokta"
            ]
        );
        send(&mut app, Event::Search("POLİGON".into()));
        assert_eq!(rows(&app), ["» Kategorisiz (1)", "p-nokta"]);
        // The tool's name is found too: Çoklu çizgi.
        send(&mut app, Event::Search("çoklu".into()));
        assert_eq!(rows(&app), ["» Kadastro (1)", "p-ada"]);
        send(&mut app, Event::Search(String::new()));
        // A group's row closes and opens it.
        send(&mut app, Event::Toggle("Kadastro".into()));
        assert_eq!(
            rows(&app),
            ["» Kadastro (2)", "» Kategorisiz (1)", "p-nokta"]
        );
        send(&mut app, Event::Toggle("Kadastro".into()));
        assert_eq!(rows(&app).len(), 5);
    }

    #[test]
    fn a_press_draws_and_the_templates_drawn_with_come_first() {
        let mut app = app();
        send(&mut app, Event::Draw("p-parsel".into()));
        assert_eq!(app.session.tool_id(), "polygon");
        assert_eq!(
            app.template.as_ref().map(|r| r.id.as_str()),
            Some("p-parsel")
        );
        let _ = app.update(Message::Run("tool.cancel"));
        send(&mut app, Event::Draw("p-nokta".into()));
        let _ = app.update(Message::Run("tool.cancel"));
        assert_eq!(
            rows(&app)[..3],
            ["» Son kullanılanlar (2)", "p-nokta", "p-parsel"]
        );
        // While searching, not (“ada” alone is in Kadastro too).
        send(&mut app, Event::Search("ada sın".into()));
        assert_eq!(rows(&app), ["» Kadastro (1)", "p-ada"]);
    }

    /// A key pressed as the window sends it.
    fn key(app: &mut App, named: Named) {
        let _ = app.update(Message::Key(KeyPress {
            key: Key::Named(named),
            physical: Physical::Unidentified(NativeCode::Unidentified),
            modifiers: Modifiers::default(),
            text: None,
            repeat: false,
        }));
    }

    #[test]
    fn down_in_the_search_gives_the_keys_to_the_list_and_enter_draws() {
        let mut app = app();
        send(&mut app, Event::Down);
        assert!(app.templates_panel.keyboard);
        key(&mut app, Named::ArrowDown);
        key(&mut app, Named::Enter);
        // Ada sınırı was first, Parsel sınırı next.
        assert_eq!(
            app.template.as_ref().map(|r| r.id.as_str()),
            Some("p-parsel")
        );
        assert!(!app.templates_panel.keyboard);
    }

    #[test]
    fn a_copy_goes_to_kitapligim_and_sil_asks_first() {
        let mut app = app();
        let user_before = app.styles.library.items(Some(Source::User)).len();
        send(&mut app, Event::Copy("p-ada".into(), Source::User));
        assert_eq!(
            app.styles.library.items(Some(Source::User)).len(),
            user_before + 1
        );
        send(&mut app, Event::Remove("p-ada".into()));
        assert_eq!(app.dialog, Some(Dialog::RemoveTemplate));
        assert!(app.styles.library.get("p-ada").is_some(), "asked, not gone");
        let _ = app.update(Message::DialogConfirmed);
        assert!(app.styles.library.get("p-ada").is_none());
        // The drawing's own part follows: the template is gone from the project file too.
        let doc = &app.document.as_ref().expect("a drawing").model;
        assert!(!doc.styles().items.iter().any(|i| i["id"] == "p-ada"));
    }
}
