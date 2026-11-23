//! What a CBS project shows by what is selected and which layer is active (docs/adr/9185): one decision, the project's
//! type with the active layer's kind and source and the selection, for the ribbon's contextual tab, Menülerde ara, the
//! commands run and the drawing's menus. The matrix: an empty CBS project, a point, a line, an area, a raster, a point
//! cloud, a network's layer, a read-only table, a table in its edit session and a mixed selection. A CAD project left
//! for a CBS project brings none of its own back; the building's IFC is out of reach in CBS and in reach in a
//! building's CAD project; the CBS bottom strip says Harita and has no Kalınlık.

use kentos_commands::{Entity, EntityBase, RasterEntity, RasterFields, RasterSample, Workspace};
use kentos_domain::Slot;
use kentos_ui::widget::Menu;

use crate::app::{App, Message};
use crate::data_layers_tests::parcels;
use crate::files_testing::{app_with_drawing, last_said};
use crate::ribbon_search::Hit;

fn gis_app() -> App {
    let mut app = app_with_drawing();
    let _ = app.update(Message::Run("workspace.gis"));
    app
}

/// The first object of a kind, wherever it is.
fn slot_of(app: &App, kind: &str) -> Slot {
    let doc = &app.document.as_ref().expect("a drawing").model;
    doc.layers()
        .leaves()
        .iter()
        .flat_map(|n| doc.layer_slots(&n.id).map(|(_, s)| s).collect::<Vec<_>>())
        .find(|s| doc.get(*s).is_some_and(|e| e.kind() == kind))
        .unwrap_or_else(|| panic!("no {kind}"))
}

/// The contextual tab beside the others.
fn context(app: &App) -> Option<&'static str> {
    app.context_tab().map(|t| t.id)
}

/// A new layer made the active one (Katman ekle); its id.
fn new_active_layer(app: &mut App) -> String {
    let _ = app.update(Message::Run("layer.new"));
    app.document.as_ref().expect("a drawing").model.layers().active().to_owned()
}

fn add(app: &mut App, json: serde_json::Value) {
    let e: Entity = serde_json::from_value(json).expect("an object");
    app.document.as_mut().expect("a drawing").model.add(e).expect("added");
}

/// A 700 × 300 colour raster at 1.5 m on the layer: the GeoTIFF of the tiles' cases when `file` (its picture drawn),
/// else only its frame.
fn raster_on(app: &mut App, layer: &str) {
    raster_file_on(app, layer, None);
}

