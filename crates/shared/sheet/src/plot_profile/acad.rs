//! AutoCAD plot style tables read into a KentOS plot profile (docs/adr/1130): a colour-dependent `.ctb` (255 pens, one
//! per ACI 1–255) or a named `.stb`. The file is a 60-byte header — `PIAFILEVERSION_2.0,CTBVER1,compress\r\npmzlibcodec`
//! (or `STBVER1`), then three little-endian u32: an Adler-32, the inflated length and the compressed length — and a
//! zlib stream of `key=value` text in `name{ … }` blocks. Every field is mapped explicitly; what KentOS cannot draw the
//! same way is named in the report (lost, approximate or a note), never cut silently (docs/adr/0609 §3). Nothing is
//! written back: a `.ctb`/`.stb` writer is not part of this reader.
use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use super::{PlotColor, PlotProfile, PlotStyle, StyleEntry, StyleKey, TableKind};
use crate::{LineCap, LineJoin, Result, SheetError};

/// The most bytes a table file is read from, and the most its text inflates to.
pub const MAX_FILE: usize = 4 * 1024 * 1024;
pub const MAX_TEXT: usize = 8 * 1024 * 1024;
/// The header's fixed words before the three numbers.
const HEAD_CTB: &[u8] = b"PIAFILEVERSION_2.0,CTBVER1,compress\r\npmzlibcodec";
const HEAD_STB: &[u8] = b"PIAFILEVERSION_2.0,STBVER1,compress\r\npmzlibcodec";
const HEADER: usize = 60;
/// A colour that is the object's: `-1`, and the ACI-typed `0xC3FFFFFF` some writers give.
const OBJECT_COLOR: i64 = -1;
const OBJECT_COLOR2: i64 = -1_006_632_961;
/// The linetype that is the object's; 0 is continuous, 1–30 AutoCAD's own plot patterns.
const OBJECT_LINETYPE: i64 = 31;
/// AutoCAD's plot linetypes 1–30, as its editor names them (0 is Solid).
pub const LINETYPES: [&str; 31] = [
    "Solid",
    "Dashed",
    "Dotted",
    "Dash Dot",
    "Short Dash",
    "Medium Dash",
    "Long Dash",
    "Short Dash x2",
    "Medium Dash x2",
    "Long Dash x2",
    "Medium Long Dash",
    "Medium Dash Short Dash Short Dash",
    "Long Dash Short Dash",
    "Long Dash Dot Dot",
    "Long Dash Dot",
    "Medium Dash Dot Short Dash Dot",
    "Sparse Dot",
    "ISO Dash",
    "ISO Dash Space",
    "ISO Long Dash Dot",
    "ISO Long Dash Double Dot",
    "ISO Long Dash Triple Dot",
    "ISO Dot",
    "ISO Long Dash Short Dash",
    "ISO Long Dash Double Short Dash",
    "ISO Dash Dot",
    "ISO Double Dash Dot",
    "ISO Dash Double Dot",
    "ISO Double Dash Double Dot",
    "ISO Dash Triple Dot",
    "ISO Double Dash Triple Dot",
];

/// Which AutoCAD table a file is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/", rename = "PlotStyleFormat"))]
#[cfg_attr(feature = "schema", schemars(rename = "PaperPlotStyleFormat"))]
#[serde(rename_all = "lowercase")]
pub enum StyleFormat {
    Ctb,
    Stb,
}

/// Where an imported profile came from: the file's format, the SHA-256 of its bytes and its name (docs/adr/0609 §1).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/", rename = "PlotSource"))]
#[cfg_attr(feature = "schema", schemars(rename = "PaperPlotSource"))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlotSource {
    pub format: StyleFormat,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = r"^[0-9a-f]{64}$")))]
    pub sha256: String,
    /// The file's name, without its folder.
    pub file: String,
}

/// How much a reported field differs from what AutoCAD plots.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/", rename = "PlotLossSeverity"))]
#[cfg_attr(feature = "schema", schemars(rename = "PaperPlotLossSeverity"))]
#[serde(rename_all = "camelCase")]
pub enum Severity {
    /// Not carried: the object's own look is plotted instead.
    Lost,
    /// Carried by KentOS's nearest rule, which may differ from AutoCAD's.
    Approximate,
    /// Said for the record; what is plotted is the same.
    Note,
}

