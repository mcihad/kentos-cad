//! The catalog's Sunucu işleri tab message by message, without a server (docs/adr/0547): the jobs asked when the
//! tab shows a project, polled while one waits or runs and let go of with the tab, “Projeyi dışa aktar” asked once
//! whatever the retries, Durdur, İndir as the catalog's download, what the account may not do and why, a job
//! that is still there when the window and the app were closed (the server keeps it, the next look lists it), and
//! the pictures:
//!
//! ```text
//! cargo test -p kentos-desktop --bin kentos-cad catalog_jobs_tests::screens -- --ignored --nocapture
//! ```
//!
//! They go to `.run/shots/isler-*.png`.

use std::time::{Duration, Instant};

use kentos_cloud::ApiFailure;
use kentos_commands::{
    CatalogView, ProjectPermission, ProjectStorage, ServerJob, ServerJobArtifact, ServerJobError,
    ServerJobList, ServerJobState,
};
use serde_json::json;

use crate::app::App;
use crate::cloud::Event;
use crate::cloud::catalog::Tab;
use crate::cloud::catalog_actions::Fetch;
use crate::cloud::catalog_jobs::{JobsState, Working};
use crate::cloud::catalog_tests::{SECOND, catalog, listed, owned};
use crate::cloud::tests::{PROJECT, cloud, signed_in};

/// A job's id: the server's are UUIDs (the client parses them).
fn jid(n: u8) -> String {
    format!("0199bbbb-0000-7000-8000-0000000000{n:02}")
}

fn job(id: &str, state: ServerJobState) -> ServerJob {
    ServerJob {
        id: id.into(),
        project: PROJECT.into(),
        kind: "project.export".into(),
        input: json!({}),
        input_sha256: "0".repeat(64),
        base_revision: "4".into(),
        state,
        progress: 0.0,
        progress_text: None,
        attempts: 1,
        max_attempts: 3,
        timeout_seconds: 600,
        cancel_requested: false,
        created_by: "0199aaaa-0000-7000-8000-00000000a1a1".into(),
        created_at: "2026-10-20T09:54:00Z".into(),
        started_at: None,
        finished_at: None,
        result: None,
        artifact: None,
        error: None,
    }
}

fn done(id: &str) -> ServerJob {
    ServerJob {
        finished_at: Some("2026-10-20T09:54:05Z".into()),
        progress: 1.0,
        result: Some(json!({ "revision": "4", "objects": 6, "eventCursor": "12" })),
        artifact: Some(ServerJobArtifact {
            size: "1638".into(),
            sha256: "c".repeat(64),
            name: "Ada 1246 ölçü projesi.kcad".into(),
        }),
        ..job(id, ServerJobState::Succeeded)
    }
}

/// The catalog on “Projelerim” with the project selected and its Sunucu işleri tab shown.
fn jobs_of(storage: ProjectStorage) -> App {
    let mut app = signed_in();
    let mut p = owned(PROJECT, "Ada 1246 ölçü projesi");
    p.storage = storage;
    listed(&mut app, CatalogView::Mine, vec![p, owned(SECOND, "Başka")]);
    cloud(&mut app, Event::CatalogPick(PROJECT.into()));
    cloud(&mut app, Event::CatalogTab(Tab::Jobs));
    app
}

fn asked(app: &App) -> u64 {
    catalog(app).jobs.request().expect("the jobs are asked")
}

fn answer(app: &mut App, jobs: Vec<ServerJob>) {
    let id = asked(app);
    cloud(
        app,
        Event::JobsLoaded {
            id,
            result: Ok(ServerJobList { jobs }),
        },
    );
}

fn acting(app: &App) -> u64 {
    catalog(app).jobs.acting().expect("a request on its way")
}

