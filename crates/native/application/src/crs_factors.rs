//! `cad.crs.factors` v1 (plan/map-cad.md MAP-02, docs/adr/1231): at given points of the project's map, the projection's
//! point scale, the height's factor R/(R + h) along the east (Euler's radius at azimuth 90°, the prime vertical's N) and
//! their product. Each point may have its own height, an ellipsoidal height or the drawing's height made ellipsoidal by
//! the project's heights (`survey_heights`), and lines between the points get the factors a length measured along them
//! passes: the ones Kutupsal alım reduces its shots with (docs/adr/2900). Read only; the computation is geometry-core's
//! (`crs::ground`), the one the survey windows and the desktop's Ölçeklerin özeti use. The shared cases are
//! `fixtures/commands/v1/cad.crs.factors.json`.

use kentos_commands::{
    CommandResult, CommandWarning, CrsFactors, CrsFactorsRead, LineFactors, MOST_FACTOR_LINES, MOST_FACTOR_POINTS,
    PointFactors, SurveySettings,
};
use kentos_domain::Document;
use kentos_geometry_core::Vec2;
use kentos_geometry_core::crs::ground::{Grid, grid_factor, has_point_scale, line_factors, point_scale};

use crate::ExecutionContext;
use crate::checks::{Stop, error};
use crate::codes;
use crate::survey_heights::{self, Unmade};

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.to_owned())))
}

