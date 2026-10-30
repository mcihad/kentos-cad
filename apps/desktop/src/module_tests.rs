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
    // A building: Mimari, no Plan and no Ölçme (docs/adr/0802); its Sorgu is the last of Görünüm's panels.
    let app = cad_drawing(Some(CadWork::Architecture));
    let tabs = ids(app.ribbon_tabs());
    assert!(
        tabs.contains(&"arch") && !tabs.contains(&"plan") && !tabs.contains(&"survey"),
        "{tabs:?}"
    );
    assert_eq!(panels(&app, "view").last(), Some(&"Sorgu"));
    // A map: Plan and the whole Ölçme, no Mimari; Görünüm has no Sorgu, Ölçme starts with it.
    let app = cad_drawing(Some(CadWork::Map));
    let tabs = ids(app.ribbon_tabs());
    assert!(
        tabs.contains(&"plan") && !tabs.contains(&"arch"),
        "{tabs:?}"
    );
    assert!(!panels(&app, "view").contains(&"Sorgu"));
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
            // Its queries on Giriş, its office standard.
            "tool.measure",
            "tool.area",
            "tool.measureAngle",
            "standards.check",
            "standards.apply",
            "layer.impact",
        ],
        &[
            // The raster contextual tab's look and analysis.
            "raster.style",
            "raster.georef",
            "raster.mosaic",
            "raster.stac",
            "tool.stationOffset",
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
            // Its north arrow, the KentOS block library (Kuzey oku, antet) and a scene's band composite.
            "tool.northArrow",
            "block.library",
            "processing.run.remote.composite",
            // A map's terrain, its scanned sheets and its overlays.
            "processing.run.surface.slope",
            "processing.run.surface.contours",
            "processing.run.interpolation.idw",
            "processing.run.interpolation.tin",
            "processing.run.raster.mosaic",
            "processing.run.scan.captureLine",
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
            // CBS's spatial statistics, sampling, suitability, remote sensing and meshes, its geostatistics, raster
            // analysis, hydrology, distance and generalization.
            "processing.run.stats.meanCenter",
            "processing.run.stats.moransI",
            "processing.run.stats.dbscan",
            "processing.run.density.kernel",
            "processing.run.density.line",
            "processing.run.proximity.matrix",
            "processing.run.proximity.hub",
            "processing.run.proximity.nearest",
            "processing.run.sample.random",
            "processing.run.sample.subdivide",
            "processing.run.suitability.roc",
            "processing.run.remote.index",
            "processing.run.remote.qaMask",
            "processing.run.multidim.meshCalculator",
            "processing.run.interpolation.kriging",
            "processing.run.hydrology.watershed",
            "processing.run.distance.cost",
            "processing.run.raster.calculator",
            "processing.run.generalize.smooth",
            "raster.stac",
            "mesh.query",
            "mesh.add",
            // The drawing standard is KentOS Mimari's.
            "standards.check",
            "standards.apply",
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
            // A parcel's edge lengths and sheet are Harita's writing; the text styles CAD's, the drawing standard
            // KentOS Mimari's.
            "map.edgeLengths",
            "processing.model.builtin.parcelSheet",
            "style.textStyles",
            "standards.check",
            "standards.apply",
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
    // KentOS GIS: no wall, space, block or dimension tool, nor Harita's parcels (Parsel oluştur, İfraz, the parcel
    // sheet's corner numbers and edges).
    let mut gis = program(Some(Module::Gis));
    for query in ["duvar", "mahal", "km yaz", "kısıt", "ifraz", "parsel"] {
        let hits = found(&mut gis, query);
        assert!(hits.is_empty(), "GIS: {query}: {hits:?}");
    }
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

/// A tool's method typed by its own name follows the program's ribbon (docs/adr/0802): Ölçülendirme's Semt,
/// a survey's bearing, is KentOS Harita's; KentOS Mimari's Ölçülendirme has no Semt to start.
#[test]
fn a_method_typed_by_its_name_is_the_programs_own() {
    let mut mimari = cad_drawing(Some(CadWork::Architecture));
    mimari.program = Some(Module::Architecture);
    let _ = mimari.run_typed("SEMT");
    assert!(!mimari.session.is_running(), "no Semt in KentOS Mimari");
    assert_eq!(
        last_said(&mimari),
        "“Semt” KentOS Mimari'de yok; KentOS Harita'da var."
    );
    // Its own methods start: Ölçülendirme's Koordinat (DOR).
    let _ = mimari.run_typed("DOR");
    assert_eq!(mimari.session.tool_id(), "dimension");
    let mut harita = cad_drawing(Some(CadWork::Map));
    harita.program = Some(Module::Map);
    let _ = harita.run_typed("SEMT");
    assert_eq!(harita.session.tool_id(), "dimension");
    let mut all = cad_drawing(Some(CadWork::Architecture));
    let _ = all.run_typed("SEMT");
    assert_eq!(all.session.tool_id(), "dimension", "kentos-cad starts it");
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

// ── Where each command belongs (docs/adr/0802, the owner's later decisions) ────

/// What `read` finds in the app `make` gives, dropped after: one app at a time on a test's stack (an app is large).
fn with_app<T>(make: impl FnOnce() -> App, read: impl FnOnce(&mut App) -> T) -> T {
    let mut app = make();
    read(&mut app)
}

/// The sample drawing on screen as a CAD project of `work` in `module`'s program.
fn cad_in(module: Module, work: CadWork) -> App {
    let mut app = cad_drawing(Some(work));
    app.program = Some(module);
    app
}

/// The sample drawing on screen as a CBS project in KentOS GIS.
fn gis_drawing() -> App {
    let mut app = app_with_drawing();
    app.program = Some(Module::Gis);
    app
}

/// The commands of a program's panel by its tab (a contextual one too) and its name.
fn panel_commands(app: &App, tab: &str, panel: &str) -> Option<Vec<&'static str>> {
    app.ribbon_tabs()
        .chain(app.contextual_tabs())
        .find(|t| t.id == tab)
        .and_then(|t| t.panels.iter().find(|p| p.label == panel))
        .map(crate::catalog::Panel::commands)
}

/// KentOS Harita keeps map making, survey and terrain: TIN and IDW, the terrain's surfaces and contours, a mosaic,
/// a clip and a resampling, the scanned sheet's tools, overlays, conversions, routes and point clouds. Geostatistics,
/// raster analysis, hydrology, distance, generalization, proximity, Voronoi, remote sensing, STAC and meshes are
/// KentOS GIS's analysis; GIS and kentos-cad offer every one.
#[test]
fn kentos_harita_keeps_its_terrain_and_leaves_the_analysis_to_kentos_gis() {
    let keeps = [
        "processing.run.interpolation.tin",
        "processing.run.interpolation.idw",
        "processing.run.surface.slope",
        "processing.run.surface.aspect",
        "processing.run.surface.hillshade",
        "processing.run.surface.colorRelief",
        "processing.run.surface.contours",
        "processing.run.raster.mosaic",
        "processing.run.raster.clipByMask",
        "processing.run.raster.resample",
        "processing.run.raster.toPolygons",
        "processing.run.raster.cog",
        "processing.run.scan.captureLine",
        "processing.run.scan.closeArea",
        "processing.run.scan.contourElevations",
        "processing.run.generate.delaunay",
        "processing.run.generate.bounding",
        "processing.run.attributes.fromInside",
        "processing.run.attributes.fromEnclosing",
        "processing.run.statistics.summary",
        "processing.run.geometry.buffer",
        "processing.run.geometry.dissolve",
        "processing.run.geometry.symDifference",
        "processing.run.convert.pointsToPath",
        "processing.run.convert.densify",
        "processing.run.sample.alongLines",
        "processing.run.linearRef.createRoutes",
        "processing.run.pointcloud.ground",
        "processing.run.remote.composite",
        "raster.add",
        "raster.mosaic",
        "basemap.osmStandard",
        "service.add",
        "source.postgis",
        "address.search",
    ];
    let analysis = [
        "processing.run.interpolation.kriging",
        "processing.run.interpolation.spline",
        "processing.run.interpolation.naturalNeighbor",
        "processing.run.generate.concaveHull",
        "processing.run.generate.pole",
        "processing.run.generate.voronoi",
        "processing.run.proximity.nearest",
        "processing.run.proximity.neighbors",
        "processing.run.proximity.shortestLine",
        "processing.run.generalize.smooth",
        "processing.run.generalize.orthogonalize",
        "processing.run.generalize.building",
        "processing.run.generalize.aggregate",
        "processing.run.generalize.dropParts",
        "processing.run.generalize.eliminate",
        "processing.run.generalize.coverageSimplify",
        "processing.run.raster.calculator",
        "processing.run.raster.reclassify",
        "processing.run.raster.focalStatistics",
        "processing.run.raster.cellStatistics",
        "processing.run.raster.histogram",
        "processing.run.raster.zonalStatistics",
        "processing.run.raster.toLines",
        "processing.run.raster.toPoints",
        "processing.run.raster.rasterize",
        "processing.run.surface.curvature",
        "processing.run.surface.ruggedness",
        "processing.run.surface.insolation",
        "processing.run.hydrology.fill",
        "processing.run.hydrology.flowDirection",
        "processing.run.hydrology.flowAccumulation",
        "processing.run.hydrology.wetness",
        "processing.run.hydrology.pourPoint",
        "processing.run.hydrology.watershed",
        "processing.run.hydrology.basins",
        "processing.run.hydrology.streams",
        "processing.run.distance.euclidean",
        "processing.run.distance.cost",
        "processing.run.distance.path",
        "processing.run.distance.corridor",
        "processing.run.remote.index",
        "processing.run.remote.supervised",
        "processing.run.remote.unsupervised",
        "processing.run.remote.accuracy",
        "processing.run.remote.change",
        "processing.run.remote.qaMask",
        "processing.run.remote.pansharpen",
        "processing.run.remote.split",
        "processing.run.stats.meanCenter",
        "processing.run.stats.hotSpot",
        "raster.stac",
        "mesh.query",
    ];
    offers_as(Some(Module::Map), &keeps, &analysis);
    let every: Vec<&str> = keeps.iter().chain(&analysis).copied().collect();
    offers_as(Some(Module::Gis), &every, &[]);
    offers_as(None, &every, &[]);
    // By name too: Komut ara finds none of them in KentOS Harita, KentOS GIS finds Kriging.
    let queries = ["kriging", "havza", "stac", "voronoi", "genelleştir"];
    let harita = with_app(
        || program(Some(Module::Map)),
        |app| queries.iter().map(|q| found(app, q)).collect::<Vec<_>>(),
    );
    for (query, hits) in queries.iter().zip(&harita) {
        assert!(
            !hits.iter().any(|id| analysis.contains(&id.as_str())),
            "Harita: {query}: {hits:?}"
        );
    }
    let gis = with_app(|| program(Some(Module::Gis)), |app| found(app, "kriging"));
    assert!(
        gis.contains(&"processing.run.interpolation.kriging".to_owned()),
        "{gis:?}"
    );
}

/// The drawing standard (Mimari ofis: A-DUVAR …) is KentOS Mimari's; Değişikliğin etkisi stays everywhere, and a CBS
/// project's standard is its data profile.
#[test]
fn the_drawing_standard_is_kentos_mimaris() {
    let standard = ["standards.check", "standards.apply"];
    offers_as(Some(Module::Architecture), &standard, &[]);
    offers_as(Some(Module::Map), &["layer.impact"], &standard);
    offers_as(
        Some(Module::Gis),
        &["layer.impact", "profile.window"],
        &standard,
    );
    let standart = |module: Module, tab: &'static str| {
        with_app(
            || program(Some(module)),
            |app| panel_commands(app, tab, "Standart"),
        )
    };
    assert_eq!(
        standart(Module::Architecture, "manage").expect("Yönet › Standart"),
        ["standards.check", "standards.apply", "layer.impact"]
    );
    assert_eq!(
        standart(Module::Map, "manage").expect("Yönet › Standart"),
        ["layer.impact"]
    );
    assert_eq!(
        standart(Module::Gis, "data").expect("Veri › Standart"),
        ["profile.window", "lineTypes.manage", "layer.impact"]
    );
}

