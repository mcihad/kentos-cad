//! The reviews a project keeps of its monitoring jobs (plan/map-cad.md MAP-19, docs/adr/5220): a monitoring job's check
//! (`cad.asbuilt.check` with `kind: monitoring`) as it was reviewed, kept under a name. It holds the job as asked (its
//! layers, epochs and days, limits, line and thresholds), the thresholds' source as the user wrote it, each mark's
//! last movement, speed and state, the digest of the points the job read, and the reviewer's own statement: role,
//! title, name, day, phrase (“Kontrol edilmiştir”, “Görülmüştür”) and the kind of signature. KentOS writes what the
//! reviewer states; it checks no one's authority or signature. When the points read change the review belongs to
//! the earlier result: its digest no longer matches.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::review_notes::review_label_ok;

/// The most reviews a project keeps, and the most marks one keeps.
pub const MAX_MONITORING_REVIEWS: usize = 1_000;
pub const MAX_REVIEWED_MARKS: usize = 100_000;
/// The most characters of a review's name, a layer's, a mark's, a reviewer's name and title.
pub const MAX_MONITORING_LABEL: usize = 120;
/// The most characters of the thresholds' source and of a note.
pub const MAX_MONITORING_TEXT: usize = 500;
/// The most earlier epochs a review keeps.
pub const MAX_REVIEWED_EPOCHS: usize = 1_000;

/// Who reviewed, as the regulations name them (Genelge 2025/4 m. 3, m. 12(16)–(20); BÖHHBÜY m. 76): the engineer
/// responsible for the job, the control engineer, the technical staff with the control, the one who approves.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum ReviewerRole {
    Responsible,
    Control,
    Staff,
    Approver,
}

/// The phrase written over the signature (Genelge 2025/4 m. 12(20)): “Kontrol edilmiştir” or “Görülmüştür”.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum ReviewPhrase {
    Checked,
    Seen,
}

/// How the reviewer signed, as stated: by hand on the paper, or with a secure electronic signature (5070 m. 5).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum SignatureKind {
    Wet,
    Electronic,
}

/// The reviewer's statement: role, title (unvan), name, day (`YYYY-MM-DD`), phrase, signature and a note.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct MonitoringReviewer {
    pub role: ReviewerRole,
    pub title: String,
    pub name: String,
    pub date: String,
    pub phrase: ReviewPhrase,
    pub signature: SignatureKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub note: Option<String>,
}

/// An earlier epoch of the job: its layer and day (`YYYY-MM-DD`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ReviewedEpoch {
    pub layer: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub date: Option<String>,
}

/// A threshold level's limits as kept: horizontal and vertical movement (m), horizontal and vertical speed (m a year).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ReviewedLevel {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub horizontal: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub vertical: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m/yr")))]
    pub horizontal_rate: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m/yr")))]
    pub vertical_rate: Option<f64>,
}

/// The two thresholds as kept, with their source: who set them, by which document or decision, when.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ReviewedThresholds {
    pub warning: ReviewedLevel,
    pub alarm: ReviewedLevel,
    pub source: String,
}

/// The job as it was checked: the layer measured now, the earlier epoch's (`design`), the earlier epochs and days, this
/// epoch's day, the names' attribute, the control marks, the limits (m), the line's confidence and the thresholds.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ReviewedJob {
    pub measured: String,
    pub design: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<ReviewedEpoch>>", optional))]
    pub epochs: Vec<ReviewedEpoch>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub date: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub field: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub control: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub horizontal: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub vertical: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub confidence: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub thresholds: Option<ReviewedThresholds>,
}

/// A mark's state against the thresholds, as kept.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum ReviewedState {
    Normal,
    Warning,
    Alarm,
}

/// A mark as reviewed: its last horizontal and vertical movement from its reference (m), its horizontal speed (m a
/// year; the line's, else the mean), its state and the limits it went beyond.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ReviewedMark {
    pub name: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub horizontal: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub vertical: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m/yr")))]
    pub speed: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub state: Option<ReviewedState>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub exceeded: Vec<String>,
}

