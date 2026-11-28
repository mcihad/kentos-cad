//! Uygulama ayarları › Kısayollar' checks and edits over the keymap (docs/adr/9308): what giving a key or a name to
//! a command would do, and the two settings' values after it is given, taken back or made KentOS's again. Entries this
//! version does not use stay in the values as they were.

use kentos_commands::{normalize_alias, normalize_chord, reserved_key};
use serde_json::{Map, Value};

use super::{Keymap, Source, kentos_name, sheet_command, system_key};
use crate::app::fold;
use crate::catalog::{Command, catalog};

/// What giving a key to a command would do.
#[derive(Debug, Clone)]
pub enum KeyCheck {
    /// The key goes to the command.
    Free,
    /// It is already the command's.
    Own,
    /// KentOS or the operating system keeps it: why.
    Kept(&'static str),
    /// Another command has it: that command, and whether it is the user's key or its own.
    Taken(&'static Command, Source),
    /// It cannot be read as a key: why.
    Unreadable(&'static str),
}

/// What giving a name to a command would do.
#[derive(Debug, Clone)]
pub enum NameCheck {
    /// The name, as it is kept, goes to the command.
    Free(String),
    /// It is already the command's.
    Own,
    /// KentOS or a module's program means something else by it: why.
    Kept(String),
    /// The user gave it another command: that command, and the name as kept.
    Taken(&'static Command, String),
    /// It cannot be a name: why.
    Unreadable(&'static str),
}

impl Keymap {
    /// What giving `text` (a key as the input routing writes it) to command `id` would do.
    pub fn check_key(&self, id: &str, text: &str) -> KeyCheck {
        let chord = match normalize_chord(text) {
            Ok(chord) => chord,
            Err(e) => return KeyCheck::Unreadable(e.message()),
        };
        if let Some(why) = reserved_key(&chord)
            .map(|r| r.message())
            .or_else(|| system_key(&chord))
        {
            return KeyCheck::Kept(why);
        }
        if let Some(owner) = self.taken.get(&chord) {
            return match catalog().get(owner) {
                Some(c) if c.id != id => KeyCheck::Taken(c, Source::User),
                _ => KeyCheck::Own,
            };
        }
        // Another command's own key, the sheet designer's aside (its keys are its own while a sheet is in front).
        let owner = catalog().commands().iter().find(|c| {
            !sheet_command(c.id)
                && !self.user.contains_key(c.id)
                && c.shortcuts.contains(&chord.as_str())
        });
        match owner {
            Some(c) if c.id != id => KeyCheck::Taken(c, Source::Default),
            Some(_) => KeyCheck::Own,
            None => KeyCheck::Free,
        }
    }

    /// What giving the name `text` to command `id` would do.
    pub fn check_name(&self, id: &str, text: &str) -> NameCheck {
        let kept = match normalize_alias(text) {
            Ok(kept) => kept,
            Err(e) => return NameCheck::Unreadable(e.message()),
        };
        if crate::alias_profile::kentos_meaning(&kept) == Some(id) {
            return NameCheck::Own;
        }
        if let Some(why) = kentos_name(&kept) {
            return NameCheck::Kept(why);
        }
        match self.names.iter().find(|(n, _)| fold(n) == fold(&kept)) {
            Some((_, owner)) if *owner == id => NameCheck::Own,
            Some((n, owner)) => match catalog().get(owner) {
                Some(c) => NameCheck::Taken(c, n.clone()),
                None => NameCheck::Free(kept),
            },
            None => NameCheck::Free(kept),
        }
    }

    /// `keyboard.shortcuts` with `chord` given to command `id` after its keys, taken from the command that had it.
    pub fn shortcuts_with(&self, id: &'static str, chord: &str) -> Value {
        let mut map = self.shortcut_map();
        if let Some(owner) = self.taken.get(chord).filter(|o| **o != id) {
            let rest: Vec<String> = self
                .user
                .get(owner)
                .into_iter()
                .flatten()
                .filter(|k| *k != chord)
                .cloned()
                .collect();
            put_keys(&mut map, owner, rest);
        }
        let mut keys: Vec<String> = catalog()
            .get(id)
            .map(|c| self.keys_of(c).into_iter().map(str::to_owned).collect())
            .unwrap_or_default();
        if !keys.iter().any(|k| k == chord) {
            keys.push(chord.to_owned());
        }
        put_keys(&mut map, id, keys);
        Value::Object(map)
    }

    /// `keyboard.shortcuts` with `chord` taken from command `id`.
    pub fn shortcuts_without(&self, id: &'static str, chord: &str) -> Value {
        let mut map = self.shortcut_map();
        let keys: Vec<String> = catalog()
            .get(id)
            .map(|c| {
                self.keys_of(c)
                    .into_iter()
                    .filter(|k| *k != chord)
                    .map(str::to_owned)
                    .collect()
            })
            .unwrap_or_default();
        put_keys(&mut map, id, keys);
        Value::Object(map)
    }

    /// `keyboard.shortcuts` without the user's keys of command `id`: its own again.
    pub fn shortcuts_default(&self, id: &str) -> Value {
        let mut map = self.shortcut_map();
        map.remove(id);
        Value::Object(map)
    }

    /// `keyboard.aliases` with the name (as kept) given to command `id`, taken from the command that had it.
    pub fn aliases_with(&self, name: &str, id: &'static str) -> Value {
        let mut map = self.alias_map();
        map.retain(|n, _| fold(n) != fold(name));
        map.insert(name.to_owned(), Value::from(id));
        Value::Object(map)
    }

    /// `keyboard.aliases` without the name.
    pub fn aliases_without(&self, name: &str) -> Value {
        let mut map = self.alias_map();
        map.retain(|n, _| fold(n) != fold(name));
        Value::Object(map)
    }

    /// `keyboard.aliases` without the user's names of command `id`.
    pub fn aliases_default(&self, id: &str) -> Value {
        let mut map = self.alias_map();
        map.retain(|_, c| c.as_str() != Some(id));
        Value::Object(map)
    }

    /// The keys' setting as kept, entries this version does not use too.
    fn shortcut_map(&self) -> Map<String, Value> {
        self.source.0.as_object().cloned().unwrap_or_default()
    }

    fn alias_map(&self) -> Map<String, Value> {
        self.source.1.as_object().cloned().unwrap_or_default()
    }
}

/// A command's keys into the setting: none when they are the command's own, as they were.
fn put_keys(map: &mut Map<String, Value>, id: &str, keys: Vec<String>) {
    let own = catalog().get(id).is_some_and(|c| {
        c.shortcuts.len() == keys.len() && c.shortcuts.iter().zip(&keys).all(|(a, b)| a == b)
    });
    if own {
        map.remove(id);
    } else {
        map.insert(id.to_owned(), Value::from(keys));
    }
}
