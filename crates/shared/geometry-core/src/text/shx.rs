//! Bounded SHX shape programs, independent of fonts, the filesystem and any
//! renderer. See Autodesk's Special Codes Reference, ADR 0930 and ADR 1120.
//!
//! AutoCAD-86 shapes 1.0/1.1, unifont 1.0 and bigfont 1.0. Unsupported forms
//! (an extended Big Font's scaled `7,0` primitive) fail explicitly; a requested
//! shape or text run is either completely decoded or returns no geometry.
use crate::{
    Vec2,
    geom::bulge::bulge_arc,
    jsmath::{acos, cos, sin, js_max},
};
use std::collections::{BTreeMap, BTreeSet};

pub const MAX_BYTES: usize = 4 * 1024 * 1024;
const MAX_POINTS: usize = 32_768;
const MAX_STEPS: usize = 100_000;
const MAX_DEPTH: usize = 32;
const TOL: f64 = 0.0001;

#[derive(Clone, Debug)]
struct Shape<'a> {
    name: &'a [u8],
    code: &'a [u8],
}

/// The compiled file's form, from its signature (ADR 1120 §2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// `AutoCAD-86 shapes 1.0/1.1`: one-byte numbers, one-byte subshape operand.
    Shapes,
    /// `AutoCAD-86 unifont 1.0`: Unicode numbers, a two-byte subshape operand.
    Unifont,
    /// `AutoCAD-86 bigfont 1.0`: two-byte codes behind escape (lead) byte ranges.
    BigFont,
}

/// A text font's descriptor: shape 0 of a shapes/big font, the definition
/// record of a unifont. `above` is the capital height in vector units.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FontInfo {
    pub above: u8,
    pub below: u8,
    pub modes: u8,
    /// Unifont only: 0 Unicode, 1 packed multibyte, 2 shape file.
    pub encoding: Option<u8>,
    /// Unifont only, bit coded: 1 the font may not be embedded, 2 read-only embedding.
    pub embed: Option<u8>,
    /// Extended Big Font descriptor (`height, 0, modes, width, 0`).
    pub extended: bool,
}

/// Source coordinates are vector units, deliberately not normalized to a
/// font's capital height. LIN's S parameter scales a single vector unit.
#[derive(Clone, Debug)]
pub struct ShapeFile<'a> {
    kind: Kind,
    shapes: BTreeMap<u16, Shape<'a>>,
    font: Option<FontInfo>,
    /// Big Font escape (lead byte) ranges, inclusive.
    ranges: Vec<(u16, u16)>,
}

const SHAPES_10: &[u8] = b"AutoCAD-86 shapes 1.0\r\n\x1a";
const SHAPES_11: &[u8] = b"AutoCAD-86 shapes 1.1\r\n\x1a";
const UNIFONT: &[u8] = b"AutoCAD-86 unifont 1.0\r\n\x1a";
const BIGFONT: &[u8] = b"AutoCAD-86 bigfont 1.0\r\n\x1a";

/// The form a signature names, before anything else is read.
pub fn kind_of(data: &[u8]) -> Option<Kind> {
    if data.starts_with(SHAPES_10) || data.starts_with(SHAPES_11) {
        Some(Kind::Shapes)
    } else if data.starts_with(UNIFONT) {
        Some(Kind::Unifont)
    } else if data.starts_with(BIGFONT) {
        Some(Kind::BigFont)
    } else {
        None
    }
}

/// A record's name and its NUL-terminated program.
fn split(record: &[u8]) -> Result<(&[u8], &[u8]), String> {
    let n = record
        .iter()
        .position(|v| *v == 0)
        .ok_or("SHX şekil adı sonlandırılmamış.")?;
    Ok((&record[..n], &record[n + 1..]))
}

