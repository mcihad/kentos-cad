//! `file.export.landxml` v1 (plan/road.md Y10.2, docs/adr/0908 §6; ROAD-IO-05): a road project's terrain surfaces,
//! alignments and their profiles written to a LandXML 1.2 file, whole or not at all (written beside it, read back as
//! the reader reads it, then moved over it). The drawing does not change. The shared cases are
//! `fixtures/commands/v1/file.export.landxml.json`.

use std::path::{Path, PathBuf};

use kentos_commands::{
    CommandResult, CommandWarning, ExportLandxml, LandxmlAlignmentWritten, LandxmlExported, LandxmlSurfaceWritten,
};
use kentos_domain::Document;
use kentos_formats::landxml_write::{
    WAlignment, WCurve, WElement, WEquation, WGeom, WLandXml, WProfile, WPvi, WSurface, write,
};
use kentos_road::alignment::element::{Element, Kind};
use kentos_road::profile::vertical::VCurve;
use sha2::{Digest, Sha256};

use crate::ExecutionContext;
use crate::checks::{self, Stop};
use crate::codes;
use crate::road_alignments::{self as alignments, Built};
use crate::road_terrains::{self as terrains, fail};

fn bad(path: &str, words: String) -> Stop {
    fail(codes::INVALID_LANDXML, words, path)
}

fn losses(message: String, path: &str) -> CommandWarning {
    CommandWarning { code: codes::EXPORT_LOSSES.into(), message, path: Some(path.into()) }
}

/// An element's LandXML geometry (docs/adr/0908 §6): a curve's centre on its inner side and its PI where the
/// tangents meet (none from half a circle on); a clothoid's PI where its end tangents meet (its middle point where
/// they are parallel).
pub fn element_geometry(e: &Element) -> WGeom {
    let end = e.point(e.length);
    let (c0, s0) = (e.theta0.cos(), e.theta0.sin());
    match e.kind {
        Kind::Tangent => WGeom::Line { start: e.start, end },
        Kind::Arc => {
            let k = e.k0;
            let r = 1.0 / k.abs();
            let center = [e.start[0] - s0 / k, e.start[1] + c0 / k];
            let delta = k.abs() * e.length;
            let pi = (delta < std::f64::consts::PI - 1e-9).then(|| {
                let t = r * (0.5 * delta).tan();
                [e.start[0] + t * c0, e.start[1] + t * s0]
            });
            WGeom::Curve { start: e.start, end, center, pi, radius: r, clockwise: k < 0.0 }
        }
        Kind::Spiral => {
            let th1 = e.theta(e.length);
            let (c1, s1) = (th1.cos(), th1.sin());
            let cross = c0 * s1 - s0 * c1;
            let pi = if cross.abs() < 1e-12 {
                e.point(0.5 * e.length)
            } else {
                let (dx, dy) = (end[0] - e.start[0], end[1] - e.start[1]);
                let t = (dx * s1 - dy * c1) / cross;
                [e.start[0] + t * c0, e.start[1] + t * s0]
            };
            let radius = |k: f64| (k != 0.0).then(|| 1.0 / k.abs());
            WGeom::Spiral {
                start: e.start,
                end,
                pi,
                radius_start: radius(e.k0),
                radius_end: radius(e.k1),
                clockwise: e.k0 + e.k1 < 0.0,
            }
        }
    }
}

/// The km equations of a built alignment, as LandXML's `StaEquation`s, and its km at the start.
fn equations(b: &Built) -> (f64, Vec<WEquation>) {
    let start = b.regions.first().map_or(0.0, |r| r.ahead);
    let mut out = Vec::new();
    for w in b.regions.windows(2) {
        let (p, r) = (w[0], w[1]);
        out.push(WEquation {
            internal: start + r.at,
            back: p.ahead + p.dir * (r.at - p.at),
            ahead: r.ahead,
            increasing: r.dir > 0.0,
        });
    }
    (start, out)
}

/// The `$`-less stamp, `YYYY-MM-DDThh:mm:ss`, as the file's date and time; none where it does not read.
fn stamp_of(text: &str) -> Option<(String, String)> {
    let (d, t) = text.split_once('T')?;
    let ok_date = d.len() == 10 && d.as_bytes()[4] == b'-' && d.as_bytes()[7] == b'-';
    let ok_time = t.len() == 8 && t.as_bytes()[2] == b':' && t.as_bytes()[5] == b':';
    let digits = |s: &str| s.chars().filter(|c| *c != '-' && *c != ':').all(|c| c.is_ascii_digit());
    (ok_date && ok_time && digits(d) && digits(t)).then(|| (d.to_owned(), t.to_owned()))
}

/// Now, UTC, as a stamp.
fn now() -> (String, String) {
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs()) as i64;
    let (y, m, d) = kentos_geometry_core::time::civil_from_days(secs.div_euclid(86_400));
    let s = secs.rem_euclid(86_400);
    (format!("{y:04}-{m:02}-{d:02}"), format!("{:02}:{:02}:{:02}", s / 3600, s / 60 % 60, s % 60))
}