#[test]
fn the_tab_asks_the_jobs_polls_while_one_is_under_way_and_lets_go() {
    let mut app = jobs_of(ProjectStorage::Database);
    assert_eq!(catalog(&app).jobs.state, JobsState::Loading);
    let first = asked(&app);
    // Another project: the late answer is dropped.
    cloud(&mut app, Event::CatalogPick(SECOND.into()));
    cloud(
        &mut app,
        Event::JobsLoaded {
            id: first,
            result: Ok(ServerJobList {
                jobs: vec![done(&jid(0))],
            }),
        },
    );
    assert_eq!(catalog(&app).jobs.state, JobsState::Loading);
    cloud(&mut app, Event::CatalogPick(PROJECT.into()));
    // A job running: looked at again soon, the list staying until the answer.
    answer(
        &mut app,
        vec![job(&jid(1), ServerJobState::Running), done(&jid(0))],
    );
    assert!(catalog(&app).jobs.waits(), "a timer waits while a job runs");
    let _ = app.cloud_tick(Instant::now() + Duration::from_millis(100));
    assert!(catalog(&app).jobs.request().is_none(), "not yet");
    let _ = app.cloud_tick(Instant::now() + Duration::from_secs(3));
    assert!(catalog(&app).jobs.request().is_some(), "asked again");
    assert!(
        matches!(catalog(&app).jobs.state, JobsState::Loaded(_)),
        "quietly: the list stays until the answer"
    );
    // The job is over now: the log says so once and no timer waits.
    answer(&mut app, vec![done(&jid(1)), done(&jid(0))]);
    assert!(!catalog(&app).jobs.waits());
    assert_eq!(
        crate::cloud::tests::last_said(&app),
        "Sunucu işi bitti: Projeyi dışa aktar (.kcad)."
    );
    // A later look at the same list says nothing more.
    cloud(&mut app, Event::JobsRetry);
    answer(&mut app, vec![done(&jid(1)), done(&jid(0))]);
    let lines = crate::cloud::tests::said(&app)
        .iter()
        .filter(|l| l.starts_with("Sunucu işi bitti"))
        .count();
    assert_eq!(lines, 1);
    // Bilgiler: the tab lets go of it.
    cloud(&mut app, Event::CatalogTab(Tab::Info));
    assert_eq!(catalog(&app).jobs.state, JobsState::None);
    assert!(!catalog(&app).jobs.waits());
}

#[test]
fn a_poll_that_fails_keeps_the_list_and_tries_again() {
    let mut app = jobs_of(ProjectStorage::Database);
    answer(&mut app, vec![job(&jid(1), ServerJobState::Queued)]);
    let _ = app.cloud_tick(Instant::now() + Duration::from_secs(3));
    let id = asked(&app);
    cloud(
        &mut app,
        Event::JobsLoaded {
            id,
            result: Err(ApiFailure::new(0, "network", "Sunucuya ulaşılamadı.")),
        },
    );
    let c = catalog(&app);
    assert_eq!(c.jobs.list().len(), 1, "the list stays");
    assert!(
        c.jobs
            .status
            .as_ref()
            .is_some_and(|s| s.error && s.text.contains("yeniden denenecek"))
    );
    assert!(c.jobs.waits());
    // The first look failing is a failure of the tab, with its retry.
    let mut app = jobs_of(ProjectStorage::Database);
    let id = asked(&app);
    cloud(
        &mut app,
        Event::JobsLoaded {
            id,
            result: Err(ApiFailure::new(500, "internal", "Sunucuda bir hata oldu.")),
        },
    );
    assert_eq!(
        catalog(&app).jobs.state,
        JobsState::Failed("Sunucuda bir hata oldu.".into())
    );
}

