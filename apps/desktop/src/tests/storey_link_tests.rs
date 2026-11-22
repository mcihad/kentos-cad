//! Katlar arası açıklıklar (docs/adr/8086; Binaların Yangından Korunması Hakkında Yönetmelik 4/g, 24/4, 24/5, 26/1) on a
//! two-storey office worked by hand: Zemin kat ±0.00 and 1. kat +3.00, each a 20 × 10 m plan inside 20 cm walls holding
//! one room, Ofis, drawn to the walls' faces; the building used as a büro. With no floor slab drawn on 1. kat the two
//! storeys are two compartments; 1. kat's floor slab with a 4 × 4 m hole (a gallery) joins them into one and the hole is
//! BYKHY 4/g's atrium (its smoke control unseen: expert review); a stair from Zemin kat arriving through the hole makes it
//! a stair well, no atrium, the storeys still one compartment.
//!
//! ```text
//! RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib storey_link_tests::screens -- --ignored --nocapture
//! ```

use kentos_commands::{ArchAuditOutcome as O, ArchAuditState as S};
use kentos_launch::Module;
use serde_json::{Value, json};

use crate::app::{App, Message};
use crate::files_testing::drive;
use crate::recovery::Recovery;
use crate::settings::Settings;

fn run(app: &mut App, name: &str, input: Value) -> Value {
    let doc = app.document.as_mut().expect("a drawing");
    let answer = kentos_native_application::registry::run(
        &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
        name,
        kentos_commands::CallOp::Execute,
        input,
    )
    .expect("runs");
    assert_eq!(answer["status"], "completed", "{name}: {answer}");
    answer
}

fn p(x: f64, y: f64) -> Value {
    json!({"x": x, "y": y})
}

/// The office's two storeys, no floor slab yet; applied for on 2.7.2026.
fn office() -> App {
    kentos_postgis::connections::keep_in_memory();
    let (mut app, _) = App::start_in(Some(Module::Architecture), None, Settings::memory(), Recovery::off());
    app.db_store = Box::leak(Box::new(kentos_postgis::connections::ConnectionStore::open(None)));
    app.hand_over = |m, _| Ok(m.program().to_owned());
    let task = app.update(Message::Run("file.new"));
    drive(&mut app, task);
    run(
        &mut app,
        "cad.settings.set",
        json!({ "arch": { "levels": [
            {"id": "k00", "name": "Zemin kat", "elevation": 0.0, "height": 3.0},
            {"id": "k01", "name": "1. kat", "elevation": 3.0, "height": 3.0}
        ] } }),
    );
    run(
        &mut app,
        "cad.program.edit",
        json!({"program": {
            "buildings": [{"id": "a", "name": "A Blok", "levels": ["k00", "k01"], "usage": "Büro"}],
            "units": [{"id": "b1", "number": "1", "nature": "Büro"}]
        }}),
    );
    let layer = app.document.as_ref().expect("a drawing").model.layers().active().to_owned();
    for level in ["k00", "k01"] {
        let wall = |a: (f64, f64), b: (f64, f64)| json!({"pts": [p(a.0, a.1), p(b.0, b.1)], "width": 0.2, "arch": {"level": level}});
        run(
            &mut app,
            "cad.walls.create",
            json!({"layerId": layer, "walls": [
                wall((0.0, 0.0), (20.0, 0.0)), wall((20.0, 0.0), (20.0, 10.0)), wall((20.0, 10.0), (0.0, 10.0)),
                wall((0.0, 10.0), (0.0, 0.0))
            ]}),
        );
        let rect = vec![p(0.1, 0.1), p(19.9, 0.1), p(19.9, 9.9), p(0.1, 9.9)];
        run(
            &mut app,
            "cad.spaces.create",
            json!({"layerId": layer, "spaces": [{"parts": [{"outer": {"pts": rect}}], "name": "Ofis", "unit": "b1", "arch": {"level": level}}]}),
        );
    }
    crate::regulation_dates_tests::dated(&mut app, Some("02.07.2026"));
    app
}

/// 1. kat's floor slab, a 4 × 4 m hole in its middle.
fn gallery(app: &mut App) {
    let layer = app.document.as_ref().expect("a drawing").model.layers().active().to_owned();
    run(
        app,
        "cad.slabs.create",
        json!({"layerId": layer, "slabs": [{
            "parts": [{"outer": {"pts": [p(0.0, 0.0), p(20.0, 0.0), p(20.0, 10.0), p(0.0, 10.0)]},
                       "holes": [{"pts": [p(8.0, 3.0), p(12.0, 3.0), p(12.0, 7.0), p(8.0, 7.0)]}]}],
            "thickness": 0.15, "arch": {"level": "k01"}
        }]}),
    );
}

/// A stair on Zemin kat from (10, 1.4) up to (10, 6.2), arriving through the hole.
fn stair(app: &mut App) {
    let layer = app.document.as_ref().expect("a drawing").model.layers().active().to_owned();
    run(
        app,
        "cad.stairs.create",
        json!({"layerId": layer, "stairs": [{"pts": [p(10.0, 1.4), p(10.0, 6.2)], "width": 1.2, "risers": 18,
            "riserHeight": 0.1667, "tread": 0.28, "railings": "both", "arch": {"level": "k00"}}]}),
    );
}

