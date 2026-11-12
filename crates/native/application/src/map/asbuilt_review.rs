//! `cad.asbuilt.review` v1 (plan/map-cad.md MAP-19, docs/adr/5220): İzleme incelemesi. A monitoring job's check kept in
//! the project's settings under a name with the reviewer's statement (contracts' `monitoring_reviews`): the job as
//! asked, its thresholds with their source, each mark's last movement, speed and state, and the digest of the points
//! the job reads. One undo step; the same name replaced, `remove` removes.
//!
//! The digest is SHA-256 of the job's layers, the earlier epochs oldest first, then the earlier epoch's (`design`),
//! then this epoch's (`measured`): each a line `k<TAB>layer`, then its points as the design's digest writes them
//! (`p<TAB>name<TAB>x<TAB>y<TAB>z`, six decimals, `-` for no height), in the drawing's order. A review is `current`
//! while the digest of its job's points now is the one kept. The shared cases are
//! `fixtures/commands/v1/cad.asbuilt.review.json`.

use std::collections::HashSet;

use kentos_commands::encoding::sha256_hex;
use kentos_commands::{
    AsbuiltCheck, AsbuiltChecked, AsbuiltKind, AsbuiltLevel, AsbuiltReview, AsbuiltReviewRow, AsbuiltReviewed, AsbuiltState,
    CommandResult, MAX_MONITORING_LABEL, MonitoringReview, ReviewedEpoch, ReviewedJob, ReviewedLevel, ReviewedMark,
    ReviewedState, ReviewedThresholds, monitoring_reviews_problem, review_label_ok, reviewer_problem, text_ok,
};
use kentos_domain::Document;

use crate::ExecutionContext;
use crate::asbuilt_check::{NAME_FIELD, points};
use crate::checks::{self, Stop, error};
use crate::codes;

/// The undo step's name.
pub const STEP: &str = "İzleme incelemesi";

fn bad(message: &str, path: &str) -> Stop {
    Stop::failed(error(codes::INVALID_INPUT, message.to_owned(), Some(path.to_owned())))
}

/// The digest of the points a kept job reads now (the module's rule).
pub fn digest_of(doc: &Document, job: &ReviewedJob) -> String {
    let field = job.field.as_deref().map(str::trim).unwrap_or(NAME_FIELD);
    let layers = job.epochs.iter().map(|e| e.layer.as_str()).chain([job.design.as_str(), job.measured.as_str()]);
    let mut text = String::new();
    for layer in layers {
        text.push_str(&format!("k\t{layer}\n"));
        for p in points(doc, layer, field, None) {
            let z = p.z.map_or_else(|| "-".to_owned(), |z| format!("{z:.6}"));
            text.push_str(&format!("p\t{}\t{:.6}\t{:.6}\t{z}\n", p.name, p.p.x, p.p.y));
        }
    }
    sha256_hex(text.as_bytes())
}

/// Every kept review with whether its points are still the ones reviewed.
pub fn rows(doc: &Document, list: &[MonitoringReview]) -> Vec<AsbuiltReviewRow> {
    list.iter()
        .map(|r| {
            let count = |s: ReviewedState| r.marks.iter().filter(|m| m.state == Some(s)).count() as u32;
            AsbuiltReviewRow {
                name: r.name.clone(),
                reviewer: r.reviewer.name.clone(),
                date: r.reviewer.date.clone(),
                phrase: r.reviewer.phrase,
                alarm: count(ReviewedState::Alarm),
                warning: count(ReviewedState::Warning),
                current: digest_of(doc, &r.job) == r.digest,
            }
        })
        .collect()
}

fn level(l: &AsbuiltLevel) -> ReviewedLevel {
    ReviewedLevel { horizontal: l.horizontal, vertical: l.vertical, horizontal_rate: l.horizontal_rate, vertical_rate: l.vertical_rate }
}

/// The job as asked, with the thresholds' source.
fn job_of(check: &AsbuiltCheck, source: Option<&str>) -> ReviewedJob {
    ReviewedJob {
        measured: check.measured.clone(),
        design: check.design.clone().unwrap_or_default(),
        epochs: check.epochs.iter().map(|e| ReviewedEpoch { layer: e.layer.clone(), date: e.date.clone() }).collect(),
        date: check.date.clone(),
        field: check.field.clone(),
        control: check.control.clone(),
        horizontal: check.horizontal,
        vertical: check.vertical,
        confidence: check.trend.as_ref().map(|t| t.confidence),
        thresholds: check.thresholds.as_ref().map(|t| ReviewedThresholds {
            warning: level(&t.warning),
            alarm: level(&t.alarm),
            source: source.unwrap_or_default().to_owned(),
        }),
    }
}

/// Each mark as reviewed: with a series its track's last step, speed and state; without, its movement from the earlier
/// epoch. A name met again is the first's (the job matches marks by name).
fn marks_of(checked: &AsbuiltChecked) -> Vec<ReviewedMark> {
    let mut seen = HashSet::new();
    if !checked.tracks.is_empty() {
        return checked
            .tracks
            .iter()
            .filter_map(|t| {
                let last = t.steps.last()?.as_ref()?;
                seen.insert(t.name.clone()).then(|| ReviewedMark {
                    name: t.name.clone(),
                    horizontal: last.horizontal,
                    vertical: last.dz,
                    speed: t.fit.as_ref().map(|f| f.speed).or(t.rate_horizontal),
                    state: t.state.map(|s| match s {
                        AsbuiltState::Normal => ReviewedState::Normal,
                        AsbuiltState::Warning => ReviewedState::Warning,
                        AsbuiltState::Alarm => ReviewedState::Alarm,
                    }),
                    exceeded: t.exceeded.clone(),
                })
            })
            .collect();
    }
    checked
        .points
        .iter()
        .filter_map(|p| {
            let h = p.horizontal?;
            seen.insert(p.name.clone()).then(|| ReviewedMark {
                name: p.name.clone(),
                horizontal: h,
                vertical: p.dh,
                speed: None,
                state: None,
                exceeded: Vec::new(),
            })
        })
        .collect()
}