#[test]
fn an_export_is_asked_once_whatever_the_retries_and_its_job_is_listed_at_once() {
    let mut app = jobs_of(ProjectStorage::Database);
    answer(&mut app, Vec::new());
    cloud(&mut app, Event::JobExport);
    assert_eq!(catalog(&app).jobs.working, Some(Working::Asking));
    let request = catalog(&app)
        .jobs
        .pending_request()
        .expect("the request id is kept")
        .to_owned();
    // A second click while the request is on its way asks nothing.
    let first = acting(&app);
    cloud(&mut app, Event::JobExport);
    assert_eq!(acting(&app), first);
    // The ask failed (no answer): the next ask carries the same request id, so the server gives the first job back.
    cloud(
        &mut app,
        Event::JobAsked {
            id: first,
            result: Err(ApiFailure::new(0, "network", "Sunucuya ulaşılamadı.")),
        },
    );
    assert!(catalog(&app).jobs.working.is_none());
    assert!(catalog(&app).jobs.status.as_ref().is_some_and(|s| s.error));
    cloud(&mut app, Event::JobExport);
    assert_eq!(catalog(&app).jobs.pending_request(), Some(request.as_str()));
    // It was made: the job is first in the list, the request id is let go, a timer polls it.
    let second = acting(&app);
    cloud(
        &mut app,
        Event::JobAsked {
            id: second,
            result: Ok(job(&jid(1), ServerJobState::Queued)),
        },
    );
    let c = catalog(&app);
    assert_eq!(
        c.jobs
            .list()
            .iter()
            .map(|j| j.id.as_str())
            .collect::<Vec<_>>(),
        [jid(1).as_str()]
    );
    assert!(c.jobs.pending_request().is_none());
    assert!(c.jobs.waits());
    assert!(
        c.jobs
            .status
            .as_ref()
            .is_some_and(|s| !s.error && s.text.contains("Pencereyi kapatsanız da sürer"))
    );
    // The next export is a new request.
    cloud(&mut app, Event::JobExport);
    assert_ne!(catalog(&app).jobs.pending_request(), Some(request.as_str()));
}

#[test]
fn the_organisations_limit_is_said_in_the_servers_words() {
    let mut app = jobs_of(ProjectStorage::Database);
    answer(&mut app, Vec::new());
    cloud(&mut app, Event::JobExport);
    let id = acting(&app);
    let mut refused = ApiFailure::new(
        429,
        "quota_exceeded",
        "Kurumunuzun sırada ve çalışan iş sınırına (50) ulaşıldı; bir iş bitince ya da durdurulunca yeniden isteyin.",
    );
    refused = refused.with_retryable(true);
    cloud(
        &mut app,
        Event::JobAsked {
            id,
            result: Err(refused),
        },
    );
    let said = catalog(&app).jobs.status.clone().expect("said");
    assert!(said.error);
    assert!(said.text.contains("iş sınırına (50)"), "{}", said.text);
    // Nothing was made, and the request id stays for a retry.
    assert!(catalog(&app).jobs.list().is_empty());
    assert!(catalog(&app).jobs.pending_request().is_some());
}

#[test]
fn a_job_is_stopped_by_its_asker_or_a_holder_of_project_edit() {
    let mut app = jobs_of(ProjectStorage::Database);
    answer(
        &mut app,
        vec![job(&jid(1), ServerJobState::Running), done(&jid(0))],
    );
    // A finished job cannot be stopped.
    cloud(&mut app, Event::JobStop(jid(0)));
    assert!(catalog(&app).jobs.acting().is_none());
    cloud(&mut app, Event::JobStop(jid(1)));
    assert_eq!(catalog(&app).jobs.working, Some(Working::Stopping(jid(1))));
    let id = acting(&app);
    // The server marked it; a running job is stopped at its worker's next beat.
    cloud(
        &mut app,
        Event::JobStopped {
            id,
            result: Ok(ServerJob {
                cancel_requested: true,
                ..job(&jid(1), ServerJobState::Running)
            }),
        },
    );
    assert!(catalog(&app).jobs.working.is_none());
    assert!(catalog(&app).jobs.list()[0].cancel_requested);
    assert!(
        catalog(&app)
            .jobs
            .status
            .as_ref()
            .is_some_and(|s| s.text.contains("Durdurma istendi"))
    );
    assert!(
        catalog(&app).jobs.waits(),
        "looked at until the worker stops it"
    );
    // Asked again while the stop is pending: nothing.
    cloud(&mut app, Event::JobStop(jid(1)));
    assert!(catalog(&app).jobs.acting().is_none());
    // Someone else's job, without project.edit: not offered.
    let other = ServerJob {
        created_by: "someone-else".into(),
        ..job(&jid(2), ServerJobState::Queued)
    };
    answer_more(&mut app, other);
    app.cloud
        .catalog
        .as_mut()
        .and_then(|c| c.projects.iter_mut().find(|p| p.id == PROJECT))
        .expect("listed")
        .access
        .permissions = vec![
        ProjectPermission::Read,
        ProjectPermission::Download,
        ProjectPermission::FeatureWrite,
        ProjectPermission::JobsRun,
    ];
    cloud(&mut app, Event::JobStop(jid(2)));
    assert!(catalog(&app).jobs.acting().is_none());
    // A refusal of the server is said in its words.
    app.cloud
        .catalog
        .as_mut()
        .and_then(|c| c.projects.iter_mut().find(|p| p.id == PROJECT))
        .expect("listed")
        .access
        .permissions = ProjectPermission::ALL.to_vec();
    cloud(&mut app, Event::JobStop(jid(2)));
    let id = acting(&app);
    cloud(
        &mut app,
        Event::JobStopped {
            id,
            result: Err(ApiFailure::new(
                403,
                "forbidden",
                "Bu işi yalnız isteyen durdurabilir.",
            )),
        },
    );
    assert!(
        catalog(&app)
            .jobs
            .status
            .as_ref()
            .is_some_and(|s| s.error && s.text.contains("yalnız isteyen"))
    );
}

