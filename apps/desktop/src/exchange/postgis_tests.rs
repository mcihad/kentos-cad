//! PostGIS'e bağlan on the desktop (docs/adr/0505): the address keeps no password, the tables come from
//! `cad.sources.describe`, the chosen ones are bound with `cad.sources.edit`, a table in another system waits for
//! Projenin sistemine çevir. With KENTOS_PG_URL (the throwaway database of scripts/fixtures/postgis_cases.py) the
//! window lists and binds a real database. Pictures: `postgis-*` (`screens`, ignored).

use super::postgis::{Event as P, State, list_tables};
use super::tests::{count, run, send};
use super::{Event, Window};
use crate::app::App;
use crate::files_testing::app_with_drawing;
use kentos_commands::SourceTable;

fn window(app: &App) -> &State {
    match &app.exchange {
        Some(Window::Postgis(s)) => s,
        _ => panic!("the PostGIS window"),
    }
}

/// Two tables as a provider lists them: parcels in the project's system, a point table in WGS 84.
fn tables() -> Vec<SourceTable> {
    serde_json::from_value(serde_json::json!([
        { "name": "kadastro.parsel", "data": "features", "rows": 3, "geometry": "MULTIPOLYGON", "srid": 5256,
          "columns": [{ "name": "ada", "declared": "text", "kind": "text" }, { "name": "parsel", "declared": "integer", "kind": "integer" }],
          "capabilities": { "read": true, "insert": true, "update": true, "delete": true, "transaction": true,
                            "z": false, "m": false, "curves": false } },
        { "name": "kadastro.yer", "data": "features", "rows": 1, "geometry": "POINT", "srid": 4326,
          "columns": [{ "name": "ad", "declared": "text", "kind": "text" }],
          "capabilities": { "read": true, "insert": false, "update": false, "delete": false, "transaction": true,
                            "z": false, "m": false, "curves": false } }
    ]))
    .expect("tables")
}

#[test]
fn an_address_with_a_password_is_refused_before_anything_is_reached() {
    let why =
        list_tables("postgresql://cbs:gizli@veri.belediye.local/kent".into()).expect_err("refused");
    assert!(why.contains("parola"), "{why}");
}

#[test]
fn the_window_lists_binds_and_says_why_not() {
    let mut app = app_with_drawing();
    let before = count(&app);
    run(&mut app, "source.postgis");
    send(
        &mut app,
        Event::Postgis(P::Address(
            "postgresql://cbs@veri.belediye.local/kent".into(),
        )),
    );
    send(&mut app, Event::Postgis(P::Listed(Ok(tables()))));
    assert_eq!(window(&app).chosen, vec![false, false]);
    send(&mut app, Event::Postgis(P::Toggle(0)));
    send(
        &mut app,
        Event::Postgis(P::Listed(Err("bağlanılamadı".into()))),
    );
    assert!(window(&app).tables.is_empty());
    assert_eq!(window(&app).status, Some((true, "bağlanılamadı".into())));
    // A refused bind changes nothing and leaves the window open with the reason.
    send(&mut app, Event::Postgis(P::Listed(Ok(tables()))));
    send(&mut app, Event::Postgis(P::Toggle(0)));
    if kentos_native_application::provider::postgis().is_none() {
        send(&mut app, Event::Postgis(P::Run));
        assert_eq!(count(&app), before);
        assert!(
            window(&app)
                .status
                .as_ref()
                .is_some_and(|(e, t)| *e && t.contains("PostGIS sağlayıcısı")),
            "{:?}",
            window(&app).status
        );
    }
}

