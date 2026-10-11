//! A photo's EXIF (docs/adr/0225 §3; EXIF 2.32, CIPA DC-008): where it was
//! taken, how high, the direction it looked, when, and with which camera,
//! read from a JPEG's APP1 (`Exif\0\0` and a TIFF header) or a TIFF file's
//! own header. Only the header is read: the JPEG's segments up to its first
//! scan, the TIFF's IFD0, its Exif IFD and its GPS IFD (no other IFD, none
//! twice). A place's degrees, minutes and seconds are three rationals, added
//! as fractions and rounded to the nearest float64 once (`field::exact`).
//! Nothing the file says panics or loops: offsets are checked, an IFD has at
//! most `MAX_ENTRIES` entries.
//!
//! The place is WGS 84 as the file wrote it; the app moves it into the
//! project's system (docs/adr/0167). Its height is above sea level, its
//! datum unsaid: the app writes it as an attribute, not an elevation
//! (docs/adr/0169). `scripts/fixtures/exif_cases.py` writes the photos of
//! `fixtures/exif/v1` by hand and their expected values from the rules, and
//! reads them back with Pillow.

use kentos_contracts::PhotoExif;

use crate::field::exact::Exact;

/// The most entries an IFD may have; past it the header is broken.
pub const MAX_ENTRIES: usize = 1_000;

const NO_JPEG: &str = "Dosya JPEG değil.";
const NO_EXIF: &str = "Fotoğrafta EXIF bilgisi yok.";
const BROKEN: &str = "Fotoğrafın EXIF bilgisi bozuk.";
const NO_GPS: &str = "Fotoğrafta GPS konumu yok.";
const BAD_GPS: &str = "Fotoğrafın GPS konumu okunamadı.";
const FAR_GPS: &str = "Fotoğrafın GPS konumu enlem ya da boylam sınırlarının dışında.";

/// Where a JPEG's EXIF is: its TIFF bytes, none, or past the bytes given
/// (the caller reads more of the file: `needs_more`).
enum Found<'a> {
    Tiff(&'a [u8]),
    None,
    Truncated,
}

/// The TIFF bytes of a JPEG's EXIF; `None` when the file is no JPEG.
fn jpeg(bytes: &[u8]) -> Option<Found<'_>> {
    if bytes.get(..2) != Some(&[0xFF, 0xD8]) {
        return None;
    }
    let mut i = 2;
    loop {
        // Fill bytes before a marker.
        while bytes.get(i) == Some(&0xFF) && bytes.get(i + 1) == Some(&0xFF) {
            i += 1;
        }
        let (Some(&0xFF), Some(&marker)) = (bytes.get(i), bytes.get(i + 1)) else {
            return Some(if i + 1 >= bytes.len() {
                Found::Truncated
            } else {
                Found::None
            });
        };
        // The image's data (start of scan) or its end: no EXIF after them.
        if marker == 0xDA || marker == 0xD9 {
            return Some(Found::None);
        }
        // Markers without a length.
        if marker == 0x01 || (0xD0..=0xD7).contains(&marker) {
            i += 2;
            continue;
        }
        let Some(len) = bytes
            .get(i + 2..i + 4)
            .map(|b| usize::from(u16::from_be_bytes([b[0], b[1]])))
        else {
            return Some(Found::Truncated);
        };
        if len < 2 {
            return Some(Found::None);
        }
        let end = i + 2 + len;
        if marker == 0xE1 {
            let Some(data) = bytes.get(i + 4..end) else {
                return Some(Found::Truncated);
            };
            if let Some(tiff) = data.strip_prefix(b"Exif\0\0") {
                return Some(Found::Tiff(tiff));
            }
        }
        if end > bytes.len() {
            return Some(Found::Truncated);
        }
        i = end;
    }
}

/// Whether more of the file is needed to find its EXIF: the bytes given end
/// before the JPEG's first scan and no EXIF came before.
pub fn needs_more(bytes: &[u8]) -> bool {
    matches!(jpeg(bytes), Some(Found::Truncated))
}

/// A TIFF header's byte order.
#[derive(Clone, Copy)]
struct Tiff<'a> {
    data: &'a [u8],
    little: bool,
}

/// An IFD entry: its type, count and where its value is.
#[derive(Clone, Copy)]
struct Entry {
    kind: u16,
    count: u32,
    /// The value's bytes' offset in the TIFF data.
    at: usize,
}

