//! PDF/A-2 denetimi (docs/adr/8037, M37): the ISO 19005-2 rules a sheet file can break, read back from its bytes.
//!
//! The 2026 digital project regulation asks a drawing as a PDF/A file (RG 5.8.2026/33331 Md. 4/2). This is KentOS's own
//! reading of the clauses its sheet writer touches, not a certification: it says what it read and found, by clause.
//!
//! - 6.1.2 header: `%PDF-` and a comment of at least four bytes over 127 on the next line; 6.1.3 trailer: the file's
//!   id, no encryption; nothing but an end of line after the last `%%EOF`.
//! - 6.6.2.1 the catalog's XMP metadata stream, uncompressed, with the PDF/A identification (part 2; level A, B or U);
//!   6.6.2.3.1 each information dictionary entry equal to its XMP property.
//! - 6.2.3 a `GTS_PDFA1` output intent whose ICC profile is a display or printer profile with as many components as
//!   its `N`; 6.2.4.3 a device CMYK picture only with a CMYK output intent.
//! - 6.2.8 no image `Interpolate` true, `Alternates` or `OPI`; 6.2.5 no `TR`, a `TR2` only `Default`, no `HTO`.
//! - 6.2.11.4.1 every font's program embedded; 6.2.11.3.2 a `CIDFontType2`'s `CIDToGIDMap`.
//! - 6.6.1 / 6.8 no JavaScript, no catalog actions, embedded files only declared PDF/A ones are not checked (none
//!   expected); 6.9 every optional content configuration named, none with `AS`.
//!
//! Content stream operators (a device colour set by `k`, `K`) are not read: the sheet writer paints RGB only.

use std::collections::BTreeSet;

use super::content::decode_text_string;
use super::file::PdfFile;
use super::object::{Dict, Obj};

/// A broken rule: its clause of ISO 19005-2 and what was found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PdfaFinding {
    pub clause: &'static str,
    pub message: String,
}

/// What the file says it is.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PdfaReport {
    /// `pdfaid:part` and `pdfaid:conformance` from the XMP metadata; none when it does not say.
    pub part: Option<u8>,
    pub conformance: Option<String>,
    pub findings: Vec<PdfaFinding>,
}

impl PdfaReport {
    /// A PDF/A-2 file by everything this reads.
    pub fn conforms(&self) -> bool {
        self.part == Some(2) && self.findings.is_empty()
    }
}

fn found(out: &mut Vec<PdfaFinding>, clause: &'static str, message: impl Into<String>) {
    out.push(PdfaFinding {
        clause,
        message: message.into(),
    });
}

/// An XMP property's simple value (`<ns:Name>v</ns:Name>` or `ns:Name="v"`), or a language alternative's or a
/// sequence's items; entities undone.
fn xmp_values(xmp: &str, property: &str) -> Option<Vec<String>> {
    let open = format!("<{property}>");
    let close = format!("</{property}>");
    if let Some(start) = xmp.find(&open) {
        let body = &xmp[start + open.len()..];
        let body = &body[..body.find(&close)?];
        if body.contains("<rdf:li") {
            let mut items = Vec::new();
            let mut rest = body;
            while let Some(at) = rest.find("<rdf:li") {
                let after = &rest[at..];
                let gt = after.find('>')?;
                let end = after.find("</rdf:li>")?;
                items.push(unescape(&after[gt + 1..end]));
                rest = &after[end + 9..];
            }
            return Some(items);
        }
        return Some(vec![unescape(body.trim())]);
    }
    let attr = format!("{property}=\"");
    let start = xmp.find(&attr)? + attr.len();
    let end = xmp[start..].find('"')?;
    Some(vec![unescape(&xmp[start..start + end])])
}

fn unescape(s: &str) -> String {
    s.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
}

