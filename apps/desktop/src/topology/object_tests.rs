//! One object's topology findings (topology/object.rs, docs/adr/3800) on the shared cases' drawing
//! (`fixtures/commands/v1/cad.topology.check.json`: five parcels, A1–A2 and A3–A4 overlap, A5 has a 0.5 m edge): in
//! Harita's Öznitelikler, A1 says it was not checked, then after Denetle its one finding (the check's own words and
//! key); the finding's button opens the Topoloji tab on it; a change to the drawing is said; A5's short edge; GIS shows
//! the section, Yol does not. And Formla düzenle's lines on a building with short edges. And the pictures.

use kentos_domain::Slot;
use kentos_launch::Module;
use serde_json::{Value, json};

use crate::app::{App, Message};
use crate::bottom::BottomTab;
use crate::topology::Event as T;
use crate::topology::object::ObjectFindings;

fn opened(module: Module) -> App {
    let cases: Value =
        serde_json::from_str(include_str!("../../../../fixtures/commands/v1/cad.topology.check.json")).expect("cases");
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases["setup"].clone()).expect("reads");
    let (mut app, _) = App::boot(None);
    app.program = Some(module);
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

/// The panel's Topoloji section: its title and its rows (label, value, a note).
fn section(app: &App) -> Option<(String, Vec<(String, String, bool)>)> {
    let doc = app.document.as_ref()?;
    let panel = app.properties_panel(doc);
    panel.sections.iter().find(|s| s.id == "topology").map(|s| {
        (s.title.to_string(), s.rows.iter().map(|r| (r.label.to_string(), r.value.clone(), r.note)).collect())
    })
}

#[test]
fn oznitelikler_says_the_selected_parcel_s_findings() {
    let mut app = opened(Module::Map);
    app.selection.set([Slot(1)]);
    let (title, rows) = section(&app).expect("a section: rules name the layer");
    assert_eq!((title.as_str(), rows[0].1.as_str()), ("Topoloji", "Denetlenmedi: Topoloji sekmesinde Denetle."));
    let _ = app.update(Message::Topology(T::Check));
    let (title, rows) = section(&app).expect("a section");
    assert_eq!(title, "Topoloji (1)");
    let ObjectFindings::Checked { findings, old } = app.object_findings(Slot(1)).expect("findings") else {
        panic!("checked")
    };
    assert!(!old);
    let f = &findings[0];
    assert_eq!((rows[0].0.as_str(), rows[0].1.as_str()), (f.problem.as_str(), f.measure.as_str()));
    assert_eq!(rows[1], (String::new(), format!("{} · kural: {}", f.state, f.rule), true));
    assert_eq!(f.state, "Değerlendirilmedi");
    // The check's own key: Göster opens the tab on that finding, its objects chosen.
    let c = app.topology_checked().expect("a check");
    let at = c.findings.iter().position(|x| c.key(x) == f.key).expect("the finding");
    let _ = app.update(Message::Topology(T::Show(f.key.clone())));
    assert_eq!((app.bottom_tab, app.command_expanded), (BottomTab::Topology, true));
    assert_eq!(app.topology.selected, vec![at]);
    let mut chosen = app.selection.ids().to_vec();
    chosen.sort_by_key(|s| s.0);
    assert_eq!(chosen, vec![Slot(1), Slot(2)], "A1 and A2, the overlap's objects");
    // A5: its short edge.
    app.selection.set([Slot(5)]);
    let ObjectFindings::Checked { findings, .. } = app.object_findings(Slot(5)).expect("findings") else { panic!() };
    assert_eq!(findings.len(), 1);
    assert_eq!(section(&app).expect("a section").1[0].0, findings[0].problem);
    // A change since the check is said: A2 moved half a metre east.
    let doc = app.document.as_mut().expect("a drawing");
    let mut moved = doc.model.get(Slot(2)).expect("A2").clone();
    if let kentos_commands::Entity::Polygon(p) = &mut moved {
        for q in &mut p.pts {
            q.x += 0.5;
        }
    }
    doc.model.update_many(vec![(Slot(2), moved)], "Taşı");
    app.selection.set([Slot(1)]);
    let (_, rows) = section(&app).expect("a section");
    assert!(rows.iter().any(|r| r.1.starts_with("Çizim denetimden sonra değişti")), "{rows:?}");
}

