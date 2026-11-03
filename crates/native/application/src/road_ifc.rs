//! `file.export.ifc` v1 (plan/road.md Y10.3, docs/adr/0908 §8; ROAD-IO-06): a road project's alignments and their
//! profiles written to an IFC 4.3 file (`IFC4X3_ADD2`) by `kentos-formats`' `ifc_alignment`, whole or not at all. The
//! drawing does not change. The shared cases are `fixtures/commands/v1/file.export.ifc.json`.

use std::path::{Path, PathBuf};

use kentos_commands::{CommandResult, CommandWarning, ExportIfc, IfcAlignmentWritten, IfcExported};
use kentos_domain::Document;
use kentos_formats::ifc_alignment::{Alignment, Equation, HKind, HSeg, IfcFile, VKind, VSeg, Vertical, write};
use kentos_road::alignment::element::Kind;
use kentos_road::profile::vertical::Piece;
use sha2::{Digest, Sha256};

use crate::ExecutionContext;
use crate::checks::{self, Stop};
use crate::codes;
use crate::road_alignments as alignments;
use crate::road_landxml::{equations, now, stamp_of};
use crate::road_terrains::fail;

fn bad(path: &str, words: String) -> Stop {
    fail(codes::INVALID_IFC, words, path)
}

fn losses(message: String) -> CommandWarning {
    CommandWarning { code: codes::EXPORT_LOSSES.into(), message, path: Some("alignments".into()) }
}

/// A radius from a curvature (0: a tangent, infinite).
fn radius(k: f64) -> f64 {
    if k == 0.0 { 0.0 } else { 1.0 / k }
}

/// A profile's pieces as IFC's vertical segments, and its end.
fn vertical(name: &str, pieces: &[Piece]) -> Option<Vertical> {
    let mut segments = Vec::new();
    for p in pieces {
        let (s0, s1) = p.range();
        if s1 - s0 <= 0.0 {
            continue;
        }
        let (z0, g0, _) = p.at(s0);
        let (_, g1, _) = p.at(s1);
        let (kind, r) = match *p {
            Piece::Grade { .. } => (VKind::Grade, 0.0),
            Piece::Parabola { r, .. } => (VKind::Parabola, radius(r)),
            Piece::Circle { radius: rr, sag, .. } => (VKind::Circle, if sag { rr } else { -rr }),
        };
        segments.push(VSeg { kind, dist: s0, length: s1 - s0, z0, g0, g1, radius: r });
    }
    let last = pieces.last()?;
    let (_, s1) = last.range();
    let (z, g, _) = last.at(s1);
    Some(Vertical { name: name.to_owned(), segments, end: [s1, z, g] })
}

struct Planned {
    path: PathBuf,
    out: IfcExported,
    text: String,
    warnings: Vec<CommandWarning>,
}

