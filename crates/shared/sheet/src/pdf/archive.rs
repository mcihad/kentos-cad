//! PDF/A-2b (ISO 19005-2, level B; docs/adr/3037, M37): what an archival sheet file adds to the writer's document.
//!
//! The 2026 digital project regulation (RG 5.8.2026/33331, Md. 4/2 and 8/2) asks every 2-D project drawing as a PDF/A
//! file, one file a sheet, and does not say which part. Part 2 is the one that keeps a CAD sheet's layers (optional
//! content) and its transparency, both forbidden by part 1, and carries a PAdES signature (the secure electronic
//! signature of the e-PYS regulation's Md. 6).
//!
//! - **XMP metadata** (6.6): the PDF/A identification (`pdfaid:part` 2, `pdfaid:conformance` B) and the information
//!   dictionary's title, author, subject, keywords, creator, producer and dates, each equal to its entry there
//!   (6.6.2.3.1), in an uncompressed stream (6.6.2.1). The dates are the project's day at 00:00 UTC in both: the core
//!   never reads a clock.
//! - **Output intent** (6.2.3): `GTS_PDFA1` with an sRGB ICC profile made here (version 2.1, display class), so the
//!   device RGB and gray the pages paint are device independent (6.2.4.2).
//! - **No device CMYK** (6.2.4.3): a CMYK JPEG is refused with its picture named; this file has no CMYK profile.
//! - What the writer already does: every face embedded as a subset with its widths and ToUnicode map (6.2.11), the
//!   file's id (6.1.3), the binary comment after the header (6.1.2), named optional content configurations (6.9), no
//!   encryption, actions, scripts, annotations or embedded files.

/// `pdfaid:part`.
pub const PART: u8 = 2;
/// `pdfaid:conformance`.
pub const CONFORMANCE: &str = "B";
/// The output condition the profile describes.
pub const OUTPUT_CONDITION: &str = "sRGB IEC61966-2.1";

/// What the information dictionary and the XMP packet both say.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Described {
    pub title: String,
    pub author: String,
    pub subject: String,
    pub keywords: String,
    pub creator: String,
    pub producer: String,
    /// The project's day: year, month, day.
    pub date: Option<(u16, u8, u8)>,
}

/// A day of an ISO date (`2026-11-09`, a time after it ignored); none for anything else.
pub fn day_of(iso: &str) -> Option<(u16, u8, u8)> {
    let d = iso.get(..10)?;
    let (y, m, day) = (
        d.get(..4)?.parse::<u16>().ok()?,
        d.get(5..7)?.parse::<u8>().ok()?,
        d.get(8..10)?.parse::<u8>().ok()?,
    );
    (d.as_bytes().get(4) == Some(&b'-')
        && d.as_bytes().get(7) == Some(&b'-')
        && (1..=12).contains(&m)
        && (1..=31).contains(&day))
    .then_some((y, m, day))
}

fn escaped(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            // XML 1.0 has no other control characters.
            c if (c as u32) < 0x20 && !matches!(c, '\t' | '\n' | '\r') => out.push(' '),
            c => out.push(c),
        }
    }
    out
}

