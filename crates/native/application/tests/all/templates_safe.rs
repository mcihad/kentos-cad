//! Safe package settings, undo/redo, file preservation and refusals (ADR 0630).
use kentos_commands::*;
use kentos_domain::Document;
use kentos_native_application::{ExecutionContext, templates_apply};
use serde_json::{Value, json};
fn done<T: std::fmt::Debug>(r: CommandResult<T>) -> T {
    match r {
        CommandResult::Completed { output, .. } => output,
        other => panic!("{other:?}"),
    }
}
fn drawing() -> Document {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/commands/v1/cad.templates.apply.review.json"
    ))
    .unwrap();
    let value = fixture["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["name"].as_str().unwrap().starts_with("Gizli kilitli"))
        .unwrap()["setup"]
        .clone();
    Document::from_snapshot(DocumentSnapshotV1::from_json(&value.to_string()).unwrap()).unwrap()
}
fn input(doc: &Document, mode: &str) -> TemplatesApply {
    serde_json::from_value(json!({"name":"mimari", "settingsMode":mode,
        "expectedRevision":doc.revision().to_string()}))
    .unwrap()
}
#[test]
fn safe_package_preserves_entities_definitions_and_all_parts_undo_and_redo_together() {
    let mut doc = drawing();
    let before = doc.to_snapshot_v2();
    let rev = doc.revision();
    let request = input(&doc, "keep");
    let plan = done(templates_apply::plan_in(&doc, &request));
    assert_eq!(doc.to_snapshot_v2(), before);
    assert_eq!(doc.revision(), rev);
    assert_eq!(plan.unit, DrawingUnit::M);
    assert_eq!(plan.review.as_ref().unwrap().plot_scale, 100.);
    let out = done(templates_apply::execute(
        &mut ExecutionContext::new(&mut doc),
        request,
    ));
    assert_eq!(out.layers, plan.layers);
    assert_eq!(out.styles, plan.styles);
    assert_eq!(out.review, plan.review);
    let after = doc.to_snapshot_v2();
    assert_eq!(after.entities, before.entities);
    assert_eq!(doc.settings().drawing_font, Some(DrawingFont::CourierPrime));
    assert_eq!(
        doc.settings().text_styles[0],
        before.settings.text_styles[0]
    );
    assert_eq!(
        doc.settings().wall_styles[0],
        before.settings.wall_styles[0]
    );
    assert_eq!(
        doc.settings().arch.as_ref().unwrap().levels[0],
        before.settings.arch.as_ref().unwrap().levels[0]
    );
    assert_eq!(
        doc.layers().get("eski"),
        before.layers.iter().find(|l| l.id == "eski")
    );
    assert_eq!(doc.settings().plot_scale, 100.);
    assert_eq!(
        kentos_native_application::sheets_edit::sheets_in(&doc)
            .unwrap()
            .len(),
        3
    );
    let bytes = kentos_kcad::encode_verified(&after).unwrap();
    let opened = Document::from_snapshot_v2(kentos_kcad::decode(&bytes).unwrap()).unwrap();
    assert_eq!(opened.to_snapshot_v2(), after);
    assert_eq!(doc.undo(), Some("Mimari şablonu".into()));
    assert_eq!(doc.to_snapshot_v2(), before);
    assert_eq!(doc.redo(), Some("Mimari şablonu".into()));
    assert_eq!(doc.to_snapshot_v2(), after);
    let request = input(&doc, "keep");
    let rev = doc.revision();
    let repeated = done(templates_apply::execute(
        &mut ExecutionContext::new(&mut doc),
        request,
    ));
    assert!(
        repeated.layers.is_empty()
            && repeated.styles.is_empty()
            && repeated.blocks.is_empty()
            && repeated.wall_styles.is_empty()
            && repeated.levels.is_empty()
            && repeated.sheets.is_empty()
    );
    assert_eq!(doc.revision(), rev);
    assert_eq!(doc.to_snapshot_v2(), after);
}
#[test]
fn explicit_template_policy_changes_only_shown_settings_and_undo_restores_them() {
    let mut doc = drawing();
    let before = doc.to_snapshot_v2();
    let request = input(&doc, "template");
    let plan = done(templates_apply::plan_in(&doc, &request));
    assert_eq!(plan.unit, DrawingUnit::Cm);
    assert_eq!(plan.review.as_ref().unwrap().previous_unit, DrawingUnit::M);
    assert_eq!(plan.review.as_ref().unwrap().plot_scale, 50.);
    done(templates_apply::execute(
        &mut ExecutionContext::new(&mut doc),
        request,
    ));
    assert_eq!(doc.settings().unit(), DrawingUnit::Cm);
    assert_eq!(doc.settings().plot_scale, 50.);
    assert_eq!(doc.to_snapshot_v2().entities, before.entities);
    doc.undo();
    assert_eq!(doc.to_snapshot_v2(), before);
}
#[test]
fn unreadable_book_and_stale_revision_are_atomic_refusals() {
    let mut doc = drawing();
    // The KCAD envelope allows an opaque JSON book. Its core must still be readable by the command.
    doc.set_sheets(Some(ProjectSheets {
        book: json!({"format":"unknown.book","version":1}),
        pictures: vec![],
    }))
    .unwrap();
    let before = doc.to_snapshot_v2();
    let request = input(&doc, "keep");
    for operation in ["validate", "plan", "execute"] {
        let result = match operation {
            "validate" => serde_json::to_value(templates_apply::validate(
                &ExecutionContext::new(&mut doc),
                &request,
            ))
            .unwrap(),
            "plan" => serde_json::to_value(templates_apply::plan_in(&doc, &request)).unwrap(),
            _ => serde_json::to_value(templates_apply::execute(
                &mut ExecutionContext::new(&mut doc),
                request.clone(),
            ))
            .unwrap(),
        };
        assert_eq!(result["error"]["code"], "book_unreadable", "{result}");
        assert_eq!(doc.to_snapshot_v2(), before);
    }
    let mut doc = drawing();
    let request = input(&doc, "keep");
    let mut settings = doc.settings().clone();
    settings.plot_scale = 200.;
    doc.set_settings(settings);
    let before = doc.to_snapshot_v2();
    assert!(matches!(
        templates_apply::execute(&mut ExecutionContext::new(&mut doc), request),
        CommandResult::Conflict { .. }
    ));
    assert_eq!(doc.to_snapshot_v2(), before);
}