/// A job added to the list as a poll would bring it.
fn answer_more(app: &mut App, extra: ServerJob) {
    cloud(app, Event::JobsRetry);
    let mut jobs = catalog(app).jobs.list().to_vec();
    jobs.push(extra);
    answer(app, jobs);
}

#[test]
fn a_finished_export_downloads_as_the_catalogs_download_and_others_do_not() {
    let mut app = jobs_of(ProjectStorage::Database);
    answer(
        &mut app,
        vec![
            job(&jid(2), ServerJobState::Running),
            ServerJob {
                error: Some(ServerJobError {
                    code: "too_large".into(),
                    message: "Projenin dosyası çok büyük.".into(),
                }),
                ..job(&jid(1), ServerJobState::Failed)
            },
            done(&jid(0)),
        ],
    );
    // A running or a failed job has no file.
    cloud(
        &mut app,
        Event::JobDownload(job(&jid(2), ServerJobState::Running)),
    );
    assert!(catalog(&app).acting.is_none());
    cloud(
        &mut app,
        Event::JobDownload(job(&jid(1), ServerJobState::Failed)),
    );
    assert!(catalog(&app).acting.is_none());
    cloud(&mut app, Event::JobDownload(done(&jid(0))));
    let a = catalog(&app).acting.as_ref().expect("a download");
    let d = a.download.as_ref().expect("what it fetches");
    assert_eq!(
        (&d.fetch, d.name.as_str(), d.listed.as_deref()),
        (
            &Fetch::Job(jid(0)),
            "Ada 1246 ölçü projesi (veri revizyonu 4)",
            Some("c".repeat(64).as_str())
        )
    );
    // The save window closed: nothing happens.
    let id = a.id;
    cloud(&mut app, Event::CatalogDownloadTo { id, path: None });
    assert!(catalog(&app).acting.is_none());
    // Without project.download: not offered.
    app.cloud
        .catalog
        .as_mut()
        .and_then(|c| c.projects.iter_mut().find(|p| p.id == PROJECT))
        .expect("listed")
        .access
        .permissions = vec![ProjectPermission::Read, ProjectPermission::JobsRun];
    cloud(&mut app, Event::JobDownload(done(&jid(0))));
    assert!(catalog(&app).acting.is_none());
}

#[test]
fn a_file_project_and_an_account_without_the_right_are_told_why_and_ask_nothing() {
    let mut app = jobs_of(ProjectStorage::File);
    answer(&mut app, Vec::new());
    cloud(&mut app, Event::JobExport);
    assert!(
        catalog(&app).jobs.working.is_none(),
        "a file project keeps its revisions as files"
    );
    let mut app = jobs_of(ProjectStorage::Database);
    answer(&mut app, Vec::new());
    app.cloud
        .catalog
        .as_mut()
        .and_then(|c| c.projects.iter_mut().find(|p| p.id == PROJECT))
        .expect("listed")
        .access
        .permissions = vec![ProjectPermission::Read, ProjectPermission::Download];
    cloud(&mut app, Event::JobExport);
    assert!(
        catalog(&app).jobs.working.is_none(),
        "without project.jobs.run"
    );
}

