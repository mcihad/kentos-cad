//! What each KentOS program shows of its window (module_surfaces.rs, docs/adr/0802 §5): the bottom panel's tabs,
//! the dock's panels, the status bar's cells, Proje ayarları' and Uygulama ayarları' sections, the layer menu's
//! entries, the application menu's formats, Kaynaklar's sections, the hover card, the sheet's and the cloud's types
//! and the files dropped on the window. A tab or panel another program left in the shared layout is not opened
//! here, an opener of what a program has not falls back to Komut geçmişi and says where it is; kentos-cad shows
//! everything. Test code only; no program is started.

use std::path::{Path, PathBuf};

use iced::Color;
use kentos_commands::{CadWork, ProjectType, Workspace};
use kentos_launch::Module;
use kentos_ui::widget::Menu;
use kentos_ui::widget::docking::{Side, Slot};
use serde_json::json;

use crate::app::{App, Message, Panel};
use crate::bottom::BottomTab;
use crate::files_testing::{app_with_drawing, drawing, drive, last_said, scratch};
use crate::layout::{FILE_NAME, Keeper};
use crate::module_surfaces::{AppSetting, Cell, LayerEntry, PropertyRows, SourceSection};
use crate::project::settings::Section as ProjectSection;
use crate::recovery::Recovery;
use crate::settings::Settings;
use crate::settings_sections::Section as SettingsSection;

const MIMARI: Option<Module> = Some(Module::Architecture);
const HARITA: Option<Module> = Some(Module::Map);
const GIS: Option<Module> = Some(Module::Gis);
/// kentos-cad, then the three modules' programs.
const PROGRAMS: [Option<Module>; 4] = [None, MIMARI, HARITA, GIS];

/// A program as main.rs starts it (none: kentos-cad), its settings and connections in memory, starting no other.
fn started(module: Option<Module>) -> App {
    kentos_postgis::connections::keep_in_memory();
    let (mut app, _) = App::start_in(module, None, Settings::memory(), Recovery::off());
    app.db_store = Box::leak(Box::new(
        kentos_postgis::connections::ConnectionStore::open(None),
    ));
    app.hand_over = |m, _| Ok(m.program().to_owned());
    app
}

/// The sample drawing (a CAD drawing for a CAD module, so its ribbon is its own) in `module`'s program, its
/// surfaces kept as after any message.
fn drawn(module: Option<Module>) -> App {
    let mut app = app_with_drawing();
    app.program = module;
    app.hand_over = |m, _| Ok(m.program().to_owned());
    if let Some(m) = module {
        let doc = app.document.as_mut().expect("a drawing");
        let mut s = doc.model.settings().clone();
        s.workspace = Some(crate::program::type_of(m));
        s.cad_work = crate::program::works_of(m).first().copied();
        doc.model.set_settings(s);
    }
    // A message, its tasks run: the surfaces are kept and the drawing settles as after any.
    let task = app.update(Message::Swallowed);
    drive(&mut app, task);
    app
}

/// `module`'s program with the layout kept in `dir`, as main.rs opens it.
fn opened_with(module: Option<Module>, dir: &Path) -> App {
    let mut app = started(module);
    app.layout = Keeper::open(dir);
    app.apply_layout();
    app
}

/// A layout kept with `fields`, in a folder of its own.
fn kept(name: &str, fields: serde_json::Value) -> PathBuf {
    let dir = scratch(&format!("yuzey-{name}"));
    std::fs::write(dir.join(FILE_NAME), fields.to_string()).expect("a kept layout");
    dir
}

/// The panel in front of Katmanlar' slot.
fn in_front(app: &App) -> Option<Panel> {
    let Some(Slot::Docked(side, i)) = app.docks.slot(Panel::Layers) else {
        return None;
    };
    app.docks.stacks(side).get(i).and_then(|s| s.active())
}

/// A menu's labels as its debug text writes them.
fn menu_text(menu: &Menu<Message>) -> String {
    format!("{menu:?}")
}

/// Whether a menu's labels hold one starting with `label` (“Alanlar… (3)” for `Alanlar…`).
fn has(menu: &str, label: &str) -> bool {
    menu.contains(&format!("label: \"{label}"))
}

