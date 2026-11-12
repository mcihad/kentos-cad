//! Plan değişikliği taslağı (zoning_draft_window.rs, docs/adr/5040) on the shared cases' drawing of `cad.zoning.draft`:
//! opened from İmar planı denetimi, a change written in the form (the park to trade) gives the case's areas by use and
//! what m. 26 asks; Taslağı kaydet keeps what the case keeps, the decision areas untouched, in one undo step; a kept
//! draft loads back into the form and Taslağı sil takes it out. And its pictures, with Sembolü karardan ver's switch.

use crate::map;
use kentos_launch::Module;
use serde_json::Value;

use crate::app::{App, Dialog, Message};
use crate::zoning_draft_window::{Event as D, Show};

fn cases() -> Value {
    serde_json::from_str(include_str!("../../../../fixtures/commands/v1/cad.zoning.draft.json")).expect("cases")
}

fn case(name: &str) -> Value {
    cases()["cases"].as_array().expect("cases").iter().find(|c| c["name"] == name).expect("the case").clone()
}

const PARK: &str = "Park alanı ticarete dönüyor: alanlar önce ve sonra, m. 26(3) bulgusu; taslak projede, karar alanı değişmez; tek adımda geri alınır";

fn opened() -> App {
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases()["setup"].clone()).expect("reads");
    let (mut app, _) = App::boot(None);
    app.program = Some(Module::Map);
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    let _ = app.update(Message::Run("zoning.check"));
    let _ = app.update(Message::Map(map::Event::ZoningDraft(D::Open)));
    app
}

fn uid_of(decision: &str) -> String {
    let c = cases();
    let at = c["setup"]["entities"].as_array().expect("entities").iter().position(|e| e["attrs"]["KararKimlik"] == decision).expect("the area");
    c["setup"]["uids"][at].as_str().expect("uid").to_owned()
}

/// The park case written through the form: the reason, the park area, Kullanım, Ticaret alanı, Değişikliğe ekle.
fn filled() -> App {
    let mut app = opened();
    let input = &case(PARK)["steps"][0]["input"];
    let _ = app.update(Message::Map(map::Event::ZoningDraft(D::Id("T-1".into()))));
    let _ = app.update(Message::Map(map::Event::ZoningDraft(D::Reason(
        input["reason"].as_str().expect("reason").to_owned(),
    ))));
    let _ = app.update(Message::Map(map::Event::ZoningDraft(D::Area(uid_of("K1")))));
    let _ = app.update(Message::Map(map::Event::ZoningDraft(D::Field(
        "Kullanim".into(),
    ))));
    let _ = app.update(Message::Map(map::Event::ZoningDraft(D::Value(
        "ticaret".into(),
    ))));
    let _ = app.update(Message::Map(map::Event::ZoningDraft(D::Add)));
    app
}

fn attrs(app: &App) -> Vec<Value> {
    let doc = &app.document.as_ref().expect("a drawing").model;
    doc.entities().map(|e| serde_json::to_value(&e.base().attrs).expect("json")).collect()
}

/// The form's plan is the case's answer; Taslağı kaydet keeps the case's draft and nothing in force changes; one undo.
#[test]
fn a_draft_written_in_the_form_is_the_cases() {
    let mut app = filled();
    assert_eq!(
        app.dialog,
        Some(Dialog::Map(map::MapDialogKind::ZoningDraft))
    );
    assert_eq!(app.map.zoning_draft.plan, "UİP-1");
    let want = case(PARK);
    let r = app
        .map
        .zoning_draft
        .answer
        .clone()
        .expect("an answer")
        .expect("plans");
    let got = serde_json::to_value(&r).expect("json");
    let out = &want["steps"][0]["result"]["output"];
    assert_eq!(
        (&got["uses"], &got["findings"], &got["level"]),
        (&out["uses"], &out["findings"], &out["level"])
    );
    let before = attrs(&app);
    let _ = app.update(Message::Map(map::Event::ZoningDraft(D::Save)));
    let kept = app
        .document
        .as_ref()
        .expect("a drawing")
        .model
        .zoning_plan()
        .cloned()
        .expect("kept");
    assert_eq!(
        serde_json::to_value(&kept).expect("json"),
        want["steps"][0]["expect"]["settings"]["zoningPlan"]
    );
    assert_eq!(attrs(&app), before);
    let _ = app.update(Message::Map(map::Event::ZoningDraft(D::Close)));
    let _ = app.update(Message::Run("edit.undo"));
    assert!(
        app.document
            .as_ref()
            .expect("a drawing")
            .model
            .zoning_plan()
            .is_none()
    );
}

