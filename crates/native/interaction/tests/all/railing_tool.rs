//! Korkuluk (docs/adr/8045 §3): its line clicked point by point and Enter, or round a gallery's void with one click,
//! written on Etkin kat on its EK-4 layer (MM-KORK-CUBUK), 1.10 m high with a 0.10 m gap unless told (PAİY 38/1, 38/4);
//! a lower one is written and said.

use crate::common::{Bench, E, N};
use kentos_commands::{CallOp, DrawingUnit, Entity, RailingEntity, RailingInfill, Workspace};
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, registry};
use serde_json::json;

/// Zemin kat and 1. kat 3 m apart; 1. kat's floor slab (10 × 8 m) with a 4 × 3 m gallery void from (3, 2) to (7, 5).
/// 1. kat is the active level.
fn gallery() -> Bench {
    let mut b = Bench::new("line");
    b.session.exit();
    let mut s = b.doc.settings().clone();
    s.srid = 0;
    s.workspace = Some(Workspace::Cad);
    s.drawing_unit = Some(DrawingUnit::M);
    s.arch = Some(
        serde_json::from_value(json!({"levels": [
            {"id": "zemin", "name": "Zemin kat", "elevation": 0.0, "height": 3.0},
            {"id": "kat-1", "name": "1. kat", "elevation": 3.0, "height": 3.0}
        ]}))
        .unwrap(),
    );
    b.doc.set_settings(s);
    let p = |x: f64, y: f64| json!({"x": E + x, "y": N + y});
    let answer = registry::run(
        &mut ExecutionContext::new(&mut b.doc),
        "cad.slabs.create",
        CallOp::Execute,
        json!({"layerId": "cizim", "slabs": [{
            "parts": [{"outer": {"pts": [p(0.0, 0.0), p(10.0, 0.0), p(10.0, 8.0), p(0.0, 8.0)]},
                       "holes": [{"pts": [p(3.0, 2.0), p(7.0, 2.0), p(7.0, 5.0), p(3.0, 5.0)]}]}],
            "arch": {"level": "kat-1"}
        }]}),
    )
    .expect("runs");
    assert_eq!(answer["status"], "completed", "{answer}");
    b.level = Some("kat-1".into());
    b
}

fn newest_railing(b: &Bench) -> RailingEntity {
    match b.newest() {
        Entity::Railing(r) => r.clone(),
        e => panic!("a railing: {e:?}"),
    }
}

fn rel(r: &RailingEntity) -> Vec<(f64, f64)> {
    r.railing
        .pts
        .iter()
        .map(|p| {
            (
                ((p.x - E) * 1e6).round() / 1e6,
                ((p.y - N) * 1e6).round() / 1e6,
            )
        })
        .collect()
}

#[test]
fn clicks_and_enter_write_a_railing_on_its_layer_and_its_first_point_closes_one() {
    let mut b = gallery();
    b.memory.layer_keys = true;
    b.start("railing");
    assert_eq!(b.options(), ["S", "Y", "D", "A", "E", "F", "Enter"]);
    b.click(0.0, 9.0);
    b.click(4.0, 9.0);
    b.click(4.0, 12.0);
    assert_eq!(b.options(), ["S", "Y", "D", "A", "E", "F", "K", "Enter"]);
    b.confirm();
    let r = newest_railing(&b);
    assert_eq!(rel(&r), [(0.0, 9.0), (4.0, 9.0), (4.0, 12.0)]);
    let f = &r.railing;
    assert_eq!(
        (f.closed, f.height, f.width, f.infill, f.gap, f.offset),
        (false, 1.1, 0.05, RailingInfill::Balusters, None, 0.0)
    );
    assert_eq!(f.arch.level.as_deref(), Some("kat-1"));
    assert_eq!(
        b.doc
            .layers()
            .get(&r.base.layer_id)
            .map(|l| l.name.as_str()),
        Some("MM-KORK-CUBUK")
    );
    assert_eq!(b.last_level(), Some(Level::Success));
    assert!(
        b.last_text()
            .is_some_and(|t| t.starts_with("Korkuluk çizildi:") && t.contains("dikey çubuk")),
        "{:?}",
        b.last_text()
    );
    assert_eq!(b.doc.undo().as_deref(), Some("Korkuluk"));
    // Its first point again closes it round: glass, on its own layer.
    assert!(b.run(|s, cx| s.configure("D", "glass", cx)));
    b.click(0.0, 9.0);
    b.click(4.0, 9.0);
    b.click(4.0, 12.0);
    b.click(0.0, 9.0);
    let r = newest_railing(&b);
    assert!(r.railing.closed);
    assert_eq!(r.railing.pts.len(), 3);
    assert_eq!(r.railing.infill, RailingInfill::Glass);
    assert_eq!(
        b.doc
            .layers()
            .get(&r.base.layer_id)
            .map(|l| l.name.as_str()),
        Some("MM-KORK-CAM")
    );
}