/// The input's own checks, before the project's system is looked at.
fn check(input: &CrsFactors) -> Result<(), Stop> {
    let n = input.points.len();
    if n == 0 || n > MOST_FACTOR_POINTS {
        return Err(fail(
            codes::INVALID_INPUT,
            format!("1 ile {MOST_FACTOR_POINTS} arasında nokta verin; {n} verildi."),
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
    let ways = [input.height.is_some(), input.heights.is_some(), input.elevations.is_some()];
    if ways.iter().filter(|w| **w).count() > 1 {
        return Err(fail(
            codes::INVALID_INPUT,
            "Yüksekliği bir yoldan verin: height (bütün noktalara bir yükseklik), heights (noktaların elipsoit yükseklikleri) ya da elevations (çizimin kotları).".into(),
            if input.elevations.is_some() { "elevations" } else { "heights" },
        ));
    }
    if let Some(hs) = &input.heights {
        if hs.len() != n {
            return Err(fail(
                codes::INVALID_INPUT,
                format!("Her nokta için bir yükseklik verin: {n} nokta, {} yükseklik.", hs.len()),
                "heights",
            ));
        }
        if let Some(i) = hs.iter().position(|h| !SurveySettings::ground_height_holds(*h)) {
            return Err(fail(
                codes::INVALID_INPUT,
                format!("Yükseklik {} m olamaz; −500 ile 9000 m arasında bir elipsoit yüksekliği verin.", hs[i]),
                &format!("heights[{i}]"),
            ));
        }
    }
    if let Some(zs) = &input.elevations {
        if zs.len() != n {
            return Err(fail(
                codes::INVALID_INPUT,
                format!("Her nokta için bir kot verin: {n} nokta, {} kot.", zs.len()),
                "elevations",
            ));
        }
        if let Some(i) = zs.iter().position(|z| !z.is_finite()) {
            return Err(fail(codes::INVALID_INPUT, "Kot sonlu bir sayı değil.".into(), &format!("elevations[{i}]")));
        }
    }
    if let Some(ls) = &input.lines {
        if ls.len() > MOST_FACTOR_LINES {
            return Err(fail(
                codes::INVALID_INPUT,
                format!("En çok {MOST_FACTOR_LINES} çizgi verin; {} verildi.", ls.len()),
                "lines",
            ));
        }
        for (k, ends) in ls.iter().enumerate() {
            if let Some(e) = ends.iter().find(|e| **e as usize >= n) {
                return Err(fail(
                    codes::INVALID_INPUT,
                    format!("Çizginin ucu {e} bir nokta değil: {n} nokta verildi, sıraları 0'dan {}'e.", n - 1),
                    &format!("lines[{k}]"),
                ));
            }
            if input.points[ends[0] as usize] == input.points[ends[1] as usize] {
                return Err(fail(codes::INVALID_INPUT, "Çizginin iki ucu aynı yerde; uzunluğu yok.".into(), &format!("lines[{k}]")));
            }
        }
    }
    Ok(())
}

fn read(doc: &Document, input: &CrsFactors) -> Result<(CrsFactorsRead, Vec<CommandWarning>), Stop> {
    check(input)?;
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
    let at: Vec<Vec2> = input.points.iter().map(|p| Vec2::new(p[0], p[1])).collect();
    let mut warnings = Vec::new();
    // Each point's own ellipsoidal height (docs/adr/2900 §1–2), and the undulation it took.
    let mut geoid = None;
    let mut undulations = vec![None; at.len()];
    let own: Option<Vec<Option<f64>>> = match (&input.heights, &input.elevations) {
        (Some(hs), _) => Some(hs.iter().map(|h| Some(*h)).collect()),
        (_, Some(zs)) => {
            let (ns, model) = survey_heights::undulations(settings, &at).map_err(|why| {
                let code = match why {
                    Unmade::Unknown => codes::HEIGHTS_UNKNOWN,
                    Unmade::NoModel => codes::GEOID_REQUIRED,
                    Unmade::Missing(_) => codes::GEOID_MISSING,
                    Unmade::NotMetres(_) | Unmade::NoSystem => codes::INVALID_INPUT,
                };
                fail(code, why.words(), "elevations")
            })?;
            let mut hs = Vec::with_capacity(zs.len());
            for (i, (z, n)) in zs.iter().zip(&ns).enumerate() {
                let Some(n) = n else {
                    warnings.push(CommandWarning {
                        code: codes::POINTS_OUTSIDE.into(),
                        message: format!(
                            "{}. nokta “{}” geoit modelinin ızgarasının dışında; kotu elipsoit yüksekliğine çevrilmedi, yükseklik çarpanı verilmedi.",
                            i + 1,
                            model.as_deref().unwrap_or("?")
                        ),
                        path: Some(format!("elevations[{i}]")),
                    });
                    hs.push(None);
                    continue;
                };
                let h = z + n;
                if !SurveySettings::ground_height_holds(h) {
                    return Err(fail(
                        codes::INVALID_INPUT,
                        format!("Kotun elipsoit yüksekliği {h} m çıkıyor; −500 ile 9000 m arasında olmalı."),
                        &format!("elevations[{i}]"),
                    ));
                }
                if model.is_some() {
                    undulations[i] = Some(*n);
                }
                hs.push(Some(h));
            }
            geoid = model;
            Some(hs)
        }
        _ => None,
    };
    let height = match own {
        Some(_) => None,
        None => input.height.or(settings.ground_height()),
    };
    let height_of = |i: usize| match &own {
        Some(hs) => hs[i],
        None => height,
    };
    let mut points = Vec::with_capacity(at.len());
    for (i, &p) in at.iter().enumerate() {
        let scale = point_scale(&system, p);
        let h = height_of(i);
        // Along the east: Euler's radius at azimuth 90° is the prime vertical's.
        let factors =
            h.and_then(|h| grid_factor(&Grid { system: system.clone(), height: h }, p, Vec2::new(p.x + 1.0, p.y)).ok());
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
            height: own.as_ref().and_then(|hs| hs[i]),
            undulation: undulations[i],
        });
    }
    // Each line at its ends' mean height (or the one height), as Kutupsal alım reduces a shot (docs/adr/2900 §1).
    let lines = input
        .lines
        .iter()
        .flatten()
        .map(|&[a, b]| {
            let (a, b) = (a as usize, b as usize);
            let h = match &own {
                Some(hs) => hs[a].zip(hs[b]).map(|(x, y)| (x + y) / 2.0),
                None => height,
            };
            match line_factors(&system, at[a], at[b], h) {
                Ok(f) => {
                    let height_factor = f.scale.and(f.height_factor);
                    LineFactors {
                        scale: f.scale,
                        height_factor,
                        combined: f.scale.zip(height_factor).map(|(k, x)| k * x),
                        height: height_factor.and(h),
                    }
                }
                Err(_) => LineFactors { scale: None, height_factor: None, combined: None, height: None },
            }
        })
        .collect();
    let out = CrsFactorsRead {
        system: named.title,
        height,
        reduces_to_grid: settings.reduces_to_grid(),
        points,
        lines,
        geoid,
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
