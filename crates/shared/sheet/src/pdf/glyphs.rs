//! Frozen CAD glyphs: actual subset GIDs, explicit CID-to-GID mapping and
//! cluster Unicode. This path never runs a second text shaper or font matcher.
use std::collections::{BTreeMap, BTreeSet};

use kentos_geometry_core::text::shaping::{self, FontId};
use pdf_writer::types::{CidFontType, FontFlags, SystemInfo, UnicodeCmap};
use pdf_writer::{Filter, Name, Rect, Ref, Str};
use sha2::{Digest, Sha256};

use super::{GlyphText, PdfGlyph, images::deflate, write::Doc};
use crate::error::{Result, SheetError};

type Code = (u16, String);
#[derive(Default)]
pub(super) struct Usage(BTreeMap<u16, BTreeSet<Code>>);
fn bad(why: &str) -> SheetError {
    SheetError::new(
        "pdf_text_layout_invalid",
        format!(
            "Yazının ortak harf yerleşimi PDF için geçersiz ({why}). Yazıyı denetleyin; farklı harfle veya kısmi metinle çıktı alınamaz."
        ),
    )
}
impl Usage {
    pub fn add(&mut self, text: &GlyphText) -> Result<()> {
        if text.profile != "unicode-v1" {
            return Err(bad("bilinmeyen yazı profili"));
        }
        if text.glyphs.len() > shaping::MAX_GLYPHS || text.rules.len() > 1_000_000 {
            return Err(bad("yerleşim bütçesi aşıldı"));
        }
        let mut faces = BTreeMap::new();
        let mut source_bytes = 0usize;
        for g in &text.glyphs {
            source_bytes = source_bytes.saturating_add(g.text.len());
            if source_bytes > shaping::MAX_BYTES * 4 {
                return Err(bad("kaynak metin bütçesi aşıldı"));
            }
            if g.matrix.iter().any(|v| !v.is_finite() || v.abs() > 1e20)
                || g.matrix[0] * g.matrix[3] == g.matrix[1] * g.matrix[2]
                || g.text.len() > shaping::MAX_BYTES
                || g.text.encode_utf16().count() > 16_382
                || super::content::color(&g.color).is_none()
            {
                return Err(bad("harf dönüşümü, rengi veya kaynak metni"));
            }
            if let std::collections::btree_map::Entry::Vacant(entry) = faces.entry(g.font) {
                let bytes = shaping::font_bytes(FontId(g.font))
                    .ok_or_else(|| bad("bilinmeyen paketli yazı tipi"))?;
                entry.insert(
                    ttf_parser::Face::parse(bytes, 0)
                        .map_err(|_| bad("paketli yazı tipi okunamadı"))?,
                );
            }
            let face = &faces[&g.font];
            if g.glyph == 0 || g.glyph >= face.number_of_glyphs() {
                return Err(bad("eksik harf"));
            }
            let used = self.0.entry(g.font).or_default();
            used.insert((g.glyph, g.text.clone()));
            if used.len() >= u16::MAX as usize {
                return Err(bad("yazı tipi kod bütçesi aşıldı"));
            }
        }
        for r in &text.rules {
            if r.corners
                .iter()
                .flatten()
                .any(|v| !v.is_finite() || v.abs() > 1e20)
                || super::content::color(&r.color).is_none()
            {
                return Err(bad("kesir veya alt çizgi geometrisi"));
            }
        }
        Ok(())
    }
}

