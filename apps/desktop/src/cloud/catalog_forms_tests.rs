//! The catalog's project forms message by message, without a server
//! (docs/adr/0112): Proje bilgileri sends only what changed with the version
//! it showed, a copy is shown in Projelerim, a conversion is shown and
//! opened, and refusals are said in the server's words.

use kentos_cloud::ApiFailure;
use kentos_contracts::{
    CatalogView, ProjectCatalogChange, ProjectDuplicated, ProjectStorage, ProjectType,
};

use crate::app::App;
use crate::cloud::Event;
use crate::cloud::catalog::{List, Tab};
use crate::cloud::catalog_forms::{Done, Event as FormEvent, Form};
use crate::cloud::catalog_tests::{SECOND, catalog, listed, owned};
use crate::cloud::forms_plan::{self as plan, duplicate, metadata};
use crate::cloud::tests::{PROJECT, TENANT, cloud, last_said, page, signed_in};

const MADE: &str = "0199aaaa-0000-7000-8000-00000000000d";

fn form(app: &mut App, event: FormEvent) {
    cloud(app, Event::Form(event));
}

/// The catalog on “Projelerim” with the project (kept `storage`-wise) selected.
fn picked(storage: ProjectStorage) -> App {
    let mut app = signed_in();
    let mut p = owned(PROJECT, "Ada 1246 ölçü projesi");
    p.storage = storage;
    p.description = "Kadastro çalışması".into();
    p.tags = vec!["Kadıköy".into(), "2026".into()];
    p.catalog_version = "7".into();
    listed(&mut app, CatalogView::Mine, vec![p, owned(SECOND, "Başka")]);
    cloud(&mut app, Event::CatalogPick(PROJECT.into()));
    app
}

fn asked(app: &App) -> u64 {
    catalog(app)
        .form_asked
        .as_ref()
        .expect("a request on its way")
        .0
}

fn made(name: &str) -> ProjectDuplicated {
    let mut p = owned(MADE, name);
    p.tenant_id = TENANT.into();
    ProjectDuplicated {
        project: p,
        source_id: PROJECT.into(),
        objects: "1234".into(),
        replayed: false,
    }
}

#[test]
fn proje_bilgileri_sends_what_changed_and_says_it() {
    let mut app = picked(ProjectStorage::Database);
    form(&mut app, FormEvent::Edit);
    let Some(Form::Metadata(f)) = &catalog(&app).form else {
        panic!("the form");
    };
    assert_eq!(f.name, "Ada 1246 ölçü projesi");
    assert_eq!(f.tags, "Kadıköy, 2026");
    assert!(!f.savable(), "nothing changed yet");
    form(&mut app, FormEvent::Name("  Ada 1246  ".into()));
    form(&mut app, FormEvent::Type(ProjectType::Subdivision));
    form(
        &mut app,
        FormEvent::Tags("Kadıköy,  ifraz  ölçüsü ,".into()),
    );
    let Some(Form::Metadata(f)) = &catalog(&app).form else {
        panic!("the form");
    };
    let patch = f.patch();
    assert_eq!(patch.name.as_deref(), Some("Ada 1246"));
    assert_eq!(patch.project_type, Some(ProjectType::Subdivision));
    assert_eq!(patch.description, None, "unchanged");
    assert_eq!(
        patch.tags,
        Some(vec!["Kadıköy".to_owned(), "ifraz ölçüsü".to_owned()])
    );
    assert!(f.savable());
    form(&mut app, FormEvent::Submit);
    let Some(Form::Metadata(f)) = &catalog(&app).form else {
        panic!("the form waits");
    };
    assert!(f.busy);
    assert_eq!(
        f.status.as_ref().map(|s| s.text.as_str()),
        Some(plan::SAVING)
    );
    // A newer version on the server: nothing written, said in the web's words.
    let id = asked(&app);
    form(
        &mut app,
        FormEvent::Done {
            id,
            result: Err(ApiFailure::new(409, "conflict", "Sürüm değişti.")),
        },
    );
    let Some(Form::Metadata(f)) = &catalog(&app).form else {
        panic!("the form stays");
    };
    assert!(!f.busy);
    assert_eq!(
        f.status.as_ref().map(|s| (s.text.as_str(), s.error)),
        Some((
            "Proje bilgileri bu arada başka biri tarafından değiştirildi. Listeyi yenileyip yeniden deneyin.",
            true
        ))
    );
    form(&mut app, FormEvent::Submit);
    let id = asked(&app);
    let mut changed = owned(PROJECT, "Ada 1246");
    changed.project_type = ProjectType::Subdivision;
    form(
        &mut app,
        FormEvent::Done {
            id,
            result: Ok(Done::Metadata(Box::new(ProjectCatalogChange {
                project: changed,
                changed: true,
                event_seq: None,
                replayed: false,
            }))),
        },
    );
    assert!(catalog(&app).form.is_none(), "the window goes");
    assert_eq!(last_said(&app), metadata::saved("Ada 1246"));
    assert!(catalog(&app).request().is_some(), "the list is read again");
}

