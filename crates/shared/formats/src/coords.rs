//! Coordinate lists: Netcad NCN, TXT and CSV files with one point per line
//! ("P12 452345.123 4412345.678 105.20"). The reader finds the delimiter,
//! the decimal mark and a header row, suggests what each column holds and
//! lets the caller say otherwise; every line that is not a point is
//! reported with its number and the reason. Values are the float64 nearest
//! to the text, never rounded (CLAUDE.md §23).
//!
//! Axis names follow Turkish surveying: Y is to the right (east, the
//! model's x) and X is up (north, the model's y) (§5).

use std::collections::{BTreeMap, HashSet};

use kentos_contracts::{
    Bounds, CoordColumn, CoordDelimiter, CoordRead, CoordReadOptions, CoordRow, CoordWriteInput,
    DecimalMark, Entity, EntityBase, ExportReport, HeaderMode, ImportResult, LineError,
    PointEntity, TextEncoding, Vec2,
};

use crate::num::{parse_decimal, plain};
use crate::report::Report;
use crate::text;

/// Lines the detection looks at.
const SAMPLE: usize = 200;
/// Line errors returned with their text (all are counted).
const ERRORS_KEPT: usize = 200;

/// Attribute names of an imported point (as the app's own point layers use them).
pub const ATTR_NAME: &str = "Ad";
pub const ATTR_CODE: &str = "Kod";
pub const ATTR_Z: &str = "Z (m)";

/// How the report and the preview name a column.
pub fn column_label(c: CoordColumn) -> &'static str {
    match c {
        CoordColumn::Name => "Ad",
        CoordColumn::Y => "Y (sağa)",
        CoordColumn::X => "X (yukarı)",
        CoordColumn::Z => "Z (kot)",
        CoordColumn::Code => "Kod",
        CoordColumn::Skip => "Alınmaz",
    }
}

fn is_comment(line: &str) -> bool {
    let t = line.trim();
    t.is_empty() || t.starts_with('#') || t.starts_with("//")
}

fn delimiter_char(d: CoordDelimiter) -> Option<char> {
    match d {
        CoordDelimiter::Tab => Some('\t'),
        CoordDelimiter::Semicolon => Some(';'),
        CoordDelimiter::Comma => Some(','),
        CoordDelimiter::Space | CoordDelimiter::Auto => None,
    }
}

/// Fields of a line: runs of spaces for `Space`; otherwise the delimiter,
/// with CSV quoting ("P,1" and "" inside quotes), each field trimmed.
fn split(line: &str, d: CoordDelimiter) -> Vec<String> {
    let Some(sep) = delimiter_char(d) else {
        return line.split_whitespace().map(str::to_string).collect();
    };
    let mut out = Vec::new();
    let mut field = String::new();
    let mut quoted = false;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        if quoted {
            if c == '"' {
                if chars.peek() == Some(&'"') {
                    field.push('"');
                    chars.next();
                } else {
                    quoted = false;
                }
            } else {
                field.push(c);
            }
        } else if c == '"' && field.trim().is_empty() {
            field.clear();
            quoted = true;
        } else if c == sep {
            out.push(field.trim().to_string());
            field.clear();
        } else {
            field.push(c);
        }
    }
    out.push(field.trim().to_string());
    out
}

fn numeric(f: &str, comma: bool) -> bool {
    parse_decimal(f, comma).is_some()
}

fn numeric_either(f: &str) -> bool {
    numeric(f, false) || numeric(f, true)
}

/// The delimiter that gives the most lines with two or more numbers, then
/// the steadiest field count; ties go to tab, semicolon, comma, spaces.
fn detect_delimiter(sample: &[&str]) -> CoordDelimiter {
    let candidates = [
        CoordDelimiter::Tab,
        CoordDelimiter::Semicolon,
        CoordDelimiter::Comma,
        CoordDelimiter::Space,
    ];
    let mut best = (CoordDelimiter::Space, 0usize, 0usize);
    for d in candidates {
        let mut ok = 0;
        let mut counts: BTreeMap<usize, usize> = BTreeMap::new();
        for line in sample {
            let fields = split(line, d);
            if fields.iter().filter(|f| numeric_either(f)).count() >= 2 {
                ok += 1;
            }
            *counts.entry(fields.len()).or_insert(0) += 1;
        }
        let steady = counts
            .iter()
            .filter(|(n, _)| **n >= 2)
            .map(|(_, c)| *c)
            .max()
            .unwrap_or(0);
        if ok > best.1 || (ok == best.1 && steady > best.2) {
            best = (d, ok, steady);
        }
    }
    best.0
}

