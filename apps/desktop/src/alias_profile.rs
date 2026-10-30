//! AutoCAD takma adları (docs/adr/0282, CAD-INPUT-18): a profile of typed
//! names, off by default (`drafting.autocadAliases`), that lets the command
//! line take AutoCAD's English command names and their acad.pgp short names
//! (RECTANG, LA, JOG …) beside KentOS's own. The profile is data,
//! `catalog/v1/aliases-autocad.json`: a name, the AutoCAD command it stands
//! for, and the KentOS command (with a tool's method option, JOG is
//! Ölçülendirme as Kırıklı yarıçap) it starts.
//!
//! KentOS's names always win: a profile name that is already a KentOS typed
//! name (a command's alias, title or id, a method's alias) or a single-key
//! shortcut is left out of the profile when it loads (`skipped`), so T is
//! still Yazı and S still Eğri with the profile on. The profile only reaches
//! names typed in the command line and submitted; keys, option letters and
//! KentOS's names mean what they meant.

use std::sync::OnceLock;

use iced::Task;
use kentos_ui::widget::command_line::Command as LineCommand;
use serde::Deserialize;

use crate::app::{App, Message, fold};
use crate::catalog::{Command, Entry, Item, catalog};
use crate::input::release_keyboard;

/// The preference that turns the profile on (Uygulama ayarları › Görünüm › Komut satırı).
pub const SETTING: &str = "drafting.autocadAliases";

/// The profile's file, embedded.
const AUTOCAD: &str = include_str!("../../../catalog/v1/aliases-autocad.json");

/// One typed name of a profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Name {
    /// What is typed (`RECTANG`).
    pub name: &'static str,
    /// The AutoCAD command it stands for (`RECTANG`; `LA` stands for `LAYER`).
    pub autocad: &'static str,
    /// The KentOS command it starts.
    pub command: &'static str,
    /// The tool's method it starts, as the ribbon's menu gives it (`I`).
    pub option: Option<&'static str>,
}

/// A profile as loaded: the names it adds, and those KentOS already uses.
#[derive(Debug)]
pub struct Profile {
    pub title: &'static str,
    /// The names the command line takes when the profile is on, in the file's order.
    pub names: Vec<Name>,
    /// The file's names left out: KentOS already means something by them.
    pub skipped: Vec<Name>,
}

impl Profile {
    /// The name `text` is, folded as command names are.
    pub fn find(&self, text: &str) -> Option<&Name> {
        let folded = fold(text);
        self.names.iter().find(|n| fold(n.name) == folded)
    }
}

/// The AutoCAD profile.
pub fn autocad() -> &'static Profile {
    static PROFILE: OnceLock<Profile> = OnceLock::new();
    PROFILE.get_or_init(|| load(AUTOCAD).expect("catalog/v1/aliases-autocad.json reads"))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawProfile {
    format: String,
    #[allow(dead_code)]
    id: String,
    title: String,
    #[allow(dead_code)]
    source: String,
    names: Vec<RawName>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawName {
    name: String,
    autocad: String,
    command: String,
    #[serde(default)]
    option: Option<String>,
}

fn leak(text: String) -> &'static str {
    Box::leak(text.into_boxed_str())
}

/// Reads a profile and leaves out the names KentOS already uses.
fn load(text: &str) -> Result<Profile, String> {
    let raw: RawProfile = serde_json::from_str(text).map_err(|e| e.to_string())?;
    if raw.format != "kentos.aliasProfile/1" {
        return Err(format!("unknown format {}", raw.format));
    }
    let (mut names, mut skipped) = (Vec::new(), Vec::new());
    for n in raw.names {
        let name = Name {
            name: leak(n.name),
            autocad: leak(n.autocad),
            command: leak(n.command),
            option: n.option.map(leak),
        };
        if kentos_meaning(name.name).is_some() {
            skipped.push(name);
        } else {
            names.push(name);
        }
    }
    Ok(Profile {
        title: leak(raw.title),
        names,
        skipped,
    })
}