/// What the review writes: the project's reviews after it and the answer (its revision the drawing's now).
fn review(doc: &Document, input: &AsbuiltReview) -> Result<(Vec<MonitoringReview>, AsbuiltReviewed), Stop> {
    checks::revision(doc, input.expected_revision.as_deref())?;
    if !review_label_ok(&input.name, MAX_MONITORING_LABEL) {
        return Err(bad(
            &format!("İncelemenin adı boş, {MAX_MONITORING_LABEL} karakterden uzun ya da kenarında boşluk var."),
            "name",
        ));
    }
    let mut list = doc.monitoring_reviews().to_vec();
    let at = list.iter().position(|r| r.name == input.name);
    let mut out = AsbuiltReviewed {
        name: input.name.clone(),
        kept: None,
        replaced: false,
        removed: false,
        reviews: Vec::new(),
        revision: doc.revision().to_string(),
    };
    if input.remove {
        if input.check.is_some() || input.reviewer.is_some() || input.thresholds_source.is_some() {
            return Err(bad("Kaldırırken yalnız incelemenin adını verin; denetim, inceleyen ve kaynak yazılmaz.", "remove"));
        }
        let Some(at) = at else {
            return Err(Stop::failed(error(
                codes::UNKNOWN_MONITORING_REVIEW,
                format!("Projede “{}” adıyla saklanan izleme incelemesi yok.", input.name),
                Some("name".into()),
            )));
        };
        list.remove(at);
        out.removed = true;
    } else {
        let Some(check) = input.check.as_ref() else {
            return Err(bad("İncelenecek izleme işinin denetimini (check) verin.", "check"));
        };
        let Some(reviewer) = input.reviewer.as_ref() else {
            return Err(bad("İnceleyenin rolünü, unvanını, adını, gününü, ibaresini ve imza türünü (reviewer) verin.", "reviewer"));
        };
        if check.kind != Some(AsbuiltKind::Monitoring) {
            return Err(bad("İnceleme kaydı izleme işinindir (check.kind: monitoring).", "check.kind"));
        }
        match (&check.thresholds, input.thresholds_source.as_deref()) {
            (Some(_), None) => {
                return Err(bad(
                    "Eşiklerin kaynağını yazın: kim, hangi belge ya da kararla, ne zaman belirledi. KentOS eşik vermez.",
                    "thresholdsSource",
                ));
            }
            (None, Some(_)) => return Err(bad("Eşik verilmedi; kaynağı yazılmaz.", "thresholdsSource")),
            (Some(_), Some(s)) if !text_ok(s) => {
                return Err(bad("Eşiklerin kaynağı boş, 500 karakterden uzun ya da kenarında boşluk var.", "thresholdsSource"));
            }
            _ => {}
        }
        if let Some(problem) = reviewer_problem(reviewer) {
            return Err(bad(&format!("{problem}."), "reviewer"));
        }
        let checked = crate::asbuilt_check::run(doc, check)?;
        let job = job_of(check, input.thresholds_source.as_deref());
        let kept = MonitoringReview {
            name: input.name.clone(),
            digest: digest_of(doc, &job),
            marks: marks_of(&checked),
            job,
            reviewer: reviewer.clone(),
        };
        match at {
            Some(at) => {
                list[at] = kept.clone();
                out.replaced = true;
            }
            None => list.push(kept.clone()),
        }
        out.kept = Some(kept);
    }
    if let Some(problem) = monitoring_reviews_problem(&list) {
        return Err(bad(&format!("{problem}; katmanların ve noktaların adları kısa, kenarında boşluksuz olmalı."), "check"));
    }
    out.reviews = rows(doc, &list);
    Ok((list, out))
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &AsbuiltReview) -> CommandResult<()> {
    match review(cx.doc, input) {
        Ok(_) => CommandResult::Completed { output: (), warnings: Vec::new() },
        Err(stop) => stop.into(),
    }
}

/// What execute would write; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &AsbuiltReview) -> CommandResult<AsbuiltReviewed> {
    match review(cx.doc, input) {
        Ok((_, out)) => CommandResult::Completed { output: out, warnings: Vec::new() },
        Err(stop) => stop.into(),
    }
}

fn write(cx: &mut ExecutionContext<'_>, input: AsbuiltReview) -> CommandResult<AsbuiltReviewed> {
    let (list, mut out) = match review(cx.doc, &input) {
        Ok(done) => done,
        Err(stop) => return stop.into(),
    };
    cx.doc.set_monitoring_reviews(list, STEP);
    out.revision = cx.doc.revision().to_string();
    CommandResult::Completed { output: out, warnings: Vec::new() }
}

pub fn execute(cx: &mut ExecutionContext<'_>, input: AsbuiltReview) -> CommandResult<AsbuiltReviewed> {
    crate::call::recorded(cx, kentos_commands::CAD_ASBUILT_REVIEW, kentos_commands::CAD_ASBUILT_REVIEW_VERSION, input, write)
}
