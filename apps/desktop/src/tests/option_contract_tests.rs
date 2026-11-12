//! KentOS Mimari's option contract (docs/adr/8003), read from the catalog: every tool KentOS Mimari offers and
//! Harita does not is started in a building's drawing, clicked twice, and its prompt and its settings panel read at
//! each step. One name per letter in a tool, one letter per name across the tools, the contract's letters, the
//! descriptions' letters; then what a letter does while a command runs, and the cursor's Açı.

use std::collections::{BTreeMap, BTreeSet};

use iced::keyboard::key::{Code, Physical};
use iced::keyboard::{Key, Modifiers};
use iced::{Point, Rectangle, Size};
use kentos_commands::{CadWork, Workspace};
use kentos_interaction::{Level, Session};
use kentos_launch::Module;

use crate::app::{App, Message, fold};
use crate::catalog::catalog;
use crate::files_testing::{app_with_drawing, last_said};
use crate::keys::KeyPress;
use crate::viewport::Event;

/// The contract: what these names mean has one letter in every Mimari tool.
const CONTRACT: &[(&str, &str)] = &[
    ("Kalınlık", "E"),
    ("Genişlik", "E"),
    ("En", "E"),
    ("Geri", "G"),
    ("Kapat", "K"),
    ("Yükseklik", "Y"),
    ("Stil", "S"),
    ("Tür", "T"),
    ("Kat", "L"),
    ("Öteleme", "Ö"),
    ("Açı", "D"),
];

/// A drawing on a 1000 × 800 view, snapping off; a building's in KentOS Mimari (`cad`) or a CBS project's.
fn app(cad: bool) -> App {
    let mut app = app_with_drawing();
    let doc = app.document.as_mut().expect("a drawing");
    let mut s = doc.settings().clone();
    if cad {
        s.workspace = Some(Workspace::Cad);
        s.cad_work = Some(CadWork::Architecture);
    } else {
        s.workspace = Some(Workspace::Gis);
    }
    doc.model.set_settings(s);
    if cad {
        app.program = Some(Module::Architecture);
    }
    let _ = app.update(Message::Viewport(Event::Resized(Rectangle::new(Point::ORIGIN, Size::new(1000.0, 800.0)))));
    app.draft.snap = false;
    app.draft.ortho = false;
    app.draft.polar = None;
    app
}

fn click(app: &mut App, at: Point) {
    let _ = app.update(Message::Viewport(Event::Moved(at)));
    let _ = app.update(Message::Viewport(Event::Pressed(at)));
    let _ = app.update(Message::Viewport(Event::Released(at)));
}

fn press(app: &mut App, text: &str, code: Code, modifiers: Modifiers) {
    app.modifiers = modifiers;
    let _ = app.update(Message::Key(KeyPress {
        key: Key::Character(text.into()),
        physical: Physical::Code(code),
        modifiers,
        text: Some(text.to_owned()),
        repeat: false,
    }));
    app.modifiers = Modifiers::default();
}

/// The tools of KentOS Mimari's own: offered there and not in Harita.
fn mimari_tools() -> Vec<&'static str> {
    catalog()
        .commands()
        .iter()
        .filter_map(|c| {
            let tool = c.id.strip_prefix("tool.")?;
            (Session::tools().contains(&tool)
                && catalog().module_has(Module::Architecture, c.id)
                && !catalog().module_has(Module::Map, c.id))
            .then_some(c.id)
        })
        .collect()
}

/// A name as compared: without its unit or hint in brackets (`Eğim (%)`), folded.
fn base(label: &str) -> String {
    let cut = match label.rfind(" (") {
        Some(i) if label.ends_with(')') => &label[..i],
        _ => label,
    };
    fold(cut.trim())
}

/// One step of a tool: its typed options and its panel's rows, as (letter, name).
struct Step {
    tool: &'static str,
    typed: Vec<(String, String)>,
    panel: Vec<(String, String)>,
}

fn letter(key: &str) -> bool {
    key.chars().count() == 1 && key.chars().all(char::is_alphabetic)
}

fn step(app: &App, tool: &'static str) -> Step {
    let typed = app
        .session
        .prompt()
        .options
        .iter()
        .filter(|o| letter(o.key))
        .map(|o| (o.key.to_owned(), o.label.to_owned()))
        .collect();
    let panel = app
        .session
        .settings(&app.format())
        .into_iter()
        .filter(|s| !s.note && letter(s.key))
        .map(|s| (s.key.to_owned(), s.label.into_owned()))
        .collect();
    Step { tool, typed, panel }
}

/// Every Mimari tool at its start and after one and two clicks.
fn steps() -> Vec<Step> {
    let mut out = Vec::new();
    for id in mimari_tools() {
        let mut app = app(true);
        let _ = app.run(id);
        if !app.session.is_running() {
            continue;
        }
        out.push(step(&app, id));
        for at in [Point::new(300.0, 400.0), Point::new(600.0, 400.0)] {
            click(&mut app, at);
            if !app.session.is_running() {
                break;
            }
            out.push(step(&app, id));
        }
    }
    out
}

