//! The field package a returned record was made with, as `cad.asbuilt.check` reads it (plan/map-cad.md MAP-19,
//! docs/adr/2420): `saha-paketi.json` out of the `.zip` `cad.field.package` wrote (docs/adr/1920): the design digest,
//! the stamp and the drawing's revision, the system, epoch and heights, and the acceptance limits given. Nothing else of
//! the package is read; nothing is written.

use std::path::Path;

use kentos_commands::FIELD_PACKAGE_FORMAT;
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
