//! Keys, the web's way (docs/adr/0018; `apps/web/src/core/keymap.ts`,
//! `app/keybindings.ts`, `ui/bottom/CommandLine.ts`): which chord a key press
//! is, which character it types, whether that character starts a value, and
//! whether it is an option letter. [`crate::app::App::key`] decides with these
//! in ADR 0018's order.
//!
//! Iced reports a key press with the key as the layout produced it, its
//! physical position, the modifiers and the text it types. Letters and the
//! symbols `+ - ,` follow what the layout produces, so Turkish Q and F keep
//! their letters (CLAUDE.md §4.6); other keys follow their position; what
//! goes into a field is the text. A symbol typed with AltGr is text, never a
//! chord: Windows reports AltGr as Ctrl+Alt (Ctrl+Alt with a letter or a
//! digit stays a chord, Ctrl+Alt+N).

use iced::keyboard::key::{Named, Physical};
use iced::keyboard::{self, Key, Modifiers};
use iced::{Event, event, window};

use kentos_interaction::upper_tr;

use crate::app::Message;

/// Chords that still work while a text field has the keyboard: the ones the
/// web binds with `allowInInput` (app/keybindings.ts).
pub const GLOBAL: &[&str] = &[
    "Ctrl+Alt+N",
    "Ctrl+O",
    "Ctrl+S",
    "Ctrl+Shift+S",
    "Ctrl+P",
    "F1",
    "F2",
    "F3",
    "F4",
    "F6",
    "F7",
    "F8",
    "F9",
    "F10",
    "Shift+F3",
    "Ctrl+F1",
    "Alt+Q",
    "Ctrl+,",
];

/// A key press no widget took: the window's `KeyPressed`, for the app to route.
#[derive(Debug, Clone, PartialEq)]
pub struct KeyPress {
    /// The key as the layout produced it, modifiers applied (the web's `KeyboardEvent.key`).
    pub key: Key,
    /// Where it is on the keyboard (the web's `KeyboardEvent.code`).
    pub physical: Physical,
    pub modifiers: Modifiers,
    /// What it types, AltGr applied; a control character for a Ctrl chord.
    pub text: Option<String>,
    pub repeat: bool,
}

/// The subscription: every key press a widget did not capture goes to the
/// app's router. A focused text field captures what it types, so typing
/// there never reaches the drawing (ADR 0018, step 1). The modifier keys'
/// state goes to the app whatever has the focus (Shift turns ortho over).
pub fn key_event(event: Event, status: event::Status, _window: window::Id) -> Option<Message> {
    if let Event::Keyboard(keyboard::Event::ModifiersChanged(modifiers)) = event {
        return Some(Message::Modifiers(modifiers));
    }
    let Event::Keyboard(keyboard::Event::KeyPressed {
        modified_key,
        physical_key,
        modifiers,
        text,
        repeat,
        ..
    }) = event
    else {
        return None;
    };
    if status == event::Status::Captured {
        return None;
    }
    Some(Message::Key(KeyPress {
        key: modified_key,
        physical: physical_key,
        modifiers,
        text: text.map(|t| t.to_string()),
        repeat,
    }))
}

impl KeyPress {
    pub fn named(&self) -> Option<Named> {
        match self.key {
            Key::Named(named) => Some(named),
            _ => None,
        }
    }

    /// The one character the key types; none for a control character (Enter, Ctrl+Z).
    pub fn character(&self) -> Option<char> {
        let mut chars = self.text.as_deref()?.chars();
        let c = chars.next()?;
        (chars.next().is_none() && !c.is_control()).then_some(c)
    }

    /// A symbol typed with AltGr, which Windows reports as Ctrl+Alt: text, not a chord.
    pub fn is_altgr_text(&self) -> bool {
        self.modifiers.control()
            && self.modifiers.alt()
            && self.character().is_some_and(|c| !c.is_alphanumeric())
    }

    /// Ctrl, Alt or the logo key held, and not for AltGr text.
    fn chorded(&self) -> bool {
        (self.modifiers.control() || self.modifiers.alt() || self.modifiers.logo())
            && !self.is_altgr_text()
    }
}

/// The chord a key press is, as the web writes shortcuts (`Ctrl+Shift+Z`,
/// `G`, `+`, `Home`, `F3`); none for a modifier alone and for AltGr text.
pub fn chord(press: &KeyPress) -> Option<String> {
    if matches!(
        press.key,
        Key::Named(
            Named::Control
                | Named::Shift
                | Named::Alt
                | Named::AltGraph
                | Named::Super
                | Named::Meta
        )
    ) || press.is_altgr_text()
    {
        return None;
    }
    let key = key_name(press)?;
    let m = press.modifiers;
    let mut chord = String::new();
    if m.control() || m.logo() {
        chord.push_str("Ctrl+");
    }
    if m.alt() {
        chord.push_str("Alt+");
    }
    // + and − are typed with Shift on some layouts (Turkish Q: Shift+4).
    if m.shift() && key != "+" && key != "-" {
        chord.push_str("Shift+");
    }
    chord.push_str(&key);
    Some(chord)
}

