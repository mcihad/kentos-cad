//! Cloud tests: signing in, the catalog's pages and search, a project opened (late, archived, over
//! another drawing).

use super::*;

// ── Signing in ──────────────────────────────────────────────────────────

#[test]
fn sign_in_checks_the_fields_and_the_address_before_anything_is_sent() {
    let (mut app, _) = App::boot(None);
    let _ = app.run("cloud.signIn");
    assert_eq!(app.dialog, Some(Dialog::SignIn));
    let s = app.cloud.sign_in.as_ref().expect("the window");
    assert_eq!(
        s.server, "http://127.0.0.1:8787",
        "the preference's default"
    );

    cloud(&mut app, Event::SignInSubmit);
    let s = app.cloud.sign_in.as_ref().expect("the window");
    assert_eq!(s.error.as_deref(), Some("Giriş adını ve parolayı yazın."));
    assert!(!s.busy());

    cloud(&mut app, Event::SignInLogin("ayse".into()));
    cloud(&mut app, Event::SignInPassword(SECRET.into()));
    cloud(
        &mut app,
        Event::SignInServer("http://192.168.1.5:8787".into()),
    );
    cloud(&mut app, Event::SignInSubmit);
    let s = app.cloud.sign_in.as_ref().expect("the window");
    let error = s.error.as_deref().unwrap_or_default();
    assert!(
        error.contains("şifresiz http yalnız bu bilgisayardaki sunucuya"),
        "{error}"
    );
    assert!(!s.busy(), "nothing was sent");

    cloud(&mut app, Event::SignInServer("http://127.0.0.1:9".into()));
    cloud(&mut app, Event::SignInSubmit);
    let first = app
        .cloud
        .sign_in
        .as_ref()
        .and_then(|s| s.request())
        .expect("sent");

    // The server's own message goes into the window.
    cloud(
        &mut app,
        Event::SignedIn {
            id: first,
            result: Err(ApiFailure::new(
                401,
                "unauthenticated",
                "Giriş adı ya da parola yanlış.",
            )),
        },
    );
    let s = app.cloud.sign_in.as_ref().expect("the window");
    assert_eq!(s.error.as_deref(), Some("Giriş adı ya da parola yanlış."));
    assert!(app.cloud.me.is_none());

    cloud(&mut app, Event::SignInSubmit);
    let second = app
        .cloud
        .sign_in
        .as_ref()
        .and_then(|s| s.request())
        .expect("sent");
    // A late answer to the first request changes nothing.
    cloud(
        &mut app,
        Event::SignedIn {
            id: first,
            result: Ok(me()),
        },
    );
    assert!(app.cloud.me.is_none());
    cloud(
        &mut app,
        Event::SignedIn {
            id: second,
            result: Ok(me()),
        },
    );
    assert_eq!(
        app.cloud.me.as_ref().map(|m| m.user.display_name.as_str()),
        Some("Ayşe Yılmaz")
    );
    assert_eq!(app.dialog, None);
    assert!(
        app.cloud.sign_in.is_none(),
        "the password went with the window"
    );
    assert_eq!(last_said(&app), "Ayşe Yılmaz olarak giriş yapıldı.");
    // The address that worked is the preference now; the password is nowhere.
    assert_eq!(app.settings.text("cloud.server"), "http://127.0.0.1:9");
    assert!(!app.settings.export_text().contains(SECRET));
    assert!(!said(&app).iter().any(|t| t.contains(SECRET)));
}

#[test]
fn a_command_that_needs_an_account_signs_in_first_and_goes_on() {
    let (mut app, _) = App::boot(None);
    let _ = app.run("cloud.open");
    assert_eq!(app.dialog, Some(Dialog::SignIn));
    app.cloud.sign_in.as_mut().expect("the window").server = "http://127.0.0.1:9".into();
    cloud(&mut app, Event::SignInLogin("ayse".into()));
    cloud(&mut app, Event::SignInPassword(SECRET.into()));
    cloud(&mut app, Event::SignInSubmit);
    let id = app
        .cloud
        .sign_in
        .as_ref()
        .and_then(|s| s.request())
        .expect("sent");
    cloud(
        &mut app,
        Event::SignedIn {
            id,
            result: Ok(me()),
        },
    );
    assert_eq!(
        app.dialog,
        Some(Dialog::Catalog),
        "the catalog opens by itself"
    );
    assert!(app.cloud.catalog.as_ref().is_some_and(|c| c.loading()));
}

#[test]
fn esc_closes_the_sign_in_window_and_its_password_goes_with_it() {
    let (mut app, _) = App::boot(None);
    let _ = app.run("cloud.signIn");
    cloud(&mut app, Event::SignInPassword(SECRET.into()));
    app.close_dialog();
    assert_eq!(app.dialog, None);
    assert!(app.cloud.sign_in.is_none());
}