/// A date as (year, month, day, hour, minute, second, offset in minutes): a PDF date (`D:20261109000000Z`) or an XMP
/// one (`2026-11-09T00:00:00Z`); the missing parts are the first of their kind and an unsaid offset is UTC.
fn date_parts(s: &str) -> Option<[i64; 7]> {
    let s = s.trim();
    let (digits, tail): (String, &str) = match s.strip_prefix("D:") {
        Some(d) => {
            let n = d.find(|c: char| !c.is_ascii_digit()).unwrap_or(d.len());
            (d[..n].to_owned(), &d[n..])
        }
        None => {
            let (date, time) = s.split_once('T').unwrap_or((s, ""));
            let cut = time.find(['Z', '+', '-']).unwrap_or(time.len());
            // A fraction of a second is not compared.
            let clock = time[..cut].split('.').next().unwrap_or("");
            let d: String = date
                .chars()
                .chain(clock.chars())
                .filter(char::is_ascii_digit)
                .collect();
            (d, &time[cut..])
        }
    };
    let field = |from: usize, len: usize, default: i64| -> Option<i64> {
        match digits.get(from..from + len) {
            Some(t) => t.parse().ok(),
            None if digits.len() <= from => Some(default),
            None => None,
        }
    };
    let offset = match tail.chars().next() {
        None | Some('Z') => 0,
        Some(sign @ ('+' | '-')) => {
            let t: String = tail[1..].chars().filter(char::is_ascii_digit).collect();
            let h: i64 = t.get(..2).and_then(|x| x.parse().ok()).unwrap_or(0);
            let m: i64 = t.get(2..4).and_then(|x| x.parse().ok()).unwrap_or(0);
            let v = h * 60 + m;
            if sign == '-' { -v } else { v }
        }
        Some(_) => return None,
    };
    Some([
        field(0, 4, 0)?,
        field(4, 2, 1)?,
        field(6, 2, 1)?,
        field(8, 2, 0)?,
        field(10, 2, 0)?,
        field(12, 2, 0)?,
        offset,
    ])
}

/// A date as seconds of UTC from year zero (near enough to compare two dates).
fn instant(p: [i64; 7]) -> i64 {
    let days = p[0] * 372 + (p[1] - 1) * 31 + (p[2] - 1);
    ((days * 24 + p[3]) * 60 + p[4] - p[6]) * 60 + p[5]
}

/// The ICC profile's colour space as a number of components (`GRAY` 1, `RGB ` 3, `CMYK` 4) and its device class.
fn icc_header(icc: &[u8]) -> Result<(u32, [u8; 4], u8), String> {
    if icc.len() < 128 {
        return Err(format!("profil {} bayt; başlığı 128 bayttır", icc.len()));
    }
    if &icc[36..40] != b"acsp" {
        return Err("profilin imzası (acsp) yok".into());
    }
    let size = u32::from_be_bytes([icc[0], icc[1], icc[2], icc[3]]) as usize;
    if size != icc.len() {
        return Err(format!(
            "profil başlığı {size} bayt diyor, akışta {} bayt var",
            icc.len()
        ));
    }
    let n = match &icc[16..20] {
        b"GRAY" => 1,
        b"RGB " => 3,
        b"CMYK" => 4,
        other => {
            return Err(format!(
                "profilin renk uzayı {} tanınmıyor",
                String::from_utf8_lossy(other)
            ));
        }
    };
    Ok((n, [icc[12], icc[13], icc[14], icc[15]], icc[8]))
}

