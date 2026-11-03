//! `road.platforms.query` v1 (plan/road.md Y14, docs/adr/0912): every graded pad read now, or one; with `balance` the
//! offset that balances one pad's cut and fill, not written. Reads only. The shared cases are
//! `fixtures/commands/v1/road.platforms.query.json`.

use kentos_commands::{CommandResult, CommandWarning, PlatformBalanced, PlatformsQueried, PlatformsQuery};
use kentos_road::grading::{self, BalanceError};

use crate::ExecutionContext;
use crate::checks::Stop;
use crate::codes;
use crate::road_platforms as common;
use crate::road_terrains::fail;

fn answer(doc: &kentos_domain::Document, input: &PlatformsQuery) -> Result<(PlatformsQueried, Vec<CommandWarning>), Stop> {
    common::road_project(doc)?;
    let own = doc.road_platforms();
    let Some(name) = input.platform.as_deref() else {
        if input.balance.is_some() {
            return Err(fail(codes::INVALID_PLATFORM, "Dengeleme tek platform içindir: platform verin.".into(), "balance"));
        }
        let mut warnings = Vec::new();
        let mut platforms = Vec::new();
        for p in own {
            match common::read(doc, p) {
                Ok(r) => {
                    warnings.extend(r.warnings);
                    platforms.push(r.report);
                }
                Err(stop) => platforms.push(common::broken(p, crate::road_corridors::words(stop))),
            }
        }
        return Ok((PlatformsQueried { platforms, balance: None }, warnings));
    };
    let (_, def) = common::find(own, name, "platform")?;
    let r = common::read(doc, def)?;
    let mut warnings = r.warnings;
    let mut balance = None;
    if let Some(b) = &input.balance {
        if !(b.low.is_finite() && b.high.is_finite() && b.low < b.high && b.tolerance.is_finite() && b.tolerance > 0.0) {
            return Err(fail(
                codes::INVALID_PLATFORM,
                "Dengeleme aralığının alt ucu üst ucundan küçük, tolerans 0'dan büyük olmalı.".into(),
                "balance",
            ));
        }
        let terrain = common::terrain_of(doc, def)?;
        match grading::balance(&common::platform_of(def), &terrain, b.low, b.high, b.tolerance) {
            Ok(found) => {
                balance = Some(PlatformBalanced {
                    offset: found.offset,
                    elevation: def.base[2] + found.offset,
                    net: found.net,
                    iterations: found.iterations,
                    converged: found.converged,
                });
            }
            Err(e) => {
                let words = match e {
                    BalanceError::NoSignChange { low, high } => format!(
                        "{} ile {} m arasında kazı ile dolgu eşitlenmiyor (kazı − dolgu uçlarda {low:.3} ve {high:.3} m³); aralığı genişletin.",
                        b.low, b.high
                    ),
                    BalanceError::NotGraded { offset, why } => {
                        format!("{offset} m ötelemede platform bütünüyle tesviye edilmiyor: {why}")
                    }
                    BalanceError::Grading(g) => g.to_string(),
                    BalanceError::BadBracket => "Dengeleme aralığı geçersiz.".to_owned(),
                };
                warnings.push(CommandWarning {
                    code: codes::PLATFORM_NO_BALANCE.into(),
                    message: format!("“{}” platformu dengelenemedi: {words}", def.name),
                    path: Some("balance".into()),
                });
            }
        }
    }
    Ok((PlatformsQueried { platforms: vec![r.report], balance }, warnings))
}

/// Checks `input` against the document.
pub fn validate(cx: &ExecutionContext<'_>, input: &PlatformsQuery) -> CommandResult<()> {
    match answer(cx.doc, input) {
        Ok((_, warnings)) => CommandResult::Completed { output: (), warnings },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn plan(cx: &ExecutionContext<'_>, input: &PlatformsQuery) -> CommandResult<PlatformsQueried> {
    match answer(cx.doc, input) {
        Ok((output, warnings)) => CommandResult::Completed { output, warnings },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn execute(cx: &mut ExecutionContext<'_>, input: PlatformsQuery) -> CommandResult<PlatformsQueried> {
    plan(cx, &input)
}

/// The pads for a window, or why they do not read.
pub fn preview(doc: &kentos_domain::Document, input: &PlatformsQuery) -> Result<(PlatformsQueried, Vec<CommandWarning>), String> {
    answer(doc, input).map_err(crate::road_corridors::words)
}
