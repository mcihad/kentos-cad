//! Adayları uzlaştır's Alan gir (features_reconcile_fill.rs, docs/adr/2540) on the shared case's drawing whose class
//! layers carry the profile's fields: the accepted manhole's network “Atıksu” is not on the list and the door has no
//! number, as the case says; Alan gir chooses Kanalizasyon from the list and types the door's number, both are written
//! with the values the rules give; a number too long stays refused with the rule's sentence. And its pictures.

use kentos_launch::Module;
use serde_json::Value;

use crate::app::{App, Dialog, Message};
use crate::features_reconcile_window::Event as R;

const RULES: &str = "Alan kuralları: direğin türü (Aydınlatma) listenin kodu olarak yazılır";

fn case(start: &str) -> Value {
    let cases: Value =
        serde_json::from_str(include_str!("../../../fixtures/commands/v1/map.features.reconcile.json")).expect("cases");
    cases["cases"].as_array().expect("cases").iter().find(|c| c["name"].as_str().is_some_and(|n| n.starts_with(start))).cloned().expect(start)
}

fn opened() -> App {
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(case(RULES)["setup"].clone()).expect("reads");
    let (mut app, _) = App::boot(None);
    app.program = Some(Module::Map);
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    let _ = app.update(Message::Run("map.featuresReconcile"));
    assert_eq!(app.dialog, Some(Dialog::FeaturesReconcile));
    app
}

fn answer(app: &App) -> kentos_commands::FeaturesReconciled {
    app.features_reconcile.answer.clone().expect("an answer").expect("reconciled")
}

fn group(app: &App, class: &str) -> usize {
    answer(app).groups.iter().position(|g| g.class.as_deref() == Some(class) && g.status != "existing").expect(class)
}

fn send(app: &mut App, e: R) {
    let _ = app.update(Message::FeaturesReconcile(e));
}

#[test]
fn alan_gir_fills_the_list_and_the_number_and_both_are_written() {
    let mut app = opened();
    let manhole = group(&app, "rogar");
    send(&mut app, R::Accept(manhole));
    // The case's sentence: Atıksu is not on the network list, so the accepted manhole waits.
    let g = &answer(&app).groups[manhole];
    let want = &case(RULES)["steps"][0]["result"]["output"]["groups"][manhole]["invalid"];
    assert_eq!(serde_json::to_value(&g.invalid).expect("json"), *want);
    assert!(!g.writes);
    // Alan gir: the network chosen from the list (its code goes in).
    send(&mut app, R::Fill(manhole));
    assert_eq!(crate::features_reconcile_fill::fill_fields(&answer(&app).groups[manhole]), ["Sebeke"]);
    send(&mut app, R::Typed("Sebeke".into(), "kanalizasyon".into()));
    send(&mut app, R::FillApply);
    let g = &answer(&app).groups[manhole];
    assert!(g.invalid.is_empty() && g.writes, "{g:?}");
    // The door's number typed.
    let door = group(&app, "kapi");
    send(&mut app, R::Fill(door));
    assert_eq!(crate::features_reconcile_fill::fill_fields(&answer(&app).groups[door]), ["KapiNo"]);
    send(&mut app, R::Typed("KapiNo".into(), "14".into()));
    send(&mut app, R::FillApply);
    assert!(answer(&app).groups[door].writes);
    // Uzlaştır: both written with the rules' values.
    send(&mut app, R::Run);
    let doc = &app.document.as_ref().expect("a drawing").model;
    let attrs: Vec<std::collections::BTreeMap<String, String>> = doc.entities().map(|e| e.base().attrs.clone()).collect();
    assert!(attrs.iter().any(|a| a.get("Sebeke").map(String::as_str) == Some("kanalizasyon")), "the manhole");
    assert!(attrs.iter().any(|a| a.get("KapiNo").map(String::as_str) == Some("14")), "the door");
}

#[test]
fn a_value_the_rules_refuse_stays_with_its_reason() {
    let mut app = opened();
    let door = group(&app, "kapi");
    send(&mut app, R::Fill(door));
    // Nothing typed: said, nothing decided.
    send(&mut app, R::FillApply);
    assert!(app.features_reconcile.decisions.is_empty());
    assert!(crate::files_testing::last_said(&app).contains("bir değer yazın"));
    send(&mut app, R::Typed("KapiNo".into(), "1234567890123456789012345".into()));
    send(&mut app, R::FillApply);
    let g = &answer(&app).groups[door];
    assert!(!g.writes);
    assert_eq!(g.invalid.len(), 1);
    assert_eq!(g.invalid[0].message, "“Kapı no” alanı en çok 20 karakter alır; 25 karakter verildi.");
}

/// Alan gir open on the manhole (the list) and on the door (a refused number), light wide and dark narrow,
/// `.run/gui-evidence/alan-gir-2540/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib features_reconcile_fill_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/alan-gir-2540");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 760.0), "koyu-dar")] {
        for stem in ["1-rogar-listeden", "2-kapi-kurala-uymaz"] {
            let mut app = opened();
            let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
            app.apply_settings();
            let _ = app.update(Message::WindowResized(size));
            if stem.starts_with('1') {
                let manhole = group(&app, "rogar");
                send(&mut app, R::Accept(manhole));
                send(&mut app, R::Fill(manhole));
                send(&mut app, R::Typed("Sebeke".into(), "kanalizasyon".into()));
            } else {
                let door = group(&app, "kapi");
                send(&mut app, R::Fill(door));
                send(&mut app, R::Typed("KapiNo".into(), "1234567890123456789012345".into()));
                send(&mut app, R::FillApply);
                send(&mut app, R::Fill(door));
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
