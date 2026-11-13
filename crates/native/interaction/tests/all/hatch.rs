//! Tarama through the session, over the native document: the web's words,
//! the regions by closed object and by line work, islands, the boundary
//! layer, the patterns, one undo step “Tarama” (the web's `HatchTool`,
//! docs/adr/0062). The drawing is the trace's (fixtures/interaction/v1/hatch.kcad):
//! a 20 × 16 m parcel with an 8 × 6 m building inside, and line work
//! closing two 12 × 16 m faces, a 4 × 4 m closed polyline of another layer
//! inside the left one. Expected values are worked out by hand.

use crate::common;

use std::collections::BTreeMap;

use common::{Bench, E, N, rel};
use kentos_commands::{Entity, EntityBase, HatchPatternType, PathEntity, Vec2 as Wire};
use kentos_interaction::Level;

const HATCH: &str = include_str!("../../../../../fixtures/interaction/v1/hatch.kcad");

const PROMPT: &str = "Tarama: taranacak yerin içine tıklayın [Desen (D): ANSI31 / Ölçek (Ö): 1 / Açı (Ç): 0° / Ölçek tabanı (E): kâğıtta / Başlangıç (S): dünya / Ölçekli (L): kapalı / Saydamlık (M): %0 / Sıra (O): atama / Özellik al (Z) / Sınır (B): kapalı nesne / Adalar (A): taranmaz / İlişkili (İ): açık / Yazılar (Y): taranır]";

fn bench() -> Bench {
    let mut b = Bench::on(HATCH);
    b.start("hatch");
    b
}

#[test]
fn a_value_prompt_never_turns_a_canvas_click_into_a_hatch() {
    for key in ["Ö", "Ç", "M", "G", "R"] {
        let mut b = bench();
        if key == "G" {
            assert!(b.type_text("B"));
        }
        if key == "R" {
            assert!(b.type_text("degrade küre"));
        }
        assert!(b.type_text(key), "{key}");
        let step = b.session.prompt().step.clone().into_owned();
        let revision = b.doc.revision();
        let count = b.doc.len();
        let values = (
            b.memory.hatch_scale,
            b.memory.hatch_angle,
            b.memory.hatch_gap,
            b.memory.hatch_color2,
            b.memory.hatch_transparency,
        );
        assert!(
            b.session
                .prompt()
                .options
                .iter()
                .any(|o| o.key == "Enter" && o.label == "Değiştirmeden dön")
        );
        assert!(b.type_text("geçersiz değer"));
        b.move_to(-20.0, 1.0);
        let preview = b.session.preview(&Default::default()).unwrap();
        assert!(
            preview.areas.is_empty() && preview.hatch.is_empty(),
            "{key}"
        );
        b.click(-20.0, 1.0);
        assert_eq!(b.doc.revision(), revision, "{key}: a value is not a region");
        assert_eq!(b.doc.len(), count);
        assert_eq!(b.session.prompt().step.as_ref(), step);
        assert_eq!(b.last_level(), Some(Level::Warn));
        b.confirm();
        assert!(
            b.session.is_running(),
            "one confirm returns to region picking"
        );
        assert_eq!(
            (
                b.memory.hatch_scale,
                b.memory.hatch_angle,
                b.memory.hatch_gap,
                b.memory.hatch_color2,
                b.memory.hatch_transparency,
            ),
            values
        );
        assert_eq!(b.doc.revision(), revision);
        b.click(-20.0, 1.0);
        assert_eq!(b.doc.len(), count + 1);
        assert!(!b.undo_step());
        assert_eq!(b.doc.undo().as_deref(), Some("Tarama"));
        assert_eq!(b.doc.len(), count);
    }
}