#[test]
fn each_program_has_its_own_bottom_tabs() {
    use BottomTab::*;
    assert_eq!(started(None).bottom_tabs(), BottomTab::ALL);
    assert_eq!(
        started(MIMARI).bottom_tabs(),
        [History, Coords, Search, Audit, Messages, Python]
    );
    assert_eq!(
        started(HARITA).bottom_tabs(),
        [
            History,
            Coords,
            Points,
            Table,
            Statistics,
            Search,
            Address,
            Topology,
            ServiceInfo,
            Messages,
            Python
        ]
    );
    let gis = started(GIS).bottom_tabs();
    assert_eq!(gis.len(), BottomTab::ALL.len() - 1);
    assert!(!gis.contains(&Audit) && gis.contains(&Chart));
    // Komut geçmişi, Koordinat listesi, Uyarılar and Python are every program's.
    for program in PROGRAMS {
        let app = started(program);
        for tab in [History, Coords, Messages, Python] {
            assert!(app.shows_bottom_tab(tab), "{program:?} {tab:?}");
        }
    }
}

#[test]
fn a_tab_a_program_has_not_never_shows() {
    let mut app = drawn(MIMARI);
    // Clicked (as a trace would), F2 on it, opened directly: Komut geçmişi.
    let _ = app.update(Message::BottomTab(BottomTab::Table));
    assert_eq!(app.bottom_tab, BottomTab::History);
    assert!(app.command_expanded);
    app.command_expanded = false;
    app.bottom_tab = BottomTab::Chart;
    app.toggle_bottom();
    assert_eq!(app.bottom_tab, BottomTab::History);
    app.show_bottom(BottomTab::Audit);
    assert_eq!(app.bottom_tab, BottomTab::Audit, "Mimari's own");
    // A tab set from outside is put right after the next message.
    app.bottom_tab = BottomTab::Address;
    let _ = app.update(Message::Swallowed);
    assert_eq!(app.bottom_tab, BottomTab::History);
    // KentOS GIS has no Denetim.
    let mut gis = drawn(GIS);
    gis.show_bottom(BottomTab::Audit);
    assert_eq!(gis.bottom_tab, BottomTab::History);
    gis.show_bottom(BottomTab::Chart);
    assert_eq!(gis.bottom_tab, BottomTab::Chart);
}

#[test]
fn an_opener_of_a_tab_a_program_has_not_says_where_it_is() {
    let mut app = drawn(MIMARI);
    app.open_feature_table();
    assert_eq!(app.bottom_tab, BottomTab::History);
    assert!(app.command_expanded);
    assert_eq!(
        last_said(&app),
        "“Tablo” KentOS Mimari'de yok; KentOS Harita'da ve KentOS GIS'te var."
    );
    app.open_statistics(None);
    assert_eq!(app.bottom_tab, BottomTab::History);
    // Veride ara is a building's too: the room names, the doors' numbers.
    let _ = app.open_data_search();
    assert_eq!(app.bottom_tab, BottomTab::Search);
    let _ = app.open_address_search();
    assert!(last_said(&app).starts_with("“Adres” KentOS Mimari'de yok"));
    let _ = app.open_topology_check();
    assert!(last_said(&app).starts_with("“Topoloji” KentOS Mimari'de yok"));
    app.start_service_info();
    assert!(last_said(&app).starts_with("“Servis bilgisi” KentOS Mimari'de yok"));
    assert!(!app.service_info.picking, "no point is asked for");
    assert_eq!(app.bottom_tab, BottomTab::History);
    // Grafik is KentOS GIS's alone; İstatistik is Harita's too.
    let mut harita = drawn(HARITA);
    harita.open_chart(None, None, None);
    assert_eq!(harita.bottom_tab, BottomTab::History);
    assert_eq!(
        last_said(&harita),
        "“Grafik” KentOS Harita'da yok; KentOS GIS'te var."
    );
    harita.open_statistics(None);
    assert_eq!(harita.bottom_tab, BottomTab::Statistics);
    // kentos-cad opens every one.
    let mut all = drawn(None);
    all.open_chart(None, None, None);
    assert_eq!(all.bottom_tab, BottomTab::Chart);
    all.open_feature_table();
    assert_eq!(all.bottom_tab, BottomTab::Table);
}

