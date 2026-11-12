use super::*;
use iced::advanced::widget::{Id, Operation, operation::TextInput};
use iced::{Point, Rectangle, Size};
use kentos_commands::Entity;
use kentos_ui::snapshot::Input;
use std::sync::{Arc, Mutex};

fn update(app: &mut App, message: Message) {
    let _ = app.update(message);
}

fn drawing() -> App {
    let (mut app, _) = App::boot(None);
    let mut objects = crate::tools_scenes::Objects::new();
    objects.line("cizim", [0., 0.], [2., 0.]);
    objects.line("cizim", [0., 2.], [0., 0.]);
    crate::tools_scenes::open(&mut app, objects);
    crate::files_testing::make_cad(&mut app);
    update(&mut app, Message::DialogClosed);
    app
}

#[test]
fn clicking_and_editing_the_real_cursor_field_never_commits_the_scene_until_enter() {
    let mut app = drawing();
    let mut shot = crate::files_testing::offscreen(Size::new(1440., 900.));
    shot.settle(&mut app, App::view, &mut update);
    update(&mut app, Message::Run("tool.chamfer"));
    crate::tools_scenes::click(&mut app, [1., 0.]);
    crate::tools_scenes::click(&mut app, [0., 1.]);
    let at = Vec2::new(crate::tools_scenes::E + 0.5, crate::tools_scenes::N + 0.5);
    let field = Field::point("1".into(), at);
    let id = field_widget_id(field.token);
    app.field = Some(field);
    shot.settle(&mut app, App::view, &mut update);
    struct Find {
        id: Id,
        found: Arc<Mutex<Option<Rectangle>>>,
    }
    impl Operation for Find {
        fn traverse(&mut self, visit: &mut dyn FnMut(&mut dyn Operation)) {
            visit(self);
        }
        fn text_input(&mut self, id: Option<&Id>, bounds: Rectangle, _: &mut dyn TextInput) {
            if id == Some(&self.id) {
                *self.found.lock().unwrap() = Some(bounds);
            }
        }
    }
    let found = Arc::new(Mutex::new(None));
    shot.operate(
        app.view(),
        Box::new(Find {
            id: id.clone(),
            found: found.clone(),
        }),
    );
    let bounds = found
        .lock()
        .unwrap()
        .expect("an actual editable text widget");
    let click = Point::new(bounds.center_x(), bounds.center_y());
    let before = app.document.as_ref().unwrap().model.to_snapshot_v2();
    shot.input(&mut app, App::view, &mut update, Input::Move(click));
    shot.input(&mut app, App::view, &mut update, Input::Click(click));
    assert_eq!(
        app.field.as_ref().unwrap().at,
        at,
        "the editor stays reachable"
    );
    assert_eq!(
        app.document.as_ref().unwrap().model.to_snapshot_v2(),
        before,
        "the click cannot become a chamfer point"
    );
    shot.operate(app.view(), Box::new(operation::text_input::select_all(id)));
    shot.input(
        &mut app,
        App::view,
        &mut update,
        Input::Type("10cm;20cm".into()),
    );
    assert_eq!(app.field.as_ref().unwrap().text, "10cm;20cm");
    assert_eq!(
        app.field.as_ref().unwrap().preview,
        kentos_interaction::InputPreview::Ready
    );
    let shown = app
        .session
        .preview(&app.format())
        .unwrap()
        .tag
        .unwrap()
        .lines
        .join(" ");
    assert!(
        shown.contains("0.100") && shown.contains("0.200"),
        "{shown}"
    );
    assert_eq!(
        app.document.as_ref().unwrap().model.to_snapshot_v2(),
        before
    );
    shot.input(&mut app, App::view, &mut update, Input::Key(Named::Enter));
    let doc = &mut app.document.as_mut().unwrap().model;
    assert_eq!(doc.len(), 3);
    let Some(Entity::Line(first)) = doc.get(Slot(1)) else {
        panic!()
    };
    assert!((first.a.x - crate::tools_scenes::E - 0.1).abs() < 1e-8);
    let Some(Entity::Line(second)) = doc.get(Slot(2)) else {
        panic!()
    };
    assert!((second.b.y - crate::tools_scenes::N - 0.2).abs() < 1e-8);
    doc.undo().unwrap();
    assert_eq!(doc.to_snapshot_v2(), before);
}