/// The point, unless the fields use decimal commas and nothing else.
fn detect_decimal(rows: &[Vec<String>], d: CoordDelimiter) -> DecimalMark {
    if d == CoordDelimiter::Comma {
        return DecimalMark::Point;
    }
    let (mut point, mut comma) = (0, 0);
    for f in rows.iter().flatten() {
        if f.contains('.') && numeric(f, false) {
            point += 1;
        } else if f.contains(',') && numeric(f, true) {
            comma += 1;
        }
    }
    if comma > point {
        DecimalMark::Comma
    } else {
        DecimalMark::Point
    }
}

fn looks_like_header(first: &[String], rest: &[Vec<String>], comma: bool) -> bool {
    first.len() >= 2
        && first.iter().all(|f| !numeric(f, comma))
        && rest
            .iter()
            .take(20)
            .any(|r| r.iter().filter(|f| numeric(f, comma)).count() >= 2)
}

/// Lower-case ASCII with the Turkish letters folded and marks dropped ("Sağa (m)" → "sagam").
fn fold(s: &str) -> String {
    s.chars()
        .filter_map(|c| match c {
            'ç' | 'Ç' => Some('c'),
            'ğ' | 'Ğ' => Some('g'),
            'ı' | 'İ' | 'I' => Some('i'),
            'ö' | 'Ö' => Some('o'),
            'ş' | 'Ş' => Some('s'),
            'ü' | 'Ü' => Some('u'),
            c if c.is_ascii_alphanumeric() => Some(c.to_ascii_lowercase()),
            _ => None,
        })
        .collect()
}

/// What a header name says a column holds (Turkish names first: Y is east):
/// the whole name ("Nokta Adı"), without a unit ("Kot m"), else its first
/// known word ("Sağa (Y)", "Z (m)").
fn role_of_header(name: &str) -> Option<CoordColumn> {
    let n = fold(name);
    role_of_folded(&n)
        .or_else(|| n.strip_suffix('m').and_then(role_of_folded))
        .or_else(|| {
            name.split(|c: char| !c.is_alphanumeric())
                .find_map(|w| role_of_folded(&fold(w)))
        })
}

fn role_of_folded(n: &str) -> Option<CoordColumn> {
    Some(match n {
        "y" | "saga" | "sagadeger" | "sagadegeri" | "easting" | "east" | "e" | "dogu" => {
            CoordColumn::Y
        }
        "x" | "yukari" | "yukarideger" | "yukaridegeri" | "northing" | "north" | "n" | "kuzey" => {
            CoordColumn::X
        }
        "z" | "h" | "kot" | "yukseklik" | "elevation" | "elev" | "height" => CoordColumn::Z,
        "ad" | "adi" | "nokta" | "noktaadi" | "noktano" | "noktanumarasi" | "no" | "name"
        | "point" | "pt" | "id" => CoordColumn::Name,
        "kod" | "code" | "aciklama" | "description" | "desc" | "tur" => CoordColumn::Code,
        _ => return None,
    })
}

/// The size of a column's numbers, which tells eastings from northings in Turkey:
/// TM and UTM eastings are 1e5–1e6 m, northings 3.9e6–4.7e6 m.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Magnitude {
    Easting,
    Northing,
    Other,
}

fn magnitude(values: &mut [f64]) -> Magnitude {
    if values.is_empty() {
        return Magnitude::Other;
    }
    values.sort_by(f64::total_cmp);
    let mid = values[values.len() / 2].abs();
    if (1.0e5..1.0e6).contains(&mid) {
        Magnitude::Easting
    } else if (1.0e6..1.0e7).contains(&mid) {
        Magnitude::Northing
    } else {
        Magnitude::Other
    }
}

struct Suggestion {
    columns: Vec<CoordColumn>,
    hint: Option<String>,
}