#[test]
fn the_mimari_tools_are_the_buildings_own() {
    let tools = mimari_tools();
    for id in ["tool.wall", "tool.door", "tool.window", "tool.space", "tool.slab", "tool.stair", "tool.ramp", "tool.column", "tool.grid"] {
        assert!(tools.contains(&id), "{id} is one of KentOS Mimari's own tools: {tools:?}");
    }
    for id in ["tool.line", "tool.circle", "tool.polyline"] {
        assert!(!tools.contains(&id), "{id} is a drawing tool every program offers");
    }
}

/// A letter has one name in a tool: the prompt and the settings panel say the same (docs/adr/8003). The duvar said
/// Genişlik (E) and its panel Kalınlık; a door's Uzaklık (U) was Uçtan uzaklık beside it.
#[test]
fn the_prompt_and_the_panel_name_a_letter_alike() {
    let mut wrong = Vec::new();
    for s in steps() {
        for (key, name) in &s.typed {
            for (k, n) in s.panel.iter().filter(|(k, _)| k == key) {
                if base(name) != base(n) {
                    wrong.push(format!("{} {k}: istem “{name}”, panel “{n}”", s.tool));
                }
            }
        }
    }
    wrong.dedup();
    assert!(wrong.is_empty(), "aynı harf iki ad taşıyor:\n{}", wrong.join("\n"));
}

/// A name typed by a letter has that letter in every Mimari tool (docs/adr/8003): Genişlik was E on a door and G on
/// a stair, so E pressed for a stair's width started Esnet.
#[test]
fn a_name_has_one_letter_in_every_mimari_tool() {
    let mut letters: BTreeMap<String, BTreeMap<String, BTreeSet<&str>>> = BTreeMap::new();
    for s in steps() {
        for (key, name) in &s.typed {
            letters.entry(base(name)).or_default().entry(key.clone()).or_default().insert(s.tool);
        }
    }
    let wrong: Vec<String> = letters
        .iter()
        .filter(|(_, keys)| keys.len() > 1)
        .map(|(name, keys)| format!("{name}: {keys:?}"))
        .collect();
    assert!(wrong.is_empty(), "bir ad birden çok harfte:\n{}", wrong.join("\n"));
}

#[test]
fn the_contract_letters_hold_in_prompt_and_panel() {
    let mut wrong = Vec::new();
    let mut seen = BTreeSet::new();
    for s in steps() {
        for (key, name) in s.typed.iter().chain(&s.panel) {
            if let Some((word, want)) = CONTRACT.iter().find(|(w, _)| fold(w) == base(name)) {
                seen.insert(*word);
                if key != want {
                    wrong.push(format!("{} {word}: {key}, sözleşmede {want}", s.tool));
                }
            }
        }
    }
    wrong.dedup();
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
    for word in ["Kalınlık", "Genişlik", "En", "Geri", "Kapat", "Yükseklik", "Stil", "Tür", "Kat", "Öteleme", "Açı"] {
        assert!(seen.contains(word), "{word} is offered by some Mimari tool");
    }
}

/// A letter is one option in a step.
#[test]
fn a_step_offers_each_letter_once() {
    for s in steps() {
        let mut keys = BTreeSet::new();
        for (key, name) in &s.typed {
            assert!(keys.insert(key.clone()), "{}: {key} twice ({name})", s.tool);
        }
    }
}