/// `above, below, modes, 0`, a unifont's `above, below, modes, encoding,
/// type, 0` or an extended Big Font's `height, 0, modes, width, 0`.
fn descriptor(kind: Kind, rest: &[u8]) -> Result<FontInfo, String> {
    let info = match (kind, rest) {
        (Kind::Unifont, [above, below, modes, encoding, embed, 0]) => FontInfo {
            above: *above,
            below: *below,
            modes: *modes,
            encoding: Some(*encoding),
            embed: Some(*embed),
            extended: false,
        },
        (Kind::Shapes | Kind::BigFont, [above, below, modes, 0]) => FontInfo {
            above: *above,
            below: *below,
            modes: *modes,
            encoding: None,
            embed: None,
            extended: false,
        },
        (Kind::BigFont, [height, 0, modes, _width, 0]) => FontInfo {
            above: *height,
            below: 0,
            modes: *modes,
            encoding: None,
            embed: None,
            extended: true,
        },
        _ => return Err("SHX yazı fontu tanımı (0 numaralı kayıt) geçersiz.".into()),
    };
    if info.above == 0 || !matches!(info.modes, 0 | 2) || info.encoding.is_some_and(|e| e > 2) {
        return Err("SHX yazı fontu tanımının yükseklik/kip/kodlama değeri geçersiz.".into());
    }
    Ok(info)
}

struct Bytes<'a> {
    data: &'a [u8],
    at: usize,
}
impl<'a> Bytes<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], String> {
        let end = self.at.checked_add(n).ok_or("SHX kaydı taşmış.")?;
        let out = self
            .data
            .get(self.at..end)
            .ok_or("SHX kaydı yarım kalmış.")?;
        self.at = end;
        Ok(out)
    }
    fn byte(&mut self) -> Result<u8, String> {
        Ok(self.take(1)?[0])
    }
    fn word(&mut self) -> Result<u16, String> {
        let b = self.take(2)?;
        Ok(u16::from_le_bytes([b[0], b[1]]))
    }
    fn pair(&mut self) -> Result<Vec2, String> {
        let b = self.take(2)?;
        Ok(Vec2::new(f64::from(b[0] as i8), f64::from(b[1] as i8)))
    }
}

impl<'a> ShapeFile<'a> {
    pub fn read(data: &'a [u8]) -> Result<Self, String> {
        if data.len() > MAX_BYTES {
            return Err("SHX kaynağı 4 MiB sınırını aşıyor.".into());
        }
        match kind_of(data) {
            Some(Kind::Shapes) => Self::read_shapes(data),
            Some(Kind::Unifont) => Self::read_unifont(data),
            Some(Kind::BigFont) => Self::read_bigfont(data),
            None => Err("Bu SHX biçimi desteklenmiyor; AutoCAD-86 shapes 1.0/1.1, unifont 1.0 \
                         veya bigfont 1.0 gerekli."
                .into()),
        }
    }

    fn empty(kind: Kind) -> Self {
        Self {
            kind,
            shapes: BTreeMap::new(),
            font: None,
            ranges: Vec::new(),
        }
    }

    fn insert(
        &mut self,
        names: &mut BTreeSet<Vec<u8>>,
        number: u16,
        record: &'a [u8],
    ) -> Result<(), String> {
        let (name, code) = split(record)?;
        if number == 0 {
            // The classic text font descriptor, not executable code. A shape
            // file may carry another record 0; it simply is not a text font.
            self.font = descriptor(self.kind, code).ok();
            return Ok(());
        }
        if code.last() != Some(&0) {
            return Err("SHX şekil programı sonlandırılmamış.".into());
        }
        if !name.is_empty() && !names.insert(name.to_ascii_lowercase()) {
            return Err("SHX içinde aynı adlı birden fazla şekil var.".into());
        }
        if self.shapes.insert(number, Shape { name, code }).is_some() {
            return Err("SHX dizin kaydı yinelenmiş.".into());
        }
        Ok(())
    }

    fn done(self) -> Result<Self, String> {
        if self.shapes.is_empty() {
            return Err("SHX içinde çizilebilir şekil yok.".into());
        }
        Ok(self)
    }

    fn read_shapes(data: &'a [u8]) -> Result<Self, String> {
        let mut b = Bytes { data, at: 24 };
        let first = b.word()?;
        let last = b.word()?;
        let count = usize::from(b.word()?);
        if count == 0 || count > 16_384 {
            return Err("SHX şekil sayısı geçersiz.".into());
        }
        let mut index = Vec::with_capacity(count);
        let mut numbers = BTreeSet::new();
        for _ in 0..count {
            let number = b.word()?;
            let len = usize::from(b.word()?);
            if len < 2 || !numbers.insert(number) {
                return Err("SHX dizin kaydı geçersiz veya yinelenmiş.".into());
            }
            index.push((number, len));
        }
        if index.first().map(|s| s.0) != Some(first) || index.last().map(|s| s.0) != Some(last) {
            return Err("SHX dizininin ilk/son şekli uyuşmuyor.".into());
        }
        let mut out = Self::empty(Kind::Shapes);
        let mut names = BTreeSet::new();
        for (number, len) in index {
            let record = b.take(len)?;
            out.insert(&mut names, number, record)?;
        }
        if b.take(3)? != b"EOF" || b.at != data.len() {
            return Err("SHX dosya sonu geçersiz.".into());
        }
        out.done()
    }

