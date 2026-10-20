//! A project's server jobs as the desktop shows and asks them (docs/adr/0547; the server's side is docs/adr/0542).
//!
//! The server keeps the jobs: a window or the app closed, the job goes on, and the next look finds it. What lives here
//! is only what the desktop decides without a pixel: the request for an export, the words for kinds, states and
//! failures, who may ask, stop and download, and when to look again. No UI, no request: the host calls the client
//! ([`crate::Cloud::server_jobs`] …) and draws these answers.

use std::time::Duration;

use kentos_contracts::{
    CheckpointCreate, PROJECT_CHECKPOINT_JOB, PROJECT_EXPORT_JOB, ProjectExportInput,
    ProjectPermission, ProjectState, ProjectStorage, ServerJob, ServerJobCreate, ServerJobState,
};
use uuid::Uuid;

/// While a job waits or runs the list is looked at this often.
pub const POLL_ACTIVE: Duration = Duration::from_secs(2);

/// A fresh id for one request: a retry of the same request sends the same id and gets the same job, so the id is
/// made once when the user asks and kept with the request.
pub fn request_id() -> String {
    format!("desktop-{}", Uuid::new_v4())
}

/// The request for “Projeyi dışa aktar (.kcad)”: the project as it is when the job runs, or only if it is still at
/// `expected` (a data revision) then.
pub fn export_request(request_id: &str, expected: Option<&str>) -> ServerJobCreate {
    let input = ProjectExportInput {
        expected_revision: expected.map(str::to_owned),
    };
    ServerJobCreate {
        kind: PROJECT_EXPORT_JOB.to_owned(),
        input: serde_json::to_value(input).unwrap_or_default(),
        request_id: request_id.to_owned(),
        timeout_seconds: None,
    }
}

/// The request for a checkpoint made as a job.
pub fn checkpoint_request(request_id: &str, name: &str, note: Option<&str>) -> ServerJobCreate {
    let input = CheckpointCreate {
        name: name.to_owned(),
        note: note.filter(|n| !n.trim().is_empty()).map(str::to_owned),
        file_revision: None,
    };
    ServerJobCreate {
        kind: PROJECT_CHECKPOINT_JOB.to_owned(),
        input: serde_json::to_value(input).unwrap_or_default(),
        request_id: request_id.to_owned(),
        timeout_seconds: None,
    }
}

/// What a job is, in a row's first line.
pub fn kind_label(job: &ServerJob) -> String {
    match job.kind.as_str() {
        PROJECT_EXPORT_JOB => "Projeyi dışa aktar (.kcad)".to_owned(),
        PROJECT_CHECKPOINT_JOB => {
            let name = job.input["name"].as_str().unwrap_or_default();
            if name.is_empty() {
                "Kontrol noktası oluştur".to_owned()
            } else {
                format!("Kontrol noktası: {name}")
            }
        }
        other => other.to_owned(),
    }
}

/// How a state reads in a chip.
pub fn state_label(state: ServerJobState) -> &'static str {
    match state {
        ServerJobState::Queued => "Sırada",
        ServerJobState::Running => "Çalışıyor",
        ServerJobState::Succeeded => "Bitti",
        ServerJobState::Failed => "Başarısız",
        ServerJobState::Cancelled => "Durduruldu",
    }
}

/// How far a running job is, as “%40”; none while it waits or after it ended.
pub fn progress_text(job: &ServerJob) -> Option<String> {
    (job.state == ServerJobState::Running).then(|| {
        let percent = (job.progress.clamp(0.0, 1.0) * 100.0).round();
        format!("%{percent:.0}")
    })
}

/// Why a job failed, in words: the server's own message, and what to do for the codes that have an advice.
pub fn failure_text(job: &ServerJob) -> Option<String> {
    let error = job.error.as_ref()?;
    let advice = match error.code.as_str() {
        "timeout" => {
            " İşi daha uzun bir süreyle yeniden isteyin ya da sunucu yöneticisine başvurun."
        }
        "lease_lost" => {
            " İşi çalıştıran sunucu işçisi birkaç kez kayboldu; işi yeniden isteyin, sürerse sunucu yöneticisine bildirin."
        }
        "revision_conflict" => " Proje, işin istendiği andan sonra değişti; işi yeniden isteyin.",
        "forbidden" => " Yetkiniz kaldırılmış ya da değişmiş olabilir; proje sahibine başvurun.",
        "storage" => {
            " Sunucunun dosya deposuna yazılamadı (yer ya da yetki); sunucu yöneticisine bildirin."
        }
        "too_large" => {
            " Sunucu yöneticisi dosya boyu sınırını yükseltmedikçe bu proje dışa aktarılamaz."
        }
        _ => "",
    };
    Some(format!("{}{advice}", error.message.trim_end()))
}