// ── The catalog ─────────────────────────────────────────────────────────

#[test]
fn the_catalog_pages_its_lists_and_drops_late_answers() {
    let mut app = signed_in();
    let _ = app.run("cloud.open");
    assert_eq!(app.dialog, Some(Dialog::Catalog));
    let first = app
        .cloud
        .catalog
        .as_ref()
        .and_then(|c| c.request())
        .expect("asked");
    cloud(
        &mut app,
        Event::CatalogView(List::View(CatalogView::Shared)),
    );
    let second = app
        .cloud
        .catalog
        .as_ref()
        .and_then(|c| c.request())
        .expect("asked");
    assert_ne!(first, second);
    cloud(
        &mut app,
        Event::CatalogPage {
            id: first,
            more: false,
            result: Ok(page(
                vec![summary(OTHER, "Eski liste", ProjectStorage::File)],
                1,
                None,
            )),
        },
    );
    let c = app.cloud.catalog.as_ref().expect("open");
    assert!(c.projects.is_empty(), "the older list's answer is dropped");

    cloud(
        &mut app,
        Event::CatalogPage {
            id: second,
            more: false,
            result: Ok(page(
                vec![
                    summary(PROJECT, "Ada 101", ProjectStorage::Database),
                    summary(OTHER, "Yol projesi", ProjectStorage::File),
                ],
                3,
                Some("sonraki"),
            )),
        },
    );
    let c = app.cloud.catalog.as_ref().expect("open");
    assert_eq!((c.projects.len(), c.total), (2, 3));
    cloud(&mut app, Event::CatalogMore);
    let third = app
        .cloud
        .catalog
        .as_ref()
        .and_then(|c| c.request())
        .expect("asked");
    let mut last = summary(
        "0199aaaa-0000-7000-8000-00000000000a",
        "Parsel",
        ProjectStorage::File,
    );
    last.state = ProjectState::Archived;
    cloud(
        &mut app,
        Event::CatalogPage {
            id: third,
            more: true,
            result: Ok(page(vec![last], 3, None)),
        },
    );
    let c = app.cloud.catalog.as_ref().expect("open");
    assert_eq!(c.projects.len(), 3, "the next page comes below");
    assert!(c.next.is_none());

    // The server's failure says why, with a retry.
    cloud(&mut app, Event::CatalogRetry);
    let fourth = app
        .cloud
        .catalog
        .as_ref()
        .and_then(|c| c.request())
        .expect("asked");
    cloud(
        &mut app,
        Event::CatalogPage {
            id: fourth,
            more: false,
            result: Err(ApiFailure::new(
                503,
                "unavailable",
                "Sunucu şu an yanıt veremiyor.",
            )),
        },
    );
    let c = app.cloud.catalog.as_ref().expect("open");
    assert_eq!(c.error.as_deref(), Some("Sunucu şu an yanıt veremiyor."));
}

#[test]
fn the_search_goes_to_the_server_once_typing_rests() {
    let mut app = signed_in();
    let _ = app.run("cloud.open");
    let first = app.cloud.catalog.as_ref().and_then(|c| c.request());
    cloud(&mut app, Event::CatalogSearch("ada".into()));
    let now = Instant::now();
    let _ = app.cloud_tick(now);
    assert_eq!(
        app.cloud.catalog.as_ref().and_then(|c| c.request()),
        first,
        "not yet"
    );
    let _ = app.cloud_tick(now + Duration::from_millis(300));
    let c = app.cloud.catalog.as_ref().expect("open");
    assert_ne!(c.request(), first, "asked with the search");
    assert!(c.search_at.is_none());
}

#[test]
fn kurum_projeleri_lists_the_chosen_organisation() {
    let mut app = signed_in();
    let orgs = app.organizations();
    assert_eq!(orgs, [(TENANT.to_owned(), "Harita Bürosu".to_owned())]);
    let _ = app.run("cloud.open");
    let c = app.cloud.catalog.as_ref().expect("open");
    assert_eq!(c.org.as_deref(), Some(TENANT), "the first organisation");
    cloud(
        &mut app,
        Event::CatalogView(List::View(CatalogView::Organization)),
    );
    assert!(app.cloud.catalog.as_ref().is_some_and(|c| c.loading()));
    // Without an organisation the list says so and asks nothing.
    app.cloud.catalog.as_mut().expect("open").org = None;
    cloud(&mut app, Event::CatalogRetry);
    assert!(app.cloud.catalog.as_ref().is_some_and(|c| !c.loading()));
}

