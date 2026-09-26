//! The web runner's variants and the keyboards that type a trace's
//! characters (`LAYOUTS` and `keyFor` in apps/web/scripts/e2e/interaction.mjs):
//! a US or a Turkish Q keyboard as Iced reports its key presses, AltGr as
//! Ctrl+Alt, as Windows reports it.

use iced::keyboard::key::{Code, Named, Physical};
use iced::keyboard::{self, Key, Location, Modifiers};

/// Which keyboard types the characters.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Layout {
    Us,
    TurkishQ,
}

/// The web runner's variants: a keyboard and the screen's device pixels per logical pixel.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Variant {
    pub id: &'static str,
    pub layout: Layout,
    pub dpr: f32,
}

pub const VARIANTS: [Variant; 3] = [
    Variant {
        id: "us",
        layout: Layout::Us,
        dpr: 1.0,
    },
    Variant {
        id: "tr-q",
        layout: Layout::TurkishQ,
        dpr: 1.0,
    },
    Variant {
        id: "hidpi",
        layout: Layout::Us,
        dpr: 2.0,
    },
];

impl Variant {
    pub fn by_id(id: &str) -> Option<Self> {
        VARIANTS.into_iter().find(|v| v.id == id)
    }
}

/// One key as a keyboard reports it: the key without and with modifiers, its
/// position, the modifiers and the text it types.
pub(super) struct Stroke {
    key: Key,
    modified: Key,
    code: Code,
    location: Location,
    pub(super) modifiers: Modifiers,
    text: Option<String>,
}

fn character(c: &str) -> Key {
    Key::Character(c.into())
}

/// The key that types `ch` on `layout` (the web runner's `LAYOUTS` and `keyFor`).
pub(super) fn stroke_for(ch: &str, layout: Layout) -> Result<Stroke, String> {
    let named = |named: Named, code: Code, text: Option<&str>| Stroke {
        key: Key::Named(named),
        modified: Key::Named(named),
        code,
        location: Location::Standard,
        modifiers: Modifiers::empty(),
        text: text.map(str::to_owned),
    };
    // Named keys, with the control characters a platform reports as their text.
    match ch {
        "Enter" => return Ok(named(Named::Enter, Code::Enter, Some("\r"))),
        "Esc" => return Ok(named(Named::Escape, Code::Escape, Some("\u{1b}"))),
        "Tab" => return Ok(named(Named::Tab, Code::Tab, Some("\t"))),
        "Backspace" => return Ok(named(Named::Backspace, Code::Backspace, Some("\u{8}"))),
        "Space" | " " => return Ok(named(Named::Space, Code::Space, Some(" "))),
        "Delete" => return Ok(named(Named::Delete, Code::Delete, Some("\u{7f}"))),
        "F3" => return Ok(named(Named::F3, Code::F3, None)),
        "F8" => return Ok(named(Named::F8, Code::F8, None)),
        _ => {}
    }
    let plain = |base: &str, code: Code| Stroke {
        key: character(base),
        modified: character(ch),
        code,
        location: Location::Standard,
        modifiers: Modifiers::empty(),
        text: Some(ch.to_owned()),
    };
    let shifted = |base: &str, code: Code| Stroke {
        modifiers: Modifiers::SHIFT,
        ..plain(base, code)
    };
    let digit = |d: char| -> Option<Code> {
        Some(match d {
            '0' => Code::Digit0,
            '1' => Code::Digit1,
            '2' => Code::Digit2,
            '3' => Code::Digit3,
            '4' => Code::Digit4,
            '5' => Code::Digit5,
            '6' => Code::Digit6,
            '7' => Code::Digit7,
            '8' => Code::Digit8,
            '9' => Code::Digit9,
            _ => return None,
        })
    };
    let mut chars = ch.chars();
    let (Some(c), None) = (chars.next(), chars.next()) else {
        return Err(format!("“{ch}” için tuş yok"));
    };
    if let Some(code) = digit(c) {
        return Ok(plain(ch, code));
    }
    let symbol = match (layout, c) {
        (Layout::Us, '.') => Some(plain(".", Code::Period)),
        (Layout::Us, ',') => Some(plain(",", Code::Comma)),
        (Layout::Us, ';') => Some(plain(";", Code::Semicolon)),
        (Layout::Us, '-') => Some(plain("-", Code::Minus)),
        (Layout::Us, '+') => Some(Stroke {
            location: Location::Numpad,
            ..plain("+", Code::NumpadAdd)
        }),
        (Layout::Us, '@') => Some(shifted("2", Code::Digit2)),
        (Layout::Us, '<') => Some(plain("<", Code::IntlBackslash)),
        // Turkish Q: + is Shift+4, − sits right of *, @ is AltGr+Q (Ctrl+Alt on Windows).
        (Layout::TurkishQ, '.') => Some(plain(".", Code::Slash)),
        (Layout::TurkishQ, ',') => Some(plain(",", Code::Backslash)),
        (Layout::TurkishQ, ';') => Some(shifted(",", Code::Backslash)),
        (Layout::TurkishQ, '-') => Some(plain("-", Code::Equal)),
        (Layout::TurkishQ, '+') => Some(shifted("4", Code::Digit4)),
        (Layout::TurkishQ, '@') => Some(Stroke {
            modifiers: Modifiers::CTRL | Modifiers::ALT,
            ..plain("q", Code::KeyQ)
        }),
        (Layout::TurkishQ, '<') => Some(plain("<", Code::IntlBackslash)),
        _ => None,
    };
    if let Some(stroke) = symbol {
        return Ok(stroke);
    }
    if c.is_alphabetic() {
        // An option letter is typed without Shift; the app compares upper case.
        let lower = match c {
            'I' => 'ı',
            'İ' => 'i',
            c => c.to_lowercase().next().unwrap_or(c),
        };
        let code = letter_code(lower, layout).ok_or(format!("“{ch}” için tuş yok"))?;
        let text = lower.to_string();
        return Ok(Stroke {
            key: character(&text),
            modified: character(&text),
            code,
            location: Location::Standard,
            modifiers: Modifiers::empty(),
            text: Some(text),
        });
    }
    Err(format!("“{ch}” için tuş yok"))
}

