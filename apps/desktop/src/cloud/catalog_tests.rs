//! The catalog window message by message, without a server (docs/adr/0086):
//! its lists, filters and order, the selected project's details asked once
//! the selection rests, and the lifecycle's actions with their questions,
//! lines and the list asked again. Requests are dropped unrun; their
//! answers are handed to the app.

use std::time::{Duration, Instant};

use kentos_cloud::ApiFailure;
use kentos_contracts::{
    CatalogSort, CatalogView, ProjectCatalogChange, ProjectDetails, ProjectPermission,
    ProjectPurged, ProjectState, ProjectStorage, ProjectSummary, ProjectType,
};

use crate::app::{App, Dialog};
use crate::cloud::catalog::{Details, List, Step, Tab};
use crate::cloud::catalog_actions::{Act, Acted};
use crate::cloud::tests::{PROJECT, cloud, database, last_said, page, said, signed_in, summary};
use crate::cloud::{Event, plan};

const SECOND: &str = "0199aaaa-0000-7000-8000-00000000000b";

/// A project the account owns with every right.
fn owned(id: &str, name: &str) -> ProjectSummary {
    let mut p = summary(id, name, ProjectStorage::Database);
    p.access.permissions = ProjectPermission::ALL.to_vec();
    p
}

/// The catalog on `view`, its first page answered with `projects`.
fn listed(app: &mut App, view: CatalogView, projects: Vec<ProjectSummary>) {
    if app.cloud.catalog.is_none() {
        let _ = app.run("cloud.open");
    }
    cloud(app, Event::CatalogView(List::View(view)));
    let id = app
        .cloud
        .catalog
        .as_ref()
        .and_then(|c| c.request())
        .expect("asked");
    let total = u32::try_from(projects.len()).unwrap_or(0);
    let mut answer = page(projects, total, None);
    answer.trash_retention_days = 30;
    cloud(
        app,
        Event::CatalogPage {
            id,
            more: false,
            result: Ok(answer),
        },
    );
}

fn catalog(app: &App) -> &crate::cloud::Catalog {
    app.cloud.catalog.as_ref().expect("the window")
}

/// The action on its way: its id.
fn acting(app: &App) -> u64 {
    catalog(app)
        .acting
        .as_ref()
        .expect("an action on its way")
        .id
}

fn changed(p: ProjectSummary) -> Result<Acted, ApiFailure> {
    Ok(Acted::Changed(Box::new(ProjectCatalogChange {
        project: p,
        changed: true,
        event_seq: None,
        replayed: false,
    })))
}

#[test]
fn a_list_has_its_orders_types_and_the_trash() {
    let mut app = signed_in();
    listed(&mut app, CatalogView::Recent, Vec::new());
    assert_eq!(
        catalog(&app).sort,
        CatalogSort::Opened,
        "its own order first"
    );
    listed(&mut app, CatalogView::Trash, Vec::new());
    assert_eq!(catalog(&app).sort, CatalogSort::Trashed);
    assert_eq!(catalog(&app).retention, 30);
    let before = catalog(&app).request();
    cloud(&mut app, Event::CatalogSort(CatalogSort::Name));
    assert_eq!(catalog(&app).sort, CatalogSort::Name);
    assert_ne!(catalog(&app).request(), before, "asked again in that order");
    let before = catalog(&app).request();
    cloud(&mut app, Event::CatalogKind(Some(ProjectType::Gis)));
    assert_eq!(catalog(&app).kind, Some(ProjectType::Gis));
    assert_ne!(catalog(&app).request(), before, "asked again with the type");
    // Empty with a filter on: the web's words.
    let id = catalog(&app).request().expect("asked");
    cloud(
        &mut app,
        Event::CatalogPage {
            id,
            more: false,
            result: Ok(page(Vec::new(), 0, None)),
        },
    );
    assert_eq!(
        crate::cloud::catalog::empty_text(catalog(&app), 1),
        plan::EMPTY_SEARCH
    );
    // Another list starts with its own order again.
    listed(&mut app, CatalogView::Mine, Vec::new());
    assert_eq!(catalog(&app).sort, CatalogSort::Updated);
}

