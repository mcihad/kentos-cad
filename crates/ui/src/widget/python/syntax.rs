//! Incremental Python highlighting shared by the editor and REPL transcript.
//! Ranges are UTF-8 byte ranges, as required by Iced's text highlighter.

use std::ops::Range;

use iced::advanced::text::highlighter::{self, Format, Highlighter as _};
use iced::{Color, Font, Theme};

use crate::theme::{Mode, Tokens};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Keyword,
    Constant,
    Builtin,
    String,
    Number,
    Comment,
    Decorator,
    Definition,
    Operator,
    Punctuation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct StringState {
    quote: u8,
    triple: bool,
    formatted: bool,
}

pub(crate) const KEYWORDS: &[&str] = &[
    "and", "as", "assert", "async", "await", "break", "class", "continue", "def", "del", "elif",
    "else", "except", "finally", "for", "from", "global", "if", "import", "in", "is", "lambda",
    "match", "case", "nonlocal", "not", "or", "pass", "raise", "return", "try", "while", "with",
    "yield",
];
pub(crate) const BUILTINS: &[&str] = &[
    "abs",
    "all",
    "any",
    "ascii",
    "bin",
    "bool",
    "breakpoint",
    "bytearray",
    "bytes",
    "callable",
    "chr",
    "classmethod",
    "compile",
    "complex",
    "delattr",
    "dict",
    "dir",
    "divmod",
    "enumerate",
    "eval",
    "exec",
    "filter",
    "float",
    "format",
    "frozenset",
    "getattr",
    "globals",
    "hasattr",
    "hash",
    "help",
    "hex",
    "id",
    "input",
    "int",
    "isinstance",
    "issubclass",
    "iter",
    "len",
    "list",
    "locals",
    "map",
    "max",
    "memoryview",
    "min",
    "next",
    "object",
    "oct",
    "open",
    "ord",
    "pow",
    "print",
    "property",
    "range",
    "repr",
    "reversed",
    "round",
    "set",
    "setattr",
    "slice",
    "sorted",
    "staticmethod",
    "str",
    "sum",
    "super",
    "tuple",
    "type",
    "vars",
    "zip",
    "Exception",
    "ValueError",
    "TypeError",
    "RuntimeError",
    "ZeroDivisionError",
];

/// Line states are kept only up to the earliest invalidated line.
pub struct Python {
    starts: Vec<Option<StringState>>,
    line: usize,
}

impl highlighter::Highlighter for Python {
    type Settings = ();
    type Highlight = Kind;
    type Iterator<'a> = std::vec::IntoIter<(Range<usize>, Kind)>;

    fn new(_: &()) -> Self {
        Self {
            starts: vec![None],
            line: 0,
        }
    }

    fn update(&mut self, _: &()) {
        *self = Self::new(&());
    }

    fn change_line(&mut self, line: usize) {
        self.line = self.line.min(line);
        self.starts.truncate(self.line + 1);
    }

    fn highlight_line(&mut self, line: &str) -> Self::Iterator<'_> {
        let (spans, end) = scan(line, self.starts.get(self.line).copied().flatten());
        self.line += 1;
        self.starts.truncate(self.line);
        self.starts.push(end);
        spans.into_iter()
    }

    fn current_line(&self) -> usize {
        self.line
    }
}

/// Highlights a complete source, preserving multiline strings between lines.
pub fn lines(source: &str) -> Vec<Vec<(Range<usize>, Kind)>> {
    let mut highlighter = Python::new(&());
    source
        .split('\n')
        .map(|line| highlighter.highlight_line(line).collect())
        .collect()
}

fn identifier_end(source: &str, start: usize) -> usize {
    source[start..]
        .char_indices()
        .take_while(|(_, ch)| {
            *ch == '_'
                || ch.is_alphanumeric()
                || (!ch.is_ascii() && !ch.is_whitespace() && !ch.is_ascii_punctuation())
        })
        .last()
        .map_or(start, |(i, ch)| start + i + ch.len_utf8())
}

