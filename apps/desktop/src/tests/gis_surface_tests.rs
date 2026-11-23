//! A CBS project in kentos-cad offers the CBS module's commands alone (docs/adr/9127, 9195 §2): Menülerde ara finds no
//! building command, and one run any way (search, command line, shortcut) is refused, said where it is; the CBS ribbon
//! carries no building model export, no user coordinate system panel and no point cloud tools in Analiz (they come with
//! a selected point cloud). A building's project finds and runs Duvar.

use kentos_launch::Module;

use crate::app::{App, Message};
use crate::files_testing::{app_with_drawing, last_said};
use crate::ribbon_search::Hit;

fn gis_app() -> App {
    let mut app = app_with_drawing();
    let _ = app.update(Message::Run("workspace.gis"));
    app
}

/// The commands Menülerde ara finds for `text`.
fn found(app: &mut App, text: &str) -> Vec<&'static str> {
    app.search_typed(text.into());
    app.hits()
        .into_iter()
        .filter_map(|h| match h {
            Hit::Command(c) => Some(c.id),
            _ => None,
        })
        .collect()
}

/// Every command a tab of the drawing's ribbon offers.
fn tab_commands(app: &App, tab: &str) -> Vec<&'static str> {
    app.ribbon_tabs()
        .find(|t| t.id == tab)
        .map(|t| t.panels.iter().flat_map(|p| p.commands()).collect())
        .unwrap_or_default()
}

#[test]
fn duvar_finds_no_building_command_in_a_cbs_project_and_does_not_run() {
    let mut app = gis_app();
    assert!(app.program.is_none(), "kentos-cad, every module");
    assert_eq!(app.active_module(), Some(Module::Gis), "the screen is the project's module's");
    assert!(!app.offers("tool.wall") && app.offers("file.export.gpkg"), "the CBS module's commands alone");
    let ids = found(&mut app, "duvar");
    assert!(!ids.contains(&"tool.wall"), "{ids:?}");
    assert!(!ids.iter().any(|id| id.starts_with("arch.")), "{ids:?}");
    // Run all the same (an alias typed, a shortcut): refused, and said where it is.
    let _ = app.update(Message::Run("tool.wall"));
    let said = last_said(&app);
    assert!(said.contains("“Duvar” bu CBS projesinde yok") && said.contains("KentOS Mimari"), "{said}");
}

#[test]
fn the_cbs_ribbon_carries_no_building_export_ucs_or_point_cloud_analysis() {
    let app = gis_app();
    let file = tab_commands(&app, "file");
    assert!(!file.contains(&"arch.ifcExport") && !file.contains(&"arch.ifcExportCut"), "{file:?}");
    assert!(file.contains(&"file.export.gpkg"), "the map's exports stay");
    let view = tab_commands(&app, "view");
    assert!(!view.iter().any(|id| id.starts_with("ucs.") || id.starts_with("tool.ucs")), "{view:?}");
    let analysis = tab_commands(&app, "analysis");
    assert!(!analysis.iter().any(|id| id.starts_with("pointcloud.")), "{analysis:?}");
    // The point cloud tools are the selected point cloud's, and Menülerde ara still finds them.
    let mut app = app;
    assert!(app.offers("pointcloud.profile"));
    assert!(found(&mut app, "nokta bulutu profil").contains(&"pointcloud.profile"));
    assert!(!app.offers("arch.ifcExport"));
}

#[test]
fn a_buildings_project_finds_and_runs_duvar() {
    let mut app = app_with_drawing();
    crate::files_testing::make_cad(&mut app);
    let doc = app.document.as_mut().expect("a drawing");
    let mut settings = doc.settings().clone();
    settings.cad_work = Some(kentos_commands::CadWork::Architecture);
    doc.model.set_settings(settings);
    assert_eq!(app.active_module(), Some(Module::Architecture));
    assert!(app.offers("tool.wall"));
    let ids = found(&mut app, "duvar");
    assert!(ids.contains(&"tool.wall"), "{ids:?}");
}