/// What a report line is about, in the order the report lists them (docs/adr/1130 §3).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/", rename = "PlotLossKind"))]
#[cfg_attr(feature = "schema", schemars(rename = "PaperPlotLossKind"))]
#[serde(rename_all = "camelCase")]
pub enum LossKind {
    ColorKind,
    ModeColor,
    Grayscale,
    GrayscaleColor,
    Dither,
    ScreenWithoutDither,
    PolicyBits,
    PhysicalPen,
    VirtualPen,
    Linetype,
    LineweightOutside,
    LineweightRounded,
    EndStyle,
    JoinStyle,
    FillStyle,
    Description,
    NormalChanged,
    UnknownField,
    ScaleFactor,
    Checksum,
    Length,
}

impl LossKind {
    pub fn severity(self) -> Severity {
        use LossKind::*;
        match self {
            Grayscale | GrayscaleColor | Dither | LineweightRounded => Severity::Approximate,
            ScreenWithoutDither | Checksum | Length => Severity::Note,
            _ => Severity::Lost,
        }
    }

    /// The report's words for it; the styles and the values it names follow them.
    pub fn words(self) -> &'static str {
        use LossKind::*;
        match self {
            ColorKind => "Renk türü RGB ya da ACI değil; nesnenin rengi basılır.",
            ModeColor => "mode_color rengi color'dan farklı; anlamı belgelenmediği için kullanılmadı.",
            Grayscale => "Gri tonlama KentOS'un sRGB parlaklık kuralıyla (0,2126 R + 0,7152 G + 0,0722 B) yapılır; AutoCAD'in dönüşümü farklı olabilir.",
            GrayscaleColor => "Gri tonlama ve açık renk birlikte: rengin gri karşılığı parlaklık kuralıyla hesaplandı.",
            Dither => "Titreşim (dither) yok: tarama beyaz kâğıda düz karışım rengi olarak basılır.",
            ScreenWithoutDither => "Titreşim kapalıyken AutoCAD taramayı uygulamaz; renk tam yoğunlukta basılır.",
            PolicyBits => "color_policy'de bilinmeyen bitler var; yok sayıldı.",
            PhysicalPen => "Fiziksel kalem numarası kalemli çiziciye özgüdür; PDF ve resim çıktısında kullanılmaz.",
            VirtualPen => "Sanal kalem numarası sürücüye özgüdür; kullanılmaz.",
            Linetype => "AutoCAD'in baskı çizgi tipi KentOS desenine eşlenmedi; nesnenin çizgi tipi basılır.",
            LineweightOutside => "Kalınlık kalınlık tablosunun dışında; nesnenin kalınlığı basılır (bu değerin AutoCAD'de “Nesnenin kalınlığı” olduğu bağımsız olarak doğrulanmadı).",
            LineweightRounded => "Kalınlık mikrometreye yuvarlandı (yarım sıfırdan uzağa).",
            EndStyle => "Elmas uç KentOS'ta yok; nesnenin ucu basılır.",
            JoinStyle => "Elmas birleşim KentOS'ta yok; nesnenin birleşimi basılır.",
            FillStyle => "Dolgu deseni eşlenmedi; nesnenin dolgusu basılır.",
            Description => "Stil açıklaması baskı profiline taşınmadı.",
            NormalChanged => "Normal stili KentOS'ta nesnenin görünüşüdür; dosyadaki değişiklikleri alınmadı.",
            UnknownField => "Bilinmeyen alanlar okunmadı.",
            ScaleFactor => "ISO dışı çizgi tipi ve dolgu ölçeği uygulanmadı (çizgi tipi ve dolgu eşlenmediği için).",
            Checksum => "Üst bilgideki Adler-32 sıkıştırılmış veriye uymuyor; zlib akışının kendi denetimi geçti.",
            Length => "Üst bilgideki açılmış uzunluk metne uymuyor; metin zlib akışından okundu.",
        }
    }
}

