//! The user's keys and typed names (docs/adr/9308): the two settings that hold them, how a key chord and a typed
//! name are written, and the keys KentOS keeps for itself.
//!
//! A chord is written as the input routing writes a key press (`Ctrl+Shift+L`, `F5`, `Alt+Q`): the modifiers in the
//! order Ctrl, Alt, Shift, then one key. Ctrl is the platform's command key (⌘ on macOS). Shift is not written with
//! `+` and `-`, which some keyboards type with it. A typed name is letters and digits, a letter first, kept in
//! Turkish upper case (`çz` → `ÇZ`).
//!
//! The settings keep what they are given as long as its shape is right (a map of texts); what an entry means (its
//! command, its chord) is read where it is used, against that host's commands. So an entry a newer version wrote,
//! or one of a command this version no longer has, is kept in the file and reported, never lost: the settings' rule
//! for keys a newer version wrote (docs/adr/0023).

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// `keyboard.shortcuts`: a command's id → its keys, in their order. A command not listed keeps its own keys; an empty
/// list takes them away. A key listed for one command is no other command's.
pub const SHORTCUTS_SETTING: &str = "keyboard.shortcuts";

/// `keyboard.aliases`: a typed name → the command it starts.
pub const ALIASES_SETTING: &str = "keyboard.aliases";

/// The most characters of a typed name.
pub const ALIAS_MAX: usize = 32;

/// The keys a chord may end in that are words, as the input routing names them.
const NAMED_KEYS: [&str; 14] = [
    "Esc",
    "Enter",
    "Space",
    "Delete",
    "Backspace",
    "Tab",
    "Home",
    "End",
    "PageUp",
    "PageDown",
    "Up",
    "Down",
    "Left",
    "Right",
];

/// Other spellings read as a named key.
const KEY_SPELLINGS: [(&str, &str); 6] = [
    ("escape", "Esc"),
    ("return", "Enter"),
    ("del", "Delete"),
    ("boşluk", "Space"),
    ("pgup", "PageUp"),
    ("pgdn", "PageDown"),
];

/// Why a chord could not be read.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChordError {
    /// Nothing, or modifiers without a key (`Ctrl+`, `Shift`).
    NoKey,
    /// A key this version does not name (`Ctrl+Ş`, `F25`, `NumLock`).
    UnknownKey,
    /// A modifier twice, an empty one or one this version does not know (`Ctrl+Ctrl+L`, `Hyper+L`).
    BadModifier,
}

impl ChordError {
    /// What is wrong and how to write it, in Turkish.
    pub fn message(self) -> &'static str {
        match self {
            Self::NoKey => {
                "Kısayolun tuşu yok; bir harf, rakam ya da F1 gibi bir tuş ekleyin (ör. Ctrl+Shift+L)."
            }
            Self::UnknownKey => {
                "Bu tuş kısayol olamaz; A–Z, 0–9, F1–F24, Delete, Home, End, Page Up, Page Down, ok tuşları, + − ve virgülden birini seçin."
            }
            Self::BadModifier => {
                "Değiştirici tuşlar Ctrl, Alt ve Shift'tir, her biri bir kez yazılır (ör. Ctrl+Alt+K)."
            }
        }
    }
}

