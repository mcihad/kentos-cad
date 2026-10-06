//! DXF text values as the user sees them: bytes in the drawing's code page
//! (Windows-1254 for Turkish AutoCAD before 2007, UTF-8 after), characters
//! outside it as `\U+XXXX`, TEXT control codes (`%%d` → °) and MTEXT
//! formatting (`\P` paragraphs, `{\H2.5x;…}` runs): a multi-line text's
//! line breaks and letter formats (docs/adr/0182 §5), the rest stripped.

use kentos_contracts::{TextRun, TextScript};

use super::aci;
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

/// MTEXT content as KentOS's multi-line text (docs/adr/0182 §5): its
/// letters, line breaks included, and their formats, as the file and the
/// commands take them.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MtextContent {
    pub text: String,
    pub runs: Vec<TextRun>,
    /// A width switch (`\W`) before its first letter, outside any group: the
    /// text's width factor (KentOS writes it so); none for 1.
    pub width_factor: Option<f64>,
    /// The formatting it dropped, as the report names it, in this order:
    /// height, fraction, font, over- and strike-through line, width, letter
    /// spacing, slant, paragraph format.
    pub dropped: Vec<&'static str>,
}

/// The formats a letter is written in.
#[derive(Clone, Debug, Default, PartialEq)]
struct Format {
    bold: bool,
    italic: bool,
    underline: bool,
    script: Option<TextScript>,
    color: Option<String>,
}

impl Format {
    fn plain(&self) -> bool {
        !self.bold
            && !self.italic
            && !self.underline
            && self.script.is_none()
            && self.color.is_none()
    }
}

const DROPPED: [&str; 8] = [
    "yükseklik",
    "kesir",
    "yazı tipi",
    "üst ve üstü çizili çizgi",
    "genişlik",
    "harf aralığı",
    "eğiklik açısı",
    "paragraf biçimi",
];

/// An MTEXT colour switch's value: `\C` an ACI index (0 and 256, by block
/// and by layer, the text's own), `\c` a true colour written blue, green,
/// red from the high byte (AutoCAD's order, as ezdxf reads it).
fn switch_color(body: &str, aci_index: bool) -> Option<Option<String>> {
    let v: i64 = body.trim().parse().ok()?;
    if aci_index {
        return Some(match v {
            1..=255 => Some(aci::color(v as u8)),
            _ => None,
        });
    }
    let v = v & 0xFF_FFFF;
    let rgb = ((v & 0xFF) << 16) | (v & 0xFF00) | (v >> 16);
    Some(Some(aci::true_color(rgb)))
}

