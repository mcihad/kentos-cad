//! PostGIS'e bağlan and the connection window on the desktop (docs/adr/0505, 0623): a connection is made in a window
//! (no address typed), kept on the device with or without its password, edited, copied, removed and taken from a
//! connections file; its tables come from the catalogue, the chosen ones are bound with `cad.sources.edit`, a table in
//! another system waits for Projenin sistemine çevir; a server that wants a password asks for it once. With
//! KENTOS_PG_URL (the throwaway database of scripts/fixtures/postgis_cases.py) the window lists and binds a real
//! database. Pictures: `postgis-*`, `baglanti-*` (`screens`, ignored).

use std::sync::Arc;

use super::postgis::{Event as P, State, keep_connection};
use super::tests::{count, run, send};
use super::{Event, Window};
use crate::app::{App, Message};
use crate::db_connections::{Event as Db, Retry};
use crate::files_testing::app_with_drawing;
use kentos_postgis::connections::{self, DbConnection, Key, SslMode};
use kentos_postgis::probe::{Catalog, Diagnosis, Problem, Relation, RelationKind, ServerFacts};

fn window(app: &App) -> &State {
    match &app.exchange {
        Some(Window::Postgis(s)) => s,
        _ => panic!("the PostGIS window"),
    }
}

fn db(app: &mut App, e: Db) {
    let _ = app.update(Message::Db(e));
}

/// A connection of the test's own name (the store is the process's, shared by the tests).
fn connection(name: &str) -> DbConnection {
    DbConnection {
        name: name.into(),
        host: "veri.belediye.local".into(),
        database: "kent".into(),
        user: "cbs".into(),
        ..DbConnection::default()
    }
}

fn relation(schema: &str, name: &str, geometry: Option<&str>, srid: Option<u32>, key: Option<&str>) -> Relation {
    Relation {
        schema: schema.into(),
        name: name.into(),
        kind: RelationKind::Table,
        geometry: geometry.map(str::to_owned),
        srid,
        geometry_columns: usize::from(geometry.is_some()),
        estimate: Some(1240),
        comment: None,
        key: key.map(str::to_owned),
        readable: true,
        writable: true,
    }
}

/// A municipality's catalogue: parcels in the project's system, a point table in WGS 84, owners without geometry,
/// a table without a key.
fn catalog() -> Arc<Catalog> {
    Arc::new(Catalog {
        facts: ServerFacts {
            version: "PostgreSQL 16.4".into(),
            postgis: Some("3.4.3".into()),
            user: "cbs".into(),
            database: "kent".into(),
            schemas: 2,
            spatial: 3,
            tables: 4,
        },
        relations: vec![
            relation("kadastro", "malik", None, None, Some("id")),
            relation("kadastro", "parsel", Some("MULTIPOLYGON"), Some(5256), Some("fid")),
            relation("kadastro", "yer", Some("POINT"), Some(4326), Some("fid")),
            relation("imar", "plan_notu", Some("POINT"), Some(4326), None),
        ],
    })
}

fn editor(app: &App) -> &crate::db_connections::Editor {
    app.db_editor.as_ref().expect("the connection window")
}

#[test]
fn a_connection_is_made_in_a_window_and_never_typed_as_an_address() {
    let mut app = app_with_drawing();
    let name = "Bağlantı penceresi";
    let _ = app.db_store.remove(name);
    run(&mut app, "source.postgis");
    // Yeni… from the window: the connection window over it.
    send(&mut app, Event::Postgis(P::New));
    assert!(app.db_editor.is_some());
    for e in [
        Db::Name(name.into()),
        Db::Host("veri.belediye.local".into()),
        Db::Port("54x33".into()),
        Db::Database("kent".into()),
        Db::User("cbs".into()),
        Db::Password("gizli".into()),
    ] {
        db(&mut app, e);
    }
    assert_eq!(editor(&app).port, "5433", "the port takes digits only");
    db(&mut app, Db::Save);
    assert!(app.db_editor.is_none(), "saved and closed");
    let saved = app.db_store.get(name).expect("kept on the device");
    assert_eq!(saved.address(), "postgresql://cbs@veri.belediye.local:5433/kent");
    assert_eq!(saved.password, None, "Parolayı sakla was not ticked");
    // The password stays for this session only.
    assert_eq!(connections::password_for(&Key::of(&saved)).as_deref(), Some("gizli"));
    // The window chose it and lists it.
    assert_eq!(window(&app).connection.as_deref(), Some(name));
    assert!(window(&app).listing);
    send(&mut app, Event::Postgis(P::Listed(name.into(), Ok(catalog()), true)));
    assert!(window(&app).catalog.is_some());
    assert!(window(&app).status.as_ref().is_some_and(|(e, t)| !*e && t.contains("PostgreSQL 16.4")));
    connections::forget(&saved);
    let _ = app.db_store.remove(name);
}

