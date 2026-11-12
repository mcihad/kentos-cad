//! The pictures of docs/adr/0326: a plan's three pipes noted by one leader of three arms whose tips are bound to the
//! pipes' ends, two door balloons collected on one leader, the noted leader selected with Öznitelikler's leader rows
//! (kilavuz-kollar); three notes chosen with Hizala's window (kilavuz-hizala); Kılavuz stilleri (kilavuz-stilleri).
//! `tools_screens` takes them in both themes at 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=kilavuz-kollar,kilavuz-hizala,kilavuz-stilleri cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use kentos_domain::Slot;
use serde_json::{Value, json};

use crate::app::App;
use crate::files_testing::make_cad;
use crate::tools_scenes::{E, N, Objects, hover, layer, open_edited, run};
use crate::tools_screens::Scene;

const BALLOON: &str = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3b01";

fn command(app: &mut App, name: &str, input: Value) {
    let doc = app.document.as_mut().expect("a drawing");
    let answer = kentos_native_application::registry::run(
        &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
        name,
        kentos_commands::CallOp::Execute,
        input,
    )
    .expect("runs");
    assert_eq!(answer["status"], "completed", "{name}: {answer}");
}

fn uid(app: &App, slot: u32) -> String {
    app.document
        .as_ref()
        .and_then(|d| d.model.uid(Slot(slot)))
        .map(|u| u.to_string())
        .expect("an object")
}

struct Plan {
    pipe_note: u32,
    notes: [u32; 3],
}

fn drawing(app: &mut App) -> Plan {
    let mut o = Objects::new();
    let pipe = |o: &mut Objects, y: f64| {
        o.push("boru", json!({ "kind": "line", "a": { "x": E, "y": N + y }, "b": { "x": E + 8.0, "y": N + y } }))
    };
    let pipes = [pipe(&mut o, 0.0), pipe(&mut o, 1.2), pipe(&mut o, 2.4)];
    let pipe_note = o.push(
        "not",
        json!({ "kind": "leader", "pts": [{ "x": E + 8.0, "y": N + 2.4 }, { "x": E + 10.0, "y": N + 4.5 }],
                "text": "Ø150 PVC\nTS EN 1401", "height": 0.3, "rotation": 0.0 }),
    );
    let door = |o: &mut Objects, x: f64, no: &str| {
        o.push(
            "not",
            json!({ "kind": "insert", "block": BALLOON, "p": { "x": E + x, "y": N - 3.0 }, "scale": 1.0, "rotation": 0.0, "attrs": { "NO": no } }),
        )
    };
    let balloons = [door(&mut o, 0.0, "K-01"), door(&mut o, 2.0, "K-02")];
    let door_leader = |o: &mut Objects, x: f64| {
        o.push(
            "not",
            json!({ "kind": "leader", "pts": [{ "x": E + x, "y": N - 1.5 }, { "x": E + x + 1.5, "y": N - 3.5 }], "height": 0.3, "rotation": 0.0 }),
        )
    };
    let doors = [door_leader(&mut o, 1.0), door_leader(&mut o, 3.5)];
    let note = |o: &mut Objects, x: f64, y: f64, words: &str| {
        o.push(
            "not",
            json!({ "kind": "leader", "pts": [{ "x": E + x, "y": N + y }, { "x": E + x + 1.5, "y": N + y + 1.0 + x / 10.0 }],
                    "text": words, "height": 0.3, "rotation": 0.0 }),
        )
    };
    let notes = [
        note(&mut o, 14.0, 0.0, "Rögar R1"),
        note(&mut o, 17.0, 0.6, "Rögar R2"),
        note(&mut o, 20.0, -0.4, "Rögar R3"),
    ];
    open_edited(app, o, |d| {
        d["layers"] = json!([
            layer("boru", "Borular", "#3E63DD", 0.5),
            layer("not", "Notlar", "#E5484D", 0.25)
        ]);
        d["activeLayer"] = json!("not");
        d["settings"]["plotScale"] = json!(50);
        d["blocks"] = json!([{ "id": BALLOON, "name": "Balon", "base": { "x": 0, "y": 0 },
            "entities": [{ "kind": "circle", "id": 1, "layerId": "", "attrs": {}, "c": { "x": 0, "y": 0 }, "r": 0.45 }],
            "attributes": [{ "tag": "NO", "value": "K-?", "p": { "x": 0, "y": 0 }, "height": 0.25, "rotation": 0, "align": "middleCenter" }] }]);
    });
    make_cad(app);
    // Two more arms to the other pipes' ends, each tip bound to its end; the first arm's tip too.
    let end = |app: &App, pipe: u32| json!({ "uid": uid(app, pipe), "place": { "kind": "end" } });
    for (i, p) in pipes.iter().enumerate().take(2) {
        let y = 1.2 * i as f64;
        let input = json!({ "edit": { "kind": "addArm", "uid": uid(app, pipe_note), "pts": [{ "x": E + 8.0, "y": N + y }], "tip": end(app, *p) } });
        command(app, "cad.leaders.edit", input);
    }
    let input = json!({ "edit": { "kind": "bind", "uid": uid(app, pipe_note), "arm": 0, "tip": end(app, pipes[2]) } });
    command(app, "cad.leaders.edit", input);
    // The balloons' numbers.
    for (b, no) in balloons.iter().zip(["K-01", "K-02"]) {
        command(
            app,
            "cad.entities.set",
            json!({ "uids": [uid(app, *b)], "attrs": { "NO": no }, "operation": "attributes" }),
        );
    }
    // The door balloons on their leaders, then collected on the first.
    for (l, b) in doors.iter().zip(balloons) {
        command(
            app,
            "cad.leaders.edit",
            json!({ "edit": { "kind": "blocks", "uid": uid(app, *l), "blocks": [uid(app, b)] } }),
        );
    }
    command(
        app,
        "cad.leaders.edit",
        json!({ "edit": { "kind": "collect", "uids": [uid(app, doors[0]), uid(app, doors[1])] } }),
    );
    // The pipes grow a metre: the arms' tips follow their ends.
    let ids: Vec<String> = pipes.iter().map(|p| uid(app, *p)).collect();
    command(
        app,
        "cad.entities.transform",
        json!({ "uids": ids, "transform": { "kind": "scale", "center": { "x": E, "y": N }, "factor": 1.125 } }),
    );
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: E - 1.5,
            min_y: N - 6.0,
            max_x: E + 25.0,
            max_y: N + 7.0,
        },
        24.0,
    );
    Plan { pipe_note, notes }
}

