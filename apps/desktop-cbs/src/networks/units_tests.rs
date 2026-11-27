//! The networks' results with their units and Hizmet alanı's settings in the pane (the second round's T33, T35;
//! docs/adr/9355), on the review's roads: A (500000, 4320000), C (500030, 4320000), B (500030, 4320040); AC 30 m. The
//! network made of the layer has Süre at its fallback 50 km/sa.
//!
//! - En kısa yol A → C: 30 m in 2.16 seconds, said “Süre 2.16 sn” (it read “0.0 dk”); Rota's columns “Uzunluk (m)” 30
//!   and “Süre (sn)” 2.16.
//! - Hizmet alanı: the breaks, the margin and the two outputs written in the pane; only the reached ways written when
//!   the areas are off; the last output is not switched off; a wrong break said and kept as it was.

use std::collections::BTreeMap;

use super::prerequisite_tests::{click, prompt, roads, send, settings};
use super::service_area_tests::{A, service_area, typed};
use crate::app::{App, Message};

const C: (f64, f64) = (500_030.0, 4_320_000.0);

/// The attributes of the objects of the layer named `name`.
fn attrs(app: &App, name: &str) -> Vec<BTreeMap<String, String>> {
    let doc = &app.document.as_ref().expect("open").model;
    let Some(layer) = doc.layers().leaves().into_iter().find(|n| n.name == name) else {
        return Vec::new();
    };
    doc.by_layer(&layer.id).map(|e| e.base().attrs.clone()).collect()
}

/// A setting of the running tool written in the pane, as Enter or leaving the field writes it.
fn set(app: &mut App, key: &'static str, value: &str) {
    let tool = app.session.tool_id();
    send(
        app,
        Message::ToolSetting(kentos_desktop::tool_settings::Commit { tool, key, value: value.to_owned() }),
    );
}

fn setting(app: &App, label: &str) -> String {
    settings(app).into_iter().find(|(l, _)| l == label).map(|(_, v)| v).unwrap_or_else(|| panic!("no {label}"))
}

fn log(app: &App) -> Vec<String> {
    app.log.lines().map(|l| l.text.clone()).collect()
}

#[test]
fn a_30_m_route_says_its_time_in_seconds_and_its_columns_carry_their_units() {
    let mut app = roads();
    send(&mut app, Message::Run("tool.netRoute"));
    send(&mut app, Message::PromptOption("O"));
    click(&mut app, A.0, A.1);
    click(&mut app, C.0, C.1);
    assert!(prompt(&app).contains("Uzunluk 30.000 m, Süre 2.16 sn"), "{}", prompt(&app));
    typed(&mut app, "");
    assert!(
        log(&app).iter().any(|l| l.starts_with("En kısa yol: Uzunluk 30.000 m, Süre 2.16 sn;")),
        "{:?}",
        log(&app)
    );
    let rota = attrs(&app, "Rota");
    let [a] = &rota[..] else { panic!("one route: {rota:?}") };
    assert_eq!(a.get("Uzunluk (m)").map(String::as_str), Some("30.000"), "{a:?}");
    assert_eq!(a.get("Süre (sn)").map(String::as_str), Some("2.16"), "{a:?}");
    assert!(!a.contains_key("Süre") && !a.contains_key("Uzunluk"), "{a:?}");
}

#[test]
fn the_pane_writes_the_breaks_the_margin_and_which_outputs_are_made() {
    let mut app = service_area();
    let f = app.format();
    assert_eq!(setting(&app, "Aralıklar (m)"), "500 1000 1500");
    assert_eq!(setting(&app, "Yaklaşık alan"), "Açık");
    assert_eq!(setting(&app, "Ulaşılan yollar"), "Kapalı");
    // The breaks and the margin as typed in the pane, the unit shown after them taken as it is.
    set(&mut app, "serviceBreaks", "35 m");
    assert_eq!(setting(&app, "Aralıklar (m)"), "35");
    assert!(prompt(&app).contains("Aralıklar (R): 35 m"), "{}", prompt(&app));
    set(&mut app, "serviceTrim", "20");
    assert_eq!(setting(&app, "Kenar payı"), f.length(20.0));
    // A wrong break is said; the breaks stay as they were.
    set(&mut app, "serviceBreaks", "30 20");
    assert_eq!(setting(&app, "Aralıklar (m)"), "35");
    assert!(log(&app).iter().any(|l| l.starts_with("Aralıklar artan")), "{:?}", log(&app));
    // Only the reached ways: their layer and column said before the run.
    set(&mut app, "serviceLines", "yes");
    set(&mut app, "serviceAreas", "no");
    assert_eq!(
        setting(&app, "Çıktı"),
        "Çıktılar: “Ulaşılan yollar” (Aralık (m)): ağın ulaşılan parçaları."
    );
    assert!(prompt(&app).contains("Çizgiler (Ç): açık"), "the line's switch is the same setting: {}", prompt(&app));
    // The last output is not switched off.
    set(&mut app, "serviceLines", "no");
    assert_eq!(setting(&app, "Ulaşılan yollar"), "Açık");
    assert!(log(&app).iter().any(|l| l.contains("en az bir çıktı yazılır")), "{:?}", log(&app));
    click(&mut app, A.0, A.1);
    typed(&mut app, "");
    assert!(log(&app).iter().any(|l| l == "Hizmet alanı: 3 ulaşılan yol parçası yazıldı."), "{:?}", log(&app));
    let lines = attrs(&app, "Ulaşılan yollar");
    assert_eq!(lines.len(), 3);
    assert!(lines.iter().all(|a| a.get("Aralık (m)").map(String::as_str) == Some("0–35")), "{lines:?}");
    assert!(attrs(&app, "Yaklaşık hizmet alanı").is_empty(), "no area written");
}

/// The pictures (`.run/gui-evidence/kullanim-9355/`): Rota's table with its columns' units after the 30 m route, and
/// Hizmet alanı's settings in the pane.
#[test]
#[ignore = "pictures"]
fn pictures() {
    crate::files_testing::usage_shots("9355", "01-rota-birimler", || {
        let mut app = roads();
        send(&mut app, Message::Run("tool.netRoute"));
        send(&mut app, Message::PromptOption("O"));
        click(&mut app, A.0, A.1);
        click(&mut app, C.0, C.1);
        typed(&mut app, "");
        send(&mut app, Message::Run("data.featureTable"));
        let rota = {
            let doc = &app.document.as_ref().expect("open").model;
            doc.layers().leaves().into_iter().find(|n| n.name == "Rota").map(|n| n.id.clone()).expect("Rota")
        };
        send(&mut app, Message::Gis(crate::gis::Event::Features(crate::features::Event::Layer(rota))));
        app
    });
    crate::files_testing::usage_shots("9355", "02-hizmet-ayarlari", || {
        let mut app = service_area();
        set(&mut app, "serviceBreaks", "35 m");
        set(&mut app, "serviceLines", "yes");
        app
    });
}
