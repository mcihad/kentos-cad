//! Dikme and Kayıt (`tool.post`, `tool.rail`, docs/adr/8070 §8): Kolon writing a post and Kiriş writing a rail, each a
//! member of its type on EK-2's Diğer (MM-DIGR-DIKME, MM-DIGR-KAYIT). A post is 10/10 Çelik and not darkened until
//! changed, of its storey's height until Yükseklik (Ü) is typed; a rail 5/10 Çelik at 1.00 m over its floor, no Tür.

use crate::common::{Bench, E, N};
use kentos_commands::{DrawingUnit, Entity, Workspace};

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

#[test]
fn the_post_tool_writes_a_post_of_its_own_height() {
    let mut b = bench("post");
    assert_eq!(b.session.tool_id(), "post");
    let chips: Vec<String> = b
        .session
        .prompt()
        .options
        .iter()
        .map(|o| o.label.to_string())
        .collect();
    assert!(
        chips.iter().any(|c| c == "Yükseklik") && chips.iter().any(|c| c == "Malzeme"),
        "{chips:?}"
    );
    assert!(!chips.iter().any(|c| c == "Aks kesişimleri"), "{chips:?}");
    // Its storey's height first: none of its own.
    b.click(1.0, 1.0);
    let Entity::Column(c) = b.newest().clone() else {
        panic!("a column")
    };
    assert_eq!(c.column.member, Some(kentos_commands::Member::post()));
    assert_eq!(
        (c.column.width, c.column.depth, c.column.height),
        (0.1, Some(0.1), None)
    );
    assert_eq!(c.column.material.as_deref(), Some("Çelik"));
    assert!(!c.column.fill, "a post is not the frame's: not darkened");
    assert_eq!((c.column.at.x - E, c.column.at.y - N), (1.0, 1.0));
    assert_eq!(
        b.doc
            .layers()
            .get(&c.base.layer_id)
            .map(|l| l.name.as_str()),
        Some("MM-DIGR-DIKME")
    );
    assert_eq!(b.last_text(), Some("1 dikme konuldu."));
    // 2.40 m and Ahşap typed; a height of 0 is refused and the old one stays.
    assert!(b.run(|s, cx| s.input("Y", cx)));
    assert!(b.run(|s, cx| s.input("2.4", cx)));
    assert!(b.run(|s, cx| s.input("O", cx)));
    assert!(b.run(|s, cx| s.input("0.15", cx)));
    assert!(!b.run(|s, cx| s.configure("Y", "0", cx)));
    assert!(b.run(|s, cx| s.configure("M", "Ahşap", cx)));
    b.click(3.0, 1.0);
    let Entity::Column(c) = b.newest().clone() else {
        panic!("a column")
    };
    assert_eq!((c.column.height, c.column.depth), (Some(2.4), Some(0.15)));
    assert_eq!(c.column.material.as_deref(), Some("Ahşap"));
    // Kat: its storey's height again.
    assert!(b.run(|s, cx| s.configure("Ü", "Kat", cx)));
    b.click(5.0, 1.0);
    let Entity::Column(c) = b.newest().clone() else {
        panic!("a column")
    };
    assert_eq!(c.column.height, None);
    assert_eq!(b.session.tool_id(), "post");
}

#[test]
fn the_rail_tool_writes_a_rail_at_its_base() {
    let mut b = bench("rail");
    assert_eq!(b.session.tool_id(), "rail");
    let chips: Vec<String> = b
        .session
        .prompt()
        .options
        .iter()
        .map(|o| o.label.to_string())
        .collect();
    assert!(
        chips.iter().any(|c| c == "Kattan yükseklik") && chips.iter().any(|c| c == "Malzeme"),
        "{chips:?}"
    );
    assert!(
        !chips.iter().any(|c| c == "Tür" || c == "Kot farkı"),
        "{chips:?}"
    );
    assert!(b.run(|s, cx| s.configure("A", "0.9", cx)));
    assert!(
        !b.run(|s, cx| s.configure("T", "lintel", cx)),
        "a rail has no Tür"
    );
    b.click(0.0, 0.0);
    b.click(6.0, 0.0);
    b.confirm();
    let Entity::Beam(k) = b.newest().clone() else {
        panic!("a beam")
    };
    assert_eq!(k.beam.member, Some(kentos_commands::Member::rail()));
    assert_eq!(k.beam.role, kentos_commands::BeamRole::Beam);
    assert_eq!(
        (k.beam.width, k.beam.height, k.beam.base_offset),
        (0.05, 0.1, Some(0.9))
    );
    assert_eq!(
        (k.beam.offset, k.beam.material.as_deref()),
        (0.0, Some("Çelik"))
    );
    assert_eq!(
        b.doc
            .layers()
            .get(&k.base.layer_id)
            .map(|l| l.name.as_str()),
        Some("MM-DIGR-KAYIT")
    );
    assert_eq!(b.last_text(), Some("Kayıt çizildi: 5/10."));
    assert_eq!(b.session.tool_id(), "rail");
}

/// Kayıt's Yükselme (docs/adr/8130): 1.5 typed, two clicks 4 m apart and Enter write a sloped member whose end is 1.50 m
/// over its start; three points with a rise are refused with the rule's words and nothing is written; 0 draws a level one
/// again.
#[test]
fn the_rail_tool_writes_a_sloped_member_on_two_points() {
    let mut b = bench("rail");
    let chips: Vec<String> = b
        .session
        .prompt()
        .options
        .iter()
        .map(|o| o.label.to_string())
        .collect();
    assert!(chips.iter().any(|c| c == "Yükselme"), "{chips:?}");
    assert!(b.run(|s, cx| s.configure("U", "1.5", cx)));
    b.click(0.0, 0.0);
    b.click(4.0, 0.0);
    b.confirm();
    let Entity::Beam(k) = b.newest().clone() else {
        panic!("a beam")
    };
    assert_eq!((k.beam.rise, k.beam.pts.len()), (Some(1.5), 2));
    let count = |b: &Bench| {
        b.doc
            .entities()
            .filter(|e| matches!(e, Entity::Beam(_)))
            .count()
    };
    let before = count(&b);
    b.click(0.0, 2.0);
    b.click(4.0, 2.0);
    b.click(4.0, 5.0);
    b.confirm();
    assert_eq!(count(&b), before, "a bent sloped member is not written");
    assert!(
        b.last_text()
            .is_some_and(|t| t.contains("iki noktası olur")),
        "{:?}",
        b.last_text()
    );
    assert!(b.run(|s, cx| s.configure("U", "0", cx)));
    b.click(0.0, 8.0);
    b.click(4.0, 8.0);
    b.confirm();
    let Entity::Beam(k) = b.newest().clone() else {
        panic!("a beam")
    };
    assert_eq!(k.beam.rise, None);
}