/// Whether anything of the list is still to change: the host looks again after [`POLL_ACTIVE`] while this is true.
pub fn any_active(jobs: &[ServerJob]) -> bool {
    jobs.iter().any(|j| !j.state.is_over())
}

/// When to look at the list again: soon while a job waits or runs, never on a timer when all are over.
pub fn poll_after(jobs: &[ServerJob]) -> Option<Duration> {
    any_active(jobs).then_some(POLL_ACTIVE)
}

fn missing(right: ProjectPermission, what: &str) -> String {
    format!(
        "Bu projede {what} yetkiniz yok ({}); proje sahibine ya da yöneticisine başvurun.",
        right.name()
    )
}

/// Why the account may not ask for an export here (`project.jobs.run` and `project.download`; a database project
/// that is not in the archive or the trash), or none. The server decides; this only keeps a button honest.
pub fn why_not_export(
    permissions: &[ProjectPermission],
    storage: ProjectStorage,
    state: ProjectState,
) -> Option<String> {
    if state == ProjectState::Trashed {
        return Some("Çöp kutusundaki projede iş istenmez.".to_owned());
    }
    if storage == ProjectStorage::File {
        return Some(
            "Dosya olarak saklanan projenin .kcad dosyası zaten revizyonlarıdır; Geçmiş sekmesinden indirin."
                .to_owned(),
        );
    }
    if !permissions.contains(&ProjectPermission::JobsRun) {
        return Some(missing(ProjectPermission::JobsRun, "sunucu işi isteme"));
    }
    if !permissions.contains(&ProjectPermission::Download) {
        return Some(missing(ProjectPermission::Download, "indirme"));
    }
    None
}

/// Why the account may not stop this job, or none: its asker or a holder of `project.edit` may, and only while it
/// waits or runs.
pub fn why_not_stop(
    job: &ServerJob,
    me: Option<&str>,
    permissions: &[ProjectPermission],
) -> Option<String> {
    if job.state.is_over() {
        return Some("İş zaten bitti.".to_owned());
    }
    if job.cancel_requested {
        return Some(
            "İşin durdurulması istendi; çalıştıran bir sonraki yoklamada durdurur.".to_owned(),
        );
    }
    let asker = me.is_some_and(|m| m == job.created_by);
    if !asker && !permissions.contains(&ProjectPermission::Edit) {
        return Some(
            "Bu işi yalnız isteyen ya da projeyi düzenleme yetkisi (project.edit) olan durdurabilir."
                .to_owned(),
        );
    }
    None
}

/// Why the job's file cannot be downloaded, or none: only a succeeded export has one, and the account needs
/// `project.download`.
pub fn why_not_download(job: &ServerJob, permissions: &[ProjectPermission]) -> Option<String> {
    if job.artifact.is_none() {
        return Some(match job.state {
            ServerJobState::Succeeded => "Bu işin indirilecek bir dosyası yok.".to_owned(),
            ServerJobState::Failed => "İş başarısız oldu; indirilecek dosya yok.".to_owned(),
            ServerJobState::Cancelled => "İş durduruldu; indirilecek dosya yok.".to_owned(),
            _ => "İş bitince indirilebilir.".to_owned(),
        });
    }
    if !permissions.contains(&ProjectPermission::Download) {
        return Some(missing(ProjectPermission::Download, "indirme"));
    }
    None
}

/// The file name a job's download is offered under: the server's, without its `.kcad` (the save window adds it).
pub fn download_name(job: &ServerJob) -> String {
    let name = job.artifact.as_ref().map_or("proje", |a| a.name.as_str());
    let base = name.strip_suffix(".kcad").unwrap_or(name);
    let revision = job.result.as_ref().and_then(|r| r["revision"].as_str());
    match revision {
        Some(r) => format!("{base} (veri revizyonu {r})"),
        None => base.to_owned(),
    }
}

