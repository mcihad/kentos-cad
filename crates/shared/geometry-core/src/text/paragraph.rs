//! Multi-line text (docs/adr/0182 §2): where a text's lines break and stand,
//! and its letter formats' runs as an editor changes them. Both platforms
//! take these from here (the web through WASM); the cases of
//! `fixtures/text/v1/paragraph.json` are an independent reference's
//! (scripts/fixtures/paragraph_cases.py).
//!
//! - The text breaks into paragraphs at `\n`; with a box width each paragraph
//!   wraps word by word (a word: letters that are not spaces): a word that
//!   would carry the line past the box starts the next one, the spaces before
//!   it dropped; a word wider than the box stays whole on its own line. A
//!   line ends at its last letter that is not a space.
//! - A letter is as wide as its face's advance (the bold table for a bold
//!   letter), times 0.6 raised or lowered, over 1000, times the height and the
//!   width factor; a line is the sum of its letters, left to right.
//! - The lines' baselines are 5/3 of the height times the line spacing apart.
//!   The box is the box width, or the widest line; each line stands at its
//!   alignment's share of what is left of the box (left 0, centre ½, right 1).

use super::{Font, advance_of};
use crate::vec2::Vec2;

/// A run raised over the line or lowered under it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Script {
    Super,
    Sub,
}

/// A range of a text's letters (Unicode scalar values) and their format.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Run {
    pub start: u32,
    pub end: u32,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub script: Option<Script>,
    pub color: Option<String>,
}

impl Run {
    fn has_format(&self) -> bool {
        self.bold || self.italic || self.underline || self.script.is_some() || self.color.is_some()
    }

    fn same_format(&self, o: &Run) -> bool {
        self.bold == o.bold
            && self.italic == o.italic
            && self.underline == o.underline
            && self.script == o.script
            && self.color == o.color
    }

    /// The format alone, as a run of no letters.
    fn format(&self) -> Run {
        Run {
            start: 0,
            end: 0,
            ..self.clone()
        }
    }
}

/// The share of a line's height a raised or lowered letter takes.
pub const SCRIPT_SIZE: f64 = 0.6;
/// The baselines' distance in heights at line spacing 1 (AutoCAD's MTEXT).
pub const PITCH: f64 = 5.0 / 3.0;

/// One line: its letters `start..end` (Unicode scalar values of the text),
/// its width, metres, where it stands from the box's left, and how far its
/// baseline is under the first line's.
#[derive(Clone, Debug, PartialEq)]
pub struct Line {
    pub start: usize,
    pub end: usize,
    pub width: f64,
    pub x: f64,
    pub y: f64,
}

/// A text's lines and its box: the box's width and the baselines' distance.
#[derive(Clone, Debug, PartialEq)]
pub struct Layout {
    pub lines: Vec<Line>,
    pub width: f64,
    pub pitch: f64,
}

/// What lays a text out: its letters, formats, height and width factor, box
/// width and line spacing, and its alignment's share along the box.
#[derive(Clone, Copy, Debug)]
pub struct Paragraph<'a> {
    pub text: &'a str,
    pub runs: &'a [Run],
    pub height: f64,
    pub width_factor: f64,
    pub box_width: Option<f64>,
    pub line_spacing: f64,
    pub along: f64,
    pub font: Font,
    /// Every letter bold: the text's own bold (docs/adr/0183 §2), its runs' on top.
    pub bold: bool,
}

/// A letter's advance in thousandths of an em (§2): the bold table's for a
/// bold letter, 0.6 of it raised or lowered. What a line's letters stand on,
/// for the overlays that draw a line run by run.
pub fn advance(font: Font, c: char, bold: bool, script: bool) -> f64 {
    let a = f64::from(advance_of(font, c, bold));
    if script { a * SCRIPT_SIZE } else { a }
}

/// Each letter's width in thousandths of an em at the text's height (bold from the bold table, raised or
/// lowered at 0.6).
fn advances(letters: &[char], runs: &[Run], font: Font, all_bold: bool) -> Vec<f64> {
    let mut out = Vec::with_capacity(letters.len());
    let mut run = runs.iter().peekable();
    for (i, &c) in letters.iter().enumerate() {
        while run.peek().is_some_and(|r| (r.end as usize) <= i) {
            run.next();
        }
        let format = run.peek().filter(|r| (r.start as usize) <= i);
        let bold = all_bold || format.is_some_and(|r| r.bold);
        let a = f64::from(advance_of(font, c, bold));
        out.push(if format.is_some_and(|r| r.script.is_some()) {
            a * SCRIPT_SIZE
        } else {
            a
        });
    }
    out
}