#[test]
fn a_tab_another_program_left_is_komut_gecmisi_here_and_stays_kept_for_it() {
    let dir = kept(
        "alt",
        json!({ "bottomExpanded": true, "bottomTab": "chart", "dockTab": "blocks" }),
    );
    let gis = opened_with(GIS, &dir);
    assert_eq!(gis.bottom_tab, BottomTab::Chart);
    // KentOS GIS has no Bloklar: Katmanlar in front.
    assert_eq!(in_front(&gis), Some(Panel::Layers));
    let mut harita = opened_with(HARITA, &dir);
    assert_eq!(harita.bottom_tab, BottomTab::History);
    assert!(harita.command_expanded);
    assert_eq!(in_front(&harita), Some(Panel::Blocks));
    // Its own fallback is not written over the other program's tab.
    let _ = harita.update(Message::Swallowed);
    assert_eq!(harita.layout.kept()["bottomTab"], "chart");
    // A tab of its own chosen is kept for every program.
    let _ = harita.update(Message::BottomTab(BottomTab::Coords));
    assert_eq!(harita.layout.kept()["bottomTab"], "coords");
    let mut gis = opened_with(GIS, &dir);
    let _ = gis.update(Message::Swallowed);
    assert_eq!(
        gis.layout.kept()["dockTab"],
        "blocks",
        "Bloklar stays kept for the others"
    );
    // KentOS Mimari has neither Tablo nor Kaynaklar; kentos-cad has both.
    let both = json!({ "bottomExpanded": true, "bottomTab": "table", "dockTab": "sources" });
    let mimari = opened_with(MIMARI, &kept("mimari", both.clone()));
    assert_eq!(mimari.bottom_tab, BottomTab::History);
    assert_eq!(in_front(&mimari), Some(Panel::Layers));
    let cad = opened_with(None, &kept("cad", both));
    assert_eq!(cad.bottom_tab, BottomTab::Table);
    assert_eq!(in_front(&cad), Some(Panel::Sources));
}

#[test]
fn each_program_docks_its_own_panels() {
    use Panel::*;
    let expected: [(Option<Module>, &[Panel]); 4] = [
        (None, &Panel::ALL),
        (MIMARI, &[Layers, Blocks, Templates, Properties]),
        (
            HARITA,
            &[Layers, Processing, Blocks, Templates, Sources, Properties],
        ),
        (GIS, &[Layers, Processing, Templates, Sources, Properties]),
    ];
    for (program, panels) in expected {
        let app = started(program);
        for panel in Panel::ALL {
            assert_eq!(
                app.docks.contains(panel),
                panels.contains(&panel),
                "{program:?} {panel:?}"
            );
            assert_eq!(app.shows_panel(panel), panels.contains(&panel));
        }
        assert_eq!(in_front(&app), Some(Layers), "{program:?}");
        assert!(app.right_panel_shown());
    }
}

#[test]
fn f4_and_the_openers_keep_to_the_programs_panels() {
    let mut gis = drawn(GIS);
    assert!(
        !gis.docks.contains(Panel::Blocks),
        "closed when the program is set"
    );
    // F4 hides every panel, Kaynaklar too, and shows them again; Bloklar never.
    gis.toggle_right_panel();
    assert!(!gis.right_panel_shown());
    assert!(!gis.docks.is_shown(Panel::Sources));
    gis.toggle_right_panel();
    assert!(gis.right_panel_shown());
    assert!(!gis.docks.contains(Panel::Blocks));
    // With the panels closed one by one, F4 opens the program's own, Katmanlar in front.
    gis.hidden_docks = None;
    for panel in Panel::ALL {
        gis.docks.close(panel);
    }
    gis.toggle_right_panel();
    assert!(gis.docks.is_shown(Panel::Properties));
    assert!(!gis.docks.contains(Panel::Blocks));
    assert_eq!(in_front(&gis), Some(Panel::Layers));
    // Bloklar asked for in KentOS GIS: said, not docked.
    gis.show_blocks_panel();
    assert!(!gis.docks.contains(Panel::Blocks));
    assert_eq!(
        last_said(&gis),
        "“Bloklar” KentOS GIS'te yok; KentOS Mimari'de ve KentOS Harita'da var."
    );
    // Kaynaklar in KentOS Mimari: the same.
    let mut mimari = drawn(MIMARI);
    mimari.show_sources();
    assert!(!mimari.docks.contains(Panel::Sources));
    assert!(last_said(&mimari).starts_with("“Kaynaklar” KentOS Mimari'de yok"));
    // A panel docked by another way (a command whose file this does not know) goes after the next message.
    mimari.docks.show(Panel::Processing, Side::Right);
    let _ = mimari.update(Message::Swallowed);
    assert!(!mimari.docks.contains(Panel::Processing));
    // KentOS Harita's own.
    let mut harita = drawn(HARITA);
    harita.show_blocks_panel();
    assert_eq!(in_front(&harita), Some(Panel::Blocks));
    harita.show_sources();
    assert_eq!(in_front(&harita), Some(Panel::Sources));
}

