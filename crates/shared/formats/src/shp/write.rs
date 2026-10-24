//! ESRI Shapefile out (docs/adr/0514, plan/gis.md CBS-05): a layer's objects as one Shapefile of each geometry family
//! they have (points, lines, areas), every set its .shp, .shx, .dbf, .cpg and, when the project has a system the
//! writer can name, .prj. A Shapefile holds one geometry type: a layer with points and areas is two sets, named after
//! the layer with `_nokta`, `_cizgi`, `_alan`. What the format cannot hold is written as the nearest thing it can and
//! said, never dropped silently:
//!
//! - curves (circles, arcs, ellipses, bulged paths and areas) as chords within 0.1 mm of the curve; NURBS as before;
//! - an insert as its block's objects placed, one row a family with the insert's attributes;
//! - a record without a geometry as a row with a null shape (a layer of only records: a .dbf alone);
//! - a vertex without an elevation, beside ones with, as 0 (the format has no “none” there);
//! - a line's measures (M, docs/adr/0224) where its path is straight; a vertex without one as the specification's
//!   “no data” (below −10³⁸);
//! - field names longer than ten bytes cut and made unique, types as dBASE has them: text C (254 bytes at most),
//!   integer N (18 digits at most, else C so no reader takes it as a float), decimal N with its fraction digits (one
//!   at least, so a reader takes it for a real; 15 significant digits at most, else C), date D, yes or no L, date-time and time C; a value its field's rule
//!   refuses turns its column to C; empty text and no value are one in a C column (both spaces).
//!
//! The bytes are the same for the same drawing: the .dbf's date is 1 January 1980, as the zip writer's.

use std::collections::{BTreeMap, BTreeSet};

use kentos_contracts::{
    BlockDefinition, Entity, ExportReport, LayerField, LayerFieldKind, check_value,
};

use crate::blocks::Placing;
use crate::export_geom::{Geometry, Words, geometry};
use crate::gis::Ring;
use crate::report::Report;
use crate::text::windows1254_byte;

/// What the Shapefile says of what it cannot hold.
pub(crate) const SHAPEFILE: Words = Words {
    ring_turn: "Shapefile'ın kuralına çevrildi (dış sınır saat yönünde, delikler saat yönünün tersine); köşeler aynı",
    curve: "Shapefile'da eğri yok: eğriden en çok 0,1 mm sapan kirişlerle yazıldı",
    path_holes: "açık çizginin delikleri Shapefile'da gösterilemez; yazılmadı",
    arc_area: "Shapefile'da eğri yok: yayları eğriden en çok 0,1 mm sapan kirişlerle yazıldı",
    hatch: "deseni Shapefile'da gösterilemez; sınırı alan olarak yazıldı",
    text: "Shapefile'da yazı nesnesi yok; yazılmadı",
    dimension: "Shapefile'da ölçü nesnesi yok; yazılmadı",
    table: "Shapefile'da tablo nesnesi yok; yazılmadı",
    image: "Shapefile'da resim nesnesi yok; yazılmadı",
    cloud: "Shapefile'da nokta bulutu yok; yazılmadı",
    record: "geometrisiz kayıt boş şekille (Null Shape) yazıldı",
    raster: "Shapefile'da raster yok; yazılmadı",
    leader: "kırık çizgisi çizgi olarak yazıldı; ok başı ve not Shapefile'da gösterilemez",
    block_empty: "bloğunda Shapefile'a yazılabilecek nesne yok; yazılmadı",
    block: "yerleştirmeler bloklarının nesneleriyle, açılmış olarak yazıldı; her geometri türü kendi Shapefile'ında bir satır, yerleştirmenin öznitelikleriyle",
    infinite: "uçsuz doğru ve ışın Shapefile'da gösterilemez; yazılmadı",
    measure: "Shapefile'a çizginin ölçüleri (M) yazılmadı",
    mixed_z: None,
    chord: Some(1e-4),
    measures: true,
};

/// The text code page of the .dbf.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DbfEncoding {
    /// UTF-8, named in the .cpg (what QGIS and GDAL write today).
    Utf8,
    /// Windows-1254 (Turkish Windows), named in the .cpg and the language driver byte: for programs that read only
    /// the old code pages.
    Windows1254,
}