/// What KentOS itself means by a typed name, if anything: the command whose
/// alias, title or id it is, the tool whose method's alias it is, or the
/// command a single key starts (`S` is Eğri's key, so it is not Esnet's name).
pub fn kentos_meaning(text: &str) -> Option<&'static str> {
    let folded = fold(text);
    let commands = catalog().commands();
    if let Some(c) = commands.iter().find(|c| {
        c.id == text || fold(c.title) == folded || c.aliases.iter().any(|a| fold(a) == folded)
    }) {
        return Some(c.id);
    }
    if let Some(e) = catalog().method_by_alias(text) {
        return Some(e.id);
    }
    commands
        .iter()
        .find(|c| {
            c.shortcuts.iter().any(|s| {
                let key = s.split(' ').next().unwrap_or(s);
                key.chars().count() == 1 && fold(key) == folded
            })
        })
        .map(|c| c.id)
}

/// The ribbon's entry of a tool's method: its tool and option.
pub fn method(id: &str, option: &str) -> Option<&'static Entry> {
    catalog()
        .every_tab()
        .flat_map(|tab| tab.panels.iter())
        .flat_map(|panel| panel.items.iter())
        .filter_map(|item| match item {
            Item::Split { entries, .. } => Some(entries.iter()),
            _ => None,
        })
        .flatten()
        .find(|e| e.id == id && e.option == Some(option))
}

/// A command's names in the command line's list with the profile on: its own
/// (its first is the list's name), then the profile's for it, methods left out.
pub fn line_aliases(command: &Command) -> &'static [&'static str] {
    let own = command.aliases.get(1..).unwrap_or(&[]);
    let added: Vec<&'static str> = autocad()
        .names
        .iter()
        .filter(|n| n.command == command.id && n.option.is_none())
        .map(|n| n.name)
        .collect();
    if added.is_empty() {
        return own;
    }
    Box::leak([own, added.as_slice()].concat().into_boxed_slice())
}

