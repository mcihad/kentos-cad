//! The drawing's faces in the PDF (design §9a): each face the document
//! writes with is embedded once, as a subset of the glyphs it shows
//! (`subsetter`), a CID font with `Identity-H` encoding and a `ToUnicode`
//! map, so the text is selectable and searchable, Turkish letters
//! included. A glyph's advance (`/W`) is the core's own from its metrics
//! table, so every line sits where the core laid it out and the screen
//! drew it. A text is written in the pieces its faces draw
//! ([`text::runs`]): a character its face lacks in the first drawing face
//! that has it, one no face has as a “?” — never the box of a missing
//! glyph (design §6).

use std::collections::{BTreeMap, BTreeSet};

use pdf_writer::types::{CidFontType, FontFlags, SystemInfo, UnicodeCmap};
use pdf_writer::{Filter, Name, Rect, Str};
use sha2::{Digest, Sha256};

use super::images::deflate;
use super::write::Doc;
use super::{PdfFace, PdfFont};
use crate::error::{Result, SheetError};
use crate::style::TextStyle;
use crate::text::{self, FaceMetrics};

/// A face of the table: its id, weight and slant.
pub(super) type Key = (String, u16, bool);

/// The face the core lays `font` out with (an unknown family is barlow, a missing weight the
/// nearest, as CSS matches it); none only if the table is empty.
pub(super) fn face_of(font: &str, weight: u16, italic: bool) -> Option<&'static FaceMetrics> {
    text::face(&TextStyle {
        font: font.to_owned(),
        size: 1000,
        weight,
        italic,
        color: String::new(),
    })
}

fn key_of(m: &FaceMetrics) -> Key {
    (m.font.clone(), m.weight, m.italic)
}

/// The characters the document writes, by face.
#[derive(Default)]
pub(super) struct Usage {
    chars: BTreeMap<Key, BTreeSet<char>>,
}

impl Usage {
    /// A text in a face: each of its pieces' characters to the face that draws them.
    pub fn add(&mut self, font: &str, weight: u16, italic: bool, text: &str) {
        if let Some(m) = face_of(font, weight, italic) {
            for r in text::runs(m, text) {
                self.chars
                    .entry(key_of(r.face))
                    .or_default()
                    .extend(r.text.chars());
            }
        }
    }

    pub fn faces(&self) -> Vec<PdfFace> {
        self.chars
            .keys()
            .map(|(font, weight, italic)| PdfFace {
                font: font.clone(),
                weight: *weight,
                italic: *italic,
            })
            .collect()
    }
}

/// A face embedded in the document.
pub(super) struct Embedded {
    /// Its name in a page's resources (`F1` …).
    pub name: String,
    /// The Type 0 font object.
    pub font: pdf_writer::Ref,
    pub metrics: &'static FaceMetrics,
    /// Each character's CID (its glyph in the subset); 0: the face lacks it (`.notdef`).
    cids: BTreeMap<char, u16>,
    /// Each character's advance, font units.
    advances: BTreeMap<char, i64>,
}

impl Embedded {
    /// A piece's text as the font's codes: two bytes a character.
    fn encode(&self, text: &str) -> Vec<u8> {
        text.chars()
            .flat_map(|c| self.cids.get(&c).copied().unwrap_or(0).to_be_bytes())
            .collect()
    }

    /// A piece's width in ems.
    fn width_em(&self, text: &str) -> f64 {
        let units: i64 = text
            .chars()
            .map(|c| {
                self.advances
                    .get(&c)
                    .copied()
                    .unwrap_or_else(|| self.metrics.advance(c))
            })
            .sum();
        units as f64 / f64::from(self.metrics.units_per_em.max(1))
    }

    /// The capitals' height in ems (a middle anchor centres on it).
    pub fn cap_em(&self) -> f64 {
        f64::from(self.metrics.cap_height) / f64::from(self.metrics.units_per_em.max(1))
    }
}

/// The document's faces, embedded.
#[derive(Default)]
pub(super) struct Fonts {
    by_key: BTreeMap<Key, Embedded>,
}

impl Fonts {
    /// The embedded face `font` is written with.
    pub fn get(&self, font: &str, weight: u16, italic: bool) -> Option<&Embedded> {
        face_of(font, weight, italic).and_then(|m| self.by_key.get(&key_of(m)))
    }

    /// A text in `font` as the pieces its faces draw, each with its embedded face and codes.
    pub fn runs(
        &self,
        font: &str,
        weight: u16,
        italic: bool,
        text: &str,
    ) -> Vec<(&Embedded, Vec<u8>)> {
        let Some(m) = face_of(font, weight, italic) else {
            return Vec::new();
        };
        text::runs(m, text)
            .into_iter()
            .filter_map(|r| {
                let e = self.by_key.get(&key_of(r.face))?;
                Some((e, e.encode(&r.text)))
            })
            .collect()
    }

