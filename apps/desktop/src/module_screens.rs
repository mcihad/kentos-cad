//! Pictures of KentOS Mimari, Harita and GIS for the owner (docs/adr/0801):
//! each CAD work's ribbon, the start screen in each module, the wizard
//! limited to the module, the work question, the hand-over question and
//! Proje ayarları's type and work. Dark and light, 1440×900 and 1100×650, the
//! questions also with 18 px text (`-dar-buyuk`), `.run/shots/moduller-*`.
//! Not a correctness test (module_tests.rs is).
//!
//! ```text
//! KENTOS_SHOTS_ONLY=serit cargo test -p kentos-desktop module_screens -- --ignored --nocapture --test-threads=1
//! ```
//!
//! `KENTOS_SHOTS_ONLY` (comma separated) keeps the pictures whose name has one of the words.

use std::path::{Path, PathBuf};

use iced::Size;
use kentos_commands::{CadWork, Workspace};
use kentos_launch::Module;
use kentos_ui::snapshot::Snapshot;
use serde_json::Value;

use crate::app::{App, Message};
use crate::files_testing::{drawing, drive, scratch};
use crate::opening::Purpose;
use crate::project::ask_work::{Choice, Event as WorkEvent};

/// A message the scene is set up with, its tasks run to their end.
fn update(app: &mut App, message: Message) {
    let task = app.update(message);
    drive(app, task);
}

/// A message while the picture settles: its tasks are the window's.
fn settle(app: &mut App, message: Message) {
    let _ = app.update(message);
}

fn wanted(name: &str) -> bool {
    match std::env::var("KENTOS_SHOTS_ONLY") {
        Ok(only) if !only.trim().is_empty() => only
            .split(',')
            .map(str::trim)
            .any(|word| !word.is_empty() && name.contains(word)),
        _ => true,
    }
}

fn out() -> PathBuf {
    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    out
}

fn shoot(app: &mut App, size: Size, name: &str) {
    let mut snapshot = Snapshot::new(size).expect("a renderer");
    snapshot.settle(app, App::view, &mut settle);
    let file = out().join(format!("{name}.png"));
    snapshot
        .render(app.view(), &app.theme())
        .save(&file)
        .expect("writes the picture");
    println!("{}", file.display());
}

/// The window's sizes and themes the pictures are taken in, with their names' endings.
fn frames() -> Vec<(Size, &'static str, String)> {
    let mut all = Vec::new();
    for (size, tag) in [
        (Size::new(1440.0, 900.0), ""),
        (Size::new(1100.0, 650.0), "-dar"),
    ] {
        for (theme, suffix) in [("dark", ""), ("light", "-acik")] {
            all.push((size, theme, format!("{tag}{suffix}")));
        }
    }
    all
}

/// The frames with the large text too (18 px in the narrow window): the questions and the wizard must fit it.
fn frames_and_large() -> Vec<(Size, &'static str, String)> {
    let mut all = frames();
    all.push((
        Size::new(1100.0, 650.0),
        "dark-large",
        "-dar-buyuk".to_owned(),
    ));
    all
}

/// An app as `module`'s program (none: kentos-cad), in a theme (`dark-large`: dark, 18 px text), starting no
/// program.
fn program(module: Option<Module>, theme: &str) -> App {
    let (mut app, _) = App::boot(None);
    app.program = module;
    app.hand_over = |m, _| Ok(m.program().to_owned());
    let (theme, size) = match theme.strip_suffix("-large") {
        Some(theme) => (theme, 18),
        None => (theme, 13),
    };
    let _ = app.settings.choose(&[
        ("appearance.theme", Value::from(theme)),
        ("appearance.textSize", Value::from(size)),
    ]);
    app.apply_settings();
    app
}

/// The sample drawing as a project of this type and work, saved in `dir` as `name`.
fn saved_as(dir: &Path, name: &str, kind: Option<Workspace>, work: Option<CadWork>) -> PathBuf {
    let mut doc = drawing(0);
    let mut s = doc.settings().clone();
    s.workspace = kind;
    s.cad_work = work;
    doc.model.set_settings(s);
    let path = dir.join(name);
    crate::document::write(&doc.model.to_snapshot_v2(), &path).expect("writes");
    path
}

fn open(app: &mut App, path: &Path) {
    let task = app.start_opening(path.to_path_buf(), Purpose::File);
    drive(app, task);
}