/// What each column likely holds: the header's names, else the Netcad order
/// (name, Y, X, Z) adjusted by the numbers' sizes.
fn suggest(
    header: Option<&[String]>,
    rows: &[Vec<String>],
    ncols: usize,
    comma: bool,
) -> Suggestion {
    let column_values = |c: usize| -> Vec<f64> {
        rows.iter()
            .filter_map(|r| r.get(c))
            .filter_map(|f| parse_decimal(f, comma))
            .collect()
    };
    // Numeric when most of its fields are numbers (a few bad lines must not hide a coordinate column).
    let is_numeric = |c: usize| {
        let present = rows
            .iter()
            .filter(|r| r.get(c).is_some_and(|f| !f.is_empty()))
            .count();
        present > 0 && column_values(c).len() * 2 > present
    };
    let mag = |c: usize| magnitude(&mut column_values(c));

    let mut hint = None;
    if let Some(names) = header {
        let mut cols: Vec<CoordColumn> = (0..ncols)
            .map(|c| {
                names
                    .get(c)
                    .and_then(|n| role_of_header(n))
                    .unwrap_or(CoordColumn::Skip)
            })
            .collect();
        let y = cols.iter().position(|c| *c == CoordColumn::Y);
        let x = cols.iter().position(|c| *c == CoordColumn::X);
        if let (Some(y), Some(x)) = (y, x) {
            // International files call the easting X; the numbers show it.
            if mag(y) == Magnitude::Northing && mag(x) == Magnitude::Easting {
                cols.swap(y, x);
                hint = Some("Başlıktaki X ve Y uluslararası anlamda görünüyor (X doğu, Y kuzey); sayıların büyüklüğüne göre Y (sağa) ve X (yukarı) yer değiştirerek önerildi. Önizlemeyi denetleyin.".to_string());
            }
            return Suggestion {
                columns: cols,
                hint,
            };
        }
    }

    let numeric_cols: Vec<usize> = (0..ncols).filter(|&c| is_numeric(c)).collect();
    let mut cols = vec![CoordColumn::Skip; ncols];
    let mut nums = numeric_cols.clone();
    let integer_names = rows
        .iter()
        .filter_map(|r| r.first())
        .all(|f| !f.contains(['.', ',']));
    // Netcad writes the name first, and names are often numbers: the sizes
    // of the numbers decide where the coordinates start.
    let first_is_name = if !numeric_cols.contains(&0) {
        true
    } else {
        let (m0, m1, m2) = (mag(0), mag(1), mag(2));
        if m1 != Magnitude::Other && m2 != Magnitude::Other {
            true
        } else if m0 != Magnitude::Other && m1 != Magnitude::Other {
            false
        } else if numeric_cols.len() >= 4 {
            true
        } else {
            numeric_cols.len() == 3 && integer_names
        }
    };
    if first_is_name && ncols > 1 {
        cols[0] = CoordColumn::Name;
        nums.retain(|&c| c != 0);
    }
    if nums.len() < 2 {
        return Suggestion {
            columns: cols,
            hint: Some(
                "İki sayısal sütun bulunamadı; Y (sağa) ve X (yukarı) sütunlarını elle seçin."
                    .to_string(),
            ),
        };
    }
    let (a, b) = (nums[0], nums[1]);
    if mag(a) == Magnitude::Northing && mag(b) == Magnitude::Easting {
        cols[a] = CoordColumn::X;
        cols[b] = CoordColumn::Y;
        hint = Some("İlk sayısal sütun X (yukarı) gibi büyük değerler taşıyor; sütunlar X, Y sırasında önerildi. Önizlemeyi denetleyin.".to_string());
    } else {
        cols[a] = CoordColumn::Y;
        cols[b] = CoordColumn::X;
    }
    if let Some(&c) = nums.get(2) {
        cols[c] = CoordColumn::Z;
    }
    // A text column after the coordinates is usually a point code.
    if let Some(c) =
        (1..ncols).find(|&c| cols[c] == CoordColumn::Skip && !numeric_cols.contains(&c))
    {
        cols[c] = CoordColumn::Code;
    }
    Suggestion {
        columns: cols,
        hint,
    }
}

/// Why a row is not a point.
enum RowError {
    Missing {
        role: CoordColumn,
        need: usize,
        have: usize,
    },
    Empty {
        role: CoordColumn,
    },
    NotNumber {
        role: CoordColumn,
        value: String,
    },
}

impl RowError {
    fn generic(&self) -> String {
        match self {
            RowError::Missing { role, .. } => format!("{} alanı eksik", column_label(*role)),
            RowError::Empty { role } => format!("{} değeri boş", column_label(*role)),
            RowError::NotNumber { role, .. } => {
                format!("{} değeri sayı değil", column_label(*role))
            }
        }
    }

    fn detail(&self) -> String {
        match self {
            RowError::Missing { role, need, have } => {
                format!(
                    "{have} alan var; {} için en az {need} alan gerekiyor.",
                    column_label(*role)
                )
            }
            RowError::Empty { role } => format!("{} değeri boş.", column_label(*role)),
            RowError::NotNumber { role, value } => {
                format!("{} değeri “{value}” sayı değil.", column_label(*role))
            }
        }
    }
}

