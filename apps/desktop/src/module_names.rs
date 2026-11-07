//! A module's own typed names (docs/adr/3002): names that mean the module's work in its own program and come there
//! before the names the whole of KentOS shares. KentOS Mimari's are `catalog/v1/names-mimari.json`:
//! - PENCERE is the window tool there; Pencere yakınlaştır keeps Z and PENCEREBUYUT.
//! - Two-letter names for the main tools: DU Duvar, KP Kapı, PN Pencere, MH Mahal …
//! - The words a designer says: ODA, LAVABO …
//!
//! A name takes a shared KentOS name only where the file says so (`overrides`, with its reason); every other name
//! is one KentOS gives no meaning (`alias_profile::kentos_meaning`), as the tests hold. The names reach what is typed
//! by name: the command line (typed and sent, its list), Komut ara and the shortcuts window. Keys, option letters
//! and kentos-cad, which offers every module at once, are not touched.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use iced::Task;
use kentos_launch::Module;
use kentos_ui::widget::command_line::Command as LineCommand;
use serde::Deserialize;

use crate::app::{App, Message, fold};
use crate::catalog::catalog;
use crate::input::release_keyboard;

/// KentOS Mimari's names, embedded.
const MIMARI: &str = include_str!("../../../catalog/v1/names-mimari.json");

/// One typed name of a module.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Name {
    /// What is typed (`DU`).
    pub name: &'static str,
    /// The command it starts.
    pub command: &'static str,
    /// The command whose shared name this module takes for its own (PENCERE: Pencere yakınlaştır).
    pub overrides: Option<&'static str>,
    /// Why it takes it.
    pub why: Option<&'static str>,
}

/// A module's names as loaded.
#[derive(Debug)]
pub struct Names {
    pub module: Module,
    pub title: &'static str,
    /// In the file's order.
    pub names: Vec<Name>,
}

impl Names {
    /// The name `text` is, folded as command names are.
    pub fn find(&self, text: &str) -> Option<&Name> {
        let folded = fold(text);
        self.names.iter().find(|n| fold(n.name) == folded)
    }

    /// A command's names, in the file's order.
    pub fn of_command<'a>(&'a self, id: &'a str) -> impl Iterator<Item = &'static str> + 'a {
        self.names.iter().filter(move |n| n.command == id).map(|n| n.name)
    }

    /// The shared names this module takes, as (the command that has them elsewhere, the name).
    pub fn taken(&self) -> impl Iterator<Item = (&'static str, &'static str)> + '_ {
        self.names.iter().filter_map(|n| Some((n.overrides?, n.name)))
    }

    /// A command's names as this module's program reads them: its own, a name the module takes for another
    /// command left out, the module's names for it added. `None` when they are the catalog's.
    pub fn aliases_of(&self, id: &str, own: &[&'static str]) -> Option<Vec<&'static str>> {
        let added: Vec<&'static str> = self.of_command(id).collect();
        let taken: Vec<&'static str> = self.taken().filter(|(from, _)| *from == id).map(|(_, n)| n).collect();
        if added.is_empty() && taken.is_empty() {
            return None;
        }
        // A name the module takes from another command leads (the list names the row by it: PENCERE, not PENCEREKOY).
        let leading: Vec<&'static str> =
            self.names.iter().filter(|n| n.command == id && n.overrides.is_some()).map(|n| n.name).collect();
        let mut names = leading;
        for a in own.iter().copied().filter(|a| !taken.iter().any(|t| fold(t) == fold(a))) {
            if !names.iter().any(|n| fold(n) == fold(a)) {
                names.push(a);
            }
        }
        for n in added {
            if !names.iter().any(|a| fold(a) == fold(n)) {
                names.push(n);
            }
        }
        Some(names)
    }

    /// A row of the command line's list in this program (`aliases_of` applied to its name and aliases). Rows are
    /// built each frame: a changed row is made once and kept.
    pub fn line(&self, id: &'static str, line: LineCommand<'static>) -> LineCommand<'static> {
        type Made = HashMap<(&'static str, usize, usize), (&'static str, &'static [&'static str])>;
        static MADE: OnceLock<Mutex<Made>> = OnceLock::new();
        let mut own = vec![line.name];
        own.extend(line.aliases.iter().copied());
        let Some(names) = self.aliases_of(id, &own) else {
            return line;
        };
        let key = (id, line.name.as_ptr() as usize, line.aliases.as_ptr() as usize);
        let mut made = MADE.get_or_init(Default::default).lock().unwrap_or_else(|e| e.into_inner());
        let (name, aliases) = *made.entry(key).or_insert_with(|| match names.split_first() {
            Some((first, rest)) => (*first, Box::leak(rest.to_vec().into_boxed_slice()) as &'static [&'static str]),
            None => (line.name, &[]),
        });
        LineCommand { name, aliases, ..line }
    }
}

