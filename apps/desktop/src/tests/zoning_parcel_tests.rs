//! Parselin plan kararları in Proje ayarları › Mimari › İmar durumu (docs/adr/8072 §5): the site plan's parcel across
//! two decisions of the zoning plan (the shared cases' drawing, fixtures/commands/v1/cad.zoning.parcel.json), each with
//! its values, its plan and approval date and its notes; the typed emsal and TAKS against the plan's in the warning
//! colour; nothing written into the fields until a decision's Değerlerini al (docs/adr/8140) takes its values with
//! their source, and nothing saved until Kaydet.
//!
//! ```text
//! RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib zoning_parcel_tests::screens -- --ignored --nocapture
//! RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib zoning_parcel_tests::take_screens -- --ignored --nocapture
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
    let lines = crate::zoning_badge::plan_lines(app.document.as_ref().expect("a drawing"));
    let words: Vec<&str> = lines.iter().map(|(w, _)| w.as_str()).collect();
    assert!(words.iter().any(|w| w.starts_with("K1 (300,00 m²): Kullanım konut")), "{words:#?}");
    assert!(words.iter().any(|w| w.starts_with("K2 (600,00 m²)")), "{words:#?}");
    assert!(words.contains(&"Kaynak: UIP-1 Merkez uygulama imar planı (yururlukte, onay 2024-05-14, Meclis kararı 2024/112)"));
    assert!(words.contains(&"Plan notu 2: Çatı katı yapılamaz."));
    // The typed values against the plan's, in the warning colour; the fields untouched.
    assert!(lines.contains(&("İmar durumunda emsal 1,20; plan kararı KAKS 1.20 / 2,00.".to_owned(), true)), "{lines:#?}");
    let zoning = app.document.as_ref().expect("a drawing").model.settings().arch.as_ref().and_then(|a| a.zoning.clone());
    assert_eq!(zoning.and_then(|z| z.far), Some(1.2));
}

fn send(app: &mut App, e: SettingsEvent) {
    let _ = app.update(settings_message(e));
}

/// İmar durumu as Proje ayarları holds it now.
fn form(app: &App) -> &crate::project::arch::ZoningTexts {
    match app.project.as_ref() {
        Some(crate::project::Window::Settings(s)) => s.zoning_texts(),
        _ => panic!("Proje ayarları is open"),
    }
}

/// The plan notes as the form holds them: measure, value, source.
fn notes(app: &App) -> Vec<(kentos_commands::PlanNoteKey, String, String)> {
    form(app).plan_notes.iter().map(|n| (n.key, n.value.clone(), n.source.clone())).collect()
}

// The plan by its id, in force (no state said); its name and basis stay in the decision's Kaynak line.
const K1: &str = "UIP-1, onay 2024-05-14, karar K1";
const K2: &str = "UIP-1, onay 2024-05-14, karar K2";

