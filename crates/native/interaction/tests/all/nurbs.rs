//! NURBS eğrisi (docs/adr/0259 §6): the tool asks `cad.nurbs.create`. What
//! it writes for the points clicked is exactly what the command writes for
//! the same request, as Python and MCP send it, bit for bit; its preview is
//! the command's plan; Yöntem, Derece and Kapat reach the command's input.

use crate::common::{Bench, E, N};
use kentos_contracts::{CommandResult, Entity, NurbsBy, NurbsCreate, Vec2};
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, nurbs};

const PTS: [[f64; 2]; 5] = [
    [0.0, 0.0],
    [12.0, 7.5],
    [25.0, 3.0],
    [38.0, 11.0],
    [52.0, 6.25],
];

fn at(p: [f64; 2]) -> Vec2 {
    Vec2 {
        x: E + p[0],
        y: N + p[1],
    }
}

/// What the command writes for `by` and `closed` on a drawing like the tool's.
fn by_command(by: NurbsBy, closed: bool) -> Entity {
    let mut b = Bench::new("nurbs");
    let input = NurbsCreate {
        line_type: None,
        layer_id: b.doc.layers().active().to_owned(),
        by,
        closed,
        color: None,
        line_weight: None,
        attrs: None,
        label: None,
        symbol: None,
        expected_revision: None,
    };
    let result = nurbs::execute(&mut ExecutionContext::new(&mut b.doc), input);
    assert!(
        matches!(result, CommandResult::Completed { .. }),
        "{result:?}"
    );
    b.newest().clone()
}

#[test]
fn the_tool_writes_what_the_command_writes_through_the_points() {
    let mut b = Bench::new("nurbs");
    for p in PTS {
        b.click(p[0], p[1]);
    }
    b.confirm();
    assert_eq!(b.last_level(), Some(Level::Success), "{:?}", b.last_text());
    let want = by_command(
        NurbsBy::Fit {
            pts: PTS.iter().map(|p| at(*p)).collect(),
            degree: Some(3),
        },
        false,
    );
    assert_eq!(b.newest(), &want);
    let Entity::Nurbs(n) = b.newest() else {
        panic!("a NURBS curve")
    };
    assert_eq!(n.fit.as_ref().map(Vec::len), Some(5));
    assert_eq!(n.ctrl.first(), Some(&at(PTS[0])));
    assert!(
        b.last_text()
            .is_some_and(|t| t.starts_with("NURBS eğrisi eklendi: 5 geçiş noktası"))
    );
    // One undo step, Ekle.
    assert_eq!(b.doc.undo().as_deref(), Some("Ekle"));
    assert!(!b.doc.entities().any(|e| matches!(e, Entity::Nurbs(_))));
}

#[test]
fn control_points_degree_and_kapat_reach_the_command() {
    let mut b = Bench::new("nurbs");
    assert!(b.type_text("Y"));
    assert!(b.type_text("D"));
    assert!(b.type_text("2"));
    for p in &PTS[..4] {
        b.click(p[0], p[1]);
    }
    assert!(b.type_text("K"));
    let want = by_command(
        NurbsBy::Control {
            degree: 2,
            ctrl: PTS[..4].iter().map(|p| at(*p)).collect(),
            weights: None,
            knots: None,
        },
        true,
    );
    assert_eq!(b.newest(), &want);
    let Entity::Nurbs(n) = b.newest() else {
        panic!("a NURBS curve")
    };
    assert!(n.closed && n.periodic);
    assert_eq!(n.ctrl.len(), 6);
    // The session keeps the method and the degree.
    assert!(b.memory.nurbs_control);
    assert_eq!(b.memory.nurbs_degree, 2);
    // A degree out of range is said and not taken.
    assert!(b.type_text("D"));
    assert!(b.type_text("12"));
    assert_eq!(b.last_level(), Some(Level::Warn));
    assert_eq!(b.memory.nurbs_degree, 2);
}

