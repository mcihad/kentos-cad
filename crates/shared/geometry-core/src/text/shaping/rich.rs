//! Rich MTEXT runs over one measured glyph plan. Stored indices stay Unicode
//! scalars; all conversions to UTF-8 occur once, before shaping.
use std::ops::Range;
use unicode_segmentation::UnicodeSegmentation;

use super::{Atom, Error, Format, Glyph, Line, Shift, Style, paragraph};
use crate::{
    jsmath::{js_max, js_min},
    text::paragraph::{Run, SCRIPT_SIZE, STACK_BAR, STACK_OVER, STACK_UNDER, Script},
};

/// A filled decoration in the piece's em coordinates, positive y up. Its
/// colour follows its original run; no renderer estimates its width again.
#[derive(Clone, Debug, PartialEq)]
pub struct Rule {
    pub piece: usize,
    pub scalars: Range<u32>,
    pub rect: [f64; 4],
    pub run: usize,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Layout {
    pub flow: paragraph::Layout,
    pub rules: Vec<Rule>,
}
struct Stack {
    tag: usize,
    atom: Atom,
    glyphs: Vec<Glyph>,
    rules: Vec<[f64; 4]>,
}

fn style_of<'a>(base: Style<'a>, run: &Run) -> Style<'a> {
    Style {
        weight: if run.bold {
            base.weight.max(600)
        } else {
            base.weight
        },
        italic: base.italic || run.italic,
        shift: match run.script {
            Some(Script::Super) => Shift::Super,
            Some(Script::Sub) => Shift::Sub,
            _ => Shift::Baseline,
        },
        ..base
    }
}
fn union(a: Option<[f64; 4]>, b: [f64; 4]) -> Option<[f64; 4]> {
    Some(match a {
        None => b,
        Some(a) => [
            js_min(a[0], b[0]),
            js_min(a[1], b[1]),
            js_max(a[2], b[2]),
            js_max(a[3], b[3]),
        ],
    })
}
fn moved(mut line: Line, scale: f64, at: [f64; 2], byte: usize, scalar: u32) -> Line {
    for g in &mut line.glyphs {
        g.origin = [at[0] + g.origin[0] * scale, at[1] + g.origin[1] * scale];
        g.pen = at[0] + g.pen * scale;
        g.size *= scale;
        g.advance *= scale;
        g.bytes.start += byte;
        g.bytes.end += byte;
        g.scalars.start += scalar;
        g.scalars.end += scalar;
    }
    line.advance *= scale;
    line.ink = line.ink.map(|b| {
        [
            at[0] + b[0] * scale,
            at[1] + b[1] * scale,
            at[0] + b[2] * scale,
            at[1] + b[3] * scale,
        ]
    });
    line
}
fn stack(
    text: &str,
    run: &Run,
    bytes: Range<usize>,
    tag: usize,
    style: Style<'_>,
) -> Result<Stack, Error> {
    let body = &text[bytes.clone()];
    let Some((mark, ch)) = body
        .char_indices()
        .find(|(_, c)| matches!(c, '/' | '#' | '^'))
    else {
        return Err(Error::InvalidStack { bytes });
    };
    let start = bytes.start;
    let q = start + mark;
    let plain = Style {
        shift: Shift::Baseline,
        ..style
    };
    let upper = super::line(&text[start..q], plain)?;
    let lower = super::line(&text[q + 1..bytes.end], plain)?;
    let (num, den) = (upper.advance * SCRIPT_SIZE, lower.advance * SCRIPT_SIZE);
    let mut parts = Vec::new();
    let mut rules = Vec::new();
    let width;
    let lower_scalar = text[..q + 1].chars().count() as u32;
    if ch == '#' {
        let slash = super::line("/", plain)?;
        let gap = slash.advance * SCRIPT_SIZE;
        parts.push(moved(
            upper,
            SCRIPT_SIZE,
            [0., STACK_OVER],
            start,
            run.start,
        ));
        parts.push(moved(slash, SCRIPT_SIZE, [num, 0.], q, lower_scalar - 1));
        parts.push(moved(
            lower,
            SCRIPT_SIZE,
            [num + gap, STACK_UNDER],
            q + 1,
            lower_scalar,
        ));
        width = num + gap + den;
    } else {
        width = js_max(num, den);
        parts.push(moved(
            upper,
            SCRIPT_SIZE,
            [(width - num) / 2., STACK_OVER],
            start,
            run.start,
        ));
        parts.push(moved(
            lower,
            SCRIPT_SIZE,
            [(width - den) / 2., STACK_UNDER],
            q + 1,
            lower_scalar,
        ));
        if ch == '/' {
            rules.push([0., STACK_BAR - 0.06 * SCRIPT_SIZE, width, STACK_BAR]);
        }
    }
    let mut ink = None;
    let mut glyphs = Vec::new();
    for part in parts {
        if let Some(b) = part.ink {
            ink = union(ink, b);
        }
        glyphs.extend(part.glyphs);
    }
    for &b in &rules {
        ink = union(ink, b);
    }
    if run.underline && width > 0. {
        let top = ink.map_or(-0.25, |b| b[1]) - 0.12 * SCRIPT_SIZE;
        let b = [0., top - 0.06 * SCRIPT_SIZE, width, top];
        rules.push(b);
        ink = union(ink, b);
    }
    Ok(Stack {
        tag,
        atom: Atom {
            tag,
            bytes,
            advance: width,
            ink,
        },
        glyphs,
        rules,
    })
}