impl DbfEncoding {
    fn cpg(self) -> &'static str {
        match self {
            DbfEncoding::Utf8 => "UTF-8",
            DbfEncoding::Windows1254 => "1254",
        }
    }

    /// The language driver byte (offset 29): none for UTF-8 (the .cpg names it), 0xCA for Turkish Windows.
    fn driver(self) -> u8 {
        match self {
            DbfEncoding::Utf8 => 0,
            DbfEncoding::Windows1254 => 0xCA,
        }
    }

    /// `s` in the code page; a character it lacks is `?`, counted in `replaced`.
    fn encode(self, s: &str, replaced: &mut u32) -> Vec<u8> {
        match self {
            DbfEncoding::Utf8 => s.as_bytes().to_vec(),
            DbfEncoding::Windows1254 => s
                .chars()
                .map(|c| {
                    windows1254_byte(c).unwrap_or_else(|| {
                        *replaced += 1;
                        b'?'
                    })
                })
                .collect(),
        }
    }

    /// The bytes of `c` in the code page, if it has it.
    fn char_bytes(self, c: char) -> Option<Vec<u8>> {
        match self {
            DbfEncoding::Utf8 => Some(c.to_string().into_bytes()),
            DbfEncoding::Windows1254 => windows1254_byte(c).map(|b| vec![b]),
        }
    }
}

/// A layer to write: its name, its fields and its objects in the drawing's order.
pub struct Layer<'a> {
    pub id: &'a str,
    pub name: &'a str,
    pub fields: &'a [LayerField],
    pub objects: Vec<&'a Entity>,
}

/// What to write.
pub struct Input<'a> {
    pub layers: Vec<Layer<'a>>,
    pub blocks: &'a [BlockDefinition],
    /// The .prj's text: the project's system as WKT 1; none writes no .prj.
    pub prj: Option<String>,
    pub encoding: DbfEncoding,
}

/// One Shapefile written: its name (without extension), the layer it came from, its rows, its shape type's name
/// (`POLYGONZ` …; none for a table alone) and its files (extension, bytes).
pub struct Set {
    pub name: String,
    pub layer: String,
    pub rows: u32,
    pub shape: Option<&'static str>,
    pub files: Vec<(&'static str, Vec<u8>)>,
}

/// A column whose name is not its attribute's.
pub struct Renamed {
    pub set: String,
    pub attribute: String,
    pub column: String,
}

/// A column written as text that its field's type would not hold, and why.
pub struct AsText {
    pub set: String,
    pub column: String,
    pub reason: &'static str,
}

/// Everything written, and what could not be written as it is.
#[derive(Default)]
pub struct Written {
    pub sets: Vec<Set>,
    /// Objects not written, by kind (Turkish, as the GeoPackage writer names them).
    pub left: BTreeMap<String, u32>,
    /// Objects with curves written as chords.
    pub approximated: u32,
    pub renamed: Vec<Renamed>,
    pub as_text: Vec<AsText>,
    /// Text values cut to the column's 254 bytes.
    pub cut: u32,
    /// Characters the code page lacks, written `?`.
    pub replaced: u32,
    /// Columns past dBASE's 255, not written.
    pub columns_left: u32,
    /// Empty texts written (spaces: read back as no value).
    pub blank_text: u32,
    /// Texts ending in spaces (readers trim them).
    pub trailing_spaces: u32,
    pub report: ExportReport,
}

/// Shape type codes (ESRI Shapefile Technical Description, 1998).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Family {
    Point,
    Line,
    Area,
}

impl Family {
    fn suffix(self) -> &'static str {
        match self {
            Family::Point => "_nokta",
            Family::Line => "_cizgi",
            Family::Area => "_alan",
        }
    }
}

/// A row's shape in its family: its parts (a point family's parts are one point each).
#[derive(Clone, Debug, Default)]
struct Shape {
    parts: Vec<Ring>,
}

/// A row: its shape (none: a null shape) and the object whose attributes it carries.
struct Row<'a> {
    shape: Option<Shape>,
    of: &'a Entity,
}

/// Below this an M value is “no data”.
const NO_M: f64 = -1.0e39;

/// The kind a left-out object is counted under.
fn left_kind(e: &Entity) -> &'static str {
    match e {
        Entity::Text(_) => "yazı",
        Entity::Dimension(_) => "ölçü",
        Entity::Insert(_) => "blok",
        Entity::Table(_) => "tablo",
        Entity::Image(_) => "resim",
        Entity::Raster(_) => "raster",
        Entity::PointCloud(_) => "nokta bulutu",
        Entity::Xline(_) | Entity::Ray(_) => "yardımcı çizgi",
        _ => "başka nesne",
    }
}

