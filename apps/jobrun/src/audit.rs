//! An extension's audit rule run as a server job (plan/eklenti.md ADIM 20, docs/adr/8057 §42): the folder's drawing
//! measured with the desktop's code, the same rule run through `kentos.ext_host` as the desktop runs it, and its
//! findings written with their provenance (`AuditReport`, the answer's `details`).
//!
//! - **The rule's package** is the one beside the request (`extension.kek`): opened by the package inspector, its content
//!   digest, id and version the request's, or nothing runs; its files are put in the folder (`extension/`) and the rule
//!   is found by its local id in the manifest's audits.
//! - **No network:** a package that asks for the `network` capability is refused; the box gives a rule none.
//! - **Its Python** is the environment the server prepared for that extension version, given by the request
//!   (`python`), run offline: isolated (`-I`), its bytecode in the folder, with an empty environment as the whole
//!   process has. Without one the rule cannot run and the job says so.
//! - **The facts and the provenance** are `kentos_processing::audit_job`'s and `kentos_contracts::audit_job`'s, the
//!   desktop's own: the same drawing and rule give the same report on both.

use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use kentos_contracts::audit_job::{
    AUDIT_TIMEOUT_SECONDS, AuditReport, AuditRun, RuleRef, check_audit_run, counts, digest_of,
    findings_of,
};
use kentos_contracts::processing_job::{ProcessFailure, Processed};
use kentos_extensions::manifest::Capability;
use kentos_processing::audit_job;
use kentos_python_env::process::{Ended, Pool, Start};
use serde_json::{Value, json};

/// The package beside the request, and where its files are put.
pub const PACKAGE: &str = "extension.kek";
const UNPACKED: &str = "extension";
/// The rule's bytecode, in the folder.
const BYTECODE: &str = "pyc";

/// The peak resident memory of the rule's processes, kilobytes: read from `/proc/<pid>/status` (`VmHWM`) just before
/// they are closed, for the job's own usage (docs/adr/8057 §58); 0 where the system does not say it.
static RULE_PEAK_KB: AtomicU64 = AtomicU64::new(0);

/// What the rule's processes took at their peak, kilobytes.
pub fn rule_peak_kb() -> u64 {
    RULE_PEAK_KB.load(Ordering::Relaxed)
}

fn note_peaks(pool: &Pool) {
    for process in pool.running() {
        let peak = std::fs::read_to_string(format!("/proc/{}/status", process.pid))
            .ok()
            .and_then(|status| {
                let line = status.lines().find(|l| l.starts_with("VmHWM:"))?;
                line.split_whitespace().nth(1)?.parse::<u64>().ok()
            });
        if let Some(kb) = peak {
            RULE_PEAK_KB.fetch_max(kb, Ordering::Relaxed);
        }
    }
}

/// Runs a rule's `audit` invoke in the process `start` makes and gives its answer (`done`), or why there is none.
pub trait RuleRunner {
    fn run(
        &self,
        start: Start,
        invoke: Value,
        timeout: Duration,
        stopped: &dyn Fn() -> bool,
    ) -> Result<Value, String>;
}

/// The extension's Python through `kentos.ext_host`, as the desktop's Eklenti denetimi runs it.
pub struct ExtHost {
    pool: Arc<Pool>,
}

impl Default for ExtHost {
    fn default() -> Self {
        ExtHost {
            pool: Arc::new(Pool::new(Duration::from_secs(60))),
        }
    }
}

impl RuleRunner for ExtHost {
    fn run(
        &self,
        start: Start,
        invoke: Value,
        timeout: Duration,
        stopped: &dyn Fn() -> bool,
    ) -> Result<Value, String> {
        let ended = self.pool.run(
            "audit",
            &|| Ok(start.clone()),
            invoke,
            timeout,
            stopped,
            &mut |_| {},
        );
        note_peaks(&self.pool);
        self.pool.close_all();
        match ended {
            Ended::Answered(done) => Ok(done),
            Ended::StoppedBefore => Err("Başlamadan durduruldu.".to_owned()),
            Ended::Killed { timed_out: true } => {
                Err("Süresini aştı; eklentinin süreci sonlandırıldı.".to_owned())
            }
            Ended::Killed { timed_out: false } => {
                Err("Durdur'a cevap vermedi; eklentinin süreci sonlandırıldı.".to_owned())
            }
            Ended::NotStarted(why) => Err(why),
            Ended::Crashed(words) => {
                Err(format!("Eklentinin süreci kendiliğinden kapandı: {words}"))
            }
        }
    }
}

fn invalid(message: impl Into<String>) -> ProcessFailure {
    ProcessFailure::new("invalid", message)
}

/// The rule's interpreter's arguments: isolated (no user site, no environment variables, not the working folder on
/// the path), its bytecode in `pycache`; the desktop's (`interpreter_args`).
fn interpreter_args(pycache: &Path) -> Vec<std::ffi::OsString> {
    let mut prefix = std::ffi::OsString::from("pycache_prefix=");
    prefix.push(pycache);
    vec![
        "-I".into(),
        "-X".into(),
        prefix,
        "-m".into(),
        "kentos.ext_host".into(),
    ]
}

