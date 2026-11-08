//! The field package a returned record was made with, as `cad.asbuilt.check` reads it (plan/map-cad.md MAP-19,
//! docs/adr/2420): `saha-paketi.json` out of the `.zip` `cad.field.package` wrote (docs/adr/1920): the design digest,
//! the stamp and the drawing's revision, the system, epoch and heights, and the acceptance limits given. Asked
//! (docs/adr/2820), its design too: `noktalar.csv`'s points and `cizgiler.csv`'s lines, each number as the package wrote
//! it (the shortest text that reads back the same f64), so the digest of what is read is the package's. Nothing is
//! written.

use std::path::Path;

use kentos_commands::{FIELD_PACKAGE_FILES, FIELD_PACKAGE_FORMAT, Vec2};
use kentos_formats::zip;
use serde_json::Value;

use crate::checks::{Stop, error};
use crate::codes;

fn refuse(code: &str, message: String) -> Stop {
    Stop::failed(error(code, message, Some("package".to_owned())))
}

/// What the check takes from a package.
pub(crate) struct Package {
    pub(crate) path: String,
    pub(crate) stamp: String,
    pub(crate) revision: String,
    pub(crate) digest: String,
    pub(crate) epsg: Option<String>,
    pub(crate) epoch: Option<f64>,
    pub(crate) heights: Option<String>,
    pub(crate) horizontal: Option<f64>,
    pub(crate) vertical: Option<f64>,
    pub(crate) quality: Option<f64>,
}

/// The package at `text`, refused at `package` as the contract says.
pub(crate) fn read(text: &str) -> Result<Package, Stop> {
    let path = Path::new(text);
    if text.trim().is_empty() || !path.is_absolute() {
        return Err(refuse(codes::INVALID_FILE_PATH, "Saha paketinin yolu boş ya da göreli; mutlak bir .zip yolu verin.".into()));
    }
    if !text.to_ascii_lowercase().ends_with(".zip") {
        return Err(refuse(codes::INVALID_FILE_PATH, format!("“{text}” bir .zip dosyası değil (saha paketi).")));
    }
    let name = path.file_name().map_or_else(|| text.to_owned(), |n| n.to_string_lossy().into_owned());
    let bytes = match std::fs::read(path) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(refuse(codes::FILE_NOT_FOUND, format!("“{text}” saha paketi yok; Saha paketi yaz… ile yazılan .zip'i seçin.")));
        }
        Err(e) => return Err(refuse(codes::INVALID_ASBUILT_CHECK, format!("“{name}” okunamadı: {e}."))),
    };
    let not_a_package = |why: String| refuse(codes::INVALID_ASBUILT_CHECK, format!("“{name}” bir saha paketi değil: {why}."));
    let limits = zip::Limits::default();
    let listed = zip::list(&bytes, &limits).map_err(not_a_package)?;
    let manifest = kentos_commands::FIELD_PACKAGE_FILES[0];
    let entries =
        zip::unpack(&bytes, &listed, &limits, |l| l.file_name() == manifest).map_err(not_a_package)?;
    let Some(entry) = entries.first() else {
        return Err(not_a_package(format!("içinde {manifest} yok")));
    };
    let v: Value = serde_json::from_slice(&entry.data).map_err(|e| not_a_package(format!("{manifest} okunamadı ({e})")))?;
    if v["format"] != FIELD_PACKAGE_FORMAT || v["version"] != 1 {
        return Err(not_a_package(format!("{manifest} {FIELD_PACKAGE_FORMAT} 1 biçiminde değil")));
    }
    let text_of = |v: &Value| v.as_str().map(str::to_owned);
    let Some(digest) = text_of(&v["designDigest"]).filter(|d| !d.is_empty()) else {
        return Err(not_a_package(format!("{manifest} tasarım özetini (designDigest) taşımıyor")));
    };
    let limit = |key: &str| v["acceptance"][key].as_f64().filter(|x| x.is_finite() && *x > 0.0);
    Ok(Package {
        path: text.to_owned(),
        stamp: text_of(&v["stamp"]).unwrap_or_default(),
        revision: text_of(&v["revision"]).unwrap_or_default(),
        digest,
        epsg: text_of(&v["system"]["epsg"]),
        epoch: v["epoch"].as_f64(),
        heights: text_of(&v["heights"]),
        horizontal: limit("horizontal"),
        vertical: limit("vertical"),
        quality: limit("quality"),
    })
}