#[test]
fn the_window_lists_chooses_and_waits_for_the_system_question() {
    let mut app = app_with_drawing();
    let name = "Liste penceresi";
    keep_connection(&app, &connection(name));
    run(&mut app, "source.postgis");
    let at = app.db_store.list().iter().position(|c| c.name == name).expect("listed");
    send(&mut app, Event::Postgis(P::Choose(at)));
    send(&mut app, Event::Postgis(P::Listed(name.into(), Ok(catalog()), true)));
    // A table without a key cannot be chosen; the others can.
    send(&mut app, Event::Postgis(P::Toggle("kadastro.parsel".into())));
    send(&mut app, Event::Postgis(P::Toggle("kadastro.yer".into())));
    assert_eq!(window(&app).chosen.len(), 2);
    // Tablo ara narrows Görünenleri seç.
    send(&mut app, Event::Postgis(P::All(false)));
    assert!(window(&app).chosen.is_empty());
    send(&mut app, Event::Postgis(P::Filter("PARS".into())));
    send(&mut app, Event::Postgis(P::All(true)));
    assert_eq!(
        window(&app).chosen.iter().cloned().collect::<Vec<_>>(),
        ["kadastro.parsel"],
        "only what the search shows"
    );
    // A list for another connection than the chosen one is dropped.
    send(&mut app, Event::Postgis(P::Listed("başka".into(), Err(Diagnosis { problem: Problem::Host, message: "yok".into() }), true)));
    assert!(window(&app).catalog.is_some());
    // A refusal says why and empties the list.
    send(&mut app, Event::Postgis(P::Listed(
        name.into(),
        Err(Diagnosis { problem: Problem::Database, message: "Sunucuda “kent” adlı bir veritabanı yok.".into() }),
        true,
    )));
    assert!(window(&app).catalog.is_none());
    assert!(window(&app).status.as_ref().is_some_and(|(e, t)| *e && t.contains("veritabanı yok")));
    assert!(app.db_password.is_none(), "no password asked for a database that is not there");
    let _ = app.db_store.remove(name);
}

#[test]
fn a_server_that_wants_a_password_asks_once_and_gives_the_list_back() {
    let mut app = app_with_drawing();
    let name = "Parola sorusu";
    keep_connection(&app, &connection(name));
    run(&mut app, "source.postgis");
    let at = app.db_store.list().iter().position(|c| c.name == name).expect("listed");
    send(&mut app, Event::Postgis(P::Choose(at)));
    let wants = Diagnosis {
        problem: Problem::PasswordNeeded,
        message: "Sunucu “cbs” kullanıcısı için parola istiyor.".into(),
    };
    send(&mut app, Event::Postgis(P::Listed(name.into(), Err(wants.clone()), true)));
    let asked = app.db_password.as_ref().expect("the password question");
    assert_eq!(asked.retry, Retry::Postgis(name.into()));
    // Typed and tried: the try's answer (a wrong one keeps the question open).
    db(&mut app, Db::Asked("gizli".into()));
    db(&mut app, Db::AskKeep);
    db(&mut app, Db::AskSubmit);
    let generation = app.db_password.as_ref().map(|a| a.generation).expect("open");
    db(&mut app, Db::AskTried(generation, Err(Diagnosis { problem: Problem::Password, message: "yanlış".into() })));
    assert!(app.db_password.as_ref().is_some_and(|a| a.password.is_empty() && a.reason.contains("kabul edilmedi")));
    db(&mut app, Db::Asked("doğru".into()));
    db(&mut app, Db::AskSubmit);
    let generation = app.db_password.as_ref().map(|a| a.generation).expect("open");
    db(&mut app, Db::AskTried(generation, Ok(catalog())));
    assert!(app.db_password.is_none());
    assert!(window(&app).catalog.is_some(), "the list came back");
    // Parolayı sakla was ticked: the device keeps it.
    assert_eq!(app.db_store.get(name).and_then(|c| c.password).as_deref(), Some("doğru"));
    // Closing the question does not ask again.
    send(&mut app, Event::Postgis(P::Listed(name.into(), Err(wants), true)));
    assert!(app.db_password.is_some());
    db(&mut app, Db::AskClose);
    assert!(app.db_password.is_none());
    assert!(window(&app).status.as_ref().is_some_and(|(e, _)| *e));
    connections::forget(&connection(name));
    let _ = app.db_store.remove(name);
}

