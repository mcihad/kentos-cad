//! The pictures of docs/adr/0327: a plan's doors and windows beside a cost table whose counts are block count fields,
//! its amounts formulas, its heading filled and its total bold, open in Tabloyu düzenle with the total's formula in
//! the bar (tablo-formul); the same drawing with its block schedule and a table linked to a CSV file, the linked
//! table selected with Öznitelikler's source rows (tablo-baglanti); DXF dışa aktar with the annotations' forms
//! (dxf-bicimler). `tools_screens` takes them in both themes at 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=tablo-formul,tablo-baglanti,dxf-bicimler cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use kentos_commands::DxfForm;
use kentos_domain::Slot;
use serde_json::{Value, json};

use crate::app::{App, Message};
use crate::exchange::dxf_export::{Event as Dxf, Form};
use crate::files_testing::make_cad;
use crate::tables::editor::Event as Edit;
use crate::tools_scenes::{E, N, Objects, layer, open_edited, run};
use crate::tools_screens::Scene;

const DOOR: &str = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3c01";
const WINDOW: &str = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3c02";

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

fn xy(x: f64, y: f64) -> Value {
    json!({ "x": E + x, "y": N + y })
}

/// A table of `cells` at `p` (east, north from the origin), its columns `columns` wide, rows of 0.6 m, a heading row.
fn table(
    o: &mut Objects,
    p: [f64; 2],
    columns: &[f64],
    cells: Value,
    source: Option<Value>,
) -> u32 {
    let n = cells.as_array().map_or(0, Vec::len);
    let mut t = json!({ "kind": "table", "p": xy(p[0], p[1]), "rotation": 0.0, "height": 0.25,
                        "rows": vec![0.6; n], "columns": columns, "cells": cells, "header": true,
                        "font": "arimo" });
    if let Some(source) = source {
        t["source"] = source;
    }
    o.push("tablo", t)
}

struct Plan {
    cost: u32,
    linked: u32,
}