/// The package's design (docs/adr/2820): `noktalar.csv` (`Ad;Y;X;Z;Kimlik`, Y east, X north; a name may hold `;`, the
/// four fields after it never do) and `cizgiler.csv` (`Kimlik;Sıra;Y;X;Bulge`, a line starting at Sıra 1, each vertex's
/// bulge the edge's to the next). Refused at `package` when a file is not there or a row does not read.
pub(crate) fn design(text: &str) -> Result<(Vec<crate::asbuilt_check::Point>, Vec<crate::asbuilt_check::Line>), Stop> {
    let name = Path::new(text).file_name().map_or_else(|| text.to_owned(), |n| n.to_string_lossy().into_owned());
    let unread = |why: String| refuse(codes::INVALID_ASBUILT_CHECK, format!("“{name}” paketinin tasarım dosyaları okunamadı: {why}."));
    let bytes = std::fs::read(text).map_err(|e| unread(e.to_string()))?;
    let limits = zip::Limits::default();
    let listed = zip::list(&bytes, &limits).map_err(unread)?;
    let wanted = [FIELD_PACKAGE_FILES[1], FIELD_PACKAGE_FILES[2]];
    let entries = zip::unpack(&bytes, &listed, &limits, |l| wanted.contains(&l.file_name())).map_err(unread)?;
    let file = |n: &str| -> Result<String, Stop> {
        let e = entries.iter().find(|e| e.name == n).ok_or_else(|| unread(format!("içinde {n} yok")))?;
        String::from_utf8(e.data.clone()).map_err(|_| unread(format!("{n} UTF-8 değil")))
    };
    let number = |t: &str, row: usize, n: &str| -> Result<f64, Stop> {
        t.trim().parse::<f64>().ok().filter(|v| v.is_finite()).ok_or_else(|| unread(format!("{n}'nin {row}. satırında “{t}” bir sayı değil")))
    };
    let mut points = Vec::new();
    let pts_file = FIELD_PACKAGE_FILES[1];
    for (k, line) in file(pts_file)?.lines().enumerate().skip(1).filter(|(_, l)| !l.trim().is_empty()) {
        let row = k + 1;
        let f: Vec<&str> = line.rsplitn(5, ';').collect();
        let [uid, z, north, east, label] = f[..] else {
            return Err(unread(format!("{pts_file}'nin {row}. satırında beş alan yok")));
        };
        points.push(crate::asbuilt_check::Point {
            uid: uid.trim().to_owned(),
            name: label.to_owned(),
            p: Vec2 { x: number(east, row, pts_file)?, y: number(north, row, pts_file)? },
            z: if z.trim().is_empty() { None } else { Some(number(z, row, pts_file)?) },
            sigma: None,
        });
    }
    let mut lines: Vec<crate::asbuilt_check::Line> = Vec::new();
    let lines_file = FIELD_PACKAGE_FILES[2];
    for (k, line) in file(lines_file)?.lines().enumerate().skip(1).filter(|(_, l)| !l.trim().is_empty()) {
        let row = k + 1;
        let f: Vec<&str> = line.split(';').collect();
        let [uid, order, east, north, bulge] = f[..] else {
            return Err(unread(format!("{lines_file}'nin {row}. satırında beş alan yok")));
        };
        let at = Vec2 { x: number(east, row, lines_file)?, y: number(north, row, lines_file)? };
        let bulge = number(bulge, row, lines_file)?;
        match (order.trim(), lines.last_mut()) {
            ("1", _) | (_, None) => lines.push(crate::asbuilt_check::Line { uid: uid.trim().to_owned(), pts: vec![at], bulges: vec![bulge] }),
            (_, Some(l)) => {
                l.pts.push(at);
                l.bulges.push(bulge);
            }
        }
    }
    // A vertex's bulge is its edge's to the next: the last vertex's is not an edge's.
    for l in &mut lines {
        l.bulges.truncate(l.pts.len().saturating_sub(1));
    }
    lines.retain(|l| l.pts.len() >= 2);
    Ok((points, lines))
}
