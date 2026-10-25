use crate::common::{Bench, E, N};
use kentos_commands::{Entity, WallJustify};
fn setup() -> Bench {
    let mut b = Bench::new("wallFromLines");
    let layer = b.doc.layers().active().to_owned();
    let e:Entity=serde_json::from_value(serde_json::json!({"kind":"polygon","id":0,"layerId":layer,"pts":[{"x":E,"y":N},{"x":E+6.,"y":N},{"x":E+6.,"y":N+4.},{"x":E,"y":N+4.}],"attrs":{"office":"A"}})).unwrap();
    let slots = b.doc.add_many(vec![e], "Test source").unwrap();
    b.selection.set(slots);
    b.start("wallFromLines");
    b
}
fn set(b: &mut Bench, k: &str, v: &str) -> bool {
    b.run(|s, cx| s.configure(k, v, cx))
}
#[test]
fn contour_settings_are_visible_read_only_preview_then_one_commit_and_undo() {
    let mut b = setup();
    let source = b.newest().clone();
    let before = b.doc.to_snapshot_v2();
    assert!(set(&mut b, "M", "Kapalı kontur"));
    assert!(set(&mut b, "H", "İç net"));
    assert!(set(&mut b, "E", "250mm"));
    assert!(set(&mut b, "K", "Koru"));
    let rows = b.session.settings(&Default::default());
    assert!(rows.iter().any(|r| r.key == "H" && r.value == "İç net"));
    assert!(!rows.iter().any(|r| r.key == "Ö"));
    assert!(!set(&mut b, "E", "-1m"));
    assert_eq!(b.doc.to_snapshot_v2(), before);
    assert!(
        !b.session
            .preview(&Default::default())
            .unwrap()
            .areas
            .is_empty()
    );
    b.confirm();
    assert_eq!(b.doc.len(), 2);
    let Entity::Wall(w) = b.newest() else {
        panic!()
    };
    assert_eq!(w.wall.width, 0.25);
    assert_eq!(w.wall.justify, WallJustify::Left);
    assert_eq!(w.base.attrs, source.base().attrs);
    b.doc.undo().unwrap();
    assert_eq!(b.doc.to_snapshot_v2().entities, before.entities);
}
#[test]
fn switching_modes_keeps_legacy_pairs_shortcut_and_visible_source_deletion() {
    let mut b = setup();
    assert!(set(&mut b, "M", "contours"));
    assert!(b.type_text("Ç"));
    assert!(b.memory.wall_pairs);
    assert!(!b.memory.wall_contours);
    assert!(set(&mut b, "M", "each"));
    assert!(!b.memory.wall_pairs);
    assert!(!b.memory.wall_contours);
    assert!(set(&mut b, "M", "contours"));
    assert!(set(&mut b, "K", "Sil"));
    let source = b.newest().clone();
    b.confirm();
    assert_eq!(b.doc.len(), 1);
    assert!(matches!(b.newest(), Entity::Wall(_)));
    b.doc.undo().unwrap();
    assert_eq!(b.newest(), &source);
}
#[test]
fn stale_contour_preview_refuses_before_refreshing_and_requires_a_new_explicit_confirm() {
    let mut b = setup();
    assert!(set(&mut b, "M", "contours"));
    assert!(set(&mut b, "K", "keep"));
    let mut settings = b.doc.settings().clone();
    settings.length_decimals = 4;
    b.doc.set_settings(settings);
    let before = b.doc.to_snapshot_v2();
    b.confirm();
    assert_eq!(b.doc.to_snapshot_v2(), before);
    b.confirm();
    assert_eq!(b.doc.len(), 2);
}

#[test]
fn shared_axes_preview_reports_ownership_tolerance_and_requires_one_confirm() {
    let mut b = Bench::new("wallFromLines");
    let layer = b.doc.layers().active().to_owned();
    let entities:Vec<Entity>=(0..2).map(|i|{let x=E+f64::from(i)*4.;serde_json::from_value(serde_json::json!({"kind":"polygon","id":0,"layerId":layer,"pts":[{"x":x,"y":N},{"x":x+4.,"y":N},{"x":x+4.,"y":N+3.},{"x":x,"y":N+3.}],"attrs":{"oda":i.to_string()}})).unwrap()}).collect();
    let slots = b.doc.add_many(entities, "Konturlar").unwrap();
    b.selection.set(slots);
    b.start("wallFromLines");
    let before = b.doc.to_snapshot_v2();
    assert!(set(&mut b, "M", "contours"));
    assert!(
        b.session
            .prompt()
            .text()
            .contains("Eksen referansını seçin")
    );
    assert!(set(&mut b, "H", "center"));
    assert!(set(&mut b, "K", "keep"));
    let text = b.session.prompt().text();
    assert!(text.contains("3 duvar"));
    assert!(text.contains("0.000001 m"));
    assert!(text.contains("ilk kaynaktan"));
    let preview = b.session.preview(&Default::default()).unwrap();
    assert!(!preview.areas.is_empty());
    assert_eq!(b.doc.to_snapshot_v2(), before);
    b.confirm();
    assert_eq!(b.doc.len(), 5);
    b.doc.undo().unwrap();
    assert_eq!(b.doc.to_snapshot_v2().entities, before.entities);
}
