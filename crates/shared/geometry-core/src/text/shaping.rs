//! Shared OpenType/bidi glyph placement for the explicit Unicode text profile (ADR 0914).
//! This module does not change legacy text layout. All coordinates are ems,
//! all source ranges refer to the unchanged logical input, and no OS font is read.
use std::ops::Range;
use std::sync::OnceLock;

use crate::jsmath::{js_max, js_min};
use rustybuzz::{BufferClusterLevel, Direction, Feature, UnicodeBuffer};
use unicode_bidi::{BidiInfo, Level};
use unicode_segmentation::UnicodeSegmentation;

use super::font_data::{DRAWING_FACES, DRAWING_KEYS, FALLBACK_FACES, FALLBACK_KEYS};

pub const MAX_BYTES: usize = 65_536;
pub const MAX_GLYPHS: usize = MAX_BYTES * 4;
/// Bound input complexity independently of byte/glyph counts.
pub const MAX_FORMATS: usize = 4096;

/// Index of an exact bundled font file, not a system-dependent family match.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FontId(pub u16);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BaseDirection {
    #[default]
    Auto,
    LeftToRight,
    RightToLeft,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Style<'a> {
    pub family: &'a str,
    pub weight: u16,
    pub italic: bool,
    pub direction: BaseDirection,
    pub language: Option<&'a str>,
    pub kerning: bool,
    pub ligatures: bool,
}
impl Default for Style<'_> {
    fn default() -> Self {
        Self {
            family: "barlow",
            weight: 400,
            italic: false,
            direction: BaseDirection::Auto,
            language: None,
            kerning: true,
            ligatures: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Error {
    TooLong,
    LineBreak,
    UnknownFamily,
    BadFont(FontId),
    BadLanguage,
    Unsupported { bytes: Range<usize> },
    InvalidCluster,
    InvalidRange,
    InvalidFormats,
    TooManyGlyphs,
    TooManyFormats,
}

/// Baseline coordinates, positive y upward. A fallback italic face can carry
/// a synthetic slant; outlines, ink bounds and consumers must apply it alike.
#[derive(Clone, Debug, PartialEq)]
pub struct Glyph {
    pub font: FontId,
    pub id: u16,
    pub bytes: Range<usize>,
    pub scalars: Range<u32>,
    pub bidi_level: u8,
    pub origin: [f64; 2],
    pub advance: f64,
    pub slant: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Line {
    pub requested: FontId,
    pub glyphs: Vec<Glyph>,
    pub advance: f64,
    /// Actual outline union, separate from advance width. None for spaces/empty text.
    pub ink: Option<[f64; 4]>,
}

pub fn font_count() -> usize {
    DRAWING_FACES.len() + FALLBACK_FACES.len()
}
pub fn font_bytes(id: FontId) -> Option<&'static [u8]> {
    let i = usize::from(id.0);
    if i < DRAWING_FACES.len() {
        DRAWING_FACES.get(i).copied()
    } else {
        FALLBACK_FACES.get(i - DRAWING_FACES.len()).copied()
    }
}
pub fn font_key(id: FontId) -> Option<(&'static str, u16, bool)> {
    let i = usize::from(id.0);
    if i < DRAWING_KEYS.len() {
        DRAWING_KEYS.get(i).copied()
    } else {
        FALLBACK_KEYS.get(i - DRAWING_KEYS.len()).copied()
    }
}
fn parsed(id: FontId) -> Result<&'static rustybuzz::Face<'static>, Error> {
    static FACES: OnceLock<Vec<Option<rustybuzz::Face<'static>>>> = OnceLock::new();
    FACES
        .get_or_init(|| {
            (0..font_count())
                .map(|i| {
                    font_bytes(FontId(i as u16))
                        .and_then(|bytes| rustybuzz::Face::from_slice(bytes, 0))
                })
                .collect()
        })
        .get(usize::from(id.0))
        .and_then(Option::as_ref)
        .ok_or(Error::BadFont(id))
}

/// CSS-style slope then weight selection, within one exact family.
pub fn select_font(family: &str, weight: u16, italic: bool) -> Option<FontId> {
    let candidates: Vec<_> = (0..font_count())
        .filter_map(|i| {
            let id = FontId(i as u16);
            let (f, w, s) = font_key(id)?;
            (f == family).then_some((id, w, s))
        })
        .collect();
    let slope = italic && candidates.iter().any(|(_, _, s)| *s);
    let candidates: Vec<_> = candidates
        .into_iter()
        .filter(|(_, _, s)| *s == slope)
        .collect();
    let rank = |w: u16| -> (u8, u16) {
        if w == weight {
            (0, 0)
        } else if (400..=500).contains(&weight) {
            if w > weight && w <= 500 {
                (1, w - weight)
            } else if w < weight {
                (2, weight - w)
            } else {
                (3, w - weight)
            }
        } else if weight < 400 {
            if w < weight {
                (1, weight - w)
            } else {
                (2, w - weight)
            }
        } else if w > weight {
            (1, w - weight)
        } else {
            (2, weight - w)
        }
    };
    candidates
        .into_iter()
        .min_by_key(|(_, w, _)| rank(*w))
        .map(|(id, _, _)| id)
}

fn shape_buffer(
    face: &rustybuzz::Face<'_>,
    text: &str,
    rtl: bool,
    style: &Style<'_>,
) -> Result<rustybuzz::GlyphBuffer, Error> {
    shape_buffer_context(face, text, rtl, style, "", "")
}

fn shape_buffer_context(
    face: &rustybuzz::Face<'_>,
    text: &str,
    rtl: bool,
    style: &Style<'_>,
    before: &str,
    after: &str,
) -> Result<rustybuzz::GlyphBuffer, Error> {
    let mut buffer = UnicodeBuffer::new();
    buffer.push_str(text);
    buffer.set_pre_context(before);
    buffer.set_post_context(after);
    buffer.set_direction(if rtl {
        Direction::RightToLeft
    } else {
        Direction::LeftToRight
    });
    buffer.set_cluster_level(BufferClusterLevel::MonotoneGraphemes);
    if let Some(language) = style.language {
        buffer.set_language(language.parse().map_err(|_| Error::BadLanguage)?);
    }
    buffer.guess_segment_properties();
    let features = [
        Feature::new(
            rustybuzz::ttf_parser::Tag::from_bytes(b"kern"),
            u32::from(style.kerning),
            ..,
        ),
        Feature::new(
            rustybuzz::ttf_parser::Tag::from_bytes(b"liga"),
            u32::from(style.ligatures),
            ..,
        ),
        Feature::new(
            rustybuzz::ttf_parser::Tag::from_bytes(b"clig"),
            u32::from(style.ligatures),
            ..,
        ),
    ];
    Ok(rustybuzz::shape(face, &features, buffer))
}
fn supports(id: FontId, text: &str, rtl: bool, style: &Style<'_>) -> Result<bool, Error> {
    let face = parsed(id)?;
    if text
        .chars()
        .all(|c| face.glyph_index(c).is_some_and(|g| g.0 != 0))
    {
        return Ok(true);
    }
    // HarfBuzz may compose a decomposed sequence into an existing glyph even
    // when the subset has no standalone mark. Probe without normalizing input.
    Ok(shape_buffer(face, text, rtl, style)?
        .glyph_infos()
        .iter()
        .all(|g| g.glyph_id != 0))
}

/// A non-overlapping logical byte range with its complete shaping style.
/// Boundaries must be extended grapheme boundaries; a format cannot detach a mark.
#[derive(Clone, Debug)]
pub struct Format<'a> {
    pub bytes: Range<usize>,
    pub style: Style<'a>,
}

/// Resolve bidi once for the whole logical paragraph, then reorder each actual
/// wrapped line from these levels (UAX #9 L1/L2). Re-detecting direction on each
/// substring loses the context of numbers, brackets and directional isolates.
pub struct Paragraph<'a> {
    text: &'a str,
    bidi: BidiInfo<'a>,
    scalar_at: Vec<u32>,
    boundaries: Vec<usize>,
}

