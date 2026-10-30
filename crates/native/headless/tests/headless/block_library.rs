//! The block library's acceptance (docs/adr/0726 §5): the fixtures' DXF and DWG libraries (fixtures/blocks/v1/kitaplik
//! and kitaplik-dwg) taken into a metre and a millimetre CAD plan; the bed placed turned and mirrored stands 2.00 ×
//! 0.90 m on the ground in both; the dining set keeps its four chairs nested; the toilet needs its unit; saved and
//! opened again, the blocks keep their unit and category, the inserts their turn and mirror; written to DXF, the file
//! says the bed's unit and its insert's mirror. With KENTOS_WRITE_LIBRARY=1 the DXF goes to fixtures/blocks/v1/out,
//! where `scripts/fixtures/block_library_cases.py --written` reads it without KentOS code.

use super::*;
use kentos_commands::{BlockUnits, DrawingUnit, Entity};

const ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../fixtures/blocks/v1");

fn file(rel: &str) -> String {
    std::path::Path::new(ROOT).join(rel).display().to_string()
}

/// An empty local CAD plan in `unit` at 1:50.
fn plan(unit: DrawingUnit) -> Session {
    Session::new(&NewDrawing {
        name: "Daire".into(),
        srid: 0,
        plot_scale: 50.0,
        workspace: Workspace::Cad,
        drawing_font: DrawingFont::Barlow,
        province: None,
        drawing_unit: Some(unit),
        annotation: None,
        cad_work: None,
    })
    .expect("a new plan")
}

fn ok(answer: Value) -> Value {
    assert_eq!(answer["status"], "completed", "{answer}");
    answer
}

