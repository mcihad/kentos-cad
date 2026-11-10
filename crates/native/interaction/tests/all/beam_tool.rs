//! Kiriş aracı (docs/adr/8041 §1): two clicks and Enter write a beam on its axis at the tool's size, on the
//! load-bearing beams' layer (MM-TASI-KIRIS), its number from Numara and the next beam the next number (K101 → K102),
//! as the column tool's; the tool stays for the next beam.

use crate::common::{Bench, E, N};
use kentos_commands::{DrawingUnit, Entity, WallJustify, Workspace};

fn bench() -> Bench {
    let mut b = Bench::new("wall");
    let mut settings = b.doc.settings().clone();
    settings.workspace = Some(Workspace::Cad);
    settings.srid = 0;
    settings.drawing_unit = Some(DrawingUnit::M);
    b.doc.set_settings(settings);
    b
}

fn newest_beam(b: &Bench) -> kentos_commands::BeamEntity {
    match b.newest() {
        Entity::Beam(k) => k.clone(),
        e => panic!("a beam: {e:?}"),
    }
}

#[test]
fn two_clicks_write_a_beam_and_the_next_takes_the_next_number() {
    let mut b = bench();
    // Katman anahtarları on (docs/adr/8008): each kind on its own layer.
    b.memory.layer_keys = true;
    b.start("beam");
    assert!(b.run(|s, cx| s.configure("N", "K101", cx)));
    assert!(b.run(|s, cx| s.configure("H", "left", cx)));
    b.click(0.0, 0.0);
    b.click(5.0, 0.0);
    b.confirm();
    let k = newest_beam(&b);
    assert_eq!(k.beam.tag.as_deref(), Some("K101"));
    assert_eq!(
        (k.beam.width, k.beam.height),
        (0.25, 0.5),
        "25/50 unless told"
    );
    assert_eq!(k.beam.justify, WallJustify::Left);
    assert_eq!(
        k.beam
            .pts
            .iter()
            .map(|p| (p.x - E, p.y - N))
            .collect::<Vec<_>>(),
        [(0.0, 0.0), (5.0, 0.0)]
    );
    assert_eq!(b.last_text(), Some("Kiriş çizildi: K101 25/50."));
    assert_eq!(
        b.doc
            .layers()
            .get(&k.base.layer_id)
            .map(|l| l.name.as_str()),
        Some("MM-TASI-KIRIS")
    );
    // The tool stays: the next beam K102 at 30/60.
    assert!(b.run(|s, cx| s.configure("E", "0.3", cx)));
    assert!(b.run(|s, cx| s.configure("Y", "0.6", cx)));
    b.click(5.0, 0.0);
    b.click(5.0, 4.0);
    b.confirm();
    let k = newest_beam(&b);
    assert_eq!(k.beam.tag.as_deref(), Some("K102"));
    assert_eq!(k.beam.size_text(), "30/60");
    // No number: none given, none made up.
    assert!(b.run(|s, cx| s.configure("N", "", cx)));
    b.click(0.0, 4.0);
    b.click(5.0, 4.0);
    b.confirm();
    assert_eq!(newest_beam(&b).beam.tag, None);
    assert_eq!(b.last_text(), Some("Kiriş çizildi: 30/60."));
}

#[test]
fn a_beam_too_narrow_to_write_says_why_and_writes_nothing() {
    let mut b = bench();
    b.start("beam");
    // Genişlik 0 is no width: the tool keeps 25 cm and says so.
    assert!(!b.run(|s, cx| s.configure("E", "0", cx)));
    assert_eq!(b.last_level(), Some(kentos_interaction::Level::Warn));
    let before = b.doc.len();
    b.click(0.0, 0.0);
    b.click(0.0, 0.0);
    b.confirm();
    assert_eq!(b.doc.len(), before, "a beam of one point is no beam");
}

