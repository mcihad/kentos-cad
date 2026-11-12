use super::*;
use kentos_commands::WallJustify;

/// kentos-cad's Duvar (no style: its thickness is the user's), its first point clicked.
fn wall_started() -> App {
    let (mut app, _) = App::boot(None);
    let mut objects = crate::tools_scenes::Objects::new();
    objects.line("cizim", [0., 0.], [2., 0.]);
    crate::tools_scenes::open(&mut app, objects);
    crate::files_testing::make_cad(&mut app);
    let _ = app.update(Message::DialogClosed);
    let _ = app.update(Message::Run("tool.wall"));
    crate::tools_scenes::click(&mut app, [0., 5.]);
    app
}

fn at() -> Vec2 {
    Vec2::new(crate::tools_scenes::E + 3.0, crate::tools_scenes::N + 5.0)
}

fn setting(app: &App) -> Option<&'static str> {
    app.field
        .as_ref()
        .and_then(|f| f.setting.as_ref())
        .map(|s| s.key)
}

#[test]
fn tab_takes_the_value_field_from_the_locks_to_duvar_s_thickness_and_justification() {
    let mut app = wall_started();
    let keys: Vec<&str> = app.field_settings().iter().map(|s| s.key).collect();
    assert_eq!(keys, ["E", "H"], "Kalınlık and Hiza");
    app.field = Some(Field::point("3".into(), at()));
    app.tab_field();
    assert_eq!(
        app.field.as_ref().unwrap().mode,
        Some(LockAsk::Angle),
        "3 locked the length"
    );
    app.tab_field();
    assert_eq!(setting(&app), Some("E"), "no angle typed: Kalınlık");
    app.field.as_mut().unwrap().text = "0.3".into();
    app.tab_field();
    assert!(
        (app.memory.wall_width - 0.3).abs() < 1e-9,
        "written as the pane writes it"
    );
    assert_eq!(setting(&app), Some("H"));
    app.tab_field();
    let field = app.field.as_ref().unwrap();
    assert_eq!(
        (field.setting.as_ref(), field.mode),
        (None, Some(LockAsk::Length)),
        "after the last, the length"
    );
    app.tab_field();
    app.tab_field();
    assert_eq!(
        setting(&app),
        Some("E"),
        "round again: Açı left empty, Kalınlık"
    );
    app.tab_field();
    app.field.as_mut().unwrap().text = "sa".into();
    let _ = app.submit_field();
    assert_eq!(app.memory.wall_justify, WallJustify::Right, "sa: Sağ");
    assert!(app.field.is_none(), "Enter writes and the field goes");
    assert!(
        app.session.is_running(),
        "the wall goes on from its first point"
    );
}

#[test]
fn the_down_arrow_over_the_drawing_opens_the_field_on_the_tool_s_first_setting() {
    let mut app = wall_started();
    crate::tools_scenes::hover(&mut app, [3.3, 7.4]);
    assert!(app.field.is_none());
    let down = crate::keys::KeyPress {
        key: iced::keyboard::Key::Named(Named::ArrowDown),
        physical: iced::keyboard::key::Physical::Code(iced::keyboard::key::Code::ArrowDown),
        modifiers: iced::keyboard::Modifiers::empty(),
        text: None,
        repeat: false,
    };
    let _ = app.key(down);
    assert_eq!(
        setting(&app),
        Some("E"),
        "the box beside the cursor, on Kalınlık"
    );
}

#[test]
fn a_choice_is_taken_by_the_start_of_its_word_or_the_arrows() {
    let mut app = wall_started();
    app.field = Some(Field::point(String::new(), at()));
    app.tab_field();
    app.tab_field();
    assert_eq!(
        setting(&app),
        Some("H"),
        "nothing typed: Kalınlık kept, Hiza"
    );
    app.field.as_mut().unwrap().text = "s".into();
    app.tab_field();
    assert_eq!(
        setting(&app),
        Some("H"),
        "s is Sol and Sağ: it stays and says so"
    );
    assert!(crate::files_testing::last_said(&app).contains("bir harf daha"));
    app.field.as_mut().unwrap().text = "so".into();
    app.tab_field();
    assert_eq!(app.memory.wall_justify, WallJustify::Left, "so: Sol");
    app.field = Some(Field::point(String::new(), at()));
    app.tab_field();
    app.tab_field();
    let before = app.memory.wall_justify;
    app.cycle_setting(true);
    assert_ne!(app.memory.wall_justify, before, "↓ writes the next at once");
    assert_eq!(
        app.field
            .as_ref()
            .and_then(|f| f.setting.as_ref())
            .map(|s| s.value.as_str()),
        Some(app.memory.wall_justify.label())
    );
}
