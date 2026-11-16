//! Giydirme cephe (`tool.curtainWall`, docs/adr/8070 §7): Duvar drawing a curtain wall. Its chips are Kalınlık, Hiza,
//! Öteleme, Panel eni (P) and Panel boyu (B), no Stil; P and B take the grid's largest spacing (0.3 to 10 m); the wall
//! written carries the grid, Cam and Alüminyum, and no components, on EK-2's Giydirme Cephe (MM-GIYC-GENEL).

use crate::common::{Bench, rel};
use kentos_commands::{DrawingUnit, Entity, Workspace};

fn bench() -> Bench {
    let mut b = Bench::new("curtainWall");
    let mut settings = b.doc.settings().clone();
    settings.srid = 0;
    settings.workspace = Some(Workspace::Cad);
    settings.drawing_unit = Some(DrawingUnit::M);
    b.doc.set_settings(settings);
    b
}

#[test]
fn the_curtain_wall_tool_draws_a_wall_with_its_grid() {
    let mut b = bench();
    assert_eq!(b.session.tool_id(), "curtainWall");
    let chips: Vec<String> = b.session.prompt().options.iter().map(|o| o.label.to_string()).collect();
    assert!(chips.iter().any(|c| c == "Panel eni") && chips.iter().any(|c| c == "Panel boyu"), "{chips:?}");
    assert!(!chips.iter().any(|c| c == "Stil"), "{chips:?}");
    // Panel eni 1.25 m, Panel boyu 11 m refused (at most 10), then 1.5 m.
    // A value typed is taken as it is entered (the command line's Enter is the input itself).
    assert!(b.type_text("P"));
    assert!(b.type_text("1.25"));
    assert!(b.type_text("B"));
    assert!(b.type_text("11"));
    assert_eq!(b.memory.curtain_rows, 1.2, "11 m is past 10 m: refused, the old value stays");
    assert!(b.type_text("B"));
    assert!(b.type_text("1.5"));
    b.click(0.0, 0.0);
    b.click(6.0, 0.0);
    b.confirm();
    assert_eq!(b.doc.len(), 1);
    let Entity::Wall(w) = b.newest() else {
        panic!("a wall")
    };
    let pts: Vec<[f64; 2]> = w.wall.pts.iter().copied().map(rel).collect();
    assert_eq!(pts, [[0.0, 0.0], [6.0, 0.0]]);
    assert!(w.wall.components.is_empty() && w.wall.style.is_none());
    let grid = w.wall.curtain.as_ref().expect("its grid");
    assert_eq!((grid.spacing, grid.rows), (1.25, 1.5));
    assert_eq!((grid.panel.as_deref(), grid.frames.as_deref()), (Some("Cam"), Some("Alüminyum")));
    assert_eq!(b.session.tool_id(), "curtainWall");
}
