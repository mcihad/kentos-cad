//! Esnet over a drawing whose objects are tied to others (docs/adr/0297 §2,
//! §3): what the shared cases cannot set up, since their drawing holds no
//! persistent ids to link a text to. A linked label text and an associative
//! hatch do not stretch by themselves; they follow their objects, and the
//! plan shows them as they will be (`dependents`), also on a copy kept
//! between previews.

use kentos_domain::contracts::{
    CommandResult, DocumentSnapshotV1, EntitiesStretch, EntitiesStretchPlan, Entity, EntityId,
    HatchAssoc, StretchWindow, Vec2,
};
use kentos_domain::{Document, Slot};
use kentos_native_application::{ExecutionContext, codes, stretch};
use serde_json::json;

/// An area (1), a labelled polyline (2), a text linked to 2 (3) and a hatch tied to 1 (4).
fn drawing() -> Document {
    let snapshot = json!({"format":"kentos.document","version":1,"name":"Esnet ve bağlar","settings":{"srid":5256,
        "lengthDecimals":3,"areaDecimals":2,"areaUnit":"m2","angleUnit":"grad","plotScale":100},
        "origin":{"x":0,"y":0},"layers":[{"id":"a","name":"A","type":"layer","visible":true,"locked":false,
        "expanded":true,"style":{"color":"fg","lineType":"continuous","lineWeight":0.18},"children":[]}],
        "activeLayer":"a","entities":[
        {"kind":"polygon","id":1,"layerId":"a","attrs":{},"pts":[{"x":0,"y":0},{"x":10,"y":0},{"x":10,"y":10},{"x":0,"y":10}]},
        {"kind":"polyline","id":2,"layerId":"a","attrs":{},"label":"Yol","pts":[{"x":0,"y":40},{"x":10,"y":40},{"x":10,"y":50}]},
        {"kind":"text","id":3,"layerId":"a","attrs":{},"p":{"x":5,"y":40},"text":"Yol","height":2.5,"rotation":0},
        {"kind":"hatch","id":4,"layerId":"a","attrs":{},"ring":[{"x":0,"y":0},{"x":10,"y":0},{"x":10,"y":10},{"x":0,"y":10}],
         "pattern":{"type":"lines","angle":0,"spacing":1.5}}],
        "styles":{"items":[],"categories":[]}});
    let mut doc = Document::from_snapshot(
        DocumentSnapshotV1::from_json(&snapshot.to_string()).expect("reads"),
    )
    .expect("opens");
    let id = |doc: &Document, s: u32| EntityId(doc.uid(Slot(s)).expect("known").into_bytes());
    let Some(Entity::Text(t)) = doc.get(Slot(3)).cloned() else {
        panic!("a text")
    };
    let linked = Entity::Text(kentos_domain::contracts::TextEntity {
        label_of: Some(id(&doc, 2)),
        label_scale: Some(100.0),
        ..t
    });
    assert!(doc.update(Slot(3), linked));
    let Some(Entity::Hatch(h)) = doc.get(Slot(4)).cloned() else {
        panic!("a hatch")
    };
    let tied = Entity::Hatch(kentos_domain::contracts::HatchEntity {
        boundary: None,
        assoc: Some(HatchAssoc {
            gap: None,
            outer: Some(id(&doc, 1)),
            bounds: Vec::new(),
            islands: Vec::new(),
            cutouts: Vec::new(),
            seed: Vec2 { x: 5.0, y: 5.0 },
        }),
        ..h
    });
    assert!(doc.update(Slot(4), tied));
    doc
}

fn uid(doc: &Document, s: u32) -> String {
    doc.uid(Slot(s)).expect("known").to_string()
}

fn input(min: (f64, f64), max: (f64, f64), dx: f64, dy: f64) -> EntitiesStretch {
    EntitiesStretch {
        window: StretchWindow {
            min: Vec2 { x: min.0, y: min.1 },
            max: Vec2 { x: max.0, y: max.1 },
        },
        dx,
        dy,
        uids: None,
        topology: None,
        expected_revision: None,
    }
}

fn plan(doc: &Document, input: &EntitiesStretch) -> EntitiesStretchPlan {
    match stretch::plan_in(doc, None, input) {
        CommandResult::Completed { output, .. } => output,
        other => panic!("not completed: {other:?}"),
    }
}

fn ids(list: &[Entity]) -> Vec<u32> {
    list.iter().map(|e| e.base().id).collect()
}