/// The bytes a value of each type takes.
fn type_size(kind: u16) -> Option<usize> {
    Some(match kind {
        1 | 2 | 6 | 7 => 1,
        3 | 8 => 2,
        4 | 9 | 11 => 4,
        5 | 10 | 12 => 8,
        _ => return None,
    })
}

impl<'a> Tiff<'a> {
    fn new(data: &'a [u8]) -> Option<Self> {
        let little = match data.get(..4)? {
            [b'I', b'I', 42, 0] => true,
            [b'M', b'M', 0, 42] => false,
            _ => return None,
        };
        Some(Self { data, little })
    }

    fn u16_at(&self, at: usize) -> Option<u16> {
        let b = self.data.get(at..at + 2)?;
        Some(if self.little {
            u16::from_le_bytes([b[0], b[1]])
        } else {
            u16::from_be_bytes([b[0], b[1]])
        })
    }

    fn u32_at(&self, at: usize) -> Option<u32> {
        let b = self.data.get(at..at + 4)?;
        let a = [b[0], b[1], b[2], b[3]];
        Some(if self.little {
            u32::from_le_bytes(a)
        } else {
            u32::from_be_bytes(a)
        })
    }

    /// The entries of the IFD at `offset`, by tag; none when it is broken.
    fn ifd(&self, offset: usize) -> Option<Vec<(u16, Entry)>> {
        let count = usize::from(self.u16_at(offset)?);
        if count > MAX_ENTRIES || offset + 2 + 12 * count > self.data.len() {
            return None;
        }
        let mut out = Vec::with_capacity(count);
        for k in 0..count {
            let e = offset + 2 + 12 * k;
            let tag = self.u16_at(e)?;
            let kind = self.u16_at(e + 2)?;
            let n = self.u32_at(e + 4)?;
            let Some(size) = type_size(kind) else {
                // A type this reader does not know: its entry is passed over.
                continue;
            };
            let bytes = size.checked_mul(usize::try_from(n).ok()?)?;
            let at = if bytes <= 4 {
                e + 8
            } else {
                usize::try_from(self.u32_at(e + 8)?).ok()?
            };
            if at.checked_add(bytes)? > self.data.len() {
                return None;
            }
            out.push((tag, Entry { kind, count: n, at }));
        }
        Some(out)
    }

    /// An ASCII value up to its first NUL, without the blanks around it;
    /// none when empty or of another type.
    fn ascii(&self, e: Entry) -> Option<String> {
        if e.kind != 2 {
            return None;
        }
        let raw = self.data.get(e.at..e.at + usize::try_from(e.count).ok()?)?;
        let end = raw.iter().position(|&b| b == 0).unwrap_or(raw.len());
        let text = String::from_utf8_lossy(&raw[..end]);
        let text = text.trim_matches(|c| matches!(c, ' ' | '\t' | '\r' | '\n'));
        (!text.is_empty()).then(|| text.to_owned())
    }

    /// The `k`-th rational of a RATIONAL value; none when it is not one.
    fn rational(&self, e: Entry, k: u32) -> Option<Exact> {
        if e.kind != 5 || k >= e.count {
            return None;
        }
        let at = e.at + 8 * usize::try_from(k).ok()?;
        Exact::ratio(
            i128::from(self.u32_at(at)?),
            i128::from(self.u32_at(at + 4)?),
        )
    }

    /// The first byte of a BYTE value.
    fn byte(&self, e: Entry) -> Option<u8> {
        (e.kind == 1 || e.kind == 7).then(|| self.data.get(e.at).copied())?
    }

    /// The first value of a SHORT or LONG value, as an offset.
    fn offset(&self, e: Entry) -> Option<usize> {
        let v = match e.kind {
            3 => u32::from(self.u16_at(e.at)?),
            4 => self.u32_at(e.at)?,
            _ => return None,
        };
        usize::try_from(v).ok()
    }
}

fn find(entries: &[(u16, Entry)], tag: u16) -> Option<Entry> {
    entries.iter().find(|(t, _)| *t == tag).map(|(_, e)| *e)
}

/// Three rationals as degrees: d + m/60 + s/3600, exactly.
fn degrees(t: &Tiff<'_>, e: Entry) -> Option<Exact> {
    if e.count != 3 {
        return None;
    }
    let d = t.rational(e, 0)?;
    let m = t.rational(e, 1)?.over(60);
    let s = t.rational(e, 2)?.over(3600);
    d.sum(m)?.sum(s)
}