fn raster_file_on(app: &mut App, layer: &str, file: Option<String>) {
    let e = Entity::Raster(RasterEntity {
        base: EntityBase {
            look: Default::default(),
            id: 0,
            layer_id: layer.into(),
            color: None,
            attrs: Default::default(),
            label: None,
            symbol: None,
            line_weight: None,
            label_pins: Vec::new(),
            attachments: Vec::new(),
        },
        raster: RasterFields {
            affine: [487200.0, 1.5, 0.0, 4420600.0, 0.0, -1.5],
            width: 700,
            height: 300,
            bands: 3,
            sample: RasterSample::U8,
            asset: None,
            file,
            url: None,
            srid: 5256,
            style: kentos_formats::raster::style::default_style(3, RasterSample::U8, false),
            opacity: None,
            dataset: None,
            georef: None,
            tiles: None,
        },
    });
    app.document.as_mut().expect("a drawing").model.add(e).expect("added");
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
fn the_contextual_tab_follows_the_selection_and_the_active_layer() {
    let mut app = gis_app();
    // An empty CBS project on a vector layer: its own tabs, no contextual tab.
    assert_eq!(context(&app), None);
    let tabs: Vec<&str> = app.ribbon_tabs().map(|t| t.id).collect();
    assert_eq!(tabs, ["file", "map", "edit", "data", "analysis", "survey", "view", "extensions", "output"]);
    // A point, a line, a path, an area: each its own tab; a mixed selection Seçim.
    for (kind, tab) in [("point", "ctx.point"), ("line", "ctx.line"), ("polyline", "ctx.line"), ("polygon", "ctx.area")] {
        app.selection.set(vec![slot_of(&app, kind)]);
        assert_eq!(context(&app), Some(tab), "{kind}");
    }
    app.selection.set(vec![slot_of(&app, "point"), slot_of(&app, "polygon")]);
    assert_eq!(context(&app), Some("selection"));
    app.selection.set(Vec::new());
    app.follow_selection_tab();

    // A layer of rasters made active: the Raster tab with nothing selected, beside the others until chosen.
    let orto = new_active_layer(&mut app);
    assert_eq!(context(&app), None, "an empty layer");
    raster_on(&mut app, &orto);
    assert_eq!(context(&app), Some("ctx.raster"));
    assert_ne!(app.shown_tab(), "ctx.raster", "beside the chosen tab, not over it");
    let _ = app.update(Message::RibbonTab("ctx.raster"));
    assert_eq!(app.shown_tab(), "ctx.raster");
    assert!(app.offers("raster.style"));
    // Another layer made active: the tab goes, the chosen tab shows again.
    app.layer_pressed("parsel".into());
    app.follow_selection_tab();
    assert_eq!(context(&app), None);
    assert!(app.ribbon_tabs().any(|t| t.id == app.shown_tab()), "{}", app.shown_tab());
    // A selection wins over the layer: an area selected on the raster layer's neighbour opens Alan.
    app.layer_pressed(orto.clone());
    assert_eq!(context(&app), Some("ctx.raster"));
    app.selection.set(vec![slot_of(&app, "polygon")]);
    assert_eq!(context(&app), Some("ctx.area"));
    app.selection.set(Vec::new());

    // A layer of point clouds: Nokta bulutu.
    let bulut = new_active_layer(&mut app);
    add(
        &mut app,
        serde_json::json!({
            "kind": "pointcloud", "id": 0, "layerId": bulut, "attrs": {},
            "sources": [{ "file": "bulut.xyz", "format": "xyz", "count": 50,
                          "bounds": [500000.0, 4400000.0, 100.0, 500100.0, 4400100.0, 150.0] }],
            "bounds": [500000.0, 4400000.0, 100.0, 500100.0, 4400100.0, 150.0], "count": 50, "srid": 5256,
            "style": { "render": "elevation", "min": 100.0, "max": 150.0, "size": 2 }
        }),
    );
    assert_eq!(context(&app), Some("ctx.pointcloud"));

    // A network's line layer: Ağ, with the network's tools; a layer of no network none.
    let doc = app.document.as_mut().expect("a drawing");
    let mut s = doc.settings().clone();
    s.networks = serde_json::from_value(serde_json::json!([{
        "id": "yollar", "name": "Yollar", "kind": "road", "edges": [{ "layer": "cizim" }],
        "connect": "ends", "tolerance": 0.01, "direction": { "kind": "both" }
    }]))
    .expect("a network");
    doc.model.set_settings(s);
    app.layer_pressed("cizim".into());
    assert_eq!(context(&app), Some("ctx.network"));
    let network = app.context_tab().expect("Ağ");
    assert_eq!(network.label, "Ağ");
    let tools: Vec<&str> = network.panels.iter().flat_map(|p| p.commands()).collect();
    for id in ["network.manage", "tool.netRoute", "tool.netServiceArea", "tool.netTrace"] {
        assert!(tools.contains(&id) && app.offers(id), "{id}");
    }
    app.layer_pressed("parsel".into());
    assert_eq!(context(&app), None);
}

#[test]
fn a_read_only_table_offers_no_drawing_and_one_in_its_session_its_own_geometry() {
    crate::data_layers::register();
    let path = parcels("gorunurluk", 3).display().to_string();
    let mut app = gis_app();
    let mut read_only = crate::data_layers::describe(&path, "parseller").expect("described");
    read_only.data.read_only = true;
    let ids = app
        .add_data_layers(vec![crate::data_layers::describe(&path, "parseller").expect("described"), read_only])
        .expect("added");
    // The read-only table: no drawing tool, said why; the table and its style stay.
    app.layer_pressed(ids[1].clone());
    assert!(!app.available("tool.polygon") && !app.available("tool.point"));
    assert!(app.why_disabled("tool.polygon").is_some_and(|w| w.contains("salt okunur")));
    assert!(app.available("data.featureTable") && app.available("style.layerStyle"));
    // The writable one: closed, then in its edit session the area tool and not the point tool (its geometry).
    app.layer_pressed(ids[0].clone());
    assert!(!app.available("tool.polygon"));
    let doc = app.document.as_mut().expect("a drawing");
    doc.model.start_editing(&ids[0]).expect("editing");
    assert!(app.available("tool.polygon"), "in its session");
    assert!(!app.available("tool.point"), "a table of areas takes no point");
}

#[test]
fn a_cad_project_left_for_a_cbs_project_brings_nothing_of_its_own_back() {
    let mut app = app_with_drawing();
    crate::files_testing::make_cad(&mut app);
    // A dimension selected in CAD: its own tab.
    app.selection.set(vec![slot_of(&app, "dimension")]);
    assert_eq!(context(&app), Some("ctx.dimension"));
    let _ = app.update(Message::RibbonTab("ctx.dimension"));
    assert_eq!(app.shown_tab(), "ctx.dimension");
    // A CBS project opened: the CBS tabs, none of the CAD tab's, the building's commands out of reach.
    let gis = crate::project::wizard::new_document(&kentos_project::new_project::NewProject {
        name: "Kent".into(),
        srid: 5256,
        plot_scale: 1000.0,
        workspace: Workspace::Gis,
        drawing_font: kentos_commands::DrawingFont::Barlow,
        province: Some(58),
        drawing_unit: None,
        annotation: None,
        cad_work: None,
        heights: None,
    })
    .expect("a CBS project");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(gis)))));
    assert_eq!(app.work_mode(), Workspace::Gis);
    assert_eq!(context(&app), None);
    assert!(app.ribbon_tabs().any(|t| t.id == app.shown_tab()), "a CBS tab in front: {}", app.shown_tab());
    assert!(!app.ribbon_tabs().any(|t| t.id == "arch"));
    assert!(!app.offers("tool.wall") && !app.offers("arch.ifcExport"));
    // Its bottom strip: no Kalınlık lamp (Çizgi kalınlığı stays in Görünüm), the drawing's tab named Harita.
    assert!(!app.shows_aid("view.lineWeights"));
    assert!(app.offers("view.lineWeights") && app.ribbon_tabs().any(|t| t.id == "view" && t.panels.iter().any(|p| p.commands().contains(&"view.lineWeights"))));
    assert_eq!(kentos_sheet_ui::status::model_name(Some(app.work_mode())), "Harita");
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
    assert_eq!(kentos_sheet_ui::status::model_name(Some(cad.work_mode())), "Model");
    cad.layer_pressed("cizim".into());
    cad.layer_pressed("cizim".into());
    assert_ne!(context(&cad), Some("ctx.network"), "the network's tab is CBS's");
}

