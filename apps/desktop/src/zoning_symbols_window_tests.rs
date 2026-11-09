//! Sembolü karardan ver (zoning_symbols_window.rs, docs/adr/4240) on the shared cases' drawing of `cad.zoning.symbolize`:
//! opened from İmar planı denetimi, its rows are the levels and uses the case's answer has, with the case's counts; a
//! symbol chosen for the open konut row fills it; Sembolleri ver writes what the case with that pick writes, the
//! decisions untouched, in one undo step. And its pictures.

use kentos_launch::Module;
use serde_json::Value;

use crate::app::{App, Dialog, Message};
use crate::zoning_symbols_window::{Event as Z, pairs, rows};

fn cases() -> Value {
    serde_json::from_str(include_str!("../../../fixtures/commands/v1/cad.zoning.symbolize.json")).expect("cases")
}

fn case(name: &str) -> Value {
    cases()["cases"].as_array().expect("cases").iter().find(|c| c["name"] == name).expect("the case").clone()
}

fn opened() -> App {
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases()["setup"].clone()).expect("reads");
    let (mut app, _) = App::boot(None);
    app.program = Some(Module::Map);
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    let _ = app.update(Message::Run("zoning.check"));
    let _ = app.update(Message::ZoningSymbols(Z::Open));
    app
}

const EVERY: &str = "Bütün karar alanları KentOS'un tablosuyla: park ve spor sembolünü alır, yanlış sembollü ticaret düzelir, konut ve NİP parkı açık kalır, planı, kademesi ya da kullanımı olmayan atlanır; geri alma";
const KONUT: &str = "Kullanıcının seçtiği sembol: UİP konut alanı Gelişme konut alanı";

fn symbols(app: &App) -> Vec<(u32, Option<String>, Value)> {
    let doc = &app.document.as_ref().expect("a drawing").model;
    doc.entities().map(|e| (e.base().id, e.base().symbol.clone(), serde_json::to_value(&e.base().attrs).expect("json"))).collect()
}

/// The window's plan is the case's answer; its rows the levels and uses with a symbol or without one.
#[test]
fn the_rows_are_the_levels_and_uses_of_the_case() {
    let app = opened();
    assert_eq!(app.dialog, Some(Dialog::ZoningSymbols));
    let r = app.zoning_symbols.answer.clone().expect("an answer").expect("plans");
    let want = &case(EVERY)["steps"][0]["result"]["output"];
    assert_eq!(
        (u64::from(r.set), u64::from(r.same), u64::from(r.unmapped), u64::from(r.skipped)),
        (
            want["set"].as_u64().expect("n"),
            want["same"].as_u64().expect("n"),
            want["unmapped"].as_u64().expect("n"),
            want["skipped"].as_u64().expect("n")
        )
    );
    let table = kentos_commands::builtin_zoning_symbols().expect("the table");
    let shown = rows(&r, &table);
    let konut = shown.iter().find(|row| row[1] == "Konut alanı").expect("a konut row");
    assert_eq!((konut[0].as_str(), konut[2].as_str(), konut[3].as_str()), ("Uygulama imar planı", "— seçilmedi", "—"));
    let park = shown.iter().find(|row| row[1] == "Park" && row[0] == "Uygulama imar planı").expect("a park row");
    assert_eq!((park[2].as_str(), park[3].as_str()), ("Park", "KentOS"));
}

/// The konut row pressed and Gelişme konut alanı chosen: the plan is the user case's; Sembolleri ver writes its symbols,
/// every attribute as it was; one undo takes them back.
#[test]
fn a_symbol_chosen_for_an_open_use_is_written_and_decisions_stay() {
    let mut app = opened();
    let before = symbols(&app);
    let r = app.zoning_symbols.answer.clone().expect("an answer").expect("plans");
    let at = pairs(&r).iter().position(|p| p.level == "uip" && p.usage == "konut").expect("the konut row");
    let _ = app.update(Message::ZoningSymbols(Z::Choose(at)));
    let _ = app.update(Message::ZoningSymbols(Z::Pick("mpyy.uip.konut.gelisme-konut-alani".into())));
    let r = app.zoning_symbols.answer.clone().expect("an answer").expect("plans");
    let want = case(KONUT);
    assert_eq!(serde_json::to_value(&r).expect("json")["areas"], want["steps"][0]["result"]["output"]["areas"]);
    let _ = app.update(Message::ZoningSymbols(Z::Write));
    assert_eq!(app.dialog, None);
    let after = symbols(&app);
    for (slot, fields) in want["steps"][0]["expect"]["fields"].as_object().expect("fields") {
        let got = after.iter().find(|(id, _, _)| id.to_string() == *slot).expect("the object");
        assert_eq!(got.1.as_deref(), fields["symbol"].as_str(), "{slot}");
    }
    assert_eq!(after.iter().map(|(_, _, a)| a.clone()).collect::<Vec<_>>(), before.iter().map(|(_, _, a)| a.clone()).collect::<Vec<_>>());
    let _ = app.update(Message::Run("edit.undo"));
    assert_eq!(symbols(&app), before);
}

/// `.run/gui-evidence/sembol-karar-4240/`: İmar planı denetimi with its new button; the window with the open konut
/// row; the row pressed and its symbol chosen.
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib zoning_symbols_window_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/sembol-karar-4240");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 760.0), "koyu-dar")] {
        for (file, step) in [("0-imar-denetimi", 0), ("1-sembol-penceresi", 1), ("2-konut-secildi", 2)] {
            let mut app = opened();
            let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
            app.apply_settings();
            let _ = app.update(Message::WindowResized(size));
            let _ = app.update(Message::Run("view.zoomExtents"));
            match step {
                0 => {
                    let _ = app.update(Message::ZoningSymbols(Z::Close));
                    let _ = app.update(Message::Run("zoning.check"));
                }
                2 => {
                    let r = app.zoning_symbols.answer.clone().expect("an answer").expect("plans");
                    let at = pairs(&r).iter().position(|p| p.usage == "konut").expect("the konut row");
                    let _ = app.update(Message::ZoningSymbols(Z::Choose(at)));
                    let _ = app.update(Message::ZoningSymbols(Z::Pick("mpyy.uip.konut.gelisme-konut-alani".into())));
                }
                _ => {}
            }
            let mut snapshot = Snapshot::new(size).expect("a renderer");
            let mut update = |app: &mut App, m: Message| {
                let _ = app.update(m);
            };
            snapshot.settle(&mut app, App::view, &mut update);
            let path = out.join(format!("{file}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&path).expect("writes");
            println!("{}", path.display());
        }
    }
}
