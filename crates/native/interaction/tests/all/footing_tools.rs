//! Temel (`tool.footingPad`, `tool.footingStrip`, docs/adr/8172 §3): a pad footing put by a click, under the column
//! clicked and turned with it; a strip footing along its axis, Enter writing it. Both on their base 80 cm under the
//! floor until changed, each on its EK-2 layer (MM-TEML-TEKIL, MM-TEML-SUREKLI).

use crate::common::{Bench, E, N};
use kentos_commands::{BeamRole, DrawingUnit, Entity, Workspace};

fn bench(tool: &str) -> Bench {
    let mut b = Bench::new(tool);
    let mut settings = b.doc.settings().clone();
    settings.workspace = Some(Workspace::Cad);
    settings.srid = 0;
    settings.drawing_unit = Some(DrawingUnit::M);
    b.doc.set_settings(settings);
    // Katman anahtarları on (docs/adr/8008): each kind on its own layer.
    b.memory.layer_keys = true;
    b
}

fn chips(b: &Bench) -> Vec<String> {
    b.session.prompt().options.iter().map(|o| o.label.to_string()).collect()
}

/// The newest object, a footing.
fn footing(b: &Bench) -> kentos_commands::BeamFields {
    let Entity::Beam(k) = b.newest().clone() else {
        panic!("a footing is a beam's role")
    };
    *k.beam
}

fn layer_of(b: &Bench) -> String {
    let id = b.newest().base().layer_id.clone();
    b.doc.layers().get(&id).map(|l| l.name.clone()).unwrap_or_default()
}

/// Its axis's middle (from E, N), its direction in degrees and its length.
fn axis(k: &kentos_commands::BeamFields) -> ([f64; 2], f64, f64) {
    let (a, z) = (k.pts[0], k.pts[1]);
    let (dx, dy) = (z.x - a.x, z.y - a.y);
    (
        [(a.x + z.x) / 2.0 - E, (a.y + z.y) / 2.0 - N],
        dy.atan2(dx).to_degrees(),
        (dx * dx + dy * dy).sqrt(),
    )
}

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-6
}

#[test]
fn the_pad_tool_puts_a_pad_under_the_column_it_clicks_turned_with_it() {
    // A column at (2, 2), turned 30°.
    let mut b = bench("column");
    assert!(b.run(|s, cx| s.configure("D", "30", cx)));
    b.click(2.0, 2.0);
    let Entity::Column(c) = b.newest().clone() else {
        panic!("a column")
    };
    assert!(near(c.column.rotation.to_degrees(), 30.0));

    b.start("footingPad");
    assert_eq!(b.session.tool_id(), "footingPad");
    let shown = chips(&b);
    for want in ["Tür", "Genişlik", "Boy", "Yükseklik", "Kattan yükseklik", "Açı", "Numara"] {
        assert!(shown.iter().any(|c| c == want), "{want}: {shown:?}");
    }
    assert!(!shown.iter().any(|c| c == "Hiza" || c == "Kot farkı"), "{shown:?}");

    // A click a little off the column's middle, inside it: the pad's middle is the column's, turned with it.
    b.click(2.08, 1.95);
    let k = footing(&b);
    assert_eq!(k.role, BeamRole::PadFooting);
    assert_eq!((k.width, k.height, k.base_offset), (1.5, 0.5, Some(-0.8)));
    assert_eq!(k.pts.len(), 2);
    let (middle, turn, length) = axis(&k);
    assert!(near(middle[0], 2.0) && near(middle[1], 2.0), "{middle:?}");
    assert!(near(turn, 30.0), "{turn}");
    assert!(near(length, 1.5), "{length}");
    assert_eq!(layer_of(&b), "MM-TEML-TEKIL");
    assert_eq!(b.last_text(), Some("Tekil temel kondu: 150×150/50."));

    // Off any column: its middle on the click, turned by Açı; 2 × 1.2, numbered from T101.
    assert!(b.run(|s, cx| s.configure("E", "1.2", cx)));
    assert!(b.run(|s, cx| s.configure("B", "2", cx)));
    assert!(b.run(|s, cx| s.configure("D", "90", cx)));
    assert!(b.run(|s, cx| s.configure("N", "T101", cx)));
    b.click(8.0, 2.0);
    let k = footing(&b);
    let (middle, turn, length) = axis(&k);
    assert!(near(middle[0], 8.0) && near(middle[1], 2.0), "{middle:?}");
    assert!(near(turn, 90.0) && near(length, 2.0) && near(k.width, 1.2), "{turn} {length} {}", k.width);
    assert_eq!(k.tag.as_deref(), Some("T101"));
    assert_eq!(b.last_text(), Some("Tekil temel kondu: T101 120×200/50."));
    b.click(12.0, 2.0);
    assert_eq!(footing(&b).tag.as_deref(), Some("T102"), "the next one the next number");

    // A size of 0 is refused and the old one stays; the tool stays on.
    assert!(!b.run(|s, cx| s.configure("Y", "0", cx)));
    b.click(16.0, 2.0);
    assert_eq!(footing(&b).height, 0.5);
    assert_eq!(b.session.tool_id(), "footingPad");
}