/// Yazı stilleri and an area's Yaz ve tara (corner coordinates, edge lengths, Çoklu tara, the parcel sheet) are
/// CAD's: KentOS GIS shows neither; KentOS Harita keeps the edge lengths and the parcel sheet in Plan › Plan
/// yazıları too.
#[test]
fn kentos_gis_has_no_text_styles_and_no_yaz_ve_tara() {
    with_app(
        || program(Some(Module::Gis)),
        |gis| {
            assert!(panel_commands(gis, "ctx.area", "Alan").is_some());
            assert_eq!(panel_commands(gis, "ctx.area", "Yaz ve tara"), None);
            let text = panel_commands(gis, "ctx.text", "Yazı").expect("a text's Yazı");
            assert!(!text.contains(&"style.textStyles"), "{text:?}");
            for id in [
                "style.textStyles",
                "map.edgeLengths",
                "processing.model.builtin.parcelSheet",
            ] {
                assert!(!gis.offers(id), "GIS does not offer {id}");
            }
            assert!(!gis.offers_model("builtin.parcelSheet"));
        },
    );
    with_app(
        || program(Some(Module::Map)),
        |harita| {
            assert!(panel_commands(harita, "ctx.area", "Yaz ve tara").is_some());
            let text = panel_commands(harita, "ctx.text", "Yazı").expect("a text's Yazı");
            assert!(text.contains(&"style.textStyles"));
            let writing =
                panel_commands(harita, "plan", "Plan yazıları").expect("Plan › Plan yazıları");
            for id in ["map.edgeLengths", "processing.model.builtin.parcelSheet"] {
                assert!(writing.contains(&id), "{id}");
                assert!(harita.offers(id), "Harita offers {id}");
            }
            assert!(harita.offers_model("builtin.parcelSheet"));
        },
    );
}