/// A chord as the input routing writes it, or why it cannot be one. Upper or lower case, the modifiers in any order
/// and a few other spellings of the keys (`escape`, `del`) are read; what comes back is the one way (`Ctrl+Shift+L`).
pub fn normalize_chord(text: &str) -> Result<String, ChordError> {
    let text = text.trim();
    if text.is_empty() {
        return Err(ChordError::NoKey);
    }
    // The + key: alone, or after the modifiers (`Ctrl++`).
    let (modifiers, key) = if text == "+" {
        ("", "+")
    } else if let Some(rest) = text.strip_suffix("++") {
        (rest, "+")
    } else {
        match text.rsplit_once('+') {
            Some((rest, key)) => (rest, key),
            None => ("", text),
        }
    };
    let key = key_name(key.trim())?;
    let (mut ctrl, mut alt, mut shift) = (false, false, false);
    if !modifiers.is_empty() {
        for part in modifiers.split('+') {
            let flag = match part.trim().to_lowercase().as_str() {
                "ctrl" | "control" => &mut ctrl,
                "alt" => &mut alt,
                "shift" => &mut shift,
                "" => return Err(ChordError::NoKey),
                _ => return Err(ChordError::BadModifier),
            };
            if *flag {
                return Err(ChordError::BadModifier);
            }
            *flag = true;
        }
    }
    let mut chord = String::new();
    if ctrl {
        chord.push_str("Ctrl+");
    }
    if alt {
        chord.push_str("Alt+");
    }
    // + and − are typed with Shift on some keyboards: the input routing never writes Shift with them.
    if shift && key != "+" && key != "-" {
        chord.push_str("Shift+");
    }
    chord.push_str(&key);
    Ok(chord)
}

/// A key's name as the input routing writes it.
fn key_name(key: &str) -> Result<String, ChordError> {
    if key.is_empty() {
        return Err(ChordError::NoKey);
    }
    let mut chars = key.chars();
    if let (Some(c), None) = (chars.next(), chars.next()) {
        if c.is_ascii_alphabetic() {
            return Ok(c.to_ascii_uppercase().to_string());
        }
        if c.is_ascii_digit() || matches!(c, '+' | '-' | ',') {
            return Ok(c.to_string());
        }
        return Err(ChordError::UnknownKey);
    }
    let lower = key.to_lowercase();
    if matches!(lower.as_str(), "ctrl" | "control" | "alt" | "shift") {
        return Err(ChordError::NoKey);
    }
    // F1 … F24, written without a leading zero.
    if let Some(digits) = lower.strip_prefix('f')
        && !digits.starts_with('0')
        && digits.chars().all(|c| c.is_ascii_digit())
        && let Ok(n) = digits.parse::<u8>()
        && (1..=24).contains(&n)
    {
        return Ok(format!("F{n}"));
    }
    if let Some(name) = NAMED_KEYS.iter().find(|k| k.to_lowercase() == lower) {
        return Ok((*name).to_owned());
    }
    KEY_SPELLINGS
        .iter()
        .find(|(spelling, _)| *spelling == lower)
        .map(|(_, name)| (*name).to_owned())
        .ok_or(ChordError::UnknownKey)
}

/// A key KentOS keeps for itself in every host: no command takes it from the user (the input routing's own keys,
/// docs/adr/0018).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReservedKey {
    /// Esc, with any modifiers.
    Escape,
    /// Enter, with any modifiers.
    Enter,
    /// Space alone.
    Space,
    /// Tab, with any modifiers.
    Tab,
    /// Backspace alone.
    Backspace,
    /// An arrow key alone.
    Arrow,
    /// A digit (with Shift too), `+`, `-` or `,` alone: what starts a typed value.
    Value,
}

impl ReservedKey {
    /// Why the key is KentOS's, in Turkish.
    pub fn message(self) -> &'static str {
        match self {
            Self::Escape => {
                "Esc çalışan komutu ve seçimi bırakır, pencereyi kapatır; her zaman böyle kalır."
            }
            Self::Enter => {
                "Enter komutu onaylar ya da bitirir, satırda ve alanlarda yazılanı alır; her zaman böyle kalır."
            }
            Self::Space => "Boşluk, Enter gibi komutu onaylar; her zaman böyle kalır.",
            Self::Tab => {
                "Tab kenet adayları ve pencerenin alanları arasında gezer; her zaman böyle kalır."
            }
            Self::Backspace => "Geri silme yazılanı siler; her zaman böyle kalır.",
            Self::Arrow => {
                "Ok tuşu tek başına değer alanını açar, komut satırının geçmişinde ve listelerde gezer; Ctrl, Alt ya da Shift ile kullanılabilir."
            }
            Self::Value => {
                "Rakamlar, + − ve virgül yazılan değeri (uzunluk, açı, koordinat) başlatır; Shift ile rakam da klavyeye göre @ ya da ( yazar. Ctrl ya da Alt ile kullanılabilir."
            }
        }
    }
}

