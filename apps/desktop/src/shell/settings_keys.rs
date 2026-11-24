//! Uygulama ayarları › Kısayollar (docs/adr/9308): the commands on the left, searched by their title, key or name,
//! each with its keys and where they come from (KentOS's own or the user's); the chosen command's form on the right:
//! its keys (Tuş ekle waits for a key to be pressed, × takes one away), KentOS's names and the user's (Ekle), and
//! Varsayılana dön. A key or a name another command has is said with that command, and moved only when the user says
//! Taşı. Hepsini varsayılana döndür empties both. Everything goes into the window's draft: Kaydet puts it in use at
//! once, Vazgeç leaves it as it was (settings_view.rs). The chosen command's form is details.rs's.

use iced::widget::{Column, button, row, space};
use iced::{Center, Element, Task};
use kentos_commands::{ALIASES_SETTING, SHORTCUTS_SETTING};
use kentos_launch::Module;
use kentos_ui::widget::app_menu::Item;
use kentos_ui::widget::list_details::ListDetails;
use kentos_ui::widget::{Banner, Severity};
use kentos_ui::{label, style};
use serde_json::Value;

use crate::app::{App, Message};
use crate::catalog::{Command, catalog};
use crate::exchange::apply::fold_turkish;
use crate::keymap::{KeyCheck, Keymap, NameCheck, Source, not_bindable};
use crate::keys::KeyPress;
use crate::settings_view::{Edit, SettingsDraft};

/// The search field.
pub const SEARCH: &str = "settings-keys-search";
/// The new name's field.
pub const NAME: &str = "settings-keys-name";

/// The most commands the list shows: a search narrows the rest.
const LIST_MAX: usize = 150;

/// The section's own state in the window's draft.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct KeysPage {
    pub query: String,
    pub chosen: Option<&'static str>,
    /// Tuş ekle was pressed: the next key pressed goes to the chosen command.
    pub recording: bool,
    /// A key or a name another command has, waiting for Taşı or Vazgeç.
    pub pending: Option<Pending>,
    /// What the last step said.
    pub note: Option<(Severity, String)>,
    /// The new name being typed.
    pub name: String,
}

