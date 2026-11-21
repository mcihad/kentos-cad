//! Dosya › Veri kaynağı (docs/adr/9107): a project saved into the GeoPackage its data layer reads, opened from there in
//! a tab of its own after the file moved to another folder, its layer found through its path from the file.

use crate::files_testing::drive;
use crate::project_store::Event;

#[test]
fn a_project_kept_in_its_geopackage_opens_from_there_and_moves_with_the_file() {
    crate::data_layers::register();
    let gpkg = crate::data_layers_tests::parcels("proje-deposu", 30);
    let address = gpkg.display().to_string();
    let mut app = crate::files_testing::app_with_drawing();
    app.add_data_layers(vec![crate::data_layers::describe(&address, "parseller").expect("described")])
        .expect("added");

    // Kaydet: the place is the data layer's file; nothing kept there yet.
    let task = app.open_project_store(true);
    drive(&mut app, task);
    let w = app.gis.project_store.as_ref().expect("the window");
    assert_eq!(w.path_for_test(), address);
    assert_eq!(w.listed_for_test(), Some(Vec::new()));
    let _ = app.project_store_event(Event::Name("Sivas altyapı".into()));
    let task = app.project_store_event(Event::Run);
    drive(&mut app, task);
    assert!(app.gis.project_store.is_none(), "the window closed");
    let doc = app.document.as_ref().expect("a drawing");
    assert_eq!(doc.stored_source().map(|s| s.name.as_str()), Some("Sivas altyapı"));
    assert!(doc.path.is_none() && !doc.dirty(), "saved, with no file of its own");
    let kept = kentos_native_application::project_store::list(&address, None).expect("listed");
    assert_eq!(kept.iter().map(|p| p.name.as_str()).collect::<Vec<_>>(), ["Sivas altyapı"]);
    // The file's tables are as they were: the project is not a layer.
    let tables: Vec<String> = kentos_data::registry()
        .source(&address)
        .expect("source")
        .tables()
        .expect("tables")
        .into_iter()
        .map(|t| t.name)
        .collect();
    assert_eq!(tables, ["parseller"]);

    // The file moved: opened from its new place, the layer reads the moved file.
    let folder = crate::files_testing::scratch("proje-tasindi");
    let moved = folder.join(gpkg.file_name().expect("a name"));
    std::fs::copy(&gpkg, &moved).expect("copied");
    kentos_data::registry().forget(&address);
    std::fs::remove_file(&gpkg).expect("the old place emptied");
    let task = app.open_project_store(false);
    drive(&mut app, task);
    let _ = app.project_store_event(Event::Path(moved.display().to_string()));
    let task = app.project_store_event(Event::List);
    drive(&mut app, task);
    let _ = app.project_store_event(Event::Select(0));
    let task = app.project_store_event(Event::Run);
    drive(&mut app, task);
    let doc = app.document.as_ref().expect("opened");
    assert_eq!(doc.stored_source().and_then(|s| s.file()), Some(moved.clone()));
    let layer = doc
        .model
        .layers()
        .leaves()
        .into_iter()
        .find(|n| n.data.is_some())
        .cloned()
        .expect("its data layer");
    assert_eq!(layer.data.as_ref().map(|d| d.address.clone()), Some(moved.display().to_string()));
    let settings = doc.model.settings().clone();
    let rows = kentos_native_application::layers_rows::read(
        &layer,
        &settings,
        &kentos_commands::LayersRows { layer: layer.id.clone(), count: Some(true), limit: Some(0), ..Default::default() },
        &|| false,
    )
    .expect("read from the moved file");
    assert_eq!(rows.total, Some(30));
    let _ = std::fs::remove_dir_all(&folder);
}

#[test]
fn a_kept_project_is_removed_only_after_sil() {
    crate::data_layers::register();
    let mut app = crate::files_testing::app_with_drawing();
    let folder = crate::files_testing::scratch("proje-sil");
    let gpkg = folder.join("depo.gpkg");
    let address = gpkg.display().to_string();
    let bytes = kentos_kcad::encode_verified(&app.document.as_ref().expect("a drawing").model.to_snapshot_v2()).expect("encoded");
    kentos_native_application::project_store::write(&address, None, "Eski", &bytes).expect("kept");
    kentos_native_application::project_store::write(&address, None, "Yeni", &bytes).expect("kept");
    let task = app.open_project_store(false);
    drive(&mut app, task);
    let _ = app.project_store_event(Event::Path(address.clone()));
    let task = app.project_store_event(Event::List);
    drive(&mut app, task);
    let _ = app.project_store_event(Event::Select(0));
    // Projeyi sil… asks; Vazgeç keeps it.
    let _ = app.project_store_event(Event::Delete);
    let _ = app.project_store_event(Event::DeleteCancelled);
    let kept = |a: &str| kentos_native_application::project_store::list(a, None).expect("listed").into_iter().map(|p| p.name).collect::<Vec<_>>();
    assert_eq!(kept(&address), ["Eski", "Yeni"]);
    // Sil removes it and the list is read again.
    let _ = app.project_store_event(Event::Delete);
    let task = app.project_store_event(Event::DeleteConfirmed);
    drive(&mut app, task);
    assert_eq!(kept(&address), ["Yeni"]);
    assert_eq!(app.gis.project_store.as_ref().and_then(|w| w.listed_for_test()), Some(vec!["Yeni".to_owned()]));
    let _ = std::fs::remove_dir_all(&folder);
}