/// `YYYY:MM:DD HH:MM:SS` as ISO 8601 (`YYYY-MM-DDTHH:MM:SS`); none when it
/// is not a moment (the “unknown” zeros among them).
fn moment(text: &str) -> Option<String> {
    let b = text.as_bytes();
    if b.len() != 19
        || b[4] != b':'
        || b[7] != b':'
        || b[10] != b' '
        || b[13] != b':'
        || b[16] != b':'
    {
        return None;
    }
    let num = |r: std::ops::Range<usize>| -> Option<u32> {
        let s = text.get(r)?;
        s.bytes()
            .all(|c| c.is_ascii_digit())
            .then(|| s.parse().ok())?
    };
    let (year, month, day) = (num(0..4)?, num(5..7)?, num(8..10)?);
    let (hour, minute, second) = (num(11..13)?, num(14..16)?, num(17..19)?);
    let ok = year >= 1
        && (1..=12).contains(&month)
        && (1..=31).contains(&day)
        && hour <= 23
        && minute <= 59
        && second <= 59;
    ok.then(|| {
        format!(
            "{}-{}-{}T{}:{}:{}",
            &text[0..4],
            &text[5..7],
            &text[8..10],
            &text[11..13],
            &text[14..16],
            &text[17..19]
        )
    })
}

/// An offset `+HH:MM` or `-HH:MM` within ±14:00.
fn offset_ok(text: &str) -> bool {
    let b = text.as_bytes();
    if b.len() != 6 || !(b[0] == b'+' || b[0] == b'-') || b[3] != b':' {
        return false;
    }
    let digits = |s: &[u8]| s.iter().all(u8::is_ascii_digit);
    if !digits(&b[1..3]) || !digits(&b[4..6]) {
        return false;
    }
    let hours = u32::from(b[1] - b'0') * 10 + u32::from(b[2] - b'0');
    let minutes = u32::from(b[4] - b'0') * 10 + u32::from(b[5] - b'0');
    minutes <= 59 && (hours < 14 || (hours == 14 && minutes == 0))
}

/// The GPS's UTC moment: its date stamp and the floor of its time stamp.
fn gps_moment(t: &Tiff<'_>, gps: &[(u16, Entry)]) -> Option<String> {
    let date = t.ascii(find(gps, 0x001D)?)?;
    let time = find(gps, 0x0007)?;
    if time.count != 3 {
        return None;
    }
    let part = |k| t.rational(time, k).map(Exact::floor);
    let (h, m, s) = (part(0)?, part(1)?, part(2)?);
    if !(0..=23).contains(&h) || !(0..=59).contains(&m) || !(0..=59).contains(&s) {
        return None;
    }
    moment(&format!("{date} {h:02}:{m:02}:{s:02}")).map(|t| format!("{t}Z"))
}

/// The camera: its model, after its maker when the model does not start with it.
fn camera(make: Option<String>, model: Option<String>) -> Option<String> {
    match (make, model) {
        (Some(make), Some(model)) => {
            let starts = model.to_lowercase().starts_with(&make.to_lowercase());
            Some(if starts {
                model
            } else {
                format!("{make} {model}")
            })
        }
        (make, model) => model.or(make),
    }
}

/// What a JPEG's (or a TIFF's) EXIF says; its `problem` says why it has no place.
pub fn read(bytes: &[u8]) -> PhotoExif {
    let tiff = match jpeg(bytes) {
        Some(Found::Tiff(t)) => t,
        Some(Found::None | Found::Truncated) => return problem(NO_EXIF),
        None if bytes.starts_with(b"II*\0") || bytes.starts_with(b"MM\0*") => bytes,
        None => return problem(NO_JPEG),
    };
    read_tiff(tiff).unwrap_or_else(|| problem(BROKEN))
}

fn problem(why: &str) -> PhotoExif {
    PhotoExif {
        problem: Some(why.to_owned()),
        ..PhotoExif::default()
    }
}