/// Whether an object's line is a curve's chords (an insert: one of its block's).
fn curved(e: &Entity, blocks: &Placing) -> bool {
    use crate::geom::has_arcs;
    match e {
        Entity::Circle(_)
        | Entity::Arc(_)
        | Entity::Ellipse(_)
        | Entity::Spline(_)
        | Entity::Nurbs(_) => true,
        Entity::Polyline(p) | Entity::Polygon(p) => {
            p.bulges.as_deref().is_some_and(has_arcs)
                || p.holes
                    .iter()
                    .flatten()
                    .any(|h| h.bulges.as_deref().is_some_and(has_arcs))
                || p.parts
                    .iter()
                    .flatten()
                    .any(|q| q.bulges.as_deref().is_some_and(has_arcs))
        }
        Entity::Insert(i) => blocks.placed(i).iter().any(|q| curved(q, blocks)),
        _ => false,
    }
}

/// The geometry sorted into its families: each family's parts (an area's rings turned to the Shapefile's rule).
fn sort(g: Geometry, into: &mut BTreeMap<Family, Shape>) {
    match g {
        Geometry::Point(p, z) => into
            .entry(Family::Point)
            .or_default()
            .parts
            .push(Ring::new(vec![p], [z])),
        Geometry::Points(ps) => into
            .entry(Family::Point)
            .or_default()
            .parts
            .extend(ps.into_iter().map(|(p, z)| Ring::new(vec![p], [z]))),
        Geometry::Line(r) => into.entry(Family::Line).or_default().parts.push(r),
        Geometry::Lines(rs) => into.entry(Family::Line).or_default().parts.extend(rs),
        Geometry::Area(rings) => into
            .entry(Family::Area)
            .or_default()
            .parts
            .extend(rings.into_iter().map(turned)),
        Geometry::Areas(parts) => into
            .entry(Family::Area)
            .or_default()
            .parts
            .extend(parts.into_iter().flatten().map(turned)),
        Geometry::Collection(members) => {
            for m in members {
                sort(m, into);
            }
        }
    }
}

/// A closed ring the other way round: RFC 7946's outline (counter-clockwise) becomes the Shapefile's (clockwise),
/// a hole the other way; its first point stays first.
fn turned(mut r: Ring) -> Ring {
    r.pts.reverse();
    if let Some(zs) = r.zs.as_mut() {
        zs.reverse();
    }
    if let Some(ms) = r.ms.as_mut() {
        ms.reverse();
    }
    r
}

/// A layer's or a set's file name: the characters no file system takes as `_`; never empty.
fn file_name(name: &str) -> String {
    let s: String = name
        .trim()
        .chars()
        .map(|c| {
            if c.is_control() || matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') {
                '_'
            } else {
                c
            }
        })
        .collect();
    let s = s.trim_end_matches(['.', ' ']).to_owned();
    if s.is_empty() { "katman".into() } else { s }
}

/// The column's dBASE kind as written.
#[derive(Clone, Debug, PartialEq)]
enum Kind {
    /// C of a width (bytes).
    Text(usize),
    /// N of a width and fraction digits.
    Number(usize, usize),
    Date,
    Logical,
}

/// A column: its name in the file (≤ 10 bytes), its kind, the attribute it reads and the field whose rule makes its
/// values canonical (a typed column's).
struct Column {
    name: Vec<u8>,
    kind: Kind,
    key: String,
    field: Option<LayerField>,
}

/// A decimal's text with exactly `d` fraction digits; none when it has more, or is not a plain decimal.
fn fixed(v: &str, d: usize) -> Option<String> {
    let (neg, body) = v.strip_prefix('-').map_or((false, v), |b| (true, b));
    let (int, frac) = body.split_once('.').unwrap_or((body, ""));
    if int.is_empty() && frac.is_empty()
        || !int.bytes().all(|b| b.is_ascii_digit())
        || !frac.bytes().all(|b| b.is_ascii_digit())
        || frac.len() > d
    {
        return None;
    }
    let int = if int.is_empty() { "0" } else { int };
    let mut s = String::new();
    if neg {
        s.push('-');
    }
    s.push_str(int);
    if d > 0 {
        s.push('.');
        s.push_str(frac);
        s.extend(std::iter::repeat_n('0', d - frac.len()));
    }
    Some(s)
}

/// Significant digits of a plain decimal (leading zeros of the whole part aside).
fn significant(v: &str) -> usize {
    let body = v.trim_start_matches('-');
    let (int, frac) = body.split_once('.').unwrap_or((body, ""));
    int.trim_start_matches('0').len() + frac.len()
}

/// The values a column's attribute takes over the rows, canonical by its field's rule (none: no value).
fn canonical(field: Option<&LayerField>, raw: Option<&String>) -> Result<Option<String>, ()> {
    let Some(raw) = raw else {
        return Ok(None);
    };
    match field {
        Some(f) if f.kind != LayerFieldKind::Text => match check_value(f, raw) {
            Ok(c) if c.is_empty() => Ok(None),
            Ok(c) => Ok(Some(c)),
            Err(_) => Err(()),
        },
        _ => Ok(Some(raw.clone())),
    }
}

