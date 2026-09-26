//! A Shapefile's attribute table: a dBASE III file (.dbf). Its text is in
//! the code page the .cpg file names, else the one the table's language
//! driver byte says, else Windows-1254 (Turkish Windows, where such files
//! come from here; said in the report, never guessed silently). Values
//! become text attributes: character fields trimmed, numbers as written,
//! logicals as true/false, dates as YYYY-MM-DD; memo and binary fields are
//! left out and reported.

use std::collections::BTreeMap;

use crate::text::{Encoding, decode};

/// Where the table's encoding came from.
#[derive(Clone, Debug, PartialEq)]
pub enum EncodingSource {
    /// The .cpg file named it.
    Cpg(String),
    /// The .cpg file named something unknown: the default was taken.
    UnknownCpg(String),
    /// The table's language driver byte (offset 29).
    Driver(u8),
    /// Nothing said it.
    Default,
}

/// The encoding of a table from its .cpg text and its language driver byte.
pub fn encoding(cpg: Option<&[u8]>, driver: u8) -> (Encoding, EncodingSource) {
    if let Some(cpg) = cpg {
        let text = String::from_utf8_lossy(cpg);
        let key: String = text
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect::<String>()
            .to_ascii_uppercase();
        let enc = match key.as_str() {
            "UTF-8" | "UTF8" | "65001" => Some(Encoding::Utf8),
            "1254" | "CP1254" | "WINDOWS-1254" | "ANSI1254" => Some(Encoding::Windows1254),
            "1252" | "CP1252" | "WINDOWS-1252" | "ANSI1252" => Some(Encoding::Windows1252),
            "ISO-8859-9" | "ISO8859-9" | "28599" | "LATIN5" => Some(Encoding::Iso8859_9),
            "857" | "CP857" | "IBM857" | "OEM857" => Some(Encoding::Cp857),
            "850" | "CP850" | "IBM850" => Some(Encoding::Cp850),
            "437" | "CP437" | "IBM437" => Some(Encoding::Cp437),
            _ => None,
        };
        let shown = text.trim().chars().take(40).collect::<String>();
        return match enc {
            Some(e) => (e, EncodingSource::Cpg(shown)),
            None => (Encoding::Windows1254, EncodingSource::UnknownCpg(shown)),
        };
    }
    let enc = match driver {
        0xCA => Encoding::Windows1254,
        0x6B | 0x88 => Encoding::Cp857,
        0x03 | 0x58 | 0x59 => Encoding::Windows1252,
        0x02 | 0x0A | 0x0E | 0x10 | 0x12 | 0x14 | 0x16 | 0x1A | 0x1D | 0x25 | 0x37 => {
            Encoding::Cp850
        }
        0x01 | 0x09 | 0x0B | 0x0D | 0x0F | 0x11 | 0x15 | 0x18 | 0x19 | 0x1B => Encoding::Cp437,
        _ => return (Encoding::Windows1254, EncodingSource::Default),
    };
    (enc, EncodingSource::Driver(driver))
}

/// A field of the table.
pub struct Field {
    pub name: String,
    pub kind: u8,
    /// Where it starts in a record (after the deletion flag) and its width.
    offset: usize,
    len: usize,
}

/// The table, read lazily record by record.
pub struct Table<'a> {
    b: &'a [u8],
    pub fields: Vec<Field>,
    header_len: usize,
    record_len: usize,
    /// Complete records the file holds (at most what its header says).
    pub count: usize,
    /// What the header says.
    pub declared: usize,
    enc: Encoding,
}

fn u16_le(b: &[u8], at: usize) -> Option<usize> {
    Some(usize::from(u16::from_le_bytes(
        b.get(at..at + 2)?.try_into().ok()?,
    )))
}

fn u32_le(b: &[u8], at: usize) -> Option<usize> {
    usize::try_from(u32::from_le_bytes(b.get(at..at + 4)?.try_into().ok()?)).ok()
}

