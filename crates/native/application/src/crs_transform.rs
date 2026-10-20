//! `cad.crs.transform` v1 (docs/adr/0519 §3): points between coordinate systems with their heights, and how. Each
//! side is a system of the registry or the project's own with what its heights are and its epoch; the horizontal
//! way is the geometry core's (`crs::operation`), recorded with its steps, a PROJ pipeline, the grids, the area of use
//! and the stated accuracy. Heights go only where KentOS has an engine for them: ellipsoidal heights ride the datum
//! shift, orthometric heights of one vertical system stay (or change unit); between ellipsoidal and orthometric
//! heights, between two vertical datums and between two epochs the call is refused by name, never done by a
//! weaker way. The report keeps the source's uncertainty, the transformation's accuracy, the topology tolerance
//! and the shown decimals apart. It writes nothing. The shared cases are
//! `fixtures/commands/v1/cad.crs.transform.json` (`scripts/fixtures/crs_vertical_cases.py`, PROJ through pyproj).

use kentos_commands::{
    AccuracyReport, AreaOfUse, CRS_ENGINE, CommandResult, CommandWarning, CrsOperation, CrsPoint,
    CrsSide, CrsTransform, CrsTransformed, GridUse, HeightKind, HeightMethod, HeightReference,
    MAX_CRS_POINTS, OperationStep, ProjectSettings, SourceUncertainty, StepKind, UnreachedPoint,
};
use kentos_domain::Document;
use kentos_geometry_core::crs::operation::{self as op, Operation};
use kentos_geometry_core::crs::{self as core, Unreached};
use kentos_geometry_core::vec2::Vec2;

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.to_owned())))
}

/// Checks `input`.
pub fn validate(cx: &ExecutionContext<'_>, input: &CrsTransform) -> CommandResult<()> {
    match prepare(cx.doc.settings(), input) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// The points; the same answer as [`execute`].
pub fn plan(cx: &ExecutionContext<'_>, input: &CrsTransform) -> CommandResult<CrsTransformed> {
    answer(cx.doc, input)
}

/// The points. Writes nothing.
pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: CrsTransform,
) -> CommandResult<CrsTransformed> {
    answer(cx.doc, &input)
}

/// A side as the transformation reads it.
struct Side {
    system: core::System,
    title: String,
    /// None: not said.
    heights: Option<HeightReference>,
    epoch: Option<f64>,
}

fn side(settings: &ProjectSettings, given: Option<&CrsSide>, path: &str) -> Result<Side, Stop> {
    let given = given.cloned().unwrap_or_default();
    let (system, title, own) = match given.srid {
        Some(srid) => {
            let found = kentos_project::crs::system(srid)
                .filter(|s| !s.is_local())
                .and_then(|s| {
                    s.transform_system()
                        .map(|t| (t, kentos_project::crs::title(s)))
                });
            let Some((system, title)) = found else {
                return Err(fail(
                    codes::UNKNOWN_SRID,
                    format!(
                        "EPSG:{srid} koordinat sistemleri kaydında yok ya da bir yere bağlı değil. Kayıttaki bir sistemin kodunu verin ya da kodu bırakın (projenin sistemi)."
                    ),
                    &format!("{path}.srid"),
                ));
            };
            (system, title, false)
        }
        None => {
            let Some(named) = kentos_project::systems::own(settings) else {
                return Err(fail(
                    codes::NO_SYSTEM,
                    format!(
                        "{path}: projenin koordinat sistemi yok (yerel proje); sistemi EPSG koduyla verin (srid)."
                    ),
                    path,
                ));
            };
            let Some(system) = named.system else {
                return Err(fail(
                    codes::NO_SYSTEM,
                    format!(
                        "{path}: projenin sistemi “{}” dönüşümle okunamıyor (tabanı kayıtta olmayan yerel sistem).",
                        named.name
                    ),
                    path,
                ));
            };
            (system, named.title, true)
        }
    };
    let heights = match given.heights {
        Some(h) => Some(h),
        None if own => settings.heights.clone(),
        None => None,
    };
    if let Some((at, why)) = heights.as_ref().and_then(|h| h.problem()) {
        return Err(fail(
            codes::INVALID_HEIGHTS,
            format!("{path} yüksekliği: {why}."),
            &format!("{path}.heights.{at}"),
        ));
    }
    let epoch = match given.epoch {
        Some(e) => Some(e),
        None if own => settings.coordinate_epoch,
        None => None,
    };
    if let Some(why) = epoch.and_then(kentos_commands::epoch_problem) {
        return Err(fail(
            codes::INVALID_EPOCH,
            format!("{path} epoku: {why}."),
            &format!("{path}.epoch"),
        ));
    }
    Ok(Side {
        system,
        title,
        heights: heights.filter(|h| h.kind != HeightKind::Unknown),
        epoch,
    })
}