#[test]
fn the_connection_window_says_what_is_missing_and_fills_from_a_pasted_address() {
    let mut app = app_with_drawing();
    db(&mut app, Db::New(crate::db_connections::Then::Browse));
    db(&mut app, Db::Save);
    let e = editor(&app);
    assert!(e.tried);
    assert!(app.db_editor.is_some(), "nothing saved with fields missing");
    db(&mut app, Db::PasteToggle);
    db(&mut app, Db::Paste("postgresql://ali:s%C3%BCper@10.0.0.5:6543/kent_veri?sslmode=require".into()));
    db(&mut app, Db::PasteApply);
    let e = editor(&app);
    assert_eq!((e.host.as_str(), e.port.as_str(), e.database.as_str(), e.user.as_str()), ("10.0.0.5", "6543", "kent_veri", "ali"));
    assert_eq!(e.password, "süper", "the address's password goes to the password field");
    assert_eq!(e.ssl, SslMode::Require);
    assert_eq!(e.name, "kent_veri");
    assert!(e.paste.is_none());
    // A string that is no address says so and changes nothing.
    db(&mut app, Db::PasteToggle);
    db(&mut app, Db::Paste("veri.gpkg".into()));
    db(&mut app, Db::PasteApply);
    assert!(editor(&app).paste_problem.is_some());
    assert_eq!(editor(&app).host, "10.0.0.5");
    db(&mut app, Db::Close);
    assert!(app.db_editor.is_none());
}

#[test]
fn a_connection_is_edited_copied_and_removed() {
    let mut app = app_with_drawing();
    let name = "Düzenlenen";
    let mut c = connection(name);
    c.password = Some("eski".into());
    keep_connection(&app, &c);
    db(&mut app, Db::Edit(name.into(), crate::db_connections::Then::Browse));
    let e = editor(&app);
    assert!(e.kept && e.keep_password && e.password.is_empty(), "the saved password is not shown");
    db(&mut app, Db::Host("yeni.belediye.local".into()));
    db(&mut app, Db::Name("Düzenlendi".into()));
    db(&mut app, Db::Save);
    assert!(app.db_store.get(name).is_none(), "renamed");
    let edited = app.db_store.get("Düzenlendi").expect("kept");
    assert_eq!(edited.host, "yeni.belediye.local");
    assert_eq!(edited.password.as_deref(), Some("eski"), "the saved password stays when nothing is typed");
    // Kopyasını oluştur: a new connection of a free name, its password with it.
    db(&mut app, Db::Duplicate("Düzenlendi".into()));
    let e = editor(&app);
    assert!(e.previous.is_none());
    assert_eq!(e.name, "Düzenlendi (2)");
    db(&mut app, Db::Close);
    // Sil asks, then removes.
    db(&mut app, Db::AskRemove("Düzenlendi".into()));
    assert_eq!(app.db_removing.as_deref(), Some("Düzenlendi"));
    db(&mut app, Db::Remove);
    assert!(app.db_store.get("Düzenlendi").is_none());
}

#[test]
fn a_connections_file_comes_in_with_free_names() {
    let mut app = app_with_drawing();
    let _ = app.db_store.remove("Ekipten");
    keep_connection(&app, &connection("Ekipten"));
    let given = DbConnection {
        name: "Ekipten".into(),
        host: "db.kurum.local".into(),
        database: "cbs".into(),
        user: "ayse".into(),
        ssl: SslMode::Require,
        ..DbConnection::default()
    };
    let broken = DbConnection {
        name: "Sunucusuz".into(),
        host: String::new(),
        ..given.clone()
    };
    let file = connections::export(&[given, broken], false);
    db(&mut app, Db::ImportRead(Some(Ok(("/tmp/baglantilar.json".into(), file)))));
    let taken = app.db_store.get("Ekipten (2)").expect("taken with a free name");
    assert_eq!((taken.user.as_str(), taken.ssl), ("ayse", SslMode::Require));
    assert!(app.db_store.get("Sunucusuz").is_none(), "one the file cannot give whole is said, not taken");
    let _ = app.db_store.remove("Ekipten");
    let _ = app.db_store.remove("Ekipten (2)");
}