/// Değerlerini al (docs/adr/8140): each decision gives the values written as numbers its fields take, its button after
/// its last line; K1's emsal and TAKS go into İmar durumu with their source and its storey count into Plan notları, K2's
/// over them with its building height; Vazgeç writes nothing, Kaydet writes them through `cad.settings.set`.
#[test]
fn a_decision_s_values_are_taken_with_their_source_and_saved_by_kaydet() {
    use kentos_commands::BuildingLayout::{Block, Detached};
    use kentos_commands::PlanNoteKey::{BuildingHeight, Storeys};
    let mut app = opened();
    let (lines, takes) = {
        let doc = app.document.as_ref().expect("a drawing");
        (crate::zoning_badge::plan_lines(doc), crate::zoning_badge::plan_takes(doc))
    };
    assert_eq!(takes.len(), 2, "{takes:#?}");
    let (after, k1) = takes[0].clone();
    assert_eq!(lines[after].0, "Plan notu 2: Çatı katı yapılamaz.", "K1's button after its notes");
    assert_eq!((k1.far, k1.coverage, k1.height, k1.storeys), (Some(1.2), Some(0.4), None, Some(3.0)));
    // Nizam “A”, the profile's code (docs/adr/8141).
    assert_eq!(k1.layout, Some(Detached));
    assert_eq!(k1.source, K1);
    // Kullanım “konut”, the profile's code: a dwelling area (docs/adr/8143).
    assert_eq!(k1.zone, Some(kentos_commands::Zone::Residential));
    assert_eq!(
        k1.words(false),
        "Emsal 1,20, TAKS 0,40, nizam ayrık ve bölge konut alanı İmar durumu'na, kat adedi 3 Plan notları'na."
    );
    let (after, k2) = takes[1].clone();
    assert_eq!(lines[after].0, "Plan notu 1: Bodrum katlar emsale dahil değildir.");
    // “2,00” is read as 2 as the conflicts read it; Kullanım, Nizam and the setbacks are never taken.
    assert_eq!((k2.far, k2.coverage, k2.height, k2.storeys), (Some(2.0), Some(0.5), Some(15.5), None));
    assert_eq!((k2.layout, k2.zone), (Some(Block), Some(kentos_commands::Zone::Mixed)));
    assert_eq!(
        k2.words(false),
        "Emsal 2,00, TAKS 0,50, nizam blok ve bölge ticaret + konut alanı İmar durumu'na, bina yüksekliği 15,50 m Plan \
         notları'na."
    );
    // The typed emsal and TAKS are K1's, but no source says so: K1's are not held yet.
    assert!(!form(&app).holds(&k1));

    send(&mut app, SettingsEvent::TakePlanValues(k1.clone()));
    let f = form(&app);
    assert_eq!((f.far.as_str(), f.far_source.as_str()), ("1.2", K1));
    assert_eq!((f.coverage.as_str(), f.coverage_source.as_str()), ("0.4", K1));
    assert_eq!((f.layout, f.zone), (Some(Detached), Some(kentos_commands::Zone::Residential)));
    assert_eq!(notes(&app), [(Storeys, "3".to_owned(), K1.to_owned())]);
    assert!(form(&app).holds(&k1) && !form(&app).holds(&k2), "K1's button is spent, K2's is not");
    // Not saved: the project is as it was.
    let saved = |app: &App| app.document.as_ref().expect("a drawing").model.settings().arch.clone().unwrap_or_default();
    assert_eq!(saved(&app).zoning.as_ref().and_then(|z| z.far_source.clone()), None);

    send(&mut app, SettingsEvent::TakePlanValues(k2.clone()));
    let f = form(&app);
    assert_eq!((f.far.as_str(), f.far_source.as_str()), ("2", K2));
    assert_eq!((f.coverage.as_str(), f.coverage_source.as_str()), ("0.5", K2));
    assert_eq!((f.layout, f.zone), (Some(Block), Some(kentos_commands::Zone::Mixed)));
    // K1's storey count stays with its source; K2's height takes a row of its own.
    assert_eq!(
        notes(&app),
        [(Storeys, "3".to_owned(), K1.to_owned()), (BuildingHeight, "15.5".to_owned(), K2.to_owned())]
    );
    assert!(!form(&app).holds(&k1) && form(&app).holds(&k2));

    // Vazgeç: the project as it was.
    let _ = app.update(Message::Project(Box::new(crate::project::Event::Close)));
    assert!(app.project.is_none(), "closed");
    let arch = saved(&app);
    let z = arch.zoning.clone().expect("İmar durumu");
    assert_eq!((z.far, z.far_source), (Some(1.2), None), "Vazgeç writes nothing");
    assert!(arch.plan_notes.is_empty() && arch.layout.is_none() && arch.zone.is_none());

    // Again, and Kaydet writes them through `cad.settings.set`.
    let _ = app.update(Message::Run("file.settings"));
    send(&mut app, SettingsEvent::Section(crate::project::settings::Section::Arch));
    send(&mut app, SettingsEvent::TakePlanValues(k1));
    send(&mut app, SettingsEvent::TakePlanValues(k2));
    send(&mut app, SettingsEvent::Save);
    assert!(app.project.is_none(), "saved");
    let arch = saved(&app);
    let z = arch.zoning.clone().expect("İmar durumu");
    assert_eq!((z.parcel_area, z.far, z.coverage), (900.0, Some(2.0), Some(0.5)));
    assert_eq!((z.far_source.as_deref(), z.coverage_source.as_deref()), (Some(K2), Some(K2)));
    assert_eq!((arch.layout, arch.zone), (Some(Block), Some(kentos_commands::Zone::Mixed)));
    let written: Vec<_> = arch.plan_notes.iter().map(|n| (n.key, n.value, n.source.as_str())).collect();
    assert_eq!(written, [(Storeys, 3.0, K1), (BuildingHeight, 15.5, K2)]);
    // The plan decisions read the saved emsal: K1 says otherwise, in the warning colour.
    let lines = crate::zoning_badge::plan_lines(app.document.as_ref().expect("a drawing"));
    assert!(lines.contains(&("İmar durumunda emsal 2,00; plan kararı KAKS 1.20 / 2,00.".to_owned(), true)), "{lines:#?}");
}

