//! New objects take the current line weight, as on the web (docs/adr/0139):
//! the ribbon's Kalınlık goes into the draft (`Draft::line_weight`) and every
//! tool that draws with lines writes it explicitly in its command's input
//! (CMD-07; the web's `weight()`, `writeObjects`). A point and a text are not
//! drawn with lines and take none; “Katmana göre” writes none; 0 is the
//! thinnest line, not “none”.

mod common;

use common::Bench;

/// How a test draws one object with a tool.
type Draw<'a> = &'a dyn Fn(&mut Bench);

/// Draws with `tool` at the current weight `weight` and says the newest object's.
fn drawn(tool: &str, weight: Option<f64>, draw: impl FnOnce(&mut Bench)) -> Option<f64> {
    let mut b = Bench::new(tool);
    b.draft.line_weight = weight;
    let before = b.doc.len();
    draw(&mut b);
    assert!(b.doc.len() > before, "{tool} wrote an object");
    b.newest().base().line_weight
}

#[test]
fn every_tool_that_draws_with_lines_writes_the_current_line_weight() {
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
    let arc = |b: &mut Bench| {
        b.click(10.0, 0.0);
        b.click(0.0, 10.0);
        b.click(-10.0, 0.0);
    };
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
        ("arc", &arc),
        ("ellipse", &ellipse),
    ];
    for (tool, draw) in tools {
        assert_eq!(drawn(tool, Some(0.5), draw), Some(0.5), "{tool}");
        assert_eq!(
            drawn(tool, Some(0.0), draw),
            Some(0.0),
            "{tool}: 0 is the thinnest line"
        );
        assert_eq!(
            drawn(tool, None, draw),
            None,
            "{tool}: by layer, none written"
        );
    }
}

#[test]
fn a_point_and_a_text_take_no_line_weight() {
    let point = |b: &mut Bench| b.click(1.0, 2.0);
    assert_eq!(drawn("point", Some(0.5), point), None, "point");
    let text = |b: &mut Bench| {
        b.click(4.0, 2.0);
        b.run(|s, cx| s.text_typed(Some("Ada 101"), cx));
    };
    assert_eq!(drawn("text", Some(0.5), text), None, "text");
}
