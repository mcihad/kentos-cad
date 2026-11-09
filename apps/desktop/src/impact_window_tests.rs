//! Senaryo ölçüleri (impact_window.rs, docs/adr/1440) on the shared cases' drawing of `cad.scenarios.impact`: the
//! scenario's measures and changed decisions; Raporu kaydet writes the CSV and keeps the stamp, so after the
//! field state changes the comparison says the scenario is stale; nothing is written to the drawing; the impervious
//! shares and the access typed give the shared case's measures, a share that does not read is said (docs/adr/2040);
//! Erişimi çizimde göster leaves the scenario's access on the drawing until İşareti kaldır (docs/adr/2260).
//! Harita and GIS offer it, Yol and Mimari do not. And its pictures.

use kentos_launch::Module;
use serde_json::Value;

use crate::app::{App, Dialog, Message};
use crate::impact_window::{Event as I, View};

fn opened(program: Option<Module>) -> App {
    let cases: Value =
        serde_json::from_str(include_str!("../../../fixtures/commands/v1/cad.scenarios.impact.json")).expect("cases");
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases["setup"].clone()).expect("reads");
    let (mut app, _) = App::boot(None);
    app.program = program;
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

#[test]
fn the_alternative_is_measured_saved_and_found_stale_when_the_base_moves() {
    let mut app = opened(Some(Module::Map));
    let revision = app.document.as_ref().map(|d| d.model.revision());
    let _ = app.update(Message::Run("scenario.impact"));
    assert_eq!(app.dialog, Some(Dialog::Impact));
    let r = app.impact.answer.clone().expect("an answer").expect("compared");
    let capacity = r.measures.iter().find(|m| m.id == "capacity").expect("capacity");
    assert!((capacity.difference + 1375.0).abs() < 1e-6, "{capacity:?}");
    let changes: Vec<&str> = r.changes.iter().map(|c| c.change.as_str()).collect();
    assert_eq!(changes, ["decision", "geometry", "removed", "added"]);
    assert_eq!(r.stale, None);
    let _ = app.update(Message::Impact(I::View(View::Changes)));
    // Raporu kaydet: the CSV, and the stamp kept.
    let dir = std::env::temp_dir().join(format!("kentos-senaryo-olculeri-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a folder");
    let file = dir.join("rapor.csv");
    app.impact_save(file.clone());
    let text = std::fs::read_to_string(&file).expect("written");
    assert!(text.starts_with(&format!("Senaryo;Alternatif A;Taban damgası;{}", r.stamp)), "{text}");
    assert!(text.contains("K2;Kararı değişti;ticaret-konut;konut;1500;1500;2625;2250"), "{text}");
    let _ = std::fs::remove_dir_all(dir);
    assert_eq!(app.impact.answer.clone().expect("an answer").expect("compared").stale, Some(false));
    assert_eq!(app.document.as_ref().map(|d| d.model.revision()), revision, "nothing written");
    // The field state changes: K1 moves; the alternative is stale.
    let _ = app.update(Message::Impact(I::Close));
    let doc = app.document.as_mut().expect("a drawing");
    let slot = kentos_domain::Slot(1);
    let Some(kentos_commands::Entity::Polygon(mut k1)) = doc.model.get(slot).cloned() else { panic!("K1") };
    k1.pts[1].x += 1.0;
    doc.model.update_many(vec![(slot, kentos_commands::Entity::Polygon(k1))], "Taşı");
    let _ = app.update(Message::Run("scenario.impact"));
    assert_eq!(app.impact.answer.clone().expect("an answer").expect("compared").stale, Some(true));
}

/// docs/adr/2660: the sewer line's layer chosen and 40 m typed give the shared case's infrastructure measures.
#[test]
fn the_area_near_the_infrastructure_is_the_case_s() {
    let cases: Value =
        serde_json::from_str(include_str!("../../../fixtures/commands/v1/cad.scenarios.impact.json")).expect("cases");
    let case = cases["cases"]
        .as_array()
        .expect("cases")
        .iter()
        .find(|c| c["name"] == "altyapıya yakınlık: kanalizasyon hattına 40 m")
        .expect("the case")
        .clone();
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(case["setup"].clone()).expect("reads");
    let (mut app, _) = App::boot(None);
    app.program = Some(Module::Map);
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    let _ = app.update(Message::Run("scenario.impact"));
    assert!(app.impact.layers.iter().any(|(id, name)| id == "altyapi" && name == "Kanalizasyon hattı"), "{:?}", app.impact.layers);
    let _ = app.update(Message::Impact(I::InfraLayer("altyapi".into())));
    let _ = app.update(Message::Impact(I::InfraDistance("40".into())));
    let r = app.impact.answer.clone().expect("an answer").expect("compared");
    let want = case["steps"][0]["result"]["output"]["measures"].as_array().expect("measures").clone();
    for id in ["infrastructure", "infrastructureShare"] {
        let got = r.measures.iter().find(|m| m.id == id).expect(id);
        let w = want.iter().find(|m| m["id"] == id).expect(id);
        assert_eq!(got.label, w["label"].as_str().expect("label"));
        assert!((got.base - w["base"].as_f64().expect("base")).abs() < 1e-9, "{got:?}");
        assert!((got.alternative - w["alternative"].as_f64().expect("alt")).abs() < 1e-9, "{got:?}");
    }
    // docs/adr/3060: the nearness is the case's lists; Altyapıyı çizimde göster leaves the scenario side's areas near
    // and beyond and the layer's lines on the drawing until İşareti kaldır.
    let want = &case["steps"][0]["result"]["output"]["near"];
    let near = r.near.clone().expect("the nearness");
    let got = serde_json::to_value(&near).expect("writes");
    for key in ["baseNear", "baseFar", "baseLines", "alternativeNear", "alternativeFar", "alternativeLines"] {
        assert_eq!(got[key], want[key], "{key}");
    }
    // docs/adr/3460: each measured area joined to the case's nearest line at the case's place (its centroid computed
    // two ways near 4 400 000 m, within a micrometre).
    for key in ["baseLinks", "alternativeLinks"] {
        let (g, w) = (got[key].as_array().expect(key), want[key].as_array().expect(key));
        assert_eq!(g.len(), w.len(), "{key}");
        for (a, b) in g.iter().zip(w) {
            assert_eq!((&a["area"], &a["line"]), (&b["area"], &b["line"]));
            for k in ["from", "to"] {
                for i in 0..2 {
                    assert!((a[k][i].as_f64().expect(k) - b[k][i].as_f64().expect(k)).abs() < 1e-6, "{key} {k}");
                }
            }
            assert!((a["distance"].as_f64().expect("d") - b["distance"].as_f64().expect("d")).abs() < 1e-6);
        }
    }
    let _ = app.update(Message::Impact(I::DrawNear));
    assert_eq!(app.dialog, None);
    let m = app.impact_reach_marks().expect("the marks");
    assert_eq!(m.reached.len() + m.unreached.len(), near.alternative_near.len() + near.alternative_far.len());
    assert_eq!(m.lines.len(), near.alternative_lines.len());
    assert_eq!(m.links.len(), near.alternative_links.len(), "each area to its nearest line");
    assert!(m.links.iter().all(|(_, _, words)| words.ends_with(" m")), "{:?}", m.links);
    assert!(m.lines.iter().all(|l| l.len() >= 2) && m.targets.is_empty());
    let _ = app.update(Message::Run("data.unmark"));
    assert!(app.impact_reach_marks().is_none());
    // A distance that does not read is said.
    let _ = app.update(Message::Run("scenario.impact"));
    let _ = app.update(Message::Impact(I::InfraLayer("altyapi".into())));
    let _ = app.update(Message::Impact(I::InfraDistance("kırk".into())));
    assert_eq!(app.impact.answer.clone().expect("an answer"), Err("Altyapı uzaklığı “kırk” bir sayı değil.".to_owned()));
}

/// docs/adr/2040: the user's impervious shares and access typed in the form give the shared case's measures.
#[test]
fn the_impervious_shares_and_the_access_typed_give_the_case_s_measures() {
    let mut app = opened(Some(Module::Map));
    let _ = app.update(Message::Run("scenario.impact"));
    for e in [
        I::Shares("konut 0,6; ticaret-konut 0,8; park 0,1; egitim 0,5; sanayi 0,9; yesil 0,05".into()),
        I::From("konut".into()),
        I::To("park, yesil".into()),
        I::Distance("40".into()),
    ] {
        let _ = app.update(Message::Impact(e));
    }
    let r = app.impact.answer.clone().expect("an answer").expect("compared");
    let cases: Value =
        serde_json::from_str(include_str!("../../../fixtures/commands/v1/cad.scenarios.impact.json")).expect("cases");
    let want = cases["cases"].as_array().expect("cases").iter().find(|c| c["name"] == "geçirimsizlik ve erişim birlikte")
        .expect("the case")["steps"][0]["result"]["output"]["measures"]
        .clone();
    for id in ["impervious", "access", "accessShare"] {
        let got = r.measures.iter().find(|m| m.id == id).expect("the measure");
        let w = want.as_array().expect("measures").iter().find(|m| m["id"] == id).expect("the case's");
        assert!((got.alternative - w["alternative"].as_f64().unwrap_or(f64::NAN)).abs() < 1e-9, "{id}");
        assert!((got.base - w["base"].as_f64().unwrap_or(f64::NAN)).abs() < 1e-9, "{id}");
    }
    let _ = app.update(Message::Impact(I::Shares("konut altmış".into())));
    assert_eq!(app.impact.answer.clone().expect("an answer"), Err("“altmış” bir sayı değil (geçirimsizlik payı).".into()));
}

/// The access typed in the form: housing to park and green area within 40 m (the shared case's question).
fn access_asked(app: &mut App) {
    let _ = app.update(Message::Run("scenario.impact"));
    for e in [I::From("konut".into()), I::To("park, yesil".into()), I::Distance("40".into())] {
        let _ = app.update(Message::Impact(e));
    }
}

/// docs/adr/2260: Erişimi çizimde göster leaves the scenario's access on the drawing as the shared case says; with
/// the targets deleted the marks are worked out again and nothing is reached; İşareti kaldır takes them away.
#[test]
fn the_access_stays_on_the_drawing_until_unmarked() {
    let mut app = opened(Some(Module::Map));
    access_asked(&mut app);
    let cases: Value =
        serde_json::from_str(include_str!("../../../fixtures/commands/v1/cad.scenarios.impact.json")).expect("cases");
    let want = cases["cases"].as_array().expect("cases").iter().find(|c| c["name"] == "erişim: konuttan parka ve yeşil alana 40 m")
        .expect("the case")["steps"][0]["result"]["output"]["reach"]
        .clone();
    let count = |k: &str| want[k].as_array().map_or(0, Vec::len);
    let _ = app.update(Message::Impact(I::DrawAccess));
    assert_eq!(app.dialog, None);
    let m = app.impact_reach_marks().expect("the marks");
    assert_eq!(
        (m.reached.len(), m.unreached.len(), m.targets.len()),
        (count("alternativeReached"), count("alternativeUnreached"), count("alternativeTargets"))
    );
    assert!(!m.reached.is_empty() && !m.unreached.is_empty());
    let targets: Vec<kentos_domain::Slot> = want["alternativeTargets"]
        .as_array()
        .expect("targets")
        .iter()
        .filter_map(|u| u.as_str().and_then(|u| kentos_domain::Uuid::parse_str(u).ok()))
        .filter_map(|u| app.document.as_ref().and_then(|d| d.model.slot_of(u)))
        .collect();
    app.selection.set(targets);
    let _ = app.update(Message::Run("tool.erase"));
    let m = app.impact_reach_marks().expect("the marks again");
    assert!(m.reached.is_empty() && m.targets.is_empty(), "{m:?}");
    let _ = app.update(Message::Run("data.unmark"));
    assert!(app.impact_reach_marks().is_none());
}

/// `.run/gui-evidence/senaryo-erisim-2260/`: the window with Erişimi çizimde göster, and the scenario's access on the
/// drawing.
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib impact_window_tests::access_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn access_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/senaryo-erisim-2260");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 700.0), "koyu-dar")] {
        for (draw, stem) in [(false, "1-pencere"), (true, "2-cizimde")] {
            let mut app = opened(Some(Module::Map));
            let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
            app.apply_settings();
            let _ = app.update(Message::WindowResized(size));
            let _ = app.update(Message::Run("view.zoomExtents"));
            access_asked(&mut app);
            if draw {
                let _ = app.update(Message::Impact(I::DrawAccess));
            }
            let mut snapshot = Snapshot::new(size).expect("a renderer");
            let mut update = |app: &mut App, m: Message| {
                let _ = app.update(m);
            };
            snapshot.settle(&mut app, App::view, &mut update);
            let file = out.join(format!("{stem}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
            println!("{}", file.display());
        }
    }
}

#[test]
fn harita_and_gis_offer_it_yol_and_mimari_do_not() {
    for (module, yes) in [(Module::Map, true), (Module::Gis, true), (Module::Road, false), (Module::Architecture, false)] {
        assert_eq!(opened(Some(module)).offers("scenario.impact"), yes, "{module:?}");
    }
}

/// `.run/gui-evidence/senaryo-olculeri-1440/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib impact_window_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/senaryo-olculeri-1440");
    std::fs::create_dir_all(&out).expect("a folder");
    for (view, stem) in [(View::Measures, "1-olculer"), (View::Changes, "2-degisen-kararlar")] {
        for (theme, size, name) in
            [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 700.0), "koyu-dar")]
        {
            let mut app = opened(Some(Module::Map));
            let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
            app.apply_settings();
            let _ = app.update(Message::WindowResized(size));
            let _ = app.update(Message::Run("view.zoomExtents"));
            let _ = app.update(Message::Run("scenario.impact"));
            // docs/adr/2040: the user's shares and access, so the measures show them.
            for e in [
                I::Shares("konut 0,6; ticaret-konut 0,8; park 0,1; egitim 0,5; sanayi 0,9; yesil 0,05".into()),
                I::From("konut".into()),
                I::To("park, yesil".into()),
                I::Distance("40".into()),
            ] {
                let _ = app.update(Message::Impact(e));
            }
            let _ = app.update(Message::Impact(I::View(view)));
            let mut snapshot = Snapshot::new(size).expect("a renderer");
            let mut update = |app: &mut App, m: Message| {
                let _ = app.update(m);
            };
            snapshot.settle(&mut app, App::view, &mut update);
            let file = out.join(format!("{stem}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
            println!("{}", file.display());
        }
    }
}