/// The column's kind from its field and the values it holds; why it is text when its field's type is not.
fn kind_of(
    field: Option<&LayerField>,
    values: &[Option<String>],
    enc: DbfEncoding,
) -> (Kind, Option<&'static str>) {
    let text_width = |len: Option<u32>| {
        let most = values
            .iter()
            .flatten()
            .map(|v| enc.encode(v, &mut 0).len())
            .max()
            .unwrap_or(0);
        Kind::Text(most.max(len.unwrap_or(0) as usize).clamp(1, 254))
    };
    let Some(f) = field else {
        return (text_width(None), None);
    };
    match f.kind {
        LayerFieldKind::Text => (text_width(f.length), None),
        // A 64-bit whole number as a whole number too (docs/adr/0629 §4).
        LayerFieldKind::Integer | LayerFieldKind::BigInt => {
            let width = values
                .iter()
                .flatten()
                .map(String::len)
                .max()
                .unwrap_or(1)
                .max(1);
            if width > 18 {
                (
                    text_width(None),
                    Some(
                        "18 haneyi aşan tam sayılar dBASE okuyucularında kayan noktaya döner; tam basamaklarıyla metin sütununa yazıldı",
                    ),
                )
            } else {
                (Kind::Number(width, 0), None)
            }
        }
        LayerFieldKind::Decimal => {
            let frac = |v: &String| v.split_once('.').map_or(0, |(_, f)| f.len());
            let d = values
                .iter()
                .flatten()
                .map(frac)
                .max()
                .unwrap_or(0)
                .max(f.scale.unwrap_or(0) as usize)
                // At least one: a reader takes N without fraction digits for an integer.
                .max(1);
            let digits = values
                .iter()
                .flatten()
                .map(|v| significant(v))
                .max()
                .unwrap_or(0);
            if d > 15 || digits > 15 {
                return (
                    text_width(None),
                    Some(
                        "15 anlamlı basamağı aşan ondalık sayılar dBASE okuyucularında yuvarlanır; tam basamaklarıyla metin sütununa yazıldı",
                    ),
                );
            }
            let width = values
                .iter()
                .flatten()
                .filter_map(|v| fixed(v, d))
                .map(|s| s.len())
                .max()
                .unwrap_or(if d > 0 { d + 2 } else { 1 })
                .max(1);
            (Kind::Number(width, d), None)
        }
        LayerFieldKind::Date => (Kind::Date, None),
        LayerFieldKind::Boolean => (Kind::Logical, None),
        LayerFieldKind::DateTime => (
            text_width(None),
            Some("dBASE'te tarih-saat türü yok; YYYY-AA-GGTSS:DD:SS metniyle yazıldı"),
        ),
        LayerFieldKind::Time => (
            text_width(None),
            Some("dBASE'te saat türü yok; SS:DD:SS metniyle yazıldı"),
        ),
        // docs/adr/0629 §4: what dBASE has no type for goes as text, said.
        LayerFieldKind::Instant => (
            text_width(None),
            Some("dBASE'te saat dilimli zaman türü yok; YYYY-AA-GGTSS:DD:SS ve dilimiyle metin olarak yazıldı"),
        ),
        LayerFieldKind::Uuid => (text_width(None), None),
        LayerFieldKind::Json => (
            text_width(None),
            Some("dBASE'te JSON türü yok; metin olarak yazıldı"),
        ),
        LayerFieldKind::Binary => (
            text_width(None),
            Some("dBASE'te ikili tür yok; base64 metni olarak yazıldı"),
        ),
    }
}

/// A name cut to `room` bytes in the code page, at a character's end, without a space at its end; a character the
/// code page lacks is `_`.
fn cut_name(name: &str, enc: DbfEncoding, room: usize) -> Vec<u8> {
    let mut out = Vec::new();
    for c in name.chars() {
        let b = enc.char_bytes(c).unwrap_or_else(|| vec![b'_']);
        if out.len() + b.len() > room {
            break;
        }
        out.extend(b);
    }
    // A name cut after a space would end in one: readers trim it, and the next name would match.
    while out.last() == Some(&b' ') {
        out.pop();
    }
    if out.is_empty() {
        out.extend_from_slice(b"ALAN");
    }
    out
}

fn shown(bytes: &[u8], enc: DbfEncoding) -> String {
    match enc {
        DbfEncoding::Utf8 => String::from_utf8_lossy(bytes).into_owned(),
        DbfEncoding::Windows1254 => crate::text::decode(bytes, crate::text::Encoding::Windows1254),
    }
}

