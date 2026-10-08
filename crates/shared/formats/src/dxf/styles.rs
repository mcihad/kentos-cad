//! A DXF's text and dimension styles (docs/adr/0183 §7): its STYLE and
//! DIMSTYLE records as KentOS's styles. A record KentOS wrote for a style
//! carries the style itself in its KENTOS data (`face`, `look`), read back
//! exactly; one it wrote for a styleless face is no style (`styleless`).
//! Standard, KentOS's or another program's, is Standart: no style (a text on
//! it has the project's typeface, a dimension its record's look as its own).
//! Another program's STYLE names a typeface by its file (3) or by
//! ACAD's extended data (its family and bold and italic flags): Arial and
//! Arimo are Arimo, Courier is Courier Prime, KentOS's seven families are
//! themselves, any other (an SHX font, Times, Calibri …) the project's
//! typeface, said. Its fixed height (40), width (41) and slant (50) come
//! with it. A DIMSTYLE gives its value's height (DIMTXT), arrowheads
//! (DIMTSZ: oblique ticks of √2 × DIMTSZ; else DIMBLK's name), their size
//! (DIMASZ), the extension lines' offset and reach (DIMEXO, DIMEXE), the
//! value's gap (DIMGAP) and place (DIMTAD 0: centred), decimals (DIMDEC),
//! prefix and suffix (DIMPOST around “<>”), unit (DIMLFAC 100: cm, 1000:
//! mm), the typeface of its text style (DIMTXSTY), all times DIMSCALE; its
//! lines' colours (DIMCLRD, DIMCLRE, DIMCLRT: an ACI number; BYBLOCK,
//! BYLAYER and 7 the object's own), weights (DIMLWD, DIMLWE: hundredths of
//! a mm) and types (DIMLTYPE, DIMLTEX1 else DIMLTEX2: an LTYPE's handle),
//! docs/adr/0205 §6.
//!
//! Only the styles the read objects follow are kept, numbered as the reader
//! meets them; their sizes are paper mm at the project's scale.

use std::collections::HashMap;

use kentos_contracts::{
    DimensionArrow, DimensionStyleDef, DimensionTextPlace, DrawingFont, DrawingUnit, LineType,
    TextStyleDef,
};

use super::lexer::Pair;
use crate::num::{parse_int, parse_real};

/// ACAD's flags of a TrueType style's extended data (1071): italic, bold.
const ACAD_ITALIC: i64 = 0x0100_0000;
const ACAD_BOLD: i64 = 0x0200_0000;

/// A STYLE record as the file has it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct StyleRecord {
    pub name: String,
    /// Its typeface's file (3): `arial.ttf`, `romans.shx`.
    pub file: String,
    /// ACAD's family name and flags, when it has them.
    pub family: Option<String>,
    pub acad_flags: i64,
    /// Fixed height (40, drawing units; 0 none), width (41) and slant (50, degrees).
    pub height: f64,
    pub width: f64,
    pub oblique: f64,
    /// The style as KentOS wrote it (its KENTOS data); a record KentOS wrote for a styleless face.
    pub kentos: Option<TextStyleDef>,
    pub kentos_styleless: bool,
}

impl StyleRecord {
    /// Whether texts naming it follow a style of the project: not Standard
    /// (Standart), not a record KentOS wrote for a styleless face.
    pub fn is_style(&self) -> bool {
        !self.kentos_styleless && !is_standard(&self.name)
    }
}

/// Whether a record's name is Standard's (Standart; case aside).
pub fn is_standard(name: &str) -> bool {
    name.trim().eq_ignore_ascii_case("STANDARD")
}