    /// Records follow one another after the definition record; there is no
    /// index (ADR 1120 §2). The 32-bit count includes the definition record.
    fn read_unifont(data: &'a [u8]) -> Result<Self, String> {
        let mut b = Bytes {
            data,
            at: UNIFONT.len(),
        };
        let low = b.word()?;
        let high = b.word()?;
        let count = u32::from(low) | (u32::from(high) << 16);
        if !(2..=65_537).contains(&count) {
            return Err("SHX unifont kayıt sayısı geçersiz.".into());
        }
        let def = usize::from(b.word()?);
        let (_, rest) = split(b.take(def)?)?;
        let mut out = Self::empty(Kind::Unifont);
        out.font = Some(descriptor(Kind::Unifont, rest)?);
        let mut names = BTreeSet::new();
        for _ in 1..count {
            let number = b.word()?;
            let len = usize::from(b.word()?);
            if number == 0 || len < 2 {
                return Err("SHX unifont kaydı geçersiz.".into());
            }
            let record = b.take(len)?;
            out.insert(&mut names, number, record)?;
        }
        if b.at != data.len() {
            return Err("SHX unifont dosyasının sonunda beklenmeyen veri var.".into());
        }
        out.done()
    }

    /// An index of (code, length, absolute offset) entries; all-zero entries
    /// are empty slots. The first header word is read, not interpreted.
    fn read_bigfont(data: &'a [u8]) -> Result<Self, String> {
        let mut b = Bytes {
            data,
            at: BIGFONT.len(),
        };
        let _header = b.word()?;
        let count = usize::from(b.word()?);
        let nranges = usize::from(b.word()?);
        if count == 0 || nranges == 0 || nranges > 256 {
            return Err("SHX bigfont dizini veya kaçış aralığı sayısı geçersiz.".into());
        }
        let mut out = Self::empty(Kind::BigFont);
        for _ in 0..nranges {
            let (begin, end) = (b.word()?, b.word()?);
            if begin > end || end > 0xff || begin == 0 {
                return Err("SHX bigfont kaçış aralığı geçersiz.".into());
            }
            out.ranges.push((begin, end));
        }
        let mut index = Vec::with_capacity(count);
        for _ in 0..count {
            let number = b.word()?;
            let len = usize::from(b.word()?);
            let low = b.word()?;
            let high = b.word()?;
            let offset = usize::from(low) | (usize::from(high) << 16);
            if number == 0 && len == 0 && offset == 0 {
                continue;
            }
            index.push((number, len, offset));
        }
        let start = b.at;
        let mut names = BTreeSet::new();
        let mut numbers = BTreeSet::new();
        for (number, len, offset) in index {
            let end = offset.checked_add(len).ok_or("SHX bigfont kaydı taşmış.")?;
            if len < 2 || offset < start || end > data.len() || !numbers.insert(number) {
                return Err("SHX bigfont dizin kaydı geçersiz veya yinelenmiş.".into());
            }
            out.insert(&mut names, number, &data[offset..end])?;
        }
        if numbers.contains(&0) && out.font.is_none() {
            return Err("SHX bigfont yazı fontu tanımı geçersiz.".into());
        }
        out.done()
    }

    pub fn kind(&self) -> Kind {
        self.kind
    }
    /// The text font descriptor, when the file is a text font.
    pub fn font(&self) -> Option<FontInfo> {
        self.font
    }