/// A kept draft loads back into the form; Taslağı sil takes it out of the project and the form starts a new one.
#[test]
fn a_kept_draft_loads_and_is_removed() {
    let mut app = filled();
    let _ = app.update(Message::Map(map::Event::ZoningDraft(D::Save)));
    let _ = app.update(Message::Map(map::Event::ZoningDraft(D::Open)));
    assert_eq!(app.map.zoning_draft.id, "T-2");
    let _ = app.update(Message::Map(map::Event::ZoningDraft(D::Draft(
        "T-1".into(),
    ))));
    let s = &app.map.zoning_draft;
    assert_eq!(
        (s.id.as_str(), s.plan.as_str(), s.changes.len()),
        ("T-1", "UİP-1", 1)
    );
    assert_eq!(
        (s.changes[0].field.as_str(), s.changes[0].value.as_str()),
        ("Kullanim", "ticaret")
    );
    let _ = app.update(Message::Map(map::Event::ZoningDraft(D::Remove)));
    assert!(
        app.document
            .as_ref()
            .expect("a drawing")
            .model
            .zoning_plan()
            .is_none()
    );
    assert_eq!(app.map.zoning_draft.id, "T-1");
    assert!(app.map.zoning_draft.changes.is_empty());
}

/// `.run/gui-evidence/plan-taslak-5040/`: İmar planı denetimi with its new button; the draft with its change; the
/// areas by use; what m. 26 asks; Sembolü karardan ver with Seçimleri projede sakla.
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib zoning_draft_window_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/plan-taslak-5040");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 760.0), "koyu-dar")] {
        for (file, step) in
            [("0-imar-denetimi", 0), ("1-taslak", 1), ("2-kullanimlar", 2), ("3-mevzuat", 3), ("4-sembol-sakla", 4)]
        {
            let mut app = filled();
            let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
            app.apply_settings();
            let _ = app.update(Message::WindowResized(size));
            let _ = app.update(Message::Run("view.zoomExtents"));
            match step {
                0 => {
                    let _ = app.update(Message::Map(map::Event::ZoningDraft(D::Close)));
                    let _ = app.update(Message::Run("zoning.check"));
                }
                1 => {
                    let _ =
                        app.update(Message::Map(map::Event::ZoningDraft(D::Area(uid_of("K2")))));
                    let _ = app.update(Message::Map(map::Event::ZoningDraft(D::Field(
                        "KAKS".into(),
                    ))));
                    let _ = app.update(Message::Map(map::Event::ZoningDraft(D::Value(
                        "2.00".into(),
                    ))));
                    let _ = app.update(Message::Map(map::Event::ZoningDraft(D::Add)));
                }
                2 | 3 => {
                    let _ =
                        app.update(Message::Map(map::Event::ZoningDraft(D::Area(uid_of("K2")))));
                    let _ = app.update(Message::Map(map::Event::ZoningDraft(D::Field(
                        "KAKS".into(),
                    ))));
                    let _ = app.update(Message::Map(map::Event::ZoningDraft(D::Value(
                        "2.00".into(),
                    ))));
                    let _ = app.update(Message::Map(map::Event::ZoningDraft(D::Add)));
                    let show = if step == 2 {
                        Show::Uses
                    } else {
                        Show::Findings
                    };
                    let _ = app.update(Message::Map(map::Event::ZoningDraft(D::Show(show))));
                }
                _ => {
                    use crate::zoning_symbols_window::{Event as Z, pairs};
                    let _ = app.update(Message::Map(map::Event::ZoningDraft(D::Close)));
                    let _ = app.update(Message::Map(map::Event::ZoningSymbols(Z::Open)));
                    let r = app
                        .map
                        .zoning_symbols
                        .answer
                        .clone()
                        .expect("an answer")
                        .expect("plans");
                    let at = pairs(&r)
                        .iter()
                        .position(|p| p.usage == "konut")
                        .expect("the konut row");
                    let _ = app.update(Message::Map(map::Event::ZoningSymbols(Z::Choose(at))));
                    let _ = app.update(Message::Map(map::Event::ZoningSymbols(Z::Pick(
                        "mpyy.uip.konut.gelisme-konut-alani".into(),
                    ))));
                    let _ = app.update(Message::Map(map::Event::ZoningSymbols(Z::Write)));
                    let _ = app.update(Message::Map(map::Event::ZoningSymbols(Z::Open)));
                    let _ = app.update(Message::Map(map::Event::ZoningSymbols(Z::Choose(at))));
                }
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
