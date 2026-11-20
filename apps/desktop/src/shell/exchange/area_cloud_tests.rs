//! GIS-036 (docs/adr/1093): Nokta bulutu alan istatistiği and Nokta bulutu profili name the drawing's one cloud and take
//! it, offer Tüm bulutlar or one of several, say a drawing with none; the area or the line is picked on the drawing with
//! Sahneden seç and the window comes back with its values; a flat area is refused; the answer is written in the
//! project's units (m², points per m², m).

use kentos_commands::{CommandResult, PointCloudsRegion};
use kentos_native_application::{ExecutionContext, pointcloud_areas};
use serde_json::{Value, json};

use super::area_queries::{Event as A, Outcome};
use super::area_tests::{opened, pts, select, shapes};
use super::tests::{run, send};
use super::{Event, Window};
use crate::app::{App, Dialog, Message};
use crate::files_testing::app_with_drawing;

/// The region cases' drawing with only its first cloud (las12-f0.las) and the shapes.
fn one_cloud(app: &mut App, extra: Vec<Value>) {
    opened(app, "cad.pointclouds.region.json", extra);
    let doc = &mut app.document.as_mut().expect("open").model;
    let others: Vec<kentos_domain::Slot> = doc
        .entities_with_uids()
        .filter(|(_, e)| {
            matches!(e, kentos_commands::Entity::PointCloud(_)) && u64::from(e.base().id) != 1
        })
        .filter_map(|(u, _)| doc.slot_of(u))
        .collect();
    doc.remove(&others);
}

fn region(app: &App) -> &super::area_queries::RegionState {
    match &app.exchange {
        Some(Window::CloudRegion(s)) => s,
        _ => panic!("the area window"),
    }
}

#[test]
fn one_cloud_is_named_and_taken_several_are_chosen_from() {
    let mut app = app_with_drawing();
    one_cloud(&mut app, shapes("bulut"));
    select(&mut app, 100);
    run(&mut app, "pointcloud.region");
    let s = region(&app);
    assert_eq!(s.clouds.all.len(), 1);
    assert!(
        s.clouds.all[0].1.starts_with("“las12-f0.las” · "),
        "{:?}",
        s.clouds.all
    );
    // Several: Tüm bulutlar first, one of them reads that one only, as the command does with its id.
    let mut app = app_with_drawing();
    opened(&mut app, "cad.pointclouds.region.json", shapes("bulut"));
    select(&mut app, 100);
    run(&mut app, "pointcloud.region");
    assert!(region(&app).clouds.all.len() > 1);
    assert_eq!(region(&app).clouds.chosen, 0, "Tüm bulutlar");
    send(&mut app, Event::Area(A::Cloud(1)));
    send(&mut app, Event::Area(A::Run));
    let s = region(&app);
    let Some(Outcome::Region(Ok((got, _)))) = &s.result else {
        panic!("answered: {:?}", s.result);
    };
    assert_eq!(got.clouds.len(), 1, "one cloud read");
    let input = PointCloudsRegion {
        ring: s.shape.as_ref().expect("a shape").ring.clone(),
        holes: Vec::new(),
        uids: vec![s.clouds.all[0].0.clone()],
        drawing: None,
    };
    let mut model = app.document.as_ref().expect("open").model.clone();
    let CommandResult::Completed { output, .. } =
        pointcloud_areas::region::plan(&ExecutionContext::new(&mut model), &input)
    else {
        panic!("the command answers");
    };
    assert_eq!(*got, output);
}

#[test]
fn a_drawing_without_a_cloud_says_so_and_runs_nothing() {
    let mut app = app_with_drawing();
    opened(&mut app, "cad.pointclouds.region.json", shapes("bulut"));
    let doc = &mut app.document.as_mut().expect("open").model;
    let clouds: Vec<kentos_domain::Slot> = doc
        .entities_with_uids()
        .filter(|(_, e)| matches!(e, kentos_commands::Entity::PointCloud(_)))
        .filter_map(|(u, _)| doc.slot_of(u))
        .collect();
    doc.remove(&clouds);
    select(&mut app, 100);
    run(&mut app, "pointcloud.region");
    assert!(region(&app).clouds.all.is_empty());
    send(&mut app, Event::Area(A::Run));
    assert!(
        region(&app).result.is_none(),
        "nothing runs without a cloud"
    );
}

