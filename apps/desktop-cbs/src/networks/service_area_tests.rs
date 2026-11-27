//! Hizmet alanı's breaks and what it writes (the CBS review's 17th item, docs/adr/9333), on the review's roads: A
//! (500000, 4320000), C (500030, 4320000), B (500030, 4320040); AC 30 m, CB 40 m, AB 50 m. Before anything is shown
//! the breaks, Kenar payı and the lines' switch stand in the tool's settings and first on its line. At 35 m from A the
//! network reaches AC whole, the first 5 m of CB and the first 35 m of AB: those go to “Ulaşılan yollar”. The area,
//! those lines' 50 m surroundings, goes to “Yaklaşık hizmet alanı” and holds B, which the network does not reach.

use super::prerequisite_tests::{click, prompt, roads, send, settings};
use crate::app::{App, Message};
use crate::files_testing::drive;

pub(super) const A: (f64, f64) = (500_000.0, 4_320_000.0);
pub(super) const B: (f64, f64) = (500_030.0, 4_320_040.0);

/// Hizmet alanı on the roads' network (made of the layer, Katmandan ağ oluştur).
pub(super) fn service_area() -> App {
    let mut app = roads();
    send(&mut app, Message::Run("tool.netServiceArea"));
    send(&mut app, Message::PromptOption("O"));
    app
}

pub(super) fn typed(app: &mut App, text: &str) {
    let task = app.submit_line(text);
    drive(app, task);
}

/// The objects of the layer named `name`, with their shapes.
fn objects(app: &App, name: &str) -> Vec<(kentos_geometry_core::entity::Shape, std::collections::BTreeMap<String, String>)> {
    let doc = &app.document.as_ref().expect("open").model;
    let Some(layer) = doc.layers().leaves().into_iter().find(|n| n.name == name) else {
        return Vec::new();
    };
    doc.by_layer(&layer.id)
        .map(|e| (kentos_native_application::geometry::shape(e), e.base().attrs.clone()))
        .collect()
}

/// 35 m from A, the lines written too.
pub(super) fn at_35_m(app: &mut App) {
    send(app, Message::PromptOption("R"));
    typed(app, "35");
    send(app, Message::PromptOption("Ç"));
    click(app, A.0, A.1);
}

#[test]
fn the_breaks_and_the_margin_stand_first_and_35_m_reach_what_the_network_reaches() {
    let mut app = service_area();
    let line = prompt(&app);
    assert!(
        line.starts_with(
            "Hizmet alanı: tesise tıklayın ya da Y,X yazın [Aralıklar (R): 500 1000 1500 m / Kenar payı (K): \
             50.000 m / Ağ (A): qa_yollar ağı / Maliyet (M): Uzunluk / Çizgiler (Ç): kapalı / Engel (E) / "
        ),
        "{line}"
    );
    let f = app.format();
    let rows = settings(&app);
    let shown: Vec<(&str, &str)> = rows.iter().map(|(l, v)| (l.as_str(), v.as_str())).collect();
    assert_eq!(
        shown[..8],
        [
            ("Ağ", "qa_yollar ağı"),
            ("Maliyet", "Uzunluk, metre"),
            ("Tolerans", f.length(0.01).as_str()),
            ("Bağlanma", "uçlarda"),
            ("Aralıklar (m)", "500 1000 1500"),
            ("Kenar payı", f.length(50.0).as_str()),
            ("Yaklaşık alan", "Açık"),
            ("Ulaşılan yollar", "Kapalı"),
        ]
    );
    assert_eq!(
        rows[8].1,
        format!(
            "Çıktılar: “Yaklaşık hizmet alanı” (Başlangıç (m), Bitiş (m)): ulaşılan yolların {} çevresi, yaklaşık.",
            f.length(50.0)
        )
    );
    at_35_m(&mut app);
    assert_eq!(settings(&app)[4].1, "35");
    assert_eq!(settings(&app)[7].1, "Açık");
    assert!(prompt(&app).contains("1 tesis; 3 çizgi, 1 alan"), "{}", prompt(&app));
    typed(&mut app, "");
    let log: Vec<String> = app.log.lines().map(|l| l.text.clone()).collect();
    assert!(
        log.iter().any(|l| *l
            == format!(
                "Hizmet alanı: 1 yaklaşık alan ve 3 ulaşılan yol parçası yazıldı; alan, yolların {} çevresidir.",
                f.length(50.0)
            )),
        "{:?}",
        &log[log.len().saturating_sub(4)..]
    );
    // The reach: AC whole, the first 5 m of CB, the first 35 m of AB.
    let lines = objects(&app, "Ulaşılan yollar");
    let mut lengths: Vec<f64> = lines
        .iter()
        .map(|(s, _)| kentos_geometry_core::entity::entity_length(s).expect("a length"))
        .collect();
    lengths.sort_by(f64::total_cmp);
    assert_eq!(lengths.len(), 3, "{lengths:?}");
    for (got, want) in lengths.iter().zip([5.0, 30.0, 35.0]) {
        assert!((got - want).abs() < 1e-6, "{lengths:?}");
    }
    assert!(lines.iter().all(|(_, a)| a.get("Aralık (m)").map(String::as_str) == Some("0–35")), "{lines:?}");
    // The area: the lines' 50 m surroundings, so B (35 m from the end of CB's part) is in it though not reached.
    let areas = objects(&app, "Yaklaşık hizmet alanı");
    let [(kentos_geometry_core::entity::Shape::Polygon { pts, .. }, attrs)] = &areas[..] else {
        panic!("one area: {areas:?}");
    };
    assert!(
        kentos_geometry_core::geometry::point_in_polygon(kentos_interaction::Vec2::new(B.0, B.1), pts),
        "B in the area"
    );
    assert_eq!(attrs.get("Bitiş (m)").map(String::as_str), Some(f.length(35.0).trim_end_matches(" m")));
}

/// The view drawn back so that the area's 50 m surroundings show whole.
pub(super) fn drawn_back(app: &mut App) {
    for _ in 0..3 {
        send(app, Message::Run("view.zoomOut"));
    }
}

/// The pictures (`.run/gui-evidence/kullanim-9333/`): Hizmet alanı before anything is shown, its breaks, margin and the
/// lines' switch in Öznitelikler and first on the line; 35 m from A, the reached lines inside the area; both written.
#[test]
#[ignore = "pictures: cargo test -p kentos-desktop-cbs service_area_tests::screens -- --ignored --nocapture"]
fn screens() {
    crate::drawing_fonts::load();
    crate::files_testing::usage_shots("9333", "01-hizmet-alani-baslangic", service_area);
    crate::files_testing::usage_shots("9333", "02-hizmet-alani-35m", || {
        let mut app = service_area();
        at_35_m(&mut app);
        drawn_back(&mut app);
        app
    });
    crate::files_testing::usage_shots("9333", "03-hizmet-alani-yazildi", || {
        let mut app = service_area();
        at_35_m(&mut app);
        typed(&mut app, "");
        drawn_back(&mut app);
        app
    });
}