/// KentOS Mimari has no tab of surveying: its queries (Mesafe ölç, Alan hesapla, Koordinat oku, Açı ölç, Bilgi) are
/// Görünüm's Sorgu, one button that runs the last used as AutoCAD's Home › Utilities › Measure (Giriş has no room
/// for a panel at 1440 px), and Komut ara says so. KentOS Harita keeps its Ölçme, Sorgu first.
#[test]
fn kentos_mimari_has_no_olcme_and_its_queries_are_gorunums() {
    let query = [
        "tool.measure",
        "tool.area",
        "crs.query",
        "tool.measureAngle",
        "tool.identify",
    ];
    with_app(
        || cad_in(Module::Architecture, CadWork::Architecture),
        |app| {
            let tabs = ids(app.ribbon_tabs());
            assert!(!tabs.contains(&"survey"), "{tabs:?}");
            assert!(!app.ribbon_tabs().any(|t| t.label == "Ölçme"));
            assert_eq!(
                panel_commands(app, "view", "Sorgu").expect("Görünüm › Sorgu"),
                query
            );
            let sorgu = app
                .ribbon_tabs()
                .find(|t| t.id == "view")
                .and_then(|t| t.panels.iter().find(|p| p.label == "Sorgu"))
                .expect("Görünüm › Sorgu");
            assert!(
                matches!(sorgu.items.as_slice(), [Item::Split { key, entries, .. }]
                    if *key == "archQuery" && entries[0].title == "Mesafe ölç"),
                "one split: {:?}",
                sorgu.items
            );
            assert_eq!(panel_commands(app, "home", "Sorgu"), None);
            for id in query {
                assert!(app.offers(id), "Mimari offers {id}");
            }
            let _ = app.update(Message::RibbonSearch("mesafe ölç".into()));
            let rows = app.search_rows();
            let row = rows
                .iter()
                .find(|r| r.title == "Mesafe ölç")
                .expect("Komut ara finds Mesafe ölç");
            assert_eq!(row.detail, "Görünüm › Sorgu");
            let _ = app.update(Message::Run("tool.measure"));
            assert_eq!(app.session.tool_id(), "measure");
        },
    );
    // KentOS Harita: Ölçme with its Sorgu first, none on Görünüm.
    with_app(
        || program(Some(Module::Map)),
        |harita| {
            let survey = harita
                .ribbon_tabs()
                .find(|t| t.id == "survey")
                .expect("Harita's Ölçme");
            assert_eq!((survey.label, survey.panels[0].label), ("Ölçme", "Sorgu"));
            assert!(survey.panels[0].commands().contains(&"tool.stationOffset"));
            assert_eq!(panel_commands(harita, "view", "Sorgu"), None);
        },
    );
}