#[test]
fn each_program_has_its_own_status_cells() {
    use Cell::*;
    let expected: [(Option<Module>, &[Cell]); 4] = [
        (
            None,
            &[MapCrs, SecondCrs, Topology, Overlap, Scenario, Level],
        ),
        (MIMARI, &[Level]),
        (HARITA, &[MapCrs, SecondCrs, Topology, Overlap]),
        (GIS, &[MapCrs, SecondCrs, Topology, Overlap, Scenario]),
    ];
    for (program, cells) in expected {
        let app = started(program);
        for cell in [MapCrs, SecondCrs, Topology, Overlap, Scenario, Level] {
            assert_eq!(
                app.shows_cell(cell),
                cells.contains(&cell),
                "{program:?} {cell:?}"
            );
        }
        // The lamps follow: every other drafting aid is every program's.
        for (id, _) in crate::view::STATUS_AIDS {
            let own = match id {
                "draft.topology" => cells.contains(&Topology),
                "draft.overlap" => cells.contains(&Overlap),
                _ => true,
            };
            assert_eq!(app.shows_aid(id), own, "{program:?} {id}");
        }
    }
}

#[test]
fn the_cells_with_values_follow_the_program() {
    // A second system: its cell in a map's programs, none in KentOS Mimari, nor its width.
    let mut harita = drawn(HARITA);
    let _ = harita.update(Message::SecondCrs(crate::second_crs::Event::Choose(Some(
        2322,
    ))));
    assert!(harita.second_cell(None).is_some());
    assert!(harita.second_width() > 0.0);
    let menu = menu_text(&harita.crs_cell_menu());
    assert!(has(&menu, "İkinci sistem") && has(&menu, "Harita sistemi"));
    let mut mimari = drawn(MIMARI);
    let _ = mimari.update(Message::SecondCrs(crate::second_crs::Event::Choose(Some(
        2322,
    ))));
    assert!(mimari.second_cell(None).is_none());
    assert_eq!(mimari.second_width(), 0.0);
    let menu = menu_text(&mimari.crs_cell_menu());
    assert!(!has(&menu, "İkinci sistem") && !has(&menu, "Harita sistemi"));
    // Levels: the Kat cell and the tools' Kat row in KentOS Mimari alone.
    for (program, shown) in [(MIMARI, true), (HARITA, false), (GIS, false), (None, true)] {
        let mut app = drawn(program);
        let doc = app.document.as_mut().expect("a drawing");
        let mut s = doc.model.settings().clone();
        s.arch.get_or_insert_with(Default::default).levels = vec![kentos_commands::ArchLevel {
            id: "zemin".into(),
            name: "Zemin kat".into(),
            elevation: 0.0,
            height: None,
        }];
        doc.model.set_settings(s);
        assert_eq!(app.level_words().is_some(), shown, "{program:?}");
        assert_eq!(app.level_setting("wall").is_some(), shown, "{program:?}");
    }
}

#[test]
fn each_program_has_its_own_settings_sections() {
    use ProjectSection::*;
    assert_eq!(started(None).project_sections(), ProjectSection::ALL);
    assert_eq!(
        started(MIMARI).project_sections(),
        [General, Units, Crs, Variables, Arch]
    );
    for program in [HARITA, GIS] {
        assert_eq!(
            started(program).project_sections(),
            [General, Units, Crs, Heights, Survey, Variables],
            "{program:?}"
        );
    }
    // Uygulama ayarları: Yeni projeler is not KentOS Mimari's; the type is kentos-cad's alone.
    let rows = [
        AppSetting::Geographic,
        AppSetting::FormOnCreate,
        AppSetting::NewProjects,
        AppSetting::NewProjectType,
    ];
    for (program, shown) in [
        (None, &rows[..]),
        (MIMARI, &[][..]),
        (HARITA, &rows[..3]),
        (GIS, &rows[..3]),
    ] {
        let app = started(program);
        for row in rows {
            assert_eq!(
                app.shows_setting(row),
                shown.contains(&row),
                "{program:?} {row:?}"
            );
        }
        for section in SettingsSection::ALL {
            let own = section != SettingsSection::NewProjects || program != MIMARI;
            assert_eq!(
                app.shows_settings_section(section),
                own,
                "{program:?} {section:?}"
            );
        }
    }
}

#[test]
fn a_settings_window_opens_on_a_section_the_program_has() {
    let open_app = || {
        Message::Project(Box::new(crate::project::Event::Settings(
            crate::project::settings::Event::OpenApp,
        )))
    };
    let mut mimari = drawn(MIMARI);
    // Proje ayarları' Uygulama ayarlarını aç: Görünüm in KentOS Mimari, Yeni projeler elsewhere.
    let _ = mimari.project_command("file.settings");
    let _ = mimari.update(open_app());
    assert_eq!(
        mimari.settings_draft.as_ref().map(|d| d.section),
        Some(SettingsSection::Appearance)
    );
    let mut gis = drawn(GIS);
    let _ = gis.project_command("file.settings");
    let _ = gis.update(open_app());
    assert_eq!(
        gis.settings_draft.as_ref().map(|d| d.section),
        Some(SettingsSection::NewProjects)
    );
}