/// MTEXT content as a multi-line text (docs/adr/0182 §5): `\P` (and the
/// column and dimension breaks `\N`, `\X`) a line break, `\L…\l` underline,
/// `\f…|b1|i1;` bold and italic, `\C`/`\c` a colour, `\S…^;` raised and
/// `\S^…;` lowered letters, `{…}` groups; `%%d %%p %%c` ° ± Ø. Heights,
/// fractions (which read "1/2"), fonts, over- and strike-through lines,
/// widths, letter spacing, slants and paragraph formats are dropped and
/// named. Each line's trailing spaces go.
pub fn mtext_content(s: &str) -> MtextContent {
    let chars: Vec<char> = s.chars().collect();
    let mut letters: Vec<(char, Format)> = Vec::with_capacity(chars.len());
    let mut groups: Vec<Format> = Vec::new();
    let mut cur = Format::default();
    let mut dropped = [false; DROPPED.len()];
    let mut faces: Vec<String> = Vec::new();
    let mut width_factor = None;
    let body_end = |from: usize| {
        chars[from.min(chars.len())..]
            .iter()
            .position(|&c| c == ';')
            .map_or(chars.len(), |k| from + k)
    };
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        let next = chars.get(i + 1).copied();
        match (c, next) {
            ('\\', Some(k)) => {
                // A switch with a value runs to its ";" (read only by those that have one).
                let value = || {
                    let end = body_end(i + 2);
                    let body: String = chars[(i + 2).min(end)..end].iter().collect();
                    (body, (end + 1).min(chars.len()))
                };
                match k {
                    'P' | 'X' | 'N' => {
                        letters.push(('\n', cur.clone()));
                        i += 2;
                    }
                    '~' => {
                        letters.push((' ', cur.clone()));
                        i += 2;
                    }
                    '\\' | '{' | '}' => {
                        letters.push((k, cur.clone()));
                        i += 2;
                    }
                    'L' | 'l' => {
                        cur.underline = k == 'L';
                        i += 2;
                    }
                    'O' | 'K' => {
                        dropped[3] = true;
                        i += 2;
                    }
                    'o' | 'k' => i += 2,
                    'S' => {
                        let (body, after) = value();
                        let sep = body.find(['^', '/', '#']);
                        let (upper, lower, mark) = match sep {
                            Some(at) => (&body[..at], &body[at + 1..], body[at..].chars().next()),
                            None => (body.as_str(), "", None),
                        };
                        let raised = mark == Some('^')
                            && lower.trim().is_empty()
                            && !upper.trim().is_empty();
                        let lowered = mark == Some('^')
                            && upper.trim().is_empty()
                            && !lower.trim().is_empty();
                        if raised || lowered {
                            let mut f = cur.clone();
                            f.script = Some(if raised {
                                TextScript::Super
                            } else {
                                TextScript::Sub
                            });
                            let words = if raised { upper.trim() } else { lower.trim() };
                            letters.extend(words.chars().map(|l| (l, f.clone())));
                        } else {
                            let words = match mark {
                                Some(_) => format!("{}/{}", upper.trim(), lower.trim()),
                                None => upper.trim().to_owned(),
                            };
                            letters.extend(words.chars().map(|l| (l, cur.clone())));
                            dropped[1] |= mark.is_some();
                        }
                        i = after;
                    }
                    'f' | 'F' => {
                        let (body, after) = value();
                        let mut parts = body.split('|');
                        // One face for all of it is as a style's font, which KentOS's own replaces too; faces mixed are said.
                        if let Some(face) = parts.next().map(|f| f.trim().to_lowercase())
                            && !face.is_empty()
                            && !faces.contains(&face)
                        {
                            faces.push(face);
                            dropped[2] |= faces.len() > 1;
                        }
                        for part in parts {
                            match part.as_bytes() {
                                [b'b', v, ..] => cur.bold = *v == b'1',
                                [b'i', v, ..] => cur.italic = *v == b'1',
                                _ => {}
                            }
                        }
                        i = after;
                    }
                    'C' | 'c' => {
                        let (body, after) = value();
                        if let Some(color) = switch_color(&body, k == 'C') {
                            cur.color = color;
                        }
                        i = after;
                    }
                    'W' | 'w'
                        if letters.is_empty() && groups.is_empty() && width_factor.is_none() =>
                    {
                        let (body, after) = value();
                        let factor = body.trim().trim_end_matches(['x', 'X']).parse::<f64>().ok();
                        match factor {
                            Some(1.0) => {}
                            Some(f) if kentos_contracts::width_factor_ok(f) => {
                                width_factor = Some(f)
                            }
                            _ => dropped[4] = true,
                        }
                        i = after;
                    }
                    'H' | 'h' | 'W' | 'w' | 'T' | 't' | 'Q' | 'q' | 'p' => {
                        let (body, after) = value();
                        let at = match k {
                            'H' | 'h' => 0,
                            'W' | 'w' => 4,
                            'T' | 't' => 5,
                            'Q' | 'q' => 6,
                            _ => 7,
                        };
                        // A factor of 1, a slant of 0 and a paragraph of no indent left aligned change nothing.
                        let number = body.trim().trim_end_matches(['x', 'X']).parse::<f64>().ok();
                        let plain = match k {
                            'H' | 'h' => body.trim().ends_with(['x', 'X']) && number == Some(1.0),
                            'W' | 'w' | 'T' | 't' => number == Some(1.0),
                            'Q' | 'q' => number == Some(0.0),
                            _ => {
                                !body.chars().any(|c| ('1'..='9').contains(&c))
                                    && !["qc", "qr", "qj", "qd"].iter().any(|q| body.contains(q))
                            }
                        };
                        dropped[at] |= !plain;
                        i = after;
                    }
                    'A' | 'a' => i = value().1,
                    _ => {
                        letters.push((k, cur.clone()));
                        i += 2;
                    }
                }
            }
            ('{', _) => {
                groups.push(cur.clone());
                i += 1;
            }
            ('}', _) => {
                if let Some(f) = groups.pop() {
                    cur = f;
                }
                i += 1;
            }
            ('%', Some('%')) => {
                let code = chars.get(i + 2).map(char::to_ascii_lowercase);
                match code {
                    Some('d') => letters.push(('°', cur.clone())),
                    Some('p') => letters.push(('±', cur.clone())),
                    Some('c') => letters.push(('Ø', cur.clone())),
                    Some('%') => letters.push(('%', cur.clone())),
                    Some('u' | 'o' | 'k') => {}
                    Some(d) if d.is_ascii_digit() => {
                        let digits: String = chars[i + 2..]
                            .iter()
                            .take(3)
                            .take_while(|c| c.is_ascii_digit())
                            .collect();
                        if let Some(l) = digits.parse::<u32>().ok().and_then(char::from_u32) {
                            letters.push((l, cur.clone()));
                        }
                        i += 2 + digits.len();
                        continue;
                    }
                    _ => {
                        letters.push(('%', cur.clone()));
                        i += 1;
                        continue;
                    }
                }
                i += 3;
            }
            _ => {
                letters.push((c, cur.clone()));
                i += 1;
            }
        }
    }
    // Each line's trailing spaces go.
    let mut kept: Vec<(char, Format)> = Vec::with_capacity(letters.len());
    let mut line: Vec<(char, Format)> = Vec::new();
    for l in letters
        .into_iter()
        .chain(std::iter::once(('\n', Format::default())))
    {
        if l.0 == '\n' {
            while line.last().is_some_and(|(c, _)| c.is_whitespace()) {
                line.pop();
            }
            kept.append(&mut line);
            kept.push(l);
        } else {
            line.push(l);
        }
    }
    kept.pop();
    let mut runs: Vec<TextRun> = Vec::new();
    for (i, (_, f)) in kept.iter().enumerate() {
        if f.plain() {
            continue;
        }
        let i = i as u32;
        match runs.last_mut() {
            Some(r)
                if r.end == i
                    && r.bold == f.bold
                    && r.italic == f.italic
                    && r.underline == f.underline
                    && r.script == f.script
                    && r.color == f.color =>
            {
                r.end = i + 1;
            }
            _ => runs.push(TextRun {
                start: i,
                end: i + 1,
                bold: f.bold,
                italic: f.italic,
                underline: f.underline,
                script: f.script,
                color: f.color.clone(),
            }),
        }
    }
    MtextContent {
        text: kept.into_iter().map(|(c, _)| c).collect(),
        runs,
        width_factor,
        dropped: DROPPED
            .iter()
            .zip(dropped)
            .filter_map(|(name, on)| on.then_some(*name))
            .collect(),
    }
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