    /// The shape number of a character in a shapes 1.0/1.1 or unifont text
    /// font. A classic font is addressed by a code page the file does not
    /// name: only ASCII is unambiguous (ADR 1120 §3).
    pub fn glyph(&self, ch: char) -> Result<u16, String> {
        let font = self
            .font
            .ok_or("SHX dosyası bir yazı fontu değil (yazı fontu tanımı yok).")?;
        let code = u32::from(ch);
        let number = match self.kind {
            Kind::Shapes if code < 0x80 => code as u16,
            Kind::Shapes => {
                return Err(format!(
                    "“{ch}” (U+{code:04X}) klasik SHX fontunda kod sayfasına bağlıdır; dosya kod \
                     sayfasını bildirmez. Bir unifont kullanın."
                ));
            }
            Kind::Unifont if font.encoding != Some(0) => {
                return Err("SHX unifont kodlaması Unicode değil; karakterler çözülemez.".into());
            }
            Kind::Unifont => u16::try_from(code)
                .map_err(|_| format!("“{ch}” (U+{code:X}) SHX unifont aralığının dışında."))?,
            Kind::BigFont => {
                return Err(
                    "Big Font tek başına yazı fontu değildir; karakteri \\M+ koduyla verin.".into(),
                );
            }
        };
        if self.shapes.contains_key(&number) {
            Ok(number)
        } else {
            Err(format!("“{ch}” (U+{code:04X}) karakteri SHX fontunda yok."))
        }
    }

    /// A Big Font character by its two-byte code (lead byte in an escape range).
    pub fn big_glyph(&self, code: u16) -> Result<u16, String> {
        if self.kind != Kind::BigFont || self.font.is_none() {
            return Err("SHX dosyası bir Big Font değil.".into());
        }
        let lead = code >> 8;
        if !self.ranges.iter().any(|(b, e)| (*b..=*e).contains(&lead)) {
            return Err(format!(
                "{code:04X} kodunun ilk baytı Big Font kaçış aralıklarında değil."
            ));
        }
        if self.shapes.contains_key(&code) {
            Ok(code)
        } else {
            Err(format!("{code:04X} kodlu karakter Big Font içinde yok."))
        }
    }

    pub fn len(&self) -> usize {
        self.shapes.len()
    }
    pub fn is_empty(&self) -> bool {
        self.shapes.is_empty()
    }

    /// Resolve a LIN name or the decimal shape number retained by DXF/DWG.
    pub fn number(&self, name: &str) -> Option<u16> {
        let by_name = self
            .shapes
            .iter()
            .find(|(_, s)| s.name.eq_ignore_ascii_case(name.as_bytes()))
            .map(|(n, _)| *n);
        by_name
            .or_else(|| name.strip_prefix('#').unwrap_or(name).parse().ok())
            .filter(|n| self.shapes.contains_key(n))
    }

    pub fn draw(&self, name: &str) -> Result<Vec<Vec<Vec2>>, String> {
        let number = self
            .number(name)
            .ok_or_else(|| format!("“{name}” şekli SHX kaynağında bulunamadı."))?;
        self.draw_number(number)
    }

    pub fn draw_number(&self, number: u16) -> Result<Vec<Vec<Vec2>>, String> {
        let mut vm = Vm {
            file: self,
            at: Vec2::new(0., 0.),
            pen: true,
            scale: 1.,
            stack: Vec::new(),
            active: Vec::new(),
            paths: Vec::new(),
            path: Vec::new(),
            points: 0,
            steps: 0,
        };
        vm.run(number)?;
        if !vm.stack.is_empty() {
            return Err("SHX konum yığını dengeli değil.".into());
        }
        vm.flush();
        Ok(vm.paths)
    }
}

/// A character of an SHX text: a Unicode character for the primary font, or a
/// Big Font code given as AutoCAD's `\M+cXXXX` (c 1–5, the code page index;
/// XXXX the two-byte code, ADR 1120 §3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextChar {
    Char(char),
    Big(u16),
}

pub fn text_chars(text: &str) -> Vec<TextChar> {
    let chars: Vec<char> = text.chars().collect();
    let mut out = Vec::with_capacity(chars.len());
    let mut i = 0;
    while i < chars.len() {
        let code = chars.get(i..i + 8).and_then(|w| {
            let head: String = w[..3].iter().collect();
            let hex: String = w[4..].iter().collect();
            (head.eq_ignore_ascii_case("\\m+")
                && ('1'..='5').contains(&w[3])
                && hex.chars().all(|c| c.is_ascii_hexdigit()))
            .then(|| u16::from_str_radix(&hex, 16).ok())
            .flatten()
        });
        if let Some(code) = code {
            out.push(TextChar::Big(code));
            i += 8;
        } else {
            out.push(TextChar::Char(chars[i]));
            i += 1;
        }
    }
    out
}

