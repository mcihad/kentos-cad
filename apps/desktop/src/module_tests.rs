//! KentOS Mimari, Harita and GIS as programs of their own (docs/adr/0801):
//! the ribbon cut to a CAD drawing's work, the work asked once, a file of
//! another module handed to its program, the wizard and the start screen
//! limited to the module, and the module kept with a recent file. Test code
//! only; the hand-over starts no program here (`App::hand_over` is replaced).

use std::cell::RefCell;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use kentos_commands::{CadWork, Workspace};
use kentos_launch::Module;

use crate::app::{App, Message};
use crate::catalog::{Item, Tab, catalog};
use crate::files_testing::{app_with_drawing, drawing, drive, last_said, scratch};
use crate::opening::Purpose;
use crate::project::ask_work::{Choice, Event as WorkEvent};

thread_local! {
    /// The files this thread's apps handed over, to which module.
    static HANDED: RefCell<Vec<(Module, PathBuf)>> = const { RefCell::new(Vec::new()) };
}

fn record(module: Module, path: &Path) -> Result<String, String> {
    HANDED.with(|h| h.borrow_mut().push((module, path.to_path_buf())));
    Ok("deneme".into())
}

fn handed() -> Vec<(Module, PathBuf)> {
    HANDED.with(|h| std::mem::take(&mut *h.borrow_mut()))
}

/// An app working as `program`'s module (none: kentos-cad), without a drawing, starting no program.
fn program(module: Option<Module>) -> App {
    let (mut app, _) = App::boot(None);
    app.program = module;
    app.hand_over = record;
    let _ = handed();
    app
}

/// The sample drawing as a project of this type and work, saved in `dir`.
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

fn work(app: &mut App, e: WorkEvent) {
    let task = app.update(Message::Project(Box::new(crate::project::Event::Work(e))));
    drive(app, task);
}

fn hand(app: &mut App, e: crate::project::hand_over::Event) {
    let _ = app.update(Message::Project(Box::new(crate::project::Event::Hand(e))));
}

fn ids<'a>(tabs: impl Iterator<Item = &'a Tab>) -> Vec<&'static str> {
    tabs.map(|t| t.id).collect()
}

fn panels(app: &App, tab: &str) -> Vec<&'static str> {
    app.ribbon_tabs()
        .find(|t| t.id == tab)
        .map(|t| t.panels.iter().map(|p| p.label).collect())
        .unwrap_or_default()
}

/// Every command a ribbon's tabs name.
fn commands<'a>(tabs: impl Iterator<Item = &'a Tab>) -> BTreeSet<&'static str> {
    let mut out = BTreeSet::new();
    for panel in tabs.flat_map(|t| t.panels.iter()) {
        out.extend(panel.overflow.iter().copied());
        for item in &panel.items {
            match item {
                Item::Command { id, .. } => {
                    out.insert(*id);
                }
                Item::Split { entries, .. } => out.extend(entries.iter().map(|e| e.id)),
                Item::Menu { ids, .. } => out.extend(ids.iter().copied()),
                Item::Builtin(_) => {}
            }
        }
    }
    out
}

/// The sample drawing on screen as a CAD project drawn for `work`.
fn cad_drawing(work: Option<CadWork>) -> App {
    let mut app = app_with_drawing();
    let doc = app.document.as_mut().expect("a drawing");
    let mut s = doc.settings().clone();
    s.workspace = Some(Workspace::Cad);
    s.cad_work = work;
    doc.model.set_settings(s);
    app
}

// ── The ribbon ─────────────────────────────────────────────────────────────