/// The XMP packet (UTF-8): the PDF/A identification and `d`'s entries; an empty entry is left out, as the information
/// dictionary leaves it out.
pub fn xmp(d: &Described) -> Vec<u8> {
    let mut x = String::new();
    x.push_str("<?xpacket begin=\"\u{feff}\" id=\"W5M0MpCehiHzreSzNTczkc9d\"?>\n");
    x.push_str("<x:xmpmeta xmlns:x=\"adobe:ns:meta/\">\n");
    x.push_str(" <rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">\n");
    x.push_str("  <rdf:Description rdf:about=\"\"\n");
    x.push_str("    xmlns:dc=\"http://purl.org/dc/elements/1.1/\"\n");
    x.push_str("    xmlns:xmp=\"http://ns.adobe.com/xap/1.0/\"\n");
    x.push_str("    xmlns:pdf=\"http://ns.adobe.com/pdf/1.3/\"\n");
    x.push_str("    xmlns:pdfaid=\"http://www.aiim.org/pdfa/ns/id/\">\n");
    x.push_str(&format!("   <pdfaid:part>{PART}</pdfaid:part>\n"));
    x.push_str(&format!(
        "   <pdfaid:conformance>{CONFORMANCE}</pdfaid:conformance>\n"
    ));
    x.push_str("   <dc:format>application/pdf</dc:format>\n");
    if !d.title.is_empty() {
        x.push_str(&format!(
            "   <dc:title><rdf:Alt><rdf:li xml:lang=\"x-default\">{}</rdf:li></rdf:Alt></dc:title>\n",
            escaped(&d.title)
        ));
    }
    if !d.author.is_empty() {
        x.push_str(&format!(
            "   <dc:creator><rdf:Seq><rdf:li>{}</rdf:li></rdf:Seq></dc:creator>\n",
            escaped(&d.author)
        ));
    }
    if !d.subject.is_empty() {
        x.push_str(&format!(
            "   <dc:description><rdf:Alt><rdf:li xml:lang=\"x-default\">{}</rdf:li></rdf:Alt></dc:description>\n",
            escaped(&d.subject)
        ));
    }
    if !d.keywords.is_empty() {
        x.push_str(&format!(
            "   <pdf:Keywords>{}</pdf:Keywords>\n",
            escaped(&d.keywords)
        ));
    }
    if !d.producer.is_empty() {
        x.push_str(&format!(
            "   <pdf:Producer>{}</pdf:Producer>\n",
            escaped(&d.producer)
        ));
    }
    if !d.creator.is_empty() {
        x.push_str(&format!(
            "   <xmp:CreatorTool>{}</xmp:CreatorTool>\n",
            escaped(&d.creator)
        ));
    }
    if let Some((y, m, day)) = d.date {
        let when = format!("{y:04}-{m:02}-{day:02}T00:00:00Z");
        x.push_str(&format!("   <xmp:CreateDate>{when}</xmp:CreateDate>\n"));
        x.push_str(&format!("   <xmp:ModifyDate>{when}</xmp:ModifyDate>\n"));
    }
    x.push_str("  </rdf:Description>\n");
    x.push_str(" </rdf:RDF>\n");
    x.push_str("</x:xmpmeta>\n");
    x.push_str("<?xpacket end=\"w\"?>");
    x.into_bytes()
}

/// A number as ICC's `s15Fixed16Number`.
fn s15(v: f64) -> [u8; 4] {
    (libm::round(v * 65536.0) as i32).to_be_bytes()
}

fn xyz_tag(x: f64, y: f64, z: f64) -> Vec<u8> {
    let mut t = b"XYZ \0\0\0\0".to_vec();
    for v in [x, y, z] {
        t.extend(s15(v));
    }
    t
}

/// The sRGB display profile (IEC 61966-2-1) as an ICC version 2.1 matrix/TRC profile: its description, white point
/// (D65), D50-adapted primaries and the sRGB tone curve as a 1024-point table shared by the three channels. Made here
/// so it is the same bytes on every target and carries no one's copyright.
pub fn srgb_icc() -> Vec<u8> {
    let desc = {
        let ascii = format!("{OUTPUT_CONDITION}\0");
        let mut t = b"desc\0\0\0\0".to_vec();
        t.extend((ascii.len() as u32).to_be_bytes());
        t.extend(ascii.as_bytes());
        // No Unicode or ScriptCode description.
        t.extend(0u32.to_be_bytes());
        t.extend(0u32.to_be_bytes());
        t.extend(0u16.to_be_bytes());
        t.push(0);
        t.extend([0u8; 67]);
        t
    };
    let cprt = {
        let mut t = b"text\0\0\0\0".to_vec();
        t.extend(b"No copyright, use freely\0");
        t
    };
    let trc = {
        let n = 1024u32;
        let mut t = b"curv\0\0\0\0".to_vec();
        t.extend(n.to_be_bytes());
        for i in 0..n {
            let v = f64::from(i) / f64::from(n - 1);
            let linear = if v <= 0.04045 {
                v / 12.92
            } else {
                libm::pow((v + 0.055) / 1.055, 2.4)
            };
            t.extend((libm::round(linear * 65535.0) as u16).to_be_bytes());
        }
        t
    };
    let blocks: Vec<Vec<u8>> = vec![
        desc,
        cprt,
        xyz_tag(0.9505, 1.0, 1.0891),
        xyz_tag(0.4360747, 0.2225045, 0.0139322),
        xyz_tag(0.3850649, 0.7168786, 0.0971045),
        xyz_tag(0.1430804, 0.0606169, 0.7141733),
        trc,
    ];
    // Tag, the block it reads.
    let tags: [(&[u8; 4], usize); 9] = [
        (b"desc", 0),
        (b"cprt", 1),
        (b"wtpt", 2),
        (b"rXYZ", 3),
        (b"gXYZ", 4),
        (b"bXYZ", 5),
        (b"rTRC", 6),
        (b"gTRC", 6),
        (b"bTRC", 6),
    ];
    let table = 4 + tags.len() * 12;
    let mut offsets = Vec::with_capacity(blocks.len());
    let mut at = 128 + table;
    for b in &blocks {
        offsets.push(at);
        at += b.len().div_ceil(4) * 4;
    }
    let size = at;
    let mut out = vec![0u8; 128];
    out[..4].copy_from_slice(&(size as u32).to_be_bytes());
    out[8..12].copy_from_slice(&[0x02, 0x10, 0x00, 0x00]);
    out[12..16].copy_from_slice(b"mntr");
    out[16..20].copy_from_slice(b"RGB ");
    out[20..24].copy_from_slice(b"XYZ ");
    // Made 2026-11-09 00:00:00 (a fixed day: the same bytes every time).
    for (k, v) in [2026u16, 11, 9, 0, 0, 0].iter().enumerate() {
        out[24 + 2 * k..26 + 2 * k].copy_from_slice(&v.to_be_bytes());
    }
    out[36..40].copy_from_slice(b"acsp");
    // The profile connection space's illuminant, D50.
    out[68..72].copy_from_slice(&s15(0.9642));
    out[72..76].copy_from_slice(&s15(1.0));
    out[76..80].copy_from_slice(&s15(0.8249));
    out.extend((tags.len() as u32).to_be_bytes());
    for (sig, block) in tags {
        out.extend(sig);
        out.extend((offsets[block] as u32).to_be_bytes());
        out.extend((blocks[block].len() as u32).to_be_bytes());
    }
    for b in &blocks {
        out.extend(b);
        out.resize(out.len().div_ceil(4) * 4, 0);
    }
    debug_assert_eq!(out.len(), size);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn u32_at(b: &[u8], at: usize) -> u32 {
        u32::from_be_bytes(b[at..at + 4].try_into().unwrap())
    }

    #[test]
    fn the_srgb_profile_is_a_whole_v2_display_profile() {
        let p = srgb_icc();
        assert_eq!(
            u32_at(&p, 0) as usize,
            p.len(),
            "the header's size is the profile's"
        );
        assert_eq!(&p[12..24], b"mntrRGB XYZ ");
        assert_eq!(&p[36..40], b"acsp");
        assert_eq!(p[8], 2, "version 2");
        let count = u32_at(&p, 128) as usize;
        assert_eq!(count, 9);
        let mut sigs = Vec::new();
        for k in 0..count {
            let at = 132 + 12 * k;
            let (offset, size) = (u32_at(&p, at + 4) as usize, u32_at(&p, at + 8) as usize);
            assert_eq!(offset % 4, 0, "tag data starts on four bytes");
            assert!(offset + size <= p.len());
            sigs.push(String::from_utf8(p[at..at + 4].to_vec()).unwrap());
            let kind = &p[offset..offset + 4];
            match &p[at..at + 4] {
                b"desc" => assert_eq!(kind, b"desc"),
                b"cprt" => assert_eq!(kind, b"text"),
                b"rTRC" | b"gTRC" | b"bTRC" => {
                    assert_eq!(kind, b"curv");
                    assert_eq!(u32_at(&p, offset + 8), 1024);
                    // 0 → 0, 1 → 1, and the middle grey near 21.4 %.
                    let at = |i: usize| {
                        u16::from_be_bytes([p[offset + 12 + 2 * i], p[offset + 13 + 2 * i]])
                    };
                    assert_eq!((at(0), at(1023)), (0, 65535));
                    assert!((f64::from(at(512)) / 65535.0 - 0.2147).abs() < 0.002);
                }
                _ => assert_eq!(kind, b"XYZ "),
            }
        }
        assert_eq!(
            sigs,
            [
                "desc", "cprt", "wtpt", "rXYZ", "gXYZ", "bXYZ", "rTRC", "gTRC", "bTRC"
            ]
        );
        assert_eq!(srgb_icc(), p, "the same bytes every time");
        // For a colour management library to read it (docs/adr/3037 measured it with Little CMS).
        if let Ok(out) = std::env::var("KENTOS_ICC_OUT") {
            std::fs::write(out, &p).unwrap();
        }
    }

    #[test]
    fn the_xmp_packet_says_part_2_b_and_the_information_entries() {
        let d = Described {
            title: "A-101 Zemin kat planı".into(),
            author: "Ayşe <Mimar> & Ortak".into(),
            subject: "Konut".into(),
            keywords: String::new(),
            creator: "KentOS pafta düzeni".into(),
            producer: "KentOS".into(),
            date: day_of("2026-11-09"),
        };
        let x = String::from_utf8(xmp(&d)).unwrap();
        assert!(x.starts_with("<?xpacket begin=\"\u{feff}\""));
        assert!(x.ends_with("<?xpacket end=\"w\"?>"));
        assert!(x.contains("<pdfaid:part>2</pdfaid:part>"));
        assert!(x.contains("<pdfaid:conformance>B</pdfaid:conformance>"));
        assert!(x.contains("<rdf:li xml:lang=\"x-default\">A-101 Zemin kat planı</rdf:li>"));
        assert!(x.contains("<rdf:li>Ayşe &lt;Mimar&gt; &amp; Ortak</rdf:li>"));
        assert!(x.contains("<xmp:CreateDate>2026-11-09T00:00:00Z</xmp:CreateDate>"));
        assert!(!x.contains("pdf:Keywords"), "an empty entry is left out");
        assert_eq!(day_of("2026-13-01"), None);
        assert_eq!(day_of("2026/11/09"), None);
    }
}
