//! Aquaveo SMS's 2DM meshes and ASCII DAT datasets (docs/adr/0243 §4).
//! 2DM: `ND id x y z` nodes, `E3T`/`E4Q` elements and the second-order
//! `E6T`, `E8Q`, `E9Q` by their corner nodes; nodes and elements are taken
//! in the order of their ids (gaps allowed). DAT: `DATASET` blocks of scalar
//! (`BEGSCL`) or vector (`BEGVEC`) values, one per node or per element, a
//! step at a time (`TS istat time`, the elements' activity first when
//! `istat` is 1), times in `TIMEUNITS` from `RT_JULIAN` when it is given.

use std::collections::HashMap;

use crate::raster::RasterError;

/// The most lines a 2DM or DAT may have.
const MAX_LINES: usize = 400_000_000;

/// A 2DM mesh.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Sms2dm {
    pub x: Vec<f64>,
    pub y: Vec<f64>,
    /// The nodes' heights (Taban kotu).
    pub z: Vec<f64>,
    /// Faces' nodes, indices from 0, in the elements' id order.
    pub faces: Vec<Vec<u32>>,
    /// Line elements and other cards left out, counted.
    pub skipped: usize,
}

fn fields(line: &str) -> impl Iterator<Item = &str> {
    line.split_ascii_whitespace()
}

fn err(line: usize, why: impl std::fmt::Display) -> RasterError {
    RasterError::new(format!("{line}. satır: {why}"))
}

/// A 2DM file's mesh.
pub fn read_2dm(bytes: &[u8]) -> Result<Sms2dm, RasterError> {
    let text = String::from_utf8_lossy(bytes);
    let mut first = true;
    let mut nodes: Vec<(u64, f64, f64, f64)> = Vec::new();
    let mut elements: Vec<(u64, Vec<u64>, usize)> = Vec::new();
    let mut skipped = 0usize;
    for (k, line) in text.lines().enumerate() {
        let n = k + 1;
        if n > MAX_LINES {
            return Err(RasterError::new("2DM dosyası çok uzun."));
        }
        let mut f = fields(line);
        let Some(card) = f.next() else { continue };
        if first {
            if !card.eq_ignore_ascii_case("MESH2D") {
                return Err(RasterError::new(
                    "Dosya 2DM değil: ilk satır MESH2D olmalı.",
                ));
            }
            first = false;
            continue;
        }
        let num = |s: Option<&str>| -> Result<f64, RasterError> {
            s.and_then(|s| s.parse::<f64>().ok())
                .filter(|v| v.is_finite())
                .ok_or_else(|| err(n, "sayı okunamadı."))
        };
        let id = |s: Option<&str>| -> Result<u64, RasterError> {
            s.and_then(|s| s.parse::<u64>().ok())
                .filter(|&v| v > 0)
                .ok_or_else(|| err(n, "numara okunamadı (pozitif tam sayı olmalı)."))
        };
        match card.to_ascii_uppercase().as_str() {
            "ND" => {
                let i = id(f.next())?;
                let (x, y) = (num(f.next())?, num(f.next())?);
                let z = f.next().map_or(Ok(0.0), |s| num(Some(s)))?;
                nodes.push((i, x, y, z));
            }
            c @ ("E3T" | "E4Q" | "E6T" | "E8Q" | "E9Q") => {
                let (count, corners): (usize, &[usize]) = match c {
                    "E3T" => (3, &[0, 1, 2]),
                    "E4Q" => (4, &[0, 1, 2, 3]),
                    "E6T" => (6, &[0, 2, 4]),
                    "E8Q" => (8, &[0, 2, 4, 6]),
                    _ => (9, &[0, 2, 4, 6]),
                };
                let i = id(f.next())?;
                let mut all = Vec::with_capacity(count);
                for _ in 0..count {
                    all.push(id(f.next())?);
                }
                elements.push((i, corners.iter().map(|&c| all[c]).collect(), n));
            }
            "E2L" | "E3L" => skipped += 1,
            _ => {}
        }
    }
    if first {
        return Err(RasterError::new("2DM dosyası boş."));
    }
    nodes.sort_by_key(|t| t.0);
    if let Some(w) = nodes.windows(2).find(|w| w[0].0 == w[1].0) {
        return Err(RasterError::new(format!(
            "2DM'de {} numaralı düğüm iki kez var.",
            w[0].0
        )));
    }
    if nodes.len() < 3 {
        return Err(RasterError::new("2DM'de en az üç düğüm olmalı."));
    }
    let index: HashMap<u64, u32> = nodes
        .iter()
        .enumerate()
        .map(|(k, t)| (t.0, k as u32))
        .collect();
    elements.sort_by_key(|e| e.0);
    if let Some(w) = elements.windows(2).find(|w| w[0].0 == w[1].0) {
        return Err(RasterError::new(format!(
            "2DM'de {} numaralı eleman iki kez var.",
            w[0].0
        )));
    }
    if elements.is_empty() {
        return Err(RasterError::new("2DM'de üçgen ya da dörtgen eleman yok."));
    }
    let mut faces = Vec::with_capacity(elements.len());
    for (eid, corners, line) in &elements {
        let mut face = Vec::with_capacity(corners.len());
        for c in corners {
            match index.get(c) {
                Some(&k) => face.push(k),
                None => {
                    return Err(err(
                        *line,
                        format!("{eid} numaralı eleman olmayan {c} numaralı düğümü anıyor."),
                    ));
                }
            }
        }
        faces.push(face);
    }
    Ok(Sms2dm {
        x: nodes.iter().map(|t| t.1).collect(),
        y: nodes.iter().map(|t| t.2).collect(),
        z: nodes.iter().map(|t| t.3).collect(),
        faces,
        skipped,
    })
}