/// A command's row in the command line's list with the profile on: its names
/// and the profile's (`line_aliases`). A command with no typed name of its own
/// is listed by its first AutoCAD name in the file, not its id (LA for Katman ve
/// öznitelik paneli); Enter runs that name, which the profile knows.
pub fn line_command(command: &Command, row: LineCommand<'static>) -> LineCommand<'static> {
    let names = line_aliases(command);
    match (command.aliases.is_empty(), names.split_first()) {
        (true, Some((first, rest))) => LineCommand { name: first, ..row }.aliases(rest),
        _ => row.aliases(names),
    }
}

/// The profile's methods as rows of their own in the command line's list: a
/// row of the tool would run its name, without the method (JOG is
/// Ölçülendirme as Kırıklı yarıçap).
pub fn method_lines() -> Vec<(&'static str, LineCommand<'static>)> {
    autocad()
        .names
        .iter()
        .filter_map(|n| {
            let entry = method(n.command, n.option?)?;
            let command = catalog().get(n.command)?;
            let title = leak(format!("{} › {}", command.title, entry.label));
            let description = leak(format!("AutoCAD: {}", n.autocad));
            Some((
                command.id,
                LineCommand::new(n.name, title)
                    .description(description)
                    .icon(command.icon),
            ))
        })
        .collect()
}

/// A row of the shortcuts window being gathered: its command and method, what it is, its names.
type Gathered = (
    (&'static str, Option<&'static str>),
    String,
    Vec<&'static str>,
);

/// The shortcuts window's rows (shortcuts.rs): with the profile on, what each
/// command is and its AutoCAD names, in the file's order, then the names
/// KentOS keeps for itself; off, where to turn it on.
pub fn shortcut_rows(on: bool) -> Vec<(String, String)> {
    let profile = autocad();
    if !on {
        return vec![(
            format!(
                "{} (RECTANG, LA, JOG…): Uygulama ayarları › Görünüm › Komut satırı'nda açılır",
                profile.title
            ),
            "Kapalı".to_owned(),
        )];
    }
    let mut rows: Vec<Gathered> = Vec::new();
    for n in &profile.names {
        let key = (n.command, n.option);
        if let Some((_, _, names)) = rows.iter_mut().find(|(k, _, _)| *k == key) {
            names.push(n.name);
            continue;
        }
        let title = catalog().get(n.command).map_or(n.command, |c| c.title);
        let what = match n.option.and_then(|option| method(n.command, option)) {
            Some(entry) => format!("{title} › {}", entry.label),
            None => title.to_owned(),
        };
        rows.push((key, what, vec![n.name]));
    }
    let mut rows: Vec<(String, String)> = rows
        .into_iter()
        .map(|(_, what, names)| (what, names.join(", ")))
        .collect();
    if !profile.skipped.is_empty() {
        let kept: Vec<&str> = profile.skipped.iter().map(|n| n.name).collect();
        rows.push((
            "KentOS'ta başka bir işi olduğu için AutoCAD anlamıyla çalışmayanlar".to_owned(),
            kept.join(", "),
        ));
    }
    rows
}

impl App {
    /// Whether the command line takes AutoCAD's names (`drafting.autocadAliases`).
    pub(crate) fn autocad_names(&self) -> bool {
        self.settings.bool(SETTING)
    }

    /// Starts what a profile name typed in the command line names, when the
    /// profile is on and the name is not one of the user's models; none
    /// otherwise, and the line goes on as before. KentOS's own names never
    /// reach here: the profile left them out.
    pub(crate) fn run_profile_name(&mut self, text: &str) -> Option<Task<Message>> {
        if !self.autocad_names() {
            return None;
        }
        let name = *autocad().find(text)?;
        let folded = fold(text);
        if self
            .user_models()
            .any(|m| fold(&m.label) == folded || fold(&format!("{}…", m.label)) == folded)
        {
            return None;
        }
        // A module's program runs only its own commands (docs/adr/0802).
        if let Some(why) = catalog()
            .get(name.command)
            .and_then(|c| self.not_offered(c.id, c.title))
        {
            self.error(why);
            return Some(Task::none());
        }
        self.remember(text);
        let task = match name.option.and_then(|option| method(name.command, option)) {
            Some(entry) => {
                let option = entry.option.unwrap_or_default();
                self.run_method(entry.id, option, entry.label)
            }
            None => self.run(name.command),
        };
        // As a tool typed by its KentOS name: the drawing takes the keyboard.
        if name.command.starts_with("tool.") && self.session.is_running() {
            self.line_focused = false;
            return Some(Task::batch([task, release_keyboard()]));
        }
        Some(task)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::Standing;
    use std::collections::BTreeSet;

    /// The file's names KentOS already means something else by: they are
    /// left out, KentOS's meaning stays (docs/adr/0282 §3).
    const SKIPPED: [(&str, &str); 8] = [
        ("B", "tool.break"),
        ("CP", "tool.selectPolygon"),
        ("D", "tool.dimension"),
        ("I", "tool.perpIn"),
        ("P", "tool.polyline"),
        ("POLYGON", "tool.polygon"),
        ("S", "tool.spline"),
        ("T", "tool.text"),
    ];

    #[test]
    fn every_name_starts_a_command_the_desktop_runs() {
        let profile = autocad();
        assert_eq!(profile.title, "AutoCAD takma adları");
        let not_run: Vec<&str> = profile
            .names
            .iter()
            .chain(&profile.skipped)
            .filter(|n| catalog().get(n.command).map(|c| c.standing) != Some(Standing::Ported))
            .map(|n| n.name)
            .collect();
        assert!(
            not_run.is_empty(),
            "names of commands the desktop does not run: {not_run:?}"
        );
        for n in profile.names.iter().chain(&profile.skipped) {
            if let Some(option) = n.option {
                assert!(
                    method(n.command, option).is_some(),
                    "{}: {} has no method {option}",
                    n.name,
                    n.command
                );
            }
            assert!(
                n.name
                    .chars()
                    .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit()),
                "{}: names are written as AutoCAD writes them",
                n.name
            );
            assert!(!n.autocad.is_empty(), "{}: the AutoCAD command", n.name);
        }
    }

    #[test]
    fn no_name_of_the_profile_names_two_things() {
        let profile = autocad();
        let mut seen = BTreeSet::new();
        for n in profile.names.iter().chain(&profile.skipped) {
            assert!(
                seen.insert(fold(n.name)),
                "{} is in the profile twice",
                n.name
            );
        }
    }

    /// KentOS's Turkish names and keys win: a profile name KentOS already
    /// uses is left out, and the file holds no name KentOS already gives
    /// the same command (it would add nothing).
    #[test]
    fn kentos_names_and_keys_win() {
        let profile = autocad();
        let skipped: Vec<(&str, &str)> = profile
            .skipped
            .iter()
            .map(|n| (n.name, kentos_meaning(n.name).expect("a KentOS meaning")))
            .collect();
        assert_eq!(skipped, SKIPPED);
        for n in &profile.skipped {
            assert_ne!(
                kentos_meaning(n.name),
                Some(n.command),
                "{} is already KentOS's name for {}; leave it out of the file",
                n.name,
                n.command
            );
        }
        for n in &profile.names {
            assert_eq!(kentos_meaning(n.name), None, "{} is KentOS's", n.name);
        }
    }

    #[test]
    fn the_command_line_lists_the_profile_s_names() {
        let rectangle = catalog().get("tool.rectangle").expect("Dikdörtgen");
        assert!(line_aliases(rectangle).contains(&"RECTANG"));
        assert!(
            line_aliases(rectangle).contains(&"DIKDORTGEN"),
            "its own names stay"
        );
        let dimension = catalog().get("tool.dimension").expect("Ölçülendirme");
        assert!(
            !line_aliases(dimension).contains(&"JOG"),
            "a method has its own row"
        );
        let jog = method_lines();
        let (_, jog) = jog.iter().find(|(_, l)| l.name == "JOG").expect("JOG's row");
        assert_eq!(jog.title, "Ölçülendirme › Kırıklı yarıçap");
        assert_eq!(jog.description, "AutoCAD: DIMJOGGED");
    }

    fn with_profile(on: bool) -> App {
        let mut app = crate::files_testing::app_with_drawing();
        let refused = app
            .settings
            .choose(&[(SETTING, serde_json::Value::Bool(on))]);
        assert!(refused.is_empty(), "{refused:?}");
        app.apply_settings();
        assert_eq!(app.autocad_names(), on);
        app
    }

    /// What Enter runs from the command line's list: the profile's names are
    /// listed only with the profile on; a method's name runs as typed.
    #[test]
    fn the_line_s_list_offers_the_names_with_the_profile_on() {
        use kentos_ui::widget::command_line::{Prompt, Suggested, suggested};
        let first = |app: &App, typed: &str| {
            suggested(&app.line_commands(), None::<&Prompt<'_, Message>>, typed)
                .into_iter()
                .next()
        };
        let off = with_profile(false);
        assert!(
            !matches!(first(&off, "JOG"), Some(Suggested::Command(name)) if name == "JOG"),
            "not listed with the profile off"
        );
        let mut on = with_profile(true);
        assert!(matches!(first(&on, "JOG"), Some(Suggested::Command(name)) if name == "JOG"));
        // Katman ve öznitelik paneli has no typed name of its own: listed as LA, ahead of LAYISO and the like.
        assert!(matches!(first(&on, "LA"), Some(Suggested::Command(name)) if name == "LA"));
        let Some(Suggested::Command(name)) = first(&on, "RECTANG") else {
            panic!("RECTANG is listed");
        };
        let _ = on.update(Message::CommandRun(name));
        assert_eq!(on.session.tool_id(), "rectangle");
    }

    #[test]
    fn off_by_default_and_the_names_are_unknown_then() {
        let mut app = crate::files_testing::app_with_drawing();
        assert!(!app.autocad_names(), "off by default");
        let _ = app.update(Message::CommandRun("JOG".into()));
        assert!(!app.session.is_running());
        assert_eq!(
            app.log.last().map(|l| l.text.as_str()),
            Some("“JOG” adında bir komut yok. Tüm komutlar ve kısayollar için F1’e basın.")
        );
    }

    #[test]
    fn with_the_profile_on_autocad_names_run_and_kentos_names_keep_their_meaning() {
        let mut app = with_profile(true);
        let _ = app.update(Message::CommandRun("rectang".into()));
        assert_eq!(
            app.session.tool_id(),
            "rectangle",
            "folded as KentOS's names are"
        );
        let _ = app.run("tool.cancel");
        // A method: Ölçülendirme as Kırıklı yarıçap.
        let _ = app.update(Message::CommandRun("JOG".into()));
        assert_eq!(app.session.tool_id(), "dimension");
        let _ = app.run("tool.cancel");
        // KentOS's names: T is Yazı, not AutoCAD's MTEXT; the Turkish names work.
        let _ = app.update(Message::CommandRun("T".into()));
        assert_eq!(app.session.tool_id(), "text");
        let _ = app.run("tool.cancel");
        let _ = app.update(Message::CommandRun("DİKDÖRTGEN".into()));
        assert_eq!(app.session.tool_id(), "rectangle");
        let _ = app.run("tool.cancel");
        // The line's ↑ brings back what was typed.
        assert!(
            app.typed.iter().any(
                |e| matches!(e, kentos_ui::widget::command_line::Entry::Input(t) if t == "JOG")
            )
        );
    }
}