/// Ekle's picture panel is named for what it holds: KentOS Mimari's pictures and point clouds, no raster; KentOS
/// Harita's pictures, rasters and point clouds; STAC is KentOS GIS's.
#[test]
fn ekle_s_picture_panel_is_named_for_what_it_holds() {
    let (named, rasters) = with_app(
        || program(Some(Module::Architecture)),
        |app| {
            (
                panel_commands(app, "insert", "Resim ve nokta bulutu"),
                panel_commands(app, "insert", "Resim ve raster"),
            )
        },
    );
    assert_eq!(
        named.expect("Mimari's panel"),
        ["tool.imageInsert", "pointcloud.add", "tool.imageClip"]
    );
    assert_eq!(rasters, None);
    let harita = with_app(
        || program(Some(Module::Map)),
        |app| panel_commands(app, "insert", "Resim ve raster"),
    )
    .expect("Harita's panel");
    for id in [
        "tool.imageInsert",
        "raster.add",
        "pointcloud.add",
        "raster.georef",
        "raster.gpkgRead",
    ] {
        assert!(harita.contains(&id), "{id}");
    }
    assert!(!harita.contains(&"raster.stac"));
    // A CAD drawing not asked its work shows the whole panel by its own name.
    let whole = with_app(
        || cad_drawing(None),
        |app| panel_commands(app, "insert", "Resim ve raster"),
    );
    assert!(whole.is_some_and(|ids| ids.contains(&"raster.add")));
}