#[test]
fn separate_identity_and_name_collisions_are_both_reported_and_preserved() {
    let mut doc = drawing();
    let mut settings = doc.settings().clone();
    settings.arch.as_mut().unwrap().levels.push(ArchLevel {
        id: "kurum-zemin".into(),
        name: "Zemin kat".into(),
        elevation: -1.,
        height: Some(4.),
    });
    doc.set_settings(settings);
    let before = doc.settings().arch.as_ref().unwrap().levels.clone();
    let request = input(&doc, "keep");
    let plan = done(templates_apply::plan_in(&doc, &request));
    let kept: Vec<_> = plan
        .review
        .as_ref()
        .unwrap()
        .preserved
        .iter()
        .filter(|p| p.kind == "level")
        .collect();
    assert_eq!(kept.len(), 2);
    assert_eq!(kept[0].matched_by, "id");
    assert_eq!(kept[1].matched_by, "name");
    done(templates_apply::execute(
        &mut ExecutionContext::new(&mut doc),
        request,
    ));
    for level in before {
        assert!(
            doc.settings()
                .arch
                .as_ref()
                .unwrap()
                .levels
                .contains(&level)
        );
    }
}

#[test]
fn package_undo_keeps_unrelated_preferences_and_settings_it_did_not_change() {
    let mut doc = drawing();
    let request = input(&doc, "keep");
    done(templates_apply::execute(
        &mut ExecutionContext::new(&mut doc),
        request,
    ));
    let mut settings = doc.settings().clone();
    settings.length_decimals = 5;
    settings.plot_scale = 250.;
    settings.drawing_unit = Some(DrawingUnit::Mm);
    settings.arch.as_mut().unwrap().phase_view = Some(PhaseView::Proposed);
    doc.set_settings(settings);
    doc.undo();
    assert_eq!(doc.settings().length_decimals, 5);
    assert_eq!(doc.settings().plot_scale, 250.);
    assert_eq!(doc.settings().unit(), DrawingUnit::Mm);
    assert_eq!(
        doc.settings().arch.as_ref().unwrap().phase_view,
        Some(PhaseView::Proposed)
    );
}
