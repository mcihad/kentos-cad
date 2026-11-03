//! App state acceptance: the worker, review, explicit keep, stale results and real Job commit.
use super::*;
use kentos_commands::{DimensionsCreate, DocumentSnapshotV1};
use kentos_domain::{Document as Model, Slot};
use kentos_native_application::ExecutionContext;
use serde_json::{Value, json};

fn app_with_array_source() -> App {
    let fixture: Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/commands/v1/cad.arrays.adopt.json"
    )))
    .unwrap();
    let mut model =
        Model::from_snapshot(DocumentSnapshotV1::from_json(&fixture["setup"].to_string()).unwrap())
            .unwrap();
    let a = model.uid(Slot(4)).unwrap().to_string();
    let b = model.uid(Slot(5)).unwrap().to_string();
    let input: DimensionsCreate = serde_json::from_value(json!({"measure":{"kind":"aligned",
        "a":{"kind":"place","uid":a,"path":[0],"place":{"kind":"start"}},
        "b":{"kind":"place","uid":b,"path":[0],"place":{"kind":"start"}}},
        "at":{"x":7.,"y":1.},"layerId":"a","height":0.18}))
    .unwrap();
    let made = kentos_native_application::dimensions_create::execute(
        &mut ExecutionContext::new(&mut model),
        input,
    );
    assert!(matches!(made, CommandResult::Completed { .. }), "{made:?}");
    let mut app = crate::files_testing::app_with_drawing();
    app.document = Some(crate::document::Document::from_v2(model.to_snapshot_v2(), None).unwrap());
    crate::files_testing::make_cad(&mut app);
    app.selection.set([Slot(4)]);
    app
}
fn complete_search(app: &mut App) {
    let _ = app.array_find_event(Event::Run);
    assert_eq!(app.dialog, None);
    let w = app.blocks_panel.arrays.as_ref().unwrap();
    let token = w.token;
    let doc = &app.document.as_ref().unwrap().model;
    let input = w.input(doc).unwrap();
    let answer = kentos_native_application::arrays_find::inspect(doc, &input, || false);
    let _ = app.array_find_event(Event::Ready(token, Box::new(answer)));
}
fn complete_review(app: &mut App) {
    let w = app.blocks_panel.arrays.as_ref().unwrap();
    let token = w.token;
    let answer = kentos_native_application::arrays_adopt::inspect(
        &app.document.as_ref().unwrap().model,
        &w.review.as_ref().unwrap().input,
        || false,
    );
    let _ = app.array_find_event(Event::Reviewed(token, Box::new(answer)));
}
fn ready_review(app: &mut App) {
    app.open_array_find();
    complete_search(app);
    let _ = app.array_find_event(Event::Review(0));
    complete_review(app);
}

