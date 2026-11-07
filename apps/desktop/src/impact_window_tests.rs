//! Alternatif etkisi (impact_window.rs, docs/adr/1440) on the shared cases' drawing of `cad.scenarios.impact`: the
//! alternative's measures and changed decisions; Raporu kaydet writes the CSV and keeps the stamp, so after the
//! field state changes the comparison says the alternative is stale; nothing is written to the drawing; the impervious
//! shares and the access typed give the shared case's measures, a share that does not read is said (docs/adr/2040).
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
    let dir = std::env::temp_dir().join(format!("kentos-alternatif-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a folder");
    let file = dir.join("rapor.csv");
    app.impact_save(file.clone());
    let text = std::fs::read_to_string(&file).expect("written");
    assert!(text.starts_with(&format!("Alternatif;Alternatif A;Taban damgası;{}", r.stamp)), "{text}");
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

#[test]
fn harita_and_gis_offer_it_yol_and_mimari_do_not() {
    for (module, yes) in [(Module::Map, true), (Module::Gis, true), (Module::Road, false), (Module::Architecture, false)] {
        assert_eq!(opened(Some(module)).offers("scenario.impact"), yes, "{module:?}");
    }
}

/// `.run/gui-evidence/alternatif-etkisi-1440/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib impact_window_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/alternatif-etkisi-1440");
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