/// Each CAD work's ribbon, on its own tab and where its queries are (KentOS Mimari's on Görünüm, it has no
/// Ölçme, docs/adr/0802; KentOS Harita's on Ölçme): `moduller-serit-<iş>-<sekme>`.
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn ribbons() {
    let dir = scratch("moduller-serit");
    for (work, word, tabs) in [
        (CadWork::Architecture, "mimari", ["arch", "view"]),
        (CadWork::Map, "harita", ["plan", "survey"]),
    ] {
        let path = saved_as(
            &dir,
            &format!("{}.kcad", work.name().replace(" / ", "-")),
            Some(Workspace::Cad),
            Some(work),
        );
        for (size, theme, suffix) in frames() {
            for tab in tabs {
                let name = format!("moduller-serit-{word}-{tab}{suffix}");
                if !wanted(&name) {
                    continue;
                }
                let mut app = program(Some(crate::program::module_of_work(work)), theme);
                open(&mut app, &path);
                app.choose_tab(tab);
                shoot(&mut app, size, &name);
            }
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// The start screen in each module, a few recent files of every module kept: `moduller-baslangic-<modül>`.
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn start_screens() {
    for m in Module::ALL {
        for (size, theme, suffix) in frames_and_large() {
            let name = format!("moduller-baslangic-{}{suffix}", m.id());
            if !wanted(&name) {
                continue;
            }
            let mut app = program(Some(m), theme);
            for (file, info, module) in [
                (
                    "/home/kullanici/Projeler/Eski pafta.kcad",
                    "412 nesne · TUREF / TM36",
                    None,
                ),
                (
                    "/home/kullanici/Projeler/Mahalle analizi.kcad",
                    "18.240 nesne · TUREF / TM36",
                    Some(Module::Gis),
                ),
                (
                    "/home/kullanici/Projeler/Ada 112 vaziyet.kcad",
                    "1.306 nesne · TUREF / TM36",
                    Some(Module::Map),
                ),
                (
                    "/home/kullanici/Projeler/Konut A blok.kcad",
                    "2.874 nesne · Yerel (koordinat sistemi yok)",
                    Some(Module::Architecture),
                ),
                (
                    "/home/kullanici/Projeler/Merdiven detayı.kcad",
                    "96 nesne · Yerel (koordinat sistemi yok)",
                    Some(Module::Architecture),
                ),
            ] {
                app.recent.add_in(Path::new(file), info.into(), module);
            }
            app.open_start();
            shoot(&mut app, size, &name);
        }
    }
}

/// Yeni proje in each module: what the project is for (or where), and the type a module makes:
/// `moduller-sihirbaz-<modül>`, `moduller-sihirbaz-<modül>-tur`.
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn wizards() {
    use crate::project::wizard::Event as W;
    for m in Module::ALL {
        for (size, theme, suffix) in frames_and_large() {
            for (step, word) in [(None, ""), (Some(0), "-tur")] {
                let name = format!("moduller-sihirbaz-{}{word}{suffix}", m.id());
                if !wanted(&name) {
                    continue;
                }
                let mut app = program(Some(m), theme);
                update(&mut app, Message::Run("file.new"));
                if m == Module::Map {
                    update(
                        &mut app,
                        Message::Project(Box::new(crate::project::Event::New(W::Province(58)))),
                    );
                }
                if let Some(step) = step {
                    update(
                        &mut app,
                        Message::Project(Box::new(crate::project::Event::New(W::Go(step)))),
                    );
                }
                shoot(&mut app, size, &name);
            }
        }
    }
}

/// The work question: kentos-cad's over the drawing, KentOS Mimari's before it opens (its own work, then
/// KentOS Harita's chosen), and a project without a type in KentOS Harita: `moduller-is-*`.
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn work_questions() {
    let dir = scratch("moduller-is");
    let cad = saved_as(&dir, "Eski çizim.kcad", Some(Workspace::Cad), None);
    let typeless = saved_as(&dir, "Türü yok.kcad", None, None);
    for (size, theme, suffix) in frames_and_large() {
        for (scene, module, file, choose) in [
            ("cad", None, &cad, None),
            ("mimari", Some(Module::Architecture), &cad, None),
            (
                "mimari-harita",
                Some(Module::Architecture),
                &cad,
                Some(Choice::Work(CadWork::Map)),
            ),
            ("turu-yok", Some(Module::Map), &typeless, None),
        ] {
            let name = format!("moduller-is-{scene}{suffix}");
            if !wanted(&name) {
                continue;
            }
            let mut app = program(module, theme);
            open(&mut app, file);
            if let Some(c) = choose {
                update(
                    &mut app,
                    Message::Project(Box::new(crate::project::Event::Work(WorkEvent::Choose(c)))),
                );
            }
            shoot(&mut app, size, &name);
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// Proje ayarları's Proje türü row, the work beside the type: a building in KentOS Mimari, a CAD drawing whose
/// work is not asked yet in kentos-cad: `moduller-ayarlar-*`.
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn settings_screens() {
    let dir = scratch("moduller-ayarlar");
    let building = saved_as(
        &dir,
        "Konut A blok.kcad",
        Some(Workspace::Cad),
        Some(CadWork::Architecture),
    );
    let cad = saved_as(&dir, "Eski çizim.kcad", Some(Workspace::Cad), None);
    for (size, theme, suffix) in frames() {
        for (scene, module, file) in [
            ("mimari", Some(Module::Architecture), &building),
            ("is-yok", None, &cad),
        ] {
            let name = format!("moduller-ayarlar-{scene}{suffix}");
            if !wanted(&name) {
                continue;
            }
            let mut app = program(module, theme);
            open(&mut app, file);
            if app.asking_work() {
                update(
                    &mut app,
                    Message::Project(Box::new(crate::project::Event::Work(WorkEvent::Later))),
                );
            }
            update(&mut app, Message::Run("file.settings"));
            shoot(&mut app, size, &name);
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// A file of another module: a CBS project in KentOS Mimari, a CAD drawing whose work is not known in KentOS
/// GIS: `moduller-devir-*`.
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn hand_over_questions() {
    let dir = scratch("moduller-devir");
    let gis = saved_as(&dir, "Mahalle analizi.kcad", Some(Workspace::Gis), None);
    let cad = saved_as(&dir, "Eski çizim.kcad", Some(Workspace::Cad), None);
    for (size, theme, suffix) in frames_and_large() {
        for (scene, module, file) in [
            ("cbs", Module::Architecture, &gis),
            ("cad", Module::Gis, &cad),
        ] {
            let name = format!("moduller-devir-{scene}{suffix}");
            if !wanted(&name) {
                continue;
            }
            let mut app = program(Some(module), theme);
            open(&mut app, file);
            shoot(&mut app, size, &name);
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
}
