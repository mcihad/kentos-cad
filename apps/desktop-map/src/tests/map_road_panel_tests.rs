//! Yol projesi in a Harita project (road_project_panel.rs, plan/road.md Y13.3): its tree shows the zero line stale and
//! draws it again. Harita's profile docks the panel, so this runs with the module registered (docs/adr/9199 §4.2).

use crate::app::{App, Message, Panel};
use crate::road_project_panel::Event as P;
use kentos_commands::RoadNodeState as S;
use serde_json::{Value, json};

fn send(app: &mut App, e: P) {
    let _ = app.update(Message::TerrainWork(
        crate::terrain_work::Event::RoadProject(e),
    ));
}

/// A rebuild, run as a job (docs/adr/2440): the panel looked at until it is over.
fn rebuild(app: &mut App, e: P) {
    send(app, e);
    let start = std::time::Instant::now();
    while app.terrain_work.road_project.running() {
        assert!(
            start.elapsed() < std::time::Duration::from_secs(120),
            "the rebuild ends"
        );
        std::thread::sleep(std::time::Duration::from_millis(5));
        send(app, P::Tick);
    }
}

fn tree(app: &App) -> kentos_commands::RoadProjectTree {
    app.road_project_read()
        .expect("a drawing")
        .expect("a road project")
}

/// A KentOS Harita project (docs/adr/1720): road_terrain_zeroline_cases.py's map project, the zero line of Tümsek over
/// Düz drawn, then one of Tümsek's heights set (an edit of its surface): Proje ağacı shows the zero line stale under
/// its base, Hepsini yeniden üret draws it again, and the drawing saves (`.kcad` schema 198).
fn map_app() -> App {
    let (mut app, _) = App::boot(None);
    app.program = Some(kentos_launch::Module::Map);
    let cases: Value = serde_json::from_str(include_str!(
        "../../../../fixtures/commands/v1/road.terrains.zeroline.json"
    ))
    .expect("cases");
    let case = cases["cases"]
        .as_array()
        .expect("cases")
        .iter()
        .find(|c| c["setup"]["settings"]["cadWork"] == "map")
        .expect("a map case");
    let snapshot: kentos_commands::DocumentSnapshotV2 =
        serde_json::from_value(case["setup"].clone()).expect("the drawing reads");
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    crate::files_testing::command(
        &mut app,
        "road.terrains.zeroline",
        json!({"base": "Düz", "comparison": "Tümsek", "layerId": "sifir"}),
    );
    let surface = json!({"id": "t-10", "name": "Tümsek", "points": {"layers": ["tumsek"]},
                         "edits": [{"op": "setHeight", "x": 6000.0, "y": 0.0, "z": 119.0}]});
    crate::files_testing::command(
        &mut app,
        "road.terrains.edit",
        json!({"operation": "set", "surface": surface}),
    );
    app
}

#[test]
fn a_map_project_s_tree_shows_its_zero_line_stale_and_draws_it_again() {
    let mut app = map_app();
    assert_eq!(app.panel_title(Panel::RoadProject), "Proje ağacı");
    assert!(app.shows_panel(Panel::RoadProject), "Harita has the tree");
    let t = tree(&app);
    let zero = t
        .nodes
        .iter()
        .find(|n| n.kind == kentos_commands::RoadNodeKind::Drawing)
        .expect("the zero line");
    assert_eq!(zero.state, S::Stale, "{zero:?}");
    assert_eq!(zero.why, ["“Tümsek” yüzeyi çizildiğinden beri değişti."]);
    rebuild(&mut app, P::Rebuild(None, false));
    let t = tree(&app);
    assert!(
        t.nodes
            .iter()
            .filter(|n| n.kind == kentos_commands::RoadNodeKind::Drawing)
            .all(|d| d.state == S::Current)
    );
    let doc = app.document.as_ref().expect("a drawing");
    let bytes = kentos_kcad::encode(&doc.model.to_snapshot_v2());
    assert!(
        bytes.is_ok(),
        "a map project with its drawings saves: {:?}",
        bytes.err()
    );
}
