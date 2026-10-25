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

fn with_levels() -> Bench {
    let mut b = setup();
    let mut settings = b.doc.settings().clone();
    settings.arch = Some(
        serde_json::from_value(serde_json::json!({"levels":[
            {"id":"zemin","name":"Zemin kat","elevation":0,"height":3},
            {"id":"kat-1","name":"1. kat","elevation":3,"height":3}
        ]}))
        .unwrap(),
    );
    b.doc.set_settings(settings);
    b.start("wallFromLines");
    b
}
#[test]
fn visible_level_choice_is_read_only_and_replans_before_a_single_confirm() {
    let mut b = with_levels();
    assert!(set(&mut b, "M", "contours"));
    assert!(set(&mut b, "K", "keep"));
    let before = b.doc.to_snapshot_v2();
    let rows = b.session.settings(&Default::default());
    let level = rows.iter().find(|r| r.key == "L").unwrap();
    assert_eq!(level.value, "Belirtilmedi");
    assert!(
        level
            .choices
            .iter()
            .any(|c| c.command == Some("arch.settings"))
    );
    assert!(set(&mut b, "L", "id:zemin"));
    assert!(set(&mut b, "L", "1. kat"));
    assert!(!set(&mut b, "L", "id:unknown"));
    assert_eq!(
        b.session
            .settings(&Default::default())
            .iter()
            .find(|r| r.key == "L")
            .unwrap()
            .value,
        "1. kat"
    );
    assert_eq!(b.doc.to_snapshot_v2(), before);
    b.confirm();
    assert_eq!(b.doc.len(), 2);
    let Entity::Wall(w) = b.newest() else {
        panic!()
    };
    assert_eq!(w.wall.arch.level.as_deref(), Some("kat-1"));
    b.doc.undo().unwrap();
    assert_eq!(b.doc.to_snapshot_v2().entities, before.entities);
    b.start("wallFromLines");
    assert_eq!(
        b.session
            .settings(&Default::default())
            .iter()
            .find(|r| r.key == "L")
            .unwrap()
            .value,
        "Belirtilmedi"
    );
}
#[test]
fn a_removed_target_level_is_never_silently_replaced_by_unassigned() {
    let mut b = with_levels();
    assert!(set(&mut b, "M", "contours"));
    assert!(set(&mut b, "L", "id:zemin"));
    let mut settings = b.doc.settings().clone();
    settings
        .arch
        .as_mut()
        .unwrap()
        .levels
        .retain(|l| l.id != "zemin");
    b.doc.set_settings(settings);
    let before = b.doc.to_snapshot_v2();
    b.confirm(); // stale revision refresh
    b.confirm(); // unknown level refusal, still no write
    assert_eq!(b.doc.to_snapshot_v2(), before);
    assert_eq!(
        b.session
            .settings(&Default::default())
            .iter()
            .find(|r| r.key == "L")
            .unwrap()
            .value,
        "Bulunamadı: zemin"
    );
    assert!(set(&mut b, "L", "id:kat-1"));
    b.confirm();
    let Entity::Wall(w) = b.newest() else {
        panic!()
    };
    assert_eq!(w.wall.arch.level.as_deref(), Some("kat-1"));
}
#[test]
fn level_shortcut_cycles_and_duplicate_labels_do_not_guess_the_target() {
    let mut b = with_levels();
    assert!(b.type_text("L"));
    assert_eq!(
        b.session
            .settings(&Default::default())
            .iter()
            .find(|r| r.key == "L")
            .unwrap()
            .value,
        "Zemin kat"
    );
    assert!(b.type_text("L"));
    assert!(b.type_text("L"));
    assert_eq!(
        b.session
            .settings(&Default::default())
            .iter()
            .find(|r| r.key == "L")
            .unwrap()
            .value,
        "Belirtilmedi"
    );
    let mut settings = b.doc.settings().clone();
    let levels = &mut settings.arch.as_mut().unwrap().levels;
    levels[0].id = "Floor".into();
    levels[1].id = "floor".into();
    levels[0].name = "Ortak ad".into();
    levels[1].name = "Ortak ad".into();
    b.doc.set_settings(settings);
    b.start("wallFromLines");
    assert!(!set(&mut b, "L", "Ortak ad"));
    assert!(set(&mut b, "L", "id:floor"));
    assert!(set(&mut b, "M", "contours"));
    b.confirm();
    let Entity::Wall(w) = b.newest() else {
        panic!()
    };
    assert_eq!(w.wall.arch.level.as_deref(), Some("floor"));
}
