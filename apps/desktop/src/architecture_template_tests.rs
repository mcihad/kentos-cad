use super::*;
use crate::files_testing::{app_with_drawing, make_cad};
use kentos_ui::snapshot::{Input, Snapshot};
fn app() -> App {
    let mut app = app_with_drawing();
    make_cad(&mut app);
    let mut settings = app.document.as_ref().unwrap().settings().clone();
    settings.srid = 0;
    settings.drawing_unit = Some(kentos_commands::DrawingUnit::M);
    settings.plot_scale = 100.;
    app.document.as_mut().unwrap().model.set_settings(settings);
    app
}
fn apply_event(app: &App) -> Message {
    let w = app.arch_template.as_ref().unwrap();
    Message::ArchTemplate(Event::Apply {
        session: w.session,
        input: w.input.clone(),
    })
}
#[test]
fn opening_changing_policy_and_cancelling_a_template_write_nothing() {
    let mut app = app();
    let before = app.document.as_ref().unwrap().model.to_snapshot_v2();
    let _ = app.update(Message::Run("arch.template"));
    assert_eq!(app.dialog, Some(Dialog::ArchTemplate));
    let w = app.arch_template.as_ref().unwrap();
    assert_eq!(w.input.settings_mode, Some(TemplateSettingsMode::Keep));
    assert_eq!(w.plan.unit, kentos_commands::DrawingUnit::M);
    assert_eq!(w.plan.review.as_ref().unwrap().plot_scale, 100.);
    let session = w.session;
    let _ = app.update(Message::ArchTemplate(Event::Mode {
        session,
        mode: TemplateSettingsMode::Template,
    }));
    let w = app.arch_template.as_ref().unwrap();
    assert_eq!(w.plan.unit, kentos_commands::DrawingUnit::Cm);
    assert_eq!(w.plan.review.as_ref().unwrap().plot_scale, 50.);
    assert_eq!(
        app.document.as_ref().unwrap().model.to_snapshot_v2(),
        before
    );
    let late = apply_event(&app);
    let _ = app.update(Message::DialogClosed);
    assert!(app.arch_template.is_none());
    let _ = app.update(late);
    assert_eq!(
        app.document.as_ref().unwrap().model.to_snapshot_v2(),
        before
    );
}
#[test]
fn applying_the_shown_plan_preserves_scale_and_undoes_the_whole_package() {
    let mut app = app();
    let before = app.document.as_ref().unwrap().model.to_snapshot_v2();
    let _ = app.update(Message::Run("arch.template"));
    let message = apply_event(&app);
    let _ = app.update(message);
    assert!(app.arch_template.is_none() && app.dialog.is_none());
    let doc = &mut app.document.as_mut().unwrap().model;
    assert_eq!(doc.settings().unit(), kentos_commands::DrawingUnit::M);
    assert_eq!(doc.settings().plot_scale, 100.);
    assert_eq!(
        kentos_native_application::sheets_edit::sheets_in(doc)
            .unwrap()
            .len(),
        3
    );
    assert_eq!(
        doc.journal().last().unwrap().command.as_deref(),
        Some("cad.templates.apply")
    );
    doc.undo();
    assert_eq!(doc.to_snapshot_v2(), before);
}
#[test]
fn changed_revisions_refresh_the_review_without_applying_and_old_documents_cannot_receive_it() {
    let mut app = app();
    let _ = app.update(Message::Run("arch.template"));
    let old = apply_event(&app);
    let mut settings = app.document.as_ref().unwrap().settings().clone();
    settings.plot_scale = 200.;
    app.document.as_mut().unwrap().model.set_settings(settings);
    let before = app.document.as_ref().unwrap().model.to_snapshot_v2();
    let _ = app.update(old);
    assert_eq!(
        app.document.as_ref().unwrap().model.to_snapshot_v2(),
        before
    );
    assert!(app.arch_template.as_ref().unwrap().notice.is_some());
    assert_eq!(
        app.arch_template
            .as_ref()
            .unwrap()
            .plan
            .review
            .as_ref()
            .unwrap()
            .plot_scale,
        200.
    );
    let old = apply_event(&app);
    let mut next = app_with_drawing();
    make_cad(&mut next);
    app.document = next.document.take();
    let before = app.document.as_ref().unwrap().model.to_snapshot_v2();
    let _ = app.update(old);
    assert_eq!(
        app.document.as_ref().unwrap().model.to_snapshot_v2(),
        before
    );
}
#[test]
fn the_narrow_preview_keeps_apply_and_cancel_visible_and_cancel_is_interactive() {
    use kentos_ui::theme::typography::{self, Typography};
    let _guard = crate::appearance::tests::TYPOGRAPHY.lock();
    for size in [13, 18] {
        for width in [1100., 1440.] {
            let mut app = app();
            let _ = app
                .settings
                .choose(&[("appearance.textSize", serde_json::json!(size))]);
            app.apply_settings();
            app.window_size = iced::Size::new(width, 650.);
            let before = app.document.as_ref().unwrap().model.to_snapshot_v2();
            let _ = app.update(Message::Run("arch.template"));
            let mut shot = Snapshot::software(app.window_size).unwrap();
            shot.settle(&mut app, App::view, &mut |a, m| {
                let _ = a.update(m);
            });
            for name in ["Uygula", "Vazgeç"] {
                let r = crate::files_testing::find_text(&mut shot, &app, name).expect(name);
                assert!(
                    r.x >= 0. && r.x + r.width <= width && r.y >= 0. && r.y + r.height <= 650.,
                    "{size}/{width}: {name} {r:?}"
                );
            }
            let cancel = crate::files_testing::find_text(&mut shot, &app, "Vazgeç").unwrap();
            shot.input(
                &mut app,
                App::view,
                &mut |a, m| {
                    let _ = a.update(m);
                },
                Input::Click(cancel.center()),
            );
            assert!(app.dialog.is_none());
            assert_eq!(
                app.document.as_ref().unwrap().model.to_snapshot_v2(),
                before
            );
        }
    }
    typography::set(Typography::DEFAULT);
}