/// The web's `keyFromEvent`: a letter by the character it types (ı counts
/// as I), `+ - ,` as typed, anything else by its position.
fn key_name(press: &KeyPress) -> Option<String> {
    if let Key::Character(c) = &press.key {
        let mut chars = c.chars();
        if let (Some(ch), None) = (chars.next(), chars.next()) {
            let upper: String = if ch == 'ı' {
                "I".to_owned()
            } else {
                ch.to_uppercase().collect()
            };
            if upper.len() == 1 && upper.as_bytes()[0].is_ascii_uppercase() {
                return Some(upper);
            }
            if matches!(ch, '+' | '-' | ',') {
                return Some(ch.to_string());
            }
        }
    }
    let code = match press.physical {
        Physical::Code(code) => format!("{code:?}"),
        Physical::Unidentified(_) => return named_key(press.named()?),
    };
    if let Some(letter) = code.strip_prefix("Key") {
        return Some(letter.to_owned());
    }
    if let Some(digit) = code.strip_prefix("Digit") {
        return Some(digit.to_owned());
    }
    if code.len() <= 3 && code.starts_with('F') && code[1..].chars().all(|c| c.is_ascii_digit()) {
        return Some(code);
    }
    let named = match code.as_str() {
        "Escape" => "Esc",
        "Enter" | "NumpadEnter" => "Enter",
        "Space" => "Space",
        "Delete" => "Delete",
        "Backspace" => "Backspace",
        "Tab" => "Tab",
        "Home" => "Home",
        "End" => "End",
        "PageUp" => "PageUp",
        "PageDown" => "PageDown",
        "ArrowUp" => "Up",
        "ArrowDown" => "Down",
        "ArrowLeft" => "Left",
        "ArrowRight" => "Right",
        "NumpadAdd" => "+",
        "NumpadSubtract" => "-",
        _ => return None,
    };
    Some(named.to_owned())
}

/// A key the platform could not place, by what it means.
fn named_key(named: Named) -> Option<String> {
    let name = match named {
        Named::Escape => "Esc",
        Named::Enter => "Enter",
        Named::Space => "Space",
        Named::Delete => "Delete",
        Named::Backspace => "Backspace",
        Named::Tab => "Tab",
        Named::Home => "Home",
        Named::End => "End",
        Named::PageUp => "PageUp",
        Named::PageDown => "PageDown",
        Named::ArrowUp => "Up",
        Named::ArrowDown => "Down",
        Named::ArrowLeft => "Left",
        Named::ArrowRight => "Right",
        Named::F1 => "F1",
        Named::F2 => "F2",
        Named::F3 => "F3",
        Named::F4 => "F4",
        Named::F5 => "F5",
        Named::F6 => "F6",
        Named::F7 => "F7",
        Named::F8 => "F8",
        Named::F9 => "F9",
        Named::F10 => "F10",
        Named::F11 => "F11",
        Named::F12 => "F12",
        _ => return None,
    };
    Some(name.to_owned())
}

/// Whether a chord has Ctrl, Alt or is a function key: those reach every
/// command; single keys only the commands the desktop runs (ADR 0017).
pub fn is_chorded(chord: &str) -> bool {
    chord.starts_with("Ctrl+")
        || chord.starts_with("Alt+")
        || chord.rsplit('+').next().is_some_and(|key| {
            key.len() > 1 && key.starts_with('F') && key[1..].chars().all(|c| c.is_ascii_digit())
        })
}

/// The character that starts a typed value (the web's `keymap.fallback`):
/// a digit, `.`, `@`, `+` or `-`. `+` and `-` get here only when no
/// shortcut took them, that is while a command runs (ADR 0018).
pub fn value_start(press: &KeyPress) -> Option<char> {
    if press.chorded() {
        return None;
    }
    press
        .character()
        .filter(|c| c.is_ascii_digit() || matches!(c, '@' | '.' | '+' | '-'))
}

/// An option letter of the running command: a plain letter, upper-cased the
/// Turkish way (the web's `CommandLine.intercept`). Shift makes it a chord.
pub fn option_letter(press: &KeyPress) -> Option<String> {
    let m = press.modifiers;
    if m.control() || m.alt() || m.logo() || m.shift() {
        return None;
    }
    let c = press.character()?;
    c.is_alphabetic().then(|| upper_tr(&c.to_string()))
}

