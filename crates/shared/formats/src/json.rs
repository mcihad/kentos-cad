//! A small JSON reader for the formats that are JSON (GeoJSON, RFC 8259):
//! a pull parser over the whole text that the format walks member by
//! member, so a large feature collection is turned into objects one
//! feature at a time and never held as a tree. It keeps what a file wrote
//! where that matters: a number is its text (an attribute keeps `1.50`,
//! coordinates are parsed from it once), and a nested value can be written
//! back compactly as the file had it. Nesting is limited, every error names
//! its line, and no input can make it panic or recurse without bound.

use std::fmt::Write as _;

/// The deepest nesting of arrays and objects read (GeoJSON needs 4 for a
/// MultiPolygon's coordinates; the rest is attributes and collections).
pub const MAX_DEPTH: u32 = 64;

#[derive(Clone, Debug, PartialEq)]
pub struct JsonError {
    pub line: u32,
    pub message: String,
}

/// A value read whole (small ones: a geometry's coordinates, a `crs` member).
#[derive(Clone, Debug, PartialEq)]
pub enum Value<'a> {
    Null,
    Bool(bool),
    /// The number as the file wrote it.
    Num(&'a str),
    Str(String),
    Arr(Vec<Value<'a>>),
    Obj(Vec<(String, Value<'a>)>),
}

impl<'a> Value<'a> {
    /// The member `key` of an object (the last one, when it occurs twice).
    pub fn get(&self, key: &str) -> Option<&Value<'a>> {
        match self {
            Value::Obj(m) => m.iter().rev().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::Str(s) => Some(s),
            _ => None,
        }
    }
}

pub struct Reader<'a> {
    text: &'a str,
    b: &'a [u8],
    i: usize,
    line: u32,
    depth: u32,
}

impl<'a> Reader<'a> {
    /// A reader over `text` (a byte order mark at its start is skipped).
    pub fn new(text: &'a str) -> Reader<'a> {
        let text = text.strip_prefix('\u{feff}').unwrap_or(text);
        Reader {
            text,
            b: text.as_bytes(),
            i: 0,
            line: 1,
            depth: 0,
        }
    }

    /// The line the reader is on (1-based).
    pub fn line(&self) -> u32 {
        self.line
    }

    fn fail<T>(&self, message: impl Into<String>) -> Result<T, JsonError> {
        Err(JsonError {
            line: self.line,
            message: message.into(),
        })
    }

    fn ws(&mut self) {
        while let Some(&c) = self.b.get(self.i) {
            match c {
                b'\n' => self.line += 1,
                b' ' | b'\t' | b'\r' => {}
                _ => break,
            }
            self.i += 1;
        }
    }

    /// The next significant byte, not taken.
    pub fn peek(&mut self) -> Option<u8> {
        self.ws();
        self.b.get(self.i).copied()
    }

    fn take(&mut self, c: u8) -> bool {
        if self.peek() == Some(c) {
            self.i += 1;
            true
        } else {
            false
        }
    }

    fn expect(&mut self, c: u8) -> Result<(), JsonError> {
        if self.take(c) {
            return Ok(());
        }
        match self.b.get(self.i) {
            None => self.fail(format!(
                "dosya yarıda bitiyor; “{}” bekleniyordu",
                c as char
            )),
            Some(&x) => self.fail(format!(
                "“{}” bekleniyordu, “{}” geldi",
                c as char,
                printable(x)
            )),
        }
    }

    /// Only whitespace remains.
    pub fn end(&mut self) -> Result<(), JsonError> {
        match self.peek() {
            None => Ok(()),
            Some(x) => self.fail(format!("belgenin sonundan sonra “{}” geldi", printable(x))),
        }
    }

    fn enter(&mut self) -> Result<(), JsonError> {
        self.depth += 1;
        if self.depth > MAX_DEPTH {
            return self.fail(format!(
                "iç içe dizi ve nesneler {MAX_DEPTH} düzeyi aşıyor; dosya okunmadı"
            ));
        }
        Ok(())
    }

    /// Visits an object's members in order: `f` gets each key and must read its value.
    pub fn object(
        &mut self,
        mut f: impl FnMut(&mut Self, String) -> Result<(), JsonError>,
    ) -> Result<(), JsonError> {
        self.expect(b'{')?;
        self.enter()?;
        if !self.take(b'}') {
            loop {
                let key = self.string()?;
                self.expect(b':')?;
                f(self, key)?;
                if self.take(b',') {
                    continue;
                }
                self.expect(b'}')?;
                break;
            }
        }
        self.depth -= 1;
        Ok(())
    }

    /// Visits an array's elements in order: `f` must read each one.
    pub fn array(
        &mut self,
        mut f: impl FnMut(&mut Self) -> Result<(), JsonError>,
    ) -> Result<(), JsonError> {
        self.expect(b'[')?;
        self.enter()?;
        if !self.take(b']') {
            loop {
                f(self)?;
                if self.take(b',') {
                    continue;
                }
                self.expect(b']')?;
                break;
            }
        }
        self.depth -= 1;
        Ok(())
    }

    /// A string, its escapes decoded (a lone surrogate becomes U+FFFD).
    pub fn string(&mut self) -> Result<String, JsonError> {
        self.expect(b'"')?;
        let mut out = String::new();
        loop {
            let start = self.i;
            while let Some(&c) = self.b.get(self.i) {
                if c == b'"' || c == b'\\' || c < 0x20 {
                    break;
                }
                self.i += 1;
            }
            out.push_str(&self.text[start..self.i]);
            match self.b.get(self.i) {
                None => return self.fail("dosya bir metnin ortasında bitiyor"),
                Some(b'"') => {
                    self.i += 1;
                    return Ok(out);
                }
                Some(b'\\') => {
                    self.i += 1;
                    let e = self.b.get(self.i).copied();
                    self.i += 1;
                    match e {
                        Some(b'"') => out.push('"'),
                        Some(b'\\') => out.push('\\'),
                        Some(b'/') => out.push('/'),
                        Some(b'b') => out.push('\u{8}'),
                        Some(b'f') => out.push('\u{c}'),
                        Some(b'n') => out.push('\n'),
                        Some(b'r') => out.push('\r'),
                        Some(b't') => out.push('\t'),
                        Some(b'u') => {
                            let hi = self.hex4()?;
                            let c = if (0xD800..0xDC00).contains(&hi)
                                && self.b.get(self.i) == Some(&b'\\')
                                && self.b.get(self.i + 1) == Some(&b'u')
                            {
                                let back = self.i;
                                self.i += 2;
                                let lo = self.hex4()?;
                                if (0xDC00..0xE000).contains(&lo) {
                                    char::from_u32(0x10000 + ((hi - 0xD800) << 10) + (lo - 0xDC00))
                                } else {
                                    // Not the pair's second half: read it on its own.
                                    self.i = back;
                                    None
                                }
                            } else {
                                char::from_u32(hi)
                            };
                            out.push(c.unwrap_or(char::REPLACEMENT_CHARACTER));
                        }
                        _ => return self.fail("metinde geçersiz kaçış dizisi (\\ sonrası)"),
                    }
                }
                Some(_) => {
                    return self
                        .fail("metinde kaçışsız denetim karakteri (satır sonu ya da sekme)");
                }
            }
        }
    }

    fn hex4(&mut self) -> Result<u32, JsonError> {
        let digits = self
            .b
            .get(self.i..self.i + 4)
            .and_then(|d| std::str::from_utf8(d).ok());
        match digits.and_then(|d| u32::from_str_radix(d, 16).ok()) {
            Some(v) if digits.is_some_and(|d| d.bytes().all(|c| c.is_ascii_hexdigit())) => {
                self.i += 4;
                Ok(v)
            }
            _ => self.fail("\\u kaçışında dört onaltılık rakam bekleniyordu"),
        }
    }

    /// A number, as written (checked against the JSON grammar).
    pub fn number(&mut self) -> Result<&'a str, JsonError> {
        self.ws();
        let start = self.i;
        let digits = |r: &mut Self| {
            let s = r.i;
            while r.b.get(r.i).is_some_and(u8::is_ascii_digit) {
                r.i += 1;
            }
            r.i > s
        };
        if self.b.get(self.i) == Some(&b'-') {
            self.i += 1;
        }
        match self.b.get(self.i) {
            Some(b'0') => self.i += 1,
            Some(b'1'..=b'9') => {
                digits(self);
            }
            _ => return self.fail("sayı bekleniyordu"),
        }
        if self.b.get(self.i) == Some(&b'.') {
            self.i += 1;
            if !digits(self) {
                return self.fail("sayının ondalık noktasından sonra rakam yok");
            }
        }
        if matches!(self.b.get(self.i), Some(b'e' | b'E')) {
            self.i += 1;
            if matches!(self.b.get(self.i), Some(b'+' | b'-')) {
                self.i += 1;
            }
            if !digits(self) {
                return self.fail("sayının üssünde rakam yok");
            }
        }
        Ok(&self.text[start..self.i])
    }

    fn word(&mut self, w: &str) -> Result<(), JsonError> {
        if self.b[self.i..].starts_with(w.as_bytes()) {
            self.i += w.len();
            Ok(())
        } else {
            self.fail("değer bekleniyordu (metin, sayı, dizi, nesne, true, false ya da null)")
        }
    }

    /// Any value, read whole (nesting limited).
    pub fn value(&mut self) -> Result<Value<'a>, JsonError> {
        match self.peek() {
            Some(b'{') => {
                let mut members = Vec::new();
                self.object(|r, k| {
                    let v = r.value()?;
                    members.push((k, v));
                    Ok(())
                })?;
                Ok(Value::Obj(members))
            }
            Some(b'[') => {
                let mut items = Vec::new();
                self.array(|r| {
                    items.push(r.value()?);
                    Ok(())
                })?;
                Ok(Value::Arr(items))
            }
            Some(b'"') => Ok(Value::Str(self.string()?)),
            Some(b't') => self.word("true").map(|()| Value::Bool(true)),
            Some(b'f') => self.word("false").map(|()| Value::Bool(false)),
            Some(b'n') => self.word("null").map(|()| Value::Null),
            Some(b'-' | b'0'..=b'9') => Ok(Value::Num(self.number()?)),
            None => self.fail("dosya bir değerin yerinde bitiyor"),
            Some(x) => self.fail(format!("değer bekleniyordu, “{}” geldi", printable(x))),
        }
    }

    /// Skips any value (nesting limited).
    pub fn skip(&mut self) -> Result<(), JsonError> {
        match self.peek() {
            Some(b'{') => self.object(|r, _| r.skip()),
            Some(b'[') => self.array(Self::skip),
            _ => self.value().map(|_| ()),
        }
    }

    /// Reads any value and writes it compactly: no whitespace, members in
    /// the file's order, numbers as written, strings escaped only where
    /// JSON must (`"`, `\`, control characters).
    pub fn compact(&mut self, out: &mut String) -> Result<(), JsonError> {
        match self.peek() {
            Some(b'{') => {
                out.push('{');
                let mut first = true;
                self.object(|r, k| {
                    if !first {
                        out.push(',');
                    }
                    first = false;
                    quote(&k, out);
                    out.push(':');
                    r.compact(out)
                })?;
                out.push('}');
                Ok(())
            }
            Some(b'[') => {
                out.push('[');
                let mut first = true;
                self.array(|r| {
                    if !first {
                        out.push(',');
                    }
                    first = false;
                    r.compact(out)
                })?;
                out.push(']');
                Ok(())
            }
            _ => {
                match self.value()? {
                    Value::Str(s) => quote(&s, out),
                    Value::Num(n) => out.push_str(n),
                    Value::Bool(b) => out.push_str(if b { "true" } else { "false" }),
                    Value::Null => out.push_str("null"),
                    // Objects and arrays took the branches above.
                    Value::Arr(_) | Value::Obj(_) => {}
                }
                Ok(())
            }
        }
    }
}