/// A tool's description writes an option with its letter, `Kalınlık (E)`: the letter is the tool's (docs/adr/8003).
#[test]
fn the_descriptions_give_the_options_their_letters() {
    let mut names: BTreeMap<&str, BTreeMap<String, String>> = BTreeMap::new();
    for s in steps() {
        for (key, name) in s.typed.iter().chain(&s.panel) {
            let cut = match name.rfind(" (") {
                Some(i) if name.ends_with(')') => name[..i].to_owned(),
                _ => name.clone(),
            };
            names.entry(s.tool).or_default().insert(cut, key.clone());
        }
    }
    let mut wrong = Vec::new();
    for (id, options) in &names {
        let text = catalog().get(id).map_or("", |c| c.description);
        for (name, key) in options {
            let mut from = 0;
            while let Some(i) = text[from..].find(&format!("{name} (")) {
                let at = from + i;
                from = at + name.len() + 2;
                // Only a whole name: not the end of a longer one (Kenar in Kenar ötelemesi starts one).
                if text[..at].chars().last().is_some_and(char::is_alphabetic) {
                    continue;
                }
                let given: String = text[from..].chars().take_while(|c| c.is_uppercase()).collect();
                let ends = text[from + given.len()..].chars().next();
                if !given.is_empty() && given.chars().count() <= 2 && matches!(ends, Some(')' | ':' | ',' | ';')) && &given != key {
                    wrong.push(format!("{id}: “{name} ({given}” — the tool's letter is {key}"));
                }
            }
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// The duvar with two corners placed, the pointer beyond them.
fn wall_chain() -> App {
    let mut app = app(true);
    let _ = app.run("tool.wall");
    click(&mut app, Point::new(300.0, 400.0));
    click(&mut app, Point::new(600.0, 400.0));
    let _ = app.update(Message::Viewport(Event::Moved(Point::new(600.0, 250.0))));
    assert_eq!((app.session.tool_id(), app.session.point_count()), ("wall", 2));
    app
}

/// Shift+E is the duvar's Kalınlık, as a plain e is (docs/adr/8003); it started Uzat, dropping the wall (plan G2).
#[test]
fn a_shifted_letter_is_the_running_commands_option() {
    let mut app = wall_chain();
    press(&mut app, "E", Code::KeyE, Modifiers::SHIFT);
    assert_eq!(app.session.tool_id(), "wall");
    assert_eq!(app.session.point_count(), 2, "the corners stay");
    assert!(app.session.prompt().step.contains("kalınlı"), "the wall asks its thickness: {}", app.session.prompt().step);
}

/// E on a stair is its Genişlik now: it started Esnet and dropped the stair (research S5).
#[test]
fn e_is_the_stairs_width() {
    let mut app = app(true);
    let _ = app.run("tool.stair");
    click(&mut app, Point::new(300.0, 400.0));
    press(&mut app, "e", Code::KeyE, Modifiers::empty());
    assert_eq!(app.session.tool_id(), "stair");
    assert!(app.session.prompt().step.contains("genişli"), "{}", app.session.prompt().step);
}

/// A letter the step does not offer leaves the command running and says why (docs/adr/8003); with no command it
/// starts its tool as before, and a tool that runs over a command (Pencere yakınlaştır) still does.
#[test]
fn a_letter_that_is_not_an_option_keeps_the_command() {
    let mut app = wall_chain();
    let offered: BTreeSet<String> = app.session.prompt().options.iter().map(|o| o.key.to_owned()).collect();
    let (id, chord) = catalog()
        .commands()
        .iter()
        .filter(|c| app.offers(c.id))
        .filter_map(|c| {
            let tool = c.id.strip_prefix("tool.")?;
            let chord = c.shortcuts.iter().find(|s| s.chars().count() == 1 && s.chars().all(char::is_alphabetic))?;
            (Session::tools().contains(&tool) && !Session::is_transparent(tool) && !offered.contains(*chord))
                .then_some((c.id, *chord))
        })
        .next()
        .expect("a letter that starts another tool");
    let lower = chord.to_lowercase();
    press(&mut app, &lower, Code::KeyA, Modifiers::empty());
    assert_eq!((app.session.tool_id(), app.session.point_count()), ("wall", 2), "{chord} ({id}) left the wall running");
    assert_eq!(app.last_level, Some(Level::Warn));
    assert!(last_said(&app).starts_with("Duvar sürüyor:"), "{}", last_said(&app));
    // With nothing running, the letter starts its tool.
    let _ = app.run("tool.cancel");
    let _ = app.run("tool.cancel");
    assert_eq!(app.session.tool_id(), "select");
    press(&mut app, &lower, Code::KeyA, Modifiers::empty());
    assert_eq!(app.session.tool_id(), id.strip_prefix("tool.").unwrap(), "{chord} starts {id} when nothing runs");
}

/// The tag beside the cursor reads a CAD project's direction as Açı, east 0 and counter-clockwise; a CBS project's
/// as Semt from north, clockwise (docs/adr/0165 §4). The duvar, Çoklu çizgi and the point tools wrote Semt in both.
#[test]
fn the_cursor_reads_acı_in_a_cad_project_and_semt_in_a_cbs_one() {
    for (cad, tool, word, grads) in [(true, "tool.wall", "Açı", 0.0), (false, "tool.polyline", "Semt", 100.0)] {
        let mut app = app(cad);
        let _ = app.run(tool);
        click(&mut app, Point::new(300.0, 400.0));
        let _ = app.update(Message::Viewport(Event::Moved(Point::new(600.0, 400.0))));
        let f = app.format();
        let tag = app.session.preview(&f).and_then(|p| p.tag).expect("a tag beside the cursor");
        let want = format!("{word} {}", f.bearing(grads));
        assert!(tag.lines.contains(&want), "{tool}: {:?}, want {want}", tag.lines);
        assert!(!tag.lines.iter().any(|l| l.starts_with(if cad { "Semt" } else { "Açı" })), "{:?}", tag.lines);
    }
}
