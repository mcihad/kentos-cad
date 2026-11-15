use kentos_contracts::ProcessingUsage;
use kentos_contracts::audit_job::{Finding, RuleRef, Severity};

use super::*;

fn run() -> AuditRun {
    AuditRun {
        extension: "tr.ornek.denetim".into(),
        version: "1.0.0".into(),
        digest: format!("sha256:{}", "a".repeat(64)),
        rule: "adsiz".into(),
        python: Some("/srv/kentos/python/tr.ornek.denetim-1.0.0/bin/python".into()),
        timeout_seconds: None,
    }
}

fn report(run: &AuditRun) -> AuditReport {
    AuditReport {
        rule: RuleRef {
            extension: run.extension.clone(),
            version: run.version.clone(),
            digest: run.digest.clone(),
            rule: run.rule.clone(),
            handler: "denetim.logic:adsiz".into(),
            title: "Adsız nesneler".into(),
        },
        drawing: "7b7fea779c35a3d0".into(),
        facts: format!("sha256:{}", "b".repeat(64)),
        kentos: "0.1.0".into(),
        findings: vec![Finding {
            severity: Severity::Error,
            message: "50.00 m² çakışma".into(),
            feature_id: Some("u-2".into()),
            fix: None,
        }],
        dropped: 0,
        cut: false,
    }
}

fn planned() -> Planned {
    Planned {
        catalog: "tenant:buro".into(),
        key_input: json!({}),
    }
}

/// An input is the kind's or it is refused where it is wrong, before a job is made.
#[test]
fn an_audit_input_is_checked_when_it_is_asked_for() {
    let good = json!({ "extension": "tr.ornek.denetim", "rule": "adsiz" });
    assert_eq!(checked(&good).unwrap(), good);
    let path = |input: Value| match checked(&input) {
        Err(AppError::Invalid { path, .. }) => path,
        other => panic!("refused as an input: {other:?}"),
    };
    assert_eq!(
        path(json!({ "extension": "tr.ornek.denetim", "rule": "" })),
        Some("input.rule".into())
    );
    assert_eq!(
        path(
            json!({ "extension": "tr.ornek.denetim", "rule": "adsiz", "python": "/usr/bin/python3" })
        ),
        Some("input".into()),
        "the interpreter is the server's setting, never the request's"
    );
}

/// The job keeps the job runner's report with where the release came from, the revision read and what the run took;
/// a report of another release or rule is refused.
#[test]
fn the_result_is_the_report_of_the_release_asked() {
    let run = run();
    let processed = Processed {
        summary: "Adsız nesneler: 1 hata".into(),
        details: serde_json::to_value(report(&run)).unwrap(),
        usage: ProcessingUsage {
            cpu_seconds: 1.5,
            peak_memory_mb: 80.0,
        },
        ..Processed::default()
    };
    let kept: ProjectAuditResult =
        serde_json::from_value(result(&processed, &run, &planned(), 7).unwrap()).unwrap();
    assert_eq!(
        (
            kept.catalog.as_str(),
            kept.base_revision.as_str(),
            kept.summary.as_str()
        ),
        ("tenant:buro", "7", "Adsız nesneler: 1 hata")
    );
    assert_eq!(kept.report, report(&run));
    assert_eq!(kept.usage.cpu_seconds, 1.5);
    let mut other = run.clone();
    other.digest = format!("sha256:{}", "c".repeat(64));
    assert!(
        result(&processed, &other, &planned(), 7)
            .unwrap_err()
            .contains("istenen kuralın değil")
    );
    let bare = Processed::default();
    assert!(
        result(&bare, &run, &planned(), 7)
            .unwrap_err()
            .contains("raporunu vermedi")
    );
}
