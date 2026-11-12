//! The raster windows (docs/adr/0204 §6, §8) as the user goes through them:
//! Raster ekle writes the new layer and the raster as one undo step; Raster
//! stili shows its look on the drawing and writes it as one step, Vazgeç
//! leaves nothing; Raster oturt sets an affine as one step; a large raster
//! without overviews gets its pyramid file once, in the device's cache.

use std::path::PathBuf;

use kentos_commands::{Entity, RasterRender};

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

fn rasters(app: &App) -> Vec<kentos_commands::RasterEntity> {
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

/// Raster oturt's rows from Kullan, Denetim, Ad, Sütun, Satır, Hedef Y and Hedef X.
fn fit_rows(rows: &[[&str; 7]]) -> Vec<crate::calc::raster_fit::Row> {
    rows.iter()
        .map(|r| {
            let mut row: crate::calc::raster_fit::Row = Default::default();
            for (k, v) in r.iter().enumerate() {
                row[k] = (*v).to_owned();
            }
            row
        })
        .collect()
}

fn solve(app: &mut App) {
    let doc = app.document.as_ref().expect("a drawing");
    app.calc.raster_fit.solve(&doc.model);
}

#[test]
fn raster_oturt_sets_an_affine_as_one_step() {
    let mut app = with_raster();
    let task = app.run("raster.georef");
    drive(&mut app, task);
    // A pixel a metre and a half, 10 m east and 20 m north of where it was.
    let rows = [
        ["1", "0", "", "0", "0", "487210", "4420620"],
        ["1", "0", "", "480", "0", "487930", "4420620"],
        ["1", "0", "", "0", "324", "487210", "4420134"],
    ];
    app.calc.raster_fit.rows = fit_rows(&rows);
    solve(&mut app);
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
    // The raster keeps how it was placed (docs/adr/0518): the method and the points, its source.
    let record = r.raster.georef.as_ref().expect("the record");
    assert_eq!(record.method, kentos_commands::GeorefMethod::Affine);
    assert_eq!(record.points.len(), 3);
    assert_eq!((record.source.width, record.source.height), (r.raster.width, r.raster.height));
    let doc = app.document.as_mut().expect("a drawing");
    assert_eq!(doc.model.undo().as_deref(), Some("Raster oturt"));
}

/// Noktaları kaydet… writes QGIS's `.points` with the residuals; Noktaları al… reads it back into the rows, the check
/// point still one (docs/adr/0518 §4).
#[test]
fn raster_oturt_points_go_to_a_file_and_back() {
    let dir = crate::files_testing::scratch("raster-points");
    let file = dir.join("noktalar.points");
    let mut app = with_raster();
    let task = app.run("raster.georef");
    drive(&mut app, task);
    let rows = [
        ["1", "0", "K1", "0", "0", "487210", "4420620"],
        ["1", "0", "K2", "480", "0", "487930", "4420620"],
        ["1", "0", "K3", "0", "324", "487210", "4420134"],
        ["1", "1", "D1", "240", "162", "487570.02", "4420377.01"],
    ];
    app.calc.raster_fit.rows = fit_rows(&rows);
    solve(&mut app);
    app.picker = Picker::File(file.clone());
    send(&mut app, raster_fit_event(Fit::Export));
    let written = std::fs::read_to_string(&file).expect("written");
    // The targets' system first, the project's here (docs/adr/0594 §2), then the header.
    let mut lines = written.lines();
    assert!(lines.next().is_some_and(|l| l.starts_with("#CRS: PROJCRS[")), "{written}");
    assert_eq!(lines.next(), Some(kentos_formats::raster::gcp::HEADER), "{written}");
    assert!(written.contains(",check,D1"), "{written}");
    let before = app.calc.raster_fit.points();
    app.calc.raster_fit.rows = vec![Default::default(); 4];
    send(&mut app, raster_fit_event(Fit::Import));
    assert_eq!(app.calc.raster_fit.points(), before);
}

/// A `.points` file's `#CRS:` line is the targets' system (docs/adr/0594 §3): a definition that is no registered system
/// is kept as the window's definition, solved through, written to the record and back to the file; a registered one
/// is its EPSG code; Kaldır drops it.
#[test]
fn raster_oturt_takes_a_files_system_and_writes_it_back() {
    let dir = crate::files_testing::scratch("raster-points-crs");
    let file = dir.join("tanimli.points");
    let defined = "PROJCS[\"Belediye TM 36.5\",GEOGCS[\"TUREF\",DATUM[\"Turkish_National_Reference_Frame\",\
        SPHEROID[\"GRS 1980\",6378137,298.257222101]],PRIMEM[\"Greenwich\",0],UNIT[\"degree\",0.0174532925199433]],\
        PROJECTION[\"Transverse_Mercator\"],PARAMETER[\"latitude_of_origin\",0],PARAMETER[\"central_meridian\",36.5],\
        PARAMETER[\"scale_factor\",1],PARAMETER[\"false_easting\",500000],PARAMETER[\"false_northing\",0],UNIT[\"metre\",1]]";
    std::fs::write(
        &file,
        format!(
            "#CRS: {defined}\nmapX,mapY,sourceX,sourceY,enable\n444210,4420620,0,0,1\n444930,4420620,480,0,1\n\
             444210,4420134,0,-324,1\n"
        ),
    )
    .expect("written");
    let mut app = with_raster();
    let task = app.run("raster.georef");
    drive(&mut app, task);
    app.picker = Picker::File(file.clone());
    send(&mut app, raster_fit_event(Fit::Import));
    assert_eq!(app.calc.raster_fit.crs.as_deref(), Some(defined));
    assert!(app.calc.raster_fit.srid.is_empty());
    solve(&mut app);
    // Written back: the definition on the first line, as QGIS reads it.
    let out = dir.join("geri.points");
    app.picker = Picker::File(out.clone());
    send(&mut app, raster_fit_event(Fit::Export));
    let written = std::fs::read_to_string(&out).expect("written");
    let one_line = defined.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(written.starts_with(&format!("#CRS: {one_line}\n")), "{written}");
    send(&mut app, raster_fit_event(Fit::Apply));
    let r = rasters(&app).remove(0);
    let record = r.raster.georef.as_ref().expect("the record");
    assert_eq!((record.srid, record.crs.as_deref()), (None, Some(defined)));
    // A registered system's WKT is its code; a typed code takes the definition's place, Kaldır drops it.
    let tm30 = "PROJCS[\"TUREF / TM30\",GEOGCS[\"TUREF\",DATUM[\"Turkish_National_Reference_Frame\",\
        SPHEROID[\"GRS 1980\",6378137,298.257222101]],PRIMEM[\"Greenwich\",0],UNIT[\"degree\",0.0174532925199433]],\
        PROJECTION[\"Transverse_Mercator\"],PARAMETER[\"latitude_of_origin\",0],PARAMETER[\"central_meridian\",30],\
        PARAMETER[\"scale_factor\",1],PARAMETER[\"false_easting\",500000],PARAMETER[\"false_northing\",0],UNIT[\"metre\",1]]";
    let registered = dir.join("kayitli.points");
    std::fs::write(&registered, format!("#CRS: {tm30}\nmapX,mapY,sourceX,sourceY\n1,2,0,0\n3,4,10,-10\n5,6,0,-10\n"))
        .expect("written");
    // The window opens again on the raster: its record's definition first.
    let task = app.run("raster.georef");
    drive(&mut app, task);
    assert_eq!(app.calc.raster_fit.crs.as_deref(), Some(defined));
    app.picker = Picker::File(registered);
    send(&mut app, raster_fit_event(Fit::Import));
    assert_eq!((app.calc.raster_fit.srid.as_str(), app.calc.raster_fit.crs.as_deref()), ("5254", None));
    app.calc.raster_fit.crs = Some(defined.to_owned());
    app.calc.raster_fit.srid.clear();
    send(&mut app, raster_fit_event(Fit::ClearCrs));
    assert_eq!(app.calc.raster_fit.crs, None);
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
    app.calc.raster_fit.method = kentos_commands::GeorefMethod::Projective;
    let rows = [
        ["1", "0", "", "12.5", "14.0", "486880.6", "4420547.1"],
        ["1", "0", "", "287.0", "11.5", "487159.2", "4420549.4"],
        ["1", "0", "", "283.5", "248.0", "487158.9", "4420309.0"],
        ["1", "0", "", "9.0", "251.5", "486876.0", "4420310.6"],
    ];
    app.calc.raster_fit.rows = fit_rows(&rows);
    solve(&mut app);
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
        (4, kentos_commands::RasterSample::U8)
    );
    assert_eq!(r.raster.style.render, RasterRender::Rgb);
    // North up on the output grid: no turn in its affine.
    assert_eq!((r.raster.affine[2], r.raster.affine[4]), (0.0, 0.0));
    // The record names the source the points' pixels are of, its digest, and the output grid (docs/adr/0518).
    let record = r.raster.georef.as_ref().expect("the record");
    assert!(record.source.file.as_deref().is_some_and(|f| f.ends_with("tarama.png")));
    assert!(record.source.digest.is_some());
    let output = record.output.as_ref().expect("the output grid");
    assert_eq!((output.affine, output.width, output.height), (r.raster.affine, r.raster.width, r.raster.height));
    assert!(output.digest.is_some());
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
    use kentos_commands::RasterSample;
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

/// A GeoTIFF read from an address (docs/adr/0207 §1): the window reads its
/// header by ranges, Ekle keeps the address, and the scene's tiles are the
/// file's own pixels.
#[test]
fn raster_ekle_reads_a_geotiff_from_an_address() {
    use crate::range_server::{Mode, serve};
    let data = std::fs::read(fixture("dem.tif")).expect("the elevation model");
    let server = serve(vec![("dem.tif", data.clone())], Mode::Ranges);
    let url = server.url("dem.tif");
    let mut app = app_with_drawing();
    app.picker = Picker::File(fixture("dem.tif"));
    let task = app.run("raster.add");
    drive(&mut app, task);
    let add =
        |app: &mut App, e: super::add::Event| send(app, Message::Rasters(super::Event::Add(e)));
    add(&mut app, super::add::Event::From(super::add::From::Address));
    let s = app.rasters.add.as_ref().expect("the window");
    assert!(
        s.read.is_none() && s.name.is_empty(),
        "nothing read for an address yet"
    );
    add(&mut app, super::add::Event::Address(url.clone()));
    add(&mut app, super::add::Event::ReadAddress);
    let s = app.rasters.add.as_ref().expect("the window");
    let read = s
        .read
        .as_ref()
        .expect("read")
        .as_ref()
        .expect("a raster from the address");
    assert_eq!(
        (read.info.width, read.info.height, read.info.bands),
        (480, 324, 1)
    );
    assert_eq!(read.bytes, data.len() as u64);
    assert_eq!(s.layer, "dem");
    // Another address typed: what was read is not its until Oku.
    add(&mut app, super::add::Event::Address(format!("{url}?v=2")));
    assert!(app.rasters.add.as_ref().expect("the window").read.is_none());
    add(&mut app, super::add::Event::Address(url.clone()));
    add(&mut app, super::add::Event::ReadAddress);
    add(&mut app, super::add::Event::Run);
    let r = rasters(&app).remove(0);
    assert_eq!(r.raster.url.as_deref(), Some(url.as_str()));
    assert!(r.raster.file.is_none() && r.raster.asset.is_none());
    assert_eq!(super::key_of(&r.raster), format!("url:{url}"));
    // The same pixels by ranges as from the file.
    let from_url =
        super::tiles::open(&super::tiles::Origin::Url(url.clone()), None).expect("opens");
    let from_file =
        super::tiles::open(&super::tiles::Origin::File(fixture("dem.tif")), None).expect("opens");
    from_url.fill(0, 0, 0, 480, 324).expect("reads");
    from_file.fill(0, 0, 0, 480, 324).expect("reads");
    let region = |o: &super::tiles::Opened| {
        o.reader
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .region(0, 0, 0, 480, 324)
            .expect("the pixels")
    };
    assert!(region(&from_url) == region(&from_file), "the same pixels");
}

#[test]
fn an_address_takes_only_a_geotiff() {
    use crate::range_server::{Mode, serve};
    let png = std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/raster/v1/files/rgb.png"),
    )
    .expect("a PNG");
    let server = serve(vec![("rgb.png", png)], Mode::Ranges);
    let why = super::add::inspect_origin(&super::tiles::Origin::Url(server.url("rgb.png")))
        .expect_err("refused");
    assert!(
        why.contains("Adresten yalnız GeoTIFF (COG) eklenir"),
        "{why}"
    );
    let mut app = app_with_drawing();
    app.picker = Picker::File(fixture("dem.tif"));
    let task = app.run("raster.add");
    drive(&mut app, task);
    let add =
        |app: &mut App, e: super::add::Event| send(app, Message::Rasters(super::Event::Add(e)));
    add(&mut app, super::add::Event::From(super::add::From::Address));
    add(
        &mut app,
        super::add::Event::Address("ftp://ornek.org/dem.tif".into()),
    );
    add(&mut app, super::add::Event::ReadAddress);
    let s = app.rasters.add.as_ref().expect("the window");
    assert!(s.read.is_none());
    assert!(
        s.status_for_tests()
            .is_some_and(|w| w.contains("HTTP ya da HTTPS adresi değil")),
        "{:?}",
        s.status_for_tests()
    );
}