fn check(doc: &Document, input: &ExportIfc) -> Result<Planned, Stop> {
    alignments::road_project(doc)?;
    let text = input.path.trim();
    if text.is_empty() || !Path::new(text).is_absolute() {
        return Err(fail(codes::INVALID_FILE_PATH, "IFC'nin yolu boş ya da göreli; mutlak bir .ifc yolu verin.".into(), "path"));
    }
    if !text.to_ascii_lowercase().ends_with(".ifc") {
        return Err(fail(codes::INVALID_FILE_PATH, format!("“{text}” bir .ifc dosyası değil."), "path"));
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
    let stamp = match input.stamp.as_deref() {
        None => {
            let (d, t) = now();
            format!("{d}T{t}")
        }
        Some(s) => {
            let (d, t) = stamp_of(s)
                .ok_or_else(|| bad("stamp", format!("“{s}” bir tarih ve saat değil; YYYY-MM-DDThh:mm:ss yazın.")))?;
            format!("{d}T{t}")
        }
    };
    let mut warnings = Vec::new();
    let crs = kentos_project::systems::own(doc.settings()).map(|n| {
        let name = if n.code.starts_with("EPSG:") { n.code.clone() } else { n.name.clone() };
        (name, n.title.clone())
    });
    if crs.is_none() {
        warnings.push(losses("Projenin koordinat sistemi yok; IFC'ye harita dönüşümü (IfcMapConversion) yazılmadı.".into()));
    }
    let mut file = IfcFile {
        project: doc.name().to_owned(),
        crs: crs.clone(),
        stamp,
        application: format!("KentOS Yol {}", env!("CARGO_PKG_VERSION")),
        alignments: Vec::new(),
    };
    let mut out = Vec::new();
    for (i, asked) in chosen {
        let def = &list[i];
        if def.offset_of.is_some() {
            warnings.push(losses(format!("“{}” paralel bir aks; IFC'ye yazılmadı.", def.name)));
            continue;
        }
        let b = match alignments::build(list, def, "") {
            Ok(b) => b,
            Err(stop) if asked => {
                return Err(bad("alignments", format!("“{}” aksı kurulamıyor: {}", def.name, crate::road_corridors::words(stop))));
            }
            Err(_) => {
                warnings.push(losses(format!("“{}” aksı kurulamıyor; yazılmadı.", def.name)));
                continue;
            }
        };
        let (start_km, eqs) = equations(&b);
        if eqs.iter().any(|q| !q.increasing) {
            warnings.push(losses(format!(
                "“{}” aksının azalan km denklemi IFC'nin Pset_Stationing'inde söylenemez; km değerleri yazıldı.",
                def.name
            )));
        }
        let mut written = IfcAlignmentWritten {
            id: def.id.clone(),
            name: def.name.clone(),
            lines: 0,
            arcs: 0,
            clothoids: 0,
            referents: 1 + eqs.len() as u32,
            profiles: Vec::new(),
            vertical_segments: 0,
        };
        let horizontal: Vec<HSeg> = b
            .core
            .elements
            .iter()
            .map(|e| {
                let kind = match e.kind {
                    Kind::Tangent => {
                        written.lines += 1;
                        HKind::Line
                    }
                    Kind::Arc => {
                        written.arcs += 1;
                        HKind::Arc
                    }
                    Kind::Spiral => {
                        written.clothoids += 1;
                        HKind::Clothoid
                    }
                };
                HSeg {
                    kind,
                    start: e.start,
                    dir: e.theta0,
                    dir_end: e.theta(e.length),
                    r0: radius(e.k0),
                    r1: radius(e.k1),
                    length: e.length,
                }
            })
            .collect();
        let last = b.core.elements.last();
        let end = last.map_or([0.0, 0.0], |e| e.point(e.length));
        let end_dir = last.map_or(0.0, |e| e.theta(e.length));
        let mut verticals = Vec::new();
        if !input.without_profiles {
            for p in doc.road_profiles().iter().filter(|p| p.alignment == def.id) {
                match kentos_road::profile::vertical::build(&crate::road_profiles::pvis_of(p)).ok().and_then(|v| vertical(&p.name, &v.pieces)) {
                    Some(v) => {
                        written.vertical_segments += v.segments.len() as u32;
                        written.profiles.push(p.name.clone());
                        verticals.push(v);
                    }
                    None => warnings.push(losses(format!("“{}” boykesiti kurulamıyor; yazılmadı.", p.name))),
                }
            }
        }
        out.push(written);
        file.alignments.push(Alignment {
            name: def.name.clone(),
            horizontal,
            end,
            end_dir,
            start_km,
            equations: eqs.iter().map(|q| Equation { dist: q.internal - start_km, back: q.back, ahead: q.ahead }).collect(),
            verticals,
        });
    }
    if file.alignments.is_empty() {
        return Err(bad("", "Yazılacak bir şey yok: seçilen aks yok.".into()));
    }
    let text = write(&file);
    let sha256 = Sha256::digest(text.as_bytes()).iter().map(|b| format!("{b:02x}")).collect();
    let out = IfcExported {
        path: path.display().to_string(),
        schema: "IFC4X3_ADD2".into(),
        alignments: out,
        crs: crs.map(|c| c.0),
        bytes: text.len() as u64,
        sha256,
    };
    Ok(Planned { path, out, text, warnings })
}

/// The text written at `path` whole or not at all: beside it, read back whole, then moved over it.
pub(crate) fn write_file(path: &Path, text: &str) -> Result<(), String> {
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".kentos-yaziliyor");
    let tmp = PathBuf::from(tmp);
    let done = (|| -> Result<(), String> {
        std::fs::write(&tmp, text.as_bytes()).map_err(|e| format!("{} yazılamadı: {e}", tmp.display()))?;
        let back = std::fs::read_to_string(&tmp).map_err(|e| format!("{} geri okunamadı: {e}", tmp.display()))?;
        if back != text {
            return Err("yazılan dosya geri okununca aynı çıkmadı".into());
        }
        std::fs::rename(&tmp, path).map_err(|e| format!("{} yerine konamadı: {e}", path.display()))
    })();
    if done.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    done
}

/// Checks `input` against the document; writes nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &ExportIfc) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(p) => CommandResult::Completed { output: (), warnings: p.warnings },
        Err(stop) => stop.into(),
    }
}

/// What the file would hold; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &ExportIfc) -> CommandResult<IfcExported> {
    match check(cx.doc, input) {
        Ok(p) => CommandResult::Completed { output: p.out, warnings: p.warnings },
        Err(stop) => stop.into(),
    }
}

/// Writes the file; the drawing does not change.
pub fn execute(cx: &mut ExecutionContext<'_>, input: ExportIfc) -> CommandResult<IfcExported> {
    let p = match check(cx.doc, &input) {
        Ok(p) => p,
        Err(stop) => return stop.into(),
    };
    if let Err(why) = write_file(&p.path, &p.text) {
        return Stop::failed(checks::error(
            codes::INVALID_FILE_PATH,
            format!("IFC yazılamadı: {why}. Hiçbir şey değişmedi."),
            Some("path".into()),
        ))
        .into();
    }
    CommandResult::Completed { output: p.out, warnings: p.warnings }
}