#[test]
fn each_work_shows_its_own_tabs_and_panels() {
    // A building: Mimari, no Plan; Ölçme only its Sorgu.
    let app = cad_drawing(Some(CadWork::Architecture));
    let tabs = ids(app.ribbon_tabs());
    assert!(
        tabs.contains(&"arch") && !tabs.contains(&"plan"),
        "{tabs:?}"
    );
    assert_eq!(panels(&app, "survey"), ["Sorgu"]);
    // A map: Plan and the whole Ölçme, no Mimari.
    let app = cad_drawing(Some(CadWork::Map));
    let tabs = ids(app.ribbon_tabs());
    assert!(
        tabs.contains(&"plan") && !tabs.contains(&"arch"),
        "{tabs:?}"
    );
    assert_eq!(
        panels(&app, "survey"),
        [
            "Sorgu",
            "Saha",
            "Poligon ve alım",
            "Kestirme ve dengeleme",
            "Noktalar",
            "Oturtma"
        ]
    );
    // A CAD drawing not asked its work: the whole CAD ribbon, as before.
    let app = cad_drawing(None);
    assert_eq!(
        ids(app.ribbon_tabs()),
        ids(catalog().tabs_in(Workspace::Cad))
    );
    assert_eq!(panels(&app, "survey").len(), 6);
    // CBS is as it was.
    let app = app_with_drawing();
    assert_eq!(app.work_mode(), Workspace::Gis);
    assert_eq!(
        ids(app.ribbon_tabs()),
        ids(catalog().tabs_in(Workspace::Gis))
    );
}

#[test]
fn the_works_together_show_every_command_and_the_contextual_tabs_stay() {
    let whole = commands(catalog().tabs_in(Workspace::Cad));
    let mut union = BTreeSet::new();
    for w in CadWork::ALL {
        let cut = commands(catalog().tabs_for(Workspace::Cad, Some(w)));
        assert!(
            cut.is_subset(&whole),
            "{w:?} shows nothing the CAD ribbon has not"
        );
        union.extend(cut);
    }
    assert_eq!(union, whole, "every command is on some work's ribbon");
    // The selection's tabs are the CAD ribbon's whatever the work.
    for w in CadWork::ALL {
        let mut app = cad_drawing(Some(w));
        app.selection.set([kentos_domain::Slot(4)]);
        assert_eq!(
            ids(app.contextual_tabs()),
            ids(catalog().contextual_in(Workspace::Cad))
        );
        assert!(app.selection_tab().is_some(), "{w:?}");
    }
    // A command a work's ribbon hides still runs from kentos-cad (the type is no security boundary).
    let mut app = cad_drawing(Some(CadWork::Map));
    assert!(!commands(app.ribbon_tabs()).contains("tool.wall"));
    let _ = app.update(Message::Run("tool.wall"));
    assert_eq!(app.session.tool_id(), "wall");
}

#[test]
fn a_module_shows_its_own_ribbon_and_name_before_any_drawing() {
    let app = program(Some(Module::Architecture));
    assert_eq!(app.work_mode(), Workspace::Cad);
    assert_eq!(app.ribbon_work(), Some(CadWork::Architecture));
    assert!(ids(app.ribbon_tabs()).contains(&"arch"));
    assert_eq!(app.title(), "KentOS Mimari");
    let app = program(Some(Module::Map));
    assert_eq!(app.ribbon_work(), Some(CadWork::Map));
    assert!(ids(app.ribbon_tabs()).contains(&"plan"));
    let app = program(Some(Module::Gis));
    assert_eq!((app.work_mode(), app.ribbon_work()), (Workspace::Gis, None));
    assert_eq!(app.title(), "KentOS GIS");
    // kentos-cad is as it was: CBS without a drawing, “KentOS CAD”.
    let app = program(None);
    assert_eq!(
        (app.work_mode(), app.title().as_str()),
        (Workspace::Gis, "KentOS CAD")
    );
    // The drawing's name follows the program's, with its unsaved mark.
    let mut app = cad_drawing(Some(CadWork::Map));
    app.program = Some(Module::Map);
    let name = app.document.as_ref().expect("a drawing").name().to_owned();
    assert_eq!(app.title(), format!("KentOS Harita — {name} •"));
}

// ── The work asked once ────────────────────────────────────────────────────