/// A key or a name another command has.
#[derive(Debug, Clone, PartialEq)]
pub enum Pending {
    Key { chord: String, from: &'static str },
    Name { name: String, from: &'static str },
}

/// What the section asks.
#[derive(Debug, Clone)]
pub enum KeysEdit {
    /// Opens Uygulama ayarları on this section (Klavye kısayolları' Kısayolları değiştir…).
    Open,
    Search(String),
    Choose(&'static str),
    /// Tuş ekle: the next key pressed is the chosen command's.
    Record,
    StopRecording,
    /// A key pressed while recording, as the input routing writes it.
    Recorded(String),
    RemoveKey(String),
    /// Taşı: the pending key or name goes to the chosen command.
    Move,
    /// Vazgeç: the pending key or name stays where it is.
    Keep,
    NameTyped(String),
    AddName,
    RemoveName(String),
    /// Varsayılana dön: the chosen command's keys and names are KentOS's again.
    CommandDefaults,
    /// Hepsini varsayılana döndür.
    AllDefaults,
}

fn message(edit: KeysEdit) -> Message {
    Message::Settings(Edit::Keys(edit))
}

/// The keymap of the window's draft: what Kaydet would put in use.
pub(crate) fn draft_keymap(draft: &SettingsDraft) -> Keymap {
    let value = |key: &str| draft.values.get(key).cloned().unwrap_or_else(|| Value::Object(Default::default()));
    Keymap::new(&value(SHORTCUTS_SETTING), &value(ALIASES_SETTING))
}

/// A key as menus show it (⌘ on macOS).
fn shown(chord: &str) -> String {
    crate::shortcuts::chord(chord)
}

/// The programs that do not offer a command, in a sentence (“KentOS Harita ve KentOS GIS'te”); none when every
/// program does.
fn programs_without(id: &str) -> Option<String> {
    let without: Vec<Module> = Module::ALL.into_iter().filter(|m| !catalog().module_has(*m, id)).collect();
    let (last, rest) = without.split_last()?;
    let rest: Vec<&str> = rest.iter().map(|m| m.title()).collect();
    Some(if rest.is_empty() {
        last.in_program().to_owned()
    } else {
        format!("{} ve {}", rest.join(", "), last.in_program())
    })
}

impl App {
    /// The commands the section lists for the search, the user's first, and how many it found in all: the commands
    /// this program offers that take keys (the drawing's, which the desktop runs); with nothing typed, those with a
    /// key or a name and those the user changed.
    pub(crate) fn keys_listed(&self, map: &Keymap, query: &str) -> (Vec<&'static Command>, usize) {
        let q = fold_turkish(query.trim());
        let found = |c: &&'static Command| {
            if q.is_empty() {
                return !map.keys_of(c).is_empty()
                    || !map.names_of(c.id).is_empty()
                    || map.keys_source(c.id) == Source::User;
            }
            fold_turkish(c.title).contains(&q)
                || map.keys_of(c).iter().any(|k| fold_turkish(k).contains(&q) || fold_turkish(&shown(k)).contains(&q))
                || c.aliases.iter().any(|a| fold_turkish(a).contains(&q))
                || map.names_of(c.id).iter().any(|n| fold_turkish(n).contains(&q))
        };
        let changed = |c: &&'static Command| map.keys_source(c.id) == Source::User || !map.names_of(c.id).is_empty();
        let all: Vec<&'static Command> = catalog()
            .commands()
            .iter()
            .filter(|c| not_bindable(c).is_none() && self.offers(c.id))
            .filter(found)
            .collect();
        let total = all.len();
        let (mut first, rest): (Vec<_>, Vec<_>) = all.into_iter().partition(changed);
        first.extend(rest);
        first.truncate(LIST_MAX);
        (first, total)
    }

    /// Kısayollar' events: the draft's two settings change, nothing is used before Kaydet.
    pub(crate) fn keys_edit(&mut self, edit: KeysEdit) -> Task<Message> {
        if let KeysEdit::Open = edit {
            self.open_settings_at(crate::settings_sections::Section::Shortcuts);
            return iced::widget::operation::focus(SEARCH);
        }
        let Some(draft) = &mut self.settings_draft else {
            return Task::none();
        };
        let map = draft_keymap(draft);
        let chosen = draft.keys.chosen.and_then(|id| catalog().get(id));
        let title = chosen.map_or("", |c| c.title);
        let page = &mut draft.keys;
        // Only Tuş ekle starts waiting for a key; anything else ends it.
        if !matches!(edit, KeysEdit::Record | KeysEdit::Recorded(_)) {
            page.recording = false;
        }
        let mut shortcuts = None;
        let mut aliases = None;
        match edit {
            KeysEdit::Open => {}
            KeysEdit::Search(text) => page.query = text,
            KeysEdit::Choose(id) => {
                page.chosen = Some(id);
                page.pending = None;
                page.note = None;
                page.name.clear();
            }
            KeysEdit::Record => {
                if chosen.is_some() {
                    page.recording = true;
                    page.pending = None;
                    page.note = Some((Severity::Info, "Bir tuşa ya da tuş birleşimine basın; Esc vazgeçer.".into()));
                }
            }
            KeysEdit::StopRecording => page.note = None,
            KeysEdit::Recorded(chord) => {
                page.recording = false;
                let Some(command) = chosen else {
                    return Task::none();
                };
                match map.check_key(command.id, &chord) {
                    KeyCheck::Free => {
                        shortcuts = Some(map.shortcuts_with(command.id, &chord));
                        page.note = Some((
                            Severity::Success,
                            format!("“{}” {title} komutuna verildi. Kaydet ile geçerli olur.", shown(&chord)),
                        ));
                    }
                    KeyCheck::Own => {
                        page.note = Some((Severity::Info, format!("“{}” zaten {title} komutunun.", shown(&chord))));
                    }
                    KeyCheck::Kept(why) | KeyCheck::Unreadable(why) => {
                        page.note = Some((Severity::Warning, format!("“{}” kullanılamaz. {why}", shown(&chord))));
                    }
                    KeyCheck::Taken(other, _) => {
                        page.note = None;
                        page.pending = Some(Pending::Key { chord, from: other.id });
                    }
                }
            }
            KeysEdit::RemoveKey(chord) => {
                if let Some(command) = chosen {
                    shortcuts = Some(map.shortcuts_without(command.id, &chord));
                    page.note = Some((
                        Severity::Info,
                        format!("“{}” {title} komutundan kaldırıldı. Kaydet ile geçerli olur.", shown(&chord)),
                    ));
                }
            }
            KeysEdit::Move => match (page.pending.take(), chosen) {
                (Some(Pending::Key { chord, from }), Some(command)) => {
                    shortcuts = Some(map.shortcuts_with(command.id, &chord));
                    let other = catalog().get(from).map_or(from, |c| c.title);
                    page.note = Some((
                        Severity::Success,
                        format!("“{}” {other} komutundan alınıp {title} komutuna verildi. Kaydet ile geçerli olur.", shown(&chord)),
                    ));
                }
                (Some(Pending::Name { name, from }), Some(command)) => {
                    aliases = Some(map.aliases_with(&name, command.id));
                    let other = catalog().get(from).map_or(from, |c| c.title);
                    page.name.clear();
                    page.note = Some((
                        Severity::Success,
                        format!("“{name}” {other} komutundan alınıp {title} komutuna verildi. Kaydet ile geçerli olur."),
                    ));
                }
                _ => {}
            },
            KeysEdit::Keep => {
                if let Some(pending) = page.pending.take() {
                    let (what, from) = match &pending {
                        Pending::Key { chord, from } => (shown(chord), *from),
                        Pending::Name { name, from } => (name.clone(), *from),
                    };
                    let other = catalog().get(from).map_or(from, |c| c.title);
                    page.note = Some((Severity::Info, format!("Değişmedi: “{what}” {other} komutunda kaldı.")));
                }
            }
            KeysEdit::NameTyped(text) => page.name = text,
            KeysEdit::AddName => {
                let Some(command) = chosen else {
                    return Task::none();
                };
                match map.check_name(command.id, &page.name) {
                    NameCheck::Free(name) => {
                        aliases = Some(map.aliases_with(&name, command.id));
                        page.name.clear();
                        // The classic CAD names' profile gives way to the user's (alias_profile.rs).
                        let classic = crate::alias_profile::classic()
                            .find(&name)
                            .and_then(|n| catalog().get(n.command))
                            .map(|c| format!(" Klasik CAD komut adlarında “{name}” {} idi; sizin adınız önce gelir.", c.title))
                            .unwrap_or_default();
                        page.note = Some((
                            Severity::Success,
                            format!("“{name}” {title} komutunun adı oldu: komut satırına yazıp Enter'a basın. Kaydet ile geçerli olur.{classic}"),
                        ));
                    }
                    NameCheck::Own => {
                        page.note = Some((Severity::Info, format!("“{}” zaten {title} komutunun adı.", page.name.trim())));
                    }
                    NameCheck::Kept(why) => page.note = Some((Severity::Warning, why)),
                    NameCheck::Unreadable(why) => page.note = Some((Severity::Warning, why.to_owned())),
                    NameCheck::Taken(other, name) => {
                        page.note = None;
                        page.pending = Some(Pending::Name { name, from: other.id });
                    }
                }
            }
            KeysEdit::RemoveName(name) => {
                aliases = Some(map.aliases_without(&name));
                page.note = Some((
                    Severity::Info,
                    format!("“{name}” {title} komutundan kaldırıldı. Kaydet ile geçerli olur."),
                ));
            }
            KeysEdit::CommandDefaults => {
                if let Some(command) = chosen {
                    shortcuts = Some(map.shortcuts_default(command.id));
                    aliases = Some(map.aliases_default(command.id));
                    page.pending = None;
                    page.note = Some((
                        Severity::Info,
                        format!("{title} komutunun tuşları ve adları KentOS'un varsayılanına döndü. Kaydet ile geçerli olur."),
                    ));
                }
            }
            KeysEdit::AllDefaults => {
                shortcuts = Some(Value::Object(Default::default()));
                aliases = Some(Value::Object(Default::default()));
                page.pending = None;
                page.note = Some((
                    Severity::Info,
                    "Bütün kısayollar ve takma adlar KentOS'un varsayılanına döndü. Kaydet ile geçerli olur.".into(),
                ));
            }
        }
        if let Some(value) = shortcuts {
            draft.values.insert(SHORTCUTS_SETTING, value);
        }
        if let Some(value) = aliases {
            draft.values.insert(ALIASES_SETTING, value);
        }
        Task::none()
    }

    /// A key pressed while Kısayollar waits for one: it goes to the chosen command, whatever it is; Esc stops
    /// waiting, a modifier alone waits on. None when nothing waits.
    pub(crate) fn record_key(&mut self, press: &KeyPress) -> Option<Task<Message>> {
        if self.dialog != Some(crate::app::Dialog::Settings)
            || !self.settings_draft.as_ref().is_some_and(|d| d.keys.recording)
        {
            return None;
        }
        if press.repeat {
            return Some(Task::none());
        }
        if press.named() == Some(iced::keyboard::key::Named::Escape)
            && !press.modifiers.control()
            && !press.modifiers.alt()
            && !press.modifiers.shift()
            && !press.modifiers.logo()
        {
            return Some(self.keys_edit(KeysEdit::StopRecording));
        }
        Some(match crate::keys::chord(press) {
            Some(chord) => self.keys_edit(KeysEdit::Recorded(chord)),
            None => Task::none(),
        })
    }

    /// The section: the search and Hepsini varsayılana döndür over the list and the chosen command's form.
    pub(crate) fn shortcuts_section<'a>(&'a self, draft: &'a SettingsDraft) -> Element<'a, Message> {
        let map = draft_keymap(draft);
        let page = &draft.keys;
        let (listed, total) = self.keys_listed(&map, &page.query);
        let search = kentos_ui::widget::text_field("Komut, tuş ya da ad ara", &page.query)
            .id(SEARCH)
            .on_input(|t| message(KeysEdit::Search(t)))
            .width(270);
        let all = button(label::body("Hepsini varsayılana döndür"))
            .on_press_maybe((!map.is_default()).then_some(message(KeysEdit::AllDefaults)))
            .padding(kentos_ui::theme::metrics::action())
            .style(style::button::ghost);
        let mut top = Column::new().spacing(10).push(row![search, space::horizontal(), all].align_y(Center));
        // What the settings keep that is not used here: said, never dropped in silence.
        let unused = map.unused_entries();
        if !unused.is_empty() {
            let first: Vec<String> = unused.iter().take(3).map(|u| format!("{}: {}", u.entry, u.why)).collect();
            let more = if unused.len() > 3 { format!(" … ve {} giriş daha.", unused.len() - 3) } else { String::new() };
            top = top.push(Banner::warning(format!(
                "Ayar dosyasındaki {} giriş kullanılmıyor, saklanıyor. {}{more}",
                unused.len(),
                first.join(" · ")
            )));
        }
        let list = listed.iter().fold(Column::new().spacing(1), |list, command| {
            let keys: Vec<String> = map.keys_of(command).iter().map(|k| shown(k)).collect();
            let mut names: Vec<String> = command.aliases.iter().take(1).map(|a| (*a).to_owned()).collect();
            names.extend(map.names_of(command.id).iter().map(|n| (*n).to_owned()));
            let user = map.keys_source(command.id) == Source::User || !map.names_of(command.id).is_empty();
            let subtitle = format!(
                "{} · {}",
                if names.is_empty() { "adı yok".to_owned() } else { names.join(", ") },
                if user { "kullanıcının" } else { "varsayılan" }
            );
            let mut item = Item::new(command.title)
                .subtitle(subtitle)
                .on_press(message(KeysEdit::Choose(command.id)))
                .current(page.chosen == Some(command.id));
            if !keys.is_empty() {
                item = item.trailing(caps(keys));
            }
            list.push(item)
        });
        let list: Element<'a, Message> = if listed.is_empty() {
            label::muted("Aramayla eşleşen komut yok.").into()
        } else {
            list.into()
        };
        let foot = self.keys_foot(&map, listed.len(), total);
        let details = match page.chosen.and_then(|id| catalog().get(id)) {
            Some(command) => self.keys_details(&map, page, command),
            None => label::muted("Kısayolunu ya da adını değiştireceğiniz komutu soldan seçin.").into(),
        };
        top.push(ListDetails::new(list, details).list_foot(foot).list_width(290.0).height(400.0))
            .into()
    }

    /// Under the list: how many it shows, and the user's keys and names this program has no command for.
    fn keys_foot<'a>(&self, map: &Keymap, shown_count: usize, total: usize) -> Element<'a, Message> {
        let mut words = if shown_count < total {
            format!("{total} komuttan {shown_count} tanesi; aramayı daraltın.")
        } else {
            format!("{total} komut.")
        };
        let elsewhere = map.changed().into_iter().filter(|c| !self.offers(c.id)).count();
        if elsewhere > 0 {
            // “KentOS Harita'da”, “bu CBS projesinde”: where the list is (program.rs).
            let here = self.here_words();
            let mut chars = here.chars();
            let here = chars.next().map_or(String::new(), |c| c.to_uppercase().chain(chars).collect());
            words.push_str(&format!(
                " {here} olmayan {elsewhere} komutun tuşu ya da adı saklı; o komutların olduğu yerde geçerlidir."
            ));
        }
        label::caption(words).style(style::text::muted).into()
    }
}

#[path = "settings_keys/details.rs"]
mod details;
use details::caps;