/// The labels of each Ölçülendirme ▾ a program's ribbon shows.
fn dimension_entries(app: &App) -> Vec<Vec<&'static str>> {
    app.ribbon_tabs()
        .flat_map(|t| t.panels.iter())
        .flat_map(|p| p.items.iter())
        .filter_map(|i| match i {
            Item::Split { key, entries, .. } if *key == "dimension" => {
                Some(entries.iter().map(|e| e.label).collect())
            }
            _ => None,
        })
        .collect()
}

/// Ölçülendirme ▾'s Semt, a bearing (docs/adr/0147), is a map's: KentOS Mimari's splits have no Semt entry, its other
/// kinds stay; KentOS Harita's and a CAD drawing not asked its work have it.
#[test]
fn the_semt_dimension_is_kentos_haritas() {
    let mimari = with_app(
        || program(Some(Module::Architecture)),
        |app| dimension_entries(app),
    );
    assert_eq!(mimari.len(), 2, "Giriş › Açıklama and Açıklama › Ölçü");
    for labels in &mimari {
        assert!(
            !labels.contains(&"Semt") && labels.contains(&"Eğim"),
            "{labels:?}"
        );
    }
    let harita = with_app(|| program(Some(Module::Map)), |app| dimension_entries(app));
    let whole = with_app(|| cad_drawing(None), |app| dimension_entries(app));
    for splits in [harita, whole] {
        assert_eq!(splits.len(), 2);
        assert!(
            splits.iter().all(|labels| labels.contains(&"Semt")),
            "{splits:?}"
        );
    }
}

/// A model of the user's, Parsel ölçü yazıları's copy whose second step runs `tool`, kept in the program's library.
fn add_model(app: &mut App, id: &str, label: &str, tool: &str) {
    let mut model = app
        .processing
        .registry
        .model("builtin.parcelSheet")
        .cloned()
        .expect("the built-in model");
    model.id = id.into();
    model.label = label.into();
    model.steps[1].tool = tool.into();
    app.processing
        .registry
        .save_model(model)
        .expect("a user's model");
}