/// The text's lines and box (§2).
pub fn lay_out(p: &Paragraph<'_>) -> Layout {
    let letters: Vec<char> = p.text.chars().collect();
    let adv = advances(&letters, p.runs, p.font, p.bold);
    let scale = |sum: f64| sum / 1000.0 * p.height * p.width_factor;
    // From +0: an empty line is 0 wide, not −0 (a float sum of nothing is −0).
    let width_of = |from: usize, to: usize| scale(adv[from..to].iter().fold(0.0, |s, a| s + a));
    let mut lines = Vec::new();
    let mut start = 0;
    for (i, &c) in letters.iter().chain(std::iter::once(&'\n')).enumerate() {
        if c != '\n' {
            continue;
        }
        let end = i;
        match p.box_width {
            None => lines.push((start, end)),
            Some(w) => wrap(&letters, start, end, w, &width_of, &mut lines),
        }
        start = i + 1;
    }
    // A line ends at its last letter that is not a space: the spaces after it take no room.
    for (s, e) in lines.iter_mut() {
        while *e > *s && letters[*e - 1] == ' ' {
            *e -= 1;
        }
    }
    let widths: Vec<f64> = lines.iter().map(|&(s, e)| width_of(s, e)).collect();
    let box_width = p
        .box_width
        .unwrap_or_else(|| widths.iter().copied().fold(0.0, crate::jsmath::js_max));
    let pitch = PITCH * p.height * p.line_spacing;
    let lines = lines
        .into_iter()
        .zip(widths)
        .enumerate()
        .map(|(i, ((start, end), width))| Line {
            start,
            end,
            width,
            x: p.along * (box_width - width),
            y: i as f64 * pitch,
        })
        .collect();
    Layout {
        lines,
        width: box_width,
        pitch,
    }
}

/// One paragraph `from..to` wrapped word by word to `box_width`.
fn wrap(
    letters: &[char],
    from: usize,
    to: usize,
    box_width: f64,
    width_of: &dyn Fn(usize, usize) -> f64,
    out: &mut Vec<(usize, usize)>,
) {
    let space = |i: usize| letters[i] == ' ';
    let mut start = from;
    // The end of the last word on the line being built; none before its first word.
    let mut filled: Option<usize> = None;
    let mut i = from;
    loop {
        while i < to && space(i) {
            i += 1;
        }
        if i >= to {
            break;
        }
        let word = i;
        while i < to && !space(i) {
            i += 1;
        }
        if let Some(end) = filled
            && width_of(start, i) > box_width
        {
            out.push((start, end));
            start = word;
        }
        filled = Some(i);
    }
    out.push((start, to));
}

/// How far `p` stands over the first line's baseline for a vertical share
/// `up` of a one-line text (0 baseline, −0.2 bottom, ½ middle, 1 top), with
/// `count` lines `pitch` apart: the top and the baseline are the first
/// line's, the bottom the last line's, the middle halfway between the first
/// and the last line's middles.
pub fn rise(up: f64, height: f64, count: usize, pitch: f64) -> f64 {
    let below = count.saturating_sub(1) as f64 * pitch;
    if up < 0.0 {
        up * height - below
    } else if up > 0.0 && up < 1.0 {
        up * height - below / 2.0
    } else {
        up * height
    }
}

/// Çok satırlı yazı's box from two corners `a` and `b` (docs/adr/0182 §4)
/// for a text turned `rotation` degrees: its top left corner (the text's
/// point, its alignment the top's left) and its width along the turn; none
/// for a width under a micrometre (no box: the lines end at their breaks).
pub fn corner_box(a: Vec2, b: Vec2, rotation: f64) -> (Vec2, Option<f64>) {
    let r = rotation * crate::jsmath::PI / 180.0;
    let (c, s) = (crate::jsmath::cos(r), crate::jsmath::sin(r));
    // Along the turn (u) and up from it (v).
    let along = |p: Vec2| p.x * c + p.y * s;
    let up = |p: Vec2| -p.x * s + p.y * c;
    let left = crate::jsmath::js_min(along(a), along(b));
    let right = crate::jsmath::js_max(along(a), along(b));
    let top = crate::jsmath::js_max(up(a), up(b));
    let corner = Vec2::new(left * c - top * s, left * s + top * c);
    let width = right - left;
    (corner, (width >= 1e-6).then_some(width))
}