#[test]
fn cancelling_a_hatch_value_returns_without_a_write_or_an_undo_step() {
    let mut b = bench();
    let revision = b.doc.revision();
    assert!(b.type_text("Ö"));
    assert!(b.type_text("0"));
    b.click(-20.0, 1.0);
    assert_eq!(b.doc.revision(), revision);
    assert!(b.run(|s, cx| s.cancel(cx)));
    assert!(b.session.is_running());
    assert_eq!(b.doc.revision(), revision);
    assert!(!b.undo_step());
    assert!(b.doc.undo().is_none());
    // A valid value resumes the ordinary one-click / one-undo flow.
    assert!(b.type_text("Ö"));
    assert!(b.type_text("2"));
    b.click(-20.0, 1.0);
    assert_eq!(b.memory.hatch_scale, 2.0);
    assert_eq!(b.doc.undo().as_deref(), Some("Tarama"));
}

#[test]
fn an_origin_click_still_answers_the_point_without_filling_the_region() {
    let mut b = bench();
    assert!(b.run(|s, cx| s.choose_option("S", "nokta", cx)));
    let revision = b.doc.revision();
    b.click(-20.0, 1.0);
    assert_eq!(b.doc.revision(), revision);
    let point = b.memory.hatch_origin_at.expect("the chosen origin");
    assert!(near(point.x, E - 20.0) && near(point.y, N + 1.0));
    assert_eq!(
        b.session.prompt().text(),
        PROMPT.replace("Başlangıç (S): dünya", "Başlangıç (S): nokta")
    );
    b.click(-20.0, 1.0);
    assert_eq!(b.doc.undo().as_deref(), Some("Tarama"));
}

/// The newest object's ring's box, relative, its hole count, pattern and area.
fn newest(b: &Bench) -> ([f64; 4], usize, (HatchPatternType, f64, f64), f64) {
    let Entity::Hatch(h) = b.newest() else {
        panic!("a hatch: {:?}", b.newest());
    };
    let xs = h.ring.iter().map(|p| rel(*p));
    let bx = xs.fold([f64::MAX, f64::MAX, f64::MIN, f64::MIN], |b, [x, y]| {
        [b[0].min(x), b[1].min(y), b[2].max(x), b[3].max(y)]
    });
    let area = kentos_interaction::measures(b.newest()).0.expect("an area");
    (
        bx,
        h.holes.as_ref().map_or(0, Vec::len),
        (h.pattern.kind, h.pattern.angle, h.pattern.spacing),
        area,
    )
}

fn near(have: f64, want: f64) -> bool {
    (have - want).abs() < 1e-6
}

#[test]
fn a_click_in_a_parcel_leaves_its_building_out() {
    let mut b = bench();
    assert_eq!(b.session.prompt().text(), PROMPT);
    let count = b.doc.entities().count();
    b.click(-12.0, 8.0);
    assert_eq!(b.doc.entities().count(), count + 1);
    let (bx, holes, pattern, area) = newest(&b);
    assert_eq!(bx, [-28.0, -6.0, -8.0, 10.0]);
    assert_eq!(holes, 1);
    // ANSI31 of the library, its paper millimetres metres at 1:1000 (docs/adr/0186 §4).
    assert_eq!(pattern, (HatchPatternType::Pattern, 0.0, 1.0));
    assert!(near(area, 320.0 - 48.0), "{area}");
    assert_eq!(b.last_level(), Some(Level::Success));
    assert_eq!(
        b.last_text(),
        Some("ANSI31 tarama eklendi: 272.00 m², 1 ada taranmadı")
    );
    // It follows the parcel and its building (docs/adr/0186 §6).
    let Entity::Hatch(h) = b.newest() else {
        unreachable!()
    };
    let tie = h.assoc.as_ref().expect("a tie");
    assert_eq!(tie.islands.len(), 1);
    assert!(tie.cutouts.is_empty());
    // On the active layer; the tool waits for the next one.
    assert_eq!(b.newest().base().layer_id, "cizim");
    assert!(b.session.is_running());
    assert_eq!(b.session.prompt().text(), PROMPT);
    // Ctrl+Z is the drawing's: one step, “Tarama”.
    assert!(!b.undo_step());
    assert_eq!(b.doc.undo().as_deref(), Some("Tarama"));
    assert_eq!(b.doc.entities().count(), count);
}