/// The plan: a flat's walls, three doors and two windows, a cost table, a block schedule and a table to link to
/// `malzeme.csv`; a dimension and a leader for the DXF's forms.
fn drawing(app: &mut App) -> Plan {
    let mut o = Objects::new();
    o.path(
        "duvar",
        &[[0.0, 0.0], [12.0, 0.0], [12.0, 8.0], [0.0, 8.0]],
        true,
    );
    o.line("duvar", [7.0, 0.0], [7.0, 8.0]);
    o.line("duvar", [0.0, 4.5], [7.0, 4.5]);
    for (x, y, turn) in [
        (2.0, 4.5, 0.0),
        (7.0, 2.0, std::f64::consts::FRAC_PI_2),
        (9.5, 0.0, 0.0),
    ] {
        o.insert("kapi", DOOR, [x, y], 1.0, turn, false);
    }
    for x in [3.0, 9.0] {
        o.insert("kapi", WINDOW, [x, 8.0], 1.0, 0.0, false);
    }
    o.dimension(
        "olcu",
        None,
        [[0.0, 0.0], [12.0, 0.0]],
        None,
        -1.2,
        json!({ "height": 0.25 }),
    );
    o.push(
        "olcu",
        json!({ "kind": "leader", "pts": [xy(9.5, 4.0), xy(10.8, 5.6)], "text": "Salon\n26,50 m²",
                "height": 0.25, "rotation": 0.0 }),
    );
    let cost = table(
        &mut o,
        [15.0, 8.0],
        &[3.0, 1.5, 2.2, 2.6],
        json!([
            ["Malzeme", "Adet", "Birim fiyat", "Tutar"],
            ["Kapı", "", "1450", ""],
            ["Pencere", "", "2200", ""],
            ["Boya (m²)", "96", "35", ""],
            ["Toplam", "", "", ""]
        ]),
        None,
    );
    table(
        &mut o,
        [15.0, 3.6],
        &[3.0, 1.5],
        json!([
            ["Blok", "Adet"],
            ["Kapı", "3"],
            ["Pencere", "2"],
            ["Toplam", "5"]
        ]),
        Some(json!({ "kind": "blocks" })),
    );
    let linked = table(
        &mut o,
        [20.0, 3.6],
        &[2.4, 1.4, 2.2],
        json!([
            ["Malzeme", "Adet", "Birim fiyat"],
            ["", "", ""],
            ["", "", ""]
        ]),
        Some(json!({ "kind": "file", "name": "malzeme.csv" })),
    );
    open_edited(app, o, |d| {
        d["layers"] = json!([
            layer("duvar", "Duvarlar", "fg", 0.5),
            layer("kapi", "Kapı ve pencere", "#3E63DD", 0.25),
            layer("olcu", "Ölçüler", "#E5484D", 0.18),
            layer("tablo", "Tablolar", "fg", 0.25)
        ]);
        d["activeLayer"] = json!("tablo");
        d["settings"]["plotScale"] = json!(100);
        d["blocks"] = json!([
            { "id": DOOR, "name": "Kapı", "base": { "x": 0, "y": 0 }, "entities": [
                { "kind": "line", "id": 1, "layerId": "", "attrs": {}, "a": { "x": 0, "y": 0 }, "b": { "x": 0, "y": 0.9 } },
                { "kind": "arc", "id": 2, "layerId": "", "attrs": {}, "c": { "x": 0, "y": 0 }, "r": 0.9, "a0": 0.0,
                  "a1": std::f64::consts::FRAC_PI_2 }] },
            { "id": WINDOW, "name": "Pencere", "base": { "x": 0, "y": 0 }, "entities": [
                { "kind": "polygon", "id": 1, "layerId": "", "attrs": {}, "pts": [
                    { "x": -0.6, "y": -0.1 }, { "x": 0.6, "y": -0.1 }, { "x": 0.6, "y": 0.1 }, { "x": -0.6, "y": 0.1 }] }] }
        ]);
    });
    make_cad(app);
    let u = uid(app, cost);
    let count = |name: &str| json!({ "kind": "project", "value": "blockCount", "name": name });
    command(
        app,
        "cad.tables.edit",
        json!({ "operation": { "kind": "edit", "uid": u, "edits": [
            { "kind": "field", "row": 1, "col": 1, "source": count("Kapı") },
            { "kind": "field", "row": 2, "col": 1, "source": count("Pencere") },
            { "kind": "setCell", "row": 1, "col": 3, "words": "=B2*C2" },
            { "kind": "setCell", "row": 2, "col": 3, "words": "=B3*C3" },
            { "kind": "setCell", "row": 3, "col": 3, "words": "=B4*C4" },
            { "kind": "setCell", "row": 4, "col": 3, "words": "=TOPLA(D2:D4)" },
            { "kind": "style", "row": 0, "col": 0, "rows": 1, "cols": 4, "fill": "#e2edfb", "bold": true },
            { "kind": "style", "row": 1, "col": 1, "rows": 4, "cols": 3, "align": "right" },
            { "kind": "style", "row": 1, "col": 3, "rows": 4, "cols": 1, "decimals": 2 },
            { "kind": "style", "row": 4, "col": 0, "rows": 1, "cols": 4, "bold": true, "color": "#c46a00" }
        ] } }),
    );
    let base = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/commands/v1/tables"
    );
    command(
        app,
        "cad.tables.edit",
        json!({ "operation": { "kind": "link", "uid": uid(app, linked), "path": "malzeme.csv", "base": base } }),
    );
    if let Some(doc) = app.document.as_ref() {
        app.spatial.sync(&doc.model);
    }
    app.tab = "annotate";
    app.command_expanded = false;
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: E - 1.5,
            min_y: N - 2.5,
            max_x: E + 27.0,
            max_y: N + 9.0,
        },
        24.0,
    );
    Plan { cost, linked }
}

/// Tabloyu düzenle on the cost table, the total's amount chosen: its formula in the bar.
fn formulas(app: &mut App) {
    let plan = drawing(app);
    let _ = app.open_table_editor(Slot(plan.cost));
    let _ = app.update(Message::TableEditor(Edit::Cell(4, 3)));
    let _ = app.update(Message::TableEditor(Edit::Release));
}

/// The linked table selected: Öznitelikler's source rows, the block schedule beside it.
fn linked(app: &mut App) {
    let plan = drawing(app);
    app.selection.set([Slot(plan.linked)]);
    let upper =
        kentos_ui::widget::docking::Slot::Docked(kentos_ui::widget::docking::Side::Right, 0);
    let _ = app.update(Message::Dock(kentos_ui::widget::docking::Event::Collapsed(
        upper, true,
    )));
    let _ = app.update(Message::Properties(crate::properties::Event::Toggle(
        "general",
    )));
}

/// DXF dışa aktar with dimensions as blocks and leaders native, the data left out.
fn dxf_forms(app: &mut App) {
    let _ = drawing(app);
    run(app, "file.export.dxf");
    let dxf = |e: Dxf| Message::Exchange(Box::new(crate::exchange::Event::DxfExport(e)));
    let _ = app.update(dxf(Dxf::Dimensions(Form(DxfForm::Block))));
    let _ = app.update(dxf(Dxf::Metadata));
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("tablo-formul", formulas),
        ("tablo-baglanti", linked),
        ("dxf-bicimler", dxf_forms),
    ]
}
