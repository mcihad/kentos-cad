//! The console's help while code is typed (docs/adr/0135): the names the
//! Python process answers with (`kentos._assist`) as KentOS UI's completion
//! items, the signature line, and the code box's cursor as the character
//! offset Python counts in.

use iced::widget::text_editor;
use kentos_ui::widget::python::{CompletionItem, SymbolKind};
use serde_json::Value;

/// The process's answer as the completion list's items: each name, what it
/// is (the process's kinds: keyword, module, class, function, property,
/// value) and its hint (a signature, a type).
pub fn items(answer: &Value) -> Vec<CompletionItem> {
    let Some(list) = answer.get("items").and_then(Value::as_array) else {
        return Vec::new();
    };
    list.iter()
        .filter_map(|i| {
            let name = i.get("text")?.as_str()?;
            let kind = match i.get("kind").and_then(Value::as_str).unwrap_or_default() {
                "keyword" => SymbolKind::Keyword,
                "module" => SymbolKind::Module,
                "class" => SymbolKind::Class,
                "function" => SymbolKind::Function,
                "property" => SymbolKind::Property,
                _ => SymbolKind::Variable,
            };
            let mut item = CompletionItem::new(name, kind);
            item.detail = i
                .get("detail")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned();
            Some(item)
        })
        .collect()
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn offsets_count_characters_as_python_does() {
        let code = "ığ = 1\ncad.poly";
        assert_eq!(offset(code, 0, 2), 1, "ı is two bytes, one character");
        assert_eq!(offset(code, 1, 8), 15);
        assert_eq!(offset(code, 1, 3), 10);
    }

    #[test]
    fn the_answer_becomes_the_lists_items_with_their_kinds() {
        let items = items(&json!({"start": 4, "items": [
            {"text": "polygon", "kind": "module", "detail": "Kapalı alan"},
            {"text": "Document", "kind": "class", "detail": ""},
            {"text": "create", "kind": "function", "detail": "(doc, layer_id, pts)"},
            {"text": "pts", "kind": "value", "detail": "list"},
            {"text": "if", "kind": "keyword", "detail": ""}]}));
        let seen: Vec<(&str, SymbolKind, &str)> = items
            .iter()
            .map(|i| (i.name.as_str(), i.kind, i.detail.as_str()))
            .collect();
        assert_eq!(
            seen,
            [
                ("polygon", SymbolKind::Module, "Kapalı alan"),
                ("Document", SymbolKind::Class, ""),
                ("create", SymbolKind::Function, "(doc, layer_id, pts)"),
                ("pts", SymbolKind::Variable, "list"),
                ("if", SymbolKind::Keyword, ""),
            ]
        );
        assert!(super::items(&json!({"start": 0})).is_empty());
    }
}
