//! KentOS CBOR profile 1 (docs/specs/kcad-v2.md §5): the writer and the
//! reader. Only what the snapshot needs: unsigned and negative integers, byte
//! and text strings, arrays, maps with text keys, `false`, `true`, `null` and
//! binary64 floats; definite lengths, shortest heads, keys in RFC 8949
//! §4.2.1 order, no tags. The reader checks every rule while reading and
//! never allocates more than the input can hold: a declared length is checked
//! against the limits and against the bytes left before anything is reserved.

use crate::error::{Code, KcadError};

/// Nesting of arrays and maps; the payload's top map is 1 (§5.5).
pub const MAX_DEPTH: usize = 64;
/// Items of an array, pairs of a map (§5.5).
pub const MAX_ITEMS: u64 = 1 << 24;
/// Bytes of a text or byte string (§5.5).
pub const MAX_STRING: u64 = 1 << 24;

pub(crate) const UINT: u8 = 0;
pub(crate) const NINT: u8 = 1;
pub(crate) const BYTES: u8 = 2;
pub(crate) const TEXT: u8 = 3;
pub(crate) const ARRAY: u8 = 4;
pub(crate) const MAP: u8 = 5;
pub(crate) const TAG: u8 = 6;
pub(crate) const SIMPLE: u8 = 7;

/// Orders text keys as RFC 8949 §4.2.1 orders their encodings: shorter first, then by bytes.
pub(crate) fn key_order(a: &str, b: &str) -> std::cmp::Ordering {
    a.len()
        .cmp(&b.len())
        .then_with(|| a.as_bytes().cmp(b.as_bytes()))
}

// ── Writing ─────────────────────────────────────────────────────────────

/// Appends profile items to a buffer. Lengths and floats are checked by the
/// callers (`encode.rs`), which know the path to name in an error.
#[derive(Default)]
pub(crate) struct Writer {
    pub out: Vec<u8>,
}

impl Writer {
    pub fn head(&mut self, major: u8, arg: u64) {
        let m = major << 5;
        if arg < 24 {
            self.out.push(m | arg as u8);
        } else if arg <= u64::from(u8::MAX) {
            self.out.extend_from_slice(&[m | 24, arg as u8]);
        } else if arg <= u64::from(u16::MAX) {
            self.out.push(m | 25);
            self.out.extend_from_slice(&(arg as u16).to_be_bytes());
        } else if arg <= u64::from(u32::MAX) {
            self.out.push(m | 26);
            self.out.extend_from_slice(&(arg as u32).to_be_bytes());
        } else {
            self.out.push(m | 27);
            self.out.extend_from_slice(&arg.to_be_bytes());
        }
    }

    pub fn uint(&mut self, n: u64) {
        self.head(UINT, n);
    }

    /// A signed integer: major 0 when not negative, major 1 with `-1 - n` otherwise.
    pub fn int(&mut self, n: i64) {
        if n >= 0 {
            self.head(UINT, n as u64);
        } else {
            // −1 − n for n < 0 is 0 … 2⁶³−1; `!n` is exactly that in two's complement.
            self.head(NINT, !n as u64);
        }
    }

    /// Always binary64, whatever the value (§5.3); the caller refused NaN and infinities.
    pub fn float(&mut self, x: f64) {
        self.out.push(0xfb);
        self.out.extend_from_slice(&x.to_bits().to_be_bytes());
    }

    pub fn text(&mut self, s: &str) {
        self.head(TEXT, s.len() as u64);
        self.out.extend_from_slice(s.as_bytes());
    }

    pub fn bytes(&mut self, b: &[u8]) {
        self.head(BYTES, b.len() as u64);
        self.out.extend_from_slice(b);
    }

    pub fn bool(&mut self, b: bool) {
        self.out.push(if b { 0xf5 } else { 0xf4 });
    }

    pub fn null(&mut self) {
        self.out.push(0xf6);
    }

