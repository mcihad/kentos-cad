//! Instrument coordinate files (docs/adr/0169 §4): points sent to a total
//! station as its own coordinate records — Leica GSI-16 and GSI-8 (WI 11,
//! 81, 82, 83, 71), Topcon GTS-7 points (Topcon Link's Appendix C), Trimble
//! JobXML 5.3 (FieldBook PointRecords), Nikon RAW V2.00 (UP records) — or
//! CSV. Values are written in millimetres by the display rule
//! (docs/adr/0149). A point the format cannot carry is not written and is
//! said with its first problem, never cut to fit. The independent reference
//! is `scripts/fixtures/field_write_cases.py` (fixtures/field/v1/write.json).

use kentos_contracts::{FieldPoint, FieldSkip, FieldWrite, FieldWriteFormat, FieldWriteOptions};
use kentos_geometry_core::display::fixed;

/// What is said of a point not written; `{index}`, `{name}`, `{chars}`,
/// `{format}`, `{length}`, `{limit}`, `{what}`, `{value}` and `{job}` are
/// filled in.
pub const EMPTY: &str = "{index}. noktanın adı yok; yazılmadı.";
pub const CONTROL: &str = "“{name}” adında denetim karakteri var; yazılmadı.";
pub const BLANK: &str =
    "“{name}” adının başında ya da sonunda boşluk var; okunurken atılır; yazılmadı.";
pub const CHARS: &str = "“{name}” adındaki {chars} {format} dosyasında taşınamaz; yazılmadı.";
pub const LONG: &str =
    "“{name}” adı {length} karakter; {format} en çok {limit} karakter alır; yazılmadı.";
pub const ZEROS: &str = "“{name}” adı sıfırla başlıyor; GSI baştaki sıfırları atar; yazılmadı.";
pub const CODE_CONTROL: &str = "“{name}” noktasının kodunda denetim karakteri var; yazılmadı.";
pub const CODE_BLANK: &str =
    "“{name}” noktasının kodunun başında ya da sonunda boşluk var; okunurken atılır; yazılmadı.";
pub const CODE_CHARS: &str =
    "“{name}” noktasının kodundaki {chars} {format} dosyasında taşınamaz; yazılmadı.";
pub const CODE_LONG: &str =
    "“{name}” noktasının kodu {length} karakter; {format} en çok {limit} karakter alır; yazılmadı.";
pub const CODE_ZEROS: &str =
    "“{name}” noktasının kodu sıfırla başlıyor; GSI baştaki sıfırları atar; yazılmadı.";
pub const VALUE: &str = "“{name}” noktasının {what} değeri sayı değil; yazılmadı.";
pub const RANGE: &str =
    "“{name}” noktasının {what} değeri {value} m; {format} sözcüğüne sığmıyor; yazılmadı.";
pub const JOB: &str = "İş adı “{job}” {format} dosyasında taşınamaz; dosya yazılmadı.";

/// The format as a sentence names it.
pub fn format_name(format: FieldWriteFormat) -> &'static str {
    match format {
        FieldWriteFormat::Gsi16 => "Leica GSI-16",
        FieldWriteFormat::Gsi8 => "Leica GSI-8",
        FieldWriteFormat::Gts7 => "Topcon GTS-7",
        FieldWriteFormat::Jobxml => "Trimble JobXML",
        FieldWriteFormat::Nikon => "Nikon RAW",
        FieldWriteFormat::Csv => "CSV",
    }
}

/// A GSI word's data characters; none for the other formats.
fn gsi_width(format: FieldWriteFormat) -> Option<usize> {
    match format {
        FieldWriteFormat::Gsi16 => Some(16),
        FieldWriteFormat::Gsi8 => Some(8),
        _ => None,
    }
}

fn is_control(c: char) -> bool {
    (c as u32) < 0x20 || c as u32 == 0x7F
}

/// Whether the format does not take `c` in a name or a code.
fn foreign(c: char, format: FieldWriteFormat) -> bool {
    let code = c as u32;
    match format {
        FieldWriteFormat::Gsi16 | FieldWriteFormat::Gsi8 => !(0x21..=0x7E).contains(&code),
        FieldWriteFormat::Gts7 | FieldWriteFormat::Nikon => {
            !(0x20..=0x7E).contains(&code) || c == ','
        }
        FieldWriteFormat::Csv => c == ',',
        FieldWriteFormat::Jobxml => false,
    }
}

