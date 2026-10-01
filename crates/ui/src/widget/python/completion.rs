//! Versioned, provider-neutral Python completion. Replacement ranges are byte
//! columns, just like Iced's editor; source and cursor changes reject old results.

use super::syntax::{self, Kind};
use iced::widget::text_editor::{Content, Position};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolKind {
    Function,
    Method,
    Parameter,
    Class,
    Module,
    Package,
    Variable,
    Property,
    Keyword,
    Constant,
}
impl SymbolKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Function => "Fonksiyon",
            Self::Method => "Metot",
            Self::Parameter => "Parametre",
            Self::Class => "Sınıf",
            Self::Module => "Modül",
            Self::Package => "Paket",
            Self::Variable => "Değişken",
            Self::Property => "Özellik",
            Self::Keyword => "Anahtar",
            Self::Constant => "Sabit",
        }
    }
}
#[derive(Debug, Clone)]
pub struct CompletionItem {
    pub name: String,
    pub insert: String,
    pub kind: SymbolKind,
    pub detail: String,
    pub documentation: String,
}
impl CompletionItem {
    pub fn new(name: impl Into<String>, kind: SymbolKind) -> Self {
        let name = name.into();
        Self {
            insert: name.clone(),
            name,
            kind,
            detail: String::new(),
            documentation: String::new(),
        }
    }
}
#[derive(Debug, Clone)]
pub struct CompletionRequest {
    pub generation: u64,
    pub source: String,
    pub line: usize,
    pub column: usize,
}
#[derive(Debug, Clone)]
pub enum CompletionEvent {
    Request,
    Select(usize),
    Move(i8),
    Accept,
    Pick(usize),
    Dismiss,
}

#[derive(Debug, Default)]
pub struct CompletionState {
    pub items: Vec<CompletionItem>,
    pub selected: usize,
    pub open: bool,
    pub loading: bool,
    pub request: Option<CompletionRequest>,
    pub(crate) start: usize,
    pub(crate) end: usize,
    pub(crate) prefix: String,
    serial: u64,
    user_selected: bool,
}
impl CompletionState {
    pub fn selected(&self) -> Option<&CompletionItem> {
        self.items.get(self.selected)
    }
    pub fn dismiss(&mut self) {
        self.open = false;
        self.loading = false;
        self.request = None;
    }
    pub fn refresh(&mut self, content: &Content, explicit: bool) {
        let position = content.cursor().position;
        if content.cursor().selection.is_some() {
            self.dismiss();
            return;
        }
        let source = content.text();
        let Some(line) = source.split('\n').nth(position.line) else {
            self.dismiss();
            return;
        };
        let Some(before) = line.get(..position.column) else {
            self.dismiss();
            return;
        };
        let start = before
            .char_indices()
            .rev()
            .take_while(|(_, c)| *c == '_' || c.is_alphanumeric())
            .last()
            .map_or(position.column, |(i, _)| i);
        let prefix = &before[start..];
        let import =
            before.trim_start().starts_with("import ") || before.trim_start().starts_with("from ");
        let attribute = before[..start].ends_with('.');
        let blocked = syntax::lines(&source)
            .get(position.line)
            .is_some_and(|spans| {
                spans.iter().any(|(range, kind)| {
                    matches!(kind, Kind::Comment | Kind::String)
                        && range.start < position.column
                        && range.end >= position.column
                })
            });
        if blocked
            || (!explicit && prefix.chars().count() < 2 && !attribute && !before.ends_with('('))
        {
            self.dismiss();
            return;
        }
        self.serial = self.serial.wrapping_add(1);
        self.start = start;
        self.end = position.column
            + line[position.column..]
                .chars()
                .take_while(|c| *c == '_' || c.is_alphanumeric())
                .map(char::len_utf8)
                .sum::<usize>();
        self.prefix = prefix.to_owned();
        self.request = Some(CompletionRequest {
            generation: self.serial,
            source: source.clone(),
            line: position.line,
            column: position.column,
        });
        self.open = true;
        self.loading = true;
        self.selected = 0;
        self.user_selected = false;
        self.items = if import || attribute {
            Vec::new()
        } else {
            local_items(&source, prefix)
        };
    }
    pub fn receive(&mut self, generation: u64, mut items: Vec<CompletionItem>) -> bool {
        if !self.open
            || self
                .request
                .as_ref()
                .is_none_or(|request| request.generation != generation)
        {
            return false;
        }
        let selected = self
            .user_selected
            .then(|| self.selected().map(|item| item.name.clone()))
            .flatten();
        items.retain(|item| {
            item.name.starts_with(&self.prefix)
                && !item.insert.is_empty()
                && item.insert != self.prefix
        });
        items.sort_by_key(|item| {
            (
                item.name.starts_with('_'),
                item.kind != SymbolKind::Parameter,
                item.name.clone(),
            )
        });
        items.dedup_by(|a, b| a.name == b.name);
        items.truncate(128);
        self.selected = selected
            .and_then(|name| items.iter().position(|item| item.name == name))
            .unwrap_or(0);
        self.items = items;
        self.loading = false;
        if self.items.is_empty() {
            self.open = false;
        }
        true
    }
    pub fn navigate(&mut self, direction: i8) {
        self.user_selected = true;
        if !self.items.is_empty() {
            self.selected = (self.selected as isize + direction as isize)
                .rem_euclid(self.items.len() as isize) as usize;
        }
    }
    pub fn select(&mut self, index: usize) {
        self.user_selected = true;
        self.selected = index.min(self.items.len().saturating_sub(1));
    }
    pub(crate) fn range(&self, content: &Content) -> Option<(Position, Position)> {
        let request = self.request.as_ref()?;
        if request.source != content.text()
            || content.cursor().position
                != (Position {
                    line: request.line,
                    column: request.column,
                })
        {
            return None;
        }
        Some((
            Position {
                line: request.line,
                column: self.start,
            },
            Position {
                line: request.line,
                column: self.end,
            },
        ))
    }
}