/// A STYLE record's groups (after `0 STYLE`), with its KENTOS and ACAD data.
pub fn style_record(groups: &[Pair<'_>], dec: super::strings::Decoder) -> StyleRecord {
    let find = |code: i32| groups.iter().find(|p| p.code == code);
    let text = |code: i32| find(code).map(|p| dec.string(p.value)).unwrap_or_default();
    let real = |code: i32, or: f64| find(code).and_then(|p| parse_real(p.text())).unwrap_or(or);
    let mut r = StyleRecord {
        name: text(2),
        file: text(3),
        height: real(40, 0.0),
        width: real(41, 1.0),
        oblique: real(50, 0.0),
        ..StyleRecord::default()
    };
    // ACAD's data: 1000 the family, 1071 the flags.
    if let Some(start) = groups
        .iter()
        .position(|p| p.code == 1001 && p.text().eq_ignore_ascii_case("ACAD"))
    {
        let rest = &groups[start + 1..];
        let rest = &rest[..rest
            .iter()
            .position(|p| p.code == 1001)
            .unwrap_or(rest.len())];
        r.family = rest
            .iter()
            .find(|p| p.code == 1000)
            .map(|p| dec.string(p.value))
            .filter(|f| !f.trim().is_empty());
        r.acad_flags = rest
            .iter()
            .find(|p| p.code == 1071)
            .and_then(|p| parse_int(p.text()))
            .unwrap_or(0);
    }
    if let Some(meta) = super::entity::xdata_of(groups, dec)
        && let Some(json) = meta.face.as_deref()
    {
        if json == super::xdata::STYLELESS {
            r.kentos_styleless = true;
        } else {
            r.kentos = serde_json::from_str(json).ok();
        }
    }
    r
}

/// KentOS's family for a typeface named `name` (a file or a family); none
/// for one KentOS does not have.
pub fn family_of(name: &str) -> Option<DrawingFont> {
    let n: String = name
        .to_lowercase()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect();
    if n.starts_with("arial") || n.starts_with("arimo") {
        return Some(DrawingFont::Arimo);
    }
    if n.starts_with("cour") {
        return Some(DrawingFont::CourierPrime);
    }
    for (stem, f) in [
        ("barlow", DrawingFont::Barlow),
        ("overpass", DrawingFont::Overpass),
        ("quicksand", DrawingFont::Quicksand),
        ("architectsdaughter", DrawingFont::ArchitectsDaughter),
        ("ibmplexmono", DrawingFont::PlexMono),
        ("plexmono", DrawingFont::PlexMono),
    ] {
        if n.starts_with(stem) {
            return Some(f);
        }
    }
    None
}

/// Bold and italic as a typeface file's name says them (`arialbd`, `ariali`,
/// `arialbi`, `courbd`, `Barlow-Bold`, `-Italic`).
fn file_says(file: &str) -> (bool, bool) {
    let stem = file
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(file)
        .split('.')
        .next()
        .unwrap_or_default()
        .to_lowercase();
    if stem.contains("bold") || stem.contains("italic") || stem.contains("oblique") {
        return (
            stem.contains("bold"),
            stem.contains("italic") || stem.contains("oblique"),
        );
    }
    for base in ["arial", "cour", "arimo"] {
        if let Some(rest) = stem.strip_prefix(base) {
            return match rest {
                "bd" => (true, false),
                "i" => (false, true),
                "bi" | "z" => (true, true),
                _ => (false, false),
            };
        }
    }
    (false, false)
}

/// A paper size in mm of `units` drawing units: `metres` turns them into
/// metres, the scale is the project's.
fn mm(units: f64, metres: &dyn Fn(f64) -> f64, scale: f64) -> f64 {
    metres(units) * 1000.0 / scale
}

/// The text style a STYLE record is, its id `id`; the typeface when the
/// record's is not KentOS's (to be said), else none.
pub fn text_style(
    r: &StyleRecord,
    id: String,
    project: DrawingFont,
    metres: &dyn Fn(f64) -> f64,
    scale: f64,
) -> (TextStyleDef, Option<String>) {
    if let Some(own) = &r.kentos {
        return (
            TextStyleDef {
                id,
                name: r.name.clone(),
                ..own.clone()
            },
            None,
        );
    }
    let named = r.family.clone().unwrap_or_else(|| r.file.clone());
    let font = family_of(&named).or_else(|| family_of(&r.file));
    let (file_bold, file_italic) = file_says(&r.file);
    let style = TextStyleDef {
        id,
        name: r.name.clone(),
        font: font.unwrap_or(project),
        bold: r.acad_flags & ACAD_BOLD != 0 || file_bold,
        italic: r.acad_flags & ACAD_ITALIC != 0 || file_italic,
        oblique: (r.oblique != 0.0 && kentos_contracts::oblique_holds(r.oblique))
            .then_some(r.oblique),
        height: (r.height > 0.0)
            .then(|| mm(r.height, metres, scale))
            .filter(|h| *h > 0.0 && *h <= kentos_contracts::MAX_STYLE_MM),
        width_factor: (r.width != 1.0 && kentos_contracts::width_factor_ok(r.width))
            .then_some(r.width),
        font_file: (!r.file.trim().is_empty()).then(|| r.file.clone()),
    };
    let unmapped = font.is_none().then(|| {
        if named.trim().is_empty() {
            r.name.clone()
        } else {
            named
        }
    });
    (style, unmapped)
}

/// A DIMSTYLE's variables (or an entity's DSTYLE changes to them).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DimVars {
    pub dimtxt: Option<f64>,
    pub dimasz: Option<f64>,
    pub dimtsz: Option<f64>,
    pub dimexo: Option<f64>,
    pub dimexe: Option<f64>,
    pub dimgap: Option<f64>,
    pub dimscale: Option<f64>,
    pub dimlfac: Option<f64>,
    pub dimtad: Option<i64>,
    pub dimdec: Option<i64>,
    pub dimpost: Option<String>,
    /// DIMBLK's name (5, an older file) or its block record's handle (342).
    pub dimblk: Option<String>,
    pub dimblk_handle: Option<u64>,
    /// DIMTXSTY: the text style's record's handle (340).
    pub dimtxsty: Option<u64>,
    /// The lines' colours (DIMCLRD 176, DIMCLRE 177, DIMCLRT 178: ACI
    /// numbers), weights (DIMLWD 371, DIMLWE 372: hundredths of a mm) and
    /// types (DIMLTYPE 345, DIMLTEX1 346, DIMLTEX2 347: LTYPE handles).
    pub dimclrd: Option<i64>,
    pub dimclre: Option<i64>,
    pub dimclrt: Option<i64>,
    pub dimlwd: Option<i64>,
    pub dimlwe: Option<i64>,
    pub dimltype: Option<u64>,
    pub dimltex1: Option<u64>,
    pub dimltex2: Option<u64>,
    /// The style as KentOS wrote it (its KENTOS data).
    pub kentos: Option<DimensionStyleDef>,
}