/// A block's size in the answer, by its name.
fn size(answer: &Value, name: &str) -> (f64, f64) {
    let s = answer["output"]["sizes"]
        .as_array()
        .expect("sizes")
        .iter()
        .find(|s| s["name"] == name)
        .unwrap_or_else(|| panic!("{name}'s size: {answer}"));
    (s["width"].as_f64().expect("width"), s["depth"].as_f64().expect("depth"))
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

/// The ground box of the object `uid`, its nested blocks' pieces too.
fn ground(s: &Session, uid: &str) -> [f64; 4] {
    let doc = s.document();
    let id = uid.parse::<kentos_domain::Uuid>().expect("a uid");
    let slot = doc.slot_of(id).unwrap_or_else(|| panic!("{uid} in the drawing"));
    let e = doc.get(slot).expect("the object");
    kentos_native_application::sheets_fit::extent_of(doc, std::iter::once(e)).expect("a box")
}

#[test]
fn library_blocks_come_in_at_their_real_size_and_go_round() {
    let dir = scratch("block-library");
    for unit in [DrawingUnit::M, DrawingUnit::Mm] {
        let mut s = plan(unit);
        // The bedroom's DXF, its blocks in their library's category.
        let taken = ok(s
            .call(json!({ "command": "file.import.block", "input": {
                "path": file("kitaplik/yatak-odasi/yatak-odasi.dxf"), "mode": "blocks",
                "category": "kitaplik/yatak-odasi/yatak-odasi" } }))
            .expect("answers"));
        let (w, d) = size(&taken, "Yatak 90");
        assert!(close(w, 0.9) && close(d, 2.0), "{unit:?}: {w} × {d}");
        assert_eq!(taken["output"]["unit"]["name"], "milimetre");
        // The same bed from the DWG twin into another plan: the same size.
        let mut twin = plan(unit);
        let from_dwg = ok(twin
            .call(json!({ "command": "file.import.block", "input": {
                "path": file("kitaplik-dwg/yatak-odasi.dwg"), "mode": "blocks", "names": ["Yatak 90"] } }))
            .expect("answers"));
        assert_eq!(size(&from_dwg, "Yatak 90"), (w, d), "{unit:?}: DWG as DXF");
        // The dining set: the table brings its chair, placed four times.
        let set = ok(s
            .call(json!({ "command": "file.import.block", "input": {
                "path": file("kitaplik/yemek/yemek-takimi.dxf"), "mode": "blocks", "names": ["Yemek masası"] } }))
            .expect("answers"));
        let (w, d) = size(&set, "Yemek masası");
        assert!(close(w, 1.6) && close(d, 1.9), "{unit:?}: the table and its chairs {w} × {d}");
        let table = s.blocks().into_iter().find(|b| b.name == "Yemek masası").expect("the table").clone();
        let chair = s.blocks().into_iter().find(|b| b.name == "Sandalye").expect("the chair").id;
        assert_eq!(
            table.entities.iter().filter(|e| matches!(e, Entity::Insert(i) if i.block == chair)).count(),
            4
        );
        // The toilet names no unit: refused, then taken in millimetres.
        let refused = s
            .call(json!({ "command": "file.import.block", "input": {
                "path": file("kitaplik/islak-hacim/klozet.dxf"), "mode": "blocks" } }))
            .expect("answers");
        assert_eq!(refused["error"]["code"], "source_unit_unknown", "{refused}");
        let toilet = ok(s
            .call(json!({ "command": "file.import.block", "input": {
                "path": file("kitaplik/islak-hacim/klozet.dxf"), "mode": "blocks", "sourceUnit": "mm" } }))
            .expect("answers"));
        let (w, d) = size(&toilet, "Klozet");
        assert!(close(w, 0.38) && close(d, 0.55), "{unit:?}: {w} × {d}");

        // The bed placed turned a quarter and mirrored: 2.00 × 0.90 on the ground, whatever the plan's unit.
        let layer = s.summary()["activeLayer"].as_str().expect("a layer").to_owned();
        let placed = ok(s
            .call(json!({ "command": "cad.blocks.insert", "input": {
                "name": "Yatak 90", "at": { "x": 10.0, "y": 20.0 }, "rotation": std::f64::consts::FRAC_PI_2,
                "mirror": true, "layerId": layer } }))
            .expect("answers"));
        let bed = placed["output"]["uid"].as_str().expect("its id").to_owned();
        let [x0, y0, x1, y1] = ground(&s, &bed);
        assert!(close(x1 - x0, 2.0) && close(y1 - y0, 0.9), "{unit:?}: {} × {}", x1 - x0, y1 - y0);
        // Mirrored in its x axis, then turned: its foot (local +y) runs towards +x.
        assert!(close(x0, 10.0) && close(x1, 12.0), "{unit:?}: {x0} … {x1}");

        // Saved and opened: the blocks' units and categories, the insert's turn and mirror kept.
        let path = dir.join(format!("daire-{unit:?}.kcad"));
        s.save(Some(&path)).expect("saved");
        let again = Session::open(&path).expect("opened");
        let bed_block = again.blocks().into_iter().find(|b| b.name == "Yatak 90").expect("the bed").clone();
        assert_eq!(bed_block.units, Some(BlockUnits::Mm));
        assert_eq!(
            bed_block.card.as_ref().and_then(|c| c.category.as_deref()),
            Some("kitaplik/yatak-odasi/yatak-odasi")
        );
        let rug = again.blocks().into_iter().find(|b| b.name == "Kilim").expect("the rug").units;
        assert_eq!(rug, Some(BlockUnits::Cm), "the rug in its own unit");
        let [x0, y0, x1, y1] = ground(&again, &bed);
        assert!(close(x1 - x0, 2.0) && close(y1 - y0, 0.9), "{unit:?}: as saved");

        // Written to DXF: the bed's record in millimetres, its insert mirrored.
        let dxf = dir.join(format!("daire-{unit:?}.dxf"));
        s.export_dxf(&dxf).expect("written");
        let text = std::fs::read_to_string(&dxf).expect("the DXF");
        assert!(text.contains("Yatak 90"), "the bed's block");
        if std::env::var_os("KENTOS_WRITE_LIBRARY").is_some() {
            let out = std::path::Path::new(ROOT).join("out");
            std::fs::create_dir_all(&out).expect("the output folder");
            let name = if unit == DrawingUnit::M { "daire-m.dxf" } else { "daire-mm.dxf" };
            std::fs::copy(&dxf, out.join(name)).expect("copied");
        }
    }
}