pub(super) struct Embedded {
    pub name: String,
    pub font: Ref,
    codes: BTreeMap<Code, u16>,
}
impl Embedded {
    pub fn code(&self, glyph: &PdfGlyph) -> Option<[u8; 2]> {
        self.codes
            .get(&(glyph.glyph, glyph.text.clone()))
            .map(|v| v.to_be_bytes())
    }
}
#[derive(Default)]
pub(super) struct Fonts(BTreeMap<u16, Embedded>);
impl Fonts {
    pub fn get(&self, id: u16) -> Option<&Embedded> {
        self.0.get(&id)
    }
    pub fn embed(doc: &mut Doc, usage: &Usage) -> Result<Self> {
        let mut fonts = BTreeMap::new();
        for (&id, codes) in &usage.0 {
            let bytes = shaping::font_bytes(FontId(id)).ok_or_else(|| bad("yazı tipi"))?;
            let face = ttf_parser::Face::parse(bytes, 0).map_err(|_| bad("TrueType"))?;
            let glyphs: Vec<_> = codes.iter().map(|(g, _)| *g).collect();
            let remap = subsetter::GlyphRemapper::new_from_glyphs_sorted(&glyphs);
            let subset =
                subsetter::subset(bytes, 0, &remap).map_err(|_| bad("yazı tipi alt kümesi"))?;
            let key = (format!("unicode-v1/{id}"), 400, false);
            let old_base = super::fonts::base_name(&face, &key, &glyphs);
            // Instantiated static weights may share an upstream PostScript
            // name. Include exact face bytes, so their subset names cannot alias.
            let mut hash = Sha256::new();
            hash.update(bytes);
            for gid in &glyphs {
                hash.update(gid.to_be_bytes());
            }
            let tag: String = hash
                .finalize()
                .iter()
                .take(6)
                .map(|b| char::from(b'A' + b % 26))
                .collect();
            let base = format!(
                "{tag}+{}",
                old_base.split_once('+').map_or(old_base.as_str(), |p| p.1)
            );
            let name = Name(base.as_bytes());
            let type0 = doc.alloc();
            let cid = doc.alloc();
            let descriptor = doc.alloc();
            let cmap = doc.alloc();
            let file = doc.alloc();
            let gid_map = doc.alloc();
            doc.chunk
                .type0_font(type0)
                .base_font(name)
                .encoding_predefined(Name(b"Identity-H"))
                .descendant_font(cid)
                .to_unicode(cmap);
            let assigned: BTreeMap<_, _> = codes
                .iter()
                .enumerate()
                .map(|(i, c)| (c.clone(), (i + 1) as u16))
                .collect();
            let upm = f64::from(face.units_per_em());
            let k = |v: i16| (f64::from(v) * 1000. / upm) as f32;
            let mut gids = vec![0u8; (assigned.len() + 1) * 2];
            let mut unicode = UnicodeCmap::new(
                Name(b"Custom"),
                SystemInfo {
                    registry: Str(b"Adobe"),
                    ordering: Str(b"UCS"),
                    supplement: 0,
                },
            );
            let mut widths = Vec::with_capacity(assigned.len());
            for ((gid, text), code) in &assigned {
                let subset_gid = remap
                    .get(*gid)
                    .ok_or_else(|| bad("alt kümede harf bulunamadı"))?;
                gids[usize::from(*code) * 2..usize::from(*code) * 2 + 2]
                    .copy_from_slice(&subset_gid.to_be_bytes());
                widths.push(
                    f32::from(
                        face.glyph_hor_advance(ttf_parser::GlyphId(*gid))
                            .unwrap_or(0),
                    ) * 1000.
                        / upm as f32,
                );
                unicode.pair_with_multiple(*code, text.chars());
            }
            {
                let mut f = doc.chunk.cid_font(cid);
                f.subtype(CidFontType::Type2)
                    .base_font(name)
                    .system_info(SystemInfo {
                        registry: Str(b"Adobe"),
                        ordering: Str(b"Identity"),
                        supplement: 0,
                    })
                    .font_descriptor(descriptor)
                    .default_width(0.)
                    .cid_to_gid_map_stream(gid_map);
                f.widths().consecutive(1, widths);
            }
            let b = face.global_bounding_box();
            let mut flags = FontFlags::NON_SYMBOLIC;
            if face.is_monospaced() {
                flags |= FontFlags::FIXED_PITCH;
            }
            if face.is_italic() {
                flags |= FontFlags::ITALIC;
            }
            doc.chunk
                .font_descriptor(descriptor)
                .name(name)
                .flags(flags)
                .bbox(Rect::new(k(b.x_min), k(b.y_min), k(b.x_max), k(b.y_max)))
                .italic_angle(face.italic_angle())
                .ascent(k(face.ascender()))
                .descent(k(face.descender()))
                .cap_height(k(face.capital_height().unwrap_or(face.ascender())))
                .stem_v(80.)
                .font_file2(file);
            doc.chunk
                .stream(gid_map, &deflate(&gids))
                .filter(Filter::FlateDecode);
            doc.chunk
                .cmap(cmap, &deflate(unicode.finish().as_slice()))
                .name(Name(b"Custom"))
                .system_info(SystemInfo {
                    registry: Str(b"Adobe"),
                    ordering: Str(b"UCS"),
                    supplement: 0,
                })
                .filter(Filter::FlateDecode);
            doc.chunk
                .stream(file, &deflate(&subset))
                .filter(Filter::FlateDecode)
                .pair(Name(b"Length1"), subset.len() as i32);
            fonts.insert(
                id,
                Embedded {
                    name: format!("U{id}"),
                    font: type0,
                    codes: assigned,
                },
            );
        }
        Ok(Self(fonts))
    }
}
