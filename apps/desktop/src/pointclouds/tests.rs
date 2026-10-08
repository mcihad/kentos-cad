//! The point clouds of the desktop (docs/adr/0207): the shared fixtures
//! added, indexed, drawn and queried as the windows do it.

use super::*;

#[test]
fn a_files_stem_and_its_library_id() {
    assert_eq!(file_stem("/veri/bulut.copc.laz"), "bulut");
    assert_eq!(file_stem("https://ornek.org/k/ada.laz?sig=1"), "ada");
    assert_eq!(file_stem("noktalar.xyz"), "noktalar");
    assert!(cloud_id(b"abc").starts_with("pointcloud-"));
    assert_eq!(cloud_id(b"abc").len(), "pointcloud-".len() + 16);
}

#[test]
fn a_clouds_look_is_written_through_the_properties_edit() {
    let mut app = crate::files_testing::app_with_drawing();
    crate::cloud_scenes::opened(&mut app, [0.0, 0.0, 120.0, 90.0]);
    let doc = app.document.as_mut().expect("a drawing");
    let (slot, mut c) = doc
        .model
        .entities()
        .find_map(|e| match e {
            Entity::PointCloud(c) => Some((kentos_domain::Slot(c.base.id), c.clone())),
            _ => None,
        })
        .expect("the cloud");
    c.cloud.style.render = kentos_contracts::CloudRender::Classification;
    let said =
        kentos_interaction::properties::set_geometry(&mut doc.model, slot, &Entity::PointCloud(c));
    assert!(said.is_empty(), "{said:?}");
    let Some(Entity::PointCloud(after)) = doc.model.get(slot) else {
        panic!("the cloud");
    };
    assert_eq!(
        after.cloud.style.render,
        kentos_contracts::CloudRender::Classification
    );
}

use crate::app::{App, Message, Picker};
use crate::files_testing::{app_with_drawing, drive};
use crate::range_server::{Mode, serve};

fn koy() -> std::path::PathBuf {
    crate::cloud_scenes::folder().join("koy.laz")
}

fn send(app: &mut App, e: add::Event) {
    let task = app.update(Message::PointClouds(Event::Add(e)));
    drive(app, task);
}