#[test]
fn kaynaklar_lists_a_connection_and_asks_before_carrying_a_table_into_the_project_system() {
    let mut app = app_with_drawing();
    let name = "Kaynaklardaki";
    keep_connection(&app, &connection(name));
    let key = crate::sources::connection_key(name);
    let _ = app.update(Message::Sources(crate::sources::Event::Toggle(key.clone())));
    assert!(matches!(app.sources.catalogs.get(name), Some(crate::sources::Browse::Reading)));
    let _ = app.update(Message::Sources(crate::sources::Event::Browsed(name.into(), Ok(catalog()), true)));
    let lines = app.source_lines();
    let keys: Vec<&str> = lines.iter().map(|l| l.key()).collect();
    assert!(keys.contains(&format!("sc:{name}\0kadastro").as_str()), "{keys:?}");
    assert!(keys.contains(&format!("t:{name}\0kadastro.parsel").as_str()), "{keys:?}");
    assert!(!keys.contains(&format!("t:{name}\0kadastro.malik").as_str()), "tables without geometry hidden by default");
    // A table in another system asks first; Vazgeç binds nothing.
    let before = count(&app);
    let _ = app.update(Message::Sources(crate::sources::Event::AddTable(name.into(), "kadastro.yer".into(), false)));
    assert!(app.sources.reproject.is_some());
    let _ = app.update(Message::Sources(crate::sources::Event::Reproject(false)));
    assert!(app.sources.reproject.is_none());
    assert_eq!(count(&app), before);
    // A table without a key says why.
    let _ = app.update(Message::Sources(crate::sources::Event::Browsed(name.into(), Ok(catalog()), true)));
    let _ = app.update(Message::Sources(crate::sources::Event::AddTable(name.into(), "imar.plan_notu".into(), false)));
    assert_eq!(count(&app), before);
    // A server that wants a password: the question opens with its row waiting.
    let _ = app.update(Message::Sources(crate::sources::Event::Browsed(
        name.into(),
        Err(Diagnosis { problem: Problem::PasswordNeeded, message: "parola".into() }),
        true,
    )));
    assert_eq!(app.db_password.as_ref().map(|a| a.retry.clone()), Some(Retry::Browse(name.into())));
    db(&mut app, Db::AskClose);
    assert!(app.db_password.is_none(), "closed, not asked again");
    let _ = app.db_store.remove(name);
}

#[test]
#[ignore = "needs the throwaway PostGIS of scripts/fixtures/postgis_cases.py (KENTOS_PG_URL)"]
fn a_real_database_is_listed_and_bound() {
    let Ok(url) = std::env::var("KENTOS_PG_URL") else {
        return;
    };
    kentos_postgis::register();
    let mut c = connections::parse(&url).expect("KENTOS_PG_URL").connection;
    c.name = "Gerçek veritabanı".into();
    let mut app = app_with_drawing();
    keep_connection(&app, &c);
    let before = count(&app);
    run(&mut app, "source.postgis");
    let at = app.db_store.list().iter().position(|x| x.name == c.name).expect("listed");
    send(&mut app, Event::Postgis(P::Choose(at)));
    let listed = kentos_postgis::probe::browse(&c, None).map(Arc::new);
    send(&mut app, Event::Postgis(P::Listed(c.name.clone(), listed, true)));
    assert!(window(&app).catalog.is_some(), "{:?}", window(&app).status);
    send(&mut app, Event::Postgis(P::Toggle("kadastro.parsel".into())));
    send(&mut app, Event::Postgis(P::Run));
    assert!(
        app.exchange.is_none(),
        "{:?}",
        app.exchange.as_ref().map(|_| window(&app).status.clone())
    );
    assert!(count(&app) > before);
    assert!(
        app.document
            .as_ref()
            .is_some_and(|d| d.settings().sources.len() == 1)
    );
    let _ = app.db_store.remove(&c.name);
}