/// `.run/gui-evidence/senaryo-altyapi-2660/`: the measures with the area near the sewer line, 40 m.
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib impact_window_tests::infra_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn infra_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let cases: Value =
        serde_json::from_str(include_str!("../../../fixtures/commands/v1/cad.scenarios.impact.json")).expect("cases");
    let setup = cases["cases"]
        .as_array()
        .expect("cases")
        .iter()
        .find(|c| c["name"] == "altyapıya yakınlık: kanalizasyon hattına 40 m")
        .expect("the case")["setup"]
        .clone();
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/senaryo-altyapi-2660");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 700.0), "koyu-dar")] {
        let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(setup.clone()).expect("reads");
        let (mut app, _) = App::boot(None);
        app.program = Some(Module::Map);
        let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
        let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let _ = app.update(Message::Run("view.zoomExtents"));
        let _ = app.update(Message::Run("scenario.impact"));
        for e in [I::InfraLayer("altyapi".into()), I::InfraDistance("40".into())] {
            let _ = app.update(Message::Impact(e));
        }
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("1-olculer-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
        // docs/adr/3060: Altyapıyı çizimde göster, `.run/gui-evidence/altyapi-cizimde-3060/`.
        let _ = app.update(Message::Impact(I::DrawNear));
        snapshot.settle(&mut app, App::view, &mut update);
        let near = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/altyapi-cizimde-3060");
        std::fs::create_dir_all(&near).expect("a folder");
        let file = near.join(format!("altyapi-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
    }
}

/// docs/adr/3460: the roads as the infrastructure, 30 m; Altyapıyı çizimde göster joins each of the scenario's areas to
/// its nearest road (the area between the two roads to the first, by the drawing's order), the distance at the middle;
/// light wide and dark narrow, `.run/gui-evidence/en-yakin-hat-3460/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib impact_window_tests::link_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn link_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let cases: Value =
        serde_json::from_str(include_str!("../../../fixtures/commands/v1/cad.scenarios.impact.json")).expect("cases");
    let setup = cases["cases"]
        .as_array()
        .expect("cases")
        .iter()
        .find(|c| c["name"] == "senaryonun yerine geçtiği katman altyapı: her taraf kendi yollarıyla")
        .expect("the case")
        .get("setup")
        .cloned()
        .unwrap_or_else(|| cases["setup"].clone());
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/en-yakin-hat-3460");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 700.0), "koyu-dar")] {
        let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(setup.clone()).expect("reads");
        let (mut app, _) = App::boot(None);
        app.program = Some(Module::Map);
        let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
        let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let _ = app.update(Message::Run("view.zoomExtents"));
        let _ = app.update(Message::Run("scenario.impact"));
        for e in [I::InfraLayer("yol".into()), I::InfraDistance("30".into())] {
            let _ = app.update(Message::Impact(e));
        }
        let _ = app.update(Message::Impact(I::DrawNear));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("baglar-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
    }
}
