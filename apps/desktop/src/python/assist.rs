//! The console's help while code is typed (docs/adr/0135): the completion
//! list and the signature line the Python process answers with
//! (`kentos._assist`), and the code box's cursor as the character offset
//! Python counts in.

use std::sync::Arc;

use iced::widget::text_editor::{self, Action, Cursor, Edit, Position};
use serde_json::Value;

/// One entry of the list: a name, what it is, and a hint (a signature, a type).
#[derive(Clone, Debug, PartialEq)]
pub struct Choice {
    pub text: String,
    pub kind: String,
    pub detail: String,
}

/// The names that can end the word that starts at `start` (a character offset).
#[derive(Clone, Debug, PartialEq)]
pub struct List {
    pub start: usize,
    all: Vec<Choice>,
    /// The entries that still fit what was typed, by their place in `all`.
    pub shown: Vec<usize>,
    pub active: usize,
}

impl List {
    /// The process's answer as a list; none when nothing can end the word.
    pub fn of(answer: &Value) -> Option<Self> {
        let start = usize::try_from(answer.get("start")?.as_u64()?).ok()?;
        let all: Vec<Choice> = answer
            .get("items")?
            .as_array()?
            .iter()
            .filter_map(|i| {
                Some(Choice {
                    text: i.get("text")?.as_str()?.to_owned(),
                    kind: i
                        .get("kind")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_owned(),
                    detail: i
                        .get("detail")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_owned(),
                })
            })
            .collect();
        if all.is_empty() {
            return None;
        }
        let shown = (0..all.len()).collect();
        Some(Self {
            start,
            all,
            shown,
            active: 0,
        })
    }

    /// What was typed since the list opened narrows it; false when the word
    /// ended or nothing fits any more.
    pub fn follow(&mut self, code: &str, cursor: usize) -> bool {
        if cursor < self.start {
            return false;
        }
        let typed: String = code
            .chars()
            .skip(self.start)
            .take(cursor - self.start)
            .collect();
        if typed.chars().any(|c| !(c.is_alphanumeric() || c == '_')) {
            return false;
        }
        let lower = typed.to_lowercase();
        self.shown = self
            .all
            .iter()
            .enumerate()
            .filter(|(_, c)| {
                c.text.starts_with(&typed) || c.text.to_lowercase().starts_with(&lower)
            })
            .map(|(i, _)| i)
            .collect();
        self.active = self.active.min(self.shown.len().saturating_sub(1));
        !self.shown.is_empty()
    }

    pub fn step(&mut self, by: i32) {
        let n = self.shown.len();
        if n > 0 {
            let at = (self.active as i64 + i64::from(by)).rem_euclid(n as i64);
            self.active = usize::try_from(at).unwrap_or(0);
        }
    }

    /// The entry taken: the one clicked (its place among those shown), or the active one.
    pub fn chosen(&self, which: Option<usize>) -> Option<&Choice> {
        let at = which.unwrap_or(self.active);
        self.shown.get(at).and_then(|i| self.all.get(*i))
    }

    /// The entries shown, in order.
    pub fn entries(&self) -> impl Iterator<Item = &Choice> {
        self.shown.iter().filter_map(|i| self.all.get(*i))
    }
}

/// The call the cursor is in: its label, the first paragraph of its help,
/// and the parameter being written.
#[derive(Clone, Debug, PartialEq)]
pub struct Signature {
    pub label: String,
    pub doc: String,
    pub argument: Option<String>,
}

impl Signature {
    pub fn of(answer: &Value) -> Option<Self> {
        Some(Self {
            label: answer.get("label")?.as_str()?.to_owned(),
            doc: answer
                .get("doc")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned(),
            argument: answer
                .get("argument")
                .and_then(Value::as_str)
                .map(str::to_owned),
        })
    }
}

/// The code and the cursor as a character offset in it.
pub fn caret(content: &text_editor::Content) -> (String, usize) {
    let code = content.text();
    let c = content.cursor();
    let at = offset(&code, c.position.line, c.position.column);
    (code, at)
}

/// A line's byte column as a character offset in the whole text.
pub fn offset(code: &str, line: usize, column: usize) -> usize {
    let mut chars = 0;
    for (i, l) in code.split('\n').enumerate() {
        if i == line {
            let head = l.get(..column.min(l.len())).unwrap_or(l);
            return chars + head.chars().count();
        }
        chars += l.chars().count() + 1;
    }
    chars.saturating_sub(1)
}

/// A character offset as a line and its byte column.
pub fn position(code: &str, offset: usize) -> (usize, usize) {
    let mut left = offset;
    let mut last = (0, 0);
    for (i, l) in code.split('\n').enumerate() {
        let n = l.chars().count();
        if left <= n {
            let byte = l.char_indices().nth(left).map_or(l.len(), |(b, _)| b);
            return (i, byte);
        }
        left -= n + 1;
        last = (i, l.len());
    }
    last
}

/// Replaces the characters `start..end` of the box's code with `text`.
pub fn replace(
    content: &mut text_editor::Content,
    code: &str,
    start: usize,
    end: usize,
    text: &str,
) {
    let (l0, c0) = position(code, start);
    let (l1, c1) = position(code, end.max(start));
    content.move_to(Cursor {
        position: Position {
            line: l1,
            column: c1,
        },
        selection: (start != end).then_some(Position {
            line: l0,
            column: c0,
        }),
    });
    content.perform(Action::Edit(Edit::Paste(Arc::new(text.to_owned()))));
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn offsets_count_characters_as_python_does() {
        let code = "ığ = 1\ncad.poly";
        assert_eq!(offset(code, 0, 2), 1, "ı is two bytes, one character");
        assert_eq!(offset(code, 1, 8), 15);
        assert_eq!(position(code, 15), (1, 8));
        assert_eq!(position(code, 1), (0, 2));
        assert_eq!(position(code, 99), (1, 8));
    }

    #[test]
    fn the_list_narrows_as_the_word_grows_and_closes_when_it_ends() {
        let mut list = List::of(&json!({"start": 4, "items": [
            {"text": "polygon", "kind": "module", "detail": ""},
            {"text": "polyline", "kind": "module", "detail": ""},
            {"text": "point", "kind": "module", "detail": ""}]}))
        .expect("a list");
        assert!(list.follow("cad.poly", 8));
        assert_eq!(
            list.entries().map(|c| c.text.as_str()).collect::<Vec<_>>(),
            ["polygon", "polyline"]
        );
        list.step(1);
        assert_eq!(list.chosen(None).map(|c| c.text.as_str()), Some("polyline"));
        list.step(1);
        assert_eq!(
            list.chosen(None).map(|c| c.text.as_str()),
            Some("polygon"),
            "round"
        );
        assert!(list.follow("cad.POLYG", 9), "the case need not match");
        assert!(!list.follow("cad.polygon.", 12), "a dot ends the word");
        assert!(!list.follow("cad", 3), "back before the word");
        assert!(List::of(&json!({"start": 0, "items": []})).is_none());
    }
}
