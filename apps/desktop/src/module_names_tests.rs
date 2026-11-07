//! KentOS Mimari's own typed names (docs/adr/3002): what the file holds, and what typing them does in KentOS Mimari
//! and in kentos-cad. The expected commands are the file's words, never a run's.

use std::collections::BTreeSet;

use kentos_commands::{CadWork, Workspace};
use kentos_launch::Module;

use super::*;
use crate::alias_profile::kentos_meaning;
use crate::catalog::{Standing, catalog};
use crate::files_testing::app_with_drawing;

fn mimari_names() -> &'static Names {
    of(Module::Architecture).expect("KentOS Mimari has names")
}

/// A building's drawing open in `program` (none: kentos-cad).
fn app_in(program: Option<Module>) -> App {
    let mut app = app_with_drawing();
    let doc = app.document.as_mut().expect("a drawing");
    let mut s = doc.settings().clone();
    s.workspace = Some(Workspace::Cad);
    s.cad_work = Some(CadWork::Architecture);
    doc.model.set_settings(s);
    app.program = program;
    app
}

/// Types `text` in the command line and sends it; the running tool's id after.
fn typed(app: &mut App, text: &str) -> &'static str {
    let _ = app.run_typed(text);
    let id = app.session.tool_id();
    let _ = app.run("tool.cancel");
    let _ = app.run("tool.cancel");
    id
}

#[test]
fn the_mimari_names_read_and_each_starts_a_command_kentos_mimari_runs() {
    let names = mimari_names();
    assert_eq!(names.module, Module::Architecture);
    assert!(names.names.len() >= 15, "the main tools and the designer's words");
    let mut seen = BTreeSet::new();
    for n in &names.names {
        assert!(seen.insert(fold(n.name)), "{} is in the file twice", n.name);
        let command = catalog().get(n.command).unwrap_or_else(|| panic!("{}: no command {}", n.name, n.command));
        assert_eq!(command.standing, Standing::Ported, "{}: {} does not run on the desktop", n.name, n.command);
        assert!(
            catalog().module_has(Module::Architecture, n.command),
            "{}: KentOS Mimari does not offer {}",
            n.name,
            n.command
        );
    }
    for n in ["DU", "DD", "OB", "KP", "PN", "MH", "DS", "KL", "AK", "MR", "RM", "MD", "ET"] {
        assert_eq!(names.find(n).map(|n| n.name.chars().count()), Some(2), "{n} is a two-letter name");
    }
}

/// A module's name takes a shared KentOS name only where the file says so, with its reason; every other name is one
/// KentOS gives no meaning (no alias, title, id, method or single key), so nothing a user knows changes.
#[test]
fn a_name_takes_a_kentos_name_only_where_the_file_says_so() {
    for n in &mimari_names().names {
        match n.overrides {
            Some(other) => {
                assert_eq!(kentos_meaning(n.name), Some(other), "{} overrides {other}", n.name);
                assert!(n.why.is_some_and(|w| !w.is_empty()), "{}: why it takes the name", n.name);
            }
            None => assert_eq!(kentos_meaning(n.name), None, "{} already means something in KentOS", n.name),
        }
    }
    let taken: Vec<_> = mimari_names().taken().collect();
    assert_eq!(taken, [("tool.zoomWindow", "PENCERE")]);
}

#[test]
fn in_kentos_mimari_pencere_is_the_window_and_two_letters_start_the_main_tools() {
    let mut app = app_in(Some(Module::Architecture));
    assert_eq!(typed(&mut app, "pencere"), "window");
    assert_eq!(typed(&mut app, "PENCERE"), "window");
    assert_eq!(typed(&mut app, "du"), "wall");
    assert_eq!(typed(&mut app, "kp"), "door");
    assert_eq!(typed(&mut app, "pn"), "window");
    assert_eq!(typed(&mut app, "mh"), "space");
    assert_eq!(typed(&mut app, "oda"), "space");
    // Pencere yakınlaştır keeps its key and its other names.
    assert_eq!(typed(&mut app, "pencerebuyut"), "zoomWindow");
}

#[test]
fn kentos_cad_keeps_pencere_for_the_zoom_window() {
    let mut app = app_in(None);
    assert_eq!(typed(&mut app, "pencere"), "zoomWindow");
    assert_eq!(typed(&mut app, "du"), "select", "DU is a name of KentOS Mimari's only: here it starts nothing");
}

#[test]
fn the_command_line_lists_pencere_under_the_window_in_kentos_mimari_only() {
    let holds = |line: &LineCommand<'static>| {
        std::iter::once(line.name).chain(line.aliases.iter().copied()).any(|a| fold(a) == fold("PENCERE"))
    };
    let app = app_in(Some(Module::Architecture));
    let rows: Vec<_> = app.line_commands().into_iter().filter(holds).collect();
    assert_eq!(rows.iter().map(|r| r.title).collect::<Vec<_>>(), ["Pencere"]);
    assert_eq!(rows[0].name, "PENCERE", "the row is named by the name it takes");
    assert_eq!(rows[0].aliases, ["PENCEREKOY", "WINDOW", "PN"]);
    let zoom = app.line_commands().into_iter().find(|r| r.title == "Pencere yakınlaştır").expect("listed");
    assert!(zoom.aliases.contains(&"PENCEREBUYUT"), "its other names stay: {:?}", zoom.aliases);
    let app = app_in(None);
    let rows: Vec<_> = app.line_commands().into_iter().filter(holds).collect();
    assert_eq!(rows.iter().map(|r| r.title).collect::<Vec<_>>(), ["Pencere yakınlaştır"]);
}

#[test]
fn komut_ara_finds_the_window_first_in_kentos_mimari() {
    let first = |names| crate::ribbon_search::search_in("pencere", names)[0].id;
    assert_eq!(first(Some(mimari_names())), "tool.window");
    assert_eq!(first(None), "tool.zoomWindow");
    let lavabo = crate::ribbon_search::search_in("lavabo", Some(mimari_names()));
    assert_eq!(lavabo.first().map(|c| c.id), Some("block.library"));
}

#[test]
fn the_shortcuts_window_lists_the_names_by_command() {
    let rows = shortcut_rows(mimari_names());
    let window = rows.iter().find(|(what, _)| what == "Pencere").expect("Pencere's row");
    assert_eq!(window.1, "PENCERE, PN");
    let wall = rows.iter().find(|(what, _)| what == "Duvar").expect("Duvar's row");
    assert_eq!(wall.1, "DU");
}