/// One line of the report: a kind, the styles it touches (an ACI as its number, a named style by its name) and the
/// file's values that made it, each once in first-seen order.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/", rename = "PlotLoss"))]
#[cfg_attr(feature = "schema", schemars(rename = "PaperPlotLoss"))]
#[serde(rename_all = "camelCase")]
pub struct Loss {
    pub kind: LossKind,
    pub severity: Severity,
    pub words: String,
    pub styles: Vec<String>,
    pub values: Vec<String>,
}

/// A table read: its profile (with its source) and what the reading said.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/", rename = "PlotStyleRead"))]
#[cfg_attr(feature = "schema", schemars(rename = "PaperPlotStyleRead"))]
#[serde(rename_all = "camelCase")]
pub struct Read {
    pub profile: PlotProfile,
    /// The table's own description.
    pub description: String,
    /// Styles in the file (a CTB's 255 pens; a STB's styles, Normal among them).
    pub styles: u32,
    /// Styles carried without a lost or approximate field.
    pub exact: u32,
    pub losses: Vec<Loss>,
}

fn bad(code: &str, message: impl Into<String>) -> SheetError {
    SheetError::new(code, message)
}

fn unreadable(message: impl Into<String>) -> SheetError {
    bad("bad_plot_style_file", message)
}

/// A node of the text: a value, or a block of named nodes in their order.
enum Node {
    Value(String),
    Block(Vec<(String, Node)>),
}

/// A value as written: a string's leading quote taken off (the writer gives no closing one); another value's
/// ` (…)` appendix dropped, as the format's readers do.
fn value(raw: &str) -> String {
    let v = raw.trim();
    if let Some(s) = v.strip_prefix('"') {
        return s.to_owned();
    }
    if v.ends_with(')') {
        return v.split(' ').next().unwrap_or_default().to_owned();
    }
    v.to_owned()
}

fn parse(text: &str) -> Result<Vec<(String, Node)>> {
    let mut stack: Vec<(String, Vec<(String, Node)>)> = vec![(String::new(), Vec::new())];
    for (n, line) in text.split('\n').enumerate() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if t == "}" {
            if stack.len() < 2 {
                return Err(unreadable(format!("{}. satırda açılmamış bir blok kapanıyor.", n + 1)));
            }
            let (name, items) = stack.pop().unwrap_or_default();
            if let Some(top) = stack.last_mut() {
                top.1.push((name, Node::Block(items)));
            }
        } else if let Some(name) = t.strip_suffix('{') {
            if stack.len() > 4 {
                return Err(unreadable(format!("{}. satırda bloklar çok derin.", n + 1)));
            }
            stack.push((name.trim().to_owned(), Vec::new()));
        } else if let Some((k, v)) = t.split_once('=') {
            if let Some(top) = stack.last_mut() {
                top.1.push((k.trim().to_owned(), Node::Value(value(v))));
            }
        } else {
            return Err(unreadable(format!("{}. satır “ad=değer” ya da blok değil.", n + 1)));
        }
    }
    if stack.len() != 1 {
        return Err(unreadable("Bir blok kapanmadan dosya bitiyor."));
    }
    Ok(stack.pop().map(|s| s.1).unwrap_or_default())
}

fn int(v: &str, what: &str) -> Result<i64> {
    v.parse::<i64>()
        .map_err(|_| unreadable(format!("{what} bir tamsayı olmalı (“{v}”).")))
}

fn boolean(v: &str, what: &str) -> Result<bool> {
    match v.to_ascii_uppercase().as_str() {
        "TRUE" => Ok(true),
        "FALSE" => Ok(false),
        _ => Err(unreadable(format!("{what} TRUE ya da FALSE olmalı (“{v}”)."))),
    }
}

