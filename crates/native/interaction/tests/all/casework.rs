//! Mutfak dizisi (docs/adr/0706, 0727): the start, the end and the side clicked write a straight kitchen run in one undo
//! step — its cabinets, sink, cooktop, countertop and filler — with KentOS's library blocks it brings; an L or a U is
//! written with its end, on the side it turns to, each corner square to the leg before. The preview is the command's
//! plan and writes nothing. Evye and Ocak start Otomatik; a run too short is said, never written.

use crate::common::{Bench, E, N};
use kentos_commands::{DrawingUnit, Entity, Workspace};

fn bench() -> Bench {
    let mut b = Bench::new("casework");
    let mut settings = b.doc.settings().clone();
    settings.workspace = Some(Workspace::Cad);
    settings.srid = 0;
    settings.drawing_unit = Some(DrawingUnit::M);
    b.doc.set_settings(settings);
    b.start("casework");
    b
}

/// The inserts written, by their blocks' names, in the drawing's order.
fn inserted(b: &Bench) -> Vec<String> {
    b.doc
        .entities()
        .filter_map(|e| match e {
            Entity::Insert(i) => b.doc.block(i.block).map(|d| d.name.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn the_start_the_end_and_the_side_write_the_run_in_one_step() {
    let mut b = bench();
    b.click(0., 0.);
    b.click(3.6, 0.);
    b.move_to(1.8, -1.0);
    let before = b.doc.to_snapshot_v2();
    let preview = b.session.preview(&Default::default()).expect("the plan drawn");
    assert!(preview.strokes.len() > 12, "the cabinets drawn: {}", preview.strokes.len());
    let tag = preview.tag.expect("the run in words").lines.join(" | ");
    assert!(tag.contains("5 modül"), "{tag}");
    assert!(tag.contains("evye 2. · ocak 4. modül"), "{tag}");
    assert_eq!(b.doc.to_snapshot_v2(), before, "the preview writes nothing");
    b.click(1.8, -1.0);
    assert_eq!(
        inserted(&b),
        ["Alt dolap", "Alt dolap", "Alt dolap", "Alt dolap", "Alt dolap", "Evye", "Ocak"]
    );
    // The countertop on the pointer's side: below the line walked east.
    let top = b
        .doc
        .entities()
        .find_map(|e| match e {
            Entity::Polygon(p) => Some(p.pts.clone()),
            _ => None,
        })
        .expect("the countertop");
    assert!(top.iter().all(|p| p.y <= N + 1e-9 && p.y >= N - 0.6 - 1e-9), "{top:?}");
    assert!(top.iter().any(|p| (p.x - (E + 3.6)).abs() < 1e-9));
    // 0.90 + 4 × 0.60 = 3.30: a 0.30 m filler at the end.
    assert!(b.doc.entities().any(|e| matches!(e, Entity::Polyline(p) if p.closed)));
    assert!(b.doc.block_named("Alt dolap").is_some());
    assert_eq!(b.doc.undo().as_deref(), Some("Mutfak dizisi"));
    assert_eq!(b.doc.entities().count(), 0, "one undo takes the whole run back");
    assert!(b.doc.block_named("Alt dolap").is_none(), "and the blocks it brought");
}

#[test]
fn a_typed_length_enter_and_the_values_of_the_settings() {
    let mut b = bench();
    assert!(b.run(|s, cx| s.configure("M", "45 cm", cx)));
    assert!(b.run(|s, cx| s.configure("U", "yes", cx)));
    assert!(b.run(|s, cx| s.configure("E", "none", cx)));
    assert!(b.run(|s, cx| s.configure("O", "1", cx)));
    assert!(b.run(|s, cx| s.configure("Y", "left", cx)));
    assert!(!b.run(|s, cx| s.configure("M", "50 cm", cx)), "not one of the cabinet's widths");
    b.click(0., 0.);
    b.move_to(2.0, 0.);
    assert!(b.type_text("1.95"));
    b.move_to(1.0, -1.0);
    b.confirm();
    // Four modules, the cooktop's 60 cm in the first and three of 45 cm, a wall cabinet over each; on the left of the
    // line, as Yan says.
    assert_eq!(
        inserted(&b),
        [
            "Alt dolap", "Alt dolap", "Alt dolap", "Alt dolap", "Ocak", "Üst dolap", "Üst dolap", "Üst dolap",
            "Üst dolap"
        ]
    );
    let top = b
        .doc
        .entities()
        .find_map(|e| match e {
            Entity::Polygon(p) => Some(p.pts.clone()),
            _ => None,
        })
        .expect("the countertop");
    assert!(top.iter().all(|p| p.y >= N - 1e-9), "on the line's left: {top:?}");
    assert!(top.iter().any(|p| (p.x - (E + 1.95)).abs() < 1e-9), "the typed length: {top:?}");
    assert_eq!(b.memory.casework_module, 0.45, "the module stays for the next run");
}

#[test]
fn a_run_shorter_than_a_module_is_said_and_never_written() {
    let mut b = bench();
    b.click(0., 0.);
    b.click(0.2, 0.);
    b.move_to(0.1, -1.0);
    let preview = b.session.preview(&Default::default()).expect("the tag");
    assert!(preview.tag.expect("why").lines.join(" ").contains("modüle"));
    let before = b.doc.to_snapshot_v2();
    b.click(0.1, -1.0);
    assert_eq!(b.doc.to_snapshot_v2(), before, "nothing written");
    assert!(b.undo_step(), "the end taken back");
    assert_eq!(b.points(), 1);
}

/// The run the drawing keeps (docs/adr/0727 §5).
fn record(b: &Bench) -> kentos_commands::CaseworkRecord {
    assert_eq!(b.doc.casework().len(), 1, "one run");
    (*b.doc.casework()[0]).clone()
}

#[test]
fn an_l_is_written_with_its_end_on_the_side_it_turns_to() {
    let mut b = bench();
    assert!(b.type_text("B"), "Biçim: L");
    b.click(0., 3.0);
    b.click(4.2, 3.0);
    // The pointer off the square: the end goes along the square to the first leg.
    b.move_to(4.5, 0.6);
    let preview = b.session.preview(&Default::default()).expect("the plan drawn");
    let last = *preview.path.last().expect("the path");
    assert!((last.x - (E + 4.2)).abs() < 1e-9, "squared: {last:?}");
    let before = b.doc.to_snapshot_v2();
    assert_eq!(b.doc.to_snapshot_v2(), before, "the preview writes nothing");
    b.click(4.5, 0.6);
    let r = record(&b);
    assert_eq!(r.recipe.through.as_ref().map(Vec::len), Some(1));
    assert_eq!(r.recipe.side, Some(kentos_commands::CaseworkSide::Right), "it turns right");
    assert!((r.recipe.to.x - (E + 4.2)).abs() < 1e-9);
    // The countertop under the north leg and left of the east one.
    let top = b
        .doc
        .entities()
        .find_map(|e| match e {
            Entity::Polygon(p) => Some(p.pts.clone()),
            _ => None,
        })
        .expect("the countertop");
    assert!(top.iter().all(|p| p.y <= N + 3.0 + 1e-9 && p.x <= E + 4.2 + 1e-9), "{top:?}");
    assert!(r.members.iter().any(|m| m.part == kentos_commands::CaseworkPart::Corner), "a blind corner");
    assert_eq!(b.doc.undo().as_deref(), Some("Mutfak dizisi"));
    assert!(b.doc.casework().is_empty(), "one undo takes the run back");
}

#[test]
fn a_u_with_an_open_corner_an_oven_and_a_fridge() {
    let mut b = bench();
    assert!(b.run(|s, cx| s.configure("B", "u", cx)));
    assert!(b.run(|s, cx| s.configure("K", "open", cx)));
    assert!(b.run(|s, cx| s.configure("F", "yes", cx)));
    assert!(b.run(|s, cx| s.configure("Z", "end", cx)));
    assert!(!b.run(|s, cx| s.configure("B", "z", cx)), "not a form");
    b.click(0., 0.6);
    b.click(0., 3.0);
    b.click(4.2, 3.0);
    b.click(4.2, 0.6);
    let r = record(&b);
    assert_eq!(r.recipe.through.as_ref().map(Vec::len), Some(2), "a U");
    assert_eq!(r.recipe.corner, Some(kentos_commands::CaseworkCorner::Open));
    assert_eq!(r.recipe.oven, Some(true));
    assert_eq!(r.recipe.fridge, Some(kentos_commands::CaseworkEnd::End));
    assert!(r.members.iter().any(|m| m.part == kentos_commands::CaseworkPart::Fridge));
    assert_eq!(b.memory.casework_corners, 2, "the form stays for the next run");
}

/// Otomatik leaves a blind corner's module plain (docs/adr/0727 §1, §7): in this U the module after the sink is the
/// first corner's last one, its front behind the north leg's cabinets, so the dishwasher takes the one before the sink;
/// a dishwasher typed into it is said in the preview's tag.
#[test]
fn otomatik_puts_no_appliance_in_a_blind_corner() {
    let mut b = bench();
    assert!(b.run(|s, cx| s.configure("B", "u", cx)));
    assert!(b.run(|s, cx| s.configure("L", "auto", cx)));
    assert!(b.run(|s, cx| s.configure("Z", "start", cx)));
    b.click(0., 0.3);
    b.click(0., 3.0);
    b.click(4.2, 3.0);
    b.click(4.2, 0.3);
    let r = record(&b);
    assert_eq!(r.recipe.sink, Some(2));
    assert_eq!(r.recipe.dishwasher, Some(1), "not the blind 3rd module");
    let runs = match kentos_native_application::casework_query::plan_in(&b.doc, &Default::default()) {
        kentos_commands::CommandResult::Completed { output, .. } => output,
        other => panic!("{other:?}"),
    };
    let blind: Vec<_> = runs.runs[0].modules.iter().filter(|m| m.blind == Some(true)).collect();
    assert!(!blind.is_empty(), "a U with blind corners");
    assert!(
        blind.iter().all(|m| m.kind == kentos_commands::CaseworkModuleKind::Base),
        "Otomatik put nothing there: {blind:?}"
    );

    let mut b = bench();
    assert!(b.run(|s, cx| s.configure("B", "u", cx)));
    assert!(b.run(|s, cx| s.configure("L", "3", cx)));
    assert!(b.run(|s, cx| s.configure("Z", "start", cx)));
    b.click(0., 0.3);
    b.click(0., 3.0);
    b.click(4.2, 3.0);
    b.move_to(4.2, 0.3);
    let tag = b
        .session
        .preview(&Default::default())
        .and_then(|p| p.tag)
        .expect("the run in words")
        .lines
        .join(" | ");
    assert!(tag.contains("Kör köşede: 3. modül"), "{tag}");
}

/// A door on the wall the run stands along is said in the tag before the run is written, and with it (docs/adr/0727 §2).
#[test]
fn a_door_before_the_run_is_said_before_it_is_written() {
    use kentos_commands::CallOp;
    use kentos_native_application::{ExecutionContext, registry};
    use serde_json::{Value, json};
    fn run(b: &mut Bench, name: &str, input: Value) -> Value {
        let answer = registry::run(&mut ExecutionContext::new(&mut b.doc), name, CallOp::Execute, input).expect("runs");
        assert_eq!(answer["status"], "completed", "{name}: {answer}");
        answer
    }
    let mut b = bench();
    // A wall whose south face is the line y 3.00, a door K1 in it.
    let wall = run(
        &mut b,
        "cad.walls.create",
        json!({"layerId": "cizim", "walls": [{
            "pts": [{"x": E - 0.125, "y": N + 3.125}, {"x": E + 4.325, "y": N + 3.125}],
            "width": 0.25, "justify": "center",
        }]}),
    );
    let host = wall["output"]["uids"][0].clone();
    run(
        &mut b,
        "cad.openings.create",
        json!({"layerId": "cizim", "openings": [{"host": host, "at": 1.4, "number": "K1"}]}),
    );
    b.click(0., 3.0);
    b.click(4.2, 3.0);
    b.move_to(2.0, 2.0);
    let tag = b
        .session
        .preview(&Default::default())
        .and_then(|p| p.tag)
        .expect("the run in words")
        .lines
        .join(" ");
    assert!(tag.contains("1. kol K1 kapısının önüne geliyor"), "{tag}");
    assert!(b.doc.casework().is_empty(), "the preview writes nothing");
    let said = b.said(0).len();
    b.click(2.0, 2.0);
    assert_eq!(b.doc.casework().len(), 1, "written");
    assert!(
        b.said(said).iter().any(|(_, text)| text.contains("K1 kapısının önüne geliyor")),
        "{:?}",
        b.said(said)
    );
}

/// An L whose points are typed is written with its last one, and said as written, not as the preview of the points
/// before it.
#[test]
fn a_run_typed_is_said_as_it_is_written() {
    let mut b = bench();
    assert!(b.type_text("B"), "Biçim: L");
    for p in [[0.0, 3.0], [4.2, 3.0], [4.2, 0.6]] {
        assert!(b.type_text(&format!("{},{}", E + p[0], N + p[1])), "{p:?}");
    }
    assert_eq!(record(&b).recipe.through.as_ref().map(Vec::len), Some(1), "an L");
    let said = b.last_text().expect("said");
    assert!(said.starts_with("Mutfak dizisi çizildi: 8 modül"), "{said}");
    assert!(said.contains("Tezgâh 6.600 m"), "{said}");
}