#[test]
fn invalid_live_value_stays_editable_and_one_scene_click_applies_the_corrected_pair() {
    let mut app = drawing();
    let mut shot = crate::files_testing::offscreen(Size::new(1440., 900.));
    shot.settle(&mut app, App::view, &mut update);
    update(&mut app, Message::Run("tool.chamfer"));
    crate::tools_scenes::click(&mut app, [1., 0.]);
    crate::tools_scenes::click(&mut app, [0., 1.]);
    let before = app.document.as_ref().unwrap().model.to_snapshot_v2();
    let field = Field::point(
        String::new(),
        Vec2::new(crate::tools_scenes::E + 0.5, crate::tools_scenes::N + 0.5),
    );
    let token = field.token;
    app.field = Some(field);
    update(
        &mut app,
        Message::CursorValueInput {
            token,
            text: "10m;20m".into(),
            pasted: true,
        },
    );
    assert_eq!(
        app.field.as_ref().unwrap().preview,
        kentos_interaction::InputPreview::Invalid
    );
    update(&mut app, Message::CursorValueSubmit(token));
    assert_eq!(app.field.as_ref().unwrap().token, token);
    assert_eq!(
        app.document.as_ref().unwrap().model.to_snapshot_v2(),
        before
    );
    update(
        &mut app,
        Message::CursorValueInput {
            token,
            text: "10cm;20cm".into(),
            pasted: true,
        },
    );
    assert_eq!(
        app.field.as_ref().unwrap().preview,
        kentos_interaction::InputPreview::Ready
    );
    assert_eq!(
        app.document.as_ref().unwrap().model.to_snapshot_v2(),
        before
    );
    crate::tools_scenes::click(&mut app, [0.5, 0.5]);
    assert!(app.field.is_none());
    let doc = &mut app.document.as_mut().unwrap().model;
    assert_eq!(doc.len(), 3);
    let Some(Entity::Line(a)) = doc.get(Slot(1)) else {
        panic!()
    };
    assert!((a.a.x - crate::tools_scenes::E - 0.1).abs() < 1e-8);
    let Some(Entity::Line(b)) = doc.get(Slot(2)) else {
        panic!()
    };
    assert!((b.b.y - crate::tools_scenes::N - 0.2).abs() < 1e-8);
    doc.undo();
    assert_eq!(doc.to_snapshot_v2(), before);
}

#[test]
fn right_click_menu_and_quick_confirm_apply_the_typed_pair_once() {
    for quick in [false, true] {
        let mut app = drawing();
        let mut shot = crate::files_testing::offscreen(Size::new(1440., 900.));
        shot.settle(&mut app, App::view, &mut update);
        update(&mut app, Message::Run("tool.chamfer"));
        crate::tools_scenes::click(&mut app, [1., 0.]);
        crate::tools_scenes::click(&mut app, [0., 1.]);
        app.memory.chamfer = Some((0.4, 0.4));
        let before = app.document.as_ref().unwrap().model.to_snapshot_v2();
        let field = Field::point(
            String::new(),
            Vec2::new(crate::tools_scenes::E, crate::tools_scenes::N),
        );
        let token = field.token;
        app.field = Some(field);
        update(
            &mut app,
            Message::CursorValueInput {
                token,
                text: "10cm;20cm".into(),
                pasted: true,
            },
        );
        let _ = app.settings.choose(&[(
            crate::drawing_menus::RIGHT_CLICK_ENTER,
            serde_json::Value::Bool(quick),
        )]);
        let _ = app.right_clicked(Point::new(700., 500.));
        if !quick {
            assert_eq!(
                app.document.as_ref().unwrap().model.to_snapshot_v2(),
                before
            );
            assert_eq!(app.field.as_ref().unwrap().text, "10cm;20cm");
            assert!(format!("{:?}", app.drawing_menu_items()).contains("Uygula: 10cm;20cm"));
            update(
                &mut app,
                Message::DrawingMenu(crate::drawing_menus::Event::Closed),
            );
            update(&mut app, Message::Run("tool.confirm"));
        }
        assert!(app.field.is_none());
        let doc = &mut app.document.as_mut().unwrap().model;
        assert_eq!(doc.len(), 3);
        let Some(Entity::Line(a)) = doc.get(Slot(1)) else {
            panic!()
        };
        let Some(Entity::Line(b)) = doc.get(Slot(2)) else {
            panic!()
        };
        assert!((a.a.x - crate::tools_scenes::E - 0.1).abs() < 1e-8);
        assert!((b.b.y - crate::tools_scenes::N - 0.2).abs() < 1e-8);
        doc.undo();
        assert_eq!(doc.to_snapshot_v2(), before);
    }
}

