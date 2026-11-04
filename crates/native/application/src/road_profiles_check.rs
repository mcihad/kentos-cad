//! `road.profiles.check` v1 (plan/road.md Y21, docs/adr/0922 §2): a profile checked against the user's design rules
//! (`kentos_road::profile::checks`). Reads only. The shared cases are `fixtures/commands/v1/road.profiles.check.json`.

use kentos_commands::{CommandResult, CommandWarning, DesignFinding, DesignRule, RoadProfilesCheck, RoadProfilesChecked};
use kentos_domain::Document;
use kentos_road::profile::checks::{self, ProfileRules, Rule};

use crate::ExecutionContext;
use crate::checks::Stop;
use crate::codes;
use crate::road_alignments_check::leasts;
use crate::road_profiles as common;
use crate::road_terrains::fail;

fn words(rule: Rule, of: &[String], value: f64, limit: f64) -> String {
    let (a, b) = (of.first().map_or("", String::as_str), of.get(1).map_or("", String::as_str));
    match rule {
        Rule::Grade => format!("{a}–{b} eğimi %{:.3}, en çok %{:.3}.", 100.0 * value, 100.0 * limit),
        Rule::LeastGrade => format!("{a}–{b} eğimi %{:.3}, en az %{:.3}.", 100.0 * value, 100.0 * limit),
        Rule::CrestK => format!("{a} tepe kurbunun K'si {value:.3}, en az {limit}."),
        Rule::SagK => format!("{a} çukur kurbunun K'si {value:.3}, en az {limit}."),
        Rule::Curve => format!("{a} düşey kurbu {value:.3} m, en az {limit} m."),
        Rule::GradeLength => format!("{a} ile {b} kurpları arasındaki düz eğim {value:.3} m, en az {limit} m."),
    }
}

fn rule_of(rule: Rule) -> DesignRule {
    match rule {
        Rule::Grade => DesignRule::Grade,
        Rule::LeastGrade => DesignRule::LeastGrade,
        Rule::CrestK => DesignRule::CrestK,
        Rule::SagK => DesignRule::SagK,
        Rule::Curve => DesignRule::Curve,
        Rule::GradeLength => DesignRule::GradeLength,
    }
}

/// The check now, and its warnings.
pub(crate) fn answer(doc: &Document, input: &RoadProfilesCheck) -> Result<(RoadProfilesChecked, Vec<CommandWarning>), Stop> {
    common::road_project(doc)?;
    let (_, def) = common::find(common::profiles(doc), &input.profile, "profile")?;
    let rules = ProfileRules {
        most_grade: input.most_grade,
        least_grade: input.least_grade,
        least_crest_k: input.least_crest_k,
        least_sag_k: input.least_sag_k,
        least_curve: input.least_curve,
        least_grade_length: input.least_grade_length,
    };
    let given = [
        (rules.most_grade, "mostGrade"),
        (rules.least_grade, "leastGrade"),
        (rules.least_crest_k, "leastCrestK"),
        (rules.least_sag_k, "leastSagK"),
        (rules.least_curve, "leastCurve"),
        (rules.least_grade_length, "leastGradeLength"),
    ];
    if given.iter().all(|g| g.0.is_none()) {
        return Err(fail(
            codes::INVALID_DESIGN_CHECK,
            "Denetlenecek bir kural verin: tasarım standardının değerleri kullanıcınındır.".into(),
            "profile",
        ));
    }
    leasts(&given)?;
    let built = common::build(doc, def)?;
    let findings: Vec<DesignFinding> = checks::check(&built.vertical, &common::pvis_of(def), &rules)
        .into_iter()
        .map(|f| DesignFinding {
            rule: rule_of(f.rule),
            message: words(f.rule, &f.pvis, f.value, f.limit),
            of: f.pvis,
            from: f.from,
            to: f.to,
            value: f.value,
            limit: Some(f.limit),
        })
        .collect();
    let mut warnings = Vec::new();
    let report = common::report(doc, def, &built);
    if report.stale {
        warnings.push(CommandWarning {
            code: codes::PROFILE_STALE.into(),
            message: format!(
                "“{}” boykesitinin aksı değişti (boykesit {}. revizyona göre kurulu, aks şimdi {}.); PVI'ları denetleyip aksa yeniden bağlayın.",
                report.name, report.alignment_revision, report.alignment_now
            ),
            path: Some("profile".into()),
        });
    }
    if !findings.is_empty() {
        warnings.push(CommandWarning {
            code: codes::DESIGN_RULE_BROKEN.into(),
            message: format!("“{}” boykesitinde {} yer verilen tasarım kurallarına uymuyor.", def.name, findings.len()),
            path: Some("profile".into()),
        });
    }
    Ok((RoadProfilesChecked { profile: def.id.clone(), findings }, warnings))
}

/// Checks `input` against the document.
pub fn validate(cx: &ExecutionContext<'_>, input: &RoadProfilesCheck) -> CommandResult<()> {
    match answer(cx.doc, input) {
        Ok((_, warnings)) => CommandResult::Completed { output: (), warnings },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn plan(cx: &ExecutionContext<'_>, input: &RoadProfilesCheck) -> CommandResult<RoadProfilesChecked> {
    match answer(cx.doc, input) {
        Ok((output, warnings)) => CommandResult::Completed { output, warnings },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn execute(cx: &mut ExecutionContext<'_>, input: RoadProfilesCheck) -> CommandResult<RoadProfilesChecked> {
    plan(cx, &input)
}

/// The check for a window, or why it does not read.
pub fn preview(doc: &Document, input: &RoadProfilesCheck) -> Result<(RoadProfilesChecked, Vec<CommandWarning>), String> {
    answer(doc, input).map_err(crate::road_corridors::words)
}