/// The layer menu's entries a program shows: the special ones by the profile, the rest everywhere.
#[test]
fn each_program_has_its_own_layer_menu() {
    let special = [
        ("Zaman ayarları…", Some(LayerEntry::Time)),
        ("Koordinat sistemi", Some(LayerEntry::Crs)),
        ("Stil yükle…", Some(LayerEntry::StyleFiles)),
        ("Görünüşü SLD olarak kaydet…", Some(LayerEntry::Sld)),
        ("QML'den görünüş al…", Some(LayerEntry::Qml)),
        ("MapLibre stilinden görünüş al…", Some(LayerEntry::MapLibre)),
        ("Veri kümesi bilgisi…", Some(LayerEntry::Dataset)),
        ("Katman türü", Some(LayerEntry::Kind)),
        // Every program's.
        ("Alanlar…", None),
        ("Formlar…", None),
        ("Süzgeç…", None),
        ("Etiketler…", None),
        ("Katman stili…", None),
        ("Değişikliğin etkisi…", None),
        ("Kenet", None),
        ("Yeniden adlandır", None),
    ];
    for program in PROGRAMS {
        let app = drawn(program);
        let doc = app.document.as_ref().expect("a drawing");
        let layers = doc.model.layers();
        let node = layers
            .get(layers.active())
            .expect("the active layer")
            .clone();
        let menu = menu_text(&app.layer_menu(&node, true));
        for (label, entry) in special {
            let own = entry.is_none_or(|e| app.shows_layer_entry(e));
            assert_eq!(has(&menu, label), own, "{program:?} {label}");
        }
        // İstatistik and Grafik as the bottom panel's tabs.
        assert_eq!(
            has(&menu, "İstatistik"),
            app.shows_bottom_tab(BottomTab::Statistics),
            "{program:?}"
        );
        assert_eq!(has(&menu, "Grafik"), app.shows_bottom_tab(BottomTab::Chart));
    }
    // The profiles: KentOS Mimari none, Harita no time, scenarios or layer kinds, GIS all.
    let mimari = started(MIMARI);
    let harita = started(HARITA);
    let gis = started(GIS);
    for entry in [
        LayerEntry::Time,
        LayerEntry::Scenario,
        LayerEntry::Crs,
        LayerEntry::StyleFiles,
        LayerEntry::Sld,
        LayerEntry::Qml,
        LayerEntry::MapLibre,
        LayerEntry::Dataset,
        LayerEntry::Kind,
        LayerEntry::Service,
        LayerEntry::Source,
    ] {
        assert!(!mimari.shows_layer_entry(entry), "{entry:?}");
        assert!(gis.shows_layer_entry(entry), "{entry:?}");
        let map = !matches!(
            entry,
            LayerEntry::Time | LayerEntry::Scenario | LayerEntry::Kind
        );
        assert_eq!(harita.shows_layer_entry(entry), map, "{entry:?}");
    }
}

#[test]
fn each_program_lists_its_own_file_formats() {
    let drawings = [
        "file.import.dwg",
        "file.import.dxf",
        "file.import.ncz",
        "file.export.dwg",
        "file.export.dxf",
        "file.export.pdf",
    ];
    let gis_files = [
        "file.import.ncn",
        "file.import.shp",
        "file.import.geojson",
        "file.import.kml",
        "file.import.gnss",
        "file.export.ncn",
        "file.export.geojson",
        "file.export.kml",
        "file.export.gpx",
    ];
    for program in PROGRAMS {
        let app = started(program);
        let (imports, exports) = (app.listed_formats(false), app.listed_formats(true));
        for id in drawings {
            assert!(
                imports.contains(&id) || exports.contains(&id),
                "{program:?} {id}"
            );
        }
        for id in gis_files {
            let listed = imports.contains(&id) || exports.contains(&id);
            assert_eq!(listed, program != MIMARI, "{program:?} {id}");
        }
    }
    assert_eq!(
        started(MIMARI).listed_formats(false),
        ["file.import.dwg", "file.import.dxf", "file.import.ncz"]
    );
    assert_eq!(
        started(MIMARI).listed_formats(true),
        ["file.export.dwg", "file.export.dxf", "file.export.pdf"]
    );
}

