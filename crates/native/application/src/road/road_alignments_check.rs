//! `road.alignments.check` v1 (plan/road.md Y21, docs/adr/0922 §1): an alignment checked against the user's design
//! rules (`kentos_road::alignment::checks`). Reads only. The shared cases are
//! `fixtures/commands/v1/road.alignments.check.json`.

use kentos_commands::{AlignmentsCheck, AlignmentsChecked, CommandResult, CommandWarning, CurvePair, DesignFinding, DesignRule};
use kentos_domain::Document;
use kentos_road::alignment::chain::MEET;
use kentos_road::alignment::checks::{self, AlignmentRules, Rule};

use crate::ExecutionContext;
use crate::checks::Stop;
use crate::codes;
use crate::road_alignments as alignments;
use crate::road_terrains::fail;

fn invalid(message: String, path: &str) -> Stop {
    fail(codes::INVALID_DESIGN_CHECK, message, path)
}

/// The least values given checked: each above 0.
pub(crate) fn leasts(given: &[(Option<f64>, &str)]) -> Result<(), Stop> {
    for (v, path) in given {
        if v.is_some_and(|v| !(v.is_finite() && v > 0.0)) {
            return Err(invalid(format!("{path} 0'dan büyük olmalı."), path));
        }
    }
    Ok(())
}

fn words(rule: Rule, of: &[String], value: f64, limit: f64) -> String {
    let (a, b) = (of.first().map_or("", String::as_str), of.get(1).map_or("", String::as_str));
    match rule {
        Rule::Radius => format!("{a} kurbunun yarıçapı {value:.3} m, en az {limit} m."),
        Rule::Arc => format!("{a} kurbunun daire boyu {value:.3} m, en az {limit} m."),
        Rule::Spiral => format!("{a} kurbunun klotoidi {value:.3} m, en az {limit} m."),
        Rule::Tangent => format!("{a} ile {b} arasındaki aliyman {value:.3} m, en az {limit} m."),
        Rule::Reverse => format!("{a} ile {b} ters dönüyor; aradaki aliyman {value:.3} m, en az {limit} m."),
        Rule::BrokenBack => format!("{a} ile {b} aynı yöne dönüyor; aradaki aliyman {value:.3} m, en az {limit} m."),
        Rule::RadiusRatio => format!("{a} ile {b} bitişik; yarıçapların oranı {value:.3}, en çok {limit}."),
        Rule::Kink => format!("{a} PI'sinde yön {value:.4}° dönüyor, kurp yok."),
    }
}

fn rule_of(rule: Rule) -> DesignRule {
    match rule {
        Rule::Radius => DesignRule::Radius,
        Rule::Arc => DesignRule::Arc,
        Rule::Spiral => DesignRule::Spiral,
        Rule::Tangent => DesignRule::Tangent,
        Rule::Reverse => DesignRule::Reverse,
        Rule::BrokenBack => DesignRule::BrokenBack,
        Rule::RadiusRatio => DesignRule::RadiusRatio,
        Rule::Kink => DesignRule::Kink,
    }
}

/// The check now, and its warnings.
pub(crate) fn answer(doc: &Document, input: &AlignmentsCheck) -> Result<(AlignmentsChecked, Vec<CommandWarning>), Stop> {
    alignments::road_project(doc)?;
    let list = alignments::alignments(doc);
    let (_, def) = alignments::find(list, &input.alignment, "alignment")?;
    if def.offset_of.is_some() {
        return Err(invalid(format!("“{}” paralel aks; kendi PI'si yok, kaynağını denetleyin.", def.name), "alignment"));
    }
    let rules = AlignmentRules {
        least_radius: input.least_radius,
        least_arc: input.least_arc,
        least_spiral: input.least_spiral,
        least_tangent: input.least_tangent,
        least_reverse: input.least_reverse,
        least_broken_back: input.least_broken_back,
        most_radius_ratio: input.most_radius_ratio,
        kinks: input.kinks,
    };
    let given = [
        (rules.least_radius, "leastRadius"),
        (rules.least_arc, "leastArc"),
        (rules.least_spiral, "leastSpiral"),
        (rules.least_tangent, "leastTangent"),
        (rules.least_reverse, "leastReverse"),
        (rules.least_broken_back, "leastBrokenBack"),
    ];
    if given.iter().all(|g| g.0.is_none()) && rules.most_radius_ratio.is_none() && !rules.kinks {
        return Err(invalid("Denetlenecek bir kural verin: tasarım standardının değerleri kullanıcınındır.".into(), "alignment"));
    }
    leasts(&given)?;
    if rules.most_radius_ratio.is_some_and(|r| !(r.is_finite() && r >= 1.0)) {
        return Err(invalid("mostRadiusRatio 1'den küçük olamaz: büyük yarıçap küçüğe bölünür.".into(), "mostRadiusRatio"));
    }
    let built = alignments::build(list, def, "")?;
    let found = checks::check(&built.core, &alignments::pis_of(def), &rules);
    let findings: Vec<DesignFinding> = found
        .into_iter()
        .map(|f| DesignFinding {
            rule: rule_of(f.rule),
            message: words(f.rule, &f.pis, f.value, f.limit),
            of: f.pis,
            from: f.from,
            to: f.to,
            value: f.value,
            limit: (f.rule != Rule::Kink).then_some(f.limit),
        })
        .collect();
    let pairs = checks::pairs(&built.core)
        .into_iter()
        .map(|p| CurvePair { joined: p.tangent < MEET, between: p.between, reverse: p.reverse, tangent: p.tangent, from: p.from, to: p.to })
        .collect();
    let mut warnings = Vec::new();
    if !findings.is_empty() {
        warnings.push(CommandWarning {
            code: codes::DESIGN_RULE_BROKEN.into(),
            message: format!("“{}” aksında {} yer verilen tasarım kurallarına uymuyor.", def.name, findings.len()),
            path: Some("alignment".into()),
        });
    }
    Ok((AlignmentsChecked { alignment: def.id.clone(), pairs, findings }, warnings))
}

/// Checks `input` against the document.
pub fn validate(cx: &ExecutionContext<'_>, input: &AlignmentsCheck) -> CommandResult<()> {
    match answer(cx.doc, input) {
        Ok((_, warnings)) => CommandResult::Completed { output: (), warnings },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn plan(cx: &ExecutionContext<'_>, input: &AlignmentsCheck) -> CommandResult<AlignmentsChecked> {
    match answer(cx.doc, input) {
        Ok((output, warnings)) => CommandResult::Completed { output, warnings },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn execute(cx: &mut ExecutionContext<'_>, input: AlignmentsCheck) -> CommandResult<AlignmentsChecked> {
    plan(cx, &input)
}

/// The check for a window, or why it does not read.
pub fn preview(doc: &Document, input: &AlignmentsCheck) -> Result<(AlignmentsChecked, Vec<CommandWarning>), String> {
    answer(doc, input).map_err(crate::road_corridors::words)
}
