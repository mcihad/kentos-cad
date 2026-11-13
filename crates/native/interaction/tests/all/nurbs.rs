//! NURBS eğrisi (docs/adr/0259 §6): the tool asks `cad.nurbs.create`. What
//! it writes for the points clicked is exactly what the command writes for
//! the same request, as Python and MCP send it, bit for bit; its preview is
//! the command's plan; Yöntem, Derece and Kapat reach the command's input.

use crate::common::{Bench, E, N};
use kentos_commands::{CommandResult, Entity, NurbsBy, NurbsCreate, Vec2};
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
    edit: kentos_commands::CurveEdit,
) -> Entity {
    use kentos_native_application::curves_edit;
    let mut doc = doc.clone();
    let input = kentos_commands::CurvesEdit {
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
    use kentos_commands::CurveEdit;
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

fn curve_editor() -> Bench {
    let mut b = Bench::new("nurbs");
    b.draft.snap = false;
    for p in PTS {
        b.click(p[0], p[1]);
    }
    b.confirm();
    let slot = kentos_domain::Slot(b.newest().base().id);
    b.selection.set([slot]);
    b.start("curveEdit");
    b
}

#[test]
fn curve_settings_separate_display_from_geometry_and_keep_the_selected_curve_identity() {
    use kentos_interaction::{Format, MarkerShape};
    let mut b = curve_editor();
    let before = b.doc.to_snapshot_v2();
    let rows = b.session.settings(&Format::default());
    assert!(
        rows.iter()
            .any(|r| r.key == "source" && r.value.contains("derece 3"))
    );
    for (mode, rings, squares) in [
        ("fit", true, false),
        ("control", false, true),
        ("both", true, true),
    ] {
        assert!(b.run(|s, cx| s.configure("point-view", mode, cx)));
        let drawn = b.session.preview(&Format::default()).unwrap();
        assert_eq!(
            drawn
                .markers
                .iter()
                .any(|m| matches!(m.shape, MarkerShape::Ring(_))),
            rings
        );
        assert_eq!(
            drawn
                .markers
                .iter()
                .any(|m| matches!(m.shape, MarkerShape::Square(_))),
            squares
        );
        assert_eq!(
            b.doc.to_snapshot_v2(),
            before,
            "changing display is readonly"
        );
    }
    assert!(!b.run(|s, cx| s.configure("point-view", "guess", cx)));
    assert_eq!(b.doc.to_snapshot_v2(), before);
    assert!(
        b.session
            .settings(&Format::default())
            .iter()
            .any(|r| r.key == "authority" && r.value.contains("geçiş verisini bırakır"))
    );
}

#[test]
fn a_weight_refusal_keeps_the_point_selected_until_a_valid_value_is_given() {
    use kentos_interaction::Format;
    let mut b = curve_editor();
    let before = b.doc.to_snapshot_v2();
    let slot = kentos_domain::Slot(b.newest().base().id);
    let control = match b.doc.get(slot).unwrap() {
        Entity::Nurbs(n) => n.ctrl[1],
        _ => panic!(),
    };
    assert!(b.run(|s, cx| s.configure("control-edit", "L", cx)));
    b.click(control.x - E, control.y - N);
    assert!(
        b.session
            .settings(&Format::default())
            .iter()
            .any(|r| r.key == "weight")
    );
    assert!(!b.run(|s, cx| s.configure("weight", "0", cx)));
    assert_eq!(b.doc.to_snapshot_v2(), before);
    assert!(b.session.prompt().text().contains("ağırlığı yazın"));
    let want = edited_by_command(
        &b.doc,
        slot,
        kentos_commands::CurveEdit::Weight {
            index: 1,
            weight: 2.5,
        },
    );
    assert!(b.run(|s, cx| s.configure("weight", "2.5", cx)));
    assert_eq!(b.doc.get(slot), Some(&want));
    let Entity::Nurbs(n) = b.doc.get(slot).unwrap() else {
        panic!()
    };
    assert!(
        n.fit.is_none(),
        "the documented authority switch uses the existing command"
    );
    b.doc.undo();
    assert_eq!(b.doc.to_snapshot_v2(), before);
}

#[test]
fn tangent_settings_show_the_semantic_plan_before_the_click_and_keep_directed_end_derivatives() {
    use kentos_commands::{CurveEdit, CurvesEdit};
    use kentos_geometry_core::geom::nurbs::OwnedNurbs;
    use kentos_interaction::Format;
    use kentos_native_application::curves_edit;
    let mut b = curve_editor();
    let before = b.doc.to_snapshot_v2();
    let slot = kentos_domain::Slot(b.newest().base().id);
    assert!(b.run(|s, cx| s.configure("fit-edit", "G", cx)));
    assert!(
        b.session
            .settings(&Format::default())
            .iter()
            .any(|r| r.key == "continuity" && r.value.contains("G2"))
    );
    b.click(0., 10.);
    b.move_to(62., 6.25);
    let edit = CurveEdit::Tangents {
        start: Some(Vec2 { x: 0., y: 10. }),
        end: Some(Vec2 { x: 10., y: 0. }),
    };
    let request = CurvesEdit {
        uid: b.doc.uid(slot).unwrap().to_string(),
        edit: edit.clone(),
        expected_revision: None,
    };
    let CommandResult::Completed { output, .. } = curves_edit::plan_in(&b.doc, &request) else {
        panic!()
    };
    let want = edited_by_command(&b.doc, slot, edit);
    let preview = b.session.preview(&Format::default()).unwrap();
    assert!(!preview.strokes.is_empty());
    assert_eq!(output.changed.first(), Some(&want));
    assert_eq!(b.doc.to_snapshot_v2(), before);
    b.click(62., 6.25);
    assert_eq!(b.doc.get(slot), Some(&want));
    let Entity::Nurbs(n) = &want else { panic!() };
    let c = OwnedNurbs {
        degree: n.degree as usize,
        ctrl: n
            .ctrl
            .iter()
            .map(|p| kentos_interaction::Vec2::new(p.x, p.y))
            .collect(),
        knots: n.knots.clone(),
        weights: n.weights.clone(),
    };
    let (a, z) = c.view().domain();
    for p in &preview.strokes.first().unwrap().pts {
        let on = c.view().point(c.view().closest(*p));
        assert!(
            (on.x - p.x).hypot(on.y - p.y) < 1e-6,
            "preview must lie on the planned curve"
        );
    }
    let start = c.view().derivs(a, 1)[1];
    let end = c.view().derivs(z, 1)[1];
    assert!(start.y > 0. && start.x.abs() < 1e-7);
    assert!(end.x > 0. && end.y.abs() < 1e-7);
    b.doc.undo();
    assert_eq!(b.doc.to_snapshot_v2(), before);
}

#[test]
fn close_open_and_polyline_conversion_are_explicit_settings_actions_with_single_undo() {
    let mut b = curve_editor();
    let before = b.doc.to_snapshot_v2();
    let slot = kentos_domain::Slot(b.newest().base().id);
    let want = edited_by_command(&b.doc, slot, kentos_commands::CurveEdit::Close);
    assert!(b.run(|s, cx| s.configure("curve-action", "K", cx)));
    assert_eq!(b.doc.get(slot), Some(&want));
    let Entity::Nurbs(n) = b.doc.get(slot).unwrap() else {
        panic!()
    };
    assert!(n.closed && n.periodic);
    let open = edited_by_command(&b.doc, slot, kentos_commands::CurveEdit::Open);
    assert!(b.run(|s, cx| s.configure("curve-action", "A", cx)));
    assert_eq!(b.doc.get(slot), Some(&open));
    b.doc.undo();
    assert_eq!(b.doc.get(slot), Some(&want));
    b.doc.undo();
    assert_eq!(b.doc.to_snapshot_v2(), before);
    b.move_to(0., 0.);
    assert!(b.run(|s, cx| s.configure("curve-action", "Ç", cx)));
    b.run(|s, cx| s.cancel(cx));
    assert_eq!(
        b.doc.to_snapshot_v2(),
        before,
        "choosing conversion then Esc writes nothing"
    );
}

#[test]
fn a_visible_fit_handle_moves_directly_but_cancel_and_hidden_handles_never_write() {
    use kentos_interaction::Format;
    let mut b = curve_editor();
    let before = b.doc.to_snapshot_v2();
    let slot = kentos_domain::Slot(b.newest().base().id);
    b.click(PTS[1][0], PTS[1][1]);
    assert!(b.session.prompt().text().contains("yeni yerini"));
    b.move_to(14., 9.);
    assert_eq!(b.doc.to_snapshot_v2(), before);
    b.run(|s, cx| s.cancel(cx));
    assert_eq!(b.doc.to_snapshot_v2(), before);
    b.click(PTS[1][0], PTS[1][1]);
    let want = edited_by_command(
        &b.doc,
        slot,
        kentos_commands::CurveEdit::MoveFit {
            index: 1,
            at: at([14., 9.]),
        },
    );
    b.click(14., 9.);
    assert_eq!(b.doc.get(slot), Some(&want));
    b.doc.undo();
    b.run(|s, cx| s.document_changed(cx));
    assert_eq!(b.doc.to_snapshot_v2(), before);
    let drawn = b.session.preview(&Format::default()).unwrap();
    assert!(
        drawn
            .markers
            .iter()
            .any(|m| m.at == kentos_interaction::Vec2::new(E + PTS[1][0], N + PTS[1][1]))
    );
    assert!(
        !drawn
            .markers
            .iter()
            .any(|m| m.at == kentos_interaction::Vec2::new(E + 14., N + 9.))
    );
    assert!(b.run(|s, cx| s.configure("point-view", "fit", cx)));
    let Entity::Nurbs(n) = b.doc.get(slot).unwrap() else {
        panic!()
    };
    let hidden = n.ctrl[1];
    b.click(hidden.x - E, hidden.y - N);
    assert!(
        !b.session
            .prompt()
            .text()
            .contains("denetim noktasının yeni")
    );
    assert_eq!(b.doc.to_snapshot_v2(), before);
    assert!(
        b.session
            .settings(&Format::default())
            .iter()
            .any(|r| r.key == "fit-edit")
    );
}

#[test]
fn curve_point_input_uses_project_units_relative_anchor_and_ortho_like_other_point_tools() {
    let mut b = curve_editor();
    let mut settings = b.doc.settings().clone();
    settings.workspace = Some(kentos_commands::Workspace::Cad);
    settings.srid = 0;
    settings.drawing_unit = Some(kentos_commands::DrawingUnit::Mm);
    b.doc.set_settings(settings);
    let before = b.doc.to_snapshot_v2();
    let slot = kentos_domain::Slot(b.newest().base().id);
    b.click(PTS[1][0], PTS[1][1]);
    assert_eq!(
        b.session.snap_from(),
        Some(kentos_interaction::Vec2::new(E + 12., N + 7.5))
    );
    assert!(b.type_text("@1000,500"));
    let want = edited_by_command_from(
        &before,
        slot,
        kentos_commands::CurveEdit::MoveFit {
            index: 1,
            at: at([13., 8.]),
        },
    );
    assert_eq!(b.doc.get(slot), Some(&want));
    b.doc.undo();
    b.move_to(0., 0.);
    b.draft.ortho = true;
    b.click(PTS[1][0], PTS[1][1]);
    b.click(14., 8.);
    let want = edited_by_command_from(
        &before,
        slot,
        kentos_commands::CurveEdit::MoveFit {
            index: 1,
            at: at([14., 7.5]),
        },
    );
    assert_eq!(b.doc.get(slot), Some(&want));
    b.doc.undo();
    b.run(|s, cx| s.cancel(cx));
    assert!(b.session.snap_from().is_none());
    assert_eq!(b.doc.to_snapshot_v2(), before);
}

fn edited_by_command_from(
    snapshot: &kentos_commands::DocumentSnapshotV2,
    slot: kentos_domain::Slot,
    edit: kentos_commands::CurveEdit,
) -> Entity {
    let doc = kentos_domain::Document::from_snapshot_v2(snapshot.clone()).unwrap();
    edited_by_command(&doc, slot, edit)
}

/// UR-087: a curve drawn with Eğri is not taken by Eğriyi düzenle; selected or clicked, the tool says so and how it
/// is edited instead.
#[test]
fn a_drawn_curve_is_said_not_taken_by_curve_edit() {
    let mut b = Bench::new("spline");
    b.draft.snap = false;
    for p in [[2.0, 1.0], [4.0, 3.0], [6.0, 1.0]] {
        b.click(p[0], p[1]);
    }
    b.confirm();
    let curve = kentos_domain::Slot(b.newest().base().id);
    assert!(matches!(b.newest(), Entity::Spline(_)), "{:?}", b.newest());
    b.selection.set([curve]);
    b.start("curveEdit");
    assert!(b.last_text().is_some_and(|t| t.starts_with("Eğri aracıyla çizilen eğri burada düzenlenmez")), "{:?}", b.last_text());
    b.selection.set([]);
    b.start("curveEdit");
    b.spatial.sync(&b.doc);
    b.click(4.0, 3.0);
    assert!(b.last_text().is_some_and(|t| t.starts_with("Eğri aracıyla çizilen eğri burada düzenlenmez")), "{:?}", b.last_text());
}