fn number_end(bytes: &[u8], start: usize) -> usize {
    let mut i = start;
    if bytes.get(start) == Some(&b'0')
        && bytes
            .get(start + 1)
            .is_some_and(|b| matches!(b, b'x' | b'X' | b'o' | b'O' | b'b' | b'B'))
    {
        i += 2;
        while bytes
            .get(i)
            .is_some_and(|b| b.is_ascii_hexdigit() || *b == b'_')
        {
            i += 1;
        }
        return i;
    }
    while bytes
        .get(i)
        .is_some_and(|b| b.is_ascii_digit() || *b == b'_')
    {
        i += 1;
    }
    if bytes.get(i) == Some(&b'.') {
        i += 1;
        while bytes
            .get(i)
            .is_some_and(|b| b.is_ascii_digit() || *b == b'_')
        {
            i += 1;
        }
    }
    if bytes.get(i).is_some_and(|b| matches!(b, b'e' | b'E')) {
        let exponent = i;
        i += 1;
        if bytes.get(i).is_some_and(|b| matches!(b, b'+' | b'-')) {
            i += 1;
        }
        let digits = i;
        while bytes
            .get(i)
            .is_some_and(|b| b.is_ascii_digit() || *b == b'_')
        {
            i += 1;
        }
        if i == digits {
            i = exponent;
        }
    }
    if bytes.get(i).is_some_and(|b| matches!(b, b'j' | b'J')) {
        i += 1;
    }
    i
}

fn scan(
    source: &str,
    string: Option<StringState>,
) -> (Vec<(Range<usize>, Kind)>, Option<StringState>) {
    scan_depth(source, string, 0)
}

fn scan_depth(
    source: &str,
    mut string: Option<StringState>,
    depth: u8,
) -> (Vec<(Range<usize>, Kind)>, Option<StringState>) {
    let bytes = source.as_bytes();
    let mut spans = Vec::new();
    let mut i = 0;
    let mut defining = false;
    while i < bytes.len() {
        let start = i;
        if string.is_none() {
            match bytes[i] {
                b'#' => {
                    spans.push((i..bytes.len(), Kind::Comment));
                    break;
                }
                b'\'' | b'"' => {
                    let quote = bytes[i];
                    let triple = bytes
                        .get(i..i + 3)
                        .is_some_and(|b| b.iter().all(|b| *b == quote));
                    string = Some(StringState {
                        quote,
                        triple,
                        formatted: false,
                    });
                    i += if triple { 3 } else { 1 };
                }
                b'@' if source[..i].trim().is_empty() => {
                    i += 1;
                    while i < bytes.len() {
                        let end = identifier_end(source, i);
                        if end == i {
                            break;
                        }
                        i = end;
                        if bytes.get(i) == Some(&b'.') {
                            i += 1;
                        } else {
                            break;
                        }
                    }
                    spans.push((start..i, Kind::Decorator));
                    continue;
                }
                b'0'..=b'9' => {
                    i = number_end(bytes, i);
                    spans.push((start..i, Kind::Number));
                    continue;
                }
                b'.' if bytes.get(i + 1).is_some_and(u8::is_ascii_digit) => {
                    i = number_end(bytes, i);
                    spans.push((start..i, Kind::Number));
                    continue;
                }
                _ => {
                    let ch = source[i..].chars().next().unwrap_or(' ');
                    if ch == '_' || ch.is_alphabetic() {
                        i = identifier_end(source, i);
                        let word = &source[start..i];
                        let prefix = matches!(
                            word.to_ascii_lowercase().as_str(),
                            "r" | "b" | "u" | "f" | "br" | "rb" | "fr" | "rf" | "t" | "tr" | "rt"
                        );
                        if prefix && bytes.get(i).is_some_and(|b| matches!(b, b'\'' | b'"')) {
                            let quote = bytes[i];
                            let triple = bytes
                                .get(i..i + 3)
                                .is_some_and(|b| b.iter().all(|b| *b == quote));
                            string = Some(StringState {
                                quote,
                                triple,
                                formatted: word.contains(['f', 'F', 't', 'T']),
                            });
                            i += if triple { 3 } else { 1 };
                        } else {
                            let kind = if defining {
                                defining = false;
                                Some(Kind::Definition)
                            } else if KEYWORDS.contains(&word) {
                                defining = matches!(word, "def" | "class");
                                Some(Kind::Keyword)
                            } else if matches!(word, "True" | "False" | "None" | "self" | "cls") {
                                Some(Kind::Constant)
                            } else if BUILTINS.contains(&word) {
                                Some(Kind::Builtin)
                            } else {
                                None
                            };
                            if let Some(kind) = kind {
                                spans.push((start..i, kind));
                            }
                            continue;
                        }
                    } else {
                        i += ch.len_utf8();
                        if b"+-*/%=<>!&|^~@".contains(&bytes[start]) {
                            spans.push((start..i, Kind::Operator));
                        } else if b"()[]{}:.,;".contains(&bytes[start]) {
                            spans.push((start..i, Kind::Punctuation));
                        }
                        continue;
                    }
                }
            }
        }

        if let Some(open) = string {
            let mut segment = start;
            while i < bytes.len() {
                if bytes[i] == b'\\' {
                    i += 1;
                    if i < bytes.len() {
                        i += source[i..].chars().next().map_or(1, char::len_utf8);
                    }
                    continue;
                }
                let delimiter = if open.triple { 3 } else { 1 };
                if bytes
                    .get(i..i + delimiter)
                    .is_some_and(|b| b.iter().all(|b| *b == open.quote))
                {
                    i += delimiter;
                    string = None;
                    break;
                }
                // Balanced replacement fields are coloured as Python. Escaped
                // braces stay inside the literal; unfinished fields stay strings.
                if open.formatted && bytes[i] == b'{' && depth < 16 {
                    if bytes.get(i + 1) == Some(&b'{') {
                        i += 2;
                        continue;
                    }
                    if let Some(end) = replacement_end(source, i + 1) {
                        if segment < i {
                            spans.push((segment..i, Kind::String));
                        }
                        spans.push((i..i + 1, Kind::Punctuation));
                        let (inside, _) = scan_depth(&source[i + 1..end], None, depth + 1);
                        spans.extend(
                            inside
                                .into_iter()
                                .map(|(r, k)| (r.start + i + 1..r.end + i + 1, k)),
                        );
                        spans.push((end..end + 1, Kind::Punctuation));
                        i = end + 1;
                        segment = i;
                        continue;
                    }
                }
                i += source[i..].chars().next().map_or(1, char::len_utf8);
            }
            if segment < i {
                spans.push((segment..i, Kind::String));
            }
            if string.is_some() && !open.triple && !source.ends_with('\\') {
                string = None;
            }
        }
    }
    (spans, string)
}

