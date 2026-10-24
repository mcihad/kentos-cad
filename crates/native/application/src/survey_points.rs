//! `cad.survey.points` v1 (docs/adr/0070, 0071, 0622 §9): Poligon hesabı, Kutupsal alım, Önden and Geriden kestirme
//! computed with the shared core (`survey::traverse`, `survey::polar`, `survey::intersection`) and written as named
//! points through `cad.entities.create`, one undo step named after the window. Each point carries its name as its
//! label and its `Ad`, the window's kind as `Tür`, its height as z and `Z (m)`. The shared cases are
//! `fixtures/commands/v1/cad.survey.points.json` (`scripts/fixtures/survey_points_cases.py`).

use std::collections::BTreeMap;

use kentos_commands::{
    AngleUnit, CommandResult, CreateOperation, EntitiesCreate, EntitiesCreatePlan, Entity,
    EntityGeometry, KnownPoint, NewObject, ProjectSettings, SurveyPoints, SurveyPointsWritten,
    SurveyWork, TraverseShape, Vec2 as Wire,
};
use kentos_domain::Document;
use kentos_geometry_core::crs::ground::Grid;
use kentos_geometry_core::survey::Unit;
use kentos_geometry_core::survey::intersection::{forward_intersection, resection};
use kentos_geometry_core::survey::polar::{PolarInput, Shot, polar_survey};
use kentos_geometry_core::survey::traverse::{TraverseInput, traverse};
use kentos_geometry_core::vec2::Vec2;
use kentos_project::format::fixed;
use kentos_project::systems;

use crate::checks::{self, Stop, error};
use crate::{ExecutionContext, codes, create};

/// Why Uzunlukları projeksiyona indir cannot be turned on (docs/adr/0171 §4).
pub const NEEDS_HEIGHT: &str = "Ortalama elipsoit yüksekliği yazılınca açılır.";
pub const NEEDS_SYSTEM: &str =
    "Projenin koordinat sistemi yok: uzunluklar projeksiyona indirilemez.";
pub const NEEDS_SCALE: &str = "Projenin sisteminde bir noktanın tek ölçeği yok (coğrafi sistem, Pseudo-Mercator ya da afinle bağlı yerel sistem): uzunluklar projeksiyona indirilemez.";

/// The grid the survey windows take measured lengths to (docs/adr/0171 §4): the project asks for it (Uzunlukları
/// projeksiyona indir, with a height) and its system has one scale at a point; none otherwise.
pub fn survey_grid(settings: &ProjectSettings) -> Option<Grid> {
    if !settings.reduces_to_grid() || why_not_grid(settings).is_some() {
        return None;
    }
    Some(Grid {
        system: systems::own(settings)?.system?,
        height: settings.ground_height()?,
    })
}

/// Why the survey windows cannot take lengths to the grid with these settings, or none: a height, a system, and
/// one scale at a point in it.
pub fn why_not_grid(settings: &ProjectSettings) -> Option<&'static str> {
    if settings.ground_height().is_none() {
        return Some(NEEDS_HEIGHT);
    }
    match systems::own(settings).and_then(|n| n.system) {
        None => Some(NEEDS_SYSTEM),
        Some(s) if !kentos_geometry_core::crs::ground::has_point_scale(&s) => Some(NEEDS_SCALE),
        Some(_) => None,
    }
}

/// A point the survey writes.
#[derive(Clone, Debug, PartialEq)]
pub struct NewPoint {
    pub name: String,
    pub p: Vec2,
    pub z: Option<f64>,
}

/// The points as `cad.entities.create` takes them: named, of the window's kind, with their heights.
pub fn new_objects(points: &[NewPoint], kind: &str) -> Vec<NewObject> {
    points
        .iter()
        .map(|pt| {
            let mut attrs = BTreeMap::from([
                ("Ad".to_owned(), pt.name.clone()),
                ("Tür".to_owned(), kind.to_owned()),
            ]);
            if let Some(z) = pt.z {
                attrs.insert("Z (m)".to_owned(), fixed(z, 3));
            }
            NewObject {
                line_type: None,
                geometry: EntityGeometry::Point {
                    p: Wire {
                        x: pt.p.x,
                        y: pt.p.y,
                    },
                    z: pt.z,
                    parts: None,
                },
                color: None,
                line_weight: None,
                attrs: Some(attrs),
                label: Some(pt.name.clone()),
                label_of: None,
                label_scale: None,
                symbol: None,
                order: None,
                transparency: None,
            }
        })
        .collect()
}

fn refuse(code: &str, message: impl Into<String>, path: &str) -> Stop {
    Stop::failed(error(code, message.into(), Some(path.into())))
}