/// The TIFF header's answers; none when IFD0 itself is broken.
fn read_tiff(data: &[u8]) -> Option<PhotoExif> {
    let t = Tiff::new(data)?;
    let ifd0 = t.ifd(usize::try_from(t.u32_at(4)?).ok()?)?;
    let mut out = PhotoExif {
        camera: camera(
            find(&ifd0, 0x010F).and_then(|e| t.ascii(e)),
            find(&ifd0, 0x0110).and_then(|e| t.ascii(e)),
        ),
        ..PhotoExif::default()
    };
    // The Exif IFD: when it was taken, and its offset.
    let exif = find(&ifd0, 0x8769)
        .and_then(|e| t.offset(e))
        .and_then(|at| t.ifd(at));
    if let Some(exif) = &exif {
        let local = find(exif, 0x9003)
            .and_then(|e| t.ascii(e))
            .and_then(|s| moment(&s));
        let zone = find(exif, 0x9011)
            .and_then(|e| t.ascii(e))
            .filter(|z| offset_ok(z));
        out.taken = local.map(|m| match zone {
            Some(z) => format!("{m}{z}"),
            None => m,
        });
    }
    let Some(gps) = find(&ifd0, 0x8825).and_then(|e| t.offset(e)) else {
        out.problem = Some(NO_GPS.into());
        return Some(out);
    };
    let Some(gps) = t.ifd(gps) else {
        out.problem = Some(BAD_GPS.into());
        return Some(out);
    };
    if out.taken.is_none() {
        out.taken = gps_moment(&t, &gps);
    }
    // Its height: below sea level when the reference is 1.
    if let Some(h) = find(&gps, 0x0006).and_then(|e| t.rational(e, 0)) {
        let below = find(&gps, 0x0005).and_then(|e| t.byte(e)) == Some(1);
        out.altitude = Some(if below { h.negated() } else { h }.value());
    }
    // The direction it looked, from the true or the magnetic north.
    if let Some(d) = find(&gps, 0x0011).and_then(|e| t.rational(e, 0)) {
        out.direction = Some(d.value());
        out.magnetic = find(&gps, 0x0010).and_then(|e| t.ascii(e)).as_deref() == Some("M");
    }
    let refs = (
        find(&gps, 0x0001).and_then(|e| t.ascii(e)),
        find(&gps, 0x0003).and_then(|e| t.ascii(e)),
    );
    let values = (find(&gps, 0x0002), find(&gps, 0x0004));
    match (refs, values) {
        ((None, None), (None, None)) => out.problem = Some(NO_GPS.into()),
        ((Some(ns), Some(ew)), (Some(lat), Some(lon))) => {
            let (Some(lat), Some(lon)) = (degrees(&t, lat), degrees(&t, lon)) else {
                out.problem = Some(BAD_GPS.into());
                return Some(out);
            };
            let south = match ns.as_str() {
                "N" => false,
                "S" => true,
                _ => {
                    out.problem = Some(BAD_GPS.into());
                    return Some(out);
                }
            };
            let west = match ew.as_str() {
                "E" => false,
                "W" => true,
                _ => {
                    out.problem = Some(BAD_GPS.into());
                    return Some(out);
                }
            };
            if lat.size_cmp(90) == std::cmp::Ordering::Greater
                || lon.size_cmp(180) == std::cmp::Ordering::Greater
            {
                out.problem = Some(FAR_GPS.into());
                return Some(out);
            }
            out.latitude = Some(if south { lat.negated() } else { lat }.value());
            out.longitude = Some(if west { lon.negated() } else { lon }.value());
        }
        _ => out.problem = Some(BAD_GPS.into()),
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn what_is_not_a_photo_says_why() {
        assert_eq!(read(b"GIF89a").problem.as_deref(), Some(NO_JPEG));
        assert_eq!(
            read(&[0xFF, 0xD8, 0xFF, 0xD9]).problem.as_deref(),
            Some(NO_EXIF)
        );
        assert_eq!(
            read(&[0xFF, 0xD8, 0xFF, 0xE1, 0x00]).problem.as_deref(),
            Some(NO_EXIF)
        );
        assert!(needs_more(&[0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0, 0]));
        assert!(!needs_more(&[0xFF, 0xD8, 0xFF, 0xD9]));
        assert_eq!(
            moment("2024:05:17 10:32:05").as_deref(),
            Some("2024-05-17T10:32:05")
        );
        assert_eq!(moment("0000:00:00 00:00:00"), None);
        assert!(offset_ok("+03:00") && offset_ok("-14:00") && !offset_ok("+14:30"));
        assert_eq!(
            camera(Some("Canon".into()), Some("Canon EOS R5".into())).as_deref(),
            Some("Canon EOS R5")
        );
        assert_eq!(
            camera(Some("Apple".into()), Some("iPhone 15".into())).as_deref(),
            Some("Apple iPhone 15")
        );
    }
}