/// Font, baseline shift and fraction geometry share one flow plan. A paint
/// boundary through a ligature needs component clipping: until that renderer
/// path is available it is reported, never painted as the wrong source colour.
pub fn layout(
    text: &str,
    runs: &[Run],
    style: Style<'_>,
    settings: paragraph::Settings<'_>,
) -> Result<Layout, Error> {
    if text.len() > super::MAX_BYTES {
        return Err(Error::TooLong);
    }
    if runs.len() > super::MAX_FORMATS {
        return Err(Error::TooManyFormats);
    }
    if style.shift != Shift::Baseline {
        return Err(Error::InvalidFormats);
    }
    let offsets: Vec<_> = text
        .char_indices()
        .map(|(at, _)| at)
        .chain(std::iter::once(text.len()))
        .collect();
    let boundaries: Vec<_> = text
        .grapheme_indices(true)
        .map(|(at, _)| at)
        .chain(std::iter::once(text.len()))
        .collect();
    let mut formats = Vec::new();
    let mut stacks = Vec::new();
    let mut previous = 0;
    for (index, run) in runs.iter().enumerate() {
        let (a, b) = (run.start as usize, run.end as usize);
        if a < previous
            || a >= b
            || b >= offsets.len()
            || boundaries.binary_search(&offsets[a]).is_err()
            || boundaries.binary_search(&offsets[b]).is_err()
        {
            return Err(Error::InvalidFormats);
        }
        previous = b;
        let bytes = offsets[a]..offsets[b];
        let own = style_of(style, run);
        if run.script == Some(Script::Stack) {
            stacks.push(stack(text, run, bytes.clone(), index, own)?);
        }
        formats.push(Format { bytes, style: own });
    }
    let atoms: Vec<_> = stacks.iter().map(|s| s.atom.clone()).collect();
    let mut flow = paragraph::layout_with_atoms(text, style, &formats, &atoms, settings)?;
    let mut rules = Vec::new();
    for (piece_index, piece) in flow.pieces.iter_mut().enumerate() {
        let rule_start = rules.len();
        for object in &piece.line.objects {
            let stack = stacks
                .iter()
                .find(|s| s.tag == object.tag)
                .ok_or(Error::InvalidAtoms)?;
            for glyph in &stack.glyphs {
                let mut glyph = glyph.clone();
                glyph.origin[0] += object.origin[0];
                glyph.origin[1] += object.origin[1];
                glyph.pen += object.origin[0];
                piece.line.glyphs.push(glyph);
            }
            for b in &stack.rules {
                rules.push(Rule {
                    piece: piece_index,
                    scalars: runs[stack.tag].start..runs[stack.tag].end,
                    rect: [
                        b[0] + object.origin[0],
                        b[1] + object.origin[1],
                        b[2] + object.origin[0],
                        b[3] + object.origin[1],
                    ],
                    run: stack.tag,
                });
            }
        }
        piece.line.objects.clear();
        piece.line.glyphs.sort_by(|a, b| a.pen.total_cmp(&b.pen));
        let look = |at: u32| {
            let i = runs.partition_point(|r| r.end <= at);
            runs.get(i).filter(|r| r.start <= at).map(|r| (i, r))
        };
        let mut underline: Option<(usize, f64, f64)> = None;
        let mut finish = |group: Option<(usize, f64, f64)>| {
            if let Some((index, left, right)) = group
                && right > left
            {
                let r = &runs[index];
                let s = style_of(style, r);
                let top = s.shift.lift() - 0.12 * s.shift.scale();
                rules.push(Rule {
                    piece: piece_index,
                    scalars: r.start..r.end,
                    rect: [left, top - 0.06 * s.shift.scale(), right, top],
                    run: index,
                });
            }
        };
        for glyph in &piece.line.glyphs {
            let first = look(glyph.scalars.start);
            let paint =
                |at| look(at).map_or((None, false), |(_, r)| (r.color.as_deref(), r.underline));
            let wanted = paint(glyph.scalars.start);
            let begin = runs.partition_point(|r| r.end <= glyph.scalars.start);
            if runs[begin..]
                .iter()
                .take_while(|r| r.start < glyph.scalars.end)
                .any(|r| {
                    (r.start >= glyph.scalars.start && paint(r.start) != wanted)
                        || (r.end < glyph.scalars.end && paint(r.end) != wanted)
                })
            {
                return Err(Error::MixedClusterPaint {
                    bytes: glyph.bytes.clone(),
                });
            }
            let under = first
                .filter(|(_, r)| r.underline && r.script != Some(Script::Stack))
                .map(|(i, _)| i);
            match (underline.as_mut(), under) {
                (Some((index, left, right)), Some(next)) if *index == next => {
                    *left = js_min(*left, glyph.pen);
                    *right = js_max(*right, glyph.pen + glyph.advance);
                }
                (_, next) => {
                    finish(underline.take());
                    underline = next.map(|i| (i, glyph.pen, glyph.pen + glyph.advance));
                }
            }
        }
        finish(underline.take());
        for rule in &rules[rule_start..] {
            piece.line.ink = union(piece.line.ink, rule.rect);
        }
    }
    flow.ink = None;
    for piece in &flow.pieces {
        if let Some(b) = piece.line.ink {
            let (x, y) = (settings.height * settings.width_factor, settings.height);
            flow.ink = union(
                flow.ink,
                [
                    piece.at[0] + b[0] * x,
                    piece.at[1] + b[1] * y,
                    piece.at[0] + b[2] * x,
                    piece.at[1] + b[3] * y,
                ],
            );
        }
    }
    if flow.ink.is_some_and(|b| b.iter().any(|v| !v.is_finite())) {
        return Err(Error::InvalidGeometry);
    }
    Ok(Layout { flow, rules })
}