/// A raster linked to a GeoPackage tile table (docs/adr/0627 §3): its scene key opens the table from its file, each
/// level reads its zoom's tiles (a level the table has no tiles for is worked out), the pixels the application reads
/// of the same raster; the drawing's tiles come from the file, a pyramid's name the file's and the table's.
#[test]
fn a_linked_tile_table_draws_from_its_file() {
    use kentos_native_application::ExecutionContext;
    use kentos_native_application::gpkg_tiles::read;
    use kentos_native_application::raster_files;
    let tiles = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/gpkg-tiles/v1");
    for (file, levels) in [("orto.gpkg", 3usize), ("jpeg.gpkg", 2), ("kapsam-float.gpkg", 2)] {
        let setup = serde_json::json!({
            "format": "kentos.document", "version": 1, "name": "Bağlı",
            "settings": { "srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg",
                          "plotScale": 1000, "workspace": "gis" },
            "origin": { "x": 0.0, "y": 0.0 },
            "layers": [{ "id": "altlik", "name": "Altlık", "type": "layer", "visible": true, "locked": false,
                         "expanded": true, "style": { "color": "fg", "lineType": "continuous", "lineWeight": 0.25 },
                         "children": [] }],
            "activeLayer": "altlik", "entities": [], "styles": { "items": [], "categories": [] }
        });
        let snapshot = kentos_commands::DocumentSnapshotV1::from_json(&setup.to_string()).expect("setup");
        let mut model = kentos_domain::Document::from_snapshot(snapshot).expect("setup");
        let path = tiles.join(file);
        let kentos_commands::CommandResult::Completed { output, .. } = read::execute(
            &mut ExecutionContext::new(&mut model),
            kentos_commands::RastersGpkgRead {
                path: path.display().to_string(),
                table: None,
                zoom: None,
                area: None,
                output: None,
                layer: Some("altlik".into()),
                overwrite: None,
                link: Some(true),
                drawing: None,
                expected_revision: None,
            },
        ) else {
            panic!("{file}: not linked")
        };
        let uid = kentos_domain::Uuid::parse_str(output.uid.as_deref().expect("added")).expect("a uid");
        let slot = model.slot_of(uid).expect("the raster");
        let Some(Entity::Raster(r)) = model.get(slot) else {
            panic!("{file}: no raster")
        };
        let fields = r.raster.clone();
        let key = super::key_of(&fields);
        assert!(key.starts_with("gpkg:"), "{key}");
        let origin = super::origin_of(&key, &model, None).expect("the table's origin");
        assert!(matches!(&origin, super::tiles::Origin::Table(p, _) if *p == path), "{origin:?}");
        let opened = super::tiles::open(&origin, None).expect("opens");
        assert!(
            opened.identity.as_deref().is_some_and(|id| id.starts_with("gpkg|")),
            "{:?}",
            opened.identity
        );
        let mut theirs = raster_files::open_tiles(&path, &fields).expect("the application opens it");
        let sizes: Vec<(u32, u32)> = {
            let reader = opened.reader.lock().expect("the reader");
            reader.levels.iter().map(|l| (l.width, l.height)).collect()
        };
        assert_eq!(sizes.len(), levels, "{file}");
        for (k, (w, h)) in sizes.into_iter().enumerate() {
            opened.fill(k, 0, 0, w, h).expect("its tiles");
            let ours = opened
                .reader
                .lock()
                .expect("the reader")
                .region(k, 0, 0, w, h)
                .expect("the level's pixels");
            let want = theirs.region_at(k, 0, 0, w, h).expect("read").expect("pixels");
            assert_eq!(ours.samples, want.samples, "{file} level {k}");
        }
    }
}