// ── Runs as an editor changes them ──────────────────────────────────────

/// Which format a toggle sets: bold, italic, underline, raised, lowered, or a
/// colour (none: the text's own).
#[derive(Clone, Debug, PartialEq)]
pub enum Toggle {
    Bold,
    Italic,
    Underline,
    Super,
    Sub,
    Color(Option<String>),
}

/// Every letter's format, as runs of one letter.
fn spread(runs: &[Run], len: usize) -> Vec<Run> {
    let mut out = vec![Run::default(); len];
    for r in runs {
        for f in out
            .iter_mut()
            .take((r.end as usize).min(len))
            .skip(r.start as usize)
        {
            *f = r.format();
        }
    }
    out
}

/// Letters' formats as runs: one per stretch of one format, formatless letters left out.
fn gather(formats: &[Run]) -> Vec<Run> {
    let mut out: Vec<Run> = Vec::new();
    for (i, f) in formats.iter().enumerate() {
        if !f.has_format() {
            continue;
        }
        let i = i as u32;
        match out.last_mut() {
            Some(last) if last.end == i && last.same_format(f) => last.end = i + 1,
            _ => out.push(Run {
                start: i,
                end: i + 1,
                ..f.format()
            }),
        }
    }
    out
}

/// The runs written as the file and the commands take them (docs/adr/0182
/// §1): inside `len` letters, in order, apart, each with a format, touching
/// runs of one format joined.
pub fn normalize(runs: &[Run], len: usize) -> Vec<Run> {
    gather(&spread(runs, len))
}

/// The format `t` toggled over the letters `start..end` of a text of `len`
/// letters: when every one of them has it, it goes from all; else all take
/// it. Raised and lowered exclude each other; a colour is set (or cleared,
/// `None`) over the range rather than toggled.
pub fn toggle(runs: &[Run], len: usize, start: usize, end: usize, t: &Toggle) -> Vec<Run> {
    let mut formats = spread(runs, len);
    let (start, end) = (start.min(len), end.min(len));
    if start >= end {
        return gather(&formats);
    }
    let range = &mut formats[start..end];
    let has = |f: &Run| match t {
        Toggle::Bold => f.bold,
        Toggle::Italic => f.italic,
        Toggle::Underline => f.underline,
        Toggle::Super => f.script == Some(Script::Super),
        Toggle::Sub => f.script == Some(Script::Sub),
        Toggle::Color(_) => false,
    };
    let on = !range.iter().all(has);
    for f in range.iter_mut() {
        match t {
            Toggle::Bold => f.bold = on,
            Toggle::Italic => f.italic = on,
            Toggle::Underline => f.underline = on,
            Toggle::Super => f.script = on.then_some(Script::Super),
            Toggle::Sub => f.script = on.then_some(Script::Sub),
            Toggle::Color(c) => f.color.clone_from(c),
        }
    }
    gather(&formats)
}