/// A string as JSON writes it, escaped only where it must be.
pub fn quote(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

fn printable(c: u8) -> String {
    if c.is_ascii_graphic() {
        (c as char).to_string()
    } else {
        format!("0x{c:02X}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_numbers_as_written_and_writes_back_compactly() {
        let mut r = Reader::new(
            "{ \"a\" : [1.50, -0.0, 1e3, true, null], \"b\": {\"s\": \"x\\\"y\\n\\u00e7\\ud83d\\ude00\"} }",
        );
        let mut out = String::new();
        r.compact(&mut out).expect("json");
        r.end().expect("end");
        assert_eq!(
            out,
            "{\"a\":[1.50,-0.0,1e3,true,null],\"b\":{\"s\":\"x\\\"y\\nç😀\"}}"
        );
    }

    #[test]
    fn refuses_what_json_refuses_with_its_line() {
        for (text, line) in [
            ("{\n\"a\": 01}", 2),
            ("[1,]", 1),
            ("{\"a\" 1}", 1),
            ("\"a\nb\"", 1),
            ("[1.]", 1),
            ("[\"\\x\"]", 1),
            ("{\"a\":", 1),
            ("[1] 2", 1),
        ] {
            let mut r = Reader::new(text);
            let e = r.value().and_then(|_| r.end()).expect_err(text);
            assert_eq!(e.line, line, "{text}: {}", e.message);
        }
    }

    #[test]
    fn deep_nesting_is_refused_without_recursing_without_bound() {
        let deep = "[".repeat(100_000);
        let mut r = Reader::new(&deep);
        let e = r.skip().expect_err("deep");
        assert!(e.message.contains("64"), "{}", e.message);
    }
}