/// How heights go between two sides, or why they cannot.
enum Heights {
    None,
    Unchanged,
    Same,
    /// Times this.
    Unit(f64, &'static str, &'static str),
    Ellipsoidal,
}

fn heights(from: &Side, to: &Side, any: bool) -> Result<Heights, Stop> {
    if !any {
        return Ok(Heights::None);
    }
    let (a, b) = match (&from.heights, &to.heights) {
        (None, None) => return Ok(Heights::Unchanged),
        (Some(a), Some(b)) => (a, b),
        (Some(_), None) | (None, Some(_)) => {
            let unknown = if from.heights.is_none() { "from" } else { "to" };
            return Err(fail(
                codes::HEIGHTS_UNKNOWN,
                format!(
                    "Kotlar verildi; {unknown} tarafının yüksekliği bilinmiyor, öbürününki biliniyor. Kotlar bir varsayımla çevrilmez: iki tarafın da yüksekliğini (heights) söyleyin ya da kotları vermeyin."
                ),
                &format!("{unknown}.heights"),
            ));
        }
    };
    match (a.kind, b.kind) {
        (HeightKind::Ellipsoidal, HeightKind::Ellipsoidal) => Ok(Heights::Ellipsoidal),
        (HeightKind::Orthometric, HeightKind::Orthometric) => {
            if a.vertical_srid == b.vertical_srid {
                return Ok(Heights::Same);
            }
            match (a.vertical(), b.vertical()) {
                (Some(va), Some(vb)) if va.datum == vb.datum => {
                    Ok(Heights::Unit(va.metres / vb.metres, va.unit, vb.unit))
                }
                _ => Err(fail(
                    codes::VERTICAL_UNSUPPORTED,
                    format!(
                        "Kotlar “{}” ile “{}” arasında çevrilmeli: iki ayrı düşey datum. KentOS'ta düşey datum dönüşümü yok; PROJ'un yaptığı gibi kotları olduğu gibi bırakan “kaba” bir yol da kullanılmaz. Kotları başka bir yazılımla çevirip verin ya da aynı düşey sistemi seçin.",
                        a.title(),
                        b.title()
                    ),
                    "to.heights.verticalSrid",
                )),
            }
        }
        _ => {
            let geoid = [a, b]
                .iter()
                .find_map(|h| h.geoid.as_ref())
                .map(|g| format!(" (projede adı geçen geoit: {})", g.name))
                .unwrap_or_default();
            Err(fail(
                codes::GEOID_REQUIRED,
                format!(
                    "Kotlar {} ile {} arasında çevrilmeli: bunun için geoit modeli gerekir{geoid}. KentOS'ta geoit ızgarası okuyan motor yok; kotlar sıfır geoit ayrımıyla çevrilmez. Kotları geoit modeliyle başka bir yazılımda çevirip verin.",
                    a.title(),
                    b.title()
                ),
                "to.heights.kind",
            ))
        }
    }
}

struct Prepared {
    from: Side,
    to: Side,
    heights: Heights,
    operation: Operation,
    from_epoch: Option<f64>,
    to_epoch: Option<f64>,
    uncertainty: SourceUncertainty,
}

