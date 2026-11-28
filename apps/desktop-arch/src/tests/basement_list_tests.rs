//! 5/8'in bodrum listesi (docs/adr/8148; PAİY 5/8, 4/1-o; Otopark Yönetmeliği 4/1-ç and Ek-1; Sığınak Yönetmeliği 7/a,
//! 8/a) on emsal_limits_tests' two-storey plan: each 5 × 6 m bay's regulation gross 30 m²; on the ground flat 1, the
//! stair hall (the common area's) and flat 2; in the basement the bays each test names. The parcel 400 m², emsal 1.50:
//! the 30 % is 180 m², never reached.
//!
//! Worked by hand from 5/8's list ("binaların bodrum katlarında yapılan; … bu hesaba dâhil edilmeksizin emsal
//! haricidir"):
//! - d: the basement wholly under the ground (−3.00, its top at ±0.00): the common area's car park, stair hall and
//!   boiler room, 90 m², all apart from the 30 %. Flat 1's own car park is no common area's: a's.
//! - a: the basement half out of the ground (−2.00, the ground storey raised on it): no d. The two flats of 30 m² need
//!   a third of a place each (Ek-1, under 80 m²), one place together: 20 m² of mandatory car parks, twice 40 m². The
//!   30 m² car park apart; two car parks of 60 m², 40 m² of them. The stair hall and the boiler room (b: no least area
//!   computed) within.
//! - ç and e: the common masjid's 30 m² apart (a building with dwellings: up to 150 m²), the common children's play
//!   room's 30 m² (up to 100 m²); the shelter counted in whole: Sığınak Yönetmeliği 7/a asks none of a dwelling
//!   building of fewer than 10 units, so 22/1-g lets none out (docs/adr/8149).
//! - b: ten flats (eight with no room yet, two people each, 8/a): 20 people, 20 m² net. 22/1-g lets out the shelter's
//!   gross by its net, 20 × 30 / 25.7125 = 23.33495… m² (6.66504… m² in); b takes it all apart from the 30 %.
//!
//! ```text
//! RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib basement_list_tests::screens -- --ignored --nocapture
//! ```

use kentos_domain::zoning::BasementBent as B;
use serde_json::json;

use crate::app::{App, Message};
use crate::emsal_limits_tests::{basement_alone, building, command, near, report, row};

/// Each bent's (bent, rooms, area, allowed, apart), to 0.0001 m².
fn bents(app: &App) -> Vec<(B, usize, f64, f64, f64)> {
    let mm = |x: f64| (x * 1e4).round() / 1e4;
    report(app)
        .basement
        .iter()
        .map(|b| {
            (
                b.bent,
                b.rooms.len(),
                mm(b.area),
                mm(b.allowed),
                mm(b.apart),
            )
        })
        .collect()
}

/// Katlar alanı's counts: in, out within the 30 %, out apart from it (m², to 0.0001).
fn counted(app: &App) -> (f64, f64, f64) {
    let r = report(app);
    let mm = |x: f64| (x * 1e4).round() / 1e4;
    (mm(r.included), mm(r.exempt), mm(r.outside))
}

#[test]
fn a_basement_under_the_ground_keeps_its_common_rooms_apart_from_the_30_percent() {
    let app = building(
        [("Otopark", "o"), ("Merdiven", "o"), ("Kazan dairesi", "o")],
        -3.0,
    );
    assert_eq!(counted(&app), (60.0, 30.0, 90.0));
    assert!(near(report(&app).floors, 60.0));
    assert_eq!(bents(&app), [(B::Buried, 3, 90.0, 90.0, 90.0)]);
    assert_eq!(
        row(&app, "Emsal harici, %30 dışında").as_deref(),
        Some("90.00")
    );
    assert_eq!(
        row(&app, "— gömülü bodrumun ortak alanları").as_deref(),
        Some("90.00")
    );
    // The basement's emsal zone counted out names the part.
    let doc = &app.document.as_ref().expect("a drawing").model;
    let (m, _, _) = kentos_native_application::arch_ifc::model(doc);
    let named: Vec<&str> = m
        .zones
        .iter()
        .filter_map(|z| z.description.as_deref())
        .filter(|d| d.contains("bodrumda"))
        .collect();
    assert_eq!(named.len(), 1, "{:?}", m.zones);
    assert!(
        named[0].contains("5/8: bodrumda 90.00 m² emsal harici, %30 dışında"),
        "{}",
        named[0]
    );
    // Flat 1's own car park is no common area's: a's (flat 1 now 60 m², still a third of a place: 40 m² apart).
    let own = building(
        [("Otopark", "d1"), ("Merdiven", "o"), ("Kazan dairesi", "o")],
        -3.0,
    );
    assert_eq!(
        bents(&own),
        [
            (B::Buried, 2, 60.0, 60.0, 60.0),
            (B::Parking, 1, 30.0, 40.0, 30.0)
        ]
    );
    assert_eq!(counted(&own), (60.0, 30.0, 90.0));
}

