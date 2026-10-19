//! A font's text as letters (ISO 32000-1 §9.6–§9.10): a simple font's one-byte codes through its encoding
//! (WinAnsi, MacRoman, Standard, its Differences) or its ToUnicode map; a composite font's two-byte codes through its
//! ToUnicode map. A code that reads as no letter is counted, never guessed. Glyph widths give where the next string
//! starts.

use std::collections::HashMap;

use super::file::PdfFile;
use super::object::{Dict, Obj};

/// A font as text reads.
pub struct Font {
    /// Bytes a code takes: 1 (simple) or 2 (composite).
    pub code_bytes: usize,
    to_unicode: HashMap<u32, String>,
    encoding: Option<[Option<char>; 256]>,
    widths: HashMap<u32, f64>,
    default_width: f64,
    /// Type 3: its glyph space to text space (horizontal factor).
    pub scale: f64,
}

fn win_ansi(b: u8) -> Option<char> {
    const HIGH: [u16; 32] = [
        0x20AC, 0, 0x201A, 0x0192, 0x201E, 0x2026, 0x2020, 0x2021, 0x02C6, 0x2030, 0x0160, 0x2039, 0x0152, 0, 0x017D, 0,
        0, 0x2018, 0x2019, 0x201C, 0x201D, 0x2022, 0x2013, 0x2014, 0x02DC, 0x2122, 0x0161, 0x203A, 0x0153, 0, 0x017E,
        0x0178,
    ];
    match b {
        0x20..=0x7E | 0xA0..=0xFF => Some(b as char),
        0x80..=0x9F => char::from_u32(u32::from(HIGH[usize::from(b - 0x80)])).filter(|c| *c != '\0'),
        b'\t' | b'\n' | b'\r' => Some(' '),
        _ => None,
    }
}

fn mac_roman(b: u8) -> Option<char> {
    const HIGH: [u16; 128] = [
        0xC4, 0xC5, 0xC7, 0xC9, 0xD1, 0xD6, 0xDC, 0xE1, 0xE0, 0xE2, 0xE4, 0xE3, 0xE5, 0xE7, 0xE9, 0xE8, 0xEA, 0xEB, 0xED,
        0xEC, 0xEE, 0xEF, 0xF1, 0xF3, 0xF2, 0xF4, 0xF6, 0xF5, 0xFA, 0xF9, 0xFB, 0xFC, 0x2020, 0xB0, 0xA2, 0xA3, 0xA7,
        0x2022, 0xB6, 0xDF, 0xAE, 0xA9, 0x2122, 0xB4, 0xA8, 0x2260, 0xC6, 0xD8, 0x221E, 0xB1, 0x2264, 0x2265, 0xA5, 0xB5,
        0x2202, 0x2211, 0x220F, 0x3C0, 0x222B, 0xAA, 0xBA, 0x3A9, 0xE6, 0xF8, 0xBF, 0xA1, 0xAC, 0x221A, 0x192, 0x2248,
        0x2206, 0xAB, 0xBB, 0x2026, 0xA0, 0xC0, 0xC3, 0xD5, 0x152, 0x153, 0x2013, 0x2014, 0x201C, 0x201D, 0x2018,
        0x2019, 0xF7, 0x25CA, 0xFF, 0x178, 0x2044, 0x20AC, 0x2039, 0x203A, 0xFB01, 0xFB02, 0x2021, 0xB7, 0x201A,
        0x201E, 0x2030, 0xC2, 0xCA, 0xC1, 0xCB, 0xC8, 0xCD, 0xCE, 0xCF, 0xCC, 0xD3, 0xD4, 0xF8FF, 0xD2, 0xDA, 0xDB,
        0xD9, 0x131, 0x2C6, 0x2DC, 0xAF, 0x2D8, 0x2D9, 0x2DA, 0xB8, 0x2DD, 0x2DB, 0x2C7,
    ];
    match b {
        0x20..=0x7E => Some(b as char),
        0x80..=0xFF => char::from_u32(u32::from(HIGH[usize::from(b - 0x80)])),
        _ => None,
    }
}

