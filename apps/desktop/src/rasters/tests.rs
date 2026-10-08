//! The raster windows (docs/adr/0204 §6, §8) as the user goes through them:
//! Raster ekle writes the new layer and the raster as one undo step; Raster
//! stili shows its look on the drawing and writes it as one step, Vazgeç
//! leaves nothing; Raster oturt sets an affine as one step; a large raster
//! without overviews gets its pyramid file once, in the device's cache.

use std::path::PathBuf;

use kentos_contracts::{Entity, RasterRender};

use crate::app::{App, Message, Picker};
use crate::calc::raster_fit::{Event as Fit, raster_fit_event};
use crate::files_testing::{app_with_drawing, drive};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/interaction/v1/rasters")
        .join(name)
}

fn send(app: &mut App, message: Message) {
    let task = app.update(message);
    drive(app, task);
}

fn rasters(app: &App) -> Vec<kentos_contracts::RasterEntity> {
    app.document
        .as_ref()
        .expect("a drawing")
        .model
        .entities()
        .filter_map(|e| match e {
            Entity::Raster(r) => Some(r.clone()),
            _ => None,
        })
        .collect()
}

/// A drawing with the elevation model added through Raster ekle, the raster selected.
fn with_raster() -> App {
    let mut app = app_with_drawing();
    app.picker = Picker::File(fixture("dem.tif"));
    let task = app.run("raster.add");
    drive(&mut app, task);
    send(
        &mut app,
        Message::Rasters(super::Event::Add(super::add::Event::Run)),
    );
    let slot = {
        let doc = app.document.as_ref().expect("a drawing");
        doc.model
            .entities()
            .find(|e| matches!(e, Entity::Raster(_)))
            .map(|e| kentos_domain::Slot(e.base().id))
            .expect("the raster")
    };
    app.selection.set(vec![slot]);
    app
}

#[test]
fn raster_ekle_writes_its_layer_and_the_raster_as_one_step() {
    let mut app = app_with_drawing();
    let layers_before = app
        .document
        .as_ref()
        .expect("a drawing")
        .model
        .layers()
        .leaves()
        .len();
    app.picker = Picker::File(fixture("dem.tif"));
    let task = app.run("raster.add");
    drive(&mut app, task);
    let s = app.rasters.add.as_ref().expect("the window");
    let read = s.read.as_ref().expect("read").as_ref().expect("a raster");
    assert_eq!(
        (read.info.width, read.info.height, read.info.bands),
        (480, 324, 1)
    );
    assert_eq!(read.style.render, RasterRender::RampShade);
    send(
        &mut app,
        Message::Rasters(super::Event::Add(super::add::Event::Run)),
    );
    assert!(app.rasters.add.is_none(), "the window closed");
    let all = rasters(&app);
    assert_eq!(all.len(), 1);
    let r = &all[0];
    assert_eq!(r.raster.affine, [487200.0, 1.6, 0.0, 4420600.0, 0.0, -1.6]);
    assert!(
        r.raster
            .file
            .as_deref()
            .is_some_and(|f| f.ends_with("dem.tif"))
    );
    let doc = app.document.as_mut().expect("a drawing");
    let layers = doc.model.layers();
    assert_eq!(layers.leaves().len(), layers_before + 1);
    assert_eq!(
        layers
            .get(&r.base.layer_id)
            .map(|l| l.name.clone())
            .as_deref(),
        Some("dem")
    );
    // One step takes both back.
    assert_eq!(doc.model.undo().as_deref(), Some("Raster ekle"));
    assert!(rasters(&app).is_empty());
    let doc = app.document.as_ref().expect("a drawing");
    assert_eq!(doc.model.layers().leaves().len(), layers_before);
}

#[test]
fn a_file_naming_no_system_waits_for_the_users_yes() {
    let mut app = app_with_drawing();
    app.picker = Picker::File(fixture("tarama.png"));
    let task = app.run("raster.add");
    drive(&mut app, task);
    send(
        &mut app,
        Message::Rasters(super::Event::Add(super::add::Event::Run)),
    );
    assert!(rasters(&app).is_empty(), "not without the yes");
    send(
        &mut app,
        Message::Rasters(super::Event::Add(super::add::Event::Confirm)),
    );
    send(
        &mut app,
        Message::Rasters(super::Event::Add(super::add::Event::Run)),
    );
    let all = rasters(&app);
    assert_eq!(all.len(), 1);
    // The world file placed it; the system is the project's, taken by the user.
    assert_eq!(
        all[0].raster.affine,
        [486870.0, 1.0, 0.0, 4420560.0, 0.0, -1.0]
    );
    assert_eq!(all[0].raster.srid, 0);
}

