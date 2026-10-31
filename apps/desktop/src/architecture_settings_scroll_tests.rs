//! A real widget scroll survives edits, but must not hide the next tool's controls.
use super::*;
use crate::app::Panel;
use crate::files_testing::{app_with_drawing, make_cad, offscreen};
use iced::advanced::widget::{Id, Operation, operation::Scrollable};
use iced::{Rectangle, Size, Vector};
use kentos_ui::snapshot::Snapshot;
use std::sync::{Arc, Mutex};

fn drive(shot: &mut Snapshot, app: &mut App, task: Task<Message>) {
    use iced::futures::StreamExt as _;
    if let Some(mut stream) = iced_runtime::task::into_stream(task) {
        while let Some(action) = iced::futures::executor::block_on(stream.next()) {
            if let iced_runtime::Action::Widget(op) = action {
                shot.operate(app.view(), op);
            }
        }
    }
}

/// Only the properties dock is on the right; the ribbon is above and the
/// console below. Inspect the actual widget offset, not a helper's requested offset.
fn offset(shot: &mut Snapshot, app: &App, down: bool) -> f32 {
    struct Find {
        down: bool,
        found: Arc<Mutex<Option<f32>>>,
    }
    impl Operation for Find {
        fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
            operate(self);
        }
        fn scrollable(
            &mut self,
            _: Option<&Id>,
            bounds: Rectangle,
            _: Rectangle,
            translation: Vector,
            state: &mut dyn Scrollable,
        ) {
            if bounds.x > 600.0 && bounds.y > 100.0 && bounds.height > 100.0 {
                *self.found.lock().unwrap() = Some(translation.y);
                if self.down {
                    state.snap_to(iced::widget::scrollable::RelativeOffset::END.into());
                }
            }
        }
    }
    let found = Arc::new(Mutex::new(None));
    shot.operate(
        app.view(),
        Box::new(Find {
            down,
            found: found.clone(),
        }),
    );
    found.lock().unwrap().expect("properties scroll widget")
}

#[test]
fn a_new_tool_reveals_defaults_without_resetting_scroll_on_setting_edits() {
    let _guard = crate::appearance::tests::TYPOGRAPHY.lock();
    let mut app = app_with_drawing();
    make_cad(&mut app);
    app.selection.set([kentos_domain::Slot(1)]);
    let _ = app
        .settings
        .choose(&[("appearance.textSize", serde_json::json!(18))]);
    app.apply_settings();
    app.window_size = Size::new(1100., 650.);
    for panel in [
        Panel::Layers,
        Panel::Processing,
        Panel::Blocks,
        Panel::Templates,
        Panel::Sources,
        Panel::Navigator,
    ] {
        app.docks.close(panel);
    }
    app.docks
        .dock(Panel::Properties, kentos_ui::widget::docking::Side::Right);
    app.docks
        .set_size(kentos_ui::widget::docking::Side::Right, 250.);
    let before = app.document.as_ref().unwrap().model.to_snapshot_v2();
    let selection = app.selection.ids().to_vec();
    let mut shot = offscreen(app.window_size);
    let task = app.start_tool("tag");
    drive(&mut shot, &mut app, task);
    offset(&mut shot, &app, true);
    assert!(offset(&mut shot, &app, false) > 40.);
    let task = app.tool_setting(Commit {
        tool: "tag",
        key: "Y",
        value: "3.5 mm".into(),
    });
    let previous = offset(&mut shot, &app, false);
    drive(&mut shot, &mut app, task);
    assert_eq!(
        offset(&mut shot, &app, false),
        previous,
        "Editing must retain scroll"
    );
    let task = app.start_tool("space");
    drive(&mut shot, &mut app, task);
    assert_eq!(
        offset(&mut shot, &app, false),
        0.,
        "New settings must start in view"
    );
    offset(&mut shot, &app, true);
    assert!(offset(&mut shot, &app, false) > 40.);
    let task = app.start_tool("tag");
    drive(&mut shot, &mut app, task);
    assert_eq!(offset(&mut shot, &app, false), 0.);
    assert_eq!(
        app.document.as_ref().unwrap().model.to_snapshot_v2(),
        before
    );
    assert_eq!(app.selection.ids(), selection);
    kentos_ui::theme::typography::set(kentos_ui::theme::typography::Typography::DEFAULT);
}