/// A glyph name's letter (the Adobe Glyph List's names a drawing's text uses, `uniXXXX`, `uXXXX`).
pub fn glyph_letter(name: &str) -> Option<char> {
    if let Some(h) = name.strip_prefix("uni").filter(|h| h.len() == 4) {
        return u32::from_str_radix(h, 16).ok().and_then(char::from_u32);
    }
    if let Some(h) = name.strip_prefix('u').filter(|h| (4..=6).contains(&h.len()) && h.chars().all(|c| c.is_ascii_hexdigit())) {
        return u32::from_str_radix(h, 16).ok().and_then(char::from_u32);
    }
    if name.len() == 1 && name.chars().all(|c| c.is_ascii_alphabetic()) {
        return name.chars().next();
    }
    const NAMES: &[(&str, char)] = &[
        ("space", ' '), ("exclam", '!'), ("quotedbl", '"'), ("numbersign", '#'), ("dollar", '$'), ("percent", '%'),
        ("ampersand", '&'), ("quotesingle", '\''), ("quoteright", '’'), ("quoteleft", '‘'), ("parenleft", '('),
        ("parenright", ')'), ("asterisk", '*'), ("plus", '+'), ("comma", ','), ("hyphen", '-'), ("minus", '−'),
        ("period", '.'), ("slash", '/'), ("zero", '0'), ("one", '1'), ("two", '2'), ("three", '3'), ("four", '4'),
        ("five", '5'), ("six", '6'), ("seven", '7'), ("eight", '8'), ("nine", '9'), ("colon", ':'), ("semicolon", ';'),
        ("less", '<'), ("equal", '='), ("greater", '>'), ("question", '?'), ("at", '@'), ("bracketleft", '['),
        ("backslash", '\\'), ("bracketright", ']'), ("asciicircum", '^'), ("underscore", '_'), ("grave", '`'),
        ("braceleft", '{'), ("bar", '|'), ("braceright", '}'), ("asciitilde", '~'), ("degree", '°'),
        ("plusminus", '±'), ("multiply", '×'), ("divide", '÷'), ("Gbreve", 'Ğ'), ("gbreve", 'ğ'),
        ("Idotaccent", 'İ'), ("Idot", 'İ'), ("dotlessi", 'ı'), ("Scedilla", 'Ş'), ("scedilla", 'ş'),
        ("Ccedilla", 'Ç'), ("ccedilla", 'ç'), ("Odieresis", 'Ö'), ("odieresis", 'ö'), ("Udieresis", 'Ü'),
        ("udieresis", 'ü'), ("Adieresis", 'Ä'), ("adieresis", 'ä'), ("eacute", 'é'), ("Eacute", 'É'),
        ("endash", '–'), ("emdash", '—'), ("bullet", '•'), ("ellipsis", '…'), ("quotedblleft", '“'),
        ("quotedblright", '”'), ("section", '§'), ("paragraph", '¶'), ("copyright", '©'), ("registered", '®'),
        ("mu", 'µ'), ("twosuperior", '²'), ("threesuperior", '³'), ("onehalf", '½'), ("onequarter", '¼'),
        ("Euro", '€'), ("periodcentered", '·'), ("nbspace", '\u{a0}'), ("diameter", '⌀'), ("Phi", 'Φ'), ("phi", 'φ'),
        ("Delta", 'Δ'), ("increment", '∆'), ("Omega", 'Ω'), ("alpha", 'α'), ("beta", 'β'), ("pi", 'π'),
    ];
    NAMES.iter().find(|(n, _)| *n == name).map(|(_, c)| *c)
}

