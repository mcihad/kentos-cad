//! Text encodings of the files survey offices exchange: UTF-8 (with or
//! without a byte order mark), UTF-16 (Excel's "Unicode text"), the
//! Windows code pages older AutoCAD and Netcad files use: 1254 (Turkish)
//! and 1252 (Western European), and those of dBASE tables (a Shapefile's
//! .dbf, docs/adr/0046): ISO-8859-9 (Latin-5) and the DOS code pages 857
//! (Turkish), 850 and 437. Bytes a code page leaves undefined become
//! U+FFFD (Windows: the C1 control of the same value, as WHATWG decodes),
//! never dropped silently.

/// A text encoding a reader can decode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Encoding {
    Utf8,
    Utf16Le,
    Utf16Be,
    Windows1252,
    Windows1254,
    Iso8859_9,
    Cp857,
    Cp850,
    Cp437,
}

impl Encoding {
    /// How the report names it.
    pub fn label(self) -> &'static str {
        match self {
            Encoding::Utf8 => "UTF-8",
            Encoding::Utf16Le | Encoding::Utf16Be => "UTF-16",
            Encoding::Windows1252 => "Windows-1252 (Batı Avrupa)",
            Encoding::Windows1254 => "Windows-1254 (Türkçe)",
            Encoding::Iso8859_9 => "ISO-8859-9 (Latin-5, Türkçe)",
            Encoding::Cp857 => "CP857 (DOS Türkçe)",
            Encoding::Cp850 => "CP850 (DOS Batı Avrupa)",
            Encoding::Cp437 => "CP437 (DOS ABD)",
        }
    }

    /// The encoding's usual name ("Windows-1254", "CP857").
    pub fn name(self) -> &'static str {
        match self {
            Encoding::Utf8 => "UTF-8",
            Encoding::Utf16Le => "UTF-16LE",
            Encoding::Utf16Be => "UTF-16BE",
            Encoding::Windows1252 => "Windows-1252",
            Encoding::Windows1254 => "Windows-1254",
            Encoding::Iso8859_9 => "ISO-8859-9",
            Encoding::Cp857 => "CP857",
            Encoding::Cp850 => "CP850",
            Encoding::Cp437 => "CP437",
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

/// Characters of bytes 0x80–0xFF in IBM code page 857 (DOS Turkish);
/// 0xD5, 0xE7 and 0xF2 are undefined.
const CP857_HIGH: [u16; 128] = [
    0x00C7, 0x00FC, 0x00E9, 0x00E2, 0x00E4, 0x00E0, 0x00E5, 0x00E7, 0x00EA, 0x00EB, 0x00E8, 0x00EF,
    0x00EE, 0x0131, 0x00C4, 0x00C5, 0x00C9, 0x00E6, 0x00C6, 0x00F4, 0x00F6, 0x00F2, 0x00FB, 0x00F9,
    0x0130, 0x00D6, 0x00DC, 0x00F8, 0x00A3, 0x00D8, 0x015E, 0x015F, 0x00E1, 0x00ED, 0x00F3, 0x00FA,
    0x00F1, 0x00D1, 0x011E, 0x011F, 0x00BF, 0x00AE, 0x00AC, 0x00BD, 0x00BC, 0x00A1, 0x00AB, 0x00BB,
    0x2591, 0x2592, 0x2593, 0x2502, 0x2524, 0x00C1, 0x00C2, 0x00C0, 0x00A9, 0x2563, 0x2551, 0x2557,
    0x255D, 0x00A2, 0x00A5, 0x2510, 0x2514, 0x2534, 0x252C, 0x251C, 0x2500, 0x253C, 0x00E3, 0x00C3,
    0x255A, 0x2554, 0x2569, 0x2566, 0x2560, 0x2550, 0x256C, 0x00A4, 0x00BA, 0x00AA, 0x00CA, 0x00CB,
    0x00C8, 0xFFFD, 0x00CD, 0x00CE, 0x00CF, 0x2518, 0x250C, 0x2588, 0x2584, 0x00A6, 0x00CC, 0x2580,
    0x00D3, 0x00DF, 0x00D4, 0x00D2, 0x00F5, 0x00D5, 0x00B5, 0xFFFD, 0x00D7, 0x00DA, 0x00DB, 0x00D9,
    0x00EC, 0x00FF, 0x00AF, 0x00B4, 0x00AD, 0x00B1, 0xFFFD, 0x00BE, 0x00B6, 0x00A7, 0x00F7, 0x00B8,
    0x00B0, 0x00A8, 0x00B7, 0x00B9, 0x00B3, 0x00B2, 0x25A0, 0x00A0,
];

/// Characters of bytes 0x80–0xFF in IBM code page 850 (DOS Western European).
const CP850_HIGH: [u16; 128] = [
    0x00C7, 0x00FC, 0x00E9, 0x00E2, 0x00E4, 0x00E0, 0x00E5, 0x00E7, 0x00EA, 0x00EB, 0x00E8, 0x00EF,
    0x00EE, 0x00EC, 0x00C4, 0x00C5, 0x00C9, 0x00E6, 0x00C6, 0x00F4, 0x00F6, 0x00F2, 0x00FB, 0x00F9,
    0x00FF, 0x00D6, 0x00DC, 0x00F8, 0x00A3, 0x00D8, 0x00D7, 0x0192, 0x00E1, 0x00ED, 0x00F3, 0x00FA,
    0x00F1, 0x00D1, 0x00AA, 0x00BA, 0x00BF, 0x00AE, 0x00AC, 0x00BD, 0x00BC, 0x00A1, 0x00AB, 0x00BB,
    0x2591, 0x2592, 0x2593, 0x2502, 0x2524, 0x00C1, 0x00C2, 0x00C0, 0x00A9, 0x2563, 0x2551, 0x2557,
    0x255D, 0x00A2, 0x00A5, 0x2510, 0x2514, 0x2534, 0x252C, 0x251C, 0x2500, 0x253C, 0x00E3, 0x00C3,
    0x255A, 0x2554, 0x2569, 0x2566, 0x2560, 0x2550, 0x256C, 0x00A4, 0x00F0, 0x00D0, 0x00CA, 0x00CB,
    0x00C8, 0x0131, 0x00CD, 0x00CE, 0x00CF, 0x2518, 0x250C, 0x2588, 0x2584, 0x00A6, 0x00CC, 0x2580,
    0x00D3, 0x00DF, 0x00D4, 0x00D2, 0x00F5, 0x00D5, 0x00B5, 0x00FE, 0x00DE, 0x00DA, 0x00DB, 0x00D9,
    0x00FD, 0x00DD, 0x00AF, 0x00B4, 0x00AD, 0x00B1, 0x2017, 0x00BE, 0x00B6, 0x00A7, 0x00F7, 0x00B8,
    0x00B0, 0x00A8, 0x00B7, 0x00B9, 0x00B3, 0x00B2, 0x25A0, 0x00A0,
];

/// Characters of bytes 0x80–0xFF in IBM code page 437 (the original PC's).
const CP437_HIGH: [u16; 128] = [
    0x00C7, 0x00FC, 0x00E9, 0x00E2, 0x00E4, 0x00E0, 0x00E5, 0x00E7, 0x00EA, 0x00EB, 0x00E8, 0x00EF,
    0x00EE, 0x00EC, 0x00C4, 0x00C5, 0x00C9, 0x00E6, 0x00C6, 0x00F4, 0x00F6, 0x00F2, 0x00FB, 0x00F9,
    0x00FF, 0x00D6, 0x00DC, 0x00A2, 0x00A3, 0x00A5, 0x20A7, 0x0192, 0x00E1, 0x00ED, 0x00F3, 0x00FA,
    0x00F1, 0x00D1, 0x00AA, 0x00BA, 0x00BF, 0x2310, 0x00AC, 0x00BD, 0x00BC, 0x00A1, 0x00AB, 0x00BB,
    0x2591, 0x2592, 0x2593, 0x2502, 0x2524, 0x2561, 0x2562, 0x2556, 0x2555, 0x2563, 0x2551, 0x2557,
    0x255D, 0x255C, 0x255B, 0x2510, 0x2514, 0x2534, 0x252C, 0x251C, 0x2500, 0x253C, 0x255E, 0x255F,
    0x255A, 0x2554, 0x2569, 0x2566, 0x2560, 0x2550, 0x256C, 0x2567, 0x2568, 0x2564, 0x2565, 0x2559,
    0x2558, 0x2552, 0x2553, 0x256B, 0x256A, 0x2518, 0x250C, 0x2588, 0x2584, 0x258C, 0x2590, 0x2580,
    0x03B1, 0x00DF, 0x0393, 0x03C0, 0x03A3, 0x03C3, 0x00B5, 0x03C4, 0x03A6, 0x0398, 0x03A9, 0x03B4,
    0x221E, 0x03C6, 0x03B5, 0x2229, 0x2261, 0x00B1, 0x2265, 0x2264, 0x2320, 0x2321, 0x00F7, 0x2248,
    0x00B0, 0x2219, 0x00B7, 0x221A, 0x207F, 0x00B2, 0x25A0, 0x00A0,
];

/// A byte of a single-byte code page as a character.
fn single_byte(b: u8, enc: Encoding) -> char {
    let high = usize::from(b.wrapping_sub(0x80));
    let code: u32 = match (b, enc) {
        (0x00..=0x7F, _) => u32::from(b),
        (_, Encoding::Cp857) => u32::from(CP857_HIGH[high]),
        (_, Encoding::Cp850) => u32::from(CP850_HIGH[high]),
        (_, Encoding::Cp437) => u32::from(CP437_HIGH[high]),
        // Latin-5: Latin-1 with the six Turkish letters, C1 controls kept.
        (0xD0, Encoding::Iso8859_9) => 0x011E,
        (0xDD, Encoding::Iso8859_9) => 0x0130,
        (0xDE, Encoding::Iso8859_9) => 0x015E,
        (0xF0, Encoding::Iso8859_9) => 0x011F,
        (0xFD, Encoding::Iso8859_9) => 0x0131,
        (0xFE, Encoding::Iso8859_9) => 0x015F,
        (_, Encoding::Iso8859_9) => u32::from(b),
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
        Encoding::Windows1252
        | Encoding::Windows1254
        | Encoding::Iso8859_9
        | Encoding::Cp857
        | Encoding::Cp850
        | Encoding::Cp437 => bytes.iter().map(|&b| single_byte(b, enc)).collect(),
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
    fn the_turkish_letters_of_the_dbase_code_pages() {
        // From the code page charts (IBM 857; ISO/IEC 8859-9), not from the tables above.
        let cp857 = [
            0x80, 0x87, 0x8D, 0x98, 0x9E, 0x9F, 0xA6, 0xA7, 0x94, 0x99, 0x81, 0x9A,
        ];
        assert_eq!(decode(&cp857, Encoding::Cp857), "ÇçıİŞşĞğöÖüÜ");
        assert_eq!(
            decode(
                &[0xD0, 0xDD, 0xDE, 0xF0, 0xFD, 0xFE, 0xC7, 0xE7],
                Encoding::Iso8859_9
            ),
            "ĞİŞğışÇç"
        );
        assert_eq!(
            decode(&[0x80, 0x9A, 0xD5, 0xE7, 0xF2], Encoding::Cp857),
            "ÇÜ\u{FFFD}\u{FFFD}\u{FFFD}"
        );
        assert_eq!(decode(&[0x9B, 0xE1, 0xD5], Encoding::Cp850), "øßı");
        assert_eq!(decode(&[0x9B, 0xE1, 0xE3], Encoding::Cp437), "¢ßπ");
    }

    #[test]
    fn utf16_decodes_both_byte_orders() {
        let le: Vec<u8> = "Ş1".encode_utf16().flat_map(|u| u.to_le_bytes()).collect();
        let be: Vec<u8> = "Ş1".encode_utf16().flat_map(|u| u.to_be_bytes()).collect();
        assert_eq!(decode(&le, Encoding::Utf16Le), "Ş1");
        assert_eq!(decode(&be, Encoding::Utf16Be), "Ş1");
    }
}