/// Millimetres as written to micrometres: exact to three decimals, else rounded half away from zero (and said).
fn micrometres(v: &str, what: &str) -> Result<(i32, bool)> {
    let wrong = || unreadable(format!("{what} 0 ile 100 mm arasında ondalık bir sayı olmalı (“{v}”)."));
    let (whole, frac) = v.split_once('.').unwrap_or((v, ""));
    if whole.is_empty() || !whole.bytes().all(|b| b.is_ascii_digit()) || !frac.bytes().all(|b| b.is_ascii_digit())
    {
        return Err(wrong());
    }
    let whole: i64 = whole.parse().map_err(|_| wrong())?;
    let digits: Vec<i64> = frac.bytes().map(|b| i64::from(b - b'0')).collect();
    let at = |k: usize| digits.get(k).copied().unwrap_or(0);
    let mut um = whole.saturating_mul(1000) + at(0) * 100 + at(1) * 10 + at(2);
    let rest = digits.iter().skip(3).any(|d| *d != 0);
    if at(3) >= 5 {
        um += 1;
    }
    if um > 100_000 {
        return Err(wrong());
    }
    Ok((um as i32, rest))
}

/// The losses of the read, gathered by kind.
#[derive(Default)]
struct Report {
    lines: std::collections::BTreeMap<LossKind, (Vec<String>, Vec<String>)>,
}

impl Report {
    fn add(&mut self, kind: LossKind, style: Option<&str>, value: Option<String>) {
        let (styles, values) = self.lines.entry(kind).or_default();
        if let Some(s) = style
            && !styles.iter().any(|x| x == s)
        {
            styles.push(s.to_owned());
        }
        if let Some(v) = value
            && !values.contains(&v)
        {
            values.push(v);
        }
    }

    fn done(self) -> Vec<Loss> {
        self.lines
            .into_iter()
            .map(|(kind, (styles, values))| Loss {
                kind,
                severity: kind.severity(),
                words: kind.words().to_owned(),
                styles,
                values,
            })
            .collect()
    }
}

/// The fields a style knows; another is reported as unknown.
const FIELDS: [&str; 16] = [
    "name",
    "localized_name",
    "description",
    "color",
    "mode_color",
    "color_policy",
    "physical_pen_number",
    "virtual_pen_number",
    "screen",
    "linepattern_size",
    "linetype",
    "adaptive_linetype",
    "lineweight",
    "fill_style",
    "end_style",
    "join_style",
];

