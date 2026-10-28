//! Blok ekle says what it places (docs/adr/0726 §3, §4): beside the cursor the footprint's real width × depth in the
//! drawing's unit and the block's unit, a mark at its base point; a dynamic block's sizes chosen before the click
//! (Varyant, the pane's rows) go into the insert and its ghost. The drawing: an empty local plan in metres or in
//! millimetres with KentOS's block library (catalog/v1/blocks/kentos-temel.kblok), whose single bed is 0.90 × 2.00 m
//! and 1.20 m wide in its “Genç 120 × 200” row.

use crate::common::{Bench, E, N};
use kentos_commands::{Entity, ParamValue};
use kentos_interaction::Format;
use serde_json::{Value, json};

const LIBRARY: &str = include_str!("../../../../../catalog/v1/blocks/kentos-temel.kblok");
const BED: &str = "Tek kişilik yatak";

/// An empty CAD plan in `unit` with the library's blocks and Blok ekle placing the bed.
fn bench(unit: &str) -> Bench {
    let blocks: Value = serde_json::from_str::<Value>(LIBRARY).expect("the library")["blocks"].clone();
    let doc = json!({
        "format": "kentos.document", "version": 1, "name": "Kitaplık",
        "settings": {"srid": 0, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg",
                     "plotScale": 50, "workspace": "cad", "drawingUnit": unit},
        "origin": {"x": 0, "y": 0},
        "layers": [{"id": "mobilya", "name": "Mobilya", "type": "layer", "visible": true, "locked": false,
                    "expanded": true, "style": {"color": "fg", "lineType": "continuous", "lineWeight": 0.25},
                    "children": []}],
        "activeLayer": "mobilya", "entities": [], "styles": {"items": [], "categories": []}, "blocks": blocks,
    });
    let mut b = Bench::on(&doc.to_string());
    b.draft.snap = false;
    b.memory.block_insert = b.doc.block_named(BED).map(|d| d.id);
    b.start("blockInsert");
    b
}

fn format(b: &Bench) -> Format {
    Format::of(b.doc.settings())
}

fn tag(b: &Bench) -> Vec<String> {
    b.session
        .preview(&format(b))
        .and_then(|p| p.tag)
        .map(|t| t.lines)
        .unwrap_or_default()
}

fn row(b: &Bench, label: &str) -> Option<String> {
    b.session
        .settings(&format(b))
        .into_iter()
        .find(|s| s.label == label)
        .map(|s| s.value)
}

fn size(b: &Bench, w: f64, d: f64) -> String {
    let f = format(b);
    format!("{} × {} {}", f.length_bare(w), f.length_bare(d), f.length_unit_label())
}

#[test]
fn the_tag_says_the_real_size_and_unit_before_the_click() {
    for unit in ["m", "mm"] {
        let mut b = bench(unit);
        b.move_to(1.0, 1.0);
        let lines = tag(&b);
        assert_eq!(lines.first(), Some(&format!("{BED}: {}", size(&b, 0.9, 2.0))), "{unit}");
        assert!(lines.get(1).is_some_and(|l| l.contains("birimi metre")), "{unit}: {lines:?}");
        // The pane says the same; the base point is marked where the click puts it.
        assert_eq!(row(&b, "En × boy"), Some(size(&b, 0.9, 2.0)), "{unit}");
        assert_eq!(row(&b, "Bloğun birimi").as_deref(), Some("metre"), "{unit}");
        let preview = b.session.preview(&format(&b)).expect("the ghost");
        assert!(
            preview.markers.iter().any(|m| (m.at.x - (E + 1.0)).abs() < 1e-9 && (m.at.y - (N + 1.0)).abs() < 1e-9),
            "{unit}: the base point marked"
        );
        // A factor and a turn are said and the size follows them; a mirror is said.
        assert!(b.run(|s, cx| s.configure("scale", "2", cx)));
        assert!(b.run(|s, cx| s.configure("turn", "90", cx)));
        assert!(b.run(|s, cx| s.configure("mirror", "yes", cx)));
        b.move_to(1.0, 1.0);
        let lines = tag(&b);
        assert_eq!(lines.first(), Some(&format!("{BED}: {}", size(&b, 1.8, 4.0))), "{unit}");
        assert!(lines[1].contains("dönüş 90.0°") && lines[1].contains("aynalı"), "{unit}: {lines:?}");
    }
}

#[test]
fn a_size_row_chosen_before_the_click_goes_into_the_insert() {
    let mut b = bench("m");
    // Varyant (V) steps the bed's size table: its first row, then the next.
    assert!(b.type_text("V"));
    assert_eq!(row(&b, "Boyut").as_deref(), Some("Tek 90 × 200"), "Varyant takes the table's first row");
    assert!(b.type_text("V"));
    assert_eq!(row(&b, "Boyut").as_deref(), Some("Tek 100 × 200"), "and then its next");
    assert!(b.run(|s, cx| s.configure("p1", "Genç 120 × 200", cx)), "the pane chooses a row by its label");
    // The row sets En: its length row says what the click places.
    assert_eq!(row(&b, "En"), Some(format(&b).length(1.2)));
    b.move_to(0.0, 0.0);
    assert_eq!(tag(&b).first(), Some(&format!("{BED}: {}", size(&b, 1.2, 2.0))), "the ghost is the variant");
    b.click(0.0, 0.0);
    let Entity::Insert(placed) = b.newest() else {
        panic!("an insert: {:?}", b.newest());
    };
    assert_eq!(placed.parameters.get("En"), Some(&ParamValue::Number(1.2)), "{:?}", placed.parameters);
    assert_eq!(b.doc.undo().as_deref(), Some("Ekle"));
    // A row that is not one of the table's is said, and nothing is chosen.
    assert!(!b.run(|s, cx| s.configure("p1", "Kral 200 × 200", cx)));
    assert!(b.last_text().is_some_and(|t| t.contains("bir seçenek değil")), "{:?}", b.last_text());
}

#[test]
fn a_length_beyond_its_rule_is_said_in_the_pane() {
    let mut b = bench("mm");
    // En's rule is 0.80–1.40 m: 1600 mm is said, and the ghost keeps the bed as it is.
    assert!(b.run(|s, cx| s.configure("p0", "1600", cx)));
    b.move_to(0.0, 0.0);
    let problem = b
        .session
        .settings(&format(&b))
        .into_iter()
        .find(|s| s.key == "problem")
        .map(|s| s.value);
    assert!(problem.is_some(), "the refusal said in the pane");
    assert_eq!(row(&b, "En × boy"), Some(size(&b, 0.9, 2.0)));
    // A length in the rule is the drawing's millimetres: 1000 mm.
    assert!(b.run(|s, cx| s.configure("p0", "1000", cx)));
    b.move_to(0.0, 0.0);
    assert_eq!(row(&b, "En × boy"), Some(size(&b, 1.0, 2.0)));
    b.click(0.0, 0.0);
    let Entity::Insert(placed) = b.newest() else {
        panic!("an insert");
    };
    assert_eq!(placed.parameters.get("En"), Some(&ParamValue::Number(1.0)));
}