/// A ToUnicode CMap's codes and letters (its `bfchar` and `bfrange` sections).
fn read_cmap(text: &[u8]) -> (HashMap<u32, String>, usize) {
    let s = String::from_utf8_lossy(text);
    let mut map = HashMap::new();
    let mut code_len = 1;
    let hex = |t: &str| -> Option<Vec<u8>> {
        let t = t.trim().trim_start_matches('<').trim_end_matches('>');
        let t: String = t.chars().filter(|c| !c.is_whitespace()).collect();
        (0..t.len() / 2).map(|i| u8::from_str_radix(&t[2 * i..2 * i + 2], 16).ok()).collect()
    };
    let num = |b: &[u8]| b.iter().fold(0u32, |a, &x| a << 8 | u32::from(x));
    let utf16 = |b: &[u8]| -> String {
        let units: Vec<u16> = b.chunks(2).map(|c| u16::from(c[0]) << 8 | u16::from(*c.get(1).unwrap_or(&0))).collect();
        String::from_utf16_lossy(&units)
    };
    let tokens: Vec<&str> = {
        // Split into <…>, […] and words.
        let mut out = Vec::new();
        let b = s.as_bytes();
        let mut i = 0;
        while i < b.len() {
            match b[i] {
                b'<' => {
                    let end = s[i..].find('>').map_or(b.len(), |e| i + e + 1);
                    out.push(&s[i..end]);
                    i = end;
                }
                b'[' | b']' => {
                    out.push(&s[i..i + 1]);
                    i += 1;
                }
                c if c.is_ascii_whitespace() => i += 1,
                _ => {
                    let end = s[i..].find(|c: char| c.is_ascii_whitespace() || c == '<' || c == '[' || c == ']').map_or(b.len(), |e| i + e);
                    out.push(&s[i..end]);
                    i = end.max(i + 1);
                }
            }
        }
        out
    };
    let mut i = 0;
    while i < tokens.len() {
        match tokens[i] {
            "begincodespacerange" => {
                if let Some(lo) = tokens.get(i + 1).and_then(|t| hex(t)) {
                    code_len = lo.len().max(1);
                }
                i += 1;
            }
            "beginbfchar" => {
                i += 1;
                while i + 1 < tokens.len() && tokens[i] != "endbfchar" {
                    if let (Some(src), Some(dst)) = (hex(tokens[i]), hex(tokens[i + 1])) {
                        code_len = src.len().max(1);
                        map.insert(num(&src), utf16(&dst));
                    }
                    i += 2;
                }
            }
            "beginbfrange" => {
                i += 1;
                while i + 2 < tokens.len() && tokens[i] != "endbfrange" {
                    let (Some(lo), Some(hi)) = (hex(tokens[i]), hex(tokens[i + 1])) else {
                        i += 1;
                        continue;
                    };
                    code_len = lo.len().max(1);
                    let (a, b) = (num(&lo), num(&hi));
                    if tokens[i + 2] == "[" {
                        let mut j = i + 3;
                        let mut c = a;
                        while j < tokens.len() && tokens[j] != "]" {
                            if let Some(d) = hex(tokens[j]) {
                                map.insert(c, utf16(&d));
                            }
                            c += 1;
                            j += 1;
                        }
                        i = j + 1;
                    } else {
                        if let Some(d) = hex(tokens[i + 2]) {
                            let mut units: Vec<u16> =
                                d.chunks(2).map(|c| u16::from(c[0]) << 8 | u16::from(*c.get(1).unwrap_or(&0))).collect();
                            for c in a..=b.min(a + 65535) {
                                map.insert(c, String::from_utf16_lossy(&units));
                                if let Some(last) = units.last_mut() {
                                    *last = last.wrapping_add(1);
                                }
                            }
                        }
                        i += 3;
                    }
                }
            }
            _ => i += 1,
        }
    }
    (map, code_len)
}