/// The whole drawing checked in Denetim, the standards packages asked and answered.
fn checked(app: &mut App) {
    let _ = app.update(Message::Run("arch.audit"));
    app.arch_audit_standards_settle();
    assert_eq!(app.arch.arch_audit.regulation_failed, None, "the national regulation's rule answers");
}

/// The atriums' findings (state, message) and the rows of 24/5 and 24/6 (outcome, note).
#[allow(clippy::type_complexity)]
fn said(app: &App) -> (Vec<(Option<S>, String)>, Vec<(String, O, String)>) {
    let audited = app.arch.arch_audit.audited.as_ref().expect("checked");
    let found = audited.findings.iter().filter(|f| f.code == "bykhy_atrium").map(|f| (f.state, f.message.clone())).collect();
    let set = app.arch.arch_audit.rule_set.as_ref().expect("the rule set");
    let rows = set
        .rules
        .iter()
        .filter(|r| r.rule == "BYKHY 24/5" || r.rule == "BYKHY 24/6")
        .map(|r| (r.rule.clone(), r.state, r.note.clone()))
        .collect();
    (found, rows)
}

#[test]
fn a_gallery_joins_two_storeys_into_one_compartment_and_is_an_atrium_but_a_stair_well_is_not() {
    let mut app = office();
    checked(&mut app);
    let (found, rows) = said(&app);
    assert!(found.is_empty(), "{found:#?}");
    assert_eq!(rows[0].1, O::OutOfScope, "{rows:#?}");
    assert!(rows[1].2.starts_with("2 kompartıman"), "each storey its own: {rows:#?}");
    // The floor slab with its gallery: one link, the two offices.
    gallery(&mut app);
    let doc = &app.document.as_ref().expect("a drawing").model;
    let facts = kentos_processing::audit_facts::building::measure(doc).normalized();
    let ofis = |level: &str| facts.spaces.iter().find(|s| s.level.as_deref() == Some(level)).expect("Ofis").uid.clone();
    assert_eq!(facts.links.len(), 1, "{:#?}", facts.links);
    let l = &facts.links[0];
    assert_eq!((l.kind.as_str(), l.below.as_deref(), l.above.as_deref()), ("hole", Some("k00"), Some("k01")));
    assert_eq!((l.rooms_below.clone(), l.rooms_above.clone()), (vec![ofis("k00")], vec![ofis("k01")]));
    assert!((l.area - 16.0).abs() < 1e-9 && !l.stair_well, "{l:#?}");
    checked(&mut app);
    let (found, rows) = said(&app);
    assert_eq!(found.len(), 1, "{found:#?}");
    assert_eq!(found[0].0, Some(S::ExpertReview));
    assert!(
        found[0].1.starts_with("Katlar arası boşluk (2 kat, en dar yerinde 16.00 m²) BYKHY 4/g'nin atriumu sayılır"),
        "{found:#?}"
    );
    assert_eq!(rows[0].1, O::ExpertReview, "{rows:#?}");
    assert!(rows[1].2.starts_with("1 kompartıman"), "the gallery joins them: {rows:#?}");
    // A stair arriving through the hole: a stair well, no atrium; the storeys still joined.
    stair(&mut app);
    let doc = &app.document.as_ref().expect("a drawing").model;
    let facts = kentos_processing::audit_facts::building::measure(doc).normalized();
    let kinds: Vec<(&str, bool)> = facts.links.iter().map(|l| (l.kind.as_str(), l.stair_well)).collect();
    assert_eq!(kinds.len(), 2, "{:#?}", facts.links);
    assert!(kinds.contains(&("hole", true)) && kinds.contains(&("stair", false)), "{kinds:?}");
    checked(&mut app);
    let (found, rows) = said(&app);
    assert!(found.is_empty(), "{found:#?}");
    assert_eq!(rows[0].1, O::OutOfScope, "{rows:#?}");
    assert!(rows[1].2.starts_with("1 kompartıman"), "{rows:#?}");
}

/// 1. kat with its gallery and Denetim with the atrium's finding, light and dark; `.run/gui-evidence/atrium-8086/`.
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/atrium-8086");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik"), ("dark", Size::new(1280.0, 800.0), "koyu")] {
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        let mut app = office();
        gallery(&mut app);
        let _ = app.update(Message::ActiveLevel(Some("k01".into())));
        let _ = app.update(Message::LevelView(kentos_interaction::level_view::LevelView::Active));
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        checked(&mut app);
        let audited = app.arch.arch_audit.audited.as_ref().expect("checked");
        app.arch.arch_audit.selected = audited.findings.iter().position(|f| f.code == "bykhy_atrium");
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        snapshot.settle(&mut app, App::view, &mut update);
        app.selection.set(Vec::new());
        let _ = app.update(Message::Run("view.zoomExtents"));
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("1-atrium-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
    }
}
