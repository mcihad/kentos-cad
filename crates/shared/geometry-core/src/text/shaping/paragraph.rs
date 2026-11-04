//! Physical paragraph flow over the shared glyph layout. Legacy paragraphs do
//! not call this module. Every returned glyph keeps the original scalar/byte
//! range; wrapping never reshapes a substring with a new inferred direction.
use std::ops::Range;
use unicode_segmentation::UnicodeSegmentation;

use super::{Atom, Error, Format, Line, Paragraph, Style};
use crate::jsmath::{js_max, js_min};
use crate::text::paragraph::{DEFAULT_TAB, Flow, PITCH};

/// Geometry in model units; em size retains the existing CAD height meaning.
#[derive(Clone, Copy, Debug)]
pub struct Settings<'a> {
    pub height: f64,
    pub width_factor: f64,
    pub box_width: Option<f64>,
    pub line_spacing: f64,
    pub along: f64,
    pub flow: Option<&'a Flow>,
}

/// One actual line or a tab-separated piece of it. Glyph origins/ink remain
/// in ems; `at` is its baseline in model units, positive y up. The one affine
/// transform is (height * width_factor, height), followed by text rotation.
#[derive(Clone, Debug, PartialEq)]
pub struct Piece {
    pub bytes: Range<usize>,
    pub scalars: Range<u32>,
    pub at: [f64; 2],
    pub line: Line,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Layout {
    pub pieces: Vec<Piece>,
    pub width: f64,
    pub pitch: f64,
    pub depth: usize,
    /// Actual contours' union in model units, separate from the paragraph box.
    pub ink: Option<[f64; 4]>,
}

struct Row {
    pieces: Vec<Piece>,
    left: f64,
    width: f64,
}

/// Reject invalid geometry instead of producing NaN boxes or silently choosing
/// a different metric profile. The public command's contract applies the same
/// CAD ranges before it stores a paragraph.
fn valid(s: &Settings<'_>) -> bool {
    s.height.is_finite()
        && s.height > 0.
        && s.width_factor.is_finite()
        && s.width_factor > 0.
        && s.box_width.is_none_or(|w| w.is_finite() && w > 0.)
        && s.line_spacing.is_finite()
        && s.line_spacing > 0.
        && s.along.is_finite()
        && (0. ..=1.).contains(&s.along)
        && s.flow.is_none_or(|f| {
            f.first.is_finite()
                && f.left.is_finite()
                && f.left >= 0.
                && f.left + f.first >= 0.
                && f.tabs
                    .iter()
                    .enumerate()
                    .all(|(i, t)| t.is_finite() && *t > 0. && (i == 0 || *t > f.tabs[i - 1]))
                && f.columns.is_none_or(|c| {
                    c.count >= 2
                        && c.count <= 16
                        && c.gap.is_finite()
                        && c.gap >= 0.
                        && c.height.is_finite()
                        && c.height > 0.
                        && s.box_width.is_some()
                })
        })
}

/// A paragraph with the existing MTEXT word/indent/tab/column policy, measured
/// and emitted from identical shaped lines. Formats are logical UTF-8 ranges
/// across the full input (which may contain several newline paragraphs).
/// Superscript/fractions are represented by a higher-level rich-run adapter;
/// this primitive accepts font styles, not scalar advance approximations.
pub fn layout(
    text: &str,
    style: Style<'_>,
    formats: &[Format<'_>],
    s: Settings<'_>,
) -> Result<Layout, Error> {
    layout_with_atoms(text, style, formats, &[], s)
}

/// The same paragraph flow with atomic inline objects. Spaces inside an
/// object's original source do not become wrap/trim opportunities.
pub fn layout_with_atoms(
    text: &str,
    style: Style<'_>,
    formats: &[Format<'_>],
    atoms: &[Atom],
    s: Settings<'_>,
) -> Result<Layout, Error> {
    if !valid(&s) {
        return Err(Error::InvalidGeometry);
    }
    if text.len() > super::MAX_BYTES {
        return Err(Error::TooLong);
    }
    if formats.len() > super::MAX_FORMATS {
        return Err(Error::TooManyFormats);
    }
    // Validate ranges against the full source before rebasing individual
    // paragraphs. In particular an out-of-range format cannot disappear.
    let boundaries: Vec<_> =
        unicode_segmentation::UnicodeSegmentation::grapheme_indices(text, true)
            .map(|(at, _)| at)
            .chain(std::iter::once(text.len()))
            .collect();
    super::validate_atoms(text, atoms, &boundaries)?;
    let atom_interior = |at| {
        let i = atoms.partition_point(|a| a.bytes.end <= at);
        atoms.get(i).is_some_and(|a| a.bytes.start < at)
    };
    let mut previous = 0;
    for f in formats {
        if f.bytes.start < previous
            || f.bytes.start >= f.bytes.end
            || f.bytes.end > text.len()
            || boundaries.binary_search(&f.bytes.start).is_err()
            || boundaries.binary_search(&f.bytes.end).is_err()
            || atom_interior(f.bytes.start)
            || atom_interior(f.bytes.end)
        {
            return Err(Error::InvalidFormats);
        }
        if super::select_font(f.style.family, f.style.weight, f.style.italic).is_none() {
            return Err(Error::UnknownFamily);
        }
        previous = f.bytes.end;
    }
    let empty = Flow::default();
    let flow = s.flow.unwrap_or(&empty);
    let xscale = s.height * s.width_factor;
    if !xscale.is_finite() || xscale <= 0. {
        return Err(Error::InvalidGeometry);
    }
    let first = (flow.left + flow.first) * s.height;
    let left = flow.left * s.height;
    let stop = |at: f64| {
        let next = match flow.tabs.iter().find(|&&t| t * s.height > at) {
            Some(t) => *t * s.height,
            None => {
                let last = flow.tabs.last().copied().unwrap_or(0.) * s.height;
                let every = DEFAULT_TAB * s.height;
                last + (((at - last) / every).floor() + 1.) * every
            }
        };
        if left > at && left < next { left } else { next }
    };
    let pitch = PITCH * s.height * s.line_spacing;
    if !first.is_finite()
        || !left.is_finite()
        || !pitch.is_finite()
        || pitch <= 0.
        || flow.tabs.iter().any(|t| !(*t * s.height).is_finite())
    {
        return Err(Error::InvalidGeometry);
    }
    // CR, LF and CRLF end paragraphs. Keep both CRLF scalars in source
    // indexing, and retain an empty final paragraph after a final separator.
    let mut ranges = Vec::new();
    let (mut start, mut scalar, mut start_scalar) = (0, 0, 0);
    let mut chars = text.char_indices().peekable();
    while let Some((at, c)) = chars.next() {
        scalar += 1;
        if c == '\r' || c == '\n' {
            ranges.push((start..at, start_scalar));
            let mut end = at + c.len_utf8();
            if c == '\r' && chars.peek().is_some_and(|(_, c)| *c == '\n') {
                let (at, _) = chars.next().ok_or(Error::InvalidRange)?;
                end = at + 1;
                scalar += 1;
            }
            start = end;
            start_scalar = scalar;
        }
    }
    ranges.push((start..text.len(), start_scalar));
    let mut rows = Vec::new();
    for (range, scalar_offset) in ranges {
        let offset = range.start;
        let body = &text[range];
        let scalar_bytes: Vec<_> = body
            .char_indices()
            .map(|(at, _)| at)
            .chain(std::iter::once(body.len()))
            .collect();
        let atoms: Vec<_> = atoms
            .iter()
            .filter(|a| a.bytes.start >= offset && a.bytes.end <= offset + body.len())
            .map(|a| Atom {
                bytes: a.bytes.start - offset..a.bytes.end - offset,
                ..a.clone()
            })
            .collect();
        let paragraph = Paragraph::with_atoms(body, style.direction, &atoms)?;
        let atom_at = |at| {
            let i = atoms.partition_point(|a| a.bytes.end <= at);
            atoms.get(i).is_some_and(|a| a.bytes.contains(&at))
        };
        let formats: Vec<_> = formats
            .iter()
            .filter_map(|f| {
                let a = f.bytes.start.max(offset);
                let b = f.bytes.end.min(offset + body.len());
                (a < b).then(|| Format {
                    bytes: a - offset..b - offset,
                    style: f.style,
                })
            })
            .collect();
        let measure = |from: usize, to: usize, indent: f64| -> Result<Row, Error> {
            let mut pieces = Vec::new();
            let mut at = indent;
            let mut start = from;
            let mut trimmed = to;
            for cluster in body[from..to].graphemes(true).rev() {
                if cluster != " " || atom_at(trimmed - cluster.len()) {
                    break;
                }
                trimmed -= cluster.len();
            }
            for end in body[from..trimmed]
                .match_indices('\t')
                .map(|(i, _)| from + i)
                .chain(std::iter::once(trimmed))
            {
                let a = formats.partition_point(|f| f.bytes.end <= start);
                let b = formats.partition_point(|f| f.bytes.start < end);
                let mut line = paragraph.line(start..end, style, &formats[a..b.max(a)])?;
                let width = line.advance * xscale;
                for object in &mut line.objects {
                    object.bytes.start += offset;
                    object.bytes.end += offset;
                    object.scalars.start += scalar_offset;
                    object.scalars.end += scalar_offset;
                }
                for glyph in &mut line.glyphs {
                    glyph.bytes.start += offset;
                    glyph.bytes.end += offset;
                    glyph.scalars.start += scalar_offset;
                    glyph.scalars.end += scalar_offset;
                }
                pieces.push(Piece {
                    bytes: offset + start..offset + end,
                    scalars: scalar_offset
                        + scalar_bytes
                            .binary_search(&start)
                            .map_err(|_| Error::InvalidRange)? as u32
                        ..scalar_offset
                            + scalar_bytes
                                .binary_search(&end)
                                .map_err(|_| Error::InvalidRange)?
                                as u32,
                    at: [at, 0.],
                    line,
                });
                at += width;
                if end < trimmed {
                    at = stop(at);
                }
                start = end + 1;
            }
            Ok(Row {
                pieces,
                left: indent,
                width: at - indent,
            })
        };
        let Some(width) = s.box_width else {
            rows.push(measure(0, body.len(), first)?);
            continue;
        };
        // Candidate boundaries are word ends, matching the existing MTEXT
        // policy. Exponential then binary search avoids reshaping successively
        // larger prefixes of a long line, or half the remaining document for
        // every short line. Whitespace separates shaping/joining contexts.
        let mut words = Vec::new();
        let mut word_start = None;
        for (at, cluster) in body.grapheme_indices(true) {
            if cluster == " " && !atom_at(at) {
                if let Some(begin) = word_start.take() {
                    words.push((begin, at));
                }
            } else if word_start.is_none() {
                word_start = Some(at);
            }
        }
        if let Some(begin) = word_start {
            words.push((begin, body.len()));
        }
        if words.is_empty() {
            rows.push(measure(0, body.len(), first)?);
        }
        let (mut word, mut from, mut indent) = (0, 0, first);
        while word < words.len() {
            let mut end = word;
            let mut best = measure(from, words[end].1, indent)?;
            // The first word stays whole even when wider than its box.
            if indent + best.width <= width {
                let mut step = 1usize;
                let mut upper = words.len();
                loop {
                    let next = end.saturating_add(step).min(words.len() - 1);
                    if next == end {
                        break;
                    }
                    let candidate = measure(from, words[next].1, indent)?;
                    if indent + candidate.width > width {
                        upper = next;
                        break;
                    }
                    best = candidate;
                    end = next;
                    step = step.saturating_mul(2);
                }
                let mut lower = end + 1;
                while lower < upper {
                    let mid = lower + (upper - lower) / 2;
                    let candidate = measure(from, words[mid].1, indent)?;
                    if indent + candidate.width <= width {
                        best = candidate;
                        end = mid;
                        lower = mid + 1;
                    } else {
                        upper = mid;
                    }
                }
            }
            rows.push(best);
            from = words.get(end + 1).map_or(body.len(), |w| w.0);
            word = end + 1;
            indent = left;
        }
    }
    let column_width = s
        .box_width
        .unwrap_or_else(|| rows.iter().map(|r| r.left + r.width).fold(0., js_max));
    let (per, count, step) = match flow.columns {
        Some(c) => (
            ((c.height / pitch).floor() as usize).max(1),
            c.count as usize,
            column_width + c.gap * s.height,
        ),
        None => (usize::MAX, 1, 0.),
    };
    let mut out = Layout {
        pieces: Vec::new(),
        width: column_width * count as f64
            + count.saturating_sub(1) as f64 * flow.columns.map_or(0., |c| c.gap * s.height),
        pitch,
        depth: 0,
        ink: None,
    };
    for (index, row) in rows.into_iter().enumerate() {
        let col = (index / per).min(count - 1);
        let row_number = index - col * per;
        out.depth = out.depth.max(row_number + 1);
        let shift = s.along * (column_width - row.left - row.width) + col as f64 * step;
        for mut p in row.pieces {
            p.at[0] += shift;
            p.at[1] = -(row_number as f64) * pitch;
            if let Some(b) = p.line.ink {
                let b = [
                    p.at[0] + b[0] * xscale,
                    p.at[1] + b[1] * s.height,
                    p.at[0] + b[2] * xscale,
                    p.at[1] + b[3] * s.height,
                ];
                out.ink = Some(match out.ink {
                    None => b,
                    Some(a) => [
                        js_min(a[0], b[0]),
                        js_min(a[1], b[1]),
                        js_max(a[2], b[2]),
                        js_max(a[3], b[3]),
                    ],
                });
            }
            out.pieces.push(p);
        }
    }
    if !out.width.is_finite()
        || out.ink.is_some_and(|b| b.iter().any(|v| !v.is_finite()))
        || !pitch.is_finite()
        || out
            .pieces
            .iter()
            .any(|p| p.at.iter().any(|x| !x.is_finite()))
    {
        return Err(Error::InvalidGeometry);
    }
    Ok(out)
}