impl DimVars {
    /// `self`'s values, `under`'s where `self` has none.
    pub fn over(self, under: &DimVars) -> DimVars {
        DimVars {
            dimtxt: self.dimtxt.or(under.dimtxt),
            dimasz: self.dimasz.or(under.dimasz),
            dimtsz: self.dimtsz.or(under.dimtsz),
            dimexo: self.dimexo.or(under.dimexo),
            dimexe: self.dimexe.or(under.dimexe),
            dimgap: self.dimgap.or(under.dimgap),
            dimscale: self.dimscale.or(under.dimscale),
            dimlfac: self.dimlfac.or(under.dimlfac),
            dimtad: self.dimtad.or(under.dimtad),
            dimdec: self.dimdec.or(under.dimdec),
            dimpost: self.dimpost.or_else(|| under.dimpost.clone()),
            dimblk: self.dimblk.or_else(|| under.dimblk.clone()),
            dimblk_handle: self.dimblk_handle.or(under.dimblk_handle),
            dimtxsty: self.dimtxsty.or(under.dimtxsty),
            dimclrd: self.dimclrd.or(under.dimclrd),
            dimclre: self.dimclre.or(under.dimclre),
            dimclrt: self.dimclrt.or(under.dimclrt),
            dimlwd: self.dimlwd.or(under.dimlwd),
            dimlwe: self.dimlwe.or(under.dimlwe),
            dimltype: self.dimltype.or(under.dimltype),
            dimltex1: self.dimltex1.or(under.dimltex1),
            dimltex2: self.dimltex2.or(under.dimltex2),
            kentos: self.kentos.or_else(|| under.kentos.clone()),
        }
    }

    /// One variable by its group code and value (a table record's groups,
    /// or DSTYLE's code and value pairs).
    pub fn set(&mut self, code: i64, value: &str) {
        let real = || parse_real(value);
        let int = || parse_int(value);
        let handle = || u64::from_str_radix(value.trim(), 16).ok();
        match code {
            140 => self.dimtxt = real(),
            41 => self.dimasz = real(),
            142 => self.dimtsz = real(),
            42 => self.dimexo = real(),
            44 => self.dimexe = real(),
            147 => self.dimgap = real(),
            40 => self.dimscale = real(),
            144 => self.dimlfac = real(),
            77 => self.dimtad = int(),
            271 => self.dimdec = int(),
            3 => self.dimpost = Some(value.to_owned()),
            5 => self.dimblk = Some(value.to_owned()),
            342 => self.dimblk_handle = handle(),
            340 => self.dimtxsty = handle(),
            176 => self.dimclrd = int(),
            177 => self.dimclre = int(),
            178 => self.dimclrt = int(),
            371 => self.dimlwd = int(),
            372 => self.dimlwe = int(),
            345 => self.dimltype = handle(),
            346 => self.dimltex1 = handle(),
            347 => self.dimltex2 = handle(),
            _ => {}
        }
    }
}