    /// A text's width in ems of its size (what an anchor moves by), piece by piece.
    pub fn width_em(&self, font: &str, weight: u16, italic: bool, text: &str) -> f64 {
        let Some(m) = face_of(font, weight, italic) else {
            return 0.0;
        };
        text::runs(m, text)
            .iter()
            .filter_map(|r| Some(self.by_key.get(&key_of(r.face))?.width_em(&r.text)))
            .sum()
    }

    /// Each used face as a subset with its widths and its `ToUnicode` map.
    pub fn embed(doc: &mut Doc, usage: &Usage, files: &[PdfFont]) -> Result<Fonts> {
        let mut by_key = BTreeMap::new();
        for (n, (key, chars)) in usage.chars.iter().enumerate() {
            let label = || format!("{} {}{}", key.0, key.1, if key.2 { " italik" } else { "" });
            let file = files
                .iter()
                .find(|f| f.font == key.0 && f.weight == key.1 && f.italic == key.2)
                .ok_or_else(|| {
                    SheetError::new(
                        "pdf_font_missing",
                        format!(
                            "“{}” yazı tipinin dosyası verilmedi: çizimin TTF dosyasını PDF girdilerine ekleyin.",
                            label()
                        ),
                    )
                })?;
            let metrics = face_of(&key.0, key.1, key.2).ok_or_else(|| {
                SheetError::new("pdf_font_missing", format!("“{}” tabloda yok.", label()))
            })?;
            let bad = |why: &str| {
                SheetError::new(
                    "pdf_font_bad",
                    format!(
                        "“{}” yazı tipinin dosyası okunamadı ({why}): çizimin TTF dosyasını verin.",
                        label()
                    ),
                )
            };
            let face = ttf_parser::Face::parse(&file.data, 0).map_err(|_| bad("TrueType değil"))?;
            let upm = f64::from(face.units_per_em().max(1));
            // A file without a glyph the table says the face has is not the drawing's face: its
            // box would be printed where the screen shows the letter.
            let lacking: String = chars
                .iter()
                .filter(|c| face.glyph_index(**c).is_none_or(|g| g.0 == 0))
                .collect();
            if !lacking.is_empty() {
                return Err(bad(&format!("“{lacking}” karakterleri dosyada yok")));
            }
            // Each character's glyph in the face, and its advance: the table's, else the face's.
            let mut glyphs: BTreeMap<char, u16> = BTreeMap::new();
            let mut advances: BTreeMap<char, i64> = BTreeMap::new();
            for &c in chars {
                let gid = face.glyph_index(c).map_or(0, |g| g.0);
                glyphs.insert(c, gid);
                let own = (gid != 0)
                    .then(|| face.glyph_hor_advance(ttf_parser::GlyphId(gid)))
                    .flatten()
                    .map(|a| (f64::from(a) * f64::from(metrics.units_per_em) / upm).round() as i64);
                let adv = metrics
                    .advance_of(c)
                    .or(own)
                    .unwrap_or_else(|| i64::from(metrics.fallback));
                advances.insert(c, adv);
            }
            let used: Vec<u16> = glyphs.values().copied().filter(|g| *g != 0).collect();
            let remapper = subsetter::GlyphRemapper::new_from_glyphs_sorted(&used);
            let subset = subsetter::subset(&file.data, 0, &remapper)
                .map_err(|e| bad(&format!("alt küme: {e}")))?;
            let cids: BTreeMap<char, u16> = glyphs
                .iter()
                .map(|(c, g)| {
                    (
                        *c,
                        if *g == 0 {
                            0
                        } else {
                            remapper.get(*g).unwrap_or(0)
                        },
                    )
                })
                .collect();
            let name = format!("F{}", n + 1);
            let base = base_name(&face, key, &used);
            let font = write_font(doc, &base, &face, metrics, &cids, &advances, &subset);
            by_key.insert(
                key.clone(),
                Embedded {
                    name,
                    font,
                    metrics,
                    cids,
                    advances,
                },
            );
        }
        Ok(Fonts { by_key })
    }
}

/// The subset's PostScript name with its tag: six capitals from the face and its glyphs, so
/// the same subset has the same name (and two subsets of a face differ).
fn base_name(face: &ttf_parser::Face<'_>, key: &Key, glyphs: &[u16]) -> String {
    let ps = face
        .names()
        .into_iter()
        .filter(|n| n.name_id == ttf_parser::name_id::POST_SCRIPT_NAME)
        .find_map(|n| n.to_string())
        .unwrap_or_else(|| format!("{}-{}", key.0, key.1));
    let clean: String = ps
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .collect();
    let mut h = Sha256::new();
    h.update(clean.as_bytes());
    for g in glyphs {
        h.update(g.to_be_bytes());
    }
    let d = h.finalize();
    let tag: String = d
        .iter()
        .take(6)
        .map(|b| char::from(b'A' + b % 26))
        .collect();
    format!("{tag}+{clean}")
}

