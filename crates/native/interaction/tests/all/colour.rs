//! New objects take the current colour, as on the web (docs/adr/0089): the
//! ribbon's Renk goes into the draft (`Draft::color`) and every drawing
//! tool writes it explicitly in its command's input (CMD-07; the web's
//! `colour()`, `writeObjects`); “Katmana göre” writes none.

use crate::common;

use common::Bench;

const RED: &str = "#E5484D";

/// How a test draws one object with a tool.
type Draw<'a> = &'a dyn Fn(&mut Bench);

/// Draws with `tool` in the current colour `color` and says the newest object's.
fn drawn(tool: &str, color: Option<&'static str>, draw: impl FnOnce(&mut Bench)) -> Option<String> {
    let mut b = Bench::new(tool);
    b.draft.color = color.and_then(kentos_interaction::DraftColor::new);
    let before = b.doc.len();
    draw(&mut b);
    assert!(b.doc.len() > before, "{tool} wrote an object");
    b.newest().base().color.clone()
}

#[test]
fn every_kind_of_drawing_tool_writes_the_current_colour() {
    let line = |b: &mut Bench| {
        b.click(0.0, 0.0);
        b.click(10.0, 0.0);
    };
    let area = |b: &mut Bench| {
        b.click(0.0, 0.0);
        b.click(10.0, 0.0);
        b.click(10.0, 10.0);
        b.confirm();
    };
    let circle = |b: &mut Bench| {
        b.click(0.0, 0.0);
        b.click(3.0, 4.0);
    };
    let rectangle = |b: &mut Bench| {
        b.click(0.0, 0.0);
        b.click(10.0, 5.0);
    };
    let point = |b: &mut Bench| b.click(1.0, 2.0);
    // An ellipse is written by cad.entities.create, as the tools without a command of their own.
    let ellipse = |b: &mut Bench| {
        b.click(-20.0, 10.0);
        b.click(0.0, 10.0);
        assert!(b.type_text("4"));
    };
    let tools: [(&str, Draw<'_>); 6] = [
        ("line", &line),
        ("polygon", &area),
        ("circle", &circle),
        ("rectangle", &rectangle),
        ("point", &point),
        ("ellipse", &ellipse),
    ];
    for (tool, draw) in tools {
        assert_eq!(drawn(tool, Some(RED), draw).as_deref(), Some(RED), "{tool}");
        assert_eq!(
            drawn(tool, None, draw),
            None,
            "{tool}: by layer, none written"
        );
    }
}
