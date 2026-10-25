//! Map services of docs/adr/0208 over a CBS project in TUREF / TM33
//! (EPSG:5255) at Ankara's Kızılay: a block of parcels and a building drawn
//! over a ready basemap, its tiles from the service itself (the network is
//! needed), in the project's system. The web's are `shots.mjs services`.
//! `tools_screens` takes them in the dark and the light theme at 1440×900
//! and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=servis-osm,servis-uydu,servis-vektor,servis-sivas,servis-topo,servis-atif,servis-baglantilar,servis-pencere,servis-wms,servis-wms-cizim,servis-arcgis,servis-veri,servis-ogc-uygunluk,servis-veri-alindi,servis-oznitelik,servis-bilgi cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use kentos_ui::snapshot::Snapshot;

use crate::app::{App, Message};
use crate::document::Document;
use crate::tools_screens::Pointed;

/// Kızılay in TUREF / TM33 (pyproj: 32.8541 E, 39.9208 N).
const X0: f64 = 487_526.0;
const Y0: f64 = 4_420_745.0;

fn layer(id: &str, name: &str, color: &str, extra: serde_json::Value) -> serde_json::Value {
    let mut node = serde_json::json!({
        "id": id, "name": name, "type": "layer", "visible": true, "locked": false, "expanded": true,
        "style": {"color": color, "lineType": "continuous", "lineWeight": 0.35, "fill": format!("{color}33")},
        "children": []
    });
    if let (Some(n), Some(e)) = (node.as_object_mut(), extra.as_object()) {
        n.extend(e.clone());
    }
    node
}

fn rect(id: u32, layer: &str, x: f64, y: f64, w: f64, h: f64, name: &str) -> serde_json::Value {
    serde_json::json!({
        "kind": "polygon", "id": id, "layerId": layer, "attrs": {"Ada": "1234", "Parsel": name},
        "label": name,
        "pts": [{"x": x, "y": y}, {"x": x + w, "y": y}, {"x": x + w, "y": y + h}, {"x": x, "y": y + h}]
    })
}

/// The drawing: parcels and a building over `service`, the view fitted to the block.
pub(crate) fn opened(app: &mut App, service: serde_json::Value, connections: serde_json::Value) {
    opened_at(app, service, connections, 5255, (X0, Y0), (260.0, 170.0));
}

/// As `opened`, in the system `srid`, the block and the view around `centre` (half the view's width and height).
fn opened_at(
    app: &mut App,
    service: serde_json::Value,
    connections: serde_json::Value,
    srid: u32,
    centre: (f64, f64),
    half: (f64, f64),
) {
    let (x0, y0) = centre;
    let mut entities = Vec::new();
    let mut id = 1;
    for (i, w) in [24.0, 20.0, 26.0, 22.0].iter().enumerate() {
        let x = x0 - 60.0 + [0.0, 24.0, 44.0, 70.0][i];
        entities.push(rect(
            id,
            "parsel",
            x,
            y0 - 10.0,
            *w,
            34.0,
            &format!("{}", 11 + i),
        ));
        id += 1;
    }
    entities.push(rect(id, "yapi", x0 - 52.0, y0 + 2.0, 12.0, 14.0, "A Blok"));
    let drawing = serde_json::json!({
        "format": "kentos.document", "version": 1, "name": "Kızılay",
        "settings": {"srid": srid, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
            "plotScale": 1000, "workspace": "gis", "drawingFont": "barlow", "connections": connections},
        "origin": {"x": x0, "y": y0},
        "layers": [
            layer("parsel", "Parsel", "#E5484D", serde_json::json!({})),
            layer("yapi", "Yapı", "#3E63DD", serde_json::json!({})),
            layer("altlik", "Altlık", "fg", serde_json::json!({"service": service})),
        ],
        "activeLayer": "parsel",
        "entities": entities,
        "styles": {"items": [], "categories": []}
    });
    let snapshot = kentos_commands::DocumentSnapshotV1::from_json(&drawing.to_string())
        .expect("the drawing reads");
    let doc = Document::new(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app.tab = "map";
    app.command_expanded = false;
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: x0 - half.0,
            min_y: y0 - half.1,
            max_x: x0 + half.0,
            max_y: y0 + half.1,
        },
        24.0,
    );
}

/// A ready basemap's service and the connection it names.
pub(crate) fn preset(id: &str) -> (serde_json::Value, serde_json::Value) {
    let p = kentos_services::presets::preset(id).expect("a preset");
    let service = serde_json::to_value(kentos_services::presets::layer_of(p)).expect("serializes");
    let connections =
        serde_json::to_value(p.connection.iter().collect::<Vec<_>>()).expect("serializes");
    (service, connections)
}

