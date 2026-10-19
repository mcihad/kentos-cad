//! PDF objects and their reader (ISO 32000-1 §7.2–§7.3): numbers, strings (literal with escapes, hexadecimal), names
//! (`#xx` escapes), arrays, dictionaries, indirect references (`n g R`) and the keywords a content stream uses. The
//! reader never panics on a broken file: it says where it stopped.

use std::collections::BTreeMap;

/// A PDF object.
#[derive(Clone, Debug, PartialEq)]
pub enum Obj {
    Null,
    Bool(bool),
    Num(f64),
    Str(Vec<u8>),
    Name(String),
    Array(Vec<Obj>),
    Dict(Dict),
    Ref(u32, u16),
    /// A stream: its dictionary and its bytes as written (not yet decoded).
    Stream(Dict, Vec<u8>),
}

pub type Dict = BTreeMap<String, Obj>;

impl Obj {
    pub fn num(&self) -> Option<f64> {
        match self {
            Obj::Num(n) => Some(*n),
            _ => None,
        }
    }
    pub fn name(&self) -> Option<&str> {
        match self {
            Obj::Name(n) => Some(n),
            _ => None,
        }
    }
    pub fn array(&self) -> Option<&[Obj]> {
        match self {
            Obj::Array(a) => Some(a),
            _ => None,
        }
    }
    pub fn dict(&self) -> Option<&Dict> {
        match self {
            Obj::Dict(d) | Obj::Stream(d, _) => Some(d),
            _ => None,
        }
    }
    pub fn bytes(&self) -> Option<&[u8]> {
        match self {
            Obj::Str(s) => Some(s),
            _ => None,
        }
    }
}

/// A content stream's or a file's token.
#[derive(Clone, Debug, PartialEq)]
pub enum Token {
    Obj(Obj),
    /// An operator or a keyword (`obj`, `endobj`, `stream`, `R`, `re`, `Tj` …).
    Word(String),
    ArrayStart,
    ArrayEnd,
    DictStart,
    DictEnd,
}

pub fn is_white(b: u8) -> bool {
    matches!(b, b' ' | b'\n' | b'\r' | b'\t' | b'\x0c' | b'\0')
}

pub fn is_delim(b: u8) -> bool {
    matches!(b, b'(' | b')' | b'<' | b'>' | b'[' | b']' | b'{' | b'}' | b'/' | b'%')
}

/// The most nested arrays and dictionaries the reader follows.
const MAX_DEPTH: usize = 64;

