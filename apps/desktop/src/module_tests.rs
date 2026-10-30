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
    // KentOS Mimari: walls and the shell's; no basemap, raster, parcel, address or CBS time. Koordinat oku is any
    // drawing's query (AutoCAD's ID).
    let app = program(Some(Module::Architecture));
    for id in [
        "tool.wall",
        "tool.space",
        "layer.new",
        "file.save",
        "tool.line",
        "pointcloud.add",
        "crs.query",
    ] {
        assert!(app.offers(id), "Mimari offers {id}");
    }
    for id in [
        "basemap.osmStandard",
        "service.add",
        "raster.add",
        "tool.parcel",
        "address.search",
        "time.slider",
        "file.import.shp",
        "file.export.atlas",
        "tool.netRoute",
    ] {
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

// ── The strict split (docs/adr/0802): each module's command set ───────────

/// Asserts what a program offers and what it does not.
fn offers_as(module: Option<Module>, yes: &[&str], no: &[&str]) {
    let app = program(module);
    for id in yes {
        assert!(app.offers(id), "{module:?} offers {id}");
    }
    for id in no {
        assert!(!app.offers(id), "{module:?} does not offer {id}");
    }
}

/// What Komut ara finds for a text in a program: its commands' ids, the user's models as `processing.model.<id>`.
fn found(app: &mut App, query: &str) -> Vec<String> {
    app.ribbon_search = query.into();
    app.hits()
        .into_iter()
        .map(|h| match h {
            crate::ribbon_search::Hit::Command(c) => c.id.to_owned(),
            crate::ribbon_search::Hit::Model { id, .. } => format!("processing.model.{id}"),
        })
        .collect()
}

#[test]
fn kentos_mimari_offers_a_buildings_commands_and_no_maps() {
    offers_as(
        Some(Module::Architecture),
        &[
            // AutoCAD's BOUNDARY and ID; the project's own coordinate system stays settable.
            "tool.boundary",
            "crs.query",
            "crs.set",
            "view.properties",
            "processing.run.selection.byExpression",
            "processing.run.selection.byLocation",
            "data.search",
            "data.compare",
            "attachment.add",
            "attachment.addLink",
            "tool.identify",
            "tool.hatchSelected",
            "tool.areaUnion",
            "block.library",
            "style.legend",
            "template.panel",
            "tool.northArrow",
            "pointcloud.style",
        ],
        &[
            // The raster contextual tab's look and analysis.
            "raster.style",
            "raster.georef",
            "raster.mosaic",
            "processing.run.raster.cog",
            "processing.run.surface.slope",
            "processing.run.hydrology.watershed",
            "processing.run.hydrology.basins",
            "processing.run.distance.cost",
            "processing.run.raster.calculator",
            "processing.run.scan.captureLine",
            "processing.run.suitability.weightedOverlay",
            "processing.run.remote.index",
            "processing.run.multidim.series",
            "mesh.query",
            // A parcel's and a route's annotation.
            "tool.coordinateVertices",
            "map.edgeLengths",
            "processing.model.builtin.parcelSheet",
            "tool.stationLabels",
            "tool.vertexPoints",
            "tool.labelsToText",
            // CBS's layers, labels, filter, data model, charts and toolbox.
            "layer.labels",
            "tool.labelMove",
            "view.unplacedLabels",
            "layer.filter",
            "layer.filterFromSelection",
            "layer.newPoint",
            "layer.newArea",
            "layer.info",
            "layer.dataset",
            "data.featureTable",
            "layer.fields",
            "data.statistics",
            "data.chart",
            "chart.bar",
            "data.forms",
            "data.domains",
            "data.subtypes",
            "data.sources",
            "processing.toolbox",
            "processing.newModel",
            "processing.history",
            // The shell's map and CBS commands.
            "draft.overlap",
            "draft.overlap.layer",
            "draft.topologyPoints",
            "analysis.slope",
            "map.contours",
            "map.parcelReport",
            "map.sheet",
            "tool.subdivide",
            "tool.stakeout",
            "sheet.atlas",
            "processing.run.address.geocode",
            "processing.run.address.reverse",
            "processing.run.network.closestFacility",
            "processing.run.attributes.calculate",
            "processing.run.layers.merge",
            "processing.run.geometry.reproject",
            "processing.run.points.numberVertices",
            "workspace.cad",
            "workspace.gis",
        ],
    );
}

#[test]
fn kentos_harita_offers_a_maps_commands_and_leaves_cbs_analysis_to_kentos_gis() {
    offers_as(
        Some(Module::Map),
        &[
            // Its north arrow, the KentOS block library (Kuzey oku, antet) and STAC's band composite.
            "tool.northArrow",
            "block.library",
            "raster.stac",
            "processing.run.remote.composite",
            // A map's terrain and infrastructure analysis.
            "processing.run.surface.slope",
            "processing.run.hydrology.watershed",
            "processing.run.distance.cost",
            "processing.run.interpolation.idw",
            "processing.run.proximity.nearest",
            "processing.run.geometry.buffer",
            // CBS's layers, labels, filter and data, and the toolbox's tools no ribbon holds.
            "layer.labels",
            "layer.filter",
            "layer.newPoint",
            "layer.dataset",
            "data.featureTable",
            "data.forms",
            "processing.toolbox",
            "processing.history",
            "processing.run.attributes.calculate",
            "processing.run.address.geocode",
            "processing.run.layers.splitByAttribute",
            // A map's shell commands.
            "analysis.slope",
            "map.contours",
            "map.profile",
            "map.parcelReport",
            "tool.subdivide",
            "tool.stakeout",
            "draft.overlap",
            "sheet.atlas",
            "crs.set",
            "view.properties",
            "edit.selectFilter.point",
        ],
        &[
            // The architectural kinds of Seçim süzgeci.
            "edit.selectFilter.wall",
            "edit.selectFilter.space",
            "edit.selectFilter.slab",
            "edit.selectFilter.viewmark",
            // CBS's spatial statistics, sampling, suitability, remote sensing and meshes.
            "processing.run.stats.meanCenter",
            "processing.run.stats.moransI",
            "processing.run.stats.dbscan",
            "processing.run.density.kernel",
            "processing.run.density.line",
            "processing.run.proximity.matrix",
            "processing.run.proximity.hub",
            "processing.run.sample.random",
            "processing.run.sample.subdivide",
            "processing.run.suitability.roc",
            "processing.run.remote.index",
            "processing.run.remote.qaMask",
            "processing.run.multidim.meshCalculator",
            "mesh.query",
            "mesh.add",
            // Charts, network analysis and the type's switch.
            "data.chart",
            "chart.pie",
            "processing.run.network.closestFacility",
            "processing.run.network.odMatrix",
            "processing.run.network.serviceAreas",
            "workspace.cad",
            "workspace.plan3d",
        ],
    );
}

#[test]
fn kentos_gis_offers_cbs_and_no_cad_drafting() {
    offers_as(
        Some(Module::Gis),
        &[
            // Its recorded measures and points, a dataset's record, fitting.
            "cogo.check",
            "cogo.update",
            "layer.dataset",
            "point.editor",
            "crs.points",
            "file.import.gnss",
            "transform.fit",
            "transform.edgematch",
            "crs.transform",
            // CBS analysis and charts.
            "processing.run.network.closestFacility",
            "processing.run.stats.moransI",
            "processing.run.density.kernel",
            "processing.run.remote.index",
            "mesh.query",
            "data.chart",
            "chart.bar",
            "tool.vertexPoints",
            "map.edgeLengths",
            "map.profile",
            "analysis.volume",
            "sheet.atlas",
            "draft.overlap",
            "view.properties",
            "edit.selectFilter.insert",
        ],
        &[
            "edit.selectFilter.wall",
            "edit.selectFilter.space",
            "edit.selectFilter.tag",
            // Harita's drafting and survey.
            "tool.planRoad",
            "tool.roadJunctions",
            "tool.medianClose",
            "tool.parcel",
            "tool.coordinateLabel",
            "tool.coordinateVertices",
            "tool.stationLabels",
            "calc.fieldbook",
            "calc.traverse",
            "calc.polar",
            "calc.stakeout",
            "calc.network",
            "calc.levelNetwork",
            "field.send",
            "tool.subdivide",
            "tool.stakeout",
            "map.parcelReport",
            // CAD's constraints, dimensions, leaders, tables, hatches, annotative scales, pictures and blocks.
            "tool.constraint",
            "dimension.space",
            "dimension.break",
            "tool.dimensionReassociate",
            "style.dimensionStyles",
            "leader.addArm",
            "leader.blocks",
            "leader.styles",
            "table.edit",
            "table.blockSchedule",
            "tool.hatchSelected",
            "tool.hatchEdit",
            "edit.hatchesToBack",
            "hatchPatterns.manage",
            "annotation.make",
            "tool.annotativePlace",
            "tool.imageClip",
            "tool.wall",
            "tool.blockDefine",
            "block.panel",
            "workspace.gis",
        ],
    );
}

#[test]
fn kentos_cad_offers_every_command_and_a_module_offers_what_its_ribbon_shows() {
    let app = program(None);
    for c in catalog().commands() {
        assert!(app.offers(c.id), "kentos-cad offers {}", c.id);
    }
    assert!(app.offers("processing.model.user.olculer"));
    // Every command a module's ribbon shows, its selection's tabs too, it offers and runs: no button is refused.
    for (module, work) in [
        (Module::Architecture, Some(CadWork::Architecture)),
        (Module::Map, Some(CadWork::Map)),
        (Module::Gis, None),
    ] {
        let mut app = match work {
            Some(_) => cad_drawing(work),
            None => app_with_drawing(),
        };
        app.program = Some(module);
        for id in commands(app.ribbon_tabs().chain(app.contextual_tabs())) {
            assert!(app.offers(id), "{module:?} shows and offers {id}");
            assert_eq!(app.not_offered(id, id), None, "{module:?}: {id}");
        }
    }
}

#[test]
fn komut_ara_finds_no_other_modules_work() {
    let mut mimari = program(Some(Module::Architecture));
    for query in ["havza", "tesis", "parsel", "altlık", "osm", "raster"] {
        let hits = found(&mut mimari, query);
        assert!(hits.is_empty(), "Mimari: {query}: {hits:?}");
    }
    // An attachment's web address is no address search; Ölçülendirme's slope dimension no terrain slope.
    let hits = found(&mut mimari, "adres");
    assert!(
        !hits
            .iter()
            .any(|id| id.starts_with("address.") || id.contains(".address.")),
        "{hits:?}"
    );
    assert_eq!(hits, ["attachment.addLink"]);
    let hits = found(&mut mimari, "eğim");
    assert_eq!(hits, ["tool.dimension"], "the slope dimension, by its EĞİM");
    // KentOS GIS: no wall, space, block or dimension tool, nor Harita's parcels (Parsel oluştur, İfraz).
    let mut gis = program(Some(Module::Gis));
    for query in ["duvar", "mahal", "km yaz", "kısıt", "ifraz"] {
        let hits = found(&mut gis, query);
        assert!(hits.is_empty(), "GIS: {query}: {hits:?}");
    }
    let hits = found(&mut gis, "parsel");
    assert_eq!(
        hits,
        ["processing.model.builtin.parcelSheet"],
        "a layer's parcel sheet, not the CAD tool"
    );
    for query in ["blok", "ölçü", "kılavuz", "tarama"] {
        let hits = found(&mut gis, query);
        assert!(
            !hits.iter().any(|id| id.starts_with("block.")
                || id.starts_with("tool.block")
                || id.starts_with("dimension.")
                || id.starts_with("leader.")
                || id.starts_with("table.")
                || id.starts_with("tool.hatch")
                || id.starts_with("tool.dim")),
            "GIS: {query}: {hits:?}"
        );
    }
    // KentOS Harita: basemaps as CBS has them; no wall.
    let mut harita = program(Some(Module::Map));
    let hits = found(&mut harita, "osm");
    assert!(hits.contains(&"basemap.osmStandard".to_owned()), "{hits:?}");
    assert!(found(&mut harita, "altlık").contains(&"basemap.remove".to_owned()));
    assert!(found(&mut harita, "duvar").is_empty());
}

#[test]
fn run_refuses_another_modules_command_and_says_which_program_has_it() {
    let mut app = cad_drawing(Some(CadWork::Architecture));
    app.program = Some(Module::Architecture);
    let _ = app.update(Message::Run("processing.toolbox"));
    let said = last_said(&app);
    assert_eq!(
        said,
        "“İşlem araç kutusu” KentOS Mimari'de yok; KentOS Harita'da ve KentOS GIS'te var."
    );
    assert!(!app.available("processing.toolbox"));
    assert_eq!(app.why_disabled("processing.toolbox"), Some(said));
    // A tool of another module does not start, whatever asked for it.
    let _ = app.update(Message::Run("tool.parcel"));
    assert_ne!(app.session.tool_id(), "parcel");
    assert!(
        last_said(&app).contains("KentOS Harita'da var"),
        "{}",
        last_said(&app)
    );
    // Its own run: İçine tıklayarak alan and Koordinat oku.
    assert!(app.available("tool.boundary"));
    let _ = app.update(Message::Run("tool.boundary"));
    assert_eq!(app.session.tool_id(), "boundary");
    // KentOS GIS refuses Harita's drafting; the type's switch is kentos-cad's alone.
    let mut gis = app_with_drawing();
    gis.program = Some(Module::Gis);
    let _ = gis.update(Message::Run("tool.planRoad"));
    assert_eq!(
        last_said(&gis),
        "“Plan yolu” KentOS GIS'te yok; KentOS Harita'da var."
    );
    let _ = gis.update(Message::Run("workspace.cad"));
    assert_eq!(last_said(&gis), "“CAD” KentOS GIS'te yok.");
    assert_eq!(gis.work_mode(), Workspace::Gis);
    assert!(gis.why_disabled("workspace.cad").is_some());
    // kentos-cad runs any (the type is no security boundary).
    let mut all = cad_drawing(Some(CadWork::Architecture));
    let _ = all.update(Message::Run("tool.parcel"));
    assert_eq!(all.session.tool_id(), "parcel");
}

#[test]
fn the_quick_bar_the_shortcuts_window_and_the_drawing_menus_list_the_programs_own() {
    use kentos_ui::widget::Menu;
    // The layout is every program's: Duvar put on the bar in KentOS Mimari is not on KentOS Harita's.
    let mut mimari = program(Some(Module::Architecture));
    let _ = mimari.update(Message::QuickAccess("tool.wall".into(), true));
    assert!(mimari.quick_bar().contains(&"tool.wall".to_owned()));
    let mut harita = program(Some(Module::Map));
    harita
        .layout
        .keep_quick_access(&mimari.layout.quick_access(), std::time::Instant::now());
    assert_eq!(harita.layout.quick_access(), ["tool.wall"]);
    assert_eq!(harita.quick_bar(), ["file.save", "edit.undo", "edit.redo"]);
    // Nor does Harita put another module's command on it.
    let _ = harita.update(Message::QuickAccess("tool.door".into(), true));
    assert!(
        !harita
            .layout
            .quick_access()
            .contains(&"tool.door".to_owned())
    );
    let _ = harita.update(Message::QuickAccess("tool.parcel".into(), true));
    assert!(harita.quick_bar().contains(&"tool.parcel".to_owned()));
    // F1 lists the keys the program runs.
    let titles = crate::shortcuts::listed(&harita, "");
    assert!(
        !titles.contains(&"Üst kat") && !titles.contains(&"Alt kat"),
        "{titles:?}"
    );
    assert!(titles.contains(&"Parsel oluştur"));
    let titles = crate::shortcuts::listed(&mimari, "");
    for gone in ["Parsel oluştur", "Aplikasyon", "İfraz"] {
        assert!(!titles.contains(&gone), "{gone}: {titles:?}");
    }
    assert!(titles.contains(&"Üst kat"));
    let all = crate::shortcuts::listed(&program(None), "");
    assert!(all.contains(&"Parsel oluştur") && all.contains(&"Üst kat"));
    // The drawing's menus: Topolojik düzenleme is a map's aid.
    let item = |app: &App, id| format!("{:?}", app.command_item(Menu::new(), id));
    assert!(!item(&mimari, "draft.topology").contains("Topolojik"));
    assert!(item(&harita, "draft.topology").contains("Topolojik"));
    assert!(item(&mimari, "view.zoomExtents").contains("Tümünü göster"));
}

#[test]
fn the_toolbox_its_tools_and_the_users_models_are_where_islem_arac_kutusu_is() {
    // The user's model, as one saved in KentOS Harita's toolbox (islemler.json is every program's).
    let add_model = |app: &mut App| {
        let mut model = app
            .processing
            .registry
            .model("builtin.parcelSheet")
            .cloned()
            .expect("the built-in model");
        model.id = "user.olculer".into();
        model.label = "Ölçülerim".into();
        app.processing
            .registry
            .save_model(model)
            .expect("a user's model");
    };
    let mut mimari = program(Some(Module::Architecture));
    add_model(&mut mimari);
    assert!(found(&mut mimari, "ölçülerim").is_empty());
    assert!(!mimari.offers_model("user.olculer"));
    let _ = mimari.update(Message::Run("processing.model.builtin.parcelSheet"));
    assert!(last_said(&mimari).contains("KentOS Mimari'de yok"));
    // Mimari's toolbox keeps İfadeyle seç and Konuma göre seç alone.
    assert!(
        mimari.offers_tool("selection.byExpression") && mimari.offers_tool("selection.byLocation")
    );
    assert!(!mimari.offers_tool("geometry.buffer") && !mimari.offers_tool("address.geocode"));
    assert_eq!(mimari.processing_meta(), "2 araç");
    let mut harita = program(Some(Module::Map));
    add_model(&mut harita);
    assert_eq!(
        found(&mut harita, "ölçülerim"),
        ["processing.model.user.olculer"]
    );
    assert!(harita.offers_tool("geometry.buffer") && harita.offers_tool("address.geocode"));
    assert!(!harita.offers_tool("network.closestFacility") && !harita.offers_tool("stats.moransI"));
    let gis = program(Some(Module::Gis));
    assert!(gis.offers_tool("network.closestFacility") && gis.offers_tool("stats.moransI"));
    // The counts: each program's own; kentos-cad's every tool.
    let all = program(None);
    let count = |app: &App| -> usize {
        app.processing_meta()
            .trim_end_matches(" araç")
            .parse()
            .expect("a count")
    };
    assert_eq!(count(&all), all.processing.registry.tools().len());
    assert!(
        count(&mimari) < count(&harita)
            && count(&harita) < count(&gis)
            && count(&gis) <= count(&all)
    );
}