#[test]
fn the_program_names_itself() {
    for (program, title) in [
        (None, "KentOS CAD"),
        (MIMARI, "KentOS Mimari"),
        (HARITA, "KentOS Harita"),
        (GIS, "KentOS GIS"),
    ] {
        assert_eq!(started(program).program_title(), title);
    }
}

#[test]
fn kaynaklar_and_the_properties_follow_the_program() {
    for (program, own) in [(None, true), (MIMARI, false), (HARITA, true), (GIS, true)] {
        let app = drawn(program);
        for section in [SourceSection::Databases, SourceSection::Services] {
            assert_eq!(app.shows_source_section(section), own, "{program:?}");
        }
        for rows in [PropertyRows::Crs, PropertyRows::Services] {
            assert_eq!(app.shows_property_rows(rows), own, "{program:?}");
        }
        // Çizim's coordinate system with nothing selected.
        let doc = app.document.as_ref().expect("a drawing");
        let panel = app.properties_panel(doc);
        let rows: Vec<&str> = panel.sections[0]
            .rows
            .iter()
            .map(|r| r.label.as_ref())
            .collect();
        assert_eq!(
            rows.contains(&"Koordinat sistemi"),
            own,
            "{program:?} {rows:?}"
        );
        assert_eq!(rows.contains(&"SRID"), own);
        assert!(rows.contains(&"Çizim ölçeği"));
        // The hover card's parcel rows (the sample's parcel 4: Ada 101, Parsel 7).
        let parcel = doc.model.get(kentos_domain::Slot(4)).expect("the parcel");
        let card = crate::hover_card::card_with(
            parcel,
            &doc.model,
            &kentos_interaction::Format::of(doc.settings()),
            |_| Color::BLACK,
            app.shows_parcel_rows(),
        );
        assert_eq!(card.title == "Parsel 7", own, "{program:?} {}", card.title);
        assert_eq!(card.rows.iter().any(|r| r.0 == "Ada"), own);
    }
}

#[test]
fn the_sheet_and_the_cloud_start_with_the_programs_type() {
    for (program, sheet, cloud, upload) in [
        (None, None, None, ProjectType::Cad),
        (
            MIMARI,
            Some(ProjectType::Architecture),
            Some(ProjectType::Architecture),
            ProjectType::Architecture,
        ),
        (
            HARITA,
            Some(ProjectType::ZoningPlan),
            None,
            ProjectType::Cad,
        ),
        (
            GIS,
            Some(ProjectType::Gis),
            Some(ProjectType::Gis),
            ProjectType::Gis,
        ),
    ] {
        let mut app = drawn(program);
        assert_eq!(app.sheet_context().project_type, sheet, "{program:?}");
        assert_eq!(app.cloud_kind(), cloud);
        assert_eq!(app.upload_kind(), upload);
        let _ = app.open_catalog();
        assert_eq!(
            app.cloud.catalog.as_ref().and_then(|c| c.kind),
            cloud,
            "{program:?}"
        );
    }
}

/// The sample drawing as a project of this type and work, saved in `dir` as `name`.
fn saved_as(dir: &Path, name: &str, kind: Workspace, work: Option<CadWork>) -> PathBuf {
    let mut doc = drawing(0);
    let mut s = doc.settings().clone();
    s.workspace = Some(kind);
    s.cad_work = work;
    doc.model.set_settings(s);
    let path = dir.join(name);
    crate::document::write(&doc.model.to_snapshot_v2(), &path).expect("writes");
    path
}