/// Big letters the Turkish way (i İ, ı I), for names compared alike.
fn upper(s: &str) -> String {
    s.chars()
        .flat_map(|c| match c {
            'i' => vec!['İ'],
            'ı' => vec!['I'],
            c => c.to_uppercase().collect(),
        })
        .collect()
}

/// A known point's place: a named point of the drawing (its label, else its `Ad`), or the coordinates given.
fn known(doc: &Document, k: &KnownPoint, path: &str) -> Result<Vec2, Stop> {
    match k {
        KnownPoint::At { at } => {
            checks::point(*at, "Bilinen noktanın", &format!("{path}.at"))?;
            Ok(Vec2::new(at.x, at.y))
        }
        KnownPoint::Named { name } => {
            let key = upper(name.trim());
            doc.entities()
                .find_map(|e| match e {
                    Entity::Point(pt) => {
                        let own = pt
                            .base
                            .label
                            .as_deref()
                            .or_else(|| pt.base.attrs.get("Ad").map(String::as_str))
                            .unwrap_or("");
                        (!key.is_empty() && upper(own) == key).then(|| Vec2::new(pt.p.x, pt.p.y))
                    }
                    _ => None,
                })
                .ok_or_else(|| {
                    refuse(
                        codes::POINT_NOT_FOUND,
                        format!(
                            "“{}” adlı nokta çizimde yok. Adını denetleyin ya da koordinatlarını verin.",
                            name.trim()
                        ),
                        &format!("{path}.name"),
                    )
                })
        }
    }
}

fn unit_name(doc: &Document, input: &SurveyPoints) -> &'static str {
    match input.unit.unwrap_or(doc.settings().angle_unit) {
        AngleUnit::Deg => "deg",
        AngleUnit::Grad => "grad",
    }
}

/// What a computation writes.
struct Laid {
    points: Vec<NewPoint>,
    kind: &'static str,
    operation: CreateOperation,
    angle_misclosure: Option<f64>,
    linear_misclosure: Option<f64>,
}

fn refused(e: String) -> Stop {
    refuse(codes::SURVEY_REFUSED, e, "work")
}

fn finite_all(values: &[f64], what: &str, path: &str) -> Result<(), Stop> {
    for (i, v) in values.iter().enumerate() {
        checks::finite(*v, what, "Sonlu bir sayı verin.", &format!("{path}[{i}]"))?;
    }
    Ok(())
}