/// A model is offered where the program offers every tool its steps run: a model of the user's that fills a DEM's
/// sinks (hydrology) is KentOS GIS's, not KentOS Harita's; one that writes edge lengths is both's (docs/adr/0802).
#[test]
fn a_model_is_offered_where_every_tool_of_its_steps_is() {
    for (module, sinks, edges) in [
        (Some(Module::Map), false, true),
        (Some(Module::Gis), true, true),
        (Some(Module::Architecture), false, false),
        (None, true, true),
    ] {
        let (offered, listed, typed) = with_app(
            || program(module),
            |app| {
                add_model(app, "user.havzam", "Havzam", "hydrology.fill");
                add_model(
                    app,
                    "user.kenarlarim",
                    "Kenarlarım",
                    "annotation.edgeLengths",
                );
                let offered = (
                    app.offers_model("user.havzam"),
                    app.offers_model("user.kenarlarim"),
                );
                // The ribbon's model buttons and the command line's names read the user's models; Komut ara too.
                let listed = app.user_models().any(|m| m.id == "user.havzam");
                let typed = found(app, "havzam") == ["processing.model.user.havzam"];
                (offered, listed, typed)
            },
        );
        assert_eq!(offered, (sinks, edges), "{module:?}");
        assert_eq!((listed, typed), (sinks, sinks), "{module:?}");
    }
    // Typed in KentOS Harita, its name opens nothing.
    let opened = with_app(
        || cad_in(Module::Map, CadWork::Map),
        |app| {
            add_model(app, "user.havzam", "Havzam", "hydrology.fill");
            let _ = app.run_typed("Havzam");
            app.dialog == Some(crate::app::Dialog::Processing)
        },
    );
    assert!(!opened);
}

/// The tabs of a program's ribbon, its selection's too, that do not fit 1100 px at this text size (DESIGN.md
/// §7.3.1).
fn misfits(app: &mut App, size: u32) -> Vec<String> {
    use kentos_ui::widget::ribbon::{self, Group};
    let _ = app
        .settings
        .choose(&[("appearance.textSize", serde_json::json!(size))]);
    app.apply_settings();
    let tabs: Vec<&'static Tab> = app.ribbon_tabs().chain(app.contextual_tabs()).collect();
    let mut wrong = Vec::new();
    for tab in tabs {
        let groups: Vec<Group<'static, Message>> = tab
            .panels
            .iter()
            .filter_map(|p| app.ribbon_group(tab.id, p))
            .collect();
        let widths: Vec<[f32; 4]> = groups.iter().map(Group::widths).collect();
        let keep: Vec<bool> = groups.iter().map(Group::keeps).collect();
        let (levels, overflow) = ribbon::fit(&widths, &keep, 1100.0);
        if overflow {
            wrong.push(format!(
                "{:?} {} {size} px: {levels:?}",
                app.program, tab.id
            ));
        }
    }
    wrong
}

/// Every program's ribbon fits a 1100 px window at 13 and 18 px text without scrolling (DESIGN.md §7.3.1), its
/// selection's tabs too: KentOS Mimari's Görünüm with its Sorgu, Ekle with its pictures (docs/adr/0802).
#[test]
fn each_programs_ribbon_fits_1100_pixels_at_small_and_large_text() {
    use kentos_ui::theme::typography::{self, Typography};
    let _typography = crate::appearance::tests::TYPOGRAPHY.lock();
    typography::load();
    let mut wrong = Vec::new();
    for size in [13, 18] {
        wrong.extend(with_app(
            || cad_in(Module::Architecture, CadWork::Architecture),
            |app| misfits(app, size),
        ));
        wrong.extend(with_app(
            || cad_in(Module::Map, CadWork::Map),
            |app| misfits(app, size),
        ));
        wrong.extend(with_app(gis_drawing, |app| misfits(app, size)));
    }
    typography::set(Typography::DEFAULT);
    assert!(wrong.is_empty(), "does not fit 1100 px: {wrong:?}");
}