#[test]
fn a_job_is_still_there_when_the_window_and_the_app_were_closed() {
    // The first app asks for the job; the server has it (the answer the second look will give).
    let mut app = jobs_of(ProjectStorage::Database);
    answer(&mut app, Vec::new());
    cloud(&mut app, Event::JobExport);
    let id = acting(&app);
    let running = ServerJob {
        progress: 0.3,
        ..job(&jid(1), ServerJobState::Running)
    };
    cloud(
        &mut app,
        Event::JobAsked {
            id,
            result: Ok(job(&jid(1), ServerJobState::Queued)),
        },
    );
    drop(app);
    // A new app, the same account: the tab lists the job the server kept, and goes on polling it.
    let mut app = jobs_of(ProjectStorage::Database);
    answer(&mut app, vec![running]);
    assert_eq!(catalog(&app).jobs.list().len(), 1);
    assert_eq!(catalog(&app).jobs.list()[0].state, ServerJobState::Running);
    assert!(catalog(&app).jobs.waits());
}

mod screens {
    use super::*;
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    fn shot(snapshot: &mut Snapshot, app: &mut App, name: &str) {
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        snapshot.settle(app, App::view, &mut update);
        let _ = snapshot.render(app.view(), &app.theme());
        app.sync_device();
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
        std::fs::create_dir_all(&dir).expect("a folder");
        let path = dir.join(format!("isler-{name}.png"));
        snapshot
            .render(app.view(), &app.theme())
            .save(&path)
            .expect("the image is written");
        println!("görüntü: {}", path.display());
    }

    #[test]
    #[ignore = "pictures"]
    fn the_tab_in_its_states() {
        let mut snapshot = Snapshot::new(Size::new(1440.0, 900.0)).expect("a renderer");
        let mut app = jobs_of(ProjectStorage::Database);
        shot(&mut snapshot, &mut app, "yukleniyor");
        answer(&mut app, Vec::new());
        shot(&mut snapshot, &mut app, "bos");
        let failed = ServerJob {
            finished_at: Some("2026-10-20T09:40:30Z".into()),
            created_at: "2026-10-20T09:40:00Z".into(),
            attempts: 1,
            error: Some(ServerJobError {
                code: "too_large".into(),
                message: "Projenin dosyası 3 221 225 472 bayt; sunucunun bir işe verdiği sınır 2 147 483 648 bayt.".into(),
            }),
            ..job(&jid(4), ServerJobState::Failed)
        };
        let checkpoint = ServerJob {
            kind: "project.checkpoint".into(),
            input: json!({ "name": "Belediyeye teslim" }),
            finished_at: Some("2026-10-20T09:30:02Z".into()),
            created_at: "2026-10-20T09:30:00Z".into(),
            result: Some(json!({ "checkpoint": { "name": "Belediyeye teslim" } })),
            ..job(&jid(5), ServerJobState::Succeeded)
        };
        answer_more_shot(
            &mut app,
            vec![
                ServerJob {
                    created_at: "2026-10-20T10:06:00Z".into(),
                    ..job(&jid(1), ServerJobState::Queued)
                },
                ServerJob {
                    created_at: "2026-10-20T10:05:00Z".into(),
                    progress: 0.4,
                    started_at: Some("2026-10-20T10:05:01Z".into()),
                    ..job(&jid(2), ServerJobState::Running)
                },
                ServerJob {
                    created_at: "2026-10-20T09:54:00Z".into(),
                    ..done(&jid(3))
                },
                failed,
                checkpoint,
                ServerJob {
                    created_at: "2026-10-20T09:00:00Z".into(),
                    finished_at: Some("2026-10-20T09:00:09Z".into()),
                    ..job(&jid(6), ServerJobState::Cancelled)
                },
            ],
        );
        shot(&mut snapshot, &mut app, "liste");
        // A viewer: Projeyi dışa aktar and Durdur are off, İndir stays.
        app.cloud
            .catalog
            .as_mut()
            .and_then(|c| c.projects.iter_mut().find(|p| p.id == PROJECT))
            .expect("listed")
            .access
            .permissions = vec![
            ProjectPermission::Read,
            ProjectPermission::Download,
            ProjectPermission::History,
        ];
        shot(&mut snapshot, &mut app, "izleyici");
    }

