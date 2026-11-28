//! Kullanıcının kısayolları ve takma adları (docs/adr/9308): the keys the user gives commands and the names they
//! type for them, from two settings (`keyboard.shortcuts`, `keyboard.aliases`), over the catalog's own.
//!
//! - **One key, one command.** A key the user gives a command is no other command's: the command that had it by
//!   default loses it while the user's stands, and gets it back when the user's goes ("Varsayılana dön"). A command
//!   the user lists has exactly the keys listed; an empty list takes its keys away.
//! - **The person's, not the module's.** The same keymap in kentos-cad and in every module's program; a key of a
//!   command the program does not offer does nothing there (`App::offers`), it does not fall to another command.
//! - **The drawing's keys.** The sheet designer's keys are its own while a sheet is in front (its commands keep
//!   their keys), and Esc and Enter's commands keep theirs; neither takes the user's.
//! - **Nothing is lost.** An entry this version cannot use (an unknown command, a key it cannot read, one KentOS or
//!   the operating system keeps) stays in the settings, is not used and is said in Uygulama ayarları › Kısayollar.
//!
//! The keymap is built when the settings change (`App::apply_settings`) and kept for the window's thread: the ribbon's
//! menus are built later, in closures that hold no app, and read it here ([`active`]). What giving a key or a name
//! would do and the settings' values after it are edits.rs's; the operating system's keys system.rs's.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use kentos_commands::{
    ALIASES_SETTING, SHORTCUTS_SETTING, alias_entries, normalize_alias, normalize_chord,
    reserved_key, shortcut_entries,
};
use kentos_launch::Module;
use serde_json::Value;

use crate::app::{App, fold};
use crate::catalog::{Command, Standing, catalog};

#[path = "keymap/edits.rs"]
mod edits;
#[path = "keymap/system.rs"]
mod system;

pub use edits::{KeyCheck, NameCheck};
pub use system::{System, system_key, system_key_on};

/// The commands whose keys are KentOS's own (Esc cancels, Enter and Space confirm): the user gives them none.
pub const FIXED: [&str; 2] = ["tool.cancel", "tool.confirm"];

/// Whether a command is the sheet designer's: its keys work while a sheet is in front and are its own.
pub fn sheet_command(id: &str) -> bool {
    id.starts_with("sheet.")
}

/// Why the user cannot give a command keys or names; none when they can (a drawing's command the desktop runs).
pub fn not_bindable(command: &Command) -> Option<&'static str> {
    if sheet_command(command.id) {
        Some("Pafta tasarımcısının tuşları pafta öndeyken geçerlidir ve bu sürümde değiştirilemez.")
    } else if FIXED.contains(&command.id) {
        Some("Esc, Enter ve Boşluk komutun kendisinindir; değiştirilemez.")
    } else if command.standing != Standing::Ported {
        Some("Bu komut masaüstünde henüz çalışmıyor.")
    } else {
        None
    }
}

/// Where a command's keys or names come from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// KentOS's own.
    Default,
    /// The user's.
    User,
}

/// An entry the settings keep that the keymap does not use, and why.
#[derive(Debug, Clone, PartialEq)]
pub struct Unused {
    /// `keyboard.shortcuts` or `keyboard.aliases`.
    pub setting: &'static str,
    /// The entry as written: the command and its key, or the name.
    pub entry: String,
    pub why: String,
}

/// The user's keys and names over the catalog's.
#[derive(Debug, Default)]
pub struct Keymap {
    /// The two settings' values it was built from: the draft's or the settings', kept whole.
    source: (Value, Value),
    /// The user's keys by command, written one way, in their order; a command listed with none has none.
    user: HashMap<&'static str, Vec<String>>,
    /// Every key the user gave a command, and that command: no other command's.
    taken: HashMap<String, &'static str>,
    /// The user's typed names as kept (`ÇZ`), and their commands.
    names: Vec<(String, &'static str)>,
    unused: Vec<Unused>,
}

impl Keymap {
    /// The keymap of these two settings' values.
    pub fn new(shortcuts: &Value, aliases: &Value) -> Self {
        let mut map = Self {
            source: (shortcuts.clone(), aliases.clone()),
            ..Self::default()
        };
        for (id, keys) in shortcut_entries(shortcuts) {
            map.read_keys(&id, keys);
        }
        for (name, id) in alias_entries(aliases) {
            map.read_name(&name, id);
        }
        map
    }