#[test]
fn search_show_review_assignment_and_real_job_preserve_all_roots_dimension_refs_and_one_undo() {
    let mut app = app_with_array_source();
    let before = app.document.as_ref().unwrap().model.to_snapshot_v2();
    let selected = app.selection.ids().to_vec();
    ready_review(&mut app);
    assert_eq!(app.selection.ids(), selected);
    assert_eq!(
        app.dialog_title().as_deref(),
        Some("Diziye bağlamayı incele")
    );
    let _ = app.array_find_event(Event::Show(0));
    assert_eq!(app.dialog, None);
    assert_eq!(app.selection.ids().len(), 5);
    assert_eq!(
        app.document.as_ref().unwrap().model.to_snapshot_v2(),
        before
    );
    let _ = app.array_find_event(Event::Review(0));
    let w = app.blocks_panel.arrays.as_ref().unwrap();
    let r = w.review.as_ref().unwrap();
    assert_eq!(r.choices.len(), 1);
    assert_eq!(r.place, 2.);
    assert_eq!(r.plan.as_ref().unwrap().missing, 3);
    assert_eq!(r.plan.as_ref().unwrap().kept, 0);
    let _ = app.array_find_event(Event::KeepOptions);
    let _ = app.array_find_event(Event::Assign);
    complete_review(&mut app);
    let r = app
        .blocks_panel
        .arrays
        .as_ref()
        .unwrap()
        .review
        .as_ref()
        .unwrap();
    assert_eq!(
        (
            r.plan.as_ref().unwrap().missing,
            r.plan.as_ref().unwrap().kept
        ),
        (2, 1)
    );
    assert!(available_choices(r).is_empty());
    assert!(r.input.items[1].kept);
    assert!(
        app.array_find_control(Control::Press("Yer 2 atamasını kaldır"))
            .unwrap()
            .is_some()
    );
    assert!(
        app.array_find_control(Control::Press("Yer 1 atamasını kaldır"))
            .unwrap()
            .is_none()
    );
    assert_eq!(
        app.document.as_ref().unwrap().model.to_snapshot_v2(),
        before,
        "review is read only"
    );
    let _ = app.array_find_event(Event::Unassign(1));
    complete_review(&mut app);
    assert_eq!(
        app.blocks_panel
            .arrays
            .as_ref()
            .unwrap()
            .review
            .as_ref()
            .unwrap()
            .plan
            .as_ref()
            .unwrap()
            .missing,
        3
    );
    let _ = app.array_find_event(Event::Assign);
    complete_review(&mut app);
    let _ = app.array_find_event(Event::Apply);
    for _ in 0..5000 {
        let _ = app.array_find_event(Event::Tick);
        if app.blocks_panel.arrays.is_none() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    assert!(
        app.blocks_panel.arrays.is_none(),
        "job must finish and close review"
    );
    let model = &mut app.document.as_mut().unwrap().model;
    let after = model.to_snapshot_v2();
    assert_eq!(after.entities, before.entities);
    assert_eq!(after.uids, before.uids);
    assert_eq!(after.blocks, before.blocks);
    assert_eq!(after.settings, before.settings);
    assert_eq!(after.arrays.len(), 1);
    assert!(after.arrays[0].items[1].kept);
    assert!(after.arrays[0].items[2].uid.is_none() && after.arrays[0].items[6].uid.is_none());
    assert_eq!(
        model.undo().as_deref(),
        Some("Mevcut blokları diziye bağla")
    );
    assert_eq!(model.to_snapshot_v2(), before);
    model.redo();
    assert_eq!(model.to_snapshot_v2(), after);
}

#[test]
fn stopped_and_late_results_never_replace_a_new_search_or_commit_a_cancelled_job() {
    let mut app = app_with_array_source();
    app.open_array_find();
    let _ = app.array_find_event(Event::Run);
    let w = app.blocks_panel.arrays.as_ref().unwrap();
    let token = w.token;
    let answer = kentos_native_application::arrays_find::inspect(
        &app.document.as_ref().unwrap().model,
        &w.input(&app.document.as_ref().unwrap().model).unwrap(),
        || false,
    );
    let before = app.document.as_ref().unwrap().model.to_snapshot_v2();
    let _ = app.array_find_event(Event::Stop);
    let _ = app.array_find_event(Event::Ready(token, Box::new(answer)));
    assert!(app.blocks_panel.arrays.as_ref().unwrap().output.is_none());
    complete_search(&mut app);
    let _ = app.array_find_event(Event::Review(0));
    complete_review(&mut app);
    let _ = app.array_find_event(Event::Apply);
    assert!(app.blocks_panel.arrays.as_ref().unwrap().job.is_some());
    let _ = app.array_find_event(Event::Stop);
    let _ = app.array_find_event(Event::Tick);
    assert_eq!(
        app.document.as_ref().unwrap().model.to_snapshot_v2(),
        before
    );
    assert!(
        app.array_find_control(Control::Press("Diziye bağla"))
            .unwrap()
            .is_none()
    );
    let _ = app.array_find_event(Event::Refresh);
    complete_review(&mut app);
    assert!(
        app.array_find_control(Control::Press("Diziye bağla"))
            .unwrap()
            .is_some(),
        "stop has a direct recovery path"
    );
}

#[test]
fn a_changed_document_rejects_old_review_and_late_query_even_after_undo() {
    for reviewing in [false, true] {
        let mut app = app_with_array_source();
        app.open_array_find();
        if reviewing {
            complete_search(&mut app);
            let _ = app.array_find_event(Event::Review(0));
            complete_review(&mut app);
        } else {
            let _ = app.array_find_event(Event::Run);
        }
        let w = app.blocks_panel.arrays.as_ref().unwrap();
        let token = w.token;
        let input = w.input(&app.document.as_ref().unwrap().model).unwrap();
        let answer = kentos_native_application::arrays_find::inspect(
            &app.document.as_ref().unwrap().model,
            &input,
            || false,
        );
        let doc = &mut app.document.as_mut().unwrap().model;
        let Entity::Insert(mut moved) = doc.get(Slot(1)).unwrap().clone() else {
            panic!("insert")
        };
        moved.p.x += 1.;
        doc.update(Slot(1), Entity::Insert(moved));
        doc.undo();
        let before = doc.to_snapshot_v2();
        let event = if reviewing {
            Event::Apply
        } else {
            Event::Ready(token, Box::new(answer))
        };
        let _ = app.array_find_event(event);
        assert_eq!(
            app.document.as_ref().unwrap().model.to_snapshot_v2(),
            before
        );
        let w = app.blocks_panel.arrays.as_ref().unwrap();
        assert!(w.job.is_none());
        assert!(!w.busy);
        assert!(w.refused.as_ref().unwrap().contains("Çizim değişti"));
        assert!(w.review.as_ref().is_none_or(|r| r.plan.is_none()));
    }
}

#[test]
fn occupied_source_places_are_never_overwritten_and_invalid_local_assignment_blocks_apply() {
    let mut app = app_with_array_source();
    ready_review(&mut app);
    let before = app.document.as_ref().unwrap().model.to_snapshot_v2();
    for place in [1., 4., f64::NAN, 2.5, 10001.] {
        let _ = app.array_find_event(Event::KeepPlace(place));
        let _ = app.array_find_event(Event::Assign);
        assert!(app.blocks_panel.arrays.as_ref().unwrap().refused.is_some());
        assert!(
            app.array_find_control(Control::Press("Diziye bağla"))
                .unwrap()
                .is_none()
        );
        let _ = app.array_find_event(Event::Apply);
        assert!(app.blocks_panel.arrays.as_ref().unwrap().job.is_none());
        assert_eq!(
            app.document.as_ref().unwrap().model.to_snapshot_v2(),
            before
        );
    }
    let _ = app.array_find_event(Event::KeepPlace(2.));
    let _ = app.array_find_event(Event::Assign);
    complete_review(&mut app);
    assert!(
        app.array_find_control(Control::Press("Diziye bağla"))
            .unwrap()
            .is_some()
    );
}

#[test]
fn missing_source_invalid_tolerance_and_cancelled_options_leave_existing_search_intact() {
    let mut app = app_with_array_source();
    app.selection.clear();
    app.open_array_find();
    assert!(
        app.array_find_control(Control::Press("Bul"))
            .unwrap()
            .is_none()
    );
    app.array_find_close_dialog();
    assert!(app.blocks_panel.arrays.is_none());
    app.selection.set([Slot(4)]);
    app.open_array_find();
    let _ = app.array_find_event(Event::Tolerance(0.));
    assert!(
        app.array_find_control(Control::Press("Bul"))
            .unwrap()
            .is_none()
    );
    let _ = app.array_find_event(Event::Tolerance(1e-6));
    complete_search(&mut app);
    let before = app.blocks_panel.arrays.as_ref().unwrap().output.clone();
    let _ = app.array_find_event(Event::Settings);
    let _ = app.array_find_event(Event::Hidden);
    let _ = app.array_find_event(Event::Tolerance(0.0001));
    app.array_find_close_dialog();
    let w = app.blocks_panel.arrays.as_ref().unwrap();
    assert!(!w.hidden);
    assert_eq!(w.tolerance, 1e-6);
    assert_eq!(w.output, before);
    let _ = app.array_find_event(Event::Review(0));
    complete_review(&mut app);
    let _ = app.array_find_event(Event::ShowKept);
    assert_eq!(app.dialog, None);
    assert_eq!(app.selection.ids(), [Slot(2)]);
    let _ = app.array_find_event(Event::Review(0));
    assert_eq!(app.dialog, Some(Dialog::ArrayFind));
    assert!(
        app.blocks_panel
            .arrays
            .as_ref()
            .unwrap()
            .review
            .as_ref()
            .unwrap()
            .plan
            .is_some()
    );
    app.open_block_find();
    assert!(
        app.blocks_panel.arrays.is_none(),
        "the old block workflow remains independent"
    );
    assert!(app.blocks_panel.find.is_some());
}

#[test]
fn typed_aliases_open_the_same_cad_dialog_and_never_run_without_a_document() {
    for name in ["array.find", "DIZIBUL", "dizibul", "DİZİBUL", "FINDARRAYS"] {
        let mut app = app_with_array_source();
        let _ = app.run_typed(name);
        assert_eq!(app.dialog, Some(Dialog::ArrayFind), "{name}: {:?}", app.log);
        assert!(app.blocks_panel.arrays.as_ref().unwrap().source.is_some());
    }
    let mut architecture = app_with_array_source();
    architecture.program = Some(kentos_launch::Module::Architecture);
    assert!(architecture.available("array.find"));
    let _ = architecture.run_typed("DIZIBUL");
    assert_eq!(architecture.dialog, Some(Dialog::ArrayFind));
    let (empty, _) = App::boot(None);
    assert!(!empty.available("array.find"));
}

#[test]
fn queries_respect_python_echo_and_closing_the_dock_cancels_the_worker() {
    use kentos_ui::widget::python::EntryKind;
    let mut app = app_with_array_source();
    app.open_array_find();
    complete_search(&mut app);
    assert!(!app.python.repl.entries().any(|e| e.kind == EntryKind::Echo));
    assert!(
        app.settings
            .choose(&[(crate::python::echo::SETTING, json!(true))])
            .is_empty()
    );
    let _ = app.array_find_event(Event::Run);
    let echoes: Vec<_> = app
        .python
        .repl
        .entries()
        .filter(|e| e.kind == EntryKind::Echo)
        .collect();
    assert_eq!(echoes.len(), 1);
    assert!(echoes[0].text.contains("cad.arrays.find"));
    let cancel = Arc::clone(&app.blocks_panel.arrays.as_ref().unwrap().cancel);
    let _ = app.update(Message::Dock(kentos_ui::widget::docking::Event::Closed(
        crate::app::Panel::Blocks,
    )));
    assert!(cancel.load(Ordering::Relaxed));
    assert!(app.blocks_panel.arrays.is_none());
}
