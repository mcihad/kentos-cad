//! Mimari işaretler through the session (docs/adr/0721): Kuzey oku places the north arrow (the library's block added
//! with it) turned to the plan's, a typed or the true north and leaves; Revizyon işaretle writes a cloud and its delta
//! with the revision's number, the drawing's last by default; Malzeme lejandı places the walls' material legend and
//! makes it again in place. The bench's drawing is in TUREF / TM36 at 1:1000; points are east and north differences
//! from (E, N).

use crate::common::{Bench, E, N, rel};
use kentos_commands::{Entity, InsertEntity, LEGEND_BLOCK, NORTH_BLOCK, REVISION_BLOCK};

fn bench() -> Bench {
    let mut b = Bench::on(include_str!("../../../../../fixtures/interaction/v1/empty.kcad"));
    b.draft.snap = false;
    b
}

fn insert(e: &Entity) -> &InsertEntity {
    let Entity::Insert(i) = e else {
        panic!("an insert: {e:?}");
    };
    i
}

fn block_of(b: &Bench, i: &InsertEntity) -> String {
    b.doc.block(i.block).map(|d| d.name.clone()).unwrap_or_default()
}

#[test]
fn a_north_arrow_goes_where_clicked_with_the_library_block_and_the_tool_leaves() {
    let mut b = bench();
    b.start("northArrow");
    assert_eq!(b.options(), ["Y", "A", "B"]);
    b.move_to(10.0, 20.0);
    assert!(!b.session.preview(&Default::default()).expect("a preview").strokes.is_empty(), "the ghost of the library's arrow");
    b.click(10.0, 20.0);
    let i = insert(b.newest()).clone();
    assert_eq!(block_of(&b, &i), NORTH_BLOCK);
    assert_eq!((rel(i.p), i.rotation), ([10.0, 20.0], 0.0));
    // 15 mm across at 1:1000: 15 m, the library's arrow 1 m across.
    assert_eq!(i.scale, 15.0 / 1000.0 * 1000.0 / 1.0);
    assert_eq!(b.last_text(), Some("Kuzey oku yerleşti: planın kuzeyi."));
    assert_ne!(b.session.tool_id(), "northArrow", "the tool leaves once the arrow is placed");
    assert_eq!(b.doc.undo().as_deref(), Some("Kuzey oku"));
    assert!(b.doc.block_named(NORTH_BLOCK).is_none(), "the block went with the arrow");
}

#[test]
fn a_north_arrow_takes_a_typed_angle_a_size_and_the_true_north() {
    let mut b = bench();
    b.start("northArrow");
    assert!(b.type_text("A"));
    assert!(b.type_text("30"));
    assert!(b.type_text("B"));
    assert!(b.type_text("20"));
    b.click(0.0, 0.0);
    let i = insert(b.newest()).clone();
    assert_eq!((i.rotation, i.scale), (30f64.to_radians(), 20.0 / 1000.0 * 1000.0 / 1.0));
    assert_eq!(b.last_text(), Some("Kuzey oku yerleşti: planın kuzeyinden 30°."));
    // Coğrafi kuzey: west of the 36° meridian true north leans east, a clockwise (negative) turn.
    b.start("northArrow");
    assert!(b.run(|s, cx| s.configure("Y", "coğrafi", cx)));
    b.click(5.0, 5.0);
    let i = insert(b.newest()).clone();
    assert!(i.rotation < 0.0 && i.rotation > -0.01, "{}", i.rotation);
    assert!(b.last_text().is_some_and(|t| t.contains("meridyen yakınsaması")));
    // A local project has no meridian: Coğrafi kuzey is not offered, chosen it is refused and said, the plan's north
    // kept (docs/adr/0802).
    let mut b = bench();
    let mut s = b.doc.settings().clone();
    s.srid = 0;
    b.doc.set_settings(s);
    b.start("northArrow");
    assert!(!b.run(|s, cx| s.configure("Y", "coğrafi", cx)));
    assert!(b.last_text().is_some_and(|t| t.contains("coğrafi kuzey bulunamaz")));
    assert_eq!(b.session.tool_id(), "northArrow");
    b.click(5.0, 5.0);
    assert_eq!(insert(b.newest()).rotation, 0.0, "the plan's north");
}

#[test]
fn a_revision_mark_writes_its_cloud_and_delta_and_takes_the_last_number() {
    let mut b = bench();
    b.start("revisionMark");
    assert_eq!(b.session.prompt().text().split(':').next(), Some("Revizyon işaretle"));
    assert!(b.options().contains(&"R"));
    b.click(0.0, 0.0);
    b.move_to(20.0, 10.0);
    let p = b.session.preview(&Default::default()).expect("a preview");
    assert!(p.strokes.len() > 1, "the cloud and the delta's ghost");
    b.click(20.0, 10.0);
    let tag = insert(b.newest()).clone();
    assert_eq!(block_of(&b, &tag), REVISION_BLOCK);
    assert_eq!(tag.base.attrs.get("REV").map(String::as_str), Some("1"));
    assert_eq!(b.last_text(), Some("Revizyon 1 işaretlendi: bulut ve üçgen."));
    // Revizyon B typed: the next mark writes it.
    assert!(b.type_text("R"));
    assert!(b.type_text("B"));
    b.click(30.0, 0.0);
    b.click(40.0, 10.0);
    assert_eq!(insert(b.newest()).base.attrs.get("REV").map(String::as_str), Some("B"));
    // A new run takes the drawing's last revision.
    b.start("revisionMark");
    assert!(b.session.prompt().text().contains("Revizyon (R): B"), "{}", b.session.prompt().text());
    assert_eq!(b.doc.undo().as_deref(), Some("Revizyon"));
}

