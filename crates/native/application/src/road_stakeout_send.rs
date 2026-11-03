//! `road.stakeout.send` v1 (plan/road.md Y10.4, docs/adr/0908 §9; ROAD-IO-13): the stakeout table's points written to
//! an instrument: as a LandXML string road (the corridor's alignment, its own profile and each code's points as a
//! `PlanFeature` string, what Trimble Access, Leica Captivate and Topcon MAGNET read), or to an instrument's coordinate
//! file by docs/adr/0169 §4's writers (Leica GSI-16 and GSI-8, Topcon GTS-7, Trimble JobXML, Nikon RAW, CSV). Whole or
//! not at all; the drawing does not change. The shared cases are `fixtures/commands/v1/road.stakeout.send.json`.

use std::path::{Path, PathBuf};

use kentos_commands::{
    CommandResult, CommandWarning, StakeRow, StakeoutDistances, StakeoutLeft, StakeoutQuery, StakeoutSend,
    StakeoutSendFormat, StakeoutSent,
};
use kentos_commands::{FieldPoint, FieldWriteFormat, FieldWriteOptions};
use kentos_domain::Document;
use kentos_formats::landxml_write::{WFeature, WLandXml};
use sha2::{Digest, Sha256};

use crate::ExecutionContext;
use crate::checks::{self, Stop};
use crate::codes;
use crate::road_alignments as alignments;
use crate::road_landxml::{alignment_written, now, stamp_of};
use crate::road_terrains::fail;

/// The largest first number (eight digits, what GSI-8's name word holds).
pub const MOST_FIRST: u32 = 99_999_999;

fn bad(path: &str, words: String) -> Stop {
    fail(codes::INVALID_STAKEOUT_SEND, words, path)
}

fn warning(code: &str, message: String) -> CommandWarning {
    CommandWarning { code: code.into(), message, path: Some("format".into()) }
}

/// The instrument writer's format, none for LandXML.
fn field_format(f: StakeoutSendFormat) -> Option<FieldWriteFormat> {
    match f {
        StakeoutSendFormat::Landxml => None,
        StakeoutSendFormat::Gsi16 => Some(FieldWriteFormat::Gsi16),
        StakeoutSendFormat::Gsi8 => Some(FieldWriteFormat::Gsi8),
        StakeoutSendFormat::Gts7 => Some(FieldWriteFormat::Gts7),
        StakeoutSendFormat::Jobxml => Some(FieldWriteFormat::Jobxml),
        StakeoutSendFormat::Nikon => Some(FieldWriteFormat::Nikon),
        StakeoutSendFormat::Csv => Some(FieldWriteFormat::Csv),
    }
}

/// The file's extension and the format's name.
pub fn extension(f: StakeoutSendFormat) -> (&'static str, &'static str) {
    match f {
        StakeoutSendFormat::Landxml => ("xml", "LandXML"),
        StakeoutSendFormat::Gsi16 => ("gsi", "Leica GSI-16"),
        StakeoutSendFormat::Gsi8 => ("gsi", "Leica GSI-8"),
        StakeoutSendFormat::Gts7 => ("xyz", "Topcon GTS-7"),
        StakeoutSendFormat::Jobxml => ("jxl", "Trimble JobXML"),
        StakeoutSendFormat::Nikon => ("raw", "Nikon RAW"),
        StakeoutSendFormat::Csv => ("csv", "CSV"),
    }
}

/// Each code's strings in the table (docs/adr/0908 §9): the rows' (station, region) pairs in order are the table's
/// sections; a code's points follow them and its string breaks at a section without it; a point equal to the one before
/// (a station two regions share) is taken once. Codes in the order they first come; a code with more than one string
/// names them “code (1)”, “code (2)”. Strings of one point are not strings: their count is given back.
pub fn strings(rows: &[StakeRow]) -> (Vec<WFeature>, u32) {
    let mut keys: Vec<(f64, u32)> = Vec::new();
    let mut at: Vec<usize> = Vec::with_capacity(rows.len());
    for r in rows {
        if keys.last() != Some(&(r.station, r.region)) {
            keys.push((r.station, r.region));
        }
        at.push(keys.len() - 1);
    }
    let mut order: Vec<&str> = Vec::new();
    for r in rows {
        if !order.contains(&r.code.as_str()) {
            order.push(&r.code);
        }
    }
    let mut out = Vec::new();
    let mut short = 0;
    for code in order {
        let mut runs: Vec<Vec<[f64; 3]>> = Vec::new();
        let mut last: Option<usize> = None;
        for (r, k) in rows.iter().zip(&at) {
            if r.code != code {
                continue;
            }
            let p = [r.x, r.y, r.z];
            match (last, runs.last_mut()) {
                (Some(l), Some(run)) if l + 1 >= *k => {
                    let q = run[run.len() - 1];
                    let same = (0..3).all(|i| (p[i] - q[i]).abs() <= 1e-9);
                    if !same {
                        run.push(p);
                    }
                }
                _ => runs.push(vec![p]),
            }
            last = Some(*k);
        }
        let long: Vec<Vec<[f64; 3]>> = runs.into_iter().filter(|r| r.len() >= 2 || { short += 1; false }).collect();
        let many = long.len() > 1;
        for (i, points) in long.into_iter().enumerate() {
            let name = if many { format!("{code} ({})", i + 1) } else { code.to_owned() };
            out.push(WFeature { name, points });
        }
    }
    (out, short)
}

