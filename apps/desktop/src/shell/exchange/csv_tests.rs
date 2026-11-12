//! CSV içe ve dışa aktar on the desktop (docs/adr/0522): the window finds the geometry's columns by their names and
//! shows the plan of `file.import.csv`; İçe aktar takes the rows as one undo step; Dışa aktar… writes through
//! `file.export.csv`. Pictures: `csv-*` (`screens`, ignored).

use std::path::PathBuf;

use super::csv_exchange::{Event as C, Form, InForm, Sep};
use super::tests::{count, run, said, send};
use super::{Event, Window};
use crate::app::{App, Message, Picker};
use crate::files_testing::{app_with_drawing, scratch};
use kentos_commands::{CsvGeometry, CsvSeparator};

fn csv(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/csv/v1")
        .join(name)
}

fn importing(app: &App) -> &super::csv_exchange::ImportState {
    match &app.exchange {
        Some(Window::CsvImport(s)) => s,
        _ => panic!("the CSV import window"),
    }
}

#[test]
fn a_csv_goes_in_as_one_undo_step_its_geometry_found_by_name() {
    let mut app = app_with_drawing();
    let before = count(&app);
    app.picker = Picker::File(csv("parsel.csv"));
    run(&mut app, "file.import.csv");
    let s = importing(&app);
    assert_eq!(s.form, InForm::Wkt);
    assert_eq!(s.columns[s.wkt], "geom");
    let planned = s.plan.as_ref().expect("planned").0.objects;
    assert!(planned > 0);
    send(&mut app, Event::Csv(C::Run));
    assert!(app.exchange.is_none());
    assert_eq!(count(&app), before + planned as usize);
    assert!(
        said(&app, "“parsel.csv” alındı"),
        "{:?}",
        app.log.lines().last().map(|l| l.text.clone())
    );
    let _ = app.update(Message::Run("edit.undo"));
    assert_eq!(count(&app), before);
}

#[test]
fn x_and_y_columns_are_found_and_can_be_chosen_again() {
    let mut app = app_with_drawing();
    app.picker = Picker::File(csv("nokta.csv"));
    run(&mut app, "file.import.csv");
    let s = importing(&app);
    assert_eq!(s.form, InForm::Xy);
    assert_eq!(s.columns[s.x], "X");
    assert_eq!(s.columns[s.y], "Y");
    assert_eq!(s.columns[s.z - 1], "Z");
    // Geometrisiz: every row a record.
    send(&mut app, Event::Csv(C::InForm(InForm::None)));
    let out = &importing(&app).plan.as_ref().expect("planned").0;
    assert_eq!(
        out.objects,
        out.tables.iter().map(|t| t.records).sum::<u32>()
    );
}

#[test]
fn a_file_in_another_system_waits_for_the_move_to_be_asked() {
    let mut app = app_with_drawing();
    app.picker = Picker::File(csv("wgs84.csv"));
    run(&mut app, "file.import.csv");
    assert!(importing(&app).plan.is_err(), "nothing moved silently");
    send(&mut app, Event::Csv(C::Reproject));
    assert!(importing(&app).plan.is_ok());
}

#[test]
fn the_drawing_goes_out_as_a_csv_and_its_types() {
    let mut app = app_with_drawing();
    run(&mut app, "file.export.csv");
    assert!(matches!(app.exchange, Some(Window::CsvExport(_))));
    send(
        &mut app,
        Event::Csv(C::Separator(Sep(CsvSeparator::Semicolon))),
    );
    let path = scratch("csv-disa").join("cizim.csv");
    send(&mut app, Event::Csv(C::SavePicked(Some(path.clone()))));
    assert!(app.exchange.is_none(), "written and closed");
    assert!(path.exists());
    assert!(path.with_extension("csvt").exists());
    let text = std::fs::read_to_string(&path).expect("written");
    assert!(text.starts_with("WKT;"), "{}", &text[..text.len().min(80)]);
}

#[test]
#[ignore = "pictures, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    let only = std::env::var("KENTOS_SHOTS_ONLY").unwrap_or_default();
    let wanted = |name: &str| only.is_empty() || only.split(',').any(|w| name.contains(w));
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
            let fresh = || {
                let mut app = app_with_drawing();
                let _ = app
                    .settings
                    .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
                app.apply_settings();
                app
            };
            let mut app = fresh();
            app.picker = Picker::File(csv("parsel.csv"));
            run(&mut app, "file.import.csv");
            picture(&mut app, &format!("csv-{mode}-1-ice-aktar"));
            send(&mut app, Event::Csv(C::Run));
            picture(&mut app, &format!("csv-{mode}-2-alindi"));
            let mut app = fresh();
            app.picker = Picker::File(csv("nokta.csv"));
            run(&mut app, "file.import.csv");
            picture(&mut app, &format!("csv-{mode}-3-xy"));
            let mut app = fresh();
            app.picker = Picker::File(csv("wgs84.csv"));
            run(&mut app, "file.import.csv");
            picture(&mut app, &format!("csv-{mode}-4-sistem"));
            let mut app = fresh();
            run(&mut app, "file.export.csv");
            picture(&mut app, &format!("csv-{mode}-5-disa-aktar"));
            send(&mut app, Event::Csv(C::Form(Form(CsvGeometry::Xy))));
            picture(&mut app, &format!("csv-{mode}-6-xy-disa"));
        }
    }
}