#[test]
fn the_list_keys_move_the_selection() {
    let mut app = signed_in();
    listed(
        &mut app,
        CatalogView::Mine,
        vec![owned(PROJECT, "Ada 101"), owned(SECOND, "Ada 102")],
    );
    cloud(&mut app, Event::CatalogStep(Step::Down));
    assert_eq!(catalog(&app).picked.as_deref(), Some(PROJECT), "the first");
    cloud(&mut app, Event::CatalogStep(Step::Down));
    cloud(&mut app, Event::CatalogStep(Step::Down));
    assert_eq!(
        catalog(&app).picked.as_deref(),
        Some(SECOND),
        "stays on the last"
    );
    cloud(&mut app, Event::CatalogStep(Step::Home));
    assert_eq!(catalog(&app).picked.as_deref(), Some(PROJECT));
    cloud(&mut app, Event::CatalogStep(Step::End));
    assert_eq!(catalog(&app).picked.as_deref(), Some(SECOND));
    cloud(&mut app, Event::CatalogStep(Step::Up));
    assert_eq!(catalog(&app).picked.as_deref(), Some(PROJECT));
}

#[test]
fn the_details_are_asked_once_the_selection_rests() {
    let mut app = signed_in();
    let mut trashed = owned(SECOND, "Eski");
    trashed.state = ProjectState::Trashed;
    listed(
        &mut app,
        CatalogView::Mine,
        vec![owned(PROJECT, "Ada 101"), trashed],
    );
    cloud(&mut app, Event::CatalogPick(PROJECT.into()));
    assert_eq!(catalog(&app).details, Details::Loading);
    let now = Instant::now();
    let _ = app.cloud_tick(now);
    assert_eq!(catalog(&app).details_request(), None, "not yet");
    let _ = app.cloud_tick(now + Duration::from_millis(200));
    let first = catalog(&app).details_request().expect("asked");
    // A late answer for another selection is dropped.
    cloud(&mut app, Event::CatalogPick(SECOND.into()));
    assert_eq!(
        catalog(&app).details,
        Details::None,
        "the trash is not counted"
    );
    cloud(&mut app, Event::CatalogPick(PROJECT.into()));
    let answer = ProjectDetails {
        project: owned(PROJECT, "Ada 101"),
        bounds: None,
        feature_count: "1284".into(),
        layer_count: 7,
    };
    cloud(
        &mut app,
        Event::CatalogDetails {
            id: first,
            result: Ok(Details::Database(Box::new(answer.clone()))),
        },
    );
    assert_eq!(
        catalog(&app).details,
        Details::Loading,
        "the old answer is dropped"
    );
    let _ = app.cloud_tick(Instant::now() + Duration::from_millis(200));
    let second = catalog(&app).details_request().expect("asked again");
    cloud(
        &mut app,
        Event::CatalogDetails {
            id: second,
            result: Ok(Details::Database(Box::new(answer))),
        },
    );
    assert!(matches!(catalog(&app).details, Details::Database(_)));
    // The same project picked again keeps them; the tab goes back to Bilgiler on another.
    cloud(&mut app, Event::CatalogTab(Tab::History));
    cloud(&mut app, Event::CatalogPick(PROJECT.into()));
    assert!(matches!(catalog(&app).details, Details::Database(_)));
    assert_eq!(catalog(&app).tab, Tab::History);
    cloud(&mut app, Event::CatalogPick(SECOND.into()));
    assert_eq!(catalog(&app).tab, Tab::Info);
}