/// A road network on Çizim, the sample's lines (docs/adr/0209 §2).
fn network_on_cizim(app: &mut App) {
    let doc = app.document.as_mut().expect("a drawing");
    let mut s = doc.settings().clone();
    s.networks = serde_json::from_value(serde_json::json!([{
        "id": "yollar", "name": "Yollar", "kind": "road", "edges": [{ "layer": "cizim" }],
        "connect": "ends", "tolerance": 0.01, "direction": { "kind": "both" }
    }]))
    .expect("a network");
    doc.model.set_settings(s);
}

/// Pictures for the owner (docs/adr/9185): a raster layer's Raster tab beside the others and chosen, a network's layer
/// with Ağ, and a CAD drawing's bottom strip (Model, Kalınlık) to set the CBS one (Harita, no Kalınlık) against; light
/// 1440 × 900 and dark 1100 × 760 into `.run/gui-evidence/kullanim-9185` (never committed).
///
/// ```text
/// cargo test -p kentos-desktop --lib gis_visibility_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use kentos_commands::{LayerEditOperation, LayersEdit};

    use crate::files_testing::usage_shots;

    let raster_layer = || {
        let mut app = gis_app();
        let mut input = LayersEdit::new(LayerEditOperation::Add);
        input.name = Some("Ortofoto 2024".into());
        input.id = Some("orto".into());
        input.activate = Some(true);
        app.edit_layers(input).expect("added");
        let tif = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/gpkg-tiles/v1/kaynak/rgb.tif");
        raster_file_on(&mut app, "orto", Some(tif.display().to_string()));
        let _ = app.update(Message::Run("view.zoomExtents"));
        app
    };
    usage_shots("9185", "1-raster-katmani", raster_layer);
    usage_shots("9185", "2-raster-sekmesi", || {
        let mut app = raster_layer();
        let _ = app.update(Message::RibbonTab("ctx.raster"));
        app
    });
    usage_shots("9185", "3-ag-katmani", || {
        let mut app = gis_app();
        network_on_cizim(&mut app);
        app.layer_pressed("cizim".into());
        let _ = app.update(Message::RibbonTab("ctx.network"));
        app
    });
    usage_shots("9185", "4-cad-alt-serit", || {
        let mut app = app_with_drawing();
        crate::files_testing::make_cad(&mut app);
        app
    });
}