/// A 10 m wall along y 0 (the wall tool's own width) with a 1 m door at its middle, the beam tool not started.
fn wall_with_door() -> (Bench, kentos_domain::Slot) {
    let mut b = bench();
    b.start("wall");
    b.click(0., 0.);
    b.click(10., 0.);
    b.confirm();
    b.confirm();
    b.start("door");
    assert!(b.run(|session, cx| session.configure("E", "1 m", cx)));
    b.click(5., 0.1);
    b.confirm();
    let door = kentos_domain::Slot(b.newest().base().id);
    assert!(matches!(b.doc.get(door), Some(Entity::Opening(_))));
    (b, door)
}

/// Lento (docs/adr/8041 §3): Tür Lento, a click on the door puts a lintel over it: on the wall's axis from 20 cm before
/// its jamb to 20 cm past the other (TBDY 2018 11.5.3), as wide as the wall, 20 cm high, its underside at the door's
/// head, on MM-LENT-GENEL, numbered L101; a click on bare wall writes nothing and says so.
#[test]
fn a_click_on_a_door_puts_a_lintel_over_it() {
    let (mut b, door) = wall_with_door();
    let Some(Entity::Opening(o)) = b.doc.get(door).cloned() else { panic!("the door") };
    let Some(Entity::Wall(w)) = b.doc.entities().find(|e| e.kind() == "wall").cloned() else {
        panic!("the wall")
    };
    b.memory.layer_keys = true;
    b.start("beam");
    assert!(b.run(|s, cx| s.configure("T", "lintel", cx)));
    assert!(b.run(|s, cx| s.configure("N", "L101", cx)));
    let count = b.doc.len();
    b.click(2.0, 0.0);
    assert_eq!(b.doc.len(), count, "no door there: nothing written");
    assert_eq!(b.last_text(), Some("Burada kapı ya da pencere yok."));
    b.click(5.0, 0.0);
    let Entity::Beam(l) = b.newest().clone() else { panic!("a lintel") };
    assert_eq!(l.beam.role, kentos_commands::BeamRole::Lintel);
    assert_eq!(l.beam.tag.as_deref(), Some("L101"));
    assert!((l.beam.width - w.wall.width).abs() < 1e-9, "as wide as the wall");
    assert!((l.beam.height - 0.2).abs() < 1e-9);
    assert_eq!(l.beam.base_offset, Some(o.opening.head.unwrap_or(2.1)));
    let xs: Vec<f64> = l.beam.pts.iter().map(|p| p.x - E).collect();
    assert!((xs[0] - 4.3).abs() < 1e-6 && (xs[1] - 5.7).abs() < 1e-6, "{xs:?}");
    assert!(l.beam.pts.iter().all(|p| (p.y - N).abs() < 1e-6), "on the wall's axis");
    assert_eq!(
        b.doc.layers().get(&l.base.layer_id).map(|x| x.name.as_str()),
        Some("MM-LENT-GENEL")
    );
    let said = format!("Lento kondu: L101 {}/20.", (w.wall.width * 100.0).round());
    assert_eq!(b.last_text(), Some(said.as_str()));
}

/// Hatıl (docs/adr/8041 §3): drawn as a beam, 20/20 at 1.00 m over its storey's floor, on MM-HATL-GENEL.
#[test]
fn a_bond_beam_is_drawn_as_a_beam_at_its_height() {
    let mut b = bench();
    b.memory.layer_keys = true;
    b.start("beam");
    assert!(b.run(|s, cx| s.configure("T", "bondBeam", cx)));
    b.click(0.0, 0.0);
    b.click(4.0, 0.0);
    b.confirm();
    let Entity::Beam(h) = b.newest().clone() else { panic!("a bond beam") };
    assert_eq!(h.beam.role, kentos_commands::BeamRole::BondBeam);
    assert_eq!((h.beam.width, h.beam.height, h.beam.base_offset), (0.2, 0.2, Some(1.0)));
    assert_eq!(h.beam.offset, 0.0);
    assert_eq!(
        b.doc.layers().get(&h.base.layer_id).map(|x| x.name.as_str()),
        Some("MM-HATL-GENEL")
    );
}