fn window(app: &mut App, e: crate::services::window::Event) {
    let _ = app.update(Message::Services(crate::services::app::Event::Window(e)));
}

/// Harita servisi on kind `kind` (its place in the window's list), `url`
/// read as Bağlan reads it, and `pick` picked when given.
fn connected(app: &mut App, kind: usize, url: &str, pick: &str) {
    let _ = app.run("service.add");
    window(app, crate::services::window::Event::Kind(kind));
    window(app, crate::services::window::Event::Url(url.into()));
    let k = crate::services::window::KINDS[kind]
        .0
        .expect("a service kind");
    let answer = crate::services::window::connect_now(k, url, None);
    window(app, crate::services::window::Event::Connected(answer));
    if !pick.is_empty() || k == kentos_commands::ServiceKind::Arcgis {
        window(app, crate::services::window::Event::Pick(pick.into()));
    }
}

/// A project in Web Mercator over Kansas with GeoSolutions' GeoServer's US states as a WMS.
fn states(app: &mut App) {
    let service = serde_json::json!({
        "kind": "wms", "url": "https://gs-stable.geo-solutions.it/geoserver/wms", "layers": ["topp:states"],
        "srid": 3857, "version": "1.3.0", "format": "image/png", "transparent": true
    });
    let drawing = serde_json::json!({
        "format": "kentos.document", "version": 1, "name": "Kansas",
        "settings": {"srid": 3857, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
            "plotScale": 1000, "workspace": "gis", "drawingFont": "barlow"},
        "origin": {"x": -10_900_000.0, "y": 4_700_000.0},
        "layers": [
            layer("eyalet", "ABD eyaletleri", "fg", serde_json::json!({"service": service})),
            layer("altlik", "Altlık", "fg", serde_json::json!({"service": preset("osm-standard").0})),
        ],
        "activeLayer": "eyalet",
        "entities": [],
        "styles": {"items": [], "categories": []}
    });
    let snapshot = kentos_commands::DocumentSnapshotV1::from_json(&drawing.to_string())
        .expect("the drawing reads");
    let doc = Document::new(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app.tab = "map";
    app.command_expanded = false;
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: -11_600_000.0,
            min_y: 4_300_000.0,
            max_x: -10_200_000.0,
            max_y: 5_100_000.0,
        },
        24.0,
    );
}

fn feed_event(app: &mut App, e: crate::services::feed_window::Event) {
    let _ = app.update(Message::Services(crate::services::app::Event::Feed(e)));
}

/// Servisten veri al read from Esri's world countries layer, the first type picked.
fn feed_connected(app: &mut App) {
    let _ = app.run("service.feed");
    feed_event(app, crate::services::feed_window::Event::Kind(2));
    let url = "https://services.arcgis.com/P3ePLMYs2RVChkJx/arcgis/rest/services/World_Countries_(Generalized)/FeatureServer";
    feed_event(app, crate::services::feed_window::Event::Url(url.into()));
    let answer = (|| {
        let (mut c, mut next) =
            kentos_services::feed::FeedConnecting::start(kentos_commands::FeedKind::Arcgis, url)?;
        while let Some(req) = next {
            let body = crate::services::net::send_text(req, None, url, 6)?;
            next = c.answer(&body)?;
        }
        Ok(Box::new(c))
    })();
    feed_event(app, crate::services::feed_window::Event::Connected(answer));
}

/// Servisten veri al on an OGC API service of the test's own answers (no network): its collection and its conformance
/// page without Part 3, a filter typed (docs/adr/0513).
fn ogc_connected(app: &mut App) {
    let _ = app.run("service.feed");
    feed_event(app, crate::services::feed_window::Event::Kind(1));
    let url = "https://cbs.ornek.gov.tr/ogc/collections";
    feed_event(app, crate::services::feed_window::Event::Url(url.into()));
    let collections = r#"{"collections":[{"id":"durak","title":"Otobüs durakları",
        "links":[{"rel":"items","href":"https://cbs.ornek.gov.tr/ogc/collections/durak/items"}],
        "crs":["http://www.opengis.net/def/crs/OGC/1.3/CRS84","http://www.opengis.net/def/crs/EPSG/0/5255"]}]}"#;
    let conformance = r#"{"conformsTo":["http://www.opengis.net/spec/ogcapi-features-1/1.0/conf/core",
        "http://www.opengis.net/spec/ogcapi-features-1/1.0/conf/geojson",
        "http://www.opengis.net/spec/ogcapi-features-2/1.0/conf/crs"]}"#;
    let answer = (|| {
        let (mut c, mut next) = kentos_services::feed::FeedConnecting::start(
            kentos_commands::FeedKind::OgcFeatures,
            url,
        )?;
        while next.is_some() {
            next = c.answer(collections)?;
        }
        if c.conformance_request().is_some() {
            c.read_conformance(Some(conformance));
        }
        Ok(Box::new(c))
    })();
    feed_event(app, crate::services::feed_window::Event::Connected(answer));
    feed_event(
        app,
        crate::services::feed_window::Event::Item("durak".into()),
    );
    feed_event(
        app,
        crate::services::feed_window::Event::Filter("hat = '410'".into()),
    );
}

