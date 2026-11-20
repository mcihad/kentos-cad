//! A step asking for words over the drawing (UR2-047, UR2-048): the letters typed are its text, not shortcuts or
//! option letters; its option's button is chosen as the option.

use super::*;
use iced::keyboard::key::{NativeCode, Physical};
use iced::keyboard::{Key, Modifiers};

fn letter(app: &mut App, c: &str) {
    let _ = app.update(Message::Key(crate::keys::KeyPress {
        key: Key::Character(c.into()),
        physical: Physical::Unidentified(NativeCode::Unidentified),
        modifiers: Modifiers::default(),
        text: Some(c.to_owned()),
        repeat: false,
    }));
}

fn named_ucs(app: &App) -> Vec<String> {
    let doc = app.document.as_ref().expect("a drawing");
    doc.model.settings().named_ucs.iter().map(|n| n.name.clone()).collect()
}

#[test]
fn a_name_typed_over_the_drawing_is_the_name_from_its_first_letter() {
    let mut app = crate::files_testing::app_with_drawing();
    let _ = app.update(Message::Run("tool.ucsNamed"));
    assert_eq!(app.session.tool_id(), "ucsNamed");
    assert!(app.session.prompt().step.contains("kaydedileceği adı"), "no system kept: the name to save it as");
    // P and A are Çoklu çizgi's and Yay's shortcuts; Ş is Turkish.
    for c in ["P", "a", "r", "Ş", "e", "l"] {
        letter(&mut app, c);
    }
    assert_eq!(app.command_input, "ParŞel");
    assert_eq!(app.session.tool_id(), "ucsNamed", "the command still runs");
    if std::env::var_os("KENTOS_SHOTS").is_some() {
        let mut shot = crate::files_testing::offscreen(iced::Size::new(1440., 900.));
        shot.settle(&mut app, App::view, &mut |app: &mut App, m| drop(app.update(m)));
        crate::files_testing::evidence(&mut shot, &app, "ur2-047", "adli-ucs-ad");
    }
    let _ = app.update(Message::CommandSubmitted);
    assert_eq!(named_ucs(&app), ["ParŞel"]);
}

#[test]
fn the_save_button_asks_for_the_name_and_a_one_letter_name_is_kept() {
    let mut app = crate::files_testing::app_with_drawing();
    let _ = app.update(Message::Run("tool.ucsNamed"));
    let _ = app.update(Message::CommandInput("parsel".into()));
    let _ = app.update(Message::CommandSubmitted);
    // Again, with a system kept: the step asks which to go to; Kaydet is its button.
    let _ = app.update(Message::Run("tool.ucsNamed"));
    assert!(app.session.prompt().step.contains("geçilecek"), "{}", app.session.prompt().step);
    let _ = app.prompt_option("K");
    assert!(app.session.prompt().step.contains("kaydedileceği adı"), "Kaydet asks for the name");
    // A one-letter name typed is a name.
    letter(&mut app, "K");
    let _ = app.update(Message::CommandSubmitted);
    assert_eq!(named_ucs(&app), ["parsel", "K"]);
    // Its name in full chooses the option too, while no system has that name; K typed goes to the system K.
    let _ = app.update(Message::Run("tool.ucsNamed"));
    let _ = app.update(Message::CommandInput("kaydet".into()));
    let _ = app.update(Message::CommandSubmitted);
    assert!(app.session.prompt().step.contains("kaydedileceği adı"));
    let _ = app.update(Message::Run("tool.cancel"));
}
