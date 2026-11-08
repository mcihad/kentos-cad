//! Yeniden izdüşür's preview (reproject_window.rs, docs/adr/2500) on the drawing of `cad.layers.reproject`'s shared
//! cases: a layer's Koordinat sistemi ▸ Yeniden izdüşür ▸ opens the window on the command's plan, which is the shared
//! case's (each object's shift, the notes, the shifts summed up, the chain's verdict, the warnings) with nothing
//! written; Uygula writes the case's move as one undo step, Vazgeç nothing; a layer whose text and circle do not move
//! says each reason counted, as the case does, and Nesneleri seç selects the objects. And its pictures.

use kentos_domain::Slot;
use serde_json::Value;

use crate::app::{App, Dialog, Message};
use crate::layer_crs::Event as L;
use crate::reproject_window::Event as R;

fn cases() -> Value {
    serde_json::from_str(include_str!("../../../fixtures/commands/v1/cad.layers.reproject.json")).expect("cases")
}

/// A shared case by the start of its name.
fn case(name: &str) -> Value {
    cases()["cases"]
        .as_array()
        .expect("cases")
        .iter()
        .find(|c| c["name"].as_str().is_some_and(|n| n.starts_with(name)))
        .cloned()
        .expect(name)
}

/// The cases' drawing (a version 1 document, given its objects' ids as any opened file is).
fn opened() -> App {
    let v1: kentos_commands::DocumentSnapshotV1 = serde_json::from_value(cases()["setup"].clone()).expect("reads");
    let snapshot = kentos_commands::migrate_v1(v1).expect("migrates");
    let (mut app, _) = App::boot(None);
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

/// A case's expected JSON with the drawing's own ids and revision for `$uidOf:N` and `$current`.
fn resolved(app: &App, v: &Value) -> Value {
    let doc = &app.document.as_ref().expect("a drawing").model;
    match v {
        Value::String(s) if s.starts_with("$uidOf:") => {
            let slot: u32 = s["$uidOf:".len()..].parse().expect("a slot");
            Value::from(doc.uid(Slot(slot)).expect("an id").to_string())
        }
        Value::String(s) if s == "$current" => Value::from(doc.revision().to_string()),
        Value::Array(a) => Value::Array(a.iter().map(|x| resolved(app, x)).collect()),
        Value::Object(o) => Value::Object(o.iter().map(|(k, x)| (k.clone(), resolved(app, x))).collect()),
        other => other.clone(),
    }
}

/// The same JSON, numbers within a millimetre (the case's `within`), `$any` taking anything.
fn close(got: &Value, want: &Value, at: &str) {
    match (got, want) {
        (_, Value::String(s)) if s == "$any" => {}
        (Value::Number(a), Value::Number(b)) => {
            let (a, b) = (a.as_f64().expect("a number"), b.as_f64().expect("a number"));
            assert!((a - b).abs() <= 1e-3, "{at}: {a} ≠ {b}");
        }
        (Value::Array(a), Value::Array(b)) => {
            assert_eq!(a.len(), b.len(), "{at}: length");
            for (i, (x, y)) in a.iter().zip(b).enumerate() {
                close(x, y, &format!("{at}[{i}]"));
            }
        }
        (Value::Object(a), Value::Object(b)) => {
            let mut keys: Vec<&String> = a.keys().chain(b.keys()).collect();
            keys.sort();
            keys.dedup();
            for k in keys {
                close(a.get(k).unwrap_or(&Value::Null), b.get(k).unwrap_or(&Value::Null), &format!("{at}.{k}"));
            }
        }
        _ => assert_eq!(got, want, "{at}"),
    }
}

fn plan(app: &App) -> (Value, Value) {
    let (out, warnings) = app.reproject.plan.clone().expect("a plan").expect("not refused");
    (serde_json::to_value(out).expect("json"), serde_json::to_value(warnings).expect("json"))
}

fn point(app: &App, slot: u32) -> kentos_commands::Entity {
    app.document.as_ref().expect("a drawing").model.get(Slot(slot)).cloned().expect("the object")
}

#[test]
fn the_preview_is_the_shared_case_s_plan_and_uygula_writes_it() {
    let mut app = opened();
    let revision = app.document.as_ref().map(|d| d.model.revision());
    let before = (point(&app, 15), point(&app, 16));
    let _ = app.update(Message::LayerCrs(L::Reproject("uzak".into(), Some(2322))));
    assert_eq!(app.dialog, Some(Dialog::Reproject));
    let c = case("önizleme: kullanım alanlarının dışındaki nokta");
    let want = resolved(&app, &c["steps"][0]["result"]);
    let (out, warnings) = plan(&app);
    close(&out, &want["output"], "plan");
    close(&warnings, &want["warnings"], "warnings");
    // The point far north is listed first, with its three notes; nothing is written yet.
    assert_eq!(out["shifts"][0]["notes"].as_array().map(Vec::len), Some(3));
    assert_eq!(app.document.as_ref().map(|d| d.model.revision()), revision);
    assert_eq!((point(&app, 15), point(&app, 16)), before);
    // Uygula writes the case's move.
    let _ = app.update(Message::Reproject(R::Apply));
    assert_eq!(app.dialog, None);
    let moved = resolved(&app, &c["steps"][1]["expect"]["entities"]);
    for slot in [15, 16] {
        close(
            &serde_json::to_value(point(&app, slot)).expect("json"),
            &moved[slot.to_string()],
            &format!("object {slot}"),
        );
    }
    assert!(crate::files_testing::last_said(&app).contains("2 nesnesi"), "{}", crate::files_testing::last_said(&app));
    // One undo step takes it back.
    let _ = app.update(Message::Run("edit.undo"));
    assert_eq!((point(&app, 15), point(&app, 16)), before);
}

#[test]
fn vazgec_writes_nothing_and_the_old_sheet_s_preview_is_the_case_s() {
    let mut app = opened();
    let revision = app.document.as_ref().map(|d| d.model.revision());
    let _ = app.update(Message::LayerCrs(L::Reproject("eski".into(), Some(2322))));
    let want = resolved(&app, &case("plan hiçbir şey yazmaz")["steps"][0]["result"]);
    let (out, warnings) = plan(&app);
    close(&out, &want["output"], "plan");
    close(&warnings, &want["warnings"], "warnings");
    let _ = app.update(Message::Reproject(R::Close));
    assert_eq!(app.dialog, None);
    assert_eq!(app.document.as_ref().map(|d| d.model.revision()), revision);
}

#[test]
fn a_refused_layer_says_each_reason_and_selects_its_objects() {
    let mut app = opened();
    let _ = app.update(Message::LayerCrs(L::Reproject("yazilar".into(), Some(4326))));
    let want = resolved(&app, &case("ret: nedenler karışıkken")["steps"][0]["result"]["error"]);
    let refused = app.reproject.plan.clone().expect("a plan").expect_err("refused");
    assert_eq!(Value::from(refused.message.clone()), want["message"]);
    assert_eq!(serde_json::to_value(&refused.uids).expect("json"), want["uids"]);
    // Uygula waits; Nesneleri seç selects the three objects and closes the window.
    let _ = app.update(Message::Reproject(R::Apply));
    let _ = app.update(Message::Reproject(R::Select));
    assert_eq!(app.dialog, None);
    let mut picked: Vec<u32> = app.selection.ids().iter().map(|s| s.0).collect();
    picked.sort_unstable();
    assert_eq!(picked, vec![17, 18, 19]);
}

/// The preview of the far point's layer (its notes), of the old sheet's (a plain move) and the refused layer's, light
/// and wide, dark and narrow; `.run/gui-evidence/yeniden-izdusur-2500/`:
///
/// ```text
/// cargo test -p kentos-desktop --lib reproject_window_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/yeniden-izdusur-2500");
    std::fs::create_dir_all(&out).expect("a folder");
    for (layer, from, stem) in [("uzak", 2322, "1-notlu"), ("eski", 2322, "2-eski-pafta"), ("yazilar", 4326, "3-ret")] {
        for (theme, size, name) in
            [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 760.0), "koyu-dar")]
        {
            let mut app = opened();
            let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
            app.apply_settings();
            let _ = app.update(Message::WindowResized(size));
            let _ = app.update(Message::Run("view.zoomExtents"));
            let _ = app.update(Message::LayerCrs(L::Reproject(layer.into(), Some(from))));
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