fn local_items(source: &str, prefix: &str) -> Vec<CompletionItem> {
    let mut symbols = BTreeMap::new();
    for name in syntax::KEYWORDS {
        symbols.insert(
            (*name).to_owned(),
            CompletionItem::new(*name, SymbolKind::Keyword),
        );
    }
    for name in syntax::BUILTINS {
        symbols.insert(
            (*name).to_owned(),
            CompletionItem::new(
                *name,
                if name.chars().next().is_some_and(char::is_uppercase) {
                    SymbolKind::Class
                } else {
                    SymbolKind::Function
                },
            ),
        );
    }
    for name in ["True", "False", "None"] {
        symbols.insert(name.into(), CompletionItem::new(name, SymbolKind::Constant));
    }
    for line in source.lines() {
        let line = line.trim();
        let (rest, kind) = if let Some(rest) = line
            .strip_prefix("def ")
            .or_else(|| line.strip_prefix("async def "))
        {
            (rest, SymbolKind::Function)
        } else if let Some(rest) = line.strip_prefix("class ") {
            (rest, SymbolKind::Class)
        } else if line.contains('=') && !line.starts_with('#') {
            (line, SymbolKind::Variable)
        } else {
            continue;
        };
        let name: String = rest
            .chars()
            .take_while(|c| *c == '_' || c.is_alphanumeric())
            .collect();
        if name.is_empty() {
            continue;
        }
        let mut item = CompletionItem::new(&name, kind);
        item.detail = line.split(':').next().unwrap_or(line).to_owned();
        item.documentation = "Bu betikte tanımlı.".into();
        symbols.insert(name, item);
    }
    symbols
        .into_values()
        .filter(|item| item.name.starts_with(prefix) && item.insert != prefix)
        .take(128)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::widget::text_editor::{Action, Motion};
    #[test]
    fn imports_comments_unicode_and_stale_requests_have_distinct_contexts() {
        let mut state = CompletionState::default();
        let mut content = Content::with_text("import ma");
        content.perform(Action::Move(Motion::DocumentEnd));
        state.refresh(&content, false);
        assert!(state.open && state.items.is_empty());
        let old = state.request.clone().unwrap();
        content = Content::with_text("# pri");
        content.perform(Action::Move(Motion::DocumentEnd));
        state.refresh(&content, true);
        assert!(!state.open);
        assert!(!state.receive(
            old.generation,
            vec![CompletionItem::new("math", SymbolKind::Module)]
        ));
        content = Content::with_text("ölçü = 42\nöl");
        content.perform(Action::Move(Motion::DocumentEnd));
        state.refresh(&content, false);
        assert_eq!(state.selected().unwrap().name, "ölçü");
        assert_eq!(state.range(&content).unwrap().0.column, 0);
    }
}