struct Row {
    name: String,
    y: f64,
    x: f64,
    z: Option<(f64, String)>,
    code: Option<String>,
}

fn read_row(fields: &[String], cols: &[CoordColumn], comma: bool) -> Result<Row, RowError> {
    let find = |role: CoordColumn| cols.iter().position(|c| *c == role);
    let number = |role: CoordColumn| -> Result<Option<(f64, String)>, RowError> {
        let Some(i) = find(role) else { return Ok(None) };
        match fields.get(i).map(String::as_str) {
            None | Some("") if role == CoordColumn::Z => Ok(None),
            None => Err(RowError::Missing {
                role,
                need: i + 1,
                have: fields.len(),
            }),
            Some("") => Err(RowError::Empty { role }),
            Some(f) => parse_decimal(f, comma)
                .map(|v| {
                    Some((
                        v,
                        if comma {
                            f.replace(',', ".")
                        } else {
                            f.to_string()
                        },
                    ))
                })
                .ok_or_else(|| RowError::NotNumber {
                    role,
                    value: f.to_string(),
                }),
        }
    };
    let y = number(CoordColumn::Y)?;
    let x = number(CoordColumn::X)?;
    let z = number(CoordColumn::Z)?;
    let text = |role: CoordColumn| {
        find(role)
            .and_then(|i| fields.get(i))
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    };
    match (y, x) {
        (Some((y, _)), Some((x, _))) => Ok(Row {
            name: text(CoordColumn::Name).unwrap_or_default(),
            y,
            x,
            z,
            code: text(CoordColumn::Code),
        }),
        // Both roles exist whenever this is reached (checked before the rows are read).
        _ => Err(RowError::Missing {
            role: CoordColumn::Y,
            need: 1,
            have: fields.len(),
        }),
    }
}

fn point_entity(row: Row) -> Entity {
    let mut attrs = BTreeMap::new();
    if !row.name.is_empty() {
        attrs.insert(ATTR_NAME.to_string(), row.name.clone());
    }
    if let Some(code) = &row.code {
        attrs.insert(ATTR_CODE.to_string(), code.clone());
    }
    if let Some((_, text)) = &row.z {
        attrs.insert(ATTR_Z.to_string(), text.clone());
    }
    Entity::Point(PointEntity {
        base: EntityBase {
            id: 0,
            layer_id: String::new(),
            color: None,
            attrs,
            label: (!row.name.is_empty()).then(|| row.name.clone()),
            symbol: None,
        },
        p: Vec2 { x: row.y, y: row.x },
        z: row.z.map(|(v, _)| v),
    })
}

