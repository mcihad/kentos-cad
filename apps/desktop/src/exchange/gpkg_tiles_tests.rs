//! GeoPackage karolarını oku and Rasteri GeoPackage karosu olarak yaz on the desktop (docs/adr/0556 §5): the read
//! window lists the tables and zooms and shows `cad.rasters.gpkgread`'s plan, Raster olarak ekle… writes the GeoTIFF
//! and adds the raster as one undo step; the write window shows `cad.rasters.gpkgwrite`'s plan and Yaz… writes the
//! tile table off the window's thread. Pictures: `gpkg-karo-*` (`screens`, ignored).

use std::path::PathBuf;

use kentos_commands::{Entity, EntityBase, RasterEntity, RasterFields, RasterSample};

use super::gpkg_tiles::Event as T;
use super::tests::{count, run, said, send};
use super::{Event, Window};
use crate::app::{App, Picker};
use crate::files_testing::{app_with_drawing, scratch};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/gpkg-tiles/v1")
        .join(name)
}

/// A drawing in the files' system (TUREF / TM36).
fn app(srid: u32) -> App {
    let mut app = app_with_drawing();
    let doc = app.document.as_mut().expect("a drawing");
    let mut settings = doc.settings().clone();
    settings.srid = srid;
    doc.model.set_settings(settings);
    app
}

fn rasters(app: &App) -> usize {
    app.document
        .as_ref()
        .expect("a drawing")
        .model
        .entities()
        .filter(|e| matches!(e, Entity::Raster(_)))
        .count()
}

/// The drawing with a raster linked to a GeoTIFF of the fixtures.
fn with_raster(app: &mut App, file: &str, width: u32, height: u32, bands: u32, pixel: f64) {
    with_sample(app, file, width, height, bands, pixel, RasterSample::U8);
}

fn with_sample(
    app: &mut App,
    file: &str,
    width: u32,
    height: u32,
    bands: u32,
    pixel: f64,
    sample: RasterSample,
) {
    let layer = app
        .document
        .as_ref()
        .expect("a drawing")
        .model
        .layers()
        .active()
        .to_owned();
    let entity = Entity::Raster(RasterEntity {
        base: EntityBase {
            look: Default::default(),
            id: 0,
            layer_id: layer,
            color: None,
            attrs: Default::default(),
            label: None,
            symbol: None,
            line_weight: None,
            label_pins: Vec::new(),
            attachments: Vec::new(),
        },
        raster: RasterFields {
            affine: [487200.0, pixel, 0.0, 4420600.0, 0.0, -pixel],
            width,
            height,
            bands,
            sample,
            asset: None,
            file: Some(fixture(&format!("kaynak/{file}")).display().to_string()),
            url: None,
            srid: 5256,
            style: kentos_formats::raster::style::default_style(bands, sample, false),
            opacity: None,
            dataset: None,
            georef: None,
            tiles: None,
        },
    });
    app.document
        .as_mut()
        .expect("a drawing")
        .model
        .add(entity)
        .expect("added");
}

fn reading(app: &App) -> &super::gpkg_tiles::ReadState {
    match &app.exchange {
        Some(Window::GpkgTilesRead(s)) => s,
        _ => panic!("the read window"),
    }
}

fn writing(app: &App) -> &super::gpkg_tiles::WriteState {
    match &app.exchange {
        Some(Window::GpkgTilesWrite(s)) => s,
        _ => panic!("the write window"),
    }
}

#[test]
fn the_window_lists_the_tables_plans_the_zoom_and_adds_the_raster_as_one_step() {
    let mut app = app(5256);
    app.picker = Picker::File(fixture("iki-tablo.gpkg"));
    run(&mut app, "raster.gpkgRead");
    let s = reading(&app);
    assert_eq!(
        s.tables
            .iter()
            .map(|t| t.table.as_str())
            .collect::<Vec<_>>(),
        ["a", "b"]
    );
    let (out, _) = s.plan.as_ref().expect("a plan");
    assert_eq!(
        (out.table.as_str(), out.zoom, out.width, out.height),
        ("a", 1, 300, 200)
    );
    // Another table and its coarser zoom.
    send(&mut app, Event::GpkgTiles(T::Table(1)));
    send(&mut app, Event::GpkgTiles(T::Zoom(0)));
    let (out, _) = reading(&app).plan.as_ref().expect("a plan");
    assert_eq!(
        (out.table.as_str(), out.zoom, out.width, out.height),
        ("b", 0, 130, 130)
    );
    assert_eq!(out.affine[1], 2.0, "a pixel of two metres");
    // Raster olarak ekle…: the GeoTIFF is written and the raster goes onto a new layer.
    let folder = scratch("gpkg-karo");
    let tif = folder.join("b-z0.tif");
    app.picker = Picker::File(tif.clone());
    let before = count(&app);
    send(&mut app, Event::GpkgTiles(T::Save));
    assert!(app.exchange.is_none(), "the window closes");
    assert!(tif.is_file());
    assert_eq!(rasters(&app), 1);
    assert_eq!(count(&app), before + 1);
    assert!(
        said(&app, "130 × 130")
            || said(&app, "4 karo")
            || said(&app, "1 karo")
            || said(&app, "karo okundu")
    );
    // One step takes back the layer and the raster; the file stays.
    let _ = app.update(crate::app::Message::Run("edit.undo"));
    assert_eq!(rasters(&app), 0);
    assert!(tif.is_file());
}