#[test]
fn a_basement_half_out_of_the_ground_keeps_its_car_park_apart_up_to_twice_the_mandatory_area() {
    let app = building(
        [("Otopark", "o"), ("Merdiven", "o"), ("Kazan dairesi", "o")],
        -2.0,
    );
    assert_eq!(bents(&app), [(B::Parking, 1, 30.0, 40.0, 30.0)]);
    assert_eq!(counted(&app), (60.0, 90.0, 30.0));
    assert_eq!(
        row(&app, "— bodrumda otopark, zorunlu alanın 2 katına kadar").as_deref(),
        Some("30.00")
    );
    // Two car parks, 60 m²: 40 m² apart, 20 m² each, the rest within the 30 %.
    let two = building(
        [("Otopark", "o"), ("Otopark", "o"), ("Kazan dairesi", "o")],
        -2.0,
    );
    assert_eq!(bents(&two), [(B::Parking, 2, 60.0, 40.0, 40.0)]);
    assert_eq!(counted(&two), (60.0, 80.0, 40.0));
    let parts: Vec<f64> = report(&two)
        .rooms
        .iter()
        .filter(|r| r.apart > 0.0)
        .map(|r| r.apart)
        .collect();
    assert!(
        parts.len() == 2 && parts.iter().all(|a| near(*a, 20.0)),
        "{parts:?}"
    );
}

#[test]
fn the_masjid_and_the_children_s_room_are_apart_and_a_shelter_none_asks_counts_in() {
    let app = building(
        [("Mescit", "o"), ("Çocuk oyun alanı", "o"), ("Sığınak", "o")],
        -2.0,
    );
    assert_eq!(
        bents(&app),
        [
            (B::Masjid, 1, 30.0, 150.0, 30.0),
            (B::Children, 1, 30.0, 100.0, 30.0)
        ]
    );
    assert_eq!(counted(&app), (90.0, 30.0, 60.0));
    assert!(near(report(&app).shelter_over, 30.0));
    assert_eq!(
        row(&app, "Sığınağın gerekli alanı aşan kısmı").as_deref(),
        Some("30.00")
    );
    assert_eq!(
        row(&app, "— bodrumda mescit, 150 m²'ye kadar").as_deref(),
        Some("30.00")
    );
    assert_eq!(
        row(
            &app,
            "— bodrumda çocuk oyun ve bakım alanları, 100 m²'ye kadar"
        )
        .as_deref(),
        Some("30.00")
    );
    assert_eq!(row(&app, "— bodrumda sığınak"), None);
}

/// The shelter, the basement's stair hall and the boiler room in the basement half out of the ground, and eight more
/// flats with no room tied to them yet: ten dwellings, Sığınak Yönetmeliği 7/a no longer spares the building.
fn ten_flats() -> App {
    let mut app = building(
        [("Sığınak", "o"), ("Merdiven", "o"), ("Kazan dairesi", "o")],
        -2.0,
    );
    let mut units = vec![
        json!({"id": "d1", "number": "1", "nature": "Daire"}),
        json!({"id": "d2", "number": "2", "nature": "Daire"}),
        json!({"id": "e1", "number": "E1", "kind": "annex", "annexOf": "d1", "nature": "Depo"}),
        json!({"id": "o", "number": "O", "name": "Ortak alan", "kind": "common"}),
        json!({"id": "k", "number": "K", "name": "Kapıcı dairesi", "kind": "common"}),
    ];
    units.extend(
        (3..=10)
            .map(|n| json!({"id": format!("d{n}"), "number": n.to_string(), "nature": "Daire"})),
    );
    command(
        &mut app,
        "cad.program.edit",
        json!({"program": {"units": units}}),
    );
    app
}