/// Whether a chord (as [`normalize_chord`] writes it) is one KentOS keeps for itself.
pub fn reserved_key(chord: &str) -> Option<ReservedKey> {
    let key = chord.rsplit('+').next().unwrap_or(chord);
    let key = if chord.ends_with("++") || chord == "+" {
        "+"
    } else {
        key
    };
    match key {
        "Esc" => return Some(ReservedKey::Escape),
        "Enter" => return Some(ReservedKey::Enter),
        "Tab" => return Some(ReservedKey::Tab),
        _ => {}
    }
    match chord {
        "Space" => Some(ReservedKey::Space),
        "Backspace" => Some(ReservedKey::Backspace),
        "Up" | "Down" | "Left" | "Right" => Some(ReservedKey::Arrow),
        "+" | "-" | "," => Some(ReservedKey::Value),
        _ => {
            let digit = |k: &str| k.len() == 1 && k.as_bytes()[0].is_ascii_digit();
            (digit(chord) || chord.strip_prefix("Shift+").is_some_and(digit))
                .then_some(ReservedKey::Value)
        }
    }
}

/// Why a typed name could not be one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AliasError {
    Empty,
    /// More than [`ALIAS_MAX`] characters.
    TooLong,
    /// It begins with a digit: a digit typed first starts a value.
    DigitFirst,
    /// A space, a sign or a mark: only letters and digits.
    BadCharacter,
}

impl AliasError {
    /// What is wrong and how to write it, in Turkish.
    pub fn message(self) -> &'static str {
        match self {
            Self::Empty => "Takma ad boş; komut satırına yazılacak adı girin.",
            Self::TooLong => "Takma ad en çok 32 karakter olur; kısaltın.",
            Self::DigitFirst => {
                "Takma ad harfle başlar: rakamla başlayan yazı komut satırında değer sayılır."
            }
            Self::BadCharacter => {
                "Takma adda yalnız harf ve rakam olur; boşluk ve işaretleri çıkarın."
            }
        }
    }
}

/// A typed name as it is kept, in Turkish upper case (`çz` → `ÇZ`, `i` → `İ`), or why it cannot be one.
pub fn normalize_alias(text: &str) -> Result<String, AliasError> {
    let text = text.trim();
    let Some(first) = text.chars().next() else {
        return Err(AliasError::Empty);
    };
    if text.chars().count() > ALIAS_MAX {
        return Err(AliasError::TooLong);
    }
    if first.is_ascii_digit() || first.is_numeric() {
        return Err(AliasError::DigitFirst);
    }
    if !text.chars().all(char::is_alphanumeric) {
        return Err(AliasError::BadCharacter);
    }
    Ok(text
        .chars()
        .flat_map(|c| match c {
            'i' => vec!['İ'],
            'ı' => vec!['I'],
            c => c.to_uppercase().collect(),
        })
        .collect())
}

/// `keyboard.shortcuts` as written: each command and its keys, in the map's order. An entry whose keys are not a
/// list of texts gives none here; its command and why are the host's to say.
pub fn shortcut_entries(value: &Value) -> Vec<(String, Option<Vec<String>>)> {
    let Some(map) = value.as_object() else {
        return Vec::new();
    };
    map.iter()
        .map(|(command, keys)| {
            let keys = keys.as_array().and_then(|list| {
                list.iter()
                    .map(|k| k.as_str().map(str::to_owned))
                    .collect::<Option<Vec<String>>>()
            });
            (command.clone(), keys)
        })
        .collect()
}

/// `keyboard.aliases` as written: each typed name and its command, in the map's order (a command that is not a text
/// gives none).
pub fn alias_entries(value: &Value) -> Vec<(String, Option<String>)> {
    let Some(map) = value.as_object() else {
        return Vec::new();
    };
    map.iter()
        .map(|(name, command)| (name.clone(), command.as_str().map(str::to_owned)))
        .collect()
}