#[test]
fn inside_the_building_only_the_building() {
    let mut b = bench();
    b.click(-20.0, 1.0);
    let (bx, holes, _, area) = newest(&b);
    assert_eq!((bx, holes), ([-24.0, -2.0, -16.0, 4.0], 0));
    assert!(near(area, 48.0), "{area}");
    assert_eq!(b.last_text(), Some("ANSI31 tarama eklendi: 48.00 m²"));
}

#[test]
fn islands_off_fill_the_whole_parcel() {
    let mut b = bench();
    assert!(b.type_text("a"));
    assert!(
        b.session
            .prompt()
            .text()
            .contains("/ Adalar (A): taranır /")
    );
    b.click(-12.0, 8.0);
    let (_, holes, _, area) = newest(&b);
    assert_eq!(holes, 0);
    assert!(near(area, 320.0), "{area}");
    assert_eq!(b.last_text(), Some("ANSI31 tarama eklendi: 320.00 m²"));
}

#[test]
fn no_closed_object_around_the_click_is_said() {
    let mut b = bench();
    let count = b.doc.entities().count();
    // Inside the line work: closed by lines, not by an object.
    b.click(8.0, -3.0);
    assert_eq!(b.doc.entities().count(), count);
    assert_eq!(b.last_level(), Some(Level::Warn));
    assert_eq!(
        b.last_text(),
        Some(
            "Tıklanan noktayı çevreleyen kapalı bir alan, daire ya da kapalı eğri yok. Çizgilerle çevrili yerler için “Sınır: çizgiler” seçin."
        )
    );
}

#[test]
fn by_lines_the_face_around_the_click_with_its_island() {
    let mut b = bench();
    assert!(b.type_text("B"));
    assert_eq!(
        b.session.prompt().text(),
        "Tarama: taranacak yerin içine tıklayın [Desen (D): ANSI31 / Ölçek (Ö): 1 / Açı (Ç): 0° / Ölçek tabanı (E): kâğıtta / Başlangıç (S): dünya / Ölçekli (L): kapalı / Saydamlık (M): %0 / Sıra (O): atama / Özellik al (Z) / Sınır (B): çizgiler / Adalar (A): taranmaz / İlişkili (İ): açık / Yazılar (Y): taranır / Sınır katmanı (K): tümü / Boşluk (G): 0]"
    );
    b.click(8.0, -3.0);
    let (bx, holes, _, area) = newest(&b);
    assert_eq!((bx, holes), ([2.0, -6.0, 14.0, 10.0], 1));
    assert!(near(area, 192.0 - 16.0), "{area}");
    assert_eq!(
        b.last_text(),
        Some("ANSI31 tarama eklendi: 176.00 m², 1 ada taranmadı")
    );
    // Tied to the line work closing its face, its island too (docs/adr/0328 §2).
    let Entity::Hatch(h) = b.newest() else {
        unreachable!()
    };
    let tie = h.assoc.as_ref().expect("tied");
    assert!(
        tie.outer.is_none() && !tie.bounds.is_empty() && !tie.islands.is_empty(),
        "{tie:?}"
    );
    // Outside every face.
    b.click(0.0, 18.0);
    assert_eq!(
        b.last_text(),
        Some(
            "Tıklanan yer çizgilerle kapalı bir bölgenin içinde değil; görünüm dışındaki çizgiler sayılmaz."
        )
    );
}