fn lay(doc: &Document, input: &SurveyPoints) -> Result<Laid, Stop> {
    checks::revision(doc, input.expected_revision.as_deref())?;
    let unit = unit_name(doc, input);
    let settings = doc.settings();
    match &input.work {
        SurveyWork::Traverse {
            shape,
            start,
            back,
            end,
            fore,
            angles,
            distances,
            names,
        } => {
            finite_all(angles, "Açı", "work.angles")?;
            finite_all(distances, "Uzunluk", "work.distances")?;
            let start = known(doc, start, "work.start")?;
            let back = known(doc, back, "work.back")?;
            let (end, fore) = match shape {
                TraverseShape::Closed => (Some(start), Some(back)),
                TraverseShape::Open => (None, None),
                TraverseShape::Connected => {
                    let Some(end) = end else {
                        return Err(refuse(
                            codes::SURVEY_REFUSED,
                            "Bağlı poligonun bitiş noktasını verin.",
                            "work.end",
                        ));
                    };
                    let end = known(doc, end, "work.end")?;
                    let fore = fore
                        .as_ref()
                        .map(|f| known(doc, f, "work.fore"))
                        .transpose()?;
                    (Some(end), fore)
                }
            };
            let r = traverse(&TraverseInput {
                unit: unit.to_owned(),
                start,
                back,
                end,
                fore,
                angles: angles.clone(),
                distances: distances.clone(),
                grid: survey_grid(settings),
            })
            .map_err(refused)?;
            let points = r
                .points
                .iter()
                .enumerate()
                .map(|(i, &p)| NewPoint {
                    name: names
                        .get(i)
                        .map(|n| n.trim())
                        .filter(|n| !n.is_empty())
                        .map_or_else(|| format!("P{}", i + 1), str::to_owned),
                    p,
                    z: None,
                })
                .collect();
            Ok(Laid {
                points,
                kind: "Poligon noktası",
                operation: CreateOperation::Traverse,
                angle_misclosure: r.angle_misclosure,
                linear_misclosure: r.linear_misclosure,
            })
        }
        SurveyWork::Polar {
            station,
            back,
            back_reading,
            station_z,
            instrument_height,
            shots,
        } => {
            checks::finite(
                *back_reading,
                "Bakılan noktanın okuması",
                "Sonlu bir sayı verin.",
                "work.backReading",
            )?;
            if shots.is_empty() {
                return Err(refuse(
                    codes::SURVEY_REFUSED,
                    "En az bir alım verin.",
                    "work.shots",
                ));
            }
            let station = known(doc, station, "work.station")?;
            let back = known(doc, back, "work.back")?;
            let points = polar_survey(&PolarInput {
                unit: unit.to_owned(),
                station,
                back,
                back_reading: *back_reading,
                station_z: *station_z,
                instrument_height: *instrument_height,
                shots: shots
                    .iter()
                    .map(|s| Shot {
                        reading: s.reading,
                        distance: s.distance,
                        zenith: s.zenith,
                        target_height: s.target_height,
                    })
                    .collect(),
                refraction: Some(settings.refraction()),
                grid: survey_grid(settings),
            })
            .map_err(refused)?;
            Ok(Laid {
                points: points
                    .iter()
                    .zip(shots)
                    .map(|(p, s)| NewPoint {
                        name: s.name.trim().to_owned(),
                        p: p.p,
                        z: p.z,
                    })
                    .collect(),
                kind: "Alım noktası",
                operation: CreateOperation::PolarSurvey,
                angle_misclosure: None,
                linear_misclosure: None,
            })
        }
        SurveyWork::Forward {
            a,
            b,
            alpha,
            beta,
            name,
        } => {
            let a = known(doc, a, "work.a")?;
            let b = known(doc, b, "work.b")?;
            let unit = Unit::parse(unit).map_err(refused)?;
            let p = forward_intersection(unit, a, b, *alpha, *beta).map_err(refused)?;
            Ok(Laid {
                points: vec![NewPoint {
                    name: name.trim().to_owned(),
                    p,
                    z: None,
                }],
                kind: "Kestirme noktası",
                operation: CreateOperation::ForwardIntersection,
                angle_misclosure: None,
                linear_misclosure: None,
            })
        }
        SurveyWork::Resection {
            a,
            b,
            c,
            alpha,
            beta,
            name,
        } => {
            let a = known(doc, a, "work.a")?;
            let b = known(doc, b, "work.b")?;
            let c = known(doc, c, "work.c")?;
            let unit = Unit::parse(unit).map_err(refused)?;
            let r = resection(unit, a, b, c, *alpha, *beta).map_err(refused)?;
            Ok(Laid {
                points: vec![NewPoint {
                    name: name.trim().to_owned(),
                    p: r.p,
                    z: None,
                }],
                kind: "Kestirme noktası",
                operation: CreateOperation::Resection,
                angle_misclosure: None,
                linear_misclosure: None,
            })
        }
    }
}

fn create_input(laid: &Laid, input: &SurveyPoints) -> EntitiesCreate {
    EntitiesCreate {
        layer_id: input.layer_id.clone(),
        objects: new_objects(&laid.points, laid.kind),
        operation: Some(laid.operation),
        expected_revision: input.expected_revision.clone(),
    }
}

/// The computation's checks and `cad.entities.create`'s.
pub fn validate(cx: &ExecutionContext<'_>, input: &SurveyPoints) -> CommandResult<()> {
    match lay(cx.doc, input) {
        Ok(laid) => create::validate(cx, &create_input(&laid, input)),
        Err(stop) => stop.into(),
    }
}

/// The points as they would be written.
pub fn plan(cx: &ExecutionContext<'_>, input: &SurveyPoints) -> CommandResult<EntitiesCreatePlan> {
    match lay(cx.doc, input) {
        Ok(laid) => create::plan(cx, &create_input(&laid, input)),
        Err(stop) => stop.into(),
    }
}

/// Computes and writes the points, one undo step named after the window.
pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: SurveyPoints,
) -> CommandResult<SurveyPointsWritten> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_SURVEY_POINTS,
        kentos_commands::CAD_SURVEY_POINTS_VERSION,
        input,
        write_execute,
    )
}

fn write_execute(
    cx: &mut ExecutionContext<'_>,
    input: SurveyPoints,
) -> CommandResult<SurveyPointsWritten> {
    let laid = match lay(cx.doc, &input) {
        Ok(laid) => laid,
        Err(stop) => return stop.into(),
    };
    match create::write_in(cx, create_input(&laid, &input)) {
        CommandResult::Completed { output, warnings } => CommandResult::Completed {
            output: SurveyPointsWritten {
                ids: output.ids,
                uids: output.created,
                angle_misclosure: laid.angle_misclosure,
                linear_misclosure: laid.linear_misclosure,
                revision: output.revision,
            },
            warnings,
        },
        other => crate::trim::relay(other),
    }
}