impl<'a> Table<'a> {
    /// Reads the header; fields that do not fit the record are left out.
    pub fn open(b: &'a [u8], enc: Encoding) -> Result<Table<'a>, String> {
        let (Some(declared), Some(header_len), Some(record_len)) =
            (u32_le(b, 4), u16_le(b, 8), u16_le(b, 10))
        else {
            return Err(
                "“.dbf” dosyası 32 baytlık başlıktan kısa: öznitelik tablosu okunamadı.".into(),
            );
        };
        if b.len() < 32 {
            return Err(
                "“.dbf” dosyası 32 baytlık başlıktan kısa: öznitelik tablosu okunamadı.".into(),
            );
        }
        if header_len < 33 || record_len < 1 {
            return Err("“.dbf” başlığı bozuk (başlık ya da kayıt uzunluğu geçersiz): öznitelik tablosu okunamadı.".into());
        }
        let mut fields = Vec::new();
        let mut offset = 1;
        let mut at = 32;
        while at + 32 <= b.len().min(header_len) && b[at] != 0x0D {
            let d = &b[at..at + 32];
            let name_len = d[..11].iter().position(|&c| c == 0).unwrap_or(11);
            let name = decode(&d[..name_len], enc);
            let len = usize::from(d[16]);
            fields.push(Field {
                name,
                kind: d[11],
                offset,
                len,
            });
            offset += len;
            at += 32;
        }
        let whole = b.len().saturating_sub(header_len) / record_len;
        Ok(Table {
            b,
            fields,
            header_len,
            record_len,
            count: declared.min(whole),
            declared,
            enc,
        })
    }

    /// Fields beyond the record's length (a broken header): their names.
    pub fn outside(&self) -> impl Iterator<Item = &Field> {
        self.fields
            .iter()
            .filter(move |f| f.offset + f.len > self.record_len)
    }

    /// The `i`th record: whether it is deleted, and its attributes.
    pub fn record(&self, i: usize) -> Option<(bool, BTreeMap<String, String>)> {
        if i >= self.count {
            return None;
        }
        let start = self.header_len + i * self.record_len;
        let rec = self.b.get(start..start + self.record_len)?;
        let deleted = rec.first() == Some(&b'*');
        let mut attrs = BTreeMap::new();
        for f in &self.fields {
            if f.name.is_empty() {
                continue;
            }
            let Some(raw) = rec.get(f.offset..f.offset + f.len) else {
                continue;
            };
            if let Some(v) = value(f.kind, raw, self.enc) {
                attrs.insert(f.name.clone(), v);
            }
        }
        Some((deleted, attrs))
    }
}

fn blank(c: char) -> bool {
    c == ' ' || c == '\0'
}

/// ASCII text (numbers, logicals, dates); any other byte is U+FFFD.
fn ascii(raw: &[u8]) -> String {
    raw.iter()
        .map(|&b| {
            if b.is_ascii() {
                b as char
            } else {
                char::REPLACEMENT_CHARACTER
            }
        })
        .collect()
}

/// A field's value as text; None leaves the attribute out. Padding is
/// spaces, or NULs as some writers put.
fn value(kind: u8, raw: &[u8], enc: Encoding) -> Option<String> {
    let v = match kind {
        b'C' => decode(raw, enc).trim_end_matches(blank).to_string(),
        b'N' | b'F' => ascii(raw).trim_matches(blank).to_string(),
        b'L' => match ascii(raw).trim_matches(blank) {
            "T" | "t" | "Y" | "y" => "true".to_string(),
            "F" | "f" | "N" | "n" => "false".to_string(),
            _ => return None,
        },
        b'D' => {
            let t = ascii(raw).trim_matches(blank).to_string();
            if t == "00000000" {
                return None;
            }
            if t.len() == 8 && t.bytes().all(|b| b.is_ascii_digit()) {
                format!("{}-{}-{}", &t[..4], &t[4..6], &t[6..])
            } else {
                t
            }
        }
        _ => return None,
    };
    (!v.is_empty()).then_some(v)
}

/// The type letter of a field as the report names it.
pub fn kind_name(kind: u8) -> String {
    match kind {
        b'M' => "M (not/memo)".into(),
        b'B' => "B (ikili)".into(),
        b'G' => "G (OLE)".into(),
        b'I' => "I (tamsayı, ikili)".into(),
        b'T' | b'@' => "T (tarih-saat)".into(),
        b'Y' => "Y (para)".into(),
        b'O' => "O (çift duyarlı, ikili)".into(),
        k if k.is_ascii_graphic() => format!("{}", k as char),
        k => format!("0x{k:02X}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A table of fields (name, type, width) and rows of raw values.
    fn table(fields: &[(&str, u8, u8)], rows: &[(&[u8], bool)], driver: u8) -> Vec<u8> {
        let record_len: usize = 1 + fields.iter().map(|f| usize::from(f.2)).sum::<usize>();
        let header_len = 32 + 32 * fields.len() + 1;
        let mut b = vec![0u8; 32];
        b[0] = 3;
        b[4..8].copy_from_slice(&(rows.len() as u32).to_le_bytes());
        b[8..10].copy_from_slice(&(header_len as u16).to_le_bytes());
        b[10..12].copy_from_slice(&(record_len as u16).to_le_bytes());
        b[29] = driver;
        for (name, kind, len) in fields {
            let mut d = [0u8; 32];
            d[..name.len()].copy_from_slice(name.as_bytes());
            d[11] = *kind;
            d[16] = *len;
            b.extend_from_slice(&d);
        }
        b.push(0x0D);
        for (raw, deleted) in rows {
            b.push(if *deleted { b'*' } else { b' ' });
            b.extend_from_slice(raw);
        }
        b.push(0x1A);
        b
    }

    #[test]
    fn values_become_text_as_the_rules_say() {
        let b = table(
            &[
                ("AD", b'C', 6),
                ("ALAN", b'N', 8),
                ("OK", b'L', 1),
                ("TARIH", b'D', 8),
                ("NOT", b'M', 10),
            ],
            &[
                (b"\xD0\xFDk   1234.50 T20260926          ", false),
                (b"      \0\0\0\0\0\0\0\0?00000000xxxxxxxxxx", true),
            ],
            0xCA,
        );
        let (enc, src) = encoding(None, b[29]);
        assert_eq!(
            (enc, src),
            (Encoding::Windows1254, EncodingSource::Driver(0xCA))
        );
        let t = Table::open(&b, enc).expect("table");
        assert_eq!(t.count, 2);
        let (deleted, a) = t.record(0).expect("row");
        assert!(!deleted);
        let got: Vec<(&str, &str)> = a.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
        assert_eq!(
            got,
            vec![
                ("AD", "Ğık"),
                ("ALAN", "1234.50"),
                ("OK", "true"),
                ("TARIH", "2026-09-26")
            ]
        );
        let (deleted, a) = t.record(1).expect("row");
        assert!(deleted && a.is_empty(), "{a:?}");
        assert!(t.record(2).is_none());
    }

    #[test]
    fn the_cpg_names_the_encoding_before_the_driver_byte() {
        assert_eq!(encoding(Some(b"UTF-8\r\n"), 0xCA).0, Encoding::Utf8);
        assert_eq!(encoding(Some(b"ANSI 1254"), 0x00).0, Encoding::Windows1254);
        assert_eq!(encoding(Some(b"oem 857"), 0x00).0, Encoding::Cp857);
        assert_eq!(encoding(Some(b"ISO 8859-9"), 0x00).0, Encoding::Iso8859_9);
        let (e, s) = encoding(Some(b"KOI8-R"), 0x57);
        assert_eq!(
            (e, s),
            (
                Encoding::Windows1254,
                EncodingSource::UnknownCpg("KOI8-R".into())
            )
        );
        assert_eq!(
            encoding(None, 0x57),
            (Encoding::Windows1254, EncodingSource::Default)
        );
        assert_eq!(
            encoding(None, 0x6B),
            (Encoding::Cp857, EncodingSource::Driver(0x6B))
        );
    }

    #[test]
    fn a_cut_or_lying_table_reads_what_it_holds() {
        let mut b = table(&[("A", b'C', 4)], &[(b"abcd", false), (b"efgh", false)], 0);
        // The header claims a million records; only the complete ones count.
        b[4..8].copy_from_slice(&1_000_000u32.to_le_bytes());
        b.truncate(b.len() - 3);
        let t = Table::open(&b, Encoding::Windows1254).expect("table");
        assert_eq!((t.count, t.declared), (1, 1_000_000));
        assert!(Table::open(&b[..20], Encoding::Utf8).is_err());
        let mut broken = b.clone();
        broken[8..10].copy_from_slice(&0u16.to_le_bytes());
        assert!(Table::open(&broken, Encoding::Utf8).is_err());
    }
}
