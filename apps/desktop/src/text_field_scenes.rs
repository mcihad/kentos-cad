//! The pictures of docs/adr/0325: a plan's rooms with their labels' areas as fields, one room's label broken after
//! its room was deleted, the Salon label selected with Öznitelikler's Alan rows (yazi-alanlar); Alan ekle over the
//! Mutfak label with the room picked (yazi-alan-ekle); Numarala over three door labels (yazi-numarala); Yazı
//! denetimi's findings (yazi-denetimi). `tools_screens` takes them in both themes at 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=yazi-alanlar,yazi-alan-ekle,yazi-numarala,yazi-denetimi cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use kentos_domain::Slot;
use serde_json::{Value, json};

use crate::app::App;
use crate::files_testing::make_cad;
use crate::tools_scenes::{E, N, Objects, hover, layer, open_edited, run};
use crate::tools_screens::Scene;

fn command(app: &mut App, name: &str, input: Value) {
    let doc = app.document.as_mut().expect("a drawing");
    let answer = kentos_native_application::registry::run(
        &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
        name,
        kentos_contracts::CallOp::Execute,
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

/// Three rooms, their labels, three doors' labels and a note with faults.
struct Plan {
    rooms: [u32; 3],
    labels: [u32; 3],
    doors: [u32; 3],
}

fn drawing(app: &mut App) -> Plan {
    let mut o = Objects::new();
    let room = |o: &mut Objects, x: f64, y: f64, w: f64, h: f64, name: &str| {
        o.push(
            "mekan",
            json!({ "kind": "polygon", "attrs": { "Ad": name },
                    "pts": [{ "x": E + x, "y": N + y }, { "x": E + x + w, "y": N + y }, { "x": E + x + w, "y": N + y + h }, { "x": E + x, "y": N + y + h }] }),
        )
    };
    let salon = room(&mut o, 0.0, 0.0, 6.4, 4.75, "Salon");
    let mutfak = room(&mut o, 6.4, 0.0, 3.6, 4.75, "Mutfak");
    let banyo = room(&mut o, 6.4, 4.75, 3.6, 2.5, "Banyo");
    let text = |o: &mut Objects, x: f64, y: f64, words: &str| {
        o.push(
            "yazi",
            json!({ "kind": "text", "p": { "x": E + x, "y": N + y }, "height": 0.3, "rotation": 0.0, "align": "middleCenter", "text": words }),
        )
    };
    let labels = [
        text(&mut o, 3.2, 2.4, "Salon "),
        text(&mut o, 8.2, 2.4, "Mutfak "),
        text(&mut o, 8.2, 6.0, "Banyo "),
    ];
    let doors = [
        text(&mut o, 2.0, -0.6, "K"),
        text(&mut o, 7.2, -0.6, "K"),
        text(&mut o, 7.6, 4.2, "K"),
    ];
    let _note = text(
        &mut o,
        3.2,
        6.4,
        "Plan  notu: ölçüler ölçüler metredir (kot ±0.00",
    );
    open_edited(app, o, |d| {
        d["layers"] = json!([
            layer("mekan", "Mekanlar", "#30A46C", 0.35),
            layer("yazi", "Yazı", "#3E63DD", 0.25)
        ]);
        d["activeLayer"] = json!("yazi");
        d["settings"]["plotScale"] = json!(50);
    });
    make_cad(app);
    for (label, room) in labels.iter().zip([salon, mutfak, banyo]) {
        let at = if *label == labels[1] { 7 } else { 6 };
        let room_uid = uid(app, room);
        command(
            app,
            "cad.fields.edit",
            json!({ "edit": { "kind": "insert", "uid": uid(app, *label), "start": at, "end": at,
                    "source": { "kind": "object", "uid": room_uid, "value": "area" },
                    "format": { "decimals": 2, "comma": true, "suffix": " m²" } } }),
        );
    }
    // The bathroom goes: its label's field breaks (####) and says why.
    command(
        app,
        "cad.entities.delete",
        json!({ "uids": [uid(app, banyo)] }),
    );
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: E - 1.5,
            min_y: N - 1.5,
            max_x: E + 11.5,
            max_y: N + 8.0,
        },
        24.0,
    );
    Plan {
        rooms: [salon, mutfak, banyo],
        labels,
        doors,
    }
}

fn fields(app: &mut App) {
    let plan = drawing(app);
    app.tab = "annotate";
    app.selection.set([Slot(plan.labels[0])]);
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

fn insert(app: &mut App) {
    let plan = drawing(app);
    app.tab = "annotate";
    // The Mutfak label and its room: the room is the field's object.
    app.selection
        .set([Slot(plan.labels[1]), Slot(plan.rooms[1])]);
    run(app, "text.fieldInsert");
    let _ = app.update(crate::app::Message::TextTools(
        crate::text_tools::Event::Value(1),
    ));
    let _ = app.update(crate::app::Message::TextTools(
        crate::text_tools::Event::Decimals("1".into()),
    ));
    let _ = app.update(crate::app::Message::TextTools(
        crate::text_tools::Event::Prefix(", çevre ".into()),
    ));
    let _ = app.update(crate::app::Message::TextTools(
        crate::text_tools::Event::Suffix(" m".into()),
    ));
}

fn number(app: &mut App) {
    let plan = drawing(app);
    app.tab = "annotate";
    app.selection.set(plan.doors.map(Slot));
    run(app, "text.number");
    let _ = app.update(crate::app::Message::TextTools(
        crate::text_tools::Event::Digits("2".into()),
    ));
    let _ = app.update(crate::app::Message::TextTools(
        crate::text_tools::Event::NumberPrefix("K-".into()),
    ));
}

fn check(app: &mut App) {
    let _ = drawing(app);
    app.tab = "annotate";
    app.selection.clear();
    run(app, "text.check");
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("yazi-alanlar", fields),
        ("yazi-alan-ekle", insert),
        ("yazi-numarala", number),
        ("yazi-denetimi", check),
    ]
}