fn replacement_end(source: &str, start: usize) -> Option<usize> {
    let mut depth = 0;
    let mut quote = None;
    let mut escaped = false;
    for (offset, ch) in source[start..].char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        if let Some(q) = quote {
            if ch == q {
                quote = None;
            }
            continue;
        }
        match ch {
            '\'' | '"' => quote = Some(ch),
            '{' => depth += 1,
            '}' if depth == 0 => return Some(start + offset),
            '}' => depth -= 1,
            _ => {}
        }
    }
    None
}

/// Distinct syntax colours, adjusted for all four library themes. Custom
/// application accents colour the operator and active line, not every token.
pub fn color(kind: Kind, theme: &Theme) -> Color {
    let t = Tokens::of(theme);
    if t.mode == Mode::HighContrast {
        return match kind {
            Kind::Keyword | Kind::Decorator => Color::from_rgb8(222, 170, 255),
            Kind::String => t.success,
            Kind::Number => t.warning,
            Kind::Constant | Kind::Builtin | Kind::Definition => t.info,
            Kind::Comment => t.muted,
            _ => t.text,
        };
    }
    let rgb = match (t.is_dark, kind) {
        (true, Kind::Keyword) => 0xc4a5f5,
        (false, Kind::Keyword) => 0x7340a0,
        (true, Kind::Definition | Kind::Builtin) => 0x81c7ee,
        (false, Kind::Definition | Kind::Builtin) => 0x176495,
        (true, Kind::Constant) => 0xf1b18d,
        (false, Kind::Constant) => 0x99500e,
        (true, Kind::String) => 0xa0d9ac,
        (false, Kind::String) => 0x26733c,
        (true, Kind::Number | Kind::Decorator) => 0xe8cc8d,
        (false, Kind::Number | Kind::Decorator) => 0x855b10,
        (_, Kind::Comment) => return t.muted,
        (_, Kind::Operator) => return t.accent_hover,
        (_, Kind::Punctuation) => return t.muted,
    };
    Color::from_rgb8((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8)
}

pub fn format(kind: &Kind, theme: &Theme) -> Format<Font> {
    Format {
        color: Some(color(*kind, theme)),
        font: Some(if matches!(kind, Kind::Keyword | Kind::Definition) {
            super::strong_font()
        } else {
            super::font()
        }),
    }
}

/// A conservative Enter heuristic. The host interpreter remains the authority
/// for incomplete code (e.g. `codeop.compile_command` in the showcase).
pub fn ready(source: &str) -> bool {
    if source.trim().is_empty() {
        return false;
    }
    let mut string = None;
    let mut depth = 0_i32;
    let mut suite = false;
    let mut last = String::new();
    for line in source.split('\n') {
        let (spans, end) = scan(line, string);
        string = end;
        let mut visible = line.as_bytes().to_vec();
        for (range, kind) in spans {
            if matches!(kind, Kind::String | Kind::Comment) {
                visible[range].fill(b' ');
            }
        }
        let code = String::from_utf8_lossy(&visible);
        for ch in code.chars() {
            match ch {
                '(' | '[' | '{' => depth += 1,
                ')' | ']' | '}' => depth -= 1,
                _ => {}
            }
        }
        let trimmed = code.trim_end();
        suite |= depth == 0 && trimmed.ends_with(':');
        if !trimmed.trim().is_empty() {
            last = trimmed.to_owned();
        }
    }
    string.is_none()
        && depth <= 0
        && !last.ends_with('\\')
        && (!suite
            || source
                .rsplit_once('\n')
                .is_some_and(|(_, last)| last.trim().is_empty()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_do_not_swallow_operators_or_identifiers() {
        let source = "1e-3 + .5 + 0xFF + 4j + 12.foo";
        let spans = lines(source).remove(0);
        let numbers: Vec<_> = spans
            .iter()
            .filter(|(_, k)| *k == Kind::Number)
            .map(|(r, _)| &source[r.clone()])
            .collect();
        assert_eq!(numbers, ["1e-3", ".5", "0xFF", "4j", "12."]);
        assert!(
            !spans
                .iter()
                .any(|(r, k)| *k == Kind::Operator && &source[r.clone()] == "-")
        );
    }

    #[test]
    fn edits_rehighlight_multiline_strings_and_utf8_ranges() {
        let mut h = Python::new(&());
        h.highlight_line("s = '''başlangıç").for_each(drop);
        assert_eq!(
            h.highlight_line("çizim # metin").collect::<Vec<_>>(),
            [(0.."çizim # metin".len(), Kind::String)]
        );
        h.change_line(0);
        h.highlight_line("s = 1").for_each(drop);
        assert!(
            h.highlight_line("# artık yorum")
                .any(|(_, k)| k == Kind::Comment)
        );
        for source in [
            "'\\ç' + 1",
            "def ölç(alan):",
            "π = f'alan={len(öğeler)}'",
            "'🦀'",
            "@tools.cache",
        ] {
            for (r, _) in lines(source).remove(0) {
                assert!(source.get(r).is_some());
            }
        }
    }

    #[test]
    fn fstrings_prefixes_and_decorators_keep_their_structure() {
        let source = "f'alan={round(alan, 2)} m²' # sonuç";
        let spans = lines(source).remove(0);
        assert!(
            spans
                .iter()
                .any(|(r, k)| *k == Kind::Builtin && &source[r.clone()] == "round")
        );
        assert!(
            spans
                .iter()
                .any(|(r, k)| *k == Kind::Number && &source[r.clone()] == "2")
        );
        let triple = lines("rf'''çizim\n# metin\n'''\n# yorum");
        assert_eq!(triple[1][0].1, Kind::String);
        assert_eq!(triple[3][0].1, Kind::Comment);
    }

    #[test]
    fn enter_waits_for_python_blocks_but_ignores_comments_and_strings() {
        assert!(ready("print('a: [') # :"));
        assert!(ready("x = {'a': 1}"));
        assert!(!ready("for x in range(3):\n    print(x)"));
        assert!(ready("for x in range(3):\n    print(x)\n"));
        assert!(!ready("s = '''a\nb"));
        assert!(!ready("print(\n    1"));
        assert!(!ready("x = 1 + \\"));
    }
}
