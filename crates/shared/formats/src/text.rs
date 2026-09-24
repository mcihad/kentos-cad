//! Text encodings of the files survey offices exchange: UTF-8 (with or
//! without a byte order mark), UTF-16 (Excel's "Unicode text"), and the
//! Windows code pages older AutoCAD and Netcad files use: 1254 (Turkish)
//! and 1252 (Western European). Undefined bytes become U+FFFD and are
//! counted, never dropped silently.

/// A text encoding a reader can decode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Encoding {
    Utf8,
    Utf16Le,
    Utf16Be,
    Windows1252,
    Windows1254,
}

impl Encoding {
    /// How the report names it.
    pub fn label(self) -> &'static str {
        match self {
            Encoding::Utf8 => "UTF-8",
            Encoding::Utf16Le | Encoding::Utf16Be => "UTF-16",
            Encoding::Windows1252 => "Windows-1252 (Batı Avrupa)",
            Encoding::Windows1254 => "Windows-1254 (Türkçe)",
        }
    }
}

/// A byte order mark at the start: its encoding and length.
pub fn bom(bytes: &[u8]) -> Option<(Encoding, usize)> {
    match bytes {
        [0xEF, 0xBB, 0xBF, ..] => Some((Encoding::Utf8, 3)),
        [0xFF, 0xFE, ..] => Some((Encoding::Utf16Le, 2)),
        [0xFE, 0xFF, ..] => Some((Encoding::Utf16Be, 2)),
        _ => None,
    }
}

/// The encoding of a text file with no declared encoding: its byte order
/// mark, else UTF-8 when the bytes are valid UTF-8, else Windows-1254 (the
/// code page of Turkish Windows, where such files come from).
pub fn sniff(bytes: &[u8]) -> (Encoding, usize) {
    if let Some(b) = bom(bytes) {
        return b;
    }
    if std::str::from_utf8(bytes).is_ok() {
        (Encoding::Utf8, 0)
    } else {
        (Encoding::Windows1254, 0)
    }
}

/// Characters of bytes 0x80–0x9F in Windows-1252 (WHATWG index; the
/// undefined ones map to the C1 control of the same value).
const W1252_HIGH: [u16; 32] = [
    0x20AC, 0x0081, 0x201A, 0x0192, 0x201E, 0x2026, 0x2020, 0x2021, 0x02C6, 0x2030, 0x0160, 0x2039,
    0x0152, 0x008D, 0x017D, 0x008F, 0x0090, 0x2018, 0x2019, 0x201C, 0x201D, 0x2022, 0x2013, 0x2014,
    0x02DC, 0x2122, 0x0161, 0x203A, 0x0153, 0x009D, 0x017E, 0x0178,
];

/// A byte of a single-byte code page as a character.
fn single_byte(b: u8, enc: Encoding) -> char {
    let code: u32 = match (b, enc) {
        (0x00..=0x7F, _) => u32::from(b),
        (0x80..=0x9F, Encoding::Windows1254) if b == 0x8E || b == 0x9E => u32::from(b),
        (0x80..=0x9F, _) => u32::from(W1252_HIGH[usize::from(b - 0x80)]),
        (0xD0, Encoding::Windows1254) => 0x011E,
        (0xDD, Encoding::Windows1254) => 0x0130,
        (0xDE, Encoding::Windows1254) => 0x015E,
        (0xF0, Encoding::Windows1254) => 0x011F,
        (0xFD, Encoding::Windows1254) => 0x0131,
        (0xFE, Encoding::Windows1254) => 0x015F,
        _ => u32::from(b),
    };
    char::from_u32(code).unwrap_or(char::REPLACEMENT_CHARACTER)
}

/// Decodes bytes (after any byte order mark) as `enc`; invalid sequences become U+FFFD.
pub fn decode(bytes: &[u8], enc: Encoding) -> String {
    match enc {
        Encoding::Utf8 => String::from_utf8_lossy(bytes).into_owned(),
        Encoding::Utf16Le | Encoding::Utf16Be => {
            let units = bytes.chunks_exact(2).map(|c| {
                if enc == Encoding::Utf16Le {
                    u16::from_le_bytes([c[0], c[1]])
                } else {
                    u16::from_be_bytes([c[0], c[1]])
                }
            });
            char::decode_utf16(units)
                .map(|r| r.unwrap_or(char::REPLACEMENT_CHARACTER))
                .collect()
        }
        Encoding::Windows1252 | Encoding::Windows1254 => {
            bytes.iter().map(|&b| single_byte(b, enc)).collect()
        }
    }
}