/// One style's fields read into a KentOS style; `key` names it in the report. Returns whether it lost or approximated
/// anything.
fn style(
    fields: &[(String, Node)],
    key: &str,
    weights: &[(i32, bool)],
    report: &mut Report,
) -> Result<(PlotStyle, bool, String)> {
    let mut s = PlotStyle::default();
    let before: usize = report.lines.iter().filter(|(k, _)| k.severity() != Severity::Note).map(|(_, (st, _))| st.iter().filter(|x| *x == key).count()).sum();
    let get = |name: &str| {
        fields.iter().find_map(|(k, n)| match n {
            Node::Value(v) if k == name => Some(v.as_str()),
            _ => None,
        })
    };
    let at = |f: &str| format!("Stil {key}: {f}");
    for (k, n) in fields {
        if !FIELDS.contains(&k.as_str()) || matches!(n, Node::Block(_)) {
            report.add(LossKind::UnknownField, Some(key), Some(k.clone()));
        }
    }
    // Colour: the object's, or an RGB/ACI-typed 0xTTRRGGBB written as a signed number.
    let color = match get("color") {
        Some(v) => int(v, &at("color"))?,
        None => OBJECT_COLOR,
    };
    if !(-(1_i64 << 31)..(1_i64 << 32)).contains(&color) {
        return Err(unreadable(format!("{} 32 bitlik bir renk olmalı.", at("color"))));
    }
    let mut rgb = None;
    if color != OBJECT_COLOR && color != OBJECT_COLOR2 {
        let u = (color as u64 & 0xFFFF_FFFF) as u32;
        let kind = u >> 24;
        if kind == 0xC2 || kind == 0xC3 {
            rgb = Some([(u >> 16) as u8, (u >> 8) as u8, u as u8]);
        } else {
            report.add(LossKind::ColorKind, Some(key), Some(format!("{kind:02X}")));
        }
        if let Some(m) = get("mode_color")
            && int(m, &at("mode_color"))? != color
        {
            report.add(LossKind::ModeColor, Some(key), None);
        }
    }
    let policy = match get("color_policy") {
        Some(v) => int(v, &at("color_policy"))?,
        None => 1,
    };
    if !(0..=255).contains(&policy) {
        return Err(unreadable(format!("{} 0–255 olmalı.", at("color_policy"))));
    }
    if policy & !7 != 0 {
        report.add(LossKind::PolicyBits, Some(key), Some(policy.to_string()));
    }
    let (dither, gray) = (policy & 1 != 0, policy & 2 != 0);
    s.color = match (rgb, gray) {
        (None, false) => PlotColor::Object,
        (None, true) => {
            report.add(LossKind::Grayscale, Some(key), None);
            PlotColor::Grayscale
        }
        (Some([r, g, b]), false) => PlotColor::Color {
            value: format!("#{r:02x}{g:02x}{b:02x}"),
        },
        (Some([r, g, b]), true) => {
            report.add(LossKind::GrayscaleColor, Some(key), None);
            let y = (2126 * u32::from(r) + 7152 * u32::from(g) + 722 * u32::from(b) + 5000) / 10000;
            PlotColor::Color {
                value: format!("#{y:02x}{y:02x}{y:02x}"),
            }
        }
    };
    let screen = match get("screen") {
        Some(v) => int(v, &at("screen"))?,
        None => 100,
    };
    if !(0..=100).contains(&screen) {
        return Err(unreadable(format!("{} 0–100 olmalı.", at("screen"))));
    }
    if screen < 100 {
        if dither {
            report.add(LossKind::Dither, Some(key), None);
            s.screening = screen as u8;
        } else {
            report.add(LossKind::ScreenWithoutDither, Some(key), Some(screen.to_string()));
        }
    }
    for (field, kind) in [("physical_pen_number", LossKind::PhysicalPen), ("virtual_pen_number", LossKind::VirtualPen)] {
        let n = match get(field) {
            Some(v) => int(v, &at(field))?,
            None => 0,
        };
        if !(0..=255).contains(&n) {
            return Err(unreadable(format!("{} 0–255 olmalı.", at(field))));
        }
        if n != 0 {
            report.add(kind, Some(key), Some(n.to_string()));
        }
    }
    let linetype = match get("linetype") {
        Some(v) => int(v, &at("linetype"))?,
        None => OBJECT_LINETYPE,
    };
    match linetype {
        0 => s.dash = Some(Vec::new()),
        1..=30 => report.add(LossKind::Linetype, Some(key), Some(format!("{linetype}: {}", LINETYPES[linetype as usize]))),
        OBJECT_LINETYPE => {}
        _ => return Err(unreadable(format!("{} 0–31 olmalı.", at("linetype")))),
    }
    if let Some(v) = get("adaptive_linetype") {
        boolean(v, &at("adaptive_linetype"))?;
    }
    if let Some(v) = get("linepattern_size")
        && v.parse::<f64>().map_or(true, |x| !x.is_finite())
    {
        return Err(unreadable(format!("{} bir sayı olmalı.", at("linepattern_size"))));
    }
    if let Some(v) = get("lineweight") {
        let i = int(v, &at("lineweight"))?;
        if i < 0 {
            return Err(unreadable(format!("{} negatif olamaz.", at("lineweight"))));
        }
        match weights.get(i as usize) {
            // A table value of 0 is the object's lineweight, as the format's reference reader reads it.
            Some((0, _)) => {}
            Some((um, rounded)) => {
                s.line_weight = Some(*um);
                if *rounded {
                    report.add(LossKind::LineweightRounded, Some(key), Some(i.to_string()));
                }
            }
            None => report.add(LossKind::LineweightOutside, Some(key), Some(i.to_string())),
        }
    }
    let end = match get("end_style") {
        Some(v) => int(v, &at("end_style"))?,
        None => 4,
    };
    s.cap = match end {
        0 => Some(LineCap::Butt),
        1 => Some(LineCap::Square),
        2 => Some(LineCap::Round),
        3 => {
            report.add(LossKind::EndStyle, Some(key), None);
            None
        }
        4 => None,
        _ => return Err(unreadable(format!("{} 0–4 olmalı.", at("end_style")))),
    };
    let join = match get("join_style") {
        Some(v) => int(v, &at("join_style"))?,
        None => 5,
    };
    s.join = match join {
        0 => Some(LineJoin::Miter),
        1 => Some(LineJoin::Bevel),
        2 => Some(LineJoin::Round),
        3 => {
            report.add(LossKind::JoinStyle, Some(key), None);
            None
        }
        5 => None,
        _ => return Err(unreadable(format!("{} 0–3 ya da 5 olmalı.", at("join_style")))),
    };
    let fill = match get("fill_style") {
        Some(v) => int(v, &at("fill_style"))?,
        None => 73,
    };
    match fill {
        73 => {}
        64..=72 => report.add(LossKind::FillStyle, Some(key), Some(fill.to_string())),
        _ => return Err(unreadable(format!("{} 64–73 olmalı.", at("fill_style")))),
    }
    if get("description").is_some_and(|d| !d.is_empty()) {
        report.add(LossKind::Description, Some(key), None);
    }
    let after: usize = report.lines.iter().filter(|(k, _)| k.severity() != Severity::Note).map(|(_, (st, _))| st.iter().filter(|x| *x == key).count()).sum();
    Ok((s, after > before, get("name").unwrap_or_default().to_owned()))
}