/// The columns of a set: its layer's fields, then its other attributes by name; dBASE's ten-byte names made unique
/// whatever their case.
fn columns(
    set: &str,
    fields: &[LayerField],
    rows: &[Row],
    enc: DbfEncoding,
    out: &mut Written,
) -> Vec<Column> {
    let mut keys: Vec<(String, Option<&LayerField>)> = Vec::new();
    for f in fields {
        if !keys.iter().any(|(k, _)| *k == f.name) {
            keys.push((f.name.clone(), Some(f)));
        }
    }
    let extra: BTreeSet<&String> = rows.iter().flat_map(|r| r.of.base().attrs.keys()).collect();
    for k in extra {
        if !keys.iter().any(|(n, _)| n == k) {
            keys.push((k.clone(), None));
        }
    }
    if keys.len() > 255 {
        out.columns_left += (keys.len() - 255) as u32;
        keys.truncate(255);
    }
    let mut taken: BTreeSet<String> = BTreeSet::new();
    let mut cols = Vec::new();
    for (key, field) in keys {
        let mut name = cut_name(&key, enc, 10);
        let mut n = 1;
        while !taken.insert(shown(&name, enc).to_lowercase()) {
            let tail = format!("_{n}");
            name = cut_name(&key, enc, 10 - tail.len());
            name.extend_from_slice(tail.as_bytes());
            n += 1;
        }
        let shown_name = shown(&name, enc);
        if shown_name != key {
            out.renamed.push(Renamed {
                set: set.to_owned(),
                attribute: key.clone(),
                column: shown_name.clone(),
            });
        }
        let mut refused = false;
        let values: Vec<Option<String>> = rows
            .iter()
            .map(|r| {
                canonical(field, r.of.base().attrs.get(&key)).unwrap_or_else(|()| {
                    refused = true;
                    None
                })
            })
            .collect();
        let (kind, reason) = if refused {
            let raw: Vec<Option<String>> = rows
                .iter()
                .map(|r| r.of.base().attrs.get(&key).cloned())
                .collect();
            (
                kind_of(None, &raw, enc).0,
                Some(
                    "alanın kuralına uymayan değerleri var; değerler yazıldığı gibi metin sütununa yazıldı",
                ),
            )
        } else {
            kind_of(field, &values, enc)
        };
        if let Some(reason) = reason {
            out.as_text.push(AsText {
                set: set.to_owned(),
                column: shown_name.clone(),
                reason,
            });
        }
        let typed = matches!(kind, Kind::Number(..) | Kind::Date | Kind::Logical);
        cols.push(Column {
            name,
            kind,
            key,
            field: if typed { field.cloned() } else { None },
        });
    }
    cols
}

/// The .dbf of the rows.
fn dbf(
    cols: &[Column],
    rows: &[Row],
    enc: DbfEncoding,
    out: &mut Written,
) -> Result<Vec<u8>, String> {
    let width = |k: &Kind| match k {
        Kind::Text(w) | Kind::Number(w, _) => *w,
        Kind::Date => 8,
        Kind::Logical => 1,
    };
    let record_len = 1 + cols.iter().map(|c| width(&c.kind)).sum::<usize>();
    let header_len = 32 + 32 * cols.len() + 1;
    let (Ok(record_len16), Ok(header_len16), Ok(count)) = (
        u16::try_from(record_len),
        u16::try_from(header_len),
        u32::try_from(rows.len()),
    ) else {
        return Err(
            "dBASE tablosunun satırı 65 535 baytı aşıyor; daha az ya da daha kısa sütunla yazın."
                .into(),
        );
    };
    let mut b = Vec::with_capacity(header_len + record_len * rows.len() + 1);
    b.push(0x03);
    // 1 January 1980: the same drawing makes the same bytes.
    b.extend_from_slice(&[80, 1, 1]);
    b.extend_from_slice(&count.to_le_bytes());
    b.extend_from_slice(&header_len16.to_le_bytes());
    b.extend_from_slice(&record_len16.to_le_bytes());
    b.extend_from_slice(&[0; 17]);
    b.push(enc.driver());
    b.extend_from_slice(&[0, 0]);
    for c in cols {
        let mut d = [0u8; 32];
        d[..c.name.len()].copy_from_slice(&c.name);
        let (letter, w, dec) = match c.kind {
            Kind::Text(w) => (b'C', w, 0),
            Kind::Number(w, d) => (b'N', w, d),
            Kind::Date => (b'D', 8, 0),
            Kind::Logical => (b'L', 1, 0),
        };
        d[11] = letter;
        d[16] = w as u8;
        d[17] = dec as u8;
        b.extend_from_slice(&d);
    }
    b.push(0x0D);
    for r in rows {
        b.push(b' ');
        for c in cols {
            let raw = r.of.base().attrs.get(&c.key);
            match &c.kind {
                Kind::Text(w) => {
                    let mut cell = Vec::new();
                    if let Some(v) = raw {
                        if v.is_empty() {
                            out.blank_text += 1;
                        } else if v.ends_with(' ') {
                            out.trailing_spaces += 1;
                        }
                        for ch in v.chars() {
                            let bytes = match enc.char_bytes(ch) {
                                Some(bs) => bs,
                                None => {
                                    out.replaced += 1;
                                    vec![b'?']
                                }
                            };
                            if cell.len() + bytes.len() > *w {
                                out.cut += 1;
                                break;
                            }
                            cell.extend(bytes);
                        }
                    }
                    cell.resize(*w, b' ');
                    b.extend(cell);
                }
                Kind::Number(w, d) => {
                    let v = canonical(c.field.as_ref(), raw).ok().flatten();
                    let text = v.and_then(|v| fixed(&v, *d)).unwrap_or_default();
                    b.extend(std::iter::repeat_n(b' ', w - text.len().min(*w)));
                    b.extend(text.bytes().take(*w));
                }
                Kind::Date => {
                    let v = canonical(c.field.as_ref(), raw).ok().flatten();
                    let text = v.map(|v| v.replace('-', "")).filter(|t| t.len() == 8);
                    b.extend(text.unwrap_or_else(|| "        ".into()).bytes());
                }
                Kind::Logical => {
                    let v = canonical(c.field.as_ref(), raw).ok().flatten();
                    b.push(match v.as_deref() {
                        Some("true") => b'T',
                        Some("false") => b'F',
                        _ => b'?',
                    });
                }
            }
        }
    }
    b.push(0x1A);
    Ok(b)
}

