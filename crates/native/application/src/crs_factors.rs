//! `cad.crs.factors` v1 (plan/map-cad.md MAP-02, docs/adr/1231): at given points of the project's map, the projection's
//! point scale, the height's factor R/(R + h) along the east (Euler's radius at azimuth 90°, the prime vertical's N) and
//! their product. Read only; the computation is geometry-core's (`crs::ground`), the one the survey windows and the
//! desktop's Ölçeklerin özeti use. The shared cases are `fixtures/commands/v1/cad.crs.factors.json`.

use kentos_commands::{
    CommandResult, CommandWarning, CrsFactors, CrsFactorsRead, MOST_FACTOR_POINTS, PointFactors, SurveySettings,
};
use kentos_domain::Document;
use kentos_geometry_core::Vec2;
use kentos_geometry_core::crs::ground::{Grid, grid_factor, has_point_scale, point_scale};

use crate::ExecutionContext;
use crate::checks::{Stop, error};
use crate::codes;

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.to_owned())))
}

fn read(doc: &Document, input: &CrsFactors) -> Result<(CrsFactorsRead, Vec<CommandWarning>), Stop> {
    if input.points.is_empty() || input.points.len() > MOST_FACTOR_POINTS {
        return Err(fail(
            codes::INVALID_INPUT,
            format!("1 ile {MOST_FACTOR_POINTS} arasında nokta verin; {} verildi.", input.points.len()),
            "points",
        ));
    }
    if let Some(i) = input.points.iter().position(|p| !(p[0].is_finite() && p[1].is_finite())) {
        return Err(fail(codes::INVALID_INPUT, "Noktanın koordinatı sonlu bir sayı değil.".into(), &format!("points[{i}]")));
    }
    if let Some(h) = input.height.filter(|h| !SurveySettings::ground_height_holds(*h)) {
        return Err(fail(
            codes::INVALID_INPUT,
            format!("Yükseklik {h} m olamaz; −500 ile 9000 m arasında bir ortalama elipsoit yüksekliği verin."),
            "height",
        ));
    }
    let settings = doc.settings();
    let Some(named) = kentos_project::systems::own(settings) else {
        return Err(fail(
            codes::NO_SYSTEM,
            "Proje kendi koordinatlarında (yerel); projeksiyon ölçeği yok. Projeye bir koordinat sistemi atayın.".into(),
            "points",
        ));
    };
    let Some(system) = named.system.filter(has_point_scale) else {
        return Err(fail(
            codes::NO_POINT_SCALE,
            format!(
                "{}: bu sistemde bir noktanın tek ölçeği yok (coğrafi sistem, Pseudo-Mercator ya da afinle bağlı yerel sistem).",
                named.title
            ),
            "points",
        ));
    };
    let height = input.height.or(settings.ground_height());
    let mut warnings = Vec::new();
    let mut points = Vec::with_capacity(input.points.len());
    for (i, p) in input.points.iter().enumerate() {
        let at = Vec2::new(p[0], p[1]);
        let scale = point_scale(&system, at);
        // Along the east: Euler's radius at azimuth 90° is the prime vertical's.
        let factors = height.and_then(|h| {
            grid_factor(&Grid { system: system.clone(), height: h }, at, Vec2::new(at.x + 1.0, at.y)).ok()
        });
        if scale.is_none() {
            warnings.push(CommandWarning {
                code: codes::POINT_BEYOND_SYSTEM.into(),
                message: format!("{}. nokta {} sisteminin ulaştığı yerin dışında; çarpanları verilmedi.", i + 1, named.title),
                path: Some(format!("points[{i}]")),
            });
        }
        let height_factor = scale.and(factors.map(|f| f.height_factor));
        points.push(PointFactors {
            scale,
            height_factor,
            combined: scale.zip(height_factor).map(|(k, h)| k * h),
        });
    }
    let out = CrsFactorsRead {
        system: named.title,
        height,
        reduces_to_grid: settings.reduces_to_grid(),
        points,
        revision: doc.revision().to_string(),
    };
    Ok((out, warnings))
}

pub fn validate(cx: &ExecutionContext<'_>, input: &CrsFactors) -> CommandResult<()> {
    match read(cx.doc, input) {
        Ok((_, warnings)) => CommandResult::Completed { output: (), warnings },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn plan(cx: &ExecutionContext<'_>, input: &CrsFactors) -> CommandResult<CrsFactorsRead> {
    match read(cx.doc, input) {
        Ok((output, warnings)) => CommandResult::Completed { output, warnings },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn execute(cx: &mut ExecutionContext<'_>, input: CrsFactors) -> CommandResult<CrsFactorsRead> {
    plan(cx, &input)
}
