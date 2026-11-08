//! İmar planı denetimi (zoning_window.rs, docs/adr/1420, 2020) on the shared cases' drawings of `cad.zoning.check`: the
//! messy plan's seventeen findings, the Alan group's four, Seç ve göster selecting their objects; the clean plan none;
//! a broken road axis's Kopuk yol in the Alan group, its road selected; a plan outside its upper plan; the decisions
//! inside a road's planned width (docs/adr/2240), road and decisions selected;
//! a drawing without the profile's layers offered Profili uygula, after which the layers are there. Harita and GIS
//! offer it, Yol and Mimari do not. And its pictures.

use kentos_domain::Slot;
use kentos_launch::Module;
use serde_json::Value;

use crate::app::{App, Dialog, Message};
use crate::zoning_window::{Event as Z, Group};

fn opened(program: Option<Module>, case: &str) -> App {
    let cases: Value =
        serde_json::from_str(include_str!("../../../fixtures/commands/v1/cad.zoning.check.json")).expect("cases");
    let setup = cases["cases"].as_array().expect("cases").iter().find(|c| c["name"] == case).expect("the case")["setup"]
        .clone();
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(setup).expect("reads");
    let (mut app, _) = App::boot(None);
    app.program = program;
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

const MESSY: &str = "karışık plan: her bulgu türü, türün sırasıyla";
const GOOD: &str = "temiz plan: bulgu yok";
const BARE: &str = "profil katmanları yoksa söylenir";

#[test]
fn the_messy_plan_is_found_out_and_its_areas_shown() {
    let mut app = opened(Some(Module::Map), MESSY);
    let _ = app.update(Message::Run("zoning.check"));
    assert_eq!(app.dialog, Some(Dialog::Zoning));
    let r = app.zoning.answer.clone().expect("an answer").expect("checked");
    assert_eq!((r.plans, r.decisions, r.notes, r.findings.len()), (4, 7, 3, 17));
    assert_eq!(r.counts.get("outsideUpper"), Some(&1), "UIP-3 lies outside UIP-1");
    let _ = app.update(Message::Zoning(Z::Group(Group::Areas)));
    let _ = app.update(Message::Zoning(Z::Show));
    assert_eq!(app.dialog, None);
    // UIP-1 (uncovered), K1, K2 and K5 (the two overlaps, K5 outside).
    assert_eq!(app.selection.ids(), [Slot(5), Slot(9), Slot(6), Slot(1)]);
}

const ROAD_GAP: &str = "kopuk yol aksı: ucu ne yola ne plan sınırına değiyor";
const OUTSIDE_UPPER: &str = "alt plan üst planın sınırının dışına taşıyor";

/// docs/adr/2020: a road axis that ends in the middle of the plan is a Kopuk yol in the Alan group, and Seç ve göster
/// selects it; a plan outside its upper plan is an Üst plan dışına taşma in the Plan ve kademe group.
#[test]
fn a_broken_road_and_a_plan_outside_its_upper_plan() {
    let mut app = opened(Some(Module::Map), ROAD_GAP);
    let _ = app.update(Message::Run("zoning.check"));
    let r = app.zoning.answer.clone().expect("an answer").expect("checked");
    let kinds: Vec<&str> = r.findings.iter().map(|f| f.kind.as_str()).collect();
    assert_eq!(kinds, ["roadGap"]);
    assert_eq!(crate::zoning_window::kind_word("roadGap"), "Kopuk yol");
    assert!(r.findings[0].message.contains("(Y 487050.000, X 4420045.000)"), "{}", r.findings[0].message);
    let _ = app.update(Message::Zoning(Z::Group(Group::Areas)));
    let _ = app.update(Message::Zoning(Z::Show));
    assert_eq!(app.selection.ids().len(), 1, "the road");
    let mut app = opened(Some(Module::Map), OUTSIDE_UPPER);
    let _ = app.update(Message::Run("zoning.check"));
    let r = app.zoning.answer.clone().expect("an answer").expect("checked");
    assert_eq!(r.findings.iter().map(|f| f.kind.as_str()).collect::<Vec<_>>(), ["outsideUpper"]);
    assert!(Group::Plans.holds("outsideUpper"));
    assert_eq!(r.findings[0].area, Some(1000.0), "UIP-1's 20 m × 50 m past NIP-1");
}

const ROAD_WIDTH: &str = "yol genişliği çizilenden geniş: iki yandaki dört karar yola giriyor";

/// docs/adr/2240: Y1's 12 m reach 1 m into the four blocks either side: four Yola giren karar in the Alan
/// group, 45 m² each; Seç ve göster selects the road and the four decisions.
#[test]
fn the_decisions_inside_a_road_s_width() {
    let mut app = opened(Some(Module::Map), ROAD_WIDTH);
    let _ = app.update(Message::Run("zoning.check"));
    let r = app.zoning.answer.clone().expect("an answer").expect("checked");
    let kinds: Vec<&str> = r.findings.iter().map(|f| f.kind.as_str()).collect();
    assert_eq!(kinds, ["roadWidth"; 4]);
    assert!(r.findings.iter().all(|f| f.area == Some(45.0)), "{:?}", r.findings);
    assert_eq!(crate::zoning_window::kind_word("roadWidth"), "Yola giren karar");
    assert!(Group::Areas.holds("roadWidth"));
    assert!(r.findings[0].message.starts_with("“Y1” yolunun 12 m genişliği içine Konut alanı kararı “A1”"));
    let _ = app.update(Message::Zoning(Z::Group(Group::Areas)));
    let _ = app.update(Message::Zoning(Z::Show));
    assert_eq!(app.selection.ids().len(), 5, "the road and the four decisions");
}

const ROAD_PARCEL: &str = "yol genişliği çizilenden geniş: yola giren kararlar ve parseller";

/// docs/adr/2640: Y1's 12 m reach 1 m into the three parcels under and over it, 20, 25 and 45 m²: three Yola giren
/// parsel in the Alan group after the four decisions; Seç ve göster selects the road, the decisions and the parcels.
#[test]
fn the_parcels_inside_a_road_s_width() {
    let mut app = opened(Some(Module::Map), ROAD_PARCEL);
    let _ = app.update(Message::Run("zoning.check"));
    let r = app.zoning.answer.clone().expect("an answer").expect("checked");
    let parcels: Vec<&kentos_commands::ZoningFinding> = r.findings.iter().filter(|f| f.kind == "roadParcel").collect();
    assert_eq!(parcels.iter().map(|f| f.area).collect::<Vec<_>>(), [Some(20.0), Some(25.0), Some(45.0)]);
    assert_eq!(crate::zoning_window::kind_word("roadParcel"), "Yola giren parsel");
    assert!(Group::Areas.holds("roadParcel"));
    assert_eq!(parcels[0].message, "“Y1” yolunun 12 m genişliği içine “101/1” parseli giriyor.");
    let _ = app.update(Message::Zoning(Z::Group(Group::Areas)));
    let _ = app.update(Message::Zoning(Z::Show));
    assert_eq!(app.selection.ids().len(), 8, "the road, the four decisions and the three parcels");
}

#[test]
fn the_clean_plan_has_no_finding() {
    let mut app = opened(Some(Module::Map), GOOD);
    let _ = app.update(Message::Run("zoning.check"));
    let r = app.zoning.answer.clone().expect("an answer").expect("checked");
    assert!(r.findings.is_empty(), "{:?}", r.findings);
}

#[test]
fn profili_uygula_adds_the_missing_layers() {
    let mut app = opened(Some(Module::Map), BARE);
    let _ = app.update(Message::Run("zoning.check"));
    let r = app.zoning.answer.clone().expect("an answer").expect("checked");
    assert_eq!(r.counts.get("missingLayer"), Some(&2));
    let _ = app.update(Message::Zoning(Z::ApplyProfile));
    let r = app.zoning.answer.clone().expect("an answer").expect("checked");
    assert_eq!(r.counts.get("missingLayer"), None, "{:?}", r.findings);
    let _ = app.update(Message::Run("edit.undo"));
    let _ = app.update(Message::Zoning(Z::Tolerance(0.01)));
    let r = app.zoning.answer.clone().expect("an answer").expect("checked");
    assert_eq!(r.counts.get("missingLayer"), Some(&2), "one undo step");
}

#[test]
fn harita_and_gis_offer_it_yol_and_mimari_do_not() {
    for (module, yes) in [(Module::Map, true), (Module::Gis, true), (Module::Road, false), (Module::Architecture, false)] {
        assert_eq!(opened(Some(module), GOOD).offers("zoning.check"), yes, "{module:?}");
    }
}

/// `.run/gui-evidence/imar-yol-genisligi-2240/`: the window with the four findings, and the drawing with the road and
/// the decisions Seç ve göster selected.
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib zoning_window_tests::road_width_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn road_width_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/imar-yol-genisligi-2240");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 700.0), "koyu-dar")] {
        for (show, file) in [(false, "1-pencere"), (true, "2-secilenler")] {
            let mut app = opened(Some(Module::Map), ROAD_WIDTH);
            let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
            app.apply_settings();
            let _ = app.update(Message::WindowResized(size));
            let _ = app.update(Message::Run("view.zoomExtents"));
            let _ = app.update(Message::Run("zoning.check"));
            let _ = app.update(Message::Zoning(Z::Group(Group::Areas)));
            if show {
                let _ = app.update(Message::Zoning(Z::Show));
            }
            let mut snapshot = Snapshot::new(size).expect("a renderer");
            let mut update = |app: &mut App, m: Message| {
                let _ = app.update(m);
            };
            snapshot.settle(&mut app, App::view, &mut update);
            let path = out.join(format!("{file}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&path).expect("writes");
            println!("{}", path.display());
        }
    }
}

/// `.run/gui-evidence/imar-denetimi-1420/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib zoning_window_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/imar-denetimi-1420");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 700.0), "koyu-dar")] {
        let mut app = opened(Some(Module::Map), MESSY);
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let _ = app.update(Message::Run("view.zoomExtents"));
        let _ = app.update(Message::Run("zoning.check"));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("denetim-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
        // docs/adr/2020: the broken road in the Alan group.
        let mut app = opened(Some(Module::Map), ROAD_GAP);
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let _ = app.update(Message::Run("view.zoomExtents"));
        let _ = app.update(Message::Run("zoning.check"));
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("kopuk-yol-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
    }
}

/// `.run/gui-evidence/imar-yol-parsel-2640/`: the Alan group with the decisions and the parcels inside Y1's width, and
/// the drawing with them Seç ve göster selected.
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib zoning_window_tests::road_parcel_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn road_parcel_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/imar-yol-parsel-2640");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 700.0), "koyu-dar")] {
        for (show, file) in [(false, "1-pencere"), (true, "2-secilenler")] {
            let mut app = opened(Some(Module::Map), ROAD_PARCEL);
            let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
            app.apply_settings();
            let _ = app.update(Message::WindowResized(size));
            let _ = app.update(Message::Run("view.zoomExtents"));
            let _ = app.update(Message::Run("zoning.check"));
            let _ = app.update(Message::Zoning(Z::Group(Group::Areas)));
            if show {
                let _ = app.update(Message::Zoning(Z::Show));
            }
            let mut snapshot = Snapshot::new(size).expect("a renderer");
            let mut update = |app: &mut App, m: Message| {
                let _ = app.update(m);
            };
            snapshot.settle(&mut app, App::view, &mut update);
            let path = out.join(format!("{file}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&path).expect("writes");
            println!("{}", path.display());
        }
    }
}