/// The runs after `before` became `after` in an editor: the letters both
/// begin and end with keep their formats, those taken out lose theirs, and
/// those put in take the format of the letter before them (of the one after
/// them at the start).
pub fn retext(runs: &[Run], before: &str, after: &str) -> Vec<Run> {
    let old: Vec<char> = before.chars().collect();
    let new: Vec<char> = after.chars().collect();
    let formats = spread(runs, old.len());
    let head = old.iter().zip(&new).take_while(|(a, b)| a == b).count();
    let room = old.len().min(new.len()) - head;
    let tail = old
        .iter()
        .rev()
        .zip(new.iter().rev())
        .take(room)
        .take_while(|(a, b)| a == b)
        .count();
    let inserted = new.len() - head - tail;
    let inherit = if head > 0 {
        formats[head - 1].clone()
    } else {
        formats.get(old.len() - tail).cloned().unwrap_or_default()
    };
    let mut out = Vec::with_capacity(new.len());
    out.extend_from_slice(&formats[..head]);
    out.extend(std::iter::repeat_n(inherit.format(), inserted));
    out.extend_from_slice(&formats[old.len() - tail..]);
    gather(&out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bold(start: u32, end: u32) -> Run {
        Run {
            start,
            end,
            bold: true,
            ..Run::default()
        }
    }

    #[test]
    fn a_plain_line_is_as_wide_as_the_one_line_rule() {
        let font = Font::DEFAULT;
        let text = "Parsel 101";
        let laid = lay_out(&Paragraph {
            text,
            runs: &[],
            height: 2.5,
            width_factor: 0.8,
            box_width: None,
            line_spacing: 1.0,
            along: 0.0,
            font,
            bold: false,
        });
        assert_eq!(laid.lines.len(), 1);
        assert_eq!(
            laid.lines[0].width,
            super::super::width_em(text, font) * 2.5 * 0.8
        );
    }

    #[test]
    fn toggling_twice_gives_the_runs_back_and_editing_follows_the_letters() {
        let runs = vec![bold(2, 5)];
        let on = toggle(&runs, 10, 0, 3, &Toggle::Italic);
        assert_eq!(toggle(&on, 10, 0, 3, &Toggle::Italic), runs);
        // A letter typed after a bold one is bold; one taken out of the run shortens it.
        assert_eq!(retext(&runs, "abcdefghij", "abcdXefghij"), vec![bold(2, 6)]);
        assert_eq!(retext(&runs, "abcdefghij", "abdefghij"), vec![bold(2, 4)]);
        assert_eq!(normalize(&[bold(0, 2), bold(2, 3)], 5), vec![bold(0, 3)]);
    }
}

// ── JSON (the objects' `runs`, the operations') ─────────────────────────

impl crate::api::json::FromJson for Script {
    fn from_json(v: &crate::api::json::Json) -> Result<Script, String> {
        match v {
            crate::api::json::Json::Str(s) if s == "super" => Ok(Script::Super),
            crate::api::json::Json::Str(s) if s == "sub" => Ok(Script::Sub),
            _ => Err("“super” ya da “sub” bekleniyordu".into()),
        }
    }
}

impl crate::api::json::ToJson for Script {
    fn write_json(&self, out: &mut String) {
        crate::api::json::write_str(
            out,
            match self {
                Script::Super => "super",
                Script::Sub => "sub",
            },
        );
    }
}

impl crate::api::json::FromJson for Run {
    fn from_json(v: &crate::api::json::Json) -> Result<Run, String> {
        use crate::api::json::read_field;
        let flag = |name: &str| -> Result<bool, String> {
            Ok(read_field::<Option<bool>>(v, name)?.unwrap_or(false))
        };
        Ok(Run {
            start: read_field::<usize>(v, "start")? as u32,
            end: read_field::<usize>(v, "end")? as u32,
            bold: flag("bold")?,
            italic: flag("italic")?,
            underline: flag("underline")?,
            script: read_field(v, "script")?,
            color: read_field(v, "color")?,
        })
    }
}

impl crate::api::json::ToJson for Run {
    fn write_json(&self, out: &mut String) {
        use crate::api::json::field;
        out.push('{');
        let mut first = true;
        field(out, &mut first, "start", &f64::from(self.start));
        field(out, &mut first, "end", &f64::from(self.end));
        for (name, on) in [
            ("bold", self.bold),
            ("italic", self.italic),
            ("underline", self.underline),
        ] {
            if on {
                field(out, &mut first, name, &true);
            }
        }
        field(out, &mut first, "script", &self.script);
        field(out, &mut first, "color", &self.color);
        out.push('}');
    }
}

crate::json_struct!(out Line { start, end, width, x, y });

impl crate::api::json::FromJson for Toggle {
    fn from_json(v: &crate::api::json::Json) -> Result<Toggle, String> {
        use crate::api::json::Json;
        match v {
            Json::Str(s) => match s.as_str() {
                "bold" => Ok(Toggle::Bold),
                "italic" => Ok(Toggle::Italic),
                "underline" => Ok(Toggle::Underline),
                "super" => Ok(Toggle::Super),
                "sub" => Ok(Toggle::Sub),
                _ => Err(format!("“{s}” biçimi bilinmiyor")),
            },
            // A colour, or none for the text's own: `{ "color": "#E5484D" }`, `{ "color": null }`.
            _ => Ok(Toggle::Color(crate::api::json::read_field(v, "color")?)),
        }
    }
}
