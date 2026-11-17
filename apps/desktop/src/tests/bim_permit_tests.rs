//! BIM ruhsat paketi (docs/adr/8075; plan/bim-plan.md BIM-14) in Paftaları toplu yayınla, opened from Ruhsat takımı: the
//! switch adds the IFC model and the model quality control form; Denetle writes the model in memory, reads it back and
//! answers the form; the five results show apart and the questions left to the author take Evet or Hayır with a note.
//! The section building as drawn still lacks mandatory fields, so the package's errors stop the publication; declared as
//! an author does (docs/adr/8076), its package holds and is published.

use crate::app::{App, Message};
use crate::exchange::Window;
use crate::exchange::sheet_publish::Event as Publish;
use crate::permit_package_tests::{check, opened, permit_publication, section_building, section_snapshot};
use kentos_commands::Entity;
use serde_json::{Value, json};

fn run(app: &mut App, name: &str, input: Value) -> Value {
    let doc = &mut app.document.as_mut().expect("a drawing").model;
    let answer = kentos_native_application::registry::run(
        &mut kentos_native_application::ExecutionContext::new(doc),
        name,
        kentos_commands::CallOp::Execute,
        input,
    )
    .expect("runs");
    assert_eq!(answer["status"], "completed", "{name}: {answer}");
    answer
}

fn author(value: &str) -> Value {
    json!({"value": value, "basis": {"kind": "author"}})
}

/// The section building as an author completes it for the permit's model (docs/adr/8076): the künye's digital
/// delivery (EK-1, EK-2, EK-7), the rooms' finishes, the project's system (TUREF / TM36, Antalya heights), the parcel in
/// the Vaziyet planı with ±0.00's elevation, the fire classes and exit roles, the building's fire protection class.
pub(crate) fn declared_building() -> App {
    let mut snap = section_snapshot();
    for (name, value) in [
        ("proje_kodu", "260123"),
        ("bina_kodu", "00"),
        ("revizyon_no", "000"),
        ("tapu_kaydi", "12/345"),
        ("buro_adi", "Kaya Mimarlık"),
        ("vergi_no", "1234567890"),
        ("proje_aciklama", "İki katlı konut"),
        ("posta_kutusu", "Yok"),
        ("yapi_turu", "yeni bina"),
    ] {
        snap.settings.variables.push(kentos_commands::ProjectVariable {
            name: name.into(),
            label: name.into(),
            kind: kentos_commands::VariableKind::Text,
            value: kentos_commands::VariableValue::Text(value.into()),
        });
    }
    for e in &mut snap.entities {
        if let Entity::Space(s) = e {
            s.space.finishes.floor = Some("Seramik".into());
            s.space.finishes.wall = Some("Plastik boya".into());
            s.space.finishes.ceiling = Some("Plastik boya".into());
        }
    }
    snap.settings.srid = 5256;
    snap.settings.heights = Some(kentos_commands::HeightReference {
        kind: kentos_commands::HeightKind::Orthometric,
        vertical_srid: Some(5775),
        geoid: None,
    });
    let mut app = opened(snap);
    let doc = &app.document.as_ref().expect("a drawing").model;
    let (mut lo, mut hi) = ([f64::MAX; 2], [f64::MIN; 2]);
    for e in doc.entities() {
        if let Entity::Wall(w) = e {
            for p in &w.wall.pts {
                lo = [lo[0].min(p.x), lo[1].min(p.y)];
                hi = [hi[0].max(p.x), hi[1].max(p.y)];
            }
        }
    }
    let uids = |app: &App, keep: &dyn Fn(&Entity) -> bool| -> Vec<String> {
        let doc = &app.document.as_ref().expect("a drawing").model;
        doc.entities_with_uids().filter(|(_, e)| keep(e)).map(|(u, _)| u.to_string()).collect()
    };
    let p = |x: f64, y: f64| json!({"x": x, "y": y});
    let made = run(
        &mut app,
        "cad.entities.create",
        json!({"layerId": "0", "objects": [{"geometry": {"kind": "polyline", "closed": true, "pts": [
            p(lo[0] - 3.0, lo[1] - 5.0), p(hi[0] + 3.0, lo[1] - 5.0), p(hi[0] + 3.0, hi[1] + 3.0), p(lo[0] - 3.0, hi[1] + 3.0)
        ]}}]}),
    );
    let parcel = made["output"]["created"][0].as_str().expect("the parcel").to_owned();
    {
        let doc = &mut app.document.as_mut().expect("a drawing").model;
        let mut settings = doc.settings().clone();
        use kentos_commands::{SiteEdge, SiteEdgeKind as K};
        let edge = |kind| SiteEdge { kind, setback: None };
        settings.arch.get_or_insert_with(Default::default).site = Some(kentos_commands::SitePlan {
            parcel: Some(parcel),
            edges: vec![edge(K::Front), edge(K::Side), edge(K::Back), edge(K::Side)],
            datum: Some(1285.4),
            envelope: None,
        });
        doc.set_settings(settings);
    }
    let structure = uids(&app, &|e| matches!(e, Entity::Wall(_) | Entity::Slab(_) | Entity::Column(_) | Entity::Beam(_) | Entity::Stair(_)));
    run(&mut app, "cad.arch.performance", json!({"operation": {"kind": "objects", "uids": structure, "fireRating": author("REI 60")}}));
    let doors = uids(&app, &|e| matches!(e, Entity::Opening(o) if o.opening.form == kentos_commands::OpeningForm::Door));
    run(&mut app, "cad.arch.performance", json!({"operation": {"kind": "objects", "uids": doors, "fireRating": author("EI 30"),
        "exit": {"exit": false, "basis": {"kind": "author"}}}}));
    let stairs = uids(&app, &|e| matches!(e, Entity::Stair(_)));
    run(&mut app, "cad.arch.performance", json!({"operation": {"kind": "objects", "uids": stairs, "exit": {"exit": true, "basis": {"kind": "author"}}}}));
    run(&mut app, "cad.arch.performance", json!({"operation": {"kind": "building", "fireProtectionClass": author("Konut, düşük tehlike")}}));
    app
}