/// The request's rule on the folder's drawing; what it found.
pub fn run_audit(
    dir: &Path,
    run: AuditRun,
    runner: &dyn RuleRunner,
) -> Result<Processed, ProcessFailure> {
    if let Some((path, message)) = check_audit_run(&run) {
        return Err(invalid(format!("{path}: {message}")));
    }
    let bytes = std::fs::read(dir.join(PACKAGE))
        .map_err(|e| invalid(format!("Eklentinin paketi işin klasöründe yok: {e}")))?;
    let opened = kentos_extensions::open(&bytes, &kentos_extensions::Limits::default());
    if let Some(p) = opened.inspection.problems.first() {
        return Err(invalid(format!(
            "Eklentinin paketi açılamadı: {}",
            p.message
        )));
    }
    if opened.inspection.digest.as_deref() != Some(run.digest.as_str()) {
        return Err(invalid(format!(
            "Paketin içerik özeti istenen değil ({}): başka bir sürüm ya da bozulmuş bir paket.",
            run.digest
        )));
    }
    let Some(manifest) = opened.inspection.manifest.as_ref() else {
        return Err(invalid("Paketin manifesti okunamadı."));
    };
    let version = manifest.version.to_string();
    if manifest.id != run.extension || version != run.version {
        return Err(invalid(format!(
            "Paket {} {}; istenen {} {}.",
            manifest.id, manifest.version, run.extension, run.version
        )));
    }
    let Some(entry) = manifest
        .entrypoints
        .audits
        .iter()
        .find(|a| a.id == run.rule)
    else {
        return Err(invalid(format!(
            "“{}” bu eklentinin bir denetimi değil.",
            run.rule
        )));
    };
    if manifest
        .requested_capabilities
        .iter()
        .any(|c| c.id == Capability::Network)
    {
        return Err(invalid(
            "Bulut denetim kutusu ağa çıkmaz: “network” yeteneğini isteyen eklentinin kuralı burada çalışmaz.",
        ));
    }
    let Some(python) = run.python.clone() else {
        return Err(ProcessFailure::new(
            "tool_unavailable",
            "Bu sunucuda eklentinin Python ortamı hazırlanmadı: kural çalıştırılamaz.",
        ));
    };
    let source = dir.join(UNPACKED);
    for (path, data) in &opened.files {
        if kentos_extensions::path_problem(path).is_some() {
            return Err(invalid(format!("Paketteki “{path}” yolu kullanılamaz.")));
        }
        let to = source.join(path);
        if let Some(parent) = to.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| ProcessFailure::new("failed", format!("Paket açılamadı: {e}")))?;
        }
        std::fs::write(&to, data)
            .map_err(|e| ProcessFailure::new("failed", format!("Paket açılamadı: {e}")))?;
    }
    let session = crate::open_project(dir)?;
    crate::say(0.20, "Çizimin olguları ölçülüyor");
    let doc = session.document();
    let never = || false;
    let Some(facts) = audit_job::facts_of(doc, &never) else {
        return Err(ProcessFailure::new("stopped", "İş durduruldu."));
    };
    let drawing = audit_job::drawing_stamp(doc);
    let given = digest_of(&facts);
    let cut = facts["cut"] == true;
    let mut invoke = audit_job::message(facts);
    invoke["type"] = json!("invoke");
    invoke["local"] = json!(entry.id);
    invoke["handler"] = json!(entry.handler);
    invoke["document"] = json!(false);
    let start = Start {
        python: python.into(),
        args: interpreter_args(&dir.join(BYTECODE)),
        hello: json!({
            "type": "hello",
            "protocol": 1,
            "extension": run.extension,
            "version": run.version,
            "digest": run.digest,
            "source": source.display().to_string(),
            "paths": ["src"],
        }),
    };
    crate::say(0.50, &format!("Kural çalışıyor: {}", entry.title));
    let timeout = Duration::from_secs(run.timeout_seconds.unwrap_or(AUDIT_TIMEOUT_SECONDS).into());
    let done = runner
        .run(start, invoke, timeout, &never)
        .map_err(|why| ProcessFailure::new("tool_failed", why))?;
    let (findings, dropped) = findings_of(&done).map_err(|(words, traceback)| {
        // The traceback's last 4000 characters, where the error is.
        let tail = traceback
            .map(|t| {
                let skip = t.chars().count().saturating_sub(4000);
                format!("\n{}", t.chars().skip(skip).collect::<String>())
            })
            .unwrap_or_default();
        ProcessFailure::new("tool_failed", format!("{}: {words}{tail}", entry.title))
    })?;
    crate::say(0.92, "Sonuç hazırlanıyor");
    let summary = format!(
        "{}: {}",
        entry.title,
        counts(findings.iter().map(|f| f.severity))
    );
    let report = AuditReport {
        rule: RuleRef {
            extension: run.extension,
            version: run.version,
            digest: run.digest,
            rule: entry.id.clone(),
            handler: entry.handler.clone(),
            title: entry.title.clone(),
        },
        drawing,
        facts: given,
        kentos: env!("CARGO_PKG_VERSION").to_owned(),
        findings,
        dropped,
        cut,
    };
    Ok(Processed {
        summary,
        details: serde_json::to_value(&report).unwrap_or_default(),
        ..Processed::default()
    })
}

#[cfg(test)]
mod mevzuat_tests;
#[cfg(test)]
mod tests;