/// One picture of a CAD program's tab at a window size and text size.
fn ribbon_picture(
    module: Module,
    work: CadWork,
    tab: &'static str,
    size: iced::Size,
    text: u32,
    file: &Path,
) {
    use kentos_ui::snapshot::Snapshot;
    let mut app = cad_in(module, work);
    let _ = app
        .settings
        .choose(&[("appearance.textSize", serde_json::json!(text))]);
    app.apply_settings();
    let _ = app.update(Message::WindowResized(size));
    app.choose_tab(tab);
    let mut snapshot = Snapshot::new(size).expect("a renderer");
    snapshot.settle(&mut app, App::view, &mut |app, message| {
        let _ = app.update(message);
    });
    snapshot
        .render(app.view(), &app.theme())
        .save(file)
        .expect("writes the picture");
    println!("{}", file.display());
}

/// Pictures of the tabs these decisions changed, for the owner: KentOS Mimari's Görünüm (its Sorgu) and Ekle (Resim
/// ve nokta bulutu), KentOS Harita's Ölçme and Ekle; 1440×900 at 13 px, 1100×650 at 13 and 18 px, dark;
/// `.run/shots/ayrim-serit-*`. Not a correctness test:
///
/// ```text
/// cargo test -p kentos-desktop --lib module_tests::ribbon_pictures -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn ribbon_pictures() {
    use iced::Size;
    let _typography = crate::appearance::tests::TYPOGRAPHY.lock();
    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (module, work, word, tabs) in [
        (
            Module::Architecture,
            CadWork::Architecture,
            "mimari",
            ["view", "insert"],
        ),
        (Module::Map, CadWork::Map, "harita", ["survey", "insert"]),
    ] {
        for (size, text, tag) in [
            (Size::new(1440.0, 900.0), 13, ""),
            (Size::new(1100.0, 650.0), 13, "-dar"),
            (Size::new(1100.0, 650.0), 18, "-dar-buyuk"),
        ] {
            for tab in tabs {
                let file = out.join(format!("ayrim-serit-{word}-{tab}{tag}.png"));
                ribbon_picture(module, work, tab, size, text, &file);
            }
        }
    }
    kentos_ui::theme::typography::set(kentos_ui::theme::typography::Typography::DEFAULT);
}

/// Each program's commands with their titles and the ribbon places that hold them, as Markdown, for a review
/// of where every command belongs (docs/adr/0802); not run by default:
///
/// ```text
/// KENTOS_MODULE_DUMP=/tmp/moduller.md cargo test -p kentos-desktop --lib module_tests::dump_module_commands -- --ignored
/// ```
#[test]
#[ignore = "a review's dump, run by hand"]
fn dump_module_commands() {
    use std::fmt::Write;
    let Ok(out) = std::env::var("KENTOS_MODULE_DUMP") else {
        return;
    };
    let mut text = String::new();
    for module in Module::ALL {
        let app = program(Some(module));
        let tabs: Vec<&Tab> = app.ribbon_tabs().chain(app.contextual_tabs()).collect();
        let offered: Vec<&crate::catalog::Command> = crate::catalog::catalog()
            .commands()
            .iter()
            .filter(|c| app.offers(c.id))
            .collect();
        let _ = writeln!(text, "# {} ({} komut)\n", module.title(), offered.len());
        for c in offered {
            let places: Vec<String> = tabs
                .iter()
                .flat_map(|t| {
                    t.panels
                        .iter()
                        .filter(|p| p.commands().contains(&c.id) || p.overflow.contains(&c.id))
                        .map(move |p| format!("{} › {}", t.label, p.label))
                })
                .collect();
            let places = if places.is_empty() {
                "(şeritte değil)".to_owned()
            } else {
                places.join("; ")
            };
            let _ = writeln!(text, "- `{}` {} — {}", c.id, c.title, places);
        }
        text.push('\n');
    }
    std::fs::write(&out, text).expect("the dump is written");
}