/// A decision without its plan, or that writes no number a field takes, gives nothing to take; a value out of its
/// field's bounds (a TAKS written as a percentage, a storey count with words) is not taken, nor a Nizam not the profile's
/// code (“A-3”, docs/adr/8141); a plan not in force says its state in the source (docs/adr/8140).
#[test]
fn only_numbers_a_field_takes_from_a_decision_with_its_plan_are_offered() {
    let app = opened();
    let drawn = app.document.as_ref().expect("a drawing").model.to_snapshot_v2();
    // K1's values in words or out of bounds; K2 under `plan` (UIP-2: askıda, no name, no approval written).
    let with = |plan: &str| {
        let mut snapshot = drawn.clone();
        for e in &mut snapshot.entities {
            let attrs = &mut e.base_mut().attrs;
            match attrs.get("KararKimlik").map(String::as_str) {
                Some("K1") => {
                    attrs.insert("TAKS".into(), "40".into());
                    attrs.insert("KatAdedi".into(), "Z+3".into());
                    attrs.insert("KAKS".into(), "serbest".into());
                    // Not the profile's codes: not read as ayrık (docs/adr/8141), nor as a use (docs/adr/8143).
                    attrs.insert("Nizam".into(), "A-3".into());
                    attrs.insert("Kullanim".into(), "konut+ticaret".into());
                }
                Some("K2") => {
                    attrs.insert("Plan".into(), plan.into());
                }
                _ => {}
            }
        }
        // Each drawing opened anew: its readings are its own, not the last one's (docs/adr/8140).
        let doc = Document::from_v2(snapshot, None).expect("opens");
        crate::zoning_badge::plan_takes(&doc).into_iter().map(|(_, t)| t).collect::<Vec<_>>()
    };
    let takes = with("UIP-2");
    assert_eq!(takes.len(), 1, "K1 gives nothing: {takes:#?}");
    assert_eq!(takes[0].source, "UIP-2 (askida), karar K2");
    assert_eq!((takes[0].far, takes[0].coverage, takes[0].height), (Some(2.0), Some(0.5), Some(15.5)));
    assert_eq!(takes[0].layout, Some(kentos_commands::BuildingLayout::Block));
    // A plan not drawn: its source could not be said.
    let none = with("UIP-9");
    assert!(none.is_empty(), "{none:#?}");
    // The drawing as it was, opened again: its own readings, not the last drawing's.
    assert_eq!(with("UIP-1").len(), 1, "K2 again");
}

/// Değerlerini al's pictures (docs/adr/8140): the decisions with their buttons; K1 taken (its emsal and TAKS with their
/// source in İmar durumu; its button spent, its words where they are); K1 and K2 taken, Plan notları's rows. Light and
/// dark.
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn take_screens() {
    use iced::Size;
    use kentos_ui::snapshot::{Input, Snapshot};

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/plan-degerleri-8140");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    // KENTOS_SCROLLS="decisions=2.5,fields=4,notes=1.5" moves a picture's view without building again.
    let scrolls = std::env::var("KENTOS_SCROLLS").unwrap_or_default();
    let up = |name: &str, default: f32| {
        scrolls
            .split(',')
            .filter_map(|p| p.split_once('='))
            .find(|(k, _)| *k == name)
            .and_then(|(_, n)| n.parse().ok())
            .unwrap_or(default)
    };
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik"), ("dark", Size::new(1280.0, 800.0), "koyu")] {
        for (shot, taken, back) in
            [
            ("1-degerlerini-al", 0, up("decisions", 12.0)),
            ("2-imar-durumu", 1, up("fields", 20.0)),
            ("3-alindi", 1, up("held", 12.0)),
            ("4-plan-notlari", 2, up("notes", 2.4)),
        ]
        {
            let mut app = opened();
            let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
            app.apply_settings();
            let _ = app.update(Message::WindowResized(size));
            let takes = crate::zoning_badge::plan_takes(app.document.as_ref().expect("a drawing"));
            for (_, take) in takes.into_iter().take(taken) {
                send(&mut app, SettingsEvent::TakePlanValues(take));
            }
            let mut snapshot = Snapshot::new(size).expect("a renderer");
            let mut update = |app: &mut App, message| {
                let _ = app.update(message);
            };
            snapshot.settle(&mut app, App::view, &mut update);
            // To the page's end, then up to what the picture shows, under the pointer.
            let at = iced::Point::new(size.width * 0.55, size.height * 0.5);
            snapshot.input(&mut app, App::view, &mut update, Input::Scroll(at, -60.0));
            snapshot.settle(&mut app, App::view, &mut update);
            snapshot.input(&mut app, App::view, &mut update, Input::Scroll(at, back));
            snapshot.settle(&mut app, App::view, &mut update);
            let file = out.join(format!("{shot}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
            println!("{}", file.display());
        }
    }
}