/// The ISO 19005-2 rules [`PdfaReport`] reads, read from `bytes`.
pub fn pdfa2_report(bytes: &[u8]) -> Result<PdfaReport, String> {
    let file = PdfFile::open(bytes)?;
    let mut f = Vec::new();
    let mut report = PdfaReport::default();

    // 6.1.2: the header and its binary comment.
    let second = bytes
        .iter()
        .position(|b| *b == b'\n' || *b == b'\r')
        .map(|i| &bytes[i..])
        .map(|r| {
            r.iter()
                .skip_while(|b| **b == b'\n' || **b == b'\r')
                .copied()
                .take(5)
                .collect::<Vec<_>>()
        });
    if !bytes.starts_with(b"%PDF-1.") && !bytes.starts_with(b"%PDF-2.") {
        found(&mut f, "6.1.2", "dosya %PDF- başlığıyla başlamıyor");
    }
    if !second.is_some_and(|c| c.len() == 5 && c[0] == b'%' && c[1..].iter().all(|b| *b > 127)) {
        found(
            &mut f,
            "6.1.2",
            "başlıktan sonraki satırda 127'den büyük dört baytlık yorum yok",
        );
    }
    // 6.1.3: after the last %%EOF only an end of line.
    match bytes.windows(5).rposition(|w| w == b"%%EOF") {
        Some(at)
            if bytes[at + 5..].iter().all(|b| *b == b'\n' || *b == b'\r')
                && bytes.len() - at - 5 <= 2 => {}
        _ => found(
            &mut f,
            "6.1.3",
            "son %%EOF'tan sonra satır sonundan başka veri var",
        ),
    }
    if !file
        .field(&file.trailer, "ID")
        .array()
        .is_some_and(|a| a.len() == 2)
    {
        found(&mut f, "6.1.3", "sonlandırıcıda dosya kimliği (ID) yok");
    }
    if file.trailer.contains_key("Encrypt") {
        found(&mut f, "6.1.3", "dosya şifreli");
    }

    let root = file.field(&file.trailer, "Root");
    let root = root.dict().cloned().ok_or("PDF'in kataloğu okunamadı")?;
    let info = file
        .field(&file.trailer, "Info")
        .dict()
        .cloned()
        .unwrap_or_default();

    // 6.6.2.1: XMP metadata.
    let metadata = file.resolve(root.get("Metadata").unwrap_or(&Obj::Null));
    let xmp = match &metadata {
        Obj::Stream(d, raw) => {
            let d = file.resolved_dict(d);
            if d.get("Filter").is_some_and(|v| !matches!(v, Obj::Null)) {
                found(&mut f, "6.6.2.1", "XMP üst veri akışı sıkıştırılmış");
            }
            if d.get("Type").and_then(Obj::name) != Some("Metadata")
                || d.get("Subtype").and_then(Obj::name) != Some("XML")
            {
                found(
                    &mut f,
                    "6.6.2.1",
                    "üst veri akışının türü /Metadata /XML değil",
                );
            }
            Some(String::from_utf8_lossy(raw).into_owned())
        }
        _ => {
            found(&mut f, "6.6.2.1", "katalogda XMP üst verisi (Metadata) yok");
            None
        }
    };
    if let Some(x) = &xmp {
        report.part = xmp_values(x, "pdfaid:part").and_then(|v| v.first()?.trim().parse().ok());
        report.conformance = xmp_values(x, "pdfaid:conformance").and_then(|v| v.first().cloned());
        match report.part {
            Some(2) => {}
            Some(p) => found(
                &mut f,
                "6.6.4",
                format!("XMP PDF/A-{p} diyor; bu denetim PDF/A-2'nin"),
            ),
            None => found(&mut f, "6.6.4", "XMP'de PDF/A kimliği (pdfaid:part) yok"),
        }
        if !matches!(report.conformance.as_deref(), Some("A" | "B" | "U")) {
            found(
                &mut f,
                "6.6.4",
                "XMP'de uygunluk düzeyi (pdfaid:conformance) A, B ya da U değil",
            );
        }
        // 6.6.2.3.1: each information entry equal to its XMP property.
        let text = |key: &str| match file.field(&info, key) {
            Obj::Str(s) => Some(decode_text_string(&s)),
            _ => None,
        };
        for (key, property) in [
            ("Title", "dc:title"),
            ("Author", "dc:creator"),
            ("Subject", "dc:description"),
            ("Keywords", "pdf:Keywords"),
            ("Creator", "xmp:CreatorTool"),
            ("Producer", "pdf:Producer"),
        ] {
            let Some(want) = text(key) else { continue };
            let have = xmp_values(x, property);
            let ok = match (key, &have) {
                ("Author", Some(v)) => v.len() == 1 && v[0] == want,
                (_, Some(v)) => v.first() == Some(&want),
                _ => false,
            };
            if !ok {
                found(
                    &mut f,
                    "6.6.2.3.1",
                    format!(
                        "bilgi sözlüğünün {key} değeri ({want}) XMP'nin {property} değerinden farklı"
                    ),
                );
            }
        }
        for (key, property) in [
            ("CreationDate", "xmp:CreateDate"),
            ("ModDate", "xmp:ModifyDate"),
        ] {
            let Some(want) = text(key) else { continue };
            let have = xmp_values(x, property).and_then(|v| v.first().cloned());
            let same = match (date_parts(&want), have.as_deref().and_then(date_parts)) {
                (Some(a), Some(b)) => instant(a) == instant(b),
                _ => false,
            };
            if !same {
                found(
                    &mut f,
                    "6.6.2.3.1",
                    format!(
                        "bilgi sözlüğünün {key} tarihi ({want}) XMP'nin {property} değerinden farklı"
                    ),
                );
            }
        }
    }

    // 6.2.3: the output intent.
    let mut output_n = None;
    let intents = file.field(&root, "OutputIntents");
    for oi in intents.array().unwrap_or_default() {
        let oi = file.resolve(oi);
        let Some(d) = oi.dict() else { continue };
        if file.field(d, "S").name() != Some("GTS_PDFA1") {
            continue;
        }
        let profile = file.field(d, "DestOutputProfile");
        let Obj::Stream(pd, _) = &profile else {
            found(
                &mut f,
                "6.2.3",
                "PDF/A çıktı niyetinde renk profili (DestOutputProfile) yok",
            );
            continue;
        };
        let pd = file.resolved_dict(pd);
        match file.stream_data(&profile) {
            Ok(Some(icc)) => match icc_header(&icc) {
                Ok((n, class, version)) => {
                    if pd.get("N").and_then(Obj::num) != Some(f64::from(n)) {
                        found(
                            &mut f,
                            "6.2.3",
                            format!("profilin N değeri renk uzayının {n} bileşenine uymuyor"),
                        );
                    }
                    if &class != b"mntr" && &class != b"prtr" {
                        found(
                            &mut f,
                            "6.2.3",
                            format!(
                                "profilin aygıt sınıfı {}; mntr ya da prtr olmalı",
                                String::from_utf8_lossy(&class)
                            ),
                        );
                    }
                    if version > 4 {
                        found(
                            &mut f,
                            "6.2.3",
                            format!("profilin sürümü {version}; en çok 4 olabilir"),
                        );
                    }
                    output_n = Some(n);
                }
                Err(e) => found(&mut f, "6.2.3", e),
            },
            _ => found(&mut f, "6.2.3", "çıktı niyetinin renk profili okunamadı"),
        }
    }
    if output_n.is_none() && !f.iter().any(|x| x.clause == "6.2.3") {
        found(
            &mut f,
            "6.2.3",
            "katalogda PDF/A çıktı niyeti (GTS_PDFA1) yok",
        );
    }

    // 6.6.1 / 6.8: scripts, actions.
    if root.contains_key("AA") || root.contains_key("OpenAction") {
        found(
            &mut f,
            "6.6.1",
            "katalogda açılış eylemi (OpenAction, AA) var",
        );
    }
    if let Some(names) = file.field(&root, "Names").dict() {
        if names.contains_key("JavaScript") {
            found(&mut f, "6.6.1", "katalogda JavaScript var");
        }
        if names.contains_key("EmbeddedFiles") {
            found(
                &mut f,
                "6.8",
                "dosyaya gömülü dosya var; bu denetim gömülü dosyaların PDF/A olduğunu okumuyor",
            );
        }
    }

    // 6.9: optional content configurations.
    if let Some(oc) = file.field(&root, "OCProperties").dict() {
        let mut configs = Vec::new();
        if let Obj::Dict(d) = file.field(oc, "D") {
            configs.push(d);
        }
        for c in file.field(oc, "Configs").array().unwrap_or_default() {
            if let Obj::Dict(d) = file.resolve(c) {
                configs.push(d);
            }
        }
        let mut names = BTreeSet::new();
        for c in &configs {
            match file.field(c, "Name") {
                Obj::Str(n) if names.insert(n.clone()) => {}
                Obj::Str(_) => found(&mut f, "6.9", "iki katman yapılandırmasının adı aynı"),
                _ => found(&mut f, "6.9", "katman yapılandırmasının adı (Name) yok"),
            }
            if c.contains_key("AS") {
                found(&mut f, "6.9", "katman yapılandırmasında AS var");
            }
        }
    }

    // Each page's resources, and the form objects' inside them.
    let pages = super::pages(&file).map_err(|e| e.to_string())?;
    let mut seen = BTreeSet::new();
    let mut fonts_seen = BTreeSet::new();
    let mut stack: Vec<(Dict, usize)> = pages.iter().map(|p| (p.resources.clone(), 0)).collect();
    for p in &pages {
        if p.dict.contains_key("AA") {
            found(&mut f, "6.6.1", "sayfada eylem (AA) var");
        }
    }
    while let Some((res, depth)) = stack.pop() {
        if depth > 16 {
            continue;
        }
        if let Some(fonts) = file.field(&res, "Font").dict() {
            for (name, r) in fonts {
                if let Obj::Ref(n, _) = r
                    && !fonts_seen.insert(*n)
                {
                    continue;
                }
                let Some(font) = file.resolve(r).dict().cloned() else {
                    continue;
                };
                check_font(&file, &font, name, &mut f);
            }
        }
        if let Some(states) = file.field(&res, "ExtGState").dict() {
            for (name, r) in states {
                let Some(g) = file.resolve(r).dict().cloned() else {
                    continue;
                };
                if g.contains_key("TR") {
                    found(&mut f, "6.2.5", format!("{name} grafik durumunda TR var"));
                }
                if g.get("TR2")
                    .is_some_and(|v| file.resolve(v).name() != Some("Default"))
                {
                    found(
                        &mut f,
                        "6.2.5",
                        format!("{name} grafik durumunda TR2 Default değil"),
                    );
                }
                if g.contains_key("HTO") {
                    found(&mut f, "6.2.5", format!("{name} grafik durumunda HTO var"));
                }
            }
        }
        if let Some(xobjects) = file.field(&res, "XObject").dict() {
            for (name, r) in xobjects {
                if let Obj::Ref(n, _) = r
                    && !seen.insert(*n)
                {
                    continue;
                }
                let Obj::Stream(d, _) = file.resolve(r) else {
                    continue;
                };
                let d = file.resolved_dict(&d);
                match d.get("Subtype").and_then(Obj::name) {
                    Some("Image") => {
                        if matches!(d.get("Interpolate"), Some(Obj::Bool(true))) {
                            found(&mut f, "6.2.8", format!("{name} resminde Interpolate açık"));
                        }
                        if d.contains_key("Alternates") || d.contains_key("OPI") {
                            found(
                                &mut f,
                                "6.2.8",
                                format!("{name} resminde Alternates ya da OPI var"),
                            );
                        }
                        if d.get("ColorSpace").and_then(Obj::name) == Some("DeviceCMYK")
                            && output_n != Some(4)
                        {
                            found(
                                &mut f,
                                "6.2.4.3",
                                format!("{name} resmi CMYK; çıktı niyeti CMYK değil"),
                            );
                        }
                    }
                    Some("Form") => {
                        if let Some(inner) = file.field(&d, "Resources").dict() {
                            stack.push((inner.clone(), depth + 1));
                        }
                    }
                    Some("PS") => found(&mut f, "6.2.9", format!("{name} bir PostScript nesnesi")),
                    _ => {}
                }
            }
        }
    }
    report.findings = f;
    Ok(report)
}