#[test]
fn a_projected_cad_previews_and_applies_the_office_scale_in_metres() {
    let mut app = app();
    let mut settings = app.document.as_ref().unwrap().settings().clone();
    settings.srid = 5256;
    // A legacy file can have a redundant display unit; the coordinate system wins.
    settings.drawing_unit = Some(kentos_commands::DrawingUnit::Cm);
    app.document.as_mut().unwrap().model.set_settings(settings);
    let before = app.document.as_ref().unwrap().model.to_snapshot_v2();
    let _ = app.update(Message::Run("arch.template"));
    let w = app.arch_template.as_ref().unwrap();
    assert_eq!(w.plan.unit, kentos_commands::DrawingUnit::M);
    let session = w.session;
    let _ = app.update(Message::ArchTemplate(Event::Mode {
        session,
        mode: TemplateSettingsMode::Template,
    }));
    let w = app.arch_template.as_ref().unwrap();
    assert_eq!(w.input.unit, Some(kentos_commands::DrawingUnit::M));
    assert_eq!(w.plan.unit, kentos_commands::DrawingUnit::M);
    assert_eq!(w.plan.review.as_ref().unwrap().plot_scale, 50.);
    assert_eq!(
        app.document.as_ref().unwrap().model.to_snapshot_v2(),
        before
    );
    let event = apply_event(&app);
    let _ = app.update(event);
    let doc = &mut app.document.as_mut().unwrap().model;
    assert_eq!(doc.settings().unit(), kentos_commands::DrawingUnit::M);
    assert_eq!(doc.settings().srid, 5256);
    assert_eq!(doc.settings().plot_scale, 50.);
    doc.undo();
    assert_eq!(doc.to_snapshot_v2(), before);
}