/// A DAT step: its time (in the dataset's unit), values (two per place for
/// a vector) and the elements' activity (1 active, 0 not), when given.
#[derive(Clone, Debug, PartialEq)]
pub struct DatStep {
    pub time: f64,
    pub values: Vec<f32>,
    pub active: Option<Vec<u8>>,
}

/// A DAT dataset.
#[derive(Clone, Debug, PartialEq)]
pub struct DatDataset {
    pub name: String,
    pub vector: bool,
    /// Values a step: the 2DM's nodes (`ND`) or its elements.
    pub count: usize,
    /// Elements (`NC`).
    pub cells: usize,
    /// Milliseconds a time unit.
    pub unit_ms: f64,
    /// The reference time's moment (`RT_JULIAN`), ms since 1970.
    pub reference: Option<f64>,
    pub steps: Vec<DatStep>,
}

/// An ASCII DAT file's datasets.
pub fn read_dat(bytes: &[u8]) -> Result<Vec<DatDataset>, RasterError> {
    let text = String::from_utf8_lossy(bytes);
    let mut lines = text.lines().enumerate().peekable();
    let mut out: Vec<DatDataset> = Vec::new();
    let mut cur: Option<DatDataset> = None;
    let mut seen_dataset = false;
    while let Some((k, line)) = lines.next() {
        let n = k + 1;
        let mut f = fields(line);
        let Some(card) = f.next() else { continue };
        let upper = card.to_ascii_uppercase();
        match upper.as_str() {
            "DATASET" => {
                seen_dataset = true;
                cur = Some(DatDataset {
                    name: String::new(),
                    vector: false,
                    count: 0,
                    cells: 0,
                    unit_ms: 3_600_000.0,
                    reference: None,
                    steps: Vec::new(),
                });
            }
            "BEGSCL" | "BEGVEC" => {
                let d = cur
                    .as_mut()
                    .ok_or_else(|| err(n, "DATASET'ten önce veri seti başlıyor."))?;
                d.vector = upper == "BEGVEC";
            }
            "ND" | "NC" => {
                let d = cur
                    .as_mut()
                    .ok_or_else(|| err(n, "DATASET'ten önce sayı var."))?;
                let v = f
                    .next()
                    .and_then(|s| s.parse::<usize>().ok())
                    .ok_or_else(|| err(n, "sayı okunamadı."))?;
                if upper == "ND" {
                    d.count = v;
                } else {
                    d.cells = v;
                }
            }
            "NAME" => {
                let d = cur
                    .as_mut()
                    .ok_or_else(|| err(n, "DATASET'ten önce ad var."))?;
                let rest = line.trim_start()[card.len()..].trim();
                d.name = rest.trim_matches('"').trim().to_owned();
            }
            "TIMEUNITS" => {
                let d = cur
                    .as_mut()
                    .ok_or_else(|| err(n, "DATASET'ten önce zaman birimi var."))?;
                d.unit_ms = match f.next().map(str::to_ascii_lowercase).as_deref() {
                    Some("hours" | "hour" | "h") => 3_600_000.0,
                    Some("minutes" | "minute" | "min") => 60_000.0,
                    Some("seconds" | "second" | "sec" | "s") => 1000.0,
                    Some("days" | "day" | "d") => 86_400_000.0,
                    _ => {
                        return Err(err(
                            n,
                            "zaman birimi Hours, Minutes, Seconds ya da Days olmalı.",
                        ));
                    }
                };
            }
            "RT_JULIAN" => {
                let d = cur
                    .as_mut()
                    .ok_or_else(|| err(n, "DATASET'ten önce başlangıç zamanı var."))?;
                let jd = f
                    .next()
                    .and_then(|s| s.parse::<f64>().ok())
                    .filter(|v| v.is_finite())
                    .ok_or_else(|| err(n, "Jülyen günü okunamadı."))?;
                // JD 2440587.5 is 1970-01-01 00:00 UTC.
                d.reference = Some(((jd - 2_440_587.5) * 86_400_000.0).round());
            }
            "TS" => {
                let d = cur
                    .as_mut()
                    .ok_or_else(|| err(n, "DATASET'ten önce adım var."))?;
                if d.count == 0 {
                    return Err(err(n, "adımdan önce ND verilmeli."));
                }
                let istat = f
                    .next()
                    .and_then(|s| s.parse::<i64>().ok())
                    .ok_or_else(|| err(n, "TS'nin etkinlik bayrağı okunamadı."))?;
                let time = f
                    .next()
                    .and_then(|s| s.parse::<f64>().ok())
                    .filter(|v| v.is_finite())
                    .ok_or_else(|| err(n, "TS'nin zamanı okunamadı."))?;
                let active = if istat == 1 {
                    if d.cells == 0 {
                        return Err(err(n, "etkinlik bayrağı için NC verilmeli."));
                    }
                    let mut a = Vec::with_capacity(d.cells);
                    while a.len() < d.cells {
                        let Some((k2, l2)) = lines.next() else {
                            return Err(RasterError::new(format!(
                                "DAT dosyası kesik: {}. adımın etkinlikleri eksik.",
                                d.steps.len() + 1
                            )));
                        };
                        for s in fields(l2) {
                            let v = s
                                .parse::<i64>()
                                .map_err(|_| err(k2 + 1, "etkinlik 0 ya da 1 olmalı."))?;
                            a.push(u8::from(v != 0));
                        }
                    }
                    a.truncate(d.cells);
                    Some(a)
                } else {
                    None
                };
                let per = if d.vector { 2 } else { 1 };
                let want = d.count * per;
                let mut values = Vec::with_capacity(want);
                while values.len() < want {
                    let Some((k2, l2)) = lines.next() else {
                        return Err(RasterError::new(format!(
                            "DAT dosyası kesik: {}. adımın değerleri eksik.",
                            d.steps.len() + 1
                        )));
                    };
                    for s in fields(l2) {
                        let v = s
                            .parse::<f64>()
                            .map_err(|_| err(k2 + 1, "değer okunamadı."))?;
                        values.push(v as f32);
                    }
                }
                values.truncate(want);
                d.steps.push(DatStep {
                    time,
                    values,
                    active,
                });
            }
            "ENDDS" => {
                if let Some(d) = cur.take() {
                    out.push(d);
                }
            }
            _ => {}
        }
    }
    if let Some(d) = cur.take() {
        out.push(d);
    }
    if !seen_dataset {
        return Err(RasterError::new(
            "Dosya ASCII DAT değil: DATASET satırı yok.",
        ));
    }
    for (k, d) in out.iter_mut().enumerate() {
        if d.name.is_empty() {
            d.name = format!("Veri seti {}", k + 1);
        }
        if d.steps.is_empty() {
            return Err(RasterError::new(format!(
                "“{}” veri setinin adımı yok.",
                d.name
            )));
        }
        if d.steps.windows(2).any(|w| w[1].time < w[0].time) {
            return Err(RasterError::new(format!(
                "“{}” veri setinin zamanları artmıyor.",
                d.name
            )));
        }
    }
    Ok(out)
}
