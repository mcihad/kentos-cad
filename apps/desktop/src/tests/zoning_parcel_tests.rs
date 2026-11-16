//! Parselin plan kararları in Proje ayarları › Mimari › İmar durumu (docs/adr/8072 §5): the site plan's parcel across
//! two decisions of the zoning plan (the shared cases' drawing, fixtures/commands/v1/cad.zoning.parcel.json), each with
//! its values, its plan and approval date and its notes; the typed emsal and TAKS against the plan's in the warning
//! colour; nothing written into the fields.
//!
//! ```text
//! RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib zoning_parcel_tests::screens -- --ignored --nocapture
//! ```

use crate::app::{App, Message};
use crate::document::Document;
use crate::files_testing::drive;
use crate::project::{SettingsEvent, settings_message};

/// The shared cases' drawing opened, Proje ayarları on its Mimari page.
fn opened() -> App {
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("../../../../fixtures/commands/v1/cad.zoning.parcel.json")).expect("the cases");
    let snapshot: kentos_commands::DocumentSnapshotV2 =
        serde_json::from_value(cases["setup"].clone()).expect("a snapshot");
    let model = kentos_domain::Document::from_snapshot_v2(snapshot).expect("opens");
    let (mut app, _) = App::boot(None);
    let doc = Document::from_v2(model.to_snapshot_v2(), None).expect("opens");
    let task = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    drive(&mut app, task);
    let _ = app.update(Message::Run("file.settings"));
    let _ = app.update(settings_message(SettingsEvent::Section(crate::project::settings::Section::Arch)));
    app
}

#[test]
fn imar_durumu_lists_the_parcel_s_decisions_with_their_source() {
    let app = opened();
    let lines = crate::zoning_badge::plan_lines(&app.document.as_ref().expect("a drawing").model);
    let words: Vec<&str> = lines.iter().map(|(w, _)| w.as_str()).collect();
    assert!(words.iter().any(|w| w.starts_with("K1 (300,00 m²): Kullanım konut")), "{words:#?}");
    assert!(words.iter().any(|w| w.starts_with("K2 (600,00 m²)")), "{words:#?}");
    assert!(words.contains(&"Kaynak: UIP-1 Merkez uygulama imar planı (yururlukte, onay 2024-05-14, Meclis kararı 2024/112)"));
    assert!(words.contains(&"Plan notu 2: Çatı katı yapılamaz."));
    // The typed values against the plan's, in the warning colour; the fields untouched.
    assert!(lines.contains(&("İmar durumunda emsal 1,20; plan kararı KAKS 1.20 / 2,00.".to_owned(), true)), "{lines:#?}");
    let zoning = app.document.as_ref().expect("a drawing").model.settings().arch.as_ref().and_then(|a| a.zoning);
    assert_eq!(zoning.and_then(|z| z.far), Some(1.2));
}

#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::{Input, Snapshot};

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/plan-kararlari-8072");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik"), ("dark", Size::new(1280.0, 800.0), "koyu")] {
        let mut app = opened();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        // Down the Mimari page to İmar durumu's plan decisions: the page scrolled under the pointer, step by step.
        let at = iced::Point::new(size.width * 0.55, size.height * 0.5);
        // To the page's end (İmar durumu, Yangın güvenliği and the lists close it), then up to the decisions' heading.
        snapshot.input(&mut app, App::view, &mut update, Input::Scroll(at, -60.0));
        snapshot.settle(&mut app, App::view, &mut update);
        snapshot.input(&mut app, App::view, &mut update, Input::Scroll(at, 2.5));
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("1-imar-durumu-plan-kararlari-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
    }
}