/// Sahneden seç: the window steps aside, the area is picked, Enter brings it back with the same values; the profile's
/// width typed before is kept.
#[test]
fn sahneden_sec_picks_the_area_and_the_window_comes_back_as_it_was() {
    let mut app = app_with_drawing();
    one_cloud(&mut app, shapes("bulut"));
    app.selection.clear();
    run(&mut app, "pointcloud.region");
    assert!(
        region(&app)
            .shape
            .as_ref()
            .is_err_and(|w| w.starts_with("Seçimde alan yok: Sahneden seç"))
    );
    send(&mut app, Event::Area(A::Pick));
    assert_eq!(app.dialog, None, "the window steps aside");
    select(&mut app, 100);
    assert!(app.window_picked(true));
    assert_eq!(app.dialog, Some(Dialog::Exchange));
    assert!(region(&app).shape.is_ok(), "the picked area is read");
    // The profile: its width kept across the pick.
    app.selection.clear();
    run(&mut app, "pointcloud.profile");
    send(&mut app, Event::Area(A::Width("6".into())));
    send(&mut app, Event::Area(A::Pick));
    select(&mut app, 101);
    assert!(app.window_picked(true));
    let Some(Window::CloudProfile(s)) = &app.exchange else {
        panic!("the profile window");
    };
    assert!(s.path.is_ok());
    assert_eq!(s.width, "6");
    let _ = Message::Exchange;
}

#[test]
fn a_flat_area_is_refused() {
    let mut app = app_with_drawing();
    let flat = json!({ "kind": "polyline", "id": 103, "layerId": "bulut", "attrs": {}, "closed": true,
        "pts": pts(&[(500050.0, 4400050.0), (500150.0, 4400050.0), (500250.0, 4400050.0)]) });
    one_cloud(&mut app, vec![flat]);
    select(&mut app, 103);
    run(&mut app, "pointcloud.region");
    assert!(
        region(&app)
            .shape
            .as_ref()
            .is_err_and(|w| w.starts_with("Seçilen alan geçersiz")),
        "{:?}",
        region(&app).shape.as_ref().err()
    );
}

/// The answer in the project's units: the area in m², the density per m², the corridor's length in m.
#[test]
fn the_answers_carry_their_units() {
    let mut app = app_with_drawing();
    one_cloud(&mut app, shapes("bulut"));
    select(&mut app, 100);
    run(&mut app, "pointcloud.region");
    send(&mut app, Event::Area(A::Run));
    let first = copied(&mut app);
    assert!(first.starts_with("Alan 30000"), "{first}");
    assert!(first.contains(" m²;"), "{first}");
    assert!(!first.contains("birim"), "{first}");
    // 71 points in 30 000 m²: three significant digits, not 0.002.
    assert!(first.contains("(0.00237 nokta/m²)"), "{first}");
    send(&mut app, Event::Close);
    select(&mut app, 101);
    run(&mut app, "pointcloud.profile");
    send(&mut app, Event::Area(A::Run));
    let first = copied(&mut app);
    assert!(first.starts_with("Yol 400"), "{first}");
    assert!(
        first.split(';').next().is_some_and(|t| t.ends_with(" m")),
        "{first}"
    );
}

/// The answer's first line as Panoya kopyala takes it.
fn copied(app: &mut App) -> String {
    let (r, f) = match &app.exchange {
        Some(Window::CloudRegion(s)) => (s.result.clone(), s.format),
        Some(Window::CloudProfile(s)) => (s.result.clone(), s.format),
        _ => panic!("a cloud window"),
    };
    super::area_queries::copy_text(&r.expect("answered"), &f)
        .lines()
        .next()
        .unwrap_or("")
        .to_owned()
}

/// The area window over the one cloud, answered, and the profile's empty choice: pictures `gis-036-*`.
#[test]
fn the_windows_on_the_screen() {
    let mut app = app_with_drawing();
    one_cloud(&mut app, shapes("bulut"));
    select(&mut app, 100);
    run(&mut app, "pointcloud.region");
    send(&mut app, Event::Area(A::Run));
    let mut shot = crate::files_testing::pictured(iced::Size::new(1440., 900.));
    shot.settle(&mut app, App::view, &mut |app: &mut App, m| {
        drop(app.update(m))
    });
    let name = region(&app).clouds.all[0].1.clone();
    assert!(
        crate::files_testing::find_text(&mut shot, &app, &name).is_some(),
        "the cloud is named"
    );
    assert!(crate::files_testing::find_text(&mut shot, &app, "Sahneden seç").is_some());
    crate::files_testing::evidence(&mut shot, &app, "cbs-kullanim", "gis-036-alan");
    send(&mut app, Event::Close);
    app.selection.clear();
    run(&mut app, "pointcloud.profile");
    let mut shot = crate::files_testing::pictured(iced::Size::new(1440., 900.));
    shot.settle(&mut app, App::view, &mut |app: &mut App, m| {
        drop(app.update(m))
    });
    crate::files_testing::evidence(&mut shot, &app, "cbs-kullanim", "gis-036-profil-bos");
}