#[test]
fn the_favourite_changes_in_place_and_leaves_the_favourites() {
    let mut app = signed_in();
    listed(&mut app, CatalogView::Mine, vec![owned(PROJECT, "Ada 101")]);
    cloud(&mut app, Event::CatalogPick(PROJECT.into()));
    cloud(&mut app, Event::CatalogAct(Act::Favorite));
    let id = acting(&app);
    let mut now = owned(PROJECT, "Ada 101");
    now.favorite = true;
    cloud(
        &mut app,
        Event::CatalogActed {
            id,
            result: changed(now),
        },
    );
    let c = catalog(&app);
    assert!(c.projects[0].favorite, "the row changed in place");
    assert_eq!(
        c.status.as_ref().map(|s| s.text.as_str()),
        Some("“Ada 101” favorilere eklendi.")
    );
    assert!(!c.loading(), "not asked again");
    // In “Favoriler”, taken out, it leaves the list.
    let mut fav = owned(PROJECT, "Ada 101");
    fav.favorite = true;
    listed(&mut app, CatalogView::Favorites, vec![fav]);
    cloud(&mut app, Event::CatalogPick(PROJECT.into()));
    cloud(&mut app, Event::CatalogAct(Act::Favorite));
    let id = acting(&app);
    cloud(
        &mut app,
        Event::CatalogActed {
            id,
            result: changed(owned(PROJECT, "Ada 101")),
        },
    );
    assert!(catalog(&app).loading(), "the list again");
    assert_eq!(
        catalog(&app).status.as_ref().map(|s| s.text.as_str()),
        Some("“Ada 101” favorilerden çıkarıldı.")
    );
}

#[test]
fn archive_trash_and_purge_ask_first_and_say_what_they_did() {
    let mut app = signed_in();
    listed(&mut app, CatalogView::Mine, vec![owned(PROJECT, "Ada 101")]);
    cloud(&mut app, Event::CatalogPick(PROJECT.into()));

    // Archiving: asked, declined, asked, confirmed.
    cloud(&mut app, Event::CatalogAct(Act::Archive));
    let (act, q) = app.catalog_question().expect("asked");
    assert_eq!((act, q.title), (Act::Archive, "Projeyi arşivle"));
    cloud(&mut app, Event::CatalogAnswer(false));
    assert!(catalog(&app).asking.is_none() && catalog(&app).acting.is_none());
    cloud(&mut app, Event::CatalogAct(Act::Archive));
    // Esc closes the question, not the window.
    app.close_dialog();
    assert_eq!(app.dialog, Some(Dialog::Catalog));
    assert!(catalog(&app).asking.is_none());
    cloud(&mut app, Event::CatalogAct(Act::Archive));
    cloud(&mut app, Event::CatalogAnswer(true));
    let id = acting(&app);
    let mut archived = owned(PROJECT, "Ada 101");
    archived.state = ProjectState::Archived;
    cloud(
        &mut app,
        Event::CatalogActed {
            id,
            result: changed(archived),
        },
    );
    assert_eq!(last_said(&app), "“Ada 101” arşivlendi.");
    assert_eq!(
        catalog(&app).status.as_ref().map(|s| s.text.as_str()),
        Some("“Ada 101” arşivlendi; Arşivlenmişler listesinde duruyor.")
    );
    assert!(catalog(&app).loading(), "the list again");

    // The trash: its question knows the retention; a refusal is said in the log.
    listed(&mut app, CatalogView::Mine, vec![owned(PROJECT, "Ada 101")]);
    cloud(&mut app, Event::CatalogPick(PROJECT.into()));
    cloud(&mut app, Event::CatalogAct(Act::Trash));
    let (_, q) = app.catalog_question().expect("asked");
    assert!(q.details[2].contains("30 gün içinde"), "{:?}", q.details);
    cloud(&mut app, Event::CatalogAnswer(true));
    let id = acting(&app);
    cloud(
        &mut app,
        Event::CatalogActed {
            id,
            result: Err(ApiFailure::new(
                403,
                "forbidden",
                "Bu işlem için yetkiniz yok.",
            )),
        },
    );
    assert_eq!(
        last_said(&app),
        "“Ada 101” çöp kutusuna taşınamadı: Bu işlem için yetkiniz yok."
    );
    cloud(&mut app, Event::CatalogAct(Act::Trash));
    cloud(&mut app, Event::CatalogAnswer(true));
    let id = acting(&app);
    let mut gone = owned(PROJECT, "Ada 101");
    gone.state = ProjectState::Trashed;
    gone.purge_after = Some("2026-10-25T13:00:00Z".into());
    cloud(
        &mut app,
        Event::CatalogActed {
            id,
            result: changed(gone.clone()),
        },
    );
    assert!(
        last_said(&app).starts_with("“Ada 101” çöp kutusuna taşındı; 25.10.2026"),
        "{}",
        last_said(&app)
    );

    // In the trash: removing for good asks, and says what went.
    listed(&mut app, CatalogView::Trash, vec![gone]);
    cloud(&mut app, Event::CatalogPick(PROJECT.into()));
    cloud(&mut app, Event::CatalogAct(Act::Purge));
    let (_, q) = app.catalog_question().expect("asked");
    assert_eq!(q.action, "Kalıcı olarak sil");
    cloud(&mut app, Event::CatalogAnswer(true));
    let id = acting(&app);
    cloud(
        &mut app,
        Event::CatalogActed {
            id,
            result: Ok(Acted::Purged(ProjectPurged {
                project_id: PROJECT.into(),
                name: "Ada 101".into(),
                objects: "1284".into(),
            })),
        },
    );
    assert_eq!(
        last_said(&app),
        "“Ada 101” kalıcı olarak silindi (1284 nesne)."
    );
}