#[test]
fn a_linked_text_in_the_window_follows_its_object_and_the_plan_shows_it() {
    let mut doc = drawing();
    let before = doc.get(Slot(3)).cloned();
    // The window takes the polyline's vertex (10, 40) and the text's insertion point (5, 40).
    let stretch = input((4.0, 39.0), (11.0, 41.0), 4.0, 0.0);
    let planned = plan(&doc, &stretch);
    assert_eq!(
        ids(&planned.stretched),
        vec![2],
        "the text does not stretch by itself"
    );
    assert_eq!(ids(&planned.dependents), vec![3], "it follows its polyline");
    let Some(Entity::Text(t)) = planned.dependents.first() else {
        panic!("a text")
    };
    assert!(t.label_of.is_some(), "still linked");
    assert_eq!(doc.get(Slot(3)).cloned(), before, "the plan writes nothing");

    let done = stretch::execute(&mut ExecutionContext::new(&mut doc), stretch);
    let CommandResult::Completed { output, .. } = done else {
        panic!("not completed: {done:?}")
    };
    assert_eq!(output.dependents, vec![uid(&doc, 3)]);
    assert_eq!(
        doc.get(Slot(3)),
        planned.dependents.first(),
        "the write is the plan"
    );
    doc.undo();
    assert_eq!(doc.get(Slot(3)).cloned(), before, "one undo takes it back");
}

#[test]
fn a_listed_linked_text_or_hatch_follows_its_object_with_a_warning() {
    let mut doc = drawing();
    for (slot, window) in [
        (3, ((4.0, 39.0), (6.0, 41.0))),
        (4, ((9.0, 9.0), (11.0, 11.0))),
    ] {
        let mut listed = input(window.0, window.1, 1.0, 1.0);
        listed.uids = Some(vec![uid(&doc, slot)]);
        let got = stretch::execute(&mut ExecutionContext::new(&mut doc), listed);
        let CommandResult::Failed { error } = got else {
            panic!("not refused: {got:?}")
        };
        assert_eq!(error.code, codes::NOTHING_TO_STRETCH, "{slot}");
        let both = EntitiesStretch {
            uids: Some(vec![uid(&doc, slot), uid(&doc, 1)]),
            ..input((-1.0, -1.0), (11.0, 41.0), 1.0, 1.0)
        };
        let warned = stretch::validate(&ExecutionContext::new(&mut doc), &both);
        let CommandResult::Completed { warnings, .. } = warned else {
            panic!("not completed: {warned:?}")
        };
        assert!(
            warnings
                .iter()
                .any(|w| w.code == codes::FOLLOWS_OBJECT && w.path.as_deref() == Some("uids[0]")),
            "{slot}: {warnings:?}"
        );
    }
}

#[test]
fn an_associative_hatch_follows_its_area_and_stays_tied() {
    let mut doc = drawing();
    let stretch = input((9.0, 9.0), (11.0, 11.0), 2.0, 1.0);
    let planned = plan(&doc, &stretch);
    assert_eq!(ids(&planned.stretched), vec![1]);
    assert_eq!(ids(&planned.dependents), vec![4]);
    let done = stretch::execute(&mut ExecutionContext::new(&mut doc), stretch);
    assert!(matches!(done, CommandResult::Completed { .. }), "{done:?}");
    match doc.get(Slot(4)) {
        Some(Entity::Hatch(h)) => {
            assert!(h.assoc.is_some(), "still tied");
            assert!(
                h.ring.iter().any(|p| p.x == 12.0 && p.y == 11.0),
                "{:?}",
                h.ring
            );
        }
        other => panic!("not a hatch: {other:?}"),
    }
}

#[test]
fn the_kept_copy_answers_as_a_fresh_one_and_is_renewed_when_the_drawing_changes() {
    let mut doc = drawing();
    let mut scratch = None;
    let first = input((4.0, 39.0), (11.0, 41.0), 4.0, 0.0);
    let fresh = plan(&doc, &first);
    for _ in 0..3 {
        let kept = match stretch::plan_with(&doc, None, &first, &mut scratch) {
            CommandResult::Completed { output, .. } => output,
            other => panic!("not completed: {other:?}"),
        };
        assert_eq!(kept, fresh, "the copy is left as it was");
    }
    let done = stretch::execute(&mut ExecutionContext::new(&mut doc), first.clone());
    assert!(matches!(done, CommandResult::Completed { .. }));
    // The drawing moved on: the copy is renewed, not answered from.
    let again = input((13.0, 39.0), (15.0, 41.0), -4.0, 0.0);
    let kept = match stretch::plan_with(&doc, None, &again, &mut scratch) {
        CommandResult::Completed { output, .. } => output,
        other => panic!("not completed: {other:?}"),
    };
    assert_eq!(kept, plan(&doc, &again));
}

