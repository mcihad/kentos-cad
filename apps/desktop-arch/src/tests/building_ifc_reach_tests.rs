//! The building's IFC export is out of reach in a CBS project and in reach in a building's CAD project (docs/adr/8047,
//! 9195 §5): not offered, not in the menus, not found, refused when typed in CBS; offered and found with Kalınlık and
//! Model in Mimari. Moved from the core's gis_visibility_tests.rs with the module's package (docs/adr/9199 §4.11);
//! CBS is still in the core, so its side runs here too.

use crate::app::{App, Message};
use crate::files_testing::{app_with_drawing, last_said};
use crate::ribbon_search::Hit;
use kentos_commands::Workspace;
use kentos_ui::widget::Menu;

fn gis_app() -> App {
    let mut app = app_with_drawing();
    let _ = app.update(Message::Run("workspace.gis"));
    app
}

/// The contextual tab beside the others.
fn context(app: &App) -> Option<&'static str> {
    app.context_tab().map(|t| t.id)
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

/// Whether a drawing menu (the right button's) would list the command: every one of them adds it this way.
fn in_menus(app: &App, id: &'static str) -> bool {
    !app.command_item(Menu::new(), id).is_empty()
}

#[test]
fn the_buildings_ifc_is_out_of_reach_in_cbs_and_in_reach_in_a_buildings_cad_project() {
    let mut app = gis_app();
    for id in ["arch.ifcExport", "arch.ifcExportCut"] {
        assert!(!app.offers(id) && !app.available(id), "{id}");
        assert!(!in_menus(&app, id), "the right button's menus: {id}");
    }
    let ids = found(&mut app, "ifc");
    assert!(!ids.iter().any(|id| id.starts_with("arch.ifc")), "{ids:?}");
    // Run any way (a shortcut, the command line): refused, said where it is.
    let _ = app.update(Message::Run("arch.ifcExport"));
    let said = last_said(&app);
    assert!(said.contains("bu CBS projesinde yok"), "{said}");
    // The other way: a building's CAD project finds it, lists it and keeps Kalınlık and Model; no Ağ tab.
    let mut cad = app_with_drawing();
    let doc = cad.document.as_mut().expect("a drawing");
    let mut s = doc.settings().clone();
    s.workspace = Some(Workspace::Cad);
    s.cad_work = Some(kentos_commands::CadWork::Architecture);
    s.networks = serde_json::from_value(serde_json::json!([{
        "id": "yollar", "name": "Yollar", "kind": "road", "edges": [{ "layer": "cizim" }],
        "connect": "ends", "tolerance": 0.01, "direction": { "kind": "both" }
    }]))
    .expect("a network");
    doc.model.set_settings(s);
    assert!(cad.offers("arch.ifcExport") && in_menus(&cad, "arch.ifcExport"));
    assert!(found(&mut cad, "ifc").contains(&"arch.ifcExport"));
    assert!(cad.shows_aid("view.lineWeights"));
    assert_eq!(
        kentos_sheet_ui::status::model_name(Some(cad.work_mode())),
        "Model"
    );
    cad.layer_pressed("cizim".into());
    cad.layer_pressed("cizim".into());
    assert_ne!(
        context(&cad),
        Some("ctx.network"),
        "the network's tab is CBS's"
    );
}