#[test]
fn the_trash_restores_with_its_main_button_when_the_account_may() {
    let mut app = signed_in();
    let mut gone = owned(PROJECT, "Ada 101");
    gone.state = ProjectState::Trashed;
    let mut theirs = gone.clone();
    theirs.id = SECOND.into();
    theirs.access.permissions = vec![ProjectPermission::Read];
    listed(&mut app, CatalogView::Trash, vec![gone.clone(), theirs]);
    // Without project.delete the button stays off, whatever sends it.
    cloud(&mut app, Event::CatalogPick(SECOND.into()));
    cloud(&mut app, Event::CatalogOpen);
    assert!(catalog(&app).acting.is_none());
    cloud(&mut app, Event::CatalogPick(PROJECT.into()));
    cloud(&mut app, Event::CatalogOpen);
    let id = acting(&app);
    let mut back = gone;
    back.state = ProjectState::Active;
    cloud(
        &mut app,
        Event::CatalogActed {
            id,
            result: changed(back),
        },
    );
    assert_eq!(last_said(&app), "“Ada 101” çöp kutusundan geri yüklendi.");
    assert_eq!(
        catalog(&app).status.as_ref().map(|s| s.text.as_str()),
        Some("“Ada 101” geri yüklendi; listelerinde yeniden görünür.")
    );
    assert_eq!(app.dialog, Some(Dialog::Catalog), "the window stays");
}

#[test]
fn what_the_account_may_not_do_stays_off() {
    let mut app = signed_in();
    let mut viewer = summary(PROJECT, "Ada 101", ProjectStorage::Database);
    viewer.access.permissions = vec![ProjectPermission::Read];
    listed(&mut app, CatalogView::Shared, vec![viewer]);
    cloud(&mut app, Event::CatalogPick(PROJECT.into()));
    for act in [Act::Archive, Act::Trash, Act::Download] {
        cloud(&mut app, Event::CatalogAct(act));
        assert!(
            catalog(&app).asking.is_none() && catalog(&app).acting.is_none(),
            "{act:?}"
        );
    }
}

#[test]
fn a_download_asks_where_and_a_closed_window_does_nothing() {
    let mut app = signed_in();
    listed(&mut app, CatalogView::Mine, vec![owned(PROJECT, "Ada 101")]);
    cloud(&mut app, Event::CatalogPick(PROJECT.into()));
    cloud(&mut app, Event::CatalogAct(Act::Download));
    let id = acting(&app);
    cloud(&mut app, Event::CatalogDownloadTo { id, path: None });
    assert!(catalog(&app).acting.is_none(), "nothing happens");
    cloud(&mut app, Event::CatalogAct(Act::Download));
    let id = acting(&app);
    let path = std::env::temp_dir().join(format!("kentos-indir-{}.kcad", std::process::id()));
    cloud(
        &mut app,
        Event::CatalogDownloadTo {
            id,
            path: Some(path.clone()),
        },
    );
    cloud(
        &mut app,
        Event::CatalogDownloadProgress {
            id,
            done: 1536,
            total: 3 * 1024 * 1024,
        },
    );
    let (said_now, fraction) = catalog(&app)
        .acting
        .as_ref()
        .and_then(|a| a.progress.clone())
        .expect("its progress");
    assert_eq!(said_now, "İndiriliyor: 1,5 KB / 3 MB");
    assert!(fraction > 0.0 && fraction < 0.01);
    cloud(
        &mut app,
        Event::CatalogActed {
            id,
            result: Ok(Acted::Downloaded {
                path,
                size: 3 * 1024 * 1024,
                revision: Some("42".into()),
            }),
        },
    );
    assert!(
        said(&app).iter().any(|t| t
            == &format!(
                "“kentos-indir-{}.kcad” indirildi: 3 MB, revizyon 42.",
                std::process::id()
            )),
        "{:?}",
        said(&app)
    );
    assert_eq!(
        catalog(&app).status.as_ref().map(|s| s.text.as_str()),
        Some("“Ada 101” indirildi.")
    );
}