struct Planned {
    path: PathBuf,
    out: StakeoutSent,
    text: String,
    warnings: Vec<CommandWarning>,
}

fn check(doc: &Document, input: &StakeoutSend) -> Result<Planned, Stop> {
    let query = StakeoutQuery {
        corridor: input.corridor.clone(),
        from: input.from,
        to: input.to,
        step: input.step,
        codes: input.codes.clone(),
        distances: StakeoutDistances::Grid,
    };
    let (table, mut warnings, built) = crate::road_stakeout::table(doc, &query)?;
    let (ext, format_name) = extension(input.format);
    let text = input.path.trim();
    if text.is_empty() || !Path::new(text).is_absolute() {
        return Err(fail(
            codes::INVALID_FILE_PATH,
            format!("Dosyanın yolu boş ya da göreli; mutlak bir .{ext} yolu verin."),
            "path",
        ));
    }
    if !text.to_ascii_lowercase().ends_with(&format!(".{ext}")) {
        return Err(fail(codes::INVALID_FILE_PATH, format!("“{text}” bir .{ext} dosyası değil ({format_name})."), "path"));
    }
    let path = PathBuf::from(text);
    let folder = path.parent().map(Path::to_path_buf).unwrap_or_default();
    if !folder.is_dir() {
        return Err(fail(
            codes::INVALID_FILE_PATH,
            format!("“{}” klasörü yok; dosya var olan bir klasöre yazılır.", folder.display()),
            "path",
        ));
    }
    if path.exists() && !input.overwrite {
        return Err(fail(codes::FILE_EXISTS, format!("{} var; üzerine yazmak için overwrite verin.", path.display()), "path"));
    }
    let first = input.first.unwrap_or(1);
    if !(1..=MOST_FIRST).contains(&first) {
        return Err(bad("first", format!("İlk nokta numarası 1 ile {MOST_FIRST} arası olmalı.")));
    }
    let (date, time) = match input.stamp.as_deref() {
        None => now(),
        Some(s) => stamp_of(s).ok_or_else(|| bad("stamp", format!("“{s}” bir tarih ve saat değil; YYYY-MM-DDThh:mm:ss yazın.")))?,
    };
    if table.rows.is_empty() {
        return Err(bad("", "Gönderilecek nokta yok: aralıkta ya da kodlarda kesit noktası çıkmadı.".into()));
    }
    let mut out = StakeoutSent {
        path: path.display().to_string(),
        format: input.format,
        corridor: table.corridor.clone(),
        name: table.name.clone(),
        stations: table.stations,
        points: 0,
        left: Vec::new(),
        codes_dropped: 0,
        strings: 0,
        alignment: None,
        profile: None,
        bytes: 0,
        sha256: String::new(),
    };
    let text = match field_format(input.format) {
        None => {
            let (_, def) = crate::road_corridors::find(doc.road_corridors(), &table.corridor, "corridor")?;
            let list = alignments::alignments(doc);
            let (_, adef) = alignments::find(list, &def.alignment, "corridor")?;
            let profiles: Vec<_> = doc.road_profiles().iter().filter(|p| p.id == def.profile).collect();
            let mut lost = Vec::new();
            let (a, written) = alignment_written(adef, &built, &profiles, &mut lost);
            for w in lost {
                warnings.push(warning(codes::EXPORT_LOSSES, w.message));
            }
            let (features, short) = strings(&table.rows);
            if short > 0 {
                warnings.push(warning(
                    codes::EXPORT_LOSSES,
                    format!("{short} dizi tek noktalı (o kodun komşu km'lerde noktası yok); string olarak yazılmadı."),
                ));
            }
            if features.is_empty() {
                return Err(bad("", "Gönderilecek dizi yok: hiçbir kodun iki ardışık km'de noktası yok.".into()));
            }
            out.points = features.iter().map(|f| f.points.len() as u32).sum();
            out.strings = features.len() as u32;
            out.alignment = Some(written.name);
            out.profile = written.profiles.into_iter().next();
            let file = WLandXml {
                date,
                time,
                application: "KentOS Yol".into(),
                version: env!("CARGO_PKG_VERSION").into(),
                project: doc.name().to_owned(),
                alignments: vec![a],
                features,
                features_name: table.name.clone(),
                ..WLandXml::default()
            };
            kentos_formats::landxml_write::write(&file)
        }
        Some(format) => {
            let stamp = format!("{date}T{time}");
            let given = input.job.as_deref().map(str::trim).filter(|j| !j.is_empty());
            let job = given.unwrap_or(&table.name).to_owned();
            let options = |job: &str| FieldWriteOptions { format, job: job.to_owned(), stamp: stamp.clone() };
            // A code the format cannot carry is left off its points: a lone point with only that code tells.
            let mut fits: Vec<(&str, bool)> = Vec::new();
            let mut dropped: Vec<&str> = Vec::new();
            let mut points = Vec::with_capacity(table.rows.len());
            for (i, r) in table.rows.iter().enumerate() {
                let ok = match fits.iter().find(|(c, _)| *c == r.code) {
                    Some((_, ok)) => *ok,
                    None => {
                        let probe = FieldPoint {
                            name: "1".into(),
                            east: 0.0,
                            north: 0.0,
                            elevation: Some(0.0),
                            code: Some(r.code.clone()),
                        };
                        let ok = kentos_formats::field::write::write(&[probe], &options("KentOS")).skipped.is_empty();
                        fits.push((&r.code, ok));
                        if !ok {
                            dropped.push(&r.code);
                        }
                        ok
                    }
                };
                if !ok {
                    out.codes_dropped += 1;
                }
                points.push(FieldPoint {
                    name: (u64::from(first) + i as u64).to_string(),
                    east: r.x,
                    north: r.y,
                    elevation: Some(r.z),
                    code: ok.then(|| r.code.clone()),
                });
            }
            let mut written = kentos_formats::field::write::write(&points, &options(&job));
            if written.error.is_some() && given.is_none() {
                written = kentos_formats::field::write::write(&points, &options("KentOS"));
            }
            if let Some(why) = written.error {
                return Err(bad("job", why));
            }
            if !dropped.is_empty() {
                warnings.push(warning(
                    codes::STAKEOUT_CODE_DROPPED,
                    format!(
                        "{format_name} şu kodları taşıyamıyor; {} noktanın kodu yazılmadı: {}.",
                        out.codes_dropped,
                        dropped.join(", ")
                    ),
                ));
            }
            out.left = written.skipped.iter().map(|s| StakeoutLeft { name: s.name.clone(), problem: s.problem.clone() }).collect();
            if written.written == 0 {
                let why = out.left.first().map_or(String::new(), |l| format!(" İlki: {}", l.problem));
                return Err(bad("format", format!("{format_name} dosyasına yazılabilen nokta yok.{why}")));
            }
            if !out.left.is_empty() {
                warnings.push(warning(
                    codes::EXPORT_LOSSES,
                    format!("{} nokta {format_name} dosyasında taşınamıyor; yazılmadı.", out.left.len()),
                ));
            }
            out.points = written.written;
            written.text
        }
    };
    out.bytes = text.len() as u64;
    out.sha256 = Sha256::digest(text.as_bytes()).iter().map(|b| format!("{b:02x}")).collect();
    Ok(Planned { path, out, text, warnings })
}