fn check_font(file: &PdfFile<'_>, font: &Dict, name: &str, f: &mut Vec<PdfaFinding>) {
    let subtype = file.field(font, "Subtype");
    match subtype.name() {
        Some("Type3") => {}
        Some("Type0") => {
            let descendants = file.field(font, "DescendantFonts");
            let Some(cid) = descendants
                .array()
                .and_then(|a| a.first())
                .map(|c| file.resolve(c))
            else {
                found(
                    f,
                    "6.2.11.4.1",
                    format!("{name} yazı yüzünün alt yazı yüzü yok"),
                );
                return;
            };
            let Some(cid) = cid.dict() else { return };
            if file.field(cid, "Subtype").name() == Some("CIDFontType2")
                && !cid.contains_key("CIDToGIDMap")
            {
                found(
                    f,
                    "6.2.11.3.2",
                    format!("{name} yazı yüzünde CIDToGIDMap yok"),
                );
            }
            embedded(file, cid, name, f);
        }
        _ => embedded(file, font, name, f),
    }
}

fn embedded(file: &PdfFile<'_>, font: &Dict, name: &str, f: &mut Vec<PdfaFinding>) {
    let descriptor = file.field(font, "FontDescriptor");
    let ok = descriptor.dict().is_some_and(|d| {
        ["FontFile", "FontFile2", "FontFile3"]
            .iter()
            .any(|k| d.contains_key(*k))
    });
    if !ok {
        let base = file.field(font, "BaseFont");
        found(
            f,
            "6.2.11.4.1",
            format!(
                "{name} yazı yüzü ({}) gömülü değil",
                base.name().unwrap_or("?")
            ),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xmp_values_and_dates_compare_as_their_meaning() {
        let x = "<dc:title><rdf:Alt><rdf:li xml:lang=\"x-default\">A &amp; B</rdf:li></rdf:Alt></dc:title>\
                 <pdfaid:part>2</pdfaid:part> pdf:Producer=\"KentOS\"";
        assert_eq!(xmp_values(x, "dc:title"), Some(vec!["A & B".to_owned()]));
        assert_eq!(xmp_values(x, "pdfaid:part"), Some(vec!["2".to_owned()]));
        assert_eq!(
            xmp_values(x, "pdf:Producer"),
            Some(vec!["KentOS".to_owned()])
        );
        let a = date_parts("D:20261109000000Z").unwrap();
        let b = date_parts("2026-11-09T00:00:00Z").unwrap();
        let c = date_parts("2026-11-09T03:00:00+03:00").unwrap();
        assert_eq!(instant(a), instant(b));
        assert_eq!(instant(a), instant(c));
        assert_ne!(
            instant(a),
            instant(date_parts("D:20261109010000Z").unwrap())
        );
        assert_eq!(date_parts("D:20261109"), Some([2026, 11, 9, 0, 0, 0, 0]));
    }

    /// A tool: every PDF in `KENTOS_PDFA_DIR` (a delivery folder) read by these rules (docs/adr/8037).
    #[test]
    #[ignore]
    fn pdfa_files() {
        let Some(dir) = std::env::var_os("KENTOS_PDFA_DIR") else { return };
        let mut names: Vec<_> = std::fs::read_dir(&dir).unwrap().flatten().map(|e| e.path()).collect();
        names.sort();
        for path in names.iter().filter(|p| p.extension().is_some_and(|e| e == "pdf")) {
            let bytes = std::fs::read(path).unwrap();
            let r = pdfa2_report(&bytes).unwrap();
            println!(
                "{}: PDF/A-{}{} · {} · {} bulgu",
                path.file_name().unwrap().to_string_lossy(),
                r.part.map_or("?".into(), |p| p.to_string()),
                r.conformance.as_deref().unwrap_or("?").to_lowercase(),
                if r.conforms() { "uygun" } else { "uygun değil" },
                r.findings.len()
            );
            for f in &r.findings {
                println!("  {}: {}", f.clause, f.message);
            }
        }
    }

    #[test]
    fn a_plain_pdf_is_told_what_it_lacks() {
        let pdf = b"%PDF-1.4\n1 0 obj << /Type /Catalog /Pages 2 0 R >> endobj\n\
                    2 0 obj << /Type /Pages /Kids [3 0 R] /Count 1 >> endobj\n\
                    3 0 obj << /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >> endobj\n\
                    trailer << /Root 1 0 R >>\n%%EOF\n";
        let r = pdfa2_report(pdf).unwrap();
        assert!(!r.conforms());
        let clauses: Vec<&str> = r.findings.iter().map(|x| x.clause).collect();
        assert!(clauses.contains(&"6.1.2"), "{clauses:?}");
        assert!(clauses.contains(&"6.1.3"), "{clauses:?}");
        assert!(clauses.contains(&"6.6.2.1"), "{clauses:?}");
        assert!(clauses.contains(&"6.2.3"), "{clauses:?}");
    }
}