/// An address whose server reads no ranges (docs/adr/0642 §3): the window says why and offers İndir ve dosya olarak
/// ekle; the file comes whole into the drawing's `veri` folder and is read as a linked file, the same pixels.
#[test]
fn an_address_without_ranges_is_downloaded_and_added_as_a_file() {
    use crate::range_server::{Mode, serve};
    let data = std::fs::read(fixture("dem.tif")).expect("the elevation model");
    let server = serve(vec![("dem.tif", data.clone())], Mode::Whole);
    let url = server.url("dem.tif");
    let dir = std::env::temp_dir().join(format!("kentos-indir-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a folder");
    let mut app = app_with_drawing();
    app.document.as_mut().expect("a drawing").path = Some(dir.join("cizim.kcad"));
    app.picker = Picker::File(fixture("dem.tif"));
    let task = app.run("raster.add");
    drive(&mut app, task);
    let add =
        |app: &mut App, e: super::add::Event| send(app, Message::Rasters(super::Event::Add(e)));
    add(&mut app, super::add::Event::From(super::add::From::Address));
    add(&mut app, super::add::Event::Address(format!("{url}?imza=1")));
    add(&mut app, super::add::Event::ReadAddress);
    let s = app.rasters.add.as_ref().expect("the window");
    let why = s.read.as_ref().expect("read").as_ref().expect_err("refused");
    assert!(super::add::downloadable(why), "{why}");
    add(&mut app, super::add::Event::Download);
    let s = app.rasters.add.as_ref().expect("the window");
    assert_eq!(s.from, super::add::From::File, "read now as a file");
    let want = dir.join("veri").join("dem.tif");
    assert_eq!(s.path, want);
    assert_eq!(std::fs::read(&want).expect("downloaded"), data);
    let read = s.read.as_ref().expect("read").as_ref().expect("a raster");
    assert_eq!((read.info.width, read.info.height), (480, 324));
    // Taken again: the next name, the first kept.
    assert_eq!(
        super::add::download_place(Some(&dir.join("cizim.kcad")), &url),
        dir.join("veri").join("dem-2.tif")
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// The pyramid files' budget (docs/adr/0642 §4): over it, the files used longest ago go first; the one just made, the
/// ones being written and the ones touched in the last minute stay.
#[test]
fn the_pyramid_files_keep_within_their_budget_the_oldest_going_first() {
    use std::time::{Duration, SystemTime};
    let dir = std::env::temp_dir().join(format!("kentos-piramit-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a folder");
    let now = SystemTime::now();
    let file = |name: &str, len: usize, ago: u64| {
        let p = dir.join(name);
        std::fs::write(&p, vec![0u8; len]).expect("written");
        let f = std::fs::File::options().write(true).open(&p).expect("opened");
        f.set_modified(now - Duration::from_secs(ago)).expect("time");
        p
    };
    let oldest = file("a.tif", 400, 3600 * 24);
    let old = file("b.tif", 300, 3600);
    let kept = file("c.tif", 500, 7200);
    let recent = file("d.tif", 200, 5);
    let writing = file("e.tif.part", 900, 9999);
    // 1 400 bytes of pyramids, at most 800: the oldest goes, the kept one stays, then the next oldest.
    let freed = super::pyramid::prune(&dir, 800, std::slice::from_ref(&kept));
    assert_eq!(freed, 700);
    assert!(!oldest.exists() && !old.exists());
    assert!(kept.exists() && recent.exists() && writing.exists());
    // Within the budget: nothing goes.
    assert_eq!(super::pyramid::prune(&dir, 800, &[]), 0);
    let _ = std::fs::remove_dir_all(&dir);
}

/// The first useful view of a multi-gigabyte orthophoto by HTTP ranges (docs/adr/0642 §5): its header, the whole
/// raster in a 1100-pixel window (an overview's tiles) and a close view (level 0's), each with the bytes it fetched and
/// how long it took. The file is `scripts/perf/big_cog.py`'s (.run/perf/orto-buyuk.tif). Release, by hand:
/// `cargo test --release -p kentos-desktop --lib rasters::tests::first_view_of_a_large_cog -- --ignored --nocapture`.
#[test]
#[ignore = "timings, run by hand in release"]
fn first_view_of_a_large_cog() {
    use crate::range_server::serve_paths;
    use std::time::Instant;
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/perf/orto-buyuk.tif");
    let Ok(meta) = std::fs::metadata(&path) else {
        println!("{} yok: önce /usr/bin/python3 scripts/perf/big_cog.py", path.display());
        return;
    };
    let size = meta.len();
    let server = serve_paths(vec![("orto.tif", path.clone())]);
    let url = server.url("orto.tif");
    let fetched = |server: &crate::range_server::Server| -> u64 {
        server.asked().iter().map(|(a, b)| b - a + 1).sum()
    };
    let row = |what: &str, ms: f64, bytes: u64, asks: usize| {
        println!(
            "{what:<52} {ms:>9.1} ms  {:>9.2} MB  {asks:>5} istek",
            bytes as f64 / 1e6
        );
    };
    println!("dosya {:.2} GB", size as f64 / 1e9);
    let t = Instant::now();
    let opened = super::tiles::open(&super::tiles::Origin::Url(url.clone()), None).expect("opens");
    let levels: Vec<(u32, u32)> = {
        let r = opened.reader.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        r.levels.iter().map(|l| (l.width, l.height)).collect()
    };
    row(
        &format!("açılış: başlık ({} kat)", levels.len()),
        t.elapsed().as_secs_f64() * 1e3,
        fetched(&server),
        server.asked().len(),
    );
    let s = super::tiles::service();
    let key = format!("url:{url}");
    let look = r#"{"render":"rgb","bands":[1,2,3]}"#;
    let affine = [500_000.0, 0.1, 0.0, 4_500_000.0, 0.0, -0.1];
    // Every tile of a window at a level, made by the service as the scene asks for them.
    let view = |level: usize, x0: u32, y0: u32, w: u32, h: u32| {
        let (tx0, ty0, tx1, ty1) = (x0 / 256, y0 / 256, (x0 + w - 1) / 256, (y0 + h - 1) / 256);
        let mut left: Vec<(u32, u32)> =
            (ty0..=ty1).flat_map(|ty| (tx0..=tx1).map(move |tx| (tx, ty))).collect();
        let n = left.len();
        for _ in 0..60_000 {
            s.register(&key, || Some(super::tiles::Origin::Url(url.clone())));
            s.frame();
            left.retain(|&(tx, ty)| s.tile(&key, look, &affine, level as u32, tx, ty).is_none());
            if left.is_empty() {
                return n;
            }
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        panic!("the view's tiles were not made");
    };
    // The whole raster in a window 1100 pixels wide: the first level no wider than twice that.
    let whole = levels
        .iter()
        .position(|(w, _)| *w <= 2200)
        .unwrap_or(levels.len() - 1);
    let (ww, wh) = levels[whole];
    let (before, asks) = (fetched(&server), server.asked().len());
    let t = Instant::now();
    let n = view(whole, 0, 0, ww, wh);
    row(
        &format!("bütün görüntü: {whole}. kat {ww} × {wh}, {n} karo"),
        t.elapsed().as_secs_f64() * 1e3,
        fetched(&server) - before,
        server.asked().len() - asks,
    );
    // A close view in the middle: 1100 × 650 pixels of level 0.
    let (w0, h0) = levels[0];
    let (before, asks) = (fetched(&server), server.asked().len());
    let t = Instant::now();
    let n = view(0, w0 / 2, h0 / 2, 1100, 650);
    row(
        &format!("yakın görünüm: 0. kat 1100 × 650, {n} karo"),
        t.elapsed().as_secs_f64() * 1e3,
        fetched(&server) - before,
        server.asked().len() - asks,
    );
    println!(
        "toplam aktarılan {:.2} MB, dosyanın %{:.3}'ü",
        fetched(&server) as f64 / 1e6,
        fetched(&server) as f64 / size as f64 * 100.0
    );
}