#[test]
fn nothing_follows_and_no_copy_is_made_in_a_drawing_without_ties() {
    let snapshot = json!({"format":"kentos.document","version":1,"name":"Bağsız","settings":{"srid":0,
        "lengthDecimals":3,"areaDecimals":2,"areaUnit":"m2","angleUnit":"deg","plotScale":100},
        "origin":{"x":0,"y":0},"layers":[{"id":"a","name":"A","type":"layer","visible":true,"locked":false,
        "expanded":true,"style":{"color":"fg","lineType":"continuous","lineWeight":0.18},"children":[]}],
        "activeLayer":"a","entities":[
        {"kind":"line","id":1,"layerId":"a","attrs":{},"a":{"x":0,"y":0},"b":{"x":10,"y":0}}],
        "styles":{"items":[],"categories":[]}});
    let doc = Document::from_snapshot(
        DocumentSnapshotV1::from_json(&snapshot.to_string()).expect("reads"),
    )
    .expect("opens");
    assert!(!doc.can_follow());
    let mut scratch = None;
    let Some(e) = doc.get(Slot(1)).cloned() else {
        panic!("a line")
    };
    assert!(doc.followers_with(&mut scratch, &[(Slot(1), e)]).is_empty());
    assert!(scratch.is_none(), "no copy made");
}

/// The plan's cost with things that follow, in a drawing of 100 000 more lines (docs/adr/0297 §5): a copy made
/// for each plan, and one kept between plans as the tool keeps it. Run with
/// `cargo test --release -p kentos-native-application --test all stretch_ties::timing -- --ignored --nocapture`.
#[test]
#[ignore = "a timing, run on purpose"]
fn timing() {
    use std::time::Instant;
    let mut doc = drawing();
    let mut lines = Vec::with_capacity(100_000);
    for i in 0..100_000u32 {
        let (x, y) = (
            f64::from(i % 400) * 3.0 + 100.0,
            f64::from(i / 400) * 3.0 + 100.0,
        );
        let line = json!({"kind":"line","id":0,"layerId":"a","attrs":{},"a":{"x":x,"y":y},"b":{"x":x + 2.0,"y":y}});
        lines.push(serde_json::from_value::<Entity>(line).expect("a line"));
    }
    doc.add_many(lines, "Çizgiler").expect("added");
    // The host's geometry store, as the tool has it (the window's objects are found in it).
    let store = kentos_native_application::geometry_store::store_of(&doc);
    let stretch = input((9.0, 9.0), (11.0, 11.0), 2.0, 1.0);
    let runs = 20;
    let t = Instant::now();
    for _ in 0..runs {
        let CommandResult::Completed { output, .. } =
            stretch::plan_in(&doc, Some(&store), &stretch)
        else {
            panic!("not completed")
        };
        assert_eq!(output.dependents.len(), 1);
    }
    let fresh = t.elapsed().as_secs_f64() * 1e3 / f64::from(runs);
    let mut scratch = None;
    let _ = stretch::plan_with(&doc, Some(&store), &stretch, &mut scratch);
    let t = Instant::now();
    for _ in 0..runs {
        let CommandResult::Completed { output, .. } =
            stretch::plan_with(&doc, Some(&store), &stretch, &mut scratch)
        else {
            panic!("not completed")
        };
        assert_eq!(output.dependents.len(), 1);
    }
    let kept = t.elapsed().as_secs_f64() * 1e3 / f64::from(runs);
    let lone = input((1000.0, 1000.0), (1010.0, 1010.0), 2.0, 1.0);
    let t = Instant::now();
    for _ in 0..runs {
        let _ = stretch::plan_with(&doc, Some(&store), &lone, &mut scratch);
    }
    let untied = t.elapsed().as_secs_f64() * 1e3 / f64::from(runs);
    println!(
        "100 000 çizgi: kopya her planda {fresh:.2} ms, tutulan kopya {kept:.2} ms, bağsız nesneler {untied:.2} ms"
    );
    assert!(kept < 4.0, "a preview within a frame's share: {kept:.2} ms");
}