/// Checks `input` against the document; writes nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &StakeoutSend) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(p) => CommandResult::Completed { output: (), warnings: p.warnings },
        Err(stop) => stop.into(),
    }
}

/// What the file would hold; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &StakeoutSend) -> CommandResult<StakeoutSent> {
    match check(cx.doc, input) {
        Ok(p) => CommandResult::Completed { output: p.out, warnings: p.warnings },
        Err(stop) => stop.into(),
    }
}

/// Writes the file; the drawing does not change.
pub fn execute(cx: &mut ExecutionContext<'_>, input: StakeoutSend) -> CommandResult<StakeoutSent> {
    let p = match check(cx.doc, &input) {
        Ok(p) => p,
        Err(stop) => return stop.into(),
    };
    if let Err(why) = crate::road_ifc::write_file(&p.path, &p.text) {
        return Stop::failed(checks::error(
            codes::INVALID_FILE_PATH,
            format!("Dosya yazılamadı: {why}. Hiçbir şey değişmedi."),
            Some("path".into()),
        ))
        .into();
    }
    CommandResult::Completed { output: p.out, warnings: p.warnings }
}

/// The file's text for a window or a test: the text, the answer and its warnings, or why there is none.
pub fn text(doc: &Document, input: &StakeoutSend) -> Result<(String, StakeoutSent, Vec<CommandWarning>), String> {
    check(doc, input).map(|p| (p.text, p.out, p.warnings)).map_err(crate::road_corridors::words)
}