#[test]
fn archiving_the_open_project_here_says_it_once() {
    let mut app = signed_in();
    database(&mut app);
    listed(&mut app, CatalogView::Mine, vec![owned(PROJECT, "Ada 101")]);
    cloud(&mut app, Event::CatalogPick(PROJECT.into()));
    cloud(&mut app, Event::CatalogAct(Act::Archive));
    cloud(&mut app, Event::CatalogAnswer(true));
    // Nothing waited to be sent: the command went at once.
    let id = acting(&app);
    let mut archived = owned(PROJECT, "Ada 101");
    archived.state = ProjectState::Archived;
    cloud(
        &mut app,
        Event::CatalogActed {
            id,
            result: changed(archived),
        },
    );
    assert!(said(&app).iter().any(|t| t
        == "“Ada 101” arşivlendi: salt okunur. Değiştirmek için arşivden çıkarın; çizim ekranda kalıyor."));
    let source = app
        .document
        .as_ref()
        .and_then(|d| d.cloud_source())
        .expect("still the project");
    assert!(source.archived(), "read-only here");
    assert!(app.cloud.archived_by_me);
}

#[test]
fn trashing_the_open_project_here_leaves_it_with_the_drawing() {
    let mut app = signed_in();
    database(&mut app);
    listed(&mut app, CatalogView::Mine, vec![owned(PROJECT, "Ada 101")]);
    cloud(&mut app, Event::CatalogPick(PROJECT.into()));
    let (_, q) = {
        cloud(&mut app, Event::CatalogAct(Act::Trash));
        app.catalog_question().expect("asked")
    };
    assert_eq!(
        q.details.last().map(String::as_str),
        Some(
            "Proje şu anda sizde açık: çizim ekranda kalır, dilerseniz yerel bir dosyaya kaydedin."
        )
    );
    cloud(&mut app, Event::CatalogAnswer(true));
    let id = acting(&app);
    let mut gone = owned(PROJECT, "Ada 101");
    gone.state = ProjectState::Trashed;
    cloud(
        &mut app,
        Event::CatalogActed {
            id,
            result: changed(gone),
        },
    );
    let doc = app.document.as_ref().expect("the drawing stays");
    assert!(doc.cloud_source().is_none(), "the project is left");
    assert!(said(&app).iter().any(|t| {
        t.starts_with("“Ada 101” bulut projesi çöp kutusuna taşındı. Çizim ekranda kaldı")
    }));
}