fn settled(s: &mut Snapshot, app: &mut App) {
    let mut update = |app: &mut App, message: Message| {
        let _ = app.update(message);
    };
    s.settle(app, App::view, &mut update);
}

/// Draws until the services have brought every tile the view asked for.
pub(crate) fn tiles_come(s: &mut Snapshot, app: &mut App) {
    let mut update = |app: &mut App, message: Message| {
        let _ = app.update(message);
    };
    for _ in 0..12 {
        s.settle(app, App::view, &mut update);
        let _ = s.render(app.view(), &app.theme());
        std::thread::sleep(std::time::Duration::from_millis(30));
        crate::services::hub().settle(std::time::Duration::from_secs(30));
    }
    s.settle(app, App::view, &mut update);
}

pub(crate) fn pointed() -> Vec<Pointed> {
    vec![
        (
            "servis-osm",
            |app| {
                let (s, c) = preset("osm-standard");
                opened(app, s, c);
            },
            tiles_come,
        ),
        (
            "servis-topo",
            |app| {
                let (s, c) = preset("osm-topo");
                opened(app, s, c);
            },
            tiles_come,
        ),
        (
            "servis-uydu",
            |app| {
                let (s, c) = preset("esri-imagery");
                opened(app, s, c);
            },
            tiles_come,
        ),
        (
            "servis-vektor",
            |app| {
                let (s, c) = preset("ofm-liberty");
                opened(app, s, c);
            },
            tiles_come,
        ),
        // A municipality's own vector tiles and MapLibre style (Sivas Kent Rehberi, Martin; docs/adr/0635): the
        // style read as the map's, its tiles drawn in TUREF / TM36 over the city centre.
        (
            "servis-sivas",
            |app| {
                let service = serde_json::json!({
                    "kind": "vector",
                    "url": "https://harita.sivas.bel.tr/style/kentrehberi",
                    "attribution": "© Sivas Belediyesi"
                });
                opened_at(
                    app,
                    service,
                    serde_json::json!([]),
                    5256,
                    (587_084.0, 4_402_320.0),
                    (187.0, 120.0),
                );
            },
            tiles_come,
        ),
        // A basemap that needs a key: added, and Bağlantılar opens on its connection.
        (
            "servis-baglantilar",
            |app| {
                let (s, c) = preset("osm-standard");
                opened(app, s, c);
                let _ = app.run("basemap.hgmHarita");
            },
            settled,
        ),
        // Harita servisi on its ready basemaps.
        (
            "servis-pencere",
            |app| {
                let (s, c) = preset("osm-standard");
                opened(app, s, c);
                let _ = app.run("service.add");
            },
            settled,
        ),
        // A WMS read (terrestris' OSM-WMS), its layers listed and one picked.
        (
            "servis-wms",
            |app| {
                let (s, c) = preset("osm-standard");
                opened(app, s, c);
                connected(app, 2, "https://ows.terrestris.de/osm/service", "OSM-WMS");
            },
            settled,
        ),
        // The WMS added over the basemap as a clear layer, its tiles drawn.
        (
            "servis-wms-cizim",
            |app| {
                let (s, c) = preset("osm-standard");
                opened(app, s, c);
                connected(
                    app,
                    2,
                    "https://ows.terrestris.de/osm/service",
                    "OSM-Overlay-WMS",
                );
                window(app, crate::services::window::Event::Add);
            },
            tiles_come,
        ),
        // An ArcGIS REST service read: its layers under the whole.
        (
            "servis-arcgis",
            |app| {
                let (s, c) = preset("osm-standard");
                opened(app, s, c);
                connected(
                    app,
                    6,
                    "https://services.arcgisonline.com/arcgis/rest/services/World_Topo_Map/MapServer",
                    "",
                );
            },
            settled,
        ),
        // Servisten veri al on an ArcGIS feature layer (Esri's world countries), Görünüm's area.
        (
            "servis-veri",
            |app| {
                let (s, c) = preset("osm-standard");
                opened(app, s, c);
                feed_connected(app);
            },
            settled,
        ),
        // An OGC API service's conformance in the window, a filter it does not read (docs/adr/0513).
        (
            "servis-ogc-uygunluk",
            |app| {
                let (s, c) = preset("osm-standard");
                opened(app, s, c);
                ogc_connected(app);
            },
            settled,
        ),
        // Its objects taken into a new layer that remembers its feed.
        (
            "servis-veri-alindi",
            |app| {
                let (s, c) = preset("osm-standard");
                opened(app, s, c);
                feed_connected(app);
                let feed = {
                    let w = app.feed_window.as_ref().expect("open");
                    let choice = kentos_services::feed::FeedChoice {
                        item: w.item.clone().unwrap_or_default(),
                        bbox: Some({
                            let b = app.viewport.camera.visible_bounds();
                            [b.min_x, b.min_y, b.max_x, b.max_y]
                        }),
                        ..Default::default()
                    };
                    w.connecting
                        .as_ref()
                        .expect("read")
                        .feed(&choice, 5255)
                        .expect("a feed")
                };
                let project = crate::services::systems::ProjectSystem::of(
                    app.document.as_ref().expect("open").model.settings(),
                );
                let stop = std::sync::atomic::AtomicBool::new(false);
                let taken = crate::services::feed_window::take_all(
                    &feed,
                    &project,
                    None,
                    1000,
                    false,
                    &stop,
                    |_| {},
                );
                feed_event(
                    app,
                    crate::services::feed_window::Event::Taken(taken.map(Box::new)),
                );
                app.viewport.camera.fit(
                    &kentos_render_wgpu::Bounds {
                        min_x: X0 - 900_000.0,
                        min_y: Y0 - 450_000.0,
                        max_x: X0 + 900_000.0,
                        max_y: Y0 + 450_000.0,
                    },
                    24.0,
                );
            },
            tiles_come,
        ),
        // A layer drawn from a service chosen in the tree: Öznitelikler's Servis katmanı.
        (
            "servis-oznitelik",
            |app| {
                let (s, c) = preset("osm-standard");
                opened(app, s, c);
                connected(
                    app,
                    2,
                    "https://ows.terrestris.de/osm/service",
                    "OSM-Overlay-WMS",
                );
                window(app, crate::services::window::Event::Add);
                let id = app
                    .document
                    .as_ref()
                    .and_then(|d| {
                        d.model
                            .layers()
                            .leaves()
                            .iter()
                            .find(|n| {
                                n.service
                                    .as_ref()
                                    .is_some_and(|s| s.kind == kentos_commands::ServiceKind::Wms)
                            })
                            .map(|n| n.id.clone())
                    })
                    .expect("the WMS layer");
                app.selected_layer = Some(id);
            },
            tiles_come,
        ),
        // Servis bilgisi on GeoSolutions' US states (a project in Web Mercator over Kansas).
        (
            "servis-bilgi",
            |app| {
                states(app);
                let p = kentos_interaction::Vec2::new(-10_910_000.0, 4_680_000.0);
                app.service_info.wanted = Some(p);
                let _ = app.service_info_tasks();
                // The requests waited for here, as the thread would.
                let doc = app.document.as_ref().expect("open");
                let s = doc
                    .model
                    .layers()
                    .leaves()
                    .iter()
                    .find_map(|n| n.service.clone())
                    .expect("a service");
                let answers = kentos_services::info::requests(&s, 3857, p.x, p.y, 30.0)
                    .into_iter()
                    .find_map(|(media, req)| {
                        crate::services::net::send_text(req, None, &s.url, 4)
                            .ok()
                            .map(|body| kentos_services::info::read(&media, &body))
                    })
                    .ok_or_else(|| "yanıt yok".to_owned());
                app.service_info_answered(vec![crate::services::info::Answer {
                    layer: "ABD eyaletleri".into(),
                    rows: answers,
                }]);
            },
            tiles_come,
        ),
        // The credits' card the strip opens.
        (
            "servis-atif",
            |app| {
                let (s, c) = preset("osm-topo");
                opened(app, s, c);
                app.service_credits = true;
            },
            tiles_come,
        ),
    ]
}
