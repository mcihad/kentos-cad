//! Nokta bulutu alan istatistiği, Nokta bulutu profili and Mesh sor on the desktop (docs/adr/0552): each window takes
//! its shape from the selection (an area, a line, a point), reads off the window's thread and answers what the command
//! answers when a script runs it on the same drawing. Pictures: `bulut-alan-*`, `bulut-profil-*` and `mesh-sor-*`
//! (`screens`, ignored). The drawings are the shared cases' (fixtures/commands/v1/cad.pointclouds.region.json and
//! cad.meshes.query.json) with an area, a polyline and a point added, their files' paths made whole.

use std::path::PathBuf;

use kentos_commands::{
    Entity, MeshesProfile, MeshesQuery, MeshesRegion, PointCloudsProfile, PointCloudsRegion, Vec2,
};
use kentos_native_application::{ExecutionContext, mesh_areas, meshes, pointcloud_areas};
use serde_json::{Value, json};

use super::area_queries::{Event as A, MeshMode, Outcome};
use super::tests::{run, send};
use super::{Event, Window};
use crate::app::{App, Message};
use crate::files_testing::app_with_drawing;

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures")
}

/// A shared case file's drawing with `extra` entities added, opened in the app.
fn opened(app: &mut App, file: &str, extra: Vec<Value>) {
    let text =
        std::fs::read_to_string(fixtures().join("commands/v1").join(file)).expect("the cases");
    let cases: Value = serde_json::from_str(&text).expect("JSON");
    let mut setup = cases["setup"].clone();
    for e in extra {
        setup["entities"].as_array_mut().expect("entities").push(e);
    }
    let whole = setup
        .to_string()
        .replace("$fixture:", &format!("{}/", fixtures().display()));
    let snapshot =
        kentos_commands::DocumentSnapshotV1::from_json(&whole).expect("the drawing reads");
    let doc = crate::document::Document::new(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
}

fn pts(v: &[(f64, f64)]) -> Vec<Value> {
    v.iter().map(|(x, y)| json!({ "x": x, "y": y })).collect()
}

/// An area, a path and a point over las12-f0.las's ground (the mesh drawing: a point at a mesh place), ids 100, 101, 102,
/// on `layer`.
fn shapes(layer: &str) -> Vec<Value> {
    vec![
        json!({ "kind": "polygon", "id": 100, "layerId": layer, "attrs": {},
                "pts": pts(&[(500050.0, 4400050.0), (500250.0, 4400050.0), (500250.0, 4400200.0), (500050.0, 4400200.0)]) }),
        json!({ "kind": "polyline", "id": 101, "layerId": layer, "attrs": {},
                "pts": pts(&[(500000.0, 4400150.0), (500400.0, 4400150.0)]) }),
        json!({ "kind": "point", "id": 102, "layerId": layer, "attrs": {}, "p": { "x": 500010.0, "y": 4400010.0 } }),
    ]
}

fn select(app: &mut App, id: u64) {
    let doc = app.document.as_ref().expect("open");
    let slot = doc
        .model
        .entities_with_uids()
        .find(|(_, e)| u64::from(e.base().id) == id)
        .and_then(|(u, _)| doc.model.slot_of(u))
        .expect("the entity");
    app.selection.set(vec![slot]);
}

#[test]
fn the_area_window_answers_what_the_command_answers() {
    let mut app = app_with_drawing();
    opened(&mut app, "cad.pointclouds.region.json", shapes("bulut"));
    select(&mut app, 100);
    run(&mut app, "pointcloud.region");
    send(&mut app, Event::Area(A::Run));
    let Some(Window::CloudRegion(s)) = &app.exchange else {
        panic!("the area window");
    };
    let Some(Outcome::Region(Ok((got, _)))) = &s.result else {
        panic!("answered: {:?}", s.result);
    };
    assert!(got.total.count > 0);
    let input = PointCloudsRegion {
        ring: s.shape.as_ref().expect("a shape").ring.clone(),
        holes: Vec::new(),
        uids: Vec::new(),
        drawing: None,
    };
    let mut model = app.document.as_ref().expect("open").model.clone();
    let direct = pointcloud_areas::region::plan(&ExecutionContext::new(&mut model), &input);
    let kentos_commands::CommandResult::Completed { output, .. } = direct else {
        panic!("the command answers");
    };
    assert_eq!(*got, output);
}

#[test]
fn without_an_area_in_the_selection_the_window_says_so() {
    let mut app = app_with_drawing();
    opened(&mut app, "cad.pointclouds.region.json", shapes("bulut"));
    select(&mut app, 101);
    run(&mut app, "pointcloud.region");
    let Some(Window::CloudRegion(s)) = &app.exchange else {
        panic!("the area window");
    };
    assert!(
        s.shape
            .as_ref()
            .err()
            .is_some_and(|w| w.starts_with("Seçimde alan yok"))
    );
    send(&mut app, Event::Area(A::Run));
    let Some(Window::CloudRegion(s)) = &app.exchange else {
        panic!("the area window");
    };
    assert!(s.result.is_none(), "nothing runs without an area");
    // Selecting one and pressing Seçimden al takes it.
    select(&mut app, 100);
    send(&mut app, Event::Area(A::Use));
    let Some(Window::CloudRegion(s)) = &app.exchange else {
        panic!("the area window");
    };
    assert!(s.shape.is_ok());
}

#[test]
fn the_profile_window_lists_the_stations_and_the_points() {
    let mut app = app_with_drawing();
    opened(&mut app, "cad.pointclouds.profile.json", shapes("bulut"));
    select(&mut app, 101);
    run(&mut app, "pointcloud.profile");
    send(&mut app, Event::Area(A::Width("40".into())));
    send(&mut app, Event::Area(A::Step("100".into())));
    send(&mut app, Event::Area(A::Toggle));
    send(&mut app, Event::Area(A::Run));
    let Some(Window::CloudProfile(s)) = &app.exchange else {
        panic!("the profile window");
    };
    let Some(Outcome::Profile(Ok((got, _)))) = &s.result else {
        panic!("answered: {:?}", s.result);
    };
    assert_eq!(got.bins.len(), 4);
    assert_eq!(got.length, 400.0);
    assert_eq!(got.points.len() as u64, got.count);
    let input = PointCloudsProfile {
        path: s.path.as_ref().expect("a path").0.clone(),
        width: 40.0,
        step: 100.0,
        points: true,
        uids: Vec::new(),
        drawing: None,
    };
    let mut model = app.document.as_ref().expect("open").model.clone();
    let direct = pointcloud_areas::profile::plan(&ExecutionContext::new(&mut model), &input);
    let kentos_commands::CommandResult::Completed { output, .. } = direct else {
        panic!("the command answers");
    };
    assert_eq!(*got, output);
}

#[test]
fn a_width_that_is_not_a_number_is_said() {
    let mut app = app_with_drawing();
    opened(&mut app, "cad.pointclouds.profile.json", shapes("bulut"));
    select(&mut app, 101);
    run(&mut app, "pointcloud.profile");
    send(&mut app, Event::Area(A::Width("geniş".into())));
    send(&mut app, Event::Area(A::Run));
    let Some(Window::CloudProfile(s)) = &app.exchange else {
        panic!("the profile window");
    };
    assert!(
        matches!(&s.result, Some(Outcome::Profile(Err(w))) if w.starts_with("Genişlik ve adım"))
    );
}

#[test]
fn the_mesh_window_finds_the_face_and_the_values_at_a_moment() {
    let mut app = app_with_drawing();
    opened(&mut app, "cad.meshes.query.json", Vec::new());
    run(&mut app, "mesh.query");
    send(&mut app, Event::Area(A::X("500010".into())));
    send(&mut app, Event::Area(A::Y("4420010".into())));
    send(&mut app, Event::Area(A::Time("2024-05-01 06:30".into())));
    send(&mut app, Event::Area(A::Run));
    let Some(Window::MeshQuery(s)) = &app.exchange else {
        panic!("the mesh window");
    };
    let Some(Outcome::Mesh(Ok((got, _)))) = &s.result else {
        panic!("answered: {:?}", s.result);
    };
    let face = got.face.as_ref().expect("a face");
    assert_eq!(face.face, 0);
    let depth = got
        .values
        .iter()
        .find(|v| v.variable == "depth")
        .expect("depth");
    assert_eq!(depth.step, Some(1));
    let input = MeshesQuery {
        at: Vec2 {
            x: 500010.0,
            y: 4420010.0,
        },
        uids: Vec::new(),
        time: Some(1_714_545_000_000.0),
        variables: Vec::new(),
        series: false,
        drawing: None,
    };
    let mut model = app.document.as_ref().expect("open").model.clone();
    let direct = meshes::query::plan(&ExecutionContext::new(&mut model), &input);
    let kentos_commands::CommandResult::Completed { output, .. } = direct else {
        panic!("the command answers");
    };
    assert_eq!(*got, output);
}

#[test]
fn a_moment_that_is_not_read_is_said_and_a_point_is_taken_from_the_selection() {
    let mut app = app_with_drawing();
    let mut extra = shapes("mesh");
    extra[2]["p"] = json!({ "x": 500010.0, "y": 4420010.0 });
    opened(&mut app, "cad.meshes.query.json", extra);
    select(&mut app, 102);
    run(&mut app, "mesh.query");
    let Some(Window::MeshQuery(s)) = &app.exchange else {
        panic!("the mesh window");
    };
    assert_eq!((s.x.as_str(), s.y.as_str()), ("500010", "4420010"));
    send(&mut app, Event::Area(A::Time("dün".into())));
    send(&mut app, Event::Area(A::Run));
    let Some(Window::MeshQuery(s)) = &app.exchange else {
        panic!("the mesh window");
    };
    assert!(matches!(&s.result, Some(Outcome::Mesh(Err(w))) if w.starts_with("An okunamadı")));
    assert!(
        app.document
            .as_ref()
            .expect("open")
            .model
            .entities_with_uids()
            .any(|(_, e)| matches!(e, Entity::Point(_)))
    );
}

/// An area and a path over layered.nc's two faces (docs/adr/0645 §7), ids 200 and 201.
fn mesh_shapes() -> Vec<Value> {
    vec![
        json!({ "kind": "polygon", "id": 200, "layerId": "mesh", "attrs": {},
                "pts": pts(&[(499999.0, 4419999.0), (500021.0, 4419999.0), (500021.0, 4420011.0), (499999.0, 4420011.0)]) }),
        json!({ "kind": "polyline", "id": 201, "layerId": "mesh", "attrs": {},
                "pts": pts(&[(499995.0, 4420005.0), (500025.0, 4420005.0)]) }),
    ]
}

#[test]
fn the_mesh_window_answers_an_area_and_a_path_as_the_commands_do() {
    let mut app = app_with_drawing();
    opened(&mut app, "cad.meshes.query.json", mesh_shapes());
    let uids: Vec<String> = {
        let doc = app.document.as_ref().expect("open");
        doc.model
            .entities_with_uids()
            .filter(|(_, e)| matches!(u64::from(e.base().id), 5 | 17 | 18))
            .map(|(u, _)| u.to_string())
            .collect()
    };
    assert_eq!(uids.len(), 3);
    select(&mut app, 200);
    run(&mut app, "mesh.query");
    let Some(Window::MeshQuery(s)) = &app.exchange else {
        panic!("the mesh window");
    };
    assert_eq!(s.mode, MeshMode::Area);
    send(&mut app, Event::Area(A::Run));
    let Some(Window::MeshQuery(s)) = &app.exchange else {
        panic!("the mesh window");
    };
    let Some(Outcome::MeshRegion(Ok((got, _)))) = &s.result else {
        panic!("answered: {:?}", s.result);
    };
    let ring = vec![
        Vec2 {
            x: 499999.0,
            y: 4419999.0,
        },
        Vec2 {
            x: 500021.0,
            y: 4419999.0,
        },
        Vec2 {
            x: 500021.0,
            y: 4420011.0,
        },
        Vec2 {
            x: 499999.0,
            y: 4420011.0,
        },
    ];
    let mut model = app.document.as_ref().expect("open").model.clone();
    let direct = mesh_areas::region::plan(
        &ExecutionContext::new(&mut model),
        &MeshesRegion {
            ring,
            ..MeshesRegion::default()
        },
    );
    let kentos_commands::CommandResult::Completed { output, .. } = direct else {
        panic!("the command answers");
    };
    assert_eq!(*got, output);
    // The classic file and its NetCDF-4 and Zarr copies say the same.
    let of = |u: &str| {
        got.meshes
            .iter()
            .find(|m| m.mesh == u)
            .cloned()
            .expect("the mesh")
    };
    let (a, b, c) = (of(&uids[0]), of(&uids[1]), of(&uids[2]));
    assert_eq!((a.count, a.datasets.clone()), (b.count, b.datasets.clone()));
    assert_eq!((a.count, a.datasets), (c.count, c.datasets));
    select(&mut app, 201);
    send(&mut app, Event::Area(A::Mode(MeshMode::Path)));
    send(&mut app, Event::Area(A::Use));
    send(&mut app, Event::Area(A::Step("5".into())));
    send(&mut app, Event::Area(A::Run));
    let Some(Window::MeshQuery(s)) = &app.exchange else {
        panic!("the mesh window");
    };
    let Some(Outcome::MeshProfile(Ok((got, _)))) = &s.result else {
        panic!("answered: {:?}", s.result);
    };
    let direct = mesh_areas::profile::plan(
        &ExecutionContext::new(&mut model),
        &MeshesProfile {
            path: vec![
                Vec2 {
                    x: 499995.0,
                    y: 4420005.0,
                },
                Vec2 {
                    x: 500025.0,
                    y: 4420005.0,
                },
            ],
            step: 5.0,
            ..MeshesProfile::default()
        },
    );
    let kentos_commands::CommandResult::Completed { output, .. } = direct else {
        panic!("the command answers");
    };
    assert_eq!(*got, output);
    assert!(got.meshes.iter().all(|m| m.stations.len() == 7));
}

/// The layers folded and Genel folded: the selected object's own rows in view.
fn rows_in_view(app: &mut App) {
    let upper =
        kentos_ui::widget::docking::Slot::Docked(kentos_ui::widget::docking::Side::Right, 0);
    let _ = app.update(Message::Dock(kentos_ui::widget::docking::Event::Collapsed(
        upper, true,
    )));
    let _ = app.update(Message::Properties(crate::properties::Event::Toggle(
        "general",
    )));
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
            let fresh = |file: &str| {
                let mut app = app_with_drawing();
                let _ = app
                    .settings
                    .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
                app.apply_settings();
                opened(
                    &mut app,
                    file,
                    shapes(if file.contains("meshes") {
                        "mesh"
                    } else {
                        "bulut"
                    }),
                );
                app
            };
            let mut app = fresh("cad.pointclouds.region.json");
            select(&mut app, 100);
            run(&mut app, "pointcloud.region");
            send(&mut app, Event::Area(A::Run));
            picture(&mut app, &format!("bulut-alan-{mode}-1-sonuc"));
            let mut app = fresh("cad.pointclouds.profile.json");
            select(&mut app, 101);
            run(&mut app, "pointcloud.profile");
            send(&mut app, Event::Area(A::Width("40".into())));
            send(&mut app, Event::Area(A::Step("100".into())));
            send(&mut app, Event::Area(A::Run));
            picture(&mut app, &format!("bulut-profil-{mode}-1-sonuc"));
            // Öznitelikler: a mesh raster (its edges, area, system, datasets, time steps) and a virtual cloud (its
            // files summed up, where they disagree).
            let mut app = fresh("cad.meshes.query.json");
            select(&mut app, 5);
            rows_in_view(&mut app);
            picture(&mut app, &format!("mesh-oznitelik-{mode}-1-mesh"));
            let mut app = fresh("cad.pointclouds.region.json");
            select(&mut app, 23);
            rows_in_view(&mut app);
            picture(&mut app, &format!("bulut-ozet-{mode}-1-oznitelik"));
            let mut app = fresh("cad.meshes.query.json");
            run(&mut app, "mesh.query");
            send(&mut app, Event::Area(A::X("500010".into())));
            send(&mut app, Event::Area(A::Y("4420010".into())));
            send(&mut app, Event::Area(A::Toggle));
            send(&mut app, Event::Area(A::Run));
            picture(&mut app, &format!("mesh-sor-{mode}-1-seri"));
            // Mesh sor's Alan and Hat over layered.nc and its NetCDF-4 and Zarr copies (docs/adr/0645 §7).
            let mesh_fresh = || {
                let mut app = app_with_drawing();
                let _ = app
                    .settings
                    .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
                app.apply_settings();
                opened(&mut app, "cad.meshes.query.json", mesh_shapes());
                app
            };
            let mut app = mesh_fresh();
            select(&mut app, 200);
            run(&mut app, "mesh.query");
            send(&mut app, Event::Area(A::Toggle));
            send(&mut app, Event::Area(A::Run));
            picture(&mut app, &format!("mesh-sor-{mode}-2-alan"));
            let mut app = mesh_fresh();
            select(&mut app, 201);
            run(&mut app, "mesh.query");
            send(&mut app, Event::Area(A::Step("5".into())));
            send(&mut app, Event::Area(A::Run));
            picture(&mut app, &format!("mesh-sor-{mode}-3-hat"));
            // Mesh ekle and Raster ekle with a NetCDF-4, a Zarr store and a GRIB file (docs/adr/0645 §5).
            for (name, file, command) in [
                ("1-netcdf4", "layered-nc4.nc", "mesh.add"),
                ("2-zarr", "layered.zarr", "mesh.add"),
                ("3-grib", "gfs.grib2", "raster.add"),
                ("4-netcdf4-izgara", "nc4-cf.nc", "raster.add"),
            ] {
                let mut app = mesh_fresh();
                app.picker = crate::app::Picker::File(fixtures().join("multidim/v2").join(file));
                let task = app.run(command);
                crate::files_testing::drive(&mut app, task);
                picture(&mut app, &format!("cok-boyutlu-{mode}-{name}"));
            }
        }
    }
}