#[test]
#[ignore = "needs the throwaway PostGIS of scripts/fixtures/postgis_cases.py (KENTOS_PG_URL)"]
fn a_real_database_is_listed_and_bound() {
    let Ok(url) = std::env::var("KENTOS_PG_URL") else {
        return;
    };
    kentos_postgis::register();
    let mut app = app_with_drawing();
    let before = count(&app);
    run(&mut app, "source.postgis");
    send(&mut app, Event::Postgis(P::Address(url.clone())));
    let listed = list_tables(url);
    send(&mut app, Event::Postgis(P::Listed(listed)));
    let at = window(&app)
        .tables
        .iter()
        .position(|t| t.name == "kadastro.parsel")
        .expect("the parcels listed");
    send(&mut app, Event::Postgis(P::Toggle(at)));
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
    for (width, height) in [(1440.0, 900.0), (1100.0, 650.0)] {
        let picture = |app: &mut App, name: &str| {
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
            let mut app = app_with_drawing();
            let _ = app
                .settings
                .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
            app.apply_settings();
            run(&mut app, "source.postgis");
            let address = real
                .clone()
                .unwrap_or_else(|| "postgresql://cbs@veri.belediye.local:5432/kent".into());
            send(&mut app, Event::Postgis(P::Address(address.clone())));
            picture(&mut app, &format!("postgis-{mode}-1-adres"));
            let listed = if real.is_some() {
                list_tables(address)
            } else {
                Ok(tables())
            };
            send(&mut app, Event::Postgis(P::Listed(listed)));
            let names: Vec<String> = window(&app).tables.iter().map(|t| t.name.clone()).collect();
            for (i, n) in names.iter().enumerate() {
                if n == "kadastro.parsel" || n == "kadastro.yer" {
                    send(&mut app, Event::Postgis(P::Toggle(i)));
                }
            }
            picture(&mut app, &format!("postgis-{mode}-2-tablolar"));
            send(&mut app, Event::Postgis(P::Reproject));
            picture(&mut app, &format!("postgis-{mode}-3-cevir"));
            // A query layer and the view's area (docs/adr/0582).
            for (i, n) in names.iter().enumerate() {
                if n == "sorgu.nokta_v" {
                    send(&mut app, Event::Postgis(P::Toggle(i)));
                }
            }
            send(
                &mut app,
                Event::Postgis(P::Query(
                    "SELECT fid, ad, tur, geom FROM sorgu.nokta WHERE tur = 'direk'".into(),
                )),
            );
            send(&mut app, Event::Postgis(P::Area));
            picture(&mut app, &format!("postgis-{mode}-5-sorgu-alan"));
            if real.is_some() {
                send(&mut app, Event::Postgis(P::Run));
                picture(&mut app, &format!("postgis-{mode}-4-bagli"));
            }
            // The relations window on a database's junction table and on someone else's (docs/adr/0590 §4).
            if let Some(url) = &real {
                for (schema, name) in [
                    ("iliski", "6-baglanti-tablosu"),
                    ("iliski2", "7-baska-tablo"),
                ] {
                    let mut app = app_with_drawing();
                    let _ = app
                        .settings
                        .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
                    app.apply_settings();
                    relations_on(&mut app, url, schema);
                    app.open_relations();
                    picture(&mut app, &format!("postgis-{mode}-{name}"));
                }
            }
        }
    }
}

/// `schema`'s owners and parcels bound, the parcels' `malikler` a many-to-many reference to the owners.
fn relations_on(app: &mut App, url: &str, schema: &str) {
    use kentos_commands::CallOp;
    use kentos_native_application::{ExecutionContext, registry};
    use serde_json::json;
    let model = &mut app.document.as_mut().expect("a drawing").model;
    let mut call = |command: &str, input: serde_json::Value| {
        let answer = registry::run(
            &mut ExecutionContext::new(model),
            command,
            CallOp::Execute,
            input,
        )
        .expect("runs");
        assert_eq!(answer["status"], "completed", "{answer}");
        answer["output"].clone()
    };
    let mut layer = |table: &str| {
        call(
            "cad.sources.edit",
            json!({ "edit": { "kind": "link", "path": url, "table": format!("{schema}.{table}") } }),
        )["link"]["layer"]
            .as_str()
            .expect("layer")
            .to_owned()
    };
    let owners = layer("malik");
    let parcels = layer("parsel");
    call(
        "cad.layers.edit",
        json!({ "operation": "fields", "layer": parcels, "fields": [
            { "name": "ada", "kind": "text" }, { "name": "parsel_no", "kind": "text" },
            { "name": "malikler", "kind": "text", "multiple": true } ] }),
    );
    call(
        "cad.relations.edit",
        json!({ "edit": { "kind": "create", "relation": { "id": "parsel-malik", "name": "Parselin malikleri",
            "parent": owners, "child": parcels, "field": "malikler" } } }),
    );
}