fn arms(app: &mut App) {
    let plan = drawing(app);
    app.tab = "annotate";
    app.selection.set([Slot(plan.pipe_note)]);
    let upper =
        kentos_ui::widget::docking::Slot::Docked(kentos_ui::widget::docking::Side::Right, 0);
    let _ = app.update(crate::app::Message::Dock(
        kentos_ui::widget::docking::Event::Collapsed(upper, true),
    ));
    let _ = app.update(crate::app::Message::Properties(
        crate::properties::Event::Toggle("general"),
    ));
    hover(app, [30.0, 1.0]);
}

fn align(app: &mut App) {
    let plan = drawing(app);
    app.tab = "annotate";
    app.selection.set(plan.notes.map(Slot));
    run(app, "leader.align");
    let _ = app.update(crate::app::Message::LeaderTools(
        crate::leader_tools::Event::Spacing("3".into()),
    ));
}

fn styles(app: &mut App) {
    let _ = drawing(app);
    app.tab = "annotate";
    command(
        app,
        "cad.styles.edit",
        json!({ "kind": "leader", "operation": "create",
                "leader": { "id": "", "name": "Boru notu", "height": 2.5, "arrow": "dot", "mask": true } }),
    );
    command(
        app,
        "cad.styles.edit",
        json!({ "kind": "leader", "operation": "create", "leader": { "id": "", "name": "Kapı balonu", "height": 2.0, "arrowSize": 1.5 } }),
    );
    run(app, "leader.styles");
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("kilavuz-kollar", arms),
        ("kilavuz-hizala", align),
        ("kilavuz-stilleri", styles),
    ]
}
