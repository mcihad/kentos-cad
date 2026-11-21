//! Farklı kaydet… on the desktop (docs/adr/9106 §4): the window's choices made the command's input, the export written
//! (here on the test's thread) and the written table added to the map.

use crate::layer_export::Event;

#[test]
fn a_data_layers_rows_a_condition_keeps_are_written_to_a_geopackage_and_added_to_the_map() {
    crate::data_layers::register();
    let path = crate::data_layers_tests::parcels("disa-aktar", 40);
    let address = path.display().to_string();
    let mut app = crate::files_testing::app_with_drawing();
    app.add_data_layers(vec![crate::data_layers::describe(&address, "parseller").expect("described")])
        .expect("added");
    let id = crate::data_layers_tests::node(&app, "parseller").expect("the layer").id.clone();
    let out = std::env::temp_dir().join(format!("kentos-disa-aktar-{}.gpkg", std::process::id()));
    let _ = std::fs::remove_file(&out);

    let _ = app.layer_export_event(Event::Open(id.clone(), false));
    assert!(app.gis.layer_export.is_some(), "the window opened");
    let _ = app.layer_export_event(Event::Path(out.display().to_string()));
    let _ = app.layer_export_event(Event::Name("park".into()));
    let _ = app.layer_export_event(Event::Filter("kullanim = 'Park'".into()));
    // Only two fields, one renamed.
    let _ = app.layer_export_event(Event::AllFields(false));
    let _ = app.layer_export_event(Event::FieldOn(0, true));
    let _ = app.layer_export_event(Event::FieldTo(0, "ada_no".into()));
    let _ = app.layer_export_event(Event::FieldOn(1, true));
    let window = app.gis.layer_export.clone().expect("the window");
    let input = app.export_input(&window);
    assert_eq!(input.format, kentos_commands::ExportFormat::Gpkg);
    assert_eq!(input.filter.as_deref(), Some("kullanim = 'Park'"));
    assert_eq!(input.add_to_map, Some(true));
    let fields: Vec<(String, Option<String>)> =
        input.fields.iter().flatten().map(|f| (f.name.clone(), f.to.clone())).collect();
    assert_eq!(fields, [("ada".to_owned(), Some("ada_no".to_owned())), ("kullanim".to_owned(), None)]);

    // Written off the thread in the window; here on the test's, and handed back.
    let doc = app.document.as_ref().expect("a drawing").model.reading_copy();
    let seen = std::cell::Cell::new(0u64);
    let answer = kentos_native_application::layer_export::written(&doc, &input, &|| false, &|p| {
        // Every part said: all 40 rows read of the 40 counted, the 20 kept written.
        assert_eq!(p.total, Some(40));
        seen.set(p.read);
    });
    assert_eq!(seen.get(), 40);
    let (exported, _, add) = answer.clone().expect("written");
    assert_eq!(exported.written, 20);
    assert!(add.is_some(), "the table to add");
    // Its table is listed under its own name and its system named and defined: other programs read them there.
    let gp = kentos_geopackage::GeoPackage::open(&out, false).expect("the written file");
    let (identifier, srs, wkt): (String, String, String) = gp
        .connection()
        .query_row(
            "SELECT c.identifier, s.srs_name, s.definition FROM gpkg_contents c \
             JOIN gpkg_spatial_ref_sys s ON s.srs_id = c.srs_id WHERE c.table_name = 'park'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .expect("its row");
    assert_eq!(identifier, "park");
    assert_eq!(srs, "TUREF / TM36");
    assert!(wkt.starts_with("PROJCS[\"TUREF / TM36\""), "{wkt}");
    drop(gp);
    // A late answer of an export the window no longer waits for changes nothing.
    let _ = app.layer_export_event(Event::Done(7, Box::new(Err("eski".into()))));
    assert!(app.gis.layer_export.as_ref().is_some_and(|w| w.said().is_none()), "a stale answer dropped");
    let _ = app.layer_export_event(Event::Done(0, Box::new(answer)));
    assert!(app.gis.layer_export.is_none(), "the window closed");
    let added = crate::data_layers_tests::node(&app, "park").expect("the written table on the map").clone();
    assert_eq!(added.fields.iter().map(|f| f.name.as_str()).collect::<Vec<_>>(), ["ada_no", "kullanim"]);
    let settings = app.document.as_ref().expect("a drawing").model.settings().clone();
    let rows = kentos_native_application::layers_rows::read(
        &added,
        &settings,
        &kentos_commands::LayersRows { layer: added.id.clone(), count: Some(true), limit: Some(0), ..Default::default() },
        &|| false,
    )
    .expect("read back");
    assert_eq!(rows.total, Some(20));
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(&out);
}

#[test]
fn a_stopped_export_keeps_nothing_and_a_file_it_was_to_overwrite_stays_as_it_was() {
    crate::data_layers::register();
    // More rows than one part (2048), so the export is stopped between two.
    let path = crate::data_layers_tests::parcels("disa-aktar-dur", 5000);
    let address = path.display().to_string();
    let mut app = crate::files_testing::app_with_drawing();
    app.add_data_layers(vec![crate::data_layers::describe(&address, "parseller").expect("described")])
        .expect("added");
    let id = crate::data_layers_tests::node(&app, "parseller").expect("the layer").id.clone();
    let out = std::env::temp_dir().join(format!("kentos-disa-aktar-dur-{}.gpkg", std::process::id()));
    std::fs::write(&out, b"onceki dosya").expect("a file there");

    let _ = app.layer_export_event(Event::Open(id, false));
    let _ = app.layer_export_event(Event::Path(out.display().to_string()));
    // Dosyanın üstüne yaz.
    let _ = app.layer_export_event(Event::Existing(1));
    let window = app.gis.layer_export.clone().expect("the window");
    let input = app.export_input(&window);
    assert_eq!(input.existing, Some(kentos_commands::ExportExisting::OverwriteFile));

    // Durdur after the first part.
    let doc = app.document.as_ref().expect("a drawing").model.reading_copy();
    let parts = std::cell::Cell::new(0u32);
    let answer = kentos_native_application::layer_export::written(&doc, &input, &|| parts.get() >= 1, &|p| {
        assert_eq!(p.total, Some(5000));
        parts.set(parts.get() + 1);
    });
    assert_eq!(parts.get(), 1, "stopped after one part");
    let why = answer.clone().expect_err("stopped");
    assert!(why.contains("durduruldu"), "{why}");
    assert_eq!(std::fs::read(&out).expect("the file"), b"onceki dosya", "the file there kept");
    let mut part = out.clone().into_os_string();
    part.push(".part");
    assert!(!std::path::Path::new(&part).exists(), "nothing half written left");

    // The window stays open and says why; Dışa aktar can be pressed again.
    let _ = app.layer_export_event(Event::Done(0, Box::new(answer)));
    let w = app.gis.layer_export.as_ref().expect("the window stays");
    assert_eq!(w.said(), Some(why.as_str()));
    assert!(crate::data_layers_tests::node(&app, "parseller_2").is_none(), "nothing added to the map");
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(&out);
}