#[test]
fn the_boundary_layer_is_picked_by_one_of_its_objects() {
    let mut b = bench();
    // Only with the line work.
    assert!(!b.type_text("K"));
    assert!(b.type_text("B"));
    assert!(b.type_text("K"));
    assert_eq!(
        b.session.prompt().text(),
        "Tarama: sınır olacak katmandan bir nesneye tıklayın [Tüm katmanlar (K)]"
    );
    b.click(0.0, 18.0);
    assert_eq!(
        b.last_text(),
        Some("Sınır katmanını seçmek için bir nesneye tıklayın.")
    );
    // The object under the cursor is highlighted: the left line.
    b.move_to(2.0, 1.0);
    assert_eq!(b.selection.hover().map(|s| s.0), Some(6));
    b.click(2.0, 1.0);
    assert_eq!(b.selection.hover(), None);
    assert!(
        b.session
            .prompt()
            .text()
            .ends_with("/ Sınır katmanı (K): Çizim / Boşluk (G): 0]")
    );
    // The closed polyline is on Yapılar: no island now.
    let count = b.doc.entities().count();
    b.click(8.0, -3.0);
    assert_eq!(b.doc.entities().count(), count + 1);
    let (_, holes, _, area) = newest(&b);
    assert_eq!(holes, 0);
    assert!(near(area, 192.0), "{area}");
    // K again: every visible layer.
    assert!(b.type_text("K"));
    assert!(
        b.session
            .prompt()
            .text()
            .ends_with("/ Sınır katmanı (K): tümü / Boşluk (G): 0]")
    );
    // Esc leaves the picking first, then the tool.
    assert!(b.type_text("K"));
    assert!(b.run(|s, cx| s.cancel(cx)));
    assert!(
        b.session
            .prompt()
            .text()
            .ends_with("/ Sınır katmanı (K): tümü / Boşluk (G): 0]")
    );
    assert!(!b.run(|s, cx| s.cancel(cx)));
    assert!(!b.session.is_running());
}

#[test]
fn patterns_cycle_by_their_key_or_go_by_their_name() {
    let mut b = bench();
    // D the next of the library; a name typed, that one (docs/adr/0186 §4).
    for name in ["ANSI32", "ANSI33"] {
        assert!(b.type_text("D"));
        assert!(
            b.session
                .prompt()
                .text()
                .contains(&format!("[Desen (D): {name} /")),
            "{}",
            b.session.prompt().text()
        );
    }
    assert!(b.type_text("dolu"));
    assert!(
        b.session
            .prompt()
            .text()
            .starts_with("Tarama: taranacak yerin içine tıklayın [Desen (D): Dolu / Saydamlık (M): %0 / Sıra (O): atama / Özellik al (Z) / Sınır (B):"),
        "{}",
        b.session.prompt().text()
    );
    b.click(-20.0, 1.0);
    let (_, _, pattern, _) = newest(&b);
    assert_eq!(pattern, (HatchPatternType::Solid, 0.0, 1.0));
    assert_eq!(b.last_text(), Some("Dolu tarama eklendi: 48.00 m²"));
    assert!(b.type_text("D"));
    assert_eq!(b.session.prompt().text(), PROMPT);
    // Kept for the next run; the boundary layer is the run's.
    assert!(b.type_text("çapraz"));
    assert!(b.type_text("B"));
    b.confirm();
    assert!(!b.session.is_running());
    b.start("hatch");
    assert_eq!(
        b.session.prompt().text(),
        "Tarama: taranacak yerin içine tıklayın [Desen (D): Çapraz / Ölçek (Ö): 1 / Açı (Ç): 0° / Ölçek tabanı (E): kâğıtta / Başlangıç (S): dünya / Ölçekli (L): kapalı / Saydamlık (M): %0 / Sıra (O): atama / Özellik al (Z) / Sınır (B): çizgiler / Adalar (A): taranmaz / İlişkili (İ): açık / Yazılar (Y): taranır / Sınır katmanı (K): tümü / Boşluk (G): 0]"
    );
}