#[test]
fn raster_stili_shows_its_look_then_writes_it_as_one_step() {
    let mut app = with_raster();
    let look = |e: super::look::Event| Message::Rasters(super::Event::Look(e));
    let task = app.run("raster.style");
    drive(&mut app, task);
    send(
        &mut app,
        look(super::look::Event::Render(RasterRender::Hillshade)),
    );
    // The drawing shows it while the window is open, and nothing is recorded.
    assert_eq!(
        rasters(&app)[0].raster.style.render,
        RasterRender::Hillshade
    );
    send(&mut app, look(super::look::Event::Close));
    assert_eq!(
        rasters(&app)[0].raster.style.render,
        RasterRender::RampShade
    );
    let doc = app.document.as_mut().expect("a drawing");
    assert_eq!(
        doc.model.undo().as_deref(),
        Some("Raster ekle"),
        "Vazgeç left no step"
    );
    assert_eq!(doc.model.redo().as_deref(), Some("Raster ekle"));

    let task = app.run("raster.style");
    drive(&mut app, task);
    send(
        &mut app,
        look(super::look::Event::Render(RasterRender::Gray)),
    );
    send(
        &mut app,
        look(super::look::Event::Type(
            super::look::Typed::Clear,
            "40".into(),
        )),
    );
    send(&mut app, look(super::look::Event::Apply));
    let r = rasters(&app).remove(0);
    assert_eq!(r.raster.style.render, RasterRender::Gray);
    assert_eq!(r.raster.opacity, Some(0.6));
    let doc = app.document.as_mut().expect("a drawing");
    assert_eq!(doc.model.undo().as_deref(), Some("Raster stili"));
    assert_eq!(
        rasters(&app)[0].raster.style.render,
        RasterRender::RampShade
    );
}

#[test]
fn raster_oturt_sets_an_affine_as_one_step() {
    let mut app = with_raster();
    let task = app.run("raster.georef");
    drive(&mut app, task);
    // A pixel a metre and a half, 10 m east and 20 m north of where it was.
    let rows = [
        ["1", "0", "0", "487210", "4420620"],
        ["1", "480", "0", "487930", "4420620"],
        ["1", "0", "324", "487210", "4420134"],
    ];
    app.calc.raster_fit.rows = rows
        .iter()
        .map(|r| {
            let mut row: [String; 8] = Default::default();
            for (k, v) in r.iter().enumerate() {
                row[k] = (*v).to_owned();
            }
            row
        })
        .collect();
    app.calc.raster_fit.solve();
    send(&mut app, raster_fit_event(Fit::Apply));
    let r = rasters(&app).remove(0);
    let [x0, a, b, y0, c, d] = r.raster.affine;
    for (got, want) in [
        (x0, 487210.0),
        (a, 1.5),
        (b, 0.0),
        (y0, 4420620.0),
        (c, 0.0),
        (d, -1.5),
    ] {
        assert!((got - want).abs() < 1e-6, "{:?}", r.raster.affine);
    }
    let doc = app.document.as_mut().expect("a drawing");
    assert_eq!(doc.model.undo().as_deref(), Some("Raster oturt"));
}

