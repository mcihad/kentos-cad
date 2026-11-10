//! Kiriş aracı (docs/adr/3041 §1): two clicks and Enter write a beam on its axis at the tool's size, on the
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
    // Katman anahtarları on (docs/adr/3008): each kind on its own layer.
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
