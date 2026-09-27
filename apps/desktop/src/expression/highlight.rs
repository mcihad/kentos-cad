//! The builder's colours: the core's token classes (`kentos_expression::
//! editor::tokens`) as the text editor's highlighter, the error and the
//! warnings in their tones, the parenthesis at the cursor and its pair
//! (DESIGN.md §7.16). The colours are the web's `--c-syn-*` tokens, one set
//! for the dark themes and one for the light (apps/web/src/styles/tokens.css).
//!
//! The highlighter reads a table made for the whole text at each change
//! (the tokens are the whole text's: a quoted text may run over lines), so a
//! line is coloured from its spans, not lexed alone.

use std::ops::Range;
use std::sync::Arc;

use iced::advanced::text::highlighter::{Format, Highlighter};
use iced::{Color, Font, Theme};
use kentos_expression::editor::{Class, Span, units};
use kentos_ui::theme::tokens::hex;
use kentos_ui::theme::{Mode, Tokens};

/// What a piece of the text is painted as.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Paint {
    Token(Class),
    /// Under the error: the danger tone.
    Error,
    /// A field the objects lack: the warning tone.
    Warning,
    /// The parenthesis beside the cursor and its pair.
    Match,
    /// A parenthesis without its pair.
    Unmatched,
}

/// The syntax colours of a theme (the web's `--c-syn-*`).
pub(crate) struct Syntax {
    pub field: Color,
    pub variable: Color,
    pub function: Color,
    pub keyword: Color,
    pub literal: Color,
    pub text: Color,
    pub operator: Color,
}

impl Syntax {
    pub fn of(mode: Mode) -> Syntax {
        if mode.is_dark() {
            Syntax {
                field: hex(0x7fb6ee),
                variable: hex(0x5fcfc0),
                function: hex(0xc3a0f2),
                keyword: hex(0xee8fb2),
                literal: hex(0xf0a37c),
                text: hex(0x9ed48f),
                operator: hex(0xa9b4c1),
            }
        } else {
            Syntax {
                field: hex(0x1d5ea6),
                variable: hex(0x0b776b),
                function: hex(0x6d3fb5),
                keyword: hex(0xa8356a),
                literal: hex(0xb0521b),
                text: hex(0x2c7a2f),
                operator: hex(0x4d5966),
            }
        }
    }

    /// The colour of a token's class.
    pub fn class(&self, class: Class, t: &Tokens) -> Color {
        match class {
            Class::Number | Class::Constant => self.literal,
            Class::Text => self.text,
            Class::Field => self.field,
            Class::Variable => self.variable,
            Class::Function => self.function,
            Class::Keyword => self.keyword,
            Class::Operator | Class::Paren | Class::Comma => self.operator,
            Class::Unknown => t.danger,
        }
    }
}

/// The colour of a paint in a theme.
pub(crate) fn color(paint: Paint, theme: &Theme) -> Color {
    let t = Tokens::of(theme);
    match paint {
        Paint::Token(class) => Syntax::of(Mode::of(theme)).class(class, &t),
        Paint::Error | Paint::Unmatched => t.danger,
        Paint::Warning => t.warning,
        Paint::Match => t.accent,
    }
}

/// A line's spans: byte ranges in the line and their paints.
pub(crate) type LineSpans = Vec<(Range<usize>, Paint)>;

/// The highlighter's input: each line's spans.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Lines(pub Arc<Vec<LineSpans>>);

impl Lines {
    /// The table for a text: its tokens, then the marks over them (a mark
    /// paints over a token's colour).
    pub fn of(src: &str, tokens: &[Span], marks: &[(usize, usize, Paint)]) -> Lines {
        let lengths: Vec<usize> = src.split('\n').map(str::len).collect();
        let mut lines: Vec<LineSpans> = vec![Vec::new(); lengths.len()];
        let mut put = |start: usize, end: usize, paint: Paint| {
            let (l0, b0) = units::to_line_byte(src, start);
            let (l1, b1) = units::to_line_byte(src, end);
            for (line, spans) in lines.iter_mut().enumerate().take(l1 + 1).skip(l0) {
                let from = if line == l0 { b0 } else { 0 };
                let to = if line == l1 { b1 } else { lengths[line] };
                if to > from {
                    spans.push((from..to, paint));
                }
            }
        };
        for t in tokens {
            put(t.start, t.end, Paint::Token(t.class));
        }
        for &(start, end, paint) in marks {
            put(start, end, paint);
        }
        Lines(Arc::new(lines))
    }
}

/// The text editor's highlighter over a [`Lines`] table.
pub(crate) struct Painter {
    lines: Lines,
    current: usize,
}

impl Highlighter for Painter {
    type Settings = Lines;
    type Highlight = Paint;
    type Iterator<'a> = std::vec::IntoIter<(Range<usize>, Paint)>;

    fn new(settings: &Lines) -> Self {
        Painter {
            lines: settings.clone(),
            current: 0,
        }
    }

    fn update(&mut self, settings: &Lines) {
        self.lines = settings.clone();
        self.current = 0;
    }

    fn change_line(&mut self, line: usize) {
        self.current = self.current.min(line);
    }

    fn highlight_line(&mut self, line: &str) -> Self::Iterator<'_> {
        let spans: Vec<(Range<usize>, Paint)> = self
            .lines
            .0
            .get(self.current)
            .map(|spans| {
                spans
                    .iter()
                    .filter_map(|(range, paint)| {
                        // A table a step behind the text (it is remade after the edit) never cuts a character.
                        let end = range.end.min(line.len());
                        let start = range.start.min(end);
                        (line.is_char_boundary(start) && line.is_char_boundary(end) && start < end)
                            .then_some((start..end, *paint))
                    })
                    .collect()
            })
            .unwrap_or_default();
        self.current += 1;
        spans.into_iter()
    }

    fn current_line(&self) -> usize {
        self.current
    }
}

/// A paint's look in the editor: its colour, the editor's font.
pub(crate) fn format(paint: &Paint, theme: &Theme) -> Format<Font> {
    Format {
        color: Some(color(*paint, theme)),
        font: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_table_splits_tokens_over_lines_in_bytes() {
        let src = "'ağ\nx' + 1";
        let tokens = kentos_expression::editor::tokens(src);
        let lines = Lines::of(src, &tokens, &[(9, 10, Paint::Error)]);
        let text = Paint::Token(Class::Text);
        assert_eq!(
            lines.0[0],
            vec![(0..4, text)],
            "a quoted text runs to the line's end"
        );
        assert_eq!(
            lines.0[1],
            vec![
                (0..2, text),
                (3..4, Paint::Token(Class::Operator)),
                (5..6, Paint::Token(Class::Number)),
                (5..6, Paint::Error),
            ]
        );
    }
}