#[test]
#[ignore = "writes pictures to .run/shots"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    use std::path::PathBuf;

    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    let real = std::env::var("KENTOS_PG_URL").ok();
    if real.is_some() {
        kentos_postgis::register();
    }
    let only = std::env::var("KENTOS_SHOTS_ONLY").unwrap_or_default();
    let wanted = |name: &str| only.is_empty() || only.split(',').any(|w| name.contains(w));
    let base = match &real {
        Some(url) => {
            let mut c = connections::parse(url).expect("KENTOS_PG_URL").connection;
            c.name = "Belediye".into();
            c
        }
        None => DbConnection {
            name: "Belediye".into(),
            host: "veri.belediye.local".into(),
            database: "kent".into(),
            user: "cbs".into(),
            ..DbConnection::default()
        },
    };
    let listed = || match &real {
        Some(_) => kentos_postgis::probe::browse(&base, None).map(Arc::new),
        None => Ok(catalog()),
    };
    for (width, height) in [(1440.0, 900.0), (1100.0, 650.0)] {
        let picture = |app: &mut App, name: &str| {
            if !wanted(name) {
                return;
            }
            let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
            let mut update = |app: &mut App, message| {
                let _ = app.update(message);
            };
            snapshot.settle(app, App::view, &mut update);
            let _ = snapshot.render(app.view(), &app.theme());
            let file = out.join(format!("{name}-{width}x{height}.png"));
            snapshot
                .render(app.view(), &app.theme())
                .save(&file)
                .expect("writes the picture");
            println!("{}", file.display());
        };
        for mode in ["dark", "light"] {
            let themed = || {
                let mut app = app_with_drawing();
                let _ = app
                    .settings
                    .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
                app.apply_settings();
                app
            };
            // The window with no connection yet.
            let mut app = themed();
            run(&mut app, "source.postgis");
            picture(&mut app, &format!("postgis-{mode}-0-bos"));
            // Yeni bağlantı: empty, then filled and tried.
            db(&mut app, Db::New(crate::db_connections::Then::Postgis));
            picture(&mut app, &format!("baglanti-{mode}-1-yeni"));
            for e in [
                Db::Name(base.name.clone()),
                Db::Host(base.host.clone()),
                Db::Port(base.port.to_string()),
                Db::Database(base.database.clone()),
                Db::User(base.user.clone()),
                Db::Password(std::env::var("PGPASSWORD").unwrap_or_else(|_| "gizli".into())),
            ] {
                db(&mut app, e);
            }
            let tried = match &real {
                Some(_) => kentos_postgis::probe::probe(&base, std::env::var("PGPASSWORD").ok().as_deref()),
                None => Ok(catalog().facts.clone()),
            };
            let generation = app.db_editor.as_ref().map_or(0, |e| e.generation);
            db(&mut app, Db::Tested(generation, tried));
            picture(&mut app, &format!("baglanti-{mode}-2-denendi"));
            // A wrong password said as people say it.
            let generation = app.db_editor.as_ref().map_or(0, |e| e.generation);
            db(&mut app, Db::Tested(generation, Err(Diagnosis {
                problem: Problem::Password,
                message: "Parola ya da kullanıcı adı (“cbs”) yanlış. İkisini de denetleyip yeniden deneyin.".into(),
            })));
            picture(&mut app, &format!("baglanti-{mode}-3-hata"));
            db(&mut app, Db::Save);
            // The window with the connection chosen and its tables.
            send(&mut app, Event::Postgis(P::Listed(base.name.clone(), listed(), true)));
            for t in ["kadastro.parsel", "kadastro.bina"] {
                send(&mut app, Event::Postgis(P::Toggle(t.into())));
            }
            picture(&mut app, &format!("postgis-{mode}-2-tablolar"));
            send(&mut app, Event::Postgis(P::Advanced));
            send(
                &mut app,
                Event::Postgis(P::Query("SELECT fid, ada, parsel, geom FROM kadastro.parsel WHERE ada = 105".into())),
            );
            picture(&mut app, &format!("postgis-{mode}-5-sorgu"));
            if real.is_some() {
                send(&mut app, Event::Postgis(P::Run));
                picture(&mut app, &format!("postgis-{mode}-4-bagli"));
            }
            // Kaynaklar: the connection open into its schemas and tables.
            let mut app = themed();
            keep_connection(&app, &base);
            app.show_sources();
            let key = crate::sources::connection_key(&base.name);
            let _ = app.update(Message::Sources(crate::sources::Event::Toggle(key)));
            let _ = app.update(Message::Sources(crate::sources::Event::Browsed(base.name.clone(), listed(), true)));
            picture(&mut app, &format!("baglanti-{mode}-4-kaynaklar"));
            // The password question.
            app.ask_db_password(
                &base.name,
                format!("Sunucu “{}” kullanıcısı için parola istiyor; bu bağlantının parolası bilinmiyor. Parolayı yazıp yeniden deneyin.", base.user),
                Retry::Browse(base.name.clone()),
            );
            picture(&mut app, &format!("baglanti-{mode}-5-parola"));
        }
    }
}
