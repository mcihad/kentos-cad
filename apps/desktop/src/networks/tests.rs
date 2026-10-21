//! Ağ analizi in the desktop (docs/adr/0209), on the network traces' drawing
//! (fixtures/interaction/v1/networks.kcad): Ağlar lists the project's
//! networks and checks the chosen one on the network thread, Kaydet writes a
//! changed one, En yakın tesis writes its ways in one step; and the pictures
//! for the owner (the web's `shots.mjs networks`).

use kentos_commands::DocumentSnapshotV1;
use serde_json::json;

use super::Event as Networks;
use super::window::Event;
use crate::app::{App, Message};
use crate::document::Document;
use crate::files_testing::drive;
use crate::processing::Event as Processing;
use crate::processing::dialog::RunStatus;

const DRAWING: &str = include_str!("../../../../fixtures/interaction/v1/networks.kcad");

/// The app with the network traces' drawing open.
fn app() -> App {
    let (mut app, _) = App::boot(None);
    let snapshot = DocumentSnapshotV1::from_json(DRAWING).expect("the drawing reads");
    let doc = Document::new(snapshot, None).expect("the drawing opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

/// A message and the tasks it starts, to their end (the network thread's answers come back as messages).
fn send(app: &mut App, message: Message) {
    let task = app.update(message);
    drive(app, task);
}

fn window(app: &mut App, e: Event) {
    send(app, Message::Networks(Networks::Window(e)));
}

fn processing(app: &mut App, e: Processing) {
    send(app, Message::Processing(e));
}

fn value(app: &mut App, name: &str, v: serde_json::Value) {
    processing(app, Processing::Value(name.into(), v));
}

fn layer(id: &str) -> serde_json::Value {
    json!({ "scope": "layer", "layerId": id })
}

/// Denetle's counts for the chosen network, as the window keeps them.
fn checked(app: &App) -> (usize, usize, f64) {
    let c = app
        .networks
        .window
        .as_ref()
        .and_then(|w| w.checked())
        .expect("Denetle answered");
    (c.nodes, c.pieces, c.length)
}

#[test]
fn the_window_lists_the_networks_and_checks_the_chosen_one_off_the_thread() {
    let mut app = app();
    send(&mut app, Message::Run("network.manage"));
    let names: Vec<String> = app.networks.window.as_ref().expect("Ağlar is open").names();
    assert_eq!(names, ["Yollar", "İçme suyu"]);
    window(&mut app, Event::Check);
    // The street grid: 12 crossings, 17 blocks, the curved one 123.175 m (scripts/fixtures/network_cases.py).
    let (nodes, pieces, length) = checked(&app);
    assert_eq!((nodes, pieces), (12, 17));
    assert!((length - 2_043.174_829_346_325_6).abs() < 1e-6, "{length}");
    window(&mut app, Event::Choose(1));
    window(&mut app, Event::Check);
    // The pipes: the depot, five valves and the two junctions; ten pieces, 526 m and the south branch's 54 m past V-5.
    let (nodes, pieces, length) = checked(&app);
    assert_eq!((nodes, pieces), (11, 10));
    assert!((length - 580.0).abs() < 1e-9, "{length}");
    // Denetle builds the window's definition under a key of its own: the project's network is not built for it.
    assert!(!app.networks.built_for("yollar"));
}

#[test]
fn kaydet_writes_the_changed_network_as_a_project_setting() {
    let mut app = app();
    send(&mut app, Message::Run("network.manage"));
    window(&mut app, Event::Name("Ana yollar".into()));
    window(&mut app, Event::Save);
    assert!(app.networks.window.is_none(), "Kaydet closes the window");
    let doc = &app.document.as_ref().expect("open").model;
    let names: Vec<&str> = doc
        .settings()
        .networks
        .iter()
        .map(|n| n.name.as_str())
        .collect();
    assert_eq!(names, ["Ana yollar", "İçme suyu"]);
    assert!(
        app.log
            .said(kentos_interaction::Level::Info, "Ağlar kaydedildi: 2 ağ.")
    );
    // A project setting, not an undo step (docs/adr/0209 §2).
    assert!(!doc.can_undo());
}

#[test]
fn en_yakin_tesis_writes_a_way_for_each_school_in_one_step() {
    let mut app = app();
    send(
        &mut app,
        Message::Run("processing.run.network.closestFacility"),
    );
    value(
        &mut app,
        "network",
        json!({ "network": "yollar", "cost": "Süre" }),
    );
    value(&mut app, "incidents", layer("okul"));
    value(&mut app, "facilities", layer("itfaiye"));
    value(&mut app, "count", json!(1));
    processing(&mut app, Processing::Run);
    let status = app.processing.dialog.as_ref().map(|w| w.status.clone());
    let Some(RunStatus::Ok { text, .. }) = status else {
        panic!("{status:?}");
    };
    assert_eq!(
        text,
        "3 olay için 3 tesis bulundu; 3 yol “Yollar” ağında Süre ile yazıldı."
    );
    let doc = &mut app.document.as_mut().expect("open").model;
    let written = doc
        .entities()
        .filter(|e| e.base().layer_id.starts_with("islem-en-yakin-tesis"))
        .count();
    assert_eq!(written, 3);
    assert_eq!(doc.undo().as_deref(), Some("En yakın tesis"));
}

/// Ağlar with Yollar and with İçme suyu after Denetle, En yakın tesis and
/// Maliyet matrisi after a run, two results in the drawing and the CBS
/// ribbon's Ağ panel; `.run/shots/aglar-*` (the web's `shots.mjs networks`).
/// Not run by default:
/// `cargo test -p kentos-desktop networks::tests::screens -- --ignored --nocapture`.
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    let only = std::env::var("KENTOS_SHOTS_ONLY").ok();
    let views = [
        "yollar",
        "sebeke",
        "en-yakin",
        "en-yakin-cizim",
        "matris",
        "alanlar-cizim",
        "serit",
        "donusler",
        "iz-abone",
        "terminaller",
        "terminal-iz",
        "alt-aglar",
        "alt-aglar-guncel",
        "alt-aglar-serit",
        "yapilandirma",
        "guncel-tut",
        "profiller",
        "profil-yol",
    ];
    for (mode, suffix) in [("dark", ""), ("light", "-acik")] {
        for (width, height) in [(1440.0, 900.0), (1100.0, 650.0)] {
            for name in views {
                if only
                    .as_deref()
                    .is_some_and(|o| !o.split(',').any(|x| x == name))
                {
                    continue;
                }
                let mut app = match name {
                    "iz-abone" => subscribers_app(),
                    "terminaller" | "terminal-iz" | "alt-aglar" | "alt-aglar-guncel"
                    | "alt-aglar-serit" | "yapilandirma" | "guncel-tut" => {
                        super::terminal_tests::electric_app()
                    }
                    _ => app(),
                };
                let _ = app
                    .settings
                    .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
                app.apply_settings();
                let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
                let mut update = |app: &mut App, message| send(app, message);
                snapshot.settle(&mut app, App::view, &mut update);
                send(&mut app, Message::Run("view.zoomExtents"));
                match name {
                    "yollar" | "sebeke" => {
                        send(&mut app, Message::Run("network.manage"));
                        if name == "sebeke" {
                            window(&mut app, Event::Choose(1));
                        }
                        window(&mut app, Event::Check);
                    }
                    // docs/adr/0521: turns, levels and Süre's time profile in the form.
                    "donusler" => {
                        send(&mut app, Message::Run("network.manage"));
                        window(
                            &mut app,
                            Event::CostPeriods(0, "07:00-09:30 0,5; 17:00-19:00 0,6".into()),
                        );
                        window(&mut app, Event::UTurns(2));
                        window(&mut app, Event::Delay(1, "5".into()));
                        window(&mut app, Event::Delay(2, "15".into()));
                        window(&mut app, Event::Delay(3, "30".into()));
                        window(&mut app, Event::TurnAdd);
                        window(&mut app, Event::TurnField(0, "gecikme".into()));
                        window(&mut app, Event::LevelField(0, "z_bas".into()));
                        window(&mut app, Event::LevelField(1, "z_son".into()));
                    }
                    // docs/adr/0525: Yalıtım on the branch, its subscribers ringed, the short service pipe crossed.
                    "iz-abone" => {
                        app.memory.network_trace =
                            kentos_geometry_core::ops::network::TraceKind::Isolation;
                        send(&mut app, Message::Run("tool.netTrace"));
                        click_at(&mut app, [150.0, 50.0]);
                    }
                    // docs/adr/0557: Ağlar' Terminaller and Alt ağlar sections; a trace through the transformer along its
                    // path only (the pipe on the ort terminal stays out); Alt ağlar before and after Güncelle; the ribbon.
                    "terminaller" => {
                        send(&mut app, Message::Run("network.manage"));
                    }
                    "terminal-iz" => {
                        send(&mut app, Message::Run("tool.netTrace"));
                        click_at(&mut app, [50.0, 0.0]);
                    }
                    "alt-aglar" | "alt-aglar-guncel" => {
                        send(&mut app, Message::Run("network.subnetworks"));
                        if name == "alt-aglar-guncel" {
                            send(
                                &mut app,
                                Message::Subnetworks(crate::subnetworks_window::Event::Update),
                            );
                        }
                    }
                    "alt-aglar-serit" => {
                        let _ = app.update(Message::RibbonTab("analysis"));
                    }
                    // docs/adr/0562: a device's configurations by an attribute of its points, Güncel tut chosen.
                    "yapilandirma" => {
                        send(&mut app, Message::Run("network.manage"));
                        window(&mut app, Event::DeviceBy(0, "tip".into()));
                        window(
                            &mut app,
                            Event::DeviceConfigs(
                                0,
                                "ters = x: Giriş; y: Çıkış | x, y || üçlü = yg; ag; ort | yg, ag; ag, ort"
                                    .into(),
                            ),
                        );
                        window(&mut app, Event::SubnetReach(1));
                        window(&mut app, Event::SubnetKeep(1));
                    }
                    // docs/adr/0567: Ağlar's Araç profilleri and Tarihli kapanma; a route for Kamyon, Atatürk Caddesi
                    // forbidden to it, the way around.
                    "profiller" | "profil-yol" => {
                        send(&mut app, Message::Run("network.manage"));
                        window(&mut app, Event::ProfileAdd);
                        window(&mut app, Event::ProfileText(0, 0, "Kamyon".into()));
                        window(&mut app, Event::ProfileText(0, 1, "agirlik 18; yukseklik 4".into()));
                        window(&mut app, Event::ProfileText(0, 2, "Ad = Atatürk Caddesi".into()));
                        window(&mut app, Event::ProfileText(0, 3, "40".into()));
                        window(&mut app, Event::ProfileAdd);
                        window(&mut app, Event::ProfileText(1, 0, "Yaya".into()));
                        window(&mut app, Event::ProfileText(1, 3, "5".into()));
                        window(&mut app, Event::ProfileWays(1, 1));
                        window(&mut app, Event::ClosureField(0, "kapanis_bas".into()));
                        window(&mut app, Event::ClosureField(1, "kapanis_son".into()));
                        if name == "profil-yol" {
                            window(&mut app, Event::Save);
                            app.memory.network_profile =
                                kentos_interaction::Name::new("Kamyon").expect("a name");
                            send(&mut app, Message::Run("tool.netRoute"));
                            click_at(&mut app, [-180.0, 0.0]);
                            click_at(&mut app, [180.0, 0.0]);
                        }
                    }
                    // A pipe drawn at the valve of a network that keeps its names current: Alt ağlar finds nothing stale.
                    "guncel-tut" => {
                        send(&mut app, Message::Run("network.manage"));
                        window(&mut app, Event::SubnetReach(1));
                        window(&mut app, Event::SubnetKeep(1));
                        window(&mut app, Event::Save);
                        send(&mut app, Message::Run("tool.line"));
                        click_at(&mut app, [300.0, 0.0]);
                        click_at(&mut app, [300.0, 50.0]);
                        send(&mut app, Message::Run("network.subnetworks"));
                    }
                    "en-yakin" | "en-yakin-cizim" => {
                        send(
                            &mut app,
                            Message::Run("processing.run.network.closestFacility"),
                        );
                        value(
                            &mut app,
                            "network",
                            json!({ "network": "yollar", "cost": "Süre" }),
                        );
                        value(&mut app, "incidents", layer("okul"));
                        value(&mut app, "facilities", layer("itfaiye"));
                        value(&mut app, "count", json!(1));
                        processing(&mut app, Processing::Run);
                    }
                    "matris" => {
                        send(&mut app, Message::Run("processing.run.network.odMatrix"));
                        value(
                            &mut app,
                            "network",
                            json!({ "network": "yollar", "cost": "Süre" }),
                        );
                        value(&mut app, "origins", layer("okul"));
                        value(&mut app, "destinations", layer("itfaiye"));
                        processing(&mut app, Processing::Run);
                    }
                    "alanlar-cizim" => {
                        send(
                            &mut app,
                            Message::Run("processing.run.network.serviceAreas"),
                        );
                        value(
                            &mut app,
                            "network",
                            json!({ "network": "yollar", "cost": "Süre" }),
                        );
                        value(&mut app, "facilities", layer("okul"));
                        value(&mut app, "breaks", json!("0.2 0.4"));
                        value(&mut app, "trim", json!(20));
                        processing(&mut app, Processing::Run);
                    }
                    _ => {
                        // The CBS ribbon's Analiz: Ağlar and the network tools in their own panel.
                        let _ = app.update(Message::RibbonTab("analysis"));
                    }
                }
                snapshot.settle(&mut app, App::view, &mut update);
                if name.ends_with("-cizim") {
                    // The window closed: the result in the drawing.
                    processing(&mut app, Processing::Close);
                    snapshot.settle(&mut app, App::view, &mut update);
                } else if matches!(
                    name,
                    "matris" | "yollar" | "sebeke" | "donusler" | "terminaller" | "yapilandirma" | "profiller"
                ) {
                    // The form scrolled to its end: the table, or Denetle's answer under the definition.
                    let x = if name == "matris" {
                        width / 2.0 - 120.0
                    } else {
                        width / 2.0 + 150.0
                    };
                    snapshot.step(
                        &mut app,
                        App::view,
                        &mut update,
                        &[
                            iced::Event::Mouse(iced::mouse::Event::CursorMoved {
                                position: iced::Point::new(x, height / 2.0),
                            }),
                            iced::Event::Mouse(iced::mouse::Event::WheelScrolled {
                                delta: iced::mouse::ScrollDelta::Lines {
                                    x: 0.0,
                                    y: match name {
                                        "donusler" => -14.0,
                                        "terminaller" | "yapilandirma" => -26.0,
                                        "profiller" => -34.0,
                                        _ => -40.0,
                                    },
                                },
                            }),
                        ],
                    );
                    snapshot.settle(&mut app, App::view, &mut update);
                }
                let file = out.join(format!("aglar-{name}-{width}x{height}{suffix}.png"));
                snapshot
                    .render(app.view(), &app.theme())
                    .save(&file)
                    .expect("writes the picture");
                println!("{}", file.display());
            }
        }
    }
}

/// A pipe network whose subscribers are tied to their pipes by the project's relation, with a service pipe that ends
/// 0.5 m short of its branch (docs/adr/0525): a main from the depot east, valves at 100 and 220 m, a branch north at
/// 150 m with three subscribers, two on the main past the second valve.
pub(crate) fn subscribers_app() -> App {
    let (mut app, _) = App::boot(None);
    let (e, n) = (487_000.0, 4_420_000.0);
    let layer = |id: &str, name: &str, color: &str, fields: serde_json::Value| {
        json!({ "id": id, "name": name, "type": "layer", "visible": true, "locked": false, "expanded": true,
            "style": { "color": color, "lineType": "continuous", "lineWeight": 0.5 }, "children": [], "fields": fields })
    };
    let line = |id: u32, a: [f64; 2], b: [f64; 2]| {
        json!({ "kind": "line", "id": id, "layerId": "su", "attrs": {},
            "a": { "x": e + a[0], "y": n + a[1] }, "b": { "x": e + b[0], "y": n + b[1] } })
    };
    let point = |id: u32, layer: &str, p: [f64; 2], ad: &str| json!({ "kind": "point", "id": id, "layerId": layer, "attrs": { "Ad": ad }, "p": { "x": e + p[0], "y": n + p[1] } });
    let net = json!({ "id": "su", "name": "İçme suyu", "kind": "utility", "edges": [{ "layer": "su" }],
        "junctions": [{ "layer": "vana", "role": "valve" }, { "layer": "depo", "role": "source" }],
        "connect": "ends", "tolerance": 0.01, "direction": { "kind": "digitized" } });
    let doc = json!({
        "format": "kentos.document", "version": 1, "name": "Aboneler",
        "settings": { "srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
            "plotScale": 1000, "workspace": "gis", "drawingFont": "barlow", "networks": [net] },
        "origin": { "x": e, "y": n },
        "layers": [
            layer("su", "Su hattı", "#3E63DD", json!([{ "name": "Ad", "kind": "text" }])),
            layer("vana", "Vana", "#E5484D", json!([{ "name": "Ad", "kind": "text" }])),
            layer("depo", "Depo", "#30A46C", json!([{ "name": "Ad", "kind": "text" }])),
            layer("abone", "Abone", "#F76B15", json!([{ "name": "Ad", "kind": "text" }, { "name": "hat", "kind": "text" }])),
        ],
        "activeLayer": "su",
        "entities": [
            line(1, [0.0, 0.0], [100.0, 0.0]),
            line(2, [100.0, 0.0], [150.0, 0.0]),
            line(3, [150.0, 0.0], [220.0, 0.0]),
            line(4, [220.0, 0.0], [320.0, 0.0]),
            line(5, [150.0, 0.0], [150.0, 100.0]),
            line(6, [150.0, 100.5], [150.0, 150.0]),
            point(7, "depo", [0.0, 0.0], "Depo"),
            point(8, "vana", [100.0, 0.0], "V-1"),
            point(9, "vana", [220.0, 0.0], "V-2"),
            point(10, "abone", [160.0, 30.0], "A-1"),
            point(11, "abone", [140.0, 55.0], "A-2"),
            point(12, "abone", [160.0, 85.0], "A-3"),
            point(13, "abone", [260.0, 12.0], "A-4"),
            point(14, "abone", [300.0, -12.0], "A-5"),
        ],
        "styles": { "items": [], "categories": [] },
    });
    let first = Document::new(
        DocumentSnapshotV1::from_json(&doc.to_string()).expect("reads"),
        None,
    )
    .expect("opens");
    // The subscribers' field holds their pipe's persistent id; the relation ties them.
    let mut v2 = first.model.to_snapshot_v2();
    let uid = |slot: usize| kentos_domain::Uuid::from_bytes(v2.uids[slot - 1].0).to_string();
    let ties = [(10, 5), (11, 5), (12, 5), (13, 4), (14, 4)];
    let ids: Vec<(usize, String)> = ties.iter().map(|&(s, p)| (s, uid(p))).collect();
    for (s, pipe) in ids {
        v2.entities[s - 1]
            .base_mut()
            .attrs
            .insert("hat".into(), pipe);
    }
    v2.settings.relations = vec![kentos_commands::ProjectRelation {
        id: "aboneler".into(),
        name: "Aboneler".into(),
        parent: "su".into(),
        child: "abone".into(),
        field: "hat".into(),
        on_delete: kentos_commands::RelationDelete::default(),
    }];
    let doc = Document::from_v2(v2, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    send(&mut app, Message::Run("view.zoomExtents"));
    app
}

/// A click at a scene point, its tasks (the network thread's answers) driven to their end.
pub(crate) fn click_at(app: &mut App, p: [f64; 2]) {
    use crate::viewport::Event as View;
    let [x, y] = app
        .viewport
        .camera
        .world_to_screen(kentos_interaction::Vec2::new(
            487_000.0 + p[0],
            4_420_000.0 + p[1],
        ));
    let at = iced::Point::new(x as f32, y as f32);
    for e in [View::Moved(at), View::Pressed(at), View::Released(at)] {
        send(app, Message::Viewport(e));
    }
}

/// Yalıtım on the branch: the valves V-1 and V-2 close; the three subscribers on the branch and the two past V-2 are
/// cut off and said by the relation's name; the service pipe that ends short is a connection not made: said, and the
/// trace is not one to rely on (docs/adr/0525).
#[test]
fn a_trace_names_the_subscribers_it_cuts_off_and_the_connection_not_made() {
    let mut app = subscribers_app();
    app.memory.network_trace = kentos_geometry_core::ops::network::TraceKind::Isolation;
    send(&mut app, Message::Run("tool.netTrace"));
    click_at(&mut app, [150.0, 50.0]);
    let prompt = app.session.prompt().text();
    assert!(
        prompt.contains("V-1") || prompt.contains("2 vana kapatılır"),
        "{prompt}"
    );
    assert!(prompt.contains("5 bağlı kayıt (Aboneler)"), "{prompt}");
    assert!(
        prompt.contains("doğrulanmamış 1 bağlantıdan geçti"),
        "{prompt}"
    );
    let task = app.submit_line("");
    drive(&mut app, task);
    let said = crate::files_testing::last_said(&app);
    assert!(
        said.contains("Denetle'nin bulduğu bağlantı sorunlarından"),
        "{said}"
    );
    // The selection: the pipes, the valves, the branch's three subscribers and the two cut off past V-2.
    let names: Vec<String> = app
        .selection
        .ids()
        .iter()
        .filter_map(|s| app.document.as_ref()?.model.get(*s))
        .filter(|e| e.base().layer_id == "abone")
        .filter_map(|e| e.base().attrs.get("Ad").cloned())
        .collect();
    assert_eq!(names, ["A-1", "A-2", "A-3", "A-4", "A-5"]);
}