/// The SHA-256 of the bytes, lowercase hex.
fn sha256(bytes: &[u8]) -> String {
    crate::template::sha256_hex(bytes)
}

/// Adler-32 (RFC 1950 §8.2).
fn adler32(bytes: &[u8]) -> u32 {
    let (mut a, mut b) = (1u32, 0u32);
    for chunk in bytes.chunks(5552) {
        for &x in chunk {
            a += u32::from(x);
            b += a;
        }
        a %= 65521;
        b %= 65521;
    }
    (b << 16) | a
}

/// Reads a `.ctb`/`.stb` file's bytes into a profile `id` named `name`; `file` is the file's name (its source's).
pub fn read(bytes: &[u8], id: &str, name: &str, file: &str) -> Result<Read> {
    if bytes.len() > MAX_FILE {
        return Err(bad("plot_style_file_too_large", "Baskı stili tablosu 4 MiB sınırını aşıyor."));
    }
    let format = if bytes.starts_with(HEAD_CTB) {
        StyleFormat::Ctb
    } else if bytes.starts_with(HEAD_STB) {
        StyleFormat::Stb
    } else {
        return Err(bad(
            "unsupported_plot_style_file",
            "Dosya bir AutoCAD baskı stili tablosu (.ctb/.stb, PIAFILEVERSION_2.0 üst bilgisi) değil; KentOS profili için .kplot'u Baskı profilleri penceresinden açın.",
        ));
    };
    if bytes.len() < HEADER {
        return Err(unreadable("Dosyanın üst bilgisi eksik."));
    }
    let word = |k: usize| u32::from_le_bytes([bytes[48 + 4 * k], bytes[49 + 4 * k], bytes[50 + 4 * k], bytes[51 + 4 * k]]);
    let (sum, length, packed) = (word(0), word(1), word(2));
    let body = &bytes[HEADER..];
    if packed as usize != body.len() {
        return Err(unreadable(format!(
            "Üst bilgi {packed} sıkıştırılmış bayt diyor, dosyada {} var; dosya kesilmiş ya da bozuk.",
            body.len()
        )));
    }
    let text = miniz_oxide::inflate::decompress_to_vec_zlib_with_limit(body, MAX_TEXT).map_err(|_| {
        unreadable("Sıkıştırılmış metin açılamadı (zlib akışı bozuk ya da 8 MiB sınırını aşıyor).")
    })?;
    let mut report = Report::default();
    if adler32(body) != sum {
        report.add(LossKind::Checksum, None, None);
    }
    if length as usize != text.len() {
        report.add(LossKind::Length, None, Some(format!("{length} ≠ {}", text.len())));
    }
    let mut end = text.len();
    while end > 0 && text[end - 1] == 0 {
        end -= 1;
    }
    let text = std::str::from_utf8(&text[..end]).map_err(|_| {
        bad(
            "plot_style_encoding",
            "Tablonun metni UTF-8 değil; kod sayfası bilinmediği için adlar okunamaz. Adları ASCII olan bir tablo kullanın.",
        )
    })?;
    let top = parse(text)?;
    let get = |name: &str| {
        top.iter().find_map(|(k, n)| match n {
            Node::Value(v) if k == name => Some(v.as_str()),
            _ => None,
        })
    };
    let block = |name: &str| {
        top.iter().find_map(|(k, n)| match n {
            Node::Block(b) if k == name => Some(b.as_slice()),
            _ => None,
        })
    };
    for (k, _) in &top {
        if ![
            "description",
            "aci_table_available",
            "scale_factor",
            "apply_factor",
            "custom_lineweight_display_units",
            "aci_table",
            "plot_style",
            "custom_lineweight_table",
        ]
        .contains(&k.as_str())
        {
            report.add(LossKind::UnknownField, None, Some(k.clone()));
        }
    }
    let aci = match get("aci_table_available") {
        Some(v) => boolean(v, "aci_table_available")?,
        None => return Err(unreadable("aci_table_available alanı yok.")),
    };
    if aci != (format == StyleFormat::Ctb) {
        return Err(bad(
            "plot_style_kind_mismatch",
            "Üst bilgi ile aci_table_available çelişiyor: CTB renge bağlı, STB adlı tablodur.",
        ));
    }
    if let Some(v) = get("scale_factor")
        && v.parse::<f64>().map_or(true, |x| !x.is_finite())
    {
        return Err(unreadable("scale_factor bir sayı olmalı."));
    }
    if let Some(v) = get("apply_factor")
        && boolean(v, "apply_factor")?
    {
        report.add(LossKind::ScaleFactor, None, get("scale_factor").map(str::to_owned));
    }
    // The lineweight table: index → millimetres.
    let mut weights: Vec<(i32, bool)> = Vec::new();
    if let Some(b) = block("custom_lineweight_table") {
        for (k, n) in b {
            let Node::Value(v) = n else {
                return Err(unreadable("custom_lineweight_table'da blok olmaz."));
            };
            let i = int(k, "Kalınlık tablosunun sırası")?;
            if i != weights.len() as i64 {
                return Err(unreadable("Kalınlık tablosunun sıraları 0'dan başlayıp birer artmalı."));
            }
            weights.push(micrometres(v, &format!("Kalınlık {i}"))?);
        }
    }
    let styles = block("plot_style").ok_or_else(|| unreadable("plot_style bloğu yok."))?;
    let mut entries = Vec::new();
    let mut exact = 0u32;
    let mut names = std::collections::BTreeSet::new();
    for (n, (k, node)) in styles.iter().enumerate() {
        let Node::Block(fields) = node else {
            return Err(unreadable("plot_style'da her stil bir blok olmalı."));
        };
        if int(k, "Stilin sırası")? != n as i64 {
            return Err(unreadable("Stillerin sıraları 0'dan başlayıp birer artmalı."));
        }
        let key = match format {
            StyleFormat::Ctb => (n + 1).to_string(),
            StyleFormat::Stb => fields
                .iter()
                .find_map(|(k, v)| match v {
                    Node::Value(v) if k == "name" => Some(v.clone()),
                    _ => None,
                })
                .ok_or_else(|| unreadable(format!("{}. stilin adı yok.", n + 1)))?,
        };
        if format == StyleFormat::Stb && key.eq_ignore_ascii_case("normal") {
            let mut scratch = Report::default();
            let (s, _, _) = style(fields, &key, &weights, &mut scratch)?;
            if s != PlotStyle::default() || scratch.lines.keys().any(|k| k.severity() != Severity::Note) {
                report.add(LossKind::NormalChanged, Some(&key), None);
            } else {
                exact += 1;
            }
            continue;
        }
        let (s, lost, _) = style(fields, &key, &weights, &mut report)?;
        if !lost {
            exact += 1;
        }
        let key = match format {
            StyleFormat::Ctb => StyleKey::Aci { index: (n + 1) as u8 },
            StyleFormat::Stb => {
                if key.trim().is_empty() || key.len() > 256 || key.chars().any(char::is_control) {
                    return Err(unreadable(format!("{}. stilin adı geçerli değil.", n + 1)));
                }
                if !names.insert(key.clone()) {
                    return Err(bad("duplicate_plot_style", format!("“{key}” stili tabloda iki kez var.")));
                }
                StyleKey::Named { name: key }
            }
        };
        entries.push(StyleEntry { key, style: s });
    }
    if format == StyleFormat::Ctb && styles.len() != 255 {
        return Err(unreadable(format!(
            "Renge bağlı tabloda ACI 1–255 için 255 stil olmalı; dosyada {} var.",
            styles.len()
        )));
    }
    if styles.len() > 4096 {
        return Err(bad("plot_profile_too_large", "Tablo 4096 stil sınırını aşıyor."));
    }
    let profile = PlotProfile {
        id: id.to_owned(),
        name: name.to_owned(),
        kind: match format {
            StyleFormat::Ctb => TableKind::ColorDependent,
            StyleFormat::Stb => TableKind::Named,
        },
        entries,
        assignments: Vec::new(),
        default_style: None,
        source: Some(PlotSource {
            format,
            sha256: sha256(bytes),
            file: file.to_owned(),
        }),
    };
    super::validate(&profile)?;
    Ok(Read {
        profile,
        description: get("description").unwrap_or_default().to_owned(),
        styles: styles.len() as u32,
        exact,
        losses: report.done(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(head: &[u8], text: &str) -> Vec<u8> {
        let mut body = text.as_bytes().to_vec();
        body.push(0);
        let packed = miniz_oxide::deflate::compress_to_vec_zlib(&body, 6);
        let mut out = head.to_vec();
        for n in [adler32(&packed), body.len() as u32, packed.len() as u32] {
            out.extend_from_slice(&n.to_le_bytes());
        }
        out.extend_from_slice(&packed);
        out
    }

    #[test]
    fn a_named_table_reads_its_styles_and_says_what_it_lost() {
        let text = "description=\"Deneme\naci_table_available=FALSE\nscale_factor=1.0\napply_factor=FALSE\ncustom_lineweight_display_units=0\nplot_style{\n 0{\n  name=\"Normal\n  localized_name=\"Normal\n  description=\"\n  color=-1\n  color_policy=1\n  physical_pen_number=0\n  virtual_pen_number=0\n  screen=100\n  linepattern_size=0.5\n  linetype=31\n  adaptive_linetype=TRUE\n  lineweight=0\n  fill_style=73\n  end_style=4\n  join_style=5\n }\n 1{\n  name=\"Kalin\n  color=-1023410176\n  color_policy=1\n  screen=50\n  linetype=2\n  lineweight=1\n  end_style=2\n  join_style=3\n }\n}\ncustom_lineweight_table{\n 0=0.00\n 1=0.50\n}\n";
        let r = read(&file(HEAD_STB, text), "p", "Deneme", "deneme.stb").unwrap();
        assert_eq!(r.styles, 2);
        assert_eq!(r.profile.entries.len(), 1);
        let s = &r.profile.entries[0].style;
        assert_eq!(s.color, PlotColor::Color { value: "#000000".into() });
        assert_eq!((s.screening, s.line_weight, s.cap, s.join), (50, Some(500), Some(LineCap::Round), None));
        let kinds: Vec<LossKind> = r.losses.iter().map(|l| l.kind).collect();
        assert_eq!(kinds, [LossKind::Dither, LossKind::Linetype, LossKind::JoinStyle]);
        assert_eq!(r.exact, 1);
    }

    #[test]
    fn micrometres_are_exact_to_three_decimals() {
        assert_eq!(micrometres("0.25", "x").unwrap(), (250, false));
        assert_eq!(micrometres("0.0076200000000", "x").unwrap(), (8, true));
        assert_eq!(micrometres("0.0074", "x").unwrap(), (7, true));
        assert!(micrometres("-1", "x").is_err());
        assert!(micrometres("100.001", "x").is_err());
    }
}