impl<'a> Paragraph<'a> {
    pub fn new(text: &'a str, direction: BaseDirection) -> Result<Self, Error> {
        if text.len() > MAX_BYTES {
            return Err(Error::TooLong);
        }
        if text.contains(['\n', '\r']) {
            return Err(Error::LineBreak);
        }
        let level = match direction {
            BaseDirection::Auto => None,
            BaseDirection::LeftToRight => Some(Level::ltr()),
            BaseDirection::RightToLeft => Some(Level::rtl()),
        };
        let mut scalar_at = vec![0u32; text.len() + 1];
        let mut count = 0;
        for (at, c) in text.char_indices() {
            scalar_at[at] = count;
            count += 1;
            scalar_at[at + c.len_utf8()] = count;
        }
        let mut boundaries: Vec<_> = text.grapheme_indices(true).map(|(at, _)| at).collect();
        boundaries.push(text.len());
        Ok(Self {
            text,
            bidi: BidiInfo::new(text, level),
            scalar_at,
            boundaries,
        })
    }

    /// Glyphs for an actual line between grapheme boundaries. Source ranges
    /// remain relative to the whole paragraph, not the sliced line. The base
    /// direction comes from `new`; per-format directions do not restart bidi.
    pub fn line(
        &self,
        range: Range<usize>,
        style: Style<'_>,
        formats: &[Format<'_>],
    ) -> Result<Line, Error> {
        let text = self.text;
        if range.start > range.end
            || range.end > text.len()
            || self.boundaries.binary_search(&range.start).is_err()
            || self.boundaries.binary_search(&range.end).is_err()
        {
            return Err(Error::InvalidRange);
        }
        if text[range.clone()].contains('\t') {
            return Err(Error::LineBreak);
        }
        if formats.len() > MAX_FORMATS {
            return Err(Error::TooManyFormats);
        }
        let mut previous = 0;
        for f in formats {
            if f.bytes.start < previous
                || f.bytes.start >= f.bytes.end
                || f.bytes.end > text.len()
                || self.boundaries.binary_search(&f.bytes.start).is_err()
                || self.boundaries.binary_search(&f.bytes.end).is_err()
            {
                return Err(Error::InvalidFormats);
            }
            previous = f.bytes.end;
        }
        // Resolve exact faces once per style, not once per grapheme. A dense
        // CAD paragraph must not repeatedly allocate/search the font catalogue.
        let prepared: Vec<_> = std::iter::once(style)
            .chain(formats.iter().map(|f| f.style))
            .map(|s| {
                let requested =
                    select_font(s.family, s.weight, s.italic).ok_or(Error::UnknownFamily)?;
                let fallback = [
                    "noto-sans",
                    "noto-sans-arabic",
                    "noto-sans-hebrew",
                    "noto-sans-math",
                ]
                .map(|f| select_font(f, s.weight, s.italic));
                Ok((s, requested, fallback))
            })
            .collect::<Result<_, Error>>()?;
        let requested = prepared[0].1;
        parsed(requested)?;
        let mut out = Line {
            requested,
            glyphs: Vec::new(),
            advance: 0.,
            ink: None,
        };
        if range.is_empty() {
            return Ok(out);
        }
        let Some(paragraph) = self.bidi.paragraphs.first() else {
            return Ok(out);
        };
        let (levels, runs) = self.bidi.visual_runs(paragraph, range.clone());
        let scalar_at = &self.scalar_at;
        for run in runs {
            let rtl = levels.get(run.start).is_some_and(Level::is_rtl);
            let mut spans: Vec<(Range<usize>, FontId, Style<'_>)> = Vec::new();
            for (relative, cluster) in text[run.clone()].grapheme_indices(true) {
                let start = run.start + relative;
                let end = start + cluster.len();
                let format = formats.partition_point(|f| f.bytes.end <= start);
                let index = if formats
                    .get(format)
                    .is_some_and(|f| f.bytes.contains(&start))
                {
                    format + 1
                } else {
                    0
                };
                let (style, requested, fallback) = prepared[index];
                let mut chosen = None;
                for id in std::iter::once(requested).chain(fallback.into_iter().flatten()) {
                    if supports(id, cluster, rtl, &style)? {
                        chosen = Some(id);
                        break;
                    }
                }
                let Some(id) = chosen else {
                    return Err(Error::Unsupported { bytes: start..end });
                };
                if let Some((range, last, last_style)) = spans.last_mut()
                    && *last == id
                    && *last_style == style
                    && range.end == start
                {
                    range.end = end;
                } else {
                    spans.push((start..end, id, style));
                }
            }
            if rtl {
                spans.reverse();
            }
            for (span, id, style) in spans {
                let face = parsed(id)?;
                // Shape joining across style/font boundaries, but never across a
                // real line break. The output contains only the span's own glyphs.
                let glyphs = shape_buffer_context(
                    face,
                    &text[span.clone()],
                    rtl,
                    &style,
                    &text[range.start..span.start],
                    &text[span.end..range.end],
                )?;
                let range = span;
                let mut boundaries: Vec<_> = glyphs
                    .glyph_infos()
                    .iter()
                    .map(|g| g.cluster as usize)
                    .collect();
                boundaries.push(range.len());
                boundaries.sort_unstable();
                boundaries.dedup();
                let units = f64::from(face.units_per_em());
                let slant = if style.italic && font_key(id).is_some_and(|(_, _, italic)| !italic) {
                    0.25
                } else {
                    0.
                };
                for (info, pos) in glyphs.glyph_infos().iter().zip(glyphs.glyph_positions()) {
                    if info.glyph_id == 0 {
                        return Err(Error::Unsupported {
                            bytes: range.clone(),
                        });
                    }
                    let start = info.cluster as usize;
                    let Some(&end) = boundaries.iter().find(|&&at| at > start) else {
                        return Err(Error::InvalidCluster);
                    };
                    let (a, b) = (range.start + start, range.start + end);
                    if !text.is_char_boundary(a) || !text.is_char_boundary(b) || b > range.end {
                        return Err(Error::InvalidCluster);
                    }
                    let advance = f64::from(pos.x_advance) / units;
                    let origin = [
                        out.advance + f64::from(pos.x_offset) / units,
                        f64::from(pos.y_offset) / units,
                    ];
                    if let Some(box_) = face
                        .glyph_bounding_box(rustybuzz::ttf_parser::GlyphId(info.glyph_id as u16))
                    {
                        for x in [box_.x_min, box_.x_max] {
                            for y in [box_.y_min, box_.y_max] {
                                let (x, y) = (f64::from(x) / units, f64::from(y) / units);
                                let (x, y) = (origin[0] + x + y * slant, origin[1] + y);
                                match &mut out.ink {
                                    Some(b) => {
                                        b[0] = js_min(b[0], x);
                                        b[1] = js_min(b[1], y);
                                        b[2] = js_max(b[2], x);
                                        b[3] = js_max(b[3], y);
                                    }
                                    None => out.ink = Some([x, y, x, y]),
                                }
                            }
                        }
                    }
                    out.glyphs.push(Glyph {
                        font: id,
                        id: info.glyph_id as u16,
                        bytes: a..b,
                        scalars: scalar_at[a]..scalar_at[b],
                        bidi_level: levels[run.start].number(),
                        origin,
                        advance,
                        slant,
                    });
                    out.advance += advance;
                    if out.glyphs.len() > MAX_GLYPHS {
                        return Err(Error::TooManyGlyphs);
                    }
                }
            }
        }
        Ok(out)
    }
}

/// One logical line in visual glyph order. Wrapping and tabs belong to the
/// caller; shaping never reverses or rewrites the stored string.
pub fn line(text: &str, style: Style<'_>) -> Result<Line, Error> {
    if text.contains('\t') {
        return Err(Error::LineBreak);
    }
    Paragraph::new(text, style.direction)?.line(0..text.len(), style, &[])
}