/// A set's shape type: its family's code, a point family's the multi-point one when a row has several points; + 10
/// with elevations, + 20 with measures and no elevations.
fn shape_type(family: Family, multi: bool, z: bool, m: bool) -> (i32, &'static str) {
    let base = match (family, multi) {
        (Family::Point, false) => 1,
        (Family::Point, true) => 8,
        (Family::Line, _) => 3,
        (Family::Area, _) => 5,
    };
    let names: [&'static str; 3] = match base {
        1 => ["POINT", "POINTZ", "POINTM"],
        8 => ["MULTIPOINT", "MULTIPOINTZ", "MULTIPOINTM"],
        3 => ["POLYLINE", "POLYLINEZ", "POLYLINEM"],
        _ => ["POLYGON", "POLYGONZ", "POLYGONM"],
    };
    if z {
        (base + 10, names[1])
    } else if m {
        (base + 20, names[2])
    } else {
        (base, names[0])
    }
}

/// The least and greatest of `values`; (0, 0) for none.
fn range(values: impl Iterator<Item = f64>) -> (f64, f64) {
    let mut lo = f64::INFINITY;
    let mut hi = f64::NEG_INFINITY;
    for v in values {
        lo = lo.min(v);
        hi = hi.max(v);
    }
    if lo > hi { (0.0, 0.0) } else { (lo, hi) }
}

fn f64s(out: &mut Vec<u8>, values: &[f64]) {
    for v in values {
        out.extend_from_slice(&v.to_le_bytes());
    }
}

/// A shape's vertices: (x, y, z, m), a vertex without an elevation at 0 and without a measure at “no data”.
fn vertices(s: &Shape) -> Vec<(f64, f64, f64, f64)> {
    s.parts
        .iter()
        .flat_map(|r| {
            r.pts.iter().enumerate().map(move |(i, p)| {
                let z = r.zs.as_ref().and_then(|z| z.get(i).copied().flatten());
                let m = r.ms.as_ref().and_then(|m| m.get(i).copied().flatten());
                (p.x, p.y, z.unwrap_or(0.0), m.unwrap_or(NO_M))
            })
        })
        .collect()
}