struct Planned {
    path: PathBuf,
    file: WLandXml,
    out: LandxmlExported,
    text: String,
    warnings: Vec<CommandWarning>,
}

fn check(doc: &Document, input: &ExportLandxml) -> Result<Planned, Stop> {
    alignments::road_project(doc)?;
    let text = input.path.trim();
    if text.is_empty() || !Path::new(text).is_absolute() {
        return Err(fail(codes::INVALID_FILE_PATH, "LandXML'in yolu boş ya da göreli; mutlak bir .xml yolu verin.".into(), "path"));
    }
    if !text.to_ascii_lowercase().ends_with(".xml") {
        return Err(fail(codes::INVALID_FILE_PATH, format!("“{text}” bir .xml dosyası değil."), "path"));
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
    let mut warnings = Vec::new();
    // The surfaces chosen (or every one); a chosen one must build.
    let own = terrains::surfaces(doc);
    let picked: Vec<(usize, bool)> = match &input.surfaces {
        None => (0..own.len()).map(|i| (i, false)).collect(),
        Some(names) => {
            let mut out = Vec::new();
            for (i, n) in names.iter().enumerate() {
                let at = own
                    .iter()
                    .position(|s| s.id == *n)
                    .or_else(|| own.iter().position(|s| s.name.to_lowercase() == n.to_lowercase()))
                    .ok_or_else(|| {
                        fail(codes::UNKNOWN_TERRAIN, format!("“{n}” kimlikli ya da adlı yüzey projede yok."), &format!("surfaces[{i}]"))
                    })?;
                if !out.iter().any(|(j, _)| *j == at) {
                    out.push((at, true));
                }
            }
            out
        }
    };
    let list = alignments::alignments(doc);
    let chosen: Vec<(usize, bool)> = match &input.alignments {
        None => (0..list.len()).map(|i| (i, false)).collect(),
        Some(names) => {
            let mut out = Vec::new();
            for (i, n) in names.iter().enumerate() {
                let (at, _) = alignments::find(list, n, &format!("alignments[{i}]"))?;
                if !out.iter().any(|(j, _)| *j == at) {
                    out.push((at, true));
                }
            }
            out
        }
    };
    let (date, time) = match input.stamp.as_deref() {
        None => now(),
        Some(s) => stamp_of(s).ok_or_else(|| {
            bad("stamp", format!("“{s}” bir tarih ve saat değil; YYYY-MM-DDThh:mm:ss yazın."))
        })?,
    };
    let mut file = WLandXml {
        date,
        time,
        application: "KentOS Yol".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        project: doc.name().to_owned(),
        ..WLandXml::default()
    };
    let mut out = LandxmlExported { path: path.display().to_string(), surfaces: Vec::new(), alignments: Vec::new(), bytes: 0, sha256: String::new() };
    for (i, asked) in picked {
        let def = &own[i];
        let made = terrains::make(doc, def);
        let s = match made.built() {
            Ok(s) => s,
            Err(stop) if asked => {
                return Err(bad("surfaces", format!("“{}” yüzeyi kurulamıyor: {}", def.name, crate::road_corridors::words(stop))));
            }
            Err(_) => {
                warnings.push(losses(format!("“{}” yüzeyi kurulamıyor; yazılmadı.", def.name), "surfaces"));
                continue;
            }
        };
        out.surfaces.push(LandxmlSurfaceWritten {
            id: def.id.clone(),
            name: def.name.clone(),
            points: s.vertices.len() as u32,
            triangles: s.triangles.len() as u32,
        });
        file.surfaces.push(WSurface { name: def.name.clone(), vertices: s.vertices.clone(), triangles: s.triangles.clone() });
    }
    let names: Vec<&str> = chosen.iter().map(|(i, _)| list[*i].id.as_str()).collect();
    for (i, asked) in chosen {
        let def = &list[i];
        if def.offset_of.is_some() {
            warnings.push(losses(
                format!("“{}” paralel bir aks; LandXML'in paralel aks türü yok, yazılmadı.", def.name),
                "alignments",
            ));
            continue;
        }
        let b = match alignments::build(list, def, "") {
            Ok(b) => b,
            Err(stop) if asked => {
                return Err(bad("alignments", format!("“{}” aksı kurulamıyor: {}", def.name, crate::road_corridors::words(stop))));
            }
            Err(_) => {
                warnings.push(losses(format!("“{}” aksı kurulamıyor; yazılmadı.", def.name), "alignments"));
                continue;
            }
        };
        let (sta_start, eqs) = equations(&b);
        let mut written = LandxmlAlignmentWritten {
            id: def.id.clone(),
            name: def.name.clone(),
            length: b.core.length,
            lines: 0,
            curves: 0,
            spirals: 0,
            equations: eqs.len() as u32,
            profiles: Vec::new(),
        };
        let mut elements = Vec::new();
        for e in &b.core.elements {
            match e.kind {
                Kind::Tangent => written.lines += 1,
                Kind::Arc => written.curves += 1,
                Kind::Spiral => written.spirals += 1,
            }
            elements.push(WElement {
                geom: element_geometry(e),
                station: sta_start + e.station,
                length: e.length,
                dir_start: e.theta0,
                dir_end: e.theta(e.length),
            });
        }
        let mut profiles = Vec::new();
        if !input.without_profiles {
            for p in doc.road_profiles().iter().filter(|p| p.alignment == def.id) {
                let Ok(v) = kentos_road::profile::vertical::build(&crate::road_profiles::pvis_of(p)) else {
                    warnings.push(losses(format!("“{}” boykesiti kurulamıyor; yazılmadı.", p.name), "alignments"));
                    continue;
                };
                let pvis = p
                    .pvis
                    .iter()
                    .map(|q| {
                        let found = v.curves.iter().find(|c| c.pvi == q.id);
                        let curve = match found.map(|c| (c.curve, c.end[0] - c.start[0])) {
                            Some((VCurve::Circle { radius }, length)) => WCurve::Circle { length, radius },
                            Some((VCurve::Parabola { length }, _)) => WCurve::Parabola { length },
                            Some((VCurve::Asymmetric { length_in, length_out }, _)) => {
                                WCurve::Unsymmetric { length_in, length_out }
                            }
                            _ => WCurve::None,
                        };
                        WPvi { station: sta_start + q.station, elevation: q.elevation, curve }
                    })
                    .collect();
                written.profiles.push(p.name.clone());
                profiles.push(WProfile { name: p.name.clone(), pvis });
            }
        }
        out.alignments.push(written);
        file.alignments.push(WAlignment {
            name: def.name.clone(),
            sta_start,
            length: b.core.length,
            elements,
            equations: eqs,
            profiles,
        });
    }
    if !input.without_profiles {
        for p in doc.road_profiles().iter().filter(|p| !names.contains(&p.alignment.as_str())) {
            if input.alignments.is_some() {
                continue;
            }
            warnings.push(losses(
                format!("“{}” boykesitinin aksı yazılmadı; boykesit de yazılmadı.", p.name),
                "alignments",
            ));
        }
    }
    if file.surfaces.is_empty() && file.alignments.is_empty() {
        return Err(bad("", "Yazılacak bir şey yok: seçilen yüzey ya da aks yok.".into()));
    }
    let text = write(&file);
    out.bytes = text.len() as u64;
    out.sha256 = Sha256::digest(text.as_bytes()).iter().map(|b| format!("{b:02x}")).collect();
    Ok(Planned { path, file, out, text, warnings })
}

/// The text written at `path` whole or not at all: beside it, read back by the LandXML reader (the same surfaces),
/// then moved over it.
fn write_file(path: &Path, text: &str, file: &WLandXml) -> Result<(), String> {
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".kentos-yaziliyor");
    let tmp = PathBuf::from(tmp);
    let done = (|| -> Result<(), String> {
        std::fs::write(&tmp, text.as_bytes()).map_err(|e| format!("{} yazılamadı: {e}", tmp.display()))?;
        let back = std::fs::read(&tmp).map_err(|e| format!("{} geri okunamadı: {e}", tmp.display()))?;
        let read = kentos_formats::landxml::read(&back).map_err(|e| e.words().to_owned())?;
        let same = read.surfaces.len() == file.surfaces.len()
            && read.surfaces.iter().zip(&file.surfaces).all(|(r, w)| r.points.len() == w.vertices.len() && r.faces.len() == w.triangles.len());
        if !same {
            return Err("yazılan dosya geri okununca aynı yüzeyleri vermedi".into());
        }
        std::fs::rename(&tmp, path).map_err(|e| format!("{} yerine konamadı: {e}", path.display()))
    })();
    if done.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    done
}

