//! Shared OpenType/bidi glyph placement for the explicit Unicode text profile (ADR 0914).
//! This module does not change legacy text layout. All coordinates are ems,
//! all source ranges refer to the unchanged logical input, and no OS font is read.
pub mod paragraph;
pub mod rich;
pub mod placed;
mod scripts;

use std::ops::Range;
use std::sync::OnceLock;

use crate::jsmath::{js_max, js_min};
use rustybuzz::{BufferClusterLevel, Direction, Feature, UnicodeBuffer};
use unicode_bidi::{BidiClass, BidiInfo, Level, ParagraphInfo, bidi_class};
use unicode_script::Script as WritingScript;
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

/// Baseline shift is part of glyph layout, so measurement, hit boxes and
/// drawing share the same size/offset rather than estimating scalar advances.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Shift {
    #[default]
    Baseline,
    Super,
    Sub,
}
impl Shift {
    pub fn scale(self) -> f64 {
        match self {
            Self::Baseline => 1.,
            Self::Super | Self::Sub => super::paragraph::SCRIPT_SIZE,
        }
    }
    pub fn lift(self) -> f64 {
        match self {
            Self::Baseline => 0.,
            Self::Super => 0.4,
            Self::Sub => -0.15,
        }
    }
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
    pub shift: Shift,
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
            shift: Shift::Baseline,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Error {
    TooLong,
    LineBreak,
    UnknownFamily,
    BadFont(FontId),
    BadGlyph { font: FontId, id: u16 },
    BadLanguage,
    Unsupported { bytes: Range<usize> },
    InvalidCluster,
    InvalidRange,
    InvalidFormats,
    InvalidGeometry,
    InvalidAtoms,
    InvalidStack { bytes: Range<usize> },
    MixedClusterPaint { bytes: Range<usize> },
    TooManyAtoms,
    TooManyGlyphs,
    TooManyContours,
    TooManyFormats,
}

impl Error {
    /// User-facing explanation; byte ranges remain diagnostic metadata only.
    pub fn message(&self,text:&str)->String {
        match self {
            Self::Unsupported{bytes}=> {
                let sample=text.get(bytes.clone()).unwrap_or("");
                let codes=sample.chars().take(16).map(|c|format!("U+{:04X}",c as u32)).collect::<Vec<_>>().join(", ");
                format!("“{}” karakter kümesi paketli yazı tiplerinde bulunamadı ({codes}).",sample.chars().take(24).collect::<String>())
            }
            Self::MixedClusterPaint{..}=>"Birleşik harfin veya ligatürün içinde farklı renk/alt çizgi sınırı var. Biçimi görünür harfin tamamına uygulayın.".into(),
            Self::InvalidFormats=>"Yazı biçimlerinin aralıkları görünür karakter kümelerinin sınırlarında, sıralı ve ayrı olmalı.".into(),
            Self::LineBreak=>"Bu yerleşimdeki satır/tab karakteri desteklenmiyor. Eğri üzerindeki yazıyı tek satır olarak düzenleyin.".into(),
            Self::InvalidGeometry=>"Yazının yükseklik, genişlik, aralık veya konum değerleri geçerli aralıkta değil.".into(),
            Self::InvalidStack{..}=>"Kesir biçimi için /, # veya ^ ayıracı ve geçerli bir karakter aralığı gerekli.".into(),
            Self::TooLong=>"Tek yazının ortak yerleşimi en çok 65.536 UTF-8 baytı destekler. Metni ayrı yazılara bölün.".into(),
            Self::TooManyContours=>"Tek yazının çizim karmaşıklığı sınırı aşıldı. Metni ayrı yazılara bölün; kısmi metin çizilmez.".into(),
            Self::TooManyGlyphs|Self::TooManyFormats|Self::TooManyAtoms=>"Tek yazının harf veya biçim sayısı sınırı aşıldı. Metni ayrı yazılara bölün.".into(),
            Self::UnknownFamily|Self::BadFont(_)|Self::BadGlyph{..}=>"Paketli yazı tipi bilgisiyle harf eşleşmiyor. Yazı tipi paketini doğrulayın; eksik harf basılamaz.".into(),
            Self::InvalidRange|Self::InvalidCluster|Self::InvalidAtoms=>"Yazı aralığı görünür karakter kümesini veya kesri bölüyor. Aralığı tam karakter sınırına getirin.".into(),
            Self::BadLanguage=>"Yazının dil bilgisi geçerli bir dil etiketi olmalı.".into(),
        }
    }
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
    /// Baseline pen before this glyph, distinct from its OpenType x offset.
    pub pen: f64,
    /// This glyph's em size relative to the containing text's height.
    pub size: f64,
    pub advance: f64,
    pub slant: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Line {
    pub requested: FontId,
    pub glyphs: Vec<Glyph>,
    pub objects: Vec<PlacedAtom>,
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
    shape_buffer_context(face, text, rtl, style, "", "", None)
}

fn shape_buffer_context(
    face: &rustybuzz::Face<'_>,
    text: &str,
    rtl: bool,
    style: &Style<'_>,
    before: &str,
    after: &str,
    script: Option<WritingScript>,
) -> Result<rustybuzz::GlyphBuffer, Error> {
    let mut buffer = UnicodeBuffer::new();
    buffer.push_str(text);
    buffer.set_pre_context(before);
    buffer.set_post_context(after);
    if let Some(script) = script
        .filter(|s| {
            !matches!(
                s,
                WritingScript::Common | WritingScript::Inherited | WritingScript::Unknown
            )
        })
        .and_then(|s| {
            rustybuzz::Script::from_iso15924_tag(rustybuzz::ttf_parser::Tag(s.as_iso15924_tag()))
        })
    {
        buffer.set_script(script);
    }
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

/// An atomic inline object (for example a stacked fraction). It resolves as
/// U+FFFC for bidi but its stored source/ranges are never rewritten. Its caller
/// supplies the actual measured advance/ink in ems and renders its contents.
#[derive(Clone, Debug, PartialEq)]
pub struct Atom {
    pub tag: usize,
    pub bytes: Range<usize>,
    pub advance: f64,
    pub ink: Option<[f64; 4]>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct PlacedAtom {
    pub tag: usize,
    pub bytes: Range<usize>,
    pub scalars: Range<u32>,
    pub origin: [f64; 2],
    pub advance: f64,
}

pub(super) fn validate_atoms(
    text: &str,
    atoms: &[Atom],
    boundaries: &[usize],
) -> Result<(), Error> {
    if atoms.len() > MAX_FORMATS {
        return Err(Error::TooManyAtoms);
    }
    let mut previous = 0;
    for a in atoms {
        if a.bytes.start < previous
            || a.bytes.start >= a.bytes.end
            || a.bytes.end > text.len()
            || boundaries.binary_search(&a.bytes.start).is_err()
            || boundaries.binary_search(&a.bytes.end).is_err()
            || !a.advance.is_finite()
            || a.advance < 0.
            || a.ink
                .is_some_and(|b| b.iter().any(|x| !x.is_finite()) || b[0] > b[2] || b[1] > b[3])
        {
            return Err(Error::InvalidAtoms);
        }
        if text[a.bytes.clone()]
            .chars()
            .any(|c| c == '\t' || c == '\u{2028}' || bidi_class(c) == BidiClass::B)
        {
            return Err(Error::InvalidAtoms);
        }
        previous = a.bytes.end;
    }
    Ok(())
}

/// Resolve bidi once for the whole logical paragraph, then reorder each actual
/// wrapped line from these levels (UAX #9 L1/L2). Re-detecting direction on each
/// substring loses the context of numbers, brackets and directional isolates.
pub struct Paragraph<'a> {
    text: &'a str,
    bidi: BidiInfo<'a>,
    scalar_at: Vec<u32>,
    boundaries: Vec<usize>,
    atoms: Vec<Atom>,
    scripts: Vec<WritingScript>,
}

impl<'a> Paragraph<'a> {
    pub fn new(text: &'a str, direction: BaseDirection) -> Result<Self, Error> {
        Self::with_atoms(text, direction, &[])
    }

    pub fn with_atoms(
        text: &'a str,
        direction: BaseDirection,
        atoms: &[Atom],
    ) -> Result<Self, Error> {
        if text.len() > MAX_BYTES {
            return Err(Error::TooLong);
        }
        if text
            .chars()
            .any(|c| c == '\u{2028}' || bidi_class(c) == BidiClass::B)
        {
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
        validate_atoms(text, atoms, &boundaries)?;
        let (bidi, scripts) = if atoms.is_empty() {
            (BidiInfo::new(text, level), scripts::resolve(text))
        } else {
            let mut shadow = String::new();
            let mut map = vec![0; text.len()];
            let mut from = 0;
            for atom in atoms {
                let at = shadow.len();
                shadow.push_str(&text[from..atom.bytes.start]);
                for (i, value) in map[from..atom.bytes.start].iter_mut().enumerate() {
                    *value = at + i;
                }
                let at = shadow.len();
                shadow.push('\u{fffc}');
                map[atom.bytes.clone()].fill(at);
                from = atom.bytes.end;
            }
            let at = shadow.len();
            shadow.push_str(&text[from..]);
            for (i, value) in map[from..].iter_mut().enumerate() {
                *value = at + i;
            }
            let resolved = BidiInfo::new(&shadow, level);
            let paragraph = resolved.paragraphs.first().ok_or(Error::InvalidAtoms)?;
            // Keep resolved object levels/classes on every original source byte.
            // The library's L1/L2 then uses the original line/source indices.
            let resolved_scripts = scripts::resolve(&shadow);
            let scripts = map.iter().map(|&i| resolved_scripts[i]).collect();
            (
                BidiInfo {
                    text,
                    levels: map.iter().map(|&i| resolved.levels[i]).collect(),
                    original_classes: map.iter().map(|&i| resolved.original_classes[i]).collect(),
                    paragraphs: vec![ParagraphInfo {
                        range: 0..text.len(),
                        level: paragraph.level,
                    }],
                },
                scripts,
            )
        };
        Ok(Self {
            text,
            bidi,
            scalar_at,
            boundaries,
            atoms: atoms.to_vec(),
            scripts,
        })
    }

    fn atom_interior(&self, at: usize) -> bool {
        let i = self.atoms.partition_point(|a| a.bytes.end <= at);
        self.atoms.get(i).is_some_and(|a| a.bytes.start < at)
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
            || self.atom_interior(range.start)
            || self.atom_interior(range.end)
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
                || self.atom_interior(f.bytes.start)
                || self.atom_interior(f.bytes.end)
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
            objects: Vec::new(),
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
            enum Span<'s> {
                Font(Range<usize>, FontId, Style<'s>, WritingScript),
                Object(usize),
            }
            let mut spans = Vec::new();
            for (relative, cluster) in text[run.clone()].grapheme_indices(true) {
                let start = run.start + relative;
                let end = start + cluster.len();
                let atom = self.atoms.partition_point(|a| a.bytes.end <= start);
                if let Some(a) = self.atoms.get(atom).filter(|a| a.bytes.contains(&start)) {
                    if a.bytes.start == start {
                        spans.push(Span::Object(atom));
                    }
                    continue;
                }
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
                let script = self.scripts[start];
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
                if let Some(Span::Font(range, last, last_style, last_script)) = spans.last_mut()
                    && *last == id
                    && *last_style == style
                    && *last_script == script
                    && range.end == start
                {
                    range.end = end;
                } else {
                    spans.push(Span::Font(start..end, id, style, script));
                }
            }
            if rtl {
                spans.reverse();
            }
            for span in spans {
                let (span, id, style, script) = match span {
                    Span::Font(range, id, style, script) => (range, id, style, script),
                    Span::Object(i) => {
                        let a = &self.atoms[i];
                        if let Some(b) = a.ink {
                            let b = [out.advance + b[0], b[1], out.advance + b[2], b[3]];
                            out.ink = Some(match out.ink {
                                None => b,
                                Some(v) => [
                                    js_min(v[0], b[0]),
                                    js_min(v[1], b[1]),
                                    js_max(v[2], b[2]),
                                    js_max(v[3], b[3]),
                                ],
                            });
                        }
                        out.objects.push(PlacedAtom {
                            tag: a.tag,
                            bytes: a.bytes.clone(),
                            scalars: scalar_at[a.bytes.start]..scalar_at[a.bytes.end],
                            origin: [out.advance, 0.],
                            advance: a.advance,
                        });
                        out.advance += a.advance;
                        continue;
                    }
                };
                let face = parsed(id)?;
                let before = self
                    .atoms
                    .iter()
                    .rev()
                    .find(|a| a.bytes.end <= span.start)
                    .map_or(range.start, |a| a.bytes.end.max(range.start));
                let after = self
                    .atoms
                    .iter()
                    .find(|a| a.bytes.start >= span.end)
                    .map_or(range.end, |a| a.bytes.start.min(range.end));
                // Shape joining across style/font boundaries, but never across a
                // real line break. The output contains only the span's own glyphs.
                let glyphs = shape_buffer_context(
                    face,
                    &text[span.clone()],
                    rtl,
                    &style,
                    &text[before..span.start],
                    &text[span.end..after],
                    Some(script),
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
                    let size = style.shift.scale();
                    let advance = f64::from(pos.x_advance) / units * size;
                    let origin = [
                        out.advance + f64::from(pos.x_offset) / units * size,
                        f64::from(pos.y_offset) / units * size + style.shift.lift(),
                    ];
                    if let Some(box_) = face
                        .glyph_bounding_box(rustybuzz::ttf_parser::GlyphId(info.glyph_id as u16))
                    {
                        for x in [box_.x_min, box_.x_max] {
                            for y in [box_.y_min, box_.y_max] {
                                let (x, y) =
                                    (f64::from(x) / units * size, f64::from(y) / units * size);
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
                        pen: out.advance,
                        size,
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
        if !out.advance.is_finite() || out.ink.is_some_and(|b| b.iter().any(|x| !x.is_finite())) {
            return Err(Error::InvalidGeometry);
        }
        Ok(out)
    }
}

/// One logical line in visual glyph order. Wrapping and tabs belong to the
/// caller; shaping never reverses or rewrites the stored string.
pub fn line(text: &str, style: Style<'_>) -> Result<Line, Error> {
    if text.len() > MAX_BYTES {
        return Err(Error::TooLong);
    }
    if text.contains('\t') {
        return Err(Error::LineBreak);
    }
    Paragraph::new(text, style.direction)?.line(0..text.len(), style, &[])
}

/// Exact outline operations, normalized to ems in the shaped line's baseline
/// frame (positive y up). Consumers apply their one model-to-screen/paper
/// transform; they must not shape the source string a second time.
#[derive(Clone, Debug, PartialEq)]
pub enum OutlineOp {
    Move([f64; 2]),
    Line([f64; 2]),
    Quad([f64; 2], [f64; 2]),
    Cubic([f64; 2], [f64; 2], [f64; 2]),
    Close,
}

/// A glyph's actual bundled contour in exactly the same position/slant used
/// by the shared ink measurement. Spaces have no outline; an invalid glyph
/// or face is an error rather than an empty character.
pub fn outline(glyph: &Glyph) -> Result<Vec<OutlineOp>, Error> {
    use rustybuzz::ttf_parser::{GlyphId, OutlineBuilder};
    let face = parsed(glyph.font)?;
    if glyph.origin.iter().any(|x| !x.is_finite())
        || !glyph.slant.is_finite()
        || !glyph.pen.is_finite()
        || !glyph.size.is_finite()
        || glyph.size <= 0.
    {
        return Err(Error::InvalidGeometry);
    }
    if glyph.id >= face.number_of_glyphs() {
        return Err(Error::BadGlyph {
            font: glyph.font,
            id: glyph.id,
        });
    }
    struct Builder<'a> {
        glyph: &'a Glyph,
        units: f64,
        ops: Vec<OutlineOp>,
    }
    impl Builder<'_> {
        fn point(&self, x: f32, y: f32) -> [f64; 2] {
            let (x, y) = (
                f64::from(x) / self.units * self.glyph.size,
                f64::from(y) / self.units * self.glyph.size,
            );
            [
                self.glyph.origin[0] + x + y * self.glyph.slant,
                self.glyph.origin[1] + y,
            ]
        }
    }
    impl OutlineBuilder for Builder<'_> {
        fn move_to(&mut self, x: f32, y: f32) {
            self.ops.push(OutlineOp::Move(self.point(x, y)));
        }
        fn line_to(&mut self, x: f32, y: f32) {
            self.ops.push(OutlineOp::Line(self.point(x, y)));
        }
        fn quad_to(&mut self, cx: f32, cy: f32, x: f32, y: f32) {
            self.ops
                .push(OutlineOp::Quad(self.point(cx, cy), self.point(x, y)));
        }
        fn curve_to(&mut self, ax: f32, ay: f32, bx: f32, by: f32, x: f32, y: f32) {
            self.ops.push(OutlineOp::Cubic(
                self.point(ax, ay),
                self.point(bx, by),
                self.point(x, y),
            ));
        }
        fn close(&mut self) {
            self.ops.push(OutlineOp::Close);
        }
    }
    let mut builder = Builder {
        glyph,
        units: f64::from(face.units_per_em()),
        ops: Vec::new(),
    };
    face.outline_glyph(GlyphId(glyph.id), &mut builder);
    let invalid = |p: &[f64; 2]| p.iter().any(|v| !v.is_finite());
    if builder.ops.iter().any(|op| match op {
        OutlineOp::Move(p) | OutlineOp::Line(p) => invalid(p),
        OutlineOp::Quad(a, p) => invalid(a) || invalid(p),
        OutlineOp::Cubic(a, b, p) => invalid(a) || invalid(b) || invalid(p),
        OutlineOp::Close => false,
    }) {
        return Err(Error::InvalidGeometry);
    }
    Ok(builder.ops)
}
