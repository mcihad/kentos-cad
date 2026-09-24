//! DXF text values as the user sees them: bytes in the drawing's code page
//! (Windows-1254 for Turkish AutoCAD before 2007, UTF-8 after), characters
//! outside it as `\U+XXXX`, TEXT control codes (`%%d` → °) and MTEXT
//! formatting (`\P` paragraphs, `{\H2.5x;…}` runs), which is stripped.

use crate::text::{Encoding, decode};

/// Decodes the drawing's strings once its encoding is known.
#[derive(Clone, Copy, Debug)]
pub struct Decoder {
    pub enc: Encoding,
}

impl Decoder {
    pub fn string(&self, raw: &[u8]) -> String {
        let s = decode(raw, self.enc);
        if s.contains("\\U+") || s.contains("\\u+") {
            unescape_unicode(&s)
        } else {
            s
        }
    }
}

fn hex4(chars: &[char]) -> Option<char> {
    if chars.len() < 4 {
        return None;
    }
    let s: String = chars[..4].iter().collect();
    u32::from_str_radix(&s, 16).ok().and_then(char::from_u32)
}

/// `\U+XXXX` escapes (AutoCAD writes characters its code page lacks this way) as characters.
pub fn unescape_unicode(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '\\'
            && i + 2 < chars.len()
            && (chars[i + 1] == 'U' || chars[i + 1] == 'u')
            && chars[i + 2] == '+'
            && let Some(c) = hex4(&chars[i + 3..])
        {
            out.push(c);
            i += 7;
            continue;
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

/// TEXT control codes: `%%d` °, `%%p` ±, `%%c` Ø, `%%%` %, `%%nnn` the
/// character with that code; `%%u`, `%%o`, `%%k` (underline, overline,
/// strike-through switches) are dropped.
pub fn text_codes(s: &str) -> String {
    if !s.contains("%%") {
        return s.to_string();
    }
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '%' && chars.get(i + 1) == Some(&'%') {
            match chars.get(i + 2).map(char::to_ascii_lowercase) {
                Some('d') => out.push('°'),
                Some('p') => out.push('±'),
                Some('c') => out.push('Ø'),
                Some('%') => out.push('%'),
                Some('u' | 'o' | 'k') => {}
                Some(d) if d.is_ascii_digit() => {
                    let digits: String = chars[i + 2..]
                        .iter()
                        .take(3)
                        .take_while(|c| c.is_ascii_digit())
                        .collect();
                    if let Some(c) = digits.parse::<u32>().ok().and_then(char::from_u32) {
                        out.push(c);
                    }
                    i += 2 + digits.len();
                    continue;
                }
                _ => {
                    out.push('%');
                    i += 1;
                    continue;
                }
            }
            i += 3;
            continue;
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

/// MTEXT content as plain lines: paragraphs (`\P`), column breaks (`\N`)
/// and dimension line breaks (`\X`) split lines; formatting codes, font and
/// colour switches and `{…}` groups are removed; stacked fractions
/// (`\S1/2;`) read "1/2". Control codes are applied per line.
pub fn mtext_lines(s: &str) -> Vec<String> {
    let chars: Vec<char> = s.chars().collect();
    let mut lines = vec![String::new()];
    let mut i = 0;
    let skip_to_semicolon = |from: usize| {
        chars[from..]
            .iter()
            .position(|&c| c == ';')
            .map_or(chars.len(), |k| from + k + 1)
    };
    while i < chars.len() {
        let c = chars[i];
        let cur = lines.last_mut();
        match (c, chars.get(i + 1).copied(), cur) {
            ('\\', Some(k), Some(cur)) => match k {
                'P' | 'X' | 'N' => {
                    lines.push(String::new());
                    i += 2;
                }
                '~' => {
                    cur.push(' ');
                    i += 2;
                }
                '\\' | '{' | '}' => {
                    cur.push(k);
                    i += 2;
                }
                'L' | 'l' | 'O' | 'o' | 'K' | 'k' => i += 2,
                'S' => {
                    let end = skip_to_semicolon(i + 2);
                    let body: String = chars[i + 2..end.min(chars.len())]
                        .iter()
                        .filter(|&&c| c != ';')
                        .map(|&c| if c == '^' || c == '#' { '/' } else { c })
                        .collect();
                    cur.push_str(body.trim());
                    i = end;
                }
                'A' | 'C' | 'c' | 'F' | 'f' | 'H' | 'h' | 'Q' | 'q' | 'T' | 't' | 'W' | 'w'
                | 'p' => i = skip_to_semicolon(i + 2),
                _ => {
                    cur.push(k);
                    i += 2;
                }
            },
            ('{' | '}', _, _) => i += 1,
            (_, _, Some(cur)) => {
                cur.push(c);
                i += 1;
            }
            (_, _, None) => i += 1,
        }
    }
    lines
        .into_iter()
        .map(|l| text_codes(l.trim_end()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn turkish_code_page_and_unicode_escapes() {
        let d = Decoder {
            enc: Encoding::Windows1254,
        };
        assert_eq!(d.string(b"\xDDstasyon \xFEev"), "İstasyon şev");
        assert_eq!(d.string(b"Ada \\U+0130\\U+015F x"), "Ada İş x");
        let u = Decoder {
            enc: Encoding::Utf8,
        };
        assert_eq!(u.string("Ağaç".as_bytes()), "Ağaç");
    }

    #[test]
    fn text_control_codes() {
        assert_eq!(
            text_codes("45%%d  %%p0.05  %%c20  100%%%  %%uAltı%%u çizili %%065"),
            "45°  ±0.05  Ø20  100%  Altı çizili A"
        );
        assert_eq!(text_codes("yüzde 5% kalır"), "yüzde 5% kalır");
    }

    #[test]
    fn mtext_formatting_is_removed() {
        let s = "{\\fArial|b1|i0|c162;\\H2.5x;Parsel 12}\\P\\C1;Alan: 450 m\\S2^;\\P\\pxi-3,l3;• Madde\\~1 \\{sabit\\}";
        assert_eq!(
            mtext_lines(s),
            vec!["Parsel 12", "Alan: 450 m2/", "• Madde 1 {sabit}"]
        );
        assert_eq!(mtext_lines("Tek satır"), vec!["Tek satır"]);
        assert_eq!(mtext_lines("\\A1;%%c30\\Pikinci"), vec!["Ø30", "ikinci"]);
    }
}