    /// The tab with the jobs İşlemler's tools made (docs/adr/0559): one running with the words its worker says, one
    /// committed to the project, one that wrote a file (İndir), one stopped at its CPU budget, one refused because the
    /// project moved.
    #[test]
    #[ignore = "pictures"]
    fn the_tab_with_processing_jobs() {
        for (mode, suffix) in [("dark", ""), ("light", "-acik")] {
            for (width, height) in [(1440.0, 900.0), (1100.0, 650.0)] {
                processing_jobs_shot(mode, suffix, width, height);
            }
        }
    }

    fn processing_jobs_shot(mode: &str, suffix: &str, width: f32, height: f32) {
        let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
        let mut app = jobs_of(ProjectStorage::Database);
        let _ = app
            .settings
            .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
        app.apply_settings();
        answer(&mut app, Vec::new());
        let tool = |n: u8, label: &str, mode: &str| ServerJob {
            kind: "project.processing".into(),
            input: json!({ "tool": "geometry.buffer", "label": label, "mode": mode }),
            created_at: format!("2026-10-20T10:{:02}:00Z", 10 - n),
            ..job(&jid(n), ServerJobState::Queued)
        };
        let usage = json!({ "cpuSeconds": 12.4, "peakMemoryMb": 612.0 });
        let running = ServerJob {
            progress: 0.62,
            progress_text: Some("Araç çalışıyor: Tampon".into()),
            started_at: Some("2026-10-20T10:09:02Z".into()),
            state: ServerJobState::Running,
            ..tool(1, "Tampon", "commit")
        };
        let committed = ServerJob {
            state: ServerJobState::Succeeded,
            progress: 1.0,
            finished_at: Some("2026-10-20T10:07:41Z".into()),
            result: Some(json!({
                "tool": "attributes.calculate", "mode": "commit", "baseRevision": "11", "revision": "12",
                "summary": "412 nesnede “Hesap alanı” yazıldı.", "added": 0, "changed": 412, "removed": 0,
                "usage": usage
            })),
            ..tool(2, "Öznitelik hesapla", "commit")
        };
        let filed = ServerJob {
            state: ServerJobState::Succeeded,
            progress: 1.0,
            finished_at: Some("2026-10-20T10:05:12Z".into()),
            artifact: Some(ServerJobArtifact {
                size: "2467840".into(),
                sha256: "d".repeat(64),
                name: "Ada 1246 ölçü projesi - Kesişim.gpkg".into(),
            }),
            result: Some(json!({
                "tool": "geometry.intersection", "mode": "sink", "baseRevision": "11",
                "summary": "1 284 parça yazıldı.", "added": 1284, "changed": 0, "removed": 0,
                "usage": usage
            })),
            ..tool(3, "Kesişim", "sink")
        };
        let stopped = ServerJob {
            state: ServerJobState::Failed,
            finished_at: Some("2026-10-20T10:03:30Z".into()),
            error: Some(ServerJobError {
                code: "cpu_limit".into(),
                message: "Araç 1800 sn işlemci süresi kullandı; bu işin işlemci bütçesi 1800 sn. İş durduruldu, hiçbir şey yazılmadı. Girdiyi daraltın ya da sunucu yöneticisinden bütçeyi artırmasını isteyin.".into(),
            }),
            ..tool(4, "Tampon", "commit")
        };
        let moved = ServerJob {
            state: ServerJobState::Failed,
            finished_at: Some("2026-10-20T10:01:20Z".into()),
            error: Some(ServerJobError {
                code: "revision_conflict".into(),
                message: "Proje 11. revizyonda değil (12); değişiklikler yazılmadı.".into(),
            }),
            ..tool(5, "Tampon", "commit")
        };
        answer_more_shot(&mut app, vec![running, committed, filed, stopped, moved]);
        shot(
            &mut snapshot,
            &mut app,
            &format!("islem-isleri-{width}x{height}{suffix}"),
        );
        // The details column scrolled down: the rows below.
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        snapshot.step(
            &mut app,
            App::view,
            &mut update,
            &[
                iced::Event::Mouse(iced::mouse::Event::CursorMoved {
                    position: iced::Point::new(width * 0.76, height * 0.6),
                }),
                iced::Event::Mouse(iced::mouse::Event::WheelScrolled {
                    delta: iced::mouse::ScrollDelta::Lines { x: 0.0, y: -12.0 },
                }),
            ],
        );
        shot(
            &mut snapshot,
            &mut app,
            &format!("islem-isleri-altta-{width}x{height}{suffix}"),
        );
    }

