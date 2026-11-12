//! Çatı planı paftası (docs/adr/8044 §7): Ruhsat takımı's building (`permit_set_tests`) with a hip roof over 1. kat, a
//! chimney and a roof window on it. No storey over 1. kat draws the roof whole, so the set gets “Çatı planı” after the
//! storeys' plans, EK-2's PL, CAT (Tablo 2.13, "çatı planı ve detayları") and NNN, its map the roof plan, and publishes
//! it as a PDF/A-2 file named by EK-2. The files stay in `.run/gui-evidence/mimari-m44/p4/ruhsat/` to be looked at.

use serde_json::json;

use crate::permit_set_tests::{building, particulars};

#[test]
fn a_roofed_building_s_permit_set_prints_its_roof_plan() {
    let mut app = building();
    {
        let doc = &mut app.document.as_mut().expect("a drawing").model;
        // The walls' outer faces (0, 0)–(15, 6), 50 cm out: a 16 × 7 m hip roof at 33 %; a chimney on its north face
        // and a 78 × 118 cm window on its south face.
        let eaves = kentos_native_application::roofs::eaves_from_walls(
            doc,
            kentos_commands::Vec2 { x: 7.5, y: 3.0 },
            Some("k1"),
            0.5,
        )
        .expect("the walls enclose the point");
        let n = eaves.len();
        let layer = doc.layers().active().to_owned();
        let answer = kentos_native_application::registry::run(
            &mut kentos_native_application::ExecutionContext::new(doc),
            "cad.roofs.create",
            kentos_commands::CallOp::Execute,
            json!({"layerId": layer, "roofs": [{"pts": eaves, "slopes": vec![33.0; n], "overhang": 0.5,
                "arch": {"level": "k1"}, "parts": [
                    {"kind": "chimney", "at": {"x": 4.0, "y": 4.0}, "width": 0.4, "depth": 0.4},
                    {"kind": "window", "at": {"x": 12.0, "y": 1.5}, "width": 0.78, "depth": 1.18}]}]}),
        )
        .expect("runs");
        assert_eq!(answer["status"], "completed", "{answer}");
    }
    let doc = &mut app.document.as_mut().expect("a drawing").model;
    let input = kentos_native_application::permit_set::PermitSet {
        phase: kentos_sheet::permit::Phase::Permit,
        values: particulars(),
        ..Default::default()
    };
    let (_, made, notes) = kentos_native_application::permit_set::execute(
        &mut kentos_native_application::ExecutionContext::new(doc),
        &input,
    )
    .expect("made");
    assert!(notes.is_empty(), "{notes:?}");
    let titles: Vec<String> = made
        .iter()
        .map(|m| format!("{} {}", m.number, m.title))
        .collect();
    assert_eq!(
        titles,
        [
            "UP-01 Proje bilgileri",
            "UP-02 Vaziyet planı",
            "UP-03 Zemin kat planı",
            "UP-04 1. kat planı",
            "UP-05 Çatı planı"
        ]
    );
    // Its map is the roof plan of 1. kat's roof.
    let (book, _) =
        kentos_sheet::project::read_project_sheets(doc.sheets().expect("a book")).expect("reads");
    let sheet = book
        .sheets
        .iter()
        .find(|s| s.id == made[4].sheet)
        .expect("the roof plan's sheet");
    let level = sheet
        .items
        .iter()
        .find_map(|i| match &i.kind {
            kentos_sheet::ItemKind::Map(m) => m.level.clone(),
            _ => None,
        })
        .expect("its map's level");
    assert!(level.roof && level.id == "k1", "{level:?}");
    // Published as PDF/A-2, its name EK-2's: PL, CAT, NNN.
    let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../.run/gui-evidence/mimari-m44/p4/ruhsat");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.parent().expect("a parent")).expect("a folder");
    let answer = kentos_headless::file_commands::call(
        doc,
        kentos_commands::CallEnvelope {
            command: kentos_commands::FILE_PUBLISH_SHEETS.into(),
            version: Some(1),
            op: kentos_commands::CallOp::Execute,
            request_id: None,
            expected: None,
            relax_constraints: false,
            input: json!({
                "path": dir,
                "sheets": made.iter().map(|m| m.sheet.clone()).collect::<Vec<_>>(),
                "mode": "separate",
                "pdfa": true,
                "naming": kentos_sheet::permit::FILE_NAMING
            }),
        },
    );
    let got = serde_json::to_value(&answer.result).expect("an answer");
    assert_eq!(got["status"], "completed", "{got}");
    let names: Vec<String> = got["output"]["files"]
        .as_array()
        .expect("files")
        .iter()
        .map(|f| f["name"].as_str().unwrap_or_default().to_owned())
        .collect();
    let roof = "260457_00_MM_XXXX_PL_CAT_NNN_01_000.pdf";
    assert_eq!(names.last().map(String::as_str), Some(roof), "{names:?}");
    let bytes = std::fs::read(dir.join(roof)).expect("delivered");
    let report = kentos_formats::pdf::pdfa::pdfa2_report(&bytes).expect("read back");
    assert!(report.conforms(), "{:?}", report.findings);
}
