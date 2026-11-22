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
        // Every part said. The filter is sent to the source (docs/adr/9109): the 20 rows it keeps are counted, read
        // and written, not the table's 40.
        assert_eq!(p.total, Some(20));
        seen.set(p.read);
    });
    assert_eq!(seen.get(), 20);
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

/// Kapsam (docs/adr/9112 §2): the box typed (a Turkish decimal comma read), taken from the view or from the layer's
/// extent, or drawn on the map with the window aside; a box that is no place is said and nothing runs. The system
/// chosen from the list goes to the command; KML writes WGS 84 whatever is chosen (§3).
#[test]
fn the_box_is_typed_taken_or_drawn_and_the_system_chosen_from_the_list() {
    use crate::layer_export_extent::{BAD_BOX, Extent};
    use kentos_interaction::capture::Shape;
    use kentos_interaction::Vec2;
    crate::data_layers::register();
    let path = crate::data_layers_tests::parcels("disa-kutu", 12);
    let mut app = crate::files_testing::app_with_drawing();
    app.add_data_layers(vec![crate::data_layers::describe(&path.display().to_string(), "parseller").expect("described")])
        .expect("added");
    let id = crate::data_layers_tests::node(&app, "parseller").expect("the layer").id.clone();
    let _ = app.layer_export_event(Event::Open(id, false));
    let input = |app: &crate::app::App| app.export_input(&app.gis.layer_export.clone().expect("the window"));
    assert_eq!(input(&app).bbox, None, "the whole layer");

    // Typed: west, south, east, north; a comma reads as a decimal point.
    let _ = app.layer_export_event(Event::Extent(2));
    for (i, v) in ["486400", "4420100,5", "486500", "4420200"].into_iter().enumerate() {
        let _ = app.layer_export_event(Event::Side(i, v.into()));
    }
    assert_eq!(input(&app).bbox, Some([486_400.0, 4_420_100.5, 486_500.0, 4_420_200.0]));
    // East west of west: said, nothing runs.
    let _ = app.layer_export_event(Event::Side(2, "486300".into()));
    let _ = app.layer_export_event(Event::Run);
    let w = app.gis.layer_export.as_ref().expect("the window");
    assert_eq!(w.said(), Some(BAD_BOX));
    assert!(!w.is_running(), "nothing runs");

    // From the layer: the table's box, in the project's system.
    let _ = app.layer_export_event(Event::BoxFromLayer);
    let b = input(&app).bbox.expect("a box");
    let table = crate::data_layers::extent_in_project(
        app.document.as_ref().expect("a drawing").model.layers().get(app.gis.layer_export.as_ref().expect("w").layer_id()).expect("the layer"),
        app.document.as_ref().expect("a drawing").settings(),
    )
    .expect("the table's box");
    for (got, want) in b.iter().zip(table) {
        assert!((got - want).abs() < 1e-3, "{b:?} {table:?}");
    }
    // From the view.
    let _ = app.layer_export_event(Event::BoxFromView);
    let v = app.viewport.camera.visible_bounds();
    let b = input(&app).bbox.expect("a box");
    assert!((b[0] - v.min_x).abs() < 1e-3 && (b[3] - v.max_y).abs() < 1e-3, "{b:?} {v:?}");

    // Drawn: the window steps aside and comes back with the box; Esc brings it back as it was.
    let _ = app.layer_export_event(Event::DrawBox);
    assert!(app.dialog.is_none() && app.gis.layer_export.as_ref().is_some_and(|w| w.is_drawing()));
    let corners = vec![Vec2::new(10.0, 20.0), Vec2::new(30.0, 20.0), Vec2::new(30.0, 50.0), Vec2::new(10.0, 50.0)];
    assert!(app.layer_export_box_drawn(&Some(Shape::Polygon(corners))));
    assert_eq!(app.dialog, Some(crate::app::Dialog::Gis(crate::gis::GisDialogKind::LayerExport)));
    assert_eq!(input(&app).bbox, Some([10.0, 20.0, 30.0, 50.0]));
    let _ = app.layer_export_event(Event::DrawBox);
    assert!(app.layer_export_box_drawn(&None));
    assert_eq!(input(&app).bbox, Some([10.0, 20.0, 30.0, 50.0]), "Esc keeps the box");
    assert!(!app.layer_export_box_drawn(&None), "not waiting any more: an extension's shape");

    // Görünümdekiler: the view's box, whatever the sides say.
    let _ = app.layer_export_event(Event::Extent(1));
    assert_eq!(app.gis.layer_export.as_ref().map(|w| w.extent()), Some(Extent::View));
    assert!(input(&app).bbox.is_some());

    // The system: chosen from the list; KML writes WGS 84.
    let _ = app.layer_export_event(Event::Srid(Some(4326)));
    assert_eq!(input(&app).srid, Some(4326));
    let kml = crate::layer_export::FORMATS.iter().position(|f| *f == kentos_commands::ExportFormat::Kml).expect("KML");
    let _ = app.layer_export_event(Event::Format(kml));
    assert_eq!(input(&app).srid, None, "KML: WGS 84 by its writer");
    let rows = crate::system_select::rows("Katmanın sistemi".into(), Some(5256), 5256);
    assert_eq!(rows[0].0, None, "the default first");
    assert!(rows.iter().any(|(s, n)| *s == Some(4326) && n.contains("EPSG:4326")), "the registry's systems");
    assert!(!rows.iter().skip(1).any(|(s, _)| *s == Some(5256)), "the default not twice");
    let _ = std::fs::remove_file(&path);
}

/// KML's symbols (docs/adr/9112 §4): the window asks for the layer's style unless another choice is made; the layer's
/// colour is the command's own default and is not sent; the choice goes only with KML.
#[test]
fn kml_takes_the_layers_style_unless_another_choice_is_made() {
    use kentos_commands::{ExportFormat, ExportSymbols};
    crate::data_layers::register();
    let path = crate::data_layers_tests::parcels("disa-kml", 6);
    let mut app = crate::files_testing::app_with_drawing();
    app.add_data_layers(vec![crate::data_layers::describe(&path.display().to_string(), "parseller").expect("described")])
        .expect("added");
    let id = crate::data_layers_tests::node(&app, "parseller").expect("the layer").id.clone();
    let _ = app.layer_export_event(Event::Open(id, false));
    let symbols = |app: &crate::app::App| {
        app.export_input(&app.gis.layer_export.clone().expect("the window")).options.and_then(|o| o.symbols)
    };
    assert_eq!(symbols(&app), None, "not KML: nothing sent");
    let kml = crate::layer_export::FORMATS.iter().position(|f| *f == ExportFormat::Kml).expect("KML");
    let _ = app.layer_export_event(Event::Format(kml));
    assert_eq!(symbols(&app), Some(ExportSymbols::Style), "the layer's style by default");
    let _ = app.layer_export_event(Event::Symbols(1));
    assert_eq!(symbols(&app), None, "the layer's colour: the command's default");
    let _ = app.layer_export_event(Event::Symbols(2));
    assert_eq!(symbols(&app), Some(ExportSymbols::None));
    let _ = std::fs::remove_file(&path);
}
