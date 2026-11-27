//! Duvar profili (`tool.moulding`, `tool.coping`, docs/adr/8173 §5): a click inside a building's walls runs a facade
//! moulding round their outer faces, out of them; a click on a wall puts a coping on its top, past each face. Each a
//! beam's band that is a covering, on its EK-2 layer (MM-CEPH-SILME, MM-HARP-GENEL).

use crate::common::{Bench, E, N};
use kentos_commands::{CallOp, DrawingUnit, Entity, WallJustify, Workspace};
use kentos_native_application::{ExecutionContext, registry};
use serde_json::json;

/// Zemin kat 3 m high; 25 cm walls whose outer faces make a 12 × 9 m outline and a 10 cm partition at x 5. Zemin kat
/// is the active level.
fn building(tool: &str) -> Bench {
    let mut b = Bench::new("line");
    b.session.exit();
    let mut s = b.doc.settings().clone();
    s.srid = 0;
    s.workspace = Some(Workspace::Cad);
    s.drawing_unit = Some(DrawingUnit::M);
    s.arch = Some(
        serde_json::from_value(json!({"levels": [{"id": "zemin", "name": "Zemin kat", "elevation": 0.0, "height": 3.0}]}))
            .unwrap(),
    );
    b.doc.set_settings(s);
    let p = |x: f64, y: f64| json!({"x": E + x, "y": N + y});
    let answer = registry::run(
        &mut ExecutionContext::new(&mut b.doc),
        "cad.walls.create",
        CallOp::Execute,
        json!({"layerId": "cizim", "walls": [
            {"pts": [p(0.125, 0.125), p(11.875, 0.125), p(11.875, 8.875), p(0.125, 8.875), p(0.125, 0.125)],
             "width": 0.25, "justify": "center", "arch": {"level": "zemin"}},
            {"pts": [p(5.0, 0.25), p(5.0, 8.75)], "width": 0.1, "justify": "center", "arch": {"level": "zemin"}}
        ]}),
    )
    .expect("runs");
    assert_eq!(answer["status"], "completed", "{answer}");
    b.level = Some("zemin".into());
    b.memory.layer_keys = true;
    b.start(tool);
    b
}

fn newest_profile(b: &Bench) -> kentos_commands::BeamFields {
    let Entity::Beam(k) = b.newest().clone() else {
        panic!("a profile is a beam's band")
    };
    *k.beam
}

fn layer_of(b: &Bench) -> String {
    let id = b.newest().base().layer_id.clone();
    b.doc.layers().get(&id).map(|l| l.name.clone()).unwrap_or_default()
}

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

#[test]
fn a_click_inside_the_walls_runs_a_moulding_round_their_outer_faces() {
    let mut b = building("moulding");
    assert_eq!(b.session.tool_id(), "moulding");
    b.click(2.0, 4.0);
    let k = newest_profile(&b);
    assert_eq!(k.covering, Some(kentos_commands::Covering::facade_moulding()));
    assert_eq!((k.width, k.height, k.base_offset), (0.05, 0.2, Some(2.8)));
    // Round the 12 × 9 outline once, its ends meeting on a straight edge, its body out of it.
    assert_eq!(k.pts.first(), k.pts.last());
    assert_eq!(k.justify, WallJustify::Left);
    let xs: Vec<f64> = k.pts.iter().map(|p| p.x - E).collect();
    let ys: Vec<f64> = k.pts.iter().map(|p| p.y - N).collect();
    let min = |v: &[f64]| v.iter().copied().fold(f64::MAX, f64::min);
    assert!(near(min(&xs), 0.0) && near(min(&ys), 0.0), "{xs:?} {ys:?}");
    assert!(near(xs.iter().copied().fold(f64::MIN, f64::max), 12.0), "{xs:?}");
    assert!(near(ys.iter().copied().fold(f64::MIN, f64::max), 9.0), "{ys:?}");
    let area: f64 = k.pts.windows(2).map(|w| (w[0].x - E) * (w[1].y - N) - (w[1].x - E) * (w[0].y - N)).sum();
    assert!(area > 0.0, "counter-clockwise: its right, where it lies, is out of the building");
    assert_eq!(layer_of(&b), "MM-CEPH-SILME");
    assert_eq!(b.last_text(), Some("Cephe silmesi çizildi: 5/20."));
    assert_eq!(k.arch.level.as_deref(), Some("zemin"));
}

#[test]
fn a_click_on_a_wall_puts_a_coping_on_its_top_past_each_face() {
    let mut b = building("coping");
    assert_eq!(b.session.tool_id(), "coping");
    // Off any wall: nothing written, said why.
    let before = b.doc.len();
    b.click(8.0, 4.0);
    assert_eq!(b.doc.len(), before);
    // The partition: 10 cm, its top at the storey's 3.00.
    b.click(5.0, 4.0);
    let k = newest_profile(&b);
    assert_eq!(k.covering, Some(kentos_commands::Covering::coping()));
    assert!(near(k.width, 0.2) && near(k.height, 0.05), "{} {}", k.width, k.height);
    assert_eq!(k.base_offset, Some(3.0));
    assert_eq!(k.justify, WallJustify::Center);
    let ends: Vec<(f64, f64)> = k.pts.iter().map(|p| (p.x - E, p.y - N)).collect();
    assert_eq!(ends, vec![(5.0, 0.25), (5.0, 8.75)]);
    assert_eq!(layer_of(&b), "MM-HARP-GENEL");
    assert_eq!(b.last_text(), Some("Duvar harpuştası kondu: 5 cm taşma, 5 cm."));
    // A stone coping: its material names its layer.
    assert!(b.run(|s, cx| s.configure("M", "Taş", cx)));
    b.click(11.875, 4.0);
    let k = newest_profile(&b);
    assert_eq!(k.pts.first(), k.pts.last(), "the closed wall's coping runs round it");
    assert_eq!(layer_of(&b), "MM-HARP-TAS");
}

#[test]
fn tur_switches_the_kind_and_its_sizes_and_a_drawn_moulding_takes_its_points() {
    let mut b = building("coping");
    assert!(b.type_text("T"));
    assert_eq!(b.session.tool_id(), "moulding");
    let chips: Vec<String> = b.session.prompt().options.iter().map(|o| o.label.to_string()).collect();
    assert!(chips.iter().any(|c| c == "Çıkıntı") && chips.iter().any(|c| c == "Kattan yükseklik"), "{chips:?}");
    // Çiz: the axis point by point, Enter writes it; a subasman band at 0, 3 cm out, 50 cm high.
    assert!(b.run(|s, cx| s.configure("S", "draw", cx)));
    assert!(b.run(|s, cx| s.configure("A", "0", cx)));
    assert!(b.run(|s, cx| s.configure("E", "0.03", cx)));
    assert!(b.run(|s, cx| s.configure("Y", "0.5", cx)));
    assert!(b.run(|s, cx| s.configure("H", "right", cx)));
    b.click(0.0, -1.0);
    b.click(12.0, -1.0);
    b.confirm();
    let k = newest_profile(&b);
    assert_eq!((k.width, k.height, k.base_offset, k.justify), (0.03, 0.5, Some(0.0), WallJustify::Right));
    assert_eq!(k.pts.len(), 2);
    assert_eq!(b.session.tool_id(), "moulding", "Enter writes it and the tool stays on");
}