/// The Windows-1254 byte of a character, if the code page has one.
pub fn windows1254_byte(c: char) -> Option<u8> {
    let code = u32::from(c);
    if code < 0x80 {
        return Some(code as u8);
    }
    match code {
        0x011E => Some(0xD0),
        0x0130 => Some(0xDD),
        0x015E => Some(0xDE),
        0x011F => Some(0xF0),
        0x0131 => Some(0xFD),
        0x015F => Some(0xFE),
        // Latin-1 letters Windows-1254 replaced with the Turkish ones.
        0x00D0 | 0x00DD | 0x00DE | 0x00F0 | 0x00FD | 0x00FE => None,
        0xA0..=0xFF => Some(code as u8),
        _ => W1252_HIGH
            .iter()
            .position(|&u| u32::from(u) == code && !(0x80..0xA0).contains(&code))
            .map(|i| 0x80 + i as u8)
            .filter(|&b| b != 0x8E && b != 0x9E),
    }
}

/// Encodes text as Windows-1254; a character the code page lacks is handed
/// to `other`, which writes its replacement (an escape, or '?').
pub fn encode_windows1254(s: &str, out: &mut Vec<u8>, mut other: impl FnMut(char, &mut Vec<u8>)) {
    for c in s.chars() {
        match windows1254_byte(c) {
            Some(b) => out.push(b),
            None => other(c, out),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn turkish_letters_round_trip_through_windows_1254() {
        let s = "Çağrı İşık ığdır Şükrü ÖZGÜR € ‘x’";
        let mut bytes = Vec::new();
        encode_windows1254(s, &mut bytes, |_, o| o.push(b'?'));
        assert!(!bytes.contains(&b'?'));
        assert_eq!(decode(&bytes, Encoding::Windows1254), s);
        // The six Turkish letters sit where Latin-1 has Ð Ý Þ ð ý þ.
        assert_eq!(
            decode(&[0xD0, 0xDD, 0xDE, 0xF0, 0xFD, 0xFE], Encoding::Windows1254),
            "ĞİŞğış"
        );
        assert_eq!(
            decode(&[0xD0, 0xDD, 0xDE, 0xF0, 0xFD, 0xFE], Encoding::Windows1252),
            "ÐÝÞðýþ"
        );
    }

    #[test]
    fn characters_outside_the_code_page_go_to_the_fallback() {
        let mut bytes = Vec::new();
        let mut lost = Vec::new();
        encode_windows1254("aЖ😀Ð", &mut bytes, |c, o| {
            lost.push(c);
            o.push(b'?');
        });
        assert_eq!(bytes, b"a???");
        assert_eq!(lost, vec!['Ж', '😀', 'Ð']);
    }

    #[test]
    fn sniffing_prefers_a_bom_then_utf8_then_turkish() {
        assert_eq!(sniff(b"\xEF\xBB\xBFabc"), (Encoding::Utf8, 3));
        assert_eq!(sniff(b"\xFF\xFEa\0"), (Encoding::Utf16Le, 2));
        assert_eq!(sniff("Ağaç".as_bytes()), (Encoding::Utf8, 0));
        assert_eq!(sniff(b"A\xF0a\xE7"), (Encoding::Windows1254, 0));
        assert_eq!(decode(b"A\xF0a\xE7", Encoding::Windows1254), "Ağaç");
    }

    #[test]
    fn utf16_decodes_both_byte_orders() {
        let le: Vec<u8> = "Ş1".encode_utf16().flat_map(|u| u.to_le_bytes()).collect();
        let be: Vec<u8> = "Ş1".encode_utf16().flat_map(|u| u.to_be_bytes()).collect();
        assert_eq!(decode(&le, Encoding::Utf16Le), "Ş1");
        assert_eq!(decode(&be, Encoding::Utf16Be), "Ş1");
    }
}