#[test]
fn escape_closes_the_real_menu_without_discarding_its_value_editor() {
    let mut app = drawing();
    let mut shot = crate::files_testing::offscreen(Size::new(1440., 900.));
    shot.settle(&mut app, App::view, &mut update);
    update(&mut app, Message::Run("tool.chamfer"));
    crate::tools_scenes::click(&mut app, [1., 0.]);
    crate::tools_scenes::click(&mut app, [0., 1.]);
    let before = app.document.as_ref().unwrap().model.to_snapshot_v2();
    let field = Field::point(
        "10cm;20cm".into(),
        Vec2::new(crate::tools_scenes::E, crate::tools_scenes::N),
    );
    let token = field.token;
    app.field = Some(field);
    app.refresh_value_preview();
    let _ = app.right_clicked(Point::new(700., 500.));
    assert!(app.drawing_menu.is_some());
    shot.input(&mut app, App::view, &mut update, Input::Key(Named::Escape));
    assert!(app.drawing_menu.is_none());
    // Runtime notifications follow the widget's close message. The
    // subscription in effect for this event belongs to the open menu.
    let event = iced::Event::Keyboard(iced::keyboard::Event::KeyPressed {
        key: iced::keyboard::Key::Named(Named::Escape),
        modified_key: iced::keyboard::Key::Named(Named::Escape),
        physical_key: iced::keyboard::key::Physical::Code(iced::keyboard::key::Code::Escape),
        location: iced::keyboard::Location::Standard,
        modifiers: iced::keyboard::Modifiers::empty(),
        text: None,
        repeat: false,
    });
    assert!(matches!(
        keys::key_event(
            event.clone(),
            iced::event::Status::Captured,
            iced::window::Id::unique()
        ),
        Some(Message::CapturedEscape)
    ));
    assert!(
        keys::drawing_menu_key_event(
            event,
            iced::event::Status::Captured,
            iced::window::Id::unique()
        )
        .is_none()
    );
    assert_eq!(app.field.as_ref().unwrap().token, token);
    assert_eq!(app.field.as_ref().unwrap().text, "10cm;20cm");
    assert_eq!(
        app.document.as_ref().unwrap().model.to_snapshot_v2(),
        before
    );
    update(&mut app, Message::CapturedEscape);
    assert!(
        app.field.is_none(),
        "a later Esc cancels just the value draft"
    );
    assert_eq!(
        app.document.as_ref().unwrap().model.to_snapshot_v2(),
        before
    );
}

#[test]
fn late_input_and_submit_cannot_edit_a_replacement_field_or_write_geometry() {
    let mut app = drawing();
    let old = Field::point("1".into(), Vec2::new(0., 0.));
    let old_token = old.token;
    app.field = Some(old);
    let new = Field::point("2".into(), Vec2::new(1., 1.));
    let new_token = new.token;
    app.field = Some(new);
    let before = app.document.as_ref().unwrap().model.to_snapshot_v2();
    let _ = app.value_field_input(old_token, "99".into(), false);
    let _ = app.value_field_submit(old_token);
    assert_eq!(app.field.as_ref().unwrap().text, "2");
    let _ = app.value_field_input(new_token, "10cm;20cm ".into(), true);
    assert_eq!(
        app.field.as_ref().unwrap().text,
        "10cm;20cm ",
        "paste is not Enter"
    );
    update(&mut app, Message::CapturedEscape);
    assert!(app.field.is_none());
    let _ = app.value_field_input(new_token, "777".into(), false);
    let _ = app.value_field_submit(new_token);
    assert!(app.field.is_none());
    assert_eq!(
        app.document.as_ref().unwrap().model.to_snapshot_v2(),
        before
    );
}
