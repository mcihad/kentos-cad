use serde_json::json;

use super::*;
use crate::processing_job::RunRequest;

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

/// The request as the process reads it: `{"kind": "audit", "input": {…}}`, camelCase, the optional ones left out.
#[test]
fn an_audit_request_is_a_run_request_of_its_own() {
    let request = RunRequest::Audit(run());
    let text = serde_json::to_value(&request).unwrap();
    assert_eq!(text["kind"], "audit");
    assert_eq!(text["input"]["rule"], "adsiz");
    assert!(text["input"].get("timeoutSeconds").is_none());
    let back: RunRequest = serde_json::from_value(text).unwrap();
    assert_eq!(back, request);
}

/// What a request must be: an id, a version, a content digest, a rule, an absolute interpreter, a time within bounds.
#[test]
fn an_audit_request_is_checked_field_by_field() {
    assert_eq!(check_audit_run(&run()), None);
    let fault = |edit: &dyn Fn(&mut AuditRun)| {
        let mut r = run();
        edit(&mut r);
        check_audit_run(&r).map(|(path, _)| path)
    };
    assert_eq!(
        fault(&|r| r.extension = "bir eklenti".into()),
        Some("input.extension".into())
    );
    assert_eq!(fault(&|r| r.version.clear()), Some("input.version".into()));
    for digest in ["sha256:abc", "md5:x", &format!("sha256:{}", "A".repeat(64))] {
        assert_eq!(
            fault(&|r| r.digest = digest.to_owned()),
            Some("input.digest".into()),
            "{digest}"
        );
    }
    assert_eq!(fault(&|r| r.rule = "\n".into()), Some("input.rule".into()));
    assert_eq!(
        fault(&|r| r.python = Some("python3".into())),
        Some("input.python".into()),
        "a name the PATH would find is no prepared environment"
    );
    assert_eq!(fault(&|r| r.python = None), None, "said when it runs");
    assert_eq!(
        fault(&|r| r.timeout_seconds = Some(0)),
        Some("input.timeoutSeconds".into())
    );
    assert_eq!(
        fault(&|r| r.timeout_seconds = Some(AUDIT_TIMEOUT_MAX_SECONDS + 1)),
        Some("input.timeoutSeconds".into())
    );
}

/// The rule's answer read as the desktop reads it: known severities, words and a fix's title cut, the object's id, the
/// fix's arguments as given; a failure in its words with its traceback.
#[test]
fn a_rule_s_answer_reads_as_the_desktop_reads_it() {
    let done = json!({ "ok": true, "result": { "kind": "findings", "findings": [
        { "severity": "error", "message": "x".repeat(2100), "featureId": "u-1",
          "fix": { "title": "t".repeat(130), "command": "cad.entities.set", "args": { "uids": ["u-1"] } } },
        { "severity": "fatal", "message": "bilinmeyen önem" },
        { "severity": "info", "message": "genel", "fix": { "title": "Bir", "command": "cad.x" } }
    ] } });
    let (findings, dropped) = findings_of(&done).unwrap();
    assert_eq!(dropped, 0);
    assert_eq!(findings.len(), 2, "an unknown severity is left out");
    assert_eq!(findings[0].message.chars().count(), 2000);
    let fix = findings[0].fix.as_ref().unwrap();
    assert_eq!(fix.title.chars().count(), 120);
    assert_eq!(fix.args, json!({ "uids": ["u-1"] }));
    assert_eq!(findings[0].feature_id.as_deref(), Some("u-1"));
    assert_eq!(findings[1].fix.as_ref().unwrap().args, json!({}));
    assert_eq!(
        counts(findings.iter().map(|f| f.severity)),
        "1 hata, 1 bilgi"
    );
    assert_eq!(counts([]), "Bulgu yok");
    assert_eq!(
        (Severity::Warning.code(), Severity::Warning.word()),
        ("warning", "Uyarı")
    );
    let failed =
        json!({ "ok": false, "exception": "KeyError", "message": "'no'", "error": "Traceback …" });
    assert_eq!(
        findings_of(&failed),
        Err(("KeyError: 'no'".into(), Some("Traceback …".into())))
    );
    assert_eq!(
        findings_of(&json!({ "ok": false, "stopped": true }))
            .unwrap_err()
            .0,
        "Durduruldu."
    );
    assert_eq!(
        findings_of(&json!({ "ok": true, "result": { "kind": "outputs" } }))
            .unwrap_err()
            .0,
        "Kural bulgu listesi döndürmedi."
    );
}

/// Past `MOST_FINDINGS` the findings are counted, not kept.
#[test]
fn too_many_findings_are_counted() {
    let many: Vec<Value> = (0..MOST_FINDINGS + 3)
        .map(|i| json!({ "severity": "warning", "message": format!("{i}") }))
        .collect();
    let done = json!({ "ok": true, "result": { "kind": "findings", "findings": many } });
    let (findings, dropped) = findings_of(&done).unwrap();
    assert_eq!((findings.len(), dropped), (MOST_FINDINGS, 3));
}

/// A facts' digest is its canonical JSON's: the same whatever order its keys were written in.
#[test]
fn a_digest_is_the_canonical_json_s() {
    let a = digest_of(&json!({ "kind": "audit", "facts": { "layers": [], "cut": false } }));
    let b: Value =
        serde_json::from_str(r#"{"facts":{"cut":false,"layers":[]},"kind":"audit"}"#).unwrap();
    assert_eq!(a, digest_of(&b));
    assert!(a.starts_with("sha256:") && a.len() == 71, "{a}");
}