#[test]
fn the_strip_tool_draws_a_strip_along_its_axis_and_enter_writes_it() {
    let mut b = bench("footingStrip");
    assert_eq!(b.session.tool_id(), "footingStrip");
    let shown = chips(&b);
    for want in ["Tür", "Genişlik", "Yükseklik", "Hiza", "Kattan yükseklik", "Numara"] {
        assert!(shown.iter().any(|c| c == want), "{want}: {shown:?}");
    }
    assert!(!shown.iter().any(|c| c == "Boy" || c == "Açı"), "{shown:?}");

    // An L along its axis, 1.20 m deep.
    assert!(b.run(|s, cx| s.configure("A", "-1.2", cx)));
    b.click(0.0, 0.0);
    b.click(6.0, 0.0);
    b.click(6.0, 4.0);
    assert_eq!(b.points(), 3);
    b.confirm();
    let k = footing(&b);
    assert_eq!(k.role, BeamRole::StripFooting);
    assert_eq!(k.pts.len(), 3);
    assert_eq!((k.width, k.height, k.base_offset), (0.6, 0.5, Some(-1.2)));
    assert_eq!(layer_of(&b), "MM-TEML-SUREKLI");
    assert_eq!(b.last_text(), Some("Sürekli temel çizildi: 60/50."));
    assert_eq!(b.session.tool_id(), "footingStrip", "Enter writes it and the tool stays on");

    // Tür: a pad with a pad's sizes; again: a strip with a strip's.
    assert!(b.type_text("T"));
    assert!(chips(&b).iter().any(|c| c == "Boy"));
    b.click(10.0, 10.0);
    let k = footing(&b);
    assert_eq!((k.role, k.width, k.height), (BeamRole::PadFooting, 1.5, 0.5));
    assert!(b.run(|s, cx| s.configure("T", "stripFooting", cx)));
    assert!(chips(&b).iter().any(|c| c == "Hiza"));
}

#[test]
fn the_raft_tool_writes_a_raft_past_its_outline_and_tur_gives_a_slab_s_sizes_back() {
    let mut b = bench("raft");
    assert_eq!(b.session.tool_id(), "raft");
    assert_eq!(b.session.prompt().tool, Some("Radye"));
    // Its corners drawn, Enter writes it: 50 cm past them, 60 cm thick, its top 30 cm under the floor.
    assert!(b.run(|s, cx| s.configure("S", "draw", cx)));
    for (x, y) in [(0.0, 0.0), (10.0, 0.0), (10.0, 8.0), (0.0, 8.0)] {
        b.click(x, y);
    }
    b.confirm();
    let Entity::Slab(s) = b.newest().clone() else {
        panic!("a raft is a slab")
    };
    assert_eq!(s.slab.role, kentos_commands::SlabRole::Raft);
    assert_eq!((s.slab.thickness, s.slab.offset), (0.6, -0.3));
    let ring = &s.slab.parts[0].outer.pts;
    let (min_x, max_x) = ring.iter().fold((f64::MAX, f64::MIN), |(a, z), p| (a.min(p.x - E), z.max(p.x - E)));
    let (min_y, max_y) = ring.iter().fold((f64::MAX, f64::MIN), |(a, z), p| (a.min(p.y - N), z.max(p.y - N)));
    assert!(near(min_x, -0.5) && near(max_x, 10.5) && near(min_y, -0.5) && near(max_y, 8.5), "{ring:?}");
    assert_eq!(layer_of(&b), "MM-TEML-RADYE");
    // Tür: Döşeme with a slab's sizes again, its id the slab tool's.
    assert!(b.run(|s, cx| s.configure("T", "floor", cx)));
    assert_eq!(b.session.tool_id(), "slab");
    let thickness = b.session.prompt().options.iter().find(|o| o.label == "Kalınlık").and_then(|o| o.value.clone());
    assert!(thickness.as_deref().is_some_and(|t| t.starts_with("0.15") || t.starts_with("15")), "{thickness:?}");
}