    /// An entry kept but not used, and why.
    fn skip(&mut self, setting: &'static str, entry: String, why: impl Into<String>) {
        self.unused.push(Unused {
            setting,
            entry,
            why: why.into(),
        });
    }

    fn read_keys(&mut self, id: &str, keys: Option<Vec<String>>) {
        let Some(command) = catalog().get(id) else {
            self.skip(
                SHORTCUTS_SETTING,
                id.to_owned(),
                "Bu sürümde böyle bir komut yok; kısayolu saklanıyor, kullanılmıyor.",
            );
            return;
        };
        if let Some(why) = not_bindable(command) {
            self.skip(SHORTCUTS_SETTING, command.title.to_owned(), why);
            return;
        }
        let Some(keys) = keys else {
            self.skip(
                SHORTCUTS_SETTING,
                command.title.to_owned(),
                "Tuşları bir liste değil; saklanıyor, kullanılmıyor.",
            );
            return;
        };
        let mut own = Vec::new();
        for text in keys {
            let entry = format!("{}: {text}", command.title);
            let chord = match normalize_chord(&text) {
                Ok(chord) => chord,
                Err(e) => {
                    self.skip(SHORTCUTS_SETTING, entry, e.message());
                    continue;
                }
            };
            if let Some(why) = reserved_key(&chord)
                .map(|r| r.message())
                .or_else(|| system_key(&chord))
            {
                self.skip(SHORTCUTS_SETTING, entry, why);
                continue;
            }
            match self.taken.get(&chord).copied() {
                Some(other) if other != command.id => {
                    let title = catalog().get(other).map_or(other, |c| c.title);
                    self.skip(
                        SHORTCUTS_SETTING,
                        entry,
                        format!("“{chord}” {title} komutunun; bir tuş tek komutun olur."),
                    );
                    continue;
                }
                Some(_) => continue,
                None => {}
            }
            self.taken.insert(chord.clone(), command.id);
            own.push(chord);
        }
        self.user.insert(command.id, own);
    }

    fn read_name(&mut self, name: &str, id: Option<String>) {
        let Some(command) = id.as_deref().and_then(|id| catalog().get(id)) else {
            self.skip(
                ALIASES_SETTING,
                name.to_owned(),
                "Bu sürümde böyle bir komut yok; ad saklanıyor, kullanılmıyor.",
            );
            return;
        };
        if let Some(why) = not_bindable(command) {
            self.skip(ALIASES_SETTING, name.to_owned(), why);
            return;
        }
        let kept = match normalize_alias(name) {
            Ok(kept) => kept,
            Err(e) => {
                self.skip(ALIASES_SETTING, name.to_owned(), e.message());
                return;
            }
        };
        if let Some(why) = kentos_name(&kept) {
            self.skip(ALIASES_SETTING, name.to_owned(), why);
            return;
        }
        if self.names.iter().any(|(n, _)| fold(n) == fold(&kept)) {
            self.skip(
                ALIASES_SETTING,
                name.to_owned(),
                "Aynı ad bir kez daha yazılmış; ilki geçerli.",
            );
            return;
        }
        self.names.push((kept, command.id));
    }

    /// The settings' values it was built from: `keyboard.shortcuts`, `keyboard.aliases`.
    pub fn source(&self) -> (&Value, &Value) {
        (&self.source.0, &self.source.1)
    }

    /// Whether the user set no key and no name: KentOS's own keys everywhere.
    pub fn is_default(&self) -> bool {
        self.user.is_empty() && self.names.is_empty()
    }