#[test]
fn a_table_of_another_system_cannot_be_added_as_a_raster() {
    let mut app = app(5255);
    app.picker = Picker::File(fixture("orto.gpkg"));
    run(&mut app, "raster.gpkgRead");
    let folder = scratch("gpkg-karo-sistem");
    app.picker = Picker::File(folder.join("orto.tif"));
    send(&mut app, Event::GpkgTiles(T::Save));
    let s = reading(&app);
    assert!(
        s.status.as_deref().is_some_and(|m| m.contains("EPSG:5256")),
        "{:?}",
        s.status
    );
    assert_eq!(rasters(&app), 0, "nothing was added");
    assert!(!folder.join("orto.tif").exists(), "and nothing was written");
}

#[test]
fn a_geopackage_without_tiles_is_said_not_opened() {
    let mut app = app(5256);
    app.picker = Picker::File(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/geopackage/v1/qgis.gpkg"),
    );
    run(&mut app, "raster.gpkgRead");
    assert!(app.exchange.is_none());
    assert!(
        app.log
            .lines()
            .any(|l| l.text.contains("karo (raster) tablosu yok"))
    );
}

#[test]
fn a_raster_goes_out_as_a_tile_table_off_the_windows_thread() {
    let mut app = app(5256);
    with_raster(&mut app, "rgb.tif", 700, 300, 3, 1.5);
    run(&mut app, "raster.gpkgWrite");
    let (plan, _) = writing(&app).plan.as_ref().expect("a plan");
    assert_eq!((plan.zooms.len(), plan.tiles, plan.bands), (3, 9, 4));
    assert_eq!(writing(&app).table, "rgb");
    send(&mut app, Event::GpkgTiles(T::TableName("ortofoto".into())));
    let path = scratch("gpkg-karo-yaz").join("ortofoto.gpkg");
    app.picker = Picker::File(path.clone());
    send(&mut app, Event::GpkgTiles(T::Pick));
    assert!(
        app.exchange.is_none(),
        "the window closes when the file is written"
    );
    let gp = kentos_geopackage::GeoPackage::open(&path, false).expect("a GeoPackage");
    let sets = gp.tile_sets().expect("tile sets");
    assert_eq!(
        sets.iter().map(|s| s.name.as_str()).collect::<Vec<_>>(),
        ["ortofoto"]
    );
    assert_eq!(sets[0].matrices.len(), 3);
    assert!(said(&app, &format!("{} yazıldı", path.display())));
    // A second table goes into the same file; the same name is refused.
    run(&mut app, "raster.gpkgWrite");
    send(&mut app, Event::GpkgTiles(T::Add));
    send(&mut app, Event::GpkgTiles(T::TableName("ikinci".into())));
    app.picker = Picker::File(path.clone());
    send(&mut app, Event::GpkgTiles(T::Pick));
    let gp = kentos_geopackage::GeoPackage::open(&path, false).expect("a GeoPackage");
    assert_eq!(gp.tile_sets().expect("tile sets").len(), 2);
    run(&mut app, "raster.gpkgWrite");
    send(&mut app, Event::GpkgTiles(T::Add));
    send(&mut app, Event::GpkgTiles(T::TableName("ortofoto".into())));
    send(&mut app, Event::GpkgTiles(T::Pick));
    let (error, text) = writing(&app).status.clone().expect("said");
    assert!(error && text.contains("ortofoto"), "{text}");
}

/// Tabloya bağlı (docs/adr/0627 §3): the raster is added on a new layer reading the table's tiles, no GeoTIFF asked
/// or written, one undo step.
#[test]
fn a_table_is_linked_without_a_geotiff() {
    let mut a = app(5256);
    a.picker = Picker::File(fixture("orto.gpkg"));
    run(&mut a, "raster.gpkgRead");
    send(&mut a, Event::GpkgTiles(T::Link(true)));
    assert!(reading(&a).link);
    assert!(reading(&a).plan.is_ok(), "{:?}", reading(&a).plan);
    send(&mut a, Event::GpkgTiles(T::Save));
    assert!(a.exchange.is_none(), "the window closed");
    let model = &a.document.as_ref().expect("a drawing").model;
    let r = model
        .entities()
        .find_map(|e| match e {
            Entity::Raster(r) => Some(r.clone()),
            _ => None,
        })
        .expect("a raster");
    let tiles = r.raster.tiles.as_ref().expect("linked to its table");
    assert_eq!(tiles.table, "orto");
    assert_eq!(
        r.raster.file.as_deref(),
        Some(fixture("orto.gpkg").display().to_string().as_str())
    );
    assert!(model.can_undo());
}

