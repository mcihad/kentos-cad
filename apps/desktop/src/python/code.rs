//! Python's colours in the script editor (docs/adr/0136): keywords, the
//! constants and built-in functions, strings (triple-quoted ones across
//! lines), numbers, comments, decorators and the names `def` and `class`
//! make. A line is read with what the lines above left open.

use std::ops::Range;

use iced::advanced::text::highlighter::{self, Format};
use iced::{Font, Theme};
use kentos_ui::theme::{Tokens, typography};

/// What a part of a line is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Keyword,
    Constant,
    Builtin,
    Text,
    Number,
    Comment,
    Decorator,
    Defined,
}

const KEYWORDS: &[&str] = &[
    "and", "as", "assert", "async", "await", "break", "class", "continue", "def", "del", "elif",
    "else", "except", "finally", "for", "from", "global", "if", "import", "in", "is", "lambda",
    "match", "case", "nonlocal", "not", "or", "pass", "raise", "return", "try", "while", "with",
    "yield",
];
const CONSTANTS: &[&str] = &["True", "False", "None", "self", "cls"];
const BUILTINS: &[&str] = &[
    "abs",
    "all",
    "any",
    "bool",
    "dict",
    "dir",
    "enumerate",
    "filter",
    "float",
    "format",
    "getattr",
    "hasattr",
    "help",
    "int",
    "isinstance",
    "len",
    "list",
    "map",
    "max",
    "min",
    "next",
    "open",
    "print",
    "range",
    "repr",
    "reversed",
    "round",
    "set",
    "sorted",
    "str",
    "sum",
    "tuple",
    "type",
    "zip",
];

/// The colours of the lines; `open[n]` is the triple quote line `n` starts inside, if any.
pub struct Python {
    open: Vec<Option<&'static str>>,
    line: usize,
}

impl highlighter::Highlighter for Python {
    type Settings = ();
    type Highlight = Kind;
    type Iterator<'a> = std::vec::IntoIter<(Range<usize>, Kind)>;

    fn new(_: &()) -> Python {
        Python {
            open: vec![None],
            line: 0,
        }
    }

    fn update(&mut self, _: &()) {
        self.open.truncate(1);
        self.line = 0;
    }

    fn change_line(&mut self, line: usize) {
        self.open.truncate(line + 1);
        self.line = self.line.min(line);
    }

    fn highlight_line(&mut self, line: &str) -> Self::Iterator<'_> {
        let n = self.line;
        self.line += 1;
        let start = self.open.get(n).copied().flatten();
        let (spans, end) = spans(line, start);
        self.open.truncate(n + 1);
        self.open.push(end);
        spans.into_iter()
    }

    fn current_line(&self) -> usize {
        self.line
    }
}

fn word_at(bytes: &[u8], i: usize) -> usize {
    let mut j = i;
    while j < bytes.len()
        && (bytes[j].is_ascii_alphanumeric() || bytes[j] == b'_' || bytes[j] >= 0x80)
    {
        j += 1;
    }
    j
}