/// A module's names; none for a module without a file of its own.
pub fn of(module: Module) -> Option<&'static Names> {
    match module {
        Module::Architecture => Some(mimari()),
        _ => None,
    }
}

fn mimari() -> &'static Names {
    static NAMES: OnceLock<Names> = OnceLock::new();
    NAMES.get_or_init(|| load(MIMARI).expect("catalog/v1/names-mimari.json reads"))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawNames {
    format: String,
    module: String,
    title: String,
    #[allow(dead_code)]
    source: String,
    names: Vec<RawName>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawName {
    name: String,
    command: String,
    #[serde(default)]
    overrides: Option<String>,
    #[serde(default)]
    why: Option<String>,
}

fn leak(text: String) -> &'static str {
    Box::leak(text.into_boxed_str())
}

fn load(text: &str) -> Result<Names, String> {
    let raw: RawNames = serde_json::from_str(text).map_err(|e| e.to_string())?;
    if raw.format != "kentos.moduleNames/1" {
        return Err(format!("unknown format {}", raw.format));
    }
    let module = Module::from_id(&raw.module).ok_or_else(|| format!("unknown module {}", raw.module))?;
    Ok(Names {
        module,
        title: leak(raw.title),
        names: raw
            .names
            .into_iter()
            .map(|n| Name {
                name: leak(n.name),
                command: leak(n.command),
                overrides: n.overrides.map(leak),
                why: n.why.map(leak),
            })
            .collect(),
    })
}

/// The shortcuts window's rows for a module's names: each command's title and its names, in the file's order.
pub fn shortcut_rows(names: &Names) -> Vec<(String, String)> {
    let mut rows: Vec<(&'static str, Vec<&'static str>)> = Vec::new();
    for n in &names.names {
        match rows.iter_mut().find(|(id, _)| *id == n.command) {
            Some((_, list)) => list.push(n.name),
            None => rows.push((n.command, vec![n.name])),
        }
    }
    rows.into_iter()
        .map(|(id, list)| {
            let title = catalog().get(id).map_or(id, |c| c.title);
            (title.to_owned(), list.join(", "))
        })
        .collect()
}

impl App {
    /// This program's module names; none in kentos-cad, which offers every module at once.
    pub(crate) fn module_names(&self) -> Option<&'static Names> {
        of(self.program?)
    }

    /// A name typed in the command line that is one of this program's own (docs/adr/3002): its command runs,
    /// before KentOS's shared names and the AutoCAD profile. A model of the user's by that name still comes first.
    pub(crate) fn run_module_name(&mut self, text: &str) -> Option<Task<Message>> {
        let name = *self.module_names()?.find(text)?;
        let folded = fold(text);
        if self
            .user_models()
            .any(|m| fold(&m.label) == folded || fold(&format!("{}…", m.label)) == folded)
        {
            return None;
        }
        if let Some(why) = catalog()
            .get(name.command)
            .and_then(|c| self.not_offered(c.id, c.title))
        {
            self.error(why);
            return Some(Task::none());
        }
        self.remember(text);
        let task = self.run(name.command);
        // As a tool typed by its KentOS name: the drawing takes the keyboard.
        if name.command.starts_with("tool.") && self.session.is_running() {
            self.line_focused = false;
            return Some(Task::batch([task, release_keyboard()]));
        }
        Some(task)
    }
}

#[cfg(test)]
#[path = "module_names_tests.rs"]
mod tests;