#[test]
fn a_revision_takes_its_date_and_description_and_a_known_number_brings_them() {
    let mut b = bench();
    b.start("revisionMark");
    let keys: Vec<String> = b.session.settings(&Default::default()).iter().map(|s| s.key.to_owned()).collect();
    assert_eq!(keys, ["R", "T", "A"]);
    assert!(b.type_text("T"));
    assert!(b.type_text("2026-10-20"));
    assert!(b.type_text("A"));
    assert!(b.type_text("Mutfak duvarı taşındı"));
    assert!(b.session.prompt().text().contains("Tarih (T): 2026-10-20"), "{}", b.session.prompt().text());
    b.click(0.0, 0.0);
    b.click(20.0, 10.0);
    let tag = insert(b.newest()).clone();
    let attrs = &tag.base.attrs;
    assert_eq!(
        (attrs.get("REV"), attrs.get("Tarih"), attrs.get("Açıklama")),
        (Some(&"1".to_owned()), Some(&"2026-10-20".to_owned()), Some(&"Mutfak duvarı taşındı".to_owned()))
    );
    // Revision 2, its own words; then 1 typed again: the drawing's date and description come back with it.
    assert!(b.type_text("R"));
    assert!(b.type_text("2"));
    assert!(b.session.settings(&Default::default()).iter().all(|s| s.key == "R" || s.value.is_empty()));
    assert!(b.type_text("A"));
    assert!(b.type_text("Pencere genişledi"));
    b.click(30.0, 0.0);
    b.click(40.0, 10.0);
    assert!(b.type_text("R"));
    assert!(b.type_text("1"));
    let values: Vec<String> = b.session.settings(&Default::default()).iter().map(|s| s.value.clone()).collect();
    assert_eq!(values, ["1", "2026-10-20", "Mutfak duvarı taşındı"]);
    // A control character is refused and said; the date stays.
    assert!(!b.run(|s, cx| s.configure("T", "20\n26", cx)));
    assert!(b.last_text().is_some_and(|t| t.contains("tek satır")));
}

/// A wall of `components` along y = `y`.
fn wall(b: &mut Bench, y: f64, components: serde_json::Value) {
    let e: Entity = serde_json::from_value(serde_json::json!({
        "kind": "wall", "id": 0, "layerId": "cizim", "attrs": {},
        "pts": [{"x": E, "y": N + y}, {"x": E + 5.0, "y": N + y}], "width": 0.25, "justify": "center",
        "components": components,
    }))
    .unwrap();
    b.doc.add(e).unwrap();
}

#[test]
fn a_material_legend_goes_where_clicked_and_is_made_again_in_place() {
    let mut b = bench();
    let brick = serde_json::json!({"name": "Tuğla", "material": "Tuğla", "thickness": 0.19,
        "hatch": {"pattern": "ANSI31", "scale": 1.0, "angle": 45.0}});
    let plaster = serde_json::json!({"name": "Sıva", "material": "Kireç sıva", "thickness": 0.03});
    wall(&mut b, 0.0, serde_json::json!([plaster, brick, plaster]));
    b.start("materialLegend");
    assert!(!b.options().contains(&"Y"), "no legend to make again yet");
    b.move_to(20.0, 20.0);
    assert!(!b.session.preview(&Default::default()).expect("a preview").strokes.is_empty());
    b.click(20.0, 20.0);
    let i = insert(b.newest()).clone();
    assert_eq!((block_of(&b, &i), rel(i.p)), (LEGEND_BLOCK.to_owned(), [20.0, 20.0]));
    assert_eq!(b.last_text(), Some("Malzeme lejandı yerleşti: 2 malzeme."));
    // A third material: Yenile makes the legend again in its place, nothing placed.
    let concrete = serde_json::json!({"name": "Gazbeton", "material": "Gazbeton", "thickness": 0.2,
        "hatch": {"pattern": "ANSI37", "scale": 0.5, "angle": 0.0}});
    wall(&mut b, 3.0, serde_json::json!([concrete]));
    let count = b.doc.entities().count();
    b.start("materialLegend");
    assert!(b.options().contains(&"Y"));
    assert!(b.type_text("Y"));
    assert_eq!(b.doc.entities().count(), count, "no new insert");
    assert_eq!(b.last_text(), Some("Malzeme lejandı yerinde yenilendi: 3 malzeme."));
    assert_eq!(b.doc.undo().as_deref(), Some("Malzeme lejandı"));
}
