//! `cad.field.package` v1 (plan/map-cad.md MAP-19, docs/adr/1920): Saha paketi. The design read as `cad.asbuilt.check`
//! reads it (`asbuilt_check::{points, lines}`, its digest the same), written as one `.zip`:
//!
//! - `saha-paketi.json`: `kentos.field-package` v1, the project's name, the stamp, the drawing's revision, the design's
//!   digest, the system (`EPSG:5256` and its name; a local project none), the epoch, the heights' system in words, the
//!   acceptance limits (only those given), the notes, the surface (id, name, its sources' digest) and the files;
//! - `noktalar.csv`: `Ad;Y;X;Z;Kimlik` (Y east, X north as a surveyor reads them; each number as Rust writes an f64, the
//!   shortest that reads back the same: nothing rounded);
//! - `cizgiler.csv`: `Kimlik;Sıra;Y;X;Bulge`, each line's vertices in order (a closed polyline's first again at its end,
//!   as the check reads it).
//!
//! The file is written whole or not at all; the drawing is not changed.

use std::path::{Path, PathBuf};

use kentos_commands::{
    CommandResult, FIELD_NOTE_LENGTH, FIELD_NOTES_LIMIT, FIELD_PACKAGE_FILES, FIELD_PACKAGE_FORMAT, FieldPackage,
    FieldPackaged, PackagePoint, PackageSurface,
};
use kentos_commands::encoding::sha256_hex;
use kentos_domain::Document;
use kentos_formats::zip;
use serde_json::json;

use crate::ExecutionContext;
use crate::asbuilt_check::{self as asbuilt, NAME_FIELD};
use crate::checks::{Stop, error};
use crate::codes;
use crate::road_terrains as terrains;

fn refuse(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.to_owned())))
}

/// A number as Rust writes an f64: the shortest text that reads back the same.
fn num(v: f64) -> String {
    format!("{v}")
}

struct Checked {
    out: FieldPackaged,
    files: Vec<(String, Vec<u8>)>,
    path: PathBuf,
}

fn asked(input: &FieldPackage) -> Result<(), Stop> {
    let bad = |m: String, path: &str| Err(refuse(codes::INVALID_FIELD_PACKAGE, m, path));
    if input.design.is_none() && input.lines.is_none() {
        return bad("Pakete girecek tasarım yok: tasarım noktalarının ya da çizgilerinin katmanını verin.".into(), "design");
    }
    for (v, path, words) in [
        (input.horizontal, "horizontal", "Yatay kabul sınırı"),
        (input.vertical, "vertical", "Düşey kabul sınırı"),
        (input.quality, "quality", "Ölçü kalitesi sınırı"),
    ] {
        if let Some(v) = v
            && !(v.is_finite() && v > 0.0)
        {
            return bad(format!("{words} 0'dan büyük bir sayı olmalı (m); verilmeyecekse boş bırakın."), path);
        }
    }
    if input.field.as_deref().is_some_and(|f| f.trim().is_empty()) {
        return bad("Adların özniteliği boş; özniteliğin adını yazın (yoksa Ad).".into(), "field");
    }
    if input.notes.len() > FIELD_NOTES_LIMIT {
        return bad(format!("Pakette en çok {FIELD_NOTES_LIMIT} kalite notu olur."), "notes");
    }
    if let Some(i) = input.notes.iter().position(|n| n.chars().count() > FIELD_NOTE_LENGTH) {
        return bad(format!("Bir kalite notu en çok {FIELD_NOTE_LENGTH} harf olur."), &format!("notes[{i}]"));
    }
    if let Some(s) = &input.stamp
        && crate::road_landxml::stamp_of(s).is_none()
    {
        return bad(format!("“{s}” bir tarih ve saat değil; YYYY-MM-DDThh:mm:ss yazın."), "stamp");
    }
    Ok(())
}