impl Font {
    pub fn of(file: &PdfFile<'_>, d: &Dict) -> Font {
        let subtype = file.field(d, "Subtype");
        let composite = subtype.name() == Some("Type0");
        let mut font = Font {
            code_bytes: if composite { 2 } else { 1 },
            to_unicode: HashMap::new(),
            encoding: None,
            widths: HashMap::new(),
            default_width: 0.0,
            scale: 0.001,
        };
        if let Ok(Some(cmap)) = file.stream_data(&file.field(d, "ToUnicode")) {
            let (map, len) = read_cmap(&cmap);
            font.to_unicode = map;
            if composite {
                font.code_bytes = len.clamp(1, 4);
            }
        }
        if composite {
            let desc = file.field(d, "DescendantFonts");
            if let Some(Obj::Dict(cid)) = desc.array().and_then(|a| a.first()).map(|f| file.resolve(f)) {
                font.default_width = file.field(&cid, "DW").num().unwrap_or(1000.0);
                if let Obj::Array(w) = file.field(&cid, "W") {
                    let mut i = 0;
                    while i < w.len() {
                        let first = file.resolve(&w[i]).num().unwrap_or(0.0) as u32;
                        match w.get(i + 1).map(|x| file.resolve(x)) {
                            Some(Obj::Array(list)) => {
                                for (k, x) in list.iter().enumerate() {
                                    font.widths.insert(first + k as u32, file.resolve(x).num().unwrap_or(0.0));
                                }
                                i += 2;
                            }
                            Some(Obj::Num(last)) => {
                                let wv = w.get(i + 2).map(|x| file.resolve(x)).and_then(|x| x.num()).unwrap_or(0.0);
                                for c in first..=(last as u32).min(first + 65535) {
                                    font.widths.insert(c, wv);
                                }
                                i += 3;
                            }
                            _ => break,
                        }
                    }
                }
            }
            return font;
        }
        // A simple font: its base encoding and differences.
        let mut table: [Option<char>; 256] = [None; 256];
        let enc = file.field(d, "Encoding");
        let base = match &enc {
            Obj::Name(n) => n.clone(),
            Obj::Dict(e) => file.field(e, "BaseEncoding").name().unwrap_or("WinAnsiEncoding").to_owned(),
            _ => "WinAnsiEncoding".to_owned(),
        };
        for (b, slot) in table.iter_mut().enumerate() {
            *slot = if base == "MacRomanEncoding" { mac_roman(b as u8) } else { win_ansi(b as u8) };
        }
        if let Obj::Dict(e) = &enc
            && let Obj::Array(diff) = file.field(e, "Differences")
        {
            let mut code = 0usize;
            for x in diff {
                match file.resolve(&x) {
                    Obj::Num(n) => code = n as usize,
                    Obj::Name(name) => {
                        if code < 256 {
                            table[code] = glyph_letter(&name);
                        }
                        code += 1;
                    }
                    _ => {}
                }
            }
        }
        font.encoding = Some(table);
        let first = file.field(d, "FirstChar").num().unwrap_or(0.0) as u32;
        if let Obj::Array(w) = file.field(d, "Widths") {
            for (k, x) in w.iter().enumerate() {
                font.widths.insert(first + k as u32, file.resolve(x).num().unwrap_or(0.0));
            }
        }
        if subtype.name() == Some("Type3")
            && let Obj::Array(m) = file.field(d, "FontMatrix")
        {
            font.scale = m.first().and_then(|x| file.resolve(x).num()).unwrap_or(0.001);
        }
        font.default_width = 500.0;
        font
    }

    /// A string's codes, each with its letters (none: unreadable) and its width in text space (unscaled by size).
    pub fn codes(&self, s: &[u8]) -> Vec<(u32, Option<String>, f64)> {
        s.chunks(self.code_bytes)
            .map(|c| {
                let code = c.iter().fold(0u32, |a, &x| a << 8 | u32::from(x));
                let letters = self.to_unicode.get(&code).cloned().or_else(|| {
                    self.encoding.as_ref().and_then(|t| t.get(code as usize).copied().flatten()).map(String::from)
                });
                let w = self.widths.get(&code).copied().unwrap_or(self.default_width) * self.scale;
                (code, letters, w)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tounicode_map() {
        let (m, len) = read_cmap(
            b"1 begincodespacerange <0000> <ffff> endcodespacerange 2 beginbfchar <0003> <0020> <0024> <011f> endbfchar 1 beginbfrange <0010> <0012> <0041> endbfrange",
        );
        assert_eq!(len, 2);
        assert_eq!(m[&3], " ");
        assert_eq!(m[&0x24], "ğ");
        assert_eq!(m[&0x12], "C");
        assert_eq!(glyph_letter("Idotaccent"), Some('İ'));
        assert_eq!(win_ansi(0x80), Some('€'));
    }
}