/// The characters a problem names, in their first order: “Ş”, “İ”, boşluk.
fn chars_text(s: &str, format: FieldWriteFormat) -> Option<String> {
    let mut seen: Vec<char> = Vec::new();
    for c in s.chars() {
        if foreign(c, format) && !seen.contains(&c) {
            seen.push(c);
        }
    }
    (!seen.is_empty()).then(|| {
        seen.iter()
            .map(|c| {
                if *c == ' ' {
                    "boşluk".to_owned()
                } else {
                    format!("“{c}”")
                }
            })
            .collect::<Vec<_>>()
            .join(", ")
    })
}

/// A name's (or a code's) first problem: its text and what fills it.
fn text_problem(
    s: &str,
    format: FieldWriteFormat,
    code: bool,
) -> Option<(&'static str, Vec<(&'static str, String)>)> {
    let pick = |name: &'static str, of_code: &'static str| if code { of_code } else { name };
    if s.chars().any(is_control) {
        return Some((pick(CONTROL, CODE_CONTROL), Vec::new()));
    }
    let first = s.chars().next();
    let last = s.chars().next_back();
    if first.is_some_and(char::is_whitespace) || last.is_some_and(char::is_whitespace) {
        return Some((pick(BLANK, CODE_BLANK), Vec::new()));
    }
    if let Some(chars) = chars_text(s, format) {
        return Some((pick(CHARS, CODE_CHARS), vec![("{chars}", chars)]));
    }
    if let Some(limit) = gsi_width(format) {
        let length = s.chars().count();
        if length > limit {
            return Some((
                pick(LONG, CODE_LONG),
                vec![
                    ("{length}", length.to_string()),
                    ("{limit}", limit.to_string()),
                ],
            ));
        }
        if length > 1 && s.starts_with('0') {
            return Some((pick(ZEROS, CODE_ZEROS), Vec::new()));
        }
    }
    None
}

/// A value in whole millimetres: its sign and digits (none but `0` for zero).
fn millimetres(v: f64) -> (char, String) {
    let t = fixed(v, 3);
    let sign = if t.starts_with('-') { '-' } else { '+' };
    let digits: String = t
        .trim_start_matches('-')
        .chars()
        .filter(|c| *c != '.')
        .collect();
    let digits = digits.trim_start_matches('0');
    (
        sign,
        if digits.is_empty() {
            "0".to_owned()
        } else {
            digits.to_owned()
        },
    )
}

fn fill(text: &str, pairs: &[(&str, String)]) -> String {
    pairs
        .iter()
        .fold(text.to_owned(), |t, (k, v)| t.replace(k, v))
}

/// Why `p` (the `index`th) is not written, if it is not.
fn problem(index: usize, p: &FieldPoint, format: FieldWriteFormat) -> Option<String> {
    let name = &p.name;
    if name.is_empty() {
        return Some(EMPTY.replace("{index}", &index.to_string()));
    }
    let code = p.code.as_deref().unwrap_or("");
    let found = text_problem(name, format, false).or_else(|| {
        (!code.is_empty())
            .then(|| text_problem(code, format, true))
            .flatten()
    });
    if let Some((text, mut pairs)) = found {
        pairs.push(("{name}", name.clone()));
        pairs.push(("{format}", format_name(format).to_owned()));
        return Some(fill(text, &pairs));
    }
    let values = [
        ("doğu", Some(p.east)),
        ("kuzey", Some(p.north)),
        ("kot", p.elevation),
    ];
    for (what, v) in values {
        let Some(v) = v else { continue };
        if !v.is_finite() {
            return Some(fill(
                VALUE,
                &[("{name}", name.clone()), ("{what}", what.to_owned())],
            ));
        }
        if let Some(width) = gsi_width(format)
            && millimetres(v).1.len() > width
        {
            return Some(fill(
                RANGE,
                &[
                    ("{name}", name.clone()),
                    ("{what}", what.to_owned()),
                    ("{value}", fixed(v, 3)),
                    ("{format}", format_name(format).to_owned()),
                ],
            ));
        }
    }
    None
}

/// Text or an attribute in XML.
fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// A GSI word: its index, information, sign and data right justified and
/// zero filled.
fn gsi_word(wi: &str, info: &str, sign: char, data: &str, width: usize) -> String {
    format!("{wi}{info}{sign}{data:0>width$}")
}