#[test]
fn a_copy_is_the_accounts_own_and_shown_in_projelerim() {
    let mut app = picked(ProjectStorage::Database);
    form(&mut app, FormEvent::Duplicate);
    let Some(Form::Made(f)) = &catalog(&app).form else {
        panic!("the form");
    };
    assert_eq!(f.name, "Ada 1246 ölçü projesi (kopya)");
    assert!(f.convert.is_none());
    assert_eq!(
        f.places.first().map(|p| p.tenant_id.as_str()),
        Some(TENANT),
        "the source's workspace first"
    );
    form(&mut app, FormEvent::Name("   ".into()));
    let Some(Form::Made(f)) = &catalog(&app).form else {
        panic!("the form");
    };
    assert!(!f.savable(), "a copy needs a name");
    form(&mut app, FormEvent::Name("Ada 1246 kopyası".into()));
    form(&mut app, FormEvent::Place(1));
    form(&mut app, FormEvent::Submit);
    let id = asked(&app);
    let Some(Form::Made(f)) = &catalog(&app).form else {
        panic!("the form waits");
    };
    assert_eq!(
        f.status.as_ref().map(|s| s.text.as_str()),
        Some(duplicate::RUNNING)
    );
    form(
        &mut app,
        FormEvent::Done {
            id,
            result: Ok(Done::Made(Box::new(made("Ada 1246 kopyası")))),
        },
    );
    assert_eq!(
        last_said(&app),
        "“Ada 1246 kopyası” oluşturuldu: 1.234 nesne kopyalandı."
    );
    let c = catalog(&app);
    assert!(c.form.is_none());
    assert_eq!(
        (c.list.clone(), c.tab),
        (List::View(CatalogView::Mine), Tab::Info)
    );
    let page_id = c.request().expect("the list asked");
    cloud(
        &mut app,
        Event::CatalogPage {
            id: page_id,
            more: false,
            result: Ok(page(vec![made("Ada 1246 kopyası").project], 1, None)),
        },
    );
    assert_eq!(catalog(&app).picked.as_deref(), Some(MADE), "selected");
    assert!(
        app.cloud.opening.is_none(),
        "a copy does not open by itself"
    );
}