/// A record's content (its shape type first).
fn content(t: i32, shape: Option<&Shape>, z: bool, m: bool) -> Vec<u8> {
    let mut c = Vec::new();
    let Some(s) = shape else {
        c.extend_from_slice(&0i32.to_le_bytes());
        return c;
    };
    c.extend_from_slice(&t.to_le_bytes());
    let v = vertices(s);
    let valid_m = || v.iter().map(|q| q.3).filter(|m| *m >= -1e38);
    match t {
        1 | 11 | 21 => {
            let q = v.first().copied().unwrap_or((0.0, 0.0, 0.0, NO_M));
            f64s(&mut c, &[q.0, q.1]);
            if t == 11 {
                f64s(&mut c, &[q.2, q.3]);
            } else if t == 21 {
                f64s(&mut c, &[q.3]);
            }
        }
        _ => {
            let (x0, x1) = range(v.iter().map(|q| q.0));
            let (y0, y1) = range(v.iter().map(|q| q.1));
            f64s(&mut c, &[x0, y0, x1, y1]);
            let multipoint = matches!(t, 8 | 18 | 28);
            if !multipoint {
                c.extend_from_slice(&(s.parts.len() as i32).to_le_bytes());
            }
            c.extend_from_slice(&(v.len() as i32).to_le_bytes());
            if !multipoint {
                let mut start = 0i32;
                for r in &s.parts {
                    c.extend_from_slice(&start.to_le_bytes());
                    start += r.pts.len() as i32;
                }
            }
            for q in &v {
                f64s(&mut c, &[q.0, q.1]);
            }
            if z {
                let (z0, z1) = range(v.iter().map(|q| q.2));
                f64s(&mut c, &[z0, z1]);
                for q in &v {
                    f64s(&mut c, &[q.2]);
                }
            }
            if m {
                let (m0, m1) = range(valid_m());
                f64s(&mut c, &[m0, m1]);
                for q in &v {
                    f64s(&mut c, &[q.3]);
                }
            }
        }
    }
    c
}

/// The 100-byte header of a .shp or .shx `len` bytes long.
fn header(t: i32, len: usize, bounds: [f64; 8]) -> Result<Vec<u8>, String> {
    let words = i32::try_from(len / 2).map_err(|_| {
        "Shapefile'ın bir dosyası 2 GB'ı aşıyor; katmanı bölün ya da GeoPackage'a yazın.".to_owned()
    })?;
    let mut h = Vec::with_capacity(100);
    h.extend_from_slice(&9994i32.to_be_bytes());
    h.extend_from_slice(&[0; 20]);
    h.extend_from_slice(&words.to_be_bytes());
    h.extend_from_slice(&1000i32.to_le_bytes());
    h.extend_from_slice(&t.to_le_bytes());
    f64s(&mut h, &bounds);
    Ok(h)
}

/// The .shp and .shx of the rows.
fn shp(t: i32, rows: &[Row], z: bool, m: bool) -> Result<(Vec<u8>, Vec<u8>), String> {
    let mut records = Vec::new();
    let mut index = Vec::new();
    let mut all: Vec<(f64, f64, f64, f64)> = Vec::new();
    for (i, r) in rows.iter().enumerate() {
        let c = content(t, r.shape.as_ref(), z, m);
        if let Some(s) = &r.shape {
            all.extend(vertices(s));
        }
        let offset = 100 + records.len();
        let (Ok(number), Ok(words), Ok(at)) = (
            i32::try_from(i + 1),
            i32::try_from(c.len() / 2),
            i32::try_from(offset / 2),
        ) else {
            return Err(
                "Shapefile'ın bir dosyası 2 GB'ı aşıyor; katmanı bölün ya da GeoPackage'a yazın."
                    .into(),
            );
        };
        records.extend_from_slice(&number.to_be_bytes());
        records.extend_from_slice(&words.to_be_bytes());
        records.extend(c);
        index.extend_from_slice(&at.to_be_bytes());
        index.extend_from_slice(&words.to_be_bytes());
    }
    let (x0, x1) = range(all.iter().map(|q| q.0));
    let (y0, y1) = range(all.iter().map(|q| q.1));
    let (z0, z1) = if z {
        range(all.iter().map(|q| q.2))
    } else {
        (0.0, 0.0)
    };
    let (m0, m1) = if m {
        range(all.iter().map(|q| q.3).filter(|v| *v >= -1e38))
    } else {
        (0.0, 0.0)
    };
    let bounds = [x0, y0, x1, y1, z0, z1, m0, m1];
    let mut main = header(t, 100 + records.len(), bounds)?;
    main.extend(records);
    let mut shx = header(t, 100 + index.len(), bounds)?;
    shx.extend(index);
    Ok((main, shx))
}

