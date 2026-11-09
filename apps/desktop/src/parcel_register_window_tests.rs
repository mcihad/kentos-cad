//! Parsel kayıtları (parcel_register_window.rs, docs/adr/4220) on the shared cases' drawing of `cad.parcels.record`:
//! opened from Parsel alan denetimi with the selected areas, Yeni belge says its plan as it is filled and Belgeyi kaydet
//! writes what the shared case writes (the register compared whole), in one undo step; a parcel pressed in the table
//! is closed by the document (the ifraz); a corner moved in the drawing is said, the register unchanged; a document's
//! name already used is refused before anything is written. And its pictures.

use kentos_domain::Slot;
use kentos_launch::Module;
use serde_json::Value;

use crate::app::{App, Dialog, Message};
use crate::parcel_register_window::{Event as R, Show, rows};

fn cases(file: &str) -> Value {
    let text = match file {
        "record" => include_str!("../../../fixtures/commands/v1/cad.parcels.record.json"),
        _ => include_str!("../../../fixtures/commands/v1/cad.parcels.records.json"),
    };
    serde_json::from_str(text).expect("cases")
}

fn case(file: &str, name: &str) -> Value {
    cases(file)["cases"].as_array().expect("cases").iter().find(|c| c["name"] == name).expect("the case").clone()
}

