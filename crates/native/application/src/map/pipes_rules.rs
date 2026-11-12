//! The rule package of `cad.pipes.check` (plan/map-cad.md MAP-16, docs/adr/4780): the system asked of the package
//! given or of KentOS's (`kentos_commands::pipe_rules_default`, the Ministry's 2017 sewer and storm water regulations'
//! annexes), a given package's own rules, and the classes a pipe's diameter falls in. A diameter is compared in
//! millimetres, as the tables write φ, to a billionth of the bound (the catalogue's comparison, `pipes_check`): a
//! table's φ400 holds a pipe whose attribute says 0.4 m.

use kentos_commands::{PipeRuleSystem, PipeRules, PipeSlopeClass, PipeSpanClass, PipesCheck, pipe_rules_default};

use crate::checks::Stop;
use crate::codes;
use crate::pipes_check::refuse;

fn bad(message: String, path: String) -> Stop {
    refuse(codes::INVALID_PIPE_CHECK, message, &path)
}

/// The system the check takes its limits from, refused as `cad.pipes.check`'s contract says: rules asked without a
/// diameter, a system the package does not have, a given package's system not whole.
pub(crate) fn asked(input: &PipesCheck) -> Result<Option<PipeRuleSystem>, Stop> {
    let Some(ask) = &input.rules else { return Ok(None) };
    if input.diameter.is_none() {
        return Err(bad(
            "Kural paketinin eğim sınırları borunun çap sınıfına göredir; çapın özniteliğini (diameter) de verin.".into(),
            "rules".into(),
        ));
    }
    let package: PipeRules = match &ask.package {
        Some(p) => p.clone(),
        None => pipe_rules_default().ok_or_else(|| {
            bad("KentOS'un kanal kural paketi okunamadı; paketi (rules.package) kendiniz verin.".into(), "rules".into())
        })?,
    };
    let Some(k) = package.systems.iter().position(|s| s.id == ask.system) else {
        let names: Vec<&str> = package.systems.iter().map(|s| s.id.as_str()).collect();
        return Err(bad(
            format!("“{}” sistemi kural paketinde yok; paketin sistemleri: {}.", ask.system, names.join(", ")),
            "rules.system".into(),
        ));
    };
    let system = package.systems[k].clone();
    if ask.package.is_some() {
        whole(&system, &format!("rules.package.systems[{k}]"))?;
    }
    Ok(Some(system))
}

/// A given package's system: a least cover above zero; each slope class with a range of diameters and slopes 1:A
/// above zero, its greatest no flatter than its least and its exceptional least no steeper; each spacing with a range
/// and a greatest above zero.
fn whole(s: &PipeRuleSystem, at: &str) -> Result<(), Stop> {
    let positive = |v: f64| v.is_finite() && v > 0.0;
    if s.min_cover.is_some_and(|v| !positive(v)) {
        return Err(bad("Kural paketinin en az örtüsü 0'dan büyük bir sayı olmalı.".into(), format!("{at}.minCover")));
    }
    for (j, c) in s.slopes.iter().enumerate() {
        let wrong = |why: &str| Err(bad(format!("Kural paketinin “{}” sınıfı geçerli değil: {why}", c.name), format!("{at}.slopes[{j}]")));
        let bounds = [c.from, c.over, c.to];
        if bounds.iter().all(Option::is_none) {
            return wrong("çap aralığı yok; from, over ya da to verin.");
        }
        if bounds.iter().flatten().any(|v| !(v.is_finite() && *v >= 0.0)) {
            return wrong("çapları 0 ya da büyük birer sayı olmalı (mm).");
        }
        if ![c.least, c.greatest].into_iter().chain(c.exceptional).all(positive) {
            return wrong("eğimleri 1:A biçiminde yazılır; A 0'dan büyük olmalı.");
        }
        if c.greatest > c.least {
            return wrong("en çok eğimi en az eğiminden yatık olamaz (greatest, least'ten büyük olamaz).");
        }
        if c.exceptional.is_some_and(|e| e < c.least) {
            return wrong("istisnai en az eğimi en az eğiminden dik olamaz (exceptional, least'ten küçük olamaz).");
        }
    }
    for (j, c) in s.spans.iter().enumerate() {
        let wrong = |why: &str| Err(bad(format!("Kural paketinin {}. baca aralığı geçerli değil: {why}", j + 1), format!("{at}.spans[{j}]")));
        if c.below.is_none() && c.over.is_none() {
            return wrong("çap aralığı yok; below ya da over verin.");
        }
        if !positive(c.greatest) || c.least.is_some_and(|v| !positive(v) || v > c.greatest) {
            return wrong("en çok aralığı 0'dan büyük, en az aralığı ondan büyük olmayan sayılar olmalı (m).");
        }
    }
    Ok(())
}

/// Whether `mm` is at or above `bound`, to a billionth of it.
fn at_least(mm: f64, bound: f64) -> bool {
    mm >= bound - 1e-9 * bound.abs().max(1.0)
}

/// Whether `mm` is at or below `bound`, to a billionth of it.
fn at_most(mm: f64, bound: f64) -> bool {
    mm <= bound + 1e-9 * bound.abs().max(1.0)
}

/// The first slope class holding a diameter of `mm` millimetres.
pub(crate) fn slope_class(s: &PipeRuleSystem, mm: f64) -> Option<&PipeSlopeClass> {
    s.slopes.iter().find(|c| {
        c.from.is_none_or(|b| at_least(mm, b)) && c.over.is_none_or(|b| !at_most(mm, b)) && c.to.is_none_or(|b| at_most(mm, b))
    })
}

/// The first spacing holding a diameter of `mm` millimetres.
pub(crate) fn span_class(s: &PipeRuleSystem, mm: f64) -> Option<&PipeSpanClass> {
    s.spans.iter().find(|c| c.below.is_none_or(|b| !at_least(mm, b)) && c.over.is_none_or(|b| !at_most(mm, b)))
}
