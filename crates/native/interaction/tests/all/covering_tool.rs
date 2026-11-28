//! Kaplama (`tool.covering`, docs/adr/8070 §10): a covering drawn by its corners (Köşeleri çiz) or taken from the room
//! clicked (Mahal, bound to it), an asma tavan of 12.5 mm until changed, on EK-2's Asma Tavan (MM-ASTV-GENEL).

use crate::common::{Bench, E, N};
use kentos_commands::{DrawingUnit, Entity, Workspace};

fn bench() -> Bench {
    let mut b = Bench::new("covering");
    let mut settings = b.doc.settings().clone();
    settings.workspace = Some(Workspace::Cad);
    settings.srid = 0;
    settings.drawing_unit = Some(DrawingUnit::M);
    b.doc.set_settings(settings);
    b.memory.layer_keys = true;
    b
}

#[test]
fn early_enter_keeps_manual_space_slab_and_covering_corners() {
    for tool in ["space", "slab", "covering"] {
        let mut b = bench();
        b.start(tool);
        assert!(b.run(|s, cx| s.configure("S", "draw", cx)));
        let before = b.doc.to_snapshot_v2();
        for (i, (x, y)) in [(0., 0.), (4., 0.)].into_iter().enumerate() {
            b.click(x, y);
            b.confirm();
            assert_eq!(b.session.tool_id(), tool);
            assert_eq!(b.points(), i + 1);
            assert_eq!(b.doc.to_snapshot_v2(), before);
        }
        b.click(4., 3.);
        b.confirm();
        assert_eq!(b.doc.entities().count(), before.entities.len() + 1);
        assert_eq!(b.points(), 0);
        b.confirm();
        assert_eq!(b.session.tool_id(), "select");
        b.doc.undo().unwrap();
        assert_eq!(b.doc.to_snapshot_v2(), before);
    }
}

#[test]
fn covering_thickness_shows_the_same_editable_dimension_in_panel_and_prompt() {
    for unit in [DrawingUnit::M, DrawingUnit::Mm] {
        let mut b = bench();
        let mut settings = b.doc.settings().clone();
        settings.drawing_unit = Some(unit);
        b.doc.set_settings(settings);
        b.start("covering");
        let f = b.run(|_, cx| cx.format());
        let rows = b.session.settings(&f);
        let field = &rows.iter().find(|r| r.key == "E").unwrap().value;
        assert_eq!(
            b.session
                .prompt()
                .options
                .iter()
                .find(|o| o.key == "E")
                .unwrap()
                .value
                .as_ref(),
            Some(field)
        );
        assert_eq!(b.run(|_, cx| cx.typed_length(field)), Some(0.0125));
        assert!(b.run(|s, cx| s.configure("E", field, cx)));
    }
}

#[test]
fn the_covering_tool_draws_a_ceiling_and_a_floor_covering() {
    let mut b = bench();
    assert_eq!(b.session.tool_id(), "covering");
    let chips: Vec<String> = b
        .session
        .prompt()
        .options
        .iter()
        .map(|o| o.label.to_string())
        .collect();
    for c in ["Sınır", "Tip", "Kalınlık", "Kot farkı", "Malzeme"] {
        assert!(chips.iter().any(|x| x == c), "{c} in {chips:?}");
    }
    // Köşeleri çiz: a ceiling under its storey's top, 12.5 mm.
    assert!(b.run(|s, cx| s.configure("S", "draw", cx)));
    assert!(b.run(|s, cx| s.configure("F", "2.6", cx)));
    assert!(b.run(|s, cx| s.configure("M", "Alçıpan", cx)));
    b.click(0.0, 0.0);
    b.click(4.0, 0.0);
    b.click(4.0, 3.0);
    b.confirm();
    let Entity::Slab(s) = b.newest().clone() else {
        panic!("a slab")
    };
    assert_eq!(s.slab.covering, Some(kentos_commands::Covering::ceiling()));
    assert_eq!((s.slab.thickness, s.slab.offset), (0.0125, 2.6));
    assert_eq!(s.slab.material.as_deref(), Some("Alçıpan"));
    let pts: Vec<(f64, f64)> = s.slab.parts[0]
        .outer
        .pts
        .iter()
        .map(|p| (p.x - E, p.y - N))
        .collect();
    assert_eq!(pts, [(0.0, 0.0), (4.0, 0.0), (4.0, 3.0)]);
    assert_eq!(
        b.doc
            .layers()
            .get(&s.base.layer_id)
            .map(|l| l.name.as_str()),
        Some("MM-ASTV-GENEL")
    );
    // A floor covering: 2 cm until changed, on EK-2's Zemin Kaplama; a nought thickness refused.
    assert!(b.run(|s, cx| s.configure("P", "FLOORING", cx)));
    assert!(!b.run(|s, cx| s.configure("E", "0", cx)));
    assert!(b.run(|s, cx| s.configure("F", "", cx)), "Oto");
    b.click(10.0, 0.0);
    b.click(12.0, 0.0);
    b.click(12.0, 2.0);
    b.confirm();
    let Entity::Slab(s) = b.newest().clone() else {
        panic!("a slab")
    };
    assert_eq!(
        s.slab.covering.as_ref().map(|c| c.predefined.as_str()),
        Some("FLOORING")
    );
    assert_eq!(
        (s.slab.thickness, s.slab.offset),
        (0.02, 0.02),
        "on its floor, its top its thickness over it"
    );
    assert_eq!(
        b.doc
            .layers()
            .get(&s.base.layer_id)
            .map(|l| l.name.as_str()),
        Some("MM-ZMNK-GENEL")
    );
    assert_eq!(b.session.tool_id(), "covering");
}