/// The line under a job: how big its file is and which data revision it shows, or what it made.
pub fn result_text(job: &ServerJob) -> Option<String> {
    if job.state != ServerJobState::Succeeded {
        return None;
    }
    if let Some(a) = &job.artifact {
        let bytes: f64 = a.size.parse().unwrap_or(0.0);
        let revision = job.result.as_ref().and_then(|r| r["revision"].as_str());
        let objects = job.result.as_ref().and_then(|r| r["objects"].as_u64());
        let mut parts = vec![size_text(bytes)];
        if let Some(n) = objects {
            parts.push(format!("{n} nesne"));
        }
        if let Some(r) = revision {
            parts.push(format!("veri revizyonu {r}"));
        }
        return Some(parts.join(" · "));
    }
    job.result.as_ref().and_then(|r| {
        let name = r["checkpoint"]["name"].as_str()?;
        Some(format!("“{name}” kontrol noktası oluşturuldu"))
    })
}

/// “1,6 KB”, “3,2 MB”: a size the way the rest of the catalog says it (decimal comma, 1 024 steps).
pub fn size_text(bytes: f64) -> String {
    const UNITS: [&str; 4] = ["bayt", "KB", "MB", "GB"];
    let mut value = bytes.max(0.0);
    let mut unit = 0;
    while value >= 1024.0 && unit + 1 < UNITS.len() {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        return format!("{value:.0} {}", UNITS[0]);
    }
    format!("{value:.1} {}", UNITS[unit]).replace('.', ",")
}

#[cfg(test)]
mod tests {
    use super::*;
    use kentos_contracts::{ServerJobArtifact, ServerJobError};
    use serde_json::json;

    fn job(state: ServerJobState) -> ServerJob {
        ServerJob {
            id: Uuid::nil().to_string(),
            project: Uuid::nil().to_string(),
            kind: PROJECT_EXPORT_JOB.to_owned(),
            input: json!({}),
            input_sha256: "0".repeat(64),
            base_revision: "3".to_owned(),
            state,
            progress: 0.4,
            attempts: 1,
            max_attempts: 3,
            timeout_seconds: 600,
            cancel_requested: false,
            created_by: "u1".to_owned(),
            created_at: "2026-10-20T10:00:00Z".to_owned(),
            started_at: None,
            finished_at: None,
            result: None,
            artifact: None,
            error: None,
        }
    }

    #[test]
    fn requests_carry_the_request_id_and_the_typed_input() {
        let id = request_id();
        assert!(id.starts_with("desktop-") && id.len() > 20);
        assert_ne!(id, request_id(), "every ask is its own request");
        let r = export_request(&id, Some("7"));
        assert_eq!(r.kind, "project.export");
        assert_eq!(r.input, json!({ "expectedRevision": "7" }));
        assert_eq!(r.request_id, id);
        assert_eq!(export_request(&id, None).input, json!({}));
        let c = checkpoint_request(&id, "Teslim", Some("  "));
        assert_eq!(c.kind, "project.checkpoint");
        assert_eq!(c.input, json!({ "name": "Teslim" }));
        assert_eq!(
            checkpoint_request(&id, "Teslim", Some("ilk")).input,
            json!({ "name": "Teslim", "note": "ilk" })
        );
    }

    #[test]
    fn rows_are_worded_by_kind_state_and_progress() {
        let mut j = job(ServerJobState::Running);
        assert_eq!(kind_label(&j), "Projeyi dışa aktar (.kcad)");
        assert_eq!(progress_text(&j).as_deref(), Some("%40"));
        assert_eq!(state_label(j.state), "Çalışıyor");
        j.state = ServerJobState::Queued;
        assert_eq!(progress_text(&j), None);
        j.kind = PROJECT_CHECKPOINT_JOB.to_owned();
        j.input = json!({ "name": "Teslim" });
        assert_eq!(kind_label(&j), "Kontrol noktası: Teslim");
        assert_eq!(state_label(ServerJobState::Cancelled), "Durduruldu");
    }

    #[test]
    fn failures_are_the_servers_words_plus_an_advice_for_the_known_codes() {
        let mut j = job(ServerJobState::Failed);
        assert_eq!(failure_text(&j), None);
        j.error = Some(ServerJobError {
            code: "too_large".into(),
            message: "Projenin dosyası 9 bayt.".into(),
        });
        let text = failure_text(&j).unwrap();
        assert!(text.starts_with("Projenin dosyası 9 bayt. "), "{text}");
        assert!(text.contains("sınırını"), "{text}");
        j.error = Some(ServerJobError {
            code: "failed".into(),
            message: "Bir şey oldu.".into(),
        });
        assert_eq!(failure_text(&j).as_deref(), Some("Bir şey oldu."));
    }