/// A review kept under its name (unique in the project): the job, the digest of the points it read (SHA-256, 64
/// lowercase hex), its marks and the reviewer's statement.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct MonitoringReview {
    pub name: String,
    pub job: ReviewedJob,
    pub digest: String,
    pub marks: Vec<ReviewedMark>,
    pub reviewer: MonitoringReviewer,
}

/// Whether `text` is a day `YYYY-MM-DD` of the Gregorian calendar.
pub fn review_day_ok(text: &str) -> bool {
    let b = text.as_bytes();
    if b.len() != 10 || b[4] != b'-' || b[7] != b'-' {
        return false;
    }
    let num = |r: std::ops::Range<usize>| text.get(r).filter(|s| s.bytes().all(|c| c.is_ascii_digit()))?.parse::<u32>().ok();
    let (Some(y), Some(m), Some(d)) = (num(0..4), num(5..7), num(8..10)) else {
        return false;
    };
    let leap = (y % 4 == 0 && y % 100 != 0) || y % 400 == 0;
    let days = match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return false,
    };
    (1..=days).contains(&d)
}

/// What is wrong with a reviewer's statement, in words; none when it reads.
pub fn reviewer_problem(r: &MonitoringReviewer) -> Option<String> {
    if !review_label_ok(&r.title, MAX_MONITORING_LABEL) {
        return Some("İnceleyenin unvanı boş, çok uzun ya da kenarında boşluk var".into());
    }
    if !review_label_ok(&r.name, MAX_MONITORING_LABEL) {
        return Some("İnceleyenin adı boş, çok uzun ya da kenarında boşluk var".into());
    }
    if !review_day_ok(&r.date) {
        return Some(format!("İncelemenin günü “{}” YYYY-AA-GG biçiminde bir gün değil", r.date));
    }
    if r.note.as_deref().is_some_and(|n| !text_ok(n)) {
        return Some(format!("İncelemenin notu boş ya da {MAX_MONITORING_TEXT} karakterden uzun"));
    }
    None
}

/// A free text (a source, a note): not empty, trimmed, at most 500 characters; new lines allowed.
pub fn text_ok(text: &str) -> bool {
    !text.is_empty()
        && text.trim() == text
        && text.chars().count() <= MAX_MONITORING_TEXT
        && !text.chars().any(|c| c.is_control() && c != '\n')
}

fn level_ok(l: &ReviewedLevel) -> bool {
    [l.horizontal, l.vertical, l.horizontal_rate, l.vertical_rate].iter().flatten().all(|v| v.is_finite() && *v > 0.0)
}

fn job_problem(j: &ReviewedJob) -> Option<&'static str> {
    let label = |s: &str| review_label_ok(s, MAX_MONITORING_LABEL);
    let positive = |v: Option<f64>| v.is_none_or(|v| v.is_finite() && v >= 0.0);
    let ok = label(&j.measured)
        && label(&j.design)
        && j.epochs.len() <= MAX_REVIEWED_EPOCHS
        && j.epochs.iter().all(|e| label(&e.layer) && e.date.as_deref().is_none_or(review_day_ok))
        && j.date.as_deref().is_none_or(review_day_ok)
        && j.field.as_deref().is_none_or(label)
        && j.control.iter().all(|c| label(c))
        && positive(j.horizontal)
        && positive(j.vertical)
        && j.confidence.is_none_or(|c| c.is_finite() && c > 0.0 && c < 1.0)
        && j.thresholds.as_ref().is_none_or(|t| level_ok(&t.warning) && level_ok(&t.alarm) && text_ok(&t.source));
    (!ok).then_some("işi (katmanları, günleri, sınırları ya da eşikleri) geçersiz")
}