/// A dimension line's colour from its ACI number (docs/adr/0205 §6): its
/// `#RRGGBB`; none (the object's) for BYBLOCK (0), BYLAYER (256) and 7
/// (black on paper, white on a dark screen: the object's own ink).
pub fn line_colour(aci: Option<i64>) -> Option<String> {
    let a = u8::try_from(aci?).ok().filter(|a| *a >= 1 && *a != 7)?;
    let [r, g, b] = super::aci::rgb(a);
    Some(format!("#{r:02X}{g:02X}{b:02X}"))
}

/// A dimension line's weight from DXF's hundredths of a mm: mm; none (a
/// hairline) for BYLAYER (−1), BYBLOCK (−2), the default (−3), 0 and what
/// DXF has no weight for.
pub fn line_weight_mm(w: Option<i64>) -> Option<f64> {
    w.filter(|w| (1..=211).contains(w))
        .map(|w| w as f64 / 100.0)
}

/// A dimension line's type from its LTYPE's handle: none for a continuous
/// one (and for ByLayer and ByBlock).
pub fn line_type_of(handle: Option<u64>, ltypes: &HashMap<u64, LineType>) -> Option<LineType> {
    handle
        .and_then(|h| ltypes.get(&h))
        .copied()
        .filter(|t| *t != LineType::Continuous)
}

