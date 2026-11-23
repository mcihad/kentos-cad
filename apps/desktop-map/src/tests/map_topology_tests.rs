//! One object's topology findings in Harita's Öznitelikler (topology/object.rs, docs/adr/3800): the selected parcel's
//! findings, and a finding marked from there. Harita's profile shows the section, so these run with the module
//! registered (docs/adr/9199 §4.2); the rest are the core's (gis/topology/object_tests.rs).

use crate::gis;
use kentos_domain::Slot;
use kentos_launch::Module;
use serde_json::Value;
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
#[allow(clippy::type_complexity)]
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
    assert_eq!(
        (title.as_str(), rows[0].1.as_str()),
        ("Topoloji", "Denetlenmedi: Topoloji sekmesinde Denetle.")
    );
    let _ = app.update(Message::Gis(gis::Event::Topology(T::Check)));
    let (title, rows) = section(&app).expect("a section");
    assert_eq!(title, "Topoloji (1)");
    let ObjectFindings::Checked { findings, old } = app.object_findings(Slot(1)).expect("findings")
    else {
        panic!("checked")
    };
    assert!(!old);
    let f = &findings[0];
    // Gerekçe first (docs/adr/4600), then the finding: its state in the cell, its measure and rule in the quiet line.
    assert_eq!((rows[0].0.as_str(), rows[0].1.as_str()), ("Gerekçe", ""));
    assert_eq!((rows[2].0.as_str(), rows[2].1.as_str()), (f.problem.as_str(), "Değerlendirilmedi"));
    assert_eq!(rows[3], (String::new(), format!("{} · kural: {}", f.measure, f.rule), true));
    assert_eq!(f.state, "Değerlendirilmedi");
    // The check's own key: Göster opens the tab on that finding, its objects chosen.
    let c = app.topology_checked().expect("a check");
    let at = c
        .findings
        .iter()
        .position(|x| c.key(x) == f.key)
        .expect("the finding");
    let _ = app.update(Message::Gis(gis::Event::Topology(T::Show(f.key.clone()))));
    assert_eq!(
        (app.bottom_tab, app.command_expanded),
        (BottomTab::Topology, true)
    );
    assert_eq!(app.gis.topology.selected, vec![at]);
    let mut chosen = app.selection.ids().to_vec();
    chosen.sort_by_key(|s| s.0);
    assert_eq!(
        chosen,
        vec![Slot(1), Slot(2)],
        "A1 and A2, the overlap's objects"
    );
    // A5: its short edge.
    app.selection.set([Slot(5)]);
    let ObjectFindings::Checked { findings, .. } = app.object_findings(Slot(5)).expect("findings") else { panic!() };
    assert_eq!(findings.len(), 1);
    assert_eq!(section(&app).expect("a section").1[2].0, findings[0].problem);
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

/// Docs/adr/4600: A1's finding marked from Öznitelikler: its state's drop-down offers the three marks; with Gerekçe typed
/// there İstisna writes the exception with those words, then Hata, then Değerlendirilmedi again; the tab's chosen rows
/// stay. A1, A2 and A5 selected: two findings (the overlap counted once, the short edge), both not reviewed.
#[test]
fn a_finding_is_marked_from_oznitelikler() {
    use kentos_commands::{TopologyExceptionOperation as Op, TopologyFindingState as State};
    let mut app = opened(Module::Map);
    let _ = app.update(Message::Gis(gis::Event::Topology(T::Check)));
    app.selection.set([Slot(1)]);
    let first = |app: &App| {
        let ObjectFindings::Checked { findings, .. } =
            app.object_findings(Slot(1)).expect("findings")
        else {
            panic!("checked")
        };
        findings[0].clone()
    };
    let key = first(&app).key;
    {
        let doc = app.document.as_ref().expect("a drawing");
        let panel = app.properties_panel(doc);
        let section = panel.sections.iter().find(|s| s.id == "topology").expect("the section");
        let Some(crate::properties::Editor::Select { text, items, .. }) = &section.rows[2].editor else { panic!("a drop-down") };
        let labels: Vec<String> = items
            .iter()
            .filter_map(|c| match c {
                crate::properties::Choice::Pick { label, chosen, .. } => Some(format!("{label}{}", if *chosen { " ✓" } else { "" })),
                _ => None,
            })
            .collect();
        assert_eq!((text.as_str(), labels), ("Değerlendirilmedi", vec!["Değerlendirilmedi ✓".into(), "Hata".into(), "İstisna".into()]));
    }
    let _ = app.update(Message::Properties(crate::properties::Event::Commit(
        crate::properties::Field::TopologyReason,
        " Sınır bilerek ortak ".into(),
    )));
    assert_eq!(app.gis.topology.reason, "Sınır bilerek ortak");
    let chosen = app.gis.topology.selected.clone();
    let _ = app.update(Message::Gis(gis::Event::Topology(T::Review(
        key.clone(),
        Op::Mark,
    ))));
    let f = first(&app);
    assert_eq!(
        (f.status, f.reason.as_deref()),
        (State::Exception, Some("Sınır bilerek ortak"))
    );
    let (_, rows) = section(&app).expect("a section");
    assert_eq!(rows[2].1, "İstisna");
    assert_eq!(
        rows[3],
        (String::new(), "Gerekçe: Sınır bilerek ortak".into(), true)
    );
    assert_eq!(
        app.gis.topology.selected, chosen,
        "the tab's chosen rows stay"
    );
    let _ = app.update(Message::Gis(gis::Event::Topology(T::Review(
        key.clone(),
        Op::Error,
    ))));
    assert_eq!(first(&app).status, State::Error);
    let _ = app.update(Message::Gis(gis::Event::Topology(T::Review(
        key,
        Op::Unmark,
    ))));
    assert_eq!(first(&app).status, State::Open);
    // Several objects: their findings counted by state.
    app.selection.set([Slot(1), Slot(2), Slot(5)]);
    let summary = app.selection_findings(&[Slot(1), Slot(2), Slot(5)]).expect("named").expect("checked");
    assert_eq!((summary.objects, summary.total(), summary.open), (3, 2, 2));
    let (title, rows) = section(&app).expect("a section");
    let said: Vec<(&str, &str)> = rows.iter().map(|r| (r.0.as_str(), r.1.as_str())).collect();
    assert_eq!(title, "Topoloji");
    assert_eq!(
        said,
        [("Bulgular", "2"), ("Değerlendirilmedi", "2"), ("", "Denetlenen 3 nesnenin; ortak bulgu bir kez sayılır.")]
    );
}