/// Writes the layers as Shapefiles.
pub fn write(input: &Input) -> Result<Written, String> {
    let blocks = Placing::new(input.blocks);
    let mut rep = Report::default();
    let mut out = Written::default();
    let mut names: BTreeSet<String> = BTreeSet::new();
    let mut zero_z = 0u32;
    for layer in &input.layers {
        let mut families: BTreeMap<Family, Vec<Row>> = BTreeMap::new();
        let mut records: Vec<&Entity> = Vec::new();
        for &e in &layer.objects {
            if matches!(e, Entity::Record(_)) {
                records.push(e);
                rep.count(e.kind());
                continue;
            }
            let Some(g) = geometry(e, &blocks, &mut rep, &SHAPEFILE) else {
                *out.left.entry(left_kind(e).to_owned()).or_default() += 1;
                continue;
            };
            if curved(e, &blocks) {
                out.approximated += 1;
            }
            let mut sorted = BTreeMap::new();
            sort(g, &mut sorted);
            for (family, shape) in sorted {
                if !shape.parts.is_empty() {
                    families.entry(family).or_default().push(Row {
                        shape: Some(shape),
                        of: e,
                    });
                }
            }
            rep.count(e.kind());
        }
        if families.is_empty() && records.is_empty() {
            continue;
        }
        let base = file_name(layer.name);
        let several = families.len() > 1;
        let mut groups: Vec<(Option<Family>, Vec<Row>)> = families
            .into_iter()
            .map(|(f, rows)| (Some(f), rows))
            .collect();
        let nulls = records.into_iter().map(|of| Row { shape: None, of });
        match groups.first_mut() {
            Some((_, rows)) => rows.extend(nulls),
            None => groups.push((None, nulls.collect())),
        }
        for (family, rows) in groups {
            let mut name = match family {
                Some(f) if several => format!("{base}{}", f.suffix()),
                _ => base.clone(),
            };
            let stem = name.clone();
            let mut k = 2;
            while !names.insert(name.to_lowercase()) {
                name = format!("{stem}_{k}");
                k += 1;
            }
            let cols = columns(&name, layer.fields, &rows, input.encoding, &mut out);
            let table = dbf(&cols, &rows, input.encoding, &mut out)?;
            let mut files: Vec<(&'static str, Vec<u8>)> = Vec::new();
            let mut shape_name = None;
            if let Some(f) = family {
                let shapes = || rows.iter().filter_map(|r| r.shape.as_ref());
                let has = |zs: &crate::gis::Zs| zs.iter().flatten().any(Option::is_some);
                let z = shapes().any(|s| s.parts.iter().any(|r| has(&r.zs)));
                let m = shapes().any(|s| s.parts.iter().any(|r| has(&r.ms)));
                if z {
                    zero_z += shapes()
                        .filter(|s| {
                            s.parts.iter().any(|r| {
                                r.zs.as_ref().is_none_or(|z| z.iter().any(Option::is_none))
                            })
                        })
                        .count() as u32;
                }
                let multi = f == Family::Point && shapes().any(|s| s.parts.len() > 1);
                let (t, tname) = shape_type(f, multi, z, m);
                let (main, index) = shp(t, &rows, z, m)?;
                files.push(("shp", main));
                files.push(("shx", index));
                shape_name = Some(tname);
            }
            files.push(("dbf", table));
            if family.is_some()
                && let Some(prj) = &input.prj
            {
                files.push(("prj", prj.clone().into_bytes()));
            }
            files.push(("cpg", input.encoding.cpg().as_bytes().to_vec()));
            out.sets.push(Set {
                name,
                layer: layer.id.to_owned(),
                rows: rows.len() as u32,
                shape: shape_name,
                files,
            });
        }
    }
    rep.note_n(
        "Kotsuz köşe",
        "kotu olan nesnelerle aynı Shapefile'daki kotsuz köşeler 0 kotla yazıldı (Shapefile'da köşenin kotsuz olması yok)",
        0,
        zero_z,
    );
    rep.note_n(
        "Boş metin",
        "boş metin ile değerin yokluğu dBASE'te ayrılamaz: ikisi de boşlukla yazıldı, geri okumada değer yoktur",
        0,
        out.blank_text,
    );
    rep.note_n(
        "Sondaki boşluk",
        "metnin sonundaki boşlukları dBASE okuyucuları siler",
        0,
        out.trailing_spaces,
    );
    rep.note_n(
        "Kısaltılan metin",
        "dBASE metin sütunu en çok 254 bayt alır; uzun metin kesildi",
        0,
        out.cut,
    );
    rep.note_n(
        "Kod sayfasında olmayan harf",
        "Windows-1254'te olmayan harfler ? olarak yazıldı",
        0,
        out.replaced,
    );
    rep.note_n(
        "Sütun",
        "dBASE en çok 255 sütun alır; fazlası yazılmadı",
        0,
        out.columns_left,
    );
    for r in &out.renamed {
        rep.note(
            "Sütun adı",
            &format!("“{}” özniteliği “{}” sütununa yazıldı (dBASE adı en çok 10 bayt, büyük küçük harf ayrılmaz)", r.attribute, r.column),
            0,
        );
    }
    for a in &out.as_text {
        rep.note("Sütun türü", &format!("{}: {}", a.column, a.reason), 0);
    }
    out.report = rep.export();
    Ok(out)
}