/// A DIMSTYLE record's variables, with its KENTOS data.
pub fn dim_record(groups: &[Pair<'_>], dec: super::strings::Decoder) -> DimVars {
    let mut v = DimVars::default();
    let end = groups
        .iter()
        .position(|p| p.code == 1001)
        .unwrap_or(groups.len());
    for p in &groups[..end] {
        if matches!(
            p.code,
            140 | 41
                | 142
                | 42
                | 44
                | 147
                | 40
                | 144
                | 77
                | 271
                | 3
                | 5
                | 342
                | 340
                | 176
                | 177
                | 178
                | 371
                | 372
                | 345
                | 346
                | 347
        ) {
            v.set(i64::from(p.code), &dec.string(p.value));
        }
    }
    if let Some(j) = super::entity::xdata_of(groups, dec).and_then(|m| m.look) {
        v.kentos = serde_json::from_str(&j).ok();
    }
    v
}

/// An entity's own changes to its dimension style (ACAD's DSTYLE: a 1070
/// code, then its value), for the variables `DimVars` holds.
pub fn dim_overrides(groups: &[Pair<'_>], dec: super::strings::Decoder) -> DimVars {
    let mut v = DimVars::default();
    let Some(start) = groups
        .iter()
        .position(|p| p.code == 1001 && p.text().eq_ignore_ascii_case("ACAD"))
    else {
        return v;
    };
    let rest = &groups[start + 1..];
    let rest = &rest[..rest
        .iter()
        .position(|p| p.code == 1001)
        .unwrap_or(rest.len())];
    let Some(k) = rest
        .iter()
        .position(|p| p.code == 1000 && p.text().eq_ignore_ascii_case("DSTYLE"))
    else {
        return v;
    };
    let mut code: Option<i64> = None;
    for p in &rest[k + 1..] {
        if p.code == 1002 {
            if p.text() == "}" {
                break;
            }
            continue;
        }
        match code.take() {
            None => code = (p.code == 1070).then(|| parse_int(p.text())).flatten(),
            Some(c) => v.set(c, &dec.string(p.value)),
        }
    }
    v
}

/// The arrowhead a DIMBLK names (docs/adr/0183 §7): none for the oblique
/// tick; the filled arrow for a name KentOS does not know, said (`false`).
pub fn arrow_of(name: &str) -> (Option<DimensionArrow>, bool) {
    let n = name.trim().to_uppercase();
    match n.as_str() {
        "" | "_CLOSEDFILLED" => (Some(DimensionArrow::Closed), true),
        "_NONE" => (Some(DimensionArrow::None), true),
        "_OBLIQUE" | "_ARCHTICK" => (None, true),
        "_OPEN" | "_OPEN30" | "_OPEN90" | "_CLOSEDBLANK" | "_CLOSED" => {
            (Some(DimensionArrow::Open), true)
        }
        "_DOT" | "_DOTSMALL" | "_DOTBLANK" | "_SMALL" => (Some(DimensionArrow::Dot), true),
        _ => (Some(DimensionArrow::Closed), false),
    }
}

/// What turning a DIMSTYLE into KentOS's said: an arrowhead's block KentOS
/// does not have, a DIMLFAC it does not.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DimSaid {
    pub arrow: Option<String>,
    pub lfac: Option<f64>,
}

/// The dimension style `v` is, its id `id`: sizes in paper mm (DIMSCALE
/// times them, drawing units turned into metres by `metres`, the project's
/// scale), its typeface the text style's (`font`), its line types the
/// LTYPE records' (`ltypes`, by handle).
#[allow(clippy::too_many_arguments)]
pub fn dimension_style(
    v: &DimVars,
    id: String,
    name: &str,
    blocks: &HashMap<u64, String>,
    ltypes: &HashMap<u64, LineType>,
    font: Option<DrawingFont>,
    metres: &dyn Fn(f64) -> f64,
    scale: f64,
) -> (DimensionStyleDef, DimSaid) {
    if let Some(own) = &v.kentos {
        return (
            DimensionStyleDef {
                id,
                name: name.to_owned(),
                ..own.clone()
            },
            DimSaid::default(),
        );
    }
    let k = v.dimscale.filter(|s| *s > 0.0).unwrap_or(1.0);
    let size = |x: f64| mm(x * k, metres, scale);
    // The value's height; a file without DIMTXT (or a size KentOS may not keep) takes Standart's 2.5 mm.
    let height = v
        .dimtxt
        .filter(|t| *t > 0.0)
        .map(size)
        .filter(|h| h.is_finite() && *h > 0.0 && *h <= kentos_contracts::MAX_STYLE_MM)
        .unwrap_or(kentos_contracts::STANDARD_DIMENSION_HEIGHT_MM);
    let mut said = DimSaid::default();
    let (arrow, arrow_size) = match v.dimtsz.filter(|t| *t > 0.0) {
        // Oblique ticks: DIMTSZ is half the tick's length along the line (a 45° tick of √2 × DIMTSZ).
        Some(t) => (None, Some(size(t) * std::f64::consts::SQRT_2)),
        // DIMASZ 0: AutoCAD draws no arrowheads (Yok).
        None if v.dimasz == Some(0.0) => (Some(DimensionArrow::None), None),
        None => {
            let block = v
                .dimblk
                .clone()
                .or_else(|| v.dimblk_handle.and_then(|h| blocks.get(&h).cloned()))
                .unwrap_or_default();
            let (a, known) = arrow_of(&block);
            if !known {
                said.arrow = Some(block);
            }
            (a, v.dimasz.filter(|a| *a > 0.0).map(size))
        }
    };
    let (prefix, suffix) = match v.dimpost.as_deref() {
        Some(p) if !p.is_empty() => match p.split_once("<>") {
            Some((pre, post)) => (Some(pre.to_owned()), Some(post.to_owned())),
            None => (None, Some(p.to_owned())),
        },
        _ => (None, None),
    };
    // DIMLFAC times the file's units in a metre: the value's units in a metre.
    let per_metre = |f: f64| f / metres(1.0);
    let near = |f: f64, k: f64| (f - k).abs() <= 1e-9 * k;
    let unit = match v
        .dimlfac
        .filter(|f| f.is_finite() && *f > 0.0)
        .map(per_metre)
    {
        None => None,
        Some(f) if near(f, 1.0) => None,
        Some(f) if near(f, 100.0) => Some(DrawingUnit::Cm),
        Some(f) if near(f, 1000.0) => Some(DrawingUnit::Mm),
        Some(_) => {
            said.lfac = v.dimlfac;
            None
        }
    };
    let affix = |s: Option<String>| {
        s.filter(|t| {
            let n = t.chars().count();
            n > 0 && n <= kentos_contracts::MAX_AFFIX && !t.chars().any(char::is_control)
        })
    };
    let within = |mm: f64, positive: bool| {
        (mm.is_finite()
            && mm <= kentos_contracts::MAX_STYLE_MM
            && if positive { mm > 0.0 } else { mm >= 0.0 }
            && mm / height <= kentos_contracts::MAX_DIMENSION_RATIO)
            .then_some(mm)
    };
    let style = DimensionStyleDef {
        id,
        name: name.to_owned(),
        height,
        arrow,
        arrow_size: arrow_size.and_then(|s| within(s, true)),
        ext_offset: v.dimexo.map(size).and_then(|s| within(s, false)),
        ext_beyond: v.dimexe.map(size).and_then(|s| within(s, false)),
        text_gap: v
            .dimgap
            .map(|g| size(g.abs()))
            .and_then(|s| within(s, false)),
        text_place: (v.dimtad == Some(0)).then_some(DimensionTextPlace::Centre),
        decimals: v
            .dimdec
            .filter(|d| (0..=i64::from(kentos_contracts::MAX_DIMENSION_DECIMALS)).contains(d))
            .map(|d| d as u32),
        unit,
        prefix: affix(prefix),
        suffix: affix(suffix),
        font,
        // Its lines (docs/adr/0205 §6): one type for both extension lines, the first's.
        dim_line_color: line_colour(v.dimclrd),
        dim_line_weight: line_weight_mm(v.dimlwd),
        dim_line_type: line_type_of(v.dimltype, ltypes),
        ext_color: line_colour(v.dimclre),
        ext_weight: line_weight_mm(v.dimlwe),
        ext_line_type: line_type_of(v.dimltex1, ltypes)
            .or_else(|| line_type_of(v.dimltex2, ltypes)),
        text_color: line_colour(v.dimclrt),
    };
    (style, said)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A dimension style's lines from its DIMSTYLE (docs/adr/0205 §6):
    /// DIMCLRD's ACI 1 is red, DIMCLRE's BYLAYER and DIMCLRT's 7 the object's
    /// own; DIMLWD's 35 hundredths are 0.35 mm, DIMLWE's BYBLOCK none;
    /// DIMLTYPE names a dashed LTYPE, DIMLTEX1 a continuous one, so the
    /// extension lines take DIMLTEX2's dotted one.
    #[test]
    fn a_style_s_lines_come_from_its_variables() {
        let v = DimVars {
            dimtxt: Some(2.5),
            dimclrd: Some(1),
            dimclre: Some(256),
            dimclrt: Some(7),
            dimlwd: Some(35),
            dimlwe: Some(-2),
            dimltype: Some(0x86),
            dimltex1: Some(0x26),
            dimltex2: Some(0x87),
            ..DimVars::default()
        };
        let ltypes = HashMap::from([
            (0x26, LineType::Continuous),
            (0x86, LineType::Dashed),
            (0x87, LineType::Dotted),
        ]);
        let (s, _) = dimension_style(
            &v,
            "dxf-dim-1".into(),
            "Renkli",
            &HashMap::new(),
            &ltypes,
            None,
            &|x| x,
            1000.0,
        );
        assert_eq!(
            (
                s.dim_line_color.as_deref(),
                s.ext_color.as_deref(),
                s.text_color.as_deref()
            ),
            (Some("#FF0000"), None, None)
        );
        assert_eq!((s.dim_line_weight, s.ext_weight), (Some(0.35), None));
        assert_eq!(
            (s.dim_line_type, s.ext_line_type),
            (Some(LineType::Dashed), Some(LineType::Dotted))
        );
        // DXF's weights only: 0, a negative one and one over 2.11 mm are none.
        assert_eq!(
            [Some(0), Some(-3), Some(212), Some(211), None].map(line_weight_mm),
            [None, None, None, Some(2.11), None]
        );
        assert_eq!(
            [Some(0), Some(256), Some(5)].map(line_colour),
            [None, None, Some("#0000FF".into())]
        );
    }

    /// An entity's own changes (ACAD's DSTYLE data) give its lines too: an
    /// ACI as a 1070, a weight as a 1070, a type's LTYPE as a 1005 handle.
    #[test]
    fn an_entity_s_own_changes_give_its_lines() {
        let text = "1001\nACAD\n1000\nDSTYLE\n1002\n{\n1070\n176\n1070\n3\n1070\n371\n1070\n50\n1070\n345\n1005\n86\n1070\n346\n1005\n87\n1002\n}\n";
        let mut lex = super::super::lexer::Lexer::new(text.as_bytes());
        let mut groups = Vec::new();
        while let Ok(Some(p)) = lex.next() {
            groups.push(p);
        }
        let dec = super::super::strings::Decoder {
            enc: crate::text::Encoding::Utf8,
        };
        let v = dim_overrides(&groups, dec);
        assert_eq!(
            (v.dimclrd, v.dimlwd, v.dimltype, v.dimltex1),
            (Some(3), Some(50), Some(0x86), Some(0x87))
        );
    }
}