/// A text run: each glyph's program from its own file, the pen carried from
/// one glyph's end to the next (a glyph's last pen-up move is its advance).
/// Every glyph starts with the pen down and its own font's scale, so the
/// result is in capital heights: 1 is the text height (ADR 1120 §3).
pub fn draw_text<'a, 'b>(run: &[(&'a ShapeFile<'b>, u16)]) -> Result<Vec<Vec<Vec2>>, String> {
    draw_text_advance(run).map(|(paths, _)| paths)
}

/// [`draw_text`] and where the pen stands after the run: its advance, in capital heights (a text's width before its
/// width factor, ADR 1125 §2).
pub fn draw_text_advance<'a, 'b>(
    run: &[(&'a ShapeFile<'b>, u16)],
) -> Result<(Vec<Vec<Vec2>>, Vec2), String> {
    let Some(&(first, _)) = run.first() else {
        return Ok((Vec::new(), Vec2::new(0., 0.)));
    };
    let mut vm = Vm {
        file: first,
        at: Vec2::new(0., 0.),
        pen: true,
        scale: 1.,
        stack: Vec::new(),
        active: Vec::new(),
        paths: Vec::new(),
        path: Vec::new(),
        points: 0,
        steps: 0,
    };
    for &(file, number) in run {
        let font = file
            .font
            .ok_or("SHX dosyası bir yazı fontu değil (yazı fontu tanımı yok).")?;
        vm.flush();
        vm.file = file;
        vm.pen = true;
        vm.scale = 1. / f64::from(font.above);
        vm.run(number)?;
        if !vm.stack.is_empty() {
            return Err("SHX konum yığını dengeli değil.".into());
        }
    }
    vm.flush();
    Ok((vm.paths, vm.at))
}