/// Writes `points` as `options` say.
pub fn write(points: &[FieldPoint], options: &FieldWriteOptions) -> FieldWrite {
    let format = options.format;
    let job = &options.job;
    let job_bad = match format {
        FieldWriteFormat::Nikon => job.chars().any(|c| !(0x20..=0x7E).contains(&(c as u32))),
        FieldWriteFormat::Jobxml => job.chars().any(is_control),
        _ => false,
    };
    if job_bad {
        return FieldWrite {
            text: String::new(),
            written: 0,
            skipped: Vec::new(),
            error: Some(fill(
                JOB,
                &[
                    ("{job}", job.clone()),
                    ("{format}", format_name(format).to_owned()),
                ],
            )),
        };
    }
    let stamp = escape(&options.stamp);
    let mut lines: Vec<String> = Vec::new();
    match format {
        FieldWriteFormat::Jobxml => {
            lines.push(r#"<?xml version="1.0" encoding="UTF-8"?>"#.to_owned());
            lines.push(format!(
                r#"<JOBFile jobName="{}" version="5.3" product="KentOS CAD" TimeStamp="{stamp}">"#,
                escape(job)
            ));
            lines.push("  <FieldBook>".to_owned());
        }
        FieldWriteFormat::Nikon => {
            lines.push("CO,Nikon RAW data format V2.00".to_owned());
            lines.push(format!("CO,{job}"));
            lines.push("CO,Dist Units: Metres".to_owned());
        }
        _ => {}
    }
    let mut skipped = Vec::new();
    let mut written: u32 = 0;
    for (i, p) in points.iter().enumerate() {
        let index = i + 1;
        if let Some(said) = problem(index, p, format) {
            skipped.push(FieldSkip {
                index: u32::try_from(index).unwrap_or(u32::MAX),
                name: p.name.clone(),
                problem: said,
            });
            continue;
        }
        written = written.saturating_add(1);
        let (name, code) = (&p.name, p.code.as_deref().unwrap_or(""));
        let (e, n) = (fixed(p.east, 3), fixed(p.north, 3));
        let z = p.elevation.map(|z| fixed(z, 3)).unwrap_or_default();
        match format {
            FieldWriteFormat::Gsi16 | FieldWriteFormat::Gsi8 => {
                let width = gsi_width(format).unwrap_or(16);
                let block = format!("{:04}", written % 10_000);
                let mut words = vec![gsi_word("11", &block, '+', name, width)];
                for (wi, v) in [
                    ("81", Some(p.east)),
                    ("82", Some(p.north)),
                    ("83", p.elevation),
                ] {
                    if let Some(v) = v {
                        let (sign, digits) = millimetres(v);
                        words.push(gsi_word(wi, "..10", sign, &digits, width));
                    }
                }
                if !code.is_empty() {
                    words.push(gsi_word("71", "....", '+', code, width));
                }
                let star = if format == FieldWriteFormat::Gsi16 {
                    "*"
                } else {
                    ""
                };
                let body: String = words.iter().map(|w| format!("{w} ")).collect();
                lines.push(format!("{star}{body}"));
            }
            FieldWriteFormat::Gts7 | FieldWriteFormat::Csv => {
                lines.push(format!("{name},{e},{n},{z},{code}"));
            }
            FieldWriteFormat::Nikon => lines.push(format!("UP,{name},,{n},{e},{z},{code}")),
            FieldWriteFormat::Jobxml => {
                lines.push(format!(
                    r#"    <PointRecord ID="{written:08X}" TimeStamp="{stamp}">"#
                ));
                lines.push(format!("      <Name>{}</Name>", escape(name)));
                if !code.is_empty() {
                    lines.push(format!("      <Code>{}</Code>", escape(code)));
                }
                lines.push("      <Method>KeyedIn</Method>".to_owned());
                lines.push("      <Classification>Normal</Classification>".to_owned());
                lines.push("      <Deleted>false</Deleted>".to_owned());
                lines.push("      <Grid>".to_owned());
                lines.push(format!("        <North>{n}</North>"));
                lines.push(format!("        <East>{e}</East>"));
                if p.elevation.is_some() {
                    lines.push(format!("        <Elevation>{z}</Elevation>"));
                }
                lines.push("      </Grid>".to_owned());
                lines.push("    </PointRecord>".to_owned());
            }
        }
    }
    if format == FieldWriteFormat::Jobxml {
        lines.push("  </FieldBook>".to_owned());
        lines.push("</JOBFile>".to_owned());
    }
    FieldWrite {
        text: lines.iter().map(|l| format!("{l}\r\n")).collect(),
        written,
        skipped,
        error: None,
    }
}