/// A cursor over bytes.
pub struct Lexer<'a> {
    pub src: &'a [u8],
    pub pos: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(src: &'a [u8], pos: usize) -> Self {
        Self { src, pos }
    }

    pub fn skip_white(&mut self) {
        while self.pos < self.src.len() {
            let b = self.src[self.pos];
            if is_white(b) {
                self.pos += 1;
            } else if b == b'%' {
                while self.pos < self.src.len() && !matches!(self.src[self.pos], b'\n' | b'\r') {
                    self.pos += 1;
                }
            } else {
                break;
            }
        }
    }

    /// The next token, or none at the end.
    pub fn token(&mut self) -> Result<Option<Token>, String> {
        self.skip_white();
        let Some(&b) = self.src.get(self.pos) else {
            return Ok(None);
        };
        match b {
            b'[' => {
                self.pos += 1;
                Ok(Some(Token::ArrayStart))
            }
            b']' => {
                self.pos += 1;
                Ok(Some(Token::ArrayEnd))
            }
            b'<' if self.src.get(self.pos + 1) == Some(&b'<') => {
                self.pos += 2;
                Ok(Some(Token::DictStart))
            }
            b'>' if self.src.get(self.pos + 1) == Some(&b'>') => {
                self.pos += 2;
                Ok(Some(Token::DictEnd))
            }
            b'<' => Ok(Some(Token::Obj(Obj::Str(self.hex_string()?)))),
            b'(' => Ok(Some(Token::Obj(Obj::Str(self.literal_string()?)))),
            b'/' => Ok(Some(Token::Obj(Obj::Name(self.name())))),
            b'{' | b'}' => {
                self.pos += 1;
                Ok(Some(Token::Word((b as char).to_string())))
            }
            b'+' | b'-' | b'.' | b'0'..=b'9' => Ok(Some(self.number())),
            b')' | b'>' => {
                self.pos += 1;
                Err(format!("{}. baytta beklenmeyen ‘{}’", self.pos, b as char))
            }
            _ => {
                let start = self.pos;
                while self.pos < self.src.len() && !is_white(self.src[self.pos]) && !is_delim(self.src[self.pos]) {
                    self.pos += 1;
                }
                let w = String::from_utf8_lossy(&self.src[start..self.pos]).into_owned();
                Ok(Some(match w.as_str() {
                    "true" => Token::Obj(Obj::Bool(true)),
                    "false" => Token::Obj(Obj::Bool(false)),
                    "null" => Token::Obj(Obj::Null),
                    _ => Token::Word(w),
                }))
            }
        }
    }

    fn number(&mut self) -> Token {
        let start = self.pos;
        self.pos += 1;
        while self.pos < self.src.len() && matches!(self.src[self.pos], b'0'..=b'9' | b'.' | b'-' | b'+') {
            self.pos += 1;
        }
        let text = String::from_utf8_lossy(&self.src[start..self.pos]);
        // A writer's “--5” or “5-” reads as the number it starts with, as readers do.
        let n = text.parse::<f64>().unwrap_or_else(|_| {
            let t = text.trim_start_matches(['+', '-']);
            let sign = if text.starts_with('-') { -1.0 } else { 1.0 };
            let digits: String = t.chars().take_while(|c| c.is_ascii_digit() || *c == '.').collect();
            sign * digits.parse::<f64>().unwrap_or(0.0)
        });
        Token::Obj(Obj::Num(n))
    }

    fn name(&mut self) -> String {
        self.pos += 1;
        let mut out = Vec::new();
        while self.pos < self.src.len() && !is_white(self.src[self.pos]) && !is_delim(self.src[self.pos]) {
            let b = self.src[self.pos];
            if b == b'#'
                && let Some(h) = self.src.get(self.pos + 1..self.pos + 3)
                && let Ok(v) = u8::from_str_radix(&String::from_utf8_lossy(h), 16)
            {
                out.push(v);
                self.pos += 3;
                continue;
            }
            out.push(b);
            self.pos += 1;
        }
        String::from_utf8_lossy(&out).into_owned()
    }

    fn hex_string(&mut self) -> Result<Vec<u8>, String> {
        self.pos += 1;
        let mut digits = Vec::new();
        loop {
            let Some(&b) = self.src.get(self.pos) else {
                return Err("onaltılık dizgi kapanmadan dosya bitti".into());
            };
            self.pos += 1;
            match b {
                b'>' => break,
                b'0'..=b'9' => digits.push(b - b'0'),
                b'a'..=b'f' => digits.push(b - b'a' + 10),
                b'A'..=b'F' => digits.push(b - b'A' + 10),
                _ if is_white(b) => {}
                _ => return Err(format!("onaltılık dizgide ‘{}’", b as char)),
            }
        }
        if digits.len() % 2 == 1 {
            digits.push(0);
        }
        Ok(digits.chunks(2).map(|c| c[0] << 4 | c[1]).collect())
    }

    fn literal_string(&mut self) -> Result<Vec<u8>, String> {
        self.pos += 1;
        let mut out = Vec::new();
        let mut depth = 1usize;
        loop {
            let Some(&b) = self.src.get(self.pos) else {
                return Err("dizgi kapanmadan dosya bitti".into());
            };
            self.pos += 1;
            match b {
                b'(' => {
                    depth += 1;
                    out.push(b);
                }
                b')' => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                    out.push(b);
                }
                b'\\' => {
                    let Some(&e) = self.src.get(self.pos) else { break };
                    self.pos += 1;
                    match e {
                        b'n' => out.push(b'\n'),
                        b'r' => out.push(b'\r'),
                        b't' => out.push(b'\t'),
                        b'b' => out.push(8),
                        b'f' => out.push(12),
                        b'\r' => {
                            if self.src.get(self.pos) == Some(&b'\n') {
                                self.pos += 1;
                            }
                        }
                        b'\n' => {}
                        b'0'..=b'7' => {
                            let mut v = u32::from(e - b'0');
                            for _ in 0..2 {
                                match self.src.get(self.pos) {
                                    Some(&d @ b'0'..=b'7') => {
                                        v = v * 8 + u32::from(d - b'0');
                                        self.pos += 1;
                                    }
                                    _ => break,
                                }
                            }
                            out.push((v & 0xff) as u8);
                        }
                        _ => out.push(e),
                    }
                }
                b'\r' => {
                    // An end of line in a string is a line feed.
                    if self.src.get(self.pos) == Some(&b'\n') {
                        self.pos += 1;
                    }
                    out.push(b'\n');
                }
                _ => out.push(b),
            }
        }
        Ok(out)
    }

    /// The next whole object (an array or a dictionary read through; `n g R` a reference). The keyword `stream`
    /// after a dictionary is the caller's to read.
    pub fn object(&mut self) -> Result<Obj, String> {
        self.object_at(0)
    }

    fn object_at(&mut self, depth: usize) -> Result<Obj, String> {
        if depth > MAX_DEPTH {
            return Err("iç içe dizi ve sözlükler çok derin".into());
        }
        match self.token()? {
            None => Err("nesne beklenirken dosya bitti".into()),
            Some(Token::Obj(Obj::Num(n))) => {
                // `n g R`: look ahead without losing the place.
                let save = self.pos;
                if n >= 0.0
                    && n.fract() == 0.0
                    && let Ok(Some(Token::Obj(Obj::Num(g)))) = self.token()
                    && g >= 0.0
                    && g.fract() == 0.0
                    && let Ok(Some(Token::Word(w))) = self.token()
                    && w == "R"
                {
                    return Ok(Obj::Ref(n as u32, g as u16));
                }
                self.pos = save;
                Ok(Obj::Num(n))
            }
            Some(Token::Obj(o)) => Ok(o),
            Some(Token::ArrayStart) => {
                let mut items = Vec::new();
                loop {
                    self.skip_white();
                    if self.src.get(self.pos) == Some(&b']') {
                        self.pos += 1;
                        break;
                    }
                    if self.pos >= self.src.len() {
                        return Err("dizi kapanmadan dosya bitti".into());
                    }
                    items.push(self.object_at(depth + 1)?);
                }
                Ok(Obj::Array(items))
            }
            Some(Token::DictStart) => {
                let mut d = Dict::new();
                loop {
                    match self.token()? {
                        Some(Token::DictEnd) => break,
                        Some(Token::Obj(Obj::Name(k))) => {
                            let v = self.object_at(depth + 1)?;
                            d.insert(k, v);
                        }
                        None => return Err("sözlük kapanmadan dosya bitti".into()),
                        Some(other) => return Err(format!("sözlükte ad yerine {other:?}")),
                    }
                }
                Ok(Obj::Dict(d))
            }
            Some(Token::Word(w)) => Err(format!("nesne yerine ‘{w}’")),
            Some(t) => Err(format!("nesne yerine {t:?}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn objects_read_as_written() {
        let src = b"<< /Type /Page /Kids [1 0 R 2 0 R] /A (a\\(b\\)\\101) /B <48 69> /N#20m -.5 >>";
        let o = Lexer::new(src, 0).object().unwrap();
        let d = o.dict().unwrap();
        assert_eq!(d["Type"], Obj::Name("Page".into()));
        assert_eq!(d["Kids"], Obj::Array(vec![Obj::Ref(1, 0), Obj::Ref(2, 0)]));
        assert_eq!(d["A"], Obj::Str(b"a(b)A".to_vec()));
        assert_eq!(d["B"], Obj::Str(b"Hi".to_vec()));
        assert_eq!(d["N m"], Obj::Num(-0.5));
    }
}