/// The drawing of a case (none: the file's own), open in Harita.
fn opened(setup: Value) -> App {
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(setup).expect("reads");
    let (mut app, _) = App::boot(None);
    app.program = Some(Module::Map);
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

/// Parsel alan denetimi on the selected slots, then Parsel kayıtları from it.
fn register(app: &mut App, slots: &[u32]) {
    app.selection.set(slots.iter().map(|s| Slot(*s)));
    let _ = app.update(Message::Run("parcels.check"));
    let _ = app.update(Message::ParcelRegister(R::Open));
}

fn saved_register(app: &App) -> Value {
    serde_json::to_value(&app.document.as_ref().expect("a drawing").model.settings().parcel_register).expect("json")
}

const FIRST: &str =
    "İlk belge: üç parsel; ortak sınırlar birer kez, bir parselin sınırı öbürüne numarasıyla";
const IFRAZ: &str = "İfraz: 101/7 kapanır, iki yeni parsel; ortadaki sınır ikisinin, doğu ve batı sınırları sürer";
const MOVED: &str = "Çizimde köşe taşınmış: 101/7'nin alanı değişir, belgedeki ve kayıttaki alanı değişmez; kenarları kayıttaki yerinde değil";

/// The first document from the window: A1, A2 and A3 selected (slots 1–3), named; the plan says what the case's answer
/// says, Belgeyi kaydet writes the case's register whole, the message says it, one undo takes it back.
#[test]
fn the_first_document_writes_the_shared_case_s_register() {
    let mut app = opened(cases("record")["setup"].clone());
    register(&mut app, &[1, 2, 3]);
    assert_eq!(app.dialog, Some(Dialog::ParcelRegister));
    assert!(app.parcel_register.plan.is_none(), "no plan before a name");
    let want = case("record", FIRST);
    for (field, value) in [("name", "Kadastro 1985"), ("kind", "Tapu planı"), ("date", "1985-06-12")] {
        let event = match field {
            "name" => R::Name(value.into()),
            "kind" => R::Kind(value.into()),
            _ => R::Date(value.into()),
        };
        let _ = app.update(Message::ParcelRegister(event));
    }
    let plan = app.parcel_register.plan.clone().expect("a plan").expect("plans");
    let out = &want["steps"][0]["result"]["output"];
    assert_eq!((plan.created.len() as u64, u64::from(plan.boundaries_created)), (3, out["boundariesCreated"].as_u64().expect("n")));
    let _ = app.update(Message::ParcelRegister(R::Save));
    assert_eq!(saved_register(&app), want["steps"][0]["expect"]["settings"]["parcelRegister"]);
    let said = format!(
        "Belge 1 “Kadastro 1985”: 3 parsel oluşturur (101/7, 101/8, 101/9); {} yeni sınır. Tek adımda geri alınır.",
        out["boundariesCreated"]
    );
    let log: Vec<String> = app.log.lines().map(|l| l.text.clone()).collect();
    assert!(log.iter().any(|l| l.contains(&said)), "{said}\n{log:?}");
    // The window stays and shows the register: the three parcels as recorded, 101/7 the others' neighbour.
    let r = app.parcel_register.answer.clone().expect("read");
    let f = app.format();
    let shown = rows(&r, Show::Parcels, false, &[], &f);
    assert_eq!(shown.iter().map(|row| row[4].clone()).collect::<Vec<_>>(), ["Kayıttaki gibi"; 3]);
    assert_eq!(shown[0][8], "101/8, 101/9");
    let _ = app.update(Message::Run("edit.undo"));
    assert_eq!(saved_register(&app), Value::Null);
}

/// The ifraz from the window: the register after Kadastro 1985, A11 and A12 selected (slots 11, 12); 101/7's row pressed
/// goes into Kapat; the plan names it, Belgeyi kaydet writes the shared case's register.
#[test]
fn a_parcel_pressed_in_the_table_is_closed_by_the_document() {
    let want = case("record", IFRAZ);
    let mut app = opened(want["setup"].clone());
    register(&mut app, &[11, 12]);
    let _ = app.update(Message::ParcelRegister(R::Toggle(0)));
    assert_eq!(app.parcel_register.close, [1]);
    let _ = app.update(Message::ParcelRegister(R::Name("2026/145 ifraz".into())));
    let _ = app.update(Message::ParcelRegister(R::Kind("Değişiklik".into())));
    let _ = app.update(Message::ParcelRegister(R::Date("2026-03-14".into())));
    let _ = app.update(Message::ParcelRegister(R::Note("Ölçü krokisi 2026/145.".into())));
    let r = app.parcel_register.answer.clone().expect("read");
    let f = app.format();
    assert_eq!(rows(&r, Show::Parcels, false, &app.parcel_register.close, &f)[0][9], "✓");
    let words = crate::parcel_register_window::plan_words(
        Some(&r),
        &["101/13".into(), "101/14".into()],
        &app.parcel_register.plan.clone().expect("a plan").expect("plans"),
        "2026/145 ifraz",
    );
    assert_eq!(
        words,
        "Belge 2 “2026/145 ifraz”: 2 parsel oluşturur (101/13, 101/14), 1 parsel kapatır (101/7); 5 yeni sınır, 2 kayıtlı sınır paylaşılır, 1 sınır kapanır."
    );
    let _ = app.update(Message::ParcelRegister(R::Save));
    assert_eq!(saved_register(&app), want["steps"][0]["expect"]["settings"]["parcelRegister"]);
    // Pressed again, a closed parcel's row does nothing.
    let _ = app.update(Message::ParcelRegister(R::History(true)));
    let _ = app.update(Message::ParcelRegister(R::Toggle(0)));
    assert!(app.parcel_register.close.is_empty());
}

/// docs/adr/4220's acceptance on the drawing: 101/7's corner moved, the window says the drawing parts from the record,
/// the parcel's state and areas as the shared case reads them; the register is not touched.
#[test]
fn a_moved_corner_is_said_and_the_register_stays() {
    let want = case("records", MOVED);
    let mut app = opened(want["setup"].clone());
    let before = saved_register(&app);
    register(&mut app, &[]);
    let r = app.parcel_register.answer.clone().expect("read");
    let out = &want["steps"][0]["result"]["output"];
    assert_eq!((u64::from(r.changed), u64::from(r.moved)), (out["changed"].as_u64().expect("n"), out["moved"].as_u64().expect("n")));
    let f = app.format();
    let shown = rows(&r, Show::Parcels, false, &[], &f);
    assert_eq!((shown[0][4].as_str(), shown[0][5].as_str()), ("Çizimde değişti", "200.00"));
    assert_eq!((shown[0][6].clone(), shown[0][7].clone()), (f.area(200.0), f.area(204.0)));
    assert_eq!(saved_register(&app), before);
    assert!(app.parcel_register.plan.is_none(), "nothing to make or close: no plan");
}

/// A name already used: the plan says so and Belgeyi kaydet is off; nothing is written.
#[test]
fn a_document_name_already_used_is_refused() {
    let want = case("record", IFRAZ);
    let mut app = opened(want["setup"].clone());
    let before = saved_register(&app);
    register(&mut app, &[11]);
    let _ = app.update(Message::ParcelRegister(R::Name("Kadastro 1985".into())));
    let why = app.parcel_register.plan.clone().expect("a plan").expect_err("refused");
    assert!(why.contains("“Kadastro 1985” adlı bir belge var"), "{why}");
    assert!(matches!(app.parcel_register_control(crate::traces::Control::Press(crate::parcel_register_window::SAVE)), Ok(None)));
    assert_eq!(saved_register(&app), before);
}

/// `.run/gui-evidence/parsel-kayitlari-4220/`: Parsel alan denetimi with its new button, the first document being written (Yeni belge filled), the ifraz with
/// 101/7 marked, the register after it with its history (Parseller and Sınırlar), the drawing parting from the record.
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib parcel_register_window_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/parsel-kayitlari-4220");
    std::fs::create_dir_all(&out).expect("a folder");
    type Step = fn(&mut App);
    let (check, first, ifraz, history, lines, moved): (Step, Step, Step, Step, Step, Step) =
        (check_step, first_step, ifraz_step, history_step, lines_step, moved_step);
    let record = cases("record");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 760.0), "koyu-dar")] {
        for (setup, step, file) in [
            (record["setup"].clone(), check, "0-parsel-alan-denetimi"),
            (record["setup"].clone(), first, "1-ilk-belge"),
            (case("record", IFRAZ)["setup"].clone(), ifraz, "2-ifraz"),
            (case("record", IFRAZ)["setup"].clone(), history, "3-gecmis"),
            (case("record", IFRAZ)["setup"].clone(), lines, "4-sinirlar"),
            (case("records", MOVED)["setup"].clone(), moved, "5-cizim-ayriliyor"),
        ] {
            let mut app = opened(setup);
            let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
            app.apply_settings();
            let _ = app.update(Message::WindowResized(size));
            let _ = app.update(Message::Run("view.zoomExtents"));
            step(&mut app);
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

/// Parsel alan denetimi on the three selected areas: its Parsel kayıtları… button beside Sınır grafiği….
fn check_step(app: &mut App) {
    app.selection.set([Slot(1), Slot(2), Slot(3)]);
    let _ = app.update(Message::Run("parcels.check"));
}

fn first_step(app: &mut App) {
    register(app, &[1, 2, 3]);
    for e in [R::Name("Kadastro 1985".into()), R::Kind("Tapu planı".into()), R::Date("1985-06-12".into())] {
        let _ = app.update(Message::ParcelRegister(e));
    }
}

fn ifraz_step(app: &mut App) {
    register(app, &[11, 12]);
    let _ = app.update(Message::ParcelRegister(R::Toggle(0)));
    for e in [R::Name("2026/145 ifraz".into()), R::Kind("Değişiklik".into()), R::Date("2026-03-14".into())] {
        let _ = app.update(Message::ParcelRegister(e));
    }
}

fn history_step(app: &mut App) {
    ifraz_step(app);
    let _ = app.update(Message::ParcelRegister(R::Save));
    let _ = app.update(Message::ParcelRegister(R::History(true)));
}

fn lines_step(app: &mut App) {
    history_step(app);
    let _ = app.update(Message::ParcelRegister(R::Show(Show::Boundaries)));
}

fn moved_step(app: &mut App) {
    register(app, &[]);
}