#[test]
fn a_locked_active_layer_writes_nothing() {
    let mut b = bench();
    assert!(b.doc.set_active_layer("kilitli"), "the locked layer");
    let count = b.doc.entities().count();
    b.click(-12.0, 8.0);
    assert_eq!(b.doc.entities().count(), count);
    assert_eq!(b.last_level(), Some(Level::Warn));
    assert_eq!(
        b.last_text(),
        Some(
            "“Kilitli katman” katmanı kilitli. Kilidi Katmanlar panelinden açın ya da başka bir katmanı etkinleştirin."
        )
    );
}

#[test]
fn too_dense_a_pattern_is_refused() {
    let mut b = bench();
    // A 100 km square around everything: 3 m apart, far too many lines.
    let far = |x: f64, y: f64| Wire { x: E + x, y: N + y };
    let square = Entity::Polygon(PathEntity {
        space_separator: None,
        ms: None,
        base: EntityBase {
            look: Default::default(),
            id: 0,
            layer_id: "cizim".to_owned(),
            color: None,
            attrs: BTreeMap::new(),
            label: None,
            symbol: None,
            line_weight: None,
            label_pins: Vec::new(),
            attachments: Vec::new(),
        },
        pts: vec![
            far(-50_000.0, -50_000.0),
            far(50_000.0, -50_000.0),
            far(50_000.0, 50_000.0),
            far(-50_000.0, 50_000.0),
        ],
        bulges: None,
        holes: None,
        zs: None,
        parts: None,
        closed: false,
        widths: None,
    });
    b.doc.add(square).expect("a slot");
    let count = b.doc.entities().count();
    b.click(0.0, 30.0);
    assert_eq!(b.doc.entities().count(), count);
    assert_eq!(
        b.last_text(),
        Some(
            "Desen bu bölge için çok sık; ölçeği ya da çizim ölçeğini büyütün ya da başka bir desen seçin."
        )
    );
    // Solid has no lines.
    assert!(b.type_text("dolu"));
    b.click(0.0, 30.0);
    assert_eq!(b.doc.entities().count(), count + 1);
}

/// A multi-part parcel (docs/adr/0143): the parcel gets a 10 × 10 m part
/// away from everything; a click in it fills that part alone, and a click in
/// the first part still leaves the building out.
#[test]
fn a_click_in_a_part_of_a_multi_part_parcel_fills_that_part() {
    let mut b = bench();
    // The parcel: the largest area of the drawing.
    let (slot, parcel) = (1..=b.doc.entities().count() as u32 + 8)
        .map(kentos_domain::Slot)
        .filter_map(|s| match b.doc.get(s) {
            Some(e @ Entity::Polygon(p)) => {
                Some((kentos_interaction::measures(e).0?, s, p.clone()))
            }
            _ => None,
        })
        .max_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, s, p)| (s, p))
        .expect("the parcel");
    let at = |x: f64, y: f64| Wire { x: E + x, y: N + y };
    let mut two = parcel;
    two.parts = Some(vec![kentos_commands::AreaPart {
        ms: None,
        pts: vec![
            at(40.0, 20.0),
            at(50.0, 20.0),
            at(50.0, 30.0),
            at(40.0, 30.0),
        ],
        bulges: None,
        holes: None,
        zs: None,
    }]);
    assert!(b.doc.update(slot, Entity::Polygon(two)));
    b.click(45.0, 25.0);
    let (bx, holes, _, area) = newest(&b);
    assert_eq!((bx, holes), ([40.0, 20.0, 50.0, 30.0], 0));
    assert!(near(area, 100.0), "{area}");
    b.click(-12.0, 8.0);
    let (bx, holes, _, area) = newest(&b);
    assert_eq!((bx, holes), ([-28.0, -6.0, -8.0, 10.0], 1));
    assert!(near(area, 272.0), "{area}");
}