fn prepare(settings: &ProjectSettings, input: &CrsTransform) -> Result<Prepared, Stop> {
    if input.points.is_empty() || input.points.len() > MAX_CRS_POINTS {
        return Err(fail(
            codes::NOTHING_TO_TRANSFORM,
            format!(
                "{} nokta verildi; en az 1, en çok 100 000 nokta dönüştürülür.",
                input.points.len()
            ),
            "points",
        ));
    }
    for (i, p) in input.points.iter().enumerate() {
        checks::point(
            kentos_commands::Vec2 { x: p.x, y: p.y },
            &format!("{}. noktanın", i + 1),
            &format!("points[{i}]"),
        )?;
        if let Some(z) = p.z {
            checks::finite(
                z,
                &format!("{}. noktanın kotu", i + 1),
                "Kotu sonlu bir sayıyla verin ya da bırakın.",
                &format!("points[{i}].z"),
            )?;
        }
    }
    let from = side(settings, input.from.as_ref(), "from")?;
    let to = side(settings, input.to.as_ref(), "to")?;
    let uncertainty = input.uncertainty.unwrap_or_default();
    for (v, at) in [
        (uncertainty.horizontal, "uncertainty.horizontal"),
        (uncertainty.vertical, "uncertainty.vertical"),
    ] {
        if v.is_some_and(|v| !(v.is_finite() && v >= 0.0)) {
            return Err(fail(
                codes::INVALID_UNCERTAINTY,
                "Kaynağın belirsizliği 0 ya da büyük sonlu bir sayı olmalı (metre).".into(),
                at,
            ));
        }
    }
    let heights = heights(&from, &to, input.points.iter().any(|p| p.z.is_some()))?;
    let (from_epoch, to_epoch) = match (from.epoch, to.epoch) {
        (Some(a), Some(b)) if a != b => {
            return Err(fail(
                codes::EPOCH_UNSUPPORTED,
                format!(
                    "Koordinatlar {a} epokundan {b} epokuna taşınmalı: bunun için levha hareketi (hız modeli) gerekir. KentOS'ta zamana bağlı dönüşüm yok; koordinatlar epok farkı yok sayılarak çevrilmez."
                ),
                "to.epoch",
            ));
        }
        (a, b) => (a, b.or(a)),
    };
    let keep = !matches!(heights, Heights::Ellipsoidal);
    let mut operation =
        op::operation(&from.system, &to.system, &kentos_project::systems::choices(settings), keep)
            .map_err(|u| match u {
                Unreached::NoGrid => fail(
                    codes::GRID_MISSING,
                    "Projenin bu iki datum için seçtiği ızgara bu cihazda yok. Izgarayı Proje ayarları › Izgaralar'a ekleyin; EPSG'nin daha düşük doğruluklu yoluna kendiliğinden geçilmez."
                        .into(),
                    "to",
                ),
                _ => fail(
                    codes::NO_DATUM_LINK,
                    "İki sistemin datumları arasında yol yok: projenin datumunun WGS 84'e yedi parametresi tanımlı değil."
                        .into(),
                    "to",
                ),
            })?;
    if let Heights::Unit(_, a, b) = heights {
        operation
            .pipeline
            .push_str(&format!(" +step +proj=unitconvert +z_in={a} +z_out={b}"));
    }
    Ok(Prepared {
        from,
        to,
        heights,
        operation,
        from_epoch,
        to_epoch,
        uncertainty,
    })
}

fn side_title(s: &Side) -> String {
    let mut t = s.title.clone();
    if let Some(h) = &s.heights {
        t.push_str(" + ");
        t.push_str(&h.title());
    }
    if let Some(e) = s.epoch {
        t.push_str(&format!(" @ {e}"));
    }
    t
}

fn rss(a: Option<f64>, b: Option<f64>) -> Option<f64> {
    a.zip(b).map(|(a, b)| {
        let r = (a * a + b * b).sqrt();
        (r * 1000.0).round() / 1000.0
    })
}