fn check(doc: &Document, input: &FieldPackage) -> Result<Checked, Stop> {
    asked(input)?;
    for (layer, path) in [(&input.design, "design"), (&input.lines, "lines")] {
        if let Some(id) = layer
            && doc.layers().get(id).is_none()
        {
            return Err(refuse(codes::LAYER_NOT_FOUND, format!("“{id}” kimlikli katman çizimde yok."), path));
        }
    }
    let surface = match &input.surface {
        Some(name) => {
            terrains::terrain_project(doc)?;
            let id = crate::road_profiles::surface_id(doc, name, "surface")?;
            let def = terrains::surfaces(doc).iter().find(|s| s.id == id).cloned().ok_or(Stop::Cancelled)?;
            terrains::make(doc, &def).built()?;
            Some(PackageSurface { id: def.id.clone(), name: def.name.clone(), digest: terrains::sources_digest(doc, &def) })
        }
        None => None,
    };
    let text = input.path.trim();
    if text.is_empty() || !Path::new(text).is_absolute() {
        return Err(refuse(codes::INVALID_FILE_PATH, "Paketin yolu boş ya da göreli; mutlak bir .zip yolu verin.".into(), "path"));
    }
    if !text.to_ascii_lowercase().ends_with(".zip") {
        return Err(refuse(codes::INVALID_FILE_PATH, format!("“{text}” bir .zip dosyası değil (saha paketi)."), "path"));
    }
    let path = PathBuf::from(text);
    let folder = path.parent().map(Path::to_path_buf).unwrap_or_default();
    if !folder.is_dir() {
        return Err(refuse(
            codes::INVALID_FILE_PATH,
            format!("“{}” klasörü yok; paket var olan bir klasöre yazılır.", folder.display()),
            "path",
        ));
    }
    if path.exists() && !input.overwrite {
        return Err(refuse(codes::FILE_EXISTS, format!("{} var; üzerine yazmak için overwrite verin.", path.display()), "path"));
    }
    let field = input.field.as_deref().unwrap_or(NAME_FIELD);
    let design = input.design.as_deref().map(|l| asbuilt::points(doc, l, field, None)).unwrap_or_default();
    let lines = input.lines.as_deref().map(|l| asbuilt::lines(doc, l)).unwrap_or_default();
    let digest = asbuilt::digest(&design, &lines);
    let points: Vec<PackagePoint> =
        design.iter().map(|p| PackagePoint { name: p.name.clone(), at: [p.p.x, p.p.y], z: p.z, uid: p.uid.clone() }).collect();
    let vertices: usize = lines.iter().map(|l| l.pts.len()).sum();
    let settings = doc.settings();
    let facts = kentos_project::facts::coordinate_facts(settings);
    let system = (!facts.epsg.is_empty()).then(|| kentos_project::crs::system(settings.srid).map(|s| s.name.clone())).flatten();
    let (date, time) = input.stamp.as_deref().and_then(crate::road_landxml::stamp_of).unwrap_or_else(crate::road_landxml::now);
    let stamp = format!("{date}T{time}");
    // The files.
    let mut csv = String::from("Ad;Y;X;Z;Kimlik\n");
    for p in &points {
        csv.push_str(&format!("{};{};{};{};{}\n", p.name, num(p.at[0]), num(p.at[1]), p.z.map(num).unwrap_or_default(), p.uid));
    }
    let mut strings = String::from("Kimlik;Sıra;Y;X;Bulge\n");
    for l in &lines {
        let id = &l.uid;
        for (k, v) in l.pts.iter().enumerate() {
            let bulge = l.bulges.get(k).copied().unwrap_or(0.0);
            strings.push_str(&format!("{id};{};{};{};{}\n", k + 1, num(v.x), num(v.y), num(bulge)));
        }
    }
    let mut acceptance = serde_json::Map::new();
    for (key, v) in [("horizontal", input.horizontal), ("vertical", input.vertical), ("quality", input.quality)] {
        if let Some(v) = v {
            acceptance.insert(key.into(), json!(v));
        }
    }
    let manifest = json!({
        "format": FIELD_PACKAGE_FORMAT,
        "version": 1,
        "project": doc.name(),
        "stamp": stamp,
        "revision": doc.revision().to_string(),
        "designDigest": digest,
        "system": {"epsg": (!facts.epsg.is_empty()).then(|| facts.epsg.clone()), "name": system},
        "epoch": facts.epoch,
        "heights": (!facts.heights.is_empty()).then(|| facts.heights.clone()),
        "acceptance": acceptance,
        "notes": input.notes,
        "surface": surface,
        "points": points.len(),
        "lines": lines.len(),
        "files": FIELD_PACKAGE_FILES,
    });
    let files = vec![
        (FIELD_PACKAGE_FILES[0].to_owned(), serde_json::to_vec_pretty(&manifest).unwrap_or_default()),
        (FIELD_PACKAGE_FILES[1].to_owned(), csv.into_bytes()),
        (FIELD_PACKAGE_FILES[2].to_owned(), strings.into_bytes()),
    ];
    Ok(Checked {
        out: FieldPackaged {
            path: text.to_owned(),
            bytes: 0,
            sha256: String::new(),
            digest,
            revision: doc.revision().to_string(),
            points,
            lines: lines.len() as u32,
            vertices: vertices as u32,
            surface,
            epsg: (!facts.epsg.is_empty()).then_some(facts.epsg),
            system,
            epoch: facts.epoch,
            heights: (!facts.heights.is_empty()).then_some(facts.heights),
            horizontal: input.horizontal,
            vertical: input.vertical,
            quality: input.quality,
            notes: input.notes.clone(),
            stamp,
        },
        files,
        path,
    })
}

/// Checks `input` against the document and the file system, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &FieldPackage) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(_) => CommandResult::Completed { output: (), warnings: Vec::new() },
        Err(stop) => stop.into(),
    }
}

/// What execute would write (the file's size and digest left empty); writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &FieldPackage) -> CommandResult<FieldPackaged> {
    match check(cx.doc, input) {
        Ok(c) => CommandResult::Completed { output: c.out, warnings: Vec::new() },
        Err(stop) => stop.into(),
    }
}

/// Writes the package, whole or not at all; the drawing is not changed.
pub fn execute(cx: &mut ExecutionContext<'_>, input: FieldPackage) -> CommandResult<FieldPackaged> {
    let c = match check(cx.doc, &input) {
        Ok(c) => c,
        Err(stop) => return stop.into(),
    };
    let refs: Vec<(&str, &[u8], zip::Pack)> = c.files.iter().map(|(n, d)| (n.as_str(), d.as_slice(), zip::Pack::Fast)).collect();
    let bytes = match zip::write_with(&refs) {
        Ok(b) => b,
        Err(e) => return refuse(codes::FILE_UNWRITABLE, format!("Paket kurulamadı: {e}."), "path").into(),
    };
    // Whole or not at all: a temporary file beside it, then renamed.
    let tmp = c.path.with_extension("zip.part");
    let written = std::fs::write(&tmp, &bytes).and_then(|()| std::fs::rename(&tmp, &c.path));
    if let Err(e) = written {
        let _ = std::fs::remove_file(&tmp);
        return refuse(
            codes::FILE_UNWRITABLE,
            format!("{} yazılamadı: {e}. Klasöre yazılabildiğini denetleyin.", c.path.display()),
            "path",
        )
        .into();
    }
    CommandResult::Completed {
        output: FieldPackaged { bytes: bytes.len() as u64, sha256: sha256_hex(&bytes), ..c.out },
        warnings: Vec::new(),
    }
}
