//! The block library's everyday families (docs/adr/9307): the library `kentos-temel` is taken in and every block of
//! the office, school, clinic, accessible wet-room and landscape families placed by its name with the same
//! `cad.blocks.insert` the screen, Python, MCP and the agent run, in each of the three CAD works.

use kentos_commands::{CadWork, DrawingFont, Workspace};
use kentos_headless::{NewDrawing, Session};
use serde_json::{Value, json};

const FAMILIES: &[&str] = &[
    "Toplantı masası 4 kişilik",
    "Toplantı masası 6 kişilik",
    "Toplantı masası 8 kişilik",
    "İş istasyonu",
    "Arşiv dolabı",
    "Öğrenci sırası (tek)",
    "Öğrenci sırası (çift)",
    "Öğretmen masası",
    "Yazı tahtası",
    "Hasta yatağı",
    "Muayene masası",
    "Lavabo (sağlık)",
    "Erişilebilir klozet",
    "Erişilebilir lavabo",
    "Dönüş alanı Ø150",
    "Ağaç Ø3 m",
    "Ağaç Ø5 m",
    "Ağaç Ø8 m",
    "Çalı",
    "Otomobil",
    "Bisiklet park yeri",
];

fn project(work: CadWork) -> Session {
    Session::new(&NewDrawing {
        name: "Aile".into(),
        srid: 5256,
        plot_scale: 100.0,
        workspace: Workspace::Cad,
        drawing_font: DrawingFont::Barlow,
        province: None,
        drawing_unit: None,
        annotation: None,
        cad_work: Some(work),
        heights: None,
    })
    .expect("a new project")
}

fn answer(s: &mut Session, command: &str, input: Value) -> Value {
    s.call(json!({ "command": command, "input": input }))
        .expect("answers")
}

#[test]
fn every_new_family_places_by_name_in_each_cad_work() {
    for work in [CadWork::Architecture, CadWork::Map, CadWork::Road] {
        let mut s = project(work);
        let layer = s.summary()["activeLayer"]
            .as_str()
            .expect("a layer")
            .to_owned();
        let taken = answer(
            &mut s,
            "cad.blocks.edit",
            json!({ "operation": "import", "name": "kentos-temel" }),
        );
        assert_eq!(taken["status"], "completed", "{work:?}: {taken}");
        for (i, name) in FAMILIES.iter().enumerate() {
            let placed = answer(
                &mut s,
                "cad.blocks.insert",
                json!({ "name": name, "at": { "x": 423500.0 + 12.0 * i as f64, "y": 4512300.0 }, "layerId": layer }),
            );
            assert_eq!(placed["status"], "completed", "{work:?} {name}: {placed}");
        }
    }
}