struct Vm<'a, 'b> {
    file: &'a ShapeFile<'b>,
    at: Vec2,
    pen: bool,
    scale: f64,
    stack: Vec<Vec2>,
    active: Vec<u16>,
    paths: Vec<Vec<Vec2>>,
    path: Vec<Vec2>,
    points: usize,
    steps: usize,
}
impl Vm<'_, '_> {
    fn flush(&mut self) {
        if self.path.len() >= 2 {
            self.paths.push(std::mem::take(&mut self.path));
        } else {
            self.path.clear();
        }
    }
    fn step(&mut self) -> Result<(), String> {
        self.steps += 1;
        if self.steps > MAX_STEPS {
            return Err("SHX işlem sınırı aşıldı.".into());
        }
        Ok(())
    }
    fn point(&mut self, p: Vec2) -> Result<(), String> {
        if !p.x.is_finite() || !p.y.is_finite() || js_max(p.x.abs(),p.y.abs()) > 1e9 {
            return Err("SHX şekil koordinatı sınır dışında.".into());
        }
        if self.pen && p != self.at {
            let extra = if self.path.is_empty() { 2 } else { 1 };
            self.points += extra;
            if self.points > MAX_POINTS {
                return Err("SHX kontur noktası sınırı aşıldı.".into());
            }
            if self.path.is_empty() {
                self.path.push(self.at);
            }
            self.path.push(p);
        } else if !self.pen {
            self.flush();
        }
        self.at = p;
        Ok(())
    }
    fn displacement(&mut self, p: Vec2) -> Result<(), String> {
        self.point(Vec2::new(
            self.at.x + self.scale * p.x,
            self.at.y + self.scale * p.y,
        ))
    }
    fn arc(&mut self, radius: f64, start: f64, sweep: f64) -> Result<(), String> {
        let radius = radius * self.scale;
        if !radius.is_finite()
            || radius <= 0.
            || !sweep.is_finite()
            || sweep.abs() > std::f64::consts::TAU + 1e-12
        {
            return Err("SHX yay tanımı geçersiz.".into());
        }
        let c = Vec2::new(
            self.at.x - radius * cos(start),
            self.at.y - radius * sin(start),
        );
        let end = Vec2::new(
            c.x + radius * cos(start + sweep),
            c.y + radius * sin(start + sweep),
        );
        if !self.pen {
            return self.point(end);
        }
        let step = 2. * acos((1. - TOL / radius).clamp(-1., 1.));
        let count = js_max((sweep.abs() / step).ceil(),1.);
        if !count.is_finite()
            || count > MAX_POINTS as f64
            || self.points.saturating_add(count as usize + 1) > MAX_POINTS
        {
            return Err("SHX yay doğruluk sınırı aşıldı.".into());
        }
        for i in 1..=count as usize {
            let a = start + sweep * i as f64 / count;
            self.point(Vec2::new(c.x + radius * cos(a), c.y + radius * sin(a)))?;
        }
        Ok(())
    }
    fn bulge(&mut self, p: Vec2, b: i8) -> Result<(), String> {
        if b == -128 || p.x == -128. || p.y == -128. {
            return Err("SHX bulge değeri -128 olamaz.".into());
        }
        if b == 0 {
            return self.displacement(p);
        }
        let end = Vec2::new(self.at.x + p.x * self.scale, self.at.y + p.y * self.scale);
        let arc = bulge_arc(self.at, end, f64::from(b) / 127.).ok_or("SHX bulge yayı geçersiz.")?;
        self.arc(arc.r / self.scale, arc.a0, arc.sweep)?;
        // Preserve the exact encoded displacement after flattening an arc.
        self.at = end;
        if let Some(last) = self.path.last_mut() {
            *last = end;
        }
        Ok(())
    }
    fn run(&mut self, number: u16) -> Result<(), String> {
        if self.active.len() >= MAX_DEPTH || self.active.contains(&number) {
            return Err("SHX alt şekil döngüsü veya derinlik sınırı.".into());
        }
        let code = self
            .file
            .shapes
            .get(&number)
            .ok_or_else(|| format!("SHX #{number} alt şekli bulunamadı."))?
            .code;
        self.active.push(number);
        let mut b = Bytes { data: code, at: 0 };
        let mut skip = false;
        loop {
            self.step()?;
            let op = b.byte()?;
            let apply = !skip;
            skip = false;
            match op {
                0 => {
                    if code[b.at..].iter().any(|v| *v != 0) {
                        return Err("SHX programı sonunda beklenmeyen veri.".into());
                    }
                    break;
                }
                1 if apply => self.pen = true,
                2 if apply => {
                    self.flush();
                    self.pen = false;
                }
                1 | 2 => {}
                3 | 4 => {
                    let n = b.byte()?;
                    if n == 0 {
                        return Err("SHX ölçek çarpanı sıfır olamaz.".into());
                    }
                    if apply {
                        self.scale = if op == 3 {
                            self.scale / f64::from(n)
                        } else {
                            self.scale * f64::from(n)
                        };
                        if !self.scale.is_finite() || !(1e-9..=1e9).contains(&self.scale) {
                            return Err("SHX ölçek sınırı aşıldı.".into());
                        }
                    }
                }
                5 if apply => {
                    if self.stack.len() == 4 {
                        return Err("SHX konum yığını dolu.".into());
                    }
                    self.stack.push(self.at);
                }
                6 if apply => {
                    self.flush();
                    self.at = self.stack.pop().ok_or("SHX konum yığını boş.")?;
                }
                5 | 6 => {}
                // A unifont's operand is two bytes, high first ("the compiler
                // always uses two bytes"); a Big Font's `7,0` is the extended
                // form with a scaled primitive, refused rather than guessed.
                7 => {
                    let n = match self.file.kind {
                        Kind::Shapes => u16::from(b.byte()?),
                        Kind::Unifont => u16::from_be_bytes([b.byte()?, b.byte()?]),
                        Kind::BigFont => match b.byte()? {
                            0 => {
                                b.take(6)?;
                                if apply {
                                    return Err("Genişletilmiş Big Font alt şekli (7,0) \
                                                desteklenmiyor; eksik çıktı üretilmedi."
                                        .into());
                                }
                                continue;
                            }
                            n => u16::from(n),
                        },
                    };
                    if apply {
                        self.run(n)?;
                    }
                }
                8 => {
                    let p = b.pair()?;
                    if apply {
                        self.displacement(p)?;
                    }
                }
                9 | 13 => loop {
                    self.step()?;
                    let p = b.pair()?;
                    if p.x == 0. && p.y == 0. {
                        break;
                    }
                    let bulge = if op == 13 { b.byte()? as i8 } else { 0 };
                    if apply {
                        if op == 9 {
                            self.displacement(p)?;
                        } else {
                            self.bulge(p, bulge)?;
                        }
                    }
                },
                10 | 11 => {
                    let (start_offset, end_offset, radius) = if op == 11 {
                        let start = b.byte()?;
                        let end = b.byte()?;
                        let hi = u16::from(b.byte()?);
                        (start, end, f64::from((hi << 8) | u16::from(b.byte()?)))
                    } else {
                        (0, 0, f64::from(b.byte()?))
                    };
                    let spec = b.byte()?;
                    let octant = (spec & 0x70) >> 4;
                    let count = spec & 0x0f;
                    if count > 7 {
                        return Err("SHX oktant sayısı geçersiz.".into());
                    }
                    let count = if count == 0 { 8. } else { f64::from(count) };
                    let dir = if spec & 0x80 == 0 { 1. } else { -1. };
                    let unit = std::f64::consts::FRAC_PI_4;
                    let start = unit * (f64::from(octant) + dir * f64::from(start_offset) / 256.);
                    let sweep = dir
                        * unit
                        * (count - 1.
                            + if end_offset == 0 {
                                1.
                            } else {
                                f64::from(end_offset) / 256.
                            }
                            - f64::from(start_offset) / 256.);
                    if sweep * dir <= 0. {
                        return Err("SHX kesirli yay açıları geçersiz.".into());
                    }
                    if apply {
                        self.arc(radius, start, sweep)?;
                    }
                }
                12 => {
                    let p = b.pair()?;
                    let bulge = b.byte()? as i8;
                    if apply {
                        self.bulge(p, bulge)?;
                    }
                }
                // Horizontal linetype shapes skip exactly one complete command,
                // including operands. A skipped 14 must not skip another command.
                14 if apply => skip = true,
                14 => {}
                15 => return Err("SHX ayrılmış 15 komutu desteklenmiyor.".into()),
                op if apply => {
                    const X: [f64; 16] = [
                        1., 1., 1., 0.5, 0., -0.5, -1., -1., -1., -1., -1., -0.5, 0., 0.5, 1., 1.,
                    ];
                    const Y: [f64; 16] = [
                        0., 0.5, 1., 1., 1., 1., 1., 0.5, 0., -0.5, -1., -1., -1., -1., -1., -0.5,
                    ];
                    let d = usize::from(op & 15);
                    let len = f64::from(op >> 4);
                    self.displacement(Vec2::new(X[d] * len, Y[d] * len))?;
                }
                _ => {}
            }
        }
        self.active.pop();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn file(records: &[(u16, &str, &[u8])]) -> Vec<u8> {
        let mut b = b"AutoCAD-86 shapes 1.0\r\n\x1a".to_vec();
        for n in [
            records.first().unwrap().0,
            records.last().unwrap().0,
            records.len() as u16,
        ] {
            b.extend(n.to_le_bytes());
        }
        for (n, name, code) in records {
            b.extend(n.to_le_bytes());
            b.extend(((name.len() + 1 + code.len()) as u16).to_le_bytes());
        }
        for (_, name, code) in records {
            b.extend(name.as_bytes());
            b.push(0);
            b.extend(*code);
        }
        b.extend(b"EOF");
        b
    }
    fn draw(code: &[u8]) -> Result<Vec<Vec<Vec2>>, String> {
        let b = file(&[(1, "TEST", code)]);
        ShapeFile::read(&b)?.draw("test")
    }
    #[test]
    fn vectors_pen_stack_scale_and_named_numeric_lookup() {
        let b = file(&[(
            1,
            "FENCE",
            &[5, 0x14, 6, 2, 8, 2, 0, 1, 4, 2, 0x14, 3, 2, 0x10, 0],
        )]);
        let f = ShapeFile::read(&b).unwrap();
        let expected = vec![
            vec![Vec2::new(0., 0.), Vec2::new(0., 1.)],
            vec![Vec2::new(2., 0.), Vec2::new(2., 2.), Vec2::new(3., 2.)],
        ];
        assert_eq!(f.draw("fence").unwrap(), expected);
        assert_eq!(f.draw("#1").unwrap(), expected);
        assert_eq!(f.draw("1").unwrap(), expected);
        assert!(f.draw("unknown").is_err());
    }
    #[test]
    fn subshape_inherits_pen_and_keeps_changed_scale() {
        let b = file(&[
            (1, "ROOT", &[2, 7, 2, 1, 0x10, 0]),
            (2, "SUB", &[4, 2, 0x10, 0]),
        ]);
        assert_eq!(
            ShapeFile::read(&b).unwrap().draw("root").unwrap(),
            vec![vec![Vec2::new(2., 0.), Vec2::new(4., 0.)]]
        );
    }
    #[test]
    fn vertical_skip_consumes_pairs_and_subshape_without_executing_them() {
        let out = draw(&[14, 9, 127, 127, 0, 0, 14, 7, 222, 0x10, 0]).unwrap();
        assert_eq!(out, vec![vec![Vec2::new(0., 0.), Vec2::new(1., 0.)]]);
    }
    #[test]
    fn signed_octant_full_circle_and_bulges_have_correct_endpoints() {
        for (code, end) in [
            (&[10, 2, 0x84, 0][..], Vec2::new(-4., 0.)),
            (&[12, 4, 0, 127, 0][..], Vec2::new(4., 0.)),
            (&[12, 4, 0, 129, 0][..], Vec2::new(4., 0.)),
        ] {
            let p = draw(code).unwrap();
            let last = p[0].last().unwrap();
            assert!(
                (last.x - end.x).abs() < 1e-12 && (last.y - end.y).abs() < 1e-12,
                "{last:?}"
            );
        }
        let circle = draw(&[10, 2, 0, 0]).unwrap();
        assert!(circle[0].len() > 100);
        for p in &circle[0] {
            assert!(((p.x + 2.).hypot(p.y) - 2.).abs() < 1e-12);
        }
        let positive = draw(&[13, 4, 0, 127, 4, 0, 129, 0, 0, 0]).unwrap();
        assert!(positive[0].iter().any(|p| p.y < -1.9));
        assert!(positive[0].iter().any(|p| p.y > 1.9));
    }
    #[test]
    fn documented_fractional_arc_55_to_95_degrees() {
        let p = draw(&[11, 56, 28, 0, 3, 0x12, 0]).unwrap();
        let start = (45_f64 + 56. * 45. / 256.).to_radians();
        let end = (90_f64 + 28. * 45. / 256.).to_radians();
        let last = p[0].last().unwrap();
        assert!((last.x - 3. * (cos(end) - cos(start))).abs() < 1e-12);
        assert!((last.y - 3. * (sin(end) - sin(start))).abs() < 1e-12);
        assert!(draw(&[11, 0, 0, 0, 3, 0x14, 0]).is_ok());
    }
    #[test]
    fn malformed_programs_never_return_partial_geometry() {
        for code in [
            &[0x10, 6, 0][..],
            &[5, 0][..],
            &[5, 5, 5, 5, 5, 0][..],
            &[0x10, 3, 0, 0][..],
            &[0x10, 7, 3, 0][..],
            &[0x10, 15, 0][..],
            &[9, 1, 1, 0][..],
            &[10, 0, 1, 0][..],
            &[10, 1, 0x18, 0][..],
            &[12, 1, 0, 128, 0][..],
            &[0, 0x10, 0][..],
        ] {
            assert!(draw(code).is_err(), "{code:?}");
        }
        let b = file(&[(1, "LOOP", &[7, 1, 0])]);
        assert!(ShapeFile::read(&b).unwrap().draw("loop").is_err());
        assert!(draw(&[4, 255, 4, 255, 4, 255, 4, 255, 4, 255, 0x10, 0]).is_err());
    }
    #[test]
    fn every_truncation_and_duplicate_index_is_rejected_without_panic() {
        let b = file(&[(1, "FENCE", &[0x10, 0])]);
        for end in 0..b.len() {
            assert!(ShapeFile::read(&b[..end]).is_err(), "{end}");
        }
        assert!(ShapeFile::read(&file(&[(1, "A", &[0]), (1, "B", &[0])])).is_err());
        assert!(ShapeFile::read(&file(&[(1, "A", &[0]), (2, "a", &[0])])).is_err());
        assert!(ShapeFile::read(b"AutoCAD-86 unifont 1.0\r\n\x1a").is_err());
        assert!(ShapeFile::read(b"AutoCAD-86 bigfont 1.0\r\n\x1a").is_err());
        assert!(ShapeFile::read(b"AutoCAD-86 shapes 2.0\r\n\x1a").is_err());
        assert!(ShapeFile::read(&vec![0; MAX_BYTES + 1]).is_err());
    }
}

#[cfg(test)]
#[path = "shx_font_tests.rs"]
mod font_tests;