#[test]
fn gis_shows_the_section_yol_does_not() {
    let mut gis = opened(Module::Gis);
    gis.selection.set([Slot(1)]);
    assert!(section(&gis).is_some());
    let mut road = opened(Module::Road);
    road.selection.set([Slot(1)]);
    assert!(section(&road).is_none());
}

/// Formla düzenle (forms/entry.rs) on a building whose two 14 m sides are short of a 15 m rule: its lines say the
/// findings after Denetle, none before.
#[test]
fn formla_duzenle_says_the_record_s_findings() {
    let mut app = form_on_short_edges();
    let uid = app.document.as_ref().and_then(|d| d.model.uid(Slot(1))).expect("a building").to_string();
    assert!(app.entry_topology_lines(&uid).is_empty(), "not checked yet");
    let _ = app.update(Message::Topology(T::Check));
    let lines: Vec<String> = app.entry_topology_lines(&uid).into_iter().map(|(_, t)| t).collect();
    let ObjectFindings::Checked { findings, .. } = app.object_findings(Slot(1)).expect("findings") else { panic!() };
    assert_eq!(lines[0], format!("Topoloji: son denetimde {} bulgu.", findings.len()));
    assert_eq!(lines.len(), findings.len() + 1);
    assert!(lines[1].starts_with(&findings[0].problem) && lines[1].contains(&findings[0].rule), "{lines:?}");
    assert!(app.form_entry.is_some(), "the form is open");
}

/// The four buildings and their layer's form (form_scenes.rs) in KentOS CBS, a short-edge rule of 15 m on their layer,
/// the first building's form open.
fn form_on_short_edges() -> App {
    let (mut app, _) = App::boot(None);
    app.program = Some(Module::Gis);
    crate::form_scenes::buildings(&mut app, true);
    let doc = app.document.as_mut().expect("a drawing");
    let mut s = doc.model.settings().clone();
    s.topology = Some(
        serde_json::from_value(json!({"tolerance": 0.001, "rules": [{"id": "k1", "kind": "mustNotHaveShortEdges", "layer": "bina", "value": 15.0}]}))
            .expect("rules"),
    );
    doc.model.set_settings(s);
    app.selection.set([Slot(1)]);
    let _ = app.update(Message::Run("data.formEdit"));
    app
}

/// Öznitelikler on A1 after Denetle (light wide, dark narrow) and Formla düzenle on a building with short edges,
/// `.run/gui-evidence/topoloji-nesne-3800/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib topology::object_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/topoloji-nesne-3800");
    std::fs::create_dir_all(&out).expect("a folder");
    let mut update = |app: &mut App, m: Message| {
        let _ = app.update(m);
    };
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 700.0), "koyu-dar")] {
        let mut app = opened(Module::Map);
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        snapshot.settle(&mut app, App::view, &mut update);
        let _ = app.update(Message::Topology(T::Check));
        app.selection.set([Slot(1)]);
        let _ = app.update(Message::Run("view.zoomExtents"));
        // Genel and Geometri closed on their headers: Topoloji in sight under them.
        app.props_closed.insert("general");
        app.props_closed.insert("geometry");
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("oznitelikler-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
    }
    let mut app = form_on_short_edges();
    let size = Size::new(1440.0, 900.0);
    let _ = app.update(Message::WindowResized(size));
    let _ = app.update(Message::Topology(T::Check));
    let mut snapshot = Snapshot::new(size).expect("a renderer");
    snapshot.settle(&mut app, App::view, &mut update);
    let file = out.join("formla-duzenle-acik-genis.png");
    snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
    println!("{}", file.display());
}