fn clouds(app: &App) -> Vec<kentos_contracts::PointCloudEntity> {
    app.document
        .as_ref()
        .expect("a drawing")
        .model
        .entities()
        .filter_map(|e| match e {
            Entity::PointCloud(c) => Some(c.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn an_address_is_read_by_ranges_a_block_at_a_time() {
    let data = std::fs::read(koy()).expect("the village's LiDAR");
    let server = serve(vec![("koy.laz", data.clone())], Mode::Ranges);
    let r = bytes::Remote::open(&server.url("koy.laz")).expect("opens");
    assert_eq!(r.size, data.len() as u64);
    assert_eq!(r.version.as_deref(), Some("\"v1\""));
    assert_eq!(r.read(10, 100).expect("reads"), data[10..110]);
    // Across two blocks: the missing neighbours in one request, then kept.
    let before = server.requests.load(std::sync::atomic::Ordering::SeqCst);
    let across = r.read(65_000, 2_000).expect("reads");
    assert_eq!(across, data[65_000..67_000]);
    let after = server.requests.load(std::sync::atomic::Ordering::SeqCst);
    assert_eq!(after - before, 1, "one request for the second block");
    assert_eq!(r.read(65_100, 50).expect("reads"), data[65_100..65_150]);
    assert_eq!(
        server.requests.load(std::sync::atomic::Ordering::SeqCst),
        after,
        "kept blocks are not asked again"
    );
    // Past the end is refused before a request.
    assert!(r.read(data.len() as u64 - 4, 8).is_err());
    let asked = server.asked();
    assert!(
        asked.iter().all(|&(a, b)| a <= b && b < data.len() as u64),
        "{asked:?}"
    );
}

#[test]
fn a_server_without_ranges_or_a_changed_address_is_refused() {
    let data = std::fs::read(koy()).expect("the village's LiDAR");
    let whole = serve(vec![("koy.laz", data.clone())], Mode::Whole);
    let why = bytes::Remote::open(&whole.url("koy.laz")).expect_err("refused");
    assert!(
        why.contains("parça parça okumaya (HTTP Range) izin vermiyor"),
        "{why}"
    );
    let changing = serve(vec![("koy.laz", data)], Mode::VersionFrom(2));
    let r = bytes::Remote::open(&changing.url("koy.laz")).expect("the first answer is v1");
    let why = r.read(0, 100).expect_err("v2 now");
    assert!(why.contains("açıldıktan sonra değişmiş"), "{why}");
    let missing = serve(Vec::new(), Mode::Ranges);
    let why = bytes::Remote::open(&missing.url("yok.laz")).expect_err("not there");
    assert!(why.contains("404"), "{why}");
}

#[test]
fn nokta_bulutu_ekle_writes_its_layer_and_the_cloud_as_one_step() {
    crate::cloud_scenes::test_cache();
    let mut app = app_with_drawing();
    let layers = |app: &App| {
        app.document
            .as_ref()
            .expect("a drawing")
            .model
            .layers()
            .leaves()
            .len()
    };
    let before = layers(&app);
    app.picker = Picker::File(koy());
    let task = app.run("pointcloud.add");
    drive(&mut app, task);
    let s = app.clouds.add.as_ref().expect("the window");
    let read = s.read.as_ref().expect("read").as_ref().expect("a cloud");
    assert_eq!(read.members.len(), 1);
    assert_eq!(read.members[0].count, 71_212);
    assert_eq!(read.members[0].format, kentos_contracts::CloudFormat::Laz);
    assert_eq!(s.layer, "koy");
    send(&mut app, add::Event::Run);
    assert!(app.clouds.add.is_none(), "the window closed");
    let added = clouds(&app);
    assert_eq!(added.len(), 1);
    assert_eq!(added[0].cloud.count, 71_212);
    assert!(
        added[0].cloud.sources[0]
            .file
            .as_deref()
            .is_some_and(|f| f.ends_with("koy.laz"))
    );
    assert_eq!(layers(&app), before + 1);
    let doc = app.document.as_mut().expect("a drawing");
    assert_eq!(doc.model.undo().as_deref(), Some(add::TITLE));
    assert!(clouds(&app).is_empty());
    assert_eq!(layers(&app), before, "the layer went with the cloud");
}

#[test]
fn a_cloud_is_added_from_an_address() {
    let data = std::fs::read(koy()).expect("the village's LiDAR");
    let server = serve(vec![("koy.laz", data)], Mode::Ranges);
    let url = server.url("koy.laz");
    let mut app = app_with_drawing();
    app.picker = Picker::File(koy());
    let task = app.run("pointcloud.add");
    drive(&mut app, task);
    send(&mut app, add::Event::From(add::From::Address));
    let s = app.clouds.add.as_ref().expect("the window");
    assert!(
        s.read.is_none() && s.address.is_empty(),
        "nothing read for an address yet"
    );
    send(&mut app, add::Event::Address(url.clone()));
    send(&mut app, add::Event::ReadAddress);
    let s = app.clouds.add.as_ref().expect("the window");
    let read = s
        .read
        .as_ref()
        .expect("read")
        .as_ref()
        .expect("a cloud from the address");
    assert_eq!(read.members[0].origin, bytes::Origin::Url(url.clone()));
    assert_eq!(read.members[0].count, 71_212);
    // The probe (byte 0), then the header, its tables and the sample's chunks
    // (here both of the file's two), each byte once: no request asks for what
    // an earlier one brought.
    let asked = server.asked();
    assert_eq!(asked[0], (0, 0), "the probe first");
    let asked = asked[1..].to_vec();
    let fetched: u64 = asked.iter().map(|(a, b)| b - a + 1).sum();
    assert!(
        fetched <= std::fs::metadata(koy()).expect("its size").len(),
        "{asked:?}"
    );
    let mut spans = asked.clone();
    spans.sort_unstable();
    assert!(
        spans.windows(2).all(|w| w[0].1 < w[1].0),
        "overlapping requests: {asked:?}"
    );
    send(&mut app, add::Event::Run);
    let added = clouds(&app);
    assert_eq!(added.len(), 1);
    assert_eq!(added[0].cloud.sources[0].url.as_deref(), Some(url.as_str()));
    assert!(added[0].cloud.sources[0].file.is_none());
}

#[test]
fn the_cloud_window_without_a_file_offers_an_address() {
    let mut app = app_with_drawing();
    // The dialog closed without a file: the window stays, nothing read.
    app.cloud_add_open_for_tests();
    send(&mut app, add::Event::Picked(None));
    let s = app.clouds.add.as_ref().expect("the window stays");
    assert!(s.paths.is_empty() && s.read.is_none());
    send(&mut app, add::Event::From(add::From::Address));
    assert_eq!(
        app.clouds.add.as_ref().expect("the window").from,
        add::From::Address
    );
    send(&mut app, add::Event::Close);
    assert!(app.clouds.add.is_none());
}

#[test]
fn a_virtual_cloud_is_saved_and_read_back_as_a_vpc() {
    crate::cloud_scenes::test_cache();
    let mut app = app_with_drawing();
    crate::cloud_scenes::opened(&mut app, [0.0, 0.0, 120.0, 90.0]);
    let c = clouds(&app).remove(0);
    let dir =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/pointcloud-tests");
    std::fs::create_dir_all(&dir).expect("the folder");
    let vpc = dir.join("koy.vpc");
    let text = vpc::text_of(
        &c.cloud,
        &vpc,
        Some(&crate::cloud_scenes::folder().join("..")),
    )
    .expect("written");
    let back = kentos_pointcloud::vpc::read(&text).expect("reads back");
    assert_eq!(back.len(), 1);
    assert_eq!(back[0].count, Some(71_212));
    assert!(back[0].href.ends_with("koy.laz"), "{}", back[0].href);
}

#[test]
fn xyz_sor_says_the_nearest_points_facts_by_the_tools_name() {
    use iced::{Point, Rectangle, Size};
    crate::cloud_scenes::test_cache();
    let mut app = app_with_drawing();
    crate::cloud_scenes::opened(&mut app, [20.0, 52.0, 70.0, 86.0]);
    let _ = app.update(Message::Viewport(crate::viewport::Event::Resized(
        Rectangle::new(Point::ORIGIN, Size::new(800.0, 600.0)),
    )));
    // A roof in the middle of the area, 10 pixels a metre.
    app.viewport.camera.center = kentos_interaction::Vec2::new(
        crate::cloud_scenes::X0 + 44.0,
        crate::cloud_scenes::Y0 + 70.5,
    );
    app.viewport.camera.scale = 10.0;
    let task = app.run("pointcloud.query");
    drive(&mut app, task);
    assert!(
        app.log.lines().any(|l| l.text == "Nokta bulutu XYZ sor"),
        "the command line says the tool by its name"
    );
    let p = Point::new(400.0, 300.0);
    let mut said = String::new();
    // The first click may find the file still opening (its index made once): it says so; again.
    for _ in 0..3 {
        for e in [
            crate::viewport::Event::Moved(p),
            crate::viewport::Event::Pressed(p),
            crate::viewport::Event::Released(p),
        ] {
            let task = app.update(Message::Viewport(e));
            drive(&mut app, task);
        }
        said = app.log.last().map(|l| l.text.clone()).unwrap_or_default();
        if said.contains("Y=") {
            break;
        }
        service::service().settle(std::time::Duration::from_secs(60));
    }
    assert!(said.starts_with("Nokta bulutu Köy (LiDAR): Y="), "{said}");
    assert!(said.contains("sınıf 6 (Bina)"), "{said}");
}