/// The catalog as the web's reference pictures show it
/// (apps/web/scripts/e2e/cloud-shots.mjs): its lists, the selected
/// project's pane, the trash and a question, dark and light, at 1440×900
/// and 1100×650. Not run by default:
/// `cargo test -p kentos-desktop cloud::catalog_tests::screens -- --ignored --nocapture`.
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_contracts::{Bounds, ProjectRole};
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    let project = |n: u32, name: &str, kind: ProjectType, favorite: bool| {
        let mut p = owned(&format!("0199aaaa-0000-7000-8000-0000000001{n:02}"), name);
        p.project_type = kind;
        p.favorite = favorite;
        p.tenant_name = "Örnek Harita Bürosu".into();
        p.updated_at = "2026-09-27T09:54:00Z".into();
        p.created_at = "2026-09-27T09:54:00Z".into();
        p
    };
    let mut ada = project(7, "Ada 1246 ölçü projesi", ProjectType::Subdivision, true);
    ada.description =
        "1246 ada 1–3 parsellerin ifraz ölçüleri; belediye teslimine hazırlanıyor.".into();
    ada.tags = vec!["Kadıköy".into(), "ifraz".into(), "ölçü".into()];
    let mut second = project(
        3,
        "Arazi toplulaştırma (ikinci büro)",
        ProjectType::LandReadjustment,
        false,
    );
    second.tenant_name = "İkinci Harita Bürosu".into();
    let mine = vec![
        project(1, "Köy kadastro projesi", ProjectType::Cad, false),
        project(2, "Mahalle imar projesi", ProjectType::Cad, false),
        second,
        project(4, "Yol güzergâhı", ProjectType::Road, false),
        project(5, "İmar planı taslağı", ProjectType::ZoningPlan, true),
        project(6, "Kadastro paftası 2026", ProjectType::Cad, false),
        ada.clone(),
    ];
    let trashed = |n: u32, name: &str| {
        let mut p = project(n, name, ProjectType::Cad, false);
        p.state = ProjectState::Trashed;
        p.trashed_at = Some("2026-09-27T09:54:00Z".into());
        p.trashed_by_name = Some("Ayşe Yılmaz".into());
        p.purge_after = Some("2026-10-27T09:54:00Z".into());
        p
    };
    let trash = vec![
        trashed(8, "Eski pafta kopyası"),
        trashed(9, "Deneme çizimi"),
    ];
    let mut shared = project(10, "Mehmet'in aplikasyon işi", ProjectType::Cad, false);
    shared.owner_name = "Mehmet Kaya".into();
    shared.access.role = ProjectRole::Editor;
    let scenes = ["projelerim", "cop", "soru-cop", "paylasilan", "bos-arama"];
    for (mode, suffix) in [("dark", ""), ("light", "-acik")] {
        for (width, height) in [(1440.0, 900.0), (1100.0, 650.0)] {
            for scene in scenes {
                let mut app = signed_in();
                let _ = app
                    .settings
                    .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
                app.apply_settings();
                let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
                let mut update = |app: &mut App, message| {
                    let _ = app.update(message);
                };
                snapshot.settle(&mut app, App::view, &mut update);
                match scene {
                    "cop" => {
                        listed(&mut app, CatalogView::Trash, trash.clone());
                        cloud(&mut app, Event::CatalogPick(trash[0].id.clone()));
                    }
                    "paylasilan" => {
                        listed(&mut app, CatalogView::Shared, vec![shared.clone()]);
                        cloud(&mut app, Event::CatalogPick(shared.id.clone()));
                    }
                    "bos-arama" => {
                        listed(&mut app, CatalogView::Mine, Vec::new());
                        app.cloud.catalog.as_mut().expect("open").search = "zzz".into();
                    }
                    _ => {
                        listed(&mut app, CatalogView::Mine, mine.clone());
                        let pick = if scene == "soru-cop" { &mine[4] } else { &ada };
                        cloud(&mut app, Event::CatalogPick(pick.id.clone()));
                        let _ = app.cloud_tick(Instant::now() + Duration::from_millis(200));
                        if let Some(id) = catalog(&app).details_request() {
                            cloud(
                                &mut app,
                                Event::CatalogDetails {
                                    id,
                                    result: Ok(Details::Database(Box::new(ProjectDetails {
                                        project: pick.clone(),
                                        bounds: Some(Bounds {
                                            min_x: 486_490.0,
                                            min_y: 4_420_192.0,
                                            max_x: 486_570.0,
                                            max_y: 4_420_245.0,
                                        }),
                                        feature_count: "7".into(),
                                        layer_count: 3,
                                    }))),
                                },
                            );
                        }
                        if scene == "soru-cop" {
                            cloud(&mut app, Event::CatalogAct(Act::Trash));
                        }
                    }
                }
                snapshot.settle(&mut app, App::view, &mut update);
                let file = out.join(format!(
                    "bulut-katalog-{scene}-{width}x{height}{suffix}.png"
                ));
                snapshot
                    .render(app.view(), &app.theme())
                    .save(&file)
                    .expect("writes the picture");
                println!("{}", file.display());
            }
        }
    }
}