/// Checks `input` against the document; writes nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &ExportLandxml) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(p) => CommandResult::Completed { output: (), warnings: p.warnings },
        Err(stop) => stop.into(),
    }
}

/// What the file would hold; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &ExportLandxml) -> CommandResult<LandxmlExported> {
    match check(cx.doc, input) {
        Ok(p) => CommandResult::Completed { output: p.out, warnings: p.warnings },
        Err(stop) => stop.into(),
    }
}

/// Writes the file; the drawing does not change.
pub fn execute(cx: &mut ExecutionContext<'_>, input: ExportLandxml) -> CommandResult<LandxmlExported> {
    let p = match check(cx.doc, &input) {
        Ok(p) => p,
        Err(stop) => return stop.into(),
    };
    if let Err(why) = write_file(&p.path, &p.text, &p.file) {
        return Stop::failed(checks::error(
            codes::INVALID_FILE_PATH,
            format!("LandXML yazılamadı: {why}. Hiçbir şey değişmedi."),
            Some("path".into()),
        ))
        .into();
    }
    CommandResult::Completed { output: p.out, warnings: p.warnings }
}

/// The file's text for a window or a test.
pub fn text(doc: &Document, input: &ExportLandxml) -> Result<(String, LandxmlExported, Vec<CommandWarning>), String> {
    check(doc, input).map(|p| (p.text, p.out, p.warnings)).map_err(crate::road_corridors::words)
}
