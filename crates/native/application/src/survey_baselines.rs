//! `cad.survey.baselines` v1 (plan/map-cad.md MAP-05, docs/adr/1380): GNSS baz ağı dengelemesi, read only. The input's
//! stations and baselines (or its text, read here line by line) go to the shared core
//! (`survey::adjust::gnss`); the answer is the core's, nothing is written. The shared cases are
//! `fixtures/commands/v1/cad.survey.baselines.json`.

use kentos_commands::{
    AdjustedGnssStation, BaselineDatum, BaselineResult, BaselineStation, BaselineVector, CommandResult, SurveyBaselines,
    SurveyBaselinesAdjusted,
};
use kentos_geometry_core::survey::adjust::gnss::{self, Baseline, Datum, GnssInput, GnssStation};

use crate::ExecutionContext;
use crate::checks::{Stop, error};
use crate::codes;

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.to_owned())))
}

/// A value of a text line: a number with a point before its decimals.
fn number(s: &str, line: usize) -> Result<f64, Stop> {
    s.trim().parse::<f64>().ok().filter(|v| v.is_finite()).ok_or_else(|| {
        fail(codes::INVALID_INPUT, format!("{line}. satırda “{}” bir sayı değil (ondalıktan önce nokta).", s.trim()), "text")
    })
}

/// The stations and baselines of a text (docs/adr/1380 §2): `K;ad;X;Y;Z[;sX;sY;sZ]`, `N;ad[;X;Y;Z]`,
/// `B;from;to;dX;dY;dZ;xx;xy;xz;yy;yz;zz`; `#` a remark; `;`, `,` or tabs between the values.
pub(crate) fn read_text(text: &str) -> Result<(Vec<BaselineStation>, Vec<BaselineVector>), Stop> {
    let mut stations = Vec::new();
    let mut baselines = Vec::new();
    for (i, raw) in text.lines().enumerate() {
        let line = i + 1;
        let t = raw.trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        let parts: Vec<&str> = t.split([';', ',', '\t']).map(str::trim).collect();
        let bad = || fail(codes::INVALID_INPUT, format!("{line}. satır okunamadı: K, N ya da B ile başlayıp değerleri tam olmalı."), "text");
        match parts[0].to_uppercase().as_str() {
            "K" if parts.len() == 5 || parts.len() == 8 => {
                let xyz = [number(parts[2], line)?, number(parts[3], line)?, number(parts[4], line)?];
                let sigma = if parts.len() == 8 {
                    Some([number(parts[5], line)?, number(parts[6], line)?, number(parts[7], line)?])
                } else {
                    None
                };
                stations.push(BaselineStation { name: parts[1].to_owned(), xyz: Some(xyz), fixed: sigma.is_none(), sigma });
            }
            "N" if parts.len() == 2 || parts.len() == 5 => {
                let xyz = if parts.len() == 5 {
                    Some([number(parts[2], line)?, number(parts[3], line)?, number(parts[4], line)?])
                } else {
                    None
                };
                stations.push(BaselineStation { name: parts[1].to_owned(), xyz, fixed: false, sigma: None });
            }
            "B" if parts.len() == 12 => {
                let mut v = [0.0; 9];
                for (k, p) in parts[3..12].iter().enumerate() {
                    v[k] = number(p, line)?;
                }
                baselines.push(BaselineVector {
                    from: parts[1].to_owned(),
                    to: parts[2].to_owned(),
                    d: [v[0], v[1], v[2]],
                    cov: [v[3], v[4], v[5], v[6], v[7], v[8]],
                });
            }
            _ => return Err(bad()),
        }
    }
    Ok((stations, baselines))
}

fn run(input: &SurveyBaselines) -> Result<SurveyBaselinesAdjusted, Stop> {
    let (stations, baselines) = match &input.text {
        Some(text) => {
            if !input.stations.is_empty() || !input.baselines.is_empty() {
                return Err(fail(
                    codes::INVALID_INPUT,
                    "Ağı ya metinle (text) ya da istasyon ve baz listeleriyle verin, ikisiyle birden değil.".into(),
                    "text",
                ));
            }
            read_text(text)?
        }
        None => (input.stations.clone(), input.baselines.clone()),
    };
    let core = GnssInput {
        stations: stations
            .iter()
            .map(|s| GnssStation { name: s.name.clone(), xyz: s.xyz, fixed: s.fixed, sigma: s.sigma })
            .collect(),
        baselines: baselines
            .iter()
            .map(|b| Baseline { from: b.from.clone(), to: b.to.clone(), d: b.d, cov: b.cov })
            .collect(),
        scale: input.scale.unwrap_or(1.0),
        datum: match input.datum {
            BaselineDatum::Given => Datum::Given,
            BaselineDatum::Minimal => Datum::Minimal,
        },
    };
    let r = gnss::adjust(&core).map_err(|e| fail(e.code, e.message, &e.path))?;
    let count = |n: usize| u32::try_from(n).unwrap_or(u32::MAX);
    // The worst flagged observation's baseline (a weighted station's is not a baseline).
    let worst = r.stats.worst.and_then(|i| (i < 3 * baselines.len()).then(|| count(i / 3)));
    Ok(SurveyBaselinesAdjusted {
        stations: r
            .stations
            .into_iter()
            .map(|s| AdjustedGnssStation { name: s.name, xyz: s.xyz, sxyz: s.sxyz, senu: s.senu, held: s.held })
            .collect(),
        baselines: r
            .baselines
            .into_iter()
            .zip(&baselines)
            .map(|(v, b)| BaselineResult {
                from: b.from.trim().to_owned(),
                to: b.to.trim().to_owned(),
                v: v.v,
                w: v.w,
                flag: v.flag.to_owned(),
            })
            .collect(),
        held: r.held,
        n: count(r.stats.n),
        u: count(r.stats.u),
        f: count(r.stats.f),
        omega: r.stats.omega,
        m0: r.stats.m0,
        chi2: r.stats.chi2,
        passed: r.stats.passed,
        worst,
        revision: String::new(),
    })
}

pub fn validate(_cx: &ExecutionContext<'_>, input: &SurveyBaselines) -> CommandResult<()> {
    match run(input) {
        Ok(_) => CommandResult::Completed { output: (), warnings: Vec::new() },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn plan(cx: &ExecutionContext<'_>, input: &SurveyBaselines) -> CommandResult<SurveyBaselinesAdjusted> {
    match run(input) {
        Ok(mut output) => {
            output.revision = cx.doc.revision().to_string();
            CommandResult::Completed { output, warnings: Vec::new() }
        }
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn execute(cx: &mut ExecutionContext<'_>, input: SurveyBaselines) -> CommandResult<SurveyBaselinesAdjusted> {
    plan(cx, &input)
}