#[test]
fn the_shelter_s_least_area_is_apart() {
    let app = ten_flats();
    let shelter = app
        .document
        .as_ref()
        .expect("a drawing")
        .model
        .shelter_report();
    assert_eq!((shelter.persons, shelter.exempt), (20, None));
    // 22/1-g: the needed 20 m² net in the shelter's gross by its net (its end bay: 25.7125 m² net of 30 m²).
    let kept = 20.0 * 30.0 / 25.7125;
    assert!(
        near(report(&app).shelter_over, 30.0 - kept),
        "{:?}",
        report(&app).shelter_over
    );
    assert_eq!(bents(&app), [(B::Shelter, 1, 23.335, 23.335, 23.335)]);
    assert_eq!(counted(&app), (66.665, 90.0, 23.335));
    assert_eq!(
        row(&app, "Sığınağın gerekli alanı aşan kısmı").as_deref(),
        Some("6.67")
    );
    assert_eq!(
        row(&app, "— bodrumda sığınak, gerekli alanı kadar").as_deref(),
        Some("23.33")
    );
}

/// Emsal ve TAKS tablosu beside the ten flats' shelter (22/1-g: the part past the needed area in, the rest apart from
/// the 30 % under 5/8-b), light and dark; `.run/gui-evidence/siginak-8149/`.
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn shelter_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../.run/gui-evidence/siginak-8149");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [
        ("light", Size::new(1440.0, 900.0), "acik"),
        ("dark", Size::new(1280.0, 800.0), "koyu"),
    ] {
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        let mut app = ten_flats();
        basement_alone(&mut app);
        let _ = app
            .settings
            .choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let _ = app.update(Message::Run("arch.zoningTable"));
        let _ = app.submit_line("17,6");
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        snapshot.settle(&mut app, App::view, &mut update);
        app.selection.set(Vec::new());
        let _ = app.update(Message::Run("view.zoomExtents"));
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("1-siginak-emsal-tablosu-{name}.png"));
        snapshot
            .render(app.view(), &app.theme())
            .save(&file)
            .expect("writes the picture");
        println!("{}", file.display());
    }
}

/// Emsal ve TAKS tablosu beside the basement under the ground (its common rooms apart) and beside the basement half out
/// of the ground (two car parks, 40 m² apart), light and dark; `.run/gui-evidence/bodrum-listesi-8148/`.
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../.run/gui-evidence/bodrum-listesi-8148");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [
        ("light", Size::new(1440.0, 900.0), "acik"),
        ("dark", Size::new(1280.0, 800.0), "koyu"),
    ] {
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        for (file, basement, floor) in [
            (
                "1-gomulu-bodrum",
                [("Otopark", "o"), ("Merdiven", "o"), ("Kazan dairesi", "o")],
                -3.0,
            ),
            (
                "2-yari-gomulu-otopark",
                [("Otopark", "o"), ("Otopark", "o"), ("Kazan dairesi", "o")],
                -2.0,
            ),
        ] {
            let mut app = building(basement, floor);
            basement_alone(&mut app);
            let _ = app
                .settings
                .choose(&[("appearance.theme", serde_json::Value::from(theme))]);
            app.apply_settings();
            let _ = app.update(Message::WindowResized(size));
            let _ = app.update(Message::Run("arch.zoningTable"));
            let _ = app.submit_line("17,6");
            let mut snapshot = Snapshot::new(size).expect("a renderer");
            snapshot.settle(&mut app, App::view, &mut update);
            app.selection.set(Vec::new());
            let _ = app.update(Message::Run("view.zoomExtents"));
            snapshot.settle(&mut app, App::view, &mut update);
            let file = out.join(format!("{file}-{name}.png"));
            snapshot
                .render(app.view(), &app.theme())
                .save(&file)
                .expect("writes the picture");
            println!("{}", file.display());
        }
    }
}