/// Reads a coordinate list. Never fails: an unreadable file yields no points and says why.
pub fn read(bytes: &[u8], opts: &CoordReadOptions) -> CoordRead {
    let (enc, bom) = text::sniff(bytes);
    let body = text::decode(&bytes[bom..], enc);
    let lines: Vec<(u32, &str)> = body
        .split('\n')
        .enumerate()
        .map(|(i, l)| {
            (
                u32::try_from(i + 1).unwrap_or(u32::MAX),
                l.strip_suffix('\r').unwrap_or(l),
            )
        })
        .filter(|(_, l)| !is_comment(l))
        .collect();
    let sample: Vec<&str> = lines.iter().take(SAMPLE).map(|(_, l)| *l).collect();

    let delimiter = if opts.delimiter == CoordDelimiter::Auto {
        detect_delimiter(&sample)
    } else {
        opts.delimiter
    };
    let sample_rows: Vec<Vec<String>> = sample.iter().map(|l| split(l, delimiter)).collect();
    let decimal = match opts.decimal {
        DecimalMark::Auto => detect_decimal(&sample_rows, delimiter),
        // A decimal comma cannot be told from a comma delimiter.
        DecimalMark::Comma if delimiter == CoordDelimiter::Comma => DecimalMark::Point,
        d => d,
    };
    let comma = decimal == DecimalMark::Comma;
    let header = match opts.header {
        HeaderMode::Auto => sample_rows
            .first()
            .is_some_and(|first| looks_like_header(first, &sample_rows[1..], comma)),
        HeaderMode::Yes => !sample_rows.is_empty(),
        HeaderMode::No => false,
    };
    let header_fields = if header {
        sample_rows.first().cloned().unwrap_or_default()
    } else {
        Vec::new()
    };
    let data_sample = if header {
        &sample_rows[1.min(sample_rows.len())..]
    } else {
        &sample_rows[..]
    };

    // Columns: the steadiest field count of the sample (and the header's).
    let mut counts: BTreeMap<usize, usize> = BTreeMap::new();
    for r in data_sample {
        *counts.entry(r.len()).or_insert(0) += 1;
    }
    let steady = counts
        .iter()
        .max_by_key(|(n, c)| (**c, **n))
        .map_or(0, |(n, _)| *n);
    let ncols = steady.max(header_fields.len());

    let mut hints = Vec::new();
    let columns: Vec<CoordColumn> = if opts.columns.is_empty() {
        let s = suggest(
            header.then_some(header_fields.as_slice()),
            data_sample,
            ncols,
            comma,
        );
        hints.extend(s.hint);
        s.columns
    } else {
        (0..ncols.max(opts.columns.len()))
            .map(|i| opts.columns.get(i).copied().unwrap_or(CoordColumn::Skip))
            .collect()
    };
    let has = |role: CoordColumn| columns.contains(&role);
    let usable = has(CoordColumn::Y) && has(CoordColumn::X);
    if !usable && !lines.is_empty() {
        hints.push("Sütunlardan biri Y (sağa), biri X (yukarı) olmalı; önizlemenin başlığındaki seçimlerden belirleyin.".to_string());
    }
    for role in [
        CoordColumn::Name,
        CoordColumn::Y,
        CoordColumn::X,
        CoordColumn::Z,
        CoordColumn::Code,
    ] {
        if columns.iter().filter(|c| **c == role).count() > 1 {
            hints.push(format!(
                "{} birden çok sütuna verilmiş; yalnızca ilki okunur.",
                column_label(role)
            ));
        }
    }

    let mut report = Report::default();
    report.fact("Kodlama", enc.label());
    let mut preview = Vec::new();
    let mut errors = Vec::new();
    let mut error_count = 0u32;
    let mut entities = Vec::new();
    let mut names: HashSet<String> = HashSet::new();
    let mut duplicate_names = 0u32;
    let mut points = 0u32;
    let mut bounds: Option<Bounds> = None;
    let (mut ys, mut xs) = (Vec::new(), Vec::new());
    for (k, (line, text)) in lines.iter().enumerate().skip(usize::from(header)) {
        let fields = if k < sample_rows.len() {
            sample_rows[k].clone()
        } else {
            split(text, delimiter)
        };
        let row = if usable {
            Some(read_row(&fields, &columns, comma))
        } else {
            None
        };
        let error = match &row {
            Some(Err(e)) => {
                error_count += 1;
                report.skip("Satır", &e.generic(), *line);
                if errors.len() < ERRORS_KEPT {
                    errors.push(LineError {
                        line: *line,
                        message: format!("Satır {line}: {}", e.detail()),
                    });
                }
                Some(e.detail())
            }
            _ => None,
        };
        if preview.len() < opts.preview_rows as usize {
            preview.push(CoordRow {
                line: *line,
                fields,
                error,
            });
        }
        let Some(Ok(row)) = row else { continue };
        points += 1;
        if !row.name.is_empty() && !names.insert(row.name.clone()) {
            duplicate_names += 1;
        }
        if ys.len() < SAMPLE {
            ys.push(row.y);
            xs.push(row.x);
        }
        let b = bounds.get_or_insert(Bounds {
            min_x: row.y,
            min_y: row.x,
            max_x: row.y,
            max_y: row.x,
        });
        b.min_x = b.min_x.min(row.y);
        b.max_x = b.max_x.max(row.y);
        b.min_y = b.min_y.min(row.x);
        b.max_y = b.max_y.max(row.x);
        if opts.entities {
            report.count("point");
            entities.push(point_entity(row));
        }
    }

    if usable && points > 0 {
        let (my, mx) = (magnitude(&mut ys), magnitude(&mut xs));
        if my == Magnitude::Northing && mx == Magnitude::Easting {
            hints.push("Y (sağa) sütunundaki değerler X (yukarı) gibi büyük, X sütunundakiler Y gibi; Y ve X yer değiştirmiş olabilir. Sütun sırasını denetleyin.".to_string());
        }
        // Degrees have many decimals; local survey coordinates two or three.
        let fine = ys
            .iter()
            .chain(&xs)
            .filter(|v| (**v * 1000.0).fract() != 0.0)
            .count();
        if ys.iter().all(|v| v.abs() <= 180.0)
            && xs.iter().all(|v| v.abs() <= 90.0)
            && fine * 10 >= (ys.len() + xs.len()) * 8
        {
            hints.push("Değerler enlem/boylam (derece) gibi görünüyor. Çizim araçları metre cinsinden projeksiyonlu bir sistem bekler; dosyanın sistemini denetleyin.".to_string());
        }
    }
    if duplicate_names > 0 {
        report.note_n(
            "Aynı adlı nokta",
            "başka bir noktanın adını taşıyor; ikisi de alınır",
            0,
            duplicate_names,
        );
    }
    if lines.is_empty() {
        hints.push("Dosyada okunacak satır yok.".to_string());
    }

    CoordRead {
        encoding: enc.label().to_string(),
        delimiter,
        decimal,
        header,
        header_fields,
        columns,
        preview,
        data_lines: u32::try_from(lines.len()).unwrap_or(u32::MAX),
        points,
        errors,
        error_count,
        duplicate_names,
        hints,
        bounds,
        result: opts.entities.then(|| ImportResult {
            entities,
            layers: Vec::new(),
            report: report.import(),
            bounds,
        }),
    }
}