// ── Opening ─────────────────────────────────────────────────────────────

#[test]
fn a_database_project_opens_as_the_drawing_and_says_where_it_is() {
    let mut app = signed_in();
    let _ = app.run("cloud.open");
    database(&mut app);
    let doc = app.document.as_ref().expect("open");
    let source = doc.cloud_source().expect("a cloud project");
    assert_eq!(
        (source.workspace.as_str(), source.storage()),
        ("Harita Bürosu", ProjectStorage::Database)
    );
    assert!(!doc.dirty());
    assert_eq!(app.title(), "Ada 101 — Harita Bürosu — KentOS CAD");
    assert_eq!(
        last_said(&app),
        "“Ada 101” bulut projesi açıldı (Harita Bürosu, Yönetilen PostGIS veritabanı): 13 nesne."
    );
    assert_eq!(app.dialog, None, "the catalog closed");
    assert!(app.cloud.catalog.is_none());
    assert_eq!(state(&app), SaveState::Saved);
    assert_eq!(cell_text(&app), "Buluta kaydedildi");
}

#[test]
fn a_late_open_never_replaces_a_newer_one_or_a_changed_drawing() {
    let mut app = signed_in();
    let _ = app.start_cloud_open(uuid(TENANT), uuid(PROJECT));
    let first = app.cloud.opening.as_ref().expect("opening").id;
    let _ = app.start_cloud_open(uuid(TENANT), uuid(PROJECT));
    let second = app.cloud.opening.as_ref().expect("opening").id;
    answer_open(
        &mut app,
        first,
        info(ProjectStorage::Database, true, ProjectState::Active),
    );
    assert!(app.document.is_none(), "the overtaken open is dropped");
    assert!(app.cloud.opening.is_some());
    cloud(&mut app, Event::OpenCancel);
    assert!(app.cloud.opening.is_none());
    assert!(last_said(&app).contains("açılışı durduruldu"));
    answer_open(
        &mut app,
        second,
        info(ProjectStorage::Database, true, ProjectState::Active),
    );
    assert!(app.document.is_none(), "a stopped open changes nothing");

    // The drawing on screen changed while the project was read: it stays.
    database(&mut app);
    let before = session(&app);
    let _ = app.start_cloud_open(uuid(TENANT), uuid(PROJECT));
    let id = app.cloud.opening.as_ref().expect("opening").id;
    edit(&mut app, 1.0);
    answer_open(
        &mut app,
        id,
        info(ProjectStorage::Database, true, ProjectState::Active),
    );
    assert_eq!(session(&app), before);
    assert!(last_said(&app).contains("açılış sürerken ekrandaki çizim değişti"));
}

#[test]
fn an_archived_project_opens_read_only_with_the_web_s_notice() {
    let mut app = signed_in();
    open(
        &mut app,
        info(ProjectStorage::Database, true, ProjectState::Archived),
    );
    assert_eq!(state(&app), SaveState::Archived);
    assert_eq!(cell_text(&app), "Proje arşivde");
    assert!(said(&app).iter().any(|t| t
        == "“Ada 101” arşivlenmiş bir proje: salt okunur açıldı; değişiklikler buluta kaydedilmez. Düzenlemek için arşivden çıkarılmalı ya da kopyası oluşturulmalı."));
}

#[test]
fn opening_asks_first_about_the_drawing_on_screen() {
    let mut app = signed_in();
    let _ = app.update(Message::Opened(Some(Ok(Box::new(
        crate::files_testing::drawing(0),
    )))));
    app.document.as_mut().expect("open").model.mark_unsaved();
    let _ = app.run("cloud.open");
    let id = app
        .cloud
        .catalog
        .as_ref()
        .and_then(|c| c.request())
        .expect("asked");
    cloud(
        &mut app,
        Event::CatalogPage {
            id,
            more: false,
            result: Ok(page(
                vec![summary(PROJECT, "Ada 101", ProjectStorage::Database)],
                1,
                None,
            )),
        },
    );
    cloud(&mut app, Event::CatalogPick(PROJECT.into()));
    cloud(&mut app, Event::CatalogOpen);
    let then = Then::OpenCloud {
        tenant: uuid(TENANT),
        project: uuid(PROJECT),
    };
    assert_eq!(app.dialog, Some(Dialog::Unsaved(then)));
    assert!(app.cloud.opening.is_none(), "not before the answer");
    // Vazgeç goes back to the catalog.
    app.close_dialog();
    assert_eq!(app.dialog, Some(Dialog::Catalog));
    cloud(&mut app, Event::CatalogOpen);
    let _ = app.update(Message::DialogConfirmed);
    assert!(app.cloud.opening.is_some(), "Kaydetmeden aç opens it");
}