    /// What the settings keep that is not used, and why.
    pub fn unused_entries(&self) -> &[Unused] {
        &self.unused
    }

    /// A command's keys as they work now, in their order: the user's when the user set them, else its own but those
    /// the user gave another command.
    pub fn keys_of<'a>(&'a self, command: &'a Command) -> Vec<&'a str> {
        if let Some(keys) = self.user.get(command.id) {
            return keys.iter().map(String::as_str).collect();
        }
        command
            .shortcuts
            .iter()
            .copied()
            .filter(|k| sheet_command(command.id) || !self.taken.contains_key(*k))
            .collect()
    }

    /// The key menus and tips show for a command: its first.
    pub fn first(&self, command: &Command) -> Option<String> {
        self.keys_of(command).first().map(|k| (*k).to_owned())
    }

    /// Where a command's keys come from.
    pub fn keys_source(&self, id: &str) -> Source {
        if self.user.contains_key(id) {
            Source::User
        } else {
            Source::Default
        }
    }

    /// Whether the user's keys took a key the command has by default (it is another's now).
    pub fn lost_keys<'a>(&self, command: &'a Command) -> Vec<(&'a str, &'static str)> {
        if self.user.contains_key(command.id) || sheet_command(command.id) {
            return Vec::new();
        }
        command
            .shortcuts
            .iter()
            .filter_map(|k| self.taken.get(*k).map(|owner| (*k, *owner)))
            .collect()
    }

    /// The command a key runs where `offers` says what is offered: the user's for a key the user gave (nothing when
    /// it is not offered here: the key does not fall to another command), else the first command that has it by
    /// default, in the catalog's order, as before the user set anything.
    pub fn command_for(
        &self,
        chord: &str,
        offers: impl Fn(&str) -> bool,
    ) -> Option<&'static Command> {
        if let Some(id) = self.taken.get(chord) {
            return catalog().get(id).filter(|c| offers(c.id));
        }
        catalog()
            .commands()
            .iter()
            .find(|c| !self.user.contains_key(c.id) && c.shortcuts.contains(&chord) && offers(c.id))
    }

    /// The command a key runs while a text field has the keyboard: one the catalog lets work there, by its own key, or
    /// by a key the user gave it with Ctrl, Alt or a function key (a letter types in the field).
    pub fn command_in_input(&self, chord: &str) -> Option<&'static str> {
        if let Some(id) = self.taken.get(chord) {
            return catalog()
                .get(id)
                .filter(|c| !c.shortcuts_in_input.is_empty() && crate::keys::is_chorded(chord))
                .map(|c| c.id);
        }
        catalog()
            .commands()
            .iter()
            .find(|c| !self.user.contains_key(c.id) && c.shortcuts_in_input.contains(&chord))
            .map(|c| c.id)
    }

    /// The user's names for a command, as kept.
    pub fn names_of(&self, id: &str) -> Vec<&str> {
        self.names
            .iter()
            .filter(|(_, c)| *c == id)
            .map(|(n, _)| n.as_str())
            .collect()
    }

    /// The command one of the user's names starts, folded as command names are (`çz` is `ÇZ`).
    pub fn command_named(&self, text: &str) -> Option<&'static str> {
        let folded = fold(text);
        self.names
            .iter()
            .find(|(n, _)| fold(n) == folded)
            .map(|(_, c)| *c)
    }

    /// Every command the user gave a key or a name, in the catalog's order.
    pub fn changed(&self) -> Vec<&'static Command> {
        catalog()
            .commands()
            .iter()
            .filter(|c| {
                self.user.contains_key(c.id) || self.names.iter().any(|(_, id)| *id == c.id)
            })
            .collect()
    }
}