#[test]
fn kentos_mimari_takes_drawings_dropped_on_it_and_names_the_programs_for_the_rest() {
    let dir = scratch("yuzey-birak");
    let mut app = drawn(MIMARI);
    let before = app.document.as_ref().map(|d| d.name().to_owned());
    // A building drawing opens in a tab of its own; the one on screen waits behind it.
    let plan = saved_as(
        &dir,
        "plan.kcad",
        Workspace::Cad,
        Some(CadWork::Architecture),
    );
    let task = app.sources_event(crate::sources::Event::Dropped(plan.clone()));
    drive(&mut app, task);
    let path = app.document.as_ref().and_then(|d| d.path.clone());
    assert_eq!(path.as_ref(), Some(&plan), "{}", last_said(&app));
    assert_eq!(app.doc_tabs.parked.len(), 1, "{before:?} kept in its tab");
    // A GIS file: refused, the programs that take it named.
    let shp = dir.join("parsel.shp");
    let _ = app.sources_event(crate::sources::Event::Dropped(shp));
    assert_eq!(
        last_said(&app),
        "“parsel.shp” KentOS Mimari'de açılmaz: KentOS Mimari çizimleri (KCAD, DWG, DXF, NCZ) açar, PDF'i içe aktarır. \
         Shapefile KentOS Harita'da ve KentOS GIS'te açılır."
    );
    let _ = app.sources_event(crate::sources::Event::Dropped(dir.join("not.docx")));
    assert_eq!(
        last_said(&app),
        "“not.docx” KentOS Mimari'de açılmaz: KentOS Mimari çizimleri (KCAD, DWG, DXF, NCZ) açar, PDF'i içe aktarır."
    );
    // A PDF: PDF içe aktar with it.
    let pdf = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/pdf/v1/ascii.pdf");
    let task = app.sources_event(crate::sources::Event::Dropped(pdf));
    drive(&mut app, task);
    assert!(
        matches!(app.exchange, Some(crate::exchange::Window::PdfImport(_))),
        "PDF içe aktar opens"
    );
    // KentOS Harita's drop goes to Kaynaklar, as kentos-cad's.
    let mut harita = started(HARITA);
    let _ = harita.sources_event(crate::sources::Event::Dropped(dir.join("parsel.shp")));
    assert_eq!(
        last_said(&harita),
        "Açık çizim yok. Önce bir çizim açın (Ctrl+O)."
    );
}

// ── Pictures for the owner ───────────────────────────────────────────────

/// Whether a picture is wanted: `KENTOS_SHOTS_ONLY` (comma separated) keeps the names with one of its words.
fn wanted(name: &str) -> bool {
    match std::env::var("KENTOS_SHOTS_ONLY") {
        Ok(only) if !only.trim().is_empty() => only
            .split(',')
            .map(str::trim)
            .any(|word| !word.is_empty() && name.contains(word)),
        _ => true,
    }
}

/// `.run/shots/<name>.png`, the window as `app` shows it at `size`.
fn shoot(app: &mut App, size: iced::Size, name: &str) {
    if !wanted(name) {
        return;
    }
    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    let mut snapshot = kentos_ui::snapshot::Snapshot::new(size).expect("a renderer");
    snapshot.settle(app, App::view, &mut |app: &mut App, message| {
        let _ = app.update(message);
    });
    let file = out.join(format!("{name}.png"));
    snapshot
        .render(app.view(), &app.theme())
        .save(&file)
        .expect("writes the picture");
    println!("{}", file.display());
}

/// A message the scene is set up with, its tasks run to their end.
fn update(app: &mut App, message: Message) {
    let task = app.update(message);
    drive(app, task);
}

/// Each program's window with the bottom panel and the dock open (its tabs, panels and status cells), its
/// application menu's İçe aktar and Dışa aktar, Proje ayarları' sections and Uygulama ayarları' Yeni projeler:
/// `.run/shots/ayrim-<program>-*`, 1440×900, dark and light (`-acik`). Not a correctness test (the tests above
/// are).
///
/// ```text
/// KENTOS_SHOTS_ONLY=mimari cargo test -p kentos-desktop --lib module_surface_tests::pictures -- --ignored --nocapture --test-threads=1
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn pictures() {
    use crate::app_menu::{Event as MenuEvent, Pane};
    let dir = scratch("yuzey-resim");
    let size = iced::Size::new(1440.0, 900.0);
    for (program, word, kind, work) in [
        (None, "cad", Workspace::Cad, Some(CadWork::Map)),
        (
            MIMARI,
            "mimari",
            Workspace::Cad,
            Some(CadWork::Architecture),
        ),
        (HARITA, "harita", Workspace::Cad, Some(CadWork::Map)),
        (GIS, "gis", Workspace::Gis, None),
    ] {
        let path = saved_as(&dir, &format!("{word}.kcad"), kind, work);
        for (theme, suffix) in [("dark", ""), ("light", "-acik")] {
            let mut app = started(program);
            let _ = app.settings.choose(&[
                ("appearance.theme", json!(theme)),
                ("appearance.textSize", json!(13)),
            ]);
            app.apply_settings();
            app.window_size = size;
            let task = app.start_opening(path.clone(), crate::opening::Purpose::File);
            drive(&mut app, task);
            assert!(
                app.document.is_some(),
                "{word}: the drawing opens in its own program"
            );
            app.show_bottom(BottomTab::History);
            shoot(&mut app, size, &format!("ayrim-{word}-pencere{suffix}"));
            update(&mut app, Message::AppMenu(MenuEvent::Toggle));
            update(&mut app, Message::AppMenu(MenuEvent::Show(Pane::Import)));
            shoot(&mut app, size, &format!("ayrim-{word}-ice-aktar{suffix}"));
            update(&mut app, Message::AppMenu(MenuEvent::Show(Pane::Export)));
            shoot(&mut app, size, &format!("ayrim-{word}-disa-aktar{suffix}"));
            update(&mut app, Message::AppMenu(MenuEvent::Dismiss));
            let task = app.project_command("file.settings");
            drive(&mut app, task);
            shoot(
                &mut app,
                size,
                &format!("ayrim-{word}-proje-ayarlari{suffix}"),
            );
            // Its Koordinat sistemi page: the second system is a map's programs'.
            update(
                &mut app,
                Message::Project(Box::new(crate::project::Event::Settings(
                    crate::project::settings::Event::Section(ProjectSection::Crs),
                ))),
            );
            shoot(
                &mut app,
                size,
                &format!("ayrim-{word}-koordinat-sistemi{suffix}"),
            );
            update(&mut app, Message::DialogClosed);
            // Kaynaklar in front where the program has it: Veritabanları and Servisler too.
            if app.shows_panel(Panel::Sources) {
                app.show_sources();
                shoot(&mut app, size, &format!("ayrim-{word}-kaynaklar{suffix}"));
            }
            app.open_settings_at(SettingsSection::NewProjects);
            shoot(
                &mut app,
                size,
                &format!("ayrim-{word}-uygulama-ayarlari{suffix}"),
            );
            update(&mut app, Message::DialogClosed);
        }
    }
}

