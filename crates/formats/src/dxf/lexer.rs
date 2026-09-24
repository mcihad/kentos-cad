//! DXF group pairs over the file's bytes: a group code line, then its value
//! line (CRLF or LF). Values stay raw bytes until a reader decodes them with
//! the drawing's code page. One pass, no copies.

/// A group: its code, the raw value (line ending removed) and the line number of the code.
#[derive(Clone, Copy, Debug)]
pub struct Pair<'a> {
    pub code: i32,
    pub value: &'a [u8],
    pub line: u32,
}

impl<'a> Pair<'a> {
    /// The value as ASCII text, spaces trimmed (names, numbers, flags).
    pub fn text(&self) -> &'a str {
        std::str::from_utf8(self.value).map(str::trim).unwrap_or("")
    }

    pub fn is(&self, code: i32, value: &str) -> bool {
        self.code == code && self.text().eq_ignore_ascii_case(value)
    }
}

pub struct Lexer<'a> {
    bytes: &'a [u8],
    pos: usize,
    line: u32,
    peeked: Option<Pair<'a>>,
}

impl<'a> Lexer<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        let start = if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) { 3 } else { 0 };
        Lexer { bytes, pos: start, line: 0, peeked: None }
    }

    fn read_line(&mut self) -> Option<&'a [u8]> {
        if self.pos >= self.bytes.len() {
            return None;
        }
        let rest = &self.bytes[self.pos..];
        let (mut line, advance) = match rest.iter().position(|&b| b == b'\n') {
            Some(i) => (&rest[..i], i + 1),
            None => (rest, rest.len()),
        };
        if let [head @ .., b'\r'] = line {
            line = head;
        }
        self.pos += advance;
        self.line += 1;
        Some(line)
    }

    /// The next group, or None at the end of the file.
    pub fn next(&mut self) -> Result<Option<Pair<'a>>, String> {
        if let Some(p) = self.peeked.take() {
            return Ok(Some(p));
        }
        let Some(code_line) = self.read_line() else { return Ok(None) };
        let line = self.line;
        let t = std::str::from_utf8(code_line).map(str::trim).unwrap_or("\u{FFFD}");
        // Blank lines at the very end (after EOF) are common; anywhere else they are an error below.
        if t.is_empty() && self.bytes[self.pos..].iter().all(u8::is_ascii_whitespace) {
            return Ok(None);
        }
        let Ok(code) = t.parse::<i32>() else {
            let shown: String = t.chars().take(24).collect();
            return Err(format!("Satır {line}: grup kodu bir sayı olmalı, “{shown}” bulundu. Dosya bozuk ya da ASCII DXF değil."));
        };
        let Some(value) = self.read_line() else {
            return Err(format!("Satır {line}: dosya bir grup kodundan sonra bitiyor; eksik ya da yarım kalmış bir dosya."));
        };
        Ok(Some(Pair { code, value, line }))
    }

    /// The next group without taking it.
    pub fn peek(&mut self) -> Result<Option<Pair<'a>>, String> {
        if self.peeked.is_none() {
            self.peeked = self.next()?;
        }
        Ok(self.peeked)
    }

    /// Takes the groups up to (not including) the next code 0.
    pub fn until_zero(&mut self) -> Result<Vec<Pair<'a>>, String> {
        let mut out = Vec::new();
        while let Some(p) = self.peek()? {
            if p.code == 0 {
                break;
            }
            out.push(p);
            self.peeked = None;
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_pairs_with_either_line_ending() {
        let mut l = Lexer::new(b"  0\r\nSECTION\r\n  2\nHEADER\n999\n yorum \n  0\nEOF\n\n");
        let p = l.next().expect("ok").expect("pair");
        assert_eq!((p.code, p.text(), p.line), (0, "SECTION", 1));
        assert_eq!(l.peek().expect("ok").map(|p| p.code), Some(2));
        let p = l.next().expect("ok").expect("pair");
        assert_eq!((p.code, p.value, p.line), (2, &b"HEADER"[..], 3));
        let p = l.next().expect("ok").expect("pair");
        assert_eq!((p.code, p.value), (999, &b" yorum "[..]));
        assert!(l.next().expect("ok").expect("pair").is(0, "eof"));
        assert!(l.next().expect("ok").is_none());
    }

    #[test]
    fn broken_files_say_where() {
        let mut l = Lexer::new(b"  0\nSECTION\nabc\nx\n");
        l.next().expect("ok");
        assert_eq!(l.next().err().as_deref(), Some("Satır 3: grup kodu bir sayı olmalı, “abc” bulundu. Dosya bozuk ya da ASCII DXF değil."));
        let mut l = Lexer::new(b"  0\n");
        assert!(l.next().err().is_some_and(|e| e.starts_with("Satır 1: dosya bir grup kodundan sonra bitiyor")));
    }
}