/// Why a typed name is KentOS's or a module program's (a command's own name, title or key; a module's names),
/// none when it is free.
fn kentos_name(kept: &str) -> Option<String> {
    if let Some(id) = crate::alias_profile::kentos_meaning(kept) {
        let title = catalog().get(id).map_or(id, |c| c.title);
        return Some(format!(
            "“{kept}” KentOS'ta {title} komutunun adı; başka bir ad seçin."
        ));
    }
    Module::ALL.into_iter().find_map(|m| {
        let name = crate::module_names::of(m)?.find(kept)?;
        let title = catalog()
            .get(name.command)
            .map_or(name.command, |c| c.title);
        Some(format!(
            "“{kept}” {} {title} komutunun adı; başka bir ad seçin.",
            m.in_program()
        ))
    })
}

// ── The keymap in use ─────────────────────────────────────────────────────────────────────────────────────────────

thread_local! {
    /// The keymap the window uses, put in use by `App::apply_settings`.
    static ACTIVE: RefCell<Rc<Keymap>> = RefCell::new(Rc::new(Keymap::default()));
}

/// The keymap in use.
pub fn active() -> Rc<Keymap> {
    ACTIVE.with(|active| active.borrow().clone())
}

/// The key menus and tips show for a command now ([`Keymap::first`]).
pub fn first_key(command: &Command) -> Option<String> {
    active().first(command)
}

/// A command's keys now ([`Keymap::keys_of`]).
pub fn keys(command: &Command) -> Vec<String> {
    active()
        .keys_of(command)
        .into_iter()
        .map(str::to_owned)
        .collect()
}

impl App {
    /// Puts the user's keys and names in use at once, no restart: built again only when the settings changed.
    pub(crate) fn apply_keymap(&self) {
        let shortcuts = self.settings.effective(SHORTCUTS_SETTING);
        let aliases = self.settings.effective(ALIASES_SETTING);
        ACTIVE.with(|active| {
            let mut active = active.borrow_mut();
            if active.source != (shortcuts.clone(), aliases.clone()) {
                *active = Rc::new(Keymap::new(&shortcuts, &aliases));
            }
        });
    }
}

// ── The user's names in the command line's list ──────────────────────────────────────────────────────────────────

/// A row of the command line's list with the user's names for its command (`id`): typing one suggests it, Enter
/// runs it. A command with no typed name of its own is listed by the user's first (its id is no name to type).
pub fn user_line(
    id: &'static str,
    line: kentos_ui::widget::command_line::Command<'static>,
) -> kentos_ui::widget::command_line::Command<'static> {
    let keymap = active();
    let names = keymap.names_of(id);
    if names.is_empty() {
        return line;
    }
    let own_name = catalog().get(id).is_some_and(|c| !c.aliases.is_empty()) || line.name != id;
    let mut list: Vec<&str> = if own_name {
        vec![line.name]
    } else {
        Vec::new()
    };
    list.extend(line.aliases.iter().copied());
    list.extend(names);
    let (name, aliases) = interned(&list);
    kentos_ui::widget::command_line::Command {
        name,
        aliases,
        ..line
    }
}

/// A list of names made `'static` once (the command line's rows are borrowed for the program's life): each list the
/// user's names ever made is kept, a few bytes each.
fn interned(list: &[&str]) -> (&'static str, &'static [&'static str]) {
    thread_local! {
        static MADE: RefCell<HashMap<Vec<String>, (&'static str, &'static [&'static str])>> =
            RefCell::new(HashMap::new());
    }
    let key: Vec<String> = list.iter().map(|s| (*s).to_owned()).collect();
    MADE.with(|made| {
        *made.borrow_mut().entry(key.clone()).or_insert_with(|| {
            let all: Vec<&'static str> = key
                .into_iter()
                .map(|s| &*Box::leak(s.into_boxed_str()))
                .collect();
            match all.split_first() {
                Some((first, rest)) => (
                    *first,
                    Box::leak(rest.to_vec().into_boxed_slice()) as &'static [&'static str],
                ),
                None => ("", &[]),
            }
        })
    })
}