    pub fn array(&mut self, n: usize) {
        self.head(ARRAY, n as u64);
    }

    pub fn map(&mut self, n: usize) {
        self.head(MAP, n as u64);
    }
}

// ── Reading ─────────────────────────────────────────────────────────────

/// One item's initial byte and argument.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Head {
    pub major: u8,
    pub info: u8,
    /// The argument (value or length); for major 7, the additional information.
    pub arg: u64,
    /// Where the item starts in the payload.
    pub at: usize,
}

/// A place in the document, for error messages (`document/entities/3/point/p`).
#[derive(Clone, Copy, Debug)]
pub(crate) enum Seg<'a> {
    Name(&'static str),
    Key(&'a str),
    Index(usize),
}

/// A place as the messages write it: `document/entities/3/point/p`.
pub(crate) fn render(path: &[Seg<'_>]) -> String {
    let mut out = String::new();
    for (i, seg) in path.iter().enumerate() {
        if i > 0 {
            out.push('/');
        }
        match seg {
            Seg::Name(n) => out.push_str(n),
            Seg::Key(k) => out.push_str(k),
            Seg::Index(n) => out.push_str(&n.to_string()),
        }
    }
    out
}

/// An item read by `Reader::any`.
pub(crate) enum Any<'a> {
    Text(&'a str),
    Uint(u64),
    Other,
}

/// Reads a payload item by item, the schema (`decode.rs`) saying what comes next.
pub(crate) struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
    /// Arrays and maps entered and not yet left.
    depth: usize,
    path: Vec<Seg<'a>>,
}

impl<'a> Reader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            pos: 0,
            depth: 0,
            path: Vec::with_capacity(16),
        }
    }

    pub fn at_end(&self) -> bool {
        self.pos == self.data.len()
    }

    pub fn position(&self) -> usize {
        self.pos
    }

    // ── Errors ──────────────────────────────────────────────────────────

    /// An error at the current place, `at` bytes into the payload.
    pub fn fail_at(&self, code: Code, at: usize, what: &str) -> KcadError {
        KcadError::payload(code, &render(&self.path), at, what)
    }

    pub fn fail(&self, code: Code, what: &str) -> KcadError {
        self.fail_at(code, self.pos, what)
    }

    pub fn push(&mut self, seg: Seg<'a>) {
        self.path.push(seg);
    }

    pub fn pop(&mut self) {
        self.path.pop();
    }

    // ── Items ───────────────────────────────────────────────────────────

    fn need(&self, n: u64, at: usize) -> Result<(), KcadError> {
        if n > (self.data.len() - self.pos) as u64 {
            return Err(self.fail_at(Code::CborTruncated, at, "öğe yükün sonunu aşıyor"));
        }
        Ok(())
    }

    /// The next item's head, with every rule decidable from it: reserved
    /// additional information, indefinite lengths, shortest arguments.
    pub fn head(&mut self) -> Result<Head, KcadError> {
        let at = self.pos;
        self.need(1, at)?;
        let initial = self.data[self.pos];
        self.pos += 1;
        let (major, info) = (initial >> 5, initial & 0x1f);
        if info < 24 {
            return Ok(Head {
                major,
                info,
                arg: u64::from(info),
                at,
            });
        }
        match info {
            28..=30 => {
                return Err(self.fail_at(
                    Code::Malformed,
                    at,
                    &format!("ayrılmış ek bilgi {info}"),
                ));
            }
            31 if (BYTES..=MAP).contains(&major) => {
                return Err(self.fail_at(
                    Code::IndefiniteLength,
                    at,
                    "belirsiz uzunluk kullanılamaz",
                ));
            }
            31 => {
                return Err(self.fail_at(
                    Code::Malformed,
                    at,
                    "yerinde olmayan belirsiz uzunluk ya da kırma baytı",
                ));
            }
            _ => {}
        }
        if major == SIMPLE {
            return Ok(Head {
                major,
                info,
                arg: u64::from(info),
                at,
            });
        }
        let size = 1usize << (info - 24);
        self.need(size as u64, at)?;
        let mut arg = 0u64;
        for b in &self.data[self.pos..self.pos + size] {
            arg = (arg << 8) | u64::from(*b);
        }
        self.pos += size;
        let least = match info {
            24 => 24,
            25 => 1 << 8,
            26 => 1 << 16,
            _ => 1 << 32,
        };
        if arg < least {
            return Err(self.fail_at(
                Code::NonShortest,
                at,
                &format!("{arg} en kısa biçimde yazılmamış"),
            ));
        }
        Ok(Head {
            major,
            info,
            arg,
            at,
        })
    }

    /// The rules of a major-7 item other than `false`, `true`, `null`: its value when a finite binary64.
    fn simple(&mut self, h: Head) -> Result<Option<f64>, KcadError> {
        match h.info {
            20..=22 => Ok(None),
            25 | 26 => Err(self.fail_at(
                Code::NarrowFloat,
                h.at,
                "kayan noktalı sayı binary64 (8 bayt) yazılmalı",
            )),
            27 => {
                self.need(8, h.at)?;
                let mut b = [0u8; 8];
                b.copy_from_slice(&self.data[self.pos..self.pos + 8]);
                self.pos += 8;
                let x = f64::from_bits(u64::from_be_bytes(b));
                if !x.is_finite() {
                    return Err(self.fail_at(
                        Code::NonFinite,
                        h.at,
                        "NaN ya da sonsuz sayı yazılamaz",
                    ));
                }
                Ok(Some(x))
            }
            _ => Err(self.fail_at(
                Code::SimpleValue,
                h.at,
                "bu basit değer kullanılamaz (yalnız true, false, null)",
            )),
        }
    }

    /// The raw bytes of a string whose head was read.
    fn string_body(&mut self, h: Head) -> Result<&'a [u8], KcadError> {
        if h.arg > MAX_STRING {
            return Err(self.fail_at(
                Code::TooLong,
                h.at,
                &format!("{} baytlık dizgi; sınır {MAX_STRING}", h.arg),
            ));
        }
        self.need(h.arg, h.at)?;
        let start = self.pos;
        self.pos += h.arg as usize;
        Ok(&self.data[start..self.pos])
    }

    fn text_body(&mut self, h: Head) -> Result<&'a str, KcadError> {
        let raw = self.string_body(h)?;
        std::str::from_utf8(raw)
            .map_err(|_| self.fail_at(Code::InvalidUtf8, h.at, "metin geçerli UTF-8 değil"))
    }

    /// Enters an array or map whose head was read: depth and length rules; `per` is the least bytes one entry takes.
    fn enter(&mut self, h: Head, per: u64) -> Result<usize, KcadError> {
        if self.depth + 1 > MAX_DEPTH {
            return Err(self.fail_at(
                Code::TooDeep,
                h.at,
                &format!("iç içe derinlik {MAX_DEPTH}'ı aşıyor"),
            ));
        }
        if h.arg > MAX_ITEMS {
            return Err(self.fail_at(
                Code::TooLong,
                h.at,
                &format!("{} öğe; sınır {MAX_ITEMS}", h.arg),
            ));
        }
        if h.arg * per > (self.data.len() - self.pos) as u64 {
            return Err(self.fail_at(
                Code::CborTruncated,
                h.at,
                &format!("{} öğe yükte kalan baytlara sığmıyor", h.arg),
            ));
        }
        self.depth += 1;
        Ok(h.arg as usize)
    }

    /// Leaves the array or map entered last.
    pub fn leave(&mut self) {
        self.depth -= 1;
    }

    fn wrong(&self, h: Head, want: &str) -> KcadError {
        let found = match h.major {
            UINT | NINT => "tam sayı",
            BYTES => "bayt dizgisi",
            TEXT => "metin",
            ARRAY => "dizi",
            MAP => "harita",
            TAG => "etiket",
            _ => match h.info {
                20 | 21 => "true/false",
                22 => "null",
                _ => "kayan noktalı sayı",
            },
        };
        self.fail_at(
            Code::WrongType,
            h.at,
            &format!("{want} olmalı, {found} bulundu"),
        )
    }

    /// Checks a head's own rules (tags, simple values, floats) before its type is judged.
    fn checked(&mut self) -> Result<(Head, Option<f64>), KcadError> {
        let h = self.head()?;
        match h.major {
            TAG => Err(self.fail_at(Code::Tag, h.at, "CBOR etiketi kullanılamaz")),
            SIMPLE => {
                let x = self.simple(h)?;
                Ok((h, x))
            }
            _ => Ok((h, None)),
        }
    }

    // ── Typed reads (the schema's scalars) ──────────────────────────────

    pub fn text(&mut self) -> Result<&'a str, KcadError> {
        let (h, _) = self.checked()?;
        if h.major != TEXT {
            return Err(self.wrong(h, "metin"));
        }
        self.text_body(h)
    }

    pub fn bytes(&mut self) -> Result<(&'a [u8], usize), KcadError> {
        let (h, _) = self.checked()?;
        if h.major != BYTES {
            return Err(self.wrong(h, "bayt dizgisi"));
        }
        Ok((self.string_body(h)?, h.at))
    }

    pub fn float(&mut self) -> Result<f64, KcadError> {
        match self.checked()? {
            (_, Some(x)) => Ok(x),
            (h, None) => Err(self.wrong(h, "float64")),
        }
    }

    /// An integer from 0 to `max`; a negative one is out of range, not of another type (§6.3).
    pub fn uint(&mut self, max: u64) -> Result<u64, KcadError> {
        let (h, _) = self.checked()?;
        if h.major == NINT {
            return Err(self.fail_at(
                Code::BadValue,
                h.at,
                &format!("negatif sayı aralık dışında (0…{max})"),
            ));
        }
        if h.major != UINT {
            return Err(self.wrong(h, "tam sayı"));
        }
        if h.arg > max {
            return Err(self.fail_at(
                Code::BadValue,
                h.at,
                &format!("{} aralık dışında (0…{max})", h.arg),
            ));
        }
        Ok(h.arg)
    }

    pub fn bool(&mut self) -> Result<bool, KcadError> {
        let (h, _) = self.checked()?;
        match (h.major, h.info) {
            (SIMPLE, 20) => Ok(false),
            (SIMPLE, 21) => Ok(true),
            _ => Err(self.wrong(h, "true ya da false")),
        }
    }

    /// Enters an array; returns its length. Call `leave` after its items.
    pub fn array(&mut self) -> Result<usize, KcadError> {
        let (h, _) = self.checked()?;
        if h.major != ARRAY {
            return Err(self.wrong(h, "dizi"));
        }
        self.enter(h, 1)
    }

    /// Enters a map; returns its pair count and where it starts. Call `leave` after its pairs.
    pub fn map(&mut self) -> Result<(usize, usize), KcadError> {
        let (h, _) = self.checked()?;
        if h.major != MAP {
            return Err(self.wrong(h, "harita"));
        }
        Ok((self.enter(h, 2)?, h.at))
    }

    /// A map key: text, after the previous key of the same map (`previous`, its encoded bytes).
    pub fn key(&mut self, previous: &mut Option<&'a [u8]>) -> Result<&'a str, KcadError> {
        let start = self.pos;
        self.need(1, start)?;
        if self.data[start] >> 5 != TEXT {
            return Err(self.fail_at(Code::NonTextKey, start, "harita anahtarı metin değil"));
        }
        let h = self.head()?;
        let key = self.text_body(h)?;
        let raw = &self.data[start..self.pos];
        if let Some(prev) = *previous {
            if raw == prev {
                return Err(self.fail_at(
                    Code::DuplicateKey,
                    start,
                    &format!("“{key}” anahtarı iki kez yazılmış"),
                ));
            }
            if raw < prev {
                return Err(self.fail_at(
                    Code::UnsortedKeys,
                    start,
                    &format!("“{key}” anahtarı sırasında değil"),
                ));
            }
        }
        *previous = Some(raw);
        Ok(key)
    }

    /// Any item, checked by every rule of the profile (a container with all it
    /// holds); its text or unsigned integer when it is one. The root's `format`
    /// and `version` are read so: whatever else they are, they are not this
    /// document's (§6.1).
    pub fn any(&mut self) -> Result<Any<'a>, KcadError> {
        let (h, _) = self.checked()?;
        match h.major {
            TEXT => Ok(Any::Text(self.text_body(h)?)),
            UINT => Ok(Any::Uint(h.arg)),
            BYTES => {
                self.string_body(h)?;
                Ok(Any::Other)
            }
            ARRAY | MAP => {
                let map = h.major == MAP;
                let n = self.enter(h, if map { 2 } else { 1 })?;
                let mut previous = None;
                for _ in 0..n {
                    if map {
                        self.key(&mut previous)?;
                    }
                    self.any()?;
                }
                self.leave();
                Ok(Any::Other)
            }
            _ => Ok(Any::Other),
        }
    }

    /// Whether the next item is `null` (a renderer written as null is refused as a value, §6.5).
    pub fn next_is_null(&self) -> bool {
        self.data.get(self.pos) == Some(&0xf6)
    }

    // ── Opaque parts (§6.7) ─────────────────────────────────────────────

    /// A JSON-compatible value, integers and floats kept apart.
    pub fn opaque(&mut self) -> Result<serde_json::Value, KcadError> {
        use serde_json::{Map, Number, Value};
        let (h, float) = self.checked()?;
        Ok(match h.major {
            UINT => Value::Number(Number::from(h.arg)),
            NINT => {
                // −1 − arg must fit i64: arg ≤ 2⁶³ − 1.
                let Ok(n) = i64::try_from(h.arg) else {
                    return Err(self.fail_at(
                        Code::BadValue,
                        h.at,
                        "tam sayı 64 bit aralığının dışında (en küçük −2⁶³)",
                    ));
                };
                Value::Number(Number::from(-1 - n))
            }
            TEXT => Value::String(self.text_body(h)?.to_owned()),
            ARRAY => {
                let n = self.enter(h, 1)?;
                // A JSON value is larger in memory than its smallest encoding: reserve little, grow as read.
                let mut list = Vec::with_capacity(n.min(4096));
                for i in 0..n {
                    self.push(Seg::Index(i));
                    list.push(self.opaque()?);
                    self.pop();
                }
                self.leave();
                Value::Array(list)
            }
            MAP => {
                let n = self.enter(h, 2)?;
                let mut map = Map::new();
                let mut previous = None;
                for _ in 0..n {
                    let key = self.key(&mut previous)?;
                    self.push(Seg::Key(key));
                    let value = self.opaque()?;
                    self.pop();
                    map.insert(key.to_owned(), value);
                }
                self.leave();
                Value::Object(map)
            }
            SIMPLE => match (h.info, float) {
                (20, _) => Value::Bool(false),
                (21, _) => Value::Bool(true),
                (22, _) => Value::Null,
                (_, Some(x)) => match Number::from_f64(x) {
                    Some(n) => Value::Number(n),
                    None => {
                        return Err(self.fail_at(
                            Code::NonFinite,
                            h.at,
                            "NaN ya da sonsuz sayı yazılamaz",
                        ));
                    }
                },
                _ => return Err(self.wrong(h, "JSON değeri")),
            },
            _ => {
                return Err(self.fail_at(
                    Code::WrongType,
                    h.at,
                    "JSON'la gösterilebilen bir değer olmalı (bayt dizgisi olamaz)",
                ));
            }
        })
    }
}