/// Boşluktan: a click inside the gallery's void writes a closed railing round it, its top rail's outer face on the
/// void's edge: its line 2.5 cm out onto the slab.
#[test]
fn a_click_inside_a_gallery_s_void_puts_a_railing_round_it_on_the_slab() {
    let mut b = gallery();
    b.start("railing");
    assert!(b.run(|s, cx| s.configure("S", "void", cx)));
    assert_eq!(b.options(), ["S", "Y", "D", "A", "E", "F"]);
    b.click(5.0, 3.5);
    let r = newest_railing(&b);
    assert!(r.railing.closed);
    assert_eq!(
        rel(&r),
        [
            (2.975, 1.975),
            (7.025, 1.975),
            (7.025, 5.025),
            (2.975, 5.025)
        ]
    );
    assert!(
        (r.railing.length() - 2.0 * (4.05 + 3.05)).abs() < 1e-6,
        "{}",
        r.railing.length()
    );
    // Outside any void: nothing written, said why.
    let before = b.doc.len();
    b.click(8.5, 7.0);
    assert_eq!(b.doc.len(), before);
    assert!(
        b.last_text()
            .is_some_and(|t| t.starts_with("Burada döşeme boşluğu yok")),
        "{:?}",
        b.last_text()
    );
}

/// PAİY 38/1 and 38/4: a railing lower than 1.10 m or with a gap wider than 0.10 m is written and said in red.
#[test]
fn a_railing_lower_than_paiy_s_1_10_m_is_written_and_warned_of() {
    let mut b = gallery();
    b.start("railing");
    assert!(b.run(|s, cx| s.configure("Y", "0.9", cx)));
    assert!(b.run(|s, cx| s.configure("A", "0.12", cx)));
    b.click(0.0, 9.0);
    b.move_to(4.0, 9.0);
    let preview = b.run(|s, cx| s.preview(&cx.format())).expect("its preview");
    let tag = preview.tag.expect("its tag").lines;
    assert!(tag.iter().any(|l| l.starts_with("PAİY 38/1")), "{tag:?}");
    assert!(tag.iter().any(|l| l.starts_with("PAİY 38/4")), "{tag:?}");
    assert_eq!(preview.tag_tone, kentos_interaction::Tone::Danger);
    b.click(4.0, 9.0);
    b.confirm();
    let r = newest_railing(&b);
    assert_eq!((r.railing.height, r.railing.gap), (0.9, Some(0.12)));
    assert_eq!(b.last_level(), Some(Level::Warn));
}

/// Boşluktan on a stair's void: the sides the walls run along get none, and the stretch where the stair from Zemin kat
/// arrives stays open: the north side whole, the south side in two either side of the stair's 1.20 m.
#[test]
fn a_void_s_walled_sides_and_a_stair_s_arrival_stay_open() {
    let mut b = gallery();
    let p = |x: f64, y: f64| json!({"x": E + x, "y": N + y});
    for (name, input) in [
        (
            "cad.walls.create",
            json!({"layerId": "cizim", "walls": [
                {"pts": [p(2.9, 1.0), p(2.9, 6.0)], "width": 0.2, "justify": "center", "arch": {"level": "kat-1"}},
                {"pts": [p(7.1, 1.0), p(7.1, 6.0)], "width": 0.2, "justify": "center", "arch": {"level": "kat-1"}}
            ]}),
        ),
        (
            "cad.stairs.create",
            json!({"layerId": "cizim", "stairs": [{
                "pts": [p(6.1, 6.76), p(6.1, 2.0)], "width": 1.2, "risers": 18, "riserHeight": 0.1667, "tread": 0.28,
                "arch": {"level": "zemin"}
            }]}),
        ),
    ] {
        let answer = registry::run(
            &mut ExecutionContext::new(&mut b.doc),
            name,
            CallOp::Execute,
            input,
        )
        .expect("runs");
        assert_eq!(answer["status"], "completed", "{name}: {answer}");
    }
    b.start("railing");
    assert!(b.run(|s, cx| s.configure("S", "void", cx)));
    let before = b.doc.len();
    b.click(5.0, 3.5);
    assert_eq!(b.doc.len(), before + 3, "three stretches, one undo step");
    let mut lines: Vec<Vec<(f64, f64)>> = b
        .doc
        .entities()
        .filter_map(|e| match e {
            Entity::Railing(r) => Some(rel(r)),
            _ => None,
        })
        .collect();
    lines.sort_by(|a, b| a[0].0.total_cmp(&b[0].0).then(a[0].1.total_cmp(&b[0].1)));
    assert_eq!(
        lines,
        [
            vec![(3.0, 1.975), (5.5, 1.975)],
            vec![(6.7, 1.975), (7.0, 1.975)],
            vec![(7.0, 5.025), (3.0, 5.025)],
        ]
    );
    assert!(
        b.last_text()
            .is_some_and(|t| t.starts_with("Korkuluk çizildi: 3 parça, toplam")),
        "{:?}",
        b.last_text()
    );
    assert_eq!(b.doc.undo().as_deref(), Some("Korkuluk"));
    assert_eq!(b.doc.len(), before);
}