    /// The tab with an import and an atlas as jobs (docs/adr/0561): an import running with the words its worker says, one
    /// committed with its warnings counted, one the command refused (another coordinate system), an atlas that made its
    /// PDF (İndir) and one refused because the project moved on from the pinned state.
    #[test]
    #[ignore = "pictures"]
    fn the_tab_with_import_and_atlas_jobs() {
        for (mode, suffix) in [("dark", ""), ("light", "-acik")] {
            for (width, height) in [(1440.0, 900.0), (1100.0, 650.0)] {
                import_atlas_shot(mode, suffix, width, height);
            }
        }
    }

    fn import_atlas_jobs() -> Vec<ServerJob> {
        let import = |n: u8, label: &str| ServerJob {
            kind: "project.import".into(),
            input: json!({ "upload": "0199aaaa-0000-7000-8000-000000000009", "label": label }),
            created_at: format!("2026-10-20T10:{:02}:00Z", 10 - n),
            ..job(&jid(n), ServerJobState::Queued)
        };
        let atlas = |n: u8, label: &str| ServerJob {
            kind: "project.atlas".into(),
            input: json!({ "sheet": "atlas-paftasi", "label": label }),
            created_at: format!("2026-10-20T10:{:02}:00Z", 10 - n),
            ..job(&jid(n), ServerJobState::Queued)
        };
        let usage = json!({ "cpuSeconds": 41.2, "peakMemoryMb": 1480.0 });
        let running = ServerJob {
            progress: 0.45,
            progress_text: Some("Dosya okunuyor: parseller.gpkg".into()),
            started_at: Some("2026-10-20T10:09:02Z".into()),
            state: ServerJobState::Running,
            ..import(1, "parseller.gpkg")
        };
        let done = ServerJob {
            state: ServerJobState::Succeeded,
            progress: 1.0,
            finished_at: Some("2026-10-20T10:07:41Z".into()),
            result: Some(json!({
                "command": "file.import.gpkg", "file": "binalar.gpkg", "bytes": 48_234_496, "sha256": "e".repeat(64),
                "baseRevision": "11", "revision": "12",
                "summary": "“binalar.gpkg” içe aktarıldı: 38 412 nesne eklendi, 2 katman açıldı.",
                "added": 38412, "changed": 0, "removed": 0, "metadata": true, "blocks": 0,
                "notes": [{ "level": "warn", "text": "import_losses: 3 satırın geometrisi okunamadı" }],
                "details": {}, "usage": usage
            })),
            ..import(2, "binalar.gpkg")
        };
        let refused = ServerJob {
            state: ServerJobState::Failed,
            finished_at: Some("2026-10-20T10:05:30Z".into()),
            error: Some(ServerJobError {
                code: "crs_mismatch".into(),
                message: "yollar.gpkg EPSG:4326 sisteminde, proje EPSG:5256. Koordinatların projenin sistemine çevrilmesini isteyin.".into(),
            }),
            ..import(3, "yollar.gpkg")
        };
        let pdf = ServerJob {
            state: ServerJobState::Succeeded,
            progress: 1.0,
            finished_at: Some("2026-10-20T10:04:12Z".into()),
            artifact: Some(ServerJobArtifact {
                size: "3407872".into(),
                sha256: "f".repeat(64),
                name: "Ada 1246 - Parsel atlası.pdf".into(),
            }),
            result: Some(json!({
                "sheet": "atlas-paftasi", "baseRevision": "11", "pages": 100, "first": "A1/1", "last": "A4/25",
                "pinDigest": "ab".repeat(32), "bindings": { "srid": 5253 }, "fields": ["Ada", "Parsel"],
                "withheld": ["Malik"], "usage": usage
            })),
            ..atlas(4, "Parsel atlası")
        };
        let pinned = ServerJob {
            state: ServerJobState::Failed,
            finished_at: Some("2026-10-20T10:02:20Z".into()),
            error: Some(ServerJobError {
                code: "pin_mismatch".into(),
                message: "Çizim sabitlenen durumdan değişti: katmanlar farklı.".into(),
            }),
            ..atlas(5, "Parsel atlası")
        };
        vec![running, done, refused, pdf, pinned]
    }