/// Writes points as a coordinate list (CRLF lines, the shortest exact
/// decimals). Names cannot hold the delimiter where the format has no
/// quoting (spaces in NCN): those become '_' and are reported.
pub fn write(input: &CoordWriteInput) -> (Vec<u8>, ExportReport) {
    let mut report = Report::default();
    let sep = match input.delimiter {
        CoordDelimiter::Tab => "\t",
        CoordDelimiter::Semicolon => ";",
        CoordDelimiter::Comma => ",",
        CoordDelimiter::Space | CoordDelimiter::Auto => " ",
    };
    let columns: Vec<CoordColumn> = input
        .columns
        .iter()
        .copied()
        .filter(|c| *c != CoordColumn::Skip)
        .collect();
    let quote = |s: &str| -> String {
        match delimiter_char(input.delimiter) {
            Some(d) if s.contains(d) || s.contains('"') => {
                format!("\"{}\"", s.replace('"', "\"\""))
            }
            Some(_) => s.to_string(),
            None if s.chars().any(char::is_whitespace) => {
                s.split_whitespace().collect::<Vec<_>>().join("_")
            }
            None => s.to_string(),
        }
    };
    let mut out = String::new();
    if input.header {
        let names: Vec<&str> = columns
            .iter()
            .map(|c| match c {
                CoordColumn::Name => "Ad",
                CoordColumn::Y => "Y",
                CoordColumn::X => "X",
                CoordColumn::Z => "Z",
                CoordColumn::Code => "Kod",
                CoordColumn::Skip => "",
            })
            .collect();
        out.push_str(&names.join(sep));
        out.push_str("\r\n");
    }
    for p in &input.points {
        let mut fields: Vec<String> = columns
            .iter()
            .map(|c| match c {
                CoordColumn::Name => {
                    let q = quote(&p.name);
                    if q != p.name && delimiter_char(input.delimiter).is_none() {
                        report.note(
                            "Nokta adı",
                            "adındaki boşluklar “_” oldu (NCN'de alanlar boşlukla ayrılır)",
                            0,
                        );
                    }
                    q
                }
                CoordColumn::Y => plain(p.p.x),
                CoordColumn::X => plain(p.p.y),
                CoordColumn::Z => p.z.map(plain).unwrap_or_default(),
                CoordColumn::Code => quote(p.code.as_deref().unwrap_or("")),
                CoordColumn::Skip => String::new(),
            })
            .collect();
        // Space-separated lists cannot hold an empty field: a missing Z at the end is left out.
        if delimiter_char(input.delimiter).is_none() {
            while fields.last().is_some_and(String::is_empty) {
                fields.pop();
            }
        }
        if p.z.is_none() && columns.contains(&CoordColumn::Z) {
            report.note("Kotsuz nokta", "Z değeri olmadığı için boş bırakıldı", 0);
        }
        out.push_str(&fields.join(sep));
        out.push_str("\r\n");
        report.count("point");
    }
    let bytes = match input.encoding {
        TextEncoding::Utf8 => out.into_bytes(),
        TextEncoding::Windows1254 => {
            let mut bytes = Vec::with_capacity(out.len());
            let mut lost = 0;
            text::encode_windows1254(&out, &mut bytes, |_, o| {
                lost += 1;
                o.push(b'?');
            });
            report.note_n(
                "Karakter",
                "Windows-1254'te olmadığı için “?” yazıldı; UTF-8 seçerek koruyabilirsiniz",
                0,
                lost,
            );
            bytes
        }
    };
    (bytes, report.export())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opts() -> CoordReadOptions {
        CoordReadOptions {
            preview_rows: 10,
            entities: true,
            ..Default::default()
        }
    }

    fn points(r: &CoordRead) -> Vec<(String, f64, f64, Option<f64>)> {
        r.result
            .as_ref()
            .map(|res| {
                res.entities
                    .iter()
                    .filter_map(|e| match e {
                        Entity::Point(p) => {
                            Some((p.base.label.clone().unwrap_or_default(), p.p.x, p.p.y, p.z))
                        }
                        _ => None,
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    #[test]
    fn netcad_order_with_spaces_and_numeric_names() {
        let r = read(
            b"1001 452345.123 4412345.678 105.2\n1002   452346.5\t4412346.25 106\n",
            &opts(),
        );
        assert_eq!(r.delimiter, CoordDelimiter::Space);
        assert_eq!(r.decimal, DecimalMark::Point);
        assert!(!r.header);
        assert_eq!(
            r.columns,
            vec![
                CoordColumn::Name,
                CoordColumn::Y,
                CoordColumn::X,
                CoordColumn::Z
            ]
        );
        assert_eq!(
            points(&r),
            vec![
                ("1001".into(), 452345.123, 4412345.678, Some(105.2)),
                ("1002".into(), 452346.5, 4412346.25, Some(106.0))
            ]
        );
        assert!(r.hints.is_empty(), "{:?}", r.hints);
    }

    #[test]
    fn a_turkish_spreadsheet_with_semicolons_and_decimal_commas() {
        let r = read(
            "Nokta;Sağa;Yukarı;Kot\r\nP1;452345,12;4412345,5;12,75\r\nP2;452346;4412346;\r\n"
                .as_bytes(),
            &opts(),
        );
        assert_eq!(r.delimiter, CoordDelimiter::Semicolon);
        assert_eq!(r.decimal, DecimalMark::Comma);
        assert!(r.header);
        assert_eq!(r.header_fields, vec!["Nokta", "Sağa", "Yukarı", "Kot"]);
        assert_eq!(
            r.columns,
            vec![
                CoordColumn::Name,
                CoordColumn::Y,
                CoordColumn::X,
                CoordColumn::Z
            ]
        );
        assert_eq!(
            points(&r),
            vec![
                ("P1".into(), 452345.12, 4412345.5, Some(12.75)),
                ("P2".into(), 452346.0, 4412346.0, None)
            ]
        );
        // The elevation attribute keeps the file's digits, with a decimal point.
        let Some(Entity::Point(p)) = r.result.as_ref().and_then(|x| x.entities.first()) else {
            panic!()
        };
        assert_eq!(p.base.attrs.get(ATTR_Z).map(String::as_str), Some("12.75"));
        assert_eq!(p.base.attrs.get(ATTR_NAME).map(String::as_str), Some("P1"));
    }

    #[test]
    fn csv_with_quotes_comments_and_a_code_column() {
        let text = "# ölçü 2026\n\"P,1\",452345.1,4412345.2,10,sınır taşı\n// yorum\nP2,452345.3,4412345.4,11,\"ağaç \"\"çınar\"\"\"\n";
        let r = read(text.as_bytes(), &opts());
        assert_eq!(r.delimiter, CoordDelimiter::Comma);
        assert_eq!(
            r.columns,
            vec![
                CoordColumn::Name,
                CoordColumn::Y,
                CoordColumn::X,
                CoordColumn::Z,
                CoordColumn::Code
            ]
        );
        assert_eq!(r.data_lines, 2);
        let res = r.result.expect("entities");
        let Entity::Point(p) = &res.entities[1] else {
            panic!()
        };
        assert_eq!(
            p.base.attrs.get(ATTR_CODE).map(String::as_str),
            Some("ağaç \"çınar\"")
        );
        let Entity::Point(p) = &res.entities[0] else {
            panic!()
        };
        assert_eq!(p.base.label.as_deref(), Some("P,1"));
        assert_eq!(r.preview[0].line, 2);
    }

    #[test]
    fn bad_lines_are_reported_with_their_numbers() {
        let text = "P1 452345.1 4412345.2\nP2 abc 4412345.3\nP3 452345.5\nP4 452345.6 4412345.7\n";
        let r = read(text.as_bytes(), &opts());
        assert_eq!(r.points, 2);
        assert_eq!(r.error_count, 2);
        assert_eq!(r.errors[0].line, 2);
        assert_eq!(
            r.errors[0].message,
            "Satır 2: Y (sağa) değeri “abc” sayı değil."
        );
        assert_eq!(
            r.errors[1].message,
            "Satır 3: 2 alan var; X (yukarı) için en az 3 alan gerekiyor."
        );
        assert_eq!(
            r.preview[1].error.as_deref(),
            Some("Y (sağa) değeri “abc” sayı değil.")
        );
        let skipped = &r.result.expect("entities").report.skipped;
        assert_eq!(skipped.len(), 2);
        assert_eq!(skipped[0].lines, vec![2]);
    }

    #[test]
    fn sizes_tell_northings_from_eastings() {
        // No names; the first column holds northings.
        let r = read(
            b"4412345.678 452345.123 100.5\n4412346.678 452346.123 101.5\n",
            &opts(),
        );
        assert_eq!(
            r.columns,
            vec![CoordColumn::X, CoordColumn::Y, CoordColumn::Z]
        );
        assert_eq!(
            points(&r)[0],
            (String::new(), 452345.123, 4412345.678, Some(100.5))
        );
        assert_eq!(r.hints.len(), 1);
        // Chosen by hand the other way round, the reader says what it sees.
        let r = read(
            b"4412345.678 452345.123 100.5\n",
            &CoordReadOptions {
                columns: vec![CoordColumn::Y, CoordColumn::X, CoordColumn::Z],
                ..opts()
            },
        );
        assert!(
            r.hints.iter().any(|h| h.contains("yer değiştirmiş")),
            "{:?}",
            r.hints
        );
    }

    #[test]
    fn an_international_header_is_swapped_by_the_numbers() {
        let r = read(b"name,x,y,z\nA,452345.1,4412345.2,5\n", &opts());
        assert_eq!(
            r.columns,
            vec![
                CoordColumn::Name,
                CoordColumn::Y,
                CoordColumn::X,
                CoordColumn::Z
            ]
        );
        assert_eq!(r.hints.len(), 1);
    }

    #[test]
    fn exact_values_survive_writing_and_reading() {
        let pts = vec![
            kentos_contracts::CoordPoint {
                name: "P 1".into(),
                p: Vec2 {
                    x: 452345.123,
                    y: 4412345.678,
                },
                z: Some(0.1 + 0.2),
                code: None,
            },
            kentos_contracts::CoordPoint {
                name: "Ağaç".into(),
                p: Vec2 {
                    x: 1.0 / 3.0,
                    y: -2.5e-7,
                },
                z: None,
                code: None,
            },
        ];
        for (delimiter, encoding) in [
            (CoordDelimiter::Space, TextEncoding::Utf8),
            (CoordDelimiter::Tab, TextEncoding::Windows1254),
            (CoordDelimiter::Semicolon, TextEncoding::Utf8),
            (CoordDelimiter::Comma, TextEncoding::Utf8),
        ] {
            let input = CoordWriteInput {
                points: pts.clone(),
                delimiter,
                columns: vec![
                    CoordColumn::Name,
                    CoordColumn::Y,
                    CoordColumn::X,
                    CoordColumn::Z,
                ],
                header: delimiter != CoordDelimiter::Space,
                encoding,
            };
            let (bytes, rep) = write(&input);
            assert_eq!(rep.counts.get("point"), Some(&2));
            let back = read(
                &bytes,
                &CoordReadOptions {
                    columns: input.columns.clone(),
                    ..opts()
                },
            );
            assert_eq!(back.delimiter, delimiter, "{delimiter:?}");
            assert_eq!(back.header, input.header);
            let got = points(&back);
            assert_eq!(got.len(), 2, "{delimiter:?}");
            assert_eq!(got[0].1.to_bits(), pts[0].p.x.to_bits());
            assert_eq!(got[0].2.to_bits(), pts[0].p.y.to_bits());
            assert_eq!(got[0].3.map(f64::to_bits), pts[0].z.map(f64::to_bits));
            assert_eq!(got[1].1.to_bits(), pts[1].p.x.to_bits());
            assert_eq!(got[1].2.to_bits(), pts[1].p.y.to_bits());
            assert_eq!(got[1].3, None);
            assert_eq!(got[1].0, "Ağaç");
            assert_eq!(
                got[0].0,
                if delimiter == CoordDelimiter::Space {
                    "P_1"
                } else {
                    "P 1"
                }
            );
        }
    }
}