/// Karo biçimi (docs/adr/0627 §5): a picture's tiles as lossless WebP; a height's are a coverage, no format to choose.
#[test]
fn a_picture_goes_out_as_webp_and_a_height_as_a_coverage() {
    let mut a = app(5256);
    with_raster(&mut a, "rgba.tif", 300, 200, 4, 0.5);
    run(&mut a, "raster.gpkgWrite");
    let formats = |a: &App| {
        writing(a)
            .plan
            .as_ref()
            .expect("a plan")
            .0
            .zooms
            .last()
            .map(|z| z.formats.clone())
            .unwrap_or_default()
    };
    assert_eq!(formats(&a), vec!["png".to_owned()]);
    send(&mut a, Event::GpkgTiles(T::Webp(true)));
    assert_eq!(formats(&a), vec!["webp".to_owned()]);
    let mut a = app(5256);
    with_sample(&mut a, "dem-f32.tif", 520, 300, 1, 1.0, RasterSample::F32);
    run(&mut a, "raster.gpkgWrite");
    let plan = &writing(&a).plan.as_ref().expect("a plan").0;
    let coverage = plan.coverage.as_ref().expect("a coverage");
    assert_eq!(coverage.datatype, "float");
    assert_eq!(coverage.data_null, Some(-9999.0));
}

#[test]
fn a_drawing_without_a_raster_says_so() {
    let mut app = app(5256);
    run(&mut app, "raster.gpkgWrite");
    assert!(app.exchange.is_none());
    assert!(said(&app, "Çizimde raster yok"));
}

#[test]
#[ignore = "pictures, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (width, height) in [(1440.0, 900.0), (1100.0, 650.0)] {
        let picture = |app: &mut App, name: &str| {
            let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
            let mut update = |app: &mut App, message| {
                let _ = app.update(message);
            };
            snapshot.settle(app, App::view, &mut update);
            let _ = snapshot.render(app.view(), &app.theme());
            let file = out.join(format!("{name}-{width}x{height}.png"));
            snapshot
                .render(app.view(), &app.theme())
                .save(&file)
                .expect("writes the picture");
            println!("{}", file.display());
        };
        for mode in ["dark", "light"] {
            let fresh = |srid| {
                let mut app = app(srid);
                let _ = app
                    .settings
                    .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
                app.apply_settings();
                app
            };
            let mut a = fresh(5256);
            a.picker = Picker::File(fixture("orto.gpkg"));
            run(&mut a, "raster.gpkgRead");
            picture(&mut a, &format!("gpkg-karo-{mode}-1-oku"));
            send(&mut a, Event::GpkgTiles(T::Zoom(1)));
            picture(&mut a, &format!("gpkg-karo-{mode}-2-duzey"));
            let mut a = fresh(5256);
            a.picker = Picker::File(fixture("iki-tablo.gpkg"));
            run(&mut a, "raster.gpkgRead");
            send(&mut a, Event::GpkgTiles(T::Table(1)));
            picture(&mut a, &format!("gpkg-karo-{mode}-3-iki-tablo"));
            let folder = scratch("gpkg-karo-resim");
            a.picker = Picker::File(folder.join("b.tif"));
            send(&mut a, Event::GpkgTiles(T::Save));
            picture(&mut a, &format!("gpkg-karo-{mode}-4-eklendi"));
            let mut a = fresh(5255);
            a.picker = Picker::File(fixture("orto.gpkg"));
            run(&mut a, "raster.gpkgRead");
            picture(&mut a, &format!("gpkg-karo-{mode}-5-sistem"));
            let mut a = fresh(5256);
            with_raster(&mut a, "rgb.tif", 700, 300, 3, 1.5);
            run(&mut a, "raster.gpkgWrite");
            picture(&mut a, &format!("gpkg-karo-{mode}-6-yaz"));
            // ADR 0627: the linked read, a height table, the WebP and the coverage writes.
            let mut a = fresh(5256);
            a.picker = Picker::File(fixture("orto.gpkg"));
            run(&mut a, "raster.gpkgRead");
            send(&mut a, Event::GpkgTiles(T::Link(true)));
            picture(&mut a, &format!("gpkg-karo-{mode}-7-bagli"));
            send(&mut a, Event::GpkgTiles(T::Save));
            for _ in 0..200 {
                crate::rasters::tiles::service().frame();
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            picture(&mut a, &format!("gpkg-karo-{mode}-8-bagli-eklendi"));
            let mut a = fresh(5256);
            a.picker = Picker::File(fixture("kapsam-float.gpkg"));
            run(&mut a, "raster.gpkgRead");
            picture(&mut a, &format!("gpkg-karo-{mode}-9-kapsam"));
            let mut a = fresh(5256);
            with_raster(&mut a, "rgba.tif", 300, 200, 4, 0.5);
            run(&mut a, "raster.gpkgWrite");
            send(&mut a, Event::GpkgTiles(T::Webp(true)));
            picture(&mut a, &format!("gpkg-karo-{mode}-10-webp"));
            let mut a = fresh(5256);
            with_sample(&mut a, "dem-f32.tif", 520, 300, 1, 1.0, RasterSample::F32);
            run(&mut a, "raster.gpkgWrite");
            picture(&mut a, &format!("gpkg-karo-{mode}-11-kapsam-yaz"));
        }
    }
}