/// The commands that open a tab or a panel are not offered where the tab or the panel is not shown: a ribbon button
/// would open nothing (catalog.rs's command sets and this file's profile agree).
#[test]
fn a_program_offers_no_opener_of_what_it_does_not_show() {
    let tabs = [
        ("point.editor", BottomTab::Points),
        ("data.featureTable", BottomTab::Table),
        ("data.statistics", BottomTab::Statistics),
        ("data.chart", BottomTab::Chart),
        ("data.search", BottomTab::Search),
        ("address.search", BottomTab::Address),
        ("topology.check", BottomTab::Topology),
        ("arch.audit", BottomTab::Audit),
        ("service.info", BottomTab::ServiceInfo),
    ];
    let panels = [
        ("processing.toolbox", Panel::Processing),
        ("processing.history", Panel::Processing),
        ("block.panel", Panel::Blocks),
        ("data.sources", Panel::Sources),
        ("template.panel", Panel::Templates),
    ];
    let mut wrong = Vec::new();
    for program in [MIMARI, HARITA, GIS] {
        let app = started(program);
        for (id, tab) in tabs {
            if !app.shows_bottom_tab(tab) && app.offers(id) {
                wrong.push(format!("{program:?} {id}"));
            }
        }
        for (id, panel) in panels {
            if !app.shows_panel(panel) && app.offers(id) {
                wrong.push(format!("{program:?} {id}"));
            }
        }
    }
    assert!(wrong.is_empty(), "offered without its surface: {wrong:?}");
}

/// Nokta hesapla in KentOS Mimari has no survey constructions (a route's km, a named point): not in its menus,
/// and its alias or a menu row elsewhere says where they are; a map's program and kentos-cad have every one.
#[test]
fn kentos_mimari_s_point_calculator_has_no_survey_constructions() {
    use kentos_interaction::point_calc::CalcKind;
    let mut app = drawn(MIMARI);
    let _ = app.update(Message::Run("tool.line"));
    assert!(app.session.can_calc_point());
    let menu = format!(
        "{:?}",
        crate::point_calc::calc_menu_without(app.hidden_point_calcs())
    );
    assert!(
        !has(&menu, "Km ve sapma") && !has(&menu, "Nokta adından"),
        "{menu}"
    );
    assert!(
        has(&menu, "İki nokta ortası") && has(&menu, "Açıortay"),
        "{menu}"
    );
    let _ = app.update(Message::PointCalc(CalcKind::Km));
    assert!(!app.session.nested(), "no km is asked for");
    assert_eq!(
        last_said(&app),
        "“Km ve sapma” KentOS Mimari'de yok; KentOS Harita'da ve KentOS GIS'te var."
    );
    let _ = app.update(Message::PointCalc(CalcKind::Mid));
    assert!(app.session.nested(), "a construction it has runs");
    for module in [None, HARITA, GIS] {
        let mut other = drawn(module);
        let _ = other.update(Message::Run("tool.line"));
        let _ = other.update(Message::PointCalc(CalcKind::Km));
        assert!(other.session.nested(), "{module:?}");
    }
}