#[test]
fn editor_reselect_exposes_boundary_and_region_options_and_preserves_undo() {
    let mut b = bench();
    b.draft.snap = false;
    b.memory.hatch_assoc = false;
    b.click(-20.0, 1.0);
    let original = b.newest().clone();
    b.start("hatchEdit");
    assert!(b.type_text("S"));
    let keys = b.options();
    for key in ["B", "A", "İ", "Y"] { assert!(keys.contains(&key), "{keys:?}"); }
    assert!(b.type_text("B"));
    assert!(b.memory.hatch_by_lines);
    assert!(b.options().contains(&"G"));
    assert!(b.type_text("İ"));
    assert!(b.memory.hatch_assoc);
    b.click(-20.0, 1.0);
    let before = b.doc.to_snapshot_v2();
    b.click(8.0, -3.0);
    assert_eq!(b.last_level(), Some(Level::Success), "{:?}", b.last_text());
    let Entity::Hatch(h) = b.doc.get(kentos_domain::Slot(original.base().id)).unwrap() else { panic!("hatch") };
    assert!(h.assoc.is_some());
    assert_eq!(h.pattern, match &original { Entity::Hatch(h) => h.pattern.clone(), _ => unreachable!() });
    assert_eq!(b.doc.undo().as_deref(), Some("Sınırı yeniden seç"));
    assert_eq!(b.doc.to_snapshot_v2(), before);
}

#[test]
fn editor_gap_input_does_not_reselect_on_a_canvas_click_or_invalid_number() {
    let mut b = bench();
    b.draft.snap = false;
    b.click(-20.0, 1.0);
    b.start("hatchEdit");
    assert!(b.type_text("S"));
    b.click(-20.0, 1.0);
    assert!(b.type_text("B"));
    assert!(b.type_text("G"));
    let before = b.doc.to_snapshot_v2();
    assert!(b.type_text("-1"));
    b.click(8.0, -3.0);
    assert_eq!(b.doc.to_snapshot_v2(), before);
    assert_eq!(b.memory.hatch_gap, 0.);
    assert!(b.session.prompt().text().contains("boşluk"));
    assert!(b.type_text("0.004"));
    assert_eq!(b.memory.hatch_gap, 0.004);
    assert_eq!(b.doc.to_snapshot_v2(), before);
    assert!(b.type_text("G"));
    b.confirm();
    assert!(b.session.prompt().text().contains("yeni bölgesinin"));
    assert_eq!(b.doc.to_snapshot_v2(), before);
    assert!(b.type_text("G"));
    assert!(b.run(|s, cx| s.cancel(cx)));
    assert!(b.session.prompt().text().contains("yeni bölgesinin"));
}

#[test]
fn existing_hatch_origin_moves_by_point_or_coordinate_without_resetting_its_pattern() {
    let mut b = bench();
    b.draft.snap = false;
    b.click(-20.0, 1.0);
    let original = b.newest().clone();
    let id = kentos_domain::Slot(original.base().id);
    b.start("hatchEdit");
    assert!(b.type_text("N"));
    assert!(b.options().contains(&"N"));
    b.click(-20.0, 1.0);
    let before = b.doc.to_snapshot_v2();
    b.move_to(-21.0, 2.0);
    assert_eq!(b.doc.to_snapshot_v2(), before, "origin preview is not a write");
    b.click(-21.0, 2.0);
    let Entity::Hatch(h) = b.doc.get(id).unwrap() else { panic!("hatch") };
    let Entity::Hatch(mut expected) = original.clone() else { unreachable!() };
    expected.pattern.origin = Some(Wire { x: E - 21., y: N + 2. });
    assert_eq!(h, &expected);
    assert_eq!(b.doc.undo().as_deref(), Some("Tarama başlangıcı"));
    assert_eq!(b.doc.to_snapshot_v2(), before);
    b.spatial.sync(&b.doc);
    assert!(b.type_text(&format!("{},{}", E - 22., N + 3.)));
    let Entity::Hatch(h) = b.doc.get(id).unwrap() else { panic!("hatch") };
    expected.pattern.origin = Some(Wire { x: E - 22., y: N + 3. });
    assert_eq!(h, &expected);
    assert_eq!(b.doc.undo().as_deref(), Some("Tarama başlangıcı"));
    assert_eq!(b.doc.to_snapshot_v2(), before);
}