/// One line's spans, and the triple quote left open at its end.
pub fn spans(
    line: &str,
    open: Option<&'static str>,
) -> (Vec<(Range<usize>, Kind)>, Option<&'static str>) {
    let bytes = line.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    if let Some(quote) = open {
        match line.find(quote) {
            Some(at) => {
                out.push((0..at + 3, Kind::Text));
                i = at + 3;
            }
            None => {
                out.push((0..line.len(), Kind::Text));
                return (out, Some(quote));
            }
        }
    }
    let mut defining = false;
    while i < bytes.len() {
        let c = bytes[i];
        if c == b'#' {
            out.push((i..line.len(), Kind::Comment));
            break;
        }
        if c == b'\'' || c == b'"' {
            let triple = if c == b'"' { "\"\"\"" } else { "'''" };
            if line[i..].starts_with(triple) {
                match line[i + 3..].find(triple) {
                    Some(at) => {
                        let end = i + 3 + at + 3;
                        out.push((i..end, Kind::Text));
                        i = end;
                        continue;
                    }
                    None => {
                        out.push((i..line.len(), Kind::Text));
                        return (out, Some(triple));
                    }
                }
            }
            let mut j = i + 1;
            while j < bytes.len() && bytes[j] != c {
                j += if bytes[j] == b'\\' { 2 } else { 1 };
            }
            let end = (j + 1).min(bytes.len());
            out.push((i..end, Kind::Text));
            i = end;
            continue;
        }
        if c == b'@' && line[..i].trim().is_empty() {
            let end = word_at(bytes, i + 1);
            out.push((i..end, Kind::Decorator));
            i = end;
            continue;
        }
        if c.is_ascii_digit() {
            let mut j = i;
            while j < bytes.len()
                && (bytes[j].is_ascii_alphanumeric() || matches!(bytes[j], b'.' | b'_'))
            {
                j += 1;
            }
            out.push((i..j, Kind::Number));
            i = j;
            continue;
        }
        if c.is_ascii_alphabetic() || c == b'_' || c >= 0x80 {
            let end = word_at(bytes, i);
            let word = &line[i..end];
            // A string's prefix (f"…", rb'…') belongs to the string that follows.
            let kind = if defining {
                defining = false;
                Some(Kind::Defined)
            } else if KEYWORDS.contains(&word) {
                defining = word == "def" || word == "class";
                Some(Kind::Keyword)
            } else if CONSTANTS.contains(&word) {
                Some(Kind::Constant)
            } else if BUILTINS.contains(&word) && bytes.get(end) == Some(&b'(') {
                Some(Kind::Builtin)
            } else {
                None
            };
            if let Some(kind) = kind {
                out.push((i..end, kind));
            }
            i = end;
            continue;
        }
        i += 1;
    }
    (out, None)
}

pub fn format(kind: &Kind, theme: &Theme) -> Format<Font> {
    let t = Tokens::of(theme);
    let (color, font) = match kind {
        Kind::Keyword => (t.accent, Some(typography::mono_strong())),
        Kind::Constant => (t.info, None),
        Kind::Builtin => (t.info, None),
        Kind::Text => (t.success, None),
        Kind::Number => (t.warning, None),
        Kind::Comment => (t.faint, None),
        Kind::Decorator => (t.warning, None),
        Kind::Defined => (t.text, Some(typography::mono_strong())),
    };
    Format {
        color: Some(color),
        font,
    }
}

#[cfg(test)]
mod tests {
    use super::{Kind, spans};

    fn kinds(line: &str) -> Vec<(&str, Kind)> {
        spans(line, None)
            .0
            .into_iter()
            .map(|(r, k)| (&line[r], k))
            .collect()
    }

    #[test]
    fn a_line_is_read_as_python() {
        assert_eq!(
            kinds("def alan(p):  # yarım"),
            [
                ("def", Kind::Keyword),
                ("alan", Kind::Defined),
                ("# yarım", Kind::Comment)
            ]
        );
        assert_eq!(
            kinds("for r in doc.entities(kinds='polygon'):"),
            [
                ("for", Kind::Keyword),
                ("in", Kind::Keyword),
                ("'polygon'", Kind::Text)
            ]
        );
        assert_eq!(
            kinds("x = 12.5 if a else None"),
            [
                ("12.5", Kind::Number),
                ("if", Kind::Keyword),
                ("else", Kind::Keyword),
                ("None", Kind::Constant)
            ]
        );
        assert_eq!(
            kinds("print(len(a))"),
            [("print", Kind::Builtin), ("len", Kind::Builtin)]
        );
        assert_eq!(kinds("@dataclass"), [("@dataclass", Kind::Decorator)]);
        assert_eq!(
            kinds(r#"s = "a\"b" + 'ç'"#),
            [(r#""a\"b""#, Kind::Text), ("'ç'", Kind::Text)]
        );
    }

    #[test]
    fn a_triple_quote_stays_open_across_lines() {
        let (first, open) = spans(r#"doc = """Açıklama"#, None);
        assert_eq!(open, Some("\"\"\""));
        assert_eq!(first.last().map(|(_, k)| *k), Some(Kind::Text));
        let (middle, still) = spans("orta satır", open);
        assert_eq!((middle.len(), still), (1, open));
        let (last, closed) = spans(r#"son""" + x"#, open);
        assert_eq!(closed, None);
        assert_eq!(last[0], (0..6, Kind::Text));
    }
}