/// What the key types into a text field, when it types something and is not a chord.
pub fn typed(press: &KeyPress) -> Option<&str> {
    if press.chorded() {
        return None;
    }
    press
        .text
        .as_deref()
        .filter(|t| !t.is_empty() && !t.chars().any(char::is_control))
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::keyboard::key::Code;

    fn press(key: Key, code: Code, modifiers: Modifiers, text: Option<&str>) -> KeyPress {
        KeyPress {
            key,
            physical: Physical::Code(code),
            modifiers,
            text: text.map(str::to_owned),
            repeat: false,
        }
    }

    fn ch(c: &str) -> Key {
        Key::Character(c.into())
    }

    #[test]
    fn chords_are_written_as_the_web_writes_them() {
        let ctrl_z = press(ch("z"), Code::KeyZ, Modifiers::CTRL, Some("\u{1a}"));
        assert_eq!(chord(&ctrl_z).as_deref(), Some("Ctrl+Z"));
        assert_eq!(value_start(&ctrl_z), None);
        let shift_z = press(
            ch("Z"),
            Code::KeyZ,
            Modifiers::CTRL | Modifiers::SHIFT,
            None,
        );
        assert_eq!(chord(&shift_z).as_deref(), Some("Ctrl+Shift+Z"));
        let g = press(ch("g"), Code::KeyG, Modifiers::empty(), Some("g"));
        assert_eq!(chord(&g).as_deref(), Some("G"));
        assert_eq!(option_letter(&g).as_deref(), Some("G"));
        let home = press(
            Key::Named(Named::Home),
            Code::Home,
            Modifiers::empty(),
            None,
        );
        assert_eq!(chord(&home).as_deref(), Some("Home"));
        let space = press(
            Key::Named(Named::Space),
            Code::Space,
            Modifiers::empty(),
            Some(" "),
        );
        assert_eq!(chord(&space).as_deref(), Some("Space"));
        assert_eq!(option_letter(&space), None);
    }

    #[test]
    fn turkish_q_keeps_its_letters_and_symbols() {
        // ı is on the I key and counts as I; ş sits where US has ; and is no chord.
        let dotless = press(ch("ı"), Code::KeyI, Modifiers::empty(), Some("ı"));
        assert_eq!(chord(&dotless).as_deref(), Some("I"));
        assert_eq!(option_letter(&dotless).as_deref(), Some("I"));
        let i = press(ch("i"), Code::Quote, Modifiers::empty(), Some("i"));
        assert_eq!(option_letter(&i).as_deref(), Some("İ"));
        let s = press(ch("ş"), Code::Semicolon, Modifiers::empty(), Some("ş"));
        assert_eq!(chord(&s), None);
        // + is Shift+4: a + chord (zoom when idle) and a value start, not Shift+4.
        let plus = press(ch("+"), Code::Digit4, Modifiers::SHIFT, Some("+"));
        assert_eq!(chord(&plus).as_deref(), Some("+"));
        assert_eq!(value_start(&plus), Some('+'));
        // − sits right of *.
        let minus = press(ch("-"), Code::Equal, Modifiers::empty(), Some("-"));
        assert_eq!(chord(&minus).as_deref(), Some("-"));
        // @ is AltGr+Q, reported as Ctrl+Alt: text that starts a value, no chord.
        let at = press(
            ch("@"),
            Code::KeyQ,
            Modifiers::CTRL | Modifiers::ALT,
            Some("@"),
        );
        assert!(at.is_altgr_text());
        assert_eq!(chord(&at), None);
        assert_eq!(value_start(&at), Some('@'));
        assert_eq!(typed(&at), Some("@"));
        // Ctrl+Alt with a letter stays a chord.
        let new = press(
            ch("n"),
            Code::KeyN,
            Modifiers::CTRL | Modifiers::ALT,
            Some("\u{e}"),
        );
        assert_eq!(chord(&new).as_deref(), Some("Ctrl+Alt+N"));
    }

    #[test]
    fn values_start_with_digits_and_signs_only() {
        let us_at = press(ch("@"), Code::Digit2, Modifiers::SHIFT, Some("@"));
        assert_eq!(chord(&us_at).as_deref(), Some("Shift+2"));
        assert_eq!(value_start(&us_at), Some('@'));
        let dot = press(ch("."), Code::Period, Modifiers::empty(), Some("."));
        assert_eq!(chord(&dot), None);
        assert_eq!(value_start(&dot), Some('.'));
        let comma = press(ch(","), Code::Comma, Modifiers::empty(), Some(","));
        assert_eq!(value_start(&comma), None);
        assert_eq!(typed(&comma), Some(","));
        let enter = press(
            Key::Named(Named::Enter),
            Code::Enter,
            Modifiers::empty(),
            Some("\r"),
        );
        assert_eq!((value_start(&enter), typed(&enter)), (None, None));
        assert!(is_chorded("Ctrl+S") && is_chorded("F3") && is_chorded("Shift+F3"));
        assert!(!is_chorded("G") && !is_chorded("Space") && !is_chorded("+"));
    }
}