/// Where a letter is: its Latin key, or a Turkish Q letter's own key.
fn letter_code(lower: char, layout: Layout) -> Option<Code> {
    if layout == Layout::TurkishQ {
        let turkish = match lower {
            'ı' => Some(Code::KeyI),
            'i' => Some(Code::Quote),
            'ş' => Some(Code::Semicolon),
            'ğ' => Some(Code::BracketLeft),
            'ü' => Some(Code::BracketRight),
            'ö' => Some(Code::Comma),
            'ç' => Some(Code::Period),
            _ => None,
        };
        if turkish.is_some() {
            return turkish;
        }
    }
    let ascii = match lower {
        'ı' => 'i',
        'ç' => 'c',
        'ğ' => 'g',
        'ö' => 'o',
        'ş' => 's',
        'ü' => 'u',
        c => c,
    };
    Some(match ascii.to_ascii_uppercase() {
        'A' => Code::KeyA,
        'B' => Code::KeyB,
        'C' => Code::KeyC,
        'D' => Code::KeyD,
        'E' => Code::KeyE,
        'F' => Code::KeyF,
        'G' => Code::KeyG,
        'H' => Code::KeyH,
        'I' => Code::KeyI,
        'J' => Code::KeyJ,
        'K' => Code::KeyK,
        'L' => Code::KeyL,
        'M' => Code::KeyM,
        'N' => Code::KeyN,
        'O' => Code::KeyO,
        'P' => Code::KeyP,
        'Q' => Code::KeyQ,
        'R' => Code::KeyR,
        'S' => Code::KeyS,
        'T' => Code::KeyT,
        'U' => Code::KeyU,
        'V' => Code::KeyV,
        'W' => Code::KeyW,
        'X' => Code::KeyX,
        'Y' => Code::KeyY,
        'Z' => Code::KeyZ,
        _ => return None,
    })
}

/// A trace's key (`Enter`, `G`, `-`, `+`, `Ctrl+Z`) as a stroke: a chord
/// types nothing but the control character a platform reports for it.
pub(super) fn chord_stroke(chord: &str, layout: Layout) -> Result<Stroke, String> {
    let (mods, name) = match chord {
        "+" => (Vec::new(), "+"),
        _ => {
            let mut parts: Vec<&str> = chord.split('+').collect();
            let name = parts.pop().unwrap_or(chord);
            (parts, name)
        }
    };
    let mut stroke = stroke_for(name, layout)?;
    for m in mods {
        stroke.modifiers |= match m {
            "Ctrl" => Modifiers::CTRL,
            "Alt" => Modifiers::ALT,
            "Shift" => Modifiers::SHIFT,
            other => return Err(format!("bilinmeyen değiştirici tuş {other}")),
        };
    }
    if stroke.modifiers.shift()
        && let Key::Character(c) = &stroke.modified
        && c.chars().all(char::is_alphabetic)
    {
        // Shift+letter (Shift+H, Ctrl+Shift+V): the keyboard gives the capital, the Turkish way.
        let capital = match layout {
            Layout::TurkishQ => kentos_interaction::upper_tr(c),
            Layout::Us => c.to_uppercase(),
        };
        stroke.text = Some(capital.clone());
        stroke.modified = character(&capital);
    }
    if stroke.modifiers.control() && !matches!(stroke.key, Key::Named(_)) {
        // Ctrl+letter: the ASCII control character (Ctrl+Z is U+001A).
        stroke.text = stroke
            .text
            .as_deref()
            .and_then(|t| t.chars().next())
            .filter(char::is_ascii_alphabetic)
            .map(|c| char::from((c.to_ascii_lowercase() as u8) - b'a' + 1).to_string());
    }
    Ok(stroke)
}

impl Stroke {
    pub(super) fn event(&self) -> iced::Event {
        iced::Event::Keyboard(keyboard::Event::KeyPressed {
            key: self.key.clone(),
            modified_key: self.modified.clone(),
            physical_key: Physical::Code(self.code),
            location: self.location,
            modifiers: self.modifiers,
            text: self.text.as_deref().map(Into::into),
            repeat: false,
        })
    }
}