    #[test]
    fn the_list_is_looked_at_again_only_while_something_is_under_way() {
        assert_eq!(poll_after(&[]), None);
        let done = job(ServerJobState::Succeeded);
        assert_eq!(
            poll_after(&[done.clone(), job(ServerJobState::Failed)]),
            None
        );
        assert_eq!(
            poll_after(&[done.clone(), job(ServerJobState::Queued)]),
            Some(POLL_ACTIVE)
        );
        assert_eq!(
            poll_after(&[job(ServerJobState::Running)]),
            Some(POLL_ACTIVE)
        );
    }

    #[test]
    fn who_may_ask_stop_and_download_with_the_reason_when_not() {
        use ProjectPermission::*;
        let editor = [Read, Download, History, FeatureWrite, JobsRun];
        let manager = [Read, Download, History, FeatureWrite, JobsRun, Edit];
        let viewer = [Read, Download, History];
        let db = ProjectStorage::Database;
        assert_eq!(why_not_export(&editor, db, ProjectState::Active), None);
        assert!(
            why_not_export(&viewer, db, ProjectState::Active)
                .unwrap()
                .contains("project.jobs.run")
        );
        assert!(
            why_not_export(&editor, ProjectStorage::File, ProjectState::Active)
                .unwrap()
                .contains("Geçmiş")
        );
        assert!(
            why_not_export(&editor, db, ProjectState::Trashed)
                .unwrap()
                .contains("Çöp")
        );
        assert!(
            why_not_export(&[JobsRun], db, ProjectState::Active)
                .unwrap()
                .contains("project.download")
        );

        let mut j = job(ServerJobState::Running);
        assert_eq!(why_not_stop(&j, Some("u1"), &viewer), None, "the asker");
        assert_eq!(
            why_not_stop(&j, Some("u2"), &manager),
            None,
            "a holder of project.edit"
        );
        assert!(
            why_not_stop(&j, Some("u2"), &editor)
                .unwrap()
                .contains("project.edit")
        );
        j.cancel_requested = true;
        assert!(
            why_not_stop(&j, Some("u1"), &manager)
                .unwrap()
                .contains("durdurulması istendi")
        );
        j.state = ServerJobState::Succeeded;
        assert_eq!(
            why_not_stop(&j, Some("u1"), &manager).as_deref(),
            Some("İş zaten bitti.")
        );

        let mut j = job(ServerJobState::Succeeded);
        assert!(
            why_not_download(&j, &editor)
                .unwrap()
                .contains("dosyası yok")
        );
        j.artifact = Some(ServerJobArtifact {
            size: "2048".into(),
            sha256: "0".repeat(64),
            name: "Ada 3.kcad".into(),
        });
        assert_eq!(why_not_download(&j, &editor), None);
        assert!(
            why_not_download(&j, &[Read])
                .unwrap()
                .contains("project.download")
        );
        let pending = job(ServerJobState::Queued);
        assert_eq!(
            why_not_download(&pending, &editor).as_deref(),
            Some("İş bitince indirilebilir.")
        );
    }

    #[test]
    fn a_finished_export_says_its_size_objects_and_revision() {
        let mut j = job(ServerJobState::Succeeded);
        j.artifact = Some(ServerJobArtifact {
            size: "1638".into(),
            sha256: "0".repeat(64),
            name: "Ada 3.kcad".into(),
        });
        j.result = Some(json!({ "revision": "4", "objects": 6, "eventCursor": "12" }));
        assert_eq!(
            result_text(&j).as_deref(),
            Some("1,6 KB · 6 nesne · veri revizyonu 4")
        );
        assert_eq!(download_name(&j), "Ada 3 (veri revizyonu 4)");
        assert_eq!(size_text(0.0), "0 bayt");
        assert_eq!(size_text(1023.0), "1023 bayt");
        assert_eq!(size_text(3.2 * 1024.0 * 1024.0), "3,2 MB");
        let mut c = job(ServerJobState::Succeeded);
        c.kind = PROJECT_CHECKPOINT_JOB.to_owned();
        c.result = Some(json!({ "checkpoint": { "name": "Teslim" } }));
        assert_eq!(
            result_text(&c).as_deref(),
            Some("“Teslim” kontrol noktası oluşturuldu")
        );
        assert_eq!(result_text(&job(ServerJobState::Failed)), None);
    }
}
