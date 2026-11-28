//! Kütle (`tool.mass`, docs/adr/8178): a massing study's prism on Etkin kat, on MM-XXXX-KUTLE, in one undo step. Its
//! footprint clicked corner by corner (Enter, Kapat or its first corner writes it), two opposite corners of a rectangle,
//! or the closed line, polygon or circle clicked, its arcs and courtyards kept; 9.00 m high until told.

use crate::common::{Bench, E, N, base};
use kentos_commands::{DrawingUnit, Entity, MassEntity, PathEntity, RingGeometry, Workspace};
use kentos_interaction::{Format, Level};
use serde_json::json;

/// Zemin kat and 1. kat 3 m apart; 1. kat active, the layer key on.
fn bench() -> Bench {
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
    b.level = Some("kat-1".into());
    b.memory.layer_keys = true;
    b.start("mass");
    b
}

fn masses(b: &Bench) -> Vec<&MassEntity> {
    b.doc
        .entities()
        .filter_map(|e| match e {
            Entity::Mass(m) => Some(m),
            _ => None,
        })
        .collect()
}

/// A ring's corners less (E, N), rounded to the micrometre.
fn rel(pts: &[kentos_commands::Vec2]) -> Vec<(f64, f64)> {
    let r = |v: f64| (v * 1e6).round() / 1e6;
    pts.iter().map(|p| (r(p.x - E), r(p.y - N))).collect()
}

fn layer_name(b: &Bench, m: &MassEntity) -> Option<String> {
    b.doc.layers().get(&m.base.layer_id).map(|l| l.name.clone())
}

#[test]
fn corners_and_enter_write_a_mass_on_its_layer_and_its_first_corner_closes_one() {
    let mut b = bench();
    assert_eq!(b.options(), ["S", "Y", "F", "A"]);
    for (e, n) in [(0.0, 0.0), (10.0, 0.0), (10.0, 8.0)] {
        b.click(e, n);
    }
    b.move_to(0.0, 8.0);
    assert_eq!(b.options(), ["S", "Y", "F", "A", "K", "Enter"]);
    let preview = b.session.preview(&Format::default()).expect("a preview");
    assert!(
        !preview.strokes.is_empty() && preview.areas.len() == 1,
        "the mass outlined and filled"
    );
    let tag = preview.tag.expect("a tag").lines;
    assert_eq!(tag[0], "Kütle 9.000 m");
    assert_eq!(
        tag[1], "Oturum 80.00 m² · hacim 720.00 m³",
        "the pointer's corner counted: {tag:?}"
    );
    b.click(0.0, 8.0);
    b.confirm();
    let all = masses(&b);
    assert_eq!(all.len(), 1);
    let m = all[0];
    assert_eq!(
        rel(&m.mass.parts[0].outer.pts),
        [(0.0, 0.0), (10.0, 0.0), (10.0, 8.0), (0.0, 8.0)]
    );
    assert_eq!(
        (m.mass.height, m.mass.offset, m.mass.name.as_deref()),
        (9.0, 0.0, None)
    );
    assert_eq!(m.mass.arch.level.as_deref(), Some("kat-1"));
    assert_eq!(layer_name(&b, m).as_deref(), Some("MM-XXXX-KUTLE"));
    assert_eq!(b.last_level(), Some(Level::Success));
    assert_eq!(
        b.last_text(),
        Some("Kütle çizildi: oturumu 80.00 m², 9.000 m yüksek, hacmi 720.00 m³.")
    );
    assert_eq!(b.session.tool_id(), "mass", "the tool stays for the next");
    // Its first corner again closes it: a triangle, the same layer.
    for (e, n) in [(20.0, 0.0), (26.0, 0.0), (20.0, 4.0), (20.0, 0.0)] {
        b.click(e, n);
    }
    let all = masses(&b);
    assert_eq!(all.len(), 2);
    assert_eq!(
        rel(&all[1].mass.parts[0].outer.pts),
        [(20.0, 0.0), (26.0, 0.0), (20.0, 4.0)]
    );
    assert_eq!(all[0].base.layer_id, all[1].base.layer_id);
    assert_eq!(b.doc.undo().as_deref(), Some("Kütle"));
    assert_eq!(masses(&b).len(), 1, "one undo step, one mass");
}

#[test]
fn a_rectangle_takes_two_opposite_corners_with_its_height_and_name() {
    let mut b = bench();
    assert!(b.run(|s, cx| s.configure("S", "rectangle", cx)));
    assert!(b.type_text("Y"));
    assert!(b.type_text("12"));
    assert!(b.type_text("A"));
    assert!(
        b.session.prompt().text().contains("adını yazın"),
        "{:?}",
        b.session.prompt()
    );
    assert!(b.type_text("Blok A"));
    assert!(b.type_text("F"));
    assert!(b.type_text("-1,5"));
    assert_eq!(b.options(), ["S", "Y", "F", "A"]);
    b.click(6.0, 4.0);
    b.click(0.0, 0.0);
    let m = masses(&b)[0].clone();
    assert_eq!(
        rel(&m.mass.parts[0].outer.pts),
        [(0.0, 0.0), (6.0, 0.0), (6.0, 4.0), (0.0, 4.0)],
        "counter-clockwise"
    );
    assert_eq!(
        (m.mass.height, m.mass.offset, m.mass.name.as_deref()),
        (12.0, -1.5, Some("Blok A"))
    );
    assert_eq!(
        m.mass.span(3.0),
        (1.5, 13.5),
        "its bottom 1.50 m under 1. kat's floor"
    );
    assert_eq!(
        b.last_text(),
        Some("Kütle çizildi (Blok A): oturumu 24.00 m², 12.000 m yüksek, hacmi 288.00 m³.")
    );
    // An empty name takes it away; the next rectangle has none.
    assert!(b.run(|s, cx| s.configure("A", "", cx)));
    b.click(10.0, 0.0);
    b.click(12.0, 2.0);
    assert_eq!(masses(&b)[1].mass.name, None);
}

