//! The change effects table (docs/adr/0314 §2) held to what the document does: each row's command runs on a saved
//! drawing, and its undo (an undo step or not), its dirty flag and the parts of the project it wrote (objects,
//! blocks, the layer tree, the settings, the style library, the sheet book) are the row's.

use kentos_domain::contracts::{
    CHANGE_EFFECTS, CallOp, DocumentSnapshotV1, EffectPart, EffectUndo,
};
use kentos_domain::{Document, Slot};
use kentos_native_application::{ExecutionContext, registry};
use serde_json::{Value, json};

const X: f64 = 500000.0;
const Y: f64 = 4400000.0;

fn drawing() -> Document {
    let style = json!({ "color": "fg", "lineType": "continuous", "lineWeight": 0.25 });
    let layer = |id: &str| {
        json!({ "id": id, "name": id, "type": "layer", "visible": true, "locked": false, "expanded": true,
                "style": style, "children": [] })
    };
    let square = |id: u32, x: f64| {
        json!({ "kind": "polygon", "id": id, "layerId": "duvar", "attrs": {}, "pts": [
            { "x": X + x, "y": Y }, { "x": X + x + 10.0, "y": Y }, { "x": X + x + 10.0, "y": Y + 10.0 }, { "x": X + x, "y": Y + 10.0 } ] })
    };
    let snapshot = json!({
        "format": "kentos.document", "version": 1, "name": "Etkiler",
        "settings": { "srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                      "plotScale": 100, "workspace": "cad", "drawingFont": "barlow",
                      "textStyles": [{ "id": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d0101", "name": "Ada no", "font": "arimo", "height": 3.5 }],
                      "lineTypes": [{ "name": "EKSEN", "elements": [{ "length": 12.0 }, { "length": -3.0 }], "unit": "drawing" }] },
        "origin": { "x": X, "y": Y },
        "layers": [layer("duvar"), layer("yazi")],
        "activeLayer": "duvar",
        "entities": [
            square(1, 0.0),
            square(2, 20.0),
            { "kind": "text", "id": 3, "layerId": "yazi", "attrs": {}, "p": { "x": X, "y": Y + 30.0 }, "text": "A",
              "height": 0.35, "rotation": 0, "textStyle": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d0101", "font": "arimo" },
            { "kind": "insert", "id": 4, "layerId": "duvar", "attrs": {}, "block": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4001",
              "p": { "x": X + 40.0, "y": Y }, "scale": 1, "rotation": 0 }
        ],
        "styles": { "items": [
            { "kind": "symbol", "id": "p-alan", "name": "Yeşil alan", "path": ["Semboller"],
              "symbol": { "type": "fill", "layers": [{ "id": "f", "type": "simpleFill", "color": "#30A46C" }] } }
        ], "categories": [] },
        "blocks": [{ "id": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4001", "name": "Kapı", "base": { "x": 0, "y": 0 },
                     "entities": [{ "kind": "line", "id": 1, "layerId": "0", "attrs": {}, "a": { "x": 0, "y": 0 }, "b": { "x": 1, "y": 0 } }] }]
    });
    let mut doc = Document::from_snapshot(
        DocumentSnapshotV1::from_json(&snapshot.to_string()).expect("reads"),
    )
    .expect("opens");
    let revision = doc.revision();
    doc.mark_saved(revision);
    doc
}

/// A row's command and its input on [`drawing`].
fn sample(id: &str, uid: impl Fn(u32) -> String) -> Option<(&'static str, Value)> {
    Some(match id {
        "object-look" => (
            "cad.entities.set",
            json!({ "uids": [uid(1)], "color": "#FF0000", "operation": "color" }),
        ),
        "object-order" => (
            "cad.entities.order",
            json!({ "uids": [uid(1)], "to": "front" }),
        ),
        "annotative-scales" => (
            "cad.annotations.scales",
            json!({ "operation": "make", "uids": [uid(3)], "scales": [50.0] }),
        ),
        "match" => (
            "cad.entities.match",
            json!({ "source": uid(3), "uids": [uid(2)], "properties": ["layer"] }),
        ),
        "layer-look" => (
            "cad.layers.edit",
            json!({ "operation": "style", "layer": "duvar", "color": "#00AA00" }),
        ),
        "layer-renderer" => (
            "cad.layers.renderer",
            json!({ "layer": "duvar", "renderer": { "type": "single", "symbols": { "fill": { "ref": "p-alan" } } } }),
        ),
        "layer-flags" => (
            "cad.layers.set",
            json!({ "layer": "duvar", "operation": "locked", "value": true }),
        ),
        "layer-states" => (
            "cad.layers.states",
            json!({ "operation": "save", "name": "Pafta" }),
        ),
        "annotation-styles" => (
            "cad.styles.edit",
            json!({ "kind": "text", "operation": "update", "text": {
                "id": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d0101", "name": "Ada no", "font": "overpass", "height": 3.5 } }),
        ),
        "line-types" => (
            "cad.linetypes.edit",
            json!({ "operation": "scale", "scale": 2.0 }),
        ),
        "standards" => ("cad.standards.apply", json!({ "name": "mimari-ofis" })),
        "library" => (
            "cad.library.edit",
            json!({ "operation": "rename", "id": "p-alan", "name": "Çimen" }),
        ),
        "library-assign" => (
            "cad.library.edit",
            json!({ "operation": "assign", "id": "p-alan", "uids": [uid(2)] }),
        ),
        "blocks" => (
            "cad.blocks.edit",
            json!({ "operation": "rename", "block": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4001", "name": "Kapı 90" }),
        ),
        "sheet-maps" => (
            "cad.sheets.edit",
            json!({ "ops": [{ "op": "addSheet", "sheet": { "id": "kat-plani", "items": [{
                "frame": { "height": 277000, "left": 20000, "top": 10000, "width": 390000 }, "id": "harita",
                "kind": { "type": "map", "view": { "center": { "x": 2.0, "y": 1.5 }, "rotation": 0, "scale": 50, "type": "fixed" },
                          "layers": { "type": "theme", "name": "Pafta" },
                          "layerOverrides": [{ "layer": "duvar", "frozen": true }] },
                "name": "Harita" }], "name": "Kat planı",
                "page": { "margins": { "bottom": 10000, "left": 20000, "right": 10000, "top": 10000 }, "orientation": "landscape",
                          "paper": "a3", "size": { "height": 297000, "width": 420000 } } } }] }),
        ),
        "settings" => ("cad.settings.set", json!({ "plotScale": 200 })),
        _ => return None,
    })
}

/// What a document holds of each part, to compare before and after.
fn parts(doc: &Document) -> Vec<(EffectPart, String)> {
    let objects: Vec<Value> = doc.entities().map(|e| json!(e)).collect();
    let layers = serde_json::to_string(doc.layers().nodes()).unwrap_or_default();
    vec![
        (EffectPart::Objects, json!(objects).to_string()),
        (
            EffectPart::Blocks,
            json!(doc.blocks().iter().map(|b| &**b).collect::<Vec<_>>()).to_string(),
        ),
        (EffectPart::Layers, layers),
        (EffectPart::Settings, json!(doc.settings()).to_string()),
        (EffectPart::Styles, json!(doc.styles()).to_string()),
        (EffectPart::Sheets, json!(doc.sheets()).to_string()),
    ]
}

#[test]
fn every_row_of_the_change_effects_table_is_what_its_command_does() {
    let mut wrong = Vec::new();
    for row in &CHANGE_EFFECTS {
        let Some(command) = row.command else {
            // A preference is not the drawing's: no command writes it.
            assert_eq!(row.undo, EffectUndo::None, "{}", row.id);
            assert!(!row.dirty, "{}", row.id);
            assert_eq!(row.parts, &[EffectPart::Preferences], "{}", row.id);
            continue;
        };
        let mut doc = drawing();
        let uid = |slot: u32| {
            doc.uid(Slot(slot))
                .map(|u| u.to_string())
                .unwrap_or_default()
        };
        let Some((sample_command, input)) = sample(row.id, uid) else {
            wrong.push(format!("{}: örnek girdi yok", row.id));
            continue;
        };
        assert_eq!(sample_command, command, "{}", row.id);
        let before = parts(&doc);
        let answer = registry::run(
            &mut ExecutionContext::new(&mut doc),
            command,
            CallOp::Execute,
            input,
        )
        .unwrap_or_else(|e| panic!("{}: {e:?}", row.id));
        if answer["status"] != "completed" {
            wrong.push(format!("{}: {command} yazmadı: {answer}", row.id));
            continue;
        }
        let after = parts(&doc);
        let touched: Vec<EffectPart> = before
            .iter()
            .zip(&after)
            .filter(|(b, a)| b.1 != a.1)
            .map(|(b, _)| b.0)
            .collect();
        let undo = if doc.can_undo() {
            EffectUndo::Step
        } else if doc.is_dirty() {
            EffectUndo::Outside
        } else {
            EffectUndo::None
        };
        if touched != row.parts || undo != row.undo || doc.is_dirty() != row.dirty {
            wrong.push(format!(
                "{}: tablo {:?} {:?} kirli={}, komut {:?} {:?} kirli={}",
                row.id,
                row.parts,
                row.undo,
                row.dirty,
                touched,
                undo,
                doc.is_dirty()
            ));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
