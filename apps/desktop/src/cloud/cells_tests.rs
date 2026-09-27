//! The status bar's cloud cells in the app (docs/adr/0113): the save cell's
//! words, lamp and click through a file project's Kaydet, the server cell's
//! answer, and the account menu's rows with the rights a viewer lacks.

use kentos_contracts::{CONTRACTS_VERSION, Health, ProjectState, ProjectStorage};

use crate::app::{App, Message};
use crate::cloud::cells::{Lamp, Shade};
use crate::cloud::cells_plan::{AccountRow, ServerState};
use crate::cloud::tests::{cell_text, database, edit, file_project, info, open, said, signed_in};
use crate::saving::Stage;

fn health(contracts: u32) -> Health {
    Health {
        status: "ok".into(),
        service: "kentosd".into(),
        version: "0.9.0".into(),
        commit: Some("4d67c97af7fe8fa6".into()),
        contracts,
    }
}

#[test]
fn a_file_projects_kaydet_is_said_stage_by_stage() {
    let mut app = signed_in();
    file_project(&mut app);
    let cell = app.save_cell().expect("a cloud project");
    assert_eq!(cell.view.text, "Buluta kaydedildi · r3");
    assert_eq!((cell.shade, cell.lamp), (Shade::Plain, Lamp::Good));
    assert_eq!(cell.action, Some("file.save"));
    assert!(
        cell.tip
            .description
            .contains("Çizimin dayandığı revizyon: 3.")
    );
    edit(&mut app, 1.0);
    let _ = app.update(Message::Modifiers(iced::keyboard::Modifiers::default()));
    assert_eq!(cell_text(&app), "Kaydedilmedi · r3 üstüne");
    let _ = app.run("file.save");
    assert_eq!(cell_text(&app), "Dosya hazırlanıyor…");
    assert_eq!(
        app.save_cell().and_then(|c| c.action),
        None,
        "nothing to do while it saves"
    );
    let id = app.saving.as_ref().expect("saving").id;
    let _ = app.update(Message::Saving(crate::saving::Event::Progress {
        id,
        stage: Stage::Uploading {
            done: 426,
            total: 1000,
        },
    }));
    assert_eq!(cell_text(&app), "Yükleniyor %43");
    let _ = app.update(Message::Saving(crate::saving::Event::Progress {
        id,
        stage: Stage::Uploading {
            done: 1000,
            total: 1000,
        },
    }));
    assert_eq!(cell_text(&app), "Sunucu doğruluyor…");
}

#[test]
fn a_failed_kaydet_is_said_until_the_next_one() {
    let mut app = signed_in();
    file_project(&mut app);
    edit(&mut app, 1.0);
    let _ = app.run("file.save");
    let id = app.saving.as_ref().expect("saving").id;
    let _ = app.update(Message::Saving(crate::saving::Event::Encoded {
        id,
        result: Ok(crate::cloud::Once::new(vec![1, 2, 3])),
    }));
    let _ = app.update(crate::cloud::msg(crate::cloud::Event::FileSaved {
        id,
        result: Err(kentos_cloud::ApiFailure::new(
            422,
            "invalid",
            "Dosya doğrulanamadı.",
        )),
    }));
    let cell = app.save_cell().expect("a cloud project");
    assert_eq!(cell.view.text, "Kayıt hatası");
    assert_eq!((cell.shade, cell.lamp), (Shade::Danger, Lamp::Filled));
    assert!(cell.tip.description.ends_with("Dosya doğrulanamadı."));
    assert_eq!(cell.action, Some("file.save"), "Kaydet tries again");
    // The next Kaydet says its own stages.
    let _ = app.run("file.save");
    assert_eq!(cell_text(&app), "Dosya hazırlanıyor…");
}

#[test]
fn a_database_projects_cell_says_what_waits_and_where_it_is() {
    let mut app = signed_in();
    database(&mut app);
    let cell = app.save_cell().expect("a cloud project");
    assert_eq!(cell.view.text, "Buluta kaydedildi");
    assert!(
        cell.tip
            .description
            .starts_with("Harita Bürosu › Ada 101. Değişiklikler kendiliğinden kaydedilir"),
        "{}",
        cell.tip.description
    );
    assert!(cell.tip.description.contains("Son kayıt: henüz yok."));
    edit(&mut app, 1.0);
    app.cloud_after(std::time::Instant::now());
    let cell = app.save_cell().expect("a cloud project");
    assert_eq!(cell.view.text, "Kaydedilecek: 1");
    assert_eq!(cell.action, Some("file.save"), "Ctrl+S sends it now");
}