#[test]
fn a_file_project_goes_to_postgis_and_the_new_project_opens() {
    let mut app = picked(ProjectStorage::File);
    form(&mut app, FormEvent::Convert);
    let Some(Form::Made(f)) = &catalog(&app).form else {
        panic!("the form");
    };
    let convert = f.convert.as_ref().expect("a conversion");
    assert_eq!(convert.to, ProjectStorage::Database);
    assert_eq!(convert.title, "PostGIS'e aktar");
    assert_eq!(convert.placeholder, "Ada 1246 ölçü projesi (PostGIS)");
    assert!(f.name.is_empty(), "the name is optional");
    assert!(f.savable());
    form(&mut app, FormEvent::Submit);
    let id = asked(&app);
    // An object the server did not take is named by its place in the file.
    form(
        &mut app,
        FormEvent::Done {
            id,
            result: Err(
                ApiFailure::new(422, "invalid", "Değer ±1 000 000 000 sınırını aşıyor.")
                    .with_path("entities[41].geometry"),
            ),
        },
    );
    let Some(Form::Made(f)) = &catalog(&app).form else {
        panic!("the form stays");
    };
    assert_eq!(
        f.status.as_ref().map(|s| s.text.as_str()),
        Some(
            "Değer ±1 000 000 000 sınırını aşıyor. (dosyanın 42. nesnesi; hiçbir proje oluşturulmadı)."
        )
    );
    form(&mut app, FormEvent::Submit);
    let id = asked(&app);
    form(
        &mut app,
        FormEvent::Done {
            id,
            result: Ok(Done::Made(Box::new(made(
                "Ada 1246 ölçü projesi (PostGIS)",
            )))),
        },
    );
    assert_eq!(
        last_said(&app),
        "“Ada 1246 ölçü projesi (PostGIS)” oluşturuldu: 1.234 nesne veritabanına aktarıldı."
    );
    let page_id = catalog(&app).request().expect("the list asked");
    cloud(
        &mut app,
        Event::CatalogPage {
            id: page_id,
            more: false,
            result: Ok(page(
                vec![made("Ada 1246 ölçü projesi (PostGIS)").project],
                1,
                None,
            )),
        },
    );
    assert_eq!(catalog(&app).picked.as_deref(), Some(MADE));
    assert!(app.cloud.opening.is_some(), "the new project opens");
}

#[test]
fn esc_and_vazgec_close_a_form_but_not_while_it_waits() {
    let mut app = picked(ProjectStorage::Database);
    form(&mut app, FormEvent::Convert);
    let Some(Form::Made(f)) = &catalog(&app).form else {
        panic!("the form");
    };
    assert_eq!(
        f.convert.as_ref().map(|c| c.title),
        Some("Dosya projesine çevir")
    );
    let _ = app.view();
    form(&mut app, FormEvent::Close);
    assert!(catalog(&app).form.is_none());
    form(&mut app, FormEvent::Duplicate);
    form(&mut app, FormEvent::Submit);
    form(&mut app, FormEvent::Close);
    assert!(catalog(&app).form.is_some(), "its request is on its way");
    app.close_dialog();
    assert!(catalog(&app).form.is_some(), "Esc waits too");
    assert_eq!(app.dialog, Some(crate::app::Dialog::Catalog));
}

/// The forms as the web's reference pictures show them (form-* in
/// apps/web/scripts/e2e/out/shots/cloud): Proje bilgileri, Kopyasını
/// oluştur, PostGIS'e aktar and Dosya projesine çevir, dark and light, at
/// 1440×900 and 1100×650. Not run by default:
/// `cargo test -p kentos-desktop cloud::catalog_forms_tests::screens -- --ignored --nocapture`.
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    let scenes = ["bilgiler", "kopya", "postgise-aktar", "dosyaya-cevir"];
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
                let storage = if scene == "postgise-aktar" {
                    ProjectStorage::File
                } else {
                    ProjectStorage::Database
                };
                let mut p = owned(
                    PROJECT,
                    if storage == ProjectStorage::File {
                        "Kadastro paftası 2026"
                    } else {
                        "Ada 1246 ölçü projesi"
                    },
                );
                p.storage = storage;
                p.tenant_name = "Örnek Harita Bürosu".into();
                p.project_type = ProjectType::Subdivision;
                p.description =
                    "1246 ada 1–3 parsellerin ifraz ölçüleri; belediye teslimine hazırlanıyor."
                        .into();
                p.tags = vec!["Kadıköy".into(), "ifraz".into(), "ölçü".into()];
                listed(&mut app, CatalogView::Mine, vec![p, owned(SECOND, "Başka")]);
                cloud(&mut app, Event::CatalogPick(PROJECT.into()));
                form(
                    &mut app,
                    match scene {
                        "bilgiler" => FormEvent::Edit,
                        "kopya" => FormEvent::Duplicate,
                        _ => FormEvent::Convert,
                    },
                );
                snapshot.settle(&mut app, App::view, &mut update);
                let file = out.join(format!("bulut-form-{scene}-{width}x{height}{suffix}.png"));
                snapshot
                    .render(app.view(), &app.theme())
                    .save(&file)
                    .expect("writes the picture");
                println!("{}", file.display());
            }
        }
    }
}