/// What is wrong with the kept reviews, in words; none when they read. Names unique short labels; the job's layers
/// short labels, its days days, its limits finite and not below nought, its confidence between 0 and 1, its threshold
/// limits above nought with a source; the digest 64 lowercase hex; marks at most 100 000, their names unique short
/// labels, their numbers finite (movement and speed not below nought); the reviewer's statement reads.
pub fn monitoring_reviews_problem(list: &[MonitoringReview]) -> Option<String> {
    if list.len() > MAX_MONITORING_REVIEWS {
        return Some(format!("Proje en çok {MAX_MONITORING_REVIEWS} izleme incelemesi tutar"));
    }
    let mut names = HashSet::new();
    for r in list {
        if !review_label_ok(&r.name, MAX_MONITORING_LABEL) || !names.insert(r.name.as_str()) {
            return Some(format!("“{}” izleme incelemesinin adı geçersiz ya da iki kez yazılmış", r.name));
        }
        if let Some(p) = job_problem(&r.job) {
            return Some(format!("“{}” izleme incelemesinin {p}", r.name));
        }
        if r.digest.len() != 64 || !r.digest.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)) {
            return Some(format!("“{}” izleme incelemesinin özeti 64 küçük onaltılık hane değil", r.name));
        }
        if r.marks.len() > MAX_REVIEWED_MARKS {
            return Some(format!("“{}” izleme incelemesi en çok {MAX_REVIEWED_MARKS} nokta tutar", r.name));
        }
        let mut marks = HashSet::new();
        for m in &r.marks {
            let fine = |v: f64| v.is_finite() && v >= 0.0;
            if !review_label_ok(&m.name, MAX_MONITORING_LABEL)
                || !marks.insert(m.name.as_str())
                || !fine(m.horizontal)
                || !m.vertical.is_none_or(f64::is_finite)
                || !m.speed.is_none_or(fine)
                || (m.state.is_none() && !m.exceeded.is_empty())
                || !m.exceeded.iter().all(|k| ["horizontal", "vertical", "horizontalRate", "verticalRate"].contains(&k.as_str()))
            {
                return Some(format!("“{}” izleme incelemesinin “{}” noktası geçersiz", r.name, m.name));
            }
        }
        if let Some(p) = reviewer_problem(&r.reviewer) {
            return Some(format!("“{}” izleme incelemesi: {p}", r.name));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn review() -> MonitoringReview {
        MonitoringReview {
            name: "Ekim incelemesi".into(),
            job: ReviewedJob {
                measured: "Dönem 4".into(),
                design: "Dönem 3".into(),
                epochs: vec![ReviewedEpoch { layer: "Dönem 1".into(), date: Some("2026-01-15".into()) }],
                date: Some("2026-10-15".into()),
                field: None,
                control: vec![],
                horizontal: Some(0.01),
                vertical: None,
                confidence: Some(0.95),
                thresholds: Some(ReviewedThresholds {
                    warning: ReviewedLevel { horizontal: Some(0.01), ..ReviewedLevel::default() },
                    alarm: ReviewedLevel { horizontal: Some(0.02), ..ReviewedLevel::default() },
                    source: "Proje şartnamesi".into(),
                }),
            },
            digest: "a".repeat(64),
            marks: vec![ReviewedMark {
                name: "M1".into(),
                horizontal: 0.012,
                vertical: Some(-0.003),
                speed: Some(0.02),
                state: Some(ReviewedState::Warning),
                exceeded: vec!["horizontal".into()],
            }],
            reviewer: MonitoringReviewer {
                role: ReviewerRole::Control,
                title: "Harita Mühendisi".into(),
                name: "A. Yılmaz".into(),
                date: "2026-10-20".into(),
                phrase: ReviewPhrase::Checked,
                signature: SignatureKind::Wet,
                note: None,
            },
        }
    }

    #[test]
    fn a_review_reads_and_what_it_may_not_hold() {
        assert_eq!(monitoring_reviews_problem(&[review()]), None);
        let broken = |f: &dyn Fn(&mut MonitoringReview)| {
            let mut r = review();
            f(&mut r);
            monitoring_reviews_problem(&[r])
        };
        assert!(broken(&|r| r.digest = "A".repeat(64)).is_some());
        assert!(broken(&|r| r.reviewer.date = "2026-02-30".into()).is_some());
        assert!(broken(&|r| r.job.thresholds.as_mut().unwrap().source = String::new()).is_some());
        assert!(broken(&|r| r.marks[0].exceeded = vec!["far".into()]).is_some());
        assert!(broken(&|r| r.job.confidence = Some(1.0)).is_some());
        assert!(monitoring_reviews_problem(&[review(), review()]).is_some());
        assert!(review_day_ok("2028-02-29") && !review_day_ok("2026-13-01") && !review_day_ok("2026-1-01"));
    }
}