#[test]
fn the_server_cell_says_the_servers_answer_and_the_check_at_start_is_quiet() {
    let mut app = signed_in();
    assert_eq!(
        app.server_state().0,
        ServerState::Checking,
        "not answered yet"
    );
    let before = said(&app).len();
    app.server_quiet = true;
    app.server_checked(Ok(health(CONTRACTS_VERSION)));
    assert_eq!(app.server_state(), (ServerState::Online, String::new()));
    assert_eq!(said(&app).len(), before, "the check at start says nothing");
    app.server_checked(Ok(health(CONTRACTS_VERSION + 1)));
    let (state, detail) = app.server_state();
    assert_eq!(state, ServerState::Incompatible);
    assert_eq!(
        detail,
        format!(
            "Sunucu sözleşme sürümü {}, uygulama {CONTRACTS_VERSION} bekliyor. Uygulamayı ya da sunucuyu güncelleyin.",
            CONTRACTS_VERSION + 1
        )
    );
    app.server_checked(Err("Sunucuya ulaşılamadı.".into()));
    assert_eq!(
        app.server_state(),
        (ServerState::Offline, "Sunucuya ulaşılamadı.".to_owned())
    );
    let _ = app.view();
}

#[test]
fn a_viewer_is_told_which_right_the_menus_actions_need() {
    let mut app = signed_in();
    open(
        &mut app,
        info(ProjectStorage::Database, false, ProjectState::Active),
    );
    let rows = app.account_rows();
    assert_eq!(
        rows.first(),
        Some(&AccountRow::Header("Ayşe Yılmaz · Harita Bürosu".into()))
    );
    let detail = |id: &str| {
        rows.iter().find_map(|r| match r {
            AccountRow::Command { command, detail } if *command == id => Some(detail.clone()),
            _ => None,
        })
    };
    assert_eq!(
        detail("cloud.share"),
        Some(Some(
            "Bu projede yetkiniz yok (project.share); proje sahibine ya da yöneticisine başvurun."
                .into()
        ))
    );
    assert!(detail("cloud.delete").flatten().is_some());
    assert_eq!(detail("cloud.open"), Some(None));
    let _ = app.account_menu();
}

/// The cloud cells as the web's pictures show them (status-* in
/// apps/web/scripts/e2e/out/shots/cloud): a database project saving, a file
/// project uploading, the server away while signed out, and the account menu,
/// dark and light, at 1440×900 and 1100×650. Not run by default:
/// `cargo test -p kentos-desktop cloud::cells_tests::screens -- --ignored --nocapture`.
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (mode, suffix) in [("dark", ""), ("light", "-acik")] {
        for (width, height) in [(1440.0, 900.0), (1100.0, 650.0)] {
            for scene in [
                "veritabani",
                "dosya-yukleniyor",
                "sunucu-yok",
                "hesap-menusu",
            ] {
                let mut app = signed_in();
                let _ = app
                    .settings
                    .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
                app.apply_settings();
                let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
                let mut update = |app: &mut App, message| {
                    let _ = app.update(message);
                };
                app.server_quiet = true;
                app.server_checked(Ok(health(CONTRACTS_VERSION)));
                match scene {
                    "veritabani" => {
                        database(&mut app);
                        edit(&mut app, 1.0);
                        app.cloud_after(std::time::Instant::now());
                    }
                    "dosya-yukleniyor" => {
                        file_project(&mut app);
                        edit(&mut app, 1.0);
                        let _ = app.run("file.save");
                        if let Some(id) = app.saving.as_ref().map(|s| s.id) {
                            let _ = app.update(Message::Saving(crate::saving::Event::Progress {
                                id,
                                stage: Stage::Uploading {
                                    done: 42,
                                    total: 100,
                                },
                            }));
                        }
                    }
                    "sunucu-yok" => {
                        database(&mut app);
                        app.cloud.me = None;
                        app.server_quiet = true;
                        app.server_checked(Err("Sunucuya ulaşılamadı.".into()));
                    }
                    _ => {
                        open(
                            &mut app,
                            info(ProjectStorage::Database, false, ProjectState::Active),
                        );
                    }
                }
                snapshot.settle(&mut app, App::view, &mut update);
                if scene == "hesap-menusu" {
                    // The server cell: left of the engine's cell, at the bar's right end.
                    let x = width - 110.0;
                    snapshot.input(
                        &mut app,
                        App::view,
                        &mut update,
                        kentos_ui::snapshot::Input::Click(iced::Point::new(x, height - 12.0)),
                    );
                }
                let file = out.join(format!("bulut-hucre-{scene}-{width}x{height}{suffix}.png"));
                snapshot
                    .render(app.view(), &app.theme())
                    .save(&file)
                    .expect("writes the picture");
                println!("{}", file.display());
            }
        }
    }
}