#[test]
fn the_preview_is_the_commands_plan() {
    let mut b = Bench::new("nurbs");
    b.click(PTS[0][0], PTS[0][1]);
    b.click(PTS[1][0], PTS[1][1]);
    b.move_to(PTS[2][0], PTS[2][1]);
    let preview = b.run(|s, cx| s.preview(&cx.format())).expect("a preview");
    // The curve solid, through the points and the cursor; the control polygon dashed.
    let solid = preview
        .strokes
        .iter()
        .find(|s| s.dash.is_none())
        .expect("the curve");
    let last = *solid.pts.last().expect("points");
    assert!((last.x - (E + PTS[2][0])).abs() < 1e-9 && (last.y - (N + PTS[2][1])).abs() < 1e-9);
    assert!(preview.strokes.iter().any(|s| s.dash.is_some()));
    assert_eq!(b.options(), vec!["Y", "D", "K", "G", "Enter"]);
}

// ── Eğriyi düzenle (docs/adr/0303) ──────────────────────────────────────────

/// What `cad.curves.edit` writes for `edit` on the curve at `slot` of a copy of the drawing.
fn edited_by_command(
    doc: &kentos_domain::Document,
    slot: kentos_domain::Slot,
    edit: kentos_contracts::CurveEdit,
) -> Entity {
    use kentos_native_application::curves_edit;
    let mut doc = doc.clone();
    let input = kentos_contracts::CurvesEdit {
        uid: doc.uid(slot).expect("an id").to_string(),
        edit,
        expected_revision: None,
    };
    let result = curves_edit::execute(&mut ExecutionContext::new(&mut doc), input);
    assert!(
        matches!(result, CommandResult::Completed { .. }),
        "{result:?}"
    );
    doc.get(slot).expect("there").clone()
}

#[test]
fn a_polyline_becomes_a_curve_whose_points_are_edited_through_the_command() {
    use kentos_contracts::CurveEdit;
    let mut b = Bench::new("nurbs");
    let layer = b.doc.layers().active().to_owned();
    let slot = b.add_bulged(
        &layer,
        &[[0.0, 0.0], [10.0, 0.0], [20.0, 5.0], [30.0, 0.0]],
        &[0.0, 0.4, 0.0, 0.0],
    );
    b.start("curveEdit");
    assert_eq!(
        b.session.prompt().text(),
        "Eğriyi düzenle: düzenlenecek çoklu çizgiye ya da NURBS eğrisine tıklayın"
    );
    b.click(5.0, 0.0);
    assert_eq!(b.options(), ["K", "D", "E", "N", "R"]);
    // Düzleştir: the arc edge straight, one step.
    assert!(b.type_text("D"));
    assert_eq!(b.last_text(), Some("Yay kenarları düzleşti."));
    assert_eq!(b.options(), ["K", "E", "N", "R"]);
    // Eğriye çevir: exactly what the command writes.
    let want = edited_by_command(
        &b.doc,
        slot,
        CurveEdit::ToSpline {
            degree: Some(3),
            through: true,
        },
    );
    assert!(b.type_text("E"));
    assert_eq!(b.doc.get(slot), Some(&want));
    assert_eq!(
        b.options(),
        ["K", "E", "T", "S", "G", "D", "L", "İ", "Ü", "Ç"]
    );
    // Noktayı taşı: the point clicked, then its place; the plan is drawn on the way.
    assert!(b.type_text("T"));
    b.click(10.2, 0.1);
    assert!(
        b.session.prompt().text().contains("nokta 2"),
        "{}",
        b.session.prompt().text()
    );
    b.move_to(12.0, 4.0);
    let preview = b
        .session
        .preview(&kentos_interaction::Format::default())
        .expect("a preview");
    assert!(!preview.strokes.is_empty(), "the plan is drawn");
    let want = edited_by_command(
        &b.doc,
        slot,
        CurveEdit::MoveFit {
            index: 1,
            at: at([12.0, 4.0]),
        },
    );
    b.click(12.0, 4.0);
    assert_eq!(b.last_text(), Some("Geçiş noktası taşındı."));
    assert_eq!(b.doc.get(slot), Some(&want));
    // Çoklu çizgiye çevir with the tolerance typed.
    assert!(b.type_text("Ç"));
    assert!(b.type_text("0.05"));
    assert_eq!(b.last_text(), Some("Eğri çoklu çizgi oldu."));
    assert!(matches!(b.doc.get(slot), Some(Entity::Polyline(_))));
    // Each edit its own step.
    assert_eq!(b.doc.undo().as_deref(), Some("Eğriyi düzenle"));
    assert_eq!(b.doc.get(slot), Some(&want));
    // Enter lets the curve go, then leaves.
    b.confirm();
    assert!(b.session.prompt().text().contains("tıklayın"));
}