#[test]
fn trim_reaches_a_circle_beneath_its_hatch_and_one_undo_restores_the_tie() {
    use kentos_commands::CallOp;
    use kentos_domain::Slot;
    use kentos_native_application::{ExecutionContext, registry};
    use serde_json::json;
    let mut raw: serde_json::Value = serde_json::from_str(HATCH).unwrap();
    raw["entities"] = json!([
        {"kind":"circle","id":1,"layerId":"cizim","attrs":{},"c":{"x":E,"y":N},"r":5.},
        {"kind":"line","id":2,"layerId":"cizim","attrs":{},"a":{"x":E,"y":N-6.},"b":{"x":E,"y":N+6.}}
    ]);
    let mut b = Bench::on(&raw.to_string());
    b.draft.snap = false;
    let made = registry::run(&mut ExecutionContext::new(&mut b.doc), "cad.entities.hatch", CallOp::Execute,
        json!({"region":{"kind":"point","at":{"x":E+1.,"y":N+1.},"associative":true},"pattern":{"name":"ANSI31"},"layerId":"cizim"})).unwrap();
    assert_eq!(made["status"], "completed", "{made}");
    let hatch_uid = kentos_domain::Uuid::parse_str(made["output"]["created"][0].as_str().unwrap()).unwrap();
    let hatch_slot = b.doc.slot_of(hatch_uid).unwrap();
    let Entity::Hatch(original) = b.doc.get(hatch_slot).unwrap().clone() else { panic!("hatch") };
    let before = b.doc.to_snapshot_v2();
    b.start("trim");
    assert!(b.type_text("S"));
    b.click(0., -6.);
    b.confirm();
    b.move_to(5., 0.);
    assert!(b.session.preview(&Default::default()).unwrap().strokes.iter().any(|s| s.tone == kentos_interaction::Tone::Danger));
    b.click(5., 0.);
    assert!(matches!(b.doc.get(Slot(1)), Some(Entity::Arc(_))), "source must be trimmed instead of refusing the overlaid hatch: {:?}", b.last_text());
    let Entity::Hatch(now) = b.doc.get(hatch_slot).unwrap() else { panic!("hatch") };
    assert!(now.assoc.is_none(), "open source cannot silently remain associative");
    let mut expected = original;
    expected.assoc = None;
    assert_eq!(now, &expected, "the frozen hatch keeps its analytic geometry and pattern");
    assert_eq!(b.doc.undo().as_deref(), Some("Buda"));
    assert_eq!(b.doc.to_snapshot_v2(), before);
}

/// UR-056: a project pattern chosen, the hatch is written with it and the message names it, not the library's
/// pattern the session held before.
#[test]
fn a_project_pattern_is_named_in_the_success_message() {
    let mut b = bench();
    let mut settings = b.doc.settings().clone();
    settings.hatch_patterns = vec![
        serde_json::from_value(serde_json::json!({
            "name": "QA_HATCH",
            "description": "Test horizontal",
            "lines": [{"angle": 0, "origin": [0, 0], "offset": [0, 1]}]
        }))
        .unwrap(),
    ];
    b.doc.set_settings(settings);
    assert!(b.type_text("QA_HATCH"));
    b.click(-20.0, 1.0);
    let Entity::Hatch(h) = b.newest() else {
        panic!("a hatch: {:?}", b.newest());
    };
    assert_eq!(h.pattern.name.as_deref(), Some("QA_HATCH"));
    assert_eq!(b.last_text(), Some("QA_HATCH tarama eklendi: 48.00 m²"));
    // Back to a library pattern by its name: that one is named again.
    assert!(b.type_text("ansi32"));
    b.click(-20.0, 1.0);
    assert!(b.last_text().is_some_and(|t| t.starts_with("ANSI32 tarama eklendi")), "{:?}", b.last_text());
}