fn publication(app: &App) -> &crate::exchange::sheet_publish::State {
    match &app.exchange {
        Some(Window::SheetPublish(s)) => s,
        _ => panic!("Paftaları toplu yayınla is open"),
    }
}

fn folder(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("kentos-bim-ruhsat-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

#[test]
fn the_bim_package_is_checked_from_its_file_and_waits_for_the_author_s_answers() {
    let mut app = section_building();
    let path = folder("denetim");
    permit_publication(&mut app, &path);
    let _ = app.sheet_publish_event(Publish::Bim(true));
    assert!(publication(&app).input.bim.is_some());
    check(&mut app);
    let s = publication(&app);
    let report = s.report.as_ref().expect("the check's report");
    let bim = report.bim.as_ref().expect("the package's part");
    assert_eq!(bim.rows.len(), 21);
    let open: Vec<u8> = bim.rows.iter().filter(|r| r.evidence == "inceleme").map(|r| r.no).collect();
    assert_eq!(open, [1, 7, 10], "the questions left to the author");
    assert!(!report.ready);
    let unanswered = report.issues.iter().filter(|i| i.code == "digital_form_unanswered").count();
    assert_eq!(unanswered, 3);
    // The form's 20th question holds: the building's quantities are the template's (FootPrintArea, docs/adr/8075).
    assert!(!report.issues.iter().any(|i| i.code == "digital_pset_wrong"), "{:?}", report.issues);
    // The author answers; Denetle again: the questions are no finding, the model's missing fields still are.
    for no in open {
        let _ = app.sheet_publish_event(Publish::BimAnswer(no, Some(true)));
    }
    let _ = app.sheet_publish_event(Publish::BimNote(1, "Bakanlığın sınırı yayımlanmadı".into()));
    assert!(!publication(&app).report.as_ref().is_some_and(|r| r.ready), "an answer asks Denetle again");
    check(&mut app);
    let report = publication(&app).report.as_ref().expect("the check's report");
    assert!(!report.issues.iter().any(|i| i.code == "digital_form_unanswered"));
    let row1 = report.bim.as_ref().expect("the part").rows.iter().find(|r| r.no == 1).expect("row 1");
    assert_eq!((row1.answer.as_str(), row1.note.as_str()), ("Evet", "Bakanlığın sınırı yayımlanmadı (müellif)"));
    assert!(report.issues.iter().any(|i| i.error && i.code == "digital_required_field"));
    assert!(!report.ready, "the model's missing fields stop it");
    let _ = std::fs::remove_dir_all(&path);
}

#[test]
fn a_declared_building_s_package_holds_and_is_published() {
    let mut app = declared_building();
    let path = folder("tam");
    permit_publication(&mut app, &path);
    let _ = app.sheet_publish_event(Publish::Bim(true));
    for no in [1, 7, 10] {
        let _ = app.sheet_publish_event(Publish::BimAnswer(no, Some(true)));
    }
    check(&mut app);
    let report = publication(&app).report.as_ref().expect("the check's report");
    let errors: Vec<(&str, &str)> = report.issues.iter().filter(|i| i.error).map(|i| (i.code.as_str(), i.message.as_str())).collect();
    assert!(report.ready, "{errors:#?}");
    let bim = report.bim.as_ref().expect("the package's part");
    assert_eq!(bim.results.iter().map(|r| r.holds).collect::<Vec<_>>(), [Some(true), Some(true), Some(true), Some(true), None]);
    let _ = std::fs::remove_dir_all(&path);
}

/// The window's BIM part, light and dark: the switch, the five results and the author's questions after Denetle;
/// then the declared building's package, every result holding; `.run/gui-evidence/bim-ruhsat-paketi-8075/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib bim_permit_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::{Point, Size};
    use kentos_ui::snapshot::{Input, Snapshot};
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/bim-ruhsat-paketi-8075");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name) in [("light", Size::new(1500.0, 1100.0), "acik"), ("dark", Size::new(1300.0, 1000.0), "koyu")] {
        let mut app = section_building();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let path = folder(&format!("resim-{name}"));
        permit_publication(&mut app, &path);
        let _ = app.sheet_publish_event(Publish::Bim(true));
        check(&mut app);
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        let middle = Point::new(size.width / 2.0, size.height / 2.0);
        // The window scrolled to its end, where the package's part is.
        snapshot.input(&mut app, App::view, &mut update, Input::Scroll(middle, -60.0));
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("1-bim-paketi-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
        for no in [1, 7, 10] {
            let _ = app.sheet_publish_event(Publish::BimAnswer(no, Some(true)));
        }
        let _ = app.sheet_publish_event(Publish::BimNote(1, "Bakanlığın sınırı yayımlanmadı".into()));
        check(&mut app);
        snapshot.input(&mut app, App::view, &mut update, Input::Scroll(middle, -60.0));
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("2-cevaplar-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
        let _ = std::fs::remove_dir_all(&path);
        // The declared building: the model carries every mandatory field, the package holds (docs/adr/8076).
        let mut app = declared_building();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let path = folder(&format!("tam-{name}"));
        permit_publication(&mut app, &path);
        let _ = app.sheet_publish_event(Publish::Bim(true));
        for no in [1, 7, 10] {
            let _ = app.sheet_publish_event(Publish::BimAnswer(no, Some(true)));
        }
        let _ = app.sheet_publish_event(Publish::BimNote(1, "Bakanlığın sınırı yayımlanmadı".into()));
        check(&mut app);
        snapshot.input(&mut app, App::view, &mut update, Input::Scroll(middle, -60.0));
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("3-gecen-paket-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
        let _ = std::fs::remove_dir_all(&path);
    }
}

/// Ruhsat takımı's Dijital teslim group with the künye's postal box (docs/adr/8076 §5), light and dark;
/// `.run/gui-evidence/ifc-zorunlu-8076/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib bim_permit_tests::kunye_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn kunye_screens() {
    use iced::{Point, Size};
    use kentos_ui::snapshot::{Input, Snapshot};
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/ifc-zorunlu-8076");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name) in [("light", Size::new(1500.0, 1000.0), "acik"), ("dark", Size::new(1300.0, 900.0), "koyu")] {
        let mut app = declared_building();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let _ = app.update(Message::Run("arch.permitSet"));
        assert!(matches!(app.exchange, Some(Window::PermitSet(_))), "Ruhsat takımı is open");
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        let middle = Point::new(size.width / 2.0, size.height / 2.0);
        // Down to the Dijital teslim group, past the land, the building and the author.
        snapshot.input(&mut app, App::view, &mut update, Input::Scroll(middle, -12.0));
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("1-kunye-posta-kutusu-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
    }
}