    fn import_atlas_shot(mode: &str, suffix: &str, width: f32, height: f32) {
        let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
        let mut app = jobs_of(ProjectStorage::Database);
        let _ = app
            .settings
            .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
        app.apply_settings();
        answer(&mut app, Vec::new());
        answer_more_shot(&mut app, import_atlas_jobs());
        shot(
            &mut snapshot,
            &mut app,
            &format!("ice-aktar-atlas-isleri-{width}x{height}{suffix}"),
        );
    }

    #[test]
    fn the_rows_of_an_import_and_an_atlas_are_worded_and_the_pdf_has_its_own_name() {
        use kentos_cloud::server_jobs as words;
        let jobs = import_atlas_jobs();
        assert_eq!(words::kind_label(&jobs[0]), "İçe aktar: parseller.gpkg");
        assert_eq!(
            words::progress_text(&jobs[0]).as_deref(),
            Some("%45 · Dosya okunuyor: parseller.gpkg")
        );
        assert!(
            words::result_text(&jobs[1])
                .unwrap()
                .contains("38 412 nesne eklendi")
        );
        assert!(
            words::result_text(&jobs[1])
                .unwrap()
                .contains("1 uyarı (ayrıntı iş sonucunda)")
        );
        assert!(
            words::failure_text(&jobs[2])
                .unwrap()
                .contains("Projenin sistemine çevir")
        );
        assert_eq!(words::kind_label(&jobs[3]), "Atlas (PDF): Parsel atlası");
        assert_eq!(
            words::result_text(&jobs[3]).as_deref(),
            Some(
                "100 sayfa · 3,2 MB · veri revizyonu 11 · 1 alan taşınmadı · işlemci 41,2 sn, bellek 1480 MB"
            )
        );
        assert_eq!(words::download_extension(&jobs[3]), "pdf");
        assert_eq!(
            words::download_name(&jobs[3]),
            "Ada 1246 - Parsel atlası (veri revizyonu 11)"
        );
        assert!(
            words::failure_text(&jobs[4])
                .unwrap()
                .contains("atlas planını yeniden")
        );
        // A job's file is saved under its own extension: a drawing's .kcad, a tool's .gpkg, an atlas's .pdf.
        let mut tool_job = jobs[3].clone();
        tool_job.artifact.as_mut().unwrap().name = "Ada - Kesişim.gpkg".into();
        assert_eq!(words::download_extension(&tool_job), "gpkg");
        tool_job.artifact.as_mut().unwrap().name = "Ada.kcad".into();
        assert_eq!(words::download_extension(&tool_job), "kcad");
    }

    fn answer_more_shot(app: &mut App, jobs: Vec<ServerJob>) {
        cloud(app, Event::JobsRetry);
        answer(app, jobs);
    }
}
