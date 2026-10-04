//! Cihaza gönder driven as the user drives it (docs/adr/0169 §4): from the
//! drawing's selection, Nokta editörü's rows and Aplikasyon's table; the
//! file is the shared writer's, byte for byte, written where the user says.

use kentos_contracts::{Entity, FieldWriteFormat, FieldWriteOptions};
use kentos_domain::Slot;

use super::field_send::{Event as Send, State, field_point};
use super::tests::{run, send};
use super::{Event, Window};
use crate::app::{App, Dialog, Message, Picker};
use crate::files_testing::{app_with_drawing, last_said, scratch};

fn window(app: &App) -> &State {
    match &app.exchange {
        Some(Window::FieldSend(s)) => s,
        other => panic!("the Cihaza gönder window, not {other:?}"),
    }
}

fn points_of(app: &App) -> Vec<Slot> {
    app.document
        .as_ref()
        .expect("open")
        .model
        .entities()
        .filter(|e| matches!(e, Entity::Point(_)))
        .map(|e| Slot(e.base().id))
        .collect()
}

#[test]
fn without_selected_points_the_command_says_what_to_do() {
    let mut app = app_with_drawing();
    run(&mut app, "field.send");
    assert!(app.exchange.is_none());
    assert!(
        last_said(&app).starts_with("Seçili nokta yok"),
        "{}",
        last_said(&app)
    );
}

#[test]
fn the_selected_points_go_out_as_the_writer_writes_them() {
    let mut app = app_with_drawing();
    let slots = points_of(&app);
    app.selection.set(slots.clone());
    run(&mut app, "field.send");
    assert_eq!(app.dialog, Some(Dialog::Exchange));
    let doc = app.document.as_ref().expect("open");
    let points: Vec<_> = slots
        .iter()
        .filter_map(|&s| doc.model.get(s))
        .filter_map(field_point)
        .collect();
    assert_eq!(
        window(&app).from(),
        format!("Çizimde seçili {} nokta", points.len())
    );
    // CSV: the writer's text exactly (no time stamp in it).
    send(&mut app, Event::FieldSend(Send::Format(5)));
    let want = kentos_formats::field::write::write(
        &points,
        &FieldWriteOptions {
            format: FieldWriteFormat::Csv,
            job: String::new(),
            stamp: String::new(),
        },
    );
    assert_eq!(window(&app).written().text, want.text);
    assert_eq!(
        window(&app).written().text,
        "P1,486513.341,4420189.522,1024.350,\r\n"
    );
    // Kaydet…: the bytes where the user says, the window closed, the line said.
    let path = scratch("field-send").join("noktalar.csv");
    app.picker = Picker::File(path.clone());
    send(&mut app, Event::FieldSend(Send::Run));
    assert_eq!(std::fs::read_to_string(&path).expect("written"), want.text);
    assert!(app.exchange.is_none());
    assert_eq!(app.dialog, None);
    assert_eq!(
        last_said(&app),
        "“noktalar.csv”: 1 nokta CSV (Ad, Y, X, Z, Kod) olarak yazıldı."
    );
    // The format is kept for the next time.
    app.selection.set(slots);
    run(&mut app, "field.send");
    assert_eq!(window(&app).format(), 5);
    send(&mut app, Event::FieldSend(Send::Format(0)));
}

#[test]
fn nokta_editoru_sends_its_rows() {
    let (mut app, _) = App::boot(None);
    let _ = app.update(Message::Opened(Some(Ok(Box::new(
        crate::files_testing::drawing(3),
    )))));
    let _ = app.update(Message::Points(crate::points::Event::Send));
    let s = window(&app);
    assert_eq!(s.from(), "Nokta editörünün 4 satırı");
    // The sample's P1 and N0…N2 (named by their Ad), GSI-16 by default.
    assert_eq!(s.written().written, 4);
    assert!(
        s.written()
            .text
            .starts_with("*110001+00000000000000P1 81..10+0000000486513341 ")
    );
}

#[test]
fn aplikasyon_sends_its_table_and_comes_back() {
    let mut app = app_with_drawing();
    let _ = app.run("calc.stakeout");
    let _ = app.update(Message::Calc(crate::calc::Event::Cell(0, 0, "p1".into())));
    let _ = app.update(Message::Calc(crate::calc::Event::Cell(
        1,
        0,
        "486000,4420000".into(),
    )));
    let _ = app.update(Message::Calc(crate::calc::Event::SendToDevice));
    let s = window(&app);
    assert_eq!(s.from(), "Aplikasyon tablosunun 2 noktası");
    // P1 found by its name; the point typed as Y,X has none and is said.
    assert_eq!(s.written().written, 1);
    assert_eq!(
        s.written()
            .skipped
            .iter()
            .map(|k| k.problem.as_str())
            .collect::<Vec<_>>(),
        ["2. noktanın adı yok; yazılmadı."]
    );
    send(&mut app, Event::Close);
    assert_eq!(app.dialog, Some(Dialog::Calc), "back to Aplikasyon");
}

/// Pictures of the window for the owner, dark and light, at 1440×900 and at
/// the smallest window (1100×650), written to `.run/shots` (never
/// committed); not run by default:
///
/// ```text
/// cargo test -p kentos-desktop exchange::field_send_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
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
            let fresh = || {
                let (mut app, _) = App::boot(None);
                let _ = app.update(Message::Opened(Some(Ok(Box::new(
                    crate::files_testing::drawing(3),
                )))));
                let _ = app
                    .settings
                    .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
                app.apply_settings();
                app
            };
            let mut app = fresh();
            let _ = app.update(Message::Points(crate::points::Event::Send));
            send(&mut app, Event::FieldSend(Send::Format(0)));
            picture(&mut app, &format!("cihaza-{mode}-1-gsi"));
            send(&mut app, Event::FieldSend(Send::Format(3)));
            picture(&mut app, &format!("cihaza-{mode}-2-jobxml"));

            let mut app = fresh();
            let _ = app.run("calc.stakeout");
            let _ = app.update(Message::Calc(crate::calc::Event::Cell(0, 0, "P1".into())));
            let _ = app.update(Message::Calc(crate::calc::Event::Cell(1, 0, "N1".into())));
            let _ = app.update(Message::Calc(crate::calc::Event::Cell(
                2,
                0,
                "486000,4420000".into(),
            )));
            let _ = app.update(Message::Calc(crate::calc::Event::SendToDevice));
            send(&mut app, Event::FieldSend(Send::Format(1)));
            picture(&mut app, &format!("cihaza-{mode}-3-aplikasyon-gsi8"));
            send(&mut app, Event::FieldSend(Send::Format(0)));
        }
    }
}
