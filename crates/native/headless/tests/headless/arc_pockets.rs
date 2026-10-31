//! A pocket door in an arc wall out to DXF and to a 1:50 page (docs/adr/0787 §2): its leaf a closed polyline with its
//! arcs' bulges, the door read back as itself; on paper, across the wall through its pocket the two faces and the
//! cavity's two sides, past it only the faces.
use kentos_commands::{DxfReadOptions, Entity, OpeningOperation};
use kentos_headless::Session;
use serde_json::{Value, json};

use super::special_doors::{PX, crop};

const CASES: &str = include_str!("../../../../../fixtures/commands/v1/cad.openings.create.arcpocket.json");

fn session() -> Session {
    let file: Value = serde_json::from_str(CASES).unwrap();
    let model: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(file["setup"].clone()).unwrap();
    Session::from_bytes(&kentos_kcad::encode_verified(&model).unwrap()).unwrap()
}

#[test]
fn an_arc_pocket_door_goes_to_dxf_with_its_curved_leaf_and_comes_back() {
    let mut session = session();
    let folder = super::scratch("arc-pocket-dxf");
    let path = folder.join("yayli-cep.dxf");
    let written = session
        .call(json!({"command":"file.export.dxf","input":{"path":path}}))
        .unwrap();
    assert_eq!(written["status"], "completed", "{written}");
    let text = std::fs::read_to_string(&path).unwrap();
    let lines: Vec<&str> = text.lines().map(str::trim).collect();
    // The leaves' bulges in the blocks: the arc pocket door's leaf bends (|bulge| > 0 on its two long sides).
    let bulges: Vec<f64> = lines
        .chunks(2)
        .filter(|c| c.len() == 2 && c[0] == "42")
        .filter_map(|c| c[1].parse::<f64>().ok())
        .filter(|b| b.abs() > 1e-6)
        .collect();
    assert!(bulges.len() >= 2, "the curved leaf's arcs: {bulges:?}");
    let back = kentos_formats::dxf::read(&std::fs::read(&path).unwrap(), &DxfReadOptions::default())
        .unwrap()
        .entities;
    let pockets = back
        .iter()
        .filter(|e| matches!(e, Entity::Opening(o) if o.opening.operation == OpeningOperation::Pocket))
        .count();
    assert_eq!(pockets, 2, "both pocket doors read back");
    let _ = std::fs::remove_dir_all(folder);
}

#[test]
fn an_arc_pocket_prints_at_1_50_with_its_cavity_along_the_arc() {
    let mut session = session();
    // The page's centre on the arc wall's middle at 0,934 rad, where CG1's pocket is.
    let th = 0.934_f64;
    let at = [5.0 * th.cos(), 5.0 * th.sin()];
    let sheet = json!({ "op": "addSheet", "sheet": {
        "id": "yay", "name": "Yaylı cep 1:50",
        "page": { "paper": "a3", "orientation": "landscape", "size": { "width": 420000, "height": 297000 },
            "margins": { "top": 10000, "right": 10000, "bottom": 10000, "left": 20000 } },
        "items": [{ "id": "yay-harita", "name": "Harita",
            "frame": { "left": 20000, "top": 10000, "width": 390000, "height": 277000 },
            "kind": { "type": "map", "view": { "type": "fixed", "center": { "x": at[0], "y": at[1] },
                "scale": 50, "rotation": 0 } } }]
    }});
    let done = session
        .call(json!({"command":"cad.sheets.edit","input":{"ops":[sheet]}}))
        .unwrap();
    assert_eq!(done["status"], "completed", "{done}");
    let folder = super::scratch("arc-pocket-pdf");
    let path = folder.join("yayli-cep.pdf");
    let pdf = kentos_plot::book::book_pdf(session.document(), &[], "Yay", "2026-10-31").unwrap();
    std::fs::write(&path, pdf.bytes).unwrap();
    let page = [20.0 + 390.0 / 2.0, 10.0 + 277.0 / 2.0];
    let on_page = |x: f64, y: f64| [page[0] + (x - at[0]) * 20.0, page[1] - (y - at[1]) * 20.0];
    let (cx, cy) = (on_page(at[0], at[1])[0], on_page(at[0], at[1])[1]);
    let (w, h, px) = crop(&path, [cx - 30.0, cy - 30.0, 60.0, 60.0]);
    // The dark runs along the radial line at angle `a`, from 4,75 m to 5,25 m.
    let runs = |a: f64| {
        let mut runs = 0;
        let mut inside = false;
        for k in 0..=500 {
            let r = 4.75 + 0.001 * f64::from(k);
            let p = on_page(r * a.cos(), r * a.sin());
            let (x, y) = (((p[0] - (cx - 30.0)) * PX).round() as usize, ((p[1] - (cy - 30.0)) * PX).round() as usize);
            let dark = px[y.min(h - 1) * w + x.min(w - 1)] < 160;
            if dark && !inside {
                runs += 1;
            }
            inside = dark;
        }
        runs
    };
    assert_eq!(runs(th), 4, "the faces and the pocket's sides along the arc");
    assert_eq!(runs(0.78), 2, "past the pocket only the faces");
    let _ = std::fs::remove_dir_all(folder);
}