fn write_font(
    doc: &mut Doc,
    base: &str,
    face: &ttf_parser::Face<'_>,
    metrics: &FaceMetrics,
    cids: &BTreeMap<char, u16>,
    advances: &BTreeMap<char, i64>,
    subset: &[u8],
) -> pdf_writer::Ref {
    let type0 = doc.alloc();
    let cid = doc.alloc();
    let descriptor = doc.alloc();
    let cmap = doc.alloc();
    let file = doc.alloc();
    let upm = f64::from(face.units_per_em().max(1));
    let em = f64::from(metrics.units_per_em.max(1));
    // Glyph space is a thousandth of the em.
    let k = |units: f64, of: f64| (units * 1000.0 / of) as f32;
    let name = Name(base.as_bytes());
    doc.chunk
        .type0_font(type0)
        .base_font(name)
        .encoding_predefined(Name(b"Identity-H"))
        .descendant_font(cid)
        .to_unicode(cmap);
    // Each CID's width: the core's advance of its character (the first, if two share a glyph).
    let mut widths: BTreeMap<u16, f32> = BTreeMap::new();
    for (c, id) in cids {
        if *id != 0 {
            widths
                .entry(*id)
                .or_insert_with(|| k(advances.get(c).copied().unwrap_or(0) as f64, em));
        }
    }
    let fallback = k(f64::from(metrics.fallback), em);
    {
        let mut font = doc.chunk.cid_font(cid);
        font.subtype(CidFontType::Type2)
            .base_font(name)
            .system_info(SystemInfo {
                registry: Str(b"Adobe"),
                ordering: Str(b"Identity"),
                supplement: 0,
            })
            .font_descriptor(descriptor)
            .default_width(fallback);
        {
            let mut w = font.widths();
            let mut run: Vec<f32> = Vec::new();
            let mut start = 0u16;
            for (id, width) in &widths {
                if !run.is_empty() && *id != start + run.len() as u16 {
                    w.consecutive(start, run.drain(..));
                }
                if run.is_empty() {
                    start = *id;
                }
                run.push(*width);
            }
            if !run.is_empty() {
                w.consecutive(start, run);
            }
        }
        font.cid_to_gid_map_predefined(Name(b"Identity"));
    }
    let b = face.global_bounding_box();
    let mut flags = FontFlags::NON_SYMBOLIC;
    if face.is_monospaced() {
        flags |= FontFlags::FIXED_PITCH;
    }
    if metrics.italic || face.is_italic() {
        flags |= FontFlags::ITALIC;
    }
    doc.chunk
        .font_descriptor(descriptor)
        .name(name)
        .flags(flags)
        .bbox(Rect::new(
            k(f64::from(b.x_min), upm),
            k(f64::from(b.y_min), upm),
            k(f64::from(b.x_max), upm),
            k(f64::from(b.y_max), upm),
        ))
        .italic_angle(face.italic_angle())
        .ascent(k(f64::from(face.ascender()), upm))
        .descent(k(f64::from(face.descender()), upm))
        .cap_height(k(f64::from(metrics.cap_height), em))
        // A guess from the weight, as most writers make it (no face states it).
        .stem_v((10.0 + 220.0 * (f32::from(metrics.weight) - 50.0) / 900.0).max(10.0))
        .font_file2(file);
    // Which character each CID is: the text out of the PDF is the text in.
    let mut unicode = UnicodeCmap::new(
        Name(b"Custom"),
        SystemInfo {
            registry: Str(b"Adobe"),
            ordering: Str(b"UCS"),
            supplement: 0,
        },
    );
    let mut mapped: BTreeSet<u16> = BTreeSet::new();
    for (c, id) in cids {
        if *id != 0 && mapped.insert(*id) {
            unicode.pair(*id, *c);
        }
    }
    let cmap_data = deflate(unicode.finish().as_slice());
    doc.chunk
        .cmap(cmap, &cmap_data)
        .name(Name(b"Custom"))
        .system_info(SystemInfo {
            registry: Str(b"Adobe"),
            ordering: Str(b"UCS"),
            supplement: 0,
        })
        .filter(Filter::FlateDecode);
    let packed = deflate(subset);
    doc.chunk
        .stream(file, &packed)
        .filter(Filter::FlateDecode)
        .pair(Name(b"Length1"), subset.len() as i32);
    type0
}