#[test]
fn a_closed_line_polygon_or_circle_clicked_gives_its_footprint_its_arcs_and_courtyards_kept() {
    let mut b = bench();
    let p = |x: f64, y: f64| kentos_commands::Vec2 { x: E + x, y: N + y };
    let court = PathEntity {
        space_separator: None,
        base: base("cizim"),
        pts: vec![p(0.0, 0.0), p(10.0, 0.0), p(10.0, 10.0), p(0.0, 10.0)],
        bulges: None,
        holes: Some(vec![RingGeometry {
            pts: vec![p(3.0, 3.0), p(7.0, 3.0), p(7.0, 7.0), p(3.0, 7.0)],
            bulges: None,
            zs: None,
        }]),
        zs: None,
        ms: None,
        parts: None,
        closed: false,
        widths: None,
    };
    b.doc.add(Entity::Polygon(court)).expect("a slot");
    b.add_circle("cizim", [30.0, 5.0], 3.0);
    // An open line is no footprint.
    b.add_path("cizim", &[[50.0, 0.0], [55.0, 0.0], [55.0, 5.0]], false);
    assert!(b.run(|s, cx| s.configure("S", "line", cx)));
    assert_eq!(b.options(), ["S", "Y", "F", "A"]);
    assert!(
        b.session
            .prompt()
            .text()
            .contains("kapalı çizginin, alanın ya da dairenin kenarına tıklayın")
    );
    // The polygon's edge: its courtyard stays a hole.
    b.move_to(5.0, 0.0);
    let tag = b
        .session
        .preview(&Format::default())
        .and_then(|p| p.tag)
        .expect("a tag")
        .lines;
    assert_eq!(tag[1], "Oturum 84.00 m² · hacim 756.00 m³", "{tag:?}");
    b.click(5.0, 0.0);
    let m = masses(&b)[0].clone();
    assert_eq!(m.mass.parts.len(), 1);
    assert_eq!(
        rel(&m.mass.parts[0].holes[0].pts),
        [(3.0, 3.0), (7.0, 3.0), (7.0, 7.0), (3.0, 7.0)]
    );
    // The circle's edge: two half circles.
    b.click(33.0, 5.0);
    let m = masses(&b)[1].clone();
    let ring = &m.mass.parts[0].outer;
    assert_eq!(rel(&ring.pts), [(33.0, 5.0), (27.0, 5.0)]);
    assert_eq!(ring.bulges.as_deref(), Some(&[1.0, 1.0][..]));
    let said = b.last_text().unwrap().to_owned();
    assert!(
        said.starts_with("Kütle çizildi: oturumu 28.27 m²"),
        "π·3² = 28.27: {said}"
    );
    // An open line, and nothing: said, nothing written.
    b.click(52.0, 0.0);
    assert_eq!(
        (b.last_level(), b.last_text()),
        (
            Some(Level::Info),
            Some("Burada kapalı bir çizgi, alan ya da daire yok: kenarına tıklayın.")
        )
    );
    b.click(80.0, 80.0);
    assert_eq!(masses(&b).len(), 2);
}

#[test]
fn a_height_past_its_limit_and_a_footprint_without_area_are_refused() {
    let mut b = bench();
    assert!(b.type_text("Y"));
    assert!(b.type_text("400"));
    assert_eq!(b.last_level(), Some(Level::Warn));
    assert_eq!(
        b.last_text(),
        Some("Kütlenin yüksekliği sıfırdan büyük ve en çok 300.000 m olmalı.")
    );
    assert!(b.type_text("G"), "Geri leaves the question");
    assert!(b.type_text("F"));
    assert!(b.type_text("150"));
    assert_eq!(
        b.last_text(),
        Some("Kot farkı -100.000 m ile 100.000 m arasında olmalı.")
    );
    assert!(b.type_text("G"));
    // Three corners on one line: refused, kept for an undo.
    for (e, n) in [(0.0, 0.0), (5.0, 0.0), (10.0, 0.0)] {
        b.click(e, n);
    }
    b.confirm();
    assert_eq!(b.last_level(), Some(Level::Warn));
    assert_eq!(
        b.last_text(),
        Some("Oturumun alanı yok: köşeleri bir doğrunun üstünde. Kütle çizilmedi.")
    );
    assert!(masses(&b).is_empty());
    assert_eq!(b.points(), 3, "the corners stay");
    assert!(b.undo_step());
    b.click(10.0, 6.0);
    b.confirm();
    assert_eq!(masses(&b).len(), 1, "fixed and written");
}