#[test]
fn kentos_cad_asks_a_cad_drawing_its_work_once_and_sec_writes_it() {
    let dir = scratch("is-sorusu");
    let path = saved_as(&dir, "eski-cad.kcad", Some(Workspace::Cad), None);
    let mut app = program(None);
    open(&mut app, &path);
    assert!(
        app.asking_work(),
        "a CAD drawing without a work is asked it"
    );
    let doc = app.document.as_ref().expect("on screen meanwhile");
    assert!(!doc.dirty());
    let undo = doc.model.can_undo();
    // The whole CAD ribbon while it is not answered.
    assert_eq!(app.ribbon_work(), None);
    work(&mut app, WorkEvent::Choose(Choice::Work(CadWork::Map)));
    work(&mut app, WorkEvent::Pick);
    assert!(!app.asking_work());
    let doc = app.document.as_ref().expect("a drawing");
    assert_eq!(doc.settings().cad_work, Some(CadWork::Map));
    assert!(doc.dirty(), "an edit of the project");
    assert_eq!(doc.model.can_undo(), undo, "not an undo step");
    assert_eq!(app.ribbon_work(), Some(CadWork::Map));
    assert!(
        last_said(&app).contains("işi: Harita / imar planı"),
        "{}",
        last_said(&app)
    );
    assert!(handed().is_empty(), "kentos-cad hands nothing over");
    // Saved, it opens without the question, and the recent files keep its module.
    let saved = dir.join("haritali.kcad");
    let snapshot = app
        .document
        .as_ref()
        .expect("a drawing")
        .model
        .to_snapshot_v2();
    crate::document::write(&snapshot, &saved).expect("writes");
    let mut again = program(None);
    open(&mut again, &saved);
    assert!(!again.asking_work());
    assert_eq!(again.recent.list()[0].module, Some(Module::Map));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn sonra_or_esc_leave_the_work_unasked_and_the_type_question_comes_first() {
    let dir = scratch("is-sonra");
    let path = saved_as(&dir, "eski-cad.kcad", Some(Workspace::Cad), None);
    let mut app = program(None);
    open(&mut app, &path);
    work(&mut app, WorkEvent::Later);
    let doc = app.document.as_ref().expect("a drawing");
    assert_eq!((doc.settings().cad_work, doc.dirty()), (None, false));
    assert!(
        last_said(&app).contains("bütün CAD şeridi"),
        "{}",
        last_said(&app)
    );
    // The status bar's type asks it again; Esc says the same.
    let _ = app.update(Message::AskCadWork);
    assert!(app.asking_work());
    let _ = app.update(Message::DialogClosed);
    assert!(!app.asking_work());
    assert!(last_said(&app).contains("işi seçilmedi"));
    // A project without a type: the type first, then the work.
    let typeless = saved_as(&dir, "turu-yok.kcad", None, None);
    open(&mut app, &typeless);
    assert!(!app.asking_work(), "the type question is on top");
    let _ = app.update(Message::Project(Box::new(crate::project::Event::Type(
        crate::project::TypeEvent::Choose(Workspace::Cad),
    ))));
    let _ = app.update(Message::Project(Box::new(crate::project::Event::Type(
        crate::project::TypeEvent::Pick,
    ))));
    assert!(app.asking_work(), "then the work");
    // A DWG, DXF or NCZ opened as a drawing of its own is not asked.
    let dxf = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/formats/v1/dxf-units/mm.dxf");
    let mut app = program(None);
    open(&mut app, &dxf);
    assert!(!app.asking_work());
    assert_eq!(
        app.document.as_ref().expect("the DXF").settings().cad_work,
        None,
        "kentos-cad gives a DXF no work"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

// ── A module's program ─────────────────────────────────────────────────────

#[test]
fn a_module_places_its_own_and_hands_another_modules_file_over() {
    let dir = scratch("modul-devir");
    let gis = saved_as(&dir, "mahalle.kcad", Some(Workspace::Gis), None);
    let building = saved_as(
        &dir,
        "bina.kcad",
        Some(Workspace::Cad),
        Some(CadWork::Architecture),
    );
    let mut app = program(Some(Module::Architecture));
    open(&mut app, &building);
    assert!(app.document.is_some(), "a building is KentOS Mimari's own");
    assert_eq!(app.asking_hand_over(), None);
    assert_eq!(app.recent.list()[0].module, Some(Module::Architecture));
    let before = app.document.as_ref().map(|d| d.session);
    // A CBS project: asked, never opened here.
    open(&mut app, &gis);
    assert_eq!(app.asking_hand_over(), Some(&[Module::Gis][..]));
    assert_eq!(app.document.as_ref().map(|d| d.session), before);
    hand(
        &mut app,
        crate::project::hand_over::Event::Open(Module::Gis),
    );
    assert_eq!(handed(), [(Module::Gis, gis.clone())]);
    assert_eq!(
        app.document.as_ref().map(|d| d.session),
        before,
        "nothing opened here"
    );
    assert!(
        last_said(&app).contains("KentOS GIS'te açılıyor"),
        "{}",
        last_said(&app)
    );
    // Vazgeç leaves everything as it was.
    open(&mut app, &gis);
    hand(&mut app, crate::project::hand_over::Event::Cancel);
    assert!(handed().is_empty());
    assert_eq!(app.document.as_ref().map(|d| d.session), before);
    // A program that is not found says so; nothing opens.
    app.hand_over = |m, _| Err(format!("{} bulunamadı", m.title()));
    open(&mut app, &gis);
    hand(
        &mut app,
        crate::project::hand_over::Event::Open(Module::Gis),
    );
    assert!(last_said(&app).contains("KentOS GIS bulunamadı"));
    assert_eq!(app.document.as_ref().map(|d| d.session), before);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_cad_module_asks_a_cad_file_without_a_work_before_it_opens_it() {
    let dir = scratch("modul-is");
    let path = saved_as(&dir, "eski.kcad", Some(Workspace::Cad), None);
    // This module's work: placed with it, an edit of the project.
    let mut app = program(Some(Module::Architecture));
    open(&mut app, &path);
    assert!(
        app.asking_work() && app.document.is_none(),
        "held until answered"
    );
    work(&mut app, WorkEvent::Choose(Choice::Work(CadWork::Architecture)));
    work(&mut app, WorkEvent::Pick);
    let doc = app.document.as_ref().expect("placed");
    assert_eq!(doc.settings().cad_work, Some(CadWork::Architecture));
    assert!(doc.dirty());
    assert!(handed().is_empty());
    // The other CAD module's work: handed over, nothing opened here.
    let mut app = program(Some(Module::Architecture));
    open(&mut app, &path);
    work(&mut app, WorkEvent::Choose(Choice::Work(CadWork::Map)));
    work(&mut app, WorkEvent::Pick);
    assert!(app.document.is_none());
    assert_eq!(handed(), [(Module::Map, path.clone())]);
    // Sonra: placed as it is, the module's work's ribbon shown.
    let mut app = program(Some(Module::Map));
    open(&mut app, &path);
    work(&mut app, WorkEvent::Later);
    assert!(!app.asking_work(), "asked once: not again over the drawing");
    let doc = app.document.as_ref().expect("placed");
    assert_eq!((doc.settings().cad_work, doc.dirty()), (None, false));
    assert_eq!(app.ribbon_work(), Some(CadWork::Map));
    assert!(
        last_said(&app).contains("Harita / imar planı şeridi"),
        "{}",
        last_said(&app)
    );
    // A project without a type is asked its type and work at once, the CBS project beside the works.
    let typeless = saved_as(&dir, "turu-yok.kcad", None, None);
    let mut app = program(Some(Module::Map));
    open(&mut app, &typeless);
    assert!(app.asking_work() && app.document.is_none());
    // Sonra places it as it is, shown as the program's type, and asks nothing more.
    work(&mut app, WorkEvent::Later);
    assert!(
        app.dialog.is_none(),
        "neither the type nor the work asked again"
    );
    assert_eq!(
        (app.work_mode(), app.ribbon_work()),
        (Workspace::Cad, Some(CadWork::Map))
    );
    let mut app = program(Some(Module::Map));
    open(&mut app, &typeless);
    work(&mut app, WorkEvent::Pick);
    let doc = app.document.as_ref().expect("placed");
    assert_eq!(
        (doc.settings().workspace, doc.settings().cad_work),
        (Some(Workspace::Cad), Some(CadWork::Map))
    );
    let mut app = program(Some(Module::Map));
    open(&mut app, &typeless);
    work(&mut app, WorkEvent::Choose(Choice::Gis));
    work(&mut app, WorkEvent::Pick);
    assert!(app.document.is_none());
    assert_eq!(handed(), [(Module::Gis, typeless.clone())]);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn kentos_gis_hands_a_cad_drawing_to_either_cad_program_and_a_cad_module_takes_a_dxf_as_its_work() {
    let dir = scratch("modul-cbs");
    let dxf = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/formats/v1/dxf-units/mm.dxf");
    let path = saved_as(&dir, "eski.kcad", Some(Workspace::Cad), None);
    for file in [&path, &dxf] {
        let mut app = program(Some(Module::Gis));
        open(&mut app, file);
        assert_eq!(
            app.asking_hand_over(),
            Some(&[Module::Architecture, Module::Map][..]),
            "{}",
            file.display()
        );
        assert!(app.document.is_none());
        hand(
            &mut app,
            crate::project::hand_over::Event::Open(Module::Map),
        );
        assert_eq!(handed(), [(Module::Map, file.to_path_buf())]);
    }
    // A CAD module opens a DXF as a drawing of its work, asking nothing.
    let mut app = program(Some(Module::Map));
    open(&mut app, &dxf);
    assert!(!app.asking_work() && app.asking_hand_over().is_none());
    let doc = app.document.as_ref().expect("the DXF");
    assert_eq!(doc.settings().cad_work, Some(CadWork::Map));
    assert_eq!(
        app.recent.list()[0].module,
        Some(Module::Map),
        "its source kept with the module"
    );
    let mut app = program(Some(Module::Architecture));
    open(&mut app, &dxf);
    // A millimetre drawing of its own is a building's: KentOS has two CAD works (docs/adr/0802).
    assert_eq!(
        app.document.as_ref().expect("the DXF").settings().cad_work,
        Some(CadWork::Architecture)
    );
    let _ = std::fs::remove_dir_all(&dir);
}

// ── The wizard ─────────────────────────────────────────────────────────────

fn wizard(app: &mut App, e: crate::project::wizard::Event) {
    let task = app.update(Message::Project(Box::new(crate::project::Event::New(e))));
    drive(app, task);
}

#[test]
fn a_modules_wizard_makes_its_own_projects() {
    use crate::project::wizard::Event as W;
    // KentOS Mimari asks nothing (docs/adr/0802): a building in metres at 1:100 on screen at once.
    let mut app = program(Some(Module::Architecture));
    let _ = app.update(Message::Run("file.new"));
    assert!(app.project.is_none(), "no wizard");
    let s = app.document.as_ref().expect("a new project").settings();
    assert_eq!(
        (s.workspace, s.cad_work, s.srid, s.plot_scale),
        (Some(Workspace::Cad), Some(CadWork::Architecture), 0, 100.0)
    );
    // KentOS Harita: its own window, from the coordinates on; a map on the province's system.
    let mut app = program(Some(Module::Map));
    let _ = app.update(Message::Run("file.new"));
    match &app.project {
        Some(crate::project::Window::New(s)) => {
            assert_eq!(s.first_step(), 1, "no type asked");
        }
        _ => panic!("Harita's own window"),
    }
    wizard(&mut app, W::Back);
    match &app.project {
        Some(crate::project::Window::New(s)) => {
            assert_eq!(s.step, kentos_project::wizard::Step::Coords, "Geri stops at its first step");
        }
        _ => panic!("still open"),
    }
    wizard(&mut app, W::Work(0));
    wizard(&mut app, W::Province(58));
    wizard(&mut app, W::Next);
    wizard(&mut app, W::Next);
    let s = app.document.as_ref().expect("a new project").settings();
    assert_eq!(
        (s.workspace, s.cad_work, s.srid),
        (Some(Workspace::Cad), Some(CadWork::Map), 5256)
    );
    // KentOS GIS: a CBS project.
    let mut app = program(Some(Module::Gis));
    let _ = app.update(Message::Run("file.new"));
    wizard(&mut app, W::Type(Workspace::Cad));
    wizard(&mut app, W::Next);
    wizard(&mut app, W::Next);
    let s = app.document.as_ref().expect("a new project").settings();
    assert_eq!((s.workspace, s.cad_work), (Some(Workspace::Gis), None));
    // kentos-cad's wizard writes the work too, and remembers the type as before.
    let mut app = program(None);
    let _ = app.update(Message::Run("file.new"));
    wizard(&mut app, W::Type(Workspace::Cad));
    wizard(&mut app, W::Next);
    wizard(&mut app, W::Work(1));
    wizard(&mut app, W::Province(58));
    wizard(&mut app, W::Next);
    wizard(&mut app, W::Next);
    let s = app.document.as_ref().expect("a new project").settings();
    assert_eq!(
        (s.workspace, s.cad_work),
        (Some(Workspace::Cad), Some(CadWork::Map))
    );
}

// ── What a module's program offers (docs/adr/0802) ─────────────────────────

#[test]
fn a_module_offers_its_own_commands_and_the_shells() {
    // KentOS Mimari: walls and the shell's; no basemap, raster, parcel, address or CBS time.
    let app = program(Some(Module::Architecture));
    for id in ["tool.wall", "tool.space", "layer.new", "file.save", "tool.line", "pointcloud.add"] {
        assert!(app.offers(id), "Mimari offers {id}");
    }
    for id in ["basemap.osmStandard", "service.add", "raster.add", "tool.parcel", "address.search", "time.slider",
        "crs.query", "file.import.shp", "file.export.atlas", "tool.netRoute"]
    {
        assert!(!app.offers(id), "Mimari does not offer {id}");
    }
    // KentOS Harita: basemaps, services and sources as CBS has them, its survey; no walls.
    let app = program(Some(Module::Map));
    for id in ["basemap.osmStandard", "service.add", "source.postgis", "layer.crs", "raster.add", "tool.parcel",
        "calc.traverse", "address.search", "file.import.shp", "file.export.atlas"]
    {
        assert!(app.offers(id), "Harita offers {id}");
    }
    for id in ["tool.wall", "tool.space", "sheet.levels", "time.slider", "tool.netRoute"] {
        assert!(!app.offers(id), "Harita does not offer {id}");
    }
    // KentOS GIS: CBS's own; no walls, no parcels drawn with the CAD tool.
    let app = program(Some(Module::Gis));
    for id in ["time.slider", "tool.netRoute", "basemap.osmStandard", "layer.new", "data.featureTable"] {
        assert!(app.offers(id), "GIS offers {id}");
    }
    for id in ["tool.wall", "tool.space", "tool.blockDefine", "block.panel"] {
        assert!(!app.offers(id), "GIS does not offer {id}");
    }
    // kentos-cad offers every command.
    let app = program(None);
    assert!(app.offers("tool.wall") && app.offers("time.slider") && app.offers("basemap.osmStandard"));
}

#[test]
fn komut_ara_the_command_line_and_the_shortcuts_find_only_the_modules_own() {
    let ids = |app: &App| -> Vec<&'static str> {
        app.hits()
            .into_iter()
            .filter_map(|h| match h {
                crate::ribbon_search::Hit::Command(c) => Some(c.id),
                crate::ribbon_search::Hit::Model { .. } => None,
            })
            .collect()
    };
    // Komut ara: Duvar in Mimari, not in Harita; Altlık in Harita, not in Mimari.
    let mut mimari = program(Some(Module::Architecture));
    let mut harita = program(Some(Module::Map));
    mimari.ribbon_search = "duvar".into();
    harita.ribbon_search = "duvar".into();
    assert!(ids(&mimari).contains(&"tool.wall"));
    assert!(!ids(&harita).contains(&"tool.wall"), "{:?}", ids(&harita));
    mimari.ribbon_search = "adres".into();
    harita.ribbon_search = "adres".into();
    assert!(ids(&harita).iter().any(|id| id.starts_with("address.")));
    assert!(!ids(&mimari).iter().any(|id| id.starts_with("address.")), "{:?}", ids(&mimari));
    // The command line: a name of another module's command is said, not run.
    let mut harita = program(Some(Module::Map));
    let _ = harita.run_typed("duvar");
    assert_ne!(harita.session.tool_id(), "wall");
    let said = last_said(&harita);
    assert!(said.contains("KentOS Harita'da yok") && said.contains("KentOS Mimari'de var"), "{said}");
    // The line's list holds no other module's command.
    assert!(!harita.line_commands().iter().any(|l| l.title == "Duvar"));
    // A shortcut of another module's command runs nothing here.
    for c in catalog().commands() {
        if harita.offers(c.id) {
            continue;
        }
        for chord in c.shortcuts {
            assert_ne!(harita.shortcut(chord), Some(c.id), "{chord}");
        }
    }
}