/// [`execute`] over the document.
pub fn answer(doc: &Document, input: &CrsTransform) -> CommandResult<CrsTransformed> {
    let settings = doc.settings();
    let p = match prepare(settings, input) {
        Ok(p) => p,
        Err(stop) => return stop.into(),
    };
    let choices = kentos_project::systems::choices(settings);
    let mut points = Vec::with_capacity(input.points.len());
    let mut unreached = Vec::new();
    for (i, q) in input.points.iter().enumerate() {
        let at = Vec2::new(q.x, q.y);
        let moved = match (&p.heights, q.z) {
            (Heights::Ellipsoidal, Some(z)) => {
                op::transform_height_in(&p.from.system, &p.to.system, at, z, &choices)
                    .map(|(v, h)| (v, Some(h)))
            }
            (how, z) => core::transform_in(&p.from.system, &p.to.system, at, &choices).map(|t| {
                let z = match how {
                    Heights::Unit(k, _, _) => z.map(|z| z * k),
                    _ => z,
                };
                (t.point, z)
            }),
        };
        match moved {
            Ok((v, z)) => points.push(Some(CrsPoint { x: v.x, y: v.y, z })),
            Err(why) => {
                points.push(None);
                unreached.push(UnreachedPoint {
                    index: u32::try_from(i).unwrap_or(u32::MAX),
                    reason: why.as_str().into(),
                });
            }
        }
    }
    let op = &p.operation;
    let (method, height_accuracy) = match p.heights {
        Heights::None => (HeightMethod::None, None),
        Heights::Unchanged => (HeightMethod::Unchanged, None),
        Heights::Same => (HeightMethod::Same, Some(0.0)),
        Heights::Unit(..) => (HeightMethod::Unit, Some(0.0)),
        Heights::Ellipsoidal => (HeightMethod::Ellipsoidal, op.accuracy),
    };
    let mut steps: Vec<OperationStep> = op
        .steps
        .iter()
        .map(|s| OperationStep {
            kind: match s.kind {
                op::StepKind::Conversion => StepKind::Conversion,
                op::StepKind::Helmert => StepKind::Helmert,
                op::StepKind::Null => StepKind::Null,
                op::StepKind::Grid => StepKind::Grid,
            },
            name: s.name.clone(),
            reverse: s.reverse,
            accuracy: s.accuracy,
        })
        .collect();
    if let Heights::Unit(..) = p.heights {
        let code = |s: &Side| {
            s.heights
                .as_ref()
                .and_then(|h| h.vertical_srid)
                .unwrap_or_default()
        };
        steps.push(OperationStep {
            kind: StepKind::Unit,
            name: format!("EPSG:{} → EPSG:{}", code(&p.from), code(&p.to)),
            reverse: false,
            accuracy: Some(0.0),
        });
    }
    let grids = op
        .grids
        .iter()
        .map(|id| GridUse {
            name: settings
                .datum_transforms
                .iter()
                .filter_map(|t| t.grid.as_ref())
                .find(|g| g.id == *id)
                .map_or_else(|| id.clone(), |g| g.file.clone()),
            sha256: id.clone(),
        })
        .collect();
    let mut warnings = Vec::new();
    if matches!(p.heights, Heights::Unchanged) {
        warnings.push(CommandWarning {
            code: codes::HEIGHTS_UNCHANGED.into(),
            message: "Kotların ne olduğu iki tarafta da söylenmedi; kotlar değişmeden kaldı. Elipsoit yüksekliğiyse datum değişince değişirdi: yüksekliği söyleyin (heights)."
                .into(),
            path: Some("points".into()),
        });
    }
    if p.from_epoch.is_some() || p.to_epoch.is_some() {
        warnings.push(CommandWarning {
            code: codes::EPOCH_NOT_APPLIED.into(),
            message: "Epok kayıtta; seçilen işlem zamana bağlı değil, koordinatlara uygulanmadı."
                .into(),
            path: Some("from.epoch".into()),
        });
    }
    if op.unofficial {
        warnings.push(CommandWarning {
            code: codes::UNOFFICIAL_OPERATION.into(),
            message: format!(
                "{} EPSG'nin ED50 işlemi: değerleri resmî dönüşümün değil (Tapu ve Kadastro'nun bölge parametreleri için Proje ayarları › Datum dönüşümleri).",
                op.id
            ),
            path: Some("to".into()),
        });
    }
    if !unreached.is_empty() {
        warnings.push(CommandWarning {
            code: codes::POINTS_OUTSIDE.into(),
            message: format!(
                "{} noktanın hedef sistemde değeri yok (izdüşümün ya da ızgaranın dışında); yerleri boş.",
                unreached.len()
            ),
            path: Some("points".into()),
        });
    }
    let decimals = settings.length_decimals;
    let report = AccuracyReport {
        source: p.uncertainty,
        transformation: op.accuracy,
        transformation_vertical: height_accuracy,
        combined: rss(p.uncertainty.horizontal, op.accuracy),
        combined_vertical: rss(p.uncertainty.vertical, height_accuracy),
        topology_tolerance: settings
            .topology
            .as_ref()
            .map_or(kentos_commands::TOPOLOGY_TOLERANCE, |t| t.tolerance()),
        display_decimals: decimals,
        display_step: 10f64.powi(-i32::try_from(decimals).unwrap_or(0)),
    };
    CommandResult::Completed {
        output: CrsTransformed {
            points,
            unreached,
            operation: CrsOperation {
                from: side_title(&p.from),
                to: side_title(&p.to),
                id: op.id.clone(),
                steps,
                pipeline: op.pipeline.clone(),
                engine: CRS_ENGINE.into(),
                grids,
                area: op.area.map(|(name, [w, s, e, n])| AreaOfUse {
                    name: name.into(),
                    west: w,
                    south: s,
                    east: e,
                    north: n,
                }),
                accuracy: op.accuracy,
                heights: method,
                height_accuracy,
                from_epoch: p.from_epoch,
                to_epoch: p.to_epoch,
                epoch_applied: false,
                unofficial: op.unofficial,
            },
            report,
        },
        warnings,
    }
}