/// Raster oturt with a projective transform: the raster resampled beside its
/// file (`<ad>-oturtulmus.tif`), the raster showing it, one step.
#[test]
fn raster_oturt_resamples_beside_a_linked_file() {
    let dir = crate::files_testing::scratch("raster-warp");
    let copy = dir.join("tarama.png");
    std::fs::copy(fixture("tarama.png"), &copy).expect("copied");
    std::fs::copy(fixture("tarama.pgw"), dir.join("tarama.pgw")).expect("copied");
    let mut app = app_with_drawing();
    app.picker = Picker::File(copy);
    let task = app.run("raster.add");
    drive(&mut app, task);
    send(
        &mut app,
        Message::Rasters(super::Event::Add(super::add::Event::Confirm)),
    );
    send(
        &mut app,
        Message::Rasters(super::Event::Add(super::add::Event::Run)),
    );
    let slot = kentos_domain::Slot(rasters(&app)[0].base.id);
    app.selection.set(vec![slot]);
    let task = app.run("raster.georef");
    drive(&mut app, task);
    app.calc.raster_fit.method = kentos_geometry_core::ops::georef::Method::Projective;
    let rows = [
        ["1", "12.5", "14.0", "486880.6", "4420547.1"],
        ["1", "287.0", "11.5", "487159.2", "4420549.4"],
        ["1", "283.5", "248.0", "487158.9", "4420309.0"],
        ["1", "9.0", "251.5", "486876.0", "4420310.6"],
    ];
    app.calc.raster_fit.rows = rows
        .iter()
        .map(|r| {
            let mut row: [String; 8] = Default::default();
            for (k, v) in r.iter().enumerate() {
                row[k] = (*v).to_owned();
            }
            row
        })
        .collect();
    app.calc.raster_fit.solve();
    send(&mut app, raster_fit_event(Fit::Apply));
    assert_eq!(
        app.calc.raster_fit.status, None,
        "the window said nothing against it"
    );
    let said: Vec<String> = app.log.lines().map(|l| l.text.clone()).collect();
    let r = rasters(&app).remove(0);
    let out = dir.join("tarama-oturtulmus.tif");
    assert!(
        said.iter().any(|l| l.contains("yeniden örneklendi")),
        "{said:?}"
    );
    assert!(
        out.is_file(),
        "the resampled file is written beside the source"
    );
    assert_eq!(
        r.raster.file.as_deref(),
        Some(out.to_string_lossy().as_ref())
    );
    assert_eq!(
        (r.raster.bands, r.raster.sample),
        (4, kentos_contracts::RasterSample::U8)
    );
    assert_eq!(r.raster.style.render, RasterRender::Rgb);
    // North up on the output grid: no turn in its affine.
    assert_eq!((r.raster.affine[2], r.raster.affine[4]), (0.0, 0.0));
    let doc = app.document.as_mut().expect("a drawing");
    assert_eq!(doc.model.undo().as_deref(), Some("Raster oturt"));
    assert!(
        rasters(&app)[0]
            .raster
            .file
            .as_deref()
            .is_some_and(|f| f.ends_with("tarama.png"))
    );
}

/// A raster over 4096 pixels on a side without overviews: its coarse levels
/// wait for its pyramid file, made once in the device's cache, then come.
#[test]
fn a_large_raster_gets_its_pyramid_file_once() {
    use kentos_contracts::RasterSample;
    use kentos_formats::raster::write::{Geo, Image, Writer};
    use kentos_formats::raster::{Samples, TILE};

    let dir = crate::files_testing::scratch("raster-pyramid");
    let (w, h) = (4200u32, 64u32);
    let image = Image {
        width: w,
        height: h,
        bands: 1,
        sample: RasterSample::U8,
        alpha: false,
        geo: Some(Geo {
            affine: [0.0, 1.0, 0.0, 64.0, 0.0, -1.0],
            epsg: Some(5256),
            geographic: false,
        }),
        nodata: None,
    };
    let (mut writer, header) = Writer::new(vec![image], 1).expect("a writer");
    let mut out = header;
    for ty in 0..h.div_ceil(TILE) {
        for tx in 0..w.div_ceil(TILE) {
            let mut v = vec![0u8; (TILE * TILE) as usize];
            for (k, s) in v.iter_mut().enumerate() {
                *s = ((tx * 7 + ty * 3 + k as u32 % 251) % 256) as u8;
            }
            out.extend(writer.tile(0, tx, ty, &Samples::U8(v)).expect("a tile"));
        }
    }
    let (dirs, head) = writer.finish().expect("finished");
    out.extend(dirs);
    out[..head.len()].copy_from_slice(&head);
    let file = dir.join("genis.tif");
    std::fs::write(&file, &out).expect("written");

    let cache = dir.join("cache");
    *super::pyramid::TEST_FOLDER.lock().expect("the folder") = Some(cache.clone());
    let s = super::tiles::service();
    let key = format!("file:{}", file.display());
    s.register(&key, || Some(super::tiles::Origin::File(file.clone())));
    let look = r#"{"render":"gray","bands":[1]}"#;
    let affine = [0.0, 1.0, 0.0, 64.0, 0.0, -1.0];
    let mut got = None;
    // A debug build among the other tests: up to two minutes, done as soon as it comes.
    for _ in 0..12_000 {
        // Another test's drawing may let it go (`keep_only`): the scene gives it again, as a redo does.
        s.register(&key, || Some(super::tiles::Origin::File(file.clone())));
        s.frame();
        if let Some(t) = s.tile(&key, look, &affine, 4, 0, 0) {
            got = Some(t);
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    let tile = got.expect("the coarse tile came once the pyramid was made");
    assert_eq!(tile.len(), 258 * 258 * 4);
    let made: Vec<_> = std::fs::read_dir(&cache)
        .expect("the cache folder")
        .filter_map(Result::ok)
        .filter(|e| e.path().extension().is_some_and(|x| x == "tif"))
        .collect();
    assert_eq!(made.len(), 1, "one pyramid file");
}
